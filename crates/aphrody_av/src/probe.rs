use crate::demux::{self, codec_label, kind_label};
use crate::error::AvResult;
use crate::model::{Demuxed, Track, TrackKind};
use craft_media::CodecId;
use craft_media::isobmff::AvcConfig;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct StreamInfo {
    pub index: usize,
    pub kind: &'static str,
    pub codec: String,
    pub codec_tag: Option<String>,
    /// Codec profile as `ffprobe` names it (H.264 only, from the `avcC` record).
    pub profile: Option<String>,
    /// Pixel format as `ffprobe` names it (H.264 only, from the first SPS).
    pub pix_fmt: Option<String>,
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
            let (profile, pix_fmt) = avc_description(track).unzip();
            StreamInfo {
                index: track.index,
                kind: kind_label(track.kind),
                codec: codec_label(track),
                codec_tag: track
                    .params
                    .tag
                    .map(|tag| String::from_utf8_lossy(&tag).trim().to_string()),
                profile,
                pix_fmt,
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

/// Profile and pixel format of an H.264 track whose extradata is an `avcC` record.
fn avc_description(track: &Track) -> Option<(String, String)> {
    if track.params.codec != CodecId::H264 {
        return None;
    }
    let config = AvcConfig::parse(&track.params.extradata).ok()?;
    let sps = rbsp(config.sps.first()?.get(4..)?);
    let mut bits = BitReader::new(&sps);
    bits.exp_golomb()?;
    let (mut chroma, mut depth) = (1, 8);
    if matches!(
        config.profile,
        100 | 110 | 122 | 244 | 44 | 83 | 86 | 118 | 128 | 138 | 139 | 134 | 135
    ) {
        chroma = bits.exp_golomb()?;
        if chroma == 3 {
            bits.bit()?;
        }
        depth = 8 + bits.exp_golomb()?;
    }
    let profile = match config.profile {
        66 if config.compatibility & 0x40 != 0 => "Constrained Baseline",
        66 => "Baseline",
        77 => "Main",
        88 => "Extended",
        100 if config.compatibility & 0x0c == 0x0c => "Constrained High",
        100 => "High",
        110 => "High 10",
        122 => "High 4:2:2",
        244 => "High 4:4:4 Predictive",
        44 => "High 4:4:4 Intra",
        other => return Some((format!("profile {other}"), pixel_format(chroma, depth))),
    };
    Some((profile.to_owned(), pixel_format(chroma, depth)))
}

fn pixel_format(chroma: u32, depth: u32) -> String {
    let base = match chroma {
        0 => "gray",
        1 => "yuv420p",
        2 => "yuv422p",
        _ => "yuv444p",
    };
    if depth > 8 {
        format!("{base}{depth}le")
    } else {
        base.to_owned()
    }
}

/// NAL payload with the emulation prevention bytes (`00 00 03`) removed.
fn rbsp(nal: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(nal.len());
    let mut zeros = 0;
    for &byte in nal {
        if zeros >= 2 && byte == 3 {
            zeros = 0;
            continue;
        }
        zeros = if byte == 0 { zeros + 1 } else { 0 };
        out.push(byte);
    }
    out
}

struct BitReader<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        BitReader { data, position: 0 }
    }

    fn bit(&mut self) -> Option<u32> {
        let byte = *self.data.get(self.position / 8)?;
        let bit = (byte >> (7 - self.position % 8)) & 1;
        self.position += 1;
        Some(u32::from(bit))
    }

    fn exp_golomb(&mut self) -> Option<u32> {
        let mut zeros = 0;
        while self.bit()? == 0 {
            zeros += 1;
            if zeros > 31 {
                return None;
            }
        }
        let mut value = 0u32;
        for _ in 0..zeros {
            value = (value << 1) | self.bit()?;
        }
        Some((1u32 << zeros) - 1 + value)
    }
}
