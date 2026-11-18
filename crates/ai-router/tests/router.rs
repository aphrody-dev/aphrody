//! End to end against local stand-ins of three providers (OpenAI-compatible, Anthropic, Google) on loopback ports.

use ai_router::{capi, Config, Router};
use serde_json::{json, Value};
use std::ffi::{CStr, CString};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// A loopback server answering every POST with `status` and `body`; returns its base URL and a request counter.
fn stand_in(status: u16, body: Value) -> (String, Arc<AtomicUsize>) {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let base = format!("http://{}", server.server_addr());
    let hits = Arc::new(AtomicUsize::new(0));
    let h = hits.clone();
    std::thread::spawn(move || {
        for mut req in server.incoming_requests() {
            let mut s = String::new();
            let _ = req.as_reader().read_to_string(&mut s);
            h.fetch_add(1, Ordering::SeqCst);
            let _ = req.respond(tiny_http::Response::from_string(body.to_string()).with_status_code(status));
        }
    });
    (base, hits)
}

fn completion(text: &str) -> Value {
    json!({ "id": "x", "object": "chat.completion", "choices": [{ "index": 0, "message": { "role": "assistant", "content": text }, "finish_reason": "stop" }] })
}

fn catalog(entries: &[(&str, &str, &str, &str, f64)]) -> Value {
    let mut doc = serde_json::Map::new();
    for (pid, npm, api, env, cost) in entries {
        doc.insert(
            (*pid).to_owned(),
            json!({ "id": pid, "npm": npm, "api": api, "env": [env], "models": { "m": { "name": "m", "cost": { "input": cost, "output": cost }, "limit": { "context": 100000 }, "tool_call": true, "modalities": { "input": ["text"], "output": ["text"] } } } }),
        );
    }
    Value::Object(doc)
}

fn set(k: &str) {
    // SAFETY: tests set credential variables before any thread of the router reads them
    unsafe { std::env::set_var(k, "test-credential") };
}

#[test]
fn zero_cost_first_then_failover_to_the_next_offer() {
    let (busy, busy_hits) = stand_in(429, json!({ "error": "rate limited" }));
    let (ok, ok_hits) = stand_in(200, completion("hello"));
    set("AIR_T1_A");
    set("AIR_T1_B");
    let config = Config { catalog: Some(catalog(&[("free-a", "@ai-sdk/openai-compatible", &busy, "AIR_T1_A", 0.0), ("paid-b", "@ai-sdk/openai-compatible", &ok, "AIR_T1_B", 1.5)])), local: Some(vec![]), ..Config::default() };
    let r = Router::open(&config).unwrap();
    let order: Vec<String> = r.models("auto").iter().map(|o| o.id()).collect();
    assert_eq!(order, ["free-a/m", "paid-b/m"]);
    let answer = r.chat(&json!({ "model": "auto", "messages": [{ "role": "user", "content": "hi" }] }));
    assert_eq!(answer["choices"][0]["message"]["content"], "hello");
    assert_eq!(answer["router"]["offer"], "paid-b/m");
    assert_eq!(answer["router"]["attempts"][0]["status"], 429);
    assert_eq!((busy_hits.load(Ordering::SeqCst), ok_hits.load(Ordering::SeqCst)), (1, 1));
}

#[test]
fn offers_without_credentials_are_never_tried() {
    let (ok, hits) = stand_in(200, completion("unused"));
    let config = Config { catalog: Some(catalog(&[("nokey", "@ai-sdk/openai-compatible", &ok, "AIR_T2_UNSET", 0.0)])), local: Some(vec![]), ..Config::default() };
    let r = Router::open(&config).unwrap();
    assert!(r.models("auto").is_empty());
    let answer = r.chat(&json!({ "messages": [{ "role": "user", "content": "hi" }] }));
    assert_eq!(answer["error"]["type"], "router_exhausted");
    assert_eq!(hits.load(Ordering::SeqCst), 0);
}

