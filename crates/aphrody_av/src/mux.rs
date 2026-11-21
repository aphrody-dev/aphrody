//! Container writers. Every writer takes the complete track list and the packets in interleaved
//! decode order and returns the finished file as bytes.

use crate::error::{AvError, AvResult};
use crate::model::{Codec, Format, Packet, TrackKind, rescale};
use craft_media::{CodecId, CodecParams};
use std::io::Cursor;

#[derive(Debug, Clone)]
pub struct OutTrack {
    pub kind: TrackKind,
    pub params: CodecParams,
    pub timescale: u32,
    pub frame_rate: Option<(u32, u32)>,
    pub language: Option<String>,
    /// Decoder delay in ticks (AAC priming, Opus pre-skip), written as an edit list where the container has one.
    pub delay: i64,
}

impl OutTrack {
    pub fn codec(&self) -> AvResult<Codec> {
        Codec::from_id(self.params.codec).ok_or_else(|| {
            AvError::unsupported(format!(
                "codec {} is not in the capability table",
                self.params.codec.name()
            ))
        })
    }
}

/// Whether `format` can carry `codec` when written by this crate.
pub fn mux_supports(format: Format, codec: Codec) -> bool {
    use Codec::*;
    match format {
        Format::Mp4 | Format::Mov => matches!(
            codec,
            H264 | Hevc
                | Av1
                | Vp9
                | ProRes
                | Mjpeg
                | Dnxhd
                | Apv
                | Aac
                | Alac
                | Opus
                | Flac
                | Pcm
                | Ac3
                | Eac3
        ),
        Format::Matroska => matches!(
            codec,
            H264 | Hevc
                | Av1
                | Vp8
                | Vp9
                | Mpeg2
                | Mpeg4
                | Ffv1
                | Mjpeg
                | ProRes
                | Aac
                | Alac
                | Opus
                | Vorbis
                | Flac
                | Mp2
                | Mp3
                | Ac3
                | Eac3
                | Pcm
        ),
        Format::WebM => matches!(codec, Av1 | Vp8 | Vp9 | Opus | Vorbis),
        Format::Avi => matches!(
            codec,
            H264 | Hevc | Mpeg4 | Mpeg2 | Mjpeg | Vp8 | Vp9 | Av1 | Ffv1 | Aac | Ac3 | Pcm
        ),
        Format::MpegTs => matches!(codec, H264 | Hevc | Mpeg2 | Aac | Ac3 | Eac3 | Mp2 | Mp3),
        _ => false,
    }
}

pub fn mux(format: Format, tracks: &[OutTrack], packets: &[Packet]) -> AvResult<Vec<u8>> {
    if tracks.is_empty() {
        return Err(AvError::invalid("nothing to write: no tracks"));
    }
    for track in tracks {
        let codec = track.codec()?;
        if !mux_supports(format, codec) {
            return Err(AvError::unsupported(format!(
                "{} cannot hold {} ({}): {}",
                format.name(),
                codec.name(),
                codec.long_name(),
                mux_reason(format, codec)
            )));
        }
    }
    match format {
        Format::Mp4 | Format::Mov => mux_isobmff(format, tracks, packets),
        Format::Matroska | Format::WebM => mux_matroska(format, tracks, packets),
        Format::Avi => mux_avi(tracks, packets),
        Format::MpegTs => mux_mpegts(tracks, packets),
        other => Err(AvError::unsupported(format!(
            "writing {} is not implemented; supported outputs: mp4, mov, matroska, webm, avi, mpegts and audio files",
            other.name()
        ))),
    }
}

pub fn mux_reason(format: Format, codec: Codec) -> &'static str {
    match (format, codec.kind()) {
        (Format::WebM, _) => "WebM allows only VP8, VP9, AV1 video and Opus, Vorbis audio",
        (Format::Mp4 | Format::Mov, TrackKind::Video) => {
            "craft-media's ISO BMFF writer has no sample entry for it"
        }
        (Format::Mp4 | Format::Mov, _) => {
            "craft-media's ISO BMFF writer has no sample entry for it in this build"
        }
        (Format::Avi, _) => "no AVI FourCC or wave format tag is wired for it",
        (Format::MpegTs, _) => "craft-media's transport stream writer has no stream type for it",
        _ => "not wired in this container writer",
    }
}

