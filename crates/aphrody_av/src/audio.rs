//! Audio decode (any container through `craft-audio`), channel and rate conversion, and packet
//! encoders for every audio codec that can sit inside a video container.

use crate::error::{AvError, AvResult};
use crate::model::Codec;
use craft_audio::AudioBuffer;
use craft_media::{CodecId, CodecParams};

/// Decode audio track number `ordinal` (counted among audio tracks) of a file to planar `f32`.
pub fn decode_audio(bytes: &[u8], hint: Option<&str>, ordinal: usize) -> AvResult<AudioBuffer> {
    let extension = hint.and_then(|h| h.rsplit_once('.').map(|(_, e)| e).or(Some(h)));
    let options = craft_audio::DecodeOptions::default().with_track(ordinal);
    let (_, buffer) =
        craft_audio::decode_with(bytes, extension, &options).map_err(|e| match e {
            craft_audio::AudioError::Unsupported(m) => AvError::Unsupported(m),
            other => AvError::Malformed(other.to_string()),
        })?;
    Ok(buffer)
}

/// Decode audio track `ordinal` of a demuxed input. Containers `craft-audio` can open itself go
/// through it; AVI, MPEG-TS/PS and MXF are decoded packet by packet with its track decoder.
pub fn decode_track(
    demuxed: &crate::model::Demuxed,
    bytes: &[u8],
    hint: Option<&str>,
    ordinal: usize,
) -> AvResult<AudioBuffer> {
    use crate::model::{Format, TrackKind};
    if !matches!(
        demuxed.format,
        Format::Avi | Format::MpegTs | Format::MpegPs | Format::Mxf
    ) {
        return decode_audio(bytes, hint, ordinal);
    }
    let track = demuxed
        .tracks
        .iter()
        .filter(|t| t.kind == TrackKind::Audio)
        .nth(ordinal)
        .ok_or_else(|| {
            AvError::invalid(format!("the input has no audio stream number {ordinal}"))
        })?;
    let packets: Vec<&crate::model::Packet> = demuxed.track_packets(track.index).collect();
    let lookahead: Vec<&[u8]> = packets
        .iter()
        .take(craft_audio::LOOKAHEAD_PACKETS)
        .map(|p| p.data.as_slice())
        .collect();
    let mut decoder =
        craft_audio::TrackDecoder::new(&track.params, &lookahead).map_err(audio_error)?;
    let mut planar: Vec<Vec<f32>> = Vec::new();
    for packet in &packets {
        let block = match decoder.decode(&packet.data) {
            Ok(block) => block,
            Err(_) => decoder.conceal(Some(&packet.data)).map_err(audio_error)?,
        };
        if planar.is_empty() {
            planar = vec![Vec::new(); block.channels.len()];
        }
        if block.channels.len() == planar.len() {
            for (target, source) in planar.iter_mut().zip(&block.channels) {
                target.extend_from_slice(source);
            }
        }
    }
    if planar.is_empty() {
        return Err(AvError::malformed("audio stream decoded to nothing"));
    }
    Ok(AudioBuffer {
        sample_rate: decoder.sample_rate(),
        channels: planar,
    })
}

/// Sample rate and channel count of a track whose container does not state them.
pub fn learn_format(track: &crate::model::Track, first_packets: &[&[u8]]) -> Option<(u32, u16)> {
    let mut decoder = craft_audio::TrackDecoder::new(&track.params, first_packets).ok()?;
    for packet in first_packets {
        let _ = decoder.decode(packet);
    }
    Some((decoder.sample_rate(), decoder.channels()))
}

fn audio_error(error: craft_audio::AudioError) -> AvError {
    match error {
        craft_audio::AudioError::Unsupported(m) => AvError::Unsupported(m),
        other => AvError::Malformed(other.to_string()),
    }
}

/// Windowed-sinc resampling of every channel.
pub fn resample(buffer: &AudioBuffer, rate: u32) -> AudioBuffer {
    if buffer.sample_rate == rate || buffer.sample_rate == 0 || rate == 0 {
        return AudioBuffer {
            sample_rate: rate.max(buffer.sample_rate),
            channels: buffer.channels.clone(),
        };
    }
    let ratio = f64::from(rate) / f64::from(buffer.sample_rate);
    let channels = buffer
        .channels
        .iter()
        .map(|channel| resample_channel(channel, ratio))
        .collect();
    AudioBuffer {
        sample_rate: rate,
        channels,
    }
}

