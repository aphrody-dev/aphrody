//! Pure-Rust FFmpeg replacement for any agent, library, language or OS.
//!
//! Probe, decode, encode, remux, transcode and convert, over the `craft-libs` codecs and
//! containers and `fw-video`. Surfaces: this Rust API, the `aphrody-av` CLI, a C ABI (`ffi`),
//! UniFFI bindings (`uniffi_api`), wasm-bindgen (`wasm`) and an MCP server (`mcp`).

pub mod audio;
pub mod caps;
pub mod convert;
pub mod decode;
pub mod demux;
pub mod error;
pub mod model;
pub mod mux;
pub mod probe;
pub mod service;
pub mod transcode;
pub mod video;

#[cfg(feature = "ffi")]
pub mod ffi;
#[cfg(feature = "mcp")]
pub mod mcp;
#[cfg(feature = "uniffi")]
pub mod uniffi_api;
#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
pub mod wasm;

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

pub use convert::{ConvertFormat, ConvertOptions, Converted, ConvertedFile, convert};
pub use decode::{
    AudioBlock, AudioBlocks, DecodedFrame, VideoFrames, decode_audio_blocks, decode_video,
};
pub use error::{AvError, AvResult};
pub use model::{Codec, Demuxed, Format, Packet, Track, TrackKind};
pub use probe::{ProbeInfo, StreamInfo, probe};
pub use transcode::StreamReport;
pub use transcode::{
    Scale, StreamChoice, TranscodeOptions, TranscodeReport, Transcoded, remux, transcode,
};
pub use video::{VideoDecoder, VideoEncodeConfig, VideoEncoder, Yuv420};

/// Encode planar 8-bit 4:2:0 pictures into a standalone file. Every picture must have the size
/// of the first.
pub fn encode_video(
    pictures: &[Yuv420],
    fps: (u32, u32),
    format: Format,
    codec: Codec,
    bitrate_kbps: Option<u32>,
) -> AvResult<Vec<u8>> {
    let first = pictures
        .first()
        .ok_or_else(|| AvError::invalid("no pictures to encode"))?;
    if !mux::mux_supports(format, codec) {
        return Err(AvError::unsupported(format!(
            "{} cannot hold {}: {}",
            format.name(),
            codec.name(),
            mux::mux_reason(format, codec)
        )));
    }
    let framing = if matches!(format, Format::Avi | Format::MpegTs) {
        video::Framing::AnnexB
    } else {
        video::Framing::LengthPrefixed
    };
    let mut encoder = VideoEncoder::new(&VideoEncodeConfig {
        codec,
        width: first.width,
        height: first.height,
        fps,
        bitrate_kbps,
        framing,
    })?;
    let mut coded = Vec::new();
    for picture in pictures {
        coded.extend(encoder.encode(picture)?);
    }
    coded.extend(encoder.flush()?);
    let track = mux::OutTrack {
        kind: TrackKind::Video,
        params: encoder.params().clone(),
        timescale: fps.0,
        frame_rate: Some(fps),
        language: None,
        delay: 0,
    };
    let packets: Vec<Packet> = coded
        .into_iter()
        .map(|p| Packet {
            track: 0,
            data: p.data,
            pts: p.pts * i64::from(fps.1),
            dts: p.dts * i64::from(fps.1),
            duration: i64::from(fps.1),
            key: p.key,
        })
        .collect();
    mux::mux(format, &[track], &packets)
}
