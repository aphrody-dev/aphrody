//! Registry of browsing, automation and crawl engines: where each comes from, how its binary is installed and which
//! protocol it speaks. No version is written here: installers fetch the latest. Libraries of the JS and Python crawl
//! ecosystem (Playwright, Puppeteer, Crawlee, Apify) are listed with the Rust crate of this workspace that replaces
//! them, so nothing keeps a copy of them.

use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Cdp,
    Webdriver,
    Playwright,
    Mcp,
    Cli,
    Library,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Stable,
    Beta,
    Dev,
    Canary,
}

impl Channel {
    pub const ALL: [Channel; 4] = [Channel::Stable, Channel::Beta, Channel::Dev, Channel::Canary];

    pub fn name(self) -> &'static str {
        match self {
            Channel::Stable => "stable",
            Channel::Beta => "beta",
            Channel::Dev => "dev",
            Channel::Canary => "canary",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "via", rename_all = "lowercase")]
pub enum Install {
    /// Release binary installed by `ay up <tool>`.
    Ay { tool: &'static str, bin: &'static str },
    /// Chrome for Testing (official JSON API).
    Cft { browser: &'static str, channel: Channel },
    /// Playwright browsers cache (`playwright install`), only read.
    Playwright { browser: &'static str },
    /// Built from the upstream source by the caller; `out_env` names the variable giving the build output directory.
    Build { out_env: Option<&'static str>, bin: &'static str },
    /// Used as a crate or package dependency, no binary.
    Dependency { registry: &'static str, name: &'static str },
    /// Browser installed by the system.
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Serve {
    /// Chromium family: `--headless=new` (or `old` shells that are headless already) and a DevTools port.
    Chrome { headless_flag: bool },
    Obscura,
    Lightpanda,
    Servo,
    None,
}

#[derive(Debug, Clone, Serialize)]
pub struct Engine {
    pub id: String,
    pub title: &'static str,
    pub language: &'static str,
    pub protocol: Protocol,
    pub upstream: String,
    pub install: Install,
    pub js: &'static str,
    pub gpu: &'static str,
    /// The crate of this workspace that replaces a JS or Python library, when it does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replaced_by: Option<&'static str>,
    #[serde(skip)]
    pub serve: Serve,
}

impl Engine {
    /// argv (without the executable) that exposes the protocol on 127.0.0.1:`port`.
    pub fn serve_args(&self, port: u16, profile_dir: &Path) -> Option<Vec<String>> {
        let profile = profile_dir.display();
        Some(match self.serve {
            Serve::Chrome { headless_flag } => {
                let mut args: Vec<String> = Vec::new();
                if headless_flag {
                    args.push("--headless=new".into());
                }
                args.extend([
                    format!("--remote-debugging-port={port}"),
                    "--remote-debugging-address=127.0.0.1".into(),
                    format!("--user-data-dir={profile}"),
                    "--no-first-run".into(),
                    "--no-default-browser-check".into(),
                    "--disable-background-networking".into(),
                    "--disable-sync".into(),
                    "--disable-extensions".into(),
                    "about:blank".into(),
                ]);
                args
            }
            Serve::Obscura => vec!["serve".into(), "--port".into(), port.to_string()],
            Serve::Lightpanda => vec!["serve".into(), "--host".into(), "127.0.0.1".into(), "--port".into(), port.to_string()],
            Serve::Servo => vec!["--headless".into(), format!("--webdriver={port}"), "about:blank".into()],
            Serve::None => return None,
        })
    }

    /// The engine executable when present on this machine (nothing is downloaded).
    pub fn executable(&self) -> Option<PathBuf> {
        match &self.install {
            Install::Ay { bin, .. } => local_bin(bin).or_else(|| which::which(bin).ok()),
            Install::Cft { browser, channel } => cft_index().and_then(|index| index.get(&format!("{browser}@{}", channel.name()))?.get("path")?.as_str().map(PathBuf::from)).filter(|p| p.exists()),
            Install::Playwright { browser } => playwright_executable(browser),
            Install::System => system_chrome(),
            Install::Build { out_env, bin } => {
                let out = std::env::var_os((*out_env)?)?;
                let path = Path::new(&out).join(exe(bin));
                path.exists().then_some(path)
            }
            Install::Dependency { .. } => None,
        }
    }
}

fn exe(name: &str) -> String {
    if cfg!(windows) { format!("{name}.exe") } else { name.to_string() }
}

fn home() -> Option<PathBuf> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from)
}

fn local_bin(bin: &str) -> Option<PathBuf> {
    let path = home()?.join(".local").join("bin").join(exe(bin));
    path.exists().then_some(path)
}

/// Cache of downloaded browsers: `OPEN_BROWSER_ENGINES_CACHE` (alias `BROWSER_ENGINES_CACHE`), otherwise the user cache directory of the platform.
pub fn browsers_cache() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("OPEN_BROWSER_ENGINES_CACHE").or_else(|| std::env::var_os("BROWSER_ENGINES_CACHE")) {
        return Some(PathBuf::from(dir));
    }
    let base = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from).or_else(|| home().map(|h| h.join("AppData").join("Local")))?
    } else {
        std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from).or_else(|| home().map(|h| h.join(".cache")))?
    };
    Some(base.join("browser-engines").join("browsers"))
}

