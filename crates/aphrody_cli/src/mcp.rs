use std::io::{self, BufRead as _, Write as _};
use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use aphrody_design::design_md::{
    export_css_variables, export_shadcn_components_json, export_shadcn_tailwind_css,
    export_w3c_tokens_json, export_wgsl_theme_struct, lint_design_md,
    load_bundled_root_design_md, parse_design_md,
};
use aphrody_design::http_mirror::fetch_or_fallback_article;
use aphrody_design::modern_web_guidance;
use aphrody_design::screen_capture::capture_stitch_screen_png;
use aphrody_design::stitch_client::{StitchClient, stitch_screen_to_nsh_tsx};
use aphrody_design::web_standards::{
    audit_html_standards, audit_project_standards, starter_primitive_summaries,
};
use serde_json::{Value, json};

use crate::{create_instance, doctor, shader, templates, wasm_toolchain};

const WEBASSEMBLY_ECOSYSTEM_TOML: &str =
    include_str!("../../../assets/os/webassembly_ecosystem.toml");
const SUBMODULE_PARITY_MATRIX_TOML: &str =
    include_str!("../../../assets/os/submodule_parity_matrix.toml");

#[derive(Debug, Args)]
pub(crate) struct McpArgs {}

pub(crate) fn builtin_tools() -> Vec<Value> {
    vec![
        json!({ "name": "wgpu_list_adapters", "description": "Enumerate GPU adapters on all configured wgpu backends.", "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false} }),
        json!({ "name": "wgpu_validate_wgsl", "description": "Parse and validate WGSL source using Naga v30.", "inputSchema": {"type": "object", "properties": {"source": {"type": "string"}}, "required": ["source"], "additionalProperties": false} }),
        json!({ "name": "wgpu_transpile_shader", "description": "Cross-compile WGSL source via Naga to wgsl/hlsl/msl/glsl/webgl2/spv.", "inputSchema": {"type": "object", "properties": {"source": {"type": "string"}, "target": {"type": "string"}}, "required": ["source", "target"], "additionalProperties": false} }),
        json!({ "name": "wgpu_run_compute", "description": "Inspect/validate a compute WGSL kernel against adapter limits.", "inputSchema": {"type": "object", "properties": {"source": {"type": "string"}}, "required": ["source"], "additionalProperties": false} }),
        json!({ "name": "wgpu_render_frame", "description": "Validate and prepare fullscreen shader render targets.", "inputSchema": {"type": "object", "properties": {"source": {"type": "string"}}, "required": ["source"], "additionalProperties": false} }),
        json!({ "name": "wgpu_bench", "description": "Report host GPU backend readiness for headless compute/render benchmarks.", "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false} }),
        json!({ "name": "wasm_doctor", "description": "Probe Rust wasm targets, Wasm/GPU/Shell/HTTP CLIs, and wgpu adapters.", "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false} }),
        json!({ "name": "wasm_inspect", "description": "Inspect `.wasm` binary sections, imports, exports, gzip size, and SHA-256.", "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"], "additionalProperties": false} }),
        json!({ "name": "wasm_validate", "description": "Validate a core `.wasm` or component binary with all WebAssembly features enabled.", "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"], "additionalProperties": false} }),
        json!({ "name": "wasm_optimize_lto", "description": "Run Binaryen `wasm-opt -Oz` pass on a `.wasm` file.", "inputSchema": {"type": "object", "properties": {"input": {"type": "string"}, "output": {"type": "string"}}, "required": ["input"], "additionalProperties": false} }),
        json!({ "name": "wasm_compile_aot", "description": "Compile a `.wasm` binary to `.cwasm` AOT native image with `wasmtime compile`.", "inputSchema": {"type": "object", "properties": {"input": {"type": "string"}, "output": {"type": "string"}}, "required": ["input", "output"], "additionalProperties": false} }),
        json!({ "name": "wasm_strip_sections", "description": "Strip custom sections from a core `.wasm` file using `wasm-encoder`.", "inputSchema": {"type": "object", "properties": {"input": {"type": "string"}, "output": {"type": "string"}}, "required": ["input", "output"], "additionalProperties": false} }),
        json!({ "name": "wasm_scaffold_app", "description": "Scaffold a Wasm/WebGPU + Google Design template into a target directory.", "inputSchema": {"type": "object", "properties": {"template": {"type": "string"}, "dir": {"type": "string"}, "name": {"type": "string"}}, "required": ["template", "dir"], "additionalProperties": false} }),
        json!({ "name": "wasm_list_templates", "description": "List canonical templates under templates/wasm-wgpu/.", "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false} }),
        json!({ "name": "wasm_ecosystem_query", "description": "Query synced orgs/WebAssembly repositories sorted by stars.", "inputSchema": {"type": "object", "properties": {"query": {"type": "string"}}, "additionalProperties": false} }),
        json!({ "name": "design_tokens_query", "description": "Return authoritative https://design.google/ DESIGN.md tokens in CSS, WGSL, or W3C JSON.", "inputSchema": {"type": "object", "properties": {"format": {"type": "string"}}, "additionalProperties": false} }),
        json!({ "name": "design_google_mirror", "description": "Pure-Rust live/cached mirror of https://design.google/ stories & sitemap metadata.", "inputSchema": {"type": "object", "properties": {"url": {"type": "string"}, "live": {"type": "boolean"}}, "required": ["url"], "additionalProperties": false} }),
        json!({ "name": "design_md_lint_export", "description": "Parse and WCAG-lint a DESIGN.md markdown file and export tokens.", "inputSchema": {"type": "object", "properties": {"markdown": {"type": "string"}}, "additionalProperties": false} }),
        json!({ "name": "stitch_screen_capture", "description": "Generate a DESIGN.md-constrained Stitch screen, NSH TSX view, and offscreen PNG preview.", "inputSchema": {"type": "object", "properties": {"prompt": {"type": "string"}, "output": {"type": "string"}}, "required": ["prompt", "output"], "additionalProperties": false} }),
        json!({ "name": "modern_web_guidance_audit", "description": "Audit HTML source or directory against GoogleChrome modern-web-guidance Baseline rules.", "inputSchema": {"type": "object", "properties": {"html": {"type": "string"}, "dir": {"type": "string"}}, "additionalProperties": false} }),
        json!({ "name": "cwv_lighthouse_audit", "description": "Score HTML or project directories across Core Web Vitals, Lighthouse (Perf/A11y/Best/SEO), W3C, Google Style, and Vercel UI guidelines.", "inputSchema": {"type": "object", "properties": {"html": {"type": "string"}, "dir": {"type": "string"}, "wasm_brotli_bytes": {"type": "integer"}}, "additionalProperties": false} }),
        json!({ "name": "shadcn_tailwind_export", "description": "Export shadcn/Base UI Tailwind v4 @theme CSS variables and components.json derived from https://design.google/ DESIGN.md.", "inputSchema": {"type": "object", "properties": {"format": {"type": "string"}}, "additionalProperties": false} }),
        json!({ "name": "gpui_starterpack_catalog", "description": "List the 16 canonical Base UI + shadcn + Tailwind v4 + GPUI/NSH Starter Pack primitives with WAI-ARIA roles and recipes.", "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false} }),
        json!({ "name": "submodule_parity_matrix", "description": "Query the fork submodule parity matrix comparing aphrody-core against kept vendor/{wgpu,taffy,accesskit,cosmic-text,egui,tauri/wry,craft-libs,craft-anything,mullvad,browser}.", "inputSchema": {"type": "object", "properties": {"filter": {"type": "string"}}, "additionalProperties": false} }),
        json!({ "name": "craft_expose_manifest", "description": "Expose all ArtCraft & Storytold apps across CLI, MCP, Web, Desktop, unified build and 22-target deploy.", "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false} }),
        json!({ "name": "cosmic_epoch_matrix", "description": "Expose all COSMIC Epoch + Pop!_OS subsystems across CLI, MCP, Web, Desktop, and 22 deploy targets.", "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false} }),
        json!({ "name": "installer_switch_de_plan", "description": "Plan live desktop environment switch across Aphrody Rust Userlands (GPUI, Iced COSMIC, egui+Bevy, WebGPU) and Arch DEs.", "inputSchema": {"type": "object", "properties": {"from": {"type": "string"}, "to": {"type": "string"}}, "required": ["to"], "additionalProperties": false} }),
        json!({ "name": "zed_headless_parse_ast", "description": "Parse Rust/TS/TSX/Python/JSON source using workspace Tree-sitter grammars headlessly.", "inputSchema": {"type": "object", "properties": {"language": {"type": "string"}, "source": {"type": "string"}}, "required": ["source"], "additionalProperties": false} }),
        json!({ "name": "browser_validate_url", "description": "Validate URL against absorbed agent-browser SSRF / non-public IP / IMDS security policy.", "inputSchema": {"type": "object", "properties": {"url": {"type": "string"}}, "required": ["url"], "additionalProperties": false} }),
        json!({ "name": "aphrody_suite_status", "description": "Return the consolidated Aphrody fork capability matrix across all 12 pillars, Material 3 modes, and target surfaces.", "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false} }),
        json!({ "name": "google_auth_status", "description": "Inspect Google OAuth 2.0 / gcloud / ADC authentication readiness without leaking tokens.", "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false} }),
        json!({ "name": "google_sdk_parity", "description": "Return pure-Rust SDK parity matrix covering Google Kotlin, Go, Python, TypeScript, and Rust SDKs.", "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false} }),
        json!({ "name": "google_workspace_call", "description": "Invoke a Google Workspace Discovery API endpoint (drive, gmail, calendar, docs, sheets, slides, tasks, chat).", "inputSchema": {"type": "object", "properties": {"service": {"type": "string"}, "path": {"type": "string"}, "method": {"type": "string"}}, "required": ["service", "path"], "additionalProperties": false} }),
        json!({ "name": "yt_search", "description": "Headless pure-Rust YouTube InnerTube (WEB) search.", "inputSchema": {"type": "object", "properties": {"query": {"type": "string"}}, "required": ["query"], "additionalProperties": false} }),
        json!({ "name": "yt_video_info", "description": "Headless pure-Rust YouTube player metadata, adaptive streams, captions, and SponsorBlock segments.", "inputSchema": {"type": "object", "properties": {"video_id": {"type": "string"}}, "required": ["video_id"], "additionalProperties": false} }),
        json!({ "name": "ytm_search", "description": "Headless pure-Rust YouTube Music (WEB_REMIX) search.", "inputSchema": {"type": "object", "properties": {"query": {"type": "string"}, "filter": {"type": "string"}}, "required": ["query"], "additionalProperties": false} }),
        json!({ "name": "ytm_lyrics", "description": "Headless pure-Rust YouTube Music lyrics lookup by video_id.", "inputSchema": {"type": "object", "properties": {"video_id": {"type": "string"}}, "required": ["video_id"], "additionalProperties": false} }),
        json!({ "name": "media_capabilities", "description": "Return pure-Rust zero-FFmpeg multimedia capabilities and supported codecs.", "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false} }),
        json!({ "name": "media_validate_shaders", "description": "Validate all built-in media WGSL shaders (YUV->RGB, AgX HDR, scopes, motion transitions) with Naga v30.", "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false} }),
    ]
}

pub(crate) fn dispatch_json_rpc_line(line: &str) -> Option<String> {
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
            "serverInfo": { "name": brand::WGPU_CLI_BIN, "version": env!("CARGO_PKG_VERSION") }
        })),
        "notifications/initialized" | "notifications/cancelled" => return None,
        "tools/list" => Ok(json!({ "tools": builtin_tools() })),
        "tools/call" => call_tool(&params),
        other => Err((-32601, format!("unknown method '{other}'"))),
    };

    let id = id?;
    let payload = match result {
        Ok(res) => json!({ "jsonrpc": "2.0", "id": id, "result": res }),
        Err((code, msg)) => json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": msg } }),
    };
    Some(payload.to_string())
}

fn wrap_tool_ok(structured: Value) -> Result<Value, (i32, String)> {
    let text = serde_json::to_string_pretty(&structured).unwrap_or_else(|_| structured.to_string());
    Ok(json!({
        "content": [{ "type": "text", "text": text }],
        "structuredContent": structured,
        "isError": false
    }))
}

fn call_tool(params: &Value) -> Result<Value, (i32, String)> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or((-32602, "missing tool name".to_owned()))?;
    let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));

    let val = match name {
        "wgpu_list_adapters" => {
            let instance = create_instance(wgpu::Backends::all());
            let adapters: Vec<Value> = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()))
                .into_iter()
                .map(|a| {
                    let info = a.get_info();
                    json!({"name": info.name, "backend": format!("{:?}", info.backend), "device_type": format!("{:?}", info.device_type)})
                })
                .collect();
            json!({ "adapters": adapters })
        }
        "wgpu_validate_wgsl" | "wgpu_run_compute" | "wgpu_render_frame" => {
            let src = args.get("source").and_then(Value::as_str).ok_or((-32602, "missing source".to_owned()))?;
            match shader::parse_and_validate_wgsl(src, "mcp.wgsl") {
                Ok((module, _)) => {
                    let rep = shader::inspect_module(&module, "mcp.wgsl").map_err(|e| (-32000, e.to_string()))?;
                    serde_json::to_value(rep).map_err(|e| (-32000, e.to_string()))?
                }
                Err(diag) => json!({ "valid": false, "diagnostic": diag }),
            }
        }
        "wgpu_transpile_shader" => {
            let src = args.get("source").and_then(Value::as_str).ok_or((-32602, "missing source".to_owned()))?;
            let target_str = args.get("target").and_then(Value::as_str).unwrap_or("wgsl");
            let target = match target_str {
                "glsl" => shader::TranspileTarget::Glsl,
                "webgl2" => shader::TranspileTarget::Webgl2,
                "hlsl" => shader::TranspileTarget::Hlsl,
                "msl" => shader::TranspileTarget::Msl,
                "spv" => shader::TranspileTarget::Spv,
                _ => shader::TranspileTarget::Wgsl,
            };
            match shader::transpile_wgsl_source(src, "mcp.wgsl", target, None, None, false) {
                Ok(shader::TranspiledArtifact::Text(t)) => json!({ "target": target_str, "code": t }),
                Ok(shader::TranspiledArtifact::SpirvWords(w)) => json!({ "target": "spv", "words": w.len() }),
                Err(err) => return Err((-32001, err.to_string())),
            }
        }
        "wgpu_bench" | "wasm_doctor" => serde_json::to_value(doctor::collect_doctor_report()).map_err(|e| (-32000, e.to_string()))?,
        "wasm_inspect" => {
            let path = PathBuf::from(args.get("path").and_then(Value::as_str).ok_or((-32602, "missing path".to_owned()))?);
            let rep = wasm_toolchain::inspect_file(&path).map_err(|e| (-32001, e.to_string()))?;
            serde_json::to_value(rep).map_err(|e| (-32000, e.to_string()))?
        }
        "wasm_validate" => {
            let path = PathBuf::from(args.get("path").and_then(Value::as_str).ok_or((-32602, "missing path".to_owned()))?);
            let rep = wasm_toolchain::validate_file(&path).map_err(|e| (-32001, e.to_string()))?;
            serde_json::to_value(rep).map_err(|e| (-32000, e.to_string()))?
        }
        "wasm_optimize_lto" => {
            let input = PathBuf::from(args.get("input").and_then(Value::as_str).ok_or((-32602, "missing input".to_owned()))?);
            let output = args.get("output").and_then(Value::as_str).map(PathBuf::from).unwrap_or_else(|| input.clone());
            let (before, after) = wasm_toolchain::optimize_wasm_file(&input, &output, true, false).map_err(|e| (-32001, e.to_string()))?;
            json!({ "before_bytes": before, "after_bytes": after, "output": output.display().to_string() })
        }
        "wasm_compile_aot" => {
            let input = PathBuf::from(args.get("input").and_then(Value::as_str).ok_or((-32602, "missing input".to_owned()))?);
            let output = PathBuf::from(args.get("output").and_then(Value::as_str).ok_or((-32602, "missing output".to_owned()))?);
            wasm_toolchain::compile_aot_file(&input, &output).map_err(|e| (-32001, e.to_string()))?
        }
        "wasm_strip_sections" => {
            let input = PathBuf::from(args.get("input").and_then(Value::as_str).ok_or((-32602, "missing input".to_owned()))?);
            let output = PathBuf::from(args.get("output").and_then(Value::as_str).ok_or((-32602, "missing output".to_owned()))?);
            let (before, after) = wasm_toolchain::strip_custom_sections(&input, &output, false).map_err(|e| (-32001, e.to_string()))?;
            json!({ "before_bytes": before, "after_bytes": after })
        }
        "wasm_scaffold_app" => {
            let tpl = args.get("template").and_then(Value::as_str).ok_or((-32602, "missing template".to_owned()))?;
            let dir = PathBuf::from(args.get("dir").and_then(Value::as_str).ok_or((-32602, "missing dir".to_owned()))?);
            let name = args.get("name").and_then(Value::as_str);
            let written = templates::scaffold_template(tpl, &dir, name).map_err(|e| (-32001, e.to_string()))?;
            json!({ "written_files": written.len(), "dir": dir.display().to_string() })
        }
        "wasm_list_templates" => serde_json::to_value(templates::TEMPLATES).map_err(|e| (-32000, e.to_string()))?,
        "wasm_ecosystem_query" => {
            let query = args.get("query").and_then(Value::as_str).unwrap_or("").to_ascii_lowercase();
            let parsed: toml::Value = toml::from_str(WEBASSEMBLY_ECOSYSTEM_TOML).map_err(|e| (-32000, e.to_string()))?;
            let repos = parsed.get("repos").and_then(toml::Value::as_array).cloned().unwrap_or_default();
            let matched: Vec<Value> = repos.into_iter().filter_map(|r| {
                let name = r.get("name").and_then(toml::Value::as_str)?;
                let desc = r.get("description").and_then(toml::Value::as_str).unwrap_or("");
                if query.is_empty() || name.to_ascii_lowercase().contains(&query) || desc.to_ascii_lowercase().contains(&query) {
                    serde_json::to_value(r).ok()
                } else {
                    None
                }
            }).take(25).collect();
            json!({ "count": matched.len(), "repos": matched })
        }
        "design_tokens_query" => {
            let doc = load_bundled_root_design_md().map_err(|e| (-32000, e.to_string()))?;
            let fmt = args.get("format").and_then(Value::as_str).unwrap_or("css");
            match fmt {
                "wgsl" => json!({ "source_of_truth": doc.source_of_truth, "wgsl": export_wgsl_theme_struct(&doc) }),
                "json" | "w3c" => export_w3c_tokens_json(&doc),
                _ => json!({ "source_of_truth": doc.source_of_truth, "css": export_css_variables(&doc) }),
            }
        }
        "design_google_mirror" => {
            let url = args.get("url").and_then(Value::as_str).ok_or((-32602, "missing url".to_owned()))?;
            let live = args.get("live").and_then(Value::as_bool).unwrap_or(false);
            let article = fetch_or_fallback_article(url, None, live).map_err(|e| (-32001, e.to_string()))?;
            serde_json::to_value(article).map_err(|e| (-32000, e.to_string()))?
        }
        "design_md_lint_export" => {
            let doc = if let Some(md) = args.get("markdown").and_then(Value::as_str) {
                parse_design_md(md).map_err(|e| (-32001, e.to_string()))?
            } else {
                load_bundled_root_design_md().map_err(|e| (-32000, e.to_string()))?
            };
            let lint = lint_design_md(&doc);
            json!({ "lint": lint, "css": export_css_variables(&doc) })
        }
        "stitch_screen_capture" => {
            let prompt = args.get("prompt").and_then(Value::as_str).ok_or((-32602, "missing prompt".to_owned()))?;
            let output = PathBuf::from(args.get("output").and_then(Value::as_str).ok_or((-32602, "missing output".to_owned()))?);
            let doc = load_bundled_root_design_md().map_err(|e| (-32000, e.to_string()))?;
            let screen = StitchClient::default().synthesize_screen(prompt, "DESKTOP", &doc, false).map_err(|e| (-32001, e.to_string()))?;
            capture_stitch_screen_png(&doc, &screen, 960, 540, &output).map_err(|e| (-32001, e.to_string()))?;
            json!({
                "stitch_id": screen.stitch_id,
                "png_path": output.display().to_string(),
                "tsx": stitch_screen_to_nsh_tsx(&screen),
            })
        }
        "modern_web_guidance_audit" => {
            if let Some(html) = args.get("html").and_then(Value::as_str) {
                let findings = modern_web_guidance::audit_html_source("inline.html", html);
                json!({ "findings": findings, "ok": !findings.iter().any(|f| f.severity == "error") })
            } else {
                let dir = PathBuf::from(args.get("dir").and_then(Value::as_str).unwrap_or("."));
                let report = modern_web_guidance::audit_directory(&dir).map_err(|e| (-32001, e.to_string()))?;
                serde_json::to_value(report).map_err(|e| (-32000, e.to_string()))?
            }
        }
        "cwv_lighthouse_audit" => {
            let doc = load_bundled_root_design_md().map_err(|e| (-32000, e.to_string()))?;
            if let Some(html) = args.get("html").and_then(Value::as_str) {
                let brotli = args.get("wasm_brotli_bytes").and_then(Value::as_u64).map(|n| n as usize);
                let report = audit_html_standards("inline.html", html, &doc, brotli);
                serde_json::to_value(report).map_err(|e| (-32000, e.to_string()))?
            } else {
                let dir = PathBuf::from(args.get("dir").and_then(Value::as_str).unwrap_or("templates/wasm-wgpu"));
                let report = audit_project_standards(&dir, &doc).map_err(|e| (-32001, e.to_string()))?;
                serde_json::to_value(report).map_err(|e| (-32000, e.to_string()))?
            }
        }
        "shadcn_tailwind_export" => {
            let doc = load_bundled_root_design_md().map_err(|e| (-32000, e.to_string()))?;
            let fmt = args.get("format").and_then(Value::as_str).unwrap_or("all");
            match fmt {
                "css" | "tailwind" => json!({ "css": export_shadcn_tailwind_css(&doc) }),
                "components-json" | "json" => export_shadcn_components_json(&doc),
                _ => json!({
                    "css": export_shadcn_tailwind_css(&doc),
                    "components_json": export_shadcn_components_json(&doc),
                }),
            }
        }
        "gpui_starterpack_catalog" => {
            let items = starter_primitive_summaries();
            json!({ "count": items.len(), "primitives": items })
        }
        "submodule_parity_matrix" => {
            let parsed: toml::Value = toml::from_str(SUBMODULE_PARITY_MATRIX_TOML).map_err(|e| (-32000, e.to_string()))?;
            let filter = args.get("filter").and_then(Value::as_str).unwrap_or("").to_ascii_lowercase();
            let frameworks = parsed.get("frameworks").and_then(toml::Value::as_array).cloned().unwrap_or_default();
            let filtered: Vec<Value> = frameworks
                .into_iter()
                .filter_map(|row| {
                    let id = row.get("id").and_then(toml::Value::as_str).unwrap_or("");
                    let label = row.get("label").and_then(toml::Value::as_str).unwrap_or("");
                    if filter.is_empty() || id.to_ascii_lowercase().contains(&filter) || label.to_ascii_lowercase().contains(&filter) {
                        serde_json::to_value(row).ok()
                    } else {
                        None
                    }
                })
                .collect();
            json!({
                "design_source_of_truth": parsed.get("design_source_of_truth").and_then(toml::Value::as_str).unwrap_or("https://design.google/"),
                "count": filtered.len(),
                "frameworks": filtered,
            })
        }
        "craft_expose_manifest" => {
            let catalog = aphrody_craft::CraftCatalog::load_with_user_override()
                .map_err(|e| (-32001, e.to_string()))?;
            let manifest = aphrody_craft::CraftExposureManifest::from_catalog(&catalog);
            serde_json::to_value(manifest).map_err(|e| (-32000, e.to_string()))?
        }
        "cosmic_epoch_matrix" => json!({
            "summary": cosmic_gpui::inspect_cosmic_stack(),
            "components": cosmic_gpui::COSMIC_EPOCH_COMPONENTS,
        }),
        "installer_switch_de_plan" => {
            let from = args.get("from").and_then(Value::as_str).unwrap_or("aphrody-gpui");
            let to = args
                .get("to")
                .and_then(Value::as_str)
                .ok_or((-32602, "missing `to`".to_owned()))?;
            let catalog = aphrody_installer::InstallerProfileCatalog::bundled()
                .map_err(|e| (-32001, e.to_string()))?;
            let plan = aphrody_installer::plan_desktop_switch(from, to, &catalog)
                .map_err(|e| (-32602, e.to_string()))?;
            serde_json::to_value(plan).map_err(|e| (-32000, e.to_string()))?
        }
        op @ ("zed_headless_parse_ast" | "browser_validate_url") => {
            let mut ctrl = aphrody_pilot::HeadlessZedController::default();
            if op == "browser_validate_url" {
                let mut pilot = aphrody_pilot::PilotEngine::new();
                pilot
                    .execute_tool(op, &args, &aphrody_pilot::CommandContext::default())
                    .map_err(|e| (-32002, e.to_string()))?
            } else {
                ctrl.execute_tool(op, &args)
                    .map_err(|e| (-32002, e.to_string()))?
            }
        }
        "aphrody_suite_status" => serde_json::to_value(aphrody::AphrodySuiteProfile::current())
            .map_err(|e| (-32000, e.to_string()))?,
        "google_auth_status" => serde_json::to_value(aphrody_google::GoogleAuthManager::default().status())
            .map_err(|e| (-32000, e.to_string()))?,
        "google_sdk_parity" => serde_json::to_value(aphrody_google::sdk_parity_matrix())
            .map_err(|e| (-32000, e.to_string()))?,
        "google_workspace_call" => {
            let service = args
                .get("service")
                .and_then(Value::as_str)
                .ok_or((-32602, "missing 'service' argument".to_owned()))?;
            let path = args
                .get("path")
                .and_then(Value::as_str)
                .ok_or((-32602, "missing 'path' argument".to_owned()))?;
            let method = args
                .get("method")
                .and_then(Value::as_str)
                .unwrap_or("GET");
            let client =
                aphrody_google::GoogleWorkspaceClient::new(aphrody_google::GoogleAuthManager::default());
            client
                .call(&aphrody_google::WorkspaceDiscoveryRequest {
                    service: service.to_owned(),
                    method: method.to_owned(),
                    path: path.to_owned(),
                    query: Vec::new(),
                    body: None,
                })
                .map_err(|e| (-32002, e.to_string()))?
        }
        "yt_search" => {
            let query = args
                .get("query")
                .and_then(Value::as_str)
                .ok_or((-32602, "missing 'query' argument".to_owned()))?;
            let client = aphrody_google::InnerTubeClient::discover(
                aphrody_google::InnerTubeClientProfile::Web,
            )
            .map_err(|e| (-32002, e.to_string()))?;
            let items = client
                .search(query, None)
                .map_err(|e| (-32002, e.to_string()))?;
            serde_json::to_value(items).map_err(|e| (-32000, e.to_string()))?
        }
        "yt_video_info" => {
            let video_id = args
                .get("video_id")
                .and_then(Value::as_str)
                .ok_or((-32602, "missing 'video_id' argument".to_owned()))?;
            let client = aphrody_google::InnerTubeClient::discover(
                aphrody_google::InnerTubeClientProfile::Web,
            )
            .map_err(|e| (-32002, e.to_string()))?;
            let info = client
                .player(video_id)
                .map_err(|e| (-32002, e.to_string()))?;
            let segments = aphrody_google::InnerTubeClient::sponsorblock_segments(video_id)
                .unwrap_or_default();
            json!({ "details": info, "sponsorblock": segments })
        }
        "ytm_search" => {
            let query = args
                .get("query")
                .and_then(Value::as_str)
                .ok_or((-32602, "missing 'query' argument".to_owned()))?;
            let filter = args.get("filter").and_then(Value::as_str).and_then(|f| {
                match f.to_ascii_lowercase().as_str() {
                    "songs" => Some(aphrody_google::YtMusicSearchFilter::Songs),
                    "videos" => Some(aphrody_google::YtMusicSearchFilter::Videos),
                    "albums" => Some(aphrody_google::YtMusicSearchFilter::Albums),
                    "artists" => Some(aphrody_google::YtMusicSearchFilter::Artists),
                    "playlists" => Some(aphrody_google::YtMusicSearchFilter::Playlists),
                    _ => None,
                }
            });
            let client = aphrody_google::YouTubeMusicClient::discover()
                .map_err(|e| (-32002, e.to_string()))?;
            let tracks = client
                .search(query, filter)
                .map_err(|e| (-32002, e.to_string()))?;
            serde_json::to_value(tracks).map_err(|e| (-32000, e.to_string()))?
        }
        "ytm_lyrics" => {
            let video_id = args
                .get("video_id")
                .and_then(Value::as_str)
                .ok_or((-32602, "missing 'video_id' argument".to_owned()))?;
            let client = aphrody_google::YouTubeMusicClient::discover()
                .map_err(|e| (-32002, e.to_string()))?;
            let lyrics = client
                .lyrics_for_video(video_id)
                .map_err(|e| (-32002, e.to_string()))?;
            serde_json::to_value(lyrics).map_err(|e| (-32000, e.to_string()))?
        }
        "media_capabilities" => serde_json::to_value(aphrody_media::capabilities())
            .map_err(|e| (-32000, e.to_string()))?,
        "media_validate_shaders" => {
            let report = aphrody_media::validate_all_media_shaders()
                .map_err(|e| (-32002, e.to_string()))?;
            serde_json::to_value(report).map_err(|e| (-32000, e.to_string()))?
        }
        unknown => return Err((-32601, format!("unknown tool '{unknown}'"))),
    };

    wrap_tool_ok(val)
}

pub(crate) fn run_stdio_server(_args: McpArgs) -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(reply) = dispatch_json_rpc_line(&line) {
            writeln!(out, "{reply}")?;
            out.flush()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use anyhow::Context as _;

    use super::*;

    #[test]
    fn mcp_lists_all_24_tools_and_dispatches_design_and_wasm_queries() -> Result<()> {
        let init_raw = dispatch_json_rpc_line(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
        )
        .context("initialize should respond")?;
        assert!(init_raw.contains("2025-03-26"));

        let list_raw = dispatch_json_rpc_line(
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
        )
        .context("tools/list should respond")?;
        let list_json: Value = serde_json::from_str(&list_raw)?;
        let tools = list_json["result"]["tools"]
            .as_array()
            .context("tools array")?;
        assert!(tools.len() >= 30);

        let suite_res = dispatch_json_rpc_line(
            r#"{"jsonrpc":"2.0","id":30,"method":"tools/call","params":{"name":"aphrody_suite_status","arguments":{}}}"#,
        )
        .context("aphrody_suite_status should respond")?;
        assert!(suite_res.contains("material3-kit"));
        assert!(suite_res.contains("cosmic-os"));

        let wgsl_res = dispatch_json_rpc_line(
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"wgpu_validate_wgsl","arguments":{"source":"@compute @workgroup_size(1) fn main() {}"}}}"#,
        )
        .context("wgpu_validate_wgsl should respond")?;
        assert!(wgsl_res.contains("\"valid\": true") || wgsl_res.contains("\"valid\":true"));

        let eco_res = dispatch_json_rpc_line(
            r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"wasm_ecosystem_query","arguments":{"query":"binaryen"}}}"#,
        )
        .context("wasm_ecosystem_query should respond")?;
        assert!(eco_res.contains("binaryen"));

        let tokens_res = dispatch_json_rpc_line(
            r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"design_tokens_query","arguments":{"format":"css"}}}"#,
        )
        .context("design_tokens_query should respond")?;
        assert!(tokens_res.contains("https://design.google/"));

        let tailwind_res = dispatch_json_rpc_line(
            r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"shadcn_tailwind_export","arguments":{"format":"css"}}}"#,
        )
        .context("shadcn_tailwind_export should respond")?;
        assert!(tailwind_res.contains("@theme inline"));

        let catalog_res = dispatch_json_rpc_line(
            r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"gpui_starterpack_catalog","arguments":{}}}"#,
        )
        .context("gpui_starterpack_catalog should respond")?;
        assert!(catalog_res.contains("StarterButton"));

        let parity_res = dispatch_json_rpc_line(
            r#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"submodule_parity_matrix","arguments":{"filter":"aphrody"}}}"#,
        )
        .context("submodule_parity_matrix should respond")?;
        assert!(parity_res.contains("aphrody-core"));

        let cosmic_res = dispatch_json_rpc_line(
            r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"cosmic_epoch_matrix","arguments":{}}}"#,
        )
        .context("cosmic_epoch_matrix should respond")?;
        assert!(cosmic_res.contains("cosmic-comp"));

        let craft_res = dispatch_json_rpc_line(
            r#"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"craft_expose_manifest","arguments":{}}}"#,
        )
        .context("craft_expose_manifest should respond")?;
        assert!(craft_res.contains("photocraft"));

        let switch_res = dispatch_json_rpc_line(
            r#"{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"installer_switch_de_plan","arguments":{"from":"aphrody-gpui","to":"iced-cosmic"}}}"#,
        )
        .context("installer_switch_de_plan should respond")?;
        assert!(switch_res.contains("cosmic-session"));

        let parity_google = dispatch_json_rpc_line(
            r#"{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"google_sdk_parity","arguments":{}}}"#,
        )
        .context("google_sdk_parity should respond")?;
        assert!(parity_google.contains("go-genai"));
        assert!(parity_google.contains("java-genai"));

        let media_shaders = dispatch_json_rpc_line(
            r#"{"jsonrpc":"2.0","id":13,"method":"tools/call","params":{"name":"media_validate_shaders","arguments":{}}}"#,
        )
        .context("media_validate_shaders should respond")?;
        assert!(media_shaders.contains("fs_yuv_to_rgba"));
        Ok(())
    }
}
