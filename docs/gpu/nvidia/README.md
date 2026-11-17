# NVIDIA Hardware, Architecture & Toolchain Guide (`aphrody`)

Central engineering guide for NVIDIA GPUs in `aphrody` across Vulkan 1.4, DirectX 12, CUDA 13.x, and WGPU.

---

## 1. Hardware Architecture Overview

| Parameter | Specification | Notes |
| :--- | :--- | :--- |
| **PCI Vendor ID** | `0x10DE` | NVIDIA Corporation |
| **Primary Target Architecture** | Ada Lovelace (`sm_89`) | Compute capability 8.9 (e.g. GeForce RTX 40-Series) |
| **Warp Sizing** | 32 threads | Subgroups reported as `32..=32` |
| **Tensor Units** | 4th Gen Tensor Cores | FP8, FP16, BF16, TF32, INT8 matrix acceleration |
| **Ray Tracing** | 3rd Gen RT Cores | Shader Execution Reordering (SER) |
| **Graphics APIs** | Vulkan 1.4, DirectX 12 Agility | Full WGPU and WebGPU feature conformance |

---

## 2. Toolchain Discovery & Commands

NVIDIA tools are discoverable in standard system paths:

```powershell
# Query GPU status, driver version, VRAM, and running processes
nvidia-smi

# Check CUDA compiler version
nvcc --version

# Run WGPU adapter diagnostic for NVIDIA devices
gpui-wgpu adapters --limits --features

# Profile application performance with Nsight Systems CLI
nsys profile -t vulkan,cuda,osrt --output profile_report ./my_app
```

---

## 3. Shader & Workgroup Optimization Guidelines

1. **Workgroup Size Alignment**: Always align compute shader `@workgroup_size` to multiples of 32 (e.g., `@workgroup_size(32, 1, 1)` or `@workgroup_size(128, 1, 1)`).
2. **Subgroup Intrinsics**: Use WGSL `subgroupBallot`, `subgroupShuffle`, and `subgroupAdd` to exchange data between warp lanes without writing to shared memory.
3. **Memory Alignment**:
   - Uniform buffer dynamic offsets: align to 64 bytes (Vulkan) or 256 bytes (DirectX 12).
   - Storage buffer offsets: align to 32 bytes.
   - Texture pitch alignment: `COPY_BYTES_PER_ROW_ALIGNMENT = 256`.

---

## 4. Code Examples

- [`examples/gpu/nvidia/sm89_matmul_warp.wgsl`](../../../examples/gpu/nvidia/sm89_matmul_warp.wgsl): Tiled matrix multiplication optimized for 32-thread warp execution with shared memory padding.
- [`crates/aphrody_cli/examples/ihv_vendor_matrix.rs`](../../../crates/aphrody_cli/examples/ihv_vendor_matrix.rs): Multi-vendor enumeration and compute dispatch test.