#[test]
fn anthropic_and_google_answers_come_back_as_chat_completions() {
    let (anth, _) = stand_in(200, json!({ "id": "msg", "content": [{ "type": "text", "text": "from anthropic" }], "stop_reason": "end_turn", "usage": { "input_tokens": 3, "output_tokens": 2 } }));
    let (goog, _) = stand_in(200, json!({ "candidates": [{ "content": { "parts": [{ "text": "from google" }] }, "finishReason": "STOP" }], "usageMetadata": { "promptTokenCount": 3, "candidatesTokenCount": 2 } }));
    set("AIR_T3_A");
    set("AIR_T3_G");
    let config = Config { catalog: Some(catalog(&[("anth", "@ai-sdk/anthropic", &anth, "AIR_T3_A", 1.0), ("goog", "@ai-sdk/google", &goog, "AIR_T3_G", 2.0)])), local: Some(vec![]), ..Config::default() };
    let r = Router::open(&config).unwrap();
    let a = r.chat(&json!({ "model": "anth/m", "messages": [{ "role": "system", "content": "be brief" }, { "role": "user", "content": "hi" }] }));
    assert_eq!(a["choices"][0]["message"]["content"], "from anthropic");
    assert_eq!(a["usage"]["completion_tokens"], 2);
    let g = r.chat(&json!({ "model": "goog/m", "messages": [{ "role": "user", "content": "hi" }] }));
    assert_eq!(g["choices"][0]["message"]["content"], "from google");
}

#[test]
fn local_servers_come_first_and_through_the_c_abi() {
    let (local, _) = stand_in(200, completion("local answer"));
    // a local OpenAI-compatible server lists its models at /models
    let models_server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let base = format!("http://{}", models_server.server_addr());
    let target = local.clone();
    std::thread::spawn(move || {
        for req in models_server.incoming_requests() {
            if req.url().ends_with("/models") {
                let _ = req.respond(tiny_http::Response::from_string(json!({ "data": [{ "id": "tiny" }] }).to_string()));
            } else {
                // forward chat requests to the completion stand-in
                let mut res = ureq::post(format!("{target}/chat/completions")).send_json(json!({})).unwrap();
                let _ = req.respond(tiny_http::Response::from_string(res.body_mut().read_to_string().unwrap()));
            }
        }
    });
    set("AIR_T4_P");
    let (paid, paid_hits) = stand_in(200, completion("paid answer"));
    let config = json!({ "catalog": catalog(&[("paid", "@ai-sdk/openai-compatible", &paid, "AIR_T4_P", 3.0)]), "local": [base] });
    let c = CString::new(config.to_string()).unwrap();
    let router = unsafe { capi::aphrody_router_open(c.as_ptr()) };
    assert!(!router.is_null());
    let req = CString::new(json!({ "messages": [{ "role": "user", "content": "hi" }] }).to_string()).unwrap();
    let out = unsafe { capi::aphrody_router_chat(router, req.as_ptr()) };
    let answer: Value = serde_json::from_str(unsafe { CStr::from_ptr(out) }.to_str().unwrap()).unwrap();
    unsafe { capi::aphrody_free(out) };
    assert_eq!(answer["choices"][0]["message"]["content"], "local answer");
    assert_eq!(answer["router"]["local"], true);
    assert_eq!(paid_hits.load(Ordering::SeqCst), 0);
    let m = CString::new("auto:cheap").unwrap();
    let list = unsafe { capi::aphrody_router_models(router, m.as_ptr()) };
    let offers: Value = serde_json::from_str(unsafe { CStr::from_ptr(list) }.to_str().unwrap()).unwrap();
    unsafe { capi::aphrody_free(list) };
    assert_eq!(offers.as_array().unwrap().len(), 1);
    unsafe { capi::aphrody_router_close(router) };
}

