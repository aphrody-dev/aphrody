//! Pure-Rust YouTube Music (`WEB_REMIX`) client powering both headless agent tools
//! (`aphrody ytm ...`, MCP `ytm_*`) and the GPUI `YouTubeMusic` desktop app.

#[cfg(all(not(target_family = "wasm"), feature = "native"))]
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(all(not(target_family = "wasm"), feature = "native"))]
use serde_json::json;

use crate::innertube::{InnerTubeClient, InnerTubeClientProfile, extract_runs_text};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum YtMusicSearchFilter {
    Songs,
    Videos,
    Albums,
    Artists,
    Playlists,
}

impl YtMusicSearchFilter {
    /// Returns the canonical YouTube Music `WEB_REMIX` protobuf filter parameter.
    #[must_use]
    pub const fn innertube_params(self) -> &'static str {
        match self {
            Self::Songs => "EgWKAQIIAWoKEAkQBRAKEAMQBA%3D%3D",
            Self::Videos => "EgWKAQIQAWoKEAkQChAFEAMQBA%3D%3D",
            Self::Albums => "EgWKAQIYAWoKEAkQChAFEAMQBA%3D%3D",
            Self::Artists => "EgWKAQIgAWoKEAkQChAFEAMQBA%3D%3D",
            Self::Playlists => "Eg-KAQwIABAAGAAgACgBMABqChAEEAMQCRAFEAo%3D",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YtMusicTrack {
    pub video_id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration: Option<String>,
    pub thumbnail_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YtMusicLyrics {
    pub video_id: String,
    pub source: Option<String>,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YtMusicQueueItem {
    pub video_id: String,
    pub title: String,
    pub artist: String,
    pub duration: Option<String>,
}

#[derive(Debug, Clone)]
pub struct YouTubeMusicClient {
    #[cfg_attr(any(target_family = "wasm", not(feature = "native")), allow(dead_code))]
    innertube: InnerTubeClient,
}

impl Default for YouTubeMusicClient {
    fn default() -> Self {
        Self {
            innertube: InnerTubeClient::with_defaults(InnerTubeClientProfile::WebRemix),
        }
    }
}

impl YouTubeMusicClient {
    #[must_use]
    pub fn from_innertube(innertube: InnerTubeClient) -> Self {
        Self { innertube }
    }

    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    pub fn discover() -> Result<Self> {
        Ok(Self {
            innertube: InnerTubeClient::discover(InnerTubeClientProfile::WebRemix)?,
        })
    }

    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    pub fn search(&self, query: &str, filter: Option<YtMusicSearchFilter>) -> Result<Vec<YtMusicTrack>> {
        let mut payload = json!({ "query": query });
        if let Some(selected) = filter {
            payload["params"] = json!(selected.innertube_params());
        }
        let raw = self.innertube.post_endpoint("search", payload)?;
        Ok(extract_music_tracks(&raw))
    }

    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    pub fn home_tracks(&self) -> Result<Vec<YtMusicTrack>> {
        let raw = self
            .innertube
            .post_endpoint("browse", json!({ "browseId": "FEmusic_home" }))?;
        Ok(extract_music_tracks(&raw))
    }

    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    pub fn lyrics_for_video(&self, video_id: &str) -> Result<YtMusicLyrics> {
        let next_raw = self.innertube.post_endpoint(
            "next",
            json!({
                "videoId": video_id,
                "enablePersistentPlaylistPanel": true,
                "isAudioOnly": true
            }),
        )?;
        let Some(lyrics_browse_id) = find_lyrics_browse_id(&next_raw) else {
            return Ok(YtMusicLyrics {
                video_id: video_id.to_string(),
                source: None,
                lines: Vec::new(),
            });
        };
        let browse_raw = self
            .innertube
            .post_endpoint("browse", json!({ "browseId": lyrics_browse_id }))?;
        Ok(parse_lyrics_response(video_id, &browse_raw))
    }
}

/// Recursively parses `musicResponsiveListItemRenderer` items from YouTube Music `WEB_REMIX` JSON.
#[must_use]
pub fn extract_music_tracks(raw: &Value) -> Vec<YtMusicTrack> {
    let mut tracks = Vec::new();
    collect_music_items(raw, &mut tracks);
    tracks
}

fn collect_music_items(value: &Value, out: &mut Vec<YtMusicTrack>) {
    match value {
        Value::Object(map) => {
            if let Some(renderer) = map
                .get("musicResponsiveListItemRenderer")
                .and_then(Value::as_object)
                && let Some(video_id) = renderer
                    .get("playlistItemData")
                    .and_then(|v| v.get("videoId"))
                    .and_then(Value::as_str)
            {
                let columns = renderer
                    .get("flexColumns")
                    .and_then(Value::as_array)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                let title = columns
                    .first()
                    .and_then(|c| c.pointer("/musicResponsiveListItemFlexColumnRenderer/text"))
                    .map(extract_runs_text)
                    .unwrap_or_default();
                let subtitle = columns
                    .get(1)
                    .and_then(|c| c.pointer("/musicResponsiveListItemFlexColumnRenderer/text"))
                    .map(extract_runs_text)
                    .unwrap_or_default();
                let parts: Vec<&str> = subtitle
                    .split('•')
                    .map(str::trim)
                    .filter(|p| !p.is_empty())
                    .collect();
                let artist = parts.first().copied().unwrap_or("Unknown Artist").to_string();
                let album = parts.get(1).map(|s| (*s).to_string());
                let duration = parts.last().filter(|s| s.contains(':')).map(|s| (*s).to_string());
                let thumbnail_url = renderer
                    .get("thumbnail")
                    .and_then(|v| v.pointer("/musicThumbnailRenderer/thumbnail/thumbnails/0/url"))
                    .and_then(Value::as_str)
                    .map(ToString::to_string);
                out.push(YtMusicTrack {
                    video_id: video_id.to_string(),
                    title,
                    artist,
                    album,
                    duration,
                    thumbnail_url,
                });
            }
            for child in map.values() {
                collect_music_items(child, out);
            }
        }
        Value::Array(items) => {
            for child in items {
                collect_music_items(child, out);
            }
        }
        _ => {}
    }
}

#[must_use]
pub fn find_lyrics_browse_id(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => {
            if let Some(browse_id) = map.get("browseId").and_then(Value::as_str)
                && browse_id.starts_with("MPLYt_")
            {
                return Some(browse_id.to_string());
            }
            for child in map.values() {
                if let Some(found) = find_lyrics_browse_id(child) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(items) => {
            for child in items {
                if let Some(found) = find_lyrics_browse_id(child) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

#[must_use]
pub fn parse_lyrics_response(video_id: &str, raw: &Value) -> YtMusicLyrics {
    let mut text = String::new();
    let mut source = None;
    collect_lyrics_shelf(raw, &mut text, &mut source);
    let lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect();
    YtMusicLyrics {
        video_id: video_id.to_string(),
        source,
        lines,
    }
}

fn collect_lyrics_shelf(value: &Value, text: &mut String, source: &mut Option<String>) {
    match value {
        Value::Object(map) => {
            if let Some(shelf) = map
                .get("musicDescriptionShelfRenderer")
                .and_then(Value::as_object)
            {
                if let Some(desc) = shelf.get("description") {
                    *text = extract_runs_text(desc);
                }
                if let Some(footer) = shelf.get("footer") {
                    *source = Some(extract_runs_text(footer));
                }
            }
            for child in map.values() {
                collect_lyrics_shelf(child, text, source);
            }
        }
        Value::Array(items) => {
            for child in items {
                collect_lyrics_shelf(child, text, source);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ytmusic_tracks_and_lyrics_payloads() {
        let search = json!({
            "contents": [{
                "musicResponsiveListItemRenderer": {
                    "playlistItemData": { "videoId": "track_01" },
                    "flexColumns": [
                        { "musicResponsiveListItemFlexColumnRenderer": { "text": { "runs": [{ "text": "Midnight Synth" }] } } },
                        { "musicResponsiveListItemFlexColumnRenderer": { "text": { "runs": [{ "text": "Aphrody Audio • Neon EP • 3:42" }] } } }
                    ]
                }
            }]
        });
        let tracks = extract_music_tracks(&search);
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].video_id, "track_01");
        assert_eq!(tracks[0].title, "Midnight Synth");
        assert_eq!(tracks[0].artist, "Aphrody Audio");
        assert_eq!(tracks[0].duration.as_deref(), Some("3:42"));

        let next = json!({ "tabs": [{ "browseId": "MPLYt_abc123" }] });
        assert_eq!(find_lyrics_browse_id(&next).as_deref(), Some("MPLYt_abc123"));

        let lyrics_raw = json!({
            "musicDescriptionShelfRenderer": {
                "description": { "runs": [{ "text": "Line one\nLine two" }] },
                "footer": { "runs": [{ "text": "Source: LyricFind" }] }
            }
        });
        let lyrics = parse_lyrics_response("track_01", &lyrics_raw);
        assert_eq!(lyrics.lines, vec!["Line one", "Line two"]);
        assert_eq!(lyrics.source.as_deref(), Some("Source: LyricFind"));
    }
}
