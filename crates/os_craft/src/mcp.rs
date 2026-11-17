#[cfg(not(target_family = "wasm"))]
use std::io::{BufRead, Read, Write};
#[cfg(not(target_family = "wasm"))]
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use os_pilot::{CommandContext, PilotEngine, TargetBackend, builtin_pilot_tools};
use os_session::{
    CompositorState, Rect, SessionManifest, SessionSupervisor, TilingMode, discover_system_apps,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::catalog::CraftCatalog;
use crate::pipeline::inspect_path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDescriptor {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Live shared state accessible from both GPUI desktop UI and headless MCP / HTTP agents.
pub struct McpServerState {
    pub compositor: CompositorState,
    pub supervisor: SessionSupervisor,
    pub catalog: CraftCatalog,
    pub pilot: PilotEngine,
    pub active_theme: String,
    pub active_dark: bool,
    pub active_studio_app: String,
}

impl McpServerState {
    pub fn bootstrap() -> anyhow::Result<Self> {
        let catalog = CraftCatalog::load_with_user_override()?;
        let manifest = SessionManifest::load_with_user_override()?;
        let ws_names: Vec<&str> = catalog
            .workspaces
            .default_names
            .iter()
            .map(String::as_str)
            .collect();
        let layout = TilingMode::parse(&catalog.workspaces.default_layout).unwrap_or_default();
        let mut supervisor = SessionSupervisor::new(manifest.clone())?;
        supervisor.activate_session();

        let active_theme = manifest.session.default_theme;
        let active_dark = !manifest
            .session
            .default_appearance
            .eq_ignore_ascii_case("light");
        let active_studio_app = catalog
            .apps
            .first()
            .map(|a| a.id.clone())
            .unwrap_or_else(|| "photocraft".to_owned());

        let mut compositor =
            CompositorState::new(Rect::new(0.0, 0.0, 1440.0, 900.0), &ws_names, layout);
        compositor.open_window("ArtCraft Studio", &active_studio_app);

        let pilot = PilotEngine::new();
        pilot
            .gpui
            .sync_compositor(compositor.clone(), &active_theme, active_dark);

        Ok(Self {
            compositor,
            supervisor,
            catalog,
            pilot,
            active_theme,
            active_dark,
            active_studio_app,
        })
    }
}

#[must_use]
pub fn builtin_tools() -> Vec<McpToolDescriptor> {
    let mut tools = vec![
        McpToolDescriptor {
            name: "os_workspace_list".to_owned(),
            description: "List active COSMIC workspace, layout mode, and placed window rectangles."
                .to_owned(),
            input_schema: json!({"type": "object", "properties": {}, "additionalProperties": false}),
        },
        McpToolDescriptor {
            name: "os_workspace_switch".to_owned(),
            description: "Switch the active COSMIC desktop workspace by numeric id (1..N).".to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": { "id": { "type": "integer", "minimum": 1 } },
                "required": ["id"],
                "additionalProperties": false
            }),
        },
        McpToolDescriptor {
            name: "os_layout_set".to_owned(),
            description: "Set tiling layout mode (dwindle, cosmic_grid, niri_scroll, floating)."
                .to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "mode": { "type": "string", "enum": ["dwindle", "cosmic_grid", "niri_scroll", "floating"] }
                },
                "required": ["mode"],
                "additionalProperties": false
            }),
        },
        McpToolDescriptor {
            name: "os_window_open".to_owned(),
            description: "Open a new OS window by title and app_id / ArtCraft studio id.".to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string" },
                    "app_id": { "type": "string" }
                },
                "required": ["title"],
                "additionalProperties": false
            }),
        },
        McpToolDescriptor {
            name: "os_theme_set".to_owned(),
            description: "Switch desktop theme preset id and dark/light appearance.".to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "theme": { "type": "string" },
                    "dark": { "type": "boolean" }
                },
                "required": ["theme"],
                "additionalProperties": false
            }),
        },
        McpToolDescriptor {
            name: "os_session_status".to_owned(),
            description: "Return live COSMIC session unit statuses and detected host environment."
                .to_owned(),
            input_schema: json!({"type": "object", "properties": {}, "additionalProperties": false}),
        },
        McpToolDescriptor {
            name: "os_apps_discover".to_owned(),
            description: "Discover installed OS applications from XDG/Windows/macOS directories."
                .to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": { "limit": { "type": "integer", "minimum": 1, "maximum": 200 } },
                "additionalProperties": false
            }),
        },
        McpToolDescriptor {
            name: "craft_catalog_list".to_owned(),
            description: "List all declarative ArtCraft + Craft Anything studio modes and codecs."
                .to_owned(),
            input_schema: json!({"type": "object", "properties": {}, "additionalProperties": false}),
        },
        McpToolDescriptor {
            name: "craft_sniff_file".to_owned(),
            description: "Inspect a local file via craft-sniff, craft-psd, craft-exr, craft-plist, usvg, wasmparser, and object."
                .to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": { "path": { "type": "string" } },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
        McpToolDescriptor {
            name: "craft_anything_status".to_owned(),
            description: "Discover local craft-anything submodule crates and executable runtime availability."
                .to_owned(),
            input_schema: json!({"type": "object", "properties": {}, "additionalProperties": false}),
        },
        McpToolDescriptor {
            name: "wasm_wgpu_doctor".to_owned(),
            description: "Inspect host Wasm/WebGPU toolchain binaries (`gpui-wgpu`, `wasmtime`, `wasm-opt`, `wasm-bindgen`, `wasm-tools`, `bun`)."
                .to_owned(),
            input_schema: json!({"type": "object", "properties": {}, "additionalProperties": false}),
        },
        McpToolDescriptor {
            name: "wasm_inspect_binary".to_owned(),
            description: "Inspect a `.wasm` or component file header, sections, imports, exports, and size."
                .to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": { "path": { "type": "string" } },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
    ];
    for pt in builtin_pilot_tools() {
        tools.push(McpToolDescriptor {
            name: pt.name.to_owned(),
            description: pt.description.to_owned(),
            input_schema: pt.input_schema,
        });
    }
    tools
}

