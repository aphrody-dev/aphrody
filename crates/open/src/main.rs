//! The `open` command: one binary over the surfaces of the framework.

use std::path::PathBuf;
use std::time::Duration;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "open", version, about = "open: browse, crawl, tunnel, route and render from one binary")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Features compiled into this binary.
    Features,
    /// Engine registry: list engines, resolve the Chromium that would launch.
    #[cfg(feature = "engines")]
    Engines {
        #[command(subcommand)]
        action: EnginesCmd,
    },
    /// Crawl from seed URLs (robots.txt, SSRF guard, shields), optionally indexing into a search shard.
    #[cfg(feature = "crawl")]
    Crawl(CrawlArgs),
    /// One guarded HTTP request, like curl.
    #[cfg(feature = "crawl")]
    Fetch(FetchArgs),
    /// Capture a whole site with a real browser: screenshots, rendered DOM, every response body.
    #[cfg(feature = "crawl")]
    Mirror(MirrorArgs),
    /// Search a shard written by `crawl --shard`.
    #[cfg(feature = "crawl")]
    Search { shard: PathBuf, query: String, #[arg(short, long, default_value_t = 10)] limit: usize },
    /// Agent browser: MCP over CDP, the extension relay, a scripted session.
    #[cfg(feature = "browse")]
    Browse {
        #[command(subcommand)]
        action: BrowseCmd,
    },
    /// Expose local ports through a blind TLS tunnel.
    #[cfg(feature = "tunnel")]
    Tunnel {
        #[command(subcommand)]
        action: TunnelCmd,
    },
    /// Full-screen chat over the router (OpenTUI).
    #[cfg(feature = "tui")]
    Tui,
}

#[cfg(feature = "engines")]
#[derive(Subcommand)]
enum EnginesCmd {
    /// Print the registry as JSON.
    List,
    /// The Chromium-family binary that would be launched.
    Resolve,
}

#[cfg(feature = "crawl")]
#[derive(clap::Args)]
struct CrawlArgs {
    seeds: Vec<String>,
    /// Identifies whoever runs the crawl; its product token is the robots.txt group obeyed.
    #[arg(long, default_value = "open-crawl/0.5 (+https://github.com/aphrody-labs/open)")]
    user_agent: String,
    #[arg(long, default_value_t = 20)]
    max_pages: usize,
    #[arg(long, default_value_t = 2)]
    depth: usize,
    #[arg(long, default_value_t = 8)]
    concurrency: usize,
    /// Render pages in a headless browser instead of fetching the HTML.
    #[arg(long)]
    render: bool,
    /// Seed the queue from sitemap.xml.
    #[arg(long)]
    sitemap: bool,
    /// Follow links to other hosts too.
    #[arg(long)]
    any_host: bool,
    /// Ignore robots.txt (never for sites you do not run).
    #[arg(long)]
    no_robots: bool,
    /// Allow private and loopback addresses (local sites).
    #[arg(long)]
    allow_private: bool,
    /// Index every page into this SQLite shard.
    #[arg(long)]
    shard: Option<PathBuf>,
    /// Print full pages (text included) instead of one summary line each.
    #[arg(long)]
    full: bool,
}

#[cfg(feature = "crawl")]
#[derive(clap::Args)]
struct FetchArgs {
    url: String,
    #[arg(short = 'X', long, default_value = "GET")]
    method: String,
    #[arg(short = 'H', long = "header")]
    headers: Vec<String>,
    #[arg(short = 'd', long)]
    data: Option<String>,
    /// Print only the status line and headers.
    #[arg(short = 'I', long)]
    head: bool,
    #[arg(short = 'o', long)]
    output: Option<PathBuf>,
    #[arg(long)]
    no_follow: bool,
    #[arg(long)]
    allow_private: bool,
}

