---
name: wgpu
description: >-
  Build, debug, validate, transpile, and benchmark WGPU v30.0.0 and Naga v30.0.0
  GPU rendering and compute code in aphrody. Use whenever writing or modifying
  WGSL shaders, Naga IR validation/cross-compilation (HLSL/MSL/SPIR-V/GLSL),
  crates/gpui_wgpu rendering pipelines or texture atlases, NSH shader views/effects/transitions
  (nsh-core/nsh-engine), vendor/wgpu internals (wgpu, wgpu-core, wgpu-hal, wgpu-types),
  or running GPU adapter diagnostics, headless renders, and compute tests with gpui-wgpu.
---

# WGPU & Naga (`aphrody` v30.0.0)

`aphrody` vendors `gfx-rs/wgpu` v30.0.0 (`cddb2f5d80dc0009b2ef6767635140cacf90a007`) and `naga` v30.0.0 in `vendor/wgpu`. Every GPU subsystem in the workspace — `crates/gpui_wgpu`, `crates/nsh-core`, `crates/nsh-engine`, and `crates/aphrody_cli` (`gpui-wgpu`) — shares this exact single revision.

## Read the Right Reference First

Resolve `references/` relative to `.agents/skills/wgpu/`:

| Topic | Reference File | When to Read |
| :--- | :--- | :--- |
| Vendored Crates, v30 APIs & `wgpu-hal` Backends | [references/architecture-and-crates.md](references/architecture-and-crates.md) | Creating `Instance`/`Adapter`/`Device`, using v30 API signatures (`PollType`, `CurrentSurfaceTexture`, `TexelCopy*`), debugging DX12/Vulkan/Metal/GLES backends, or modifying `vendor/wgpu` |
| Naga IR, WGSL Layouts & Cross-Compilation | [references/naga-and-wgsl.md](references/naga-and-wgsl.md) | Writing WGSL, checking `std140`/`std430` struct alignments with `Layouter`, validating shaders, handling uniformity rules, or transpiling to HLSL/MSL/SPIR-V/GLSL |
| GPUI & NSH GPU Integration | [references/gpui-and-nsh-integration.md](references/gpui-and-nsh-integration.md) | Editing `crates/gpui_wgpu` (pipelines, `WgpuAtlas`, dual-source subpixel blending, WebGL2 fallback, headless rendering, device recovery) or `nsh-core`/`nsh-engine` custom shaders |
| Full Exhaustive Spec | `aphrody/docs/wgpu/llms-full.txt` | Complete limits/features tables, full copy-paste Rust recipes, and troubleshooting catalog |

## Non-Negotiable Rules & Traps to Avoid

1. **Single Vendored Revision Invariant**:
   Never add a separate crates.io `wgpu` or `naga` version to any crate in `aphrody`. Always depend on `wgpu.workspace = true` and `naga.workspace = true` pointing to `vendor/wgpu`.
2. **Use `wgpu` v30.0.0 Exact Signatures (Do Not Use Stale v0.19–v24 APIs)**:
   - `wgpu::Instance::new(desc)` takes `InstanceDescriptor` **by value** (not `&desc`). Use `InstanceDescriptor::new_without_display_handle()`.
   - `instance.enumerate_adapters(backends).await` is **`async`**.
   - `instance.request_adapter(&options).await` returns **`Result<Adapter, RequestAdapterError>`** (not `Option<Adapter>`).
   - `DeviceDescriptor` includes `experimental_features: wgpu::ExperimentalFeatures::disabled()`, `memory_hints: wgpu::MemoryHints::MemoryUsage`, `trace: wgpu::Trace::Off`, `default_queue: wgpu::QueueDescriptor::default()`.
   - `PipelineLayoutDescriptor` uses `bind_group_layouts: &[Some(&bgl)]` (`&[Option<&BindGroupLayout>]`) and `immediate_size: 0` (replacing `push_constant_ranges`).
   - `DepthStencilState` wraps `depth_write_enabled: Some(bool)` and `depth_compare: Some(CompareFunction)` in `Option`.
   - `device.poll(wgpu::PollType::Wait { submission_index: Some(sub), timeout: Some(dur) })` or `wgpu::PollType::Poll` (replacing `Maintain::Wait` / `Maintain::Poll`).
   - `buffer_slice.get_mapped_range()` returns **`Result<BufferView, MapRangeError>`**, and you **must `drop(view)` before calling `buffer.unmap()`**.
   - Texture copy descriptors are `wgpu::TexelCopyTextureInfo`, `wgpu::TexelCopyBufferInfo`, and `wgpu::TexelCopyBufferLayout` (replacing `ImageCopy*`).
   - `surface.get_current_texture()` returns **`wgpu::CurrentSurfaceTexture`** (`Success`, `Suboptimal`, `Lost`, `Outdated`, `Timeout`, `Occluded`, `Validation`), and presentation uses `queue.present(frame)`! On `Suboptimal(frame)`, always `drop(frame)` before `surface.configure(...)`.
