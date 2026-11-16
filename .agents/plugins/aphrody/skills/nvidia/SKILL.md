---
name: nvidia
description: >-
  Architect, build, profile, compile, and optimize NVIDIA GPU infrastructure in aphrody.
  Use whenever targeting NVIDIA GPUs (vendor ID 0x10DE), Ada Lovelace sm_89 / Hopper / Blackwell architectures,
  CUDA 13.x compiler pipelines (nvcc, PTX), NVTX3 instrumentation, Nsight Systems/Compute profiling,
  warp-level primitives (32 threads, ballot/shuffle), NVML telemetry, or vendor/wgpu NVIDIA quirks.
---

# NVIDIA GPU Architecture & Toolchain (`aphrody`)

NVIDIA GPUs (Vendor ID `0x10DE`) provide high-throughput graphics and compute execution in `aphrody`. Through `vendor/wgpu` and `crates/gpui_wgpu`, NVIDIA hardware is accessed primarily via **Vulkan 1.4** (`ash` 0.38) and **DirectX 12** (`d3d12.dll` / DirectComposition), complemented by native CUDA/PTX compute shaders, NVTX3 instrumentation, and Nsight profiling.

This skill provides architectural specifications, verified toolchain paths, shader compilation patterns, and vendor-specific tuning parameters for Ada Lovelace (`sm_89`), Ampere (`sm_80`/`sm_86`), and Turing+ architectures.

---

## Toolchain & Dynamic Discovery

NVIDIA tools are resolved dynamically by `bun examples/gpu/verify.ts nvidia` across platforms via `PATH`, `CUDA_PATH`, or `<SLUG>_SDK_ROOT`:

| Tool / Component | Discovery Pattern | Role |
| :--- | :--- | :--- |
| **CUDA Compiler (`nvcc`)** | `nvcc` on `PATH` or `$CUDA_PATH/bin/nvcc` | PTX / CUDA kernel compilation |
| **NVIDIA System Management (`nvidia-smi`)** | `nvidia-smi` on `PATH` | Driver telemetry & GPU inventory |
| **Nsight Systems (`nsys`)** | `nsys` on `PATH` | System-wide timeline profiling |
| **Nsight Compute (`ncu`)** | `ncu` on `PATH` | Kernel-level occupancy & counters |
| **NVTX3 Instrumentation** | `<SLUG>_SDK_ROOT/ihv/nvidia/nvtx/nvtx3/nvtx3.hpp` | CPU/GPU timeline marker API |
| **WGPU CLI (`gpui-wgpu`)** | `gpui-wgpu` on `PATH` or `target/{debug,release}/gpui-wgpu` | Adapter probe & Naga transpile |

---

## Technical References

Consult the referenced guides in `references/` before writing or modifying NVIDIA GPU code:

| Topic | Reference Document | Key Concepts Covered |
| :--- | :--- | :--- |
| **Ada Lovelace `sm_89` Architecture** | [architecture-ada-sm89.md](references/architecture-ada-sm89.md) | SM structure, 4th Gen Tensor Cores (FP8/FP16/BF16/TF32), 3rd Gen RT Cores, L2 cache hierarchy (36–72 MB), Warp scheduler (32 threads) |
| **WGPU & Vulkan/DX12 NVIDIA Tuning** | [wgpu-nvidia-tuning.md](references/wgpu-nvidia-tuning.md) | Alignment rules (uniform 64B on Vulkan / 256B on DX12, storage 32B), subgroup operations (`subgroupBallot`, `subgroupShuffle`), barrier synchronization, NVTX range markers |

---

## Key Hardware Invariants & WGPU Tuning

1. **Vendor Identification**:
   - PCI Vendor ID: `0x10DE`
   - Device Classification: `wgpu::DeviceType::DiscreteGpu`
   - Primary Backends: `wgpu::Backends::VULKAN` and `wgpu::Backends::DX12`

2. **Warp Execution & Subgroups**:
   - Hardware Warp Size: Exactly `32` threads.
   - WGPU Subgroup Size: Reported as `32..=32` on Vulkan and DX12.
   - Compute Workgroup Sizing: Optimal thread count per workgroup is a multiple of 32 (e.g., `(32, 1, 1)`, `(64, 1, 1)`, `(128, 1, 1)`, or `(256, 1, 1)`).
   - Warp Divergence: Divergent branch execution serializes divergent paths within the 32-thread warp. Use subgroup shuffle intrinsics (`subgroupShuffle`, `subgroupBroadcast`) rather than shared memory when communicating within a warp.

3. **Buffer Alignment Requirements**:
   - `min_uniform_buffer_offset_alignment`:
     - Vulkan backend: `64` bytes.
     - DirectX 12 backend: `256` bytes (enforced by D3D12 hardware spec).
   - `min_storage_buffer_offset_alignment`: `32` bytes on both backends.
   - `COPY_BYTES_PER_ROW_ALIGNMENT`: `256` bytes for texture copy operations.

4. **NVTX Profiling Instrumentation**:
   - Annotate render passes and compute dispatches with NVTX ranges to enable pinpoint analysis in Nsight Systems:
     ```cpp
     #include <nvtx3/nvtx3.hpp>
     nvtx3::scoped_range r{"gpui_frame_render"};
     ```
   - In Rust, emit Vulkan debug marker labels (`cmd_begin_debug_utils_label_ext`) or DX12 PIX markers (`SetMarkerOnCommandList`) which Nsight intercepts automatically.
