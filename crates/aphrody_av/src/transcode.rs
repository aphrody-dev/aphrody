//! Remux and transcode: the container x codec matrix limited to what `craft-libs` can both read
//! and write.

use crate::audio;
use crate::demux;
use crate::error::{AvError, AvResult};
use crate::model::{Codec, Demuxed, Format, Packet, Track, TrackKind};
use crate::mux::{self, OutTrack};
use crate::video::{Framing, VideoDecoder, VideoEncodeConfig, VideoEncoder, Yuv420};
use craft_audio::AudioBuffer;
use craft_media::{CodecId, CodecParams};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamChoice {
    /// The container's default codec, or copy when nothing forces a re-encode.
    Auto,
    /// Copy packets unchanged.
    Copy,
    Encode(Codec),
    Disable,
}

/// `-vf scale=W:H`; a non-positive side is derived from the other keeping the aspect ratio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scale {
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone)]
pub struct TranscodeOptions {
    pub video: StreamChoice,
    pub audio: StreamChoice,
    pub video_bitrate_kbps: Option<u32>,
    pub audio_bitrate_bps: Option<u32>,
    pub scale: Option<Scale>,
    pub start_seconds: Option<f64>,
    pub duration_seconds: Option<f64>,
    pub fps: Option<(u32, u32)>,
    pub channels: Option<u16>,
    pub sample_rate: Option<u32>,
    /// File name of the input, used only to break ties between audio-only formats.
    pub input_hint: Option<String>,
}

