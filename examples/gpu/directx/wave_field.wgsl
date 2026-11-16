// WGSL 2D Wave Field Simulation Compute Shader
// Suitable for transpilation to HLSL SM 6.6 via Naga in gpui-os

struct SimulationParams {
    grid_dim: vec2<u32>,
    damping: f32,
    wave_speed_sq: f32,
    delta_time: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0)
var<uniform> params: SimulationParams;

@group(0) @binding(1)
var<storage, read> prev_state: array<f32>;

@group(0) @binding(2)
var<storage, read> curr_state: array<f32>;

@group(0) @binding(3)
var<storage, read_write> next_state: array<f32>;

fn get_index(x: u32, y: u32) -> u32 {
    return y * params.grid_dim.x + x;
}

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;

    if (x >= params.grid_dim.x || y >= params.grid_dim.y) {
        return;
    }

    let idx = get_index(x, y);

    // Boundary conditions: clamp to zero at edges
    if (x == 0u || x == params.grid_dim.x - 1u || y == 0u || y == params.grid_dim.y - 1u) {
        next_state[idx] = 0.0;
        return;
    }

    // 5-point discrete 2D Laplacian stencil
    let center = curr_state[idx];
    let left   = curr_state[get_index(x - 1u, y)];
    let right  = curr_state[get_index(x + 1u, y)];
    let up     = curr_state[get_index(x, y - 1u)];
    let down   = curr_state[get_index(x, y + 1u)];

    let laplacian = (left + right + up + down) - (4.0 * center);

    // Finite difference wave equation update:
    // u(t+dt) = 2*u(t) - u(t-dt) + c^2 * dt^2 * laplacian - damping * dt * velocity
    let prev = prev_state[idx];
    let velocity = (center - prev) / params.delta_time;
    let acceleration = params.wave_speed_sq * laplacian - (params.damping * velocity);

    let next = (2.0 * center) - prev + (acceleration * params.delta_time * params.delta_time);

    next_state[idx] = next;
}
