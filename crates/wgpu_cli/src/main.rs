mod adapters;
mod bench;
mod compute;
mod render;
mod shader;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context as _, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "gpui-wgpu",
    about = "WGPU & Naga GPU adapter, shader validation/transpilation, headless render/compute, and benchmark CLI for aphrody",
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
    // Clamp WebGPU defaults down for downlevel backends while preserving zeroed native-only
    // feature-gated limits (immediates, ray-tracing, mesh shaders, binding arrays) so device
    // creation succeeds without extra feature flags.
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

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Adapters(args) => adapters::run(args),
        Command::Validate(args) => shader::run_validate(args),
        Command::Transpile(args) => shader::run_transpile(args),
        Command::Render(args) => render::run(args),
        Command::Compute(args) => compute::run(args),
        Command::Bench(args) => bench::run(args),
    }
}