/// Dispatches one JSON-RPC 2.0 request line against [`McpServerState`].
#[must_use]
pub fn handle_json_rpc_line(state: &Arc<Mutex<McpServerState>>, line: &str) -> Option<String> {
    let parsed: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(err) => {
            return Some(
                json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": err.to_string()}})
                    .to_string(),
            );
        }
    };

    let id = parsed.get("id").cloned();
    let method = parsed.get("method").and_then(Value::as_str).unwrap_or("");
    let params = parsed.get("params").cloned().unwrap_or_else(|| json!({}));

    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2025-03-26",
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": format!("{}-os", brand::APP_SLUG), "version": env!("CARGO_PKG_VERSION") }
        })),
        "notifications/initialized" | "notifications/cancelled" => return None,
        "tools/list" => Ok(json!({ "tools": builtin_tools() })),
        "tools/call" => call_tool(state, &params),
        other => Err((-32601, format!("unknown method '{other}'"))),
    };

    let id = id?;
    let payload = match result {
        Ok(res) => json!({ "jsonrpc": "2.0", "id": id, "result": res }),
        Err((code, msg)) => {
            json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": msg } })
        }
    };
    Some(payload.to_string())
}

fn call_tool(state: &Arc<Mutex<McpServerState>>, params: &Value) -> Result<Value, (i32, String)> {
    let tool_name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or((-32602, "missing tool name".to_owned()))?;
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let mut guard = state
        .lock()
        .map_err(|e| (-32000, format!("state lock poisoned: {e}")))?;

    let structured = match tool_name {
        "os_workspace_list" => {
            let active_ws = guard.compositor.active_workspace;
            let layout = guard
                .compositor
                .workspaces
                .get(&active_ws)
                .map(|w| w.mode.slug())
                .unwrap_or("dwindle");
            let windows = guard.compositor.arrange_active_workspace();
            json!({
                "active_workspace": active_ws,
                "layout": layout,
                "theme": guard.active_theme,
                "dark": guard.active_dark,
                "windows": windows,
            })
        }
        "os_workspace_switch" => {
            let ws_id = args
                .get("id")
                .and_then(Value::as_u64)
                .ok_or((-32602, "missing workspace id".to_owned()))? as u32;
            let switched = guard.compositor.switch_workspace(ws_id);
            if !switched {
                return Err((-32602, format!("unknown workspace {ws_id}")));
            }
            json!({ "active_workspace": ws_id })
        }
        "os_layout_set" => {
            let mode_str = args
                .get("mode")
                .and_then(Value::as_str)
                .ok_or((-32602, "missing mode".to_owned()))?;
            let mode = TilingMode::parse(mode_str)
                .ok_or((-32602, format!("unknown tiling mode '{mode_str}'")))?;
            guard.compositor.set_active_mode(mode);
            json!({ "layout": mode.slug(), "windows": guard.compositor.arrange_active_workspace() })
        }
        "os_window_open" => {
            let title = args
                .get("title")
                .and_then(Value::as_str)
                .ok_or((-32602, "missing window title".to_owned()))?;
            let app_id = args
                .get("app_id")
                .and_then(Value::as_str)
                .unwrap_or(&guard.active_studio_app)
                .to_owned();
            if guard.catalog.by_id(&app_id).is_some() {
                guard.active_studio_app = app_id.clone();
            }
            let win_id = guard.compositor.open_window(title, app_id);
            json!({ "window_id": win_id, "windows": guard.compositor.arrange_active_workspace() })
        }
        "os_theme_set" => {
            let theme = args
                .get("theme")
                .and_then(Value::as_str)
                .ok_or((-32602, "missing theme".to_owned()))?
                .to_owned();
            guard.active_theme = theme;
            if let Some(dark) = args.get("dark").and_then(Value::as_bool) {
                guard.active_dark = dark;
            }
            json!({ "theme": guard.active_theme, "dark": guard.active_dark })
        }
        "os_session_status" => json!({
            "session": guard.supervisor.manifest.session,
            "units": guard.supervisor.snapshot(),
        }),
        "os_apps_discover" => {
            let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(50) as usize;
            json!({ "apps": discover_system_apps(limit) })
        }
        "craft_catalog_list" => json!({ "apps": guard.catalog.apps }),
        "craft_sniff_file" => {
            let raw_path = args
                .get("path")
                .and_then(Value::as_str)
                .ok_or((-32602, "missing file path".to_owned()))?;
            let report = inspect_path(&PathBuf::from(raw_path), Some(&guard.catalog))
                .map_err(|e| (-32001, e.to_string()))?;
            serde_json::to_value(report).map_err(|e| (-32000, e.to_string()))?
        }
        "craft_anything_status" => craft_anything_discovery_json(),
        "wasm_wgpu_doctor" => collect_wasm_wgpu_doctor_json(),
        "wasm_inspect_binary" => {
            let raw_path = args
                .get("path")
                .and_then(Value::as_str)
                .ok_or((-32602, "missing wasm file path".to_owned()))?;
            inspect_wasm_binary_json(&PathBuf::from(raw_path))
                .map_err(|e| (-32001, e.to_string()))?
        }
        other if other.starts_with("desktop_") => {
            let sync_comp = guard.compositor.clone();
            let theme = guard.active_theme.clone();
            let dark = guard.active_dark;
            guard.pilot.gpui.sync_compositor(sync_comp, &theme, dark);
            let ctx = CommandContext {
                app: args.get("app").and_then(Value::as_str).map(str::to_owned),
                window_id: args
                    .get("window_id")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                timeout_ms: args.get("timeout_ms").and_then(Value::as_u64),
                target: TargetBackend::parse(args.get("target").and_then(Value::as_str)),
                ..Default::default()
            };
            let val = guard
                .pilot
                .execute_tool(other, &args, &ctx)
                .map_err(|err| (-32002, err.to_string()))?;
            guard.compositor = guard.pilot.gpui.export_compositor();
            val
        }
        other => return Err((-32601, format!("unknown tool '{other}'"))),
    };

    if matches!(
        tool_name,
        "os_workspace_switch" | "os_layout_set" | "os_window_open" | "os_theme_set"
    ) {
        let comp = guard.compositor.clone();
        let theme = guard.active_theme.clone();
        let dark = guard.active_dark;
        guard.pilot.gpui.sync_compositor(comp, &theme, dark);
    }

    Ok(json!({
        "content": [{ "type": "text", "text": structured.to_string() }],
        "structuredContent": structured
    }))
}

