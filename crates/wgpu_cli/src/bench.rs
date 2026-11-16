use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, bail, ensure};
use clap::Args;
use serde::Serialize;

use crate::{BackendSelection, request_headless_device};

const BENCH_COMPUTE_WGSL: &str = r#"
@group(0) @binding(0)
var<storage, read_write> data: array<u32>;

@compute @workgroup_size(64, 1, 1)
fn cs_bench(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if index < arrayLength(&data) {
        var value = data[index] ^ (index * 2654435761u);
        value = (value ^ 61u) ^ (value >> 16u);
        value = value + (value << 3u);
        value = value ^ (value >> 4u);
        value = value * 668265261u;
        data[index] = value ^ (value >> 15u);
    }
}
"#;

const BENCH_RENDER_WGSL: &str = r#"
struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_bench(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    let x = f32((vertex_index << 1u) & 2u);
    let y = f32(vertex_index & 2u);
    var out: VertexOut;
    out.position = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    out.uv = vec2<f32>(x, y);
    return out;
}

@fragment
fn fs_bench(input: VertexOut) -> @location(0) vec4<f32> {
    let wave_x = 0.5 + 0.5 * cos(input.uv.x * 12.56637);
    let wave_y = 0.5 + 0.5 * sin(input.uv.y * 12.56637);
    return vec4<f32>(wave_x, wave_y, 0.5 * (wave_x + wave_y), 1.0);
}
"#;

#[derive(Debug, Args)]
pub(crate) struct BenchArgs {
    /// Number of timed iterations per microbenchmark
    #[arg(long, default_value_t = 50)]
    pub iterations: u32,

    /// Render target dimension (`size x size`) and upload/compute sizing factor
    #[arg(long, default_value_t = 1024)]
    pub size: u32,

    /// Emit machine-readable JSON benchmark report
    #[arg(long)]
    pub json: bool,

    /// Backend filter (`all`, `vulkan`, `dx12`, `metal`, `gl`)
    #[arg(long, value_enum, default_value_t = BackendSelection::All)]
    pub backends: BackendSelection,
}

#[derive(Debug, Serialize)]
struct BenchReport {
    adapter_name: String,
    backend: String,
    device_type: String,
    iterations: u32,
    size: u32,
    device_init_ms: f64,
    buffer_upload: UploadBenchMetrics,
    compute_dispatch: LatencyMetrics,
    render_pass: RenderBenchMetrics,
}

#[derive(Debug, Serialize)]
struct UploadBenchMetrics {
    buffer_bytes: u64,
    total_uploaded_bytes: u64,
    avg_us: f64,
    min_us: f64,
    p95_us: f64,
    bandwidth_gb_per_sec: f64,
}

#[derive(Debug, Serialize)]
struct LatencyMetrics {
    elements: u32,
    workgroups_x: u32,
    avg_us: f64,
    min_us: f64,
    p95_us: f64,
}

#[derive(Debug, Serialize)]
struct RenderBenchMetrics {
    width: u32,
    height: u32,
    avg_us: f64,
    min_us: f64,
    p95_us: f64,
    frames_per_sec: f64,
    megapixels_per_sec: f64,
}

