use std::fs;
use std::path::PathBuf;

use anyhow::{Context as _, Result, bail, ensure};
use clap::Args;
use serde::Serialize;

use crate::shader::parse_and_validate_wgsl;
use crate::{BackendSelection, read_mapped_buffer, request_headless_device};

#[derive(Debug, Args)]
pub(crate) struct ComputeArgs {
    /// Path to the `.wgsl` compute shader file
    pub shader: PathBuf,

    /// Compute shader entry point name
    #[arg(long, default_value = "main")]
    pub entry: String,

    /// Workgroup dispatch counts formatted as `X,Y,Z` (e.g. `1,1,1` or `16,1,1`)
    #[arg(long, default_value = "1,1,1")]
    pub workgroups: String,

    /// Number of `f32` elements (4 bytes each) in the `@group(0) @binding(0)` storage buffer
    #[arg(long, default_value_t = 64)]
    pub output_floats: usize,

    /// Emit machine-readable JSON output
    #[arg(long)]
    pub json: bool,

    /// Backend filter (`all`, `vulkan`, `dx12`, `metal`, `gl`)
    #[arg(long, value_enum, default_value_t = BackendSelection::All)]
    pub backends: BackendSelection,
}

#[derive(Debug, Serialize)]
struct ComputeReport {
    shader: String,
    entry_point: String,
    workgroup_size: [u32; 3],
    dispatched_workgroups: [u32; 3],
    adapter_name: String,
    backend: String,
    output_floats: usize,
    output_bytes: usize,
    values_f32: Vec<f32>,
    values_u32: Vec<u32>,
}

pub(crate) fn run(args: ComputeArgs) -> Result<()> {
    ensure!(args.output_floats > 0, "--output-floats must be greater than zero");
    let dispatch_counts = parse_workgroups(&args.workgroups)?;

    let path_display = args.shader.display().to_string();
    let source = fs::read_to_string(&args.shader)
        .with_context(|| format!("failed to read WGSL file '{path_display}'"))?;

    let (module, _) = match parse_and_validate_wgsl(&source, &path_display) {
        Ok(pair) => pair,
        Err(diagnostic) => {
            bail!("WGSL {} error:\n{}", diagnostic.phase, diagnostic.formatted);
        }
    };

    let (entry_name, workgroup_size) = resolve_compute_entry(&module, &args.entry)?;
    let storage_binding_read_only = detect_compute_storage_binding(&module)?;

    let (adapter, device, queue) =
        request_headless_device(args.backends.to_wgpu_backends(), "gpui-wgpu-compute")?;

    let max_per_dim = device.limits().max_compute_workgroups_per_dimension;
    for (axis_name, count) in [
        ("x", dispatch_counts[0]),
        ("y", dispatch_counts[1]),
        ("z", dispatch_counts[2]),
    ] {
        ensure!(
            count > 0 && count <= max_per_dim,
            "workgroup count {axis_name}={count} must be in 1..={max_per_dim}"
        );
    }

    let buffer_size_bytes = (args.output_floats as u64)
        .checked_mul(4)
        .context("output buffer byte size overflowed")?;
    ensure!(
        buffer_size_bytes <= device.limits().max_storage_buffer_binding_size,
        "requested output buffer ({buffer_size_bytes} B) exceeds max_storage_buffer_binding_size ({} B)",
        device.limits().max_storage_buffer_binding_size
    );

    let validation_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);

    let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("gpui-wgpu-compute-shader"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(source.as_str())),
    });

    let storage_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("gpui-wgpu-compute-storage"),
        size: buffer_size_bytes,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("gpui-wgpu-compute-staging"),
        size: buffer_size_bytes,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut optional_bind_group_layout: Option<wgpu::BindGroupLayout> = None;
    let mut optional_bind_group: Option<wgpu::BindGroup> = None;

    if let Some(read_only) = storage_binding_read_only {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gpui-wgpu-compute-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gpui-wgpu-compute-bg"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: storage_buffer.as_entire_binding(),
            }],
        });

        optional_bind_group_layout = Some(bind_group_layout);
        optional_bind_group = Some(bind_group);
    }

    let pipeline_layout = match &optional_bind_group_layout {
        Some(layout) => device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gpui-wgpu-compute-pipeline-layout"),
            bind_group_layouts: &[Some(layout)],
            immediate_size: 0,
        }),
        None => device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gpui-wgpu-compute-empty-pipeline-layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        }),
    };

    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("gpui-wgpu-compute-pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader_module,
        entry_point: Some(entry_name.as_str()),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("gpui-wgpu-compute-encoder"),
    });
    {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("gpui-wgpu-compute-pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&compute_pipeline);
        if let Some(bind_group) = &optional_bind_group {
            compute_pass.set_bind_group(0, bind_group, &[]);
        }
        compute_pass.dispatch_workgroups(dispatch_counts[0], dispatch_counts[1], dispatch_counts[2]);
    }
    encoder.copy_buffer_to_buffer(&storage_buffer, 0, &staging_buffer, 0, buffer_size_bytes);

    let submission_index = queue.submit(std::iter::once(encoder.finish()));

    if let Some(validation_error) = pollster::block_on(validation_scope.pop()) {
        bail!("WGPU compute validation error: {validation_error}");
    }

    let raw_bytes = read_mapped_buffer(&device, &staging_buffer, submission_index)?;
    let values_f32: Vec<f32> = raw_bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();
    let values_u32: Vec<u32> = raw_bytes
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();

    let adapter_info = adapter.get_info();
    let report = ComputeReport {
        shader: path_display,
        entry_point: entry_name,
        workgroup_size,
        dispatched_workgroups: dispatch_counts,
        adapter_name: adapter_info.name,
        backend: format!("{:?}", adapter_info.backend),
        output_floats: values_f32.len(),
        output_bytes: raw_bytes.len(),
        values_f32,
        values_u32,
    };

    if args.json {
        let json =
            serde_json::to_string_pretty(&report).context("failed to serialize compute output")?;
        println!("{json}");
        return Ok(());
    }

    println!(
        "Dispatched '{}' @workgroup_size({}, {}, {}) x ({}, {}, {}) on '{}' ({})",
        report.entry_point,
        report.workgroup_size[0],
        report.workgroup_size[1],
        report.workgroup_size[2],
        report.dispatched_workgroups[0],
        report.dispatched_workgroups[1],
        report.dispatched_workgroups[2],
        report.adapter_name,
        report.backend
    );
    println!(
        "Output buffer (@group(0) @binding(0), {} floats / {} bytes):",
        report.output_floats, report.output_bytes
    );
    for (row_index, chunk) in report.values_f32.chunks(8).enumerate() {
        let formatted: Vec<String> = chunk.iter().map(|val| format!("{val:10.4}")).collect();
        println!("  [{:04}] {}", row_index * 8, formatted.join(" "));
    }

    Ok(())
}

