//! UniFFI surface: the same probe, transcode and convert API as the Rust crate, for Python,
//! Kotlin and Swift (and any other UniFFI target).

use crate::caps;
use crate::convert::convert;
use crate::error::AvError;
use crate::service::{
    ConvertSpec, Sandbox, TranscodeSpec, probe_file as probe_path, transcode_file as transcode_path,
};

#[derive(Debug, uniffi::Error, thiserror::Error)]
pub enum AvFailure {
    #[error("unsupported: {message}")]
    Unsupported { message: String },
    #[error("malformed input: {message}")]
    Malformed { message: String },
    #[error("invalid argument: {message}")]
    InvalidArgument { message: String },
    #[error("i/o error: {message}")]
    Io { message: String },
    #[error("denied: {message}")]
    Denied { message: String },
}

impl From<AvError> for AvFailure {
    fn from(error: AvError) -> Self {
        let message = error.message().to_owned();
        match error {
            AvError::Unsupported(_) => AvFailure::Unsupported { message },
            AvError::Malformed(_) => AvFailure::Malformed { message },
            AvError::InvalidArgument(_) => AvFailure::InvalidArgument { message },
            AvError::Io(_) => AvFailure::Io { message },
            AvError::Denied(_) => AvFailure::Denied { message },
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct StreamRecord {
    pub index: u32,
    pub kind: String,
    pub codec: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub frame_rate: Option<f64>,
    pub frames: Option<u64>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub duration_seconds: Option<f64>,
    pub bit_rate: Option<u64>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct ProbeRecord {
    pub format: String,
    pub size_bytes: u64,
    pub duration_seconds: Option<f64>,
    pub bit_rate: Option<u64>,
    pub streams: Vec<StreamRecord>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct TranscodeOptionsRecord {
    /// Output container when the file name does not decide it.
    #[uniffi(default = None)]
    pub format: Option<String>,
    /// `copy`, `none` or a codec name.
    #[uniffi(default = None)]
    pub video_codec: Option<String>,
    #[uniffi(default = None)]
    pub audio_codec: Option<String>,
    /// `800k`, `2M`, or bits per second.
    #[uniffi(default = None)]
    pub video_bitrate: Option<String>,
    #[uniffi(default = None)]
    pub audio_bitrate: Option<String>,
    /// `W:H`, one side may be `-1`.
    #[uniffi(default = None)]
    pub scale: Option<String>,
    #[uniffi(default = None)]
    pub start: Option<String>,
    #[uniffi(default = None)]
    pub duration: Option<String>,
    #[uniffi(default = None)]
    pub fps: Option<String>,
    #[uniffi(default = None)]
    pub channels: Option<u16>,
    #[uniffi(default = None)]
    pub sample_rate: Option<u32>,
}

impl From<TranscodeOptionsRecord> for TranscodeSpec {
    fn from(record: TranscodeOptionsRecord) -> Self {
        TranscodeSpec {
            format: record.format,
            video_codec: record.video_codec,
            audio_codec: record.audio_codec,
            video_bitrate: record.video_bitrate,
            audio_bitrate: record.audio_bitrate,
            scale: record.scale,
            start: record.start,
            duration: record.duration,
            fps: record.fps,
            channels: record.channels,
            sample_rate: record.sample_rate,
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct ConvertedFileRecord {
    pub path: String,
    pub data: Vec<u8>,
}

fn record(info: crate::probe::ProbeInfo) -> ProbeRecord {
    ProbeRecord {
        format: info.format.to_owned(),
        size_bytes: info.size_bytes,
        duration_seconds: info.duration_seconds,
        bit_rate: info.bit_rate,
        streams: info
            .streams
            .into_iter()
            .map(|s| StreamRecord {
                index: s.index as u32,
                kind: s.kind.to_owned(),
                codec: s.codec,
                width: s.width,
                height: s.height,
                frame_rate: s.frame_rate,
                frames: s.frames,
                sample_rate: s.sample_rate,
                channels: s.channels,
                duration_seconds: s.duration_seconds,
                bit_rate: s.bit_rate,
            })
            .collect(),
    }
}

#[uniffi::export]
pub fn probe_bytes(data: Vec<u8>, hint: Option<String>) -> Result<ProbeRecord, AvFailure> {
    Ok(record(crate::probe::probe(&data, hint.as_deref())?))
}

#[uniffi::export]
pub fn probe_file(path: String) -> Result<ProbeRecord, AvFailure> {
    Ok(record(probe_path(&Sandbox::open(), &path)?))
}

/// Transcode an in-memory file; `output_format` is a container name such as `mp4`.
#[uniffi::export]
pub fn transcode_bytes(
    data: Vec<u8>,
    hint: Option<String>,
    output_format: String,
    options: TranscodeOptionsRecord,
) -> Result<Vec<u8>, AvFailure> {
    let mut spec: TranscodeSpec = options.into();
    spec.format = Some(output_format);
    let (format, transcode_options) = spec.resolve(None, hint.as_deref())?;
    Ok(crate::transcode::transcode(&data, format, &transcode_options)?.bytes)
}

/// Transcode a file on disk; returns the report as JSON.
#[uniffi::export]
pub fn transcode_file(
    input: String,
    output: String,
    options: TranscodeOptionsRecord,
    overwrite: bool,
) -> Result<String, AvFailure> {
    let report = transcode_path(
        &Sandbox::open(),
        &input,
        &output,
        &options.into(),
        overwrite,
    )?;
    serde_json::to_string(&report).map_err(|e| AvFailure::Malformed {
        message: e.to_string(),
    })
}

/// Vectorise a video into SVG, Markdown, HTML, JSX or GPUI documents (`formats` empty means all).
#[uniffi::export]
pub fn convert_bytes(
    data: Vec<u8>,
    hint: Option<String>,
    formats: Vec<String>,
    fps: Option<f64>,
    width: Option<u32>,
) -> Result<Vec<ConvertedFileRecord>, AvFailure> {
    let spec = ConvertSpec {
        formats,
        fps,
        width,
        max_frames: None,
    };
    let options = spec.resolve(hint.as_deref().unwrap_or("video"))?;
    let converted = convert(&data, hint.as_deref(), &options)?;
    Ok(converted
        .files
        .into_iter()
        .map(|f| ConvertedFileRecord {
            path: f.path,
            data: f.bytes,
        })
        .collect())
}

#[uniffi::export]
pub fn capabilities_json() -> String {
    serde_json::json!({ "codecs": caps::codec_caps(), "formats": caps::format_caps() }).to_string()
}

#[uniffi::export]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}
