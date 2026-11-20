use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result, bail, ensure};
use serde_json::json;

const SAMPLE_COMPUTE_WGSL: &str = r#"
@group(0) @binding(0)
var<storage, read_write> output_data: array<u32>;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if index < arrayLength(&output_data) {
        output_data[index] = (index * index) + 7u;
    }
}
"#;

fn main() -> Result<()> {
    println!("=== aphrody DirectX 12 Backend Probe ===");

    // 1. Initialize WGPU Instance with DX12 backend
    let mut instance_desc = wgpu::InstanceDescriptor::new_without_display_handle();
    instance_desc.backends = wgpu::Backends::DX12;
    let instance = wgpu::Instance::new(instance_desc);

    // 2. Enumerate & Request DirectX 12 Adapter
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
        apply_limit_buckets: false,
    }))
    .context("failed to discover a compatible DirectX 12 GPU adapter on Windows")?;

    let adapter_info = adapter.get_info();
    println!(
        "Selected GPU Adapter: {} (Driver: {}, Backend: {:?}, Type: {:?})",
        adapter_info.name, adapter_info.driver, adapter_info.backend, adapter_info.device_type
    );
    ensure!(
        adapter_info.backend == wgpu::Backend::Dx12,
        "expected DX12 backend, got {:?}",
        adapter_info.backend
    );

    // 3. Inspect DirectX 12 limits
    let limits = adapter.limits();
    let dx12_limits_report = json!({
        "adapter_name": adapter_info.name,
        "vendor_id": format!("0x{:04x}", adapter_info.vendor),
        "device_id": format!("0x{:04x}", adapter_info.device),
        "backend": format!("{:?}", adapter_info.backend),
        "max_non_sampler_bindings": limits.max_non_sampler_bindings,
        "max_storage_buffer_binding_size": limits.max_storage_buffer_binding_size,
        "max_compute_workgroup_size_x": limits.max_compute_workgroup_size_x,
        "max_compute_workgroup_storage_size": limits.max_compute_workgroup_storage_size,
        "max_compute_invocations_per_workgroup": limits.max_compute_invocations_per_workgroup,
        "max_bind_groups": limits.max_bind_groups,
    });
    println!(
        "DirectX 12 Hardware Limits:\n{}",
        serde_json::to_string_pretty(&dx12_limits_report)
            .context("failed to serialize DX12 limits report")?
    );

    // 4. Request D3D12 Logical Device & Queue
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("aphrody-dx12-probe-device"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        memory_hints: wgpu::MemoryHints::MemoryUsage,
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        trace: wgpu::Trace::Off,
        default_queue: wgpu::QueueDescriptor::default(),
    }))
    .context("failed to create DirectX 12 logical device and command queue")?;

    // 5. Naga WGSL -> HLSL SM 6.6 Transpilation Verification
    println!("\n--- Naga WGSL -> HLSL SM 6.6 Cross-Compilation ---");
    let naga_module = naga::front::wgsl::parse_str(SAMPLE_COMPUTE_WGSL)
        .context("Naga failed to parse compute WGSL source")?;

    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    let module_info = validator
        .validate(&naga_module)
        .context("Naga failed to validate compute module")?;

    let hlsl_options = naga::back::hlsl::Options {
        shader_model: naga::back::hlsl::ShaderModel::V6_6,
        fake_missing_bindings: true,
        ..naga::back::hlsl::Options::default()
    };
    let pipeline_options = naga::back::hlsl::PipelineOptions {
        entry_point: Some((naga::ShaderStage::Compute, "main".to_string())),
        multisampled: false,
    };

    let mut hlsl_out = String::new();
    let mut writer = naga::back::hlsl::Writer::new(&mut hlsl_out, &hlsl_options, &pipeline_options);
    writer
        .write(&naga_module, &module_info, None)
        .context("Naga HLSL writer failed to generate SM 6.6 HLSL")?;

    ensure!(
        !hlsl_out.is_empty(),
        "generated HLSL SM 6.6 output was unexpectedly empty"
    );
    ensure!(
        hlsl_out.contains("ByteAddressBuffer") || hlsl_out.contains("RWByteAddressBuffer"),
        "expected RWByteAddressBuffer in generated HLSL, got: {}",
        hlsl_out
    );
    println!("Successfully transpiled WGSL compute shader to HLSL SM 6.6 ({} bytes)", hlsl_out.len());

    // 6. Execute Headless Compute Pass on DirectX 12
    println!("\n--- Headless D3D12 Compute Execution & Verification ---");
    let element_count = 64usize;
    let buffer_size_bytes = (element_count * std::mem::size_of::<u32>()) as u64;

    let storage_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("dx12-probe-storage-buffer"),
        size: buffer_size_bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("dx12-probe-staging-buffer"),
        size: buffer_size_bytes,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("dx12-probe-shader"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(SAMPLE_COMPUTE_WGSL)),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("dx12-probe-bgl"),
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
        label: Some("dx12-probe-bg"),
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: storage_buffer.as_entire_binding(),
        }],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("dx12-probe-layout"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });

    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("dx12-probe-pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader_module,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("dx12-probe-encoder"),
    });
    {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("dx12-probe-pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        compute_pass.dispatch_workgroups(1, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&storage_buffer, 0, &staging_buffer, 0, buffer_size_bytes);

    let submission_index = queue.submit(std::iter::once(encoder.finish()));

    // 7. Read Back Output from D3D12 Staging Buffer
    let map_status: Arc<Mutex<Option<Result<(), wgpu::BufferAsyncError>>>> =
        Arc::new(Mutex::new(None));
    let callback_status = Arc::clone(&map_status);

    staging_buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            match callback_status.lock() {
                Ok(mut guard) => *guard = Some(result),
                Err(poisoned) => *poisoned.into_inner() = Some(result),
            }
        });

    device
        .poll(wgpu::PollType::Wait {
            submission_index,
            timeout: Some(Duration::from_secs(10)),
        })
        .context("timeout or failure while polling DirectX 12 queue submission")?;

    let callback_result = match map_status.lock() {
        Ok(mut guard) => guard.take(),
        Err(poisoned) => poisoned.into_inner().take(),
    };
    match callback_result {
        Some(Ok(())) => {}
        Some(Err(err)) => bail!("DirectX 12 staging buffer map error: {err}"),
        None => bail!("DirectX 12 buffer map callback did not complete"),
    }

    let mapped_range = staging_buffer
        .slice(..)
        .get_mapped_range()
        .context("failed to read mapped DirectX 12 buffer range")?;
    let output_values: Vec<u32> = mapped_range
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();
    drop(mapped_range);
    staging_buffer.unmap();

    // 8. Validate Computed Output Values
    ensure!(
        output_values.len() == element_count,
        "expected {} elements, got {}",
        element_count,
        output_values.len()
    );

    for (i, &actual) in output_values.iter().enumerate() {
        let expected = ((i * i) + 7) as u32;
        ensure!(
            actual == expected,
            "validation mismatch at index {i}: got {actual}, expected {expected}"
        );
    }

    println!(
        "Compute Validation Passed! Verified all {} output elements: [{}, {}, {}, {}, ... {}]",
        element_count,
        output_values[0],
        output_values[1],
        output_values[2],
        output_values[3],
        output_values[element_count - 1]
    );

    println!("\nDirectX 12 Backend Probe completed successfully.");
    Ok(())
}
