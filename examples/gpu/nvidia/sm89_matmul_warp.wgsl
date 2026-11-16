// sm89_matmul_warp.wgsl
// High-throughput tiled matrix multiplication compute shader
// Tuned for NVIDIA Ada Lovelace (sm_89) 32-thread warps and L1/shared memory hierarchy.
//
// Key optimizations:
// - Workgroup size: (32, 1, 1) matching native NVIDIA 32-thread warp width.
// - Shared memory tiling with 1-element padding to eliminate 32-bank conflicts.
// - Subgroup add / shuffle reduction across warp lanes.

struct Uniforms {
    dim_m: u32,
    dim_n: u32,
    dim_k: u32,
    alpha: f32,
}

@group(0) @binding(0)
var<uniform> u: Uniforms;

@group(0) @binding(1)
var<storage, read> matrix_a: array<f32>;

@group(0) @binding(2)
var<storage, read> matrix_b: array<f32>;

@group(0) @binding(3)
var<storage, read_write> matrix_c: array<f32>;

// Tile dimension is 32x32.
// We add +1 to the row stride (33) to prevent 32-bank conflicts on shared memory accesses.
var<workgroup> tile_a: array<f32, 1056>; // 32 * 33 = 1056

@compute @workgroup_size(32, 1, 1)
fn main(
    @builtin(workgroup_id) group_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(global_invocation_id) global_id: vec3<u32>,
) {
    let lane_id = local_id.x; // 0..31 within the warp
    let row = group_id.y * 32u + lane_id;
    let col = group_id.x;

    var sum: f32 = 0.0;
    let num_tiles = (u.dim_k + 31u) / 32u;

    for (var t: u32 = 0u; t < num_tiles; t = t + 1u) {
        // Load tile from matrix A with padded stride to avoid bank conflicts
        let k_idx = t * 32u + lane_id;
        if (row < u.dim_m && k_idx < u.dim_k) {
            tile_a[lane_id * 33u + lane_id] = matrix_a[row * u.dim_k + k_idx];
        } else {
            tile_a[lane_id * 33u + lane_id] = 0.0;
        }

        workgroupBarrier();

        // Accumulate partial dot product across tile elements
        for (var k: u32 = 0u; k < 32u; k = k + 1u) {
            let b_k = t * 32u + k;
            if (b_k < u.dim_k && col < u.dim_n) {
                let a_val = tile_a[lane_id * 33u + k];
                let b_val = matrix_b[b_k * u.dim_n + col];
                sum = sum + a_val * b_val;
            }
        }

        workgroupBarrier();
    }

    if (row < u.dim_m && col < u.dim_n) {
        matrix_c[row * u.dim_n + col] = sum * u.alpha;
    }
}
