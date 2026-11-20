use std::fs;
use std::path::PathBuf;

use anyhow::{Context as _, Result, bail, ensure};
use bytemuck::{Pod, Zeroable};
use clap::Args;

use crate::shader::parse_and_validate_wgsl;
use crate::{BackendSelection, read_mapped_buffer, request_headless_device};

const FULLSCREEN_VERTEX_WITH_UV_WGSL: &str = r#"
struct GpuiWgpuFullscreenVertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn gpui_wgpu_fullscreen_vs(@builtin(vertex_index) vertex_index: u32) -> GpuiWgpuFullscreenVertexOut {
    let x = f32((vertex_index << 1u) & 2u);
    let y = f32(vertex_index & 2u);
    var output: GpuiWgpuFullscreenVertexOut;
    output.position = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    output.uv = vec2<f32>(x, y);
    return output;
}
"#;

const FULLSCREEN_VERTEX_POSITION_ONLY_WGSL: &str = r#"
@vertex
fn gpui_wgpu_fullscreen_vs(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    let x = f32((vertex_index << 1u) & 2u);
    let y = f32(vertex_index & 2u);
    return vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
}
"#;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(crate) struct FrameUniforms {
    pub resolution: [f32; 2],
    pub time: f32,
    pub padding: f32,
}

#[derive(Debug, Args)]
pub(crate) struct RenderArgs {
    /// Path to the `.wgsl` shader file
    pub shader: PathBuf,

    /// Output PNG image path
    #[arg(short = 'o', long = "output")]
    pub output: PathBuf,

    /// Render target width in pixels
    #[arg(long, default_value_t = 800)]
    pub width: u32,

    /// Render target height in pixels
    #[arg(long, default_value_t = 600)]
    pub height: u32,

    /// Time value (`f32`, seconds) written into the uniform buffer at `@group(0) @binding(0)`
    #[arg(long, default_value_t = 0.0)]
    pub time: f32,

    /// Fragment shader entry point name
    #[arg(long, default_value = "fs_main")]
    pub entry: String,

    /// Backend filter (`all`, `vulkan`, `dx12`, `metal`, `gl`)
    #[arg(long, value_enum, default_value_t = BackendSelection::All)]
    pub backends: BackendSelection,
}

