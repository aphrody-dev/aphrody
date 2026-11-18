//! HTTP service over a router, compatible with OpenAI (`POST /v1/chat/completions`, `GET /v1/models`, and the
//! pass-through `POST /v1/embeddings`, `/v1/images/generations`, `/v1/audio/speech`, `/v1/audio/transcriptions`)
//! and Anthropic (`POST /v1/messages`) clients, plus `GET /health`. One thread per request.

use crate::policy::Kind;
use crate::{Body, Router};
use serde_json::{json, Value};
use std::io::Read;
use std::sync::Arc;
use tiny_http::{Header, Method, Response, Server};

fn reply(req: tiny_http::Request, status: u16, body: &Value) {
    let h = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).expect("static header");
    let _ = req.respond(Response::from_string(body.to_string()).with_status_code(status).with_header(h));
}

/// Tokens accepted on `/v1/*` (AI_ROUTER_TOKENS, comma-separated). Empty: no authentication, loopback only.
fn tokens() -> Vec<String> {
    std::env::var("AI_ROUTER_TOKENS").unwrap_or_default().split(',').map(str::trim).filter(|t| !t.is_empty()).map(str::to_owned).collect()
}

fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Serves until the process ends. `addr` like "127.0.0.1:0"; the bound address is returned through `on_ready`.
/// A service reachable from other hosts spends the operator's provider credentials, so binding a non-loopback
/// address needs AI_ROUTER_TOKENS (callers send one as `Authorization: Bearer`), or AI_ROUTER_OPEN=1 to accept that.
pub fn serve(router: Router, addr: &str, on_ready: impl FnOnce(String)) -> Result<(), String> {
    serve_with(router, addr, tokens(), on_ready)
}