/// Runs blocking stdio JSON-RPC 2.0 MCP loop until EOF.
#[cfg(not(target_family = "wasm"))]
pub fn serve_stdio(state: Arc<Mutex<McpServerState>>) -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(resp) = handle_json_rpc_line(&state, &line) {
            writeln!(out, "{resp}")?;
            out.flush()?;
        }
    }
    Ok(())
}

/// Resolves bind address dynamically (`explicit` -> `APHRODY_MCP_ADDR` -> ephemeral localhost).
#[cfg(not(target_family = "wasm"))]
#[must_use]
pub fn resolve_bind_addr(explicit: Option<&str>) -> String {
    if let Some(addr) = explicit
        && !addr.trim().is_empty()
    {
        return addr.to_owned();
    }
    if let Ok(addr) = brand::env_var("MCP_ADDR")
        && !addr.trim().is_empty()
    {
        return addr;
    }
    format!("{}:0", std::net::Ipv4Addr::LOCALHOST)
}

/// Minimal dependency-free HTTP/1.1 MCP & health discovery server.
#[cfg(not(target_family = "wasm"))]
pub fn serve_http(
    state: Arc<Mutex<McpServerState>>,
    addr_override: Option<&str>,
) -> anyhow::Result<()> {
    let requested = resolve_bind_addr(addr_override);
    let listener = TcpListener::bind(&requested)?;
    let local_addr = listener.local_addr()?;
    log::info!(
        "{} OS MCP HTTP endpoint listening on http://{local_addr}",
        brand::APP_NAME
    );
    println!("{{\"listening\":\"http://{local_addr}\"}}");

    for stream in listener.incoming() {
        let Ok(stream) = stream else {
            continue;
        };
        let state = Arc::clone(&state);
        std::thread::spawn(move || {
            if let Err(err) = handle_http_conn(stream, &state) {
                log::debug!("mcp http connection closed cleanly: {err:#}");
            }
        });
    }
    Ok(())
}

