use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use clap::{Args, ValueEnum};
use serde::Serialize;

#[derive(Debug, Args)]
pub(crate) struct ValidateArgs {
    /// Path to the input `.wgsl` shader file
    pub shader: PathBuf,

    /// Emit machine-readable JSON reflection output
    #[arg(long)]
    pub json: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum TranspileTarget {
    Wgsl,
    Glsl,
    #[value(name = "webgl2", alias = "glsl-es", alias = "gles")]
    Webgl2,
    Hlsl,
    Msl,
    Spv,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum ShaderStageSelection {
    Vertex,
    Fragment,
    Compute,
}

impl ShaderStageSelection {
    pub(crate) fn to_naga_stage(self) -> naga::ShaderStage {
        match self {
            Self::Vertex => naga::ShaderStage::Vertex,
            Self::Fragment => naga::ShaderStage::Fragment,
            Self::Compute => naga::ShaderStage::Compute,
        }
    }
}

#[derive(Debug, Args)]
pub(crate) struct TranspileArgs {
    /// Path to the input `.wgsl` shader file
    pub shader: PathBuf,

    /// Target shading language (`wgsl`, `glsl`, `webgl2`, `hlsl`, `msl`, `spv`)
    #[arg(long, value_enum)]
    pub target: TranspileTarget,

    /// Optional output file path (defaults to stdout)
    #[arg(short = 'o', long = "output")]
    pub output: Option<PathBuf>,

    /// Shader stage filter (`vertex`, `fragment`, `compute`)
    #[arg(long, value_enum)]
    pub stage: Option<ShaderStageSelection>,

    /// Entry point function name
    #[arg(long)]
    pub entry: Option<String>,

    /// Emit WebGL2 GLSL ES 300 when `--target glsl` is selected
    #[arg(long)]
    pub webgl2: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct ShaderValidationReport {
    pub valid: bool,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<DiagnosticReport>,
    pub entry_points: Vec<EntryPointReport>,
    pub resource_bindings: Vec<ResourceBindingReport>,
    pub overrides: Vec<OverrideReport>,
    pub immediates: Vec<ImmediateReport>,
    pub workgroup_variables: Vec<WorkgroupVariableReport>,
}

#[derive(Debug, Serialize)]
pub(crate) struct DiagnosticReport {
    pub phase: &'static str,
    pub message: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub formatted: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct EntryPointReport {
    pub name: String,
    pub stage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workgroup_size: Option<[u32; 3]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub early_depth_test: Option<String>,
    pub arguments: Vec<FunctionParameterReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_type: Option<FunctionReturnReport>,
}

#[derive(Debug, Serialize)]
pub(crate) struct FunctionParameterReport {
    pub name: String,
    pub wgsl_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct FunctionReturnReport {
    pub wgsl_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ResourceBindingReport {
    pub group: u32,
    pub binding: u32,
    pub name: String,
    pub address_space: String,
    pub wgsl_type: String,
    pub size_bytes: u32,
    pub alignment_bytes: u32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub struct_members: Vec<StructMemberLayoutReport>,
}

#[derive(Debug, Serialize)]
pub(crate) struct StructMemberLayoutReport {
    pub name: String,
    pub wgsl_type: String,
    pub offset: u32,
    pub size_bytes: u32,
    pub alignment_bytes: u32,
}

#[derive(Debug, Serialize)]
pub(crate) struct OverrideReport {
    pub id: Option<u16>,
    pub name: String,
    pub wgsl_type: String,
    pub has_default: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct ImmediateReport {
    pub name: String,
    pub wgsl_type: String,
    pub size_bytes: u32,
    pub alignment_bytes: u32,
}

#[derive(Debug, Serialize)]
pub(crate) struct WorkgroupVariableReport {
    pub name: String,
    pub wgsl_type: String,
    pub size_bytes: u32,
    pub alignment_bytes: u32,
}

pub(crate) fn parse_and_validate_wgsl(
    source: &str,
    source_label: &str,
) -> std::result::Result<(naga::Module, naga::valid::ModuleInfo), DiagnosticReport> {
    let module = match naga::front::wgsl::parse_str(source) {
        Ok(module) => module,
        Err(parse_error) => {
            let formatted = parse_error.emit_to_string_with_path(source, source_label);
            let location = parse_error
                .labels()
                .next()
                .map(|(span, _)| span.location(source));
            return Err(DiagnosticReport {
                phase: "parse",
                message: parse_error.message().to_owned(),
                line: location.map(|loc| loc.line_number),
                column: location.map(|loc| loc.line_position),
                formatted,
            });
        }
    };

    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    validator
        .subgroup_stages(naga::valid::ShaderStages::all())
        .subgroup_operations(naga::valid::SubgroupOperationSet::all());

    let module_info = match validator.validate(&module) {
        Ok(info) => info,
        Err(validation_error) => {
            let formatted = validation_error.emit_to_string_with_path(source, source_label);
            let location = validation_error.location(source);
            return Err(DiagnosticReport {
                phase: "validate",
                message: validation_error.as_inner().to_string(),
                line: location.map(|loc| loc.line_number),
                column: location.map(|loc| loc.line_position),
                formatted,
            });
        }
    };

    Ok((module, module_info))
}

pub(crate) fn inspect_module(
    module: &naga::Module,
    source_label: &str,
) -> Result<ShaderValidationReport> {
    let mut layouter = naga::proc::Layouter::default();
    layouter
        .update(module.to_ctx())
        .context("failed to compute Naga type layouts")?;

    let entry_points = module
        .entry_points
        .iter()
        .map(|entry_point| {
            let workgroup_size = match entry_point.stage {
                naga::ShaderStage::Compute | naga::ShaderStage::Task | naga::ShaderStage::Mesh => {
                    Some(entry_point.workgroup_size)
                }
                _ => None,
            };
            let arguments = entry_point
                .function
                .arguments
                .iter()
                .map(|argument| FunctionParameterReport {
                    name: argument.name.clone().unwrap_or_else(|| "_".to_owned()),
                    wgsl_type: format_wgsl_type(module, argument.ty),
                    binding: argument.binding.as_ref().map(format_binding),
                })
                .collect();
            let return_type = entry_point
                .function
                .result
                .as_ref()
                .map(|result| FunctionReturnReport {
                    wgsl_type: format_wgsl_type(module, result.ty),
                    binding: result.binding.as_ref().map(format_binding),
                });
            EntryPointReport {
                name: entry_point.name.clone(),
                stage: format!("{:?}", entry_point.stage),
                workgroup_size,
                early_depth_test: entry_point
                    .early_depth_test
                    .map(|depth_test| format!("{depth_test:?}")),
                arguments,
                return_type,
            }
        })
        .collect();

    let mut resource_bindings = Vec::new();
    let mut immediates = Vec::new();
    let mut workgroup_variables = Vec::new();

    for (_, global) in module.global_variables.iter() {
        let type_layout = layouter[global.ty];
        let alignment_bytes = type_layout.alignment * 1u32;
        let var_name = global.name.clone().unwrap_or_else(|| "_unnamed".to_owned());
        let wgsl_type = format_wgsl_type(module, global.ty);

        if let Some(binding) = global.binding {
            let struct_members = match &module.types[global.ty].inner {
                naga::TypeInner::Struct { members, .. } => members
                    .iter()
                    .map(|member| {
                        let member_layout = layouter[member.ty];
                        StructMemberLayoutReport {
                            name: member.name.clone().unwrap_or_else(|| "_".to_owned()),
                            wgsl_type: format_wgsl_type(module, member.ty),
                            offset: member.offset,
                            size_bytes: member_layout.size,
                            alignment_bytes: member_layout.alignment * 1u32,
                        }
                    })
                    .collect(),
                _ => Vec::new(),
            };
            resource_bindings.push(ResourceBindingReport {
                group: binding.group,
                binding: binding.binding,
                name: var_name,
                address_space: format_address_space(global.space),
                wgsl_type,
                size_bytes: type_layout.size,
                alignment_bytes,
                struct_members,
            });
        } else if global.space == naga::AddressSpace::Immediate {
            immediates.push(ImmediateReport {
                name: var_name,
                wgsl_type,
                size_bytes: type_layout.size,
                alignment_bytes,
            });
        } else if global.space == naga::AddressSpace::WorkGroup {
            workgroup_variables.push(WorkgroupVariableReport {
                name: var_name,
                wgsl_type,
                size_bytes: type_layout.size,
                alignment_bytes,
            });
        }
    }

    resource_bindings.sort_by_key(|item| (item.group, item.binding));

    let overrides = module
        .overrides
        .iter()
        .map(|(_, override_item)| OverrideReport {
            id: override_item.id,
            name: override_item
                .name
                .clone()
                .unwrap_or_else(|| "_unnamed".to_owned()),
            wgsl_type: format_wgsl_type(module, override_item.ty),
            has_default: override_item.init.is_some(),
        })
        .collect();

    Ok(ShaderValidationReport {
        valid: true,
        path: source_label.to_owned(),
        diagnostic: None,
        entry_points,
        resource_bindings,
        overrides,
        immediates,
        workgroup_variables,
    })
}

pub(crate) fn run_validate(args: ValidateArgs) -> Result<()> {
    let path_display = args.shader.display().to_string();
    let source = fs::read_to_string(&args.shader)
        .with_context(|| format!("failed to read WGSL file '{path_display}'"))?;

    match parse_and_validate_wgsl(&source, &path_display) {
        Ok((module, _module_info)) => {
            let report = inspect_module(&module, &path_display)?;
            if args.json {
                let json = serde_json::to_string_pretty(&report)
                    .context("failed to serialize validation report")?;
                println!("{json}");
                return Ok(());
            }
            print_validation_report(&report);
            Ok(())
        }
        Err(diagnostic) => {
            if args.json {
                let failure_report = ShaderValidationReport {
                    valid: false,
                    path: path_display,
                    diagnostic: Some(diagnostic),
                    entry_points: Vec::new(),
                    resource_bindings: Vec::new(),
                    overrides: Vec::new(),
                    immediates: Vec::new(),
                    workgroup_variables: Vec::new(),
                };
                let json = serde_json::to_string_pretty(&failure_report)
                    .context("failed to serialize validation failure report")?;
                println!("{json}");
                bail!("WGSL validation failed");
            }
            bail!("WGSL {} error:\n{}", diagnostic.phase, diagnostic.formatted);
        }
    }
}

fn print_validation_report(report: &ShaderValidationReport) {
    println!("VALID WGSL: {}", report.path);

    println!("\nEntry points ({}):", report.entry_points.len());
    if report.entry_points.is_empty() {
        println!("  (none)");
    }
    for entry_point in &report.entry_points {
        let workgroup_suffix = match entry_point.workgroup_size {
            Some([x, y, z]) => format!(" @workgroup_size({x}, {y}, {z})"),
            None => String::new(),
        };
        let depth_suffix = match &entry_point.early_depth_test {
            Some(depth_mode) => format!(" early_depth_test={depth_mode}"),
            None => String::new(),
        };
        println!(
            "  - {} [{}]{}{}",
            entry_point.name, entry_point.stage, workgroup_suffix, depth_suffix
        );
        for argument in &entry_point.arguments {
            let binding_text = argument.binding.as_deref().unwrap_or("-");
            println!(
                "      in  {}: {} ({binding_text})",
                argument.name, argument.wgsl_type
            );
        }
        if let Some(return_type) = &entry_point.return_type {
            let binding_text = return_type.binding.as_deref().unwrap_or("struct");
            println!("      out {} ({binding_text})", return_type.wgsl_type);
        }
    }

    println!("\nResource bindings ({}):", report.resource_bindings.len());
    if report.resource_bindings.is_empty() {
        println!("  (none)");
    }
    for resource in &report.resource_bindings {
        println!(
            "  - @group({}) @binding({}) {} : {} [{}] (size={} B, align={} B)",
            resource.group,
            resource.binding,
            resource.name,
            resource.wgsl_type,
            resource.address_space,
            resource.size_bytes,
            resource.alignment_bytes
        );
        for member in &resource.struct_members {
            println!(
                "      +{:<4} {:<20} : {:<18} (size={} B, align={} B)",
                member.offset,
                member.name,
                member.wgsl_type,
                member.size_bytes,
                member.alignment_bytes
            );
        }
    }

    if !report.overrides.is_empty() {
        println!("\nPipeline overrides ({}):", report.overrides.len());
        for override_item in &report.overrides {
            let id_label = match override_item.id {
                Some(id) => format!("@id({id}) "),
                None => String::new(),
            };
            println!(
                "  - {}{} : {} (default={})",
                id_label, override_item.name, override_item.wgsl_type, override_item.has_default
            );
        }
    }

    if !report.immediates.is_empty() {
        println!("\nImmediate constants ({}):", report.immediates.len());
        for immediate in &report.immediates {
            println!(
                "  - {} : {} (size={} B, align={} B)",
                immediate.name, immediate.wgsl_type, immediate.size_bytes, immediate.alignment_bytes
            );
        }
    }

    if !report.workgroup_variables.is_empty() {
        println!(
            "\nWorkgroup shared variables ({}):",
            report.workgroup_variables.len()
        );
        for variable in &report.workgroup_variables {
            println!(
                "  - {} : {} (size={} B, align={} B)",
                variable.name, variable.wgsl_type, variable.size_bytes, variable.alignment_bytes
            );
        }
    }
}

pub(crate) enum TranspiledArtifact {
    Text(String),
    SpirvWords(Vec<u32>),
}

pub(crate) fn transpile_wgsl_source(
    source: &str,
    source_label: &str,
    target: TranspileTarget,
    stage: Option<ShaderStageSelection>,
    entry_name: Option<&str>,
    webgl2: bool,
) -> Result<TranspiledArtifact> {
    let (raw_module, raw_info) = match parse_and_validate_wgsl(source, source_label) {
        Ok(pair) => pair,
        Err(diagnostic) => {
            bail!("WGSL {} error:\n{}", diagnostic.phase, diagnostic.formatted);
        }
    };

    let requires_single_entry = matches!(target, TranspileTarget::Glsl | TranspileTarget::Webgl2);
    let selected_entry = resolve_entry_point(&raw_module, stage, entry_name, requires_single_entry)?;

    let pipeline_constants = naga::back::PipelineConstants::default();
    let (module, module_info) = naga::back::pipeline_constants::process_overrides(
        &raw_module,
        &raw_info,
        selected_entry
            .as_ref()
            .map(|(entry_stage, name)| (*entry_stage, name.as_str())),
        &pipeline_constants,
    )
    .context("failed to evaluate WGSL pipeline-overridable constants")?;

    match target {
        TranspileTarget::Wgsl => {
            let text = naga::back::wgsl::write_string(
                &module,
                &module_info,
                naga::back::wgsl::WriterFlags::empty(),
            )
            .context("Naga WGSL writer failed")?;
            Ok(TranspiledArtifact::Text(text))
        }
        TranspileTarget::Glsl | TranspileTarget::Webgl2 => {
            let (entry_stage, entry_point) = selected_entry.context(
                "GLSL cross-compilation requires a single entry point (--stage and/or --entry)",
            )?;
            let use_webgl2 = webgl2 || target == TranspileTarget::Webgl2;
            if use_webgl2 && entry_stage == naga::ShaderStage::Compute {
                bail!(
                    "WebGL2 (GLSL ES 300) does not support compute shaders; use `--target glsl` for Desktop GLSL 450"
                );
            }
            let version = if use_webgl2 {
                naga::back::glsl::Version::Embedded {
                    version: 300,
                    is_webgl: true,
                }
            } else {
                naga::back::glsl::Version::Desktop(450)
            };

            let mut binding_map = naga::back::glsl::BindingMap::default();
            let mut next_binding_slot: u8 = 0;
            for (_, global) in module.global_variables.iter() {
                if let Some(binding) = global.binding {
                    if matches!(global.space, naga::AddressSpace::Handle)
                        || !version.supports_explicit_locations()
                    {
                        binding_map.insert(binding, next_binding_slot);
                        next_binding_slot = next_binding_slot.saturating_add(1);
                    }
                }
            }

            let options = naga::back::glsl::Options {
                version,
                writer_flags: naga::back::glsl::WriterFlags::ADJUST_COORDINATE_SPACE,
                binding_map,
                zero_initialize_workgroup_memory: true,
            };
            let pipeline_options = naga::back::glsl::PipelineOptions {
                shader_stage: entry_stage,
                entry_point,
                multiview: None,
            };

            let mut text = String::new();
            let mut writer = naga::back::glsl::Writer::new(
                &mut text,
                &module,
                &module_info,
                &options,
                &pipeline_options,
                naga::proc::BoundsCheckPolicies::default(),
            )
            .context("failed to initialize Naga GLSL writer")?;
            writer.write().context("Naga GLSL writer failed")?;
            Ok(TranspiledArtifact::Text(text))
        }
        TranspileTarget::Hlsl => {
            let options = naga::back::hlsl::Options {
                shader_model: naga::back::hlsl::ShaderModel::V6_0,
                fake_missing_bindings: true,
                ..naga::back::hlsl::Options::default()
            };
            let pipeline_options = naga::back::hlsl::PipelineOptions {
                entry_point: selected_entry,
            };
            let mut text = String::new();
            let mut writer = naga::back::hlsl::Writer::new(&mut text, &options, &pipeline_options);
            writer
                .write(&module, &module_info, None)
                .context("Naga HLSL writer failed")?;
            Ok(TranspiledArtifact::Text(text))
        }
        TranspileTarget::Msl => {
            let per_entry_point_map = build_msl_entry_point_resources(&module);
            let options = naga::back::msl::Options {
                lang_version: (2, 4),
                per_entry_point_map,
                fake_missing_bindings: false,
                ..naga::back::msl::Options::default()
            };
            let pipeline_options = naga::back::msl::PipelineOptions {
                entry_point: selected_entry,
                ..naga::back::msl::PipelineOptions::default()
            };
            let (text, _translation_info) =
                naga::back::msl::write_string(&module, &module_info, &options, &pipeline_options)
                    .context("Naga MSL writer failed")?;
            Ok(TranspiledArtifact::Text(text))
        }
        TranspileTarget::Spv => {
            let options = naga::back::spv::Options {
                lang_version: (1, 3),
                flags: naga::back::spv::WriterFlags::ADJUST_COORDINATE_SPACE
                    | naga::back::spv::WriterFlags::LABEL_VARYINGS
                    | naga::back::spv::WriterFlags::CLAMP_FRAG_DEPTH,
                fake_missing_bindings: true,
                ..naga::back::spv::Options::default()
            };
            let pipeline_options =
                selected_entry.map(|(shader_stage, entry_point)| naga::back::spv::PipelineOptions {
                    shader_stage,
                    entry_point,
                });
            let words = naga::back::spv::write_vec(
                &module,
                &module_info,
                &options,
                pipeline_options.as_ref(),
            )
            .context("Naga SPIR-V writer failed")?;
            Ok(TranspiledArtifact::SpirvWords(words))
        }
    }
}

pub(crate) fn run_transpile(args: TranspileArgs) -> Result<()> {
    let path_display = args.shader.display().to_string();
    let source = fs::read_to_string(&args.shader)
        .with_context(|| format!("failed to read WGSL file '{path_display}'"))?;

    let artifact = transpile_wgsl_source(
        &source,
        &path_display,
        args.target,
        args.stage,
        args.entry.as_deref(),
        args.webgl2,
    )?;

    match (artifact, args.output.as_deref()) {
        (TranspiledArtifact::Text(text), Some(output_path)) => {
            write_output_file(output_path, text.as_bytes())?;
            println!(
                "Wrote {} bytes ({:?}) to {}",
                text.len(),
                args.target,
                output_path.display()
            );
        }
        (TranspiledArtifact::Text(text), None) => {
            print!("{text}");
            if !text.ends_with('\n') {
                println!();
            }
        }
        (TranspiledArtifact::SpirvWords(words), Some(output_path)) => {
            let bytes: Vec<u8> = words.iter().flat_map(|word| word.to_le_bytes()).collect();
            write_output_file(output_path, &bytes)?;
            println!(
                "Wrote {} SPIR-V words ({} bytes) to {}",
                words.len(),
                bytes.len(),
                output_path.display()
            );
        }
        (TranspiledArtifact::SpirvWords(words), None) => {
            println!(
                "; SPIR-V binary ({} words, {} bytes) — pass -o <file.spv> to write raw binary",
                words.len(),
                words.len() * 4
            );
            if let [magic, version, generator, bound, schema, ..] = words.as_slice() {
                println!(
                    "; Magic: 0x{magic:08x} | Version: 0x{version:08x} | Generator: 0x{generator:08x} | Bound: {bound} | Schema: {schema}"
                );
            }
            for (chunk_index, chunk) in words.chunks(8).enumerate() {
                let formatted_words: Vec<String> =
                    chunk.iter().map(|word| format!("0x{word:08x}")).collect();
                println!("  [{:04}] {}", chunk_index * 8, formatted_words.join(" "));
            }
        }
    }

    Ok(())
}

fn write_output_file(path: &Path, contents: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create parent directory '{}'", parent.display())
            })?;
        }
    }
    fs::write(path, contents)
        .with_context(|| format!("failed to write output file '{}'", path.display()))
}

fn resolve_entry_point(
    module: &naga::Module,
    stage: Option<ShaderStageSelection>,
    entry_name: Option<&str>,
    require_single: bool,
) -> Result<Option<(naga::ShaderStage, String)>> {
    let desired_stage = stage.map(ShaderStageSelection::to_naga_stage);

    if desired_stage.is_none() && entry_name.is_none() {
        if !require_single {
            return Ok(None);
        }
        if module.entry_points.len() == 1 {
            let single = &module.entry_points[0];
            return Ok(Some((single.stage, single.name.clone())));
        }
        let available = format_available_entry_points(module);
        bail!(
            "shader defines {} entry points ({available}); pass `--stage` and/or `--entry` to select one",
            module.entry_points.len()
        );
    }

    let matching: Vec<&naga::EntryPoint> = module
        .entry_points
        .iter()
        .filter(|entry_point| {
            let stage_matches = desired_stage.is_none_or(|target| entry_point.stage == target);
            let name_matches = entry_name.is_none_or(|target| entry_point.name == target);
            stage_matches && name_matches
        })
        .collect();

    match matching.as_slice() {
        [selected] => Ok(Some((selected.stage, selected.name.clone()))),
        [] => {
            let available = format_available_entry_points(module);
            bail!("no matching entry point found (available: {available})");
        }
        _ => {
            let available = format_available_entry_points(module);
            bail!(
                "multiple entry points matched; disambiguate with both `--stage` and `--entry` (available: {available})"
            );
        }
    }
}

fn format_available_entry_points(module: &naga::Module) -> String {
    if module.entry_points.is_empty() {
        return "none".to_owned();
    }
    module
        .entry_points
        .iter()
        .map(|entry_point| format!("{:?}:{}", entry_point.stage, entry_point.name))
        .collect::<Vec<_>>()
        .join(", ")
}

fn build_msl_entry_point_resources(module: &naga::Module) -> naga::back::msl::EntryPointResourceMap {
    let mut resource_map = BTreeMap::new();

    for entry_point in &module.entry_points {
        let mut resources = naga::back::msl::EntryPointResources::default();
        let mut next_buffer_slot: u8 = 0;
        let mut next_texture_slot: u8 = 0;
        let mut next_sampler_slot: u8 = 0;
        let mut has_immediates = false;
        let mut needs_sizes_buffer = false;

        for (_, global) in module.global_variables.iter() {
            if global.space == naga::AddressSpace::Immediate {
                has_immediates = true;
            }
            if type_contains_runtime_array(module, global.ty) {
                needs_sizes_buffer = true;
            }

            let Some(binding) = global.binding else {
                continue;
            };

            let target = match &module.types[global.ty].inner {
                naga::TypeInner::Image {
                    class: naga::ImageClass::External,
                    ..
                } => {
                    let plane0 = next_texture_slot;
                    let plane1 = next_texture_slot.saturating_add(1);
                    let plane2 = next_texture_slot.saturating_add(2);
                    next_texture_slot = next_texture_slot.saturating_add(3);
                    let params = next_buffer_slot;
                    next_buffer_slot = next_buffer_slot.saturating_add(1);
                    naga::back::msl::BindTarget {
                        external_texture: Some(naga::back::msl::BindExternalTextureTarget {
                            planes: [plane0, plane1, plane2],
                            params,
                        }),
                        ..naga::back::msl::BindTarget::default()
                    }
                }
                naga::TypeInner::Image {
                    class: naga::ImageClass::Storage { access, .. },
                    ..
                } => {
                    let slot = next_texture_slot;
                    next_texture_slot = next_texture_slot.saturating_add(1);
                    naga::back::msl::BindTarget {
                        texture: Some(slot),
                        mutable: access.contains(naga::StorageAccess::STORE),
                        ..naga::back::msl::BindTarget::default()
                    }
                }
                naga::TypeInner::Image { .. } => {
                    let slot = next_texture_slot;
                    next_texture_slot = next_texture_slot.saturating_add(1);
                    naga::back::msl::BindTarget {
                        texture: Some(slot),
                        ..naga::back::msl::BindTarget::default()
                    }
                }
                naga::TypeInner::Sampler { .. } => {
                    let slot = next_sampler_slot;
                    next_sampler_slot = next_sampler_slot.saturating_add(1);
                    naga::back::msl::BindTarget {
                        sampler: Some(naga::back::msl::BindSamplerTarget::Resource(slot)),
                        ..naga::back::msl::BindTarget::default()
                    }
                }
                _ => {
                    let slot = next_buffer_slot;
                    next_buffer_slot = next_buffer_slot.saturating_add(1);
                    let mutable = matches!(
                        global.space,
                        naga::AddressSpace::Storage { access }
                            if access.contains(naga::StorageAccess::STORE)
                    );
                    naga::back::msl::BindTarget {
                        buffer: Some(slot),
                        mutable,
                        ..naga::back::msl::BindTarget::default()
                    }
                }
            };

            resources.resources.insert(binding, target);
        }

        if has_immediates {
            resources.immediates_buffer = Some(next_buffer_slot);
            next_buffer_slot = next_buffer_slot.saturating_add(1);
        }
        if needs_sizes_buffer {
            resources.sizes_buffer = Some(next_buffer_slot);
        }

        resource_map.insert(entry_point.name.clone(), resources);
    }

    resource_map
}

fn type_contains_runtime_array(module: &naga::Module, ty_handle: naga::Handle<naga::Type>) -> bool {
    match &module.types[ty_handle].inner {
        naga::TypeInner::Array {
            size: naga::ArraySize::Dynamic,
            ..
        } => true,
        naga::TypeInner::Struct { members, .. } => members
            .last()
            .is_some_and(|member| type_contains_runtime_array(module, member.ty)),
        _ => false,
    }
}

fn format_binding(binding: &naga::Binding) -> String {
    match binding {
        naga::Binding::BuiltIn(built_in) => format!("@builtin({built_in:?})"),
        naga::Binding::Location {
            location,
            blend_src: Some(blend_src),
            ..
        } => format!("@location({location}) @blend_src({blend_src})"),
        naga::Binding::Location { location, .. } => format!("@location({location})"),
    }
}

fn format_address_space(space: naga::AddressSpace) -> String {
    match space {
        naga::AddressSpace::Function => "function".to_owned(),
        naga::AddressSpace::Private => "private".to_owned(),
        naga::AddressSpace::WorkGroup => "workgroup".to_owned(),
        naga::AddressSpace::Uniform => "uniform".to_owned(),
        naga::AddressSpace::Storage { access } => {
            if access.contains(naga::StorageAccess::STORE) {
                "storage, read_write".to_owned()
            } else {
                "storage, read".to_owned()
            }
        }
        naga::AddressSpace::Handle => "handle".to_owned(),
        naga::AddressSpace::Immediate => "immediate".to_owned(),
        naga::AddressSpace::TaskPayload => "task_payload".to_owned(),
        naga::AddressSpace::RayPayload => "ray_payload".to_owned(),
        naga::AddressSpace::IncomingRayPayload => "incoming_ray_payload".to_owned(),
    }
}

fn format_scalar(scalar: naga::Scalar) -> String {
    match (scalar.kind, scalar.width) {
        (naga::ScalarKind::Bool, _) => "bool".to_owned(),
        (naga::ScalarKind::Sint, 4) => "i32".to_owned(),
        (naga::ScalarKind::Sint, 8) => "i64".to_owned(),
        (naga::ScalarKind::Sint, 2) => "i16".to_owned(),
        (naga::ScalarKind::Uint, 4) => "u32".to_owned(),
        (naga::ScalarKind::Uint, 8) => "u64".to_owned(),
        (naga::ScalarKind::Uint, 2) => "u16".to_owned(),
        (naga::ScalarKind::Float, 2) => "f16".to_owned(),
        (naga::ScalarKind::Float, 4) => "f32".to_owned(),
        (naga::ScalarKind::Float, 8) => "f64".to_owned(),
        (kind, width) => format!("{kind:?}{}", width * 8),
    }
}

fn vector_dim(size: naga::VectorSize) -> u8 {
    match size {
        naga::VectorSize::Bi => 2,
        naga::VectorSize::Tri => 3,
        naga::VectorSize::Quad => 4,
    }
}

pub(crate) fn format_wgsl_type(module: &naga::Module, ty_handle: naga::Handle<naga::Type>) -> String {
    let ty = &module.types[ty_handle];
    if let Some(name) = &ty.name {
        return name.clone();
    }
    match &ty.inner {
        naga::TypeInner::Scalar(scalar) => format_scalar(*scalar),
        naga::TypeInner::Vector { size, scalar } => {
            format!("vec{}<{}>", vector_dim(*size), format_scalar(*scalar))
        }
        naga::TypeInner::Matrix {
            columns,
            rows,
            scalar,
        } => format!(
            "mat{}x{}<{}>",
            vector_dim(*columns),
            vector_dim(*rows),
            format_scalar(*scalar)
        ),
        naga::TypeInner::Atomic(scalar) => format!("atomic<{}>", format_scalar(*scalar)),
        naga::TypeInner::Array { base, size, .. } => {
            let elem = format_wgsl_type(module, *base);
            match size {
                naga::ArraySize::Constant(count) => format!("array<{elem}, {count}>"),
                naga::ArraySize::Pending(_) => format!("array<{elem}, override>"),
                naga::ArraySize::Dynamic => format!("array<{elem}>"),
            }
        }
        naga::TypeInner::Struct { members, .. } => {
            format!("struct({} members)", members.len())
        }
        naga::TypeInner::Image {
            dim,
            arrayed,
            class,
        } => {
            let array_suffix = if *arrayed { "_array" } else { "" };
            let dim_text = match dim {
                naga::ImageDimension::D1 => "1d",
                naga::ImageDimension::D2 => "2d",
                naga::ImageDimension::D3 => "3d",
                naga::ImageDimension::Cube => "cube",
            };
            match class {
                naga::ImageClass::Sampled { kind, multi } => {
                    let ms_prefix = if *multi { "multisampled_" } else { "" };
                    let scalar_name = match kind {
                        naga::ScalarKind::Float => "f32",
                        naga::ScalarKind::Sint => "i32",
                        naga::ScalarKind::Uint => "u32",
                        other => return format!("texture_{ms_prefix}{dim_text}{array_suffix}<{other:?}>"),
                    };
                    format!("texture_{ms_prefix}{dim_text}{array_suffix}<{scalar_name}>")
                }
                naga::ImageClass::Depth { multi } => {
                    let ms_prefix = if *multi { "multisampled_" } else { "" };
                    format!("texture_depth_{ms_prefix}{dim_text}{array_suffix}")
                }
                naga::ImageClass::Storage { format, access } => {
                    let access_text = if access.contains(naga::StorageAccess::LOAD)
                        && access.contains(naga::StorageAccess::STORE)
                    {
                        "read_write"
                    } else if access.contains(naga::StorageAccess::STORE) {
                        "write"
                    } else {
                        "read"
                    };
                    format!("texture_storage_{dim_text}{array_suffix}<{format:?}, {access_text}>")
                }
                naga::ImageClass::External => "texture_external".to_owned(),
            }
        }
        naga::TypeInner::Sampler { comparison } => {
            if *comparison {
                "sampler_comparison".to_owned()
            } else {
                "sampler".to_owned()
            }
        }
        naga::TypeInner::BindingArray { base, size } => {
            let elem = format_wgsl_type(module, *base);
            match size {
                naga::ArraySize::Constant(count) => format!("binding_array<{elem}, {count}>"),
                naga::ArraySize::Pending(_) => format!("binding_array<{elem}, override>"),
                naga::ArraySize::Dynamic => format!("binding_array<{elem}>"),
            }
        }
        other => format!("{other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RENDER_WGSL: &str = r#"
struct FrameUniforms {
    resolution: vec2<f32>,
    time: f32,
    _pad: f32,
}

@group(0) @binding(0)
var<uniform> frame: FrameUniforms;

override BRIGHTNESS: f32 = 1.0;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    let x = f32((vertex_index << 1u) & 2u);
    let y = f32(vertex_index & 2u);
    var out: VertexOut;
    out.position = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    out.uv = vec2<f32>(x, y);
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let normalized = input.uv * (frame.resolution / max(frame.resolution, vec2<f32>(1.0)));
    return vec4<f32>(normalized.x * BRIGHTNESS, normalized.y, fract(frame.time), 1.0);
}
"#;

    const SAMPLE_COMPUTE_WGSL: &str = r#"
@group(0) @binding(0)
var<storage, read_write> output_values: array<f32>;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if index < arrayLength(&output_values) {
        output_values[index] = f32(index) * 2.0;
    }
}
"#;

    #[test]
    fn validates_and_reflects_struct_layouts_and_overrides() -> Result<()> {
        let (module, _info) = match parse_and_validate_wgsl(SAMPLE_RENDER_WGSL, "sample.wgsl") {
            Ok(pair) => pair,
            Err(diagnostic) => bail!("unexpected validation error: {}", diagnostic.formatted),
        };
        let report = inspect_module(&module, "sample.wgsl")?;
        assert!(report.valid);
        assert_eq!(report.entry_points.len(), 2);
        assert_eq!(report.resource_bindings.len(), 1);
        assert_eq!(report.resource_bindings[0].size_bytes, 16);
        assert_eq!(report.resource_bindings[0].alignment_bytes, 8);
        assert_eq!(report.resource_bindings[0].struct_members.len(), 3);
        assert_eq!(report.overrides.len(), 1);
        assert_eq!(report.overrides[0].name, "BRIGHTNESS");
        Ok(())
    }

    #[test]
    fn reports_rich_diagnostics_on_invalid_wgsl() {
        let broken = "@fragment fn fs_main() -> @location(0) vec4<f32> { return vec4<f32>(1.0, 2.0); }";
        let result = parse_and_validate_wgsl(broken, "broken.wgsl");
        assert!(result.is_err());
        if let Err(diagnostic) = result {
            assert!(diagnostic.line.is_some());
            assert!(diagnostic.formatted.contains("broken.wgsl"));
        }
    }

    #[test]
    fn transpiles_render_shader_to_all_targets() -> Result<()> {
        for target in [
            TranspileTarget::Wgsl,
            TranspileTarget::Hlsl,
            TranspileTarget::Msl,
            TranspileTarget::Spv,
        ] {
            let artifact = transpile_wgsl_source(
                SAMPLE_RENDER_WGSL,
                "sample.wgsl",
                target,
                None,
                None,
                false,
            )?;
            match artifact {
                TranspiledArtifact::Text(text) => assert!(!text.is_empty()),
                TranspiledArtifact::SpirvWords(words) => {
                    assert!(words.len() > 5);
                    assert_eq!(words[0], 0x07230203);
                }
            }
        }

        let glsl_desktop = transpile_wgsl_source(
            SAMPLE_RENDER_WGSL,
            "sample.wgsl",
            TranspileTarget::Glsl,
            Some(ShaderStageSelection::Fragment),
            Some("fs_main"),
            false,
        )?;
        match glsl_desktop {
            TranspiledArtifact::Text(text) => assert!(text.contains("#version 450 core")),
            TranspiledArtifact::SpirvWords(_) => bail!("expected text GLSL output"),
        }

        let glsl_webgl2 = transpile_wgsl_source(
            SAMPLE_RENDER_WGSL,
            "sample.wgsl",
            TranspileTarget::Webgl2,
            Some(ShaderStageSelection::Fragment),
            Some("fs_main"),
            true,
        )?;
        match glsl_webgl2 {
            TranspiledArtifact::Text(text) => assert!(text.contains("#version 300 es")),
            TranspiledArtifact::SpirvWords(_) => bail!("expected text WebGL2 GLSL output"),
        }

        let compute_msl = transpile_wgsl_source(
            SAMPLE_COMPUTE_WGSL,
            "compute.wgsl",
            TranspileTarget::Msl,
            Some(ShaderStageSelection::Compute),
            Some("main"),
            false,
        )?;
        match compute_msl {
            TranspiledArtifact::Text(text) => assert!(text.contains("kernel void")),
            TranspiledArtifact::SpirvWords(_) => bail!("expected text MSL output"),
        }

        Ok(())
    }
}
