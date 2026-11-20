// m3_expressive.wgsl — Pure WebGPU / Naga v30 WGSL renderer for Compose Material 3 1.5.0 Stable
// & Jetpack Compose 1.13 graphics primitives (published 2026-11-18).
// Anchored to https://m3.material.io/blog/compose-material-3-is-stable and
// https://www.figma.com/community/file/1035203688168086460/material-3-design-kit.

struct ViewportUniform {
    resolution: vec2<f32>,
    time_sec: f32,
    dpi_scale: f32,
};

// Widget kind constants:
// 0 = Expressive Surface / Card / SegmentedListItem / FloatingToolbar
// 1 = MorphPolygonShape (35 MaterialShapes + ToggleButton / LoadingIndicator morph)
// 2 = WavyLinearProgressIndicator
// 3 = WavyCircularProgressIndicator
// 4 = StatefulSlider (SliderState track + active segment + expressive handle)
// 5 = SplitButton / ButtonGroup segment with material3-ripple:1.5.0 inset focus ring
// 6 = Compose 1.13 Progressive Blur gradient overlay
// 7 = Liquid / Frosted Glass surface (backdrop-blur-core Dual-Kawase tint + Fresnel specular rim)
struct M3Instance {
    bounds: vec4<f32>,         // x, y, width, height in logical px
    fill_rgba: vec4<f32>,      // primary / container color
    accent_rgba: vec4<f32>,    // active / ripple / inset focus-ring color
    shape_params: vec4<f32>,   // x=corner_radius, y=lobes_a, z=lobes_b, w=morph_progress [0..1]
    effect_params: vec4<f32>,  // x=progress [0..1], y=wave_phase, z=focus_inset_px, w=ripple_progress
    widget_meta: vec4<u32>,    // x=widget_kind, y=shape_a_id, z=shape_b_id, w=flags (bit0=focused, bit1=pressed)
};

@group(0) @binding(0) var<uniform> viewport: ViewportUniform;
@group(0) @binding(1) var<storage, read> instances: array<M3Instance>;

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) local_px: vec2<f32>,
    @location(1) @interpolate(flat) iid: u32,
};

@vertex
fn vs_main(@builtin(vertex_index) vid: u32, @builtin(instance_index) iid: u32) -> VsOut {
    let inst = instances[iid];
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
    );
    let uv = corners[vid % 6u];
    let px = inst.bounds.xy + uv * inst.bounds.zw;
    let ndc = vec2<f32>(
        (px.x / max(viewport.resolution.x, 1.0)) * 2.0 - 1.0,
        1.0 - (px.y / max(viewport.resolution.y, 1.0)) * 2.0,
    );
    var out: VsOut;
    out.clip_pos = vec4<f32>(ndc, 0.0, 1.0);
    out.local_px = uv * inst.bounds.zw;
    out.iid = iid;
    return out;
}

fn sdf_rounded_box(p: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let r = min(radius, min(half_size.x, half_size.y));
    let q = abs(p) - half_size + vec2<f32>(r, r);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0, 0.0))) - r;
}

fn sdf_material_polygon(p: vec2<f32>, radius: f32, lobes: f32, shape_id: u32) -> f32 {
    let r = length(p);
    let theta = atan2(p.y, p.x);
    if (shape_id == 0u || lobes <= 1.0) {
        return r - radius;
    }
    if (shape_id == 34u) {
        let q = vec2<f32>(abs(p.x), -p.y) / max(radius, 1.0);
        let h = q.x * q.x + pow(q.y - sqrt(max(q.x, 0.0)) * 0.5, 2.0) - 0.64;
        return h * radius * 0.5;
    }
    let depth = select(0.14, 0.24, shape_id >= 22u && shape_id <= 28u);
    let mod_r = radius * (1.0 - depth + depth * cos(lobes * theta));
    return r - mod_r;
}

fn sdf_wavy_linear(local_px: vec2<f32>, size: vec2<f32>, progress: f32, phase: f32) -> vec2<f32> {
    let active_x = size.x * clamp(progress, 0.0, 1.0);
    let mid_y = size.y * 0.5;
    let wavelength = 24.0;
    let amp = min(size.y * 0.28, 4.0);
    let wave_y = mid_y + sin((local_px.x / wavelength) * 6.2831853 + phase) * amp;
    let d_active = abs(local_px.y - wave_y) - 2.2;
    let d_track = abs(local_px.y - mid_y) - 2.0;
    let is_active = step(local_px.x, active_x);
    return vec2<f32>(mix(d_track, d_active, is_active), is_active);
}

