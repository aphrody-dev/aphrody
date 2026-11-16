// vector_fma.ispc
// High-performance CPU SIMD vector FMA kernel written for Intel ISPC.
// Demonstrates SPMD vectorization compiling to AVX2 (8-wide) and AVX-512 (16-wide).

export void vector_fma_ispc(
    uniform const float a[],
    uniform const float b[],
    uniform const float c[],
    uniform float result[],
    uniform int count,
    uniform float * uniform total_sum
) {
    varying float acc = 0.0f;

    // Parallel foreach loop across SIMD lanes
    foreach (i = 0 ... count) {
        varying float val = a[i] * b[i] + c[i];
        result[i] = val;
        acc += val;
    }

    // Parallel lane reduction across all active SIMD lanes
    uniform float sum = reduce_add(acc);
    if (total_sum != NULL) {
        *total_sum = sum;
    }
}
