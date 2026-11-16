# AMD Hardware, Architecture & Toolchain Guide (`aphrody`)

Central engineering guide for AMD Radeon GPUs and AMD Zen/EPYC processors in `aphrody` across Vulkan 1.4, DirectX 12, AGS SDK, and RGA.

---

## 1. Hardware Architecture Overview

| Parameter | Specification | Notes |
| :--- | :--- | :--- |
| **PCI Vendor IDs** | `0x1002` (Radeon Discrete / APU), `0x1022` (Zen Host) | Advanced Micro Devices |
| **Primary Target Architectures** | RDNA 3 (`gfx1100`), RDNA 4 (`gfx1200`), CDNA | Workgroup Processor (WGP) architecture |
| **Wavefront Sizing** | Dual-mode Wave32 & Wave64 | Subgroups reported as `32..=64` or `32..=32` |
| **Vector Units** | SIMD32 ALUs per Compute Unit | 1-cycle Wave32 instruction issue |
| **L3 Cache** | AMD Infinity Cache (MALL) | Up to 128 MB on Navi 31/32 |
| **Graphics APIs** | Vulkan 1.4 (RADV / lavapipe), DirectX 12 | Native AGS SDK (`amd_ags_x64.dll`) interop |

---

## 2. Toolchain Discovery & Commands

AMD tools and analysis utilities are discoverable in standard paths:

```powershell
# Query OpenCL devices and platforms
clinfo -l

# Check Radeon GPU Analyzer (RGA) version and supported ASICs
rga --version
rga -s vk-spv-offline -l

# Offline disassembly and register pressure analysis of a SPIR-V shader for RDNA3
rga -s vk-spv-offline -c gfx1100 --comp shader.spv --isa isa.txt --analysis stats.csv
```

---

## 3. Shader & Workgroup Optimization Guidelines

1. **Wave32 vs Wave64 Sizing**: Set workgroup size to multiples of 64 (e.g., `@workgroup_size(64, 1, 1)` or `@workgroup_size(128, 1, 1)`) to guarantee full wave saturation across both Wave32 and Wave64 execution modes.
2. **VGPR Pressure**: Monitor register usage via `rga.exe`. Keep vector registers <= 32 VGPRs per thread to achieve 100% theoretical SIMD occupancy.
3. **Cross-Lane Wave Intrinsics**: In HLSL and WGSL, leverage `WaveActiveSum`, `WaveActiveMin`, and `WaveReadLaneFirst` for low-latency reductions without LDS bank contention.

---

## 4. Code Examples

- [`examples/gpu/amd/wave_reduce_rdna.wgsl`](../../../examples/gpu/amd/wave_reduce_rdna.wgsl): Wavefront parallel reduction shader tuned for RDNA Wave32/Wave64 pipelines.
- [`crates/wgpu_cli/examples/ihv_vendor_matrix.rs`](../../../crates/wgpu_cli/examples/ihv_vendor_matrix.rs): Multi-vendor enumeration and compute dispatch test.