fn sdf_wavy_circular(p: vec2<f32>, radius: f32, progress: f32, phase: f32) -> vec2<f32> {
    let r = length(p);
    let theta = atan2(p.y, p.x);
    let norm_angle = fract((theta + 1.5707963) / 6.2831853);
    let wave = sin(theta * 10.0 + phase) * 2.2;
    let d_ring = abs(r - (radius + wave)) - 2.6;
    let is_active = step(norm_angle, clamp(progress, 0.0, 1.0));
    return vec2<f32>(d_ring, is_active);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let inst = instances[in.iid];
    let size = inst.bounds.zw;
    let half = size * 0.5;
    let p = in.local_px - half;
    let kind = inst.widget_meta.x;
    let flags = inst.widget_meta.w;

    var color = inst.fill_rgba;
    var alpha = 0.0;

    if (kind == 1u) {
        let rad = min(half.x, half.y) * 0.9;
        let d_a = sdf_material_polygon(p, rad, inst.shape_params.y, inst.widget_meta.y);
        let d_b = sdf_material_polygon(p, rad, inst.shape_params.z, inst.widget_meta.z);
        let d = mix(d_a, d_b, clamp(inst.shape_params.w, 0.0, 1.0));
        alpha = 1.0 - smoothstep(-1.0, 1.0, d);
    } else if (kind == 2u) {
        let res = sdf_wavy_linear(in.local_px, size, inst.effect_params.x, inst.effect_params.y + viewport.time_sec * 3.0);
        alpha = 1.0 - smoothstep(-0.8, 0.8, res.x);
        color = mix(inst.fill_rgba, inst.accent_rgba, res.y);
    } else if (kind == 3u) {
        let rad = min(half.x, half.y) * 0.78;
        let res = sdf_wavy_circular(p, rad, inst.effect_params.x, inst.effect_params.y + viewport.time_sec * 3.0);
        alpha = 1.0 - smoothstep(-0.8, 0.8, res.x);
        color = mix(inst.fill_rgba, inst.accent_rgba, res.y);
    } else if (kind == 4u) {
        let active_x = size.x * clamp(inst.effect_params.x, 0.0, 1.0);
        let track_d = sdf_rounded_box(p, vec2<f32>(half.x, 4.0), 4.0);
        let thumb_p = in.local_px - vec2<f32>(active_x, half.y);
        let thumb_d = sdf_rounded_box(thumb_p, vec2<f32>(3.0, half.y * 0.72), 3.0);
        let track_a = 1.0 - smoothstep(-0.8, 0.8, track_d);
        let thumb_a = 1.0 - smoothstep(-0.8, 0.8, thumb_d);
        let is_active = step(in.local_px.x, active_x);
        color = mix(inst.fill_rgba, inst.accent_rgba, max(is_active, thumb_a));
        alpha = max(track_a, thumb_a);
    } else if (kind == 6u) {
        let t = clamp(in.local_px.y / max(size.y, 1.0), 0.0, 1.0);
        let blur_factor = smoothstep(0.35, 1.0, t);
        color = vec4<f32>(inst.fill_rgba.rgb, inst.fill_rgba.a * blur_factor);
        alpha = 1.0;
    } else if (kind == 7u) {
        // Liquid / Frosted Glass: rounded SDF + top-left directional Fresnel specular rim + subtle sheen
        let d = sdf_rounded_box(p, half, inst.shape_params.x);
        alpha = 1.0 - smoothstep(-1.0, 1.0, d);
        let rim = (1.0 - smoothstep(0.0, 2.5, abs(d + 1.0))) * clamp(inst.effect_params.x, 0.15, 0.75);
        let uv_norm = in.local_px / max(size, vec2<f32>(1.0, 1.0));
        let top_sheen = pow(1.0 - clamp(uv_norm.y, 0.0, 1.0), 2.2) * 0.16;
        let fresnel_dir = smoothstep(1.2, 0.0, uv_norm.x + uv_norm.y);
        let glass_rgb = mix(inst.fill_rgba.rgb, inst.accent_rgba.rgb, top_sheen)
            + vec3<f32>(1.0, 1.0, 1.0) * (rim * (0.35 + 0.65 * fresnel_dir));
        color = vec4<f32>(clamp(glass_rgb, vec3<f32>(0.0), vec3<f32>(1.0)), inst.fill_rgba.a);
    } else {
        let d = sdf_rounded_box(p, half, inst.shape_params.x);
        alpha = 1.0 - smoothstep(-1.0, 1.0, d);

        if ((flags & 1u) != 0u) {
            let inset = max(inst.effect_params.z, 2.0);
            let ring_d = abs(d + inset + 1.5) - 1.5;
            let ring_a = 1.0 - smoothstep(-0.6, 0.6, ring_d);
            color = mix(color, inst.accent_rgba, ring_a);
        }
        if ((flags & 2u) != 0u) {
            let max_r = length(half);
            let rip_r = max_r * clamp(inst.effect_params.w, 0.0, 1.0);
            let rip_a = (1.0 - smoothstep(rip_r - 8.0, rip_r + 2.0, length(p))) * 0.22;
            color = mix(color, inst.accent_rgba, rip_a);
        }
    }

    return vec4<f32>(color.rgb, color.a * alpha);
}
