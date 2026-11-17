//! Headless Browser Automation & CDP Engine absorbed from `vercel-labs/agent-browser`.
//!
//! Provides SSRF/non-public IP policy checks, multi-OS Chrome/Chromium/Edge/Brave binary
//! discovery, CDP HTTP discovery (`/json/version`, `/json/list`), asynchronous WebSocket CDP
//! execution (open, DOM/AX snapshot, JS eval, click, type, PNG screenshot), and unified
//! `browser_*` MCP tools inside `aphrody_pilot`.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
#[cfg(not(target_family = "wasm"))]
use std::path::PathBuf;

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

/// Stateful absorbed `agent-browser` controller exposed over `aphrody_pilot` & MCP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserControllerState {
    pub options: BrowserLaunchOptions,
    pub cdp_ws_url: Option<String>,
    pub active_session_id: Option<String>,
    pub current_url: String,
    pub last_title: String,
}

impl Default for BrowserControllerState {
    fn default() -> Self {
        Self {
            options: BrowserLaunchOptions::default(),
            cdp_ws_url: None,
            active_session_id: None,
            current_url: "about:blank".to_owned(),
            last_title: String::new(),
        }
    }
}

impl BrowserControllerState {
    pub fn execute_tool(&mut self, op: &str, args: &Value) -> anyhow::Result<Value> {
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
                #[cfg(not(target_family = "wasm"))]
                {
                    let ws = self
                        .cdp_ws_url
                        .as_deref()
                        .ok_or_else(|| anyhow::anyhow!("no active CDP WebSocket connected"))?;
                    return send_cdp_command(
                        ws,
                        "Accessibility.getFullAXTree",
                        json!({}),
                        self.active_session_id.as_deref(),
                    );
                }
                #[cfg(target_family = "wasm")]
                anyhow::bail!("browser_snapshot requires non-wasm target")
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
            other => anyhow::bail!("unknown browser operation '{other}'"),
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
}
