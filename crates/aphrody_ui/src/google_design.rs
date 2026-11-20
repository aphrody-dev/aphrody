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
pub use brand::design::BUNDLED_GOOGLE_DESIGN_TOKENS_TOML;

/// Responsive Material 3 window size class unified directly onto [`brand::design::WindowSizeTier`].
pub use brand::design::WindowSizeTier as WindowSizeClass;

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
        brand::design::sample_spring_position(self.stiffness, self.damping_ratio, t_secs)
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
    /// Expressive display hero headline preset matching Neural Expressive home canvas.
    #[must_use]
    pub fn display_hero() -> Self {
        Self {
            opsz: 48.0,
            wght: 500.0,
            wdth: 100.0,
            grad: 0.0,
            slnt: 0.0,
            rond: 40.0,
        }
    }

    /// Section headline preset for cards, panels, and docs titles.
    #[must_use]
    pub fn section_headline() -> Self {
        Self {
            opsz: 28.0,
            wght: 500.0,
            wdth: 100.0,
            grad: 0.0,
            slnt: 0.0,
            rond: 25.0,
        }
    }

    /// Interactive control preset with subtle roundness for pills and action chips.
    #[must_use]
    pub fn ui_control() -> Self {
        Self {
            opsz: 14.0,
            wght: 500.0,
            wdth: 100.0,
            grad: 0.0,
            slnt: 0.0,
            rond: 20.0,
        }
    }

    /// Compact mobile & high-density telemetry preset.
    #[must_use]
    pub fn compact_mobile() -> Self {
        Self {
            opsz: 12.0,
            wght: 500.0,
            wdth: 95.0,
            grad: 10.0,
            slnt: 0.0,
            rond: 16.0,
        }
    }

    /// Tabular label preset for counters, store metadata, and roadmaps.
    #[must_use]
    pub fn tabular_label() -> Self {
        Self {
            opsz: 11.0,
            wght: 600.0,
            wdth: 100.0,
            grad: 25.0,
            slnt: 0.0,
            rond: 0.0,
        }
    }

    /// Glanceable preset for additive transparent displays (`design.google/library/transparent-screens`):
    /// positive grade and weight keep strokes legible over bright backgrounds without halation.
    #[must_use]
    pub fn xr_glimmer_glanceable() -> Self {
        Self {
            opsz: 10.0,
            wght: 600.0,
            wdth: 104.0,
            grad: 20.0,
            slnt: 0.0,
            rond: 15.0,
        }
    }

    /// Long-form reading preset for OpenBook Quire notes on glass or paper.
    #[must_use]
    pub fn quire_ink_body() -> Self {
        Self {
            opsz: 16.0,
            wght: 420.0,
            wdth: 100.0,
            grad: 0.0,
            slnt: 0.0,
            rond: 8.0,
        }
    }

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ColorPaletteMap {
    pub light: BTreeMap<String, String>,
    pub dark: BTreeMap<String, String>,
    #[serde(default)]
    pub neural_obsidian: BTreeMap<String, String>,
    #[serde(default)]
    pub oled_pitch: BTreeMap<String, String>,
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
    #[serde(default)]
    pub comp: BTreeMap<String, toml::Table>,
}

impl GoogleDesignTokens {
    pub fn bundled() -> Result<Self> {
        toml::from_str(BUNDLED_GOOGLE_DESIGN_TOKENS_TOML)
            .context("parsing bundled google_design_tokens.toml")
    }

    /// Resolves a `ref.palette` entry; brand reference colors live only in the token manifest.
    #[must_use]
    pub fn reference_hsla(&self, key: &str) -> Option<Hsla> {
        self.reference
            .palette
            .get(key)
            .and_then(|hex| theme::try_parse_color(hex.trim()).ok())
    }

    /// Reads a numeric `comp.<component>.<key>` token (integers are widened to `f32`).
    #[must_use]
    pub fn comp_number(&self, component: &str, key: &str) -> Option<f32> {
        match self.comp.get(component)?.get(key)? {
            toml::Value::Float(value) => Some(*value as f32),
            toml::Value::Integer(value) => Some(*value as f32),
            _ => None,
        }
    }

    #[must_use]
    pub fn palette(&self, mode: AppearanceMode) -> &BTreeMap<String, String> {
        match mode {
            AppearanceMode::Light => &self.sys.color.light,
            AppearanceMode::Dark => &self.sys.color.dark,
        }
    }

