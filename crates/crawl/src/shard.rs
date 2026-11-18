//! Search shards: one SQLite database per shard (WAL) with an FTS5 index ranked by BM25, and content hashes that skip
//! unchanged documents. A shard is any name the caller chooses; how shards map to hosts is the caller's concern.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use crate::Result;
use crate::crawler::Page;

#[derive(Debug, Clone)]
pub struct Document {
    pub url: String,
    pub title: String,
    pub description: String,
    pub headings: Vec<String>,
    pub body: String,
    pub domain: String,
    pub category: String,
    pub quality_score: f64,
    pub crawled_at_ms: i64,
}

impl Document {
    pub fn from_page(page: &Page, category: &str) -> Self {
        let domain = url::Url::parse(&page.url).ok().and_then(|u| u.host_str().map(str::to_ascii_lowercase)).unwrap_or_default();
        Self {
            url: page.url.clone(),
            title: page.title.clone(),
            description: page.description.clone(),
            headings: page.headings.clone(),
            body: page.text.clone(),
            domain,
            category: category.to_string(),
            quality_score: 0.92,
            crawled_at_ms: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or_default(),
        }
    }

    pub fn content_hash(&self) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(self.url.as_bytes());
        hasher.update(b"\n");
        hasher.update(self.title.as_bytes());
        hasher.update(b"\n");
        hasher.update(self.body.as_bytes());
        hasher.finalize().to_hex().to_string()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Hit {
    pub url: String,
    pub title: String,
    pub snippet: String,
    pub domain: String,
    pub category: String,
    pub shard: String,
    pub bm25: f64,
    pub quality_score: f64,
    pub crawled_at_ms: i64,
}

impl std::fmt::Debug for Shard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Shard").field("id", &self.id).finish_non_exhaustive()
    }
}

pub struct Shard {
    pub id: String,
    conn: Mutex<Connection>,
}

/// Prefix-matches every word of the query (at most 12), joined with OR.
fn fts_query(raw: &str) -> String {
    raw.to_lowercase()
        .split(|c: char| !(c.is_alphanumeric() || matches!(c, '_' | '.' | '-')))
        .filter(|t| !t.is_empty())
        .take(12)
        .map(|t| format!("\"{}\"*", t.replace('"', "")))
        .collect::<Vec<_>>()
        .join(" OR ")
}