fn mp4_error(error: craft_media::isobmff::Error) -> AvError {
    AvError::Malformed(format!("mp4 writer: {error}"))
}

fn mp4_entry(track: &OutTrack, codec: Codec) -> AvResult<craft_media::isobmff::SampleEntry> {
    use craft_media::isobmff::{
        ApvConfig, Av1Config, AvcConfig, CodecConfig, FourCc, HevcConfig, OpusConfig, PcmConfig,
        SampleEntry, VpcConfig,
    };
    let params = &track.params;
    let size = |axis: Option<u32>| -> AvResult<u16> {
        axis.and_then(|v| u16::try_from(v).ok()).ok_or_else(|| {
            AvError::invalid("video track without a picture size (or larger than 65535)")
        })
    };
    let channels = u32::from(params.channels.unwrap_or(2));
    let rate = params.sample_rate.unwrap_or(48_000);
    Ok(match codec {
        Codec::H264 => SampleEntry::avc(
            AvcConfig::parse(&params.extradata).map_err(|_| {
                AvError::unsupported(
                    "H.264 without an avcC record (Annex B input needs repackaging first)",
                )
            })?,
            size(params.width)?,
            size(params.height)?,
        ),
        Codec::Hevc => SampleEntry::hevc(
            HevcConfig::parse(&params.extradata)
                .map_err(|_| AvError::unsupported("HEVC without an hvcC record"))?,
            size(params.width)?,
            size(params.height)?,
        ),
        Codec::Av1 => SampleEntry::av1(
            Av1Config::parse(&params.extradata)
                .map_err(|_| AvError::unsupported("AV1 without an av1C record"))?,
            size(params.width)?,
            size(params.height)?,
        ),
        Codec::Vp9 => {
            let config = VpcConfig::parse(&params.extradata)
                .map_err(|_| AvError::unsupported("VP9 without a vpcC record"))?;
            SampleEntry::video(
                FourCc(*b"vp09"),
                CodecConfig::Vp9(config),
                size(params.width)?,
                size(params.height)?,
            )
        }
        Codec::ProRes => SampleEntry::prores(
            FourCc(params.tag.unwrap_or(*b"apcn")),
            size(params.width)?,
            size(params.height)?,
        ),
        Codec::Dnxhd => SampleEntry::dnx(
            FourCc(
                params
                    .tag
                    .filter(|t| t.starts_with(b"AV"))
                    .unwrap_or(*b"AVdh"),
            ),
            size(params.width)?,
            size(params.height)?,
        ),
        Codec::Mjpeg => SampleEntry::jpeg(size(params.width)?, size(params.height)?),
        Codec::Apv => SampleEntry::apv(
            ApvConfig::from_record(params.extradata.clone()).map_err(|e| {
                AvError::unsupported(format!("APV without a usable apvC record: {e}"))
            })?,
            size(params.width)?,
            size(params.height)?,
        ),
        Codec::Aac => SampleEntry::aac(params.extradata.clone(), channels, rate),
        Codec::Alac => SampleEntry::audio(
            FourCc(*b"alac"),
            CodecConfig::Alac {
                cookie: params.extradata.clone(),
            },
            channels,
            f64::from(rate),
            16,
        ),
        Codec::Opus => {
            let head = &params.extradata;
            if head.len() < 19 || !head.starts_with(b"OpusHead") {
                return Err(AvError::unsupported("Opus without an OpusHead header"));
            }
            let config = OpusConfig {
                output_channels: head[9],
                pre_skip: u16::from_le_bytes([head[10], head[11]]),
                input_sample_rate: u32::from_le_bytes([head[12], head[13], head[14], head[15]]),
                output_gain: i16::from_le_bytes([head[16], head[17]]),
                channel_mapping_family: head[18],
                channel_mapping: head.get(19..).unwrap_or_default().to_vec(),
            };
            SampleEntry::audio(
                FourCc(*b"Opus"),
                CodecConfig::Opus(config),
                channels,
                48_000.0,
                16,
            )
        }
        Codec::Flac => {
            let streaminfo: [u8; 34] = params
                .extradata
                .get(8..42)
                .and_then(|s| s.try_into().ok())
                .ok_or_else(|| AvError::unsupported("FLAC without a STREAMINFO block"))?;
            SampleEntry::flac(&streaminfo)
        }
        Codec::Pcm => SampleEntry::pcm(PcmConfig {
            bits: 16,
            float: false,
            big_endian: false,
            signed: true,
            channels,
            sample_rate: f64::from(rate),
        }),
        Codec::Ac3 => SampleEntry::audio(
            FourCc(*b"ac-3"),
            CodecConfig::Ac3 {
                dac3: params.extradata.clone(),
            },
            channels,
            f64::from(rate),
            16,
        ),
        Codec::Eac3 => SampleEntry::audio(
            FourCc(*b"ec-3"),
            CodecConfig::Eac3 {
                dec3: params.extradata.clone(),
            },
            channels,
            f64::from(rate),
            16,
        ),
        other => {
            return Err(AvError::unsupported(format!(
                "mp4 has no sample entry for {}",
                other.name()
            )));
        }
    })
}

