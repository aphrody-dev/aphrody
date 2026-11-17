use std::fs::{self, File};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, Result, bail};
use clap::{Args, Subcommand};
use flate2::Compression;
use flate2::write::GzEncoder;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use wasm_encoder::{Module as EncodedModule, RawSection};
use wasmparser::{Parser, Payload, WasmFeatures};

#[derive(Debug, Subcommand)]
pub(crate) enum WasmSubcommand {
    /// Inspect sections, imports, exports, gzip size, SHA-256, and features of a `.wasm` binary
    Inspect(InspectArgs),
    /// Strictly validate a `.wasm` or component binary with all WebAssembly features enabled
    Validate(ValidateBinaryArgs),
    /// Run Binaryen `wasm-opt -Oz` LTO size/feature optimization on a core `.wasm` binary
    Optimize(OptimizeArgs),
    /// Compile a `.wasm` / `.wit` component into a native `.cwasm` AOT artifact via `wasmtime compile`
    Aot(AotArgs),
    /// Generate ES/WebGPU JS bindings via `wasm-bindgen` matching workspace lockstep version
    Bindgen(BindgenArgs),
    /// Compose two Wasm components via `wac plug` or `wasm-tools compose`
    Compose(ComposeArgs),
    /// Strip custom sections (optionally keeping `name`) in pure Rust via `wasmparser` + `wasm-encoder`
    Strip(StripArgs),
}

