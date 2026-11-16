// xe_tiled_conv.wgsl
// 2D image convolution compute shader
// Tuned for Intel Xe architecture (SIMD8, SIMD16, and SIMD32 wave execution).
//
// Key optimizations:
// - Workgroup size: (16, 16, 1) = 256 threads, evenly divisible by 8, 16, and 32 SIMD widths.
// - Shared memory halo caching to reduce global memory load pressure.
// - 3x3 separable kernel convolution for high arithmetic intensity.

struct Uniforms {
    width: u32,
    height: u32,
    kernel_radius: u32,
    bias: f32,
}

@group(0) @binding(0)
var<uniform> u: Uniforms;

@group(0) @binding(1)
var<storage, read> input_pixels: array<f32>;

@group(0) @binding(2)
var<storage, read_write> output_pixels: array<f32>;

// 16x16 tile with a 1-pixel halo on each side = 18x18 = 324 elements
var<workgroup> tile_cache: array<f32, 324>;

@compute @workgroup_size(16, 16, 1)
fn main(
    @builtin(workgroup_id) group_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(global_invocation_id) global_id: vec3<u32>,
) {
    let lx = local_id.x;
    let ly = local_id.y;
    let gx = global_id.x;
    let gy = global_id.y;

    // Load central tile pixel
    let tile_x = lx + 1u;
    let tile_y = ly + 1u;
    let in_bounds = (gx < u.width && gy < u.height);

    if (in_bounds) {
        tile_cache[tile_y * 18u + tile_x] = input_pixels[gy * u.width + gx];
    } else {
        tile_cache[tile_y * 18u + tile_x] = 0.0;
    }

    // Load boundary halo elements on thread edges
    if (lx == 0u) {
        let hx = select(0u, gx - 1u, gx > 0u);
        tile_cache[tile_y * 18u + 0u] = select(0.0, input_pixels[gy * u.width + hx], in_bounds && gx > 0u);
    }
    if (lx == 15u) {
        let hx = select(gx, gx + 1u, gx + 1u < u.width);
        tile_cache[tile_y * 18u + 17u] = select(0.0, input_pixels[gy * u.width + hx], in_bounds && gx + 1u < u.width);
    }
    if (ly == 0u) {
        let hy = select(0u, gy - 1u, gy > 0u);
        tile_cache[0u * 18u + tile_x] = select(0.0, input_pixels[hy * u.width + gx], in_bounds && gy > 0u);
    }
    if (ly == 15u) {
        let hy = select(gy, gy + 1u, gy + 1u < u.height);
        tile_cache[17u * 18u + tile_x] = select(0.0, input_pixels[hy * u.width + gx], in_bounds && gy + 1u < u.height);
    }

    workgroupBarrier();

    if (!in_bounds) {
        return;
    }

    // Apply 3x3 Gaussian-like smoothing filter:
    // [1, 2, 1]
    // [2, 4, 2] * (1/16)
    // [1, 2, 1]
    var conv: f32 = 0.0;
    conv = conv + tile_cache[(tile_y - 1u) * 18u + (tile_x - 1u)] * 0.0625;
    conv = conv + tile_cache[(tile_y - 1u) * 18u + tile_x] * 0.1250;
    conv = conv + tile_cache[(tile_y - 1u) * 18u + (tile_x + 1u)] * 0.0625;

    conv = conv + tile_cache[tile_y * 18u + (tile_x - 1u)] * 0.1250;
    conv = conv + tile_cache[tile_y * 18u + tile_x] * 0.2500;
    conv = conv + tile_cache[tile_y * 18u + (tile_x + 1u)] * 0.1250;

    conv = conv + tile_cache[(tile_y + 1u) * 18u + (tile_x - 1u)] * 0.0625;
    conv = conv + tile_cache[(tile_y + 1u) * 18u + tile_x] * 0.1250;
    conv = conv + tile_cache[(tile_y + 1u) * 18u + (tile_x + 1u)] * 0.0625;

    output_pixels[gy * u.width + gx] = conv + u.bias;
}
