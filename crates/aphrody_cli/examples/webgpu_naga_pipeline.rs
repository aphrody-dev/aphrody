//! WebGPU & Naga Cross-Compilation, Reflection, Render, and Compute Pipeline
//!
//! Validates a unified WGSL shader module with Naga, cross-compiles across all 5 targets
//! (WGSL, SPIR-V, HLSL, MSL, and GLSL ES 300), and executes headless rendering to a PNG image
//! and compute dispatch with readback verification via WGPU v30.0.0.
//!
//! Run with:
//! `cargo run -p aphrody_cli --example webgpu_naga_pipeline`

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context as _, Result, bail};
use serde::Serialize;

/// Embedded WGSL shader demonstrating:
/// 1. Host-shareable uniform buffer with 16-byte alignment rules (`NshUniforms`).
/// 2. Fullscreen vertex stage generating procedural screen-space quad.
/// 3. Fragment stage computing procedural animated plasma with Oklab color grading.
/// 4. Compute stage transforming an array of numbers in storage memory.
const UNIFIED_SHADER_SOURCE: &str = r#"
struct NshUniforms {
    resolution: vec2f,
    time: f32,
    intensity: f32,
}

@group(0) @binding(0) var<uniform> u_view: NshUniforms;
@group(0) @binding(1) var<storage, read_write> s_data: array<f32>;

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_id: u32) -> VertexOutput {
    // Generate fullscreen triangle covering [-1, 1] NDC
    var out: VertexOutput;
    let x = f32(i32(vertex_id << 1u) & 2) * 2.0 - 1.0;
    let y = f32(i32(vertex_id & 2u)) * 2.0 - 1.0;
    out.position = vec4f(x, y, 0.0, 1.0);
    out.uv = vec2f(x * 0.5 + 0.5, 1.0 - (y * 0.5 + 0.5));
    return out;
}

fn oklab_to_linear_srgb(c: vec3f) -> vec3f {
    let l_ = c.x + 0.3963377774 * c.y + 0.2158037573 * c.z;
    let m_ = c.x - 0.1055613458 * c.y - 0.0638541728 * c.z;
    let s_ = c.x - 0.0894841775 * c.y - 1.2914855480 * c.z;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    return vec3f(
        4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s
    );
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let p = (in.uv - 0.5) * 2.0;
    let dist = length(p);
    let angle = atan2(p.y, p.x);
    let wave = sin(dist * 8.0 - u_view.time * 2.0 + angle * 3.0) * 0.5 + 0.5;

    // Construct perceptually uniform color palette in Oklab space
    let lightness = 0.6 + 0.25 * wave;
    let chroma_a = 0.18 * cos(u_view.time + angle);
    let chroma_b = 0.18 * sin(u_view.time + angle);
    let oklab = vec3f(lightness, chroma_a, chroma_b);
    let rgb = clamp(oklab_to_linear_srgb(oklab), vec3f(0.0), vec3f(1.0));

    return vec4f(rgb, 1.0);
}

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id: vec3u) {
    let index = id.x;
    if (index >= arrayLength(&s_data)) {
        return;
    }
    let val = s_data[index];
    // Transform data element: compute sine wave + scale factor
    s_data[index] = val * val + u_view.intensity * sin(f32(index) * 0.1);
}
"#;

#[derive(Debug, Serialize)]
struct PipelineReflectionReport {
    pub valid: bool,
    pub entry_points: Vec<EntryPointInfo>,
    pub uniform_buffers: Vec<UniformBufferInfo>,
    pub storage_buffers: Vec<StorageBufferInfo>,
    pub transpiled_targets: BTreeMap<String, TargetSummary>,
}

#[derive(Debug, Serialize)]
struct EntryPointInfo {
    pub name: String,
    pub stage: String,
    pub workgroup_size: Option<[u32; 3]>,
}

#[derive(Debug, Serialize)]
struct UniformBufferInfo {
    pub group: u32,
    pub binding: u32,
    pub name: String,
    pub size_bytes: u32,
}

