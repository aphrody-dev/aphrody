//! Path-level services shared by the CLI, the C ABI, UniFFI and the MCP server: textual option
//! parsing (`-b:v 800k`, `-ss 00:00:01.5`, `scale=320:-1`), a read/write root sandbox, and
//! file-in, file-out wrappers around the byte API.

use crate::convert::{ConvertFormat, ConvertOptions};
use crate::error::{AvError, AvResult};
use crate::model::{Codec, Format};
use crate::probe::ProbeInfo;
use crate::transcode::{Scale, StreamChoice, TranscodeOptions, TranscodeReport};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

/// ffmpeg-style transcode request in text form. Unknown fields are rejected.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct TranscodeSpec {
    /// Output container (`mp4`, `mkv`, ...); defaults to the output file's extension.
    pub format: Option<String>,
    /// `copy`, `none` (no video) or a codec name.
    pub video_codec: Option<String>,
    /// `copy`, `none` (no audio) or a codec name.
    pub audio_codec: Option<String>,
    /// `800k`, `2M` or bits per second.
    pub video_bitrate: Option<String>,
    pub audio_bitrate: Option<String>,
    /// `W:H`, either side may be `-1` to keep the aspect ratio (`-vf scale=W:H` is accepted too).
    pub scale: Option<String>,
    /// Start offset: seconds, `MM:SS` or `HH:MM:SS.ms`.
    pub start: Option<String>,
    pub duration: Option<String>,
    /// `30`, `29.97` or `30000/1001`.
    pub fps: Option<String>,
    pub channels: Option<u16>,
    pub sample_rate: Option<u32>,
}

pub fn parse_bitrate(text: &str) -> AvResult<u32> {
    let trimmed = text.trim();
    let (digits, multiplier) = match trimmed.chars().last() {
        Some('k' | 'K') => (&trimmed[..trimmed.len() - 1], 1_000.0),
        Some('m' | 'M') => (&trimmed[..trimmed.len() - 1], 1_000_000.0),
        _ => (trimmed, 1.0),
    };
    let value: f64 = digits.parse().map_err(|_| {
        AvError::invalid(format!(
            "bitrate {text:?} is not a number (examples: 800k, 2M, 128000)"
        ))
    })?;
    if !value.is_finite() || value <= 0.0 || value * multiplier > f64::from(u32::MAX) {
        return Err(AvError::invalid(format!(
            "bitrate {text:?} is out of range"
        )));
    }
    Ok((value * multiplier).round() as u32)
}

pub fn parse_time(text: &str) -> AvResult<f64> {
    let mut seconds = 0.0f64;
    for part in text.trim().split(':') {
        let value: f64 = part.parse().map_err(|_| {
            AvError::invalid(format!(
                "time {text:?} is not seconds, MM:SS or HH:MM:SS.ms"
            ))
        })?;
        if !value.is_finite() || value < 0.0 {
            return Err(AvError::invalid(format!(
                "time {text:?} must not be negative"
            )));
        }
        seconds = seconds * 60.0 + value;
    }
    Ok(seconds)
}

pub fn parse_scale(text: &str) -> AvResult<Scale> {
    let spec = text.trim().trim_start_matches("scale=");
    let (width, height) = spec.split_once([':', 'x']).ok_or_else(|| {
        AvError::invalid(format!("scale {text:?} must look like W:H, W:-1 or -1:H"))
    })?;
    let parse = |side: &str| {
        side.trim()
            .parse::<i32>()
            .map_err(|_| AvError::invalid(format!("scale side {side:?} is not an integer")))
    };
    let (width, height) = (parse(width)?, parse(height)?);
    if width > 16_384 || height > 16_384 {
        return Err(AvError::invalid("scale sides are limited to 16384"));
    }
    Ok(Scale { width, height })
}

pub fn parse_fps(text: &str) -> AvResult<(u32, u32)> {
    let bad = || {
        AvError::invalid(format!(
            "frame rate {text:?} must be like 30, 29.97 or 30000/1001"
        ))
    };
    if let Some((n, d)) = text.split_once('/') {
        let (n, d): (u32, u32) = (
            n.trim().parse().map_err(|_| bad())?,
            d.trim().parse().map_err(|_| bad())?,
        );
        if n == 0 || d == 0 {
            return Err(bad());
        }
        let g = crate::demux::gcd(n, d);
        return Ok((n / g, d / g));
    }
    let value: f64 = text.trim().parse().map_err(|_| bad())?;
    if !value.is_finite() || value <= 0.0 || value > 1000.0 {
        return Err(bad());
    }
    Ok(crate::demux::snap_rate(
        (value * 1000.0).round() as u64,
        1000,
    ))
}

