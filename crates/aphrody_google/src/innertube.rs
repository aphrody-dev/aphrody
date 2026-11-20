//! Pure-Rust YouTube & YouTube Music InnerTube (`youtubei/v1`) client covering
//! `WEB`, `WEB_REMIX` (YouTube Music), `ANDROID_VR`, and `TVHTML5` profiles,
//! live HTML bootstrap extraction, search, player metadata, adaptive stream inventory,
//! and SponsorBlock segment lookup.

#[cfg(all(not(target_family = "wasm"), feature = "native"))]
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const YOUTUBE_ORIGIN: &str = "https://www.youtube.com";
pub const YTMUSIC_ORIGIN: &str = "https://music.youtube.com";
pub const SPONSORBLOCK_API_BASE: &str = "https://sponsor.ajay.app/api/skipSegments";
#[cfg(all(not(target_family = "wasm"), feature = "native"))]
const DESKTOP_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InnerTubeClientProfile {
    Web,
    WebRemix,
    AndroidVr,
    TvHtml5,
}

impl InnerTubeClientProfile {
    #[must_use]
    pub const fn client_name(self) -> &'static str {
        match self {
            Self::Web => "WEB",
            Self::WebRemix => "WEB_REMIX",
            Self::AndroidVr => "ANDROID_VR",
            Self::TvHtml5 => "TVHTML5",
        }
    }

    #[must_use]
    pub const fn default_version(self) -> &'static str {
        match self {
            Self::Web => "2.20261119.01.00",
            Self::WebRemix => "1.20261117.06.00",
            Self::AndroidVr => "1.61.48",
            Self::TvHtml5 => "7.20261115.00.00",
        }
    }

    #[must_use]
    pub const fn origin(self) -> &'static str {
        match self {
            Self::WebRemix => YTMUSIC_ORIGIN,
            Self::Web | Self::AndroidVr | Self::TvHtml5 => YOUTUBE_ORIGIN,
        }
    }

    #[must_use]
    pub const fn client_id_header(self) -> &'static str {
        match self {
            Self::Web => "1",
            Self::WebRemix => "67",
            Self::AndroidVr => "28",
            Self::TvHtml5 => "7",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InnerTubeBootstrap {
    pub profile: InnerTubeClientProfile,
    pub api_key: Option<String>,
    pub client_version: String,
    pub visitor_data: Option<String>,
    pub hl: String,
    pub gl: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InnerTubeSearchResult {
    pub video_id: String,
    pub title: String,
    pub author: String,
    pub length_text: Option<String>,
    pub thumbnail_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdaptiveStreamSummary {
    pub itag: u32,
    pub mime_type: String,
    pub bitrate: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub audio_quality: Option<String>,
    pub has_direct_url: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InnerTubeVideoDetails {
    pub video_id: String,
    pub title: String,
    pub author: String,
    pub length_seconds: u64,
    pub is_live: bool,
    pub adaptive_streams: Vec<AdaptiveStreamSummary>,
    pub caption_languages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SponsorBlockSegment {
    pub category: String,
    pub start_secs: f64,
    pub end_secs: f64,
}

/// Extracts `INNERTUBE_API_KEY`, `INNERTUBE_CLIENT_VERSION`, and `VISITOR_DATA` from YouTube HTML.
#[must_use]
pub fn parse_bootstrap_html(profile: InnerTubeClientProfile, html: &str) -> InnerTubeBootstrap {
    InnerTubeBootstrap {
        profile,
        api_key: extract_json_string_field(html, "INNERTUBE_API_KEY"),
        client_version: extract_json_string_field(html, "INNERTUBE_CLIENT_VERSION")
            .unwrap_or_else(|| profile.default_version().to_string()),
        visitor_data: extract_json_string_field(html, "VISITOR_DATA"),
        hl: extract_json_string_field(html, "HL").unwrap_or_else(|| "en".to_string()),
        gl: extract_json_string_field(html, "GL").unwrap_or_else(|| "US".to_string()),
    }
}

fn extract_json_string_field(source: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let start = source.find(&needle)?;
    let rest = &source[start + needle.len()..];
    let colon = rest.find(':')?;
    let after_colon = rest[colon + 1..].trim_start();
    let quote_stripped = after_colon.strip_prefix('"')?;
    let end = quote_stripped.find('"')?;
    Some(quote_stripped[..end].to_string())
}

#[derive(Debug, Clone)]
pub struct InnerTubeClient {
    bootstrap: InnerTubeBootstrap,
}

impl InnerTubeClient {
    #[must_use]
    pub fn with_defaults(profile: InnerTubeClientProfile) -> Self {
        Self {
            bootstrap: InnerTubeBootstrap {
                profile,
                api_key: None,
                client_version: profile.default_version().to_string(),
                visitor_data: None,
                hl: "en".to_string(),
                gl: "US".to_string(),
            },
        }
    }

    #[must_use]
    pub fn bootstrap(&self) -> &InnerTubeBootstrap {
        &self.bootstrap
    }

    #[must_use]
    pub fn context_payload(&self) -> Value {
        let mut client = json!({
            "clientName": self.bootstrap.profile.client_name(),
            "clientVersion": self.bootstrap.client_version,
            "hl": self.bootstrap.hl,
            "gl": self.bootstrap.gl,
        });
        if let Some(visitor) = &self.bootstrap.visitor_data {
            client["visitorData"] = json!(visitor);
        }
        json!({ "client": client })
    }

    /// Fetches live bootstrap metadata from `www.youtube.com` or `music.youtube.com`.
    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    pub fn discover(profile: InnerTubeClientProfile) -> Result<Self> {
        let origin = profile.origin();
        let mut response = ureq::get(origin)
            .header("User-Agent", DESKTOP_USER_AGENT)
            .header("Accept-Language", "en-US,en;q=0.9")
            .call()
            .with_context(|| format!("failed to bootstrap InnerTube from {origin}"))?;
        let html = response
            .body_mut()
            .read_to_string()
            .context("failed to read InnerTube bootstrap HTML")?;
        Ok(Self {
            bootstrap: parse_bootstrap_html(profile, &html),
        })
    }

    /// Calls any `/youtubei/v1/<endpoint>` action cleanly in pure Rust.
    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    pub fn post_endpoint(&self, endpoint: &str, mut body: Value) -> Result<Value> {
        if let Some(obj) = body.as_object_mut() {
            obj.insert("context".to_string(), self.context_payload());
        }
        let origin = self.bootstrap.profile.origin();
        let clean = endpoint.trim_start_matches('/');
        let url = match &self.bootstrap.api_key {
            Some(key) => format!("{origin}/youtubei/v1/{clean}?key={key}&prettyPrint=false"),
            None => format!("{origin}/youtubei/v1/{clean}?prettyPrint=false"),
        };
        let mut req = ureq::post(&url)
            .header("User-Agent", DESKTOP_USER_AGENT)
            .header("Origin", origin)
            .header("Content-Type", "application/json")
            .header(
                "X-YouTube-Client-Name",
                self.bootstrap.profile.client_id_header(),
            )
            .header("X-YouTube-Client-Version", &self.bootstrap.client_version);
        if let Some(visitor) = &self.bootstrap.visitor_data {
            req = req.header("X-Goog-Visitor-Id", visitor);
        }
        let encoded = serde_json::to_string(&body).context("failed to encode InnerTube JSON")?;
        let mut response = req
            .send(encoded.as_str())
            .with_context(|| format!("InnerTube request failed for {clean}"))?;
        let text = response
            .body_mut()
            .read_to_string()
            .with_context(|| format!("failed to read InnerTube response for {clean}"))?;
        serde_json::from_str(&text)
            .with_context(|| format!("invalid InnerTube JSON response for {clean}"))
    }

    /// Searches YouTube via `/youtubei/v1/search`.
    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    pub fn search(&self, query: &str, params: Option<&str>) -> Result<Vec<InnerTubeSearchResult>> {
        let mut payload = json!({ "query": query });
        if let Some(param_str) = params {
            payload["params"] = json!(param_str);
        }
        let raw = self.post_endpoint("search", payload)?;
        Ok(extract_video_renderers(&raw))
    }

    /// Fetches player metadata, adaptive stream inventory, and caption tracks via `/youtubei/v1/player`.
    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    pub fn player(&self, video_id: &str) -> Result<InnerTubeVideoDetails> {
        let raw = self.post_endpoint("player", json!({ "videoId": video_id }))?;
        Ok(parse_player_response(video_id, &raw))
    }

    /// Queries SponsorBlock skip segments for a YouTube video ID.
    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    pub fn sponsorblock_segments(video_id: &str) -> Result<Vec<SponsorBlockSegment>> {
        let url = format!("{SPONSORBLOCK_API_BASE}?videoID={video_id}");
        let mut response = match ureq::get(&url).call() {
            Ok(resp) => resp,
            Err(_) => return Ok(Vec::new()),
        };
        let raw: Value = response
            .body_mut()
            .read_to_string()
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_else(|| Value::Array(Vec::new()));
        let mut segments = Vec::new();
        if let Some(arr) = raw.as_array() {
            for entry in arr {
                let category = entry
                    .get("category")
                    .and_then(Value::as_str)
                    .unwrap_or("sponsor")
                    .to_string();
                if let Some(bounds) = entry.get("segment").and_then(Value::as_array)
                    && bounds.len() == 2
                    && let (Some(start), Some(end)) = (bounds[0].as_f64(), bounds[1].as_f64())
                {
                    segments.push(SponsorBlockSegment {
                        category,
                        start_secs: start,
                        end_secs: end,
                    });
                }
            }
        }
        Ok(segments)
    }
}

/// Recursively collects `videoRenderer` entries from InnerTube search/browse responses.
#[must_use]
pub fn extract_video_renderers(raw: &Value) -> Vec<InnerTubeSearchResult> {
    let mut results = Vec::new();
    collect_video_renderers(raw, &mut results);
    results
}

fn collect_video_renderers(value: &Value, out: &mut Vec<InnerTubeSearchResult>) {
    match value {
        Value::Object(map) => {
            if let Some(renderer) = map.get("videoRenderer").and_then(Value::as_object)
                && let Some(video_id) = renderer.get("videoId").and_then(Value::as_str)
            {
                let title = renderer
                    .get("title")
                    .map(extract_runs_text)
                    .unwrap_or_default();
                let author = renderer
                    .get("ownerText")
                    .or_else(|| renderer.get("longBylineText"))
                    .map(extract_runs_text)
                    .unwrap_or_default();
                let length_text = renderer
                    .get("lengthText")
                    .and_then(|v| v.get("simpleText"))
                    .and_then(Value::as_str)
                    .map(ToString::to_string);
                let thumbnail_url = renderer
                    .get("thumbnail")
                    .and_then(|v| v.pointer("/thumbnails/0/url"))
                    .and_then(Value::as_str)
                    .map(ToString::to_string);
                out.push(InnerTubeSearchResult {
                    video_id: video_id.to_string(),
                    title,
                    author,
                    length_text,
                    thumbnail_url,
                });
            }
            for child in map.values() {
                collect_video_renderers(child, out);
            }
        }
        Value::Array(items) => {
            for child in items {
                collect_video_renderers(child, out);
            }
        }
        _ => {}
    }
}

/// Extracts plain text from an InnerTube `{ simpleText | runs: [{ text }] }` object.
#[must_use]
pub fn extract_runs_text(value: &Value) -> String {
    if let Some(simple) = value.get("simpleText").and_then(Value::as_str) {
        return simple.to_string();
    }
    let mut text = String::new();
    if let Some(runs) = value.get("runs").and_then(Value::as_array) {
        for run in runs {
            if let Some(segment) = run.get("text").and_then(Value::as_str) {
                text.push_str(segment);
            }
        }
    }
    text
}

/// Parses an InnerTube `/youtubei/v1/player` JSON payload into structured video details.
#[must_use]
pub fn parse_player_response(video_id: &str, raw: &Value) -> InnerTubeVideoDetails {
    let title = raw
        .pointer("/videoDetails/title")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let author = raw
        .pointer("/videoDetails/author")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let length_seconds = raw
        .pointer("/videoDetails/lengthSeconds")
        .and_then(Value::as_str)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let is_live = raw
        .pointer("/videoDetails/isLive")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let mut adaptive_streams = Vec::new();
    if let Some(formats) = raw
        .pointer("/streamingData/adaptiveFormats")
        .and_then(Value::as_array)
    {
        for fmt in formats {
            adaptive_streams.push(AdaptiveStreamSummary {
                itag: fmt.get("itag").and_then(Value::as_u64).unwrap_or(0) as u32,
                mime_type: fmt
                    .get("mimeType")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                bitrate: fmt.get("bitrate").and_then(Value::as_u64).unwrap_or(0),
                width: fmt.get("width").and_then(Value::as_u64).map(|v| v as u32),
                height: fmt.get("height").and_then(Value::as_u64).map(|v| v as u32),
                audio_quality: fmt
                    .get("audioQuality")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                has_direct_url: fmt.get("url").and_then(Value::as_str).is_some(),
            });
        }
    }

    let mut caption_languages = Vec::new();
    if let Some(tracks) = raw
        .pointer("/captions/playerCaptionsTracklistRenderer/captionTracks")
        .and_then(Value::as_array)
    {
        for track in tracks {
            if let Some(lang) = track.get("languageCode").and_then(Value::as_str) {
                caption_languages.push(lang.to_string());
            }
        }
    }

    InnerTubeVideoDetails {
        video_id: video_id.to_string(),
        title,
        author,
        length_seconds,
        is_live,
        adaptive_streams,
        caption_languages,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_innertube_bootstrap_html_and_player_json() {
        let html = r#"ytcfg.set({"INNERTUBE_API_KEY":"AIzaTestKey","INNERTUBE_CLIENT_VERSION":"2.20261119.01.00","VISITOR_DATA":"Cgtsb2NhbA%3D%3D","HL":"fr","GL":"FR"});"#;
        let boot = parse_bootstrap_html(InnerTubeClientProfile::Web, html);
        assert_eq!(boot.api_key.as_deref(), Some("AIzaTestKey"));
        assert_eq!(boot.client_version, "2.20261119.01.00");
        assert_eq!(boot.hl, "fr");
        assert_eq!(boot.gl, "FR");

        let player_json = json!({
            "videoDetails": {
                "title": "Aphrody OS Demo",
                "author": "Aphrody",
                "lengthSeconds": "185",
                "isLive": false
            },
            "streamingData": {
                "adaptiveFormats": [{
                    "itag": 251,
                    "mimeType": "audio/webm; codecs=\"opus\"",
                    "bitrate": 135000,
                    "audioQuality": "AUDIO_QUALITY_MEDIUM",
                    "url": "https://example.invalid/stream"
                }]
            },
            "captions": {
                "playerCaptionsTracklistRenderer": {
                    "captionTracks": [{ "languageCode": "en" }, { "languageCode": "fr" }]
                }
            }
        });
        let details = parse_player_response("vid123", &player_json);
        assert_eq!(details.length_seconds, 185);
        assert_eq!(details.adaptive_streams.len(), 1);
        assert_eq!(details.adaptive_streams[0].itag, 251);
        assert_eq!(details.caption_languages, vec!["en", "fr"]);
    }
}
