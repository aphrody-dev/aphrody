// Validated by Naga v30 — shadcn / M3 Card Surface Shader
struct CardUniforms {
    background: vec4<f32>,
    card: vec4<f32>,
    primary: vec4<f32>,
    border: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> theme: CardUniforms;

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
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let highlight = clamp(1.0 - pos.y * 0.001, 0.0, 1.0) * 0.08;
    return vec4<f32>(mix(theme.card.rgb, theme.primary.rgb, highlight), 1.0);
}