/// `serve` with the accepted bearer tokens given (empty: no authentication, loopback only unless AI_ROUTER_OPEN=1).
pub fn serve_with(router: Router, addr: &str, accepted: Vec<String>, on_ready: impl FnOnce(String)) -> Result<(), String> {
    let loopback = addr.starts_with("127.") || addr.starts_with("[::1]") || addr.starts_with("localhost:");
    if !loopback && accepted.is_empty() && std::env::var("AI_ROUTER_OPEN").as_deref() != Ok("1") {
        return Err(format!("{addr}: set AI_ROUTER_TOKENS before serving beyond loopback (or AI_ROUTER_OPEN=1)"));
    }
    let accepted = Arc::new(accepted);
    let server = Server::http(addr).map_err(|e| format!("{addr}: {e}"))?;
    on_ready(server.server_addr().to_string());
    let router = Arc::new(router);
    for mut req in server.incoming_requests() {
        let router = router.clone();
        let accepted = accepted.clone();
        std::thread::spawn(move || {
            let path = req.url().split('?').next().unwrap_or("").to_owned();
            if path.starts_with("/v1/") && !accepted.is_empty() {
                let given = req.headers().iter().find(|h| h.field.equiv("Authorization")).map(|h| h.value.as_str().trim().to_owned()).unwrap_or_default();
                let token = given.strip_prefix("Bearer ").unwrap_or("").trim().as_bytes().to_vec();
                if !accepted.iter().any(|t| ct_eq(t.as_bytes(), &token)) {
                    return reply(req, 401, &json!({ "error": { "message": "missing or unknown bearer token", "type": "authentication_error" } }));
                }
            }
            // request bodies are read once, up to a limit, before any parsing
            let bytes = if *req.method() == Method::Post {
                let limit = max_body();
                let mut b = Vec::new();
                let _ = req.as_reader().take(limit + 1).read_to_end(&mut b);
                if b.len() as u64 > limit {
                    return reply(req, 413, &json!({ "error": { "message": format!("request body over {limit} bytes (AI_ROUTER_MAX_BODY)"), "type": "invalid_request_error" } }));
                }
                b
            } else {
                Vec::new()
            };
            match (req.method(), path.as_str()) {
                (Method::Get, "/health") => reply(req, 200, &json!({ "ok": true, "offers": router.models("auto").len() })),
                (Method::Get, "/v1/models") => {
                    let data: Vec<Value> = router.models("auto").iter().map(|o| json!({ "id": o.id(), "object": "model", "owned_by": o.provider, "local": o.local, "zero_cost": o.zero_cost() })).collect();
                    reply(req, 200, &json!({ "object": "list", "data": data }));
                }
                (Method::Post, "/v1/chat/completions") => {
                    match serde_json::from_slice::<Value>(&bytes) {
                        Err(e) => reply(req, 400, &json!({ "error": { "message": format!("invalid JSON: {e}"), "type": "invalid_request_error" } })),
                        Ok(body) if body["stream"] == true => {
                            // server-sent events: one `data:` line per chunk, the router summary as an SSE comment
                            // (clients ignore comments), then `[DONE]`
                            let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
                            std::thread::spawn(move || {
                                let summary = router.chat_stream(&body, &mut |c| {
                                    let _ = tx.send(format!("data: {c}\n\n").into_bytes());
                                });
                                if summary.get("error").is_some() {
                                    let _ = tx.send(format!("data: {summary}\n\n").into_bytes());
                                }
                                let _ = tx.send(format!(": router {}\n\ndata: [DONE]\n\n", summary["router"]).into_bytes());
                            });
                            let h = Header::from_bytes(&b"Content-Type"[..], &b"text/event-stream"[..]).expect("static header");
                            let _ = req.respond(Response::new(200.into(), vec![h], ChannelReader { rx, buf: Vec::new(), pos: 0 }, None, None));
                        }
                        Ok(body) => {
                            let answer = router.chat(&body);
                            let status = if answer.get("error").is_some() { 502 } else { 200 };
                            reply(req, status, &answer);
                        }
                    }
                }
                (Method::Post, "/v1/messages") => {
                    match serde_json::from_slice::<Value>(&bytes) {
                        Err(e) => reply(req, 400, &json!({ "type": "error", "error": { "type": "invalid_request_error", "message": format!("invalid JSON: {e}") } })),
                        Ok(body) => {
                            let answer = router.chat(&anthropic_to_openai(&body));
                            if answer.get("error").is_some() {
                                reply(req, 502, &json!({ "type": "error", "error": { "type": "api_error", "message": answer["error"]["message"] }, "router": answer["router"] }));
                            } else {
                                reply(req, 200, &openai_to_anthropic(&answer));
                            }
                        }
                    }
                }
                (Method::Post, p @ ("/v1/embeddings" | "/v1/images/generations" | "/v1/audio/speech" | "/v1/audio/transcriptions")) => {
                    let kind = match p {
                        "/v1/embeddings" => Kind::Embeddings,
                        "/v1/images/generations" => Kind::Images,
                        "/v1/audio/speech" => Kind::Speech,
                        _ => Kind::Transcription,
                    };
                    let content_type = req.headers().iter().find(|h| h.field.equiv("Content-Type")).map(|h| h.value.as_str().to_owned()).unwrap_or_default();
                    let (body, model) = if let Some(b) = content_type.split("boundary=").nth(1) {
                        let boundary = b.split(';').next().unwrap_or(b).trim_matches('"').to_owned();
                        let model = crate::transport::multipart_field(&bytes, &boundary, "model").unwrap_or_else(|| "auto".to_owned());
                        (Body::Multipart { boundary, bytes }, model)
                    } else {
                        match serde_json::from_slice::<Value>(&bytes) {
                            Ok(v) => {
                                let model = v["model"].as_str().unwrap_or("auto").to_owned();
                                (Body::Json(v), model)
                            }
                            Err(e) => return reply(req, 400, &json!({ "error": { "message": format!("invalid JSON: {e}"), "type": "invalid_request_error" } })),
                        }
                    };
                    let f = router.forward(kind, &model, p.trim_start_matches("/v1/"), &body);
                    let mut headers = vec![Header::from_bytes(&b"Content-Type"[..], f.content_type.as_bytes()).unwrap_or_else(|_| Header::from_bytes(&b"Content-Type"[..], &b"application/octet-stream"[..]).expect("static header"))];
                    let mut out = f.body;
                    if f.content_type.starts_with("application/json") && (200..300).contains(&f.status) {
                        if let Ok(mut v) = serde_json::from_slice::<Value>(&out) {
                            v["router"] = f.router.clone();
                            out = v.to_string().into_bytes();
                        }
                    } else {
                        // binary answers (audio, images as bytes) carry a short summary in a header: the offer and
                        // the number of attempts (error texts of failed attempts can be long)
                        let short = json!({ "offer": f.router["offer"], "attempts": f.router["attempts"].as_array().map_or(0, Vec::len) });
                        if let Ok(h) = Header::from_bytes(&b"X-Router"[..], short.to_string().as_bytes()) {
                            headers.push(h);
                        }
                    }
                    let _ = req.respond(Response::new(f.status.into(), headers, std::io::Cursor::new(out), None, None));
                }
                _ => reply(req, 404, &json!({ "error": { "message": "not found", "type": "not_found" } })),
            }
        });
    }
    Ok(())
}