fn resample_channel(input: &[f32], ratio: f64) -> Vec<f32> {
    const HALF_WIDTH: i64 = 16;
    let output_length = (input.len() as f64 * ratio).round() as usize;
    let cutoff = ratio.min(1.0);
    let mut out = Vec::with_capacity(output_length);
    for n in 0..output_length {
        let position = n as f64 / ratio;
        let center = position.floor() as i64;
        let mut sum = 0.0f64;
        let mut weight = 0.0f64;
        for k in (center - HALF_WIDTH + 1)..=(center + HALF_WIDTH) {
            let distance = position - k as f64;
            let x = distance * cutoff;
            let sinc = if x.abs() < 1e-9 {
                1.0
            } else {
                (std::f64::consts::PI * x).sin() / (std::f64::consts::PI * x)
            };
            let window = 0.5 + 0.5 * (std::f64::consts::PI * distance / HALF_WIDTH as f64).cos();
            let coefficient = sinc * window * cutoff;
            weight += coefficient;
            if let Some(&sample) = usize::try_from(k).ok().and_then(|i| input.get(i)) {
                sum += f64::from(sample) * coefficient;
            }
        }
        out.push(if weight.abs() > 1e-9 {
            (sum / weight) as f32
        } else {
            0.0
        });
    }
    out
}

/// Mix to `channels` (1 or 2): stereo to mono averages, mono to stereo duplicates.
pub fn remix(buffer: &AudioBuffer, channels: u16) -> AvResult<AudioBuffer> {
    let have = buffer.channels.len();
    if have == usize::from(channels) {
        return Ok(buffer.clone());
    }
    let frames = buffer.frames();
    let planar = match (have, channels) {
        (_, 1) if have >= 2 => {
            let mixed = (0..frames)
                .map(|i| buffer.channels.iter().map(|c| c[i]).sum::<f32>() / have as f32)
                .collect();
            vec![mixed]
        }
        (1, 2) => vec![buffer.channels[0].clone(), buffer.channels[0].clone()],
        (n, 2) if n > 2 => {
            let left: Vec<f32> = buffer.channels[0].clone();
            let right: Vec<f32> = buffer.channels[1].clone();
            vec![left, right]
        }
        _ => {
            return Err(AvError::unsupported(format!(
                "converting {have} channels to {channels}"
            )));
        }
    };
    Ok(AudioBuffer {
        sample_rate: buffer.sample_rate,
        channels: planar,
    })
}

/// The sample rate `codec` needs, given the input rate.
pub fn target_rate(codec: Codec, rate: u32) -> u32 {
    let nearest = |rates: &[u32]| {
        rates
            .iter()
            .copied()
            .min_by_key(|r| r.abs_diff(rate))
            .unwrap_or(rate)
    };
    match codec {
        Codec::Opus => 48_000,
        Codec::Aac => nearest(&craft_audio::aac::SAMPLE_RATES),
        Codec::Mp3 | Codec::Mp2 | Codec::Mp1 => nearest(&craft_audio::mp3::SAMPLE_RATES),
        Codec::Ac3 | Codec::Eac3 => nearest(&[32_000, 44_100, 48_000]),
        _ => rate,
    }
}

/// Channels the packet encoders here accept.
pub const MAX_CHANNELS: u16 = 2;

#[derive(Debug, Clone)]
pub struct AudioPacket {
    pub data: Vec<u8>,
    pub pts: i64,
    pub duration: i64,
}

#[derive(Debug, Clone)]
pub struct EncodedAudio {
    pub params: CodecParams,
    pub timescale: u32,
    /// Decoder delay in ticks (AAC priming, Opus pre-skip).
    pub delay: i64,
    pub packets: Vec<AudioPacket>,
}

fn to_i32_16(channel: &[f32]) -> Vec<i32> {
    channel
        .iter()
        .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0).round() as i32)
        .collect()
}

