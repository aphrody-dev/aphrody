//! Pure-Rust network media ingest parsers:
//! - HLS (`.m3u8` master & media playlists, `#EXT-X-KEY` AES-128 detection, `#EXTINF` durations)
//! - DASH (`.mpd` Representation / AdaptationSet parser)
//! - Source classification (YouTube, YouTube Music, HLS, DASH, Direct HTTP, Long-tail `yt-dlp` fallback).

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestSourceKind {
    YouTube,
    YouTubeMusic,
    HlsPlaylist,
    DashManifest,
    DirectMedia,
    ExternalExtractorFallback,
}

/// Classifies an incoming media URL for pure-Rust routing before any external fallback.
#[must_use]
pub fn classify_ingest_url(url: &str) -> IngestSourceKind {
    let lower = url.to_ascii_lowercase();
    if lower.contains("music.youtube.com") {
        IngestSourceKind::YouTubeMusic
    } else if lower.contains("youtube.com/") || lower.contains("youtu.be/") {
        IngestSourceKind::YouTube
    } else if lower.contains(".m3u8") {
        IngestSourceKind::HlsPlaylist
    } else if lower.contains(".mpd") {
        IngestSourceKind::DashManifest
    } else if [
        ".mp4", ".webm", ".mkv", ".mov", ".mp3", ".opus", ".flac", ".wav", ".m4a", ".ogg",
    ]
    .iter()
    .any(|ext| lower.contains(ext))
    {
        IngestSourceKind::DirectMedia
    } else {
        IngestSourceKind::ExternalExtractorFallback
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HlsSegment {
    pub uri: String,
    pub duration_secs: f64,
    pub encrypted_aes128: bool,
    pub key_uri: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HlsPlaylist {
    pub is_master: bool,
    pub target_duration_secs: Option<u64>,
    pub variants: Vec<String>,
    pub segments: Vec<HlsSegment>,
}

/// Parses an HLS `.m3u8` master or media playlist in pure Rust.
pub fn parse_hls_m3u8(content: &str) -> Result<HlsPlaylist> {
    let mut is_master = false;
    let mut target_duration_secs = None;
    let mut variants = Vec::new();
    let mut segments = Vec::new();
    let mut pending_duration: Option<f64> = None;
    let mut expecting_variant_uri = false;
    let mut encrypted_aes128 = false;
    let mut current_key_uri: Option<String> = None;

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("#EXT-X-TARGETDURATION:") {
            target_duration_secs = rest.trim().parse::<u64>().ok();
        } else if line.starts_with("#EXT-X-STREAM-INF:") {
            is_master = true;
            expecting_variant_uri = true;
        } else if let Some(rest) = line.strip_prefix("#EXT-X-KEY:") {
            encrypted_aes128 = rest.contains("METHOD=AES-128");
            current_key_uri = extract_attr(rest, "URI");
        } else if let Some(rest) = line.strip_prefix("#EXTINF:") {
            let num_part = rest.split(',').next().unwrap_or("0").trim();
            pending_duration = num_part.parse::<f64>().ok();
        } else if line.starts_with('#') {
            continue;
        } else if expecting_variant_uri {
            variants.push(line.to_string());
            expecting_variant_uri = false;
        } else {
            segments.push(HlsSegment {
                uri: line.to_string(),
                duration_secs: pending_duration.take().unwrap_or(0.0),
                encrypted_aes128,
                key_uri: current_key_uri.clone(),
            });
        }
    }

    Ok(HlsPlaylist {
        is_master,
        target_duration_secs,
        variants,
        segments,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashRepresentation {
    pub id: String,
    pub bandwidth: u64,
    pub mime_type: Option<String>,
    pub codecs: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// Extracts `<Representation ...>` entries from a DASH `.mpd` XML manifest.
pub fn parse_dash_mpd(xml: &str) -> Result<Vec<DashRepresentation>> {
    let mut reps = Vec::new();
    let mut cursor = xml;
    while let Some(pos) = cursor.find("<Representation") {
        let tail = &cursor[pos..];
        let Some(end) = tail.find('>') else {
            break;
        };
        let tag = &tail[..end];
        reps.push(DashRepresentation {
            id: extract_attr(tag, "id").unwrap_or_else(|| format!("rep_{}", reps.len())),
            bandwidth: extract_attr(tag, "bandwidth")
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0),
            mime_type: extract_attr(tag, "mimeType"),
            codecs: extract_attr(tag, "codecs"),
            width: extract_attr(tag, "width").and_then(|v| v.parse::<u32>().ok()),
            height: extract_attr(tag, "height").and_then(|v| v.parse::<u32>().ok()),
        });
        cursor = &tail[end + 1..];
    }
    Ok(reps)
}

fn extract_attr(source: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=\"");
    let bytes = source.as_bytes();
    let mut offset = 0;
    while let Some(rel) = source[offset..].find(&needle) {
        let pos = offset + rel;
        let boundary_ok = pos == 0
            || matches!(
                bytes[pos - 1],
                b' ' | b'\t' | b'\n' | b'\r' | b',' | b'<'
            );
        if boundary_ok {
            let rest = &source[pos + needle.len()..];
            let end = rest.find('"')?;
            return Some(rest[..end].to_string());
        }
        offset = pos + needle.len();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hls_aes128_and_dash_mpd_and_classifies_urls() {
        assert_eq!(
            classify_ingest_url("https://music.youtube.com/watch?v=abc"),
            IngestSourceKind::YouTubeMusic
        );
        assert_eq!(
            classify_ingest_url("https://www.youtube.com/watch?v=abc"),
            IngestSourceKind::YouTube
        );

        let m3u8 = "#EXTM3U\n#EXT-X-TARGETDURATION:6\n#EXT-X-KEY:METHOD=AES-128,URI=\"https://cdn.example/key.bin\"\n#EXTINF:5.5,\nseg0.ts\n#EXTINF:6.0,\nseg1.ts\n";
        let hls = parse_hls_m3u8(m3u8).expect("hls");
        assert!(!hls.is_master);
        assert_eq!(hls.target_duration_secs, Some(6));
        assert_eq!(hls.segments.len(), 2);
        assert!(hls.segments[0].encrypted_aes128);
        assert_eq!(
            hls.segments[0].key_uri.as_deref(),
            Some("https://cdn.example/key.bin")
        );

        let mpd = r#"<MPD><Period><AdaptationSet><Representation id="v1" bandwidth="2500000" mimeType="video/mp4" codecs="av01.0.08M.08" width="1920" height="1080"/></AdaptationSet></Period></MPD>"#;
        let dash = parse_dash_mpd(mpd).expect("dash");
        assert_eq!(dash.len(), 1);
        assert_eq!(dash[0].width, Some(1920));
        assert_eq!(dash[0].height, Some(1080));
    }
}
