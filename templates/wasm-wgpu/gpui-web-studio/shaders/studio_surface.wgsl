// Validated by Naga v30 — GPUI WebGPU Studio Surface Shader
struct StudioUniforms {
    surface: vec4<f32>,
    container: vec4<f32>,
    primary: vec4<f32>,
    resolution: vec2<f32>,
    time_secs: f32,
    corner_radius: f32,
}

@group(0) @binding(0)
var<uniform> u: StudioUniforms;

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOut {
    var verts = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(3.0, 1.0),
        vec2<f32>(-1.0, 1.0),
    );
    let pos = verts[idx % 3u];
    var out: VertexOut;
    out.clip_position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = pos * 0.5 + vec2<f32>(0.5, 0.5);
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let gradient = clamp(in.uv.y * 0.7 + 0.15, 0.0, 1.0);
    let base = mix(u.surface.rgb, u.container.rgb, gradient);
    let accent = smoothstep(0.45, 0.0, length(in.uv - vec2<f32>(0.5, 0.5)));
    return vec4<f32>(mix(base, u.primary.rgb, accent * 0.25), 1.0);
}
