# AMD RDNA Architecture & Wavefront Execution Reference

## 1. Architectural Foundations: RDNA 2, RDNA 3 & RDNA 4

AMD RDNA architectures diverge fundamentally from legacy GCN (Graphics Core Next) by transitioning from fixed 64-thread wavefront execution to flexible, dual-mode execution:

- **Workgroup Processor (WGP)**:
  - The fundamental compute building block in RDNA, containing 2 Compute Units (CU).
  - Each CU possesses 2 SIMD32 units, an instruction buffer, branch unit, and local vector register file.
  - 1 WGP executes either 4 independent Wave32 wavefronts per cycle or 2 Wave64 wavefronts.
- **Wave32 Execution Model**:
  - Direct 1-cycle instruction dispatch: an instruction executes across all 32 lanes in a single clock cycle.
  - Significantly lower latency for light-to-medium thread workloads compared to GCN's 4-cycle execution.
  - Halves register file footprint per active wave, dramatically reducing register pressure and doubling active wavefront count.
- **Wave64 Execution Model**:
  - Supported for high math-density compute workloads and legacy code paths.
  - Executed over 2 cycles across the SIMD32 hardware units.

---

## 2. Register Files & Occupancy Management

RDNA GPUs divide register storage into two distinct classes:

| Register Type | Scope | Capacity & Allocation Granularity | Performance Impact |
| :--- | :--- | :--- | :--- |
| **VGPR (Vector General Purpose)** | Per-thread unique values | 1,024 registers per SIMD32; allocated in blocks of 4–8 registers | Primary limiter of SIMD occupancy. If a shader exceeds 32 VGPRs in Wave32 mode, concurrent active waves per SIMD decrease. |
| **SGPR (Scalar General Purpose)** | Shared across all lanes of a wave | Scalar unit registers (800+ per CU) | Extremely efficient for uniform loop counters, buffer descriptors, resource pointers. Freeing VGPRs by moving uniforms to SGPRs directly boosts occupancy. |
| **LDS (Local Data Share)** | Workgroup-shared memory | Configurable 64 KB or 128 KB per WGP | Shared memory across all waves in a workgroup. Bank conflicts occur on 32-bit word strides across 32 banks. |

---

## 3. Wave Intrinsics & Subgroup Reduction

In WGSL, HLSL, and SPIR-V, cross-lane operations map directly to RDNA hardware instructions:

- **Lane Masking (`v_ballot_b32` / `v_ballot_b64`)**: Evaluates a boolean predicate across all active lanes and writes the resulting 32-bit or 64-bit mask to a scalar register.
- **Permute & Read (`v_readlane_b32` / `ds_bpermute_b32`)**: Allows one lane to read the scalar value of another lane without touching LDS memory.
- **Wave Arithmetic**: Logarithmic tree reduction executed in hardware without synchronization barriers (`WaveActiveSum`, `WaveActiveMin`, `WaveActiveMax`).

---

## 4. Memory Subsystem & AMD Infinity Cache

- **Infinity Cache (MALL)**: A massive high-bandwidth on-die L3 cache (up to 96–128 MB on Navi 31/32) sitting between the L2 cache and external GDDR6 memory.
- **Cache Line Sizing**:
  - L0 cache: 32 KB per WGP (low latency vector cache).
  - L1 cache: 128 KB shared across a shader array.
  - L2 cache: 4–6 MB unified across the GPU.
- **Tiling & Swizzling**: Ensure texture accesses are swizzled using standard 2D Hilbert / Morton curve layouts to maximize hit rate in L0/L1 caches.
