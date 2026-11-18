//! Headless Browser Automation & CDP Engine absorbed from `aphrody-labs/agent-browser`.
//!
//! Provides SSRF/non-public IP policy checks, multi-OS Chrome/Chromium/Edge/Brave binary
//! discovery, CDP HTTP discovery (`/json/version`, `/json/list`), asynchronous WebSocket CDP
//! execution (open, DOM/AX snapshot, JS eval, click, type, PNG screenshot), and unified
//! `browser_*` MCP tools inside `aphrody_pilot`.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
#[cfg(not(target_family = "wasm"))]
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserLaunchOptions {
    pub headless: bool,
    pub executable_path: Option<String>,
    pub cdp_url: Option<String>,
    pub allow_file_access: bool,
    pub allow_private_network: bool,
    pub webmcp: bool,
    pub webgpu: bool,
    pub user_agent: Option<String>,
}

impl Default for BrowserLaunchOptions {
    fn default() -> Self {
        Self {
            headless: true,
            executable_path: None,
            cdp_url: None,
            allow_file_access: false,
            allow_private_network: false,
            webmcp: true,
            webgpu: true,
            user_agent: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserUrlCheck {
    pub url: String,
    pub scheme: String,
    pub host: String,
    pub allowed: bool,
    pub reason: Option<String>,
}

/// True for loopback addresses including IPv4-mapped IPv6 (`::ffff:127.0.0.1`).
#[must_use]
pub fn is_loopback_ip(ip: IpAddr) -> bool {
    ip.to_canonical().is_loopback()
}

/// Link-local & cloud metadata addresses (`169.254.169.254`, `fd00:ec2::254`, `100.100.100.200`).
#[must_use]
pub fn is_link_local_or_metadata(ip: IpAddr) -> bool {
    match ip.to_canonical() {
        IpAddr::V4(v4) => v4.is_link_local() || v4 == Ipv4Addr::new(100, 100, 100, 200),
        IpAddr::V6(v6) => {
            (v6.segments()[0] & 0xffc0) == 0xfe80
                || v6 == Ipv6Addr::new(0xfd00, 0xec2, 0, 0, 0, 0, 0, 0x254)
        }
    }
}

/// True for any non-public IPv4/IPv6 range that remote browser automation must guard against SSRF.
#[must_use]
pub fn is_non_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => non_public_v4(v4),
        IpAddr::V6(v6) => non_public_v6(v6),
    }
}

fn non_public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_multicast()
        || a == 0
        || (a == 100 && (b & 0xc0) == 64)
        || (a == 192 && b == 0 && (c == 0 || c == 2))
        || (a == 198 && b == 51 && c == 100)
        || (a == 203 && b == 0 && c == 113)
        || (a == 198 && (b & 0xfe) == 18)
        || a >= 240
}

fn non_public_v6(ip: Ipv6Addr) -> bool {
    let s = ip.segments();
    let low_v4 = (s[..5] == [0; 5] && s[5] == 0xffff)
        || (s[..4] == [0; 4] && s[4] == 0xffff && s[5] == 0)
        || s[..6] == [0; 6]
        || s[..6] == [0x64, 0xff9b, 0, 0, 0, 0];
    let embedded = if low_v4 {
        Some(Ipv4Addr::new(
            (s[6] >> 8) as u8,
            s[6] as u8,
            (s[7] >> 8) as u8,
            s[7] as u8,
        ))
    } else if s[0] == 0x2002 {
        Some(Ipv4Addr::new(
            (s[1] >> 8) as u8,
            s[1] as u8,
            (s[2] >> 8) as u8,
            s[2] as u8,
        ))
    } else {
        None
    };
    if let Some(v4) = embedded {
        return non_public_v4(v4);
    }
    ip.is_multicast()
        || ip.is_loopback()
        || ip.is_unspecified()
        || (s[0] & 0xfe00) == 0xfc00
        || (s[0] & 0xffc0) == 0xfe80
}

/// Validates navigation URLs before dispatch using `agent-browser` security rules.
#[must_use]
pub fn validate_navigation_url(raw: &str, options: &BrowserLaunchOptions) -> BrowserUrlCheck {
    let trimmed = raw.trim();
    let Some((scheme, rest)) = trimmed.split_once("://") else {
        if trimmed.eq_ignore_ascii_case("about:blank") {
            return BrowserUrlCheck {
                url: trimmed.to_owned(),
                scheme: "about".to_owned(),
                host: String::new(),
                allowed: true,
                reason: None,
            };
        }
        return BrowserUrlCheck {
            url: trimmed.to_owned(),
            scheme: String::new(),
            host: String::new(),
            allowed: false,
            reason: Some("URL must use explicit http(s):// scheme".to_owned()),
        };
    };

    let scheme_lower = scheme.to_ascii_lowercase();
    let host_port = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host_only = host_port
        .rsplit('@')
        .next()
        .unwrap_or("")
        .trim_start_matches('[')
        .split(']')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();

    match scheme_lower.as_str() {
        "file" => BrowserUrlCheck {
            url: trimmed.to_owned(),
            scheme: scheme_lower,
            host: host_only,
            allowed: options.allow_file_access,
            reason: if options.allow_file_access {
                None
            } else {
                Some("file:// URLs blocked unless allow_file_access=true".to_owned())
            },
        },
        "http" | "https" => {
            if host_only.is_empty() {
                return BrowserUrlCheck {
                    url: trimmed.to_owned(),
                    scheme: scheme_lower,
                    host: host_only,
                    allowed: false,
                    reason: Some("empty URL host".to_owned()),
                };
            }
            if !options.allow_private_network {
                if host_only == "localhost"
                    || host_only.ends_with(".local")
                    || host_only.ends_with(".internal")
                    || host_only == "metadata.google.internal"
                {
                    return BrowserUrlCheck {
                        url: trimmed.to_owned(),
                        scheme: scheme_lower,
                        host: host_only,
                        allowed: false,
                        reason: Some(
                            "loopback/internal hostname blocked by browser SSRF policy".to_owned(),
                        ),
                    };
                }
                if let Ok(ip) = host_only.parse::<IpAddr>()
                    && (is_non_public_ip(ip) || is_link_local_or_metadata(ip))
                {
                    return BrowserUrlCheck {
                        url: trimmed.to_owned(),
                        scheme: scheme_lower,
                        host: host_only,
                        allowed: false,
                        reason: Some(format!("non-public IP {ip} blocked by browser SSRF policy")),
                    };
                }
            }
            BrowserUrlCheck {
                url: trimmed.to_owned(),
                scheme: scheme_lower,
                host: host_only,
                allowed: true,
                reason: None,
            }
        }
        other => BrowserUrlCheck {
            url: trimmed.to_owned(),
            scheme: other.to_owned(),
            host: host_only,
            allowed: false,
            reason: Some(format!("unsupported browser scheme '{other}'")),
        },
    }
}

/// Generates hardened Chrome headless launch arguments matching `agent-browser` WebMCP & WebGPU presets.
#[must_use]
pub fn build_chrome_launch_args(
    options: &BrowserLaunchOptions,
    user_data_dir: &str,
) -> Vec<String> {
    let mut enable_features = vec![
        "NetworkService".to_owned(),
        "NetworkServiceInProcess".to_owned(),
    ];
    if options.webmcp {
        enable_features.push("WebMCPTesting".to_owned());
        enable_features.push("DevToolsWebMCPSupport".to_owned());
    }
    if options.webgpu && cfg!(target_os = "linux") {
        enable_features.push("Vulkan".to_owned());
    }

    let mut args = vec![
        "--remote-debugging-port=0".to_owned(),
        format!("--user-data-dir={user_data_dir}"),
        "--no-first-run".to_owned(),
        "--no-default-browser-check".to_owned(),
        "--disable-background-networking".to_owned(),
        "--disable-backgrounding-occluded-windows".to_owned(),
        "--disable-component-update".to_owned(),
        "--disable-default-apps".to_owned(),
        "--disable-hang-monitor".to_owned(),
        "--disable-popup-blocking".to_owned(),
        "--disable-prompt-on-repost".to_owned(),
        "--disable-sync".to_owned(),
        "--disable-features=Translate".to_owned(),
        format!("--enable-features={}", enable_features.join(",")),
        "--metrics-recording-only".to_owned(),
        "--hide-scrollbars".to_owned(),
    ];
    if options.headless {
        args.push("--headless=new".to_owned());
        args.push("--disable-gpu-sandbox".to_owned());
    }
    if let Some(ua) = &options.user_agent {
        args.push(format!("--user-agent={ua}"));
    }
    args
}

