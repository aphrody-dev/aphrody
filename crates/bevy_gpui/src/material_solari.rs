//! MaterialX (`craft-materialx` OpenPBR / StandardSurface / glTF PBR) -> Naga v30 validated WGSL
//! shader synthesizer + Solari (`bevy_solari` / Lumen-style) Global Illumination settings.

use anyhow::{Context as _, Result};
use craft_materialx::{Document as MtlxDocument, Shader as MtlxShader, Value as MtlxValue};
use naga::valid::{Capabilities, ValidationFlags, Validator};
use serde::{Deserialize, Serialize};

use crate::ecs_world::PbrSurfaceMaterial;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SolariGiSettings {
    pub enabled: bool,
    pub restir_spatial_neighbors: u32,
    pub indirect_bounce_intensity: f32,
    pub sky_ibl_intensity: f32,
    pub ambient_occlusion_strength: f32,
}

impl Default for SolariGiSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            restir_spatial_neighbors: 4,
            indirect_bounce_intensity: 0.65,
            sky_ibl_intensity: 0.55,
            ambient_occlusion_strength: 0.80,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledMaterialShader {
    pub material: PbrSurfaceMaterial,
    pub dialect: String,
    pub wgsl_source: String,
    pub naga_validated: bool,
    pub warnings: Vec<String>,
}

/// Returns a canonical MaterialX 1.39 OpenPBR XML document anchored to Google Design Expressive primary tones.
#[must_use]
pub fn default_openpbr_materialx_xml() -> &'static str {
    r#"<?xml version="1.0"?>
<materialx version="1.39">
  <open_pbr_surface name="SR_GoogleExpressiveOrb" type="surfaceshader">
    <input name="base_color" type="color3" value="0.35, 0.68, 0.98" />
    <input name="base_metalness" type="float" value="0.72" />
    <input name="specular_roughness" type="float" value="0.22" />
    <input name="coat_weight" type="float" value="0.45" />
    <input name="emission_color" type="color3" value="0.01, 0.03, 0.06" />
  </open_pbr_surface>
  <surfacematerial name="M_GoogleExpressiveOrb" type="material">
    <input name="surfaceshader" type="surfaceshader" nodename="SR_GoogleExpressiveOrb" />
  </surfacematerial>
</materialx>
"#
}

/// Parses a MaterialX XML document via `craft-materialx`, extracts OpenPBR/Standard/glTF parameters,
/// emits a complete PBR + Solari GI WGSL shader, and validates it with `naga` v30.0.0.
pub fn compile_materialx_to_wgsl(
    mtlx_bytes: &[u8],
    gi: &SolariGiSettings,
) -> Result<CompiledMaterialShader> {
    let doc = MtlxDocument::parse(mtlx_bytes).context("parsing MaterialX XML document")?;
    let resolved = doc
        .read_material(None)
        .context("resolving surface material from MaterialX document")?;

    let mut mat = PbrSurfaceMaterial {
        name: resolved.name.clone(),
        ..PbrSurfaceMaterial::default()
    };
    let dialect = match resolved.shader {
        MtlxShader::OpenPbr => "open_pbr_surface",
        MtlxShader::StandardSurface => "standard_surface",
        MtlxShader::GltfPbr => "gltf_pbr",
    }
    .to_owned();

    for (input_name, val) in &resolved.inputs {
        if let MtlxValue::Constant { values, .. } = val {
            match input_name.as_str() {
                "base_color" if values.len() >= 3 => {
                    mat.base_color_linear = [values[0], values[1], values[2]];
                }
                "base_metalness" | "metalness" | "metallic" if !values.is_empty() => {
                    mat.metallic = values[0].clamp(0.0, 1.0);
                }
                "specular_roughness" | "roughness" if !values.is_empty() => {
                    mat.roughness = values[0].clamp(0.04, 1.0);
                }
                "coat_weight" | "coat" | "clearcoat" if !values.is_empty() => {
                    mat.clearcoat = values[0].clamp(0.0, 1.0);
                }
                "emission_color" | "emission" if values.len() >= 3 => {
                    mat.emissive_linear = [values[0], values[1], values[2]];
                }
                _ => {}
            }
        }
    }

    let wgsl_source = format!(
        r#"struct FrameUniforms {{
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    sun_dir_lux: vec4<f32>,
    gi_params: vec4<f32>,
}}

