// wave_reduce_rdna.wgsl
// High-efficiency parallel reduction compute shader
// Tuned for AMD RDNA 2/3/4 architectures (Wave32 and Wave64 execution modes).
//
// Key optimizations:
// - Workgroup size: (64, 1, 1) ensuring full wavefront saturation in both Wave32 and Wave64 modes.
// - LDS shared memory reduction tree with sequential power-of-two fold.
// - Minimal VGPR footprint to maximize concurrent WGP wave occupancy.

struct Uniforms {
    count: u32,
    stride: u32,
    pad0: u32,
    pad1: u32,
}

@group(0) @binding(0)
var<uniform> u: Uniforms;

@group(0) @binding(1)
var<storage, read> input_data: array<f32>;

@group(0) @binding(2)
var<storage, read_write> output_data: array<f32>;

var<workgroup> lds_scratch: array<f32, 64>;

@compute @workgroup_size(64, 1, 1)
fn main(
    @builtin(workgroup_id) group_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(global_invocation_id) global_id: vec3<u32>,
) {
    let tid = local_id.x;
    let gid = global_id.x;

    // Load initial element into LDS
    if (gid < u.count) {
        lds_scratch[tid] = input_data[gid];
    } else {
        lds_scratch[tid] = 0.0;
    }

    workgroupBarrier();

    // Logarithmic reduction tree across the 64-wide workgroup
    // Fits into 2 Wave32 wavefronts or 1 Wave64 wavefront
    if (tid < 32u) {
        lds_scratch[tid] = lds_scratch[tid] + lds_scratch[tid + 32u];
    }
    workgroupBarrier();

    if (tid < 16u) {
        lds_scratch[tid] = lds_scratch[tid] + lds_scratch[tid + 16u];
    }
    workgroupBarrier();

    if (tid < 8u) {
        lds_scratch[tid] = lds_scratch[tid] + lds_scratch[tid + 8u];
    }
    workgroupBarrier();

    if (tid < 4u) {
        lds_scratch[tid] = lds_scratch[tid] + lds_scratch[tid + 4u];
    }
    workgroupBarrier();

    if (tid < 2u) {
        lds_scratch[tid] = lds_scratch[tid] + lds_scratch[tid + 2u];
    }
    workgroupBarrier();

    if (tid == 0u) {
        output_data[group_id.x] = lds_scratch[0] + lds_scratch[1];
    }
}
