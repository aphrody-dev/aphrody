use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context as _, Result, bail};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct VulkanProbeReport {
    timestamp: String,
    adapter_count: usize,
    selected_adapter: SelectedAdapterInfo,
    naga_spirv_validation: NagaSpirvValidationResult,
    compute_dispatch: ComputeDispatchResult,
    render_dispatch: RenderDispatchResult,
}

#[derive(Debug, Serialize)]
struct SelectedAdapterInfo {
    name: String,
    vendor_id: u32,
    device_id: u32,
    device_type: String,
    driver: String,
    driver_info: String,
    backend: String,
    features_count: usize,
}

#[derive(Debug, Serialize)]
struct NagaSpirvValidationResult {
    wgsl_source_bytes: usize,
    spirv_word_count: usize,
    spirv_magic_number: String,
    spirv_generator_magic: u32,
    valid_spirv_header: bool,
}

#[derive(Debug, Serialize)]
struct ComputeDispatchResult {
    workgroup_count: u32,
    buffer_elements: usize,
    sample_outputs: Vec<f32>,
    status: String,
}

#[derive(Debug, Serialize)]
struct RenderDispatchResult {
    width: u32,
    height: u32,
    texture_format: String,
    output_png_path: String,
    center_pixel_rgba: [u8; 4],
    status: String,
}

const COMPUTE_WGSL: &str = r#"
@group(0) @binding(0)
var<storage, read_write> wave_buffer: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= arrayLength(&wave_buffer)) {
        return;
    }
    let coordinate = f32(index) * 0.05;
    let wave_value = sin(coordinate) * cos(coordinate * 0.5) + 0.5;
    wave_buffer[index] = wave_value;
}
"#;

const RENDER_WGSL: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let x = f32((vertex_index << 1u) & 2u);
    let y = f32(vertex_index & 2u);
    var output: VertexOutput;
    output.position = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    output.uv = vec2<f32>(x, y);
    return output;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let red_channel = in.uv.x;
    let green_channel = in.uv.y;
    let blue_channel = 0.5 * (sin(in.uv.x * 6.28318) + 1.0);
    return vec4<f32>(red_channel, green_channel, blue_channel, 1.0);
}
"#;

fn read_mapped_buffer_helper(
    device: &wgpu::Device,
    buffer: &wgpu::Buffer,
    submission_index: wgpu::SubmissionIndex,
) -> Result<Vec<u8>> {
    let map_status: Arc<Mutex<Option<Result<(), wgpu::BufferAsyncError>>>> =
        Arc::new(Mutex::new(None));
    let callback_status = Arc::clone(&map_status);

    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            match callback_status.lock() {
                Ok(mut guard) => *guard = Some(result),
                Err(poisoned) => *poisoned.into_inner() = Some(result),
            }
        });

    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission_index),
            timeout: Some(Duration::from_secs(30)),
        })
        .context("timed out or failed polling device while waiting for buffer map")?;

    let callback_result = match map_status.lock() {
        Ok(mut guard) => guard.take(),
        Err(poisoned) => poisoned.into_inner().take(),
    };

    match callback_result {
        Some(Ok(())) => {}
        Some(Err(error)) => bail!("failed to map GPU buffer asynchronously: {error}"),
        None => bail!("asynchronous buffer map callback did not complete after device poll"),
    }

    let mapped_view = buffer
        .slice(..)
        .get_mapped_range()
        .context("failed to get mapped Vulkan GPU buffer range")?;
    let data_copy = mapped_view.to_vec();
    drop(mapped_view);
    buffer.unmap();

    Ok(data_copy)
}