3. **256-Byte Row Alignment on Texture-Buffer Copies**:
   `CommandEncoder::copy_texture_to_buffer` and `copy_buffer_to_texture` require `bytes_per_row` to be a multiple of `wgpu::COPY_BYTES_PER_ROW_ALIGNMENT` (`256`) whenever `height > 1`. Always pad buffer row stride to `(width * bpp).next_multiple_of(256)` and strip row padding on CPU readback. (`Queue::write_texture` does NOT require 256-byte padding.)
4. **WGSL Host-Shareable Struct Alignment**:
   - `vec3<f32>` has **alignment 16, size 12** (unlike Rust `[f32; 3]` which has alignment 4!). Prefer `vec4<f32>` or explicit padding fields, and verify `#[repr(C)]` `Pod` structs with `gpui-wgpu validate`.
   - Arrays inside `var<uniform>` require element stride to be a multiple of **16 bytes** (`array<vec4<f32>, N>`, never `array<f32, N>`).
   - Pad `var<uniform>` structs to a multiple of **16 bytes** so GLES/WebGL2 backends lacking `BUFFER_BINDINGS_NOT_16_BYTE_ALIGNED` accept them.
5. **Memory Footprint Discipline for Desktop/UI Apps**:
   - Pass `memory_hints: wgpu::MemoryHints::MemoryUsage` on UI/shader `DeviceDescriptor`s (`Performance` suballocates 128 MB+ blocks).
   - When targeting DX12 with modest binding counts, cap `Limits { max_non_sampler_bindings: 65_536, .. }` unless 1M bindless descriptors are needed.
6. **Follow `.rules` Error Hygiene**:
   - Never call `.unwrap()` or discard fallible `Result`s with `let _ = ...`. Propagate with `?` and `.context(...)`, and wrap fallible GPU pipeline creation in `device.push_error_scope(wgpu::ErrorFilter::Validation)`.

## Fast Verification & Debugging Workflow (`gpui-wgpu`)

Use the workspace CLI crate `crates/aphrody_cli` (`gpui-wgpu`) to validate shaders, inspect layouts, test cross-compilation, and verify visual/compute output without spinning up a full UI window:

```sh
# 1. Inspect available hardware/software GPU adapters, PCI IDs, formats, limits & features
cargo run -p aphrody_cli -- adapters --limits --features

# 2. Validate WGSL syntax, types, uniformity, entry points, and exact byte offsets/alignments
cargo run -p aphrody_cli -- validate path/to/shader.wgsl

# 3. Cross-compile WGSL to HLSL, Metal MSL, GLSL (or WebGL2 ES 300), or SPIR-V
cargo run -p aphrody_cli -- transpile path/to/shader.wgsl --target hlsl
cargo run -p aphrody_cli -- transpile path/to/shader.wgsl --target msl
cargo run -p aphrody_cli -- transpile path/to/shader.wgsl --target glsl-webgl2 --stage fragment --entry fs_main

# 4. Render a fragment shader offscreen to a PNG for visual proof
cargo run -p aphrody_cli -- render path/to/shader.wgsl -o out.png --width 800 --height 600 --time 0.5

# 5. Execute a headless compute shader and inspect storage buffer output
cargo run -p aphrody_cli -- compute path/to/compute.wgsl --workgroups 4,1,1 --output-floats 32

# 6. Benchmark adapter init, upload bandwidth, compute dispatch, and offscreen render throughput
cargo run -p aphrody_cli -- bench --iterations 50 --size 1024
```

<!-- === APHRODY_AUTO_SKILLS_SYNC START === -->
## Upstream Snapshot (auto-generated `2026-11-18` via `bun tooling/bun/skills-sync.ts`)

- **crates.io** `wgpu`: `30.0.2` (updated 2026-11-12)
- **crates.io** `naga`: `30.0.2` (updated 2026-11-12)
- **GitHub** `gfx-rs/wgpu`: `v30.0.2` (2026-11-12)
<!-- === APHRODY_AUTO_SKILLS_SYNC END === -->