fn cft_index() -> Option<serde_json::Value> {
    let text = std::fs::read_to_string(browsers_cache()?.join("cft").join("channels.json")).ok()?;
    serde_json::from_str(&text).ok()
}

fn playwright_executable(browser: &str) -> Option<PathBuf> {
    let (prefix, relatives): (&str, &[&str]) = match (browser, std::env::consts::OS) {
        ("chromium", "windows") => ("chromium-", &["chrome-win64/chrome.exe", "chrome-win/chrome.exe"]),
        ("chromium", "linux") => ("chromium-", &["chrome-linux64/chrome", "chrome-linux/chrome"]),
        ("chromium", "macos") => ("chromium-", &["chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing", "chrome-mac/Chromium.app/Contents/MacOS/Chromium"]),
        ("chromium-headless-shell", "windows") => ("chromium_headless_shell-", &["chrome-headless-shell-win64/chrome-headless-shell.exe", "chrome-win/headless_shell.exe"]),
        ("chromium-headless-shell", "linux") => ("chromium_headless_shell-", &["chrome-headless-shell-linux64/chrome-headless-shell", "chrome-linux/headless_shell"]),
        ("chromium-headless-shell", "macos") => ("chromium_headless_shell-", &["chrome-headless-shell-mac-arm64/chrome-headless-shell", "chrome-mac/headless_shell"]),
        ("webkit", "windows") => ("webkit-", &["Playwright.exe"]),
        ("webkit", _) => ("webkit-", &["pw_run.sh"]),
        _ => return None,
    };
    let root = browsers_cache()?.join("ms-playwright");
    let mut revisions: Vec<(u64, PathBuf)> = std::fs::read_dir(&root)
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let rev = name.strip_prefix(prefix)?.parse::<u64>().ok()?;
            Some((rev, entry.path()))
        })
        .collect();
    revisions.sort_by(|a, b| b.0.cmp(&a.0));
    revisions.into_iter().find_map(|(_, dir)| relatives.iter().map(|r| dir.join(r)).find(|p| p.exists()))
}

fn system_chrome() -> Option<PathBuf> {
    if cfg!(windows) {
        for base in ["PROGRAMFILES", "PROGRAMFILES(X86)", "LOCALAPPDATA"] {
            let Some(base) = std::env::var_os(base) else { continue };
            for rel in ["Google/Chrome/Application/chrome.exe", "Microsoft/Edge/Application/msedge.exe"] {
                let path = Path::new(&base).join(rel);
                if path.exists() {
                    return Some(path);
                }
            }
        }
    }
    ["google-chrome-stable", "google-chrome", "chromium", "chromium-browser", "chrome", "microsoft-edge"].iter().find_map(|name| which::which(name).ok())
}

#[allow(clippy::too_many_arguments)]
fn engine(id: &str, title: &'static str, language: &'static str, protocol: Protocol, upstream: &str, install: Install, js: &'static str, gpu: &'static str, serve: Serve) -> Engine {
    Engine { id: id.into(), title, language, protocol, upstream: upstream.into(), install, js, gpu, replaced_by: None, serve }
}

fn library(id: &str, title: &'static str, language: &'static str, upstream: &str, registry: &'static str, name: &'static str, replaced_by: Option<&'static str>) -> Engine {
    Engine { replaced_by, ..engine(id, title, language, Protocol::Library, upstream, Install::Dependency { registry, name }, "none", "none", Serve::None) }
}

