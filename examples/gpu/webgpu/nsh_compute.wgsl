// NSH WebGPU Compute Shader Example
//
// Computes an arithmetic sequence into storage memory.
// Bound as `@group(0) @binding(0)` storage buffer for headless validation with `gpui-wgpu compute`.

@group(0) @binding(0) var<storage, read_write> output_data: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3u) {
    let index = id.x;
    if (index >= arrayLength(&output_data)) {
        return;
    }
    output_data[index] = f32(index) * 2.0 + 10.0;
}
