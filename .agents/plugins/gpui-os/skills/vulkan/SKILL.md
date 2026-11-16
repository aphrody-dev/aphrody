---
name: vulkan
description: >-
  Architect, build, debug, validate, compile, and benchmark Vulkan 1.3/1.4 and SPIR-V
  graphics and compute infrastructure in gpui-os. Use whenever working on Vulkan adapters,
  Synchronization2, Dynamic Rendering, Timeline Semaphores, wgpu-hal Vulkan backend (ash 0.38 & gpu-allocator 0.28),
  Khronos Validation Layers, glslangValidator/glslc/spirv-val/spirv-opt shader pipelines,
  Naga SPIR-V backend translation, or negative viewport height Y-flipping.
---

# Vulkan Architecture & Toolchain (`gpui-os`)

Vulkan is the high-performance, cross-platform graphics and compute API powering `gpui-os` across Windows, Linux, and Android. Within `gpui-os`, Vulkan drives offscreen compute pipelines, low-latency UI rendering via `crates/gpui_wgpu`, and hardware-accelerated command dispatch via the `vendor/wgpu/wgpu-hal/src/vulkan` backend built on `ash` 0.38 and `gpu-allocator` 0.28.

This skill provides normative architectural standards, verified toolchains, shader compilation pipelines (GLSL/WGSL to SPIR-V), validation layer configurations, and debugging workflows.

---

## Verified Toolchains & Paths

All tools are installed and verified in user-mode with zero elevation / UAC overhead:

| Tool / Component | Version | Verified Path |
| :--- | :--- | :--- |
| **LunarG Vulkan SDK** | `1.4.363.1` (Instance 1.4.363) | `~/.local/sdk\vulkan\`<br>`C:\VulkanSDK\1.4.363.0` (junction) |
| **glslangValidator** | `11:16.6.0` (Khronos Tool ID 8) | `~/.local/bin\glslangValidator.exe`<br>`~/.local/sdk\vulkan\Bin\glslangValidator.exe` |
| **glslc (shaderc)** | `v2026.4` (spirv-tools v2026.4) | `~/.local/bin\glslc.exe`<br>`~/.local/sdk\vulkan\Bin\glslc.exe` |
| **spirv-val** | `v2026.4` (SPIR-V 1.0–1.6, Vulkan 1.4) | `~/.local/bin\spirv-val.exe`<br>`~/.local/sdk\vulkan\Bin\spirv-val.exe` |
| **spirv-opt** | `v2026.4` (Optimization passes) | `~/.local/bin\spirv-opt.exe`<br>`~/.local/sdk\vulkan\Bin\spirv-opt.exe` |
| **spirv-dis / spirv-as** | `v2026.4` (Disassembler / Assembler) | `~/.local/bin\spirv-dis.exe`, `spirv-as.exe` |
| **spirv-cross** | `v2026.4` (Reflection & cross-compile) | `~/.local/bin\spirv-cross.exe` |
| **Khronos Validation Layer** | `1.4.363` (`VkLayer_khronos_validation`) | `~/.local/sdk\vulkan\Bin\VkLayer_khronos_validation.dll`<br>`~/.local/sdk\vulkan\Bin\VkLayer_khronos_validation.json` |
| **vulkaninfo** | `1.4.357` (System) / `1.4.363` (SDK) | `C:\Windows\System32\vulkaninfo.exe`<br>`~/.local/sdk\vulkan\Bin\vulkaninfoSDK.exe` |
| **WGPU CLI (`gpui-wgpu`)** | `v30.0.0` (Vulkan adapter & Naga SPV) | `.\target\debug\gpui-wgpu.exe` |

---

## Technical References

Read the corresponding reference guide in `references/` before writing or modifying Vulkan or SPIR-V code:

| Topic | Reference Document | Key Concepts Covered |
| :--- | :--- | :--- |
| **Vulkan Architecture** | [vulkan-architecture.md](references/vulkan-architecture.md) | Vulkan 1.3/1.4 core standards, Synchronization2 (`vkCmdPipelineBarrier2`), Dynamic Rendering (`vkCmdBeginRendering`), Timeline Semaphores, Push Constants (128–256 B) vs Descriptor Indexing, RTX 4070 Ada Lovelace capabilities |
| **SPIR-V & GLSL Pipeline** | [spirv-and-glsl-pipeline.md](references/spirv-and-glsl-pipeline.md) | `glslc` and `glslangValidator` CLI syntax, `spirv-val` / `spirv-opt` flags, Naga `back::spv` and `front::spv`, coordinate systems & viewport negative height (`height = -height`) Y-flipping rules |
| **wgpu-hal Vulkan Backend** | [wgpu-vulkan-hal.md](references/wgpu-vulkan-hal.md) | `vendor/wgpu/wgpu-hal/src/vulkan` mapping to `ash` 0.38 & `gpu-allocator` 0.28, swapchain surface lifecycle on Win32/X11/Wayland/Android, validation layer setup (`VK_LAYER_PATH`), device lost recovery |

---

## Core Vulkan Principles in `gpui-os`

1. **Hardware Baseline (NVIDIA GeForce RTX 4070)**:
   - Discrete GPU: `vendorID = 0x10de`, `deviceID = 0x2786`, Driver `621.14`.
   - Conformance: Vulkan 1.4 (`1.4.3.3`), Instance 1.4.363, Device API version 1.4.357.
   - Subgroup size: 32 threads with full ballot, shuffle, arithmetic, and quad voting support.
   - Downlevel / Fallback: Mesa Dozen Direct3D12 interop (`GPU1` hardware D3D12, `GPU2` WARP software rasterizer).

2. **Synchronization2 (`VK_KHR_synchronization2` / Vulkan 1.3 Core)**:
   - Modern Vulkan eliminates coarse `VkPipelineStageFlags` and 32-bit access masks.
   - Use `VkDependencyInfo`, `VkMemoryBarrier2`, `VkBufferMemoryBarrier2`, and `VkImageMemoryBarrier2` with 64-bit stage/access masks to avoid pipeline stalls and prevent driver false sharing.

3. **Dynamic Rendering vs Legacy RenderPasses**:
   - For UI rendering and lightweight composition passes, bypass `VkRenderPass` and `VkFramebuffer` allocations using `vkCmdBeginRendering` with `VkRenderingInfo`.
   - `wgpu-hal` automatically synthesizes dynamic rendering when supported or caches lightweight compatibility render passes on downlevel hardware.

4. **Viewport Coordinate Space & Y-Flipping**:
   - Vulkan NDC puts Y = 0 at screen center with Y pointing downwards (+1 at bottom), while WebGPU, Metal, and DirectX have Y pointing upwards.
   - `gpui-os` and `wgpu-hal` achieve parity via Vulkan 1.1+ negative viewport height:
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
   - For debugging, ensure `VK_LAYER_PATH=~/.local/sdk\vulkan\Bin` is set and validation is toggled via `WGPU_VALIDATION=1`.
