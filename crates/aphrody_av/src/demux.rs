use crate::error::{AvError, AvResult};
use crate::model::{Codec, Demuxed, Format, Packet, Track, TrackKind};
use crate::video::VideoDecoder;
use craft_media::{CodecId, CodecParams};

/// Largest input the in-memory API accepts (a guard against hostile size claims, not a format limit).
pub const MAX_INPUT_BYTES: u64 = 4 << 30;

/// Identify the container from its first bytes; `hint` is a file name or extension used only when
/// the bytes are ambiguous.
pub fn sniff(bytes: &[u8], hint: Option<&str>) -> Option<Format> {
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        return Some(if &bytes[8..12] == b"qt  " {
            Format::Mov
        } else {
            Format::Mp4
        });
    }
    if bytes.len() >= 8
        && matches!(
            &bytes[4..8],
            b"moov" | b"mdat" | b"wide" | b"free" | b"skip"
        )
    {
        return Some(Format::Mov);
    }
    if bytes.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
        let head = &bytes[..bytes.len().min(96)];
        return Some(if head.windows(4).any(|w| w == b"webm") {
            Format::WebM
        } else {
            Format::Matroska
        });
    }
    if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"AVI " {
        return Some(Format::Avi);
    }
    if bytes.starts_with(b"OggS") {
        return Some(Format::Ogg);
    }
    if craft_media::mxf::sniff(&bytes[..bytes.len().min(70_000)]) {
        return Some(Format::Mxf);
    }
    if let Some(container) = craft_audio::detect_container(bytes, None) {
        use craft_media::Container as C;
        if !matches!(
            container,
            C::Mp4 | C::QuickTime | C::Matroska | C::WebM | C::Avi
        ) {
            return Some(Format::from_container(container));
        }
    }
    if let Some(layout) = craft_media::mpegts::sniff(&bytes[..bytes.len().min(70_000)]) {
        return Some(match layout {
            craft_media::mpegts::Format::Ts { .. } => Format::MpegTs,
            craft_media::mpegts::Format::Ps { .. } => Format::MpegPs,
        });
    }
    let _ = hint;
    None
}

/// Read every track and packet of `bytes`. Formats without a packet demuxer here (audio-only raw
/// files, Ogg) return their tracks with no packets and `packetized == false`.
pub fn demux(bytes: &[u8], hint: Option<&str>) -> AvResult<(Demuxed, bool)> {
    if bytes.is_empty() {
        return Err(AvError::malformed("empty input"));
    }
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err(AvError::Denied(format!(
            "input of {} bytes exceeds the {MAX_INPUT_BYTES} byte limit",
            bytes.len()
        )));
    }
    let format = sniff(bytes, hint).ok_or_else(|| {
        AvError::malformed("unrecognised container: no known signature in the first bytes")
    })?;
    let (mut demuxed, packetized) = match format {
        Format::Mp4 | Format::Mov => (demux_isobmff(bytes, format)?, true),
        Format::Matroska | Format::WebM => (demux_matroska(bytes, format)?, true),
        Format::Avi => (demux_avi(bytes)?, true),
        Format::MpegTs | Format::MpegPs => (demux_mpegts(bytes, format)?, true),
        Format::Mxf => (demux_mxf(bytes)?, true),
        Format::Ogg => (demux_ogg(bytes)?, false),
        _ => (demux_raw_audio(bytes, format, hint)?, false),
    };
    finish(&mut demuxed);
    Ok((demuxed, packetized))
}

fn finish(demuxed: &mut Demuxed) {
    synthesize_decode_times(demuxed);
    fill_missing_sizes(demuxed);
    for track in 0..demuxed.tracks.len() {
        if demuxed.tracks[track].kind == TrackKind::Video
            && demuxed.tracks[track].frame_rate.is_none()
        {
            let rate = frame_rate_of(demuxed, track);
            demuxed.tracks[track].frame_rate = rate;
        }
        let name = match demuxed.tracks[track].codec() {
            Some(codec) => codec.name().to_string(),
            None => demuxed.tracks[track].params.codec.name().to_string(),
        };
        demuxed.tracks[track].codec_name = name;
    }
    let scales: Vec<u32> = demuxed
        .tracks
        .iter()
        .map(|track| track.timescale.max(1))
        .collect();
    demuxed.packets.sort_by_key(|packet| {
        let scale = i128::from(scales.get(packet.track).copied().unwrap_or(1));
        (i128::from(packet.dts) * 1_000_000_000 / scale, packet.track)
    });
}