fn fail(codec: &str, error: impl std::fmt::Display) -> AvError {
    AvError::Unsupported(format!("{codec} encoder refused the stream: {error}"))
}

/// Encode a whole buffer (already at the codec's rate and channel count) into packets.
pub fn encode_packets(
    buffer: &AudioBuffer,
    codec: Codec,
    bitrate_bps: Option<u32>,
) -> AvResult<EncodedAudio> {
    let rate = buffer.sample_rate;
    let channels = buffer.channels.len();
    if channels == 0 || rate == 0 {
        return Err(AvError::invalid(
            "audio buffer has no channels or no sample rate",
        ));
    }
    let frames = buffer.frames();
    let planes: Vec<&[f32]> = buffer.channels.iter().map(|c| &c[..frames]).collect();
    let base = |id: CodecId| {
        CodecParams::new(id)
            .with_sample_rate(rate)
            .with_channels(channels as u16)
    };
    match codec {
        Codec::Aac => {
            let bps = bitrate_bps.unwrap_or(64_000 * channels as u32);
            let mut encoder = craft_audio::aac::Encoder::new(craft_audio::aac::EncoderConfig::cbr(
                rate, channels, bps,
            ))
            .map_err(|e| fail("aac", e))?;
            let mut raw = encoder.encode(&planes);
            raw.extend(encoder.flush());
            let packets = raw
                .into_iter()
                .enumerate()
                .map(|(i, data)| AudioPacket {
                    data,
                    pts: i as i64 * 1024,
                    duration: 1024,
                })
                .collect();
            Ok(EncodedAudio {
                params: base(CodecId::Aac).with_extradata(encoder.audio_specific_config()),
                timescale: rate,
                delay: i64::from(encoder.priming_samples()),
                packets,
            })
        }
        Codec::Opus => {
            let bps = bitrate_bps.unwrap_or(64_000 * channels as u32);
            let mut encoder = craft_audio::opus::encoder::Encoder::new(channels as u8, bps)
                .map_err(|e| fail("opus", e))?;
            let frame = craft_audio::opus::encoder::Encoder::FRAME_SIZE;
            let interleaved = buffer.interleaved();
            let mut packets = Vec::new();
            let mut at = 0usize;
            let padded = frames.div_ceil(frame) * frame;
            let mut samples = interleaved;
            samples.resize(padded * channels, 0.0);
            while at < padded {
                let chunk = &samples[at * channels..(at + frame) * channels];
                let data = encoder.encode_float(chunk).map_err(|e| fail("opus", e))?;
                packets.push(AudioPacket {
                    data,
                    pts: at as i64,
                    duration: frame as i64,
                });
                at += frame;
            }
            Ok(EncodedAudio {
                params: base(CodecId::Opus)
                    .with_sample_rate(48_000)
                    .with_extradata(encoder.opus_head()),
                timescale: 48_000,
                delay: i64::from(encoder.pre_skip()),
                packets,
            })
        }
        Codec::Flac => {
            let mut encoder = craft_audio::flac::Encoder::new(
                craft_audio::flac::EncoderConfig::new(rate, channels as u8, 16),
            )
            .map_err(|e| fail("flac", e))?;
            let integers: Vec<Vec<i32>> = planes.iter().map(|c| to_i32_16(c)).collect();
            let refs: Vec<&[i32]> = integers.iter().map(Vec::as_slice).collect();
            let mut frames_out = encoder.encode_frames(&refs).map_err(|e| fail("flac", e))?;
            let tail = encoder.finish();
            let block = 4096i64;
            let tail_length = (frames as i64) - frames_out.len() as i64 * block;
            if !tail.is_empty() {
                frames_out.push(tail);
            }
            let mut packets = Vec::new();
            let count = frames_out.len();
            for (i, data) in frames_out.into_iter().enumerate() {
                let duration = if i + 1 == count && tail_length > 0 {
                    tail_length
                } else {
                    block
                };
                packets.push(AudioPacket {
                    data,
                    pts: i as i64 * block,
                    duration,
                });
            }
            let mut extradata = b"fLaC".to_vec();
            extradata.extend_from_slice(&[0x80, 0, 0, 34]);
            extradata.extend_from_slice(&encoder.streaminfo());
            Ok(EncodedAudio {
                params: base(CodecId::Flac).with_extradata(extradata),
                timescale: rate,
                delay: 0,
                packets,
            })
        }
        Codec::Alac => {
            let mut encoder = craft_audio::alac::Encoder::new(
                craft_audio::alac::EncoderConfig::new(rate, channels as u8, 16),
            )
            .map_err(|e| fail("alac", e))?;
            let integers: Vec<Vec<i32>> = planes.iter().map(|c| to_i32_16(c)).collect();
            let frame = 4096usize;
            let mut packets = Vec::new();
            let mut at = 0;
            while at < frames {
                let end = (at + frame).min(frames);
                let chunk: Vec<&[i32]> = integers.iter().map(|c| &c[at..end]).collect();
                let data = encoder.encode(&chunk).map_err(|e| fail("alac", e))?;
                packets.push(AudioPacket {
                    data,
                    pts: at as i64,
                    duration: (end - at) as i64,
                });
                at = end;
            }
            Ok(EncodedAudio {
                params: base(CodecId::Alac).with_extradata(encoder.cookie()),
                timescale: rate,
                delay: 0,
                packets,
            })
        }
        Codec::Vorbis => {
            let quality = bitrate_bps.map_or(3.0, |bps| {
                craft_audio::quality_for_bitrate(
                    (bps / 1000).clamp(craft_audio::OGG_BITRATES.0, craft_audio::OGG_BITRATES.1),
                    channels,
                )
            });
            let mut encoder = craft_audio::vorbis::encoder::Encoder::new(
                craft_audio::vorbis::encoder::EncoderConfig::new(rate, channels, quality),
            )
            .map_err(|e| fail("vorbis", e))?;
            let mut coded = encoder.encode(&planes).map_err(|e| fail("vorbis", e))?;
            coded.extend(encoder.finish().map_err(|e| fail("vorbis", e))?);
            let mut comment = vec![3u8];
            comment.extend_from_slice(b"vorbis");
            let vendor = b"aphrody-av";
            comment.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
            comment.extend_from_slice(vendor);
            comment.extend_from_slice(&0u32.to_le_bytes());
            comment.push(1);
            let headers = vec![encoder.identification(), comment, encoder.setup()];
            let extradata =
                craft_media::xiph_lace(&headers).ok_or_else(|| fail("vorbis", "header lacing"))?;
            let mut previous = 0i64;
            let packets = coded
                .into_iter()
                .map(|(data, granule)| {
                    let duration = (granule - previous).max(0);
                    let packet = AudioPacket {
                        data,
                        pts: previous,
                        duration,
                    };
                    previous = granule.max(previous);
                    packet
                })
                .collect();
            Ok(EncodedAudio {
                params: base(CodecId::Vorbis).with_extradata(extradata),
                timescale: rate,
                delay: 0,
                packets,
            })
        }
        Codec::Mp3 => {
            let kbps = craft_audio::mp3::nearest_bitrate(
                rate,
                bitrate_bps.map_or(64 * channels as u32, |b| b / 1000),
            )
            .ok_or_else(|| fail("mp3", format!("no bitrate for {rate} Hz")))?;
            let mut encoder =
                craft_audio::mp3::Encoder::new(craft_audio::mp3::Config::cbr(rate, channels, kbps))
                    .map_err(|e| fail("mp3", e))?;
            let mut stream = encoder.encode(&planes);
            stream.extend(encoder.flush());
            let per_frame = encoder.frame_samples() as i64;
            let packets = split_mpeg_audio(&stream)?
                .into_iter()
                .enumerate()
                .map(|(i, data)| AudioPacket {
                    data,
                    pts: i as i64 * per_frame,
                    duration: per_frame,
                })
                .collect();
            Ok(EncodedAudio {
                params: base(CodecId::Mp3),
                timescale: rate,
                delay: i64::from(encoder.delay()),
                packets,
            })
        }
        Codec::Ac3 | Codec::Eac3 => {
            use craft_audio::ac3::encoder::{Codec as Family, CodingMode, Config, Encoder};
            let mode = if channels == 1 {
                CodingMode::Mono
            } else {
                CodingMode::Stereo
            };
            let kbps = bitrate_bps.map_or(96 * channels as u32, |b| b / 1000);
            let config = Config {
                codec: if codec == Codec::Ac3 {
                    Family::Ac3
                } else {
                    Family::Eac3
                },
                sample_rate: rate,
                bitrate_kbps: kbps,
                mode,
                lfe: false,
            };
            let mut encoder = Encoder::new(config).map_err(|e| fail(codec.name(), e))?;
            let mut coded = encoder.push(&planes).map_err(|e| fail(codec.name(), e))?;
            coded.extend(encoder.finish().map_err(|e| fail(codec.name(), e))?.packets);
            let mut at = 0i64;
            let packets = coded
                .into_iter()
                .map(|p| {
                    let packet = AudioPacket {
                        data: p.data,
                        pts: at,
                        duration: i64::from(p.samples),
                    };
                    at += i64::from(p.samples);
                    packet
                })
                .collect();
            let id = if codec == Codec::Ac3 {
                CodecId::Ac3
            } else {
                CodecId::Eac3
            };
            Ok(EncodedAudio {
                params: base(id).with_extradata(encoder.codec_configuration()),
                timescale: rate,
                delay: i64::from(encoder.priming_samples()),
                packets,
            })
        }
        Codec::Pcm => {
            let chunk = 1024usize;
            let mut packets = Vec::new();
            let mut at = 0;
            while at < frames {
                let end = (at + chunk).min(frames);
                let mut data = Vec::with_capacity((end - at) * channels * 2);
                for i in at..end {
                    for channel in &planes {
                        data.extend_from_slice(
                            &(((channel[i].clamp(-1.0, 1.0) * 32767.0).round()) as i16)
                                .to_le_bytes(),
                        );
                    }
                }
                packets.push(AudioPacket {
                    data,
                    pts: at as i64,
                    duration: (end - at) as i64,
                });
                at = end;
            }
            let params = base(CodecId::Pcm).with_pcm(craft_media::PcmFormat::int(16, true, false));
            Ok(EncodedAudio {
                params,
                timescale: rate,
                delay: 0,
                packets,
            })
        }
        other => Err(AvError::unsupported(format!(
            "no {} audio encoder in this build ({})",
            other.name(),
            other.long_name()
        ))),
    }
}