#[derive(Debug, Args)]
pub(crate) struct InspectArgs {
    pub path: PathBuf,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub(crate) struct ValidateBinaryArgs {
    pub path: PathBuf,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub(crate) struct OptimizeArgs {
    pub input: PathBuf,
    #[arg(short = 'o', long = "output")]
    pub output: Option<PathBuf>,
    #[arg(long, default_value_t = true)]
    pub simd: bool,
    #[arg(long)]
    pub threads: bool,
}

#[derive(Debug, Args)]
pub(crate) struct AotArgs {
    pub input: PathBuf,
    #[arg(short = 'o', long = "output")]
    pub output: PathBuf,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub(crate) struct BindgenArgs {
    pub wasm: PathBuf,
    #[arg(long = "out-dir")]
    pub out_dir: PathBuf,
    #[arg(long, default_value = "web")]
    pub target: String,
}

#[derive(Debug, Args)]
pub(crate) struct ComposeArgs {
    pub socket: PathBuf,
    pub plug: PathBuf,
    #[arg(short = 'o', long = "output")]
    pub output: PathBuf,
}

#[derive(Debug, Args)]
pub(crate) struct StripArgs {
    pub input: PathBuf,
    #[arg(short = 'o', long = "output")]
    pub output: PathBuf,
    #[arg(long)]
    pub keep_name: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct SectionMetric {
    pub name: String,
    pub bytes: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct WasmInspectReport {
    pub path: String,
    pub is_component: bool,
    pub raw_bytes: usize,
    pub gzip_bytes: usize,
    pub sha256: String,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub custom_sections: Vec<String>,
    pub sections: Vec<SectionMetric>,
    pub uses_wasip2_or_component: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct WasmValidateReport {
    pub valid: bool,
    pub path: String,
    pub raw_bytes: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub(crate) fn run(cmd: WasmSubcommand) -> Result<()> {
    match cmd {
        WasmSubcommand::Inspect(args) => {
            let report = inspect_file(&args.path)?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "WASM {} ({}): raw={}B gzip={}B component={} imports={} exports={}",
                    report.path,
                    &report.sha256[..12],
                    report.raw_bytes,
                    report.gzip_bytes,
                    report.is_component,
                    report.imports.len(),
                    report.exports.len()
                );
            }
            Ok(())
        }
        WasmSubcommand::Validate(args) => {
            let report = validate_file(&args.path)?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else if report.valid {
                println!("VALID WASM: {} ({} bytes)", report.path, report.raw_bytes);
            }
            if !report.valid {
                bail!(
                    "Wasm validation failed for {}: {}",
                    report.path,
                    report.error.unwrap_or_default()
                );
            }
            Ok(())
        }
        WasmSubcommand::Optimize(args) => {
            let out = args.output.unwrap_or_else(|| args.input.clone());
            let (before, after) = optimize_wasm_file(&args.input, &out, args.simd, args.threads)?;
            println!(
                "Optimized {} -> {} ({}B -> {}B)",
                args.input.display(),
                out.display(),
                before,
                after
            );
            Ok(())
        }
        WasmSubcommand::Aot(args) => {
            let report = compile_aot_file(&args.input, &args.output)?;
            if args.json {
                println!("{report}");
            } else {
                println!("AOT compiled -> {}", args.output.display());
            }
            Ok(())
        }
        WasmSubcommand::Bindgen(args) => run_bindgen(&args.wasm, &args.out_dir, &args.target),
        WasmSubcommand::Compose(args) => run_compose(&args.socket, &args.plug, &args.output),
        WasmSubcommand::Strip(args) => {
            let (before, after) = strip_custom_sections(&args.input, &args.output, args.keep_name)?;
            println!(
                "Stripped {} -> {} ({}B -> {}B)",
                args.input.display(),
                args.output.display(),
                before,
                after
            );
            Ok(())
        }
    }
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for b in digest {
        if let Err(err) = write!(hex, "{b:02x}") {
            eprintln!("sha256 format error: {err}");
        }
    }
    hex
}

pub(crate) fn gzip_size(bytes: &[u8]) -> Result<usize> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(bytes)?;
    let compressed = encoder.finish()?;
    Ok(compressed.len())
}

pub(crate) fn inspect_bytes(label: &str, bytes: &[u8]) -> Result<WasmInspectReport> {
    let is_component = bytes.len() >= 8 && bytes[4..8] == [0x0d, 0x00, 0x01, 0x00];
    let mut imports = Vec::new();
    let mut exports = Vec::new();
    let mut custom_sections = Vec::new();
    let mut sections = Vec::new();

    for payload in Parser::new(0).parse_all(bytes) {
        let payload = payload.with_context(|| format!("failed parsing wasm '{label}'"))?;
        match payload {
            Payload::ImportSection(reader) => {
                let start = reader.range().start;
                let end = reader.range().end;
                sections.push(SectionMetric {
                    name: "import".to_owned(),
                    bytes: end.saturating_sub(start),
                });
                for imp in reader.into_imports() {
                    let imp = imp?;
                    imports.push(format!("{}::{}", imp.module, imp.name));
                }
            }
            Payload::ExportSection(reader) => {
                let start = reader.range().start;
                let end = reader.range().end;
                sections.push(SectionMetric {
                    name: "export".to_owned(),
                    bytes: end.saturating_sub(start),
                });
                for exp in reader {
                    let exp = exp?;
                    exports.push(exp.name.to_owned());
                }
            }
            Payload::CustomSection(reader) => {
                let name = reader.name().to_owned();
                sections.push(SectionMetric {
                    name: format!("custom:{name}"),
                    bytes: reader.data().len(),
                });
                custom_sections.push(name);
            }
            Payload::CodeSectionStart { size, .. } => {
                sections.push(SectionMetric {
                    name: "code".to_owned(),
                    bytes: size as usize,
                });
            }
            Payload::ComponentImportSection(reader) => {
                for imp in reader {
                    let imp = imp?;
                    imports.push(imp.name.name.to_owned());
                }
            }
            Payload::ComponentExportSection(reader) => {
                for exp in reader {
                    let exp = exp?;
                    exports.push(exp.name.name.to_owned());
                }
            }
            _ => {}
        }
    }

    let uses_wasip2_or_component =
        is_component || imports.iter().any(|imp| imp.starts_with("wasi:"));

    Ok(WasmInspectReport {
        path: label.to_owned(),
        is_component,
        raw_bytes: bytes.len(),
        gzip_bytes: gzip_size(bytes)?,
        sha256: sha256_hex(bytes),
        imports,
        exports,
        custom_sections,
        sections,
        uses_wasip2_or_component,
    })
}

pub(crate) fn inspect_file(path: &Path) -> Result<WasmInspectReport> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    inspect_bytes(&path.display().to_string(), &bytes)
}

pub(crate) fn validate_bytes(label: &str, bytes: &[u8]) -> WasmValidateReport {
    let mut validator = wasmparser::Validator::new_with_features(WasmFeatures::all());
    match validator.validate_all(bytes) {
        Ok(_) => WasmValidateReport {
            valid: true,
            path: label.to_owned(),
            raw_bytes: bytes.len(),
            error: None,
        },
        Err(err) => WasmValidateReport {
            valid: false,
            path: label.to_owned(),
            raw_bytes: bytes.len(),
            error: Some(err.to_string()),
        },
    }
}

pub(crate) fn validate_file(path: &Path) -> Result<WasmValidateReport> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(validate_bytes(&path.display().to_string(), &bytes))
}

#[allow(
    clippy::disallowed_methods,
    reason = "synchronous CLI subprocess wrapper in standalone binary"
)]
pub(crate) fn optimize_wasm_file(
    input: &Path,
    output: &Path,
    simd: bool,
    threads: bool,
) -> Result<(usize, usize)> {
    let before = fs::metadata(input)
        .with_context(|| format!("stat {}", input.display()))?
        .len() as usize;
    let mut args = vec![
        input.display().to_string(),
        "-Oz".to_owned(),
        "--enable-bulk-memory".to_owned(),
        "--enable-nontrapping-float-to-int".to_owned(),
        "--enable-sign-ext".to_owned(),
        "--enable-reference-types".to_owned(),
        "--enable-mutable-globals".to_owned(),
    ];
    if simd {
        args.push("--enable-simd".to_owned());
    }
    if threads {
        args.push("--enable-threads".to_owned());
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    args.push("-o".to_owned());
    args.push(output.display().to_string());

    let out = Command::new("wasm-opt")
        .args(&args)
        .output()
        .context("failed executing `wasm-opt` (is Binaryen installed on PATH?)")?;
    if !out.status.success() {
        bail!(
            "`wasm-opt` exited with {:?}: {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let after = fs::metadata(output)?.len() as usize;
    Ok((before, after))
}

#[allow(
    clippy::disallowed_methods,
    reason = "synchronous CLI subprocess wrapper in standalone binary"
)]
pub(crate) fn compile_aot_file(input: &Path, output: &Path) -> Result<serde_json::Value> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let res = Command::new("wasmtime")
        .arg("compile")
        .arg(input)
        .arg("-o")
        .arg(output)
        .output()
        .context("failed executing `wasmtime compile`")?;
    if !res.status.success() {
        bail!(
            "`wasmtime compile` exited with {:?}: {}",
            res.status.code(),
            String::from_utf8_lossy(&res.stderr)
        );
    }
    let compiled_bytes = fs::read(output)?;
    let manifest = serde_json::json!({
        "input": input.display().to_string(),
        "output": output.display().to_string(),
        "bytes": compiled_bytes.len(),
        "sha256": sha256_hex(&compiled_bytes),
    });
    let manifest_path = PathBuf::from(format!("{}.sha256.json", output.display()));
    File::create(&manifest_path)?.write_all(serde_json::to_string_pretty(&manifest)?.as_bytes())?;
    Ok(manifest)
}