/// Containers without decode timestamps (Matroska) give presentation order of reordered video;
/// replace dts by the sorted presentation times so it is monotonic, as MP4 and AVI need.
fn synthesize_decode_times(demuxed: &mut Demuxed) {
    for track in 0..demuxed.tracks.len() {
        if demuxed.tracks[track].kind != TrackKind::Video {
            continue;
        }
        let indices: Vec<usize> = demuxed
            .packets
            .iter()
            .enumerate()
            .filter(|(_, p)| p.track == track)
            .map(|(i, _)| i)
            .collect();
        let monotonic = indices
            .windows(2)
            .all(|pair| demuxed.packets[pair[0]].dts <= demuxed.packets[pair[1]].dts);
        if monotonic {
            continue;
        }
        let mut times: Vec<i64> = indices.iter().map(|&i| demuxed.packets[i].pts).collect();
        times.sort_unstable();
        for (&packet_index, time) in indices.iter().zip(times) {
            demuxed.packets[packet_index].dts = time;
        }
    }
}

/// Frames per second as a reduced rational, from the span of presentation times.
pub fn frame_rate_of(demuxed: &Demuxed, track: usize) -> Option<(u32, u32)> {
    let timescale = u64::from(demuxed.tracks.get(track)?.timescale.max(1));
    let mut times: Vec<i64> = demuxed
        .track_packets(track)
        .map(|packet| packet.pts)
        .collect();
    if times.len() < 2 {
        return None;
    }
    times.sort_unstable();
    let span = u64::try_from(times[times.len() - 1] - times[0])
        .ok()
        .filter(|&span| span > 0)?;
    let intervals = (times.len() - 1) as u64;
    Some(snap_rate(intervals.checked_mul(timescale)?, span))
}

/// Round `numerator / denominator` frames per second to a standard rate when within 0.5 %,
/// otherwise to thousandths.
pub fn snap_rate(numerator: u64, denominator: u64) -> (u32, u32) {
    let fps = numerator as f64 / denominator.max(1) as f64;
    const STANDARD: [(u32, u32); 14] = [
        (24000, 1001),
        (24, 1),
        (25, 1),
        (30000, 1001),
        (30, 1),
        (48, 1),
        (50, 1),
        (60000, 1001),
        (60, 1),
        (120, 1),
        (15, 1),
        (12, 1),
        (10, 1),
        (1, 1),
    ];
    for tolerance in [0.0002, 0.004] {
        for (n, d) in STANDARD {
            let candidate = f64::from(n) / f64::from(d);
            if (fps - candidate).abs() / candidate < tolerance {
                return (n, d);
            }
        }
    }
    let scaled = (fps * 1000.0).round().max(1.0);
    let scaled = scaled.min(f64::from(u32::MAX)) as u32;
    let g = gcd(scaled, 1000);
    (scaled / g, 1000 / g)
}

pub fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a.max(1) } else { gcd(b, a % b) }
}

fn io<E: std::fmt::Display>(label: &str) -> impl Fn(E) -> AvError + '_ {
    move |error| AvError::malformed(format!("{label}: {error}"))
}

fn demux_isobmff(bytes: &[u8], format: Format) -> AvResult<Demuxed> {
    use craft_media::isobmff::{self, TrackKind as K};
    let file = isobmff::open(bytes).map_err(io("mp4"))?;
    let mut tracks = Vec::new();
    let mut packets = Vec::new();
    for (index, track) in file.tracks.iter().enumerate() {
        let kind = match track.kind {
            K::Video => TrackKind::Video,
            K::Audio => TrackKind::Audio,
            _ => TrackKind::Other,
        };
        let mut params = track.codec_params();
        if kind == TrackKind::Video && params.width.is_none() && track.width > 0 {
            params = params.with_size(track.width, track.height);
        }
        let frame_rate = (kind == TrackKind::Video
            && !track.samples.is_empty()
            && track.duration > 0)
            .then(|| {
                snap_rate(
                    track.samples.len() as u64 * u64::from(track.timescale),
                    track.duration,
                )
            });
        tracks.push(Track {
            index,
            kind,
            params,
            timescale: track.timescale.max(1),
            duration: Some(track.duration),
            language: Some(track.language.clone()).filter(|l| !l.is_empty()),
            frame_rate: frame_rate.filter(|_| track.samples.len() > 1),
            codec_name: String::new(),
        });
        if kind == TrackKind::Other {
            continue;
        }
        for (sample_index, sample) in track.samples.iter().enumerate() {
            let data = file
                .read_sample(bytes, index, sample_index)
                .map_err(io("mp4 sample"))?;
            packets.push(Packet {
                track: index,
                data,
                pts: sample.pts,
                dts: sample.dts,
                duration: i64::from(sample.duration),
                key: sample.is_sync,
            });
        }
    }
    Ok(Demuxed {
        format,
        tracks,
        packets,
    })
}

