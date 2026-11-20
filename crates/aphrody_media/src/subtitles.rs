//! Pure-Rust SRT and WebVTT subtitle parser with Unicode Bidi direction resolution.

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use unicode_bidi::BidiInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubtitleFormat {
    Srt,
    WebVtt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubtitleCue {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    pub is_rtl: bool,
}

/// Parses SRT or WebVTT subtitles into sorted [`SubtitleCue`] entries.
pub fn parse_subtitles(format: SubtitleFormat, input: &str) -> Result<Vec<SubtitleCue>> {
    let normalized = input.replace("\r\n", "\n");
    let body = match format {
        SubtitleFormat::WebVtt => normalized
            .trim_start_matches('\u{feff}')
            .strip_prefix("WEBVTT")
            .unwrap_or(normalized.as_str()),
        SubtitleFormat::Srt => normalized.trim_start_matches('\u{feff}'),
    };

    let mut cues = Vec::new();
    for block in body.split("\n\n") {
        let lines: Vec<&str> = block
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        if lines.is_empty() {
            continue;
        }
        let Some(arrow_idx) = lines.iter().position(|line| line.contains("-->")) else {
            continue;
        };
        let timing_line = lines[arrow_idx];
        let mut parts = timing_line.split("-->");
        let Some(start_raw) = parts.next().map(str::trim) else {
            continue;
        };
        let Some(end_raw) = parts
            .next()
            .and_then(|rest| rest.split_whitespace().next())
            .map(str::trim)
        else {
            continue;
        };
        let start_ms = parse_timestamp_ms(start_raw)?;
        let end_ms = parse_timestamp_ms(end_raw)?;
        let text = lines[arrow_idx + 1..].join("\n");
        if text.is_empty() {
            continue;
        }
        let bidi = BidiInfo::new(&text, None);
        let is_rtl = bidi
            .paragraphs
            .first()
            .map(|paragraph| paragraph.level.is_rtl())
            .unwrap_or(false);
        cues.push(SubtitleCue {
            start_ms,
            end_ms,
            text,
            is_rtl,
        });
    }
    cues.sort_by_key(|cue| cue.start_ms);
    Ok(cues)
}

fn parse_timestamp_ms(raw: &str) -> Result<u64> {
    let cleaned = raw.replace(',', ".");
    let segments: Vec<&str> = cleaned.split(':').collect();
    let (hours, minutes, sec_part) = match segments.as_slice() {
        [mm, ss] => (0_u64, mm.parse::<u64>()?, *ss),
        [hh, mm, ss] => (hh.parse::<u64>()?, mm.parse::<u64>()?, *ss),
        _ => bail!("invalid subtitle timestamp: {raw}"),
    };
    let mut sec_split = sec_part.split('.');
    let secs = sec_split.next().unwrap_or("0").parse::<u64>()?;
    let millis_str = sec_split.next().unwrap_or("0");
    let padded_millis = match millis_str.len() {
        0 => 0_u64,
        1 => millis_str.parse::<u64>()? * 100,
        2 => millis_str.parse::<u64>()? * 10,
        _ => millis_str[..3].parse::<u64>()?,
    };
    Ok((((hours * 60 + minutes) * 60) + secs) * 1000 + padded_millis)
}

/// Finds all subtitle cues active at a given playback timestamp in milliseconds.
#[must_use]
pub fn active_cues_at(cues: &[SubtitleCue], position_ms: u64) -> Vec<&SubtitleCue> {
    cues.iter()
        .filter(|cue| position_ms >= cue.start_ms && position_ms <= cue.end_ms)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_srt_and_webvtt_with_bidi() {
        let vtt = "WEBVTT\n\n00:01.500 --> 00:04.250\nHello Aphrody\n\n00:05.000 --> 00:08.000\nمرحبا بكم\n";
        let cues = parse_subtitles(SubtitleFormat::WebVtt, vtt).expect("vtt cues");
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].start_ms, 1500);
        assert_eq!(cues[0].end_ms, 4250);
        assert!(!cues[0].is_rtl);
        assert!(cues[1].is_rtl);
        assert_eq!(active_cues_at(&cues, 2000).len(), 1);
    }
}
