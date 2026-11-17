use std::process::Command;

use anyhow::Result;
use clap::Args;
use serde::Serialize;

use crate::create_instance;

#[derive(Debug, Args)]
pub(crate) struct DoctorArgs {
    /// Emit machine-readable JSON diagnostics
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct CliProbeStatus {
    pub tool: &'static str,
    pub available: bool,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct AdapterProbeSummary {
    pub name: String,
    pub backend: String,
    pub device_type: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct DoctorReport {
    pub rust_targets: Vec<String>,
    pub tools: Vec<CliProbeStatus>,
    pub adapters: Vec<AdapterProbeSummary>,
    pub ready_for_wasm_webgpu: bool,
}

#[allow(
    clippy::disallowed_methods,
    reason = "synchronous CLI tool probe in standalone diagnostic binary"
)]
fn probe_cli(tool: &'static str, arg: &str) -> CliProbeStatus {
    match Command::new(tool).arg(arg).output() {
        Ok(out) if out.status.success() => {
            let line = String::from_utf8_lossy(&out.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .to_owned();
            CliProbeStatus {
                tool,
                available: true,
                version: line,
            }
        }
        _ => CliProbeStatus {
            tool,
            available: false,
            version: String::new(),
        },
    }
}

#[allow(
    clippy::disallowed_methods,
    reason = "synchronous rustup target probe in standalone diagnostic binary"
)]
fn probe_installed_wasm_targets() -> Vec<String> {
    let mut found = Vec::new();
    if let Ok(out) = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        && out.status.success()
    {
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            let t = line.trim();
            if t.contains("wasm") {
                found.push(t.to_owned());
            }
        }
    }
    found
}

pub(crate) fn collect_doctor_report() -> DoctorReport {
    let rust_targets = probe_installed_wasm_targets();
    let tools = [
        ("wasmtime", "--version"),
        ("wasm-opt", "--version"),
        ("wasm-bindgen", "--version"),
        ("wasm-tools", "--version"),
        ("wat2wasm", "--version"),
        ("wasm2wat", "--version"),
        ("wit-bindgen", "--version"),
        ("wac", "--version"),
        ("dxc", "--version"),
        ("glslangValidator", "--version"),
        ("spirv-val", "--version"),
        ("nu", "--version"),
        ("pwsh", "--version"),
        ("bun", "--version"),
        ("xh", "--version"),
        ("monolith", "--version"),
    ]
    .into_iter()
    .map(|(bin, arg)| probe_cli(bin, arg))
    .collect::<Vec<_>>();

    let instance = create_instance(wgpu::Backends::all());
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()))
        .into_iter()
        .map(|adapter| {
            let info = adapter.get_info();
            AdapterProbeSummary {
                name: info.name,
                backend: format!("{:?}", info.backend),
                device_type: format!("{:?}", info.device_type),
            }
        })
        .collect::<Vec<_>>();

    let has_wasm_opt = tools.iter().any(|t| t.tool == "wasm-opt" && t.available);
    let has_wasmtime = tools.iter().any(|t| t.tool == "wasmtime" && t.available);

    DoctorReport {
        ready_for_wasm_webgpu: !adapters.is_empty() && has_wasm_opt && has_wasmtime,
        rust_targets,
        tools,
        adapters,
    }
}

pub(crate) fn run(args: DoctorArgs) -> Result<()> {
    let report = collect_doctor_report();
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "gpui-wgpu doctor: adapters={} rust_wasm_targets={} ready={}",
            report.adapters.len(),
            report.rust_targets.join(","),
            report.ready_for_wasm_webgpu
        );
        for tool in &report.tools {
            println!(
                "  [{:<3}] {:<16} {}",
                if tool.available { "OK" } else { "NO" },
                tool.tool,
                tool.version
            );
        }
    }
    Ok(())
}