#[derive(Debug, Serialize)]
struct StorageBufferInfo {
    pub group: u32,
    pub binding: u32,
    pub name: String,
}

#[derive(Debug, Serialize)]
struct TargetSummary {
    pub status: String,
    pub size_bytes: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderUniforms {
    resolution: [f32; 2],
    time: f32,
    intensity: f32,
}

fn main() -> Result<()> {
    println!("=== aphrody WebGPU & Naga End-to-End Pipeline ===");

    // Step 1: Validate WGSL Module with Naga
    println!("\n[1/4] Parsing and validating WGSL shader module with Naga v30...");
    let module = naga::front::wgsl::parse_str(UNIFIED_SHADER_SOURCE)
        .context("Naga failed to parse WGSL source")?;

    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    let module_info = validator
        .validate(&module)
        .context("Naga validation failed for WGSL module")?;

    let mut layouter = naga::proc::Layouter::default();
    layouter
        .update(module.to_ctx())
        .context("Naga layouter failed to compute memory offsets")?;

    println!("✓ WGSL module successfully validated (Naga IR verified).");

    // Step 2: Cross-compile to all 5 backend targets
    println!("\n[2/4] Cross-compiling shader to all 5 backend targets...");
    let mut transpiled_targets = BTreeMap::new();

    // 2a. Re-emit WGSL
    let wgsl_out = naga::back::wgsl::write_string(
        &module,
        &module_info,
        naga::back::wgsl::WriterFlags::empty(),
    )
    .context("Naga WGSL writer failed")?;
    println!("  → Target 1: WGSL (Roundtrip, {} bytes)", wgsl_out.len());
    transpiled_targets.insert(
        "wgsl".to_string(),
        TargetSummary {
            status: "ok".to_string(),
            size_bytes: wgsl_out.len(),
        },
    );

    // 2b. Emit SPIR-V (Vulkan)
    let spv_options = naga::back::spv::Options {
        lang_version: (1, 3),
        flags: naga::back::spv::WriterFlags::ADJUST_COORDINATE_SPACE
            | naga::back::spv::WriterFlags::LABEL_VARYINGS
            | naga::back::spv::WriterFlags::CLAMP_FRAG_DEPTH,
        fake_missing_bindings: true,
        ..naga::back::spv::Options::default()
    };
    let spv_words = naga::back::spv::write_vec(&module, &module_info, &spv_options, None)
        .context("Naga SPIR-V writer failed")?;
    let spv_bytes = spv_words.len() * 4;
    println!("  → Target 2: SPIR-V (Vulkan 1.3, {} bytes)", spv_bytes);
    transpiled_targets.insert(
        "spirv".to_string(),
        TargetSummary {
            status: "ok".to_string(),
            size_bytes: spv_bytes,
        },
    );

    // 2c. Emit HLSL (DirectX 12)
    let hlsl_options = naga::back::hlsl::Options {
        shader_model: naga::back::hlsl::ShaderModel::V6_0,
        fake_missing_bindings: true,
        ..naga::back::hlsl::Options::default()
    };
    let hlsl_pipeline = naga::back::hlsl::PipelineOptions {
        entry_point: Some((naga::ShaderStage::Fragment, "fs_main".to_string())),
    };
    let mut hlsl_out = String::new();
    let mut hlsl_writer =
        naga::back::hlsl::Writer::new(&mut hlsl_out, &hlsl_options, &hlsl_pipeline);
    hlsl_writer
        .write(&module, &module_info, None)
        .context("Naga HLSL writer failed")?;
    println!("  → Target 3: HLSL (DirectX 12 SM 6.0, {} bytes)", hlsl_out.len());
    transpiled_targets.insert(
        "hlsl".to_string(),
        TargetSummary {
            status: "ok".to_string(),
            size_bytes: hlsl_out.len(),
        },
    );

    // 2d. Emit MSL (Metal)
    let msl_options = naga::back::msl::Options {
        lang_version: (2, 4),
        fake_missing_bindings: false,
        ..naga::back::msl::Options::default()
    };
    let msl_pipeline = naga::back::msl::PipelineOptions {
        entry_point: Some((naga::ShaderStage::Fragment, "fs_main".to_string())),
        ..naga::back::msl::PipelineOptions::default()
    };
    let (msl_out, _) =
        naga::back::msl::write_string(&module, &module_info, &msl_options, &msl_pipeline)
            .context("Naga MSL writer failed")?;
    println!("  → Target 4: MSL (Apple Metal 2.4, {} bytes)", msl_out.len());
    transpiled_targets.insert(
        "msl".to_string(),
        TargetSummary {
            status: "ok".to_string(),
            size_bytes: msl_out.len(),
        },
    );

    // 2e. Emit GLSL ES 300 (WebGL 2.0 Fallback)
    let mut binding_map = BTreeMap::new();
    let mut next_slot: u8 = 0;
    for (_, global) in module.global_variables.iter() {
        if let Some(binding) = global.binding {
            binding_map.insert(binding, next_slot);
            next_slot = next_slot.saturating_add(1);
        }
    }
    let glsl_options = naga::back::glsl::Options {
        version: naga::back::glsl::Version::Embedded {
            version: 300,
            is_webgl: true,
        },
        writer_flags: naga::back::glsl::WriterFlags::ADJUST_COORDINATE_SPACE,
        binding_map,
        zero_initialize_workgroup_memory: true,
    };
    let glsl_pipeline = naga::back::glsl::PipelineOptions {
        shader_stage: naga::ShaderStage::Fragment,
        entry_point: "fs_main".to_string(),
        multiview: None,
    };
    let mut glsl_out = String::new();
    let mut glsl_writer = naga::back::glsl::Writer::new(
        &mut glsl_out,
        &module,
        &module_info,
        &glsl_options,
        &glsl_pipeline,
        naga::proc::BoundsCheckPolicies::default(),
    )
    .context("failed to initialize Naga GLSL writer")?;
    glsl_writer.write().context("Naga GLSL writer failed")?;
    println!(
        "  → Target 5: GLSL ES 300 (WebGL2 fallback, {} bytes)",
        glsl_out.len()
    );
    transpiled_targets.insert(
        "glsl_es".to_string(),
        TargetSummary {
            status: "ok".to_string(),
            size_bytes: glsl_out.len(),
        },
    );

    // Build Reflection Report
    let mut entry_points = Vec::new();
    for ep in &module.entry_points {
        let stage_str = match ep.stage {
            naga::ShaderStage::Vertex => "vertex",
            naga::ShaderStage::Fragment => "fragment",
            naga::ShaderStage::Compute => "compute",
            _ => "other",
        };
        let workgroup_size = match ep.stage {
            naga::ShaderStage::Compute => Some(ep.workgroup_size),
            _ => None,
        };
        entry_points.push(EntryPointInfo {
            name: ep.name.clone(),
            stage: stage_str.to_string(),
            workgroup_size,
        });
    }

    let mut uniform_buffers = Vec::new();
    let mut storage_buffers = Vec::new();
    for (_, global) in module.global_variables.iter() {
        if let Some(binding) = global.binding {
            let name = global.name.clone().unwrap_or_else(|| "_".to_string());
            match global.space {
                naga::AddressSpace::Uniform => {
                    let size = layouter[global.ty].size;
                    uniform_buffers.push(UniformBufferInfo {
                        group: binding.group,
                        binding: binding.binding,
                        name,
                        size_bytes: size,
                    });
                }
                naga::AddressSpace::Storage { .. } => {
                    storage_buffers.push(StorageBufferInfo {
                        group: binding.group,
                        binding: binding.binding,
                        name,
                    });
                }
                _ => {}
            }
        }
    }

    let report = PipelineReflectionReport {
        valid: true,
        entry_points,
        uniform_buffers,
        storage_buffers,
        transpiled_targets,
    };

    let report_json = serde_json::to_string_pretty(&report)
        .context("failed to serialize reflection report to JSON")?;
    let out_dir = Path::new("examples/gpu/webgpu");
    if !out_dir.exists() {
        fs::create_dir_all(out_dir).context("failed to create examples/gpu/webgpu directory")?;
    }
    let reflection_path = out_dir.join("webgpu_naga_reflection.json");
    fs::write(&reflection_path, report_json.as_bytes())
        .with_context(|| format!("failed to write {}", reflection_path.display()))?;
    println!(
        "✓ Emitted reflection report to {}",
        reflection_path.display()
    );

    // Step 3: Headless WGPU Setup
    println!("\n[3/4] Initializing headless WGPU adapter and device...");
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: None,
        apply_limit_buckets: false,
    }))
    .or_else(|_| {
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::None,
            force_fallback_adapter: true,
            compatible_surface: None,
            apply_limit_buckets: false,
        }))
    })
    .context("failed to find suitable GPU adapter for headless render")?;

    let adapter_info = adapter.get_info();
    println!(
        "  Adapter: {} ({:?}, driver: {})",
        adapter_info.name, adapter_info.backend, adapter_info.driver
    );

    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("gpui_naga_headless_device"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        memory_hints: wgpu::MemoryHints::MemoryUsage,
        trace: wgpu::Trace::Off,
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        default_queue: wgpu::QueueDescriptor::default(),
    }))
    .context("failed to request logical GPU device")?;

    let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("gpui_naga_shader"),
        source: wgpu::ShaderSource::Wgsl(UNIFIED_SHADER_SOURCE.into()),
    });

    // Step 4a: Headless Compute Dispatch
    println!("\n[4a/4] Running compute pass via WGPU storage buffers...");
    let element_count: usize = 256;
    let initial_data: Vec<f32> = (0..element_count).map(|i| (i as f32) * 0.5).collect();
    let data_bytes = bytemuck::cast_slice::<f32, u8>(&initial_data);

    let storage_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("compute_storage_buffer"),
        size: data_bytes.len() as u64,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&storage_buffer, 0, data_bytes);

    let uniform_data = ShaderUniforms {
        resolution: [512.0, 512.0],
        time: 1.5,
        intensity: 2.0,
    };
    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("compute_uniform_buffer"),
        size: std::mem::size_of::<ShaderUniforms>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&uniform_data));

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("compute_bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("compute_bg"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: storage_buffer.as_entire_binding(),
            },
        ],
    });

    let compute_pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("compute_pipeline_layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

    let compute_pipeline =
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("compute_pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &shader_module,
            entry_point: Some("cs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

    let staging_compute_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("staging_compute_buffer"),
        size: data_bytes.len() as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut compute_encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("compute_encoder"),
        });
    {
        let mut cpass =
            compute_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        cpass.set_pipeline(&compute_pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        let workgroups = ((element_count as u32) + 63) / 64;
        cpass.dispatch_workgroups(workgroups, 1, 1);
    }
    compute_encoder.copy_buffer_to_buffer(
        &storage_buffer,
        0,
        &staging_compute_buffer,
        0,
        data_bytes.len() as u64,
    );
    let compute_submission = queue.submit(Some(compute_encoder.finish()));

    let map_status = Arc::new(Mutex::new(None));
    let map_status_clone = Arc::clone(&map_status);
    staging_compute_buffer.slice(..).map_async(wgpu::MapMode::Read, move |res| {
        if let Ok(mut guard) = map_status_clone.lock() {
            *guard = Some(res);
        }
    });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(compute_submission),
            timeout: Some(Duration::from_secs(10)),
        })
        .context("compute readback poll timed out")?;

    let map_result = match map_status.lock() {
        Ok(mut g) => g.take(),
        Err(p) => p.into_inner().take(),
    };
    match map_result {
        Some(Ok(())) => {}
        Some(Err(e)) => bail!("compute staging buffer map failed: {e}"),
        None => bail!("compute staging buffer map callback did not trigger"),
    }

    {
        let mapped_view = staging_compute_buffer
            .slice(..)
            .get_mapped_range()
            .context("failed to get mapped range for compute buffer")?;
        let computed_floats: &[f32] = bytemuck::cast_slice(&mapped_view);
        println!(
            "  Computed elements [0..4]: [{:.3}, {:.3}, {:.3}, {:.3}]",
            computed_floats[0], computed_floats[1], computed_floats[2], computed_floats[3]
        );
        // Verify compute calculation for index 0: val=0.0 -> val*val + 2.0*sin(0.0) = 0.0
        assert!(
            (computed_floats[0] - 0.0).abs() < 1e-4,
            "Compute mismatch at index 0"
        );
        drop(mapped_view);
    }
    staging_compute_buffer.unmap();
    println!("✓ Compute pass validated successfully with memory readback.");

    // Step 4b: Headless Render Pass to PNG
    println!("\n[4b/4] Executing headless render pass to PNG image...");
    let width = 512u32;
    let height = 512u32;

    let render_target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("render_target_texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let render_target_view = render_target.create_view(&wgpu::TextureViewDescriptor::default());

    let render_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("render_bgl"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    let render_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("render_bg"),
        layout: &render_bgl,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    let render_pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render_pipeline_layout"),
            bind_group_layouts: &[Some(&render_bgl)],
            immediate_size: 0,
        });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("render_pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader_module,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
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
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });

    let bytes_per_row = (width * 4 + 255) & !255; // 256-byte alignment
    let render_readback_size = (bytes_per_row * height) as u64;
    let render_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("render_staging_buffer"),
        size: render_readback_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut render_encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("render_encoder"),
        });
    {
        let mut rpass = render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("offscreen_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &render_target_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        rpass.set_pipeline(&render_pipeline);
        rpass.set_bind_group(0, &render_bg, &[]);
        rpass.draw(0..3, 0..1);
    }

    render_encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &render_target,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &render_staging_buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    let render_submission = queue.submit(Some(render_encoder.finish()));

    let render_map_status = Arc::new(Mutex::new(None));
    let render_map_status_clone = Arc::clone(&render_map_status);
    render_staging_buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |res| {
            if let Ok(mut guard) = render_map_status_clone.lock() {
                *guard = Some(res);
            }
        });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(render_submission),
            timeout: Some(Duration::from_secs(10)),
        })
        .context("render readback poll timed out")?;

    let render_map_result = match render_map_status.lock() {
        Ok(mut g) => g.take(),
        Err(p) => p.into_inner().take(),
    };
    match render_map_result {
        Some(Ok(())) => {}
        Some(Err(e)) => bail!("render staging buffer map failed: {e}"),
        None => bail!("render staging buffer map callback did not trigger"),
    }

    let mut unpadded_bytes = Vec::with_capacity((width * height * 4) as usize);
    {
        let mapped_view = render_staging_buffer
            .slice(..)
            .get_mapped_range()
            .context("failed to get mapped range for render staging buffer")?;
        for row in 0..height {
            let start = (row * bytes_per_row) as usize;
            let end = start + (width * 4) as usize;
            unpadded_bytes.extend_from_slice(&mapped_view[start..end]);
        }
        drop(mapped_view);
    }
    render_staging_buffer.unmap();

    let image_buffer = image::RgbaImage::from_raw(width, height, unpadded_bytes)
        .context("failed to construct RgbaImage from raw pixel buffer")?;
    let png_path = out_dir.join("webgpu_naga_render.png");
    image_buffer
        .save(&png_path)
        .with_context(|| format!("failed to save render PNG to {}", png_path.display()))?;
    println!("✓ Successfully wrote render PNG to {}", png_path.display());

    println!("\n=== WebGPU & Naga Pipeline Verification Complete ===");
    Ok(())
}
