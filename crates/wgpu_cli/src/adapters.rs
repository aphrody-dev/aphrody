use anyhow::{Context as _, Result};
use clap::Args;
use serde::Serialize;

use crate::{BackendSelection, create_instance};

const INSPECTED_TEXTURE_FORMATS: &[wgpu::TextureFormat] = &[
    wgpu::TextureFormat::Rgba8Unorm,
    wgpu::TextureFormat::Rgba8UnormSrgb,
    wgpu::TextureFormat::Bgra8Unorm,
    wgpu::TextureFormat::Bgra8UnormSrgb,
    wgpu::TextureFormat::R8Unorm,
    wgpu::TextureFormat::Rgba16Float,
    wgpu::TextureFormat::Rgba32Float,
    wgpu::TextureFormat::Depth24PlusStencil8,
    wgpu::TextureFormat::Depth32Float,
];

#[derive(Debug, Args)]
pub(crate) struct AdaptersArgs {
    /// Backend filter (`all`, `vulkan`, `dx12`, `metal`, `gl`)
    #[arg(long, value_enum, default_value_t = BackendSelection::All)]
    pub backends: BackendSelection,

    /// Emit machine-readable JSON output
    #[arg(long)]
    pub json: bool,

    /// Include full adapter limits
    #[arg(long)]
    pub limits: bool,

    /// Include all supported WebGPU and native WGPU feature flags
    #[arg(long)]
    pub features: bool,
}

#[derive(Debug, Serialize)]
struct AdaptersReport {
    requested_backends: String,
    adapter_count: usize,
    adapters: Vec<AdapterEntry>,
}

#[derive(Debug, Serialize)]
struct AdapterEntry {
    index: usize,
    name: String,
    backend: String,
    device_type: String,
    vendor_id: u32,
    vendor_hex: String,
    device_id: u32,
    device_hex: String,
    pci_bus_id: String,
    driver: String,
    driver_info: String,
    subgroup_min_size: u32,
    subgroup_max_size: u32,
    transient_saves_memory: Option<bool>,
    downlevel: DownlevelReport,
    texture_formats: Vec<TextureFormatCapabilityReport>,
    feature_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    features: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limits: Option<LimitsReport>,
}

#[derive(Debug, Serialize)]
struct DownlevelReport {
    shader_model: String,
    is_webgpu_compliant: bool,
    flags: Vec<String>,
    missing_webgpu_flags: Vec<String>,
}

#[derive(Debug, Serialize)]
struct TextureFormatCapabilityReport {
    format: String,
    allowed_usages: Vec<String>,
    feature_flags: Vec<String>,
    supported_sample_counts: Vec<u32>,
}

#[derive(Debug, Serialize)]
struct LimitsReport {
    max_texture_dimension_1d: u32,
    max_texture_dimension_2d: u32,
    max_texture_dimension_3d: u32,
    max_texture_array_layers: u32,
    max_bind_groups: u32,
    max_bind_groups_plus_vertex_buffers: u32,
    max_bindings_per_bind_group: u32,
    max_dynamic_uniform_buffers_per_pipeline_layout: u32,
    max_dynamic_storage_buffers_per_pipeline_layout: u32,
    max_sampled_textures_per_shader_stage: u32,
    max_samplers_per_shader_stage: u32,
    max_storage_buffers_per_shader_stage: u32,
    max_storage_buffers_in_vertex_stage: u32,
    max_storage_buffers_in_fragment_stage: u32,
    max_storage_textures_per_shader_stage: u32,
    max_storage_textures_in_vertex_stage: u32,
    max_storage_textures_in_fragment_stage: u32,
    max_uniform_buffers_per_shader_stage: u32,
    max_binding_array_elements_per_shader_stage: u32,
    max_binding_array_acceleration_structure_elements_per_shader_stage: u32,
    max_binding_array_sampler_elements_per_shader_stage: u32,
    max_uniform_buffer_binding_size: u64,
    max_storage_buffer_binding_size: u64,
    max_vertex_buffers: u32,
    max_buffer_size: u64,
    max_vertex_attributes: u32,
    max_vertex_buffer_array_stride: u32,
    max_inter_stage_shader_variables: u32,
    min_uniform_buffer_offset_alignment: u32,
    min_storage_buffer_offset_alignment: u32,
    max_color_attachments: u32,
    max_color_attachment_bytes_per_sample: u32,
    max_compute_workgroup_storage_size: u32,
    max_compute_invocations_per_workgroup: u32,
    max_compute_workgroup_size_x: u32,
    max_compute_workgroup_size_y: u32,
    max_compute_workgroup_size_z: u32,
    max_compute_workgroups_per_dimension: u32,
    max_immediate_size: u32,
    max_non_sampler_bindings: u32,
    max_multiview_view_count: u32,
}

