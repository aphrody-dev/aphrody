//! `aphrody media render` and `aphrody media timeline`: render declarative JSON/TOML compositions
//! (see `aphrody_media::motion`) to PNG frames and an H.264 MP4, with no browser, JavaScript or FFmpeg.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, anyhow, ensure};
use aphrody_media::{
    Assets, Composition, Mp4Encoder, Renderer, Timeline, TransitionBlender, TransitionKind,
    TransitionState, demultiplied_rgba,
};
use aphrody_media::motion_render::tiny_skia::Pixmap;
use bytemuck::{Pod, Zeroable};
use clap::Args;
use serde_json::json;
use sha2::{Digest as _, Sha256};

use crate::{BackendSelection, read_mapped_buffer, request_headless_device};

#[derive(Debug, Args)]
pub struct CompositionArgs {
    /// Composition file (`.json` or `.toml`)
    pub composition: PathBuf,

    /// Override a declared variable (`--var title="Launch day"`), repeatable
    #[arg(long = "var", value_parser = parse_variable)]
    pub variables: Vec<(String, String)>,
}

#[derive(Debug, Args)]
pub struct RenderArgs {
    #[command(flatten)]
    pub composition: CompositionArgs,

    /// Write every frame as `frame_000000.png` into this directory
    #[arg(long)]
    pub frames_dir: Option<PathBuf>,

    /// Write an H.264 MP4 (High profile, BT.709, no audio)
    #[arg(long)]
    pub video: Option<PathBuf>,

    /// MP4 constant-quality level (lower is better)
    #[arg(long, default_value_t = 18.0)]
    pub crf: f32,

    /// First frame to render
    #[arg(long, default_value_t = 0)]
    pub start_frame: u64,

    /// Frame to stop before (defaults to the end of the composition)
    #[arg(long)]
    pub end_frame: Option<u64>,

    /// Run clip transitions on the GPU with the `motion_transition` WGSL shader
    #[arg(long)]
    pub gpu: bool,

    /// Backend filter for `--gpu` (`all`, `vulkan`, `dx12`, `metal`, `gl`)
    #[arg(long, value_enum, default_value_t = BackendSelection::All)]
    pub backends: BackendSelection,
}

fn parse_variable(raw: &str) -> Result<(String, String), String> {
    raw.split_once('=')
        .map(|(name, value)| (name.trim().to_string(), value.to_string()))
        .ok_or_else(|| format!("expected NAME=VALUE, got `{raw}`"))
}

struct FileAssets {
    base: PathBuf,
}

impl Assets for FileAssets {
    fn read(&mut self, src: &str) -> Result<Vec<u8>> {
        ensure!(
            !src.contains("://"),
            "remote asset `{src}` is not supported; download it next to the composition"
        );
        let path = Path::new(src);
        let full = if path.is_absolute() { path.to_path_buf() } else { self.base.join(path) };
        std::fs::read(&full).with_context(|| format!("reading asset `{}`", full.display()))
    }
}

fn load_timeline(args: &CompositionArgs) -> Result<(Timeline, PathBuf)> {
    let path = &args.composition;
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading composition `{}`", path.display()))?;
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("");
    let composition = Composition::from_text(extension, &text)?;
    let overrides: BTreeMap<String, String> = args.variables.iter().cloned().collect();
    let timeline = Timeline::new(&composition, &overrides)?;
    let base = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    Ok((timeline, base))
}

pub fn run_timeline(args: CompositionArgs) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(&timeline_report(&args)?)?);
    Ok(())
}

pub fn timeline_report(args: &CompositionArgs) -> Result<serde_json::Value> {
    let (timeline, _) = load_timeline(args)?;
    let composition = timeline.composition();
    let clips: Vec<_> = timeline
        .resolved()
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let clip = timeline.clip(index);
            json!({
                "id": clip.id,
                "track": composition.tracks[entry.track].id,
                "start_frame": entry.start_frame,
                "end_frame": entry.end_frame,
                "start_seconds": timeline.frame_seconds(entry.start_frame),
                "end_seconds": timeline.frame_seconds(entry.end_frame),
            })
        })
        .collect();
    let (fps_num, fps_den) = timeline.fps();
    let report = json!({
        "id": composition.id,
        "width": composition.width,
        "height": composition.height,
        "fps": { "num": fps_num, "den": fps_den },
        "duration_frames": timeline.duration_frames(),
        "duration_seconds": timeline.duration_seconds(),
        "clips": clips,
        "paint_order": timeline.paint_order().iter().map(|&index| timeline.clip(index).id.clone()).collect::<Vec<_>>(),
        "audio_cues": timeline.audio_cues(),
    });
    Ok(report)
}