#[cfg(feature = "crawl")]
#[derive(clap::Args)]
struct MirrorArgs {
    #[arg(long)]
    site: String,
    #[arg(long)]
    out: PathBuf,
    #[arg(long, default_value_t = 4)]
    workers: usize,
    #[arg(long)]
    limit: Option<usize>,
    #[arg(long, value_delimiter = ',')]
    prefix: Vec<String>,
    /// Follow same-origin links of rendered pages instead of the sitemap alone.
    #[arg(long)]
    follow: bool,
    #[arg(long, default_value_t = 100 * 1024 * 1024)]
    max_bytes: usize,
    #[arg(long)]
    allow_private: bool,
}

#[cfg(feature = "browse")]
#[derive(Subcommand)]
enum BrowseCmd {
    /// MCP server over stdio. Without --endpoint a headless browser is launched.
    Mcp {
        #[arg(long)]
        endpoint: Option<String>,
    },
    /// Run the relay the Chrome extension pairs with.
    Relay,
    /// List the tools of the relay build as JSON.
    Tools,
    /// Pair an unpacked extension build (extension/dist) with this user's relay.
    Pair {
        #[arg(long)]
        extension: PathBuf,
        #[arg(long)]
        port: Option<u16>,
    },
    /// A scripted session: open the URL in a browser over CDP, print a snapshot, save a screenshot.
    Demo {
        url: String,
        #[arg(long, default_value = ".")]
        out: PathBuf,
        #[arg(long)]
        endpoint: Option<String>,
        /// Load this unpacked extension (extension/dist) into the launched browser; needs Chrome for Testing or Chromium.
        #[arg(long)]
        extension: Option<PathBuf>,
    },
}