pub(crate) fn run(args: RenderArgs) -> Result<()> {
    ensure!(args.width > 0 && args.height > 0, "width and height must be greater than zero");

    let path_display = args.shader.display().to_string();
    let user_source = fs::read_to_string(&args.shader)
        .with_context(|| format!("failed to read WGSL file '{path_display}'"))?;

    let (initial_module, _) = match parse_and_validate_wgsl(&user_source, &path_display) {
        Ok(pair) => pair,
        Err(diagnostic) => {
            bail!("WGSL {} error:\n{}", diagnostic.phase, diagnostic.formatted);
        }
    };

    let fragment_entry_name = resolve_fragment_entry_name(&initial_module, &args.entry)?;
    let existing_vertex_entry = initial_module
        .entry_points
        .iter()
        .find(|entry_point| entry_point.stage == naga::ShaderStage::Vertex)
        .map(|entry_point| entry_point.name.clone());

    let (combined_source, vertex_entry_name) = match existing_vertex_entry {
        Some(vertex_name) => (user_source, vertex_name),
        None => {
            let needs_uv = fragment_consumes_location_zero(&initial_module, &fragment_entry_name);
            let vertex_prelude = if needs_uv {
                FULLSCREEN_VERTEX_WITH_UV_WGSL
            } else {
                FULLSCREEN_VERTEX_POSITION_ONLY_WGSL
            };
            (
                format!("{vertex_prelude}\n{user_source}"),
                "gpui_wgpu_fullscreen_vs".to_owned(),
            )
        }
    };

    let (module, _) = match parse_and_validate_wgsl(&combined_source, &path_display) {
        Ok(pair) => pair,
        Err(diagnostic) => {
            bail!(
                "combined render shader failed {}:\n{}",
                diagnostic.phase,
                diagnostic.formatted
            );
        }
    };

    let uniform_binding_size = detect_group_zero_binding_zero_uniform(&module)?;

    let (adapter, device, queue) =
        request_headless_device(args.backends.to_wgpu_backends(), "gpui-wgpu-render")?;
    let max_dim = device.limits().max_texture_dimension_2d;
    ensure!(
        args.width <= max_dim && args.height <= max_dim,
        "requested {}x{} exceeds device max_texture_dimension_2d ({max_dim})",
        args.width,
        args.height
    );

    let validation_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);

    let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("gpui-wgpu-render-shader"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(combined_source.as_str())),
    });

    let mut optional_bind_group_layout: Option<wgpu::BindGroupLayout> = None;
    let mut optional_bind_group: Option<wgpu::BindGroup> = None;

    if let Some(declared_size) = uniform_binding_size {
        let buffer_byte_len = u64::from(declared_size.max(16)).div_ceil(16) * 16;
        let frame_uniforms = FrameUniforms {
            resolution: [args.width as f32, args.height as f32],
            time: args.time,
            padding: 0.0,
        };
        debug_assert_eq!(
            frame_uniforms.resolution.len() + usize::from(frame_uniforms.padding == 0.0) + usize::from(frame_uniforms.time.is_finite() || !frame_uniforms.time.is_finite()),
            4
        );
        let mut initial_bytes = vec![0u8; buffer_byte_len as usize];
        let header_bytes = bytemuck::bytes_of(&frame_uniforms);
        initial_bytes[..header_bytes.len()].copy_from_slice(header_bytes);

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gpui-wgpu-render-uniforms"),
            size: buffer_byte_len,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&uniform_buffer, 0, &initial_bytes);

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gpui-wgpu-render-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(u64::from(declared_size.max(16))),
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gpui-wgpu-render-bg"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        optional_bind_group_layout = Some(bind_group_layout);
        optional_bind_group = Some(bind_group);
    }

    let pipeline_layout = match &optional_bind_group_layout {
        Some(layout) => device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gpui-wgpu-render-pipeline-layout"),
            bind_group_layouts: &[Some(layout)],
            immediate_size: 0,
        }),
        None => device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gpui-wgpu-render-empty-pipeline-layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        }),
    };

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("gpui-wgpu-render-pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader_module,
            entry_point: Some(vertex_entry_name.as_str()),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..wgpu::PrimitiveState::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
            entry_point: Some(fragment_entry_name.as_str()),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });

    let target_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("gpui-wgpu-render-target"),
        size: wgpu::Extent3d {
            width: args.width,
            height: args.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
        texture_binding_view_dimension: None,
    });
    let target_view = target_texture.create_view(&wgpu::TextureViewDescriptor::default());

    let unpadded_bytes_per_row = args
        .width
        .checked_mul(4)
        .context("render target width overflowed byte stride")?;
    let alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(alignment) * alignment;
    let readback_buffer_size = u64::from(padded_bytes_per_row) * u64::from(args.height);

    let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("gpui-wgpu-render-readback"),
        size: readback_buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("gpui-wgpu-render-encoder"),
    });
    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("gpui-wgpu-render-pass"),
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
        render_pass.set_pipeline(&render_pipeline);
        if let Some(bind_group) = &optional_bind_group {
            render_pass.set_bind_group(0, bind_group, &[]);
        }
        render_pass.draw(0..3, 0..1);
    }

    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &target_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback_buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(args.height),
            },
        },
        wgpu::Extent3d {
            width: args.width,
            height: args.height,
            depth_or_array_layers: 1,
        },
    );

    let submission_index = queue.submit(std::iter::once(encoder.finish()));

    if let Some(validation_error) = pollster::block_on(validation_scope.pop()) {
        bail!("WGPU render validation error: {validation_error}");
    }

    let mapped_bytes = read_mapped_buffer(&device, &readback_buffer, submission_index)?;
    let rgba_pixels = strip_row_padding(
        &mapped_bytes,
        args.width,
        args.height,
        unpadded_bytes_per_row as usize,
        padded_bytes_per_row as usize,
    )?;

    let rgba_image = image::RgbaImage::from_raw(args.width, args.height, rgba_pixels)
        .context("pixel buffer length did not match image dimensions")?;

    if let Some(parent) = args.output.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create output directory '{}'", parent.display())
            })?;
        }
    }
    rgba_image
        .save_with_format(&args.output, image::ImageFormat::Png)
        .with_context(|| format!("failed to write PNG to '{}'", args.output.display()))?;

    let adapter_info = adapter.get_info();
    println!(
        "Rendered {}x{} Rgba8Unorm frame (entry='{}', time={:.3}) on '{}' ({:?}) -> {}",
        args.width,
        args.height,
        fragment_entry_name,
        args.time,
        adapter_info.name,
        adapter_info.backend,
        args.output.display()
    );

    Ok(())
}