#[test]
fn the_service_answers_openai_and_anthropic_clients() {
    let (ok, _) = stand_in(200, json!({ "object": "chat.completion", "choices": [{ "index": 0, "message": { "role": "assistant", "content": "served" }, "finish_reason": "stop" }], "usage": { "prompt_tokens": 4, "completion_tokens": 1 } }));
    set("AIR_T5_K");
    let config = Config { catalog: Some(catalog(&[("svc", "@ai-sdk/openai-compatible", &ok, "AIR_T5_K", 0.0)])), local: Some(vec![]), ..Config::default() };
    let router = Router::open(&config).unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || ai_router::server::serve(router, "127.0.0.1:0", |a| tx.send(a).unwrap()).unwrap());
    let base = format!("http://{}", rx.recv().unwrap());
    let models: Value = ureq::get(format!("{base}/v1/models")).call().unwrap().body_mut().read_json().unwrap();
    assert_eq!(models["data"][0]["id"], "svc/m");
    let openai: Value = ureq::post(format!("{base}/v1/chat/completions")).send_json(json!({ "messages": [{ "role": "user", "content": "hi" }] })).unwrap().body_mut().read_json().unwrap();
    assert_eq!(openai["choices"][0]["message"]["content"], "served");
    let anthropic: Value = ureq::post(format!("{base}/v1/messages"))
        .send_json(json!({ "model": "auto", "max_tokens": 64, "system": "brief", "messages": [{ "role": "user", "content": [{ "type": "text", "text": "hi" }] }] }))
        .unwrap()
        .body_mut()
        .read_json()
        .unwrap();
    assert_eq!(anthropic["type"], "message");
    assert_eq!(anthropic["content"][0]["text"], "served");
    assert_eq!(anthropic["stop_reason"], "end_turn");
    assert_eq!(anthropic["usage"]["output_tokens"], 1);
}

#[test]
fn streams_chunks_through_the_library_and_the_service() {
    // an OpenAI-compatible stand-in answering with server-sent events
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let base = format!("http://{}", server.server_addr());
    std::thread::spawn(move || {
        for req in server.incoming_requests() {
            let sse = ["Hel", "lo"].iter().map(|t| format!("data: {}\n\n", json!({ "object": "chat.completion.chunk", "choices": [{ "index": 0, "delta": { "content": t } }] }))).collect::<String>() + "data: [DONE]\n\n";
            let _ = req.respond(tiny_http::Response::from_string(sse));
        }
    });
    set("AIR_T6_S");
    let config = Config { catalog: Some(catalog(&[("sse", "@ai-sdk/openai-compatible", &base, "AIR_T6_S", 0.0)])), local: Some(vec![]), ..Config::default() };
    let router = Router::open(&config).unwrap();
    let mut text = String::new();
    let summary = router.chat_stream(&json!({ "messages": [{ "role": "user", "content": "hi" }] }), &mut |c| text.push_str(c["choices"][0]["delta"]["content"].as_str().unwrap_or("")));
    assert_eq!(text, "Hello");
    assert_eq!(summary["router"]["offer"], "sse/m");
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || ai_router::server::serve(router, "127.0.0.1:0", |a| tx.send(a).unwrap()).unwrap());
    let svc = format!("http://{}", rx.recv().unwrap());
    let body = ureq::post(format!("{svc}/v1/chat/completions")).send_json(json!({ "stream": true, "messages": [{ "role": "user", "content": "hi" }] })).unwrap().body_mut().read_to_string().unwrap();
    let streamed: String = body.lines().filter_map(|l| l.strip_prefix("data: ")).filter(|d| *d != "[DONE]").filter_map(|d| serde_json::from_str::<Value>(d).ok()).filter_map(|c| c["choices"][0]["delta"]["content"].as_str().map(str::to_owned)).collect();
    assert_eq!(streamed, "Hello");
    assert!(body.contains(": router "));
    assert!(body.trim_end().ends_with("data: [DONE]"));
}