fn mux_isobmff(format: Format, tracks: &[OutTrack], packets: &[Packet]) -> AvResult<Vec<u8>> {
    use craft_media::isobmff::{Brand, Mp4Writer, TrackConfig, WriteSample, WriterOptions};
    let brand = if format == Format::Mov {
        Brand::Mov
    } else {
        Brand::Mp4
    };
    let mut writer =
        Mp4Writer::new(Cursor::new(Vec::new()), WriterOptions::new(brand)).map_err(mp4_error)?;
    let mut indices = Vec::new();
    for track in tracks {
        let codec = track.codec()?;
        let mut config = TrackConfig::new(mp4_entry(track, codec)?, track.timescale);
        if track.delay > 0 {
            config.media_start = Some(track.delay);
            if codec == Codec::Aac {
                config.roll_distance = Some(-1);
            }
        }
        if let Some(language) = &track.language {
            if language.len() == 3 {
                config.language = language.clone();
            }
        }
        indices.push(writer.add_track(config).map_err(mp4_error)?);
    }
    for packet in packets {
        let Some(&index) = indices.get(packet.track) else {
            return Err(AvError::invalid("packet for an unknown track"));
        };
        let offset = i32::try_from(packet.pts - packet.dts)
            .map_err(|_| AvError::invalid("composition offset out of range"))?;
        writer
            .write_sample(
                index,
                WriteSample {
                    data: &packet.data,
                    duration: u32::try_from(packet.duration.max(0)).unwrap_or(0),
                    composition_offset: offset,
                    is_sync: packet.key,
                },
            )
            .map_err(mp4_error)?;
    }
    Ok(writer.finish_faststart().map_err(mp4_error)?.into_inner())
}