impl From<&wgpu::Limits> for LimitsReport {
    fn from(limits: &wgpu::Limits) -> Self {
        Self {
            max_texture_dimension_1d: limits.max_texture_dimension_1d,
            max_texture_dimension_2d: limits.max_texture_dimension_2d,
            max_texture_dimension_3d: limits.max_texture_dimension_3d,
            max_texture_array_layers: limits.max_texture_array_layers,
            max_bind_groups: limits.max_bind_groups,
            max_bind_groups_plus_vertex_buffers: limits.max_bind_groups_plus_vertex_buffers,
            max_bindings_per_bind_group: limits.max_bindings_per_bind_group,
            max_dynamic_uniform_buffers_per_pipeline_layout: limits
                .max_dynamic_uniform_buffers_per_pipeline_layout,
            max_dynamic_storage_buffers_per_pipeline_layout: limits
                .max_dynamic_storage_buffers_per_pipeline_layout,
            max_sampled_textures_per_shader_stage: limits.max_sampled_textures_per_shader_stage,
            max_samplers_per_shader_stage: limits.max_samplers_per_shader_stage,
            max_storage_buffers_per_shader_stage: limits.max_storage_buffers_per_shader_stage,
            max_storage_buffers_in_vertex_stage: limits.max_storage_buffers_in_vertex_stage,
            max_storage_buffers_in_fragment_stage: limits.max_storage_buffers_in_fragment_stage,
            max_storage_textures_per_shader_stage: limits.max_storage_textures_per_shader_stage,
            max_storage_textures_in_vertex_stage: limits.max_storage_textures_in_vertex_stage,
            max_storage_textures_in_fragment_stage: limits.max_storage_textures_in_fragment_stage,
            max_uniform_buffers_per_shader_stage: limits.max_uniform_buffers_per_shader_stage,
            max_binding_array_elements_per_shader_stage: limits
                .max_binding_array_elements_per_shader_stage,
            max_binding_array_acceleration_structure_elements_per_shader_stage: limits
                .max_binding_array_acceleration_structure_elements_per_shader_stage,
            max_binding_array_sampler_elements_per_shader_stage: limits
                .max_binding_array_sampler_elements_per_shader_stage,
            max_uniform_buffer_binding_size: limits.max_uniform_buffer_binding_size,
            max_storage_buffer_binding_size: limits.max_storage_buffer_binding_size,
            max_vertex_buffers: limits.max_vertex_buffers,
            max_buffer_size: limits.max_buffer_size,
            max_vertex_attributes: limits.max_vertex_attributes,
            max_vertex_buffer_array_stride: limits.max_vertex_buffer_array_stride,
            max_inter_stage_shader_variables: limits.max_inter_stage_shader_variables,
            min_uniform_buffer_offset_alignment: limits.min_uniform_buffer_offset_alignment,
            min_storage_buffer_offset_alignment: limits.min_storage_buffer_offset_alignment,
            max_color_attachments: limits.max_color_attachments,
            max_color_attachment_bytes_per_sample: limits.max_color_attachment_bytes_per_sample,
            max_compute_workgroup_storage_size: limits.max_compute_workgroup_storage_size,
            max_compute_invocations_per_workgroup: limits.max_compute_invocations_per_workgroup,
            max_compute_workgroup_size_x: limits.max_compute_workgroup_size_x,
            max_compute_workgroup_size_y: limits.max_compute_workgroup_size_y,
            max_compute_workgroup_size_z: limits.max_compute_workgroup_size_z,
            max_compute_workgroups_per_dimension: limits.max_compute_workgroups_per_dimension,
            max_immediate_size: limits.max_immediate_size,
            max_non_sampler_bindings: limits.max_non_sampler_bindings,
            max_multiview_view_count: limits.max_multiview_view_count,
        }
    }
}

