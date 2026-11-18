//! Crawler for open. The JavaScript and Python crawl ecosystem (Crawlee, Apify SDK and client, Playwright, curl, wget) is
//! replaced by this crate on reqwest, scraper, rusqlite and the CDP client of `browser-engines`:
//!
//! - [`Crawler`]: robots.txt (RFC 9309), SSRF guard at connect time, content shields, per-host politeness, breadth-first
//!   queue, optional rendering through a browser ([`RenderMode::Browser`]).
//! - [`shard`]: SQLite FTS5 search shards (the dataset and key-value store of an Apify actor, searchable).
//! - [`mirror`]: wget-style site capture with a real browser (screenshots, rendered DOM, every response body).
//! - [`fetch`]: the curl of this workspace, guarded.

pub mod crawler;
pub mod extract;
pub mod fetch;
pub mod mirror;
pub mod net;
pub mod render;
pub mod robots;
pub mod shard;
pub mod shields;

pub use crawler::{CrawlConfig, CrawlStats, Crawler, CrawlerOptions, Page, RenderMode};
pub use shard::{Document, Hit, Shard};

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("url: {0}")]
    Url(#[from] url::ParseError),
    #[error("{0}")]
    Ssrf(#[from] ay_open_browser_engines::SsrfError),
    #[error("cdp: {0}")]
    Cdp(#[from] ay_open_browser_engines::CdpError),
    #[error("browser: {0}")]
    Browser(String),
    #[error("blocked by content shields: {0}")]
    Shielded(String),
    #[error("blocked by robots.txt: {0}")]
    Robots(String),
    #[error("HTTP {0} while fetching {1}")]
    Status(u16, String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}