pub fn run_render(args: RenderArgs) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(&render_report(&args)?)?);
    Ok(())
}

pub fn render_report(args: &RenderArgs) -> Result<serde_json::Value> {
    ensure!(
        args.frames_dir.is_some() || args.video.is_some(),
        "pass --frames-dir and/or --video"
    );
    let (timeline, base) = load_timeline(&args.composition)?;
    let total = timeline.duration_frames();
    let end = args.end_frame.unwrap_or(total).min(total);
    ensure!(
        args.start_frame < end,
        "empty frame range {}..{end} of {total}",
        args.start_frame
    );
    let (width, height) = (timeline.composition().width, timeline.composition().height);
    let (fps_num, fps_den) = timeline.fps();
    let audio_cues = timeline.audio_cues().len();
    let composition_id = timeline.composition().id.clone();

    let mut renderer = Renderer::new(timeline, FileAssets { base });
    let mut blender_name = "cpu".to_string();
    if args.gpu {
        let blender = GpuBlender::new(args.backends, width, height)?;
        blender_name = format!("gpu:{}", blender.adapter_name);
        renderer = renderer.with_blender(Box::new(blender));
    }

    if let Some(directory) = &args.frames_dir {
        std::fs::create_dir_all(directory)
            .with_context(|| format!("creating `{}`", directory.display()))?;
    }
    let mut encoder = match &args.video {
        Some(_) => Some(Mp4Encoder::new(width, height, fps_num, fps_den, args.crf)?),
        None => None,
    };

    let mut digest = Sha256::new();
    for frame in args.start_frame..end {
        let pixmap = renderer
            .render_frame(frame)
            .with_context(|| format!("rendering frame {frame}"))?;
        digest.update(pixmap.data());
        if let Some(directory) = &args.frames_dir {
            let path = directory.join(format!("frame_{frame:06}.png"));
            image::save_buffer(
                &path,
                &demultiplied_rgba(&pixmap),
                width,
                height,
                image::ExtendedColorType::Rgba8,
            )
            .with_context(|| format!("writing `{}`", path.display()))?;
        }
        if let Some(encoder) = encoder.as_mut() {
            encoder.push_frame(&pixmap)?;
        }
    }

    let mut video_report = serde_json::Value::Null;
    if let (Some(encoder), Some(path)) = (encoder, &args.video) {
        let bytes = encoder.finish()?;
        std::fs::write(path, &bytes).with_context(|| format!("writing `{}`", path.display()))?;
        video_report = json!({
            "path": path,
            "bytes": bytes.len(),
            "sha256": hex(&Sha256::digest(&bytes)),
            "codec": "h264 (craft-video)",
            "audio": "none",
        });
    }
    let report = json!({
        "composition": composition_id,
        "width": width,
        "height": height,
        "fps": { "num": fps_num, "den": fps_den },
        "frames_rendered": end - args.start_frame,
        "frame_range": [args.start_frame, end],
        "frames_dir": args.frames_dir,
        "video": video_report,
        "transition_blender": blender_name,
        "frames_sha256": hex(&digest.finalize()),
        "audio_cues_not_mixed": audio_cues,
    });
    Ok(report)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TransitionUniforms {
    progress: f32,
    kind: u32,
    feather: f32,
    padding: f32,
    width: f32,
    height: f32,
    padding_size: [f32; 2],
}

/// Runs the `motion_transition` WGSL shader on a headless `wgpu` device. Results can differ from
/// [`aphrody_media::CpuBlender`] by rounding (and by the hardware's bilinear filter for zoom).
struct GpuBlender {
    adapter_name: String,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniforms: wgpu::Buffer,
    texture_a: wgpu::Texture,
    texture_b: wgpu::Texture,
    target: wgpu::Texture,
    readback: wgpu::Buffer,
    width: u32,
    height: u32,
    padded_row: u32,
}

impl GpuBlender {
    fn new(backends: BackendSelection, width: u32, height: u32) -> Result<Self> {
        let (adapter, device, queue) =
            request_headless_device(backends.to_wgpu_backends(), "aphrody-motion-transition")?;
        ensure!(
            width <= device.limits().max_texture_dimension_2d
                && height <= device.limits().max_texture_dimension_2d,
            "{width}x{height} exceeds the GPU texture limit"
        );
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("motion-transition"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                aphrody_media::gpu_yuv::MOTION_TRANSITION_WGSL,
            )),
        });
        let texture_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("motion-transition-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(32),
                    },
                    count: None,
                },
                texture_entry(1),
                texture_entry(2),
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("motion-transition-layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("motion-transition-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_transition"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_transition"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("motion-transition-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..wgpu::SamplerDescriptor::default()
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("motion-transition-uniforms"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let extent = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
        let make_texture = |label: &'static str, usage: wgpu::TextureUsages| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage,
                view_formats: &[],
                texture_binding_view_dimension: None,
            })
        };
        let sampled = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
        let texture_a = make_texture("motion-transition-a", sampled);
        let texture_b = make_texture("motion-transition-b", sampled);
        let target = make_texture(
            "motion-transition-out",
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        );
        let padded_row = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("motion-transition-readback"),
            size: u64::from(padded_row) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        Ok(Self {
            adapter_name: adapter.get_info().name,
            device,
            queue,
            pipeline,
            layout,
            sampler,
            uniforms,
            texture_a,
            texture_b,
            target,
            readback,
            width,
            height,
            padded_row,
        })
    }

    fn upload(&self, texture: &wgpu::Texture, pixmap: &Pixmap) {
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixmap.data(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.width * 4),
                rows_per_image: Some(self.height),
            },
            wgpu::Extent3d { width: self.width, height: self.height, depth_or_array_layers: 1 },
        );
    }
}

