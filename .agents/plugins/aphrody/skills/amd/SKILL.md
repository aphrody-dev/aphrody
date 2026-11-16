---
name: amd
description: >-
  Architect, build, analyze, compile, and optimize AMD Radeon GPU & EPYC/Ryzen SIMD infrastructure in aphrody.
  Use whenever targeting AMD hardware (vendor IDs 0x1002 / 0x1022), RDNA 3 / RDNA 4 / CDNA architectures,
  AMD GPU Services (AGS SDK v6.3.0, amd_ags_x64.dll), Radeon GPU Analyzer (rga CLI),
  wavefront execution (Wave32 vs Wave64), cross-lane wave intrinsics, or vendor/wgpu AMD quirks.
---

# AMD GPU & SIMD Architecture (`aphrody`)

AMD GPUs (Vendor ID `0x1002` for discrete Radeon graphics, `0x1022` for integrated APU/processor components) drive graphics rendering and parallel compute across Windows, Linux, and heterogeneous fleet environments. In `aphrody`, AMD hardware is integrated via **Vulkan 1.4** (RADV / AMDVLK / lavapipe), **DirectX 12** with AMD GPU Services (`amd_ags_x64.dll`), and offline shader analysis via Radeon GPU Analyzer (`rga`).

This skill provides architectural specifications, verified toolchain paths, shader compilation patterns, and wavefront tuning standards for RDNA 2/3/4 and CDNA architectures.

---

## Toolchain & Dynamic Discovery

AMD tools are resolved cross-platform via `PATH`, `<SLUG>_SDK_ROOT`, and `bun examples/gpu/verify.ts amd`:

| Tool / Component | Discovery Pattern | Role |
| :--- | :--- | :--- |
| **AMD GPU Services (AGS SDK)** | `<SLUG>_SDK_ROOT/ihv/amd/ags` (`amd_ags.h`, `amd_ags_x64.dll`) | DX12/Vulkan AMD extension API |
| **Radeon GPU Analyzer (`rga`)** | `rga` on `PATH` | Offline RDNA/CDNA ISA & register analysis |
| **ROCm / OpenCL** | `clinfo` on `PATH` | OpenCL device/rusticl discovery |
| **WGPU CLI (`gpui-wgpu`)** | `gpui-wgpu` on `PATH` or `target/{debug,release}/gpui-wgpu` | Adapter probe & WGSL validation |

---

## Technical References

Consult the referenced guides in `references/` before writing or optimizing AMD shader or driver code:

| Topic | Reference Document | Key Concepts Covered |
| :--- | :--- | :--- |
| **RDNA Wavefront Architecture** | [rdna-wavefront-architecture.md](references/rdna-wavefront-architecture.md) | Dual-mode Wave32 vs Wave64, Dual Compute Units (WGP), VGPR vs SGPR allocation, memory hierarchy (Infinity Cache), cross-lane wave operations |
| **AGS SDK & RGA Toolchain** | [ags-sdk-and-rga.md](references/ags-sdk-and-rga.md) | AGS initialization, DX12 device creation with extension flags, `rga` offline SPIR-V/DXIL compilation, VGPR register pressure & live range analysis |

---

## Key Hardware Invariants & WGPU Tuning

1. **Vendor Identification**:
   - PCI Vendor ID: `0x1002` (AMD Graphics / Radeon) and `0x1022` (AMD Host / Zen APU)
   - Device Classification: `wgpu::DeviceType::DiscreteGpu` or `IntegratedGpu`
   - Primary Backends: `wgpu::Backends::VULKAN` and `wgpu::Backends::DX12`

2. **Wavefront Sizing & Execution**:
   - RDNA architecture supports both **Wave32** (default for graphics shaders) and **Wave64** (compute mode).
   - WGPU reports AMD subgroup sizes typically as `32..=64` or `32..=32` depending on driver configuration.
   - Workgroup Sizing: Optimal thread count per workgroup is `64` or `128` (multiples of both 32 and 64), ensuring maximum hardware occupancy regardless of whether Wave32 or Wave64 mode is dispatched.

3. **AMD GPU Services (AGS) Integration in `aphrody`**:
   - `crates/gpui_windows` and `script/bundle-windows.ps1` package `amd_ags_x64.dll` alongside the application executable.
   - Allows native access to AMD-specific D3D12 capabilities:
     - Cross-lane intrinsics (`WaveReadLaneAt`, `WaveActiveBallot`).
     - Barycentric coordinates in pixel shaders without geometry passes.
     - Direct high-resolution GPU timer queries.

4. **Radeon GPU Analyzer (`rga`) Inspection Workflow**:
   - Analyze VGPR and SGPR register usage offline without requiring an AMD GPU physically installed on the compilation host:
     ```bash
     # Compile and analyze SPIR-V compute shader for RDNA3 (gfx1100)
     rga -s vk-spv-offline -c gfx1100 --comp shader.spv --isa isa.txt --analysis stats.csv
     ```
