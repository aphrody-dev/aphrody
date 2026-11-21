// ihv_vendor_matrix.rs
// Multi-vendor IHV (NVIDIA, AMD, Intel, Microsoft, Mesa) hardware matrix probe and compute validation.
// Evaluates adapter features, subgroup sizes, vendor-tuned workgroups, and runs a WGPU compute pass.
// Strictly compliant with .rules: no mod.rs, zero .unwrap(), zero let _ = <fallible>.

use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context, Result, bail, ensure};
use serde_json::json;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IhvVendor {
    Nvidia,
    Amd,
    Intel,
    Microsoft,
    Mesa,
    Other(u32),
}

impl IhvVendor {
    fn from_pci_id(vendor_id: u32) -> Self {
        match vendor_id {
            0x10DE => Self::Nvidia,
            0x1002 | 0x1022 => Self::Amd,
            0x8086 => Self::Intel,
            0x1414 => Self::Microsoft,
            0x10005 => Self::Mesa,
            other => Self::Other(other),
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Nvidia => "NVIDIA Corporation",
            Self::Amd => "Advanced Micro Devices, Inc. (AMD)",
            Self::Intel => "Intel Corporation",
            Self::Microsoft => "Microsoft Corporation (WARP)",
            Self::Mesa => "Mesa (Software / lavapipe)",
            Self::Other(_) => "Unknown / Third-Party Vendor",
        }
    }

    fn optimal_workgroup_size(&self) -> (u32, u32, u32) {
        match self {
            // NVIDIA Ada/Turing/Ampere warp size is 32 threads
            Self::Nvidia => (32, 1, 1),
            // AMD RDNA Wave32/Wave64 optimal multiple is 64 threads
            Self::Amd => (64, 1, 1),
            // Intel Xe EU / Xe-Core optimal multiple is 64 threads (accommodates SIMD8/16/32)
            Self::Intel => (64, 1, 1),
            // Software fallbacks (WARP, lavapipe) default to 64 threads
            Self::Microsoft | Self::Mesa | Self::Other(_) => (64, 1, 1),
        }
    }
}

const COMPUTE_TEST_WGSL: &str = r#"
@group(0) @binding(0)
var<storage, read> input_data: array<u32>;

@group(0) @binding(1)
var<storage, read_write> output_data: array<u32>;

@compute @workgroup_size(32, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if index < arrayLength(&input_data) {
        output_data[index] = (input_data[index] * 3u) + 7u;
    }
}
"#;

