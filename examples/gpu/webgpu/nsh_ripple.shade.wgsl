// NSH Custom Shade Snippet: `fn shade(uv: vec2f) -> vec4f`
//
// In NSH (Native Shell), apps do not author vertex shaders or entry point annotations.
// An app authors a single fragment function `shade(uv: vec2f) -> vec4f` that returns
// straight sRGB color with alpha.
//
// The NSH engine (`crates/nsh-core/src/shader.rs`) automatically wraps this source:
// 1. Adds the builtin uniform struct at `@group(0) @binding(0) var<uniform> nsh: Nsh;`
//    providing `nsh.size`, `nsh.time`, `nsh.scale`, `nsh.progress`, `nsh.overflow`.
// 2. Adds user uniforms at `@group(0) @binding(1) var<uniform> uniforms: Uniforms;`
// 3. Adds texture sampler and pictures at `@group(0) @binding(2..6)`.
// 4. Generates `vs_main` (quad) and `fs_main` (alpha premultiplication).

struct Uniforms {
    origin: vec2f,
    frequency: f32,
    decay: f32,
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

fn shade(uv: vec2f) -> vec4f {
    let aspect = nsh.size.x / max(nsh.size.y, 1.0);
    var p = uv - uniforms.origin;
    p.x = p.x * aspect;

    let dist = length(p);
    let speed = 2.5;
    let wave_phase = dist * uniforms.frequency - nsh.time * speed;
    let wave = sin(wave_phase) * exp(-dist * uniforms.decay);

    let lightness = clamp(0.5 + 0.35 * wave, 0.0, 1.0);
    let chroma_a = 0.15 * cos(nsh.time * 0.5 + dist * 4.0);
    let chroma_b = 0.15 * sin(nsh.time * 0.5 + dist * 4.0);

    let oklab = vec3f(lightness, chroma_a, chroma_b);
    let srgb = clamp(oklab_to_srgb(oklab), vec3f(0.0), vec3f(1.0));

    // Return straight sRGB with alpha:
    return vec4f(srgb, 1.0);
}