#[allow(
    clippy::disallowed_methods,
    reason = "synchronous CLI subprocess wrapper in standalone binary"
)]
pub(crate) fn run_bindgen(wasm: &Path, out_dir: &Path, target: &str) -> Result<()> {
    fs::create_dir_all(out_dir)?;
    let res = Command::new("wasm-bindgen")
        .arg(wasm)
        .arg("--target")
        .arg(target)
        .arg("--out-dir")
        .arg(out_dir)
        .output()
        .context("failed executing `wasm-bindgen`")?;
    if !res.status.success() {
        bail!(
            "`wasm-bindgen` failed ({:?}): {}",
            res.status.code(),
            String::from_utf8_lossy(&res.stderr)
        );
    }
    Ok(())
}

#[allow(
    clippy::disallowed_methods,
    reason = "synchronous CLI subprocess wrapper in standalone binary"
)]
pub(crate) fn run_compose(socket: &Path, plug: &Path, output: &Path) -> Result<()> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let res = Command::new("wac")
        .arg("plug")
        .arg("--plug")
        .arg(plug)
        .arg(socket)
        .arg("-o")
        .arg(output)
        .output();
    match res {
        Ok(out) if out.status.success() => Ok(()),
        _ => {
            let fallback = Command::new("wasm-tools")
                .arg("compose")
                .arg(socket)
                .arg("-d")
                .arg(plug)
                .arg("-o")
                .arg(output)
                .output()
                .context("failed executing both `wac plug` and `wasm-tools compose`")?;
            if !fallback.status.success() {
                bail!(
                    "`wasm-tools compose` failed ({:?}): {}",
                    fallback.status.code(),
                    String::from_utf8_lossy(&fallback.stderr)
                );
            }
            Ok(())
        }
    }
}

pub(crate) fn strip_custom_sections(
    input: &Path,
    output: &Path,
    keep_name: bool,
) -> Result<(usize, usize)> {
    let bytes = fs::read(input).with_context(|| format!("reading {}", input.display()))?;
    let before = bytes.len();
    if before >= 8 && bytes[4..8] == [0x0d, 0x00, 0x01, 0x00] {
        bail!("stripping custom sections via wasm-encoder currently targets core wasm modules");
    }

    let mut out_module = EncodedModule::new();
    for payload in Parser::new(0).parse_all(&bytes) {
        let payload = payload?;
        if let Payload::CustomSection(ref reader) = payload
            && (reader.name() != "name" || !keep_name)
        {
            continue;
        }
        if let Some((id, range)) = payload.as_section() {
            out_module.section(&RawSection {
                id,
                data: &bytes[range],
            });
        }
    }
    let stripped = out_module.finish();
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output, &stripped)?;
    Ok((before, stripped.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_encoder::{CustomSection, Module};

    #[test]
    fn validates_inspects_and_strips_custom_sections_without_shelling_out() -> Result<()> {
        let mut module = Module::new();
        module.section(&CustomSection {
            name: std::borrow::Cow::Borrowed("producers"),
            data: std::borrow::Cow::Borrowed(b"aphrody-test"),
        });
        let raw_bytes = module.finish();

        let dir = tempfile::tempdir()?;
        let in_path = dir.path().join("sample.wasm");
        let out_path = dir.path().join("stripped.wasm");
        fs::write(&in_path, &raw_bytes)?;

        let val = validate_file(&in_path)?;
        assert!(val.valid);
        let report = inspect_file(&in_path)?;
        assert_eq!(report.custom_sections, vec!["producers".to_owned()]);

        let (before, after) = strip_custom_sections(&in_path, &out_path, false)?;
        assert!(after < before);
        let after_report = inspect_file(&out_path)?;
        assert!(after_report.custom_sections.is_empty());
        Ok(())
    }
}
