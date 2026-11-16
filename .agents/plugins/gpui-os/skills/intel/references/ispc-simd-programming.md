# ISPC & CPU SIMD Vectorization Reference

## 1. Intel SPMD Program Compiler (ISPC) Foundations

ISPC is a compiler for a variant of the C programming language with extensions for SPMD (Single Program, Multiple Data) parallelism. It compiles directly to vector instructions (SSE4, AVX, AVX2, AVX-512, NEON) on modern CPUs and GPUs.

### 1.1 Core SPMD Concepts
- **`uniform` vs `varying`**:
  - `uniform`: Represents a single scalar value shared identically across all SIMD program instances (lanes). Stored in a standard CPU register.
  - `varying`: Represents a vector of values, where each SIMD program instance possesses a distinct value. Stored in a vector SIMD register (e.g., YMM on AVX2, ZMM on AVX-512).
  - Variables are `varying` by default unless explicitly decorated with `uniform`.
- **`foreach` Loop**:
  - Automatically parallelizes iteration spaces across active SIMD lanes:
    ```c
    export void saxpy_ispc(uniform float a, uniform float x[], uniform float y[], uniform int n) {
        foreach (i = 0 ... n) {
            y[i] = a * x[i] + y[i];
        }
    }
    ```
- **Reductions**:
  - `reduce_add(val)`: Sums a `varying` variable across all active SIMD lanes, returning a scalar `uniform` value.
  - `reduce_min(val)`, `reduce_max(val)`: Parallel lane min/max reductions.

---

## 2. Multi-Target Compilation & CPU Dispatch

ISPC allows compiling a single source file into multiple target vector ISAs with dynamic runtime CPU feature dispatch:

```bash
# Multi-target compilation for x86_64 CPUs:
# Generates kernel_sse4.obj, kernel_avx2.obj, kernel_avx512.obj and dispatch stub
ispc -O3 --target=sse4-i32x4,avx2-i32x8,avx512skx-x16 \
     --opt=fast-math \
     vector_ops.ispc -o vector_ops.obj -h vector_ops.h
```

### Supported Compilation Targets:
- `sse4-i32x4`: 4-wide 32-bit floats on SSE4.2 (Intel Nehalem+, AMD Bulldozer+)
- `avx2-i32x8`: 8-wide 32-bit floats on AVX2 + FMA3 (Intel Haswell+, AMD Zen 1/2/3/4)
- `avx512skx-x16`: 16-wide 32-bit floats on AVX-512 Foundation + VL/BW/DQ (Intel Skylake-X, Xeon Scalable, AMD Zen 4 EPYC)
- `gen9-x8` / `xelp-x16`: Intel Xe GPU compute target

---

## 3. Interfacing ISPC with Rust (`gpui-os`)

ISPC exports standard C ABI symbols when marked with `export`:

```rust
// In Rust:
extern "C" {
    fn saxpy_ispc(
        a: f32,
        x: *const f32,
        y: *mut f32,
        n: i32,
    );
}

pub fn run_saxpy(a: f32, x: &[f32], y: &mut [f32]) {
    assert_eq!(x.len(), y.len());
    unsafe {
        saxpy_ispc(a, x.as_ptr(), y.as_mut_ptr(), x.len() as i32);
    }
}
```

---

## 4. Performance Rules for High-Throughput SIMD

1. **Memory Coalescing**:
   - Access arrays using linear contiguous indices (`arr[i]`) within `foreach (i = 0 ... n)`.
   - Strided access (`arr[i * 4]`) forces gather/scatter vector instructions (`vgatherdps`), which are 3–5x slower than contiguous vector loads (`vmovups`).
2. **Structure of Arrays (SOA)**:
   - Prefer SOA data layouts (`struct { float x[N]; float y[N]; float z[N]; }`) over Array of Structures (`struct Point { float x, y, z; } points[N];`) to prevent gather overhead.
3. **Branch Coherence**:
   - Conditionals on `varying` variables execute both paths under lane masking. Keep branch divergence minimal inside hot inner loops.