fn craft_anything_discovery_json() -> Value {
    let vendor_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|root| root.join("vendor"));
    let manifest_root = vendor_dir.as_ref().map(|v| v.join("craft-anything"));
    let submodule_present = manifest_root
        .as_ref()
        .is_some_and(|p| p.join("Cargo.toml").is_file());
    let mut crate_dirs = Vec::new();
    if let Some(base) = &manifest_root
        && let Ok(entries) = std::fs::read_dir(base.join("crates"))
    {
        for entry in entries.flatten() {
            if entry.path().join("Cargo.toml").is_file()
                && let Some(name) = entry.file_name().to_str()
            {
                crate_dirs.push(name.to_owned());
            }
        }
        crate_dirs.sort();
    }
    let mut vendor_modules = Vec::new();
    if let Some(vroot) = &vendor_dir
        && let Ok(entries) = std::fs::read_dir(vroot)
    {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir()
                && let Some(name) = entry.file_name().to_str()
            {
                vendor_modules.push(name.to_owned());
            }
        }
        vendor_modules.sort();
    }
    let configured_bin = std::env::var("CRAFT_ANYTHING_BIN").ok();
    json!({
        "submodule_present": submodule_present,
        "submodule_crates": crate_dirs,
        "vendor_modules": vendor_modules,
        "configured_bin": configured_bin,
    })
}