#[test]
fn embeddings_and_transcriptions_go_to_offers_of_that_kind() {
    // one local server with a chat model and an embedding model; it echoes the path and the model it received
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let base = format!("http://{}/v1", server.server_addr());
    std::thread::spawn(move || {
        for mut req in server.incoming_requests() {
            let url = req.url().to_owned();
            let mut body = Vec::new();
            let _ = req.as_reader().read_to_end(&mut body);
            let answer = if url.ends_with("/models") {
                json!({ "data": [{ "id": "chat-small" }, { "id": "text-embed-small" }, { "id": "whisper-tiny" }] })
            } else {
                let text = String::from_utf8_lossy(&body).to_string();
                let model = serde_json::from_str::<Value>(&text).ok().and_then(|v| v["model"].as_str().map(str::to_owned)).unwrap_or_else(|| text.split("name=\"model\"\r\n\r\n").nth(1).and_then(|r| r.split("\r\n").next()).unwrap_or("").to_owned());
                json!({ "object": "list", "path": url, "model": model, "data": [{ "embedding": [0.1, 0.2] }] })
            };
            let _ = req.respond(tiny_http::Response::from_string(answer.to_string()).with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap()));
        }
    });
    let config = Config { catalog: Some(json!({})), local: Some(vec![base]), ..Config::default() };
    let r = Router::open(&config).unwrap();
    let f = r.forward(ai_router::policy::Kind::Embeddings, "auto", "embeddings", &ai_router::Body::Json(json!({ "model": "auto", "input": "hi" })));
    let v: Value = serde_json::from_slice(&f.body).unwrap();
    assert_eq!((f.status, v["path"].as_str(), v["model"].as_str()), (200, Some("/v1/embeddings"), Some("text-embed-small")));

    let boundary = "XyZ";
    let form = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"model\"\r\n\r\nauto\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"a.wav\"\r\nContent-Type: audio/wav\r\n\r\nRIFF\r\n--{boundary}--\r\n");
    let f = r.forward(ai_router::policy::Kind::Transcription, "auto", "audio/transcriptions", &ai_router::Body::Multipart { boundary: boundary.to_owned(), bytes: form.into_bytes() });
    let v: Value = serde_json::from_slice(&f.body).unwrap();
    assert_eq!((v["path"].as_str(), v["model"].as_str()), (Some("/v1/audio/transcriptions"), Some("whisper-tiny")));

    // through the service, with the router summary added to the JSON answer
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || ai_router::server::serve(r, "127.0.0.1:0", |a| tx.send(a).unwrap()).unwrap());
    let addr = rx.recv().unwrap();
    let mut res = ureq::post(&format!("http://{addr}/v1/embeddings")).send_json(json!({ "input": "hi" })).unwrap();
    let v: Value = serde_json::from_str(&res.body_mut().read_to_string().unwrap()).unwrap();
    assert_eq!(v["router"]["offer"].as_str().map(|s| s.ends_with("/text-embed-small")), Some(true));
}

#[test]
fn multipart_model_field_is_replaced_or_added() {
    let b = "B";
    let with = format!("--{b}\r\nContent-Disposition: form-data; name=\"model\"\r\n\r\nold\r\n--{b}--\r\n");
    let out = String::from_utf8(ai_router::transport::multipart_set(with.as_bytes(), b, "model", "new")).unwrap();
    assert!(out.contains("\r\n\r\nnew\r\n") && !out.contains("old"));
    let without = format!("--{b}\r\nContent-Disposition: form-data; name=\"file\"\r\n\r\nx\r\n--{b}--\r\n");
    let out = String::from_utf8(ai_router::transport::multipart_set(without.as_bytes(), b, "model", "m")).unwrap();
    assert!(out.starts_with(&format!("--{b}\r\nContent-Disposition: form-data; name=\"model\"\r\n\r\nm\r\n")) && out.contains("name=\"file\""));
}