fn matroska_spec(track: &OutTrack, codec: Codec) -> AvResult<craft_media::matroska::TrackSpec> {
    use craft_media::matroska::TrackKind as K;
    use craft_media::matroska::TrackSpec;
    let params = &track.params;
    let id = match codec {
        Codec::H264 => "V_MPEG4/ISO/AVC",
        Codec::Hevc => "V_MPEGH/ISO/HEVC",
        Codec::Av1 => "V_AV1",
        Codec::Vp8 => "V_VP8",
        Codec::Vp9 => "V_VP9",
        Codec::Mpeg2 => "V_MPEG2",
        Codec::Mpeg4 => "V_MPEG4/ISO/SP",
        Codec::Ffv1 => "V_FFV1",
        Codec::Mjpeg => "V_MJPEG",
        Codec::ProRes => "V_PRORES",
        Codec::Aac => "A_AAC",
        Codec::Alac => "A_ALAC",
        Codec::Opus => "A_OPUS",
        Codec::Vorbis => "A_VORBIS",
        Codec::Flac => "A_FLAC",
        Codec::Mp2 => "A_MPEG/L2",
        Codec::Mp3 => "A_MPEG/L3",
        Codec::Ac3 => "A_AC3",
        Codec::Eac3 => "A_EAC3",
        Codec::Pcm => "A_PCM/INT/LIT",
        other => {
            return Err(AvError::unsupported(format!(
                "matroska has no codec id wired for {}",
                other.name()
            )));
        }
    };
    let kind = if codec.kind() == TrackKind::Video {
        K::Video
    } else {
        K::Audio
    };
    let mut spec = TrackSpec::new(kind, id);
    spec.codec_private = match codec {
        Codec::Vp9
        | Codec::Vp8
        | Codec::Pcm
        | Codec::Mjpeg
        | Codec::Mp2
        | Codec::Mp3
        | Codec::Ac3
        | Codec::Eac3
        | Codec::ProRes => Vec::new(),
        _ => params.extradata.clone(),
    };
    if codec.kind() == TrackKind::Video {
        let (w, h) = (
            params
                .width
                .ok_or_else(|| AvError::invalid("video track without a size"))?,
            params.height.unwrap_or(0),
        );
        spec.video_size = Some((w, h));
        spec.default_duration_ns = track
            .frame_rate
            .map(|(n, d)| 1_000_000_000u64 * u64::from(d) / u64::from(n.max(1)));
    } else {
        let bits = (codec == Codec::Pcm).then_some(16u64);
        spec.audio = Some((
            f64::from(params.sample_rate.unwrap_or(48_000)),
            u64::from(params.channels.unwrap_or(2)),
            bits,
        ));
        if codec == Codec::Opus {
            let pre_skip = params
                .extradata
                .get(10..12)
                .map_or(0, |b| u64::from(u16::from_le_bytes([b[0], b[1]])));
            spec.codec_delay_ns = pre_skip * 1_000_000_000 / 48_000;
            spec.seek_pre_roll_ns = 80_000_000;
        }
    }
    spec.language = track.language.clone();
    Ok(spec)
}

fn mux_matroska(format: Format, tracks: &[OutTrack], packets: &[Packet]) -> AvResult<Vec<u8>> {
    use craft_media::matroska::{MkvWriter, MuxOptions};
    let webm = format == Format::WebM;
    let specs = tracks
        .iter()
        .map(|t| matroska_spec(t, t.codec()?))
        .collect::<AvResult<Vec<_>>>()?;
    let options = MuxOptions {
        doc_type: if webm {
            "webm".into()
        } else {
            "matroska".into()
        },
        writing_app: "aphrody-av".into(),
        ..MuxOptions::default()
    };
    let mut writer = MkvWriter::new(Cursor::new(Vec::new()), specs, options)
        .map_err(|e| AvError::Malformed(format!("matroska writer: {e}")))?;
    for packet in packets {
        let track = tracks
            .get(packet.track)
            .ok_or_else(|| AvError::invalid("packet for an unknown track"))?;
        let pts_ns = rescale(packet.pts, track.timescale, 1_000_000_000);
        let duration_ns = (packet.duration > 0).then(|| {
            u64::try_from(rescale(packet.duration, track.timescale, 1_000_000_000)).unwrap_or(0)
        });
        writer
            .write_frame(packet.track, pts_ns, packet.key, &packet.data, duration_ns)
            .map_err(|e| AvError::Malformed(format!("matroska writer: {e}")))?;
    }
    Ok(writer
        .finish()
        .map_err(|e| AvError::Malformed(format!("matroska writer: {e}")))?
        .into_inner())
}