fn demux_matroska(bytes: &[u8], format: Format) -> AvResult<Demuxed> {
    use craft_media::matroska::{self, TrackKind as K};
    let file = matroska::open(bytes).map_err(io("matroska"))?;
    let mut tracks = Vec::new();
    let mut packets = Vec::new();
    for (index, track) in file.tracks.iter().enumerate() {
        let kind = match track.kind {
            K::Video => TrackKind::Video,
            K::Audio => TrackKind::Audio,
            _ => TrackKind::Other,
        };
        let (numerator, denominator) = track.timebase;
        let timescale = u32::try_from(denominator / numerator.max(1))
            .unwrap_or(1000)
            .max(1);
        let mut params = track.codec_params();
        if params.codec == CodecId::Unknown {
            let id = track.codec_id.as_str();
            let mapped = if id.starts_with("V_MPEG2") || id.starts_with("V_MPEG1") {
                Some(CodecId::Mpeg2Video)
            } else if id.starts_with("V_MPEG4/ISO/SP")
                || id.starts_with("V_MPEG4/ISO/ASP")
                || id.starts_with("V_MPEG4/ISO/AP")
            {
                Some(CodecId::Mpeg4Video)
            } else {
                None
            };
            if let Some(mapped) = mapped {
                let size = params.width.zip(params.height);
                params = CodecParams::new(mapped).with_extradata(track.codec_private.clone());
                if let Some((w, h)) = size {
                    params = params.with_size(w, h);
                }
            }
        }
        if let Some(video) = &track.video {
            if params.width.is_none() {
                params = params.with_size(video.pixel_width, video.pixel_height);
            }
        }
        let frame_rate = track
            .default_duration_ns
            .filter(|&ns| ns > 0 && kind == TrackKind::Video)
            .map(|ns| snap_rate(1_000_000_000, ns));
        tracks.push(Track {
            index,
            kind,
            params,
            timescale,
            duration: file
                .duration_ns()
                .map(|ns| (u128::from(ns) * u128::from(timescale) / 1_000_000_000) as u64),
            language: Some(track.language.clone()).filter(|l| !l.is_empty()),
            frame_rate,
            codec_name: String::new(),
        });
        if kind == TrackKind::Other {
            continue;
        }
        for (sample_index, sample) in track.samples.iter().enumerate() {
            let data = file
                .read_sample(bytes, index, sample_index)
                .map_err(io("matroska frame"))?;
            let mut duration = sample.duration as i64;
            if duration == 0 {
                if let Some(ns) = track.default_duration_ns {
                    duration = (u128::from(ns) * u128::from(timescale) / 1_000_000_000) as i64;
                }
            }
            packets.push(Packet {
                track: index,
                data,
                pts: sample.pts,
                dts: sample.pts,
                duration,
                key: sample.keyframe,
            });
        }
    }
    Ok(Demuxed {
        format,
        tracks,
        packets,
    })
}

fn demux_avi(bytes: &[u8]) -> AvResult<Demuxed> {
    use craft_media::avi::{self, StreamKind};
    let file = avi::open(bytes).map_err(io("avi"))?;
    let mut tracks = Vec::new();
    let mut packets = Vec::new();
    for (index, stream) in file.streams.iter().enumerate() {
        let kind = match stream.kind {
            StreamKind::Video => TrackKind::Video,
            StreamKind::Audio => TrackKind::Audio,
            _ => TrackKind::Other,
        };
        let params = stream.codec_params();
        let pcm_bytes = kind == TrackKind::Audio && stream.sample_size > 0;
        let timescale = if pcm_bytes {
            params.sample_rate.unwrap_or(stream.rate.max(1))
        } else {
            stream.rate.max(1)
        };
        let frame_rate = (kind == TrackKind::Video && stream.scale > 0).then(|| {
            let g = gcd(stream.rate, stream.scale);
            (stream.rate / g, stream.scale / g)
        });
        tracks.push(Track {
            index,
            kind,
            params,
            timescale,
            duration: None,
            language: None,
            frame_rate,
            codec_name: String::new(),
        });
        if kind == TrackKind::Other {
            continue;
        }
        let mut sample_clock = 0i64;
        for (chunk_index, chunk) in stream.chunks.iter().enumerate() {
            let data = file
                .read_chunk(bytes, index, chunk_index)
                .map_err(io("avi chunk"))?;
            let (pts, duration) = if pcm_bytes {
                let frames = i64::from(chunk.size) / i64::from(stream.sample_size.max(1));
                let pts = sample_clock;
                sample_clock += frames;
                (pts, frames)
            } else {
                (
                    chunk_index as i64 * i64::from(stream.scale.max(1)),
                    i64::from(stream.scale.max(1)),
                )
            };
            packets.push(Packet {
                track: index,
                data,
                pts,
                dts: pts,
                duration,
                key: chunk.key || kind == TrackKind::Audio,
            });
        }
    }
    Ok(Demuxed {
        format: Format::Avi,
        tracks,
        packets,
    })
}

