# Vendored WGPU v30.0.0 Architecture & Crates (`vendor/wgpu`)

## 1. Crate Hierarchy in `vendor/wgpu/`

`vendor/wgpu` contains the complete `gfx-rs/wgpu` v30.0.0 workspace (`cddb2f5d80dc0009b2ef6767635140cacf90a007`) without `.git` history:

```text
wgpu (safe idiomatic Rust API + WASM navigator.gpu backend + custom backend traits)
 ├── wgpu-types (Features, Limits, TextureFormat, PollType, MemoryHints, Backends)
 ├── wgpu-core (Hub, generational IDs, SnatchLock, resource Trackers, barriers, validation)
 │    ├── wgpu-naga-bridge (Features/DownlevelFlags -> naga::valid::Capabilities)
 │    └── wgpu-hal (unsafe zero-overhead Api trait)
 │         ├── dx12   (windows 0.62, gpu-allocator 0.28, range-alloc 0.1, FXC/DXC)
 │         ├── vulkan (ash 0.38, gpu-allocator 0.28, timeline semaphores, imageless FB)
 │         ├── metal  (objc2-metal 0.3.2, CAMetalLayer, IOSurface, vertex pulling)
 │         ├── gles   (glow 0.18, khronos-egl, WGL, WebGL2)
 │         └── noop   (zero-GPU test stub)
 └── naga (WGSL/GLSL/SPIR-V frontends -> Module IR -> Validator/Layouter -> WGSL/HLSL/MSL/SPIR-V/GLSL)
```

### Key Sub-Crates
- **`vendor/wgpu/wgpu/`**:
  - `src/api/`: Public handle types (`instance.rs`, `adapter.rs`, `device.rs`, `queue.rs`, `surface.rs`, `buffer.rs`, `texture.rs`, `texture_view.rs`, `sampler.rs`, `bind_group.rs`, `bind_group_layout.rs`, `pipeline_layout.rs`, `render_pipeline.rs`, `compute_pipeline.rs`, `command_encoder.rs`, `render_pass.rs`, `compute_pass.rs`, `render_bundle.rs`, `query_set.rs`, `blas.rs`, `tlas.rs`).
  - `src/dispatch.rs`: Runtime enum dispatch (`DispatchInstance`, `DispatchDevice`, etc.) bridging `ContextWgpuCore` (native + WASM WebGL2), `ContextWebGpu` (WASM browser WebGPU), and `custom` user backends (`feature = "custom"`).
  - `src/util/`: `DeviceExt::create_buffer_init`, `StagingBelt`, `DownloadBuffer`, `TextureDataOrder`, `new_instance_with_webgpu_detection`.
- **`vendor/wgpu/wgpu-core/`**:
  - `src/id.rs` & `src/storage.rs`: Generational `Id<T>` (`index: u32`, `epoch: NonZeroU32`) + `Registry<T>`.
  - `src/snatch.rs` & `src/lock/`: `Snatchable<T>` and ranked `RwLock` discipline ensuring `buffer.destroy()` / `texture.destroy()` immediately free GPU memory without UAF or deadlocks.
  - `src/track/`: Per-buffer `BufferTracker` and per-mip/layer `TextureTracker` computing exact `BufferTransition` and `TextureTransition` state barriers at `Queue::submit`.
  - `src/init_tracker/`: Lazy zero-initialization of uninitialized buffer ranges and texture subresources required by WebGPU memory safety.
  - `src/indirect_validation/`: Compute-shader clamping of indirect draw/dispatch buffers and DX12 `SV_VertexID`/`num_workgroups` fixups.
  - `src/binding_model.rs`, `src/command/`, `src/pipeline.rs`, `src/present.rs`, `src/validation.rs`.
