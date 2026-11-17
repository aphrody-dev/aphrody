// SQLite store of the watch: normalised items with FTS5, per-source poll state, version observations and metrics.
import { Database } from "bun:sqlite";
import { mkdirSync } from "node:fs";
import { dirname } from "node:path";
import type { Item, Metric, VersionObs } from "./adapters.ts";

export interface SourceState {
  source: string;
  etag: string | null;
  lastModified: string | null;
  lastPolled: number | null;
  nextAllowed: number;
  fails: number;
  lastStatus: number | null;
  lastError: string | null;
  lastChanged: number | null;
}
export interface StoredItem extends Item {
  rowid: number;
  source: string;
  kernel: string;
  firstSeen: number;
}
export interface VersionRow {
  kernel: string;
  source: string;
  channel: string;
  version: string;
  since: number;
  checked: number;
}
export interface SearchHit {
  source: string;
  kernel: string;
  kind: string;
  id: string;
  title: string;
  url: string;
  time: number;
  version: string | null;
  snippet: string;
  rank: number;
}
export interface ApplyResult {
  inserted: number;
  changed: number;
  unchanged: number;
}

const SCHEMA = `
CREATE TABLE IF NOT EXISTS items (
  rowid INTEGER PRIMARY KEY, source TEXT NOT NULL, kernel TEXT NOT NULL, kind TEXT NOT NULL, ext_id TEXT NOT NULL,
  title TEXT NOT NULL, url TEXT NOT NULL, author TEXT NOT NULL, time INTEGER NOT NULL, updated INTEGER, version TEXT,
  body TEXT NOT NULL, meta TEXT, first_seen INTEGER NOT NULL, UNIQUE (source, kind, ext_id));
CREATE INDEX IF NOT EXISTS items_kernel_time ON items (kernel, time DESC);
CREATE INDEX IF NOT EXISTS items_kind_time ON items (kind, time DESC);
CREATE VIRTUAL TABLE IF NOT EXISTS items_fts USING fts5(title, body, author, kind UNINDEXED, source UNINDEXED, kernel UNINDEXED, version, tokenize = 'porter unicode61');
CREATE TABLE IF NOT EXISTS state (
  source TEXT PRIMARY KEY, etag TEXT, last_modified TEXT, last_polled INTEGER, next_allowed INTEGER NOT NULL DEFAULT 0,
  fails INTEGER NOT NULL DEFAULT 0, last_status INTEGER, last_error TEXT, last_changed INTEGER) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS versions (
  source TEXT NOT NULL, kernel TEXT NOT NULL, channel TEXT NOT NULL, version TEXT NOT NULL, since INTEGER NOT NULL, checked INTEGER NOT NULL,
  PRIMARY KEY (source, channel)) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS version_history (source TEXT NOT NULL, channel TEXT NOT NULL, version TEXT NOT NULL, at INTEGER NOT NULL, PRIMARY KEY (source, channel, version)) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS metrics (source TEXT NOT NULL, kernel TEXT NOT NULL, name TEXT NOT NULL, at INTEGER NOT NULL, value INTEGER NOT NULL, PRIMARY KEY (source, name, at)) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS runs (id INTEGER PRIMARY KEY, started INTEGER NOT NULL, finished INTEGER, stats TEXT);
`;

export class Store {
  readonly db: Database;

  constructor(path: string) {
    if (path !== ":memory:") mkdirSync(dirname(path), { recursive: true });
    this.db = new Database(path, { create: true, strict: true });
    this.db.run("PRAGMA journal_mode = WAL");
    this.db.run("PRAGMA synchronous = NORMAL");
    this.db.exec(SCHEMA);
  }

  close(): void {
    this.db.run("PRAGMA wal_checkpoint(TRUNCATE)");
    this.db.close();
  }

  state(source: string): SourceState {
    const row = this.db.query("SELECT * FROM state WHERE source = ?").get(source) as any;
    return {
      source, etag: row?.etag ?? null, lastModified: row?.last_modified ?? null, lastPolled: row?.last_polled ?? null,
      nextAllowed: row?.next_allowed ?? 0, fails: row?.fails ?? 0, lastStatus: row?.last_status ?? null, lastError: row?.last_error ?? null,
      lastChanged: row?.last_changed ?? null,
    };
  }

