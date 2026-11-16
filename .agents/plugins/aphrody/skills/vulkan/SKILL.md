---
name: vulkan
description: >-
  Architect, build, debug, validate, compile, and benchmark Vulkan 1.3/1.4 and SPIR-V
  graphics and compute infrastructure in aphrody. Use whenever working on Vulkan adapters,
  Synchronization2, Dynamic Rendering, Timeline Semaphores, wgpu-hal Vulkan backend (ash 0.38 & gpu-allocator 0.28),
  Khronos Validation Layers, glslangValidator/glslc/spirv-val/spirv-opt shader pipelines,
  Naga SPIR-V backend translation, or negative viewport height Y-flipping.
---

# Vulkan Architecture & Toolchain (`aphrody`)

Vulkan is the high-performance, cross-platform graphics and compute API powering `aphrody` across Windows, Linux, and Android. Within `aphrody`, Vulkan drives offscreen compute pipelines, low-latency UI rendering via `crates/gpui_wgpu`, and hardware-accelerated command dispatch via the `vendor/wgpu/wgpu-hal/src/vulkan` backend built on `ash` 0.38 and `gpu-allocator` 0.28.

This skill provides normative architectural standards, verified toolchains, shader compilation pipelines (GLSL/WGSL to SPIR-V), validation layer configurations, and debugging workflows.

---

## Toolchain & Dynamic Discovery

All tools are resolved automatically across platforms by `bun examples/gpu/verify.ts vulkan` via `PATH`, `VULKAN_SDK`, or `<SLUG>_SDK_ROOT`:

| Tool / Component | Discovery Pattern | Role |
| :--- | :--- | :--- |
| **LunarG Vulkan SDK** | `$VULKAN_SDK` or `<SLUG>_SDK_ROOT` (`vkcube`, `vulkaninfo`) | Headers, loaders, and validation layers |
| **glslangValidator** | `glslangValidator` on `PATH` / `$VULKAN_SDK/bin` | Strict Khronos GLSL/HLSL front-end to SPIR-V |
| **glslc (shaderc)** | `glslc` on `PATH` / `$VULKAN_SDK/bin` | Google compiler driver + preprocessor |
| **spirv-val / spirv-opt** | `spirv-val`, `spirv-opt` on `PATH` / `$VULKAN_SDK/bin` | SPIR-V validation and optimization passes |
| **spirv-dis / spirv-as / spirv-cross** | `spirv-dis`, `spirv-as`, `spirv-cross` on `PATH` | Disassembly, assembly, and reflection |
| **Khronos Validation Layer** | `VK_LAYER_PATH` -> `$VULKAN_SDK/bin` (`VkLayer_khronos_validation`) | Runtime Vulkan validation (`WGPU_VALIDATION=1`) |
| **vulkaninfo** | `vulkaninfo` on `PATH` or `vulkaninfoSDK` in `$VULKAN_SDK/bin` | Adapter, extensions, and subgroup queries |
| **WGPU CLI (`gpui-wgpu`)** | `gpui-wgpu` on `PATH` or `target/{debug,release}/gpui-wgpu` | Vulkan adapter probe & Naga SPV transpile |

---

## Technical References

Read the corresponding reference guide in `references/` before writing or modifying Vulkan or SPIR-V code:

| Topic | Reference Document | Key Concepts Covered |
| :--- | :--- | :--- |
| **Vulkan Architecture** | [vulkan-architecture.md](references/vulkan-architecture.md) | Vulkan 1.3/1.4 core standards, Synchronization2 (`vkCmdPipelineBarrier2`), Dynamic Rendering (`vkCmdBeginRendering`), Timeline Semaphores, Push Constants vs Descriptor Indexing, subgroup capabilities |
| **SPIR-V & GLSL Pipeline** | [spirv-and-glsl-pipeline.md](references/spirv-and-glsl-pipeline.md) | `glslc` and `glslangValidator` CLI syntax, `spirv-val` / `spirv-opt` flags, Naga `back::spv` and `front::spv`, coordinate systems & viewport negative height (`height = -height`) Y-flipping rules |
| **wgpu-hal Vulkan Backend** | [wgpu-vulkan-hal.md](references/wgpu-vulkan-hal.md) | `vendor/wgpu/wgpu-hal/src/vulkan` mapping to `ash` & `gpu-allocator`, swapchain surface lifecycle on Win32/X11/Wayland/Android, validation layer setup (`VK_LAYER_PATH`), device lost recovery |

---

## Core Vulkan Principles in `aphrody`

1. **Adapter & Capability Discovery**:
   - Query active adapters and subgroup capabilities at runtime via `gpui-wgpu adapters --backends vulkan` (discrete GPUs such as NVIDIA/AMD/Intel, integrated GPUs, or software fallbacks such as Mesa Dozen D3D12 / SwiftShader / WARP).
   - Detect `subgroup_min_size`/`subgroup_max_size` dynamically (e.g., 32 on NVIDIA/AMD Wave32, 8–32 on Intel, 64 on AMD Wave64) instead of assuming a single GPU vendor.

2. **Synchronization2 (`VK_KHR_synchronization2` / Vulkan 1.3 Core)**:
   - Modern Vulkan eliminates coarse `VkPipelineStageFlags` and 32-bit access masks.
   - Use `VkDependencyInfo`, `VkMemoryBarrier2`, `VkBufferMemoryBarrier2`, and `VkImageMemoryBarrier2` with 64-bit stage/access masks to avoid pipeline stalls and prevent driver false sharing.

3. **Dynamic Rendering vs Legacy RenderPasses**:
   - For UI rendering and lightweight composition passes, bypass `VkRenderPass` and `VkFramebuffer` allocations using `vkCmdBeginRendering` with `VkRenderingInfo`.
   - `wgpu-hal` automatically synthesizes dynamic rendering when supported or caches lightweight compatibility render passes on downlevel hardware.

4. **Viewport Coordinate Space & Y-Flipping**:
   - Vulkan NDC puts Y = 0 at screen center with Y pointing downwards (+1 at bottom), while WebGPU, Metal, and DirectX have Y pointing upwards.
   - `aphrody` and `wgpu-hal` achieve parity via Vulkan 1.1+ negative viewport height:
     ```rust
     vk::Viewport {
         x: 0.0,
         y: height as f32,
         width: width as f32,
         height: -(height as f32),
         min_depth: 0.0,
         max_depth: 1.0,
     }
     ```
   - When compiling standalone SPIR-V through Naga, `WriterFlags::ADJUST_COORDINATE_SPACE` negates `Position.y` before emitting the vertex stage output.

5. **Validation Layer Hygiene**:
   - Never run GPU validation in release or benchmark builds.
   - For debugging, ensure `VK_LAYER_PATH` points to your Vulkan SDK layer directory (e.g. `$VULKAN_SDK/bin`) and toggle validation via `WGPU_VALIDATION=1`.
