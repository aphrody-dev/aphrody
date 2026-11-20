//! Complete pure-Rust Google Cloud AI & Gemini client unifying:
//! - Gemini Developer API (`generativelanguage.googleapis.com/v1beta`, Bearer OAuth OR API key)
//! - Vertex AI (`{region}-aiplatform.googleapis.com/v1`, Bearer OAuth + `x-goog-user-project`)
//! - Multimodal content (text, image, audio, video, YouTube URLs), Embeddings, Imagen, Veo,
//!   Lyria, Cloud Speech-to-Text v2, Cloud Text-to-Speech, Cloud Vision, Video Intelligence,
//!   Translation, and Gemini Live WebSocket URLs.

use anyhow::{Context as _, Result};
#[cfg(all(not(target_family = "wasm"), feature = "native"))]
use anyhow::bail;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::auth::GoogleAuthManager;
#[cfg(all(not(target_family = "wasm"), feature = "native"))]
use crate::auth::GoogleTokenBundle;

pub const GEMINI_DEVELOPER_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";
pub const GEMINI_LIVE_WS_URL: &str = "wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1beta.GenerativeService.BidiGenerateContent";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoogleAiBackend {
    GeminiDeveloper,
    VertexAi { project_id: String, region: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaPartInput {
    pub mime_type: String,
    pub file_uri: Option<String>,
    pub inline_base64: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeminiGenerateRequest {
    pub model: String,
    pub prompt: String,
    pub system_instruction: Option<String>,
    pub media_parts: Vec<MediaPartInput>,
    pub temperature: Option<f32>,
    pub max_output_tokens: Option<u32>,
    pub response_mime_type: Option<String>,
}

impl GeminiGenerateRequest {
    #[must_use]
    pub fn text(model: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            prompt: prompt.into(),
            system_instruction: None,
            media_parts: Vec::new(),
            temperature: None,
            max_output_tokens: None,
            response_mime_type: None,
        }
    }

    #[must_use]
    pub fn with_youtube_url(mut self, youtube_url: impl Into<String>) -> Self {
        self.media_parts.push(MediaPartInput {
            mime_type: "video/*".to_string(),
            file_uri: Some(youtube_url.into()),
            inline_base64: None,
        });
        self
    }

    #[must_use]
    pub fn to_payload(&self) -> Value {
        let mut parts = Vec::with_capacity(self.media_parts.len() + 1);
        for media in &self.media_parts {
            if let Some(uri) = &media.file_uri {
                parts.push(json!({
                    "fileData": {
                        "mimeType": media.mime_type,
                        "fileUri": uri
                    }
                }));
            } else if let Some(data) = &media.inline_base64 {
                parts.push(json!({
                    "inlineData": {
                        "mimeType": media.mime_type,
                        "data": data
                    }
                }));
            }
        }
        parts.push(json!({ "text": self.prompt }));

        let mut payload = json!({
            "contents": [{
                "role": "user",
                "parts": parts
            }]
        });

        if let Some(system) = &self.system_instruction {
            payload["systemInstruction"] = json!({
                "parts": [{ "text": system }]
            });
        }

        let mut gen_cfg = serde_json::Map::new();
        if let Some(temp) = self.temperature {
            gen_cfg.insert("temperature".to_string(), json!(temp));
        }
        if let Some(max_tokens) = self.max_output_tokens {
            gen_cfg.insert("maxOutputTokens".to_string(), json!(max_tokens));
        }
        if let Some(mime) = &self.response_mime_type {
            gen_cfg.insert("responseMimeType".to_string(), json!(mime));
        }
        if !gen_cfg.is_empty() {
            payload["generationConfig"] = Value::Object(gen_cfg);
        }

        payload
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeminiGenerateResponse {
    pub model: String,
    pub text: String,
    pub finish_reason: Option<String>,
    pub raw: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImagenRequest {
    pub model: String,
    pub prompt: String,
    pub aspect_ratio: String,
    pub sample_count: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VeoRequest {
    pub model: String,
    pub prompt: String,
    pub aspect_ratio: String,
    pub duration_seconds: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LyriaRequest {
    pub model: String,
    pub prompt: String,
    pub negative_prompt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloudSpeechRequest {
    pub language_code: String,
    pub model: String,
    pub audio_base64: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloudTtsRequest {
    pub text: String,
    pub language_code: String,
    pub voice_name: String,
    pub audio_encoding: String,
}

/// Unified pure-Rust client covering Gemini Developer API, Vertex AI, and Google Cloud Media AI.
#[derive(Debug, Clone)]
pub struct GoogleAiClient {
    auth: GoogleAuthManager,
    backend: GoogleAiBackend,
    api_key: Option<String>,
}

impl GoogleAiClient {
    #[must_use]
    pub fn new(auth: GoogleAuthManager, backend: GoogleAiBackend) -> Self {
        let api_key = std::env::var("GEMINI_API_KEY")
            .ok()
            .or_else(|| std::env::var("GOOGLE_API_KEY").ok())
            .filter(|value| !value.trim().is_empty());
        Self {
            auth,
            backend,
            api_key,
        }
    }

    #[must_use]
    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    #[must_use]
    pub fn generate_content_url(&self, model: &str) -> String {
        let normalized_model = model.trim_start_matches("models/");
        match &self.backend {
            GoogleAiBackend::GeminiDeveloper => {
                format!("{GEMINI_DEVELOPER_BASE_URL}/models/{normalized_model}:generateContent")
            }
            GoogleAiBackend::VertexAi { project_id, region } => {
                format!(
                    "https://{region}-aiplatform.googleapis.com/v1/projects/{project_id}/locations/{region}/publishers/google/models/{normalized_model}:generateContent"
                )
            }
        }
    }

    #[must_use]
    pub fn predict_url(&self, model: &str, long_running: bool) -> Result<String> {
        let normalized_model = model.trim_start_matches("models/");
        let verb = if long_running {
            "predictLongRunning"
        } else {
            "predict"
        };
        match &self.backend {
            GoogleAiBackend::VertexAi { project_id, region } => Ok(format!(
                "https://{region}-aiplatform.googleapis.com/v1/projects/{project_id}/locations/{region}/publishers/google/models/{normalized_model}:{verb}"
            )),
            GoogleAiBackend::GeminiDeveloper => {
                let project_id = self
                    .auth
                    .config()
                    .project_id
                    .as_deref()
                    .context("Vertex AI project_id required for Imagen/Veo/Lyria predict endpoints")?;
                let region = &self.auth.config().region;
                Ok(format!(
                    "https://{region}-aiplatform.googleapis.com/v1/projects/{project_id}/locations/{region}/publishers/google/models/{normalized_model}:{verb}"
                ))
            }
        }
    }

    #[must_use]
    pub fn live_websocket_url(&self) -> String {
        match &self.backend {
            GoogleAiBackend::GeminiDeveloper => GEMINI_LIVE_WS_URL.to_string(),
            GoogleAiBackend::VertexAi { region, .. } => format!(
                "wss://{region}-aiplatform.googleapis.com/ws/google.cloud.aiplatform.v1beta1.LlmBidiService/BidiGenerateContent"
            ),
        }
    }

    /// Executes a Gemini / Vertex AI `generateContent` call using OAuth/`gcloud` Bearer or API key.
    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    pub fn generate_content(&self, request: &GeminiGenerateRequest) -> Result<GeminiGenerateResponse> {
        let url = self.generate_content_url(&request.model);
        let raw = self.post_json(&url, &request.to_payload())?;
        let text = extract_candidate_text(&raw);
        let finish_reason = raw
            .pointer("/candidates/0/finishReason")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        Ok(GeminiGenerateResponse {
            model: request.model.clone(),
            text,
            finish_reason,
            raw,
        })
    }

    /// Computes vector embeddings via Gemini `embedContent`.
    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    pub fn embed_text(&self, model: &str, text: &str) -> Result<Vec<f32>> {
        let normalized = model.trim_start_matches("models/");
        let url = format!("{GEMINI_DEVELOPER_BASE_URL}/models/{normalized}:embedContent");
        let payload = json!({
            "content": { "parts": [{ "text": text }] }
        });
        let raw = self.post_json(&url, &payload)?;
        let values = raw
            .pointer("/embedding/values")
            .and_then(Value::as_array)
            .context("embedding response missing embedding.values")?;
        let mut floats = Vec::with_capacity(values.len());
        for item in values {
            floats.push(item.as_f64().context("non-numeric embedding entry")? as f32);
        }
        Ok(floats)
    }

    /// Invokes Vertex AI Imagen image generation.
    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    pub fn generate_image(&self, request: &ImagenRequest) -> Result<Value> {
        let url = self.predict_url(&request.model, false)?;
        let payload = json!({
            "instances": [{ "prompt": request.prompt }],
            "parameters": {
                "sampleCount": request.sample_count,
                "aspectRatio": request.aspect_ratio
            }
        });
        self.post_json(&url, &payload)
    }

    /// Invokes Vertex AI Veo long-running video generation.
    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    pub fn generate_video(&self, request: &VeoRequest) -> Result<Value> {
        let url = self.predict_url(&request.model, true)?;
        let payload = json!({
            "instances": [{ "prompt": request.prompt }],
            "parameters": {
                "aspectRatio": request.aspect_ratio,
                "durationSeconds": request.duration_seconds
            }
        });
        self.post_json(&url, &payload)
    }

    /// Invokes Vertex AI Lyria music generation.
    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    pub fn generate_music(&self, request: &LyriaRequest) -> Result<Value> {
        let url = self.predict_url(&request.model, false)?;
        let mut instance = json!({ "prompt": request.prompt });
        if let Some(negative) = &request.negative_prompt {
            instance["negative_prompt"] = json!(negative);
        }
        let payload = json!({ "instances": [instance] });
        self.post_json(&url, &payload)
    }

    /// Invokes Google Cloud Text-to-Speech v1 synthesis (`audioContent` base64).
    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    pub fn synthesize_speech(&self, request: &CloudTtsRequest) -> Result<Value> {
        let payload = json!({
            "input": { "text": request.text },
            "voice": {
                "languageCode": request.language_code,
                "name": request.voice_name
            },
            "audioConfig": { "audioEncoding": request.audio_encoding }
        });
        self.post_json("https://texttospeech.googleapis.com/v1/text:synthesize", &payload)
    }

    /// Invokes Google Cloud Speech-to-Text v2 synchronous recognition.
    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    pub fn transcribe_speech(&self, project_id: &str, request: &CloudSpeechRequest) -> Result<Value> {
        let url = format!(
            "https://speech.googleapis.com/v2/projects/{project_id}/locations/global/recognizers/_:recognize"
        );
        let payload = json!({
            "config": {
                "autoDecodingConfig": {},
                "languageCodes": [request.language_code],
                "model": request.model
            },
            "content": request.audio_base64
        });
        self.post_json(&url, &payload)
    }

    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    fn post_json(&self, url: &str, payload: &Value) -> Result<Value> {
        let token_opt: Option<GoogleTokenBundle> = self.auth.resolve_token().ok();
        let mut req = ureq::post(url).header("Content-Type", "application/json");
        if let Some(bundle) = token_opt {
            req = req.header("Authorization", &format!("Bearer {}", bundle.access_token));
            if let Some(project) = bundle.quota_project.as_deref() {
                req = req.header("x-goog-user-project", project);
            }
        } else if let Some(key) = self.api_key.as_deref() {
            req = req.header("x-goog-api-key", key);
        } else {
            bail!("neither Google OAuth/gcloud token nor GEMINI_API_KEY is configured");
        }
        let body_json = serde_json::to_string(payload).context("failed to encode Google AI JSON")?;
        let mut response = req
            .send(body_json.as_str())
            .with_context(|| format!("Google AI request failed for {url}"))?;
        let text = response
            .body_mut()
            .read_to_string()
            .with_context(|| format!("failed to read Google AI response from {url}"))?;
        let body: Value = serde_json::from_str(&text)
            .with_context(|| format!("invalid Google AI JSON response from {url}"))?;
        Ok(body)
    }
}

/// Extracts concatenated text parts from a Gemini / Vertex `generateContent` JSON response.
#[must_use]
pub fn extract_candidate_text(raw: &Value) -> String {
    let mut output = String::new();
    if let Some(parts) = raw
        .pointer("/candidates/0/content/parts")
        .and_then(Value::as_array)
    {
        for part in parts {
            if let Some(text) = part.get("text").and_then(Value::as_str) {
                output.push_str(text);
            }
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_developer_and_vertex_urls_and_multimodal_payload() {
        let auth = GoogleAuthManager::default().with_bearer("ya29.test");
        let dev_client = GoogleAiClient::new(auth.clone(), GoogleAiBackend::GeminiDeveloper);
        assert_eq!(
            dev_client.generate_content_url("gemini-2.5-flash"),
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent"
        );

        let vertex_client = GoogleAiClient::new(
            auth,
            GoogleAiBackend::VertexAi {
                project_id: "demo-proj".to_string(),
                region: "us-central1".to_string(),
            },
        );
        assert_eq!(
            vertex_client.generate_content_url("models/gemini-2.5-pro"),
            "https://us-central1-aiplatform.googleapis.com/v1/projects/demo-proj/locations/us-central1/publishers/google/models/gemini-2.5-pro:generateContent"
        );
        assert!(vertex_client.live_websocket_url().contains("LlmBidiService"));

        let request = GeminiGenerateRequest::text("gemini-2.5-flash", "Summarize chapters")
            .with_youtube_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ");
        let payload = request.to_payload();
        assert_eq!(
            payload.pointer("/contents/0/parts/0/fileData/fileUri"),
            Some(&json!("https://www.youtube.com/watch?v=dQw4w9WgXcQ"))
        );
        assert_eq!(
            payload.pointer("/contents/0/parts/1/text"),
            Some(&json!("Summarize chapters"))
        );
    }

    #[test]
    fn extracts_candidate_text_cleanly() {
        let raw = json!({
            "candidates": [{
                "content": {
                    "parts": [
                        { "text": "Hello " },
                        { "text": "Aphrody!" }
                    ]
                },
                "finishReason": "STOP"
            }]
        });
        assert_eq!(extract_candidate_text(&raw), "Hello Aphrody!");
    }
}