/// Discovers an installed Chromium/Chrome/Edge/Brave binary on the local OS.
#[cfg(not(target_family = "wasm"))]
#[must_use]
pub fn find_chrome_executable() -> Option<PathBuf> {
    if let Ok(override_bin) = std::env::var("APHRODY_BROWSER_EXECUTABLE") {
        let p = PathBuf::from(override_bin.trim());
        if p.exists() {
            return Some(p);
        }
    }

    #[cfg(target_os = "windows")]
    {
        let mut candidates = vec![
            PathBuf::from(r"C:\Program Files\Google\Chrome\Application\chrome.exe"),
            PathBuf::from(r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe"),
            PathBuf::from(r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"),
            PathBuf::from(r"C:\Program Files\Microsoft\Edge\Application\msedge.exe"),
            PathBuf::from(r"C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe"),
        ];
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let base = PathBuf::from(local);
            candidates.insert(0, base.join(r"Google\Chrome\Application\chrome.exe"));
            candidates.insert(1, base.join(r"Chromium\Application\chrome.exe"));
            candidates.insert(
                2,
                base.join(r"BraveSoftware\Brave-Browser\Application\brave.exe"),
            );
        }
        if let Some(found) = candidates.into_iter().find(|p| p.exists()) {
            return Some(found);
        }
    }

    #[cfg(target_os = "linux")]
    {
        for name in [
            "google-chrome",
            "google-chrome-stable",
            "chromium-browser",
            "chromium",
            "brave-browser",
            "microsoft-edge",
        ] {
            for dir in ["/usr/bin", "/usr/local/bin", "/snap/bin"] {
                let candidate = PathBuf::from(dir).join(name);
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        for path in [
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
            "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        ] {
            let candidate = PathBuf::from(path);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    None
}

/// Queries an active CDP endpoint (`http://host:port/json/version`) via pure-Rust `ureq`.
#[cfg(not(target_family = "wasm"))]
pub fn discover_cdp_version(host: &str, port: u16) -> anyhow::Result<Value> {
    let endpoint = format!("http://{host}:{port}/json/version");
    let mut response = ureq::get(&endpoint).call()?;
    let text = response.body_mut().read_to_string()?;
    Ok(serde_json::from_str(&text)?)
}

/// Sends one synchronous CDP command over WebSocket using workspace `tokio` + `tokio-tungstenite`.
#[cfg(not(target_family = "wasm"))]
pub fn send_cdp_command(
    ws_url: &str,
    method: &str,
    params: Value,
    session_id: Option<&str>,
) -> anyhow::Result<Value> {
    use futures_util::{SinkExt as _, StreamExt as _};
    use tokio_tungstenite::tungstenite::Message;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let method_owned = method.to_owned();
    let ws_owned = ws_url.to_owned();
    let sid_owned = session_id.map(str::to_owned);

    rt.block_on(async move {
        let (mut ws, _) = tokio_tungstenite::connect_async(&ws_owned).await?;
        let mut payload = json!({
            "id": 1u64,
            "method": method_owned,
            "params": params,
        });
        if let Some(sid) = sid_owned {
            payload["sessionId"] = Value::String(sid);
        }
        ws.send(Message::Text(payload.to_string().into())).await?;

        let timeout = std::time::Duration::from_secs(10);
        let reply = tokio::time::timeout(timeout, async {
            while let Some(msg) = ws.next().await {
                let msg = msg?;
                if let Message::Text(txt) = msg
                    && let Ok(parsed) = serde_json::from_str::<Value>(&txt)
                    && parsed.get("id").and_then(Value::as_u64) == Some(1)
                {
                    if let Some(err) = parsed.get("error") {
                        anyhow::bail!("CDP error: {err}");
                    }
                    return Ok(parsed.get("result").cloned().unwrap_or(Value::Null));
                }
            }
            anyhow::bail!("CDP WebSocket closed before response")
        })
        .await??;

        if let Err(err) = ws.close(None).await {
            log::debug!("CDP WebSocket close warning: {err}");
        }
        Ok(reply)
    })
}

pub const MAX_HAR_ENTRIES: usize = 1024;
pub const MAX_HAR_TOTAL_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_HAR_ENTRY_BODY_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_EVENT_RING_ENTRIES: usize = 512;

// ---------------------------------------------------------------------------
// Snapshot diffing (pure-Rust Myers/LCS line diff)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffLineKind {
    Equal,
    Added,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub line: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotDiff {
    pub diff: String,
    pub additions: usize,
    pub removals: usize,
    pub unchanged: usize,
    pub changed: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<DiffLine>,
}

/// Computes a pure-Rust line-level snapshot diff without external diff crate dependencies.
#[must_use]
pub fn diff_snapshot_text(before: &str, after: &str) -> SnapshotDiff {
    let before_lines: Vec<&str> = if before.is_empty() {
        Vec::new()
    } else {
        before.lines().collect()
    };
    let after_lines: Vec<&str> = if after.is_empty() {
        Vec::new()
    } else {
        after.lines().collect()
    };

    let mut prefix = 0_usize;
    while prefix < before_lines.len()
        && prefix < after_lines.len()
        && before_lines[prefix] == after_lines[prefix]
    {
        prefix += 1;
    }

    let mut suffix = 0_usize;
    while suffix + prefix < before_lines.len()
        && suffix + prefix < after_lines.len()
        && before_lines[before_lines.len() - 1 - suffix]
            == after_lines[after_lines.len() - 1 - suffix]
    {
        suffix += 1;
    }

    let mid_before = &before_lines[prefix..before_lines.len() - suffix];
    let mid_after = &after_lines[prefix..after_lines.len() - suffix];

    let mut lines = Vec::new();
    let mut additions = 0_usize;
    let mut removals = 0_usize;
    let mut unchanged = prefix + suffix;

    for &line in &before_lines[..prefix] {
        lines.push(DiffLine {
            kind: DiffLineKind::Equal,
            line: line.to_owned(),
        });
    }

    // Bound DP table size (~2M cells) to prevent memory spikes on huge DOM snapshots.
    if mid_before.len().saturating_mul(mid_after.len()) <= 2_000_000 {
        let rows = mid_before.len();
        let cols = mid_after.len();
        let stride = cols + 1;
        let mut lcs = vec![0_usize; (rows + 1) * stride];
        for i in (0..rows).rev() {
            for j in (0..cols).rev() {
                lcs[i * stride + j] = if mid_before[i] == mid_after[j] {
                    1 + lcs[(i + 1) * stride + (j + 1)]
                } else {
                    lcs[(i + 1) * stride + j].max(lcs[i * stride + (j + 1)])
                };
            }
        }
        let (mut i, mut j) = (0_usize, 0_usize);
        while i < rows && j < cols {
            if mid_before[i] == mid_after[j] {
                unchanged += 1;
                lines.push(DiffLine {
                    kind: DiffLineKind::Equal,
                    line: mid_before[i].to_owned(),
                });
                i += 1;
                j += 1;
            } else if lcs[(i + 1) * stride + j] >= lcs[i * stride + (j + 1)] {
                removals += 1;
                lines.push(DiffLine {
                    kind: DiffLineKind::Removed,
                    line: mid_before[i].to_owned(),
                });
                i += 1;
            } else {
                additions += 1;
                lines.push(DiffLine {
                    kind: DiffLineKind::Added,
                    line: mid_after[j].to_owned(),
                });
                j += 1;
            }
        }
        while i < rows {
            removals += 1;
            lines.push(DiffLine {
                kind: DiffLineKind::Removed,
                line: mid_before[i].to_owned(),
            });
            i += 1;
        }
        while j < cols {
            additions += 1;
            lines.push(DiffLine {
                kind: DiffLineKind::Added,
                line: mid_after[j].to_owned(),
            });
            j += 1;
        }
    } else {
        for &line in mid_before {
            removals += 1;
            lines.push(DiffLine {
                kind: DiffLineKind::Removed,
                line: line.to_owned(),
            });
        }
        for &line in mid_after {
            additions += 1;
            lines.push(DiffLine {
                kind: DiffLineKind::Added,
                line: line.to_owned(),
            });
        }
    }

    for &line in &before_lines[before_lines.len() - suffix..] {
        lines.push(DiffLine {
            kind: DiffLineKind::Equal,
            line: line.to_owned(),
        });
    }

    let mut rendered = Vec::with_capacity(lines.len());
    for entry in &lines {
        match entry.kind {
            DiffLineKind::Equal => rendered.push(format!("  {}", entry.line)),
            DiffLineKind::Added => rendered.push(format!("+ {}", entry.line)),
            DiffLineKind::Removed => rendered.push(format!("- {}", entry.line)),
        }
    }

    SnapshotDiff {
        diff: rendered.join("\n"),
        additions,
        removals,
        unchanged,
        changed: additions > 0 || removals > 0,
        lines,
    }
}

#[must_use]
pub fn snapshot_comparison_text(before: &str, after: &str) -> String {
    let diff = diff_snapshot_text(before, after);
    format!(
        "changed={} (+{} / -{}, {} unchanged)\n{}",
        diff.changed, diff.additions, diff.removals, diff.unchanged, diff.diff
    )
}

// ---------------------------------------------------------------------------
// Semantic locators & ARIA AX matching
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticLocatorKind {
    Role,
    Text,
    Label,
    Placeholder,
    TestId,
    XPath,
    Css,
    AltText,
    Title,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticLocator {
    pub kind: SemanticLocatorKind,
    pub query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub exact: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nth: Option<isize>,
}

impl SemanticLocator {
    #[must_use]
    pub fn new(kind: SemanticLocatorKind, query: impl Into<String>) -> Self {
        Self {
            kind,
            query: query.into(),
            name: None,
            exact: false,
            nth: None,
        }
    }

    #[must_use]
    pub fn role(role: impl Into<String>) -> Self {
        Self::new(SemanticLocatorKind::Role, role)
    }

    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self::new(SemanticLocatorKind::Text, text)
    }

    #[must_use]
    pub fn label(label: impl Into<String>) -> Self {
        Self::new(SemanticLocatorKind::Label, label)
    }

    #[must_use]
    pub fn placeholder(placeholder: impl Into<String>) -> Self {
        Self::new(SemanticLocatorKind::Placeholder, placeholder)
    }

    #[must_use]
    pub fn testid(testid: impl Into<String>) -> Self {
        Self::new(SemanticLocatorKind::TestId, testid)
    }

    #[must_use]
    pub fn xpath(xpath: impl Into<String>) -> Self {
        Self::new(SemanticLocatorKind::XPath, xpath)
    }

    #[must_use]
    pub fn css(selector: impl Into<String>) -> Self {
        Self::new(SemanticLocatorKind::Css, selector)
    }

    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    #[must_use]
    pub const fn with_exact(mut self, exact: bool) -> Self {
        self.exact = exact;
        self
    }

    #[must_use]
    pub const fn with_nth(mut self, nth: isize) -> Self {
        self.nth = Some(nth);
        self
    }

    #[must_use]
    pub fn build_semantic_query_js(&self) -> String {
        let query_json = serde_json::to_string(&self.query).unwrap_or_else(|_| "\"\"".to_owned());
        let name_json = self
            .name
            .as_deref()
            .and_then(|n| serde_json::to_string(n).ok())
            .unwrap_or_else(|| "null".to_owned());
        let kind_str = match self.kind {
            SemanticLocatorKind::Role => "role",
            SemanticLocatorKind::Text => "text",
            SemanticLocatorKind::Label => "label",
            SemanticLocatorKind::Placeholder => "placeholder",
            SemanticLocatorKind::TestId => "testid",
            SemanticLocatorKind::XPath => "xpath",
            SemanticLocatorKind::Css => "css",
            SemanticLocatorKind::AltText => "alt_text",
            SemanticLocatorKind::Title => "title",
        };
        let nth_expr = self
            .nth
            .map_or_else(|| "null".to_owned(), |val| val.to_string());
        let exact_flag = self.exact;
        format!(
            r#"(() => {{
  const kind = "{kind_str}";
  const q = {query_json};
  const optName = {name_json};
  const exact = {exact_flag};
  const nth = {nth_expr};
  const norm = (s) => String(s || "").replace(/\s+/g, " ").trim();
  const matchText = (hay, needle) => {{
    const a = norm(hay);
    const b = norm(needle);
    return exact ? a === b : a.toLowerCase().includes(b.toLowerCase());
  }};
  const collectAll = (root, out = []) => {{
    if (!root) return out;
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT);
    let node = walker.currentNode;
    while (node) {{
      if (node.nodeType === Node.ELEMENT_NODE) {{
        out.push(node);
        if (node.shadowRoot) collectAll(node.shadowRoot, out);
      }}
      node = walker.nextNode();
    }}
    return out;
  }};
  let matches = [];
  if (kind === "xpath") {{
    const xr = document.evaluate(q, document, null, XPathResult.ORDERED_NODE_SNAPSHOT_TYPE, null);
    for (let i = 0; i < xr.snapshotLength; i++) matches.push(xr.snapshotItem(i));
  }} else if (kind === "css") {{
    matches = Array.from(document.querySelectorAll(q));
  }} else {{
    const all = collectAll(document.body || document.documentElement);
    matches = all.filter((el) => {{
      if (kind === "role") {{
        const role = el.getAttribute("role") || el.tagName.toLowerCase();
        if (role !== q.toLowerCase()) return false;
        if (optName) {{
          const acc = el.getAttribute("aria-label") || el.innerText || el.value || "";
          return matchText(acc, optName);
        }}
        return true;
      }}
      if (kind === "text") return matchText(el.innerText || el.textContent, q);
      if (kind === "label") {{
        const aria = el.getAttribute("aria-label") || "";
        return matchText(aria || el.textContent, q);
      }}
      if (kind === "placeholder") return matchText(el.getAttribute("placeholder"), q);
      if (kind === "testid") return (el.getAttribute("data-testid") || "") === q;
      if (kind === "alt_text") return matchText(el.getAttribute("alt"), q);
      if (kind === "title") return matchText(el.getAttribute("title"), q);
      return false;
    }});
  }}
  if (nth !== null) {{
    const idx = nth < 0 ? matches.length + nth : nth;
    matches = idx >= 0 && idx < matches.length ? [matches[idx]] : [];
  }}
  return matches.slice(0, 64).map((el, i) => {{
    const r = el.getBoundingClientRect ? el.getBoundingClientRect() : {{ x: 0, y: 0, width: 0, height: 0 }};
    return {{
      index: i,
      tag: el.tagName ? el.tagName.toLowerCase() : "",
      role: el.getAttribute ? (el.getAttribute("role") || el.tagName.toLowerCase()) : "",
      name: norm(el.getAttribute && (el.getAttribute("aria-label") || el.innerText || el.value) || ""),
      testid: el.getAttribute ? el.getAttribute("data-testid") : null,
      bounds: {{ x: r.x, y: r.y, width: r.width, height: r.height }}
    }};
  }});
}})()"#
        )
    }
}

/// Normalizes ARIA / HTML semantic roles to canonical lowercase tokens.
#[must_use]
pub fn normalize_ax_role(raw: &str) -> &str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "input" | "textarea" | "textbox" | "textfield" | "searchbox" => "textbox",
        "select" | "combobox" | "dropdown" => "combobox",
        "button" | "pushbutton" => "button",
        "a" | "link" | "anchor" => "link",
        "checkbox" | "checkbutton" => "checkbox",
        "radio" | "radiobutton" => "radio",
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "heading" => "heading",
        "img" | "image" => "image",
        "tab" => "tab",
        "slider" => "slider",
        "switch" => "switch",
        other => {
            let trimmed = other;
            match trimmed {
                "" => "generic",
                _ => raw.trim(),
            }
        }
    }
}

