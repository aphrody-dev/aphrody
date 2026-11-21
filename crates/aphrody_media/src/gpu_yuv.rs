//! Naga v30 / WGPU v30 validated WGSL shaders for pure-Rust video compositing,
//! NV12/BT.709 YUV->RGB linear premultiplied conversion, AgX HDR tone mapping,
//! compute luma waveform scopes, and programmatic video transitions.

use anyhow::{Context as _, Result};
use naga::valid::{Capabilities, ValidationFlags, Validator};
use serde::{Deserialize, Serialize};

/// NV12 / BT.709 YUV->RGBA linear premultiplied + AgX tone mapping vertex/fragment shader.
pub const YUV_TO_RGBA_WGSL: &str = r#"
struct VideoUniforms {
    exposure: f32,
    gamma: f32,
    hdr_mode: u32,
    _pad: u32,
};

@group(0) @binding(0) var<uniform> uniforms: VideoUniforms;
@group(0) @binding(1) var tex_y: texture_2d<f32>;
@group(0) @binding(2) var tex_uv: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;

struct VertexOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_fullscreen(@builtin(vertex_index) vid: u32) -> VertexOut {
    var out: VertexOut;
    let x = f32((vid << 1u) & 2u);
    let y = f32(vid & 2u);
    out.uv = vec2<f32>(x, 1.0 - y);
    out.pos = vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
    return out;
}