#[cfg(not(target_family = "wasm"))]
fn is_loopback_host(raw: &str) -> bool {
    let trimmed = raw.trim().to_ascii_lowercase();
    let host = if trimmed.starts_with('[') {
        trimmed
            .strip_prefix('[')
            .and_then(|s| s.split(']').next())
            .unwrap_or("")
    } else {
        trimmed.split(':').next().unwrap_or("")
    };
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

#[cfg(not(target_family = "wasm"))]
fn validate_local_http_headers(
    headers_block: &str,
    expected_token: Option<&str>,
) -> Result<(), (&'static str, &'static str)> {
    let mut host_header: Option<&str> = None;
    let mut origin_header: Option<&str> = None;
    let mut auth_header: Option<&str> = None;

    for line in headers_block.lines().skip(1) {
        let Some((name, val)) = line.split_once(':') else {
            continue;
        };
        let lower = name.trim().to_ascii_lowercase();
        let val = val.trim();
        match lower.as_str() {
            "host" => host_header = Some(val),
            "origin" => origin_header = Some(val),
            "authorization" => auth_header = Some(val),
            _ => {}
        }
    }

    if let Some(host) = host_header
        && !is_loopback_host(host)
    {
        return Err(("403 Forbidden", "forbidden non-loopback Host header"));
    }

    if let Some(origin) = origin_header {
        let origin_lower = origin.to_ascii_lowercase();
        let scheme_ok = origin_lower.starts_with(&format!("{}://", brand::URL_SCHEME))
            || origin_lower.starts_with("http://localhost")
            || origin_lower.starts_with("http://127.0.0.1")
            || origin_lower.starts_with("http://[::1]");
        if !scheme_ok {
            return Err(("403 Forbidden", "forbidden cross-origin Origin header"));
        }
    }

    if let Some(token) = expected_token {
        let provided = auth_header
            .and_then(|v| {
                v.strip_prefix("Bearer ")
                    .or_else(|| v.strip_prefix("bearer "))
            })
            .map(str::trim);
        if provided != Some(token) {
            return Err(("401 Unauthorized", "missing or invalid bearer token"));
        }
    }

    Ok(())
}

#[cfg(not(target_family = "wasm"))]
fn handle_http_conn(
    mut stream: TcpStream,
    state: &Arc<Mutex<McpServerState>>,
) -> anyhow::Result<()> {
    let mut buf = vec![0_u8; 64 * 1024];
    let n = stream.read(&mut buf)?;
    if n == 0 {
        return Ok(());
    }
    let req_str = String::from_utf8_lossy(&buf[..n]);
    let (headers_block, body_raw) = req_str.split_once("\r\n\r\n").unwrap_or((&req_str, ""));
    let mut lines = headers_block.lines();
    let request_line = lines.next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let path = parts.next().unwrap_or("/");
    let body = body_raw.trim();
    let expected_token = brand::env_var("MCP_TOKEN").ok().filter(|s| !s.is_empty());
    let accept_language = headers_block.lines().skip(1).find_map(|line| {
        let (name, val) = line.split_once(':')?;
        name.trim()
            .eq_ignore_ascii_case("accept-language")
            .then_some(val.trim())
    });
    let negotiated = os_session::negotiate_locale(
        accept_language,
        &[
            "en", "fr", "ar", "he", "es", "de", "ja", "zh-Hans", "pt", "hi", "ko", "uk",
        ],
        "en",
    );

    let (status, payload) = if let Err((err_status, reason)) =
        validate_local_http_headers(headers_block, expected_token.as_deref())
    {
        (err_status, json!({ "error": reason }).to_string())
    } else {
        match (method, path) {
            ("GET", "/health") => (
                "200 OK",
                json!({
                    "ok": true,
                    "service": brand::APP_SLUG,
                    "lang": negotiated.locale,
                    "dir": negotiated.dir,
                })
                .to_string(),
            ),
            ("GET", "/.well-known/webmcp.json") => (
                "200 OK",
                json!({
                    "lang": negotiated.locale,
                    "dir": negotiated.dir,
                    "tools": builtin_tools(),
                })
                .to_string(),
            ),
            ("POST", "/mcp") => {
                let out = handle_json_rpc_line(state, body)
                    .unwrap_or_else(|| json!({ "ok": true }).to_string());
                ("200 OK", out)
            }
            _ => ("404 Not Found", json!({ "error": "not found" }).to_string()),
        }
    };

    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Language: {}\r\nVary: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
        negotiated.content_language,
        negotiated.vary,
        payload.len()
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    Ok(())
}

#[cfg(not(target_family = "wasm"))]
#[allow(
    clippy::disallowed_methods,
    reason = "synchronous CLI tool probe in standalone MCP handler"
)]
fn collect_wasm_wgpu_doctor_json() -> Value {
    let mut tools = Vec::new();
    for bin in [
        brand::WGPU_CLI_BIN,
        "wasmtime",
        "wasm-opt",
        "wasm-bindgen",
        "wasm-tools",
        "wit-bindgen",
        "bun",
    ] {
        let out = std::process::Command::new(bin).arg("--version").output();
        let (available, version) = match out {
            Ok(res) if res.status.success() => (
                true,
                String::from_utf8_lossy(&res.stdout)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_owned(),
            ),
            _ => (false, String::new()),
        };
        tools.push(json!({ "tool": bin, "available": available, "version": version }));
    }
    json!({
        "platform": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "tools": tools,
    })
}

