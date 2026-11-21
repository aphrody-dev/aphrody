//! Pure-Rust multimedia engine for Aphrody:
//! - GPU YUV->RGB / HDR AgX / 3D LUT / scopes / transitions WGSL shaders validated with `naga 30`
//! - Pure-Rust SRT/WebVTT subtitle parser with Unicode Bidi support
//! - HLS (`.m3u8` + AES-128) & DASH (`.mpd`) network ingest parsers
//! - Declarative frame-exact programmatic video engine: JSON/TOML compositions, keyframes with
//!   springs and easing, transitions, captions, deterministic rendering and H.264 MP4 export
//! - Video intelligence engine (`watch-skill` parity): luminance scene-change detector,
//!   SHA-256 keyframe deduplication, and ranked timestamped evidence search
//! - Unified player state machine for GPUI desktop, headless agents, and WebAssembly.

use serde::{Deserialize, Serialize};

pub mod gpu_yuv;
pub mod ingest;
pub mod motion;
pub mod motion_render;
pub mod player;
pub mod subtitles;
pub mod watch;

pub use gpu_yuv::{MediaShaderKind, MediaShaderValidation, validate_all_media_shaders};
pub use ingest::{
    DashRepresentation, HlsPlaylist, HlsSegment, IngestSourceKind, parse_dash_mpd, parse_hls_m3u8,
};
pub use motion::{
    Animation, AudioCue, CaptionCue, Clip, Color, Composition, Ease, Fps, FrameScene, Keyframe,
    Layer, Length, Property, Props, ResolvedClip, SceneItem, SpringParams, Time, Timeline, Track,
    Transition, TransitionKind, TransitionState, active_caption, interpolate_clamped,
    spring_progress,
};
pub use motion_render::{
    Assets, CpuBlender, NoAssets, Renderer, TransitionBlender, Yuv420, decode_image,
    demultiplied_rgba, rgba_to_yuv420,
};
#[cfg(feature = "video-encode")]
pub use motion_render::Mp4Encoder;
pub use player::{LoopRange, MediaPlayerState, PlaybackSpeed, SubtitleTrackInfo, TrackKind};
pub use subtitles::{SubtitleCue, SubtitleFormat, parse_subtitles};
pub use watch::{KeyframeSample, SceneChangeEvent, WatchEvidenceHit, WatchIndex};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaCapabilities {
    pub zero_ffmpeg_core: bool,
    pub zero_chromium_render: bool,
    pub wgsl_shaders_count: usize,
    pub supported_containers: Vec<String>,
    pub supported_video_codecs: Vec<String>,
    pub supported_audio_codecs: Vec<String>,
    pub supported_subtitle_formats: Vec<String>,
    pub wasm_compatible: bool,
}

#[must_use]
pub fn capabilities() -> MediaCapabilities {
    MediaCapabilities {
        zero_ffmpeg_core: true,
        zero_chromium_render: true,
        wgsl_shaders_count: MediaShaderKind::ALL.len(),
        supported_containers: ["mp4", "mov", "mkv", "webm", "mpegts", "ogg", "avi", "mxf"]
            .into_iter()
            .map(ToString::to_string)
            .collect(),
        supported_video_codecs: [
            "h264", "hevc", "vp9", "av1", "vp8", "prores", "dnx", "mjpeg", "ffv1", "hap",
        ]
        .into_iter()
        .map(ToString::to_string)
        .collect(),
        supported_audio_codecs: [
            "opus", "aac", "flac", "vorbis", "mp3", "alac", "wav", "ac3", "eac3",
        ]
        .into_iter()
        .map(ToString::to_string)
        .collect(),
        supported_subtitle_formats: ["srt", "webvtt", "ttml", "cea608", "cea708", "tx3g"]
            .into_iter()
            .map(ToString::to_string)
            .collect(),
        wasm_compatible: true,
    }
}
