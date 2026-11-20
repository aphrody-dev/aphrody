# Vulkan GPU Architecture & Tooling (`aphrody`)

This directory documents the Vulkan graphics and compute architecture, compilation toolchains, hardware capabilities, and developer workflows for `aphrody`.

---

## Architecture Overview

`aphrody` leverages Vulkan across three key operational layers:

1. **`gpui_wgpu`**: High-level UI surface rendering and glyph composition via WGPU's hardware abstraction layer.
2. **`vendor/wgpu/wgpu-hal/src/vulkan`**: Low-overhead Rust translation layer dynamically dispatching to the Vulkan driver via `ash` (`0.38`) and suballocating memory blocks with `gpu-allocator` (`0.28`).
3. **`crates/aphrody_cli` (`gpui-wgpu`)**: Hardware diagnostic, shader validation, cross-compilation, and offscreen compute benchmarking CLI.

```
+-----------------------------------------------------------+
|                   aphrody UI & Applications               |
+-----------------------------------------------------------+
                             |
+-----------------------------------------------------------+
|             gpui_wgpu (Texture Atlases & Passes)          |
+-----------------------------------------------------------+
                             |
+-----------------------------------------------------------+
|               wgpu & wgpu-core (v30.0.0)                  |
+-----------------------------------------------------------+
                             |
+-----------------------------------------------------------+
|         wgpu-hal (Vulkan Backend: ash 0.38)               |
+-----------------------------------------------------------+
                             |
+-----------------------------------------------------------+
|           Vulkan Driver / ICD (e.g. NVIDIA 621.14)        |
+-----------------------------------------------------------+
                             |
+-----------------------------------------------------------+
|         GPU Hardware (NVIDIA GeForce RTX 4070)            |
+-----------------------------------------------------------+
```

---

## Verified Toolchains & Paths

Installed in user-writable `~/.local/bin` and `~/.local/sdk\vulkan\`:

| Tool | Version | Purpose |
| :--- | :--- | :--- |
| **LunarG Vulkan SDK** | `1.4.363.1` (Instance 1.4.363) | Core headers, validation layers, runtime tools |
| **glslangValidator** | `11:16.6.0` (Khronos) | Reference GLSL/HLSL to SPIR-V compilation |
| **glslc** | `shaderc v2026.4` | Clang-style GLSL to SPIR-V compiler |
| **spirv-val** | `SPIRV-Tools v2026.4` | SPIR-V 1.0–1.6 bytecode validation |
| **spirv-opt** | `SPIRV-Tools v2026.4` | SPIR-V optimization passes |
| **spirv-dis** | `SPIRV-Tools v2026.4` | SPIR-V binary disassembler |
| **vulkaninfo** | `1.4.357` / `1.4.363` | Hardware capabilities and layer inspector |
| **gpui-wgpu** | `v30.0.0` | Naga shader transpiler and Vulkan runner |

---

## Shaders & Verification Examples

Production shader examples and end-to-end verification scripts are located in:
- `examples/gpu/vulkan/`
  - `wave_compute.comp`: High-performance compute kernel generating 2D wave interference patterns.
  - `triangle.vert`: Color-interpolated vertex shader with standard clip-space output.
  - `triangle.frag`: Fragment shader rendering interpolated vertex colors.
  - `quad_vulkan.wgsl`: Fullscreen quad vertex/fragment shader in WGSL.
  - `verify.ps1`: Automated shader compilation, validation, and transpilation test script.

---

## Reference Guides

Detailed technical specifications are maintained in `.agents/plugins/aphrody/skills/vulkan/references/`:
- [Vulkan Architecture](../../../../.agents/plugins/aphrody/skills/vulkan/references/vulkan-architecture.md): Synchronization2, Dynamic Rendering, Timeline Semaphores, Descriptor Indexing.
- [SPIR-V & GLSL Pipeline](../../../../.agents/plugins/aphrody/skills/vulkan/references/spirv-and-glsl-pipeline.md): Compilation commands, optimization flags, viewport Y-flipping (`height = -height`).
- [wgpu-hal Vulkan Backend](../../../../.agents/plugins/aphrody/skills/vulkan/references/wgpu-vulkan-hal.md): Ash 0.38 linkage, gpu-allocator memory heaps, swapchain presentation, RTX 4070 profile.