/// Evaluates a `SemanticLocator` directly against a Chromium `Accessibility.getFullAXTree` JSON payload.
#[must_use]
pub fn find_ax_nodes_by_locator(ax_tree: &Value, locator: &SemanticLocator) -> Vec<Value> {
    let nodes = ax_tree
        .get("nodes")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let norm = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    let text_eq = |candidate: &str, needle: &str, exact: bool| -> bool {
        let a = norm(candidate);
        let b = norm(needle);
        if exact {
            a == b
        } else {
            a.to_ascii_lowercase().contains(&b.to_ascii_lowercase())
        }
    };

    let mut matched = Vec::new();
    for node in nodes {
        let role = node
            .get("role")
            .and_then(|r| r.get("value"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let name = node
            .get("name")
            .and_then(|n| n.get("value"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let description = node
            .get("description")
            .and_then(|d| d.get("value"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let value = node
            .get("value")
            .and_then(|v| v.get("value"))
            .and_then(Value::as_str)
            .unwrap_or("");

        let hit = match locator.kind {
            SemanticLocatorKind::Role => {
                let want_role = normalize_ax_role(&locator.query);
                let got_role = normalize_ax_role(role);
                let role_ok = got_role.eq_ignore_ascii_case(want_role);
                let name_ok = locator
                    .name
                    .as_deref()
                    .is_none_or(|n| text_eq(name, n, locator.exact));
                role_ok && name_ok
            }
            SemanticLocatorKind::Text | SemanticLocatorKind::Label => {
                text_eq(name, &locator.query, locator.exact)
                    || text_eq(description, &locator.query, locator.exact)
            }
            SemanticLocatorKind::Placeholder => {
                text_eq(description, &locator.query, locator.exact)
                    || text_eq(value, &locator.query, locator.exact)
            }
            SemanticLocatorKind::Title | SemanticLocatorKind::AltText => {
                text_eq(name, &locator.query, locator.exact)
                    || text_eq(description, &locator.query, locator.exact)
            }
            SemanticLocatorKind::TestId
            | SemanticLocatorKind::XPath
            | SemanticLocatorKind::Css => {
                let backend_id = node
                    .get("backendDOMNodeId")
                    .and_then(Value::as_u64)
                    .map(|id| id.to_string())
                    .unwrap_or_default();
                let node_id = node.get("nodeId").and_then(Value::as_str).unwrap_or("");
                backend_id == locator.query || node_id == locator.query || name == locator.query
            }
        };
        if hit {
            matched.push(node.clone());
        }
    }

    if let Some(nth) = locator.nth {
        let total = matched.len();
        let idx = if nth >= 0 {
            Some(nth as usize)
        } else {
            total.checked_sub(nth.unsigned_abs())
        };
        return idx
            .and_then(|i| matched.into_iter().nth(i))
            .into_iter()
            .collect();
    }
    matched
}

// ---------------------------------------------------------------------------
// Annotated screenshot metadata & bounding boxes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnnotatedElementBox {
    pub ref_id: String,
    pub number: usize,
    pub role: String,
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnnotatedScreenshot {
    pub format: String,
    pub width: u32,
    pub height: u32,
    pub full_page: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub base64_png: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<AnnotatedElementBox>,
}

/// Drops annotation badges whose label badges would heavily overlap an already accepted badge.
#[must_use]
pub fn filter_non_overlapping_annotations(
    boxes: Vec<AnnotatedElementBox>,
) -> Vec<AnnotatedElementBox> {
    let mut accepted: Vec<AnnotatedElementBox> = Vec::with_capacity(boxes.len());
    for candidate in boxes {
        if candidate.width <= 0.0 || candidate.height <= 0.0 {
            continue;
        }
        let overlaps = accepted.iter().any(|existing| {
            (candidate.x - existing.x).abs() < 18.0 && (candidate.y - existing.y).abs() < 14.0
        });
        if !overlaps {
            accepted.push(candidate);
        }
    }
    accepted
}

/// Projects CSS-pixel annotation boxes into rendered screenshot pixel coordinates.
#[must_use]
pub fn project_annotation_boxes(
    boxes: Vec<AnnotatedElementBox>,
    css_width: f64,
    css_height: f64,
    screenshot_width: f64,
    screenshot_height: f64,
) -> Vec<AnnotatedElementBox> {
    if css_width <= 0.0 || css_height <= 0.0 {
        return boxes;
    }
    let scale_x = screenshot_width / css_width;
    let scale_y = screenshot_height / css_height;
    boxes
        .into_iter()
        .map(|mut item| {
            item.x *= scale_x;
            item.y *= scale_y;
            item.width *= scale_x;
            item.height *= scale_y;
            item
        })
        .collect()
}

/// Computes bounded clip size for CDP `Page.captureScreenshot` full-page captures.
#[must_use]
pub fn full_page_clip_size(content_width: f64, content_height: f64) -> (f64, f64) {
    (
        content_width.clamp(1.0, 16_384.0),
        content_height.clamp(1.0, 16_384.0),
    )
}

#[must_use]
pub fn build_annotation_overlay_js(boxes: &[AnnotatedElementBox]) -> String {
    let payload = serde_json::to_string(boxes).unwrap_or_else(|_| "[]".to_owned());
    format!(
        r#"(() => {{
  const boxes = {payload};
  let root = document.getElementById("__aphrody_annot_overlay__");
  if (root) root.remove();
  root = document.createElement("div");
  root.id = "__aphrody_annot_overlay__";
  root.style.cssText = "position:fixed;inset:0;pointer-events:none;z-index:2147483647;";
  for (const b of boxes) {{
    const frame = document.createElement("div");
    frame.style.cssText = `position:fixed;left:${{b.x}}px;top:${{b.y}}px;width:${{b.width}}px;height:${{b.height}}px;border:2px solid #ef4444;box-sizing:border-box;`;
    const tag = document.createElement("span");
    tag.textContent = `[${{b.number}}]`;
    tag.style.cssText = "position:absolute;top:-16px;left:0;background:#ef4444;color:#fff;font:600 10px/14px monospace;padding:0 3px;border-radius:2px;";
    frame.appendChild(tag);
    root.appendChild(frame);
  }}
  (document.documentElement || document.body).appendChild(root);
  return boxes.length;
}})()"#
    )
}

#[must_use]
pub const fn cleanup_annotation_overlay_js() -> &'static str {
    "(() => { const el = document.getElementById('__aphrody_annot_overlay__'); if (el) el.remove(); return true; })()"
}

// ---------------------------------------------------------------------------
// Network route/intercept table & HAR 1.2 capture ringbuffer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteDisposition {
    Fulfill,
    Abort,
    Continue,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteResponse {
    pub status: u16,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkRoute {
    pub id: String,
    pub pattern: String,
    pub disposition: RouteDisposition,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<RouteResponse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method_filter: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resource_types: Vec<String>,
    #[serde(default)]
    pub hit_count: u64,
}

/// Glob matcher for URL patterns supporting `*` (single path segment) and `**` (any chars).
#[must_use]
pub fn route_url_matches(pattern: &str, url: &str) -> bool {
    let p_bytes = pattern.as_bytes();
    let u_bytes = url.as_bytes();
    let mut pi = 0_usize;
    let mut ui = 0_usize;
    let mut star_pi: Option<usize> = None;
    let mut star_ui = 0_usize;
    let mut star_double = false;

    while ui < u_bytes.len() {
        if pi + 1 < p_bytes.len() && p_bytes[pi] == b'*' && p_bytes[pi + 1] == b'*' {
            star_pi = Some(pi + 2);
            star_ui = ui;
            star_double = true;
            pi += 2;
        } else if pi < p_bytes.len() && p_bytes[pi] == b'*' {
            star_pi = Some(pi + 1);
            star_ui = ui;
            star_double = false;
            pi += 1;
        } else if pi < p_bytes.len() && (p_bytes[pi] == b'?' || p_bytes[pi] == u_bytes[ui]) {
            pi += 1;
            ui += 1;
        } else if let Some(prev_pi) = star_pi {
            if !star_double && u_bytes[star_ui] == b'/' {
                return false;
            }
            star_ui += 1;
            ui = star_ui;
            pi = prev_pi;
        } else {
            return false;
        }
    }

    while pi < p_bytes.len() && p_bytes[pi] == b'*' {
        pi += 1;
    }
    pi == p_bytes.len()
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkRouteTable {
    routes: Vec<NetworkRoute>,
    next_id: u64,
}

impl NetworkRouteTable {
    pub fn add_route(
        &mut self,
        pattern: impl Into<String>,
        disposition: RouteDisposition,
        response: Option<RouteResponse>,
        method_filter: Option<String>,
        resource_types: Vec<String>,
    ) -> NetworkRoute {
        self.next_id += 1;
        let route = NetworkRoute {
            id: format!("r{}", self.next_id),
            pattern: pattern.into(),
            disposition,
            response,
            method_filter,
            resource_types,
            hit_count: 0,
        };
        self.routes.push(route.clone());
        route
    }

    pub fn remove_matching(&mut self, pattern_or_id: Option<&str>) -> usize {
        let before = self.routes.len();
        match pattern_or_id {
            Some(target) => self
                .routes
                .retain(|route| route.pattern != target && route.id != target),
            None => self.routes.clear(),
        }
        before - self.routes.len()
    }

    pub fn select_route(
        &mut self,
        url: &str,
        method: Option<&str>,
        resource_type: Option<&str>,
    ) -> Option<NetworkRoute> {
        for route in self.routes.iter_mut().rev() {
            if !route_url_matches(&route.pattern, url) {
                continue;
            }
            if let Some(req_method) = method
                && let Some(expected_method) = &route.method_filter
                && !req_method.eq_ignore_ascii_case(expected_method)
            {
                continue;
            }
            if let Some(req_type) = resource_type
                && !route.resource_types.is_empty()
                && !route
                    .resource_types
                    .iter()
                    .any(|allowed| allowed.eq_ignore_ascii_case(req_type))
            {
                continue;
            }
            route.hit_count += 1;
            return Some(route.clone());
        }
        None
    }

    #[must_use]
    pub fn routes(&self) -> &[NetworkRoute] {
        &self.routes
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainFilter {
    #[serde(default)]
    pub allowed: Vec<String>,
    #[serde(default)]
    pub blocked: Vec<String>,
}

impl DomainFilter {
    #[must_use]
    pub fn is_allowed(&self, host: &str) -> bool {
        let host_lower = host.trim().to_ascii_lowercase();
        if self
            .blocked
            .iter()
            .any(|pattern| host_matches(&host_lower, &pattern.to_ascii_lowercase()))
        {
            return false;
        }
        if self.allowed.is_empty() {
            return true;
        }
        self.allowed
            .iter()
            .any(|pattern| host_matches(&host_lower, &pattern.to_ascii_lowercase()))
    }
}

fn host_matches(host: &str, pattern: &str) -> bool {
    if let Some(suffix) = pattern.strip_prefix("*.") {
        host == suffix || host.ends_with(&format!(".{suffix}"))
    } else {
        host == pattern
    }
}

/// Checks status filters such as `"4xx"`, `"5xx"`, `"200"`, `"400-499"`.
#[must_use]
pub fn matches_status_filter(filter: &str, status: u16) -> bool {
    let trimmed = filter.trim().to_ascii_lowercase();
    if let Some(prefix) = trimmed.strip_suffix("xx")
        && let Ok(hundreds) = prefix.parse::<u16>()
    {
        return status / 100 == hundreds;
    }
    if let Some((start_raw, end_raw)) = trimmed.split_once('-')
        && let (Ok(start), Ok(end)) = (start_raw.trim().parse::<u16>(), end_raw.trim().parse::<u16>())
    {
        return status >= start && status <= end;
    }
    trimmed.parse::<u16>().is_ok_and(|exact| status == exact)
}

#[must_use]
pub fn is_sensitive_header(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "authorization"
            | "proxy-authorization"
            | "cookie"
            | "set-cookie"
            | "x-api-key"
            | "x-auth-token"
    )
}

#[must_use]
pub fn redact_header_map(headers: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    headers
        .iter()
        .map(|(key, val)| {
            let clean = if is_sensitive_header(key) {
                "[REDACTED]".to_owned()
            } else {
                val.clone()
            };
            (key.clone(), clean)
        })
        .collect()
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarContentMode {
    #[default]
    Full,
    None,
    Attach,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarEntry {
    pub request_id: String,
    pub started_ms: u64,
    pub time_ms: f64,
    pub method: String,
    pub url: String,
    pub status: u16,
    pub status_text: String,
    #[serde(default)]
    pub request_headers: BTreeMap<String, String>,
    #[serde(default)]
    pub response_headers: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_body: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_body: Option<String>,
    pub mime_type: String,
    pub encoded_bytes: usize,
    #[serde(default)]
    pub from_route: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarRingBuffer {
    pub recording: bool,
    pub content_mode: HarContentMode,
    pub max_entries: usize,
    pub max_total_bytes: usize,
    pub total_bytes: usize,
    pub dropped_count: usize,
    entries: VecDeque<HarEntry>,
}

impl Default for HarRingBuffer {
    fn default() -> Self {
        Self {
            recording: false,
            content_mode: HarContentMode::Full,
            max_entries: MAX_HAR_ENTRIES,
            max_total_bytes: MAX_HAR_TOTAL_BYTES,
            total_bytes: 0,
            dropped_count: 0,
            entries: VecDeque::new(),
        }
    }
}

impl HarRingBuffer {
    pub fn start(&mut self, content_mode: HarContentMode, clear: bool) {
        if clear {
            self.clear();
        }
        self.content_mode = content_mode;
        self.recording = true;
    }

    pub fn stop(&mut self) {
        self.recording = false;
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.total_bytes = 0;
        self.dropped_count = 0;
    }

    pub fn push(&mut self, mut entry: HarEntry) {
        if !self.recording {
            return;
        }
        entry.request_headers = redact_header_map(&entry.request_headers);
        entry.response_headers = redact_header_map(&entry.response_headers);
        if self.content_mode == HarContentMode::None {
            entry.request_body = None;
            entry.response_body = None;
        } else {
            if let Some(body) = &mut entry.request_body
                && body.len() > MAX_HAR_ENTRY_BODY_BYTES
            {
                body.truncate(MAX_HAR_ENTRY_BODY_BYTES);
            }
            if let Some(body) = &mut entry.response_body
                && body.len() > MAX_HAR_ENTRY_BODY_BYTES
            {
                body.truncate(MAX_HAR_ENTRY_BODY_BYTES);
            }
        }
        let approx_bytes = entry.url.len()
            + entry.request_body.as_ref().map_or(0, String::len)
            + entry.response_body.as_ref().map_or(0, String::len);
        while self.entries.len() >= self.max_entries.max(1)
            || (self.total_bytes + approx_bytes > self.max_total_bytes && !self.entries.is_empty())
        {
            if let Some(oldest) = self.entries.pop_front() {
                let oldest_bytes = oldest.url.len()
                    + oldest.request_body.as_ref().map_or(0, String::len)
                    + oldest.response_body.as_ref().map_or(0, String::len);
                self.total_bytes = self.total_bytes.saturating_sub(oldest_bytes);
                self.dropped_count += 1;
            }
        }
        self.total_bytes = self.total_bytes.saturating_add(approx_bytes);
        self.entries.push_back(entry);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub fn entries(&self) -> &VecDeque<HarEntry> {
        &self.entries
    }

    #[must_use]
    pub fn export_har_1_2(&self, page_url: &str, page_title: &str) -> Value {
        let entries_json: Vec<Value> = self
            .entries
            .iter()
            .map(|entry| {
                let req_headers: Vec<Value> = entry
                    .request_headers
                    .iter()
                    .map(|(name, value)| json!({ "name": name, "value": value }))
                    .collect();
                let res_headers: Vec<Value> = entry
                    .response_headers
                    .iter()
                    .map(|(name, value)| json!({ "name": name, "value": value }))
                    .collect();
                let content_text = entry.response_body.as_deref().unwrap_or("");
                json!({
                    "startedDateTime": format!("{}", entry.started_ms),
                    "time": entry.time_ms,
                    "request": {
                        "method": entry.method,
                        "url": entry.url,
                        "httpVersion": "HTTP/1.1",
                        "headers": req_headers,
                        "queryString": [],
                        "cookies": [],
                        "headersSize": -1,
                        "bodySize": entry.request_body.as_ref().map_or(-1_isize, |b| b.len() as isize),
                    },
                    "response": {
                        "status": entry.status,
                        "statusText": entry.status_text,
                        "httpVersion": "HTTP/1.1",
                        "headers": res_headers,
                        "cookies": [],
                        "content": {
                            "size": entry.encoded_bytes,
                            "mimeType": entry.mime_type,
                            "text": content_text,
                        },
                        "redirectURL": "",
                        "headersSize": -1,
                        "bodySize": entry.encoded_bytes,
                    },
                    "cache": {},
                    "timings": { "send": 0, "wait": entry.time_ms, "receive": 0 },
                })
            })
            .collect();

        json!({
            "log": {
                "version": "1.2",
                "creator": { "name": "aphrody-browser", "version": "0.43.0" },
                "pages": [{
                    "id": "page_1",
                    "title": page_title,
                    "pageTimings": { "onLoad": -1 },
                    "url": page_url,
                }],
                "entries": entries_json,
            }
        })
    }
}

// ---------------------------------------------------------------------------
// Console & page-error ringbuffer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsoleLogEntry {
    pub level: String,
    pub text: String,
    pub timestamp_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageErrorEntry {
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
    pub timestamp_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[must_use]
pub fn format_console_arg(arg: &Value) -> String {
    if let Some(s) = arg.get("value").and_then(Value::as_str) {
        return s.to_owned();
    }
    if let Some(val) = arg.get("value")
        && !val.is_null()
    {
        return val.to_string();
    }
    if let Some(desc) = arg.get("description").and_then(Value::as_str) {
        return desc.to_owned();
    }
    if let Some(ty) = arg.get("type").and_then(Value::as_str) {
        return ty.to_owned();
    }
    arg.to_string()
}

#[must_use]
pub fn format_console_args(args: &[Value]) -> String {
    args.iter()
        .map(format_console_arg)
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserEventRingBuffer {
    pub max_entries: usize,
    console: VecDeque<ConsoleLogEntry>,
    errors: VecDeque<PageErrorEntry>,
}

impl Default for BrowserEventRingBuffer {
    fn default() -> Self {
        Self {
            max_entries: MAX_EVENT_RING_ENTRIES,
            console: VecDeque::new(),
            errors: VecDeque::new(),
        }
    }
}

impl BrowserEventRingBuffer {
    pub fn push_console(&mut self, entry: ConsoleLogEntry) {
        while self.console.len() >= self.max_entries.max(1) {
            self.console.pop_front();
        }
        self.console.push_back(entry);
    }

    pub fn push_error(&mut self, entry: PageErrorEntry) {
        while self.errors.len() >= self.max_entries.max(1) {
            self.errors.pop_front();
        }
        self.errors.push_back(entry);
    }

    #[must_use]
    pub fn console_entries(&self, level_filter: Option<&str>) -> Vec<ConsoleLogEntry> {
        self.console
            .iter()
            .filter(|entry| {
                level_filter
                    .is_none_or(|wanted| entry.level.eq_ignore_ascii_case(wanted.trim()))
            })
            .cloned()
            .collect()
    }

    #[must_use]
    pub fn error_entries(&self) -> Vec<PageErrorEntry> {
        self.errors.iter().cloned().collect()
    }

    pub fn clear(&mut self) {
        self.console.clear();
        self.errors.clear();
    }
}

// ---------------------------------------------------------------------------
// Storage / cookie state serialization
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrowserCookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    #[serde(default = "default_cookie_path")]
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires: Option<f64>,
    #[serde(default)]
    pub http_only: bool,
    #[serde(default)]
    pub secure: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub same_site: Option<String>,
}

fn default_cookie_path() -> String {
    "/".to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageEntry {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OriginStorage {
    pub origin: String,
    #[serde(default)]
    pub local_storage: Vec<StorageEntry>,
    #[serde(default)]
    pub session_storage: Vec<StorageEntry>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BrowserStorageState {
    #[serde(default)]
    pub cookies: Vec<BrowserCookie>,
    #[serde(default)]
    pub origins: Vec<OriginStorage>,
}

impl BrowserStorageState {
    pub fn set_cookie(&mut self, cookie: BrowserCookie) {
        self.cookies.retain(|existing| {
            !(existing.name == cookie.name
                && existing.domain == cookie.domain
                && existing.path == cookie.path)
        });
        self.cookies.push(cookie);
    }

    pub fn clear_cookies(&mut self, name_filter: Option<&str>) -> usize {
        let before = self.cookies.len();
        match name_filter {
            Some(name) => self.cookies.retain(|c| c.name != name),
            None => self.cookies.clear(),
        }
        before - self.cookies.len()
    }

    #[must_use]
    pub fn cookies_for_host(&self, host: Option<&str>) -> Vec<&BrowserCookie> {
        let Some(host) = host else {
            return self.cookies.iter().collect();
        };
        let host_lower = host.trim().to_ascii_lowercase();
        self.cookies
            .iter()
            .filter(|cookie| {
                let dom = cookie.domain.trim_start_matches('.').to_ascii_lowercase();
                host_lower == dom || host_lower.ends_with(&format!(".{dom}"))
            })
            .collect()
    }

    pub fn set_local_storage(&mut self, origin: &str, key: &str, value: &str) {
        if let Some(slot) = self.origins.iter_mut().find(|item| item.origin == origin) {
            if let Some(entry) = slot.local_storage.iter_mut().find(|e| e.name == key) {
                entry.value = value.to_owned();
            } else {
                slot.local_storage.push(StorageEntry {
                    name: key.to_owned(),
                    value: value.to_owned(),
                });
            }
        } else {
            self.origins.push(OriginStorage {
                origin: origin.to_owned(),
                local_storage: vec![StorageEntry {
                    name: key.to_owned(),
                    value: value.to_owned(),
                }],
                session_storage: Vec::new(),
            });
        }
    }

    #[must_use]
    pub fn get_local_storage(&self, origin: &str, key: Option<&str>) -> Vec<StorageEntry> {
        self.origins
            .iter()
            .find(|item| item.origin == origin)
            .map(|slot| {
                slot.local_storage
                    .iter()
                    .filter(|entry| key.is_none_or(|k| entry.name == k))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn to_json_pretty(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json_str(raw: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(raw)?)
    }

    #[cfg(not(target_family = "wasm"))]
    pub fn save_to_path(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        let body = self.to_json_pretty()?;
        let tmp_name = format!(
            ".{}.tmp.{}",
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("state"),
            std::process::id()
        );
        let tmp_path = path.with_file_name(tmp_name);
        std::fs::write(&tmp_path, body)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o600))?;
        }
        std::fs::rename(&tmp_path, path)?;
        Ok(())
    }

    #[cfg(not(target_family = "wasm"))]
    pub fn load_from_path(path: &Path) -> anyhow::Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        Self::from_json_str(&raw)
    }
}

// ---------------------------------------------------------------------------
// Multi-tab session manager (`t<N>` refs & label pinning)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TabRef(pub String);

impl TabRef {
    pub fn parse(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        let digits = trimmed.strip_prefix('t')?;
        if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
            Some(Self(trimmed.to_owned()))
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[must_use]
pub fn is_valid_tab_label(label: &str) -> bool {
    let trimmed = label.trim();
    !trimmed.is_empty()
        && trimmed.len() <= 64
        && trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

#[must_use]
pub fn sanitize_tab_url(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "about:blank".to_owned();
    }
    match trimmed.split_once('#') {
        Some((before, _)) => before.to_owned(),
        None => trimmed.to_owned(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserTabSession {
    pub tab_ref: TabRef,
    pub target_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub url: String,
    pub title: String,
    pub pinned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabSessionManager {
    tabs: Vec<BrowserTabSession>,
    active_index: usize,
    next_seq: u64,
}

impl Default for TabSessionManager {
    fn default() -> Self {
        let initial = BrowserTabSession {
            tab_ref: TabRef("t1".to_owned()),
            target_id: "target-1".to_owned(),
            session_id: None,
            label: None,
            url: "about:blank".to_owned(),
            title: "New Tab".to_owned(),
            pinned: false,
        };
        Self {
            tabs: vec![initial],
            active_index: 0,
            next_seq: 2,
        }
    }
}

impl TabSessionManager {
    pub fn open_tab(
        &mut self,
        url: &str,
        label: Option<&str>,
        target_id: Option<&str>,
        session_id: Option<&str>,
    ) -> anyhow::Result<&BrowserTabSession> {
        if let Some(lbl) = label
            && !is_valid_tab_label(lbl)
        {
            anyhow::bail!("invalid tab label '{lbl}'");
        }
        let seq = self.next_seq;
        self.next_seq += 1;
        let tab = BrowserTabSession {
            tab_ref: TabRef(format!("t{seq}")),
            target_id: target_id
                .map(str::to_owned)
                .unwrap_or_else(|| format!("target-{seq}")),
            session_id: session_id.map(str::to_owned),
            label: label.map(|s| s.trim().to_owned()),
            url: sanitize_tab_url(url),
            title: String::new(),
            pinned: false,
        };
        self.tabs.push(tab);
        self.active_index = self.tabs.len() - 1;
        self.active_tab()
            .ok_or_else(|| anyhow::anyhow!("opened tab vanished"))
    }

    pub fn switch_tab(&mut self, selector: &str) -> anyhow::Result<&BrowserTabSession> {
        let trimmed = selector.trim();
        let idx = self
            .tabs
            .iter()
            .position(|tab| {
                tab.tab_ref.as_str() == trimmed
                    || tab.target_id == trimmed
                    || tab.label.as_deref() == Some(trimmed)
            })
            .or_else(|| trimmed.parse::<usize>().ok().filter(|&i| i < self.tabs.len()))
            .ok_or_else(|| anyhow::anyhow!("tab '{trimmed}' not found"))?;
        self.active_index = idx;
        Ok(&self.tabs[idx])
    }

    pub fn close_tab(&mut self, selector: Option<&str>) -> anyhow::Result<BrowserTabSession> {
        if self.tabs.len() <= 1 {
            anyhow::bail!("cannot close the last remaining browser tab");
        }
        let idx = match selector {
            Some(sel) => self
                .tabs
                .iter()
                .position(|tab| {
                    tab.tab_ref.as_str() == sel.trim()
                        || tab.target_id == sel.trim()
                        || tab.label.as_deref() == Some(sel.trim())
                })
                .ok_or_else(|| anyhow::anyhow!("tab '{sel}' not found"))?,
            None => self.active_index.min(self.tabs.len() - 1),
        };
        let removed = self.tabs.remove(idx);
        if self.active_index >= self.tabs.len() {
            self.active_index = self.tabs.len() - 1;
        }
        Ok(removed)
    }

    pub fn pin_active_url(&mut self, url: &str, title: &str) {
        if let Some(tab) = self.tabs.get_mut(self.active_index) {
            tab.url = sanitize_tab_url(url);
            tab.title = title.to_owned();
            tab.pinned = true;
        }
    }

    #[must_use]
    pub fn active_tab(&self) -> Option<&BrowserTabSession> {
        self.tabs.get(self.active_index)
    }

    #[must_use]
    pub fn tabs(&self) -> &[BrowserTabSession] {
        &self.tabs
    }

    #[must_use]
    pub fn active_index(&self) -> usize {
        self.active_index
    }
}

// ---------------------------------------------------------------------------
// Browser action policy gating (fail-closed)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum BrowserPolicyDecision {
    Allow,
    Deny { reason: String },
    Confirm { category: String, token: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserActionPolicy {
    #[serde(default = "default_policy_decision")]
    pub default_decision: String,
    #[serde(default)]
    pub allow: Vec<String>,
    #[serde(default)]
    pub confirm: Vec<String>,
    #[serde(default)]
    pub deny: Vec<String>,
    #[serde(default)]
    pub confirmed_tokens: BTreeSet<String>,
    #[serde(default)]
    next_token_id: u64,
}

fn default_policy_decision() -> String {
    "allow".to_owned()
}

impl Default for BrowserActionPolicy {
    fn default() -> Self {
        Self {
            default_decision: default_policy_decision(),
            allow: Vec::new(),
            confirm: Vec::new(),
            deny: Vec::new(),
            confirmed_tokens: BTreeSet::new(),
            next_token_id: 0,
        }
    }
}

#[must_use]
pub fn action_policy_category(op: &str) -> &'static str {
    let lower = op.trim().to_ascii_lowercase();
    let clean = lower.strip_prefix("browser_").unwrap_or(&lower);
    match clean {
        "open" | "navigate" | "tab_new" => "navigate",
        "eval" | "script" => "eval",
        "route" | "unroute" | "har_start" | "har_stop" => "network",
        "state_save" | "state_load" | "cookies" | "storage" => "state_write",
        "click" | "fill" | "type" | "press" => "interact",
        "snapshot" | "snapshot_diff" | "find" | "screenshot" | "screenshot_annotated"
        | "console" | "errors" | "status" | "validate_url" | "tab_list" | "policy_check" => {
            "get"
        }
        _ => "interact",
    }
}

fn policy_rule_matches(rule: &str, category: &str, op: &str) -> bool {
    let trimmed = rule.trim().to_ascii_lowercase();
    trimmed == "*"
        || trimmed == category
        || trimmed == op.to_ascii_lowercase()
        || trimmed == op.trim_start_matches("browser_").to_ascii_lowercase()
}

impl BrowserActionPolicy {
    /// Parses policy JSON fail-closed (malformed definitions never degrade to permissive defaults).
    pub fn from_json_str(raw: &str) -> anyhow::Result<Self> {
        let parsed: Self = serde_json::from_str(raw)?;
        match parsed.default_decision.as_str() {
            "allow" | "deny" => Ok(parsed),
            other => anyhow::bail!("unsupported browser policy default '{other}'"),
        }
    }

    pub fn evaluate(
        &mut self,
        op: &str,
        confirmation_token: Option<&str>,
    ) -> BrowserPolicyDecision {
        let category = action_policy_category(op);
        if self.deny.iter().any(|r| policy_rule_matches(r, category, op)) {
            return BrowserPolicyDecision::Deny {
                reason: format!("action '{op}' (category '{category}') is denied by policy"),
            };
        }
        if let Some(token) = confirmation_token
            && self.confirmed_tokens.remove(token)
        {
            return BrowserPolicyDecision::Allow;
        }
        if self
            .confirm
            .iter()
            .any(|r| policy_rule_matches(r, category, op))
        {
            self.next_token_id += 1;
            let token = format!("confirm-{category}-{}", self.next_token_id);
            self.confirmed_tokens.insert(token.clone());
            return BrowserPolicyDecision::Confirm {
                category: category.to_owned(),
                token,
            };
        }
        if self
            .allow
            .iter()
            .any(|r| policy_rule_matches(r, category, op))
        {
            return BrowserPolicyDecision::Allow;
        }
        if self.default_decision == "deny" {
            BrowserPolicyDecision::Deny {
                reason: format!("action '{op}' denied by default-deny browser policy"),
            }
        } else {
            BrowserPolicyDecision::Allow
        }
    }
}

// ---------------------------------------------------------------------------
// Browser batch execution
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrowserBatchStep {
    pub op: String,
    #[serde(default)]
    pub args: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirm_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrowserBatchStepResult {
    pub index: usize,
    pub op: String,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrowserBatchReport {
    pub total: usize,
    pub completed: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub not_started: usize,
    pub stopped_early: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stopped_reason: Option<String>,
    pub steps: Vec<BrowserBatchStepResult>,
}

/// Stateful absorbed `agent-browser` 0.43.0 controller exposed over `aphrody_pilot` & MCP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserControllerState {
    pub options: BrowserLaunchOptions,
    pub cdp_ws_url: Option<String>,
    pub active_session_id: Option<String>,
    pub current_url: String,
    pub last_title: String,
    #[serde(default)]
    pub tabs: TabSessionManager,
    #[serde(default)]
    pub routes: NetworkRouteTable,
    #[serde(default)]
    pub har: HarRingBuffer,
    #[serde(default)]
    pub events: BrowserEventRingBuffer,
    #[serde(default)]
    pub storage: BrowserStorageState,
    #[serde(default)]
    pub policy: BrowserActionPolicy,
    #[serde(default)]
    pub last_snapshot_text: String,
}

impl Default for BrowserControllerState {
    fn default() -> Self {
        Self {
            options: BrowserLaunchOptions::default(),
            cdp_ws_url: None,
            active_session_id: None,
            current_url: "about:blank".to_owned(),
            last_title: String::new(),
            tabs: TabSessionManager::default(),
            routes: NetworkRouteTable::default(),
            har: HarRingBuffer::default(),
            events: BrowserEventRingBuffer::default(),
            storage: BrowserStorageState::default(),
            policy: BrowserActionPolicy::default(),
            last_snapshot_text: String::new(),
        }
    }
}

impl BrowserControllerState {
    pub fn execute_tool(&mut self, op: &str, args: &Value) -> anyhow::Result<Value> {
        let confirm_token = args.get("confirm_token").and_then(Value::as_str);
        if op != "browser_policy_check" && op != "browser_batch" && op != "browser_status" {
            match self.policy.evaluate(op, confirm_token) {
                BrowserPolicyDecision::Allow => {}
                BrowserPolicyDecision::Deny { reason } => {
                    anyhow::bail!("browser policy denied '{op}': {reason}");
                }
                BrowserPolicyDecision::Confirm { category, token } => {
                    anyhow::bail!(
                        "browser policy requires confirmation for '{op}' (category '{category}', confirm_token '{token}')"
                    );
                }
            }
        }

        match op {
            "browser_status" => Ok(json!({
                "state": self,
                "executable": self.discovered_executable(),
            })),
            "browser_validate_url" => {
                let url = args
                    .get("url")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow::anyhow!("missing `url` argument"))?;
                Ok(serde_json::to_value(validate_navigation_url(url, &self.options))?)
            }
            "browser_connect_cdp" => {
                let host = args.get("host").and_then(Value::as_str).unwrap_or("127.0.0.1");
                let port = args.get("port").and_then(Value::as_u64).unwrap_or(9222) as u16;
                #[cfg(not(target_family = "wasm"))]
                {
                    let info = discover_cdp_version(host, port)?;
                    if let Some(ws) = info.get("webSocketDebuggerUrl").and_then(Value::as_str) {
                        self.cdp_ws_url = Some(ws.to_owned());
                    }
                    return Ok(json!({
                        "connected": self.cdp_ws_url.is_some(),
                        "cdp_ws_url": self.cdp_ws_url,
                        "browser": info,
                    }));
                }
                #[cfg(target_family = "wasm")]
                {
                    let _ = (host, port);
                    anyhow::bail!("browser_connect_cdp requires non-wasm target")
                }
            }
            "browser_open" => {
                let url = args
                    .get("url")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow::anyhow!("missing `url` argument"))?;
                let check = validate_navigation_url(url, &self.options);
                if !check.allowed {
                    anyhow::bail!(
                        "navigation rejected: {}",
                        check.reason.unwrap_or_else(|| "blocked".to_owned())
                    );
                }
                self.current_url = url.to_owned();
                self.tabs.pin_active_url(url, &self.last_title);
                #[cfg(not(target_family = "wasm"))]
                if let Some(ws) = &self.cdp_ws_url {
                    let res = send_cdp_command(
                        ws,
                        "Page.navigate",
                        json!({ "url": url }),
                        self.active_session_id.as_deref(),
                    )?;
                    return Ok(json!({ "url": self.current_url, "cdp": res }));
                }
                Ok(json!({
                    "url": self.current_url,
                    "cdp_attached": false,
                    "validated": true,
                }))
            }
            "browser_eval" => {
                let expression = args
                    .get("expression")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow::anyhow!("missing `expression`"))?;
                #[cfg(not(target_family = "wasm"))]
                {
                    let ws = self
                        .cdp_ws_url
                        .as_deref()
                        .ok_or_else(|| anyhow::anyhow!("no active CDP WebSocket connected"))?;
                    return send_cdp_command(
                        ws,
                        "Runtime.evaluate",
                        json!({ "expression": expression, "returnByValue": true }),
                        self.active_session_id.as_deref(),
                    );
                }
                #[cfg(target_family = "wasm")]
                {
                    let _ = expression;
                    anyhow::bail!("browser_eval requires non-wasm target")
                }
            }
            "browser_snapshot" => {
                if let Some(text) = args.get("text").and_then(Value::as_str) {
                    let previous = std::mem::replace(&mut self.last_snapshot_text, text.to_owned());
                    let diff = diff_snapshot_text(&previous, text);
                    return Ok(json!({
                        "snapshot": text,
                        "diff": diff,
                    }));
                }
                #[cfg(not(target_family = "wasm"))]
                {
                    let ws = self
                        .cdp_ws_url
                        .as_deref()
                        .ok_or_else(|| anyhow::anyhow!("no active CDP WebSocket connected"))?;
                    let raw = send_cdp_command(
                        ws,
                        "Accessibility.getFullAXTree",
                        json!({}),
                        self.active_session_id.as_deref(),
                    )?;
                    let rendered = serde_json::to_string_pretty(&raw)?;
                    self.last_snapshot_text = rendered;
                    return Ok(raw);
                }
                #[cfg(target_family = "wasm")]
                anyhow::bail!("browser_snapshot requires non-wasm target")
            }
            "browser_snapshot_diff" => {
                let before = args
                    .get("before")
                    .and_then(Value::as_str)
                    .unwrap_or(&self.last_snapshot_text);
                let after = args
                    .get("after")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow::anyhow!("missing `after` snapshot text"))?;
                let diff = diff_snapshot_text(before, after);
                if args
                    .get("update_baseline")
                    .and_then(Value::as_bool)
                    .unwrap_or(true)
                {
                    self.last_snapshot_text = after.to_owned();
                }
                Ok(serde_json::to_value(diff)?)
            }
            "browser_find" => {
                let kind = match args
                    .get("by")
                    .or_else(|| args.get("kind"))
                    .and_then(Value::as_str)
                    .unwrap_or("role")
                    .to_ascii_lowercase()
                    .as_str()
                {
                    "text" => SemanticLocatorKind::Text,
                    "label" => SemanticLocatorKind::Label,
                    "placeholder" => SemanticLocatorKind::Placeholder,
                    "testid" => SemanticLocatorKind::TestId,
                    "xpath" => SemanticLocatorKind::XPath,
                    "css" => SemanticLocatorKind::Css,
                    "alt" | "alt_text" => SemanticLocatorKind::AltText,
                    "title" => SemanticLocatorKind::Title,
                    _ => SemanticLocatorKind::Role,
                };
                let query = args
                    .get("query")
                    .or_else(|| args.get("value"))
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow::anyhow!("missing `query` for browser_find"))?;
                let locator = SemanticLocator {
                    kind,
                    query: query.to_owned(),
                    name: args.get("name").and_then(Value::as_str).map(str::to_owned),
                    exact: args.get("exact").and_then(Value::as_bool).unwrap_or(false),
                    nth: args.get("nth").and_then(Value::as_i64).map(|n| n as isize),
                };
                if let Some(ax_tree) = args.get("ax_tree") {
                    let matches = find_ax_nodes_by_locator(ax_tree, &locator);
                    return Ok(json!({
                        "locator": locator,
                        "count": matches.len(),
                        "matches": matches,
                    }));
                }
                #[cfg(not(target_family = "wasm"))]
                if let Some(ws) = &self.cdp_ws_url {
                    let js = locator.build_semantic_query_js();
                    return send_cdp_command(
                        ws,
                        "Runtime.evaluate",
                        json!({ "expression": js, "returnByValue": true }),
                        self.active_session_id.as_deref(),
                    );
                }
                Ok(json!({
                    "locator": locator,
                    "script": locator.build_semantic_query_js(),
                }))
            }
            "browser_screenshot" => {
                #[cfg(not(target_family = "wasm"))]
                {
                    let ws = self
                        .cdp_ws_url
                        .as_deref()
                        .ok_or_else(|| anyhow::anyhow!("no active CDP WebSocket connected"))?;
                    return send_cdp_command(
                        ws,
                        "Page.captureScreenshot",
                        json!({ "format": "png" }),
                        self.active_session_id.as_deref(),
                    );
                }
                #[cfg(target_family = "wasm")]
                anyhow::bail!("browser_screenshot requires non-wasm target")
            }
            "browser_screenshot_annotated" => {
                let raw_boxes: Vec<AnnotatedElementBox> = args
                    .get("annotations")
                    .cloned()
                    .map(serde_json::from_value)
                    .transpose()?
                    .unwrap_or_default();
                let filtered = filter_non_overlapping_annotations(raw_boxes);
                let overlay_js = build_annotation_overlay_js(&filtered);
                Ok(json!({
                    "annotations": filtered,
                    "overlay_js": overlay_js,
                    "cleanup_js": cleanup_annotation_overlay_js(),
                }))
            }
            "browser_route" => {
                let pattern = args
                    .get("pattern")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow::anyhow!("missing `pattern` for browser_route"))?;
                let disposition = match args
                    .get("disposition")
                    .and_then(Value::as_str)
                    .unwrap_or("fulfill")
                    .to_ascii_lowercase()
                    .as_str()
                {
                    "abort" => RouteDisposition::Abort,
                    "continue" => RouteDisposition::Continue,
                    _ => RouteDisposition::Fulfill,
                };
                let response = args
                    .get("response")
                    .cloned()
                    .map(serde_json::from_value)
                    .transpose()?;
                let method = args.get("method").and_then(Value::as_str).map(str::to_owned);
                let resource_types = args
                    .get("resource_types")
                    .cloned()
                    .map(serde_json::from_value)
                    .transpose()?
                    .unwrap_or_default();
                let route = self.routes.add_route(
                    pattern,
                    disposition,
                    response,
                    method,
                    resource_types,
                );
                Ok(json!({ "route": route, "count": self.routes.routes().len() }))
            }
            "browser_unroute" => {
                let target = args
                    .get("pattern")
                    .or_else(|| args.get("id"))
                    .and_then(Value::as_str);
                let removed = self.routes.remove_matching(target);
                Ok(json!({ "removed": removed, "remaining": self.routes.routes().len() }))
            }
            "browser_har_start" => {
                let mode = match args
                    .get("content")
                    .and_then(Value::as_str)
                    .unwrap_or("full")
                    .to_ascii_lowercase()
                    .as_str()
                {
                    "none" => HarContentMode::None,
                    "attach" => HarContentMode::Attach,
                    _ => HarContentMode::Full,
                };
                let clear = args.get("clear").and_then(Value::as_bool).unwrap_or(false);
                self.har.start(mode, clear);
                Ok(json!({
                    "recording": self.har.recording,
                    "content_mode": self.har.content_mode,
                    "entries": self.har.len(),
                }))
            }
            "browser_har_stop" => {
                self.har.stop();
                let har = self.har.export_har_1_2(&self.current_url, &self.last_title);
                Ok(json!({
                    "recording": self.har.recording,
                    "entries": self.har.len(),
                    "dropped_count": self.har.dropped_count,
                    "har": har,
                }))
            }
            "browser_console" => {
                if args.get("clear").and_then(Value::as_bool).unwrap_or(false) {
                    self.events.clear();
                }
                let level = args.get("level").and_then(Value::as_str);
                let entries = self.events.console_entries(level);
                Ok(json!({ "count": entries.len(), "entries": entries }))
            }
            "browser_errors" => {
                if args.get("clear").and_then(Value::as_bool).unwrap_or(false) {
                    self.events.clear();
                }
                let errors = self.events.error_entries();
                Ok(json!({ "count": errors.len(), "errors": errors }))
            }
            "browser_state_save" => {
                let json_str = self.storage.to_json_pretty()?;
                #[cfg(not(target_family = "wasm"))]
                if let Some(path_raw) = args.get("path").and_then(Value::as_str) {
                    self.storage.save_to_path(Path::new(path_raw))?;
                    return Ok(json!({ "saved": true, "path": path_raw }));
                }
                Ok(json!({ "saved": true, "state_json": json_str }))
            }
            "browser_state_load" => {
                if let Some(raw) = args.get("state_json").and_then(Value::as_str) {
                    self.storage = BrowserStorageState::from_json_str(raw)?;
                    return Ok(json!({
                        "loaded": true,
                        "cookies": self.storage.cookies.len(),
                        "origins": self.storage.origins.len(),
                    }));
                }
                #[cfg(not(target_family = "wasm"))]
                if let Some(path_raw) = args.get("path").and_then(Value::as_str) {
                    self.storage = BrowserStorageState::load_from_path(Path::new(path_raw))?;
                    return Ok(json!({
                        "loaded": true,
                        "path": path_raw,
                        "cookies": self.storage.cookies.len(),
                        "origins": self.storage.origins.len(),
                    }));
                }
                anyhow::bail!("browser_state_load requires `state_json` or `path`")
            }
            "browser_cookies" => {
                let op_kind = args.get("action").and_then(Value::as_str).unwrap_or("list");
                match op_kind {
                    "set" => {
                        let cookie: BrowserCookie = serde_json::from_value(
                            args.get("cookie")
                                .cloned()
                                .ok_or_else(|| anyhow::anyhow!("missing `cookie`"))?,
                        )?;
                        self.storage.set_cookie(cookie);
                        Ok(json!({ "count": self.storage.cookies.len() }))
                    }
                    "clear" => {
                        let removed = self
                            .storage
                            .clear_cookies(args.get("name").and_then(Value::as_str));
                        Ok(json!({ "removed": removed, "remaining": self.storage.cookies.len() }))
                    }
                    _ => {
                        let host = args.get("host").and_then(Value::as_str);
                        let list = self.storage.cookies_for_host(host);
                        Ok(json!({ "count": list.len(), "cookies": list }))
                    }
                }
            }
            "browser_storage" => {
                let origin = args
                    .get("origin")
                    .and_then(Value::as_str)
                    .unwrap_or(&self.current_url)
                    .to_owned();
                if let (Some(key), Some(val)) = (
                    args.get("key").and_then(Value::as_str),
                    args.get("value").and_then(Value::as_str),
                ) {
                    self.storage.set_local_storage(&origin, key, val);
                }
                let items = self
                    .storage
                    .get_local_storage(&origin, args.get("key").and_then(Value::as_str));
                Ok(json!({ "origin": origin, "entries": items }))
            }
            "browser_tab_list" => Ok(json!({
                "active_index": self.tabs.active_index(),
                "tabs": self.tabs.tabs(),
            })),
            "browser_tab_new" => {
                let url = args.get("url").and_then(Value::as_str).unwrap_or("about:blank");
                let check = validate_navigation_url(url, &self.options);
                if url != "about:blank" && !check.allowed {
                    anyhow::bail!(
                        "tab navigation rejected: {}",
                        check.reason.unwrap_or_else(|| "blocked".to_owned())
                    );
                }
                let label = args.get("label").and_then(Value::as_str);
                let tab = self.tabs.open_tab(url, label, None, None)?.clone();
                self.current_url = tab.url.clone();
                Ok(json!({ "tab": tab, "active_index": self.tabs.active_index() }))
            }
            "browser_tab_switch" => {
                let selector = args
                    .get("tab")
                    .or_else(|| args.get("ref"))
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow::anyhow!("missing `tab` selector"))?;
                let tab = self.tabs.switch_tab(selector)?.clone();
                self.current_url = tab.url.clone();
                self.last_title = tab.title.clone();
                Ok(json!({ "tab": tab, "active_index": self.tabs.active_index() }))
            }
            "browser_tab_close" => {
                let selector = args.get("tab").or_else(|| args.get("ref")).and_then(Value::as_str);
                let closed = self.tabs.close_tab(selector)?;
                if let Some(active) = self.tabs.active_tab() {
                    self.current_url = active.url.clone();
                    self.last_title = active.title.clone();
                }
                Ok(json!({ "closed": closed, "active_index": self.tabs.active_index() }))
            }
            "browser_policy_check" => {
                if let Some(policy_json) = args.get("policy_json").and_then(Value::as_str) {
                    self.policy = BrowserActionPolicy::from_json_str(policy_json)?;
                }
                let action = args
                    .get("action")
                    .and_then(Value::as_str)
                    .unwrap_or("browser_status");
                let decision = self.policy.evaluate(action, confirm_token);
                Ok(serde_json::to_value(decision)?)
            }
            "browser_batch" => {
                let steps: Vec<BrowserBatchStep> = args
                    .get("steps")
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("missing `steps` array for browser_batch"))
                    .and_then(|val| serde_json::from_value(val).map_err(anyhow::Error::from))?;
                let stop_on_error = args
                    .get("stop_on_error")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
                let report = self.execute_batch(&steps, stop_on_error);
                Ok(serde_json::to_value(report)?)
            }
            other => anyhow::bail!("unknown browser operation '{other}'"),
        }
    }

    pub fn execute_batch(
        &mut self,
        steps: &[BrowserBatchStep],
        stop_on_error: bool,
    ) -> BrowserBatchReport {
        let mut results = Vec::with_capacity(steps.len());
        let mut succeeded = 0_usize;
        let mut failed = 0_usize;
        let mut stopped_early = false;
        let mut stopped_reason = None;

        for (index, step) in steps.iter().enumerate() {
            let mut merged_args = step.args.clone();
            if let (Some(token), Some(obj)) = (&step.confirm_token, merged_args.as_object_mut()) {
                obj.insert("confirm_token".to_owned(), Value::String(token.clone()));
            }
            match self.execute_tool(&step.op, &merged_args) {
                Ok(data) => {
                    succeeded += 1;
                    results.push(BrowserBatchStepResult {
                        index,
                        op: step.op.clone(),
                        ok: true,
                        data: Some(data),
                        error: None,
                    });
                }
                Err(err) => {
                    failed += 1;
                    results.push(BrowserBatchStepResult {
                        index,
                        op: step.op.clone(),
                        ok: false,
                        data: None,
                        error: Some(err.to_string()),
                    });
                    if stop_on_error {
                        stopped_early = true;
                        stopped_reason = Some("stop_on_error".to_owned());
                        break;
                    }
                }
            }
        }

        let completed = results.len();
        BrowserBatchReport {
            total: steps.len(),
            completed,
            succeeded,
            failed,
            not_started: steps.len().saturating_sub(completed),
            stopped_early,
            stopped_reason,
            steps: results,
        }
    }

    fn discovered_executable(&self) -> Option<String> {
        #[cfg(not(target_family = "wasm"))]
        {
            self.options
                .executable_path
                .clone()
                .or_else(|| find_chrome_executable().map(|p| p.display().to_string()))
        }
        #[cfg(target_family = "wasm")]
        {
            self.options.executable_path.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssrf_guard_blocks_private_loopback_and_metadata_ips() {
        let opts = BrowserLaunchOptions::default();
        assert!(validate_navigation_url("https://aphrody.com/docs", &opts).allowed);
        assert!(!validate_navigation_url("http://127.0.0.1:8080/", &opts).allowed);
        assert!(!validate_navigation_url("http://169.254.169.254/latest/meta-data", &opts).allowed);
        assert!(!validate_navigation_url("http://10.0.0.5/", &opts).allowed);
        assert!(!validate_navigation_url("file:///etc/passwd", &opts).allowed);

        let permissive = BrowserLaunchOptions {
            allow_private_network: true,
            allow_file_access: true,
            ..Default::default()
        };
        assert!(validate_navigation_url("http://127.0.0.1:9222/json", &permissive).allowed);
        assert!(validate_navigation_url("file:///C:/index.html", &permissive).allowed);
    }

    #[test]
    fn chrome_launch_args_enable_webmcp_and_headless_new() {
        let args = build_chrome_launch_args(&BrowserLaunchOptions::default(), "/tmp/aphrody-cdp");
        assert!(args.iter().any(|a| a == "--headless=new"));
        assert!(args.iter().any(|a| a.contains("WebMCPTesting")));
    }

    #[test]
    fn snapshot_diff_computes_additions_removals_and_unchanged() {
        let diff = diff_snapshot_text(
            "button \"Save\"\nlink \"Docs\"\n",
            "button \"Save\"\nbutton \"Publish\"\n",
        );
        assert!(diff.changed);
        assert_eq!(diff.unchanged, 1);
        assert_eq!(diff.removals, 1);
        assert_eq!(diff.additions, 1);
        assert!(diff.diff.contains("- link \"Docs\""));
        assert!(diff.diff.contains("+ button \"Publish\""));
    }

    #[test]
    fn semantic_locators_match_ax_tree_and_build_js() {
        let tree = json!({
            "nodes": [
                { "nodeId": "1", "role": { "value": "RootWebArea" }, "name": { "value": "Page" } },
                { "nodeId": "2", "role": { "value": "button" }, "name": { "value": "Sign In" } },
                { "nodeId": "3", "role": { "value": "button" }, "name": { "value": "Sign Up" } }
            ]
        });
        let hits = find_ax_nodes_by_locator(
            &tree,
            &SemanticLocator::role("button").with_name("Sign").with_nth(-1),
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0]["nodeId"], "3");

        let js = SemanticLocator::testid("submit-order")
            .with_exact(true)
            .build_semantic_query_js();
        assert!(js.contains("data-testid"));
        assert!(js.contains("submit-order"));
    }

    #[test]
    fn annotated_boxes_filter_overlaps_and_project_viewport() {
        let filtered = filter_non_overlapping_annotations(vec![
            AnnotatedElementBox {
                ref_id: "@e1".to_owned(),
                number: 1,
                role: "button".to_owned(),
                name: "Go".to_owned(),
                x: 10.0,
                y: 20.0,
                width: 80.0,
                height: 30.0,
            },
            AnnotatedElementBox {
                ref_id: "@e2".to_owned(),
                number: 2,
                role: "button".to_owned(),
                name: "Dup".to_owned(),
                x: 12.0,
                y: 22.0,
                width: 80.0,
                height: 30.0,
            },
        ]);
        assert_eq!(filtered.len(), 1);
        let scaled = project_annotation_boxes(filtered, 100.0, 100.0, 200.0, 200.0);
        assert_eq!(scaled[0].x, 20.0);
        assert_eq!(scaled[0].width, 160.0);
    }

    #[test]
    fn network_routes_and_har_ringbuffer_redact_headers_and_export() {
        let mut ctrl = BrowserControllerState::default();
        assert!(route_url_matches("https://*.aphrody.com/**", "https://cdn.aphrody.com/app/main.js"));
        ctrl.routes.add_route(
            "**/api/orders",
            RouteDisposition::Fulfill,
            Some(RouteResponse {
                status: 200,
                body: "{\"ok\":true}".to_owned(),
                ..Default::default()
            }),
            Some("POST".to_owned()),
            vec!["fetch".to_owned()],
        );
        let matched = ctrl
            .routes
            .select_route("https://app.aphrody.com/api/orders", Some("POST"), Some("fetch"));
        assert!(matched.is_some());

        ctrl.har.start(HarContentMode::Full, true);
        let mut req_headers = BTreeMap::new();
        req_headers.insert("Authorization".to_owned(), "Bearer top-secret".to_owned());
        ctrl.har.push(HarEntry {
            request_id: "req-1".to_owned(),
            started_ms: 1000,
            time_ms: 12.5,
            method: "POST".to_owned(),
            url: "https://app.aphrody.com/api/orders".to_owned(),
            status: 200,
            status_text: "OK".to_owned(),
            request_headers: req_headers,
            response_headers: BTreeMap::new(),
            request_body: None,
            response_body: Some("{\"ok\":true}".to_owned()),
            mime_type: "application/json".to_owned(),
            encoded_bytes: 11,
            from_route: true,
        });
        let har = ctrl.har.export_har_1_2("https://app.aphrody.com", "Orders");
        assert_eq!(har["log"]["version"], "1.2");
        assert_eq!(
            har["log"]["entries"][0]["request"]["headers"][0]["value"],
            "[REDACTED]"
        );
    }

    #[test]
    fn tabs_storage_policy_and_batch_work_end_to_end() {
        let mut ctrl = BrowserControllerState::default();
        let tab2 = ctrl
            .execute_tool(
                "browser_tab_new",
                &json!({ "url": "https://aphrody.com/docs", "label": "docs" }),
            )
            .expect("open tab");
        assert_eq!(tab2["tab"]["tab_ref"], "t2");

        let cookie_res = ctrl
            .execute_tool(
                "browser_cookies",
                &json!({
                    "action": "set",
                    "cookie": { "name": "sid", "value": "abc", "domain": "aphrody.com" }
                }),
            )
            .expect("set cookie");
        assert_eq!(cookie_res["count"], 1);
        let saved = ctrl
            .execute_tool("browser_state_save", &json!({}))
            .expect("serialize state");
        assert!(saved["state_json"].as_str().is_some());

        ctrl.policy = BrowserActionPolicy::from_json_str(
            r#"{"default_decision":"allow","confirm":["eval"],"deny":["network"]}"#,
        )
        .expect("parse policy");
        let report = ctrl.execute_batch(
            &[
                BrowserBatchStep {
                    op: "browser_tab_list".to_owned(),
                    args: json!({}),
                    confirm_token: None,
                },
                BrowserBatchStep {
                    op: "browser_har_start".to_owned(),
                    args: json!({}),
                    confirm_token: None,
                },
                BrowserBatchStep {
                    op: "browser_status".to_owned(),
                    args: json!({}),
                    confirm_token: None,
                },
            ],
            true,
        );
        assert!(report.stopped_early);
        assert_eq!(report.succeeded, 1);
        assert_eq!(report.failed, 1);
        assert_eq!(report.not_started, 1);
    }
}
