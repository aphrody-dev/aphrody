//! MCP server over stdio. Tools: `probe`, `transcode`, `convert`, `capabilities`. File access is
//! confined to the read and write roots given on the command line.

use crate::caps;
use crate::service::{
    ConvertSpec, Sandbox, TranscodeSpec, convert_file, probe_file, transcode_file,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{ServiceExt, schemars, tool, tool_router, transport::stdio};
use serde::Deserialize;

#[derive(Clone)]
pub struct AvServer {
    sandbox: Sandbox,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ProbeArgs {
    /// Path of the media file, inside a read root.
    pub path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TranscodeArgs {
    /// Input file, inside a read root.
    pub input: String,
    /// Output file, inside a write root; its extension selects the container unless `format` is set.
    pub output: String,
    /// Output container (`mp4`, `mkv`, `webm`, `avi`, `mov`, `mpegts`, `wav`, `flac`, `mp3`, `ogg`, `ac3`, `aiff`).
    pub format: Option<String>,
    /// `copy`, `none` or a codec name (`h264`, `hevc`, `av1`, `vp9`, `vp8`, `ffv1`, `prores`, `mjpeg`, `mpeg2video`, `mpeg4`, `dnxhd`, `apv`).
    pub video_codec: Option<String>,
    /// `copy`, `none` or a codec name (`aac`, `opus`, `vorbis`, `flac`, `mp3`, `ac3`, `eac3`, `alac`, `pcm_s16le`).
    pub audio_codec: Option<String>,
    /// For example `800k` or `2M`.
    pub video_bitrate: Option<String>,
    pub audio_bitrate: Option<String>,
    /// `W:H`; one side may be `-1` to keep the aspect ratio.
    pub scale: Option<String>,
    /// Start offset: seconds, `MM:SS` or `HH:MM:SS.ms`.
    pub start: Option<String>,
    pub duration: Option<String>,
    /// `30`, `29.97` or `30000/1001`.
    pub fps: Option<String>,
    pub channels: Option<u16>,
    pub sample_rate: Option<u32>,
    /// Replace the output when it exists (default false).
    pub overwrite: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ConvertArgs {
    /// Input video, inside a read root.
    pub input: String,
    /// Directory to write the documents into, inside a write root.
    pub output_dir: String,
    /// Any of `svg`, `md`, `html`, `jsx`, `gpui`; all when omitted.
    pub formats: Option<Vec<String>>,
    /// Sampled frames per second.
    pub fps: Option<f64>,
    /// Output width in pixels.
    pub width: Option<u32>,
    pub max_frames: Option<usize>,
    pub overwrite: Option<bool>,
}

fn success(value: &impl serde::Serialize) -> CallToolResult {
    match serde_json::to_string_pretty(value) {
        Ok(text) => CallToolResult::success(vec![ContentBlock::text(text)]),
        Err(error) => CallToolResult::error(vec![ContentBlock::text(format!(
            "could not serialise the result: {error}"
        ))]),
    }
}

fn failure(error: &crate::error::AvError) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(error.to_string())])
}

#[tool_router(server_handler)]
impl AvServer {
    pub fn new(sandbox: Sandbox) -> AvServer {
        AvServer { sandbox }
    }

    #[tool(
        name = "probe",
        description = "Container, streams, codecs, sizes, frame rates and duration of a media file (like ffprobe)."
    )]
    fn probe(&self, Parameters(args): Parameters<ProbeArgs>) -> CallToolResult {
        match probe_file(&self.sandbox, &args.path) {
            Ok(info) => success(&info),
            Err(error) => failure(&error),
        }
    }

    #[tool(
        name = "transcode",
        description = "Convert a media file to another container and/or codecs (like ffmpeg -i in [-c:v] [-c:a] [-b:v] [-vf scale] [-ss] [-t] out). Unsupported combinations fail with the exact reason."
    )]
    fn transcode(&self, Parameters(args): Parameters<TranscodeArgs>) -> CallToolResult {
        let spec = TranscodeSpec {
            format: args.format,
            video_codec: args.video_codec,
            audio_codec: args.audio_codec,
            video_bitrate: args.video_bitrate,
            audio_bitrate: args.audio_bitrate,
            scale: args.scale,
            start: args.start,
            duration: args.duration,
            fps: args.fps,
            channels: args.channels,
            sample_rate: args.sample_rate,
        };
        match transcode_file(
            &self.sandbox,
            &args.input,
            &args.output,
            &spec,
            args.overwrite.unwrap_or(false),
        ) {
            Ok(report) => success(&report),
            Err(error) => failure(&error),
        }
    }

    #[tool(
        name = "convert",
        description = "Vectorise a video into SVG frames and an animated SVG, Markdown, HTML, JSX or GPUI source, written into a directory."
    )]
    fn convert(&self, Parameters(args): Parameters<ConvertArgs>) -> CallToolResult {
        let spec = ConvertSpec {
            formats: args.formats.unwrap_or_default(),
            fps: args.fps,
            width: args.width,
            max_frames: args.max_frames,
        };
        match convert_file(
            &self.sandbox,
            &args.input,
            &args.output_dir,
            &spec,
            args.overwrite.unwrap_or(false),
        ) {
            Ok(summary) => success(&summary),
            Err(error) => failure(&error),
        }
    }

    #[tool(
        name = "capabilities",
        description = "The codec and container capability matrix: what can be decoded, encoded and written into which container."
    )]
    fn capabilities(&self) -> CallToolResult {
        success(
            &serde_json::json!({ "codecs": caps::codec_caps(), "formats": caps::format_caps() }),
        )
    }
}

/// Serve over stdin and stdout until the client disconnects.
pub async fn serve_stdio(sandbox: Sandbox) -> Result<(), String> {
    let service = AvServer::new(sandbox)
        .serve(stdio())
        .await
        .map_err(|e| format!("mcp initialisation failed: {e}"))?;
    service
        .waiting()
        .await
        .map_err(|e| format!("mcp session failed: {e}"))?;
    Ok(())
}