fn parse_choice(text: Option<&str>, kind: crate::model::TrackKind) -> AvResult<StreamChoice> {
    Ok(match text.map(str::trim) {
        None | Some("") | Some("auto") => StreamChoice::Auto,
        Some("copy") => StreamChoice::Copy,
        Some("none") | Some("disable") | Some("off") => StreamChoice::Disable,
        Some(name) => {
            let codec = Codec::from_name(name).ok_or_else(|| {
                let known: Vec<&str> = Codec::ALL
                    .into_iter()
                    .filter(|c| c.kind() == kind)
                    .map(Codec::name)
                    .collect();
                AvError::unsupported(format!(
                    "unknown codec {name:?}; known {} codecs: {}",
                    if kind == crate::model::TrackKind::Video {
                        "video"
                    } else {
                        "audio"
                    },
                    known.join(", ")
                ))
            })?;
            if codec.kind() != kind {
                return Err(AvError::invalid(format!(
                    "{name} is not a {} codec",
                    if kind == crate::model::TrackKind::Video {
                        "video"
                    } else {
                        "audio"
                    }
                )));
            }
            StreamChoice::Encode(codec)
        }
    })
}

impl TranscodeSpec {
    /// Resolve into an output format and options; `output_hint` is the output file name.
    pub fn resolve(
        &self,
        output_hint: Option<&str>,
        input_hint: Option<&str>,
    ) -> AvResult<(Format, TranscodeOptions)> {
        use crate::model::TrackKind::{Audio, Video};
        let format = match (&self.format, output_hint) {
            (Some(name), _) => Format::from_name(name).ok_or_else(|| AvError::unsupported(format!("unknown output format {name:?}; known: {}", Format::ALL.map(Format::name).join(", "))))?,
            (None, Some(path)) => Format::from_path(path).ok_or_else(|| AvError::invalid(format!("cannot infer the output format from {path:?}: give a known extension or the format field")))?,
            (None, None) => return Err(AvError::invalid("no output format: give the format field or an output file extension")),
        };
        let options = TranscodeOptions {
            video: parse_choice(self.video_codec.as_deref(), Video)?,
            audio: parse_choice(self.audio_codec.as_deref(), Audio)?,
            video_bitrate_kbps: self
                .video_bitrate
                .as_deref()
                .map(parse_bitrate)
                .transpose()?
                .map(|bps| (bps / 1000).max(1)),
            audio_bitrate_bps: self
                .audio_bitrate
                .as_deref()
                .map(parse_bitrate)
                .transpose()?,
            scale: self.scale.as_deref().map(parse_scale).transpose()?,
            start_seconds: self.start.as_deref().map(parse_time).transpose()?,
            duration_seconds: self.duration.as_deref().map(parse_time).transpose()?,
            fps: self.fps.as_deref().map(parse_fps).transpose()?,
            channels: self.channels,
            sample_rate: self.sample_rate,
            input_hint: input_hint.map(str::to_owned),
        };
        Ok((format, options))
    }
}

/// Options of a `convert` request in text form.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ConvertSpec {
    /// Any of `svg`, `md`, `html`, `jsx`, `gpui`; all when empty.
    pub formats: Vec<String>,
    pub fps: Option<f64>,
    pub width: Option<u32>,
    pub max_frames: Option<usize>,
}

impl ConvertSpec {
    pub fn resolve(&self, name: &str) -> AvResult<ConvertOptions> {
        let mut options = ConvertOptions {
            name: name.to_owned(),
            ..Default::default()
        };
        if !self.formats.is_empty() {
            options.formats = self
                .formats
                .iter()
                .map(|f| {
                    ConvertFormat::from_name(f).ok_or_else(|| {
                        AvError::unsupported(format!(
                            "unknown convert format {f:?}; known: svg, md, html, jsx, gpui"
                        ))
                    })
                })
                .collect::<AvResult<_>>()?;
        }
        if let Some(fps) = self.fps {
            options.fps = fps;
        }
        if self.width.is_some() {
            options.width = self.width;
        }
        options.max_frames = self.max_frames.or(options.max_frames);
        Ok(options)
    }
}

/// Which directories a caller may read from and write to. `Sandbox::open()` allows everything.
#[derive(Debug, Clone, Default)]
pub struct Sandbox {
    roots: Option<(Vec<PathBuf>, Vec<PathBuf>)>,
}

impl Sandbox {
    pub fn open() -> Sandbox {
        Sandbox { roots: None }
    }

    /// Restrict reads to `read` and writes to `write` (each must exist).
    pub fn restricted(read: &[PathBuf], write: &[PathBuf]) -> AvResult<Sandbox> {
        let canon = |paths: &[PathBuf]| -> AvResult<Vec<PathBuf>> {
            paths
                .iter()
                .map(|p| {
                    std::fs::canonicalize(p)
                        .map_err(|e| AvError::Io(format!("root {}: {e}", p.display())))
                })
                .collect()
        };
        Ok(Sandbox {
            roots: Some((canon(read)?, canon(write)?)),
        })
    }

    pub fn is_restricted(&self) -> bool {
        self.roots.is_some()
    }