fn demux_mpegts(bytes: &[u8], format: Format) -> AvResult<Demuxed> {
    use craft_media::mpegts::{self, Kind};
    let file = mpegts::open(bytes).map_err(io("mpeg-ts"))?;
    let mut tracks = Vec::new();
    let mut packets = Vec::new();
    for (index, stream) in file.streams.iter().enumerate() {
        let kind = match stream.kind() {
            Kind::Video => TrackKind::Video,
            Kind::Audio => TrackKind::Audio,
            _ => TrackKind::Other,
        };
        let mut params = stream.codec_params();
        if params.codec == CodecId::Mp2 {
            let first = file.read_unit(bytes, index, 0).unwrap_or_default();
            let layer = first.get(1).map(|b| (b >> 1) & 3);
            params.codec = match layer {
                Some(1) => CodecId::Mp3,
                Some(3) => CodecId::Mp1,
                _ => CodecId::Mp2,
            };
        }
        tracks.push(Track {
            index,
            kind,
            params,
            timescale: 90_000,
            duration: None,
            language: stream.language.clone(),
            frame_rate: None,
            codec_name: stream.codec.name(),
        });
        if kind == TrackKind::Other || stream.codec.codec_id() == CodecId::Unknown {
            continue;
        }
        let mut previous_pts = 0i64;
        for (unit_index, unit) in stream.units.iter().enumerate() {
            let data = file
                .read_unit(bytes, index, unit_index)
                .map_err(io("mpeg-ts unit"))?;
            let pts = unit.pts.or(unit.dts).unwrap_or(previous_pts);
            let dts = unit.dts.unwrap_or(pts);
            previous_pts = pts;
            packets.push(Packet {
                track: index,
                data,
                pts,
                dts,
                duration: 0,
                key: unit.key,
            });
        }
    }
    let mut demuxed = Demuxed {
        format,
        tracks,
        packets,
    };
    fill_durations(&mut demuxed);
    Ok(demuxed)
}

/// Packets that carry no duration get the gap to the next packet of their track.
fn fill_durations(demuxed: &mut Demuxed) {
    for track in 0..demuxed.tracks.len() {
        let indices: Vec<usize> = demuxed
            .packets
            .iter()
            .enumerate()
            .filter(|(_, p)| p.track == track)
            .map(|(i, _)| i)
            .collect();
        for pair in indices.windows(2) {
            if demuxed.packets[pair[0]].duration == 0 {
                demuxed.packets[pair[0]].duration =
                    (demuxed.packets[pair[1]].dts - demuxed.packets[pair[0]].dts).max(0);
            }
        }
        if let [.., before, last] = indices.as_slice() {
            demuxed.packets[*last].duration = demuxed.packets[*before].duration;
        }
    }
}

fn demux_mxf(bytes: &[u8]) -> AvResult<Demuxed> {
    use craft_media::mxf::{self, TrackKind as K};
    let file = mxf::open(bytes).map_err(io("mxf"))?;
    let mut tracks = Vec::new();
    let mut packets = Vec::new();
    for (index, track) in file.tracks.iter().enumerate() {
        let kind = match track.kind {
            K::Picture => TrackKind::Video,
            K::Sound => TrackKind::Audio,
            K::Data => TrackKind::Other,
        };
        let rate = (
            track.edit_rate.num.max(1) as u32,
            track.edit_rate.den.max(1) as u32,
        );
        let timescale = rate.0;
        let duration_ticks = track
            .duration
            .and_then(|units| u64::try_from(units).ok())
            .map(|units| units * u64::from(rate.1));
        tracks.push(Track {
            index,
            kind,
            params: track.codec_params(),
            timescale,
            duration: duration_ticks,
            language: None,
            frame_rate: (kind == TrackKind::Video).then(|| {
                let g = gcd(rate.0, rate.1);
                (rate.0 / g, rate.1 / g)
            }),
            codec_name: String::new(),
        });
        if kind != TrackKind::Video {
            continue;
        }
        for (sample_index, sample) in track.samples.iter().enumerate() {
            let data = file
                .read_sample(bytes, index, sample_index)
                .map_err(io("mxf sample"))?;
            packets.push(Packet {
                track: index,
                data,
                pts: sample.pts * i64::from(rate.1),
                dts: sample_index as i64 * i64::from(rate.1),
                duration: i64::from(rate.1),
                key: sample.key,
            });
        }
    }
    Ok(Demuxed {
        format: Format::Mxf,
        tracks,
        packets,
    })
}