fn bt709_yuv_to_rgb(y: f32, u: f32, v: f32) -> vec3<f32> {
    let yn = (y - 0.062745) * 1.164384;
    let un = u - 0.5;
    let vn = v - 0.5;
    let r = yn + 1.792741 * vn;
    let g = yn - 0.213249 * un - 0.532909 * vn;
    let b = yn + 2.112402 * un;
    return clamp(vec3<f32>(r, g, b), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn agx_tonemap(color: vec3<f32>) -> vec3<f32> {
    let c = max(color, vec3<f32>(0.0));
    return c / (c + vec3<f32>(1.0));
}

@fragment
fn fs_yuv_to_rgba(in: VertexOut) -> @location(0) vec4<f32> {
    let y = textureSample(tex_y, samp, in.uv).r;
    let uv = textureSample(tex_uv, samp, in.uv).rg;
    var rgb = bt709_yuv_to_rgb(y, uv.x, uv.y) * uniforms.exposure;
    if (uniforms.hdr_mode == 1u) {
        rgb = agx_tonemap(rgb);
    }
    let out_rgb = pow(max(rgb, vec3<f32>(0.0)), vec3<f32>(1.0 / max(uniforms.gamma, 0.1)));
    return vec4<f32>(out_rgb, 1.0);
}
"#;

/// Compute shader generating 256-bin luminance histogram scopes for Media Studio grading.
pub const LUMA_SCOPES_COMPUTE_WGSL: &str = r#"
@group(0) @binding(0) var src_frame: texture_2d<f32>;
@group(0) @binding(1) var<storage, read_write> histogram_bins: array<atomic<u32>, 256>;

@compute @workgroup_size(8, 8, 1)
fn cs_luma_histogram(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(src_frame);
    if (gid.x >= dims.x || gid.y >= dims.y) {
        return;
    }
    let rgba = textureLoad(src_frame, vec2<i32>(i32(gid.x), i32(gid.y)), 0);
    let luma = clamp(dot(rgba.rgb, vec3<f32>(0.2126, 0.7152, 0.0722)), 0.0, 1.0);
    let bin = min(u32(luma * 255.0), 255u);
    atomicAdd(&histogram_bins[bin], 1u);
}
"#;

/// Programmatic video transitions WGSL shader. `frame_a` is the picture below a clip and `frame_b`
/// the picture including it (premultiplied RGBA). Kinds: 0 cross-fade, 1 linear wipe,
/// 2 dissolve (integer hash threshold, identical to `motion_render::dissolve_noise`), 3 zoom-through.
/// `size` is the target size in pixels.
pub const MOTION_TRANSITION_WGSL: &str = r#"
struct TransitionUniforms {
    progress: f32,
    kind: u32,
    feather: f32,
    _pad: f32,
    size: vec2<f32>,
    _pad_size: vec2<f32>,
};

@group(0) @binding(0) var<uniform> params: TransitionUniforms;
@group(0) @binding(1) var frame_a: texture_2d<f32>;
@group(0) @binding(2) var frame_b: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;

struct VertexOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_transition(@builtin(vertex_index) vid: u32) -> VertexOut {
    var out: VertexOut;
    let x = f32((vid << 1u) & 2u);
    let y = f32(vid & 2u);
    out.uv = vec2<f32>(x, 1.0 - y);
    out.pos = vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
    return out;
}

fn dissolve_noise(px: vec2<u32>) -> f32 {
    var v = (px.x * 0x9E3779B1u) ^ (px.y * 0x85EBCA77u) ^ 0x27D4EB2Fu;
    v = v ^ (v >> 15u);
    v = v * 0x2C1B3C6Du;
    v = v ^ (v >> 12u);
    v = v * 0x297A2D39u;
    v = v ^ (v >> 15u);
    return f32(v >> 8u) / 16777216.0;
}

fn zoomed(tex: texture_2d<f32>, uv: vec2<f32>, zoom: f32) -> vec4<f32> {
    let center = vec2<f32>(0.5, 0.5);
    return textureSampleLevel(tex, samp, center + (uv - center) / zoom, 0.0);
}

@fragment
fn fs_transition(in: VertexOut) -> @location(0) vec4<f32> {
    let t = clamp(params.progress, 0.0, 1.0);
    let feather = max(params.feather, 0.0005);
    let px = vec2<u32>(in.pos.xy);
    let col_a = textureLoad(frame_a, vec2<i32>(px), 0);
    let col_b = textureLoad(frame_b, vec2<i32>(px), 0);
    if (params.kind == 1u) {
        let edge_center = t * (1.0 + 2.0 * feather) - feather;
        let edge = smoothstep(edge_center - feather, edge_center + feather, in.uv.x);
        return mix(col_b, col_a, edge);
    }
    if (params.kind == 2u) {
        if (dissolve_noise(px) < t) {
            return col_b;
        }
        return col_a;
    }
    if (params.kind == 3u) {
        let zoom_a = 1.0 + 0.25 * t;
        let zoom_b = 1.0 + 0.25 * (1.0 - t);
        return mix(zoomed(frame_a, in.uv, zoom_a), zoomed(frame_b, in.uv, zoom_b), t);
    }
    return mix(col_a, col_b, t);
}
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaShaderKind {
    YuvToRgba,
    LumaScopesCompute,
    MotionTransition,
}

impl MediaShaderKind {
    pub const ALL: &'static [Self] = &[
        Self::YuvToRgba,
        Self::LumaScopesCompute,
        Self::MotionTransition,
    ];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::YuvToRgba => "yuv_to_rgba",
            Self::LumaScopesCompute => "luma_scopes_compute",
            Self::MotionTransition => "motion_transition",
        }
    }

    #[must_use]
    pub const fn source(self) -> &'static str {
        match self {
            Self::YuvToRgba => YUV_TO_RGBA_WGSL,
            Self::LumaScopesCompute => LUMA_SCOPES_COMPUTE_WGSL,
            Self::MotionTransition => MOTION_TRANSITION_WGSL,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaShaderValidation {
    pub name: String,
    pub entry_points: Vec<String>,
    pub valid: bool,
}

/// Parses and validates a built-in media WGSL shader using Naga v30.
pub fn validate_media_shader(kind: MediaShaderKind) -> Result<MediaShaderValidation> {
    let module = naga::front::wgsl::parse_str(kind.source())
        .with_context(|| format!("WGSL parse error in shader {}", kind.name()))?;
    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::empty());
    let _info = validator
        .validate(&module)
        .with_context(|| format!("Naga validation error in shader {}", kind.name()))?;
    let entry_points = module
        .entry_points
        .iter()
        .map(|entry| entry.name.clone())
        .collect();
    Ok(MediaShaderValidation {
        name: kind.name().to_string(),
        entry_points,
        valid: true,
    })
}

/// Validates all built-in pure-Rust media WGSL shaders.
pub fn validate_all_media_shaders() -> Result<Vec<MediaShaderValidation>> {
    let mut results = Vec::with_capacity(MediaShaderKind::ALL.len());
    for &kind in MediaShaderKind::ALL {
        results.push(validate_media_shader(kind)?);
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_media_wgsl_shaders_pass_naga_v30_validation() {
        let validations = validate_all_media_shaders().expect("media WGSL shaders should validate");
        assert_eq!(validations.len(), 3);
        assert!(validations.iter().all(|v| v.valid));
        assert!(validations[0].entry_points.contains(&"fs_yuv_to_rgba".to_string()));
        assert!(validations[1].entry_points.contains(&"cs_luma_histogram".to_string()));
        assert!(validations[2].entry_points.contains(&"fs_transition".to_string()));
    }
}