- **`vendor/wgpu/wgpu-hal/`**:
  - `src/lib.rs`: Defines `pub trait Api` and unsafe traits `Instance`, `Surface`, `Adapter`, `Device`, `Queue`, `CommandEncoder`.
  - `src/dx12/`: Direct3D 12 backend:
    - `descriptor.rs` / `sampler.rs`: Solves D3D12's 2,048 shader-visible sampler heap cap via global sampler heap + per-group `StructuredBuffer<uint>` sampler index arrays (`nagaGroupXSamplerIndexArray`).
    - `suballocation.rs`: `gpu-allocator` `0.28` `d3d12` allocator driven by `MemoryHints`.
    - `shader_compilation.rs`: FXC (`d3dcompiler_47.dll`, SM 5.1) or DXC (`dxcompiler.dll` / `static-dxc`).
  - `src/vulkan/`: Vulkan backend (`ash` `0.38`):
    - `semaphore_list.rs`: Uses `VK_KHR_timeline_semaphore` 1:1 with `hal::Fence`, with binary semaphore fallback.
    - `device.rs` / `command.rs`: `VkRenderPass` and imageless `VkFramebuffer` caches; explicit `vkCmdPipelineBarrier`.
  - `src/metal/`: Apple Metal backend (`objc2-metal` `0.3.2`):
    - Flattens immediates + bind groups + vertex buffers into Metal `[[buffer(n)]]`, `[[texture(n)]]`, `[[sampler(n)]]` slots, passing runtime storage array lengths in `sizes_buffer`.
    - `CAMetalLayer` presentation + `IOSurface` zero-copy texture backing.
  - `src/gles/`: OpenGL 3.3+ / GLES 3.0+ / WebGL2 backend (`glow` `0.18`):
    - Pairs separate WebGPU textures and samplers into combined GL `sampler2D` units via `SamplerBindMap`.
    - Emulates `first_instance` on GLES 3.0 / WebGL2 using `naga_vs_first_instance` uniform + attribute offset shifts.
- **`vendor/wgpu/wgpu-types/`**:
  - Pure data structures (`#![no_std]`): `Features`, `Limits`, `DownlevelCapabilities`, `DownlevelFlags`, `TextureFormat`, `BufferUsages`, `TextureUsages`, `InstanceFlags`, `PollType`, `MemoryHints`, `TexelCopyBufferLayout`, `TexelCopyTextureInfo`.

## 2. Essential `wgpu` v30.0.0 API Signatures

### Initialization & Device Negotiation
```rust
let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
    backends: wgpu::Backends::all(),
    flags: wgpu::InstanceFlags::default(),
    memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
    backend_options: wgpu::BackendOptions::default(),
    display: None,
});

// Enumerating all adapters (async in v30):
let adapters: Vec<wgpu::Adapter> = instance.enumerate_adapters(wgpu::Backends::all()).await;

// Or requesting single adapter (returns Result in v30):
let adapter: wgpu::Adapter = instance
    .request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: None,
        apply_limit_buckets: false,
    })
    .await?;

let required_limits = wgpu::Limits::downlevel_defaults()
    .using_resolution(adapter.limits())
    .using_alignment(adapter.limits());

let (device, queue) = adapter
    .request_device(&wgpu::DeviceDescriptor {
        label: Some("app_device"),
        required_features: wgpu::Features::empty(),
        required_limits,
        default_queue: wgpu::QueueDescriptor::default(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::MemoryUsage,
        trace: wgpu::Trace::Off,
    })
    .await?;
```

### Synchronization, Error Scopes, and Mapping
- **Polling**:
  - Non-blocking tick: `device.poll(wgpu::PollType::Poll)?;`
  - Wait for specific submission with timeout:
    ```rust
    let submission = queue.submit(std::iter::once(encoder.finish()));
    device.poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: Some(std::time::Duration::from_secs(15)),
    })?;
    ```
- **Validation Error Scopes**:
  ```rust
  let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
  // create_shader_module / create_render_pipeline / surface.configure ...
  if let Some(error) = scope.pop().await {
      anyhow::bail!("GPU validation error: {error}");
  }
  ```
- **Buffer Mapping**:
  - `let view = buffer.slice(..).get_mapped_range()?;` (returns `Result<BufferView, MapRangeError>`).
  - Always `drop(view);` before `buffer.unmap();`.

### Dependency Pinning Notes for Root `Cargo.toml`
When `vendor/wgpu` is wired into root `Cargo.toml`:
- `wgpu = { path = "vendor/wgpu/wgpu", default-features = false }`
- `naga = { path = "vendor/wgpu/naga", default-features = false, features = ["wgsl-in"] }`
- `wgpu-hal` expects `windows = "0.62"`, `windows-core = "0.62"`, `windows-result = "0.4"`, and `gpu-allocator = "0.28"` (`features = ["hashbrown", "d3d12", "vulkan"]`), all of which already match `aphrody`'s `Cargo.lock`.
