---
name: webgpu
description: >-
  Authoritative WebGPU & WGSL engineering skill for aphrody. Use whenever authoring,
  validating, transpiling, or executing WebGPU WGSL shaders, designing GPUI/NSH shader
  pipelines, inspecting Dawn/Tint vs Naga differences, configuring WebGL2 fallbacks,
  verifying @webgpu/types with Bun TypeScript, or running GPU validation/headless compute with gpui-wgpu.
---

# WebGPU & WGSL Architecture (`aphrody`)

This skill governs all WebGPU, WGSL, Naga IR, Dawn/Tint, and browser/native GPU execution across `aphrody`. It covers the full graphics and compute pipeline across native desktop (`gpui_wgpu`, `nsh-engine`, `aphrody_cli`) and web/WASM targets (`gpui_web`, Bun/WebGPU TypeScript).

## Quick Reference Map

| Domain / Architecture | Reference Document | Key Content |
| :--- | :--- | :--- |
| **W3C WebGPU & WGSL Standards** | [references/webgpu-and-wgsl-spec.md](references/webgpu-and-wgsl-spec.md) | Object hierarchy (`GPUAdapter`, `GPUDevice`, `GPUQueue`), address spaces (`uniform`, `storage`, `workgroup`), host-sharing alignment (16-byte rules, `vec3` padding, `COPY_BYTES_PER_ROW_ALIGNMENT = 256`), uniformity analysis, dual-source blending, subgroups, WebGL2 fallback |
| **GPUI WGPU & Web Rendering Pipeline** | [references/gpui-wgpu-and-web-pipeline.md](references/gpui-wgpu-and-web-pipeline.md) | GPUI's 9 render pipelines (`quads`, `opaque_quads`, `shadows`, `path_tiles`, `underlines`, `mono_sprites`, `subpixel_sprites`, `poly_sprites`, `surfaces`), reverse-Z depth projection, Oklab gradient interpolation, subpixel dual-source glyph rendering, `WgpuHeadlessRenderer`, `gpui_web` canvas integration |
| **NSH Custom Shader Engine** | [references/nsh-shader-architecture.md](references/nsh-shader-architecture.md) | User `fn shade(uv: vec2f) -> vec4f` authoring model, Naga validation & AST wrapping, builtins uniform struct, double-buffered `MAP_READ` staging buffers, macOS `IOSurface` zero-copy presentation, GLSL ES 300 transpilation |
| **Dawn & Cross-Backend Compilation Matrix** | [references/dawn-and-cross-backend-matrix.md](references/dawn-and-cross-backend-matrix.md) | Google Dawn/Tint vs gfx-rs Naga/wgpu-hal differences, Vulkan / DX12 / Metal / WebGL2 transpilation rules, Chromium/Chrome headless flags (`--enable-unsafe-webgpu`, `--use-angle=vulkan`), CLI tooling matrix (`gpui-wgpu`, `naga`, `tint`, `wgsl-analyzer`) |

## Tooling & Dynamic Discovery

All tools are resolved cross-platform via `PATH` or `bun examples/gpu/verify.ts webgpu` (which reads binary and crate names from `product.toml`):

1. **`gpui-wgpu`** (`gpui-wgpu` on `PATH` or `target/{debug,release}/gpui-wgpu`):
   Unified WGPU & Naga CLI for aphrody (`vendor/wgpu`).
   Commands: `adapters`, `validate`, `transpile`, `render`, `compute`, `bench`.
2. **`naga`** (`naga` on `PATH`):
   Naga CLI built from `vendor/wgpu/naga-cli`. Directly validates WGSL and emits WGSL, SPIR-V, HLSL, MSL, and GLSL.
3. **`tint` & `tint_info`** (`tint`, `tint_info` on `PATH`):
   Google Dawn reference WGSL compiler and validator for official W3C WGSL semantics and AST layout inspection.
4. **`wgsl-analyzer`** (`wgsl-analyzer` on `PATH`):
   Language server and AST diagnostic engine.
5. **`bun` & `@webgpu/types`**:
   TypeScript typechecking (`bun check`) and zero-overhead verification in `examples/gpu/webgpu/` and `examples/gpu/verify.ts`.

---

## Core Invariants & Engineering Directives

### 1. Host-Shareable Struct Alignment & Stride Rules
WGSL defines strict layout rules for host-shareable structures (`var<uniform>` and `var<storage>`):
- **Scalar alignment**: `f32`, `i32`, `u32` require 4-byte alignment; `f16` requires 2-byte alignment.
- **Vector alignment**: `vec2<T>` requires $2 \times \text{align}(T)$ (8 bytes for `vec2f`); `vec3<T>` and `vec4<T>` require $4 \times \text{align}(T)$ (**16 bytes** for `vec3f` and `vec4f`).
- **`vec3<f32>` Warning**: WGSL `vec3<f32>` has **alignment 16 and size 12**. Rust `[f32; 3]` has alignment 4 and size 12. Never place an unpadded `vec3f` in a `#[repr(C)]` struct shared with WGSL. Prefer `vec4f` or insert explicit padding fields (`_pad: f32`).
- **Uniform Array Stride**: Elements of arrays in `var<uniform>` must have a stride that is a multiple of **16 bytes** (`array<vec4f, N>`).
- **16-Byte Struct Padding**: Always pad uniform structs to a multiple of 16 bytes (`size % 16 == 0`). Devices without `BUFFER_BINDINGS_NOT_16_BYTE_ALIGNED` (common on WebGL2 and older mobile GPUs) will reject unpadded uniform bindings.

