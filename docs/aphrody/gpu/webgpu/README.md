# WebGPU & WGSL Architecture in aphrody

This directory documents the WebGPU, WGSL, and cross-backend GPU rendering and compute architecture within `aphrody`.

---

## Architecture Overview

`aphrody` features a unified GPU foundation built upon `gfx-rs/wgpu` v30.0.0 and `naga` v30.0.0 (vendored in `vendor/wgpu` at revision `cddb2f5d80dc0009b2ef6767635140cacf90a007`).

The GPU stack spans three key domains:
1. **Desktop UI Rendering (`crates/gpui_wgpu`)**:
   - 9 specialized render pipelines (`opaque_quads`, `quads`, `shadows`, `path_tiles`, `underlines`, `mono_sprites`, `subpixel_sprites`, `poly_sprites`, `surfaces`).
   - Reverse-Z floating-point depth buffer (`Depth32Float`, clear=0.0, GreaterEqual compare).
   - Oklab perceptual gradient interpolation.
   - Dual-source blending (`dual_source_blending`) for subpixel font antialiasing.
   - Headless offscreen renderer (`WgpuHeadlessRenderer`) with 256-byte aligned texture readbacks.
2. **Browser Platform Backend (`crates/gpui_web`)**:
   - WebAssembly canvas integration (`HtmlCanvasElement`).
   - Native Browser WebGPU primary backend with automatic WebGL 2.0 (GLSL ES 3.00) fallback.
   - Dynamic instance streaming via Data Texture Fetch on WebGL2 where storage buffers are unavailable.
3. **Custom Shader Engine (`crates/nsh-core` & `crates/nsh-engine`)**:
   - Pure fragment function authoring: `fn shade(uv: vec2f) -> vec4f`.
   - Naga AST validation, size enforcement, and automatic wrapping with builtins (`nsh.size`, `nsh.time`, `nsh.scale`, `nsh.progress`, `nsh.overflow`).
   - Double-buffered asynchronous `MAP_READ` staging buffers for zero-stall CPU readbacks.
   - Apple `IOSurface` zero-copy presentation on macOS.

---

## Tooling & Command Line Interface

`aphrody` includes production command-line utilities for validating shaders, cross-compiling, inspecting GPU adapters, and running headless renders:

### 1. `gpui-wgpu` (`.\target\debug\gpui-wgpu.exe`)
```powershell
# Enumerate hardware adapters, capabilities, limits, and features
.\target\debug\gpui-wgpu.exe adapters --json

# Validate and reflect a WGSL shader with Naga
.\target\debug\gpui-wgpu.exe validate path\to\shader.wgsl --json

# Transpile to target shading languages
.\target\debug\gpui-wgpu.exe transpile path\to\shader.wgsl --target spv -o out.spv
.\target\debug\gpui-wgpu.exe transpile path\to\shader.wgsl --target hlsl -o out.hlsl
.\target\debug\gpui-wgpu.exe transpile path\to\shader.wgsl --target msl -o out.metal
.\target\debug\gpui-wgpu.exe transpile path\to\shader.wgsl --target webgl2 -o out.essl

# Headless offscreen render pass to PNG
.\target\debug\gpui-wgpu.exe render path\to\shader.wgsl --output render.png --width 800 --height 600

# Dispatch compute shader and verify storage buffer output
.\target\debug\gpui-wgpu.exe compute path\to\compute.wgsl --dispatch 64,1,1 --buffer-size 1024
```

### 2. Upstream Validator CLIs
- **`naga`** (`~/.local/bin\naga.exe`): Naga v30.0.0 CLI built from `vendor/wgpu/naga-cli`.
- **`tint`** (`~/.local/bin\tint.exe`): Google Dawn reference compiler (release `v20261113.214538`).
- **`wgsl-analyzer`** (`~/.local/bin\wgsl-analyzer.exe`): Standalone language server (v0.11.538).
- **`bun` & `@webgpu/types`**: Typechecked WebGPU scripting in `examples/gpu/webgpu/`.

---

## Normative Guidelines

1. **Host-Sharing Alignment**:
   - `vec3<f32>` has **alignment 16 and size 12**. Always pad or use `vec4<f32>` in Rust `bytemuck` structs.
   - All `var<uniform>` structures must be padded to a multiple of **16 bytes**.
   - Texture copy row strides must align to **256 bytes** (`COPY_BYTES_PER_ROW_ALIGNMENT`).
2. **Derivative Uniformity**:
   - `dpdx`, `dpdy`, and implicit LOD texture sampling must only occur in uniform control flow. Never place inside conditional discard blocks.
3. **Dual-Source Blending**:
   - Output 0 writes foreground color with `@blend_src(0)`.
   - Output 1 writes per-channel alpha coverage with `@blend_src(1)`.
   - Pipeline uses `src_factor: Src1` and `dst_factor: OneMinusSrc1`.

---

## Detailed References

For exhaustive deep-dives, consult the skill references:
- [W3C WebGPU & WGSL Specification Reference](../../../../.agents/plugins/aphrody/skills/webgpu/references/webgpu-and-wgsl-spec.md)
- [GPUI WGPU & Web Rendering Pipeline Architecture](../../../../.agents/plugins/aphrody/skills/webgpu/references/gpui-wgpu-and-web-pipeline.md)
- [NSH Custom Shader Architecture](../../../../.agents/plugins/aphrody/skills/webgpu/references/nsh-shader-architecture.md)
- [Dawn & Cross-Backend Compilation Matrix](../../../../.agents/plugins/aphrody/skills/webgpu/references/dawn-and-cross-backend-matrix.md)
