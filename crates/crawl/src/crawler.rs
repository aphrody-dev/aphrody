//! The crawler: robots.txt, SSRF guard, shields, politeness per host, optional rendering through a CDP browser, and a
//! breadth-first queue with bounded concurrency, depth and page count.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ay_open_browser_engines::ssrf::SsrfOptions;
use ay_open_browser_engines::SsrfGuard;
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use url::Url;

use crate::extract::extract;
use crate::render::BrowserPool;
use crate::robots::{self, RobotsRules};
use crate::shard::{Document, Shard};
use crate::shields::Shields;
use crate::{Error, Result, net};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderMode {
    /// Plain HTTP fetch of the document.
    #[default]
    Html,
    /// The page is loaded in a CDP browser and its DOM after scripts is used.
    Browser,
}

#[derive(Debug, Clone)]
pub struct CrawlerOptions {
    /// Identifies whoever runs the crawl (product token, version, contact URL); its product token is also the
    /// robots.txt group the crawler obeys.
    pub user_agent: String,
    pub respect_robots: bool,
    pub render: RenderMode,
    pub timeout: Duration,
    pub ssrf: SsrfOptions,
    pub extra_blocked_domains: Vec<String>,
    pub max_body_bytes: usize,
}

impl CrawlerOptions {
    pub fn new(user_agent: impl Into<String>) -> Self {
        Self {
            user_agent: user_agent.into(),
            respect_robots: true,
            render: RenderMode::Html,
            timeout: Duration::from_secs(15),
            ssrf: SsrfOptions::default(),
            extra_blocked_domains: Vec::new(),
            max_body_bytes: 8 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Page {
    pub url: String,
    pub status: u16,
    pub rendered: bool,
    pub title: String,
    pub description: String,
    pub headings: Vec<String>,
    pub text: String,
    pub links: Vec<String>,
    pub trackers_blocked: usize,
    pub duration_ms: f64,
    pub content_hash: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CrawlConfig {
    pub seeds: Vec<String>,
    pub max_pages: usize,
    pub max_depth: usize,
    pub concurrency: usize,
    /// Only follow links on the host of the seed that found them.
    pub same_host: bool,
    /// Only follow links whose path starts with one of these (empty: any).
    pub path_prefixes: Vec<String>,
    /// Seed the queue from sitemap.xml of each seed origin.
    pub use_sitemap: bool,
    pub shard: Option<Arc<Shard>>,
}

impl CrawlConfig {
    pub fn new(seeds: Vec<String>) -> Self {
        Self { seeds, max_pages: 100, max_depth: 2, concurrency: 8, same_host: true, path_prefixes: Vec::new(), use_sitemap: false, shard: None }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct CrawlStats {
    pub fetched: usize,
    pub failed: usize,
    pub skipped_robots: usize,
    pub links_seen: usize,
    pub errors: Vec<String>,
    pub elapsed_ms: f64,
}

pub struct Crawler {
    client: reqwest::Client,
    guard: Arc<SsrfGuard>,
    shields: Shields,
    opts: CrawlerOptions,
    robots: Mutex<HashMap<String, Arc<RobotsRules>>>,
    next_slot: Mutex<HashMap<String, Instant>>,
    browser: tokio::sync::OnceCell<Arc<BrowserPool>>,
}

impl Crawler {
    pub fn new(opts: CrawlerOptions) -> Result<Arc<Self>> {
        let guard = Arc::new(SsrfGuard::new(opts.ssrf.clone()));
        let client = net::client(guard.clone(), &opts.user_agent, opts.timeout)?;
        Ok(Arc::new(Self {
            client,
            guard,
            shields: Shields::new(opts.extra_blocked_domains.clone()),
            opts,
            robots: Mutex::default(),
            next_slot: Mutex::default(),
            browser: tokio::sync::OnceCell::new(),
        }))
    }

    pub fn guard(&self) -> &Arc<SsrfGuard> {
        &self.guard
    }

    pub fn shields(&self) -> &Shields {
        &self.shields
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn product_token(&self) -> String {
        self.opts.user_agent.split(['/', ' ']).next().unwrap_or_default().to_ascii_lowercase()
    }

    /// The rules of the origin's robots.txt; an unreachable or missing file allows everything.
    pub async fn robots_for(&self, url: &Url) -> Arc<RobotsRules> {
        let origin = url.origin().ascii_serialization();
        if let Some(rules) = self.robots.lock().await.get(&origin) {
            return rules.clone();
        }
        let rules = match self.fetch_text(&format!("{origin}/robots.txt"), Duration::from_secs(5)).await {
            Ok((200..=299, body)) => robots::parse(&body, &self.product_token()),
            _ => RobotsRules::default(),
        };
        let rules = Arc::new(rules);
        self.robots.lock().await.insert(origin, rules.clone());
        rules
    }

    async fn fetch_text(&self, url: &str, limit: Duration) -> Result<(u16, String)> {
        let url = Url::parse(url)?;
        self.guard.check_url(&url)?;
        let response = self.client.get(url).header("Accept", "text/html,application/xhtml+xml,application/xml,text/plain").timeout(limit).send().await?;
        let status = response.status().as_u16();
        let bytes = read_capped(response, self.opts.max_body_bytes).await?;
        Ok((status, String::from_utf8_lossy(&bytes).into_owned()))
    }

    /// Waits for the host's politeness slot (the robots.txt Crawl-delay).
    async fn politeness(&self, url: &Url, rules: &RobotsRules) {
        let Some(delay) = rules.crawl_delay_secs.filter(|d| *d > 0.0) else { return };
        let host = url.host_str().unwrap_or_default().to_string();
        let wait = {
            let mut slots = self.next_slot.lock().await;
            let now = Instant::now();
            let slot = slots.get(&host).copied().filter(|s| *s > now).unwrap_or(now);
            slots.insert(host, slot + Duration::from_secs_f64(delay));
            slot.saturating_duration_since(now)
        };
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
    }

    /// Fetches, extracts and optionally indexes one page.
    pub async fn page(&self, raw_url: &str) -> Result<Page> {
        let started = Instant::now();
        let url = Url::parse(raw_url)?;
        self.guard.check_url(&url)?;
        if self.shields.should_block(&url) {
            return Err(Error::Shielded(url.to_string()));
        }
        if self.opts.respect_robots {
            let rules = self.robots_for(&url).await;
            let target = format!("{}{}", url.path(), url.query().map(|q| format!("?{q}")).unwrap_or_default());
            if !rules.allows(&target) {
                return Err(Error::Robots(url.to_string()));
            }
            self.politeness(&url, &rules).await;
        }
        let (status, html, rendered) = match self.opts.render {
            RenderMode::Html => {
                let response = self.client.get(url.clone()).header("Accept", "text/html,application/xhtml+xml").send().await?;
                let status = response.status().as_u16();
                if !response.status().is_success() {
                    return Err(Error::Status(status, url.to_string()));
                }
                let bytes = read_capped(response, self.opts.max_body_bytes).await?;
                (status, String::from_utf8_lossy(&bytes).into_owned(), false)
            }
            RenderMode::Browser => {
                let pool = self.browser.get_or_try_init(|| async { BrowserPool::start(self.guard.clone(), self.shields.clone()).await.map(Arc::new) }).await?;
                (200, pool.render(&url, self.opts.timeout).await?, true)
            }
        };
        let clean = extract(&html, &url, &self.shields);
        let title = if clean.title.is_empty() { url.host_str().unwrap_or_default().to_string() } else { clean.title };
        let description = if clean.description.is_empty() { clean.text.chars().take(220).collect() } else { clean.description };
        Ok(Page {
            url: url.to_string(),
            status,
            rendered,
            title,
            description,
            headings: clean.headings,
            text: clean.text,
            links: clean.links,
            trackers_blocked: clean.trackers_blocked,
            duration_ms: (started.elapsed().as_secs_f64() * 100_000.0).round() / 100.0,
            content_hash: None,
        })
    }

    /// URLs of the origin's sitemaps (robots.txt Sitemap lines, else /sitemap.xml), nested indexes followed.
    pub async fn sitemap(&self, origin: &Url, limit: usize) -> Vec<Url> {
        let rules = self.robots_for(origin).await;
        let mut pending: Vec<String> = if rules.sitemaps.is_empty() { vec![format!("{}/sitemap.xml", origin.origin().ascii_serialization())] } else { rules.sitemaps.clone() };
        let (mut seen, mut pages) = (HashSet::new(), Vec::new());
        while let Some(next) = pending.pop() {
            if !seen.insert(next.clone()) || seen.len() > 50 || pages.len() >= limit {
                continue;
            }
            let Ok((200..=299, body)) = self.fetch_text(&next, Duration::from_secs(20)).await else { continue };
            let (nested, urls) = parse_sitemap(&body);
            pending.extend(nested);
            pages.extend(urls.into_iter().filter_map(|u| Url::parse(&u).ok()).filter(|u| u.origin() == origin.origin()));
        }
        pages.truncate(limit);
        pages
    }

    /// Breadth-first crawl. `on_page` sees every page that was fetched.
    pub async fn crawl(self: &Arc<Self>, cfg: CrawlConfig, on_page: impl Fn(&Page) + Send + Sync + 'static) -> CrawlStats {
        let started = Instant::now();
        let on_page = Arc::new(on_page);
        let mut stats = CrawlStats::default();
        let mut queue: VecDeque<(Url, usize, Url)> = VecDeque::new();
        let mut seen: HashSet<String> = HashSet::new();
        let seed_urls: Vec<Url> = cfg.seeds.iter().filter_map(|s| Url::parse(s).ok()).collect();
        for seed in &seed_urls {
            let mut first = seed.clone();
            first.set_fragment(None);
            if seen.insert(first.to_string()) {
                queue.push_back((first, 0, seed.clone()));
            }
            if cfg.use_sitemap {
                for url in self.sitemap(seed, cfg.max_pages).await {
                    if seen.insert(url.to_string()) {
                        queue.push_back((url, 1, seed.clone()));
                    }
                }
            }
        }
        let mut started_pages = 0usize;
        let mut tasks: JoinSet<(usize, Url, Result<Page>)> = JoinSet::new();
        loop {
            while tasks.len() < cfg.concurrency.max(1) && started_pages < cfg.max_pages {
                let Some((url, depth, seed)) = queue.pop_front() else { break };
                started_pages += 1;
                let crawler = self.clone();
                let shard = cfg.shard.clone();
                tasks.spawn(async move {
                    let mut result = crawler.page(url.as_str()).await;
                    if let (Ok(page), Some(shard)) = (&mut result, shard) {
                        let document = Document::from_page(page, "web");
                        let shard = shard.clone();
                        if let Ok(Ok((hash, _))) = tokio::task::spawn_blocking(move || shard.upsert(&document)).await {
                            page.content_hash = Some(hash);
                        }
                    }
                    (depth, seed, result)
                });
            }
            let Some(joined) = tasks.join_next().await else { break };
            let Ok((depth, seed, result)) = joined else { continue };
            match result {
                Ok(page) => {
                    stats.fetched += 1;
                    on_page(&page);
                    if depth < cfg.max_depth {
                        for link in &page.links {
                            stats.links_seen += 1;
                            let Ok(url) = Url::parse(link) else { continue };
                            if cfg.same_host && url.host_str() != seed.host_str() {
                                continue;
                            }
                            if !cfg.path_prefixes.is_empty() && !cfg.path_prefixes.iter().any(|p| url.path().starts_with(p.as_str())) {
                                continue;
                            }
                            if seen.insert(url.to_string()) {
                                queue.push_back((url, depth + 1, seed.clone()));
                            }
                        }
                    }
                }
                Err(Error::Robots(url)) => {
                    stats.skipped_robots += 1;
                    tracing::debug!(%url, "blocked by robots.txt");
                }
                Err(error) => {
                    stats.failed += 1;
                    if stats.errors.len() < 20 {
                        stats.errors.push(error.to_string());
                    }
                }
            }
        }
        stats.elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        stats
    }
}

async fn read_capped(mut response: reqwest::Response, cap: usize) -> Result<Vec<u8>> {
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        body.extend_from_slice(&chunk);
        if body.len() > cap {
            body.truncate(cap);
            break;
        }
    }
    Ok(body)
}

/// `(nested sitemap URLs, page URLs)` of a sitemap or sitemap index document.
pub fn parse_sitemap(xml: &str) -> (Vec<String>, Vec<String>) {
    let (mut nested, mut pages) = (Vec::new(), Vec::new());
    let mut rest = xml;
    while let Some(open) = rest.find("<loc>") {
        let after = &rest[open + 5..];
        let Some(close) = after.find("</loc>") else { break };
        let loc = after[..close].trim().replace("&amp;", "&");
        let before = &rest[..open];
        let in_index = before.rfind("<sitemap>").is_some_and(|s| before[s..].find("</sitemap>").is_none());
        if in_index { nested.push(loc) } else { pages.push(loc) }
        rest = &after[close + 6..];
    }
    (nested, pages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sitemap_index_and_urlset() {
        let xml = "<sitemapindex><sitemap><loc>https://e.test/s1.xml</loc></sitemap></sitemapindex><urlset><url><loc>https://e.test/a?x=1&amp;y=2</loc></url><url><loc>https://e.test/b</loc></url></urlset>";
        let (nested, pages) = parse_sitemap(xml);
        assert_eq!(nested, vec!["https://e.test/s1.xml"]);
        assert_eq!(pages, vec!["https://e.test/a?x=1&y=2", "https://e.test/b"]);
    }
}
