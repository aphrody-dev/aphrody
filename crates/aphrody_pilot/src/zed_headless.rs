//! Headless Zed IDE Controller, Tree-sitter AST/Symbol Analyzer, Low-Level OS Inspector, and
//! Stdio JSON-RPC 2.0 MCP Server (`aphrody --headless --mcp`).

use std::collections::BTreeMap;
#[cfg(not(target_family = "wasm"))]
use std::fs;
#[cfg(not(target_family = "wasm"))]
use std::io::{self, BufRead as _, Write as _};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::adapter::{CommandContext, PilotEngine};
use crate::model::PilotError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyntaxLanguage {
    Rust,
    TypeScript,
    Tsx,
    Python,
    Json,
}

impl SyntaxLanguage {
    #[must_use]
    pub fn from_name_or_path(raw: &str) -> Option<Self> {
        let lower = raw.trim().to_ascii_lowercase();
        if lower == "rust" || lower.ends_with(".rs") {
            Some(Self::Rust)
        } else if lower == "tsx" || lower.ends_with(".tsx") {
            Some(Self::Tsx)
        } else if lower == "typescript"
            || lower == "ts"
            || lower.ends_with(".ts")
            || lower.ends_with(".mts")
        {
            Some(Self::TypeScript)
        } else if lower == "python" || lower == "py" || lower.ends_with(".py") {
            Some(Self::Python)
        } else if lower == "json" || lower.ends_with(".json") {
            Some(Self::Json)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeSitterSymbol {
    pub kind: String,
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeSitterReport {
    pub language: SyntaxLanguage,
    pub byte_len: usize,
    pub line_count: usize,
    pub syntax_errors: usize,
    pub symbols: Vec<TreeSitterSymbol>,
    pub root_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadlessBuffer {
    pub path: String,
    pub language: Option<SyntaxLanguage>,
    pub line_count: usize,
    pub byte_len: usize,
    pub dirty: bool,
    pub content: String,
}

/// Full headless Zed workspace model driven cleanly over stdio MCP or Rust library API.
#[derive(Debug, Clone)]
pub struct HeadlessZedController {
    pub workspace_root: PathBuf,
    pub buffers: BTreeMap<String, HeadlessBuffer>,
    pub active_buffer: Option<String>,
}

impl Default for HeadlessZedController {
    fn default() -> Self {
        Self::new(
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        )
    }
}

impl HeadlessZedController {
    #[must_use]
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            buffers: BTreeMap::new(),
            active_buffer: None,
        }
    }

    pub fn open_buffer(&mut self, rel_or_abs: &str) -> anyhow::Result<&HeadlessBuffer> {
        let resolved = resolve_workspace_path(&self.workspace_root, rel_or_abs);
        #[cfg(not(target_family = "wasm"))]
        let content = fs::read_to_string(&resolved)?;
        #[cfg(target_family = "wasm")]
        let content = String::new();

        let key = resolved.display().to_string();
        let line_count = content.lines().count();
        let byte_len = content.len();
        let language = SyntaxLanguage::from_name_or_path(&key);
        self.buffers.insert(
            key.clone(),
            HeadlessBuffer {
                path: key.clone(),
                language,
                line_count,
                byte_len,
                dirty: false,
                content,
            },
        );
        self.active_buffer = Some(key.clone());
        self.buffers
            .get(&key)
            .ok_or_else(|| anyhow::anyhow!("buffer vanished after insertion"))
    }

    pub fn apply_edit(&mut self, path: &str, new_content: String) -> anyhow::Result<&HeadlessBuffer> {
        let resolved = resolve_workspace_path(&self.workspace_root, path);
        let key = resolved.display().to_string();
        let line_count = new_content.lines().count();
        let byte_len = new_content.len();
        let language = SyntaxLanguage::from_name_or_path(&key);
        self.buffers.insert(
            key.clone(),
            HeadlessBuffer {
                path: key.clone(),
                language,
                line_count,
                byte_len,
                dirty: true,
                content: new_content,
            },
        );
        self.active_buffer = Some(key.clone());
        self.buffers
            .get(&key)
            .ok_or_else(|| anyhow::anyhow!("buffer vanished after edit"))
    }

    pub fn execute_tool(&mut self, tool_name: &str, args: &Value) -> anyhow::Result<Value> {
        match tool_name {
            "zed_headless_status" => Ok(json!({
                "mode": "headless",
                "product": brand::APP_NAME,
                "workspace_root": self.workspace_root.display().to_string(),
                "open_buffers": self.buffers.keys().collect::<Vec<_>>(),
                "active_buffer": self.active_buffer,
                "supported_tree_sitter_languages": ["rust", "typescript", "tsx", "python", "json"],
                "os": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
            })),
            "zed_headless_open" => {
                let path = args
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow::anyhow!("missing `path`"))?;
                let buf = self.open_buffer(path)?;
                Ok(json!({
                    "path": buf.path,
                    "language": buf.language,
                    "line_count": buf.line_count,
                    "byte_len": buf.byte_len,
                }))
            }
            "zed_headless_read" => {
                let path = args
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow::anyhow!("missing `path`"))?;
                let resolved = resolve_workspace_path(&self.workspace_root, path);
                let key = resolved.display().to_string();
                if !self.buffers.contains_key(&key) {
                    self.open_buffer(path)?;
                }
                let buf = self
                    .buffers
                    .get(&key)
                    .ok_or_else(|| anyhow::anyhow!("missing buffer"))?;
                let start = args.get("start_line").and_then(Value::as_u64).unwrap_or(1) as usize;
                let end = args.get("end_line").and_then(Value::as_u64).unwrap_or(200) as usize;
                let lines: Vec<String> = buf
                    .content
                    .lines()
                    .enumerate()
                    .filter(|(idx, _)| idx + 1 >= start && *idx < end)
                    .map(|(idx, text)| format!("{}: {text}", idx + 1))
                    .collect();
                Ok(json!({
                    "path": buf.path,
                    "total_lines": buf.line_count,
                    "lines": lines,
                }))
            }
            "zed_headless_edit" => {
                let path = args
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow::anyhow!("missing `path`"))?;
                let content = args
                    .get("content")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow::anyhow!("missing `content`"))?;
                let write_disk = args.get("save").and_then(Value::as_bool).unwrap_or(false);
                let buf = self.apply_edit(path, content.to_owned())?.clone();
                #[cfg(not(target_family = "wasm"))]
                if write_disk {
                    fs::write(&buf.path, &buf.content)?;
                }
                #[cfg(target_family = "wasm")]
                let _ = write_disk;
                Ok(json!({
                    "path": buf.path,
                    "line_count": buf.line_count,
                    "byte_len": buf.byte_len,
                    "saved": write_disk,
                }))
            }
            "zed_headless_parse_ast" | "zed_headless_symbols" => {
                let lang_hint = args.get("language").and_then(Value::as_str);
                let path_opt = args.get("path").and_then(Value::as_str);
                let source = if let Some(src) = args.get("source").and_then(Value::as_str) {
                    src.to_owned()
                } else if let Some(p) = path_opt {
                    self.open_buffer(p)?.content.clone()
                } else {
                    anyhow::bail!("provide `source` or `path`")
                };
                let lang = lang_hint
                    .and_then(SyntaxLanguage::from_name_or_path)
                    .or_else(|| path_opt.and_then(SyntaxLanguage::from_name_or_path))
                    .unwrap_or(SyntaxLanguage::Rust);
                Ok(serde_json::to_value(analyze_with_tree_sitter(lang, &source)?)?)
            }
            other => anyhow::bail!("unknown headless zed operation '{other}'"),
        }
    }
}