fn main() -> Result<()> {
    println!("=== aphrody Multi-Vendor (NVIDIA / AMD / Intel) Hardware Matrix ===");

    // 1. Enumerate all adapters across all supported WGPU backends
    let instance_desc = wgpu::InstanceDescriptor::new_without_display_handle();
    let instance = wgpu::Instance::new(instance_desc);

    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()));
    ensure!(
        !adapters.is_empty(),
        "no graphics or compute adapters discovered by WGPU"
    );

    println!("Discovered {} total adapter(s):", adapters.len());

    let mut matrix_reports = Vec::new();
    let mut primary_adapter = None;

    for (index, adapter) in adapters.into_iter().enumerate() {
        let info = adapter.get_info();
        let vendor = IhvVendor::from_pci_id(info.vendor);
        let limits = adapter.limits();
        let features = adapter.features();
        let (wg_x, wg_y, wg_z) = vendor.optimal_workgroup_size();

        println!(
            "  [{}] {} | Backend: {:?} | DeviceType: {:?} | Vendor: {} (0x{:04x})",
            index, info.name, info.backend, info.device_type, vendor.name(), info.vendor
        );

        let report = json!({
            "index": index,
            "adapter_name": info.name,
            "backend": format!("{:?}", info.backend),
            "device_type": format!("{:?}", info.device_type),
            "vendor_pci_id": format!("0x{:04x}", info.vendor),
            "vendor_name": vendor.name(),
            "device_pci_id": format!("0x{:04x}", info.device),
            "driver_name": info.driver,
            "driver_info": info.driver_info,
            "optimal_workgroup_size": [wg_x, wg_y, wg_z],
            "supports_subgroups": features.contains(wgpu::Features::SUBGROUP),
            "supports_subgroup_barrier": features.contains(wgpu::Features::SUBGROUP_BARRIER),
            "supports_subgroup_vertex": features.contains(wgpu::Features::SUBGROUP_VERTEX),
            "supports_shader_f16": features.contains(wgpu::Features::SHADER_F16),
            "supports_timestamp_query": features.contains(wgpu::Features::TIMESTAMP_QUERY),
            "max_compute_invocations_per_workgroup": limits.max_compute_invocations_per_workgroup,
            "max_compute_workgroup_size_x": limits.max_compute_workgroup_size_x,
            "max_storage_buffer_binding_size": limits.max_storage_buffer_binding_size,
            "min_uniform_buffer_offset_alignment": limits.min_uniform_buffer_offset_alignment,
            "min_storage_buffer_offset_alignment": limits.min_storage_buffer_offset_alignment,
        });

        matrix_reports.push(report);

        if primary_adapter.is_none() && info.device_type == wgpu::DeviceType::DiscreteGpu {
            primary_adapter = Some(adapter);
        }
    }

    // Fall back to first available adapter if no discrete GPU was flagged
    let adapter = match primary_adapter {
        Some(a) => a,
        None => {
            let fallback_adapters =
                pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()));
            match fallback_adapters.into_iter().next() {
                Some(a) => a,
                None => bail!("failed to select any adapter for compute verification"),
            }
        }
    };

    let selected_info = adapter.get_info();
    let selected_vendor = IhvVendor::from_pci_id(selected_info.vendor);
    println!(
        "\nSelected Adapter for Compute Verification: {} (Vendor: {})",
        selected_info.name,
        selected_vendor.name()
    );

    // 2. Request Device and Queue for Compute Verification
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("ihv_matrix_compute_device"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        default_queue: wgpu::QueueDescriptor::default(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::Performance,
        trace: wgpu::Trace::Off,
    }))
    .context("failed to create WGPU device on selected IHV adapter")?;

    // 3. Compile WGSL Compute Shader
    let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("ihv_matrix_compute_module"),
        source: wgpu::ShaderSource::Wgsl(COMPUTE_TEST_WGSL.into()),
    });

    // 4. Create Buffers (Input, Output, Staging Readback)
    const ELEMENT_COUNT: usize = 128;
    let input_values: Vec<u32> = (0..ELEMENT_COUNT as u32).collect();
    let input_bytes: &[u8] = bytemuck::cast_slice(&input_values);
    let buffer_size = (ELEMENT_COUNT * std::mem::size_of::<u32>()) as u64;

    let input_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ihv_input_buffer"),
        size: buffer_size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&input_buffer, 0, input_bytes);

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ihv_output_buffer"),
        size: buffer_size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ihv_staging_readback_buffer"),
        size: buffer_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // 5. Create Pipeline and Bind Group
    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("ihv_compute_pipeline"),
        layout: None,
        module: &shader_module,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ihv_compute_bind_group"),
        layout: &compute_pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: input_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output_buffer.as_entire_binding(),
            },
        ],
    });

    // 6. Record and Submit Compute Pass
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("ihv_compute_encoder"),
    });

    {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("ihv_compute_pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        let workgroups = (ELEMENT_COUNT as u32).div_ceil(32);
        compute_pass.dispatch_workgroups(workgroups, 1, 1);
    }

    encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, buffer_size);
    queue.submit(Some(encoder.finish()));

    // 7. Map Staging Buffer and Read Back Results
    let (tx, rx) = mpsc::channel();
    let buffer_slice = staging_buffer.slice(..);
    buffer_slice.map_async(wgpu::MapMode::Read, move |res| {
        if let Err(err) = tx.send(res) {
            eprintln!("failed to send staging buffer map result: {err:?}");
        }
    });

    let poll_result = device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(Duration::from_secs(5)),
    });
    ensure!(
        poll_result.is_ok(),
        "device polling timed out during buffer map readback"
    );

    let map_status = rx
        .recv_timeout(Duration::from_secs(5))
        .context("channel timed out waiting for staging buffer map completion")?;
    map_status.context("staging buffer map_async failed")?;

    {
        let view = buffer_slice
            .get_mapped_range()
            .context("failed to read mapped IHV readback buffer range")?;
        let readback_slice: &[u32] = bytemuck::cast_slice(&view);

        for (i, &output_val) in readback_slice.iter().enumerate() {
            let expected_val = (i as u32) * 3 + 7;
            ensure!(
                output_val == expected_val,
                "compute verification mismatch at index {}: got {}, expected {}",
                i,
                output_val,
                expected_val
            );
        }
    }
    staging_buffer.unmap();

    println!("Compute Verification: PASS (128 elements computed and verified)");

    // 8. Output JSON Matrix Report
    let final_report = json!({
        "status": "success",
        "total_adapters_enumerated": matrix_reports.len(),
        "selected_adapter": {
            "name": selected_info.name,
            "vendor": selected_vendor.name(),
            "pci_vendor_id": format!("0x{:04x}", selected_info.vendor),
            "backend": format!("{:?}", selected_info.backend),
        },
        "compute_verification": {
            "elements_computed": ELEMENT_COUNT,
            "formula": "input[i] * 3 + 7",
            "passed": true,
        },
        "adapter_matrix": matrix_reports,
    });

    let json_output = serde_json::to_string_pretty(&final_report)
        .context("failed to serialize IHV matrix JSON output")?;
    println!("\n=== IHV Matrix JSON Output ===\n{}", json_output);

    Ok(())
}
