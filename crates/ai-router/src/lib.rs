//! Multi-provider AI router. Offers come from the models.dev catalog and from the OpenAI-compatible servers running
//! on this machine; each request is tried on the ranked offers (local first, then zero cost for this caller, then
//! the cheapest) until one answers, and the answer carries the attempts. Surfaces: this library, its C ABI
//! (`capi`, `include/ai_router.h`), the `ai-router` CLI and its OpenAI-compatible service (`server`).

pub mod capi;
pub mod catalog;
pub mod policy;
pub mod server;
pub mod transport;

use catalog::Offer;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    /// URL of a models.dev catalog document; `AI_ROUTER_CATALOG` or the public models.dev API when unset.
    pub catalog_url: Option<String>,
    /// A catalog document given inline (offline use, tests); wins over `catalog_url`.
    pub catalog: Option<Value>,
    /// OpenAI-compatible base URLs of local servers; the local servers' default ports when unset.
    pub local: Option<Vec<String>>,
    /// Offers tried for one request before giving up (default 8).
    pub max_attempts: Option<usize>,
    /// Seconds per attempt (default 120); catalog and discovery use a tenth of it.
    pub timeout_s: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct Attempt {
    pub offer: String,
    pub status: u16,
    pub ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub struct Router {
    pub offers: Vec<Offer>,
    max_attempts: usize,
    timeout: Duration,
}

impl Router {
    pub fn open(config: &Config) -> Result<Self, String> {
        let timeout = Duration::from_secs(config.timeout_s.unwrap_or(120));
        let probe = timeout / 10;
        let doc = match &config.catalog {
            Some(doc) => doc.clone(),
            None => {
                let url = config.catalog_url.clone().or_else(|| std::env::var("AI_ROUTER_CATALOG").ok()).unwrap_or_else(|| "https://models.dev/api.json".to_owned());
                catalog::fetch_json(&url, probe.max(Duration::from_secs(10)))?
            }
        };
        let mut offers = catalog::discover_local(&config.local.clone().unwrap_or_else(catalog::default_local_bases), probe.min(Duration::from_secs(2)));
        offers.extend(catalog::from_models_dev(&doc));
        Ok(Self { offers, max_attempts: config.max_attempts.unwrap_or(8), timeout })
    }

    /// Offers usable now (local, or with a credential set), in the order `auto` tries them.
    pub fn models(&self, model: &str) -> Vec<&Offer> {
        policy::rank(&self.offers, model, &policy::Needs::default())
    }

    /// An OpenAI chat-completions request; the answer is a chat.completion document with a `router` member
    /// (offer that answered, attempts), or an error document listing the attempts.
    pub fn chat(&self, request: &Value) -> Value {
        let model = request["model"].as_str().unwrap_or("auto");
        let needs = needs_of(request);
        let mut attempts = Vec::new();
        for offer in policy::rank(&self.offers, model, &needs).into_iter().take(self.max_attempts) {
            let t0 = Instant::now();
            let reply = transport::call(offer, request, self.timeout);
            let ms = t0.elapsed().as_millis();
            match reply.body {
                Ok(mut body) => {
                    attempts.push(Attempt { offer: offer.id(), status: reply.status, ms, error: None });
                    body["model"] = json!(offer.id());
                    body["router"] = json!({ "offer": offer.id(), "local": offer.local, "zero_cost": offer.zero_cost(), "attempts": attempts });
                    return body;
                }
                Err(e) => attempts.push(Attempt { offer: offer.id(), status: reply.status, ms, error: Some(e) }),
            }
        }
        let message = if attempts.is_empty() { "no usable offer: start a local server or set a provider credential" } else { "every offer tried failed" };
        json!({ "error": { "message": message, "type": "router_exhausted" }, "router": { "attempts": attempts } })
    }
}

impl Router {
    /// Streams the answer: `on_chunk` receives chat.completion.chunk documents (their `model` is the offer id).
    /// Offers are tried in order until one starts streaming; once chunks flowed there is no failover. Returns the
    /// `router` summary (offer, attempts) or an error document.
    pub fn chat_stream(&self, request: &Value, on_chunk: &mut dyn FnMut(&Value)) -> Value {
        let model = request["model"].as_str().unwrap_or("auto");
        let needs = needs_of(request);
        let mut attempts = Vec::new();
        for offer in policy::rank(&self.offers, model, &needs).into_iter().take(self.max_attempts) {
            let t0 = Instant::now();
            let id = offer.id();
            let mut forward = |c: &Value| {
                let mut c = c.clone();
                c["model"] = json!(id);
                on_chunk(&c);
            };
            let (status, outcome) = transport::stream(offer, request, self.timeout, &mut forward);
            let ms = t0.elapsed().as_millis();
            match outcome {
                Ok(_) => {
                    attempts.push(Attempt { offer: offer.id(), status, ms, error: None });
                    return json!({ "router": { "offer": offer.id(), "local": offer.local, "zero_cost": offer.zero_cost(), "attempts": attempts } });
                }
                Err(e) => attempts.push(Attempt { offer: offer.id(), status, ms, error: Some(e) }),
            }
        }
        json!({ "error": { "message": "every offer tried failed", "type": "router_exhausted" }, "router": { "attempts": attempts } })
    }
}

/// Body of a non-chat inference request: JSON (embeddings, image generation, speech) or multipart form data
/// (transcription, the audio file being one part).
pub enum Body {
    Json(Value),
    Multipart { boundary: String, bytes: Vec<u8> },
}

/// Answer of a non-chat inference, passed through as the offer sent it, with the router summary.
pub struct Forwarded {
    pub status: u16,
    pub content_type: String,
    pub body: Vec<u8>,
    pub router: Value,
}

impl Router {
    /// Embeddings, image generation, speech and transcription. The request goes unchanged, except its `model`, to
    /// `<base>/<path>` of each OpenAI-compatible offer able to do `kind`, in the policy order, until one answers 2xx.
    pub fn forward(&self, kind: policy::Kind, model: &str, path: &str, body: &Body) -> Forwarded {
        let needs = policy::Needs { kind, ..Default::default() };
        let mut attempts = Vec::new();
        let offers = policy::rank(&self.offers, model, &needs).into_iter().filter(|o| o.protocol == catalog::Protocol::OpenAi);
        for offer in offers.take(self.max_attempts) {
            let t0 = Instant::now();
            let (content_type, bytes) = match body {
                Body::Json(v) => {
                    let mut v = v.clone();
                    v["model"] = json!(offer.model);
                    ("application/json".to_owned(), v.to_string().into_bytes())
                }
                Body::Multipart { boundary, bytes } => (format!("multipart/form-data; boundary={boundary}"), transport::multipart_set(bytes, boundary, "model", &offer.model)),
            };
            let r = transport::post_bytes(offer, path, &content_type, bytes, self.timeout);
            let ms = t0.elapsed().as_millis();
            if (200..300).contains(&r.status) {
                attempts.push(Attempt { offer: offer.id(), status: r.status, ms, error: None });
                let router = json!({ "offer": offer.id(), "local": offer.local, "zero_cost": offer.zero_cost(), "attempts": attempts });
                return Forwarded { status: r.status, content_type: r.content_type, body: r.body, router };
            }
            let error = String::from_utf8_lossy(&r.body).chars().take(500).collect::<String>();
            attempts.push(Attempt { offer: offer.id(), status: r.status, ms, error: Some(if error.is_empty() { r.error } else { error }) });
        }
        let message = if attempts.is_empty() { "no usable offer for this kind of request" } else { "every offer tried failed" };
        let router = json!({ "attempts": attempts });
        let body = json!({ "error": { "message": message, "type": "router_exhausted" }, "router": router }).to_string().into_bytes();
        Forwarded { status: 502, content_type: "application/json".to_owned(), body, router }
    }
}

fn needs_of(request: &Value) -> policy::Needs {
    let messages = request["messages"].as_array().cloned().unwrap_or_default();
    let images = messages.iter().any(|m| m["content"].as_array().is_some_and(|p| p.iter().any(|x| x["type"] == "image_url")));
    // about four characters per token
    let tokens = (serde_json::to_string(&messages).map(|s| s.len()).unwrap_or(0) / 4) as u64;
    policy::Needs { kind: policy::Kind::Chat, tools: request["tools"].as_array().is_some_and(|t| !t.is_empty()), images, tokens }
}