impl Shard {
    pub fn open(id: &str, path: Option<&Path>) -> Result<Self> {
        let conn = match path {
            Some(path) => Connection::open(path)?,
            None => Connection::open_in_memory()?,
        };
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE IF NOT EXISTS documents (
               url TEXT PRIMARY KEY, title TEXT NOT NULL, description TEXT NOT NULL, headings_json TEXT NOT NULL,
               body TEXT NOT NULL, domain TEXT NOT NULL, category TEXT NOT NULL, shard TEXT NOT NULL,
               quality_score REAL NOT NULL DEFAULT 1.0, crawled_at INTEGER NOT NULL, content_hash TEXT NOT NULL);
             CREATE VIRTUAL TABLE IF NOT EXISTS search_fts USING fts5(
               url UNINDEXED, title, description, headings, body, domain, tokenize = 'unicode61 remove_diacritics 2');",
        )?;
        Ok(Self { id: id.to_string(), conn: Mutex::new(conn) })
    }

    /// Inserts or replaces a document; `(hash, false)` when the stored content is unchanged.
    pub fn upsert(&self, doc: &Document) -> Result<(String, bool)> {
        let hash = doc.content_hash();
        let mut conn = self.conn.lock().unwrap();
        let existing: Option<String> = conn.query_row("SELECT content_hash FROM documents WHERE url = ?1", [&doc.url], |r| r.get(0)).optional()?;
        if existing.as_deref() == Some(hash.as_str()) {
            return Ok((hash, false));
        }
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM search_fts WHERE url = ?1", [&doc.url])?;
        tx.execute(
            "INSERT INTO documents (url, title, description, headings_json, body, domain, category, shard, quality_score, crawled_at, content_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(url) DO UPDATE SET title = excluded.title, description = excluded.description, headings_json = excluded.headings_json,
               body = excluded.body, domain = excluded.domain, category = excluded.category, shard = excluded.shard,
               quality_score = excluded.quality_score, crawled_at = excluded.crawled_at, content_hash = excluded.content_hash",
            params![doc.url, doc.title, doc.description, serde_json::to_string(&doc.headings)?, doc.body, doc.domain, doc.category, self.id, doc.quality_score, doc.crawled_at_ms, hash],
        )?;
        tx.execute(
            "INSERT INTO search_fts (url, title, description, headings, body, domain) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![doc.url, doc.title, doc.description, doc.headings.join(" | "), doc.body, doc.domain],
        )?;
        tx.commit()?;
        Ok((hash, true))
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<Hit>> {
        let conn = self.conn.lock().unwrap();
        let fts = fts_query(query);
        if fts.is_empty() {
            let mut stmt = conn.prepare("SELECT url, title, description, domain, category, shard, quality_score, crawled_at FROM documents ORDER BY quality_score DESC, crawled_at DESC LIMIT ?1")?;
            let rows = stmt.query_map([limit as i64], |r| {
                Ok(Hit { url: r.get(0)?, title: r.get(1)?, snippet: r.get(2)?, domain: r.get(3)?, category: r.get(4)?, shard: r.get(5)?, bm25: r.get(6)?, quality_score: r.get(6)?, crawled_at_ms: r.get(7)? })
            })?;
            return Ok(rows.collect::<std::result::Result<_, _>>()?);
        }
        let mut stmt = conn.prepare(
            "SELECT d.url, d.title, snippet(search_fts, 4, '[', ']', ' ... ', 24), d.description, d.domain, d.category, d.shard,
                    bm25(search_fts, 0.0, 8.0, 4.0, 3.0, 1.5, 2.0), d.quality_score, d.crawled_at
             FROM search_fts JOIN documents d ON d.url = search_fts.url
             WHERE search_fts MATCH ?1
             ORDER BY (bm25(search_fts, 0.0, 8.0, 4.0, 3.0, 1.5, 2.0) * d.quality_score) ASC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![fts, limit as i64], |r| {
            let snippet: String = r.get(2)?;
            let description: String = r.get(3)?;
            let raw: f64 = r.get(7)?;
            let quality: f64 = r.get(8)?;
            Ok(Hit {
                url: r.get(0)?,
                title: r.get(1)?,
                snippet: if snippet.is_empty() { description } else { snippet },
                domain: r.get(4)?,
                category: r.get(5)?,
                shard: r.get(6)?,
                bm25: (raw.abs() * quality * 1000.0).round() / 1000.0,
                quality_score: quality,
                crawled_at_ms: r.get(9)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn suggest(&self, prefix: &str, limit: usize) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT title FROM documents WHERE lower(title) LIKE ?1 ORDER BY quality_score DESC LIMIT ?2")?;
        let rows = stmt.query_map(params![format!("%{}%", prefix.trim().to_lowercase()), limit as i64], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn count(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        Ok(conn.query_row("SELECT COUNT(*) FROM documents", [], |r| r.get::<_, i64>(0))? as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(url: &str, title: &str, body: &str) -> Document {
        Document { url: url.into(), title: title.into(), description: String::new(), headings: vec![], body: body.into(), domain: "e.test".into(), category: "web".into(), quality_score: 1.0, crawled_at_ms: 1 }
    }

    #[test]
    fn index_search_and_unchanged_skip() {
        let shard = Shard::open("s0", None).unwrap();
        assert!(shard.upsert(&doc("https://e.test/a", "Rust crawler", "a fast crawler written in rust")).unwrap().1);
        assert!(shard.upsert(&doc("https://e.test/b", "Cooking", "pasta and sauce")).unwrap().1);
        assert!(!shard.upsert(&doc("https://e.test/a", "Rust crawler", "a fast crawler written in rust")).unwrap().1);
        assert_eq!(shard.count().unwrap(), 2);
        let hits = shard.search("crawl", 5).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].url, "https://e.test/a");
        assert!(hits[0].snippet.contains('['));
        assert_eq!(shard.suggest("cook", 3).unwrap(), vec!["Cooking"]);
        assert_eq!(shard.search("", 5).unwrap().len(), 2);
    }
}