/// A stand-in answering every request with the given server-sent events; returns its base URL and the paths it saw.
fn sse_stand_in(events: Vec<Value>) -> (String, Arc<std::sync::Mutex<Vec<String>>>) {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let base = format!("http://{}", server.server_addr());
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let s = seen.clone();
    std::thread::spawn(move || {
        for req in server.incoming_requests() {
            s.lock().unwrap().push(req.url().to_owned());
            let body: String = events.iter().map(|e| format!("event: x\ndata: {e}\n\n")).collect();
            let _ = req.respond(tiny_http::Response::from_string(body));
        }
    });
    (base, seen)
}

#[test]
fn anthropic_and_google_stream_natively() {
    let (a, a_seen) = sse_stand_in(vec![
        json!({ "type": "message_start", "message": { "id": "m" } }),
        json!({ "type": "content_block_start", "index": 0, "content_block": { "type": "text", "text": "" } }),
        json!({ "type": "content_block_delta", "index": 0, "delta": { "type": "text_delta", "text": "Hel" } }),
        json!({ "type": "content_block_delta", "index": 0, "delta": { "type": "text_delta", "text": "lo" } }),
        json!({ "type": "content_block_start", "index": 1, "content_block": { "type": "tool_use", "id": "t1", "name": "find" } }),
        json!({ "type": "content_block_delta", "index": 1, "delta": { "type": "input_json_delta", "partial_json": "{\"q\":" } }),
        json!({ "type": "content_block_delta", "index": 1, "delta": { "type": "input_json_delta", "partial_json": "\"x\"}" } }),
        json!({ "type": "message_delta", "delta": { "stop_reason": "tool_use" } }),
        json!({ "type": "message_stop" }),
    ]);
    let (g, g_seen) = sse_stand_in(vec![
        json!({ "candidates": [{ "content": { "parts": [{ "text": "Bon" }] } }] }),
        json!({ "candidates": [{ "content": { "parts": [{ "text": "jour" }] }, "finishReason": "STOP" }] }),
    ]);
    set("AIR_T9_A");
    set("AIR_T9_G");
    for (pid, npm, base, env, want_text) in [("anth", "@ai-sdk/anthropic", &a, "AIR_T9_A", "Hello"), ("goog", "@ai-sdk/google", &g, "AIR_T9_G", "Bonjour")] {
        let config = Config { catalog: Some(catalog(&[(pid, npm, base, env, 0.0)])), local: Some(vec![]), ..Config::default() };
        let router = Router::open(&config).unwrap();
        let mut chunks = Vec::new();
        let summary = router.chat_stream(&json!({ "messages": [{ "role": "user", "content": "hi" }] }), &mut |c| chunks.push(c.clone()));
        assert_eq!(summary["router"]["offer"], format!("{pid}/m"));
        let text: String = chunks.iter().filter_map(|c| c["choices"][0]["delta"]["content"].as_str()).collect();
        assert_eq!(text, want_text);
        assert!(chunks.len() >= 2, "{pid} streamed in pieces");
        if pid == "anth" {
            let args: String = chunks.iter().filter_map(|c| c["choices"][0]["delta"]["tool_calls"][0]["function"]["arguments"].as_str()).collect();
            assert_eq!(args, "{\"q\":\"x\"}");
            assert_eq!(chunks.iter().find_map(|c| c["choices"][0]["delta"]["tool_calls"][0]["function"]["name"].as_str()), Some("find"));
            assert_eq!(chunks.last().unwrap()["choices"][0]["finish_reason"], "tool_calls");
        } else {
            assert_eq!(chunks.last().unwrap()["choices"][0]["finish_reason"], "stop");
        }
    }
    assert_eq!(a_seen.lock().unwrap()[0], "/messages");
    assert_eq!(g_seen.lock().unwrap()[0], "/models/m:streamGenerateContent?alt=sse");
}

