# GPU Architecture & Examples (`gpui-os` × `wgpu` v30.0.0)

Central index for multi-backend GPU rendering, shader validation, hardware IHV architectures, and browser tooling in `gpui-os`.

## Graphics Backends & Standards

| Backend | Docs | Unified Plugin Skill | Rust Examples (`crates/wgpu_cli/examples/`) | Shader & Tooling Examples (`examples/gpu/`) |
| --- | --- | --- | --- | --- |
| **WGPU & Naga v30** | [`docs/wgpu/llms.txt`](../wgpu/llms.txt), [`llms-full.txt`](../wgpu/llms-full.txt) | [`wgpu`](../../.agents/plugins/gpui-os/skills/wgpu/SKILL.md) | `gpui-wgpu` CLI (`crates/wgpu_cli`) | `crates/gpui_wgpu`, `crates/nsh-core/src/shader.rs` |
| **Vulkan & SPIR-V** | [`docs/gpu/vulkan/README.md`](./vulkan/README.md) | [`vulkan`](../../.agents/plugins/gpui-os/skills/vulkan/SKILL.md) | `vulkan_backend_probe` | [`examples/gpu/vulkan/`](../../examples/gpu/vulkan) |
| **DirectX 12 & HLSL** | [`docs/gpu/directx/README.md`](./directx/README.md) | [`directx`](../../.agents/plugins/gpui-os/skills/directx/SKILL.md) | `directx_backend_probe` | [`examples/gpu/directx/`](../../examples/gpu/directx) |
| **WebGPU & WGSL** | [`docs/gpu/webgpu/README.md`](./webgpu/README.md) | [`webgpu`](../../.agents/plugins/gpui-os/skills/webgpu/SKILL.md) | `webgpu_naga_pipeline` | [`examples/gpu/webgpu/`](../../examples/gpu/webgpu) |

## Hardware IHV Architectures & Accelerators

| Vendor / Architecture | Docs | Unified Plugin Skill | Key Hardware Capabilities | Examples (`examples/gpu/`) |
| --- | --- | --- | --- | --- |
| **NVIDIA (0x10DE)** | [`docs/gpu/nvidia/README.md`](./nvidia/README.md) | [`nvidia`](../../.agents/plugins/gpui-os/skills/nvidia/SKILL.md) | Ada Lovelace `sm_89`, 32-thread Warps, 4th Gen Tensor Cores, NVTX3, CUDA 13.x | [`examples/gpu/nvidia/`](../../examples/gpu/nvidia) |
| **AMD (0x1002 / 0x1022)** | [`docs/gpu/amd/README.md`](./amd/README.md) | [`amd`](../../.agents/plugins/gpui-os/skills/amd/SKILL.md) | RDNA 3/4, Dual-mode Wave32/Wave64, AGS SDK (`amd_ags_x64.dll`), RGA Analyzer | [`examples/gpu/amd/`](../../examples/gpu/amd) |
| **Intel (0x8086)** | [`docs/gpu/intel/README.md`](./intel/README.md) | [`intel`](../../.agents/plugins/gpui-os/skills/intel/SKILL.md) | Arc Xe/Xe2 (SIMD8/16/32), AVX2/AVX-512 SIMD, oneAPI Level Zero, ISPC | [`examples/gpu/intel/`](../../examples/gpu/intel) |

## Quick Commands

```powershell
# Inspect local hardware & software adapters (Vulkan, DX12, GL)
cargo run -p wgpu_cli -- adapters --limits --features

# Validate & cross-compile any WGSL shader
cargo run -p wgpu_cli -- validate crates/gpui_wgpu/src/shaders.wgsl
cargo run -p wgpu_cli -- transpile crates/gpui_wgpu/src/shaders.wgsl --target hlsl

# Run multi-vendor matrix probe
cargo run -p wgpu_cli --example ihv_vendor_matrix

# Run backend examples
cargo run -p wgpu_cli --example vulkan_backend_probe
cargo run -p wgpu_cli --example directx_backend_probe
cargo run -p wgpu_cli --example webgpu_naga_pipeline
```
