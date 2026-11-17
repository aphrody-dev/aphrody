// Validated by Naga v30 — NSH Expressive Wave WGSL Shader
struct WaveParams {
    phase: f32,
    amplitude: f32,
    pad0: f32,
    pad1: f32,
    primary: vec4<f32>,
    tertiary: vec4<f32>,
    surface: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> wave: WaveParams;

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
    var tri = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(3.0, 1.0),
        vec2<f32>(-1.0, 1.0),
    );
    return vec4<f32>(tri[idx % 3u], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    let band = 0.5 + 0.5 * sin(frag_coord.x * 0.015 + wave.phase) * wave.amplitude;
    let tint = mix(wave.primary.rgb, wave.tertiary.rgb, band);
    return vec4<f32>(mix(wave.surface.rgb, tint, 0.35), 1.0);
}
