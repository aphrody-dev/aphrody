//! Site mirror (the wget `-r -p` of a JavaScript web): every page of a site is rendered in a real browser over CDP and
//! everything it loads is kept. Per page: a full-page PNG screenshot, the rendered DOM and every network response body
//! (documents, CSS, JS, fonts, images, JSON) under `<out>/files/<host>/<path>`, plus `manifest.jsonl` (URL, status,
//! type, size). Pages come from the sitemap (nested sitemaps followed), robots.txt is honoured, `follow` walks the
//! same-origin links of rendered pages.

use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::Engine as _;
use ay_open_browser_engines::cdp::{Client, Session};
use ay_open_browser_engines::launch::{LaunchOptions, launch};
use serde_json::{Value, json};
use tokio::sync::Mutex;
use url::Url;

use crate::crawler::Crawler;
use crate::{Error, Result};

#[derive(Debug, Clone)]
pub struct MirrorConfig {
    pub site: String,
    pub out: PathBuf,
    pub workers: usize,
    pub limit: Option<usize>,
    pub max_body_bytes: usize,
    pub prefixes: Vec<String>,
    /// Follow same-origin links found in rendered pages (breadth first) instead of the sitemap alone.
    pub follow: bool,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct MirrorStats {
    pub pages: usize,
    pub failed: usize,
    pub responses: usize,
    pub bytes: u64,
}

struct Shared {
    out: PathBuf,
    max_body_bytes: usize,
    saved: Mutex<HashSet<String>>,
    manifest: Mutex<std::fs::File>,
}

pub fn file_for(out: &Path, url: &Url, mime: &str) -> PathBuf {
    let mut path = if url.path().ends_with('/') { format!("{}index", url.path()) } else { url.path().to_string() };
    let has_ext = path.rsplit('/').next().is_some_and(|last| last.rsplit_once('.').is_some_and(|(_, e)| (1..=6).contains(&e.len()) && e.bytes().all(|b| b.is_ascii_alphanumeric())));
    if !has_ext {
        if mime.contains("html") {
            path.push_str(".html");
        } else if mime.contains("json") {
            path.push_str(".json");
        }
    }
    if let Some(query) = url.query() {
        let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(query);
        path.push_str("__");
        path.push_str(&encoded[..encoded.len().min(80)]);
    }
    // Long path segments (tracking and API URLs) become a content-free hash.
    let path: String = path
        .split('/')
        .map(|seg| if seg.len() > 120 { format!("h{}", &blake3::hash(seg.as_bytes()).to_hex()[..16]) } else { seg.to_string() })
        .collect::<Vec<_>>()
        .join("/");
    let host = url.host_str().unwrap_or("unknown").replace(':', "_");
    let mut file = out.join("files").join(host);
    for seg in path.split('/').filter(|s| !s.is_empty() && *s != "." && *s != "..") {
        file.push(seg);
    }
    file
}

pub fn slug(url: &Url) -> String {
    let s = url.path().trim_matches('/').replace('/', "__");
    if s.is_empty() { "index".into() } else { s }
}

pub async fn mirror(crawler: &Arc<Crawler>, cfg: MirrorConfig) -> Result<MirrorStats> {
    std::fs::create_dir_all(&cfg.out)?;
    let site = Url::parse(&cfg.site)?;
    let in_scope = |u: &Url| cfg.prefixes.is_empty() || cfg.prefixes.iter().any(|p| u.path().starts_with(p.as_str()));
    let mut queue: VecDeque<Url> = VecDeque::new();
    let mut known: HashSet<String> = HashSet::new();
    let mut start = site.clone();
    start.set_fragment(None);
    start.set_query(None);
    if !cfg.follow {
        for url in crawler.sitemap(&site, cfg.limit.unwrap_or(usize::MAX)).await {
            let mut u = url;
            u.set_fragment(None);
            if in_scope(&u) && known.insert(u.to_string()) {
                queue.push_back(u);
            }
        }
    }
    if cfg.follow || queue.is_empty() {
        known.insert(start.to_string());
        queue.push_back(start);
    }
    let rules = crawler.robots_for(&site).await;
    let launched = launch(LaunchOptions::default()).await.map_err(|e| Error::Browser(e.to_string()))?;
    let client = Client::connect(&launched.ws_endpoint).await?;
    let shared = Arc::new(Shared {
        out: cfg.out.clone(),
        max_body_bytes: cfg.max_body_bytes,
        saved: Mutex::default(),
        manifest: Mutex::new(std::fs::OpenOptions::new().create(true).append(true).open(cfg.out.join("manifest.jsonl"))?),
    });
    let queue = Arc::new(Mutex::new((queue, known)));
    let stats = Arc::new(Mutex::new(MirrorStats::default()));
    let busy = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut handles = Vec::new();
    for _ in 0..cfg.workers.max(1) {
        let (client, shared, queue, stats, busy, rules, site, cfg) = (client.clone(), shared.clone(), queue.clone(), stats.clone(), busy.clone(), rules.clone(), site.clone(), cfg.clone());
        handles.push(tokio::spawn(async move {
            loop {
                let next = queue.lock().await.0.pop_front();
                let Some(url) = next else {
                    if busy.load(std::sync::atomic::Ordering::SeqCst) == 0 {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    continue;
                };
                let target = format!("{}{}", url.path(), url.query().map(|q| format!("?{q}")).unwrap_or_default());
                if !rules.allows(&target) {
                    continue;
                }
                busy.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let result = mirror_page(&client, &shared, &url).await;
                match result {
                    Ok(links) => {
                        stats.lock().await.pages += 1;
                        if cfg.follow {
                            let mut guard = queue.lock().await;
                            for link in links {
                                let Ok(mut l) = Url::parse(&link) else { continue };
                                l.set_fragment(None);
                                l.set_query(None);
                                if l.origin() == site.origin() && (cfg.prefixes.is_empty() || cfg.prefixes.iter().any(|p| l.path().starts_with(p.as_str()))) && cfg.limit.is_none_or(|n| guard.1.len() < n) && guard.1.insert(l.to_string()) {
                                    guard.0.push_back(l);
                                }
                            }
                        }
                    }
                    Err(error) => {
                        tracing::warn!(%url, %error, "mirror page failed");
                        stats.lock().await.failed += 1;
                        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(shared.out.join("failures.txt")) {
                            let _ = writeln!(file, "{url}\t{error}");
                        }
                    }
                }
                busy.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            }
        }));
    }
    for handle in handles {
        let _ = handle.await;
    }
    client.close().await;
    launched.close().await;
    let mut stats = stats.lock().await.clone();
    stats.responses = shared.saved.lock().await.len();
    Ok(stats)
}

struct Meta {
    url: String,
    status: u64,
    mime: String,
    kind: String,
}

/// Renders one page in a fresh tab; returns the links found in it.
async fn mirror_page(client: &Client, shared: &Arc<Shared>, url: &Url) -> Result<Vec<String>> {
    let target = client.create_target("about:blank").await?;
    let session = client.attach(&target).await?;
    let outcome = capture(&session, shared, url).await;
    session.close().await;
    outcome
}

async fn capture(session: &Session, shared: &Arc<Shared>, url: &Url) -> Result<Vec<String>> {
    session.send("Page.enable", json!({})).await?;
    session.send("Network.enable", json!({ "maxTotalBufferSize": 200_000_000, "maxResourceBufferSize": 50_000_000 })).await?;
    session.send("Emulation.setDeviceMetricsOverride", json!({ "width": 1440, "height": 900, "deviceScaleFactor": 1, "mobile": false })).await?;
    let inflight = Arc::new(std::sync::atomic::AtomicIsize::new(0));
    let collector = {
        let (session, shared, inflight) = (session.clone(), shared.clone(), inflight.clone());
        let mut events = session.events();
        tokio::spawn(async move {
            let mut metas: HashMap<String, Meta> = HashMap::new();
            while let Some(event) = events.next().await {
                let id = event.params["requestId"].as_str().unwrap_or_default().to_string();
                match event.method.as_str() {
                    "Network.requestWillBeSent" => {
                        inflight.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                    "Network.responseReceived" => {
                        let r = &event.params["response"];
                        metas.insert(id, Meta { url: r["url"].as_str().unwrap_or_default().into(), status: r["status"].as_u64().unwrap_or(0), mime: r["mimeType"].as_str().unwrap_or_default().into(), kind: event.params["type"].as_str().unwrap_or_default().into() });
                    }
                    "Network.loadingFinished" => {
                        inflight.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                        if let Some(meta) = metas.remove(&id) {
                            keep(&session, &shared, &id, meta).await;
                        }
                    }
                    "Network.loadingFailed" => {
                        inflight.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                    }
                    _ => {}
                }
            }
        })
    };
    let navigated = session.navigate(url.as_str(), Duration::from_secs(60)).await;
    let result = async {
        navigated?;
        // Scroll to trigger lazy content, then wait for the network to go quiet.
        session.evaluate("(async()=>{for(let y=0;y<document.body.scrollHeight;y+=innerHeight){scrollTo(0,y);await new Promise(r=>setTimeout(r,120))}scrollTo(0,0)})()").await?;
        let (quiet_since, limit) = (&mut None::<Instant>, Instant::now() + Duration::from_secs(20));
        while Instant::now() < limit {
            if inflight.load(std::sync::atomic::Ordering::SeqCst) <= 0 {
                if quiet_since.get_or_insert_with(Instant::now).elapsed() > Duration::from_millis(600) {
                    break;
                }
            } else {
                *quiet_since = None;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        let png = session.screenshot_png(true).await?;
        let shot = shared.out.join("screens").join(format!("{}.png", slug(url)));
        let dom = shared.out.join("dom").join(format!("{}.html", slug(url)));
        for parent in [shot.parent(), dom.parent()].into_iter().flatten() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&shot, png)?;
        let html = session.evaluate("document.documentElement.outerHTML").await?;
        std::fs::write(&dom, html.as_str().unwrap_or_default())?;
        let links = session.evaluate("[...document.querySelectorAll('a[href]')].map(a=>a.href).filter(h=>/^https?:/.test(h))").await?;
        Ok::<_, Error>(links.as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default())
    }
    .await;
    // Let the last bodies land before the collector stops.
    tokio::time::sleep(Duration::from_millis(300)).await;
    collector.abort();
    result
}

async fn keep(session: &Session, shared: &Arc<Shared>, request_id: &str, meta: Meta) {
    if !meta.url.starts_with("http") || !shared.saved.lock().await.insert(meta.url.clone()) {
        return;
    }
    let mut record = json!({ "url": meta.url, "status": meta.status, "type": meta.mime, "kind": meta.kind, "bytes": 0 });
    if let Ok(body) = session.send("Network.getResponseBody", json!({ "requestId": request_id })).await {
        let raw = body["body"].as_str().unwrap_or_default();
        let bytes = if body["base64Encoded"].as_bool().unwrap_or(false) { base64::engine::general_purpose::STANDARD.decode(raw).unwrap_or_default() } else { raw.as_bytes().to_vec() };
        record["bytes"] = Value::from(bytes.len());
        if bytes.len() <= shared.max_body_bytes {
            if let Ok(parsed) = Url::parse(&meta.url) {
                let file = file_for(&shared.out, &parsed, &meta.mime);
                match file.parent().map(std::fs::create_dir_all).transpose().and_then(|_| std::fs::write(&file, &bytes)) {
                    Ok(()) => record["file"] = Value::from(file.strip_prefix(&shared.out).unwrap_or(&file).to_string_lossy().replace('\\', "/")),
                    Err(error) => record["error"] = Value::from(error.to_string()),
                }
            }
        }
    }
    let mut manifest = shared.manifest.lock().await;
    let _ = writeln!(manifest, "{record}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names() {
        let out = Path::new("out");
        let f = |u: &str, m: &str| file_for(out, &Url::parse(u).unwrap(), m).to_string_lossy().replace('\\', "/");
        assert_eq!(f("https://e.test/", "text/html"), "out/files/e.test/index.html");
        assert_eq!(f("https://e.test/a/b.js", "text/javascript"), "out/files/e.test/a/b.js");
        assert_eq!(f("https://e.test/api/x", "application/json"), "out/files/e.test/api/x.json");
        assert!(f("https://e.test/a?q=1", "text/html").starts_with("out/files/e.test/a.html__"));
        assert_eq!(slug(&Url::parse("https://e.test/a/b/").unwrap()), "a__b");
        assert_eq!(slug(&Url::parse("https://e.test/").unwrap()), "index");
    }
}