pub(crate) fn parse_workgroups(spec: &str) -> Result<[u32; 3]> {
    let parts: Vec<&str> = spec.split(',').map(str::trim).collect();
    let [x_str, y_str, z_str] = parts.as_slice() else {
        bail!("invalid `--workgroups '{spec}'`: expected three comma-separated integers `X,Y,Z`");
    };
    let x = x_str
        .parse::<u32>()
        .with_context(|| format!("invalid X workgroup count '{x_str}'"))?;
    let y = y_str
        .parse::<u32>()
        .with_context(|| format!("invalid Y workgroup count '{y_str}'"))?;
    let z = z_str
        .parse::<u32>()
        .with_context(|| format!("invalid Z workgroup count '{z_str}'"))?;
    ensure!(
        x > 0 && y > 0 && z > 0,
        "workgroup counts must all be >= 1, got ({x}, {y}, {z})"
    );
    Ok([x, y, z])
}

fn resolve_compute_entry(module: &naga::Module, requested: &str) -> Result<(String, [u32; 3])> {
    let compute_entries: Vec<&naga::EntryPoint> = module
        .entry_points
        .iter()
        .filter(|entry_point| entry_point.stage == naga::ShaderStage::Compute)
        .collect();

    if let Some(exact) = compute_entries
        .iter()
        .find(|entry_point| entry_point.name == requested)
    {
        return Ok((exact.name.clone(), exact.workgroup_size));
    }

    if requested == "main" && compute_entries.len() == 1 {
        let only = compute_entries[0];
        return Ok((only.name.clone(), only.workgroup_size));
    }

    let available = if compute_entries.is_empty() {
        "none".to_owned()
    } else {
        compute_entries
            .iter()
            .map(|entry_point| entry_point.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    bail!("compute entry point '{requested}' not found (available @compute entries: {available})")
}

fn detect_compute_storage_binding(module: &naga::Module) -> Result<Option<bool>> {
    for (_, global) in module.global_variables.iter() {
        let Some(binding) = global.binding else {
            continue;
        };
        if binding.group == 0 && binding.binding == 0 {
            match global.space {
                naga::AddressSpace::Storage { access } => {
                    let read_only = !access.contains(naga::StorageAccess::STORE);
                    return Ok(Some(read_only));
                }
                other => {
                    bail!(
                        "`gpui-wgpu compute` expects `@group(0) @binding(0)` to be `var<storage, ...>`, found {other:?}"
                    );
                }
            }
        }
        bail!(
            "`gpui-wgpu compute` supports either zero bindings or a single `@group(0) @binding(0)` storage buffer, found `@group({}) @binding({})`",
            binding.group,
            binding.binding
        );
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_workgroup_triplets() -> Result<()> {
        assert_eq!(parse_workgroups("1,1,1")?, [1, 1, 1]);
        assert_eq!(parse_workgroups(" 16 , 4 , 2 ")?, [16, 4, 2]);
        assert!(parse_workgroups("0,1,1").is_err());
        assert!(parse_workgroups("4,4").is_err());
        Ok(())
    }
}