pub(crate) fn run(args: BenchArgs) -> Result<()> {
    ensure!(args.iterations > 0, "--iterations must be greater than zero");
    ensure!(args.size > 0, "--size must be greater than zero");

    let init_start = Instant::now();
    let (adapter, device, queue) =
        request_headless_device(args.backends.to_wgpu_backends(), "gpui-wgpu-bench")?;
    let device_init_ms = init_start.elapsed().as_secs_f64() * 1_000.0;

    let max_dim = device.limits().max_texture_dimension_2d;
    ensure!(
        args.size <= max_dim,
        "requested --size {} exceeds device max_texture_dimension_2d ({max_dim})",
        args.size
    );

    let validation_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);

    let upload_metrics = bench_buffer_upload(&device, &queue, args.iterations, args.size)?;
    let compute_metrics = bench_compute_dispatch(&device, &queue, args.iterations, args.size)?;
    let render_metrics = bench_offscreen_render(&device, &queue, args.iterations, args.size)?;

    if let Some(validation_error) = pollster::block_on(validation_scope.pop()) {
        bail!("WGPU benchmark validation error: {validation_error}");
    }

    let info = adapter.get_info();
    let report = BenchReport {
        adapter_name: info.name,
        backend: format!("{:?}", info.backend),
        device_type: format!("{:?}", info.device_type),
        iterations: args.iterations,
        size: args.size,
        device_init_ms,
        buffer_upload: upload_metrics,
        compute_dispatch: compute_metrics,
        render_pass: render_metrics,
    };

    if args.json {
        let json =
            serde_json::to_string_pretty(&report).context("failed to serialize benchmark report")?;
        println!("{json}");
        return Ok(());
    }

    println!(
        "WGPU Benchmark — '{}' ({}, {}) | iterations={} | size={}x{}",
        report.adapter_name,
        report.backend,
        report.device_type,
        report.iterations,
        report.size,
        report.size
    );
    println!("  1. Adapter + Device Init: {:8.2} ms", report.device_init_ms);
    println!(
        "  2. Buffer Upload ({} KiB/iter): avg={:8.2} us | min={:8.2} us | p95={:8.2} us | {:6.2} GB/s",
        report.buffer_upload.buffer_bytes / 1024,
        report.buffer_upload.avg_us,
        report.buffer_upload.min_us,
        report.buffer_upload.p95_us,
        report.buffer_upload.bandwidth_gb_per_sec
    );
    println!(
        "  3. Headless Compute ({} elems, {} wgs): avg={:8.2} us | min={:8.2} us | p95={:8.2} us",
        report.compute_dispatch.elements,
        report.compute_dispatch.workgroups_x,
        report.compute_dispatch.avg_us,
        report.compute_dispatch.min_us,
        report.compute_dispatch.p95_us
    );
    println!(
        "  4. Offscreen Render ({}x{} Rgba8Unorm): avg={:8.2} us | min={:8.2} us | p95={:8.2} us | {:8.1} FPS ({:6.1} MP/s)",
        report.render_pass.width,
        report.render_pass.height,
        report.render_pass.avg_us,
        report.render_pass.min_us,
        report.render_pass.p95_us,
        report.render_pass.frames_per_sec,
        report.render_pass.megapixels_per_sec
    );

    Ok(())
}

fn wait_for_submission(
    device: &wgpu::Device,
    submission_index: wgpu::SubmissionIndex,
) -> Result<()> {
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission_index),
            timeout: Some(Duration::from_secs(30)),
        })
        .context("GPU device poll timed out during benchmark")?;
    Ok(())
}

fn bench_buffer_upload(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    iterations: u32,
    size: u32,
) -> Result<UploadBenchMetrics> {
    let raw_bytes = u64::from(size)
        .saturating_mul(u64::from(size))
        .saturating_mul(4)
        .clamp(4096, 16 * 1024 * 1024)
        .min(device.limits().max_buffer_size);
    let buffer_bytes = (raw_bytes / 4) * 4;

    let payload = vec![0xA5u8; buffer_bytes as usize];
    let gpu_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("gpui-wgpu-bench-upload-buffer"),
        size: buffer_bytes,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });

    // Warm up staging ring allocator before recording samples.
    queue.write_buffer(&gpu_buffer, 0, &payload);
    let warmup_submission = queue.submit(std::iter::empty());
    wait_for_submission(device, warmup_submission)?;

    let mut samples_us = Vec::with_capacity(iterations as usize);
    let total_start = Instant::now();
    for _ in 0..iterations {
        let iter_start = Instant::now();
        queue.write_buffer(&gpu_buffer, 0, &payload);
        let submission = queue.submit(std::iter::empty());
        wait_for_submission(device, submission)?;
        samples_us.push(iter_start.elapsed().as_secs_f64() * 1_000_000.0);
    }
    let total_secs = total_start.elapsed().as_secs_f64().max(1e-9);
    let total_uploaded_bytes = buffer_bytes * u64::from(iterations);
    let bandwidth_gb_per_sec = (total_uploaded_bytes as f64 / total_secs) / 1_000_000_000.0;

    let (avg_us, min_us, p95_us) = summarize_samples(&mut samples_us);
    Ok(UploadBenchMetrics {
        buffer_bytes,
        total_uploaded_bytes,
        avg_us,
        min_us,
        p95_us,
        bandwidth_gb_per_sec,
    })
}

