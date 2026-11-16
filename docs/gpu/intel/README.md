# Intel Hardware, SIMD & Xe GPU Architecture Guide (`aphrody`)

Central engineering guide for Intel Arc Xe/Xe2 GPUs, Core/Xeon/EPYC SIMD processors, ISPC, and oneAPI Level Zero in `aphrody`.

---

## 1. Hardware Architecture Overview

| Parameter | Specification | Notes |
| :--- | :--- | :--- |
| **PCI Vendor ID** | `0x8086` | Intel Corporation |
| **GPU Target Architectures** | Intel Arc Alchemist (Xe-HPG), Battlemage (Xe2-HPG), Xe-LPG | Xe Core with Vector & Matrix (XMX) engines |
| **CPU SIMD Extensions** | SSE4.2, AVX2, AVX-512, AMX | Multi-target compilation via ISPC |
| **Subgroup Sizing** | SIMD8, SIMD16, SIMD32 | Dynamic hardware SIMD wave sizing |
| **Low-Level APIs** | oneAPI Level Zero (`ze_api.h`), Vulkan 1.4 (ANV), DirectX 12 | Bare-metal driver access via `ze_loader.dll` / `libze_loader.so` |

---

## 2. Toolchain Discovery & Commands

Intel tools, ISPC, and Level Zero headers are discoverable in standard paths:

```powershell
# Check ISPC compiler version
ispc --version

# Query OpenCL devices (CPU and GPU)
clinfo -l

# Compile an ISPC SPMD kernel to AVX2 and AVX-512 object code
ispc -O3 --target=avx2-i32x8,avx512skx-x16 kernel.ispc -o kernel.obj -h kernel.h
```

---

## 3. Shader & Vector Optimization Guidelines

1. **Subgroup Sizing**: Intel Xe compute units dynamically dispatch SIMD16 or SIMD32 waves. Size workgroups to multiples of 32 (e.g. 64, 128, 256) to ensure optimal vector execution.
2. **ISPC Memory Access**: Favor contiguous linear loads inside `foreach` loops to allow vector load instructions (`vmovups`) instead of expensive gather operations (`vgatherdps`).
3. **Level Zero Unified Shared Memory (USM)**: Use `zeMemAllocShared` when building cross-device CPU-GPU pipelines with automatic page migration.

---

## 4. Code Examples

- [`examples/gpu/intel/xe_tiled_conv.wgsl`](../../../examples/gpu/intel/xe_tiled_conv.wgsl): 2D convolution kernel structured for Intel Xe vector units.
- [`examples/gpu/intel/vector_fma.ispc`](../../../examples/gpu/intel/vector_fma.ispc): High-performance ISPC SIMD vector FMA kernel for AVX2 and AVX-512.
- [`crates/wgpu_cli/examples/ihv_vendor_matrix.rs`](../../../crates/wgpu_cli/examples/ihv_vendor_matrix.rs): Multi-vendor enumeration and compute dispatch test.
