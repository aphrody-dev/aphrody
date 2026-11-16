---
name: directx
description: >-
  Architect, build, debug, validate, compile, and profile DirectX 12 (D3D12) and HLSL
  rendering and compute infrastructure in aphrody. Use whenever working on D3D12
  swapchains, Enhanced Barriers, Root Signatures, Descriptor Heaps, Agility SDK integration,
  DXC SM 6.0–6.9 DXIL compilation, Naga HLSL backend translation, gpui_windows Direct2D/DirectWrite/D3D11/DX12
  interop, DirectComposition, or PIX GPU event profiling on Windows.
---

# DirectX 12 & HLSL Architecture (`aphrody`)

DirectX 12 is the primary low-level graphics and compute API on Windows for `aphrody`. It powers both the native Windows windowing and composition engine in `crates/gpui_windows` and the hardware-accelerated WGPU backend via `vendor/wgpu/wgpu-hal/src/dx12`.

This skill provides normative architectural standards, compilation toolchains, Agility SDK configuration, shader model guidelines (SM 5.0 through SM 6.9), and profiling workflows.

## Toolchain & Dynamic Discovery

All DirectX tools are resolved dynamically by `bun examples/gpu/verify.ts directx` via `PATH`, `<SLUG>_BIN_DIR`, `<SLUG>_SDK_ROOT`, or the newest installed Windows Kits bin directory:

| Tool / SDK | Discovery Pattern | Role |
| :--- | :--- | :--- |
| **DirectX Shader Compiler (DXC)** | `dxc` on `PATH` or `<SLUG>_SDK_ROOT/directx/dxc/bin` | DXIL compilation, SM 6.0–6.9 |
| **Legacy FXC** | `fxc` on `PATH` or latest Windows Kits `10/bin/<version>/<arch>/fxc` | Legacy SM 5.0 / 5.1 DXBC |
| **Agility SDK (D3D12)** | `<SLUG>_SDK_ROOT/directx/agility/bin` & `d3dconfig` on `PATH` | Modern D3D12 runtime + debug hooks |
| **DirectX-Headers** | `<SLUG>_SDK_ROOT/directx/headers/include/directx/` | `d3d12.h`, `d3dx12.h` |
| **WinPixEventRuntime** | `<SLUG>_SDK_ROOT/directx/pix/bin` | PIX GPU marker runtime |
| **WGPU CLI (`gpui-wgpu`)** | `gpui-wgpu` on `PATH` or `target/{debug,release}/gpui-wgpu` | DX12 probe & HLSL transpile |

---

## Technical References

Read the corresponding reference guide in `references/` before writing or modifying DirectX 12 code:

| Topic | Reference Document | Key Concepts Covered |
| :--- | :--- | :--- |
| **D3D12 Architecture** | [d3d12-architecture.md](references/d3d12-architecture.md) | Enhanced Barriers (`ID3D12GraphicsCommandList7`), Root Signatures, Descriptor Heaps, 2048-Sampler limit workaround, Agility SDK `D3D12SDKVersion` exports, `DXGI_SWAP_EFFECT_FLIP_DISCARD`, Feature Level 12_2 Ultimate |
| **HLSL, DXC & Naga** | [hlsl-dxc-and-naga.md](references/hlsl-dxc-and-naga.md) | DXC compiler flags (`-T cs_6_6 -E main`), DXIL vs legacy DXBC, Wave Intrinsics (`WaveActiveSum`, `WavePrefixSum`), Naga HLSL backend (`naga::back::hlsl`), CBV 16-byte struct / 256-byte buffer alignment |
| **GPUI & WGPU DX12** | [gpui-and-wgpu-dx12.md](references/gpui-and-wgpu-dx12.md) | `gpui_windows` D2D/DirectWrite/D3D11/DX12 interop, `wgpu-hal` DX12 backend, DirectComposition transparency, hybrid GPU selection (Optimus / Enduro), DRED crash breadcrumbs |

---

## Core DirectX 12 Principles in `aphrody`

1. **Hardware Feature Levels & Adapter Discovery**:
   - Inspect live D3D12 adapters via `gpui-wgpu adapters --backends dx12` to check Feature Level 12_2 Ultimate features (Raytracing Tier 1.1, Mesh Shaders, VRS Tier 2, Sampler Feedback, Work Graphs, SM 6.6+).
   - Fallback adapter: Microsoft Basic Render Driver (`WARP`), vendor `0x1414`, device `0x008c`, CPU software rasterizer with SM 6.6 capabilities.

2. **Agility SDK Dynamic Linkage**:
   - In modern D3D12 applications, OS-level `d3d12.dll` acts only as a loader. The application exports `D3D12SDKVersion` (e.g. `722` or `620`) and `D3D12SDKPath` (e.g. `.\D3D12\`) to dynamically load `D3D12Core.dll`.
   - Never rely on static OS runtime features when Agility SDK preview features (such as Work Graphs or Enhanced Barriers) are required.

3. **Descriptor Heap Architecture & 2048 Sampler Limit**:
   - D3D12 hard-limits visible sampler heaps to 2048 samplers across all hardware.
   - WGPU and GPUI bypass this limit by binding the entire global sampler heap as standard and comparison samplers, routing accesses through a dynamic sampler index buffer per bind group.

4. **Enhanced Barriers vs Legacy Resource Barriers**:
   - Enhanced Barriers (`D3D12_BARRIER_GROUP`) replace monolithic `ResourceBarrier` calls with separated Buffer, Texture, and Global memory barriers, preventing pipeline bubbles and unnecessary cache flushes.

5. **HLSL Compilation Rule**:
   - Modern compute and rendering passes use DXC (`dxc`) with `-T <stage>_6_6` (or higher) to generate signed DXIL.
   - Legacy `fxc` is strictly reserved for legacy D3D11 pipelines and Shader Model 5.0/5.1 compatibility checks.

6. **Error Handling & Code Hygiene (`.rules`)**:
   - Never call `.unwrap()` or discard fallible Results with `let _ =` in Rust DirectX examples or modules. Use `anyhow::Result` with `?` and descriptive `.context(...)`.
   - Use `bun examples/gpu/verify.ts directx` for cross-platform DirectX/HLSL verification (and `gpui-wgpu` / `dxc` / `fxc` directly when debugging individual shaders).
