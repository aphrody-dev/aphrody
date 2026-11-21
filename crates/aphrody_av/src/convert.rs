//! Video to vector documents (SVG, Markdown, HTML, JSX, GPUI) through `fw-video`, from any codec
//! this crate can decode and from in-memory bytes.

use crate::decode::decode_video;
use crate::error::{AvError, AvResult};
use fw_video::export::{self, Format as FwFormat, Source};
use fw_video::{Options, SvgFrame};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvertFormat {
    Svg,
    Markdown,
    Html,
    Jsx,
    Gpui,
}

impl ConvertFormat {
    pub const ALL: [ConvertFormat; 5] = [
        ConvertFormat::Svg,
        ConvertFormat::Markdown,
        ConvertFormat::Html,
        ConvertFormat::Jsx,
        ConvertFormat::Gpui,
    ];

    pub fn from_name(name: &str) -> Option<ConvertFormat> {
        FwFormat::from_name(&name.to_ascii_lowercase()).map(|format| match format {
            FwFormat::Svg => ConvertFormat::Svg,
            FwFormat::Markdown => ConvertFormat::Markdown,
            FwFormat::Html => ConvertFormat::Html,
            FwFormat::Jsx => ConvertFormat::Jsx,
            FwFormat::Gpui => ConvertFormat::Gpui,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            ConvertFormat::Svg => "svg",
            ConvertFormat::Markdown => "md",
            ConvertFormat::Html => "html",
            ConvertFormat::Jsx => "jsx",
            ConvertFormat::Gpui => "gpui",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConvertOptions {
    pub formats: Vec<ConvertFormat>,
    /// Sampled frames per second.
    pub fps: f64,
    pub width: Option<u32>,
    pub color_precision: i32,
    pub filter_speckle: usize,
    pub max_frames: Option<usize>,
    pub scene_threshold: f64,
    /// Name shown in the generated documents.
    pub name: String,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        let base = Options::default();
        ConvertOptions {
            formats: ConvertFormat::ALL.to_vec(),
            fps: base.fps,
            width: base.width,
            color_precision: base.color_precision,
            filter_speckle: base.filter_speckle,
            max_frames: base.max_frames,
            scene_threshold: base.scene_threshold,
            name: "video".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConvertedFile {
    /// Path relative to the output directory.
    pub path: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Converted {
    pub files: Vec<ConvertedFile>,
    /// Vectorised frames.
    pub frames: usize,
    /// Pictures decoded from the input.
    pub decoded: usize,
}

/// Vectorise `bytes` and render the requested documents in memory.
pub fn convert(bytes: &[u8], hint: Option<&str>, options: &ConvertOptions) -> AvResult<Converted> {
    if options.formats.is_empty() {
        return Err(AvError::invalid("no output format requested"));
    }
    let fw_options = Options {
        fps: options.fps,
        width: options.width,
        color_precision: options.color_precision,
        filter_speckle: options.filter_speckle,
        max_frames: options.max_frames,
        scene_threshold: options.scene_threshold,
    };
    if !(fw_options.fps.is_finite() && fw_options.fps > 0.0)
        || !(1..=8).contains(&fw_options.color_precision)
    {
        return Err(AvError::invalid(
            "fps must be positive and color precision 1..=8",
        ));
    }
    let step = 1.0 / fw_options.fps.max(0.001);
    let mut next_time = 0.0f64;
    let mut frames: Vec<SvgFrame> = Vec::new();
    let mut previous: Option<Vec<u8>> = None;
    let mut decoded = 0usize;
    for frame in decode_video(bytes, hint)? {
        let frame = frame?;
        decoded += 1;
        let time = frame.pts_seconds;
        if time + 1e-9 < next_time {
            continue;
        }
        if fw_options.max_frames.is_some_and(|max| frames.len() >= max) {
            break;
        }
        let rgb = frame.picture.to_rgb();
        let target_width = fw_options.width.unwrap_or(frame.picture.width) as usize;
        let (small, width, height) = fw_video::downscale(
            &rgb,
            frame.picture.width as usize,
            frame.picture.height as usize,
            target_width,
        );
        let mut svg = fw_video::vectorize(&small, width, height, &fw_options);
        svg.time = time;
        svg.change = previous
            .as_deref()
            .map_or(1.0, |p| fw_video::frame_change(p, &small));
        svg.scene_start = previous.is_none() || svg.change >= fw_options.scene_threshold;
        previous = Some(small);
        frames.push(svg);
        next_time = time + step;
    }
    if frames.is_empty() {
        return Err(AvError::invalid(
            "no frames were decoded, so there is nothing to convert",
        ));
    }
    let source = Source {
        name: options.name.clone(),
        decoded_frames: decoded,
    };
    let files = render(&frames, &source, &options.formats);
    Ok(Converted {
        files,
        frames: frames.len(),
        decoded,
    })
}

fn frame_file(index: usize) -> String {
    format!("frames/frame-{:05}.svg", index + 1)
}

fn render(frames: &[SvgFrame], source: &Source, formats: &[ConvertFormat]) -> Vec<ConvertedFile> {
    let mut files: Vec<ConvertedFile> = Vec::new();
    let mut put = |path: String, body: Vec<u8>| {
        if !files.iter().any(|f| f.path == path) {
            files.push(ConvertedFile { path, bytes: body });
        }
    };
    let needs_frames = formats.iter().any(|f| {
        matches!(
            f,
            ConvertFormat::Svg | ConvertFormat::Markdown | ConvertFormat::Gpui
        )
    });
    if needs_frames {
        for (index, frame) in frames.iter().enumerate() {
            put(frame_file(index), frame.to_svg().into_bytes());
        }
    }
    for format in formats {
        match format {
            ConvertFormat::Svg => put(
                "animated.svg".into(),
                fw_video::animated_svg(frames).into_bytes(),
            ),
            ConvertFormat::Markdown => {
                put(
                    "animated.svg".into(),
                    fw_video::animated_svg(frames).into_bytes(),
                );
                put(
                    "video.md".into(),
                    export::markdown(frames, source).into_bytes(),
                );
            }
            ConvertFormat::Html => put(
                "video.html".into(),
                export::html(frames, source).into_bytes(),
            ),
            ConvertFormat::Jsx => put(
                "VideoSvg.jsx".into(),
                export::jsx(frames, source).into_bytes(),
            ),
            ConvertFormat::Gpui => put(
                "video_svg.rs".into(),
                export::gpui(frames, source).into_bytes(),
            ),
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    files
}
