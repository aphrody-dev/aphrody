//! Pure-Rust fused HTTP/sitemap/HTML mirror replacing legacy `curl` / `wget` / `monolith`
//! subprocess calls using `ureq` (`platform-verifier` TLS), `roxmltree`, and `html5ever`.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
#[cfg(not(target_family = "wasm"))]
use std::time::Duration;

use anyhow::{Context as _, Result};
use html5ever::tendril::TendrilSink as _;
use html5ever::{ParseOpts, parse_document};
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(not(target_family = "wasm"))]
use ureq::Agent;
use url::Url;

use crate::BUNDLED_DESIGN_SITEMAP_TOML;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SitemapUrlEntry {
    pub loc: String,
    #[serde(default)]
    pub lastmod: String,
    #[serde(default)]
    pub priority: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignArticleSnapshot {
    pub url: String,
    pub title: String,
    pub description: String,
    pub headings: Vec<String>,
    pub summary: String,
    #[serde(default)]
    pub paragraphs: Vec<String>,
    #[serde(default)]
    pub live_fetched: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpstreamRepoMeta {
    pub repo: String,
    pub url: String,
    pub license: String,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StitchRouteEntry {
    pub path: String,
    pub title: String,
    pub capability: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignSitemapSnapshot {
    pub schema_version: u32,
    pub synced_at: String,
    pub design_google_sitemap_url: String,
    pub stitch_url: String,
    pub total_urls: usize,
    #[serde(default)]
    pub upstream_ecosystem: Vec<UpstreamRepoMeta>,
    #[serde(default)]
    pub stitch_routes: Vec<StitchRouteEntry>,
    #[serde(default)]
    pub featured_articles: Vec<DesignArticleSnapshot>,
    #[serde(default)]
    pub urls: Vec<SitemapUrlEntry>,
}

impl DesignSitemapSnapshot {
    pub fn bundled() -> Result<Self> {
        toml::from_str(BUNDLED_DESIGN_SITEMAP_TOML)
            .context("parsing bundled design_google_sitemap_snapshot.toml")
    }
}

/// Constructs a hardened pure-Rust HTTP client (`curl`/`wget` equivalent) with OS root certificates.
#[cfg(not(target_family = "wasm"))]
pub fn build_http_agent() -> Agent {
    Agent::config_builder()
        .http_status_as_error(true)
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                .build(),
        )
        .max_redirects(8)
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_recv_response(Some(Duration::from_secs(20)))
        .build()
        .into()
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        if let Err(err) = write!(out, "{byte:02x}") {
            log::warn!("Failed formatting digest byte: {err}");
        }
    }
    out
}

/// Downloads raw bytes from any `http://` or `https://` URL via pure-Rust `ureq`.
#[cfg(not(target_family = "wasm"))]
pub fn fetch_bytes(url: &str, bearer_or_api_key: Option<(&str, &str)>) -> Result<Vec<u8>> {
    let agent = build_http_agent();
    let mut req = agent
        .get(url)
        .header("User-Agent", "Aphrody-Design-Mirror/1.0");
    if let Some((header_name, header_val)) = bearer_or_api_key {
        req = req.header(header_name, header_val);
    }
    let mut response = req
        .call()
        .with_context(|| format!("GET request failed for {url}"))?;
    let bytes = response
        .body_mut()
        .read_to_vec()
        .with_context(|| format!("reading response body for {url}"))?;
    Ok(bytes)
}

#[cfg(target_family = "wasm")]
pub fn fetch_bytes(_url: &str, _bearer_or_api_key: Option<(&str, &str)>) -> Result<Vec<u8>> {
    anyhow::bail!("Synchronous ureq HTTP client is disabled on wasm32; use bundled snapshot")
}

pub fn fetch_text(url: &str, bearer_or_api_key: Option<(&str, &str)>) -> Result<String> {
    let bytes = fetch_bytes(url, bearer_or_api_key)?;
    String::from_utf8(bytes).with_context(|| format!("decoding UTF-8 response for {url}"))
}

#[cfg(not(target_family = "wasm"))]
pub fn post_json(
    url: &str,
    api_key_header: Option<(&str, &str)>,
    body: &serde_json::Value,
) -> Result<serde_json::Value> {
    let agent = build_http_agent();
    let payload = serde_json::to_vec(body)?;
    let mut req = agent
        .post(url)
        .header("User-Agent", "Aphrody-Design-Mirror/1.0")
        .header("Content-Type", "application/json");
    if let Some((k, v)) = api_key_header {
        req = req.header(k, v);
    }
    let mut response = req
        .send(payload.as_slice())
        .with_context(|| format!("POST JSON failed for {url}"))?;
    let raw = response.body_mut().read_to_vec()?;
    serde_json::from_slice(&raw).with_context(|| format!("parsing JSON response from {url}"))
}

#[cfg(target_family = "wasm")]
pub fn post_json(
    _url: &str,
    _api_key_header: Option<(&str, &str)>,
    _body: &serde_json::Value,
) -> Result<serde_json::Value> {
    anyhow::bail!("Synchronous ureq HTTP client is disabled on wasm32; use bundled snapshot")
}

/// Parses an XML `<urlset>` sitemap document via pure-Rust `roxmltree`.
pub fn parse_sitemap_xml(xml: &str) -> Result<Vec<SitemapUrlEntry>> {
    let doc = roxmltree::Document::parse(xml).context("parsing sitemap XML")?;
    let mut entries = Vec::new();
    for node in doc.descendants() {
        if node.tag_name().name() != "url" {
            continue;
        }
        let mut loc = String::new();
        let mut lastmod = String::new();
        let mut priority = String::new();
        for child in node.children() {
            let text = child.text().unwrap_or("").trim();
            match child.tag_name().name() {
                "loc" => loc = text.to_owned(),
                "lastmod" => lastmod = text.to_owned(),
                "priority" => priority = text.to_owned(),
                _ => {}
            }
        }
        if !loc.is_empty() {
            entries.push(SitemapUrlEntry {
                loc,
                lastmod,
                priority,
            });
        }
    }
    Ok(entries)
}

/// Fetches the live `https://design.google/sitemap.xml` with optional disk cache & bundled fallback.
pub fn mirror_design_google_sitemap(
    cache_dir: Option<&Path>,
    allow_live: bool,
) -> Result<Vec<SitemapUrlEntry>> {
    if allow_live {
        match fetch_text("https://design.google/sitemap.xml", None) {
            Ok(xml) => {
                let parsed = parse_sitemap_xml(&xml)?;
                if !parsed.is_empty() {
                    if let Some(dir) = cache_dir
                        && let Err(err) = write_cache_file(dir, "design-google-sitemap.xml", xml.as_bytes())
                    {
                        log::warn!("Failed writing sitemap cache: {err}");
                    }
                    return Ok(parsed);
                }
            }
            Err(err) => {
                log::warn!("Live design.google sitemap fetch fell back to cache/snapshot: {err}");
            }
        }
    }

    if let Some(dir) = cache_dir {
        let cached = dir.join("design-google-sitemap.xml");
        if cached.is_file()
            && let Ok(xml) = fs::read_to_string(&cached)
            && let Ok(parsed) = parse_sitemap_xml(&xml)
        {
            return Ok(parsed);
        }
    }

    Ok(DesignSitemapSnapshot::bundled()?.urls)
}

fn collect_dom_text(handle: &Handle, out: &mut String) {
    match &handle.data {
        NodeData::Text { contents } => out.push_str(&contents.borrow()),
        NodeData::Element { name, .. } => {
            if matches!(name.local.as_ref(), "script" | "style" | "noscript") {
                return;
            }
            for child in handle.children.borrow().iter() {
                collect_dom_text(child, out);
            }
        }
        _ => {
            for child in handle.children.borrow().iter() {
                collect_dom_text(child, out);
            }
        }
    }
}

fn walk_article_dom(
    handle: &Handle,
    title: &mut String,
    description: &mut String,
    headings: &mut Vec<String>,
    paragraphs: &mut Vec<String>,
    stylesheet_hrefs: &mut Vec<String>,
) {
    if let NodeData::Element { name, attrs, .. } = &handle.data {
        let tag = name.local.as_ref();
        match tag {
            "title" if title.is_empty() => {
                let mut buf = String::new();
                collect_dom_text(handle, &mut buf);
                *title = normalize_whitespace(&buf);
                return;
            }
            "meta" if description.is_empty() => {
                let attrs_ref = attrs.borrow();
                let is_desc = attrs_ref.iter().any(|a| {
                    a.name.local.as_ref() == "name"
                        && a.value.eq_ignore_ascii_case("description")
                });
                if is_desc
                    && let Some(content) = attrs_ref
                        .iter()
                        .find(|a| a.name.local.as_ref() == "content")
                {
                    *description = normalize_whitespace(&content.value);
                }
                return;
            }
            "link" => {
                let attrs_ref = attrs.borrow();
                let is_css = attrs_ref.iter().any(|a| {
                    a.name.local.as_ref() == "rel" && a.value.eq_ignore_ascii_case("stylesheet")
                });
                if is_css
                    && let Some(href) = attrs_ref.iter().find(|a| a.name.local.as_ref() == "href")
                {
                    stylesheet_hrefs.push(href.value.to_string());
                }
                return;
            }
            "h1" | "h2" | "h3" if headings.len() < 24 => {
                let mut buf = String::new();
                collect_dom_text(handle, &mut buf);
                let clean = normalize_whitespace(&buf);
                if !clean.is_empty() {
                    headings.push(clean);
                }
                return;
            }
            "p" if paragraphs.len() < 16 => {
                let mut buf = String::new();
                collect_dom_text(handle, &mut buf);
                let clean = normalize_whitespace(&buf);
                if clean.len() >= 24 {
                    paragraphs.push(clean);
                }
                return;
            }
            "script" | "style" | "noscript" => return,
            _ => {}
        }
    }

    for child in handle.children.borrow().iter() {
        walk_article_dom(child, title, description, headings, paragraphs, stylesheet_hrefs);
    }
}

fn normalize_whitespace(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Extracts structured title, meta description, headings, and body paragraphs using `html5ever`.
pub fn extract_article_from_html(url: &str, html: &str) -> (DesignArticleSnapshot, Vec<String>) {
    let dom: RcDom = parse_document(RcDom::default(), ParseOpts::default()).one(html);
    let mut title = String::new();
    let mut description = String::new();
    let mut headings = Vec::new();
    let mut paragraphs = Vec::new();
    let mut stylesheets = Vec::new();

    walk_article_dom(
        &dom.document,
        &mut title,
        &mut description,
        &mut headings,
        &mut paragraphs,
        &mut stylesheets,
    );

    let summary = paragraphs
        .first()
        .cloned()
        .unwrap_or_else(|| description.clone());
    (
        DesignArticleSnapshot {
            url: url.to_owned(),
            title,
            description,
            headings,
            summary,
            paragraphs,
            live_fetched: true,
        },
        stylesheets,
    )
}

/// Mirrors a webpage + top linked stylesheets into a single standalone HTML archive (pure-Rust `monolith` core).
pub fn mirror_to_single_file_html(url: &str, cache_dir: Option<&Path>) -> Result<(String, DesignArticleSnapshot)> {
    let html = fetch_text(url, None)?;
    let (article, stylesheets) = extract_article_from_html(url, &html);
    let base = Url::parse(url).context("parsing base URL")?;

    let mut inlined_css = String::new();
    for href in stylesheets.iter().take(4) {
        if let Ok(abs) = base.join(href)
            && let Ok(css) = fetch_text(abs.as_str(), None)
        {
            inlined_css.push_str("\n/* Inlined by aphrody_design from ");
            inlined_css.push_str(abs.as_str());
            inlined_css.push_str(" */\n");
            inlined_css.push_str(&css);
        }
    }

    let bundled = if inlined_css.is_empty() {
        html
    } else if let Some(pos) = html.find("</head>") {
        let mut out = String::with_capacity(html.len() + inlined_css.len() + 32);
        out.push_str(&html[..pos]);
        out.push_str("<style data-aphrody-mirror=\"true\">");
        out.push_str(&inlined_css);
        out.push_str("</style>");
        out.push_str(&html[pos..]);
        out
    } else {
        html
    };

    if let Some(dir) = cache_dir {
        let key = sha256_hex(url.as_bytes());
        if let Err(err) = write_cache_file(dir, &format!("{key}.html"), bundled.as_bytes()) {
            log::warn!("Failed writing single-file mirror cache: {err}");
        }
    }

    Ok((bundled, article))
}

/// Fetches a live `https://design.google/...` article or falls back cleanly to bundled snapshots.
pub fn fetch_or_fallback_article(
    url: &str,
    cache_dir: Option<&Path>,
    allow_live: bool,
) -> Result<DesignArticleSnapshot> {
    if allow_live {
        match fetch_text(url, None) {
            Ok(html) => {
                let (article, _) = extract_article_from_html(url, &html);
                if let Some(dir) = cache_dir
                    && let Ok(json_bytes) = serde_json::to_vec_pretty(&article)
                    && let Err(err) = write_cache_file(
                        dir,
                        &format!("{}.json", sha256_hex(url.as_bytes())),
                        &json_bytes,
                    )
                {
                    log::warn!("Failed caching mirrored article JSON: {err}");
                }
                return Ok(article);
            }
            Err(err) => log::warn!("Live article mirror failed for {url}, using fallback: {err}"),
        }
    }

    if let Some(dir) = cache_dir {
        let cache_file = dir.join(format!("{}.json", sha256_hex(url.as_bytes())));
        if cache_file.is_file()
            && let Ok(bytes) = fs::read(&cache_file)
            && let Ok(article) = serde_json::from_slice::<DesignArticleSnapshot>(&bytes)
        {
            return Ok(article);
        }
    }

    let snapshot = DesignSitemapSnapshot::bundled()?;
    if let Some(found) = snapshot
        .featured_articles
        .into_iter()
        .find(|a| a.url == url)
    {
        return Ok(found);
    }

    Ok(DesignArticleSnapshot {
        url: url.to_owned(),
        title: "Google Design".to_owned(),
        description: "Material 3 Expressive design system source of truth".to_owned(),
        headings: vec!["Google Design".to_owned()],
        summary: "Fallback entry while offline".to_owned(),
        paragraphs: Vec::new(),
        live_fetched: false,
    })
}

fn write_cache_file(dir: &Path, file_name: &str, bytes: &[u8]) -> Result<()> {
    fs::create_dir_all(dir)?;
    let tmp = dir.join(format!("{file_name}.tmp"));
    let final_path = dir.join(file_name);
    fs::write(&tmp, bytes)?;
    fs::rename(tmp, final_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sitemap_xml_and_extracts_html5_article() -> Result<()> {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url><loc>https://design.google/library/code-is-a-design-material</loc><lastmod>2026-01-01</lastmod><priority>0.9</priority></url>
  <url><loc>https://design.google/library/google-sans-flex-font</loc><lastmod>2026-01-02</lastmod><priority>0.8</priority></url>
</urlset>"#;
        let urls = parse_sitemap_xml(xml)?;
        assert_eq!(urls.len(), 2);
        assert_eq!(
            urls[0].loc,
            "https://design.google/library/code-is-a-design-material"
        );

        let html = r#"<!doctype html><html><head><title>Code is a Design Material</title><meta name="description" content="UX fundamentals"></head><body><h1>Code is a Design Material</h1><h2>Why thoughtwork matters</h2><p>Code bridges design intention and interactive software engineering.</p></body></html>"#;
        let (article, _) = extract_article_from_html(
            "https://design.google/library/code-is-a-design-material",
            html,
        );
        assert_eq!(article.title, "Code is a Design Material");
        assert_eq!(article.description, "UX fundamentals");
        assert_eq!(article.headings.len(), 2);
        assert!(article.summary.contains("interactive software engineering"));
        Ok(())
    }

    #[test]
    fn bundled_sitemap_snapshot_loads_real_articles() -> Result<()> {
        let snap = DesignSitemapSnapshot::bundled()?;
        assert!(snap.total_urls >= 2);
        assert!(!snap.featured_articles.is_empty());
        assert_eq!(snap.stitch_url, "https://stitch.withgoogle.com/");
        Ok(())
    }
}
