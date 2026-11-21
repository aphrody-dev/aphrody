use crate::demux::{self, codec_label, kind_label};
use crate::error::AvResult;
use crate::model::{Demuxed, TrackKind};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct StreamInfo {
    pub index: usize,
    pub kind: &'static str,
    pub codec: String,
    pub codec_tag: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub frame_rate: Option<f64>,
    pub frame_rate_ratio: Option<String>,
    pub frames: Option<u64>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub duration_seconds: Option<f64>,
    pub bit_rate: Option<u64>,
    pub language: Option<String>,
    pub extradata_bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProbeInfo {
    pub format: &'static str,
    pub format_long: &'static str,
    pub size_bytes: u64,
    pub duration_seconds: Option<f64>,
    pub bit_rate: Option<u64>,
    pub packetized: bool,
    pub streams: Vec<StreamInfo>,
}

/// Container, streams, codec parameters and duration of `bytes`. `hint` is a file name or extension.
pub fn probe(bytes: &[u8], hint: Option<&str>) -> AvResult<ProbeInfo> {
    let (demuxed, packetized) = demux::demux(bytes, hint)?;
    Ok(describe(&demuxed, bytes.len() as u64, packetized))
}

pub fn describe(demuxed: &Demuxed, size: u64, packetized: bool) -> ProbeInfo {
    let duration = demuxed.duration_seconds();
    let streams = demuxed
        .tracks
        .iter()
        .map(|track| {
            let scale = f64::from(track.timescale.max(1));
            let mut frames = 0u64;
            let mut bytes = 0u64;
            let (mut first, mut last) = (i64::MAX, i64::MIN);
            for packet in demuxed.track_packets(track.index) {
                frames += 1;
                bytes += packet.data.len() as u64;
                first = first.min(packet.pts);
                last = last.max(packet.pts + packet.duration.max(0));
            }
            let stream_duration = track
                .duration
                .map(|ticks| ticks as f64 / scale)
                .filter(|d| *d > 0.0)
                .or_else(|| (frames > 0 && last > first).then(|| (last - first) as f64 / scale));
            let bit_rate = stream_duration
                .filter(|d| *d > 0.0 && bytes > 0)
                .map(|d| (bytes as f64 * 8.0 / d) as u64);
            let (rate, ratio) = match track.frame_rate {
                Some((n, d)) if track.kind == TrackKind::Video => (
                    Some(f64::from(n) / f64::from(d.max(1))),
                    Some(format!("{n}/{d}")),
                ),
                _ => (None, None),
            };
            StreamInfo {
                index: track.index,
                kind: kind_label(track.kind),
                codec: codec_label(track),
                codec_tag: track
                    .params
                    .tag
                    .map(|tag| String::from_utf8_lossy(&tag).trim().to_string()),
                width: track.params.width,
                height: track.params.height,
                frame_rate: rate,
                frame_rate_ratio: ratio,
                frames: (frames > 0 && track.kind == TrackKind::Video).then_some(frames),
                sample_rate: track.params.sample_rate,
                channels: track.params.channels,
                duration_seconds: stream_duration,
                bit_rate,
                language: track.language.clone(),
                extradata_bytes: track.params.extradata.len(),
            }
        })
        .collect();
    let total_bytes: u64 = demuxed.packets.iter().map(|p| p.data.len() as u64).sum();
    let overall = duration
        .filter(|d| *d > 0.0 && total_bytes > 0)
        .map(|d| (total_bytes as f64 * 8.0 / d) as u64);
    ProbeInfo {
        format: demuxed.format.name(),
        format_long: demuxed.format.long_name(),
        size_bytes: size,
        duration_seconds: duration,
        bit_rate: overall,
        packetized,
        streams,
    }
}