fn resolve_workspace_path(root: &Path, candidate: &str) -> PathBuf {
    let p = Path::new(candidate);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        root.join(p)
    }
}

/// Parses source code with the exact workspace `tree-sitter` grammars and extracts functions,
/// structs, enums, traits, classes, and syntax error counts.
#[cfg(not(target_family = "wasm"))]
pub fn analyze_with_tree_sitter(
    language: SyntaxLanguage,
    source: &str,
) -> anyhow::Result<TreeSitterReport> {
    let mut parser = tree_sitter::Parser::new();
    let ts_lang: tree_sitter::Language = match language {
        SyntaxLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
        SyntaxLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        SyntaxLanguage::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
        SyntaxLanguage::Python => tree_sitter_python::LANGUAGE.into(),
        SyntaxLanguage::Json => tree_sitter_json::LANGUAGE.into(),
    };
    parser.set_language(&ts_lang)?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("tree-sitter parser returned no syntax tree"))?;

    let root = tree.root_node();
    let mut symbols = Vec::new();
    let mut syntax_errors = 0usize;
    collect_ts_nodes(root, source.as_bytes(), &mut symbols, &mut syntax_errors);

    Ok(TreeSitterReport {
        language,
        byte_len: source.len(),
        line_count: source.lines().count(),
        syntax_errors,
        symbols,
        root_kind: root.kind().to_owned(),
    })
}

