//! Standalone WGPU Compute Particles configuration and M3 Expressive uniform helper.

pub const SURFACE_RGBA: [f32; 4] = [0.0549, 0.0784, 0.1059, 1.0];
pub const PRIMARY_RGBA: [f32; 4] = [0.5608, 0.8157, 1.0000, 1.0];

pub fn particle_step_velocity(px: f32, py: f32, dt_secs: f32) -> (f32, f32) {
    let vx = -py * 0.65;
    let vy = px * 0.65;
    (px + vx * dt_secs, py + vy * dt_secs)
}
