# NVIDIA Ada Lovelace (`sm_89`) Architecture Reference

## 1. Overview & Architecture Hierarchy

NVIDIA Ada Lovelace (Compute Capability `sm_89`, GeForce RTX 40-Series) introduces architectural enhancements for real-time rasterization, neural compute, and path tracing:

- **Streaming Multiprocessors (SM)**:
  - 4 sub-cores per SM, each with independent warp scheduler, dispatch unit, and register file (16,384 x 32-bit registers per sub-core = 64K registers per SM).
  - Dual FP32 datapath: each SM sub-core can execute either 16 FP32 operations or 16 INT32 operations plus 16 FP32 operations concurrently.
  - Uniform Register File and separate branch/predicate execution pipelines.
- **4th Generation Tensor Cores**:
  - Native support for FP8 (E4M3, E5M2), FP16, BF16, TF32, INT8, and INT4 matrix multiply-accumulate (MMA).
  - Structural Sparsity: 2:4 sparse matrix multiplication doubling theoretical tensor throughput.
  - Transformer Engine for dynamic FP8/FP16 precision scaling.
- **3rd Generation RT Cores**:
  - Shader Execution Reordering (SER): Dynamically sorts divergent ray-tracing / compute threads into coherent bundles on chip.
  - Displaced Micro-Mesh (DMM) and Opacity Micromap (OMM) acceleration engines.
- **L2 Cache Hierarchy**:
  - Substantially enlarged ultra-high bandwidth L2 cache (up to 72 MB on AD102 / 36 MB on AD104 / 32 MB on AD106).
  - Dramatically reduces memory bus traffic compared to prior generations.

---

## 2. Warp Schedulers & Register Pressure

- **Warp Size**: Fixed at 32 threads (`warpSize = 32`).
- **Concurrent Warps per SM**: Up to 48 warps (1536 concurrent threads per SM).
- **Shared Memory / L1 Data Cache**: Configurable up to 128 KB per SM (typically partitioned as 100 KB L1 + 28 KB shared memory or 48 KB shared memory).
- **Occupancy Management**:
  - Aim for register usage <= 32 registers/thread for 100% theoretical occupancy (48 warps/SM).
  - For complex kernels with high register count (<= 64 registers/thread), occupancy targets 50% (24 warps/SM), which is often optimal if compute pipeline latency is hidden by math intensity.

---

## 3. Subgroup Intrinsics & Warp Shuffles

In WGSL and SPIR-V, Ada Lovelace maps subgroup operations directly to hardware warp instructions:

| Operation | WGSL Syntax | PTX / Hardware Mapping | Latency / Throughput |
| :--- | :--- | :--- | :--- |
| **Warp Ballot** | `subgroupBallot(pred)` | `vote.ballot.sync` | 1 cycle, outputs 32-bit bitmask |
| **Warp Shuffle Across** | `subgroupShuffle(val, lane)` | `shfl.sync.idx.b32` | 1 cycle direct register exchange (no shared memory) |
| **Warp Shuffle Down** | `subgroupShuffleDown(val, delta)` | `shfl.sync.down.b32` | 1 cycle logarithmic reduction |
| **Warp Broadcast** | `subgroupBroadcast(val, 0u)` | `shfl.sync.idx.b32` | 1 cycle uniform distribution |
| **Warp Reduction** | `subgroupAdd(val)` | Subgroup tree reduction | Parallel warp arithmetic |

---

## 4. CUDA & PTX Compilation Workflow

CUDA kernels targeting `sm_89` can be compiled into PTX or native CUBIN:

```bash
# Compile CUDA C++ to PTX for Ada Lovelace
nvcc -arch=sm_89 -ptx kernel.cu -o kernel.ptx

# Compile to native cubin with full optimizations
nvcc -O3 -arch=sm_89 -cubin kernel.cu -o kernel.cubin

# Profile kernel execution with Nsight Compute CLI
ncu --target-processes all --metrics sm__throughput.avg.pct_of_peak_sustained_elapsed,gpu__time_duration.sum ./my_app
```