/// Split a Layer III byte stream into frames.
pub fn split_mpeg_audio(stream: &[u8]) -> AvResult<Vec<Vec<u8>>> {
    const BITRATES_V1: [u32; 15] = [
        0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320,
    ];
    const BITRATES_V2: [u32; 15] = [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160];
    const RATES: [[u32; 3]; 3] = [
        [44_100, 48_000, 32_000],
        [22_050, 24_000, 16_000],
        [11_025, 12_000, 8_000],
    ];
    let mut frames = Vec::new();
    let mut at = 0usize;
    while at + 4 <= stream.len() {
        let header =
            u32::from_be_bytes([stream[at], stream[at + 1], stream[at + 2], stream[at + 3]]);
        if header >> 21 != 0x7FF {
            return Err(AvError::malformed("MP3 encoder output lost frame sync"));
        }
        let version_bits = (header >> 19) & 3;
        let layer_bits = (header >> 17) & 3;
        let bitrate_index = ((header >> 12) & 15) as usize;
        let rate_index = ((header >> 10) & 3) as usize;
        let padding = (header >> 9) & 1;
        if layer_bits != 1
            || version_bits == 1
            || bitrate_index == 0
            || bitrate_index == 15
            || rate_index == 3
        {
            return Err(AvError::malformed(
                "unexpected MPEG audio header in encoder output",
            ));
        }
        let (table, coefficient, row) = match version_bits {
            3 => (&BITRATES_V1, 144u32, 0),
            2 => (&BITRATES_V2, 72, 1),
            _ => (&BITRATES_V2, 72, 2),
        };
        let length =
            (coefficient * table[bitrate_index] * 1000 / RATES[row][rate_index] + padding) as usize;
        if length < 4 || at + length > stream.len() {
            break;
        }
        frames.push(stream[at..at + length].to_vec());
        at += length;
    }
    Ok(frames)
}
