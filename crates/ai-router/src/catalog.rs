//! Offers known at run time: the models.dev catalog (providers, base URLs, credential variables, prices, limits,
//! modalities) joined with the OpenAI-compatible servers running on this machine (Ollama, llama.cpp server,
//! LM Studio, or any base URL the caller lists). Nothing about a provider is written in this crate.

use serde::Serialize;
use serde_json::Value;
use std::time::Duration;

/// Wire protocol of an offer, from the models.dev `npm` adapter of its provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Protocol {
    OpenAi,
    Anthropic,
    Google,
}

#[derive(Clone, Debug, Serialize)]
pub struct Offer {
    pub provider: String,
    pub model: String,
    pub name: String,
    pub protocol: Protocol,
    pub base_url: String,
    /// Name of the environment variable holding the credential; None for local servers.
    pub credential_env: Option<String>,
    pub local: bool,
    /// USD per million tokens, input and output; 0 for local servers.
    pub cost_in: f64,
    pub cost_out: f64,
    pub context: u64,
    pub tool_call: bool,
    pub image_input: bool,
    pub audio_input: bool,
    /// Output modalities from the catalog (text, image, audio, video); text for local servers.
    pub outputs: Vec<String>,
    /// An embedding model (by catalog family or model name).
    pub embedding: bool,
    pub release_date: String,
}

impl Offer {
    pub fn id(&self) -> String {
        format!("{}/{}", self.provider, self.model)
    }
    /// The credential, read when needed and never stored or printed.
    pub fn credential(&self) -> Option<String> {
        self.credential_env.as_ref().and_then(|k| std::env::var(k).ok()).filter(|v| !v.is_empty())
    }
    /// Usable now: local, or a credential is set.
    pub fn available(&self) -> bool {
        self.local || self.credential().is_some()
    }
    pub fn zero_cost(&self) -> bool {
        self.cost_in == 0.0 && self.cost_out == 0.0
    }
}

fn protocol_of(npm: &str) -> Option<Protocol> {
    match npm {
        "@ai-sdk/anthropic" => Some(Protocol::Anthropic),
        "@ai-sdk/google" => Some(Protocol::Google),
        // adapters that need request signing or a vendor SDK are not spoken here
        "@ai-sdk/amazon-bedrock" | "@ai-sdk/google-vertex" | "@ai-sdk/google-vertex/anthropic" | "@ai-sdk/azure" => None,
        _ => Some(Protocol::OpenAi),
    }
}

/// Default endpoint of the vendors whose catalog entry has no base URL.
fn vendor_base(npm: &str) -> Option<&'static str> {
    match npm {
        "@ai-sdk/openai" => Some("https://api.openai.com/v1"),
        "@ai-sdk/anthropic" => Some("https://api.anthropic.com/v1"),
        "@ai-sdk/google" => Some("https://generativelanguage.googleapis.com/v1beta"),
        _ => None,
    }
}

/// Offers of a models.dev catalog document (`{ <provider>: { id, env, npm, api, models: { <id>: {...} } } }`).
pub fn from_models_dev(doc: &Value) -> Vec<Offer> {
    let mut out = Vec::new();
    let Some(providers) = doc.as_object() else { return out };
    for (pid, p) in providers {
        let npm = p["npm"].as_str().unwrap_or("");
        let Some(protocol) = protocol_of(npm) else { continue };
        let Some(base) = p["api"].as_str().map(str::to_owned).or_else(|| vendor_base(npm).map(str::to_owned)) else { continue };
        let env = p["env"].as_array().and_then(|e| e.first()).and_then(Value::as_str).map(str::to_owned);
        let Some(models) = p["models"].as_object() else { continue };
        for (mid, m) in models {
            let outputs: Vec<String> = m["modalities"]["output"].as_array().map(|o| o.iter().filter_map(|x| x.as_str().map(str::to_owned)).collect()).unwrap_or_else(|| vec!["text".to_owned()]);
            let family = m["family"].as_str().unwrap_or("");
            out.push(Offer {
                provider: pid.clone(),
                model: mid.clone(),
                name: m["name"].as_str().unwrap_or(mid).to_owned(),
                protocol,
                base_url: base.trim_end_matches('/').to_owned(),
                credential_env: env.clone(),
                local: false,
                cost_in: m["cost"]["input"].as_f64().unwrap_or(f64::INFINITY),
                cost_out: m["cost"]["output"].as_f64().unwrap_or(f64::INFINITY),
                context: m["limit"]["context"].as_u64().unwrap_or(0),
                tool_call: m["tool_call"].as_bool().unwrap_or(false),
                image_input: m["modalities"]["input"].as_array().is_some_and(|i| i.iter().any(|x| x == "image")),
                audio_input: m["modalities"]["input"].as_array().is_some_and(|i| i.iter().any(|x| x == "audio")),
                embedding: family.contains("embed") || mid.to_lowercase().contains("embed"),
                outputs,
                release_date: m["release_date"].as_str().unwrap_or("").to_owned(),
            });
        }
    }
    out
}

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder().timeout_global(Some(timeout)).http_status_as_error(false).build().into()
}

