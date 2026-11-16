#[cfg(not(target_family = "wasm"))]
use std::io::{BufRead, Read, Write};
#[cfg(not(target_family = "wasm"))]
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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

        Ok(Self {
            compositor,
            supervisor,
            catalog,
            active_theme,
            active_dark,
            active_studio_app,
        })
    }
}

#[must_use]
pub fn builtin_tools() -> Vec<McpToolDescriptor> {
    vec![
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
    ]
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
        other => return Err((-32601, format!("unknown tool '{other}'"))),
    };

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
    let mut lines = req_str.split("\r\n");
    let request_line = lines.next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let path = parts.next().unwrap_or("/");
    let body = req_str
        .split_once("\r\n\r\n")
        .map(|(_, b)| b.trim())
        .unwrap_or("");

    let (status, payload) = match (method, path) {
        ("GET", "/health") => (
            "200 OK",
            json!({ "ok": true, "service": brand::APP_SLUG }).to_string(),
        ),
        ("GET", "/.well-known/webmcp.json") => {
            ("200 OK", json!({ "tools": builtin_tools() }).to_string())
        }
        ("POST", "/mcp") => {
            let out = handle_json_rpc_line(state, body)
                .unwrap_or_else(|| json!({ "ok": true }).to_string());
            ("200 OK", out)
        }
        _ => ("404 Not Found", json!({ "error": "not found" }).to_string()),
    };

    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
        payload.len()
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    Ok(())
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
    }
}