pub(crate) fn run(args: AdaptersArgs) -> Result<()> {
    let requested_backends = args.backends.to_wgpu_backends();
    let instance = create_instance(requested_backends);
    let adapters = pollster::block_on(instance.enumerate_adapters(requested_backends));

    let mut entries = Vec::with_capacity(adapters.len());
    for (index, adapter) in adapters.iter().enumerate() {
        let info = adapter.get_info();
        let supported_features = adapter.features();
        let adapter_limits = adapter.limits();
        let downlevel = adapter.get_downlevel_capabilities();

        let feature_names: Vec<String> = supported_features
            .iter_names()
            .map(|(name, _)| name.to_owned())
            .collect();
        let downlevel_flags: Vec<String> = downlevel
            .flags
            .iter_names()
            .map(|(name, _)| name.to_owned())
            .collect();
        let missing_webgpu_flags: Vec<String> = (wgpu::DownlevelFlags::compliant()
            .difference(downlevel.flags))
        .iter_names()
        .map(|(name, _)| name.to_owned())
        .collect();

        let texture_formats = INSPECTED_TEXTURE_FORMATS
            .iter()
            .map(|&format| {
                let format_features = adapter.get_texture_format_features(format);
                let allowed_usages = format_features
                    .allowed_usages
                    .iter_names()
                    .map(|(name, _)| name.to_owned())
                    .collect();
                let feature_flags = format_features
                    .flags
                    .iter_names()
                    .map(|(name, _)| name.to_owned())
                    .collect();
                let supported_sample_counts = format_features.flags.supported_sample_counts();
                TextureFormatCapabilityReport {
                    format: format!("{format:?}"),
                    allowed_usages,
                    feature_flags,
                    supported_sample_counts,
                }
            })
            .collect();

        entries.push(AdapterEntry {
            index,
            name: info.name,
            backend: format!("{:?}", info.backend),
            device_type: format!("{:?}", info.device_type),
            vendor_id: info.vendor,
            vendor_hex: format!("0x{:04x}", info.vendor),
            device_id: info.device,
            device_hex: format!("0x{:04x}", info.device),
            pci_bus_id: info.device_pci_bus_id,
            driver: info.driver,
            driver_info: info.driver_info,
            subgroup_min_size: info.subgroup_min_size,
            subgroup_max_size: info.subgroup_max_size,
            transient_saves_memory: info.transient_saves_memory,
            downlevel: DownlevelReport {
                shader_model: format!("{:?}", downlevel.shader_model),
                is_webgpu_compliant: downlevel.is_webgpu_compliant(),
                flags: downlevel_flags,
                missing_webgpu_flags,
            },
            texture_formats,
            feature_count: feature_names.len(),
            features: if args.features || args.json {
                Some(feature_names)
            } else {
                None
            },
            limits: if args.limits || args.json {
                Some(LimitsReport::from(&adapter_limits))
            } else {
                None
            },
        });
    }

    let report = AdaptersReport {
        requested_backends: format!("{requested_backends:?}"),
        adapter_count: entries.len(),
        adapters: entries,
    };

    if args.json {
        let json =
            serde_json::to_string_pretty(&report).context("failed to serialize adapter report")?;
        println!("{json}");
        return Ok(());
    }

    println!(
        "Enumerated {} GPU adapter(s) (backends: {})",
        report.adapter_count, report.requested_backends
    );
    for entry in &report.adapters {
        println!();
        println!(
            "[{}] {} ({}, {})",
            entry.index, entry.name, entry.backend, entry.device_type
        );
        println!(
            "    PCI IDs:       vendor={} ({}) device={} ({}) bus='{}'",
            entry.vendor_hex,
            entry.vendor_id,
            entry.device_hex,
            entry.device_id,
            if entry.pci_bus_id.is_empty() {
                "-"
            } else {
                entry.pci_bus_id.as_str()
            }
        );
        println!(
            "    Driver:        {} {}",
            if entry.driver.is_empty() {
                "-"
            } else {
                entry.driver.as_str()
            },
            entry.driver_info
        );
        println!(
            "    Subgroups:     {}..={} | WebGPU compliant: {} (shader model: {})",
            entry.subgroup_min_size,
            entry.subgroup_max_size,
            entry.downlevel.is_webgpu_compliant,
            entry.downlevel.shader_model
        );
        if !entry.downlevel.missing_webgpu_flags.is_empty() {
            println!(
                "    Missing downlevel flags: {}",
                entry.downlevel.missing_webgpu_flags.join(", ")
            );
        }
        println!("    Downlevel flags ({}):", entry.downlevel.flags.len());
        println!("      {}", entry.downlevel.flags.join(", "));
        println!("    Key texture formats:");
        for format_cap in &entry.texture_formats {
            println!(
                "      {:22} usages=[{}] flags=[{}] msaa={:?}",
                format_cap.format,
                format_cap.allowed_usages.join("|"),
                format_cap.feature_flags.join("|"),
                format_cap.supported_sample_counts
            );
        }

        if args.features {
            if let Some(features) = &entry.features {
                println!("    Features ({}):", features.len());
                if features.is_empty() {
                    println!("      (none)");
                } else {
                    for chunk in features.chunks(4) {
                        println!("      {}", chunk.join(", "));
                    }
                }
            }
        } else {
            println!(
                "    Features:      {} supported (pass --features to list all)",
                entry.feature_count
            );
        }

        if args.limits {
            if let Some(limits) = &entry.limits {
                println!("    Limits:");
                println!(
                    "      max_texture_dimension_1d/2d/3d: {} / {} / {} (layers: {})",
                    limits.max_texture_dimension_1d,
                    limits.max_texture_dimension_2d,
                    limits.max_texture_dimension_3d,
                    limits.max_texture_array_layers
                );
                println!(
                    "      max_buffer_size: {} B | max_uniform_binding: {} B | max_storage_binding: {} B",
                    limits.max_buffer_size,
                    limits.max_uniform_buffer_binding_size,
                    limits.max_storage_buffer_binding_size
                );
                println!(
                    "      min_uniform_align: {} B | min_storage_align: {} B | max_immediate_size: {} B",
                    limits.min_uniform_buffer_offset_alignment,
                    limits.min_storage_buffer_offset_alignment,
                    limits.max_immediate_size
                );
                println!(
                    "      max_bind_groups: {} | max_bindings_per_group: {} | max_color_attachments: {}",
                    limits.max_bind_groups,
                    limits.max_bindings_per_bind_group,
                    limits.max_color_attachments
                );
                println!(
                    "      compute workgroup max: ({}, {}, {}) invocations={} shared_mem={} B dispatch_per_dim={}",
                    limits.max_compute_workgroup_size_x,
                    limits.max_compute_workgroup_size_y,
                    limits.max_compute_workgroup_size_z,
                    limits.max_compute_invocations_per_workgroup,
                    limits.max_compute_workgroup_storage_size,
                    limits.max_compute_workgroups_per_dimension
                );
            }
        }
    }

    Ok(())
}