fn main() -> Result<()> {
    println!("=== gpui-os Vulkan Backend Probe ===");

    // 1. Initialize Vulkan instance using exact wgpu v30 descriptor API
    let mut instance_descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    instance_descriptor.backends = wgpu::Backends::VULKAN;
    let instance = wgpu::Instance::new(instance_descriptor.with_env());

    // 2. Enumerate adapters on the Vulkan backend
    let mut adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::VULKAN));
    println!("Enumerated {} Vulkan adapter(s)", adapters.len());

    if adapters.is_empty() {
        bail!("no Vulkan adapters detected on the system");
    }

    // Sort to prioritize discrete GPU over integrated/virtual adapters
    adapters.sort_by_key(|adapter| match adapter.get_info().device_type {
        wgpu::DeviceType::DiscreteGpu => 0,
        wgpu::DeviceType::IntegratedGpu => 1,
        wgpu::DeviceType::VirtualGpu => 2,
        wgpu::DeviceType::Cpu => 3,
        wgpu::DeviceType::Other => 4,
    });

    let adapter_count = adapters.len();
    let selected_adapter = adapters
        .into_iter()
        .next()
        .context("failed to select primary Vulkan adapter")?;

    let adapter_info = selected_adapter.get_info();
    println!(
        "Selected Adapter: {} (Vendor: 0x{:04x}, Device: 0x{:04x}, Type: {:?})",
        adapter_info.name, adapter_info.vendor, adapter_info.device, adapter_info.device_type
    );
    println!("Driver: {} | {}", adapter_info.driver, adapter_info.driver_info);

    // 3. Request logical device and queue
    let adapter_limits = selected_adapter.limits();
    let required_limits = wgpu::Limits::default().or_worse_values_from(&adapter_limits);

    let (device, queue) = pollster::block_on(selected_adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("gpui_vulkan_backend_probe_device"),
        required_features: wgpu::Features::empty(),
        required_limits,
        default_queue: wgpu::QueueDescriptor::default(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::MemoryUsage,
        trace: wgpu::Trace::Off,
    }))
    .context("failed to create Vulkan logical device")?;

    // 4. Test programmatic Naga SPIR-V pipeline
    println!("\nValidating Naga WGSL -> SPIR-V pipeline...");
    let naga_module = naga::front::wgsl::parse_str(COMPUTE_WGSL)
        .context("failed to parse compute WGSL with naga front-end")?;

    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    let module_info = validator
        .validate(&naga_module)
        .context("failed to validate compute Naga IR module")?;

    let spv_options = naga::back::spv::Options::default();
    let spirv_words: Vec<u32> = naga::back::spv::write_vec(
        &naga_module,
        &module_info,
        &spv_options,
        None,
    )
    .context("failed to emit SPIR-V bytecode words from Naga module")?;

    let spirv_magic = spirv_words
        .first()
        .copied()
        .context("SPIR-V word vector is empty")?;
    let spirv_generator = spirv_words
        .get(2)
        .copied()
        .context("SPIR-V header missing generator magic word")?;
    let valid_header = spirv_magic == 0x07230203;

    println!(
        "Naga SPIR-V generated: {} words ({} bytes), magic: 0x{:08x} (valid: {})",
        spirv_words.len(),
        spirv_words.len() * 4,
        spirv_magic,
        valid_header
    );

    // 5. Headless GPU Compute Dispatch on Vulkan
    println!("\nExecuting headless Vulkan compute dispatch...");
    let compute_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("vulkan_probe_compute_shader"),
        source: wgpu::ShaderSource::Wgsl(COMPUTE_WGSL.into()),
    });

    const BUFFER_ELEMENT_COUNT: usize = 256;
    let buffer_byte_size = (BUFFER_ELEMENT_COUNT * std::mem::size_of::<f32>()) as wgpu::BufferAddress;

    let storage_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("vulkan_probe_storage_buffer"),
        size: buffer_byte_size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let staging_compute_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("vulkan_probe_staging_compute_buffer"),
        size: buffer_byte_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let compute_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vulkan_probe_compute_bind_group_layout"),
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

    let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("vulkan_probe_compute_bind_group"),
        layout: &compute_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: storage_buffer.as_entire_binding(),
        }],
    });

    let compute_pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("vulkan_probe_compute_pipeline_layout"),
            bind_group_layouts: &[Some(&compute_bind_group_layout)],
            immediate_size: 0,
        });

    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("vulkan_probe_compute_pipeline"),
        layout: Some(&compute_pipeline_layout),
        module: &compute_shader_module,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });

    let mut compute_encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("vulkan_probe_compute_encoder"),
        });

    {
        let mut compute_pass =
            compute_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("vulkan_probe_compute_pass"),
                timestamp_writes: None,
            });
        compute_pass.set_pipeline(&compute_pipeline);
        compute_pass.set_bind_group(0, &compute_bind_group, &[]);
        compute_pass.dispatch_workgroups((BUFFER_ELEMENT_COUNT as u32 + 63) / 64, 1, 1);
    }

    compute_encoder.copy_buffer_to_buffer(
        &storage_buffer,
        0,
        &staging_compute_buffer,
        0,
        buffer_byte_size,
    );

    let compute_submission = queue.submit(Some(compute_encoder.finish()));
    let compute_raw_bytes =
        read_mapped_buffer_helper(&device, &staging_compute_buffer, compute_submission)?;
    let compute_floats: &[f32] = bytemuck::cast_slice(&compute_raw_bytes);

    let sample_outputs: Vec<f32> = compute_floats.iter().take(8).copied().collect();
    println!("Compute dispatch complete. First 8 sample floats: {sample_outputs:?}");

    // 6. Headless Offscreen Render Pass on Vulkan
    println!("\nExecuting headless Vulkan render pass (256x256 RGBA8)...");
    const RENDER_WIDTH: u32 = 256;
    const RENDER_HEIGHT: u32 = 256;

    let render_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("vulkan_probe_render_shader"),
        source: wgpu::ShaderSource::Wgsl(RENDER_WGSL.into()),
    });

    let render_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("vulkan_probe_render_target"),
        size: wgpu::Extent3d {
            width: RENDER_WIDTH,
            height: RENDER_HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    let render_texture_view = render_texture.create_view(&wgpu::TextureViewDescriptor::default());

    let render_pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("vulkan_probe_render_pipeline_layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("vulkan_probe_render_pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &render_shader_module,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &render_shader_module,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });

    let bytes_per_pixel = 4u32;
    let unpadded_bytes_per_row = RENDER_WIDTH * bytes_per_pixel;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded_bytes_per_row = ((unpadded_bytes_per_row + align - 1) / align) * align;
    let render_buffer_size = (padded_bytes_per_row * RENDER_HEIGHT) as wgpu::BufferAddress;

    let staging_render_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("vulkan_probe_staging_render_buffer"),
        size: render_buffer_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut render_encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("vulkan_probe_render_encoder"),
        });

    {
        let mut render_pass =
            render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("vulkan_probe_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &render_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.1,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

        render_pass.set_pipeline(&render_pipeline);
        render_pass.draw(0..3, 0..1);
    }

    render_encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &render_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &staging_render_buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(RENDER_HEIGHT),
            },
        },
        wgpu::Extent3d {
            width: RENDER_WIDTH,
            height: RENDER_HEIGHT,
            depth_or_array_layers: 1,
        },
    );

    let render_submission = queue.submit(Some(render_encoder.finish()));
    let render_raw_bytes =
        read_mapped_buffer_helper(&device, &staging_render_buffer, render_submission)?;

    // Extract tightly packed RGBA pixels (stripping any row padding if needed)
    let mut packed_pixels: Vec<u8> =
        Vec::with_capacity((RENDER_WIDTH * RENDER_HEIGHT * bytes_per_pixel) as usize);
    for row_index in 0..RENDER_HEIGHT {
        let start = (row_index * padded_bytes_per_row) as usize;
        let end = start + unpadded_bytes_per_row as usize;
        packed_pixels.extend_from_slice(&render_raw_bytes[start..end]);
    }

    let center_pixel_index =
        ((RENDER_HEIGHT / 2 * RENDER_WIDTH + RENDER_WIDTH / 2) * bytes_per_pixel) as usize;
    let center_pixel_rgba = [
        packed_pixels[center_pixel_index],
        packed_pixels[center_pixel_index + 1],
        packed_pixels[center_pixel_index + 2],
        packed_pixels[center_pixel_index + 3],
    ];

    let output_png = PathBuf::from("target/vulkan_probe_render.png");
    if let Some(parent_dir) = output_png.parent() {
        if !parent_dir.exists() {
            std::fs::create_dir_all(parent_dir)
                .context("failed to create output parent directory")?;
        }
    }

    image::save_buffer(
        &output_png,
        &packed_pixels,
        RENDER_WIDTH,
        RENDER_HEIGHT,
        image::ExtendedColorType::Rgba8,
    )
    .with_context(|| format!("failed to save render PNG to '{}'", output_png.display()))?;

    println!(
        "Render pass complete. Saved PNG: {} (Center pixel RGBA: {:?})",
        output_png.display(),
        center_pixel_rgba
    );

    // 7. Write JSON report
    let probe_report = VulkanProbeReport {
        timestamp: "2026-11-16T12:00:00Z".to_string(),
        adapter_count,
        selected_adapter: SelectedAdapterInfo {
            name: adapter_info.name,
            vendor_id: adapter_info.vendor,
            device_id: adapter_info.device,
            device_type: format!("{:?}", adapter_info.device_type),
            driver: adapter_info.driver,
            driver_info: adapter_info.driver_info,
            backend: format!("{:?}", adapter_info.backend),
            features_count: selected_adapter.features().iter().count(),
        },
        naga_spirv_validation: NagaSpirvValidationResult {
            wgsl_source_bytes: COMPUTE_WGSL.len(),
            spirv_word_count: spirv_words.len(),
            spirv_magic_number: format!("0x{:08x}", spirv_magic),
            spirv_generator_magic: spirv_generator,
            valid_spirv_header: valid_header,
        },
        compute_dispatch: ComputeDispatchResult {
            workgroup_count: (BUFFER_ELEMENT_COUNT as u32 + 63) / 64,
            buffer_elements: BUFFER_ELEMENT_COUNT,
            sample_outputs,
            status: "SUCCESS".to_string(),
        },
        render_dispatch: RenderDispatchResult {
            width: RENDER_WIDTH,
            height: RENDER_HEIGHT,
            texture_format: "Rgba8Unorm".to_string(),
            output_png_path: output_png.display().to_string(),
            center_pixel_rgba,
            status: "SUCCESS".to_string(),
        },
    };

    let report_json = serde_json::to_string_pretty(&probe_report)
        .context("failed to serialize Vulkan probe report to JSON")?;

    let report_path = PathBuf::from("target/vulkan_probe_report.json");
    std::fs::write(&report_path, &report_json)
        .with_context(|| format!("failed to write probe report to '{}'", report_path.display()))?;

    println!("Report written to '{}'", report_path.display());
    println!("\n=== Vulkan Backend Probe Completed Successfully ===");
    Ok(())
}
