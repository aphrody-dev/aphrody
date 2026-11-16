// NSH Ripple Shader - Strict W3C WebGPU & Naga v30 Compliant WGSL
//
// Demonstrates:
// 1. Host-shareable uniform layout adhering to 16-byte alignment rules.
// 2. Fullscreen quad vertex generation with normalized UV coordinates.
// 3. Perceptual concentric ripple synthesis with Oklab color mapping.
// 4. Uniformity-compliant execution for all target backends.

struct RippleUniforms {
    resolution: vec2f, // View resolution in pixels [width, height]
    time: f32,         // Elapsed time in seconds
    scale: f32,        // Display DPI scale factor
    origin: vec2f,     // Center of the ripple in normalized UV [0..1]
    frequency: f32,    // Radial wave frequency
    decay: f32,        // Exponential radial decay factor
}

@group(0) @binding(0) var<uniform> u_ripple: RippleUniforms;

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_id: u32) -> VertexOutput {
    var out: VertexOutput;
    // Procedural fullscreen triangle covering [-1, 1] NDC
    let x = f32(i32(vertex_id << 1u) & 2) * 2.0 - 1.0;
    let y = f32(i32(vertex_id & 2u)) * 2.0 - 1.0;
    out.position = vec4f(x, y, 0.0, 1.0);
    out.uv = vec2f(x * 0.5 + 0.5, 1.0 - (y * 0.5 + 0.5));
    return out;
}

fn oklab_to_srgb(c: vec3f) -> vec3f {
    let l_ = c.x + 0.3963377774 * c.y + 0.2158037573 * c.z;
    let m_ = c.x - 0.1055613458 * c.y - 0.0638541728 * c.z;
    let s_ = c.x - 0.0894841775 * c.y - 1.2914855480 * c.z;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    return vec3f(
        4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s
    );
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let aspect = u_ripple.resolution.x / max(u_ripple.resolution.y, 1.0);
    var p = in.uv - u_ripple.origin;
    p.x = p.x * aspect;

    let dist = length(p);
    let speed = 2.5;
    let wave_phase = dist * u_ripple.frequency - u_ripple.time * speed;
    let wave = sin(wave_phase) * exp(-dist * u_ripple.decay);

    // Dynamic wave highlight and shadow
    let base_lightness = 0.5;
    let lightness = clamp(base_lightness + 0.35 * wave, 0.0, 1.0);
    let chroma_a = 0.15 * cos(u_ripple.time * 0.5 + dist * 4.0);
    let chroma_b = 0.15 * sin(u_ripple.time * 0.5 + dist * 4.0);

    let oklab = vec3f(lightness, chroma_a, chroma_b);
    let srgb = clamp(oklab_to_srgb(oklab), vec3f(0.0), vec3f(1.0));

    return vec4f(srgb, 1.0);
}