### 2. Texture-Buffer Copy 256-Byte Row Alignment
- When performing `copy_texture_to_buffer` or `copy_buffer_to_texture`, `bytes_per_row` **must be a multiple of 256** (`wgpu::COPY_BYTES_PER_ROW_ALIGNMENT` / `GPUBufferBindingLayout`).
- Formula for uncompressed formats (e.g., `Rgba8Unorm` with 4 bytes per pixel):
  $$\text{bytes\_per\_row} = \text{align\_up}(\text{width} \times 4, 256) = ((\text{width} \times 4 + 255) / 256) \times 256$$
- When copying rendered frames back to CPU memory, the buffer contains row padding if `width * 4` is not a multiple of 256. CPU readers must read rows using `bytes_per_row` stride and discard trailing padding bytes per row.

### 3. Dual-Source Blending for Subpixel Text
- Subpixel text antialiasing requires independent RGB coverage factors per pixel channel.
- Requires WebGPU extension / WGPU feature: `DUAL_SOURCE_BLENDING`.
- WGSL syntax:
  ```wgsl
  enable dual_source_blending;

  struct FragmentOutput {
      @location(0) @blend_src(0) color: vec4<f32>,
      @location(0) @blend_src(1) alpha: vec4<f32>,
  }
  ```
- Blending pipeline configuration:
  $$\text{Color} = \text{Src} \times \text{Src1} + \text{Dst} \times (1 - \text{Src1})$$
  ```rust
  wgpu::BlendComponent {
      src_factor: wgpu::BlendFactor::Src1,
      dst_factor: wgpu::BlendFactor::OneMinusSrc1,
      operation: wgpu::BlendOperation::Add,
  }
  ```
- If the GPU adapter lacks `DUAL_SOURCE_BLENDING` (e.g. standard WebGL2), fallback to monochrome alpha sprites (`mono_sprites`) with grayscale antialiasing.

### 4. Reverse-Z Depth Projection
- GPUI uses reverse-Z depth mapping:
  - Near plane = `1.0`, Far plane = `0.0`.
  - Depth comparison function: `wgpu::CompareFunction::GreaterEqual`.
  - Clear depth: `0.0`.
- Provides superior floating-point precision distribution near UI elements, avoiding Z-fighting across thousands of stacked layers and subpixel quads.

### 5. NSH Custom Shader Contract
- User custom WGSL files implement:
  ```wgsl
  fn shade(uv: vec2f) -> vec4f { ... }
  ```
- Coordinate system: `uv` is normalized $(0, 0)$ top-left to $(1, 1)$ bottom-right (CSS style).
- Color output: straight sRGB $(r, g, b, a)$ with straight alpha (not premultiplied; the fragment wrapper handles premultiplication).
- Builtin uniform struct:
  ```wgsl
  struct Nsh {
      size: vec2f,
      time: f32,
      scale: f32,
      progress: f32,
      @size(12) overflow: f32,
  }
  @group(0) @binding(0) var<uniform> nsh: Nsh;
  ```
- Double-buffered readback staging ensures the GPU renders without stalling CPU execution frames.

---

## Tooling Usage Guide

Run the end-to-end WebGPU verification suite cross-platform with:
```bash
bun examples/gpu/verify.ts webgpu
```

### Shader Validation & Reflection
```bash
# Validate WGSL with Naga and emit JSON reflection
gpui-wgpu validate path/to/shader.wgsl --json

# Validate WGSL with Naga CLI
naga path/to/shader.wgsl

# Validate WGSL with Google Dawn Tint
tint --parse-only true path/to/shader.wgsl
```

### Multi-Target Cross-Compilation
```bash
# Cross-compile to SPIR-V, HLSL, MSL, and GLSL
gpui-wgpu transpile path/to/shader.wgsl --target spv -o shader.spv
gpui-wgpu transpile path/to/shader.wgsl --target hlsl -o shader.hlsl
gpui-wgpu transpile path/to/shader.wgsl --target msl -o shader.metal
gpui-wgpu transpile path/to/shader.wgsl --target webgl2 -o shader.essl
```

### Headless Rendering & Compute
```bash
# Render fullscreen shader pass to PNG
gpui-wgpu render path/to/shader.wgsl --output render.png --width 800 --height 600

# Dispatch compute shader and read back storage buffer
gpui-wgpu compute path/to/compute.wgsl --dispatch 64,1,1 --buffer-size 1024
```
