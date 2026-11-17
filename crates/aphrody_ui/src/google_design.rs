//! Material 3 Expressive & Google Design (`https://design.google/`) runtime token API,
//! analytical physics springs, `GoogleSansFlexAxes`, responsive size classes, CSS/WGSL helpers.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use anyhow::{Context as _, Result};
use gpui::{Hsla, Rgba};
use serde::{Deserialize, Serialize};

use crate::cosmic_theme::{AppearanceMode, contrast_ratio};

/// Authoritative URL for Aphrody's visual design system.
pub const DESIGN_SOURCE_OF_TRUTH: &str = brand::DESIGN_SOURCE_OF_TRUTH;

/// Embedded 3-tier Material 3 Expressive + Google Sans Flex token manifest (`assets/os/google_design_tokens.toml`).
pub const BUNDLED_GOOGLE_DESIGN_TOKENS_TOML: &str =
    include_str!("../../../assets/os/google_design_tokens.toml");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowSizeClass {
    Compact,
    Medium,
    Expanded,
    Large,
    ExtraLarge,
}

impl WindowSizeClass {
    #[must_use]
    pub fn from_width_px(width_px: f32) -> Self {
        if width_px < 600.0 {
            Self::Compact
        } else if width_px < 840.0 {
            Self::Medium
        } else if width_px < 1200.0 {
            Self::Expanded
        } else if width_px < 1600.0 {
            Self::Large
        } else {
            Self::ExtraLarge
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpringSpec {
    pub stiffness: f32,
    pub damping_ratio: f32,
}

impl SpringSpec {
    /// Evaluates normalized displacement `0.0 -> 1.0` at time `t_secs` for a unit-mass
    /// damped harmonic oscillator anchored to Material 3 Expressive motion physics.
    #[must_use]
    pub fn sample_position(&self, t_secs: f32) -> f32 {
        if t_secs <= 0.0 || self.stiffness <= 0.0 {
            return 0.0;
        }
        let omega0 = self.stiffness.sqrt();
        let zeta = self.damping_ratio.max(0.05);
        if (zeta - 1.0).abs() < 1e-4 {
            // Critically damped (effects springs: zero bounce overshoot)
            let decay = (-omega0 * t_secs).exp();
            1.0 - decay * (1.0 + omega0 * t_secs)
        } else if zeta < 1.0 {
            // Underdamped (spatial springs: subtle organic settle)
            let omega_d = omega0 * (1.0 - zeta * zeta).sqrt();
            let decay = (-zeta * omega0 * t_secs).exp();
            let ratio = zeta / (1.0 - zeta * zeta).sqrt();
            1.0 - decay * ((omega_d * t_secs).cos() + ratio * (omega_d * t_secs).sin())
        } else {
            // Overdamped
            let s = (zeta * zeta - 1.0).sqrt();
            let r1 = -omega0 * (zeta - s);
            let r2 = -omega0 * (zeta + s);
            let denom = r1 - r2;
            if denom.abs() < 1e-6 {
                1.0
            } else {
                1.0 - (r2 * (r1 * t_secs).exp() - r1 * (r2 * t_secs).exp()) / -denom
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GoogleSansFlexAxes {
    pub opsz: f32,
    pub wght: f32,
    pub wdth: f32,
    pub grad: f32,
    pub slnt: f32,
    pub rond: f32,
}

impl Default for GoogleSansFlexAxes {
    fn default() -> Self {
        Self {
            opsz: 14.0,
            wght: 400.0,
            wdth: 100.0,
            grad: 0.0,
            slnt: 0.0,
            rond: 0.0,
        }
    }
}

impl GoogleSansFlexAxes {
    #[must_use]
    pub fn css_font_variation_settings(&self) -> String {
        format!(
            "\"opsz\" {:.1}, \"wght\" {:.0}, \"wdth\" {:.1}, \"GRAD\" {:.0}, \"slnt\" {:.1}, \"ROND\" {:.0}",
            self.opsz.clamp(6.0, 144.0),
            self.wght.clamp(100.0, 1000.0),
            self.wdth.clamp(25.0, 151.0),
            self.grad.clamp(-50.0, 200.0),
            self.slnt.clamp(-10.0, 0.0),
            self.rond.clamp(0.0, 100.0),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableAxisSpec {
    pub tag: String,
    pub name: String,
    pub min: f32,
    pub default_value: f32,
    pub max: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefTokens {
    pub palette: BTreeMap<String, String>,
    pub typeface: BTreeMap<String, String>,
    #[serde(default)]
    pub variable_axes: BTreeMap<String, VariableAxisSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeScale {
    pub none_px: f32,
    pub extra_small_px: f32,
    pub small_px: f32,
    pub medium_px: f32,
    pub large_px: f32,
    pub large_increased_px: f32,
    pub extra_large_px: f32,
    pub full_px: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateLayerOpacities {
    pub hover_opacity: f32,
    pub focus_opacity: f32,
    pub pressed_opacity: f32,
    pub dragged_opacity: f32,
    pub disabled_container_opacity: f32,
    pub disabled_content_opacity: f32,
    pub focus_ring_px: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionTokens {
    pub springs: BTreeMap<String, SpringSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorPaletteMap {
    pub light: BTreeMap<String, String>,
    pub dark: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SysTokens {
    pub color: ColorPaletteMap,
    pub motion: MotionTokens,
    pub shape: ShapeScale,
    pub state: StateLayerOpacities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleDesignTokens {
    pub schema_version: u32,
    pub source_of_truth: String,
    pub system: String,
    pub spec_url: String,
    pub fonts_url: String,
    #[serde(default)]
    pub stitch_url: String,
    #[serde(default)]
    pub sans_flex_family: String,
    pub synced_at: String,
    pub seed_hex: String,
    pub variant: String,
    pub principles: Vec<String>,
    #[serde(rename = "ref")]
    pub reference: RefTokens,
    pub sys: SysTokens,
}

impl GoogleDesignTokens {
    pub fn bundled() -> Result<Self> {
        toml::from_str(BUNDLED_GOOGLE_DESIGN_TOKENS_TOML)
            .context("parsing bundled google_design_tokens.toml")
    }

    #[must_use]
    pub fn palette(&self, mode: AppearanceMode) -> &BTreeMap<String, String> {
        match mode {
            AppearanceMode::Light => &self.sys.color.light,
            AppearanceMode::Dark => &self.sys.color.dark,
        }
    }

    #[must_use]
    pub fn hsla(&self, mode: AppearanceMode, role: &str) -> Option<Hsla> {
        self.palette(mode)
            .get(role)
            .and_then(|hex| theme::try_parse_color(hex.trim()).ok())
    }

    #[must_use]
    pub fn spring(&self, name: &str) -> Option<SpringSpec> {
        self.sys.motion.springs.get(name).copied()
    }

    #[must_use]
    pub fn to_css_custom_properties(&self, mode: AppearanceMode) -> String {
        let mut out = String::from(":root {\n");
        if let Some(brand_font) = self.reference.typeface.get("brand")
            && let Err(err) = writeln!(out, "  --md-ref-typeface-brand: {brand_font};")
        {
            log::warn!("format err: {err}");
        }
        if let Some(mono_font) = self.reference.typeface.get("mono")
            && let Err(err) = writeln!(out, "  --md-ref-typeface-mono: {mono_font};")
        {
            log::warn!("format err: {err}");
        }
        for (role, hex) in self.palette(mode) {
            let css_role = role.replace('_', "-");
            if let Err(err) = writeln!(out, "  --md-sys-color-{css_role}: {hex};") {
                log::warn!("format err: {err}");
            }
        }
        if let Err(err) = writeln!(
            out,
            "  --md-sys-shape-corner-small: {}px;\n  --md-sys-shape-corner-medium: {}px;\n  --md-sys-shape-corner-large: {}px;\n  --md-sys-shape-corner-extra-large: {}px;\n  --md-sys-state-hover-opacity: {};",
            self.sys.shape.small_px,
            self.sys.shape.medium_px,
            self.sys.shape.large_px,
            self.sys.shape.extra_large_px,
            self.sys.state.hover_opacity
        ) {
            log::warn!("format err: {err}");
        }
        out.push_str("}\n");
        out
    }

    #[must_use]
    pub fn to_wgsl_theme_struct(&self, mode: AppearanceMode) -> String {
        let vec4_of = |role: &str, fallback: [f32; 4]| -> [f32; 4] {
            self.hsla(mode, role).map_or(fallback, |hsla| {
                let rgb: Rgba = hsla.to_rgb();
                [rgb.r, rgb.g, rgb.b, rgb.a]
            })
        };
        let surface = vec4_of("surface", [0.055, 0.078, 0.106, 1.0]);
        let primary = vec4_of("primary", [0.561, 0.816, 1.0, 1.0]);
        let tertiary = vec4_of("tertiary", [0.776, 0.761, 1.0, 1.0]);
        let on_surface = vec4_of("on_surface", [0.878, 0.902, 0.941, 1.0]);

        format!(
            "struct GoogleDesignTheme {{\n    surface: vec4<f32>,\n    primary: vec4<f32>,\n    tertiary: vec4<f32>,\n    on_surface: vec4<f32>,\n}}\nfn google_design_default() -> GoogleDesignTheme {{\n    return GoogleDesignTheme(\n        vec4<f32>({:.4}, {:.4}, {:.4}, {:.4}),\n        vec4<f32>({:.4}, {:.4}, {:.4}, {:.4}),\n        vec4<f32>({:.4}, {:.4}, {:.4}, {:.4}),\n        vec4<f32>({:.4}, {:.4}, {:.4}, {:.4}),\n    );\n}}\n",
            surface[0],
            surface[1],
            surface[2],
            surface[3],
            primary[0],
            primary[1],
            primary[2],
            primary[3],
            tertiary[0],
            tertiary[1],
            tertiary[2],
            tertiary[3],
            on_surface[0],
            on_surface[1],
            on_surface[2],
            on_surface[3],
        )
    }

    /// Reads the root workspace `DESIGN.md` via `aphrody_design` so `aphrody_ui` and Stitch stay synced.
    pub fn bundled_design_md() -> Result<aphrody_design::design_md::DesignMdDocument> {
        aphrody_design::design_md::load_bundled_root_design_md()
    }

    #[must_use]
    pub fn contrast_ratio_roles(&self, mode: AppearanceMode, fg: &str, bg: &str) -> Option<f32> {
        let fg_c = self.hsla(mode, fg)?;
        let bg_c = self.hsla(mode, bg)?;
        Some(contrast_ratio(fg_c, bg_c))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_google_design_tokens_and_springs_are_valid() -> Result<()> {
        let tokens = GoogleDesignTokens::bundled()?;
        assert_eq!(tokens.source_of_truth, "https://design.google/");
        assert_eq!(tokens.system, "Material 3 Expressive");
        assert!(tokens.reference.variable_axes.contains_key("opsz"));

        for mode in [AppearanceMode::Light, AppearanceMode::Dark] {
            for (fg, bg) in [
                ("on_surface", "surface"),
                ("on_surface_variant", "surface_container"),
                ("on_primary", "primary"),
                ("on_secondary", "secondary"),
                ("on_tertiary", "tertiary"),
                ("on_error", "error"),
            ] {
                let ratio = tokens
                    .contrast_ratio_roles(mode, fg, bg)
                    .with_context(|| format!("missing role {fg}/{bg}"))?;
                assert!(ratio >= 4.5, "{mode:?} {fg}/{bg} contrast {ratio:.2} < 4.5");
            }
        }

        let spatial = tokens
            .spring("spatial_fast")
            .context("spatial_fast missing")?;
        assert_eq!(spatial.sample_position(0.0), 0.0);
        assert!((spatial.sample_position(2.0) - 1.0).abs() < 1e-3);

        let effects = tokens
            .spring("effects_default")
            .context("effects_default missing")?;
        for step in 1..=20 {
            let p = effects.sample_position(step as f32 * 0.02);
            assert!(p <= 1.001, "critically damped spring overshot: {p}");
        }

        assert_eq!(WindowSizeClass::from_width_px(500.0), WindowSizeClass::Compact);
        assert_eq!(WindowSizeClass::from_width_px(1024.0), WindowSizeClass::Expanded);
        assert!(tokens.to_css_custom_properties(AppearanceMode::Dark).contains("--md-sys-color-primary:"));
        assert!(tokens.to_wgsl_theme_struct(AppearanceMode::Dark).contains("struct GoogleDesignTheme"));
        assert!(GoogleSansFlexAxes::default().css_font_variation_settings().contains("\"opsz\" 14.0"));
        assert!(GoogleDesignTokens::bundled_design_md().is_ok());
        Ok(())
    }
}