/// Every known engine. The Chromium family comes first: the first one present is what `resolve_chromium` returns.
pub fn engines() -> Vec<Engine> {
    let gh = |r: &str| format!("https://github.com/{r}");
    let mut list = Vec::new();
    for channel in Channel::ALL {
        let name = channel.name();
        list.push(engine(&format!("chrome@{name}"), "Chrome for Testing", "C++", Protocol::Cdp, &gh("GoogleChromeLabs/chrome-for-testing"), Install::Cft { browser: "chrome", channel }, "v8", "dawn", Serve::Chrome { headless_flag: true }));
        list.push(engine(&format!("chrome-headless-shell@{name}"), "chrome-headless-shell", "C++", Protocol::Cdp, &gh("GoogleChromeLabs/chrome-for-testing"), Install::Cft { browser: "chrome-headless-shell", channel }, "v8", "dawn", Serve::Chrome { headless_flag: false }));
    }
    // The browser project of this stack (Blink and a lite engine behind one CDP front): its build output directory.
    list.push(engine("aphrody-browser", "Aphrody browser (Blink and a lite engine behind one CDP front)", "C++", Protocol::Cdp, "https://github.com/aphrody-labs/aphrody", Install::Build { out_env: Some("APHRODY_BROWSER_OUT"), bin: "headless_shell" }, "v8", "dawn", Serve::Chrome { headless_flag: false }));
    list.push(engine("chromium", "Chromium built from source", "C++", Protocol::Cdp, "https://chromium.googlesource.com/chromium/src", Install::Build { out_env: Some("BROWSER_ENGINES_CHROMIUM_OUT"), bin: "chrome" }, "v8", "dawn", Serve::Chrome { headless_flag: true }));
    list.push(engine("chrome", "System Chrome or Edge", "C++", Protocol::Cdp, "https://www.google.com/chrome/", Install::System, "v8", "dawn", Serve::Chrome { headless_flag: true }));
    for browser in ["chromium", "chromium-headless-shell", "webkit"] {
        let cdp = browser.starts_with("chromium");
        let mut e = engine(
            &format!("playwright-{browser}"),
            "Playwright browser cache",
            "C++",
            if cdp { Protocol::Cdp } else { Protocol::Playwright },
            &gh("microsoft/playwright"),
            Install::Playwright { browser },
            if browser == "webkit" { "javascriptcore" } else { "v8" },
            if browser == "webkit" { "none" } else { "dawn" },
            if cdp { Serve::Chrome { headless_flag: browser == "chromium" } } else { Serve::None },
        );
        if browser == "webkit" {
            e.serve = Serve::None;
        }
        list.push(e);
    }
    list.push(engine("obscura", "Obscura", "Rust", Protocol::Cdp, &gh("h4ckf0r0day/obscura"), Install::Ay { tool: "obscura", bin: "obscura" }, "v8", "none", Serve::Obscura));
    list.push(engine("lightpanda", "Lightpanda (Zig headless browser, release binary)", "Zig", Protocol::Cdp, &gh("lightpanda-io/browser"), Install::Ay { tool: "lightpanda", bin: "lightpanda" }, "v8", "none", Serve::Lightpanda));
    list.push(engine("servo", "Servo", "Rust", Protocol::Webdriver, &gh("servo/servo"), Install::Ay { tool: "servo", bin: "servo" }, "spidermonkey", "wgpu", Serve::Servo));
    list.push(engine("agent-browser", "agent-browser (CLI and MCP)", "Rust", Protocol::Cli, &gh("vercel-labs/agent-browser"), Install::Ay { tool: "agent-browser", bin: "agent-browser" }, "none", "none", Serve::None));
    list.push(engine("chrome-devtools-mcp", "Chrome DevTools MCP", "TypeScript", Protocol::Mcp, &gh("ChromeDevTools/chrome-devtools-mcp"), Install::Ay { tool: "chrome-devtools-mcp", bin: "chrome-devtools-mcp" }, "none", "none", Serve::None));
    list.push(engine("playwright-mcp", "Playwright MCP", "TypeScript", Protocol::Mcp, &gh("microsoft/playwright-mcp"), Install::Ay { tool: "playwright-mcp", bin: "playwright-mcp" }, "none", "none", Serve::None));
    list.push(engine("apify-cli", "Apify CLI", "TypeScript", Protocol::Cli, &gh("apify/apify-cli"), Install::Ay { tool: "apify-cli", bin: "apify" }, "none", "none", Serve::None));
    // Rendering and graphics libraries a browser can be built from.
    for (id, title, lang, up, js, gpu) in [
        ("blitz", "Blitz (Stylo, Taffy, anyrender)", "Rust", "DioxusLabs/blitz", "none", "wgpu"),
        ("javascriptcore", "JavaScriptCore of Bun (oven-sh/WebKit)", "C++", "oven-sh/WebKit", "javascriptcore", "none"),
        ("v8", "V8 of the Chromium checkout", "C++", "https://chromium.googlesource.com/v8/v8", "v8", "none"),
        ("dawn", "Dawn (WebGPU of Chromium)", "C++", "google/dawn", "none", "dawn"),
    ] {
        let upstream = if up.starts_with("http") { up.to_string() } else { gh(up) };
        list.push(engine(id, title, lang, Protocol::Library, &upstream, Install::Build { out_env: None, bin: "" }, js, gpu, Serve::None));
    }
    list.push(library("wgpu", "wgpu (WebGPU of Firefox, Servo, Deno)", "Rust", &gh("gfx-rs/wgpu"), "crates", "wgpu", None));
    // The JS and Python crawl ecosystem: each one maps to the Rust crate that replaces it.
    list.push(library("playwright", "Playwright (JS)", "TypeScript", &gh("microsoft/playwright"), "npm", "playwright-core", Some("browser-engines (CDP client and launcher) + browse (page tools)")));
    list.push(library("playwright-python", "Playwright for Python", "Python", &gh("microsoft/playwright-python"), "pypi", "playwright", Some("browser-engines + browse")));
    list.push(library("puppeteer", "Puppeteer", "TypeScript", &gh("puppeteer/puppeteer"), "npm", "puppeteer-core", Some("browser-engines + browse (chromiumoxide is the alternative crate)")));
    list.push(library("crawlee", "Crawlee (JS)", "TypeScript", &gh("apify/crawlee"), "npm", "crawlee", Some("crawl (queue, robots, concurrency, render, mirror)")));
    list.push(library("crawlee-python", "Crawlee for Python", "Python", &gh("apify/crawlee-python"), "pypi", "crawlee", Some("crawl")));
    list.push(library("apify-sdk", "Apify SDK (JS)", "TypeScript", &gh("apify/apify-sdk-js"), "npm", "apify", Some("crawl (storage = shard)")));
    list.push(library("apify-sdk-python", "Apify SDK (Python)", "Python", &gh("apify/apify-sdk-python"), "pypi", "apify", Some("crawl")));
    list.push(library("apify-client", "Apify client (JS)", "TypeScript", &gh("apify/apify-client-js"), "npm", "apify-client", Some("reqwest (the HTTP API of the platform)")));
    list.push(library("apify-client-python", "Apify client (Python)", "Python", &gh("apify/apify-client-python"), "pypi", "apify-client", Some("reqwest")));
    list.push(library("curl", "curl / libcurl", "C", &gh("curl/curl"), "crates", "reqwest", Some("reqwest (crawl::fetch for the command line)")));
    list.push(library("wget", "GNU wget", "C", "https://www.gnu.org/software/wget/", "crates", "reqwest", Some("crawl::mirror (recursive capture with a browser)")));
    list
}

