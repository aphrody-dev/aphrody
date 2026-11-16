---
name: intel
description: >-
  Architect, build, compile, and optimize Intel CPU SIMD (AVX2/AVX-512) & Intel Arc Xe/Xe2 GPU infrastructure in aphrody.
  Use whenever targeting Intel hardware (vendor ID 0x8086), Core / Xeon / EPYC SIMD vectorization,
  Intel SPMD Program Compiler (ispc CLI), oneAPI Level Zero runtime (ze_api.h, ze_loader),
  OpenCL CPU/GPU runtimes (clinfo, rusticl, compute-runtime), or vendor/wgpu Intel quirks.
---

# Intel SIMD & Xe GPU Architecture (`aphrody`)

Intel hardware spans high-performance vector CPU SIMD execution (SSE4.2, AVX2, AVX-512) and discrete/integrated GPU acceleration (Intel Arc Alchemist Xe-HPG, Battlemage Xe2-HPG, Meteor Lake / Arrow Lake Xe-LPG). In `aphrody`, Intel vector units accelerate image processing, font rasterization, and numerical algorithms via **ISPC** (Intel SPMD Program Compiler), while Intel GPUs are driven through **Vulkan 1.4** (ANV), **DirectX 12**, and **oneAPI Level Zero**.

This skill provides architectural specifications, verified toolchain paths, ISPC compilation patterns, and low-level Level Zero execution standards.

---

## Toolchain & Dynamic Discovery

Intel tools and SDK components are resolved cross-platform via `PATH`, `<SLUG>_SDK_ROOT`, and `bun examples/gpu/verify.ts intel`:

| Tool / Component | Discovery Pattern | Role |
| :--- | :--- | :--- |
| **Intel ISPC Compiler** | `ispc` on `PATH` | SPMD CPU SIMD multi-target compiler |
| **oneAPI Level Zero SDK** | `<SLUG>_SDK_ROOT/ihv/intel/level-zero` (`ze_api.h`, `ze_loader`) | Low-level Xe compute driver interface |
| **OpenCL SDK & Diagnostics** | `clinfo` on `PATH` | OpenCL CPU/GPU runtime diagnostics |
| **WGPU CLI (`gpui-wgpu`)** | `gpui-wgpu` on `PATH` or `target/{debug,release}/gpui-wgpu` | Adapter probe & WGSL validation |

---

## Technical References

Consult the referenced guides in `references/` before writing or optimizing Intel SIMD or Xe GPU code:

| Topic | Reference Document | Key Concepts Covered |
| :--- | :--- | :--- |
| **ISPC & CPU SIMD Vectorization** | [ispc-simd-programming.md](references/ispc-simd-programming.md) | SPMD programming model, `uniform` vs `varying`, AVX2 (`avx2-i32x8`) vs AVX-512 (`avx512skx-x16`), SOA/AOS conversions, C ABI linkage |
| **oneAPI Level Zero & Intel Xe Architecture** | [level-zero-and-xe.md](references/level-zero-and-xe.md) | Xe Vector Engines (XVE), Xe Matrix Extensions (XMX), Level Zero low-overhead driver interface (`zeInit`, `zeCommandListAppendLaunchKernel`), USM unified shared memory |

---

## Key Hardware Invariants & WGPU Tuning

1. **Vendor Identification**:
   - PCI Vendor ID: `0x8086` (Intel Graphics & Processors)
   - Device Classification: `wgpu::DeviceType::DiscreteGpu`, `IntegratedGpu`, or `Cpu`
   - Primary Backends: `wgpu::Backends::VULKAN` and `wgpu::Backends::DX12`

2. **Xe Subgroup & Execution Unit Sizing**:
   - Intel Xe Cores feature flexible Execution Units (EU / XVE) that dynamically execute at SIMD widths:
     - SIMD8 (8 threads per hardware wave)
     - SIMD16 (16 threads per hardware wave - common default for compute)
     - SIMD32 (32 threads per hardware wave - optimal for bandwidth-bound kernels)
   - WGPU reports Intel subgroup size range typically as `8..=32` or `16..=32`.
   - Compute Workgroup Sizing: Optimal thread count per workgroup is `64`, `128`, or `256` (multiples of 8, 16, and 32).

3. **CPU SIMD Vectorization with ISPC in `aphrody`**:
   - ISPC compiles SPMD kernels into native object files linking directly with Rust via `extern "C"`.
   - Generates multi-target dispatch libraries automatically (e.g. AVX2, AVX-512, and SSE4 fallback) from a single source file:
     ```bash
     ispc --target=sse4-i32x4,avx2-i32x8,avx512skx-x16 kernel.ispc -o kernel.obj -h kernel.h
     ```

4. **WGPU & Driver Quirks on Intel Hardware**:
   - Avoid non-power-of-two workgroup dimensions in compute shaders.
   - For Intel integrated GPUs sharing system memory, ensure storage buffer writes are explicitly flushed before subsequent CPU readbacks.
   - ANV Vulkan driver requires device node access to `/dev/dri/renderD128` in containerized/headless environments.