@group(0) @binding(0)
var<uniform> frame: FrameUniforms;

struct VertexInput {{
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}}

struct VertexOutput {{
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {{
    var out: VertexOutput;
    out.world_pos = in.position;
    out.world_normal = normalize(in.normal);
    out.uv = in.uv;
    // D3D12 / Metal / WebGPU clip-space depth [0, 1] invariant
    out.clip_pos = frame.view_proj * vec4<f32>(in.position, 1.0);
    return out;
}}

fn ggx_d(n_dot_h: f32, roughness: f32) -> f32 {{
    let a = roughness * roughness;
    let a2 = a * a;
    let denom = (n_dot_h * n_dot_h) * (a2 - 1.0) + 1.0;
    return a2 / max(3.14159265 * denom * denom, 0.0001);
}}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {{
    let base_color = vec3<f32>({:.4}, {:.4}, {:.4});
    let metallic = {:.4};
    let roughness = {:.4};
    let clearcoat = {:.4};
    let emissive = vec3<f32>({:.4}, {:.4}, {:.4});
    let gi_indirect = {:.4};
    let sky_ibl = {:.4};

    let n = normalize(in.world_normal);
    let v = normalize(frame.camera_pos.xyz - in.world_pos);
    let l = normalize(frame.sun_dir_lux.xyz);
    let h = normalize(v + l);

    let n_dot_l = max(dot(n, l), 0.0);
    let n_dot_v = max(dot(n, v), 0.001);
    let n_dot_h = max(dot(n, h), 0.0);
    let v_dot_h = max(dot(v, h), 0.0);

    let f0 = mix(vec3<f32>(0.04, 0.04, 0.04), base_color, metallic);
    let fresnel = f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - v_dot_h, 5.0);
    let spec = ggx_d(n_dot_h, roughness) * fresnel / max(4.0 * n_dot_v, 0.05);
    let diffuse = (vec3<f32>(1.0) - fresnel) * (1.0 - metallic) * base_color / 3.14159265;

    let sky_hemi = mix(vec3<f32>(0.05, 0.07, 0.11), vec3<f32>(0.36, 0.58, 0.88), n.y * 0.5 + 0.5);
    let solari_bounce = sky_hemi * base_color * (gi_indirect + sky_ibl) + fresnel * clearcoat * 0.18;
    let radiance = (diffuse + spec) * n_dot_l * frame.sun_dir_lux.w + solari_bounce + emissive;
    let mapped = radiance / (radiance + vec3<f32>(1.0));
    return vec4<f32>(pow(mapped, vec3<f32>(1.0 / 2.2)), 1.0);
}}
"#,
        mat.base_color_linear[0],
        mat.base_color_linear[1],
        mat.base_color_linear[2],
        mat.metallic,
        mat.roughness,
        mat.clearcoat,
        mat.emissive_linear[0],
        mat.emissive_linear[1],
        mat.emissive_linear[2],
        if gi.enabled {
            gi.indirect_bounce_intensity
        } else {
            0.0
        },
        gi.sky_ibl_intensity,
    );

    let module = naga::front::wgsl::parse_str(&wgsl_source)
        .context("Naga v30 WGSL frontend failed on generated MaterialX shader")?;
    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::default());
    validator
        .validate(&module)
        .context("Naga v30 IR validator rejected generated MaterialX shader")?;

    Ok(CompiledMaterialShader {
        material: mat,
        dialect,
        wgsl_source,
        naga_validated: true,
        warnings: resolved.warnings,
    })
}