fn bench_compute_dispatch(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    iterations: u32,
    size: u32,
) -> Result<LatencyMetrics> {
    let elements = size.saturating_mul(size).clamp(64, 4 * 1024 * 1024);
    let buffer_bytes = (u64::from(elements) * 4).min(device.limits().max_storage_buffer_binding_size);
    let clamped_elements = (buffer_bytes / 4) as u32;
    let workgroups_x = clamped_elements
        .div_ceil(64)
        .min(device.limits().max_compute_workgroups_per_dimension);

    let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("gpui-wgpu-bench-compute-shader"),
        source: wgpu::ShaderSource::Wgsl(BENCH_COMPUTE_WGSL.into()),
    });

    let storage_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("gpui-wgpu-bench-compute-buffer"),
        size: u64::from(clamped_elements) * 4,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("gpui-wgpu-bench-compute-bgl"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("gpui-wgpu-bench-compute-bg"),
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: storage_buffer.as_entire_binding(),
        }],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("gpui-wgpu-bench-compute-pl"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("gpui-wgpu-bench-compute-pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader_module,
        entry_point: Some("cs_bench"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });

    let record_dispatch = || -> wgpu::CommandBuffer {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("gpui-wgpu-bench-compute-encoder"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("gpui-wgpu-bench-compute-pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups_x, 1, 1);
        }
        encoder.finish()
    };

    let warmup_submission = queue.submit(std::iter::once(record_dispatch()));
    wait_for_submission(device, warmup_submission)?;

    let mut samples_us = Vec::with_capacity(iterations as usize);
    for _ in 0..iterations {
        let iter_start = Instant::now();
        let submission = queue.submit(std::iter::once(record_dispatch()));
        wait_for_submission(device, submission)?;
        samples_us.push(iter_start.elapsed().as_secs_f64() * 1_000_000.0);
    }

    let (avg_us, min_us, p95_us) = summarize_samples(&mut samples_us);
    Ok(LatencyMetrics {
        elements: clamped_elements,
        workgroups_x,
        avg_us,
        min_us,
        p95_us,
    })
}

fn bench_offscreen_render(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    iterations: u32,
    size: u32,
) -> Result<RenderBenchMetrics> {
    let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("gpui-wgpu-bench-render-shader"),
        source: wgpu::ShaderSource::Wgsl(BENCH_RENDER_WGSL.into()),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("gpui-wgpu-bench-render-pl"),
        bind_group_layouts: &[],
        immediate_size: 0,
    });
    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("gpui-wgpu-bench-render-pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader_module,
            entry_point: Some("vs_bench"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
            entry_point: Some("fs_bench"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });

    let target_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("gpui-wgpu-bench-render-target"),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let target_view = target_texture.create_view(&wgpu::TextureViewDescriptor::default());

    let record_frame = || -> wgpu::CommandBuffer {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("gpui-wgpu-bench-render-encoder"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("gpui-wgpu-bench-render-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&render_pipeline);
            pass.draw(0..3, 0..1);
        }
        encoder.finish()
    };

    let warmup_submission = queue.submit(std::iter::once(record_frame()));
    wait_for_submission(device, warmup_submission)?;

    let mut samples_us = Vec::with_capacity(iterations as usize);
    for _ in 0..iterations {
        let iter_start = Instant::now();
        let submission = queue.submit(std::iter::once(record_frame()));
        wait_for_submission(device, submission)?;
        samples_us.push(iter_start.elapsed().as_secs_f64() * 1_000_000.0);
    }

    let (avg_us, min_us, p95_us) = summarize_samples(&mut samples_us);
    let frames_per_sec = if avg_us > 0.0 {
        1_000_000.0 / avg_us
    } else {
        0.0
    };
    let megapixels_per_sec =
        (f64::from(size) * f64::from(size) * frames_per_sec) / 1_000_000.0;

    Ok(RenderBenchMetrics {
        width: size,
        height: size,
        avg_us,
        min_us,
        p95_us,
        frames_per_sec,
        megapixels_per_sec,
    })
}

fn summarize_samples(samples: &mut [f64]) -> (f64, f64, f64) {
    if samples.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    samples.sort_by(f64::total_cmp);
    let sum: f64 = samples.iter().sum();
    let avg = sum / (samples.len() as f64);
    let min = samples[0];
    let p95_index = ((samples.len() - 1) * 95) / 100;
    let p95 = samples[p95_index];
    (avg, min, p95)
}
