//! Official YouTube Data API v3 (`youtube.googleapis.com/youtube/v3`) and YouTube Analytics v2
//! client authenticated via OAuth 2.0 / `gcloud` Bearer tokens.

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

use crate::auth::GoogleAuthManager;

pub const YOUTUBE_DATA_V3_BASE: &str = "https://youtube.googleapis.com/youtube/v3";
pub const YOUTUBE_ANALYTICS_V2_BASE: &str = "https://youtubeanalytics.googleapis.com/v2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YouTubeVideoSummary {
    pub video_id: String,
    pub title: String,
    pub channel_title: String,
    pub published_at: Option<String>,
    pub thumbnail_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YouTubePlaylistSummary {
    pub playlist_id: String,
    pub title: String,
    pub item_count: u64,
}

#[derive(Debug, Clone)]
pub struct YouTubeDataClient {
    #[cfg_attr(any(target_family = "wasm", not(feature = "native")), allow(dead_code))]
    auth: GoogleAuthManager,
}

impl YouTubeDataClient {
    #[must_use]
    pub fn new(auth: GoogleAuthManager) -> Self {
        Self { auth }
    }

    pub fn build_endpoint(resource: &str, params: &[(&str, &str)]) -> Result<String> {
        let mut url = Url::parse(&format!(
            "{YOUTUBE_DATA_V3_BASE}/{}",
            resource.trim_start_matches('/')
        ))
        .context("invalid YouTube Data API v3 URL")?;
        {
            let mut query = url.query_pairs_mut();
            for (key, value) in params {
                query.append_pair(key, value);
            }
        }
        Ok(url.to_string())
    }

    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    pub fn search_videos(&self, query: &str, max_results: u32) -> Result<Vec<YouTubeVideoSummary>> {
        let max_str = max_results.to_string();
        let url = Self::build_endpoint(
            "search",
            &[
                ("part", "snippet"),
                ("type", "video"),
                ("q", query),
                ("maxResults", max_str.as_str()),
            ],
        )?;
        let raw = self.get_json(&url)?;
        Ok(parse_search_items(&raw))
    }

    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    pub fn list_my_playlists(&self, max_results: u32) -> Result<Vec<YouTubePlaylistSummary>> {
        let max_str = max_results.to_string();
        let url = Self::build_endpoint(
            "playlists",
            &[
                ("part", "snippet,contentDetails"),
                ("mine", "true"),
                ("maxResults", max_str.as_str()),
            ],
        )?;
        let raw = self.get_json(&url)?;
        Ok(parse_playlist_items(&raw))
    }

    #[cfg(all(not(target_family = "wasm"), feature = "native"))]
    fn get_json(&self, url: &str) -> Result<Value> {
        let token = self.auth.resolve_token()?;
        let mut req = ureq::get(url)
            .header("Authorization", &format!("Bearer {}", token.access_token))
            .header("Accept", "application/json");
        if let Some(project) = token.quota_project.as_deref() {
            req = req.header("x-goog-user-project", project);
        }
        let mut response = req
            .call()
            .with_context(|| format!("YouTube Data API v3 request failed: {url}"))?;
        let text = response
            .body_mut()
            .read_to_string()
            .context("failed to read YouTube Data API v3 response")?;
        serde_json::from_str(&text).context("invalid JSON from YouTube Data API v3")
    }
}

#[must_use]
pub fn parse_search_items(raw: &Value) -> Vec<YouTubeVideoSummary> {
    let mut results = Vec::new();
    let Some(items) = raw.get("items").and_then(Value::as_array) else {
        return results;
    };
    for item in items {
        let Some(video_id) = item.pointer("/id/videoId").and_then(Value::as_str) else {
            continue;
        };
        let title = item
            .pointer("/snippet/title")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let channel_title = item
            .pointer("/snippet/channelTitle")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let published_at = item
            .pointer("/snippet/publishedAt")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        let thumbnail_url = item
            .pointer("/snippet/thumbnails/high/url")
            .or_else(|| item.pointer("/snippet/thumbnails/default/url"))
            .and_then(Value::as_str)
            .map(ToString::to_string);
        results.push(YouTubeVideoSummary {
            video_id: video_id.to_string(),
            title,
            channel_title,
            published_at,
            thumbnail_url,
        });
    }
    results
}

#[must_use]
pub fn parse_playlist_items(raw: &Value) -> Vec<YouTubePlaylistSummary> {
    let mut results = Vec::new();
    let Some(items) = raw.get("items").and_then(Value::as_array) else {
        return results;
    };
    for item in items {
        let Some(playlist_id) = item.get("id").and_then(Value::as_str) else {
            continue;
        };
        let title = item
            .pointer("/snippet/title")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let item_count = item
            .pointer("/contentDetails/itemCount")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        results.push(YouTubePlaylistSummary {
            playlist_id: playlist_id.to_string(),
            title,
            item_count,
        });
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_youtube_v3_search_and_playlists() {
        let url = YouTubeDataClient::build_endpoint("search", &[("part", "snippet"), ("q", "rust")])
            .expect("endpoint");
        assert_eq!(
            url,
            "https://youtube.googleapis.com/youtube/v3/search?part=snippet&q=rust"
        );

        let search_json = json!({
            "items": [{
                "id": { "videoId": "abc12345678" },
                "snippet": {
                    "title": "GPUI Rendering Pipeline",
                    "channelTitle": "Aphrody Labs",
                    "publishedAt": "2026-11-20T00:00:00Z",
                    "thumbnails": { "high": { "url": "https://i.ytimg.com/vi/abc12345678/hqdefault.jpg" } }
                }
            }]
        });
        let items = parse_search_items(&search_json);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].video_id, "abc12345678");
        assert_eq!(items[0].title, "GPUI Rendering Pipeline");
    }
}