fn demux_ogg(bytes: &[u8]) -> AvResult<Demuxed> {
    let file = craft_media::ogg::open(bytes).map_err(io("ogg"))?;
    let mut tracks = Vec::new();
    for (index, stream) in file.streams.iter().enumerate() {
        let params = stream.codec_params();
        let kind = if params.codec.is_audio() {
            TrackKind::Audio
        } else if params.codec.is_video() {
            TrackKind::Video
        } else {
            TrackKind::Other
        };
        let rate = params.sample_rate.unwrap_or(48_000).max(1);
        let duration = stream
            .last_granule
            .and_then(|granule| u64::try_from(granule).ok());
        tracks.push(Track {
            index,
            kind,
            params,
            timescale: rate,
            duration,
            language: None,
            frame_rate: None,
            codec_name: stream.codec.name().to_string(),
        });
    }
    Ok(Demuxed {
        format: Format::Ogg,
        tracks,
        packets: Vec::new(),
    })
}

fn demux_raw_audio(bytes: &[u8], format: Format, hint: Option<&str>) -> AvResult<Demuxed> {
    let extension = hint.and_then(|h| h.rsplit_once('.').map(|(_, e)| e).or(Some(h)));
    let info = craft_audio::probe(bytes, extension)
        .map_err(|e| AvError::malformed(format!("{}: {e}", format.name())))?;
    let params = CodecParams::new(info.codec)
        .with_sample_rate(info.sample_rate)
        .with_channels(info.channels);
    let track = Track {
        index: 0,
        kind: TrackKind::Audio,
        params,
        timescale: info.sample_rate.max(1),
        duration: Some(info.frames),
        language: None,
        frame_rate: None,
        codec_name: String::new(),
    };
    Ok(Demuxed {
        format,
        tracks: vec![track],
        packets: Vec::new(),
    })
}

pub(crate) fn kind_label(kind: TrackKind) -> &'static str {
    match kind {
        TrackKind::Video => "video",
        TrackKind::Audio => "audio",
        TrackKind::Other => "other",
    }
}

pub(crate) fn codec_label(track: &Track) -> String {
    match Codec::from_id(track.params.codec) {
        Some(codec) => codec.name().to_string(),
        None => track.params.codec.name().to_string(),
    }
}

/// Formats such as MPEG-TS carry no picture size outside the bitstream: decode up to the first
/// picture of each video track to learn it.
pub(crate) fn fill_missing_sizes(demuxed: &mut Demuxed) {
    if demuxed.format == Format::Ogg {
        return;
    }
    for track_index in 0..demuxed.tracks.len() {
        let track = &demuxed.tracks[track_index];
        if track.kind != TrackKind::Audio
            || (track.params.sample_rate.is_some() && track.params.channels.is_some())
        {
            continue;
        }
        let first: Vec<&[u8]> = demuxed
            .packets
            .iter()
            .filter(|p| p.track == track_index)
            .take(8)
            .map(|p| p.data.as_slice())
            .collect();
        if let Some((rate, channels)) = crate::audio::learn_format(track, &first) {
            if rate > 0 && channels > 0 {
                demuxed.tracks[track_index].params = track
                    .params
                    .clone()
                    .with_sample_rate(rate)
                    .with_channels(channels);
            }
        }
    }
    for track_index in 0..demuxed.tracks.len() {
        let track = &demuxed.tracks[track_index];
        if track.kind != TrackKind::Video
            || (track.params.width.is_some() && track.params.height.is_some())
        {
            continue;
        }
        let Ok(mut decoder) = VideoDecoder::new(track) else {
            continue;
        };
        let mut size = None;
        for packet in demuxed
            .packets
            .iter()
            .filter(|p| p.track == track_index)
            .take(64)
        {
            if let Ok(frames) = decoder.decode(&packet.data, packet.pts) {
                if let Some(frame) = frames.first() {
                    size = Some((frame.width, frame.height));
                    break;
                }
            }
        }
        if size.is_none() {
            if let Ok(frames) = decoder.flush() {
                size = frames.first().map(|frame| (frame.width, frame.height));
            }
        }
        if let Some((w, h)) = size {
            demuxed.tracks[track_index].params =
                demuxed.tracks[track_index].params.clone().with_size(w, h);
        }
    }
}