#[cfg(target_family = "wasm")]
pub fn analyze_with_tree_sitter(
    language: SyntaxLanguage,
    source: &str,
) -> anyhow::Result<TreeSitterReport> {
    Ok(TreeSitterReport {
        language,
        byte_len: source.len(),
        line_count: source.lines().count(),
        syntax_errors: 0,
        symbols: Vec::new(),
        root_kind: "wasm_stub".to_owned(),
    })
}

#[cfg(not(target_family = "wasm"))]
fn collect_ts_nodes(
    node: tree_sitter::Node<'_>,
    src: &[u8],
    symbols: &mut Vec<TreeSitterSymbol>,
    syntax_errors: &mut usize,
) {
    if node.is_error() || node.is_missing() {
        *syntax_errors += 1;
    }
    let kind = node.kind();
    let is_decl = matches!(
        kind,
        "function_item"
            | "struct_item"
            | "enum_item"
            | "trait_item"
            | "impl_item"
            | "mod_item"
            | "function_declaration"
            | "class_declaration"
            | "interface_declaration"
            | "type_alias_declaration"
            | "function_definition"
            | "class_definition"
    );
    if is_decl {
        let name = node
            .child_by_field_name("name")
            .or_else(|| node.child_by_field_name("type"))
            .and_then(|n| n.utf8_text(src).ok())
            .unwrap_or("<anonymous>")
            .to_owned();
        symbols.push(TreeSitterSymbol {
            kind: kind.to_owned(),
            name,
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
        });
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_ts_nodes(child, src, symbols, syntax_errors);
    }
}