impl TransitionBlender for GpuBlender {
    fn blend(
        &mut self,
        before: &Pixmap,
        after: &Pixmap,
        state: TransitionState,
    ) -> Result<Pixmap> {
        ensure!(
            before.width() == self.width && before.height() == self.height,
            "frame size differs from the GPU blender size"
        );
        self.upload(&self.texture_a, before);
        self.upload(&self.texture_b, after);
        let kind = match state.kind {
            TransitionKind::CrossFade => 0,
            TransitionKind::LinearWipe => 1,
            TransitionKind::Dissolve => 2,
            TransitionKind::ZoomBlur => 3,
        };
        let uniforms = TransitionUniforms {
            progress: state.progress,
            kind,
            feather: state.feather,
            padding: 0.0,
            width: self.width as f32,
            height: self.height as f32,
            padding_size: [0.0; 2],
        };
        self.queue
            .write_buffer(&self.uniforms, 0, bytemuck::bytes_of(&uniforms));
        let view_a = self.texture_a.create_view(&wgpu::TextureViewDescriptor::default());
        let view_b = self.texture_b.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("motion-transition-bind-group"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.uniforms.as_entire_binding() },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view_a),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&view_b),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let target_view = self.target.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("motion-transition") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("motion-transition-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.padded_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d { width: self.width, height: self.height, depth_or_array_layers: 1 },
        );
        let submission = self.queue.submit(std::iter::once(encoder.finish()));
        let padded = read_mapped_buffer(&self.device, &self.readback, submission)?;

        let row = (self.width * 4) as usize;
        let mut data = Vec::with_capacity(row * self.height as usize);
        for line in 0..self.height as usize {
            let start = line * self.padded_row as usize;
            let slice = padded
                .get(start..start + row)
                .ok_or_else(|| anyhow!("short GPU readback"))?;
            data.extend_from_slice(slice);
        }
        let size = aphrody_media::motion_render::tiny_skia::IntSize::from_wh(self.width, self.height)
            .ok_or_else(|| anyhow!("invalid frame size"))?;
        Pixmap::from_vec(data, size)
            .ok_or_else(|| anyhow!("GPU readback did not match the frame size"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMPOSITION: &str = r##"{ "id": "cli", "width": 32, "height": 18, "fps": 10,
        "variables": { "label": "hi" },
        "tracks": [ { "id": "main", "clips": [
          { "id": "bg", "duration": 1, "layer": { "type": "shape", "fill": "#204080" } },
          { "id": "text", "start": 0.3, "duration": 0.7, "layer": { "type": "text", "text": "{{label}}", "size": 8 },
            "transition_in": { "kind": "cross_fade", "duration": 0.3 } } ] } ] }"##;

    fn write_composition(directory: &Path) -> PathBuf {
        let path = directory.join("cli.json");
        std::fs::write(&path, COMPOSITION).expect("write composition");
        path
    }

    #[test]
    fn render_writes_png_frames_and_an_mp4_with_a_stable_hash() {
        let directory = tempfile::tempdir().expect("tempdir");
        let composition = write_composition(directory.path());
        let make_args = |frames: &str, video: &str| RenderArgs {
            composition: CompositionArgs {
                composition: composition.clone(),
                variables: vec![("label".into(), "ok".into())],
            },
            frames_dir: Some(directory.path().join(frames)),
            video: Some(directory.path().join(video)),
            crf: 28.0,
            start_frame: 0,
            end_frame: None,
            gpu: false,
            backends: BackendSelection::All,
        };
        let first = render_report(&make_args("a", "a.mp4")).expect("first render");
        let second = render_report(&make_args("b", "b.mp4")).expect("second render");
        assert_eq!(first["frames_rendered"], 10);
        assert_eq!(first["frames_sha256"], second["frames_sha256"]);
        assert_eq!(first["video"]["sha256"], second["video"]["sha256"]);
        let pngs = std::fs::read_dir(directory.path().join("a")).expect("frames dir").count();
        assert_eq!(pngs, 10);
        let mp4 = std::fs::read(directory.path().join("a.mp4")).expect("mp4");
        assert_eq!(&mp4[4..8], b"ftyp");
    }

    #[test]
    fn timeline_report_lists_frame_ranges() {
        let directory = tempfile::tempdir().expect("tempdir");
        let composition = write_composition(directory.path());
        let report = timeline_report(&CompositionArgs { composition, variables: Vec::new() })
            .expect("timeline");
        assert_eq!(report["duration_frames"], 10);
        assert_eq!(report["clips"][1]["start_frame"], 3);
        assert_eq!(report["paint_order"][1], "text");
    }

    #[test]
    #[ignore = "needs a Vulkan/Metal/DX12 adapter; run with --ignored"]
    fn gpu_blender_matches_the_cpu_reference_within_rounding() {
        use aphrody_media::CpuBlender;
        let (width, height) = (32_u32, 16_u32);
        let mut before = Pixmap::new(width, height).expect("pixmap");
        let mut after = Pixmap::new(width, height).expect("pixmap");
        for (index, pixel) in before.pixels_mut().iter_mut().enumerate() {
            let value = (index % 251) as u8;
            *pixel = aphrody_media::motion_render::tiny_skia::ColorU8::from_rgba(value, 40, 200, 255).premultiply();
        }
        for (index, pixel) in after.pixels_mut().iter_mut().enumerate() {
            let value = (index * 7 % 251) as u8;
            *pixel = aphrody_media::motion_render::tiny_skia::ColorU8::from_rgba(10, value, 90, 255).premultiply();
        }
        let mut gpu = GpuBlender::new(BackendSelection::All, width, height).expect("gpu blender");
        for kind in [TransitionKind::CrossFade, TransitionKind::LinearWipe, TransitionKind::Dissolve] {
            let state = TransitionState { kind, progress: 0.4, feather: 0.1 };
            let expected = CpuBlender.blend(&before, &after, state).expect("cpu");
            let actual = gpu.blend(&before, &after, state).expect("gpu");
            let worst = expected
                .data()
                .iter()
                .zip(actual.data())
                .map(|(a, b)| a.abs_diff(*b))
                .max()
                .unwrap_or(0);
            assert!(worst <= 2, "{kind:?} differs by {worst} levels");
        }
    }
}