/// Largest request body accepted, in bytes: AI_ROUTER_MAX_BODY, else 256 MiB (audio uploads for transcription).
fn max_body() -> u64 {
    std::env::var("AI_ROUTER_MAX_BODY").ok().and_then(|v| v.parse().ok()).unwrap_or(256 * 1024 * 1024)
}

fn blocks_text(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(b) => b.iter().filter(|x| x["type"] == "text").filter_map(|x| x["text"].as_str()).collect::<Vec<_>>().join("\n"),
        _ => String::new(),
    }
}

/// An Anthropic messages request as an OpenAI chat-completions request.
pub fn anthropic_to_openai(req: &Value) -> Value {
    let mut messages = Vec::new();
    let system = blocks_text(&req["system"]);
    if !system.is_empty() {
        messages.push(json!({ "role": "system", "content": system }));
    }
    for m in req["messages"].as_array().into_iter().flatten() {
        let blocks = m["content"].as_array().cloned().unwrap_or_default();
        for r in blocks.iter().filter(|b| b["type"] == "tool_result") {
            messages.push(json!({ "role": "tool", "tool_call_id": r["tool_use_id"], "content": blocks_text(&r["content"]) }));
        }
        let calls: Vec<Value> = blocks
            .iter()
            .filter(|b| b["type"] == "tool_use")
            .map(|b| json!({ "id": b["id"], "type": "function", "function": { "name": b["name"], "arguments": b["input"].to_string() } }))
            .collect();
        let text = blocks_text(&m["content"]);
        if text.is_empty() && calls.is_empty() {
            continue;
        }
        let mut msg = json!({ "role": m["role"], "content": text });
        if !calls.is_empty() {
            msg["tool_calls"] = json!(calls);
        }
        messages.push(msg);
    }
    let mut out = json!({ "model": req["model"].as_str().unwrap_or("auto"), "messages": messages });
    for k in ["max_tokens", "temperature"] {
        if !req[k].is_null() {
            out[k] = req[k].clone();
        }
    }
    let tools: Vec<Value> = req["tools"].as_array().into_iter().flatten().map(|t| json!({ "type": "function", "function": { "name": t["name"], "description": t["description"], "parameters": t["input_schema"] } })).collect();
    if !tools.is_empty() {
        out["tools"] = json!(tools);
    }
    out
}

/// A chat.completion answer as an Anthropic message.
pub fn openai_to_anthropic(ans: &Value) -> Value {
    let msg = &ans["choices"][0]["message"];
    let mut content = Vec::new();
    if let Some(t) = msg["content"].as_str().filter(|t| !t.is_empty()) {
        content.push(json!({ "type": "text", "text": t }));
    }
    for c in msg["tool_calls"].as_array().into_iter().flatten() {
        let input: Value = serde_json::from_str(c["function"]["arguments"].as_str().unwrap_or("{}")).unwrap_or(json!({}));
        content.push(json!({ "type": "tool_use", "id": c["id"], "name": c["function"]["name"], "input": input }));
    }
    let stop = match ans["choices"][0]["finish_reason"].as_str() {
        Some("length") => "max_tokens",
        Some("tool_calls") => "tool_use",
        _ => "end_turn",
    };
    json!({
        "id": ans["id"], "type": "message", "role": "assistant", "model": ans["model"], "content": content, "stop_reason": stop,
        "usage": { "input_tokens": ans["usage"]["prompt_tokens"], "output_tokens": ans["usage"]["completion_tokens"] },
        "router": ans["router"]
    })
}

/// Bytes sent by another thread, read as a stream until that thread drops its sender.
struct ChannelReader {
    rx: std::sync::mpsc::Receiver<Vec<u8>>,
    buf: Vec<u8>,
    pos: usize,
}

impl std::io::Read for ChannelReader {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        while self.pos >= self.buf.len() {
            match self.rx.recv() {
                Ok(b) => (self.buf, self.pos) = (b, 0),
                Err(_) => return Ok(0),
            }
        }
        let n = (self.buf.len() - self.pos).min(out.len());
        out[..n].copy_from_slice(&self.buf[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}