/// Tool descriptors for absorbed `agent-browser` + Headless Zed + Tree-sitter MCP surface.
#[must_use]
pub fn builtin_headless_and_browser_tools() -> Vec<Value> {
    vec![
        json!({"name": "zed_headless_status", "description": "Inspect headless Zed workspace buffers, OS capabilities, and Tree-sitter readiness.", "inputSchema": {"type": "object", "properties": {}, "additionalProperties": true}}),
        json!({"name": "zed_headless_open", "description": "Open a source file into the headless Zed buffer table.", "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"], "additionalProperties": true}}),
        json!({"name": "zed_headless_read", "description": "Read numbered line ranges from a headless Zed buffer.", "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}, "start_line": {"type": "integer"}, "end_line": {"type": "integer"}}, "required": ["path"], "additionalProperties": true}}),
        json!({"name": "zed_headless_edit", "description": "Update a headless Zed buffer and optionally persist it to disk.", "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}, "content": {"type": "string"}, "save": {"type": "boolean"}}, "required": ["path", "content"], "additionalProperties": true}}),
        json!({"name": "zed_headless_parse_ast", "description": "Parse source or file using workspace Tree-sitter grammars (Rust/TS/TSX/Python/JSON) and report syntax errors + AST symbols.", "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}, "source": {"type": "string"}, "language": {"type": "string"}}, "additionalProperties": true}}),
        json!({"name": "zed_headless_symbols", "description": "Extract top-level functions, structs, enums, traits, and classes via Tree-sitter.", "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}, "source": {"type": "string"}, "language": {"type": "string"}}, "additionalProperties": true}}),
        json!({"name": "browser_status", "description": "Inspect absorbed agent-browser controller state and discovered Chrome/Chromium binary.", "inputSchema": {"type": "object", "properties": {}, "additionalProperties": true}}),
        json!({"name": "browser_validate_url", "description": "Check a target URL against agent-browser SSRF / private IP / IMDS / scheme policy.", "inputSchema": {"type": "object", "properties": {"url": {"type": "string"}}, "required": ["url"], "additionalProperties": true}}),
        json!({"name": "browser_connect_cdp", "description": "Attach to a local or remote Chrome DevTools Protocol /json/version WebSocket.", "inputSchema": {"type": "object", "properties": {"host": {"type": "string"}, "port": {"type": "integer"}}, "additionalProperties": true}}),
        json!({"name": "browser_open", "description": "Validate and navigate an attached CDP tab to a URL.", "inputSchema": {"type": "object", "properties": {"url": {"type": "string"}}, "required": ["url"], "additionalProperties": true}}),
        json!({"name": "browser_snapshot", "description": "Capture CDP Accessibility.getFullAXTree snapshot from active browser page.", "inputSchema": {"type": "object", "properties": {}, "additionalProperties": true}}),
        json!({"name": "browser_eval", "description": "Evaluate JavaScript inside active CDP page context.", "inputSchema": {"type": "object", "properties": {"expression": {"type": "string"}}, "required": ["expression"], "additionalProperties": true}}),
        json!({"name": "browser_screenshot", "description": "Capture PNG screenshot via CDP Page.captureScreenshot.", "inputSchema": {"type": "object", "properties": {}, "additionalProperties": true}}),
    ]
}

/// Executes stdio JSON-RPC 2.0 MCP server for `aphrody --headless --mcp`.
#[cfg(not(target_family = "wasm"))]
pub fn run_headless_mcp_stdio(workspace_root: Option<PathBuf>) -> anyhow::Result<()> {
    let mut headless =
        HeadlessZedController::new(workspace_root.unwrap_or_else(|| PathBuf::from(".")));
    let mut pilot = PilotEngine::new();
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(reply) = dispatch_headless_mcp_line(&mut headless, &mut pilot, &line) {
            writeln!(stdout, "{reply}")?;
            stdout.flush()?;
        }
    }
    Ok(())
}

/// Dispatches one JSON-RPC 2.0 MCP request line against [`HeadlessZedController`] + [`PilotEngine`].
#[must_use]
pub fn dispatch_headless_mcp_line(
    headless: &mut HeadlessZedController,
    pilot: &mut PilotEngine,
    line: &str,
) -> Option<String> {
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

    let result: Result<Value, (i32, String)> = match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2025-03-26",
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": format!("{}-headless", brand::APP_SLUG), "version": env!("CARGO_PKG_VERSION") }
        })),
        "notifications/initialized" | "notifications/cancelled" => return None,
        "tools/list" => {
            let mut tools = builtin_headless_and_browser_tools();
            for pt in crate::builtin_pilot_tools() {
                tools.push(serde_json::to_value(pt).unwrap_or_else(|_| json!({})));
            }
            Ok(json!({ "tools": tools }))
        }
        "tools/call" => {
            let tool_name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
            let res: Result<Value, String> = if tool_name.starts_with("zed_headless_") {
                headless
                    .execute_tool(tool_name, &args)
                    .map_err(|e| e.to_string())
            } else {
                pilot
                    .execute_tool(tool_name, &args, &CommandContext::default())
                    .map_err(|e: PilotError| e.to_string())
            };
            match res {
                Ok(structured) => Ok(json!({
                    "content": [{ "type": "text", "text": serde_json::to_string_pretty(&structured).unwrap_or_default() }],
                    "structuredContent": structured,
                    "isError": false
                })),
                Err(msg) => Ok(json!({
                    "content": [{ "type": "text", "text": msg }],
                    "isError": true
                })),
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_sitter_parses_rust_symbols_and_errors_headlessly() {
        let src = r#"
pub struct StudioNode {
    pub id: u32,
}
pub fn boot_cosmic() -> u32 {
    42
}
"#;
        let report = analyze_with_tree_sitter(SyntaxLanguage::Rust, src).expect("parse rust");
        assert_eq!(report.syntax_errors, 0);
        assert!(report.symbols.iter().any(|s| s.name == "StudioNode"));
        assert!(report.symbols.iter().any(|s| s.name == "boot_cosmic"));

        let broken = analyze_with_tree_sitter(SyntaxLanguage::Rust, "fn broken(((").expect("parse err");
        assert!(broken.syntax_errors > 0);
    }

    #[test]
    fn headless_mcp_roundtrip_drives_zed_tree_sitter_and_browser_policy() {
        let mut headless = HeadlessZedController::default();
        let mut pilot = PilotEngine::new();
        let reply = dispatch_headless_mcp_line(
            &mut headless,
            &mut pilot,
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"zed_headless_parse_ast","arguments":{"language":"rust","source":"pub fn hello() {}"}}}"#,
        )
        .expect("mcp reply");
        let v: Value = serde_json::from_str(&reply).expect("json");
        assert_eq!(v["result"]["structuredContent"]["symbols"][0]["name"], "hello");

        let browser_reply = dispatch_headless_mcp_line(
            &mut headless,
            &mut pilot,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"browser_validate_url","arguments":{"url":"http://169.254.169.254/latest"}}}"#,
        )
        .expect("browser reply");
        let bv: Value = serde_json::from_str(&browser_reply).expect("json");
        assert_eq!(bv["result"]["structuredContent"]["allowed"], false);
    }
}
