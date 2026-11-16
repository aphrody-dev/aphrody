---
name: directx
description: >-
  Architect, build, debug, validate, compile, and profile DirectX 12 (D3D12) and HLSL
  rendering and compute infrastructure in gpui-os. Use whenever working on D3D12
  swapchains, Enhanced Barriers, Root Signatures, Descriptor Heaps, Agility SDK integration,
  DXC SM 6.0–6.9 DXIL compilation, Naga HLSL backend translation, gpui_windows Direct2D/DirectWrite/D3D11/DX12
  interop, DirectComposition, or PIX GPU event profiling on Windows.
---

# DirectX 12 & HLSL Architecture (`gpui-os`)

DirectX 12 is the primary low-level graphics and compute API on Windows for `gpui-os`. It powers both the native Windows windowing and composition engine in `crates/gpui_windows` and the hardware-accelerated WGPU backend via `vendor/wgpu/wgpu-hal/src/dx12`.

This skill provides normative architectural standards, compilation toolchains, Agility SDK configuration, shader model guidelines (SM 5.0 through SM 6.9), and profiling workflows.

## Verified Toolchains & Paths

| Tool / SDK | Version | Verified Path |
| :--- | :--- | :--- |
| **DirectX Shader Compiler (DXC)** | `v1.9.2609.5` (DXIL, SM 6.0–6.9) | `~/.local/bin\dxc.exe`<br>`~/.local/sdk\directx\dxc\bin\x64\dxc.exe` |
| **Legacy FXC** | `10.1` (SM 5.0 / 5.1 DXBC) | `C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\fxc.exe` |
| **Agility SDK (D3D12)** | `1.722.1-preview` & `1.620.2` | `~/.local/sdk\directx\agility\bin\x64\D3D12Core.dll`<br>`~/.local/bin\d3dconfig.exe` |
| **DirectX-Headers** | `v1.620.2` (`d3d12.h`, `d3dx12.h`) | `~/.local/sdk\directx\headers\include\directx\` |
| **WinPixEventRuntime** | `1.0.240308001` (PIX Events) | `~/.local/sdk\directx\pix\bin\x64\WinPixEventRuntime.dll` |
| **WGPU CLI (`gpui-wgpu`)** | v30.0.0 (DX12 probe/transpile) | `.\target\debug\gpui-wgpu.exe` |

---

## Technical References

Read the corresponding reference guide in `references/` before writing or modifying DirectX 12 code:

| Topic | Reference Document | Key Concepts Covered |
| :--- | :--- | :--- |
| **D3D12 Architecture** | [d3d12-architecture.md](references/d3d12-architecture.md) | Enhanced Barriers (`ID3D12GraphicsCommandList7`), Root Signatures, Descriptor Heaps, 2048-Sampler limit workaround, Agility SDK `D3D12SDKVersion` exports, `DXGI_SWAP_EFFECT_FLIP_DISCARD`, RTX 4070 Feature Level 12_2 Ultimate |
| **HLSL, DXC & Naga** | [hlsl-dxc-and-naga.md](references/hlsl-dxc-and-naga.md) | DXC compiler flags (`-T cs_6_6 -E main`), DXIL vs legacy DXBC, Wave Intrinsics (`WaveActiveSum`, `WavePrefixSum`), Naga HLSL backend (`naga::back::hlsl`), CBV 16-byte struct / 256-byte buffer alignment |
| **GPUI & WGPU DX12** | [gpui-and-wgpu-dx12.md](references/gpui-and-wgpu-dx12.md) | `gpui_windows` D2D/DirectWrite/D3D11/DX12 interop, `wgpu-hal` DX12 backend, DirectComposition transparency, hybrid GPU selection (NVIDIA Optimus / AMD Enduro), DRED crash breadcrumbs |

---

## Core DirectX 12 Principles in `gpui-os`

1. **Hardware Baseline (RTX 4070 & Direct3D Feature Level 12_2)**:
   - Primary target: NVIDIA GeForce RTX 4070 (Ada Lovelace, sm_89, Feature Level 12_2 Ultimate).
   - Supported features: Raytracing Tier 1.1, Mesh Shaders, Variable Rate Shading Tier 2, Sampler Feedback, Work Graphs, Shader Model 6.6–6.9.
   - Fallback adapter: Microsoft Basic Render Driver (`WARP`), vendor `0x1414`, device `0x008c`, CPU software rasterizer with full SM 6.6 capabilities.

2. **Agility SDK Dynamic Linkage**:
   - In modern D3D12 applications, OS-level `d3d12.dll` acts only as a loader. The application exports `D3D12SDKVersion` (e.g. `722` or `620`) and `D3D12SDKPath` (e.g. `.\D3D12\`) to dynamically load `D3D12Core.dll`.
   - Never rely on static OS runtime features when Agility SDK preview features (such as Work Graphs or Enhanced Barriers) are required.

3. **Descriptor Heap Architecture & 2048 Sampler Limit**:
   - D3D12 hard-limits visible sampler heaps to 2048 samplers across all hardware.
   - WGPU and GPUI bypass this limit by binding the entire global sampler heap as standard and comparison samplers, routing accesses through a dynamic sampler index buffer per bind group.

4. **Enhanced Barriers vs Legacy Resource Barriers**:
   - Enhanced Barriers (`D3D12_BARRIER_GROUP`) replace monolithic `ResourceBarrier` calls with separated Buffer, Texture, and Global memory barriers, preventing pipeline bubbles and unnecessary cache flushes.

5. **HLSL Compilation Rule**:
   - Modern compute and rendering passes use DXC (`dxc.exe` v1.9) with `-T <stage>_6_6` (or higher) to generate signed DXIL.
   - Legacy `fxc.exe` is strictly reserved for legacy D3D11 pipelines and Shader Model 5.0/5.1 compatibility checks.

6. **Error Handling & Code Hygiene (`.rules`)**:
   - Never call `.unwrap()` or discard fallible Results with `let _ =` in Rust DirectX examples or modules. Use `anyhow::Result` with `?` and descriptive `.context(...)`.
   - Never run `cargo` commands directly (the coordinator handles compilation); use `dxc.exe`, `fxc.exe`, and `gpui-wgpu.exe` for local verification.