fn mux_avi(tracks: &[OutTrack], packets: &[Packet]) -> AvResult<Vec<u8>> {
    use craft_media::avi::WaveFormat;
    use craft_media::avi::{AudioSpec, AudioTiming, AviWriter, MuxOptions, StreamSpec, VideoSpec};
    let mut specs = Vec::new();
    for track in tracks {
        let codec = track.codec()?;
        let params = &track.params;
        if codec.kind() == TrackKind::Video {
            let (rate, scale) = track
                .frame_rate
                .ok_or_else(|| AvError::invalid("AVI needs a constant video frame rate"))?;
            let spec = match codec {
                Codec::Ffv1 => {
                    let mut spec = VideoSpec::new(
                        *b"FFV1",
                        params.width.unwrap_or(0),
                        params.height.unwrap_or(0),
                        rate,
                        scale,
                    );
                    spec.format.extra = params.extradata.clone();
                    spec
                }
                _ => VideoSpec::for_codec(params, rate, scale).ok_or_else(|| {
                    AvError::unsupported(format!("AVI has no FourCC for {}", codec.name()))
                })?,
            };
            specs.push(StreamSpec::Video(spec));
        } else {
            let rate = params.sample_rate.unwrap_or(48_000);
            let channels = params.channels.unwrap_or(2);
            specs.push(StreamSpec::Audio(match codec {
                Codec::Pcm => AudioSpec::pcm(rate, channels, 16),
                Codec::Aac => AudioSpec {
                    format: WaveFormat {
                        format_tag: 0xFF,
                        channels,
                        sample_rate: rate,
                        avg_bytes_per_sec: 16_000,
                        block_align: 1,
                        bits_per_sample: 0,
                        extra: params.extradata.clone(),
                    },
                    timing: AudioTiming::Packets {
                        samples_per_packet: 1024,
                    },
                    name: None,
                },
                Codec::Ac3 => AudioSpec {
                    format: WaveFormat {
                        format_tag: 0x2000,
                        channels,
                        sample_rate: rate,
                        avg_bytes_per_sec: 48_000,
                        block_align: 1,
                        bits_per_sample: 0,
                        extra: Vec::new(),
                    },
                    timing: AudioTiming::Packets {
                        samples_per_packet: 1536,
                    },
                    name: None,
                },
                other => {
                    return Err(AvError::unsupported(format!(
                        "AVI audio for {} is not wired",
                        other.name()
                    )));
                }
            }));
        }
    }
    let mut writer = AviWriter::new(Cursor::new(Vec::new()), specs, MuxOptions::default())
        .map_err(|e| AvError::Malformed(format!("avi writer: {e}")))?;
    for packet in packets {
        writer
            .write_chunk(packet.track, &packet.data, packet.key)
            .map_err(|e| AvError::Malformed(format!("avi writer: {e}")))?;
    }
    Ok(writer
        .finish()
        .map_err(|e| AvError::Malformed(format!("avi writer: {e}")))?
        .into_inner())
}

fn mux_mpegts(tracks: &[OutTrack], packets: &[Packet]) -> AvResult<Vec<u8>> {
    use craft_media::mpegts::mux::{Codec as C, Config, Frame, Muxer, Stream};
    let mut streams = Vec::new();
    for (index, track) in tracks.iter().enumerate() {
        let codec = track.codec()?;
        let params = &track.params;
        let kind = match codec {
            Codec::H264 => C::H264,
            Codec::Hevc => C::Hevc,
            Codec::Mpeg2 => C::Mpeg2Video,
            Codec::Mpeg4 => C::Mpeg4Part2,
            Codec::Av1 => C::Av1(
                craft_media::mpegts::mux::Av1Config::from_av1c(&params.extradata).map_err(|e| {
                    AvError::unsupported(format!("AV1 in MPEG-TS needs an av1C record: {e}"))
                })?,
            ),
            Codec::Aac => {
                let (object_type, rate, channels) =
                    craft_media::isobmff::parse_asc(&params.extradata).ok_or_else(|| {
                        AvError::unsupported("AAC without an AudioSpecificConfig")
                    })?;
                let sample_rate_index =
                    craft_audio::aac::sample_rate_index(rate).ok_or_else(|| {
                        AvError::unsupported(format!("AAC at {rate} Hz has no ADTS rate index"))
                    })?;
                C::Aac {
                    object_type,
                    sample_rate_index,
                    channels,
                }
            }
            Codec::Opus => C::Opus {
                channels: u8::try_from(params.channels.unwrap_or(2)).unwrap_or(2),
            },
            Codec::Ac3 => C::Ac3,
            Codec::Eac3 => C::Eac3,
            Codec::Mp2 | Codec::Mp3 => C::MpegAudio {
                low_sample_rate: params.sample_rate.is_some_and(|r| r < 32_000),
            },
            other => {
                return Err(AvError::unsupported(format!(
                    "mpegts has no stream type for {}",
                    other.name()
                )));
            }
        };
        streams.push(Stream::new(0x100 + u16::try_from(index).unwrap_or(0), kind));
    }
    let mut output = Vec::new();
    let mut muxer = Muxer::new(
        &mut output,
        Config {
            hold_packets: true,
            batch_packets: 348,
            ..Config::default()
        },
        &streams,
    )
    .map_err(|e| AvError::Malformed(format!("mpegts writer: {e}")))?;
    const OFFSET: i64 = 90_000;
    for packet in packets {
        let track = tracks
            .get(packet.track)
            .ok_or_else(|| AvError::invalid("packet for an unknown track"))?;
        let pts = rescale(packet.pts, track.timescale, 90_000) + OFFSET;
        let dts = rescale(packet.dts, track.timescale, 90_000) + OFFSET;
        let frame = Frame {
            data: &packet.data,
            pts,
            dts: (dts != pts).then_some(dts.min(pts)),
            key: packet.key,
        };
        muxer
            .write_frame(packet.track, &frame)
            .map_err(|e| AvError::Malformed(format!("mpegts writer: {e}")))?;
    }
    muxer
        .finish()
        .map_err(|e| AvError::Malformed(format!("mpegts writer: {e}")))?;
    Ok(output)
}