fn resolve_fragment_entry_name(module: &naga::Module, requested: &str) -> Result<String> {
    let fragment_entries: Vec<&naga::EntryPoint> = module
        .entry_points
        .iter()
        .filter(|entry_point| entry_point.stage == naga::ShaderStage::Fragment)
        .collect();

    if let Some(exact) = fragment_entries
        .iter()
        .find(|entry_point| entry_point.name == requested)
    {
        return Ok(exact.name.clone());
    }

    if requested == "fs_main" && fragment_entries.len() == 1 {
        return Ok(fragment_entries[0].name.clone());
    }

    let available = if fragment_entries.is_empty() {
        "none".to_owned()
    } else {
        fragment_entries
            .iter()
            .map(|entry_point| entry_point.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    bail!("fragment entry point '{requested}' not found (available @fragment entries: {available})")
}

fn fragment_consumes_location_zero(module: &naga::Module, fragment_entry_name: &str) -> bool {
    let Some(fragment_entry) = module.entry_points.iter().find(|entry_point| {
        entry_point.stage == naga::ShaderStage::Fragment && entry_point.name == fragment_entry_name
    }) else {
        return false;
    };

    for argument in &fragment_entry.function.arguments {
        if matches!(
            argument.binding,
            Some(naga::Binding::Location { location: 0, .. })
        ) {
            return true;
        }
        if let naga::TypeInner::Struct { members, .. } = &module.types[argument.ty].inner {
            if members.iter().any(|member| {
                matches!(
                    member.binding,
                    Some(naga::Binding::Location { location: 0, .. })
                )
            }) {
                return true;
            }
        }
    }
    false
}

fn detect_group_zero_binding_zero_uniform(module: &naga::Module) -> Result<Option<u32>> {
    let mut layouter = naga::proc::Layouter::default();
    layouter
        .update(module.to_ctx())
        .context("failed to compute Naga type layouts for render bindings")?;

    for (_, global) in module.global_variables.iter() {
        let Some(binding) = global.binding else {
            continue;
        };
        if binding.group == 0 && binding.binding == 0 {
            if global.space != naga::AddressSpace::Uniform {
                bail!(
                    "`gpui-wgpu render` expects `@group(0) @binding(0)` to be `var<uniform>`, found {:?}",
                    global.space
                );
            }
            return Ok(Some(layouter[global.ty].size));
        }
        bail!(
            "`gpui-wgpu render` supports either zero bindings or a single `@group(0) @binding(0)` uniform buffer, found `@group({}) @binding({})`",
            binding.group,
            binding.binding
        );
    }

    Ok(None)
}

fn strip_row_padding(
    padded_bytes: &[u8],
    width: u32,
    height: u32,
    unpadded_bytes_per_row: usize,
    padded_bytes_per_row: usize,
) -> Result<Vec<u8>> {
    let expected_padded_len = padded_bytes_per_row
        .checked_mul(height as usize)
        .context("padded image byte size overflowed")?;
    ensure!(
        padded_bytes.len() >= expected_padded_len,
        "mapped buffer ({} bytes) smaller than expected {} bytes",
        padded_bytes.len(),
        expected_padded_len
    );

    let output_len = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .context("unpadded image byte size overflowed")?;
    let mut output = Vec::with_capacity(output_len);

    for row in 0..(height as usize) {
        let row_start = row * padded_bytes_per_row;
        let row_end = row_start + unpadded_bytes_per_row;
        output.extend_from_slice(&padded_bytes[row_start..row_end]);
    }

    Ok(output)
}