pub fn get(id: &str) -> Option<Engine> {
    engines().into_iter().find(|e| e.id == id)
}

/// First available Chromium: `CHROME_PATH`, the Aphrody browser build, a source build, stable Chrome for Testing,
/// Playwright, the system.
pub fn resolve_chromium() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("CHROME_PATH").map(PathBuf::from).filter(|p| p.exists()) {
        return Some(path);
    }
    ["aphrody-browser", "chromium", "chrome@stable", "chrome-headless-shell@stable", "playwright-chromium", "chrome"].iter().find_map(|id| get(id)?.executable())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_libraries_name_a_replacement() {
        let list = engines();
        let mut ids: Vec<&str> = list.iter().map(|e| e.id.as_str()).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(before, ids.len());
        for id in ["playwright", "puppeteer", "crawlee", "apify-sdk", "curl", "wget"] {
            assert!(get(id).and_then(|e| e.replaced_by).is_some(), "{id}");
        }
    }

    #[test]
    fn serve_args() {
        let e = get("chrome@stable").unwrap();
        let args = e.serve_args(9222, Path::new("/tmp/p")).unwrap();
        assert!(args.contains(&"--remote-debugging-port=9222".to_string()));
        assert!(get("lightpanda").unwrap().serve_args(1, Path::new("x")).unwrap().contains(&"serve".to_string()));
        assert!(get("playwright").unwrap().serve_args(1, Path::new("x")).is_none());
    }
}
