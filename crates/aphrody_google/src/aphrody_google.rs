//! Pure-Rust Google OAuth 2.0 / `gcloud`, Google Cloud AI (Gemini, Vertex, Imagen, Veo, Lyria,
//! Speech v2, TTS, Vision, Video Intelligence), Google Workspace Discovery Service, and dual
//! YouTube / YouTube Music (Data API v3 + InnerTube `WEB` / `WEB_REMIX`) engine.
//!
//! Designed for zero-bloat native (Wayland, Windows, macOS), headless agent CLI/MCP, and
//! WebAssembly (`wasm32-unknown-unknown`) targets.

use serde::{Deserialize, Serialize};

pub mod ai;
pub mod auth;
pub mod innertube;
pub mod workspace;
pub mod youtube_data;
pub mod ytmusic;

pub use ai::{
    CloudSpeechRequest, CloudTtsRequest, GeminiGenerateRequest, GeminiGenerateResponse,
    GoogleAiBackend, GoogleAiClient, ImagenRequest, LyriaRequest, VeoRequest,
};
pub use auth::{
    CredentialSource, DeviceAuthorizationStart, GoogleAuthConfig, GoogleAuthManager,
    GoogleTokenBundle, PkceAuthRequest,
};
pub use innertube::{
    InnerTubeBootstrap, InnerTubeClient, InnerTubeClientProfile, InnerTubeSearchResult,
    InnerTubeVideoDetails, SponsorBlockSegment,
};
pub use workspace::{GoogleWorkspaceClient, WorkspaceDiscoveryRequest, WorkspaceServiceSpec};
pub use youtube_data::{YouTubeDataClient, YouTubePlaylistSummary, YouTubeVideoSummary};
pub use ytmusic::{
    YouTubeMusicClient, YtMusicLyrics, YtMusicQueueItem, YtMusicSearchFilter, YtMusicTrack,
};

/// Describes parity between Aphrody's pure-Rust modules and Google's official/community SDKs
/// across Rust, Go, TypeScript, Python, and Kotlin/JVM.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoogleSdkParityEntry {
    pub domain: String,
    pub rust_module: String,
    pub official_sdks: Vec<String>,
    pub community_references: Vec<String>,
    pub wasm_compatible: bool,
}

/// Returns the canonical parity matrix mapping Google Kotlin/Go/Python/TS/Rust SDKs to `aphrody_google`.
#[must_use]
pub fn sdk_parity_matrix() -> Vec<GoogleSdkParityEntry> {
    vec![
        GoogleSdkParityEntry {
            domain: "Auth & OAuth 2.0 / gcloud / ADC".to_string(),
            rust_module: "aphrody_google::auth".to_string(),
            official_sdks: vec![
                "googleapis/google-cloud-rust (google-cloud-auth)".to_string(),
                "googleapis/google-auth-library-nodejs".to_string(),
                "googleapis/google-auth-library-python".to_string(),
                "googleworkspace/cli".to_string(),
            ],
            community_references: vec![
                "djc/gcp_auth".to_string(),
                "dermesser/yup-oauth2".to_string(),
                "google-gemini/gemini-cli".to_string(),
            ],
            wasm_compatible: true,
        },
        GoogleSdkParityEntry {
            domain: "Gemini API, Vertex AI, Imagen, Veo, Lyria, Live, Speech, TTS, Vision".to_string(),
            rust_module: "aphrody_google::ai".to_string(),
            official_sdks: vec![
                "googleapis/go-genai".to_string(),
                "googleapis/js-genai".to_string(),
                "googleapis/python-genai".to_string(),
                "googleapis/java-genai (Kotlin/JVM)".to_string(),
                "googleapis/google-cloud-rust (aiplatform/speech/tts/vision)".to_string(),
                "google-gemini/live-api-web-console".to_string(),
            ],
            community_references: vec![
                "flachesis/gemini-rust".to_string(),
                "genkit-ai/genkit".to_string(),
            ],
            wasm_compatible: true,
        },
        GoogleSdkParityEntry {
            domain: "Google Workspace (Drive, Gmail, Calendar, Docs, Sheets, Slides, Tasks, Chat)".to_string(),
            rust_module: "aphrody_google::workspace".to_string(),
            official_sdks: vec![
                "googleworkspace/cli (Rust Discovery CLI)".to_string(),
                "googleapis/google-api-go-client".to_string(),
                "googleapis/google-api-nodejs-client".to_string(),
                "googleapis/google-api-python-client".to_string(),
            ],
            community_references: vec![
                "openclaw/gogcli".to_string(),
                "taylorwilsdon/google_workspace_mcp".to_string(),
                "rclone/rclone".to_string(),
            ],
            wasm_compatible: true,
        },
        GoogleSdkParityEntry {
            domain: "YouTube & YouTube Music (Data API v3 + InnerTube WEB / WEB_REMIX)".to_string(),
            rust_module: "aphrody_google::{youtube_data, innertube, ytmusic}".to_string(),
            official_sdks: vec![
                "youtube.googleapis.com/youtube/v3".to_string(),
                "youtubeanalytics.googleapis.com/v2".to_string(),
            ],
            community_references: vec![
                "nick42d/youtui (ytmapi-rs)".to_string(),
                "ThetaDev/rustypipe".to_string(),
                "LuanRT/YouTube.js".to_string(),
                "sigma67/ytmusicapi".to_string(),
                "pear-devs/pear-desktop".to_string(),
                "yt-dlp/yt-dlp".to_string(),
            ],
            wasm_compatible: true,
        },
        GoogleSdkParityEntry {
            domain: "Agents (ADK, A2A 1.0, MCP Toolbox, A2UI, Computer Use)".to_string(),
            rust_module: "aphrody_pilot + aphrody_nsh".to_string(),
            official_sdks: vec![
                "google/adk-go".to_string(),
                "google/adk-js".to_string(),
                "google/adk-python".to_string(),
                "google/adk-java (Kotlin/JVM)".to_string(),
                "a2aproject/A2A".to_string(),
                "googleapis/mcp-toolbox".to_string(),
                "a2ui-project/a2ui".to_string(),
                "google-gemini/computer-use-preview".to_string(),
            ],
            community_references: vec![
                "zavora-ai/adk-rust".to_string(),
                "EmilLindfors/a2a-rs".to_string(),
            ],
            wasm_compatible: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdk_parity_matrix_covers_all_google_languages() {
        let matrix = sdk_parity_matrix();
        assert_eq!(matrix.len(), 5);
        assert!(matrix.iter().all(|entry| entry.wasm_compatible));
        let joined = matrix
            .iter()
            .flat_map(|entry| entry.official_sdks.iter())
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
        assert!(joined.contains("go-genai"));
        assert!(joined.contains("python-genai"));
        assert!(joined.contains("java-genai"));
        assert!(joined.contains("google-cloud-rust"));
        assert!(joined.contains("googleworkspace/cli"));
    }
}