pub fn fetch_json(url: &str, timeout: Duration) -> Result<Value, String> {
    let mut res = agent(timeout).get(url).call().map_err(|e| format!("{url}: {e}"))?;
    if res.status().as_u16() != 200 {
        return Err(format!("{url}: HTTP {}", res.status().as_u16()));
    }
    let text = res.body_mut().with_config().limit(256 * 1024 * 1024).read_to_string().map_err(|e| format!("{url}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("{url}: {e}"))
}

/// OpenAI-compatible servers on this machine. Each base URL is asked for `/models`; those that answer contribute
/// their models as zero-cost local offers. Ollama is also asked through its own `/api/tags`.
pub fn discover_local(bases: &[String], timeout: Duration) -> Vec<Offer> {
    let mut out = Vec::new();
    for base in bases {
        let base = base.trim_end_matches('/');
        let ids: Vec<String> = match fetch_json(&format!("{base}/models"), timeout) {
            Ok(v) => v["data"].as_array().map(|d| d.iter().filter_map(|m| m["id"].as_str().map(str::to_owned)).collect()).unwrap_or_default(),
            Err(_) => match fetch_json(&format!("{}/api/tags", base.trim_end_matches("/v1")), timeout) {
                Ok(v) => v["models"].as_array().map(|d| d.iter().filter_map(|m| m["name"].as_str().map(str::to_owned)).collect()).unwrap_or_default(),
                Err(_) => continue,
            },
        };
        let host = base.split("//").nth(1).unwrap_or(base).split('/').next().unwrap_or(base).to_owned();
        for id in ids {
            // a local server does not describe its models: an embedding model is told by its name
            let embedding = id.to_lowercase().contains("embed");
            out.push(Offer {
                provider: format!("local:{host}"),
                model: id.clone(),
                name: id,
                protocol: Protocol::OpenAi,
                base_url: base.to_owned(),
                credential_env: None,
                local: true,
                cost_in: 0.0,
                cost_out: 0.0,
                context: 0,
                tool_call: true,
                image_input: false,
                audio_input: false,
                embedding,
                outputs: vec!["text".to_owned()],
                release_date: String::new(),
            });
        }
    }
    out
}

/// The OpenAI-compatible base URLs of the local servers' documented default ports, plus OLLAMA_HOST when set.
pub fn default_local_bases() -> Vec<String> {
    let mut bases = vec![
        // Ollama, llama.cpp server, LM Studio: each tool's documented default port on loopback
        "http://127.0.0.1:11434/v1".to_owned(),
        "http://127.0.0.1:8080/v1".to_owned(),
        "http://127.0.0.1:1234/v1".to_owned(),
    ];
    if let Ok(h) = std::env::var("OLLAMA_HOST") {
        let h = if h.starts_with("http") { h } else { format!("http://{h}") };
        bases.insert(0, format!("{}/v1", h.trim_end_matches('/')));
    }
    bases
}
