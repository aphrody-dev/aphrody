//! NSH Reactive WebGPU Shader uniform helpers.

pub fn normalized_spring_response(stiffness: f32, t_secs: f32) -> f32 {
    if t_secs <= 0.0 || stiffness <= 0.0 {
        return 0.0;
    }
    let omega = stiffness.sqrt();
    1.0 - (-omega * t_secs).exp() * (1.0 + omega * t_secs)
}