#[cfg(feature = "tunnel")]
#[derive(Subcommand)]
enum TunnelCmd {
    /// Create the profile's tunnel if needed and keep it connected.
    Up {
        /// name=host:port, repeatable.
        #[arg(long = "route", required = true)]
        routes: Vec<String>,
        #[arg(long, default_value = "default")]
        profile: String,
        /// API base URL of the tunnel service.
        #[arg(long)]
        api: Option<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    runtime.block_on(run(cli.command))
}

async fn run(command: Command) -> anyhow::Result<()> {
    match command {
        Command::Features => {
            println!("{}", open::FEATURES.join(" "));
            Ok(())
        }
        #[cfg(feature = "engines")]
        Command::Engines { action } => {
            match action {
                EnginesCmd::List => {
                    let list: Vec<_> = open::engines::registry::engines()
                        .into_iter()
                        .map(|e| {
                            let mut v = serde_json::to_value(&e).unwrap_or_default();
                            v["present"] = serde_json::json!(e.executable().map(|p| p.display().to_string()));
                            v
                        })
                        .collect();
                    println!("{}", serde_json::to_string_pretty(&list)?);
                }
                EnginesCmd::Resolve => match open::engines::registry::resolve_chromium() {
                    Some(path) => println!("{}", path.display()),
                    None => anyhow::bail!("no Chromium-family browser found: set CHROME_PATH"),
                },
            }
            Ok(())
        }
        #[cfg(feature = "crawl")]
        Command::Crawl(a) => crawl_cmd(a).await,
        #[cfg(feature = "crawl")]
        Command::Fetch(a) => fetch_cmd(a).await,
        #[cfg(feature = "crawl")]
        Command::Mirror(a) => mirror_cmd(a).await,
        #[cfg(feature = "crawl")]
        Command::Search { shard, query, limit } => {
            let shard = open::crawl::Shard::open("cli", Some(&shard))?;
            for hit in shard.search(&query, limit)? {
                println!("{}", serde_json::to_string(&hit)?);
            }
            Ok(())
        }
        #[cfg(feature = "browse")]
        Command::Browse { action } => browse_cmd(action).await,
        #[cfg(feature = "tunnel")]
        Command::Tunnel { action } => tunnel_cmd(action).await,
        #[cfg(feature = "tui")]
        Command::Tui => {
            let router = std::sync::Arc::new(open::router::Router::open(&open::router::Config::default()).map_err(|e| anyhow::anyhow!("{e}"))?);
            open::tui::chat::run(router).map_err(|e| anyhow::anyhow!("{e}"))
        }
    }
}

#[cfg(feature = "crawl")]
fn ssrf(allow_private: bool) -> open::engines::ssrf::SsrfOptions {
    open::engines::ssrf::SsrfOptions { allow_private_network: allow_private, ..Default::default() }
}

#[cfg(feature = "crawl")]
async fn crawl_cmd(a: CrawlArgs) -> anyhow::Result<()> {
    use open::crawl::{CrawlConfig, Crawler, CrawlerOptions, RenderMode, Shard};
    anyhow::ensure!(!a.seeds.is_empty(), "give at least one seed URL");
    let mut options = CrawlerOptions::new(a.user_agent);
    options.respect_robots = !a.no_robots;
    options.render = if a.render { RenderMode::Browser } else { RenderMode::Html };
    options.ssrf = ssrf(a.allow_private);
    let crawler = Crawler::new(options)?;
    let mut cfg = CrawlConfig::new(a.seeds);
    cfg.max_pages = a.max_pages;
    cfg.max_depth = a.depth;
    cfg.concurrency = a.concurrency;
    cfg.same_host = !a.any_host;
    cfg.use_sitemap = a.sitemap;
    if let Some(path) = &a.shard {
        cfg.shard = Some(std::sync::Arc::new(Shard::open("cli", Some(path))?));
    }
    let full = a.full;
    let stats = crawler
        .crawl(cfg, move |page| {
            if full {
                println!("{}", serde_json::to_string(page).unwrap_or_default());
            } else {
                println!("{}\t{}\t{} links\t{} chars\t{:.0} ms\t{}", page.status, page.url, page.links.len(), page.text.len(), page.duration_ms, page.title);
            }
        })
        .await;
    eprintln!("{}", serde_json::to_string(&stats)?);
    Ok(())
}

#[cfg(feature = "crawl")]
async fn fetch_cmd(a: FetchArgs) -> anyhow::Result<()> {
    use open::crawl::fetch::{Request, send};
    let mut request = Request::get(a.url);
    request.method = a.method;
    request.body = a.data.map(String::into_bytes);
    request.follow_redirects = !a.no_follow;
    request.ssrf = ssrf(a.allow_private);
    request.timeout = Duration::from_secs(30);
    for header in a.headers {
        let (name, value) = header.split_once(':').ok_or_else(|| anyhow::anyhow!("header {header:?} must be Name: value"))?;
        request.headers.push((name.trim().to_string(), value.trim().to_string()));
    }
    let response = send(request).await?;
    if a.head {
        println!("{} {}", response.version, response.status);
        for (name, value) in &response.headers {
            println!("{name}: {value}");
        }
    } else if let Some(path) = a.output {
        std::fs::write(&path, &response.body)?;
        eprintln!("{} {} bytes -> {}", response.status, response.body.len(), path.display());
    } else {
        use std::io::Write as _;
        std::io::stdout().write_all(&response.body)?;
    }
    Ok(())
}

#[cfg(feature = "crawl")]
async fn mirror_cmd(a: MirrorArgs) -> anyhow::Result<()> {
    use open::crawl::{Crawler, CrawlerOptions, mirror};
    let mut options = CrawlerOptions::new("open-mirror/0.5 (+https://github.com/aphrody-labs/open)");
    options.ssrf = ssrf(a.allow_private);
    let crawler = Crawler::new(options)?;
    let stats = mirror::mirror(&crawler, mirror::MirrorConfig { site: a.site, out: a.out, workers: a.workers, limit: a.limit, max_body_bytes: a.max_bytes, prefixes: a.prefix, follow: a.follow }).await?;
    println!("{}", serde_json::to_string(&stats)?);
    Ok(())
}

#[cfg(feature = "browse")]
async fn browse_cmd(action: BrowseCmd) -> anyhow::Result<()> {
    use std::sync::Arc;

    use open::browse::{self, DirectBrowser, core::{Browser, TabOptions, ToolContext}};

    async fn direct(endpoint: Option<String>, extension: Option<PathBuf>) -> anyhow::Result<Arc<DirectBrowser>> {
        match endpoint {
            Some(endpoint) => DirectBrowser::connect(&endpoint).await,
            None => DirectBrowser::launch(open::engines::LaunchOptions { extension, ..Default::default() }).await,
        }
    }
    match action {
        BrowseCmd::Tools => {
            let specs: Vec<_> = browse::relay_tools().iter().map(|t| t.spec()).collect();
            println!("{}", serde_json::to_string_pretty(&specs)?);
            Ok(())
        }
        BrowseCmd::Pair { extension, port } => {
            println!("{}", browse::pair_unpacked(&extension, port)?);
            Ok(())
        }
        BrowseCmd::Relay => {
            browse::serve_relay(browse::relay::ServeOptions::default()).await
        }
        BrowseCmd::Mcp { endpoint } => {
            let browser = direct(endpoint, None).await?;
            let ctx = ToolContext { browser: browser as Arc<dyn Browser>, file_roots: vec![std::env::current_dir()?] };
            browse::serve_stdio(browse::direct_tools(), ctx, "open-browse", env!("CARGO_PKG_VERSION")).await
        }
        BrowseCmd::Demo { url, out, endpoint, extension } => {
            std::fs::create_dir_all(&out)?;
            let extension = extension.map(|p| p.canonicalize()).transpose()?;
            let browser = direct(endpoint, extension).await?;
            let ctx = ToolContext { browser: browser.clone() as Arc<dyn Browser>, file_roots: vec![out.canonicalize()?] };
            let tools: std::collections::HashMap<String, _> = browse::direct_tools().into_iter().map(|t| (t.spec().name, t)).collect();
            let tab = browser.open_tab(Some(&url), &TabOptions::default()).await?;
            println!("tab {} opened at {url}", tab.tab);
            let targets = browser.client().send("Target.getTargets", serde_json::json!({}), None).await?;
            let mut kinds: std::collections::BTreeMap<String, usize> = Default::default();
            for target in targets["targetInfos"].as_array().into_iter().flatten() {
                *kinds.entry(format!("{} {}", target["type"].as_str().unwrap_or("?"), target["url"].as_str().filter(|u| u.starts_with("chrome-extension:")).unwrap_or(""))).or_default() += 1;
            }
            println!("targets: {kinds:?}");
            for (name, args) in [("snapshot", serde_json::json!({ "tab": tab.tab })), ("screenshot", serde_json::json!({ "tab": tab.tab, "path": "browse-demo.png", "fullPage": true })), ("tabs", serde_json::json!({ "action": "list" }))] {
                let output = tools[name].call(ctx.clone(), args).await?;
                for content in &output.content {
                    if let browse::core::Content::Text { text } = content {
                        println!("--- {name}{}\n{text}", if output.is_error { " (error)" } else { "" });
                    }
                }
            }
            browser.close_tab(tab.tab).await?;
            Ok(())
        }
    }
}

#[cfg(feature = "tunnel")]
async fn tunnel_cmd(action: TunnelCmd) -> anyhow::Result<()> {
    let TunnelCmd::Up { routes, profile, api } = action;
    let routes = open::tunnel::parse_routes(&routes).map_err(|e| anyhow::anyhow!(e))?;
    let names: Vec<String> = routes.keys().cloned().collect();
    let api = api.map(|a| url::Url::parse(&a)).transpose()?;
    let mut up = open::tunnel::up(&profile, routes, api).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    for name in &names {
        println!("{name}: {}", up.url(name));
    }
    let mut events = up.tunnel.subscribe();
    tokio::spawn(async move {
        while let Ok(event) = events.recv().await {
            eprintln!("{}", serde_json::to_string(&event).unwrap_or_default());
        }
    });
    up.tunnel.wait().await.map_err(|e| anyhow::anyhow!("{e}"))
}