#[cfg(target_family = "wasm")]
fn collect_wasm_wgpu_doctor_json() -> Value {
    json!({
        "platform": "wasm",
        "arch": std::env::consts::ARCH,
        "tools": [],
    })
}

fn inspect_wasm_binary_json(path: &std::path::Path) -> anyhow::Result<Value> {
    let bytes = std::fs::read(path)?;
    let is_wasm = bytes.len() >= 8 && bytes[0..4] == [0x00, 0x61, 0x73, 0x6d];
    anyhow::ensure!(is_wasm, "not a valid Wasm binary: {}", path.display());
    let is_component = bytes[4..8] == [0x0d, 0x00, 0x01, 0x00];

    #[cfg(feature = "wasm-inspect")]
    let (imports, exports, sections) = {
        let mut imports = Vec::new();
        let mut exports = Vec::new();
        let mut sections = Vec::new();
        for payload in wasmparser::Parser::new(0).parse_all(&bytes) {
            match payload? {
                wasmparser::Payload::ImportSection(reader) => {
                    sections.push("import");
                    for imp in reader.into_imports() {
                        let imp = imp?;
                        imports.push(format!("{}::{}", imp.module, imp.name));
                    }
                }
                wasmparser::Payload::ExportSection(reader) => {
                    sections.push("export");
                    for exp in reader {
                        let exp = exp?;
                        exports.push(exp.name.to_owned());
                    }
                }
                wasmparser::Payload::CustomSection(reader) => {
                    if !reader.name().is_empty() {
                        sections.push("custom");
                    }
                }
                wasmparser::Payload::CodeSectionStart { .. } => sections.push("code"),
                _ => {}
            }
        }
        (imports, exports, sections)
    };

    #[cfg(not(feature = "wasm-inspect"))]
    let (imports, exports, sections): (Vec<String>, Vec<String>, Vec<&'static str>) =
        (Vec::new(), Vec::new(), Vec::new());

    Ok(json!({
        "path": path.display().to_string(),
        "raw_bytes": bytes.len(),
        "is_component": is_component,
        "imports": imports,
        "exports": exports,
        "sections": sections,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_json_rpc_roundtrip_controls_workspaces_and_craft_catalog() {
        let state = Arc::new(Mutex::new(
            McpServerState::bootstrap().expect("bootstrap ok"),
        ));

        let init = handle_json_rpc_line(
            &state,
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
        )
        .expect("initialize returns response");
        assert!(init.contains("\"protocolVersion\""));

        let list = handle_json_rpc_line(
            &state,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
        )
        .expect("tools/list responds");
        assert!(list.contains("os_workspace_list"));
        assert!(list.contains("craft_sniff_file"));

        let switched = handle_json_rpc_line(
            &state,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"os_workspace_switch","arguments":{"id":2}}}"#,
        )
        .expect("switch workspace responds");
        assert!(switched.contains("\"active_workspace\":2"));

        assert!(list.contains("desktop_snapshot"));
        assert!(list.contains("desktop_ai_act_verify"));

        let snap = handle_json_rpc_line(
            &state,
            r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"desktop_snapshot","arguments":{"interactive":true}}}"#,
        )
        .expect("desktop_snapshot responds");
        assert!(snap.contains("Workspace 2"));

        let ai = handle_json_rpc_line(
            &state,
            r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"desktop_ai_act_verify","arguments":{"locator":{"role":"button","name":"Workspace 1 (Studio)"},"action":{"kind":"click"}}}}"#,
        )
        .expect("ai_act_verify responds");
        assert!(ai.contains("\"verified\":true"));

        let after_ws = handle_json_rpc_line(
            &state,
            r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"os_workspace_list","arguments":{}}}"#,
        )
        .expect("os_workspace_list reflects AI click");
        assert!(after_ws.contains("\"active_workspace\":1"));

        assert!(list.contains("wasm_wgpu_doctor"));
        assert!(list.contains("wasm_inspect_binary"));
        let doc_res = handle_json_rpc_line(
            &state,
            r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"wasm_wgpu_doctor","arguments":{}}}"#,
        )
        .expect("wasm_wgpu_doctor responds");
        assert!(doc_res.contains("tools"));

        let temp_wasm_dir = tempfile::tempdir().expect("tempdir creates");
        let wasm_file = temp_wasm_dir.path().join("minimal.wasm");
        std::fs::write(&wasm_file, [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00])
            .expect("writes minimal wasm");
        let insp_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "tools/call",
            "params": {
                "name": "wasm_inspect_binary",
                "arguments": { "path": wasm_file.display().to_string() }
            }
        });
        let insp_res = handle_json_rpc_line(&state, &insp_req.to_string())
            .expect("wasm_inspect_binary responds");
        assert!(insp_res.contains("\"raw_bytes\":8"));
    }

    #[cfg(not(target_family = "wasm"))]
    #[test]
    fn mcp_http_blocks_dns_rebinding_cross_origin_and_bad_bearer_token() {
        let rebind = "POST /mcp HTTP/1.1\r\nHost: evil-rebinder.example:4195\r\n";
        assert!(matches!(
            validate_local_http_headers(rebind, None),
            Err(("403 Forbidden", _))
        ));

        let bad_origin =
            "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:4195\r\nOrigin: https://evil.example\r\n";
        assert!(matches!(
            validate_local_http_headers(bad_origin, None),
            Err(("403 Forbidden", _))
        ));

        let unauth = "POST /mcp HTTP/1.1\r\nHost: localhost:4195\r\n";
        assert!(matches!(
            validate_local_http_headers(unauth, Some("secret-token")),
            Err(("401 Unauthorized", _))
        ));

        let valid = "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:4195\r\nOrigin: http://127.0.0.1:4195\r\nAuthorization: Bearer secret-token\r\n";
        assert!(validate_local_http_headers(valid, Some("secret-token")).is_ok());
    }
}