  saveState(s: SourceState): void {
    this.db
      .query(
        `INSERT INTO state (source, etag, last_modified, last_polled, next_allowed, fails, last_status, last_error, last_changed)
         VALUES ($source, $etag, $lastModified, $lastPolled, $nextAllowed, $fails, $lastStatus, $lastError, $lastChanged)
         ON CONFLICT (source) DO UPDATE SET etag = $etag, last_modified = $lastModified, last_polled = $lastPolled, next_allowed = $nextAllowed,
           fails = $fails, last_status = $lastStatus, last_error = $lastError, last_changed = $lastChanged`,
      )
      .run({ ...s });
  }

  allStates(): SourceState[] {
    return (this.db.query("SELECT source FROM state ORDER BY source").all() as { source: string }[]).map((r) => this.state(r.source));
  }

  /** Writes one page of results atomically. An item counts as changed when its updated time or title moved. */
  apply(source: string, kernel: string, items: Item[], versions: VersionObs[], metrics: Metric[], now: number): ApplyResult {
    const result: ApplyResult = { inserted: 0, changed: 0, unchanged: 0 };
    const find = this.db.query("SELECT rowid, title, updated, body, meta FROM items WHERE source = ? AND kind = ? AND ext_id = ?");
    const insert = this.db.query(
      `INSERT INTO items (source, kernel, kind, ext_id, title, url, author, time, updated, version, body, meta, first_seen)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
    );
    const update = this.db.query("UPDATE items SET title = ?, url = ?, author = ?, time = ?, updated = ?, version = ?, body = ?, meta = ? WHERE rowid = ?");
    const ftsDelete = this.db.query("DELETE FROM items_fts WHERE rowid = ?");
    const ftsInsert = this.db.query("INSERT INTO items_fts (rowid, title, body, author, kind, source, kernel, version) VALUES (?, ?, ?, ?, ?, ?, ?, ?)");

    this.db.transaction(() => {
      for (const item of items) {
        const meta = item.meta ? JSON.stringify(item.meta) : null;
        const existing = find.get(source, item.kind, item.id) as any;
        if (!existing) {
          const info = insert.run(source, kernel, item.kind, item.id, item.title, item.url, item.author, item.time, item.updated ?? null, item.version ?? null, item.body, meta, now);
          ftsInsert.run(info.lastInsertRowid, item.title, item.body, item.author, item.kind, source, kernel, item.version ?? "");
          result.inserted++;
        } else if (existing.title !== item.title || (existing.updated ?? null) !== (item.updated ?? null) || existing.meta !== meta || existing.body !== item.body) {
          update.run(item.title, item.url, item.author, item.time, item.updated ?? null, item.version ?? null, item.body, meta, existing.rowid);
          ftsDelete.run(existing.rowid);
          ftsInsert.run(existing.rowid, item.title, item.body, item.author, item.kind, source, kernel, item.version ?? "");
          result.changed++;
        } else result.unchanged++;
      }
      for (const v of versions) {
        const current = this.db.query("SELECT version, since FROM versions WHERE source = ? AND channel = ?").get(source, v.channel) as any;
        const since = current?.version === v.version ? current.since : now;
        this.db.query("INSERT OR REPLACE INTO versions (source, kernel, channel, version, since, checked) VALUES (?, ?, ?, ?, ?, ?)").run(source, kernel, v.channel, v.version, since, now);
        this.db.query("INSERT OR IGNORE INTO version_history (source, channel, version, at) VALUES (?, ?, ?, ?)").run(source, v.channel, v.version, now);
      }
      for (const m of metrics) {
        const last = this.db.query("SELECT value FROM metrics WHERE source = ? AND name = ? ORDER BY at DESC LIMIT 1").get(source, m.name) as any;
        if (!last || last.value !== m.value) this.db.query("INSERT OR REPLACE INTO metrics (source, kernel, name, at, value) VALUES (?, ?, ?, ?, ?)").run(source, kernel, m.name, now, m.value);
      }
    })();
    return result;
  }

  rowToItem(r: any): StoredItem {
    return {
      rowid: r.rowid, source: r.source, kernel: r.kernel, id: r.ext_id, kind: r.kind, title: r.title, url: r.url, author: r.author, time: r.time,
      ...(r.updated !== null ? { updated: r.updated } : {}), ...(r.version !== null ? { version: r.version } : {}), body: r.body,
      ...(r.meta ? { meta: JSON.parse(r.meta) } : {}), firstSeen: r.first_seen,
    };
  }

  items(where: { kernel?: string; source?: string; kind?: string; since?: number; limit?: number }): StoredItem[] {
    const clauses: string[] = [];
    const args: (string | number)[] = [];
    if (where.kernel) (clauses.push("kernel = ?"), args.push(where.kernel));
    if (where.source) (clauses.push("source = ?"), args.push(where.source));
    if (where.kind) (clauses.push("kind = ?"), args.push(where.kind));
    if (where.since !== undefined) (clauses.push("time >= ?"), args.push(where.since));
    const sql = `SELECT rowid, * FROM items ${clauses.length ? `WHERE ${clauses.join(" AND ")}` : ""} ORDER BY time DESC, source, ext_id ${where.limit ? `LIMIT ${Math.floor(where.limit)}` : ""}`;
    return (this.db.query(sql).all(...args) as any[]).map((r) => this.rowToItem(r));
  }

  versionRows(): VersionRow[] {
    return this.db.query("SELECT kernel, source, channel, version, since, checked FROM versions ORDER BY kernel, channel").all() as VersionRow[];
  }

  versionHistory(): { source: string; channel: string; version: string; at: number }[] {
    return this.db.query("SELECT source, channel, version, at FROM version_history ORDER BY source, channel, at, version").all() as any[];
  }

  /** Latest value of a metric and its value at (or before) a past time. */
  metric(source: string, name: string, before: number): { current: number | null; past: number | null } {
    const cur = this.db.query("SELECT value FROM metrics WHERE source = ? AND name = ? ORDER BY at DESC LIMIT 1").get(source, name) as any;
    const past = this.db.query("SELECT value FROM metrics WHERE source = ? AND name = ? AND at <= ? ORDER BY at DESC LIMIT 1").get(source, name, before) as any;
    return { current: cur?.value ?? null, past: past?.value ?? null };
  }

  metricNames(): { source: string; kernel: string; name: string }[] {
    return this.db.query("SELECT DISTINCT source, kernel, name FROM metrics ORDER BY source, name").all() as any[];
  }

  search(query: string, opts: { kernel?: string; kind?: string; limit?: number } = {}): SearchHit[] {
    const clauses = ["items_fts MATCH ?"];
    const args: (string | number)[] = [toFtsQuery(query)];
    if (opts.kernel) (clauses.push("items_fts.kernel = ?"), args.push(opts.kernel));
    if (opts.kind) (clauses.push("items_fts.kind = ?"), args.push(opts.kind));
    const rows = this.db
      .query(
        `SELECT i.source, i.kernel, i.kind, i.ext_id AS id, i.title, i.url, i.time, i.version, snippet(items_fts, 1, '[', ']', '...', 12) AS snippet, bm25(items_fts) AS rank
         FROM items_fts JOIN items i ON i.rowid = items_fts.rowid WHERE ${clauses.join(" AND ")} ORDER BY rank, i.time DESC LIMIT ${Math.floor(opts.limit ?? 20)}`,
      )
      .all(...args);
    return rows as SearchHit[];
  }

  /** Drops high-volume items (commits, mail, pages, packages) older than the retention window. */
  prune(olderThan: number, kinds: string[]): number {
    const marks = kinds.map(() => "?").join(",");
    const doomed = this.db.query(`SELECT rowid FROM items WHERE time < ? AND time > 0 AND kind IN (${marks})`).all(olderThan, ...kinds) as { rowid: number }[];
    this.db.transaction(() => {
      for (const { rowid } of doomed) {
        this.db.query("DELETE FROM items_fts WHERE rowid = ?").run(rowid);
        this.db.query("DELETE FROM items WHERE rowid = ?").run(rowid);
      }
    })();
    return doomed.length;
  }

  startRun(now: number): number {
    return Number(this.db.query("INSERT INTO runs (started) VALUES (?)").run(now).lastInsertRowid);
  }
  finishRun(id: number, now: number, stats: unknown): void {
    this.db.query("UPDATE runs SET finished = ?, stats = ? WHERE id = ?").run(now, JSON.stringify(stats), id);
  }
  counts(): { items: number; sources: number; versions: number } {
    const n = (sql: string): number => (this.db.query(sql).get() as any).n;
    return { items: n("SELECT count(*) n FROM items"), sources: n("SELECT count(*) n FROM state"), versions: n("SELECT count(*) n FROM versions") };
  }
}

/** Turns free text into a safe FTS5 query: quoted terms, prefix match on the last one. */
export function toFtsQuery(text: string): string {
  const terms = text.match(/[\p{L}\p{N}_.-]+/gu) ?? [];
  if (terms.length === 0) throw new Error("empty search query");
  return terms.map((t, i) => `"${t.replace(/"/g, '""')}"${i === terms.length - 1 ? "*" : ""}`).join(" ");
}
