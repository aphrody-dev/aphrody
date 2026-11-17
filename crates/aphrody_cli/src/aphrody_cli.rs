//! Library facade & CLI dispatcher for `aphrody_cli` (`aphrody-cli` and `gpui-wgpu`),
//! uniting WGPU/Naga v30, WebAssembly LTO/AOT, Google Design, Storytold Craft, COSMIC Epoch,
//! Headless Zed + Tree-sitter, absorbed Agent-Browser CDP, and CachyOS/Aphrody DE Switcher.

pub use aphrody;
pub use cosmic_gpui::COSMIC_EPOCH_COMPONENTS;

mod adapters;
mod bench;
mod compute;
mod design_cli;
mod doctor;
mod mcp;
mod render;
mod shader;
mod templates;
mod unified;
mod wasm_toolchain;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context as _, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "aphrody-cli",
    about = "Unified Aphrody Full-Stack Developer CLI (Desktop, WebGPU/Wasm, Design, Craft, COSMIC, Headless Zed & MCP)",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Enumerate GPU hardware and software adapters, capabilities, limits, and features
    Adapters(adapters::AdaptersArgs),
    /// Parse, validate, and reflect a WGSL shader module with Naga
    Validate(shader::ValidateArgs),
    /// Cross-compile a validated WGSL shader to WGSL, GLSL, HLSL, MSL, or SPIR-V
    Transpile(shader::TranspileArgs),
    /// Render an offscreen fullscreen pass with a WGSL shader to a PNG image
    Render(render::RenderArgs),
    /// Dispatch a headless WGSL compute shader and read back the output storage buffer
    Compute(compute::ComputeArgs),
    /// Benchmark GPU initialization, buffer upload bandwidth, compute latency, and render throughput
    Bench(bench::BenchArgs),
    /// Inspect, validate, Binaryen-optimize (-Oz), AOT-compile (.cwasm), bindgen, compose, and strip Wasm binaries
    Wasm {
        #[command(subcommand)]
        cmd: wasm_toolchain::WasmSubcommand,
    },
    /// Verify Rust wasm targets, Binaryen/Wasmtime/SH/GPU CLIs, and active wgpu adapters
    Doctor(doctor::DoctorArgs),
    /// Scaffold, build (--lto/--aot), and export Google Design / Tailwind tokens for Wasm + WebGPU apps
    App {
        #[command(subcommand)]
        cmd: templates::AppSubcommand,
    },
    /// Mirror https://design.google/, manage DESIGN.md & Stitch screens, and audit Core Web Vitals / Lighthouse
    Design {
        #[command(subcommand)]
        cmd: design_cli::DesignSubcommand,
    },
    /// Run stdio JSON-RPC 2.0 MCP server for Wasm, WebGPU, Design, Craft, COSMIC, Headless Zed & Browser
    Mcp(mcp::McpArgs),
    /// Unified full-stack build orchestrator (Desktop, Web/Wasm LTO, Headless, Craft, COSMIC, bun-release)
    Build(unified::BuildArgs),
    /// Unified 22-target deployment manifest & packager across Craft and COSMIC
    Deploy(unified::DeployArgs),
    /// Unified Rust/TS/TSX/Python Tree-sitter inspector & framework architecture matrix
    Dev {
        #[command(subcommand)]
        cmd: unified::DevSubcommand,
    },
    /// Expose Storytold / ArtCraft suite across CLI, MCP, Web, Desktop, build & deploy
    Craft {
        #[command(subcommand)]
        cmd: unified::CraftSubcommand,
    },
    /// Expose COSMIC Epoch + Pop!_OS subsystems across CLI, MCP, Web, Desktop, build & deploy
    Cosmic {
        #[command(subcommand)]
        cmd: unified::CosmicSubcommand,
    },
    /// Control headless Zed workspaces, Tree-sitter ASTs, and unified MCP mode
    Zed {
        #[command(subcommand)]
        cmd: unified::ZedSubcommand,
    },
    /// Plan CachyOS net-installations and switch live between Aphrody Rust DEs & Arch DEs
    Install {
        #[command(subcommand)]
        cmd: unified::InstallSubcommand,
    },
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub(crate) enum BackendSelection {
    #[default]
    All,
    Vulkan,
    Dx12,
    Metal,
    #[value(alias = "gles")]
    Gl,
}

