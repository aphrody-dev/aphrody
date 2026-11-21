//! Pure-Rust video intelligence & timestamped evidence engine (`watch-skill` parity):
//! - Luminance delta scene-change detection
//! - SHA-256 keyframe fingerprinting and deduplication
//! - Full-text searchable evidence index returning ranked, timestamped receipts.

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyframeSample {
    pub timestamp_ms: u64,
    pub mean_luma: f32,
    pub sha256_hex: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneChangeEvent {
    pub timestamp_ms: u64,
    pub luma_delta: f32,
    pub keyframe_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchEvidenceRecord {
    pub media_id: String,
    pub timestamp_ms: u64,
    pub end_ms: u64,
    pub source_kind: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchEvidenceHit {
    pub record: WatchEvidenceRecord,
    pub score: f32,
}

/// Computes mean luminance (`0.0..=1.0`) and SHA-256 fingerprint for an RGBA8 frame buffer.
#[must_use]
pub fn fingerprint_rgba_frame(timestamp_ms: u64, rgba_bytes: &[u8]) -> KeyframeSample {
    let mut sum_luma = 0.0_f64;
    let mut pixel_count = 0_usize;
    for chunk in rgba_bytes.chunks_exact(4) {
        let r = f64::from(chunk[0]) / 255.0;
        let g = f64::from(chunk[1]) / 255.0;
        let b = f64::from(chunk[2]) / 255.0;
        sum_luma += 0.2126 * r + 0.7152 * g + 0.0722 * b;
        pixel_count += 1;
    }
    let mean_luma = if pixel_count > 0 {
        (sum_luma / pixel_count as f64) as f32
    } else {
        0.0
    };
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(rgba_bytes);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        hex.push(HEX[(byte >> 4) as usize] as char);
        hex.push(HEX[(byte & 0x0f) as usize] as char);
    }
    KeyframeSample {
        timestamp_ms,
        mean_luma,
        sha256_hex: hex,
    }
}

/// Detects scene transitions where mean luminance changes beyond `threshold` (`0.0..=1.0`).
#[must_use]
pub fn detect_scene_changes(samples: &[KeyframeSample], threshold: f32) -> Vec<SceneChangeEvent> {
    let mut events = Vec::new();
    for pair in samples.windows(2) {
        let prev = &pair[0];
        let curr = &pair[1];
        if prev.sha256_hex == curr.sha256_hex {
            continue;
        }
        let delta = (curr.mean_luma - prev.mean_luma).abs();
        if delta >= threshold {
            events.push(SceneChangeEvent {
                timestamp_ms: curr.timestamp_ms,
                luma_delta: delta,
                keyframe_hash: curr.sha256_hex.clone(),
            });
        }
    }
    events
}

/// In-memory & JSON-serializable full-text evidence index for video transcripts, chapters, and OCR.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct WatchIndex {
    records: Vec<WatchEvidenceRecord>,
}

impl WatchIndex {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, record: WatchEvidenceRecord) {
        self.records.push(record);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Searches indexed video evidence by query terms and returns ranked timestamped hits.
    #[must_use]
    pub fn query(&self, query: &str, limit: usize) -> Vec<WatchEvidenceHit> {
        let terms: Vec<String> = query
            .split_whitespace()
            .map(|t| t.to_ascii_lowercase())
            .filter(|t| !t.is_empty())
            .collect();
        if terms.is_empty() {
            return Vec::new();
        }
        let mut hits = Vec::new();
        for record in &self.records {
            let haystack = record.text.to_ascii_lowercase();
            let mut matched_terms = 0_usize;
            for term in &terms {
                if haystack.contains(term) {
                    matched_terms += 1;
                }
            }
            if matched_terms > 0 {
                let score = matched_terms as f32 / terms.len() as f32;
                hits.push(WatchEvidenceHit {
                    record: record.clone(),
                    score,
                });
            }
        }
        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.record.timestamp_ms.cmp(&b.record.timestamp_ms))
        });
        hits.truncate(limit);
        hits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_scene_cuts_and_queries_timestamped_evidence() {
        let dark = [10_u8, 10, 10, 255].repeat(16);
        let bright = [240_u8, 240, 240, 255].repeat(16);
        let s0 = fingerprint_rgba_frame(0, &dark);
        let s1 = fingerprint_rgba_frame(1000, &dark);
        let s2 = fingerprint_rgba_frame(2000, &bright);
        let cuts = detect_scene_changes(&[s0, s1, s2], 0.35);
        assert_eq!(cuts.len(), 1);
        assert_eq!(cuts[0].timestamp_ms, 2000);

        let mut index = WatchIndex::new();
        index.insert(WatchEvidenceRecord {
            media_id: "demo".to_string(),
            timestamp_ms: 12_000,
            end_ms: 18_000,
            source_kind: "transcript".to_string(),
            text: "Configuring Material 3 Expressive liquid glass shaders in GPUI".to_string(),
        });
        let hits = index.query("material liquid gpui", 5);
        assert_eq!(hits.len(), 1);
        assert!((hits[0].score - 1.0).abs() < 1e-5);
        assert_eq!(hits[0].record.timestamp_ms, 12_000);
    }
}