/// Shift every timestamp so the earliest decode time of the whole file is zero.
pub fn rebase(tracks: &[OutTrack], packets: &mut [Packet]) {
    let earliest = packets
        .iter()
        .filter_map(|p| {
            tracks.get(p.track).map(|t| {
                i128::from(p.dts.min(p.pts)) * 1_000_000_000 / i128::from(t.timescale.max(1))
            })
        })
        .min()
        .unwrap_or(0);
    for packet in packets.iter_mut() {
        if let Some(track) = tracks.get(packet.track) {
            let shift = (earliest * i128::from(track.timescale.max(1)) / 1_000_000_000) as i64;
            packet.pts -= shift;
            packet.dts -= shift;
        }
    }
}

/// Parameter sets of an Annex B H.264 stream, and the stream converted to length-prefixed NAL units.
pub fn annexb_to_length_prefixed(
    data: &[u8],
    sps: &mut Vec<Vec<u8>>,
    pps: &mut Vec<Vec<u8>>,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 16);
    for nal in split_annexb(data) {
        let kind = nal.first().map_or(0, |b| b & 0x1F);
        match kind {
            7 => {
                if !sps.iter().any(|s| s == nal) {
                    sps.push(nal.to_vec());
                }
            }
            8 => {
                if !pps.iter().any(|p| p == nal) {
                    pps.push(nal.to_vec());
                }
            }
            9 => {}
            _ => {
                out.extend_from_slice(&(nal.len() as u32).to_be_bytes());
                out.extend_from_slice(nal);
            }
        }
    }
    out
}

/// NAL units of an Annex B buffer, start codes removed.
pub fn split_annexb(data: &[u8]) -> Vec<&[u8]> {
    let mut starts = Vec::new();
    let mut i = 0;
    while i + 3 <= data.len() {
        if data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1 {
            starts.push(i + 3);
            i += 3;
        } else {
            i += 1;
        }
    }
    let mut nals = Vec::new();
    for (n, &start) in starts.iter().enumerate() {
        let mut end = starts.get(n + 1).map_or(data.len(), |&next| next - 3);
        while end > start && data[end - 1] == 0 {
            end -= 1;
        }
        if end > start {
            nals.push(&data[start..end]);
        }
    }
    nals
}

/// Length-prefixed NAL units to Annex B.
pub fn length_prefixed_to_annexb(data: &[u8], length_size: usize) -> AvResult<Vec<u8>> {
    let mut out = Vec::with_capacity(data.len() + 16);
    let mut at = 0;
    while at < data.len() {
        let end_of_length = at + length_size;
        let prefix = data
            .get(at..end_of_length)
            .ok_or_else(|| AvError::malformed("NAL length prefix runs past the packet"))?;
        let length = prefix
            .iter()
            .fold(0usize, |acc, &b| (acc << 8) | usize::from(b));
        let end = end_of_length
            .checked_add(length)
            .filter(|&e| e <= data.len())
            .ok_or_else(|| AvError::malformed("NAL unit runs past the packet"))?;
        out.extend_from_slice(&[0, 0, 0, 1]);
        out.extend_from_slice(&data[end_of_length..end]);
        at = end;
    }
    Ok(out)
}

pub fn codec_is_h26x(id: CodecId) -> bool {
    matches!(id, CodecId::H264 | CodecId::Hevc)
}