impl BackendSelection {
    pub(crate) fn to_wgpu_backends(self) -> wgpu::Backends {
        match self {
            Self::All => wgpu::Backends::all(),
            Self::Vulkan => wgpu::Backends::VULKAN,
            Self::Dx12 => wgpu::Backends::DX12,
            Self::Metal => wgpu::Backends::METAL,
            Self::Gl => wgpu::Backends::GL,
        }
    }
}

pub(crate) fn create_instance(backends: wgpu::Backends) -> wgpu::Instance {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = backends;
    wgpu::Instance::new(descriptor.with_env())
}

pub(crate) fn request_headless_device(
    backends: wgpu::Backends,
    label: &'static str,
) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue)> {
    let instance = create_instance(backends);

    let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: None,
        apply_limit_buckets: false,
    })) {
        Ok(adapter) => adapter,
        Err(_) => match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::None,
            force_fallback_adapter: true,
            compatible_surface: None,
            apply_limit_buckets: false,
        })) {
            Ok(fallback_adapter) => fallback_adapter,
            Err(fallback_error) => {
                let mut enumerated = pollster::block_on(instance.enumerate_adapters(backends));
                if let Some(first_adapter) = enumerated.drain(..).next() {
                    first_adapter
                } else {
                    bail!(
                        "no compatible GPU adapter found for backends {backends:?}: {fallback_error}"
                    );
                }
            }
        },
    };

    let adapter_limits = adapter.limits();
    let mut required_limits = wgpu::Limits::default().or_worse_values_from(&adapter_limits);
    required_limits.max_texture_dimension_1d = adapter_limits.max_texture_dimension_1d;
    required_limits.max_texture_dimension_2d = adapter_limits.max_texture_dimension_2d;
    required_limits.max_texture_dimension_3d = adapter_limits.max_texture_dimension_3d;
    required_limits.max_buffer_size = adapter_limits.max_buffer_size;
    required_limits.max_storage_buffer_binding_size =
        adapter_limits.max_storage_buffer_binding_size;

    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some(label),
        required_features: wgpu::Features::empty(),
        required_limits,
        default_queue: wgpu::QueueDescriptor::default(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::MemoryUsage,
        trace: wgpu::Trace::Off,
    }))
    .with_context(|| {
        let info = adapter.get_info();
        format!(
            "failed to request GPU device on '{}' ({:?})",
            info.name, info.backend
        )
    })?;

    Ok((adapter, device, queue))
}

pub(crate) fn read_mapped_buffer(
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
        .context("failed while polling GPU device for buffer readback")?;

    let callback_result = match map_status.lock() {
        Ok(mut guard) => guard.take(),
        Err(poisoned) => poisoned.into_inner().take(),
    };
    match callback_result {
        Some(Ok(())) => {}
        Some(Err(error)) => bail!("failed to map GPU readback buffer: {error}"),
        None => bail!("GPU readback callback did not complete after device poll"),
    }

    let mapped_range = buffer
        .slice(..)
        .get_mapped_range()
        .context("failed to read mapped GPU buffer range")?;
    let bytes = mapped_range.to_vec();
    drop(mapped_range);
    buffer.unmap();
    Ok(bytes)
}

pub fn run_main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Adapters(args) => adapters::run(args),
        Command::Validate(args) => shader::run_validate(args),
        Command::Transpile(args) => shader::run_transpile(args),
        Command::Render(args) => render::run(args),
        Command::Compute(args) => compute::run(args),
        Command::Bench(args) => bench::run(args),
        Command::Wasm { cmd } => wasm_toolchain::run(cmd),
        Command::Doctor(args) => doctor::run(args),
        Command::App { cmd } => templates::run(cmd),
        Command::Design { cmd } => design_cli::run(cmd),
        Command::Mcp(args) => mcp::run_stdio_server(args),
        Command::Build(args) => unified::run_build(args),
        Command::Deploy(args) => unified::run_deploy(args),
        Command::Dev { cmd } => unified::run_dev(cmd),
        Command::Craft { cmd } => unified::run_craft(cmd),
        Command::Cosmic { cmd } => unified::run_cosmic(cmd),
        Command::Zed { cmd } => unified::run_zed(cmd),
        Command::Install { cmd } => unified::run_install(cmd),
    }
}
