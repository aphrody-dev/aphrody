// Validated by Naga v30 (`gpui-wgpu validate`) — Material 3 Expressive particles shader
struct SimUniforms {
    viewport: vec2<f32>,
    time_secs: f32,
    pad: f32,
    surface: vec4<f32>,
    primary: vec4<f32>,
    tertiary: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> sim: SimUniforms;

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(3.0, 1.0),
        vec2<f32>(-1.0, 1.0),
    );
    let pos = positions[idx % 3u];
    var out: VertexOut;
    out.clip_position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = pos * 0.5 + vec2<f32>(0.5, 0.5);
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let surface_rgb = select(vec3<f32>(0.055, 0.078, 0.106), sim.surface.rgb, sim.surface.w > 0.0);
    let primary_rgb = select(vec3<f32>(0.561, 0.816, 1.0), sim.primary.rgb, sim.primary.w > 0.0);
    let tertiary_rgb = select(vec3<f32>(0.776, 0.761, 1.0), sim.tertiary.rgb, sim.tertiary.w > 0.0);
    let center = vec2<f32>(0.5 + 0.18 * cos(sim.time_secs), 0.5 + 0.18 * sin(sim.time_secs));
    let dist = distance(in.uv, center);
    let glow = smoothstep(0.28, 0.0, dist);
    let color = mix(surface_rgb, primary_rgb, glow * 0.85) + tertiary_rgb * (glow * 0.15);
    return vec4<f32>(color, 1.0);
}

@compute @workgroup_size(64, 1, 1)
fn cs_step(@builtin(global_invocation_id) gid: vec3<u32>) {
    let _index = gid.x;
}