    pub fn check_read(&self, path: &str) -> AvResult<PathBuf> {
        let resolved =
            std::fs::canonicalize(path).map_err(|e| AvError::Io(format!("{path}: {e}")))?;
        if let Some((read, _)) = &self.roots {
            if !read.iter().any(|root| resolved.starts_with(root)) {
                return Err(AvError::Denied(format!("{path} is outside the read roots")));
            }
        }
        Ok(resolved)
    }

    /// A path to create: its directory must exist inside a write root and the name must be plain.
    pub fn check_write(&self, path: &str) -> AvResult<PathBuf> {
        let requested = Path::new(path);
        if requested
            .components()
            .any(|c| matches!(c, Component::ParentDir))
            && self.is_restricted()
        {
            return Err(AvError::Denied(format!("{path} contains `..`")));
        }
        let name = requested
            .file_name()
            .ok_or_else(|| AvError::invalid(format!("{path} has no file name")))?;
        let parent = match requested.parent() {
            Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
            _ => PathBuf::from("."),
        };
        let parent = std::fs::canonicalize(&parent)
            .map_err(|e| AvError::Io(format!("{}: {e}", parent.display())))?;
        let target = parent.join(name);
        if let Ok(existing) = std::fs::symlink_metadata(&target) {
            if existing.file_type().is_symlink() && self.is_restricted() {
                let resolved =
                    std::fs::canonicalize(&target).map_err(|e| AvError::Io(e.to_string()))?;
                self.check_inside_write(&resolved, path)?;
            }
        }
        self.check_inside_write(&target, path)?;
        Ok(target)
    }

    fn check_inside_write(&self, resolved: &Path, shown: &str) -> AvResult<()> {
        if let Some((_, write)) = &self.roots {
            if !write.iter().any(|root| resolved.starts_with(root)) {
                return Err(AvError::Denied(format!(
                    "{shown} is outside the write roots"
                )));
            }
        }
        Ok(())
    }
}

fn read_file(path: &Path) -> AvResult<Vec<u8>> {
    let metadata =
        std::fs::metadata(path).map_err(|e| AvError::Io(format!("{}: {e}", path.display())))?;
    if !metadata.is_file() {
        return Err(AvError::invalid(format!(
            "{} is not a file",
            path.display()
        )));
    }
    if metadata.len() > crate::demux::MAX_INPUT_BYTES {
        return Err(AvError::Denied(format!(
            "{} is {} bytes, over the in-memory limit",
            path.display(),
            metadata.len()
        )));
    }
    std::fs::read(path).map_err(|e| AvError::Io(format!("{}: {e}", path.display())))
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub fn probe_file(sandbox: &Sandbox, path: &str) -> AvResult<ProbeInfo> {
    let resolved = sandbox.check_read(path)?;
    crate::probe::probe(&read_file(&resolved)?, Some(&file_name(&resolved)))
}

pub fn transcode_file(
    sandbox: &Sandbox,
    input: &str,
    output: &str,
    spec: &TranscodeSpec,
    overwrite: bool,
) -> AvResult<TranscodeReport> {
    let source = sandbox.check_read(input)?;
    let target = sandbox.check_write(output)?;
    if target.exists() && !overwrite {
        return Err(AvError::invalid(format!(
            "{output} already exists (overwrite is off)"
        )));
    }
    if target == source {
        return Err(AvError::invalid("input and output are the same file"));
    }
    let (format, options) = spec.resolve(Some(&file_name(&target)), Some(&file_name(&source)))?;
    let result = crate::transcode::transcode(&read_file(&source)?, format, &options)?;
    std::fs::write(&target, &result.bytes)
        .map_err(|e| AvError::Io(format!("{}: {e}", target.display())))?;
    Ok(result.report)
}

#[derive(Debug, Clone, Serialize)]
pub struct ConvertSummary {
    pub frames: usize,
    pub decoded: usize,
    pub files: Vec<String>,
}

pub fn convert_file(
    sandbox: &Sandbox,
    input: &str,
    output_dir: &str,
    spec: &ConvertSpec,
    overwrite: bool,
) -> AvResult<ConvertSummary> {
    let source = sandbox.check_read(input)?;
    let directory = sandbox.check_write(output_dir.trim_end_matches(['/', '\\']))?;
    let options = spec.resolve(&file_name(&source))?;
    let converted =
        crate::convert::convert(&read_file(&source)?, Some(&file_name(&source)), &options)?;
    let mut written = Vec::new();
    for file in &converted.files {
        let target = directory.join(&file.path);
        if target.exists() && !overwrite {
            return Err(AvError::invalid(format!(
                "{} already exists (overwrite is off)",
                target.display()
            )));
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AvError::Io(format!("{}: {e}", parent.display())))?;
        }
        std::fs::write(&target, &file.bytes)
            .map_err(|e| AvError::Io(format!("{}: {e}", target.display())))?;
        written.push(file.path.clone());
    }
    Ok(ConvertSummary {
        frames: converted.frames,
        decoded: converted.decoded,
        files: written,
    })
}
