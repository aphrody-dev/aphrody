---
name: nvidia
description: >-
  Architect, build, profile, compile, and optimize NVIDIA GPU infrastructure in gpui-os.
  Use whenever targeting NVIDIA GPUs (vendor ID 0x10DE), Ada Lovelace sm_89 / Hopper / Blackwell architectures,
  CUDA 13.x compiler pipelines (nvcc, PTX), NVTX3 instrumentation, Nsight Systems/Compute profiling,
  warp-level primitives (32 threads, ballot/shuffle), NVML telemetry, or vendor/wgpu NVIDIA quirks.
---

# NVIDIA GPU Architecture & Toolchain (`gpui-os`)

NVIDIA GPUs (Vendor ID `0x10DE`) provide high-throughput graphics and compute execution in `gpui-os`. Through `vendor/wgpu` and `crates/gpui_wgpu`, NVIDIA hardware is accessed primarily via **Vulkan 1.4** (`ash` 0.38) and **DirectX 12** (`d3d12.dll` / DirectComposition), complemented by native CUDA/PTX compute shaders, NVTX3 instrumentation, and Nsight profiling.

This skill provides architectural specifications, verified toolchain paths, shader compilation patterns, and vendor-specific tuning parameters for Ada Lovelace (`sm_89`), Ampere (`sm_80`/`sm_86`), and Turing+ architectures.

---

## Verified Toolchains & Environment Discovery

NVIDIA tools are discoverable via standard environment paths and user-space prefixes:

| Tool / Component | Version / Target | Standard Path / Discovery Pattern |
| :--- | :--- | :--- |
| **CUDA Compiler (`nvcc`)** | `CUDA 13.x` | `nvcc` in PATH or `%CUDA_PATH%\bin\nvcc.exe` |
| **NVIDIA System Management (`nvidia-smi`)** | Driver 550+ / 600+ | `nvidia-smi` in PATH or `C:\Windows\System32\nvidia-smi.exe` |
| **Nsight Systems (`nsys`)** | `2026.x` CLI | `nsys` in PATH or `%ProgramFiles%\NVIDIA Corporation\Nsight Systems*\target-*\nsys.exe` |
| **Nsight Compute (`ncu`)** | `2026.x` CLI | `ncu` in PATH or `%ProgramFiles%\NVIDIA Corporation\Nsight Compute*\ncu.bat` |
| **NVTX3 Instrumentation** | `v3.7.x` C/C++ API | `~/.local/sdk/ihv/nvidia/nvtx/nvtx3/nvtx3.hpp` |
| **WGPU CLI (`gpui-wgpu`)** | `v30.0.0` (Vulkan/DX12) | `target/debug/gpui-wgpu.exe` or `gpui-wgpu` |

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