impl Default for TranscodeOptions {
    fn default() -> Self {
        TranscodeOptions {
            video: StreamChoice::Auto,
            audio: StreamChoice::Auto,
            video_bitrate_kbps: None,
            audio_bitrate_bps: None,
            scale: None,
            start_seconds: None,
            duration_seconds: None,
            fps: None,
            channels: None,
            sample_rate: None,
            input_hint: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamReport {
    pub codec: String,
    pub mode: &'static str,
    pub frames: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TranscodeReport {
    pub input_format: &'static str,
    pub output_format: &'static str,
    pub output_bytes: u64,
    pub video: Option<StreamReport>,
    pub audio: Option<StreamReport>,
}

#[derive(Debug, Clone)]
pub struct Transcoded {
    pub bytes: Vec<u8>,
    pub report: TranscodeReport,
}

pub fn audio_only(format: Format) -> bool {
    matches!(
        format,
        Format::Wav
            | Format::Aiff
            | Format::Flac
            | Format::Mp3
            | Format::Ac3
            | Format::Ogg
            | Format::Adts
            | Format::Caf
    )
}

pub fn default_video_codec(format: Format) -> Option<Codec> {
    Some(match format {
        Format::Mp4 | Format::Mov | Format::Matroska | Format::MpegTs => Codec::H264,
        Format::WebM => Codec::Vp9,
        Format::Avi => Codec::Mpeg4,
        _ => return None,
    })
}

pub fn default_audio_codec(format: Format) -> Option<Codec> {
    Some(match format {
        Format::Mp4 | Format::Mov | Format::Matroska | Format::MpegTs => Codec::Aac,
        Format::WebM => Codec::Opus,
        Format::Avi | Format::Wav | Format::Aiff => Codec::Pcm,
        Format::Mp3 => Codec::Mp3,
        Format::Flac => Codec::Flac,
        Format::Ogg => Codec::Vorbis,
        Format::Ac3 => Codec::Ac3,
        _ => return None,
    })
}

/// Copy every stream into another container without touching the coded data.
pub fn remux(input: &[u8], hint: Option<&str>, format: Format) -> AvResult<Transcoded> {
    let options = TranscodeOptions {
        video: StreamChoice::Copy,
        audio: StreamChoice::Copy,
        input_hint: hint.map(str::to_owned),
        ..Default::default()
    };
    transcode(input, format, &options)
}

pub fn transcode(input: &[u8], format: Format, options: &TranscodeOptions) -> AvResult<Transcoded> {
    let hint = options.input_hint.as_deref();
    let (demuxed, packetized) = demux::demux(input, hint)?;
    validate_options(options)?;
    let video_track = demuxed.first_track(TrackKind::Video).cloned();
    let audio_track = demuxed.first_track(TrackKind::Audio).cloned();
    let audio_ordinal = audio_track.as_ref().map_or(0, |a| {
        demuxed
            .tracks
            .iter()
            .filter(|t| t.kind == TrackKind::Audio)
            .position(|t| t.index == a.index)
            .unwrap_or(0)
    });

    let file_output = audio_only(format);
    let video_choice = match (options.video, file_output) {
        (StreamChoice::Disable, _) => StreamChoice::Disable,
        (StreamChoice::Copy | StreamChoice::Encode(_), true) => {
            return Err(AvError::unsupported(format!(
                "{} is an audio-only format; video cannot be written to it (use -vn)",
                format.name()
            )));
        }
        (_, true) => StreamChoice::Disable,
        (choice, false) => choice,
    };
    let video_choice = if video_track.is_none() {
        match video_choice {
            StreamChoice::Copy | StreamChoice::Encode(_) => {
                return Err(AvError::invalid("the input has no video stream"));
            }
            _ => StreamChoice::Disable,
        }
    } else {
        video_choice
    };
    let audio_choice = if audio_track.is_none() {
        match options.audio {
            StreamChoice::Copy | StreamChoice::Encode(_) => {
                return Err(AvError::invalid("the input has no audio stream"));
            }
            _ => StreamChoice::Disable,
        }
    } else {
        options.audio
    };
    if video_choice == StreamChoice::Disable && audio_choice == StreamChoice::Disable {
        return Err(AvError::invalid(
            "no stream selected: the input has nothing to write with these options",
        ));
    }

    let mut out_tracks: Vec<OutTrack> = Vec::new();
    let mut out_packets: Vec<Packet> = Vec::new();
    let mut video_report = None;
    let mut audio_report = None;

    if video_choice != StreamChoice::Disable {
        let track = video_track
            .as_ref()
            .ok_or_else(|| AvError::invalid("the input has no video stream"))?;
        let index = out_tracks.len();
        let (out, packets, report) = video_stream(
            &demuxed,
            track,
            format,
            video_choice,
            options,
            index,
            input.len(),
        )?;
        out_tracks.push(out);
        out_packets.extend(packets);
        video_report = Some(report);
    }

    if audio_choice != StreamChoice::Disable {
        let track = audio_track
            .as_ref()
            .ok_or_else(|| AvError::invalid("the input has no audio stream"))?;
        let copy = matches!(audio_choice, StreamChoice::Copy)
            || (audio_choice == StreamChoice::Auto
                && packetized
                && !file_output
                && audio_copyable(track, format, options));
        if file_output {
            let codec = match audio_choice {
                StreamChoice::Encode(c) => c,
                _ => default_audio_codec(format).ok_or_else(|| {
                    AvError::unsupported(format!("no default audio codec for {}", format.name()))
                })?,
            };
            if matches!(audio_choice, StreamChoice::Copy) {
                return Err(AvError::unsupported(format!(
                    "copying audio into {} is not implemented; use a re-encode",
                    format.name()
                )));
            }
            let buffer = prepare_audio(&demuxed, input, hint, audio_ordinal, codec, options)?;
            let bytes = encode_audio_file(&buffer, format, codec, options.audio_bitrate_bps)?;
            let report = TranscodeReport {
                input_format: demuxed.format.name(),
                output_format: format.name(),
                output_bytes: bytes.len() as u64,
                video: None,
                audio: Some(StreamReport {
                    codec: codec.name().into(),
                    mode: "encode",
                    frames: buffer.frames() as u64,
                    width: None,
                    height: None,
                    sample_rate: Some(buffer.sample_rate),
                    channels: Some(buffer.num_channels()),
                }),
            };
            return Ok(Transcoded { bytes, report });
        }
        let index = out_tracks.len();
        if copy {
            let codec = track.codec().ok_or_else(|| {
                AvError::unsupported(format!(
                    "cannot copy unknown audio codec {}",
                    track.params.codec.name()
                ))
            })?;
            if !mux::mux_supports(format, codec) {
                return Err(AvError::unsupported(format!(
                    "cannot copy {} into {}: {}",
                    codec.name(),
                    format.name(),
                    mux::mux_reason(format, codec)
                )));
            }
            if !packetized {
                return Err(AvError::unsupported(format!(
                    "{} input has no packet demuxer here, so its audio cannot be copied; re-encode instead",
                    demuxed.format.name()
                )));
            }
            if options.start_seconds.is_some()
                || options.duration_seconds.is_some()
                || options.channels.is_some()
                || options.sample_rate.is_some()
            {
                return Err(AvError::unsupported(
                    "-ss/-t/-ac/-ar need a re-encode: use -c:a with an encoder instead of copy",
                ));
            }
            out_tracks.push(OutTrack {
                kind: TrackKind::Audio,
                params: track.params.clone(),
                timescale: track.timescale,
                frame_rate: None,
                language: track.language.clone(),
                delay: 0,
            });
            let mut count = 0;
            for packet in demuxed.track_packets(track.index) {
                out_packets.push(Packet {
                    track: index,
                    ..packet.clone()
                });
                count += 1;
            }
            audio_report = Some(StreamReport {
                codec: codec.name().into(),
                mode: "copy",
                frames: count,
                width: None,
                height: None,
                sample_rate: track.params.sample_rate,
                channels: track.params.channels,
            });
        } else {
            let codec = match audio_choice {
                StreamChoice::Encode(c) => c,
                _ => default_audio_codec(format).ok_or_else(|| {
                    AvError::unsupported(format!("no default audio codec for {}", format.name()))
                })?,
            };
            if !mux::mux_supports(format, codec) {
                return Err(AvError::unsupported(format!(
                    "{} cannot hold {} audio: {}",
                    format.name(),
                    codec.name(),
                    mux::mux_reason(format, codec)
                )));
            }
            let buffer = prepare_audio(&demuxed, input, hint, audio_ordinal, codec, options)?;
            let encoded = audio::encode_packets(&buffer, codec, options.audio_bitrate_bps)?;
            out_tracks.push(OutTrack {
                kind: TrackKind::Audio,
                params: encoded.params.clone(),
                timescale: encoded.timescale,
                frame_rate: None,
                language: track.language.clone(),
                delay: encoded.delay,
            });
            for packet in &encoded.packets {
                out_packets.push(Packet {
                    track: index,
                    data: packet.data.clone(),
                    pts: packet.pts,
                    dts: packet.pts,
                    duration: packet.duration,
                    key: true,
                });
            }
            audio_report = Some(StreamReport {
                codec: codec.name().into(),
                mode: "encode",
                frames: buffer.frames() as u64,
                width: None,
                height: None,
                sample_rate: Some(buffer.sample_rate),
                channels: Some(buffer.num_channels()),
            });
        }
    }

    let scales: Vec<u32> = out_tracks.iter().map(|t| t.timescale.max(1)).collect();
    out_packets.sort_by_key(|p| {
        (
            i128::from(p.dts) * 1_000_000_000 / i128::from(scales[p.track]),
            p.track,
        )
    });
    mux::rebase(&out_tracks, &mut out_packets);
    let bytes = mux::mux(format, &out_tracks, &out_packets)?;
    let report = TranscodeReport {
        input_format: demuxed.format.name(),
        output_format: format.name(),
        output_bytes: bytes.len() as u64,
        video: video_report,
        audio: audio_report,
    };
    Ok(Transcoded { bytes, report })
}

fn validate_options(options: &TranscodeOptions) -> AvResult<()> {
    if let Some(start) = options.start_seconds {
        if !start.is_finite() || start < 0.0 {
            return Err(AvError::invalid(format!(
                "-ss {start} must be a non-negative number of seconds"
            )));
        }
    }
    if let Some(duration) = options.duration_seconds {
        if !duration.is_finite() || duration <= 0.0 {
            return Err(AvError::invalid(format!(
                "-t {duration} must be a positive number of seconds"
            )));
        }
    }
    if let Some(scale) = options.scale {
        if scale.width == 0 || scale.height == 0 || (scale.width < 0 && scale.height < 0) {
            return Err(AvError::invalid(
                "scale needs at least one positive side (use -1 for the other to keep the aspect ratio)",
            ));
        }
    }
    if options.channels.is_some_and(|c| !(1..=2).contains(&c)) {
        return Err(AvError::unsupported("-ac supports 1 or 2 channels"));
    }
    Ok(())
}

fn audio_copyable(track: &Track, format: Format, options: &TranscodeOptions) -> bool {
    track.codec().is_some_and(|c| mux::mux_supports(format, c))
        && options.start_seconds.is_none()
        && options.duration_seconds.is_none()
        && options.channels.is_none()
        && options.sample_rate.is_none()
        && options.audio_bitrate_bps.is_none()
        && format != Format::WebM
        && track.params.codec != CodecId::Pcm
}

fn prepare_audio(
    demuxed: &Demuxed,
    input: &[u8],
    hint: Option<&str>,
    ordinal: usize,
    codec: Codec,
    options: &TranscodeOptions,
) -> AvResult<AudioBuffer> {
    let mut buffer = audio::decode_track(demuxed, input, hint, ordinal)?;
    let rate = buffer.sample_rate;
    if options.start_seconds.is_some() || options.duration_seconds.is_some() {
        let start = (options.start_seconds.unwrap_or(0.0) * f64::from(rate)) as usize;
        let length = options
            .duration_seconds
            .map_or(usize::MAX, |d| (d * f64::from(rate)).round() as usize);
        for channel in &mut buffer.channels {
            let from = start.min(channel.len());
            let to = from.saturating_add(length).min(channel.len());
            *channel = channel[from..to].to_vec();
        }
    }
    if buffer.frames() == 0 {
        return Err(AvError::invalid(
            "the selected time range contains no audio",
        ));
    }
    let channels = options
        .channels
        .unwrap_or_else(|| buffer.num_channels().min(audio::MAX_CHANNELS));
    if buffer.num_channels() > 2 && options.channels.is_none() {
        return Err(AvError::unsupported(format!(
            "{} input channels: the {} encoder here takes 1 or 2 (pass -ac 2 to downmix by taking the front pair)",
            buffer.num_channels(),
            codec.name()
        )));
    }
    buffer = audio::remix(&buffer, channels)?;
    let wanted = options
        .sample_rate
        .unwrap_or_else(|| audio::target_rate(codec, buffer.sample_rate));
    let wanted = if options.sample_rate.is_some() {
        audio::target_rate(codec, wanted)
    } else {
        wanted
    };
    Ok(audio::resample(&buffer, wanted))
}

fn encode_audio_file(
    buffer: &AudioBuffer,
    format: Format,
    codec: Codec,
    bitrate: Option<u32>,
) -> AvResult<Vec<u8>> {
    use craft_audio::{BitDepth, EncodeFormat, EncodeOptions};
    let (target, ok) = match format {
        Format::Wav => (EncodeFormat::Wav, codec == Codec::Pcm),
        Format::Aiff => (EncodeFormat::Aiff, codec == Codec::Pcm),
        Format::Flac => (EncodeFormat::Flac, codec == Codec::Flac),
        Format::Mp3 => (EncodeFormat::Mp3, codec == Codec::Mp3),
        Format::Ac3 => (
            if codec == Codec::Eac3 {
                EncodeFormat::Eac3
            } else {
                EncodeFormat::Ac3
            },
            matches!(codec, Codec::Ac3 | Codec::Eac3),
        ),
        Format::Ogg => (EncodeFormat::Ogg, codec == Codec::Vorbis),
        other => {
            return Err(AvError::unsupported(format!(
                "writing {} files is not implemented",
                other.name()
            )));
        }
    };
    if !ok {
        return Err(AvError::unsupported(format!(
            "{} files hold {} audio, not {}",
            format.name(),
            default_audio_codec(format).map_or("other", |c| c.name()),
            codec.name()
        )));
    }
    let mut options = EncodeOptions::new(target).with_bit_depth(BitDepth::Int16);
    if let Some(bps) = bitrate {
        options = options.with_bitrate(bps);
    }
    craft_audio::encode(buffer, &options).map_err(|e| match e {
        craft_audio::AudioError::Unsupported(m) => AvError::Unsupported(m),
        other => AvError::Unsupported(format!(
            "{} encoder refused the stream: {other}",
            codec.name()
        )),
    })
}

fn framing_for(format: Format) -> Framing {
    match format {
        Format::Avi | Format::MpegTs => Framing::AnnexB,
        _ => Framing::LengthPrefixed,
    }
}

fn resolve_scale(scale: Scale, width: u32, height: u32) -> (u32, u32) {
    let derive = |known: i32, other_source: u32, known_source: u32| -> u32 {
        let value = (f64::from(known) * f64::from(other_source) / f64::from(known_source.max(1)))
            .round() as u32;
        value.max(1)
    };
    match (scale.width > 0, scale.height > 0) {
        (true, true) => (scale.width as u32, scale.height as u32),
        (true, false) => (scale.width as u32, derive(scale.width, height, width)),
        (false, true) => (derive(scale.height, width, height), scale.height as u32),
        _ => (width, height),
    }
}

type VideoOutput = (OutTrack, Vec<Packet>, StreamReport);

fn video_stream(
    demuxed: &Demuxed,
    track: &Track,
    format: Format,
    choice: StreamChoice,
    options: &TranscodeOptions,
    out_index: usize,
    _input_len: usize,
) -> AvResult<VideoOutput> {
    let needs_filter = options.scale.is_some()
        || options.start_seconds.is_some()
        || options.duration_seconds.is_some()
        || options.fps.is_some()
        || options.video_bitrate_kbps.is_some();
    let source_codec = track.codec();
    let copy = match choice {
        StreamChoice::Copy => true,
        StreamChoice::Auto => {
            !needs_filter && source_codec.is_some_and(|c| mux::mux_supports(format, c))
        }
        _ => false,
    };
    if copy {
        if needs_filter {
            return Err(AvError::unsupported(
                "-vf scale, -ss, -t, -r and -b:v need a re-encode: use -c:v with an encoder instead of copy",
            ));
        }
        let codec = source_codec.ok_or_else(|| {
            AvError::unsupported(format!(
                "cannot copy unknown video codec {}",
                track.params.codec.name()
            ))
        })?;
        if !mux::mux_supports(format, codec) {
            return Err(AvError::unsupported(format!(
                "cannot copy {} into {}: {}",
                codec.name(),
                format.name(),
                mux::mux_reason(format, codec)
            )));
        }
        let (params, packets) = reframe(track, demuxed, framing_for(format), out_index)?;
        let count = packets.len() as u64;
        let out = OutTrack {
            kind: TrackKind::Video,
            params: params.clone(),
            timescale: track.timescale,
            frame_rate: track.frame_rate,
            language: track.language.clone(),
            delay: 0,
        };
        let report = StreamReport {
            codec: codec.name().into(),
            mode: "copy",
            frames: count,
            width: params.width,
            height: params.height,
            sample_rate: None,
            channels: None,
        };
        return Ok((out, packets, report));
    }
    let codec = match choice {
        StreamChoice::Encode(c) => c,
        _ => default_video_codec(format).ok_or_else(|| {
            AvError::unsupported(format!("no default video codec for {}", format.name()))
        })?,
    };
    if codec.kind() != TrackKind::Video {
        return Err(AvError::invalid(format!(
            "{} is not a video codec",
            codec.name()
        )));
    }
    if !mux::mux_supports(format, codec) {
        return Err(AvError::unsupported(format!(
            "{} cannot hold {} video: {}",
            format.name(),
            codec.name(),
            mux::mux_reason(format, codec)
        )));
    }
    let mut decoder = VideoDecoder::new(track)?;
    let timescale = f64::from(track.timescale.max(1));
    let start = options.start_seconds.unwrap_or(0.0);
    let end = options.duration_seconds.map(|d| start + d);
    let source_rate = track.frame_rate.unwrap_or((25, 1));
    let fps = options.fps.unwrap_or(source_rate);
    let convert_rate = options.fps.is_some_and(|f| f != source_rate);

    let mut state = EncodeState {
        encoder: None,
        packets: Vec::new(),
        frames: 0,
        first_time: None,
        fps,
        codec,
        options,
        format,
        size: (0, 0),
        next_output: 0,
        previous: None,
    };
    let feed = |frames: Vec<Yuv420>, state: &mut EncodeState| -> AvResult<()> {
        for frame in frames {
            let time = frame.pts as f64 / timescale;
            if time + 1e-9 < start || end.is_some_and(|e| time >= e - 1e-9) {
                continue;
            }
            if convert_rate {
                state.push_resampled(frame, time)?;
            } else {
                state.push(frame)?;
            }
        }
        Ok(())
    };
    for packet in demuxed.track_packets(track.index) {
        let frames = decoder.decode(&packet.data, packet.pts)?;
        feed(frames, &mut state)?;
    }
    let frames = decoder.flush()?;
    feed(frames, &mut state)?;
    state.finish(f64::from(source_rate.1) / f64::from(source_rate.0))?;
    if state.frames == 0 {
        return Err(AvError::invalid(
            "no video frames were decoded in the selected range",
        ));
    }
    let mut encoder = state
        .encoder
        .take()
        .ok_or_else(|| AvError::invalid("no video frames were decoded"))?;
    let tail = encoder.flush()?;
    state.collect(tail);
    let params = encoder.params().clone();
    let (fps_num, fps_den) = fps;
    let out = OutTrack {
        kind: TrackKind::Video,
        params: params.clone(),
        timescale: fps_num,
        frame_rate: Some(fps),
        language: track.language.clone(),
        delay: 0,
    };
    let packets: Vec<Packet> = state
        .packets
        .into_iter()
        .map(|p| Packet {
            track: out_index,
            data: p.data,
            pts: p.pts * i64::from(fps_den),
            dts: p.dts * i64::from(fps_den),
            duration: i64::from(fps_den),
            key: p.key,
        })
        .collect();
    let report = StreamReport {
        codec: codec.name().into(),
        mode: "encode",
        frames: state.frames,
        width: params.width,
        height: params.height,
        sample_rate: None,
        channels: None,
    };
    Ok((out, packets, report))
}

struct EncodeState<'a> {
    encoder: Option<VideoEncoder>,
    packets: Vec<crate::video::EncodedPicture>,
    frames: u64,
    first_time: Option<f64>,
    fps: (u32, u32),
    codec: Codec,
    options: &'a TranscodeOptions,
    format: Format,
    size: (u32, u32),
    next_output: u64,
    previous: Option<(Yuv420, f64)>,
}

impl EncodeState<'_> {
    fn collect(&mut self, mut pictures: Vec<crate::video::EncodedPicture>) {
        self.packets.append(&mut pictures);
    }

    fn prepare(&mut self, frame: Yuv420) -> AvResult<Yuv420> {
        let mut frame = frame;
        if let Some(scale) = self.options.scale {
            let (w, h) = resolve_scale(scale, frame.width, frame.height);
            frame = frame.scaled(w, h)?;
        }
        let needs_even = !matches!(
            self.codec,
            Codec::Mjpeg
                | Codec::Vp8
                | Codec::Vp9
                | Codec::Ffv1
                | Codec::Dnxhd
                | Codec::Apv
                | Codec::ProRes
        );
        if needs_even {
            frame = frame.cropped_even();
        }
        Ok(frame)
    }

    fn encode(&mut self, frame: Yuv420) -> AvResult<()> {
        let frame = self.prepare(frame)?;
        if self.encoder.is_none() {
            self.size = (frame.width, frame.height);
            let config = VideoEncodeConfig {
                codec: self.codec,
                width: frame.width,
                height: frame.height,
                fps: self.fps,
                bitrate_kbps: self.options.video_bitrate_kbps,
                framing: framing_for(self.format),
            };
            self.encoder = Some(VideoEncoder::new(&config)?);
        }
        if (frame.width, frame.height) != self.size {
            return Err(AvError::unsupported(format!(
                "the picture size changed mid-stream ({}x{} to {}x{}); add -vf scale to pin it",
                self.size.0, self.size.1, frame.width, frame.height
            )));
        }
        let pictures = match self.encoder.as_mut() {
            Some(encoder) => encoder.encode(&frame)?,
            None => Vec::new(),
        };
        self.frames += 1;
        self.collect(pictures);
        Ok(())
    }

    fn push(&mut self, frame: Yuv420) -> AvResult<()> {
        self.encode(frame)
    }

    /// Frame-rate conversion: output frame `k` shows the latest source picture at or before `k / fps`.
    fn push_resampled(&mut self, frame: Yuv420, time: f64) -> AvResult<()> {
        let origin = *self.first_time.get_or_insert(time);
        let step = f64::from(self.fps.1) / f64::from(self.fps.0);
        if let Some((previous, _)) = self.previous.take() {
            while origin + self.next_output as f64 * step < time - 1e-9 {
                self.encode(previous.clone())?;
                self.next_output += 1;
            }
        }
        self.previous = Some((frame, time));
        Ok(())
    }

    fn finish(&mut self, source_step: f64) -> AvResult<()> {
        if let Some((last, time)) = self.previous.take() {
            let origin = self.first_time.unwrap_or(time);
            let step = f64::from(self.fps.1) / f64::from(self.fps.0);
            let end = time + source_step;
            let mut emitted = false;
            while origin + self.next_output as f64 * step < end - 1e-9 {
                self.encode(last.clone())?;
                self.next_output += 1;
                emitted = true;
            }
            if !emitted && self.frames == 0 {
                self.encode(last)?;
            }
        }
        Ok(())
    }
}

/// Copy a video track, converting H.264/HEVC NAL framing to what `target` containers need.
fn reframe(
    track: &Track,
    demuxed: &Demuxed,
    target: Framing,
    out_index: usize,
) -> AvResult<(CodecParams, Vec<Packet>)> {
    let mut params = track.params.clone();
    let mut packets: Vec<Packet> = demuxed
        .track_packets(track.index)
        .map(|p| Packet {
            track: out_index,
            ..p.clone()
        })
        .collect();
    if !mux::codec_is_h26x(params.codec) {
        return Ok((params, packets));
    }
    let length_prefixed = params.extradata.first() == Some(&1);
    match (length_prefixed, target) {
        (true, Framing::LengthPrefixed) | (false, Framing::AnnexB) => {}
        (true, Framing::AnnexB) => {
            let (length_size, header) = parameter_sets_annexb(&params)?;
            for packet in &mut packets {
                let body = mux::length_prefixed_to_annexb(&packet.data, length_size)?;
                packet.data = if packet.key {
                    [header.clone(), body].concat()
                } else {
                    body
                };
            }
            params.extradata.clear();
        }
        (false, Framing::LengthPrefixed) => {
            if params.codec != CodecId::H264 {
                return Err(AvError::unsupported(
                    "HEVC in Annex B cannot be repackaged for mp4/matroska: the hvcC record needs SPS field parsing that is not wired",
                ));
            }
            let (mut sps, mut pps) = (Vec::new(), Vec::new());
            for packet in &mut packets {
                packet.data = mux::annexb_to_length_prefixed(&packet.data, &mut sps, &mut pps);
            }
            if sps.is_empty() || pps.is_empty() {
                return Err(AvError::malformed(
                    "H.264 Annex B stream without SPS and PPS",
                ));
            }
            params.extradata = craft_media::isobmff::AvcConfig::new(sps, pps, 4).to_bytes();
        }
    }
    Ok((params, packets))
}

fn parameter_sets_annexb(params: &CodecParams) -> AvResult<(usize, Vec<u8>)> {
    use craft_media::isobmff::{AvcConfig, HevcConfig};
    let mut header = Vec::new();
    let length_size;
    match params.codec {
        CodecId::H264 => {
            let config = AvcConfig::parse(&params.extradata)
                .map_err(|e| AvError::malformed(format!("avcC: {e}")))?;
            length_size = usize::from(config.length_size);
            for set in config.sps.iter().chain(config.pps.iter()) {
                header.extend_from_slice(&[0, 0, 0, 1]);
                header.extend_from_slice(set);
            }
        }
        _ => {
            let config = HevcConfig::parse(&params.extradata)
                .map_err(|e| AvError::malformed(format!("hvcC: {e}")))?;
            length_size = usize::from(config.length_size);
            for set in config
                .vps()
                .into_iter()
                .chain(config.sps())
                .chain(config.pps())
            {
                header.extend_from_slice(&[0, 0, 0, 1]);
                header.extend_from_slice(set);
            }
        }
    }
    Ok((length_size, header))
}
