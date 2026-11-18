//! A stdio MCP server (newline-delimited JSON-RPC 2.0) over a set of browse tools and one browser.

use std::collections::HashMap;
use std::sync::Arc;

use browse_core::{Tool, ToolContext, ToolOutput};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

const PROTOCOLS: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

/// Serves `tools` over stdin/stdout until stdin closes. Calls run concurrently; a `notifications/cancelled` stops one.
pub async fn serve_stdio(tools: Vec<Arc<dyn Tool>>, ctx: ToolContext, name: &str, version: &str) -> anyhow::Result<()> {
    serve(tokio::io::stdin(), tokio::io::stdout(), tools, ctx, name, version).await
}

pub async fn serve<R, W>(input: R, mut output: W, tools: Vec<Arc<dyn Tool>>, ctx: ToolContext, name: &str, version: &str) -> anyhow::Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let tools: Arc<HashMap<String, Arc<dyn Tool>>> = Arc::new(tools.into_iter().map(|t| (t.spec().name, t)).collect());
    let (tx, mut rx) = mpsc::unbounded_channel::<Value>();
    let writer = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            let mut line = message.to_string();
            line.push('\n');
            if output.write_all(line.as_bytes()).await.is_err() || output.flush().await.is_err() {
                break;
            }
        }
    });
    let mut running: HashMap<String, tokio::task::AbortHandle> = HashMap::new();
    let mut lines = BufReader::new(input).lines();
    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            let _ = tx.send(json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": "Parse error" } }));
            continue;
        };
        let method = message.get("method").and_then(Value::as_str).unwrap_or_default().to_string();
        let params = message.get("params").cloned().unwrap_or(Value::Null);
        let Some(id) = message.get("id").cloned() else {
            if method == "notifications/cancelled" {
                if let Some(handle) = params.get("requestId").and_then(|r| running.remove(&r.to_string())) {
                    handle.abort();
                }
            }
            continue;
        };
        let reply = |result: Value| json!({ "jsonrpc": "2.0", "id": id, "result": result });
        match method.as_str() {
            "initialize" => {
                let asked = params.get("protocolVersion").and_then(Value::as_str).unwrap_or_default();
                let protocol = PROTOCOLS.iter().find(|v| **v == asked).unwrap_or(&PROTOCOLS[0]);
                let _ = tx.send(reply(json!({
                    "protocolVersion": protocol,
                    "capabilities": { "tools": { "listChanged": false } },
                    "serverInfo": { "name": name, "version": version },
                })));
            }
            "ping" => {
                let _ = tx.send(reply(json!({})));
            }
            "tools/list" => {
                let mut specs: Vec<_> = tools.values().map(|t| t.spec()).collect();
                specs.sort_by(|a, b| a.name.cmp(&b.name));
                let _ = tx.send(reply(json!({ "tools": specs })));
            }
            "resources/list" => {
                let _ = tx.send(reply(json!({ "resources": [] })));
            }
            "prompts/list" => {
                let _ = tx.send(reply(json!({ "prompts": [] })));
            }
            "tools/call" => {
                let tool_name = params.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
                let arguments = params.get("arguments").cloned().filter(Value::is_object).unwrap_or_else(|| json!({}));
                let (tools, ctx, tx2, key) = (tools.clone(), ctx.clone(), tx.clone(), id.to_string());
                let id2 = id.clone();
                let handle = tokio::spawn(async move {
                    let output = match tools.get(&tool_name) {
                        Some(tool) => tool.call(ctx, arguments).await.unwrap_or_else(|e| ToolOutput::error(format!("{tool_name} failed: {e:#}"))),
                        None => ToolOutput::error(format!("unknown tool {tool_name}")),
                    };
                    let _ = tx2.send(json!({ "jsonrpc": "2.0", "id": id2, "result": serde_json::to_value(output).unwrap_or(Value::Null) }));
                });
                running.retain(|_, h| !h.is_finished());
                running.insert(key, handle.abort_handle());
            }
            other => {
                let _ = tx.send(json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32601, "message": format!("Method not found: {other}") } }));
            }
        }
    }
    // stdin closed: give calls in flight a moment, then finish.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    drop(tx);
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), writer).await;
    Ok(())
}