#[test]
fn anthropic_tool_without_arguments_streams_an_empty_object() {
    let (a, _) = sse_stand_in(vec![
        json!({ "type": "message_start", "message": { "id": "m" } }),
        json!({ "type": "content_block_start", "index": 0, "content_block": { "type": "tool_use", "id": "t1", "name": "now", "input": {} } }),
        json!({ "type": "content_block_stop", "index": 0 }),
        json!({ "type": "message_delta", "delta": { "stop_reason": "tool_use" } }),
    ]);
    set("AIR_T10_A");
    let config = Config { catalog: Some(catalog(&[("anth0", "@ai-sdk/anthropic", &a, "AIR_T10_A", 0.0)])), local: Some(vec![]), ..Config::default() };
    let router = Router::open(&config).unwrap();
    let mut chunks = Vec::new();
    router.chat_stream(&json!({ "messages": [{ "role": "user", "content": "time?" }] }), &mut |c| chunks.push(c.clone()));
    assert_eq!(chunks[0]["choices"][0]["delta"]["role"], "assistant");
    let args: String = chunks.iter().filter_map(|c| c["choices"][0]["delta"]["tool_calls"][0]["function"]["arguments"].as_str()).collect();
    assert_eq!(args, "{}");
}

#[test]
fn multipart_fields_are_read_without_decoding_binary_parts() {
    let b = "zz";
    let mut form = format!("--{b}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"a.wav\"\r\n\r\n").into_bytes();
    form.extend_from_slice(&[0xff, 0x00, 0x2d, 0x2d, 0x7a, 0x7a, 0xfe]); // binary, containing "--zz" without a CRLF before it
    form.extend_from_slice(format!("\r\n--{b}\r\nContent-Disposition: form-data; name=\"model\"\r\n\r\nwhisper-x\r\n--{b}--\r\n").as_bytes());
    assert_eq!(ai_router::transport::multipart_field(&form, b, "model").as_deref(), Some("whisper-x"));
    let out = ai_router::transport::multipart_set(&form, b, "model", "m2");
    assert_eq!(ai_router::transport::multipart_field(&out, b, "model").as_deref(), Some("m2"));
    assert!(out.windows(7).any(|w| w == [0xff, 0x00, 0x2d, 0x2d, 0x7a, 0x7a, 0xfe]));
}

#[test]
fn bearer_tokens_guard_the_service_and_non_loopback_needs_them() {
    let (ok, _) = stand_in(200, completion("hi"));
    set("AIR_T11");
    let config = Config { catalog: Some(catalog(&[("p", "@ai-sdk/openai-compatible", &ok, "AIR_T11", 0.0)])), local: Some(vec![]), ..Config::default() };
    let (tx, rx) = std::sync::mpsc::channel();
    let router = Router::open(&config).unwrap();
    std::thread::spawn(move || ai_router::server::serve_with(router, "127.0.0.1:0", vec!["s3cret".into()], |a| tx.send(a).unwrap()).unwrap());
    let addr = rx.recv().unwrap();
    let body = json!({ "messages": [{ "role": "user", "content": "hi" }] });
    assert_eq!(ureq::post(format!("http://{addr}/v1/chat/completions")).config().http_status_as_error(false).build().send_json(&body).unwrap().status().as_u16(), 401);
    let mut r = ureq::post(format!("http://{addr}/v1/chat/completions")).header("Authorization", "Bearer s3cret").send_json(&body).unwrap();
    assert_eq!(serde_json::from_str::<Value>(&r.body_mut().read_to_string().unwrap()).unwrap()["choices"][0]["message"]["content"], "hi");
    assert_eq!(ureq::get(format!("http://{addr}/health")).call().unwrap().status().as_u16(), 200);
    // a public address without tokens is refused
    let router = Router::open(&config).unwrap();
    assert!(ai_router::server::serve_with(router, "0.0.0.0:0", vec![], |_| {}).is_err());
}