    /// Resolves one of the 4 named Aphrody surface modes (`light`, `dark`, `neural_obsidian`, `oled_pitch`).
    #[must_use]
    pub fn palette_by_name(&self, name: &str) -> &BTreeMap<String, String> {
        match name {
            "light" | "material_light" => &self.sys.color.light,
            "neural_obsidian" | "neural-obsidian" | "neural" => {
                if self.sys.color.neural_obsidian.is_empty() {
                    &self.sys.color.dark
                } else {
                    &self.sys.color.neural_obsidian
                }
            }
            "oled_pitch" | "oled-pitch" | "oled" => {
                if self.sys.color.oled_pitch.is_empty() {
                    &self.sys.color.dark
                } else {
                    &self.sys.color.oled_pitch
                }
            }
            _ => &self.sys.color.dark,
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

/// Brand reference colors and component metrics for the mirrored design.google references
/// (Glimmer, Gemini, YouTube, Googlebook, OpenBook Quire), resolved once from `ref.palette` and `comp.*`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct MirrorAccentPalette {
    pub glimmer_void: Hsla,
    pub glimmer_surface: Hsla,
    pub glimmer_container: Hsla,
    pub glimmer_gold: Hsla,
    pub glimmer_mint: Hsla,
    pub glimmer_lime: Hsla,
    pub gemini_spectrum: [Hsla; 3],
    pub youtube_red: Hsla,
    pub youtube_magenta: Hsla,
    pub youtube_gradient_angle_deg: f32,
    pub youtube_track_height_px: f32,
    pub googlebook_glow: [Hsla; 4],
    pub quire_rim: [Hsla; 5],
    pub quire_folio_light: Hsla,
    pub quire_folio_dark: Hsla,
    pub quire_tint_opacity: f32,
    pub quire_economy_paper_opacity: f32,
    pub quire_rim_width_px: f32,
    pub quire_card_radius_px: f32,
}

impl MirrorAccentPalette {
    pub fn bundled() -> Result<Self> {
        Self::from_tokens(&GoogleDesignTokens::bundled()?)
    }

    pub fn from_tokens(tokens: &GoogleDesignTokens) -> Result<Self> {
        let color = |key: &str| {
            tokens
                .reference_hsla(key)
                .with_context(|| format!("ref.palette.{key} missing or unparsable"))
        };
        let number = |component: &str, key: &str| {
            tokens
                .comp_number(component, key)
                .with_context(|| format!("comp.{component}.{key} missing"))
        };
        Ok(Self {
            glimmer_void: color("glimmer_void")?,
            glimmer_surface: color("glimmer_surface")?,
            glimmer_container: color("glimmer_container")?,
            glimmer_gold: color("glimmer_gold")?,
            glimmer_mint: color("glimmer_mint")?,
            glimmer_lime: color("glimmer_lime")?,
            gemini_spectrum: [
                color("gemini_blue")?,
                color("gemini_violet")?,
                color("gemini_rose")?,
            ],
            youtube_red: color("youtube_red")?,
            youtube_magenta: color("youtube_magenta")?,
            youtube_gradient_angle_deg: number("youtube_scrubber", "gradient_angle_deg")?,
            youtube_track_height_px: number("youtube_scrubber", "track_height_px")?,
            googlebook_glow: [
                color("googlebook_blue")?,
                color("googlebook_red")?,
                color("googlebook_yellow")?,
                color("googlebook_green")?,
            ],
            quire_rim: [
                color("quire_rim_1")?,
                color("quire_rim_2")?,
                color("quire_rim_3")?,
                color("quire_rim_4")?,
                color("quire_rim_5")?,
            ],
            quire_folio_light: color("quire_folio_light")?,
            quire_folio_dark: color("quire_folio_dark")?,
            quire_tint_opacity: number("quire_glass_note", "tint_opacity")?,
            quire_economy_paper_opacity: number("quire_glass_note", "economy_paper_opacity")?,
            quire_rim_width_px: number("quire_glass_note", "rim_width_dp")?,
            quire_card_radius_px: number("quire_glass_note", "card_radius_px")?,
        })
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
        assert_eq!(GoogleSansFlexAxes::display_hero().rond, 40.0);
        assert_eq!(GoogleSansFlexAxes::compact_mobile().wdth, 95.0);
        assert!(!tokens.palette_by_name("neural_obsidian").is_empty());
        assert!(!tokens.palette_by_name("oled_pitch").is_empty());
        assert!(GoogleDesignTokens::bundled_design_md().is_ok());
        Ok(())
    }

    #[test]
    fn mirrored_reference_tokens_resolve_and_stay_legible() -> Result<()> {
        let tokens = GoogleDesignTokens::bundled()?;
        let accents = MirrorAccentPalette::from_tokens(&tokens)?;

        for name in ["quire_unfold", "quire_settle", "quire_expand", "quire_fold", "quire_tick"] {
            let spring = tokens
                .spring(name)
                .with_context(|| format!("{name} spring missing"))?;
            assert!((spring.sample_position(3.0) - 1.0).abs() < 1e-2, "{name} does not settle");
        }

        for (foreground, background) in [
            (accents.glimmer_gold, accents.glimmer_surface),
            (accents.glimmer_mint, accents.glimmer_surface),
            (accents.glimmer_gold, accents.glimmer_container),
        ] {
            let ratio = contrast_ratio(foreground, background);
            assert!(ratio >= 4.5, "glimmer contrast {ratio:.2} < 4.5");
        }

        assert_eq!(accents.glimmer_void.l, 0.0);
        assert_eq!(accents.youtube_gradient_angle_deg, 45.0);
        assert!(accents.quire_economy_paper_opacity > accents.quire_tint_opacity);
        assert_eq!(GoogleSansFlexAxes::xr_glimmer_glanceable().grad, 20.0);
        assert!(
            GoogleSansFlexAxes::quire_ink_body()
                .css_font_variation_settings()
                .contains("\"opsz\" 16.0")
        );
        assert!(tokens.comp_number("missing_component", "height_px").is_none());
        Ok(())
    }
}
