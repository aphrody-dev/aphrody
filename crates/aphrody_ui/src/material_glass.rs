//! Real upstream Rust Material Design 3 HCT dynamic colors (`material-colors 0.5`),
//! Google Material Symbols & Lucide vector SVGs (`md-icons 0.4`, `lucide-static-svg 1.76`),
//! and native OS + WGPU frosted/liquid glass (`backdrop-blur-core 0.2`, `window-vibrancy 0.8`).

use backdrop_blur_core::{
    BlurRadius, BlurRequest, CornerRadius, LinearRgba, Presence, Region, Scale, Tint,
    resolve_gaussian, resolve_kawase_levels, use_dual_kawase,
};
use gpui::{Hsla, Rgba};
use material_colors::color::Rgb;
use material_colors::dynamic_color::{DynamicScheme, Platform, SpecVersion, Variant};
use material_colors::hct::Hct;
use material_colors::scheme::Scheme;

use crate::cosmic_theme::{AppearanceMode, OsColorTokens, OsLayoutTokens, OsTheme};

/// Official Google Material 3 Dynamic Color scheme variants backed by `material-colors 0.5`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum M3DynamicVariant {
    #[default]
    Expressive,
    TonalSpot,
    Vibrant,
    Fidelity,
    Content,
    Neutral,
    Monochrome,
    Rainbow,
    FruitSalad,
}

impl M3DynamicVariant {
    #[must_use]
    pub const fn to_upstream(self) -> Variant {
        match self {
            Self::Expressive => Variant::Expressive,
            Self::TonalSpot => Variant::TonalSpot,
            Self::Vibrant => Variant::Vibrant,
            Self::Fidelity => Variant::Fidelity,
            Self::Content => Variant::Content,
            Self::Neutral => Variant::Neutral,
            Self::Monochrome => Variant::Monochrome,
            Self::Rainbow => Variant::Rainbow,
            Self::FruitSalad => Variant::FruitSalad,
        }
    }
}

/// Converts a `material-colors` `Rgb` triplet into a GPUI `Hsla` color.
#[must_use]
pub fn m3_rgb_to_hsla(rgb: Rgb) -> Hsla {
    Rgba {
        r: f32::from(rgb.red) / 255.0,
        g: f32::from(rgb.green) / 255.0,
        b: f32::from(rgb.blue) / 255.0,
        a: 1.0,
    }
    .into()
}

/// Computes a full 49-role Material 3 HCT dynamic scheme using `material-colors 0.5` (`Spec2025`).
#[must_use]
pub fn compute_m3_dynamic_scheme(
    seed_rgb: (u8, u8, u8),
    dark: bool,
    variant: M3DynamicVariant,
    contrast_level: f64,
) -> Scheme {
    let source_hct = Hct::new(Rgb::new(seed_rgb.0, seed_rgb.1, seed_rgb.2));
    let dynamic = DynamicScheme::from_spec(
        source_hct,
        variant.to_upstream(),
        dark,
        Some(contrast_level.clamp(-1.0, 1.0)),
        Platform::Phone,
        SpecVersion::Spec2025,
    );
    Scheme::from(dynamic)
}

/// Builds a complete GPUI [`OsTheme`] dynamically from any RGB seed color using real Material 3
/// HCT/CAM16 tonal palettes (`material-colors 0.5`).
#[must_use]
pub fn os_theme_from_m3_seed(
    id: impl Into<String>,
    name: impl Into<String>,
    seed_rgb: (u8, u8, u8),
    dark: bool,
    variant: M3DynamicVariant,
    contrast_level: f64,
) -> OsTheme {
    let s = compute_m3_dynamic_scheme(seed_rgb, dark, variant, contrast_level);
    let colors = OsColorTokens {
        background: m3_rgb_to_hsla(s.background),
        on_background: m3_rgb_to_hsla(s.on_background),
        surface: m3_rgb_to_hsla(s.surface),
        surface_dim: m3_rgb_to_hsla(s.surface_dim),
        surface_bright: m3_rgb_to_hsla(s.surface_bright),
        surface_container_lowest: m3_rgb_to_hsla(s.surface_container_lowest),
        surface_container_low: m3_rgb_to_hsla(s.surface_container_low),
        surface_container: m3_rgb_to_hsla(s.surface_container),
        surface_container_high: m3_rgb_to_hsla(s.surface_container_high),
        surface_container_highest: m3_rgb_to_hsla(s.surface_container_highest),
        on_surface: m3_rgb_to_hsla(s.on_surface),
        on_surface_variant: m3_rgb_to_hsla(s.on_surface_variant),
        outline: m3_rgb_to_hsla(s.outline),
        outline_variant: m3_rgb_to_hsla(s.outline_variant),
        primary: m3_rgb_to_hsla(s.primary),
        primary_dim: m3_rgb_to_hsla(s.primary_fixed_dim),
        on_primary: m3_rgb_to_hsla(s.on_primary),
        primary_container: m3_rgb_to_hsla(s.primary_container),
        on_primary_container: m3_rgb_to_hsla(s.on_primary_container),
        secondary: m3_rgb_to_hsla(s.secondary),
        on_secondary: m3_rgb_to_hsla(s.on_secondary),
        secondary_container: m3_rgb_to_hsla(s.secondary_container),
        on_secondary_container: m3_rgb_to_hsla(s.on_secondary_container),
        tertiary: m3_rgb_to_hsla(s.tertiary),
        on_tertiary: m3_rgb_to_hsla(s.on_tertiary),
        error: m3_rgb_to_hsla(s.error),
        on_error: m3_rgb_to_hsla(s.on_error),
        error_container: m3_rgb_to_hsla(s.error_container),
        on_error_container: m3_rgb_to_hsla(s.on_error_container),
    };
    OsTheme {
        id: id.into(),
        name: name.into(),
        mode: if dark {
            AppearanceMode::Dark
        } else {
            AppearanceMode::Light
        },
        colors,
        layout: OsLayoutTokens::default(),
    }
}

/// Real Google Material Symbols (`md-icons 0.4`) style families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MaterialSymbolStyle {
    #[default]
    Rounded,
    RoundedFilled,
    Outlined,
    OutlinedFilled,
}

/// Curated real Google Material Symbols SVG constants (`md-icons 0.4`, tree-shaken by Fat-LTO).
#[must_use]
pub fn material_symbol_svg(name: &str, style: MaterialSymbolStyle) -> Option<&'static str> {
    use md_icons::{outlined, rounded};
    match (name, style) {
        ("home", MaterialSymbolStyle::Rounded) => Some(rounded::ICON_HOME),
        ("home", MaterialSymbolStyle::RoundedFilled) => Some(rounded::filled::ICON_HOME),
        ("home", MaterialSymbolStyle::Outlined) => Some(outlined::ICON_HOME),
        ("home", MaterialSymbolStyle::OutlinedFilled) => Some(outlined::filled::ICON_HOME),
        ("search", MaterialSymbolStyle::Rounded | MaterialSymbolStyle::RoundedFilled) => {
            Some(rounded::ICON_SEARCH)
        }
        ("search", MaterialSymbolStyle::Outlined | MaterialSymbolStyle::OutlinedFilled) => {
            Some(outlined::ICON_SEARCH)
        }
        ("settings", MaterialSymbolStyle::Rounded) => Some(rounded::ICON_SETTINGS),
        ("settings", MaterialSymbolStyle::RoundedFilled) => Some(rounded::filled::ICON_SETTINGS),
        ("settings", MaterialSymbolStyle::Outlined) => Some(outlined::ICON_SETTINGS),
        ("settings", MaterialSymbolStyle::OutlinedFilled) => Some(outlined::filled::ICON_SETTINGS),
        ("palette", MaterialSymbolStyle::Rounded) => Some(rounded::ICON_PALETTE),
        ("palette", MaterialSymbolStyle::RoundedFilled) => Some(rounded::filled::ICON_PALETTE),
        ("palette", MaterialSymbolStyle::Outlined) => Some(outlined::ICON_PALETTE),
        ("palette", MaterialSymbolStyle::OutlinedFilled) => Some(outlined::filled::ICON_PALETTE),
        ("notifications", MaterialSymbolStyle::Rounded) => Some(rounded::ICON_NOTIFICATIONS),
        ("notifications", MaterialSymbolStyle::RoundedFilled) => {
            Some(rounded::filled::ICON_NOTIFICATIONS)
        }
        ("notifications", MaterialSymbolStyle::Outlined) => Some(outlined::ICON_NOTIFICATIONS),
        ("notifications", MaterialSymbolStyle::OutlinedFilled) => {
            Some(outlined::filled::ICON_NOTIFICATIONS)
        }
        ("auto_awesome", MaterialSymbolStyle::Rounded) => Some(rounded::ICON_AUTO_AWESOME),
        ("auto_awesome", MaterialSymbolStyle::RoundedFilled) => {
            Some(rounded::filled::ICON_AUTO_AWESOME)
        }
        ("auto_awesome", MaterialSymbolStyle::Outlined) => Some(outlined::ICON_AUTO_AWESOME),
        ("auto_awesome", MaterialSymbolStyle::OutlinedFilled) => {
            Some(outlined::filled::ICON_AUTO_AWESOME)
        }
        ("layers", MaterialSymbolStyle::Rounded) => Some(rounded::ICON_LAYERS),
        ("layers", MaterialSymbolStyle::RoundedFilled) => Some(rounded::filled::ICON_LAYERS),
        ("layers", MaterialSymbolStyle::Outlined) => Some(outlined::ICON_LAYERS),
        ("layers", MaterialSymbolStyle::OutlinedFilled) => Some(outlined::filled::ICON_LAYERS),
        ("terminal", MaterialSymbolStyle::Rounded | MaterialSymbolStyle::RoundedFilled) => {
            Some(rounded::ICON_TERMINAL)
        }
        ("terminal", MaterialSymbolStyle::Outlined | MaterialSymbolStyle::OutlinedFilled) => {
            Some(outlined::ICON_TERMINAL)
        }
        ("folder", MaterialSymbolStyle::Rounded) => Some(rounded::ICON_FOLDER),
        ("folder", MaterialSymbolStyle::RoundedFilled) => Some(rounded::filled::ICON_FOLDER),
        ("folder", MaterialSymbolStyle::Outlined) => Some(outlined::ICON_FOLDER),
        ("folder", MaterialSymbolStyle::OutlinedFilled) => Some(outlined::filled::ICON_FOLDER),
        ("tune", MaterialSymbolStyle::Rounded | MaterialSymbolStyle::RoundedFilled) => {
            Some(rounded::ICON_TUNE)
        }
        ("tune", MaterialSymbolStyle::Outlined | MaterialSymbolStyle::OutlinedFilled) => {
            Some(outlined::ICON_TUNE)
        }
        _ => None,
    }
}

/// Resolves an official Lucide SVG string from `lucide-static-svg 1.76`.
#[must_use]
pub fn lucide_icon_svg(icon: lucide_static_svg::Icon) -> &'static str {
    icon.svg_str()
}

/// Unified frosted / liquid glass material presets mapped onto `backdrop-blur-core 0.2`
/// and native OS `window-vibrancy 0.8` (Windows 11 Mica/Acrylic, macOS NSVisualEffect/LiquidGlass).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GlassMaterialPreset {
    /// Subtle Windows 11 Mica / macOS UnderWindow ambient glass (18pt blur).
    Mica,
    /// Deeper Windows 11 Mica Alt / Tabbed shell chrome (24pt blur).
    MicaAlt,
    /// Frosted acrylic floating dock / toolbar (20pt dual-Kawase blur).
    #[default]
    Acrylic,
    /// High-clarity Liquid Glass with specular Fresnel edge refraction (28pt blur).
    LiquidGlass,
}

/// Resolved blur pipeline plan produced by `backdrop-blur-core 0.2` for WGPU v30 / WebGPU shaders.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedGlassPlan {
    pub preset: GlassMaterialPreset,
    pub physical_radius_px: f32,
    pub dual_kawase: bool,
    pub kawase_levels: u32,
    pub gaussian_sigma: f32,
    pub gaussian_tap_radius: i32,
    pub corner_radius_px: f32,
    pub specular_intensity: f32,
}

impl GlassMaterialPreset {
    #[must_use]
    pub const fn logical_blur_pt(self) -> f32 {
        match self {
            Self::Mica => 18.0,
            Self::MicaAlt => 24.0,
            Self::Acrylic => 20.0,
            Self::LiquidGlass => 28.0,
        }
    }

    #[must_use]
    pub const fn tint_alpha(self) -> f32 {
        match self {
            Self::Mica => 0.72,
            Self::MicaAlt => 0.78,
            Self::Acrylic => 0.64,
            Self::LiquidGlass => 0.42,
        }
    }

    #[must_use]
    pub const fn specular_intensity(self) -> f32 {
        match self {
            Self::Mica => 0.14,
            Self::MicaAlt => 0.18,
            Self::Acrylic => 0.24,
            Self::LiquidGlass => 0.46,
        }
    }

    /// Builds a `backdrop-blur-core` [`BlurRequest`] in physical pixels for a given DPI scale.
    #[must_use]
    pub fn build_blur_request(
        self,
        origin: [u32; 2],
        size: [u32; 2],
        dpi_scale: f32,
        corner_radius_pt: f32,
        tint_srgb: [u8; 3],
    ) -> BlurRequest {
        let region = Region {
            origin,
            size,
            scale: Scale::new(dpi_scale),
        };
        let alpha_u8 = (self.tint_alpha() * 255.0).round().clamp(0.0, 255.0) as u8;
        let tint = Tint::new(LinearRgba::from_srgb_unmultiplied([
            tint_srgb[0],
            tint_srgb[1],
            tint_srgb[2],
            alpha_u8,
        ]));
        BlurRequest {
            source_region: region,
            target_rect: region,
            blur_radius: BlurRadius::new(self.logical_blur_pt()),
            tint,
            corner_radius: CornerRadius::new(corner_radius_pt),
            presence: Presence::default(),
        }
    }

    /// Resolves whether the WGPU/WebGPU frosted glass pass should use Dual-Kawase pyramid or
    /// separable Gaussian blur via `backdrop-blur-core` exact threshold & kernel math.
    #[must_use]
    pub fn resolve_gpu_plan(self, dpi_scale: f32, corner_radius_pt: f32) -> ResolvedGlassPlan {
        let req = self.build_blur_request([0, 0], [512, 512], dpi_scale, corner_radius_pt, [14, 20, 27]);
        let phys = req.physical_blur_radius();
        let dual_kawase = use_dual_kawase(phys);
        let kawase_levels = resolve_kawase_levels(phys);
        let gaussian = resolve_gaussian(phys);
        ResolvedGlassPlan {
            preset: self,
            physical_radius_px: phys,
            dual_kawase,
            kawase_levels,
            gaussian_sigma: gaussian.sigma,
            gaussian_tap_radius: gaussian.tap_radius,
            corner_radius_px: corner_radius_pt * dpi_scale.max(1.0),
            specular_intensity: self.specular_intensity(),
        }
    }
}

/// Applies real native OS window glass (`window-vibrancy 0.8`) to any `HasWindowHandle` window:
/// - Windows 11: Mica, Mica Alt (`apply_tabbed`), Acrylic, or DWM Blur
/// - macOS: `NSVisualEffectView` (`HudWindow` / `UnderWindowBackground` / `Sidebar`)
#[cfg(any(target_os = "windows", target_os = "macos"))]
pub fn apply_native_window_glass(
    window: &impl raw_window_handle::HasWindowHandle,
    preset: GlassMaterialPreset,
    dark: bool,
) -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    {
        match preset {
            GlassMaterialPreset::Mica => window_vibrancy::apply_mica(window, Some(dark))?,
            GlassMaterialPreset::MicaAlt => window_vibrancy::apply_tabbed(window, Some(dark))?,
            GlassMaterialPreset::Acrylic | GlassMaterialPreset::LiquidGlass => {
                let tint = if dark {
                    Some((14, 20, 27, 150))
                } else {
                    Some((247, 249, 255, 175))
                };
                window_vibrancy::apply_acrylic(window, tint)?;
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        let material = match preset {
            GlassMaterialPreset::Mica => window_vibrancy::NSVisualEffectMaterial::UnderWindowBackground,
            GlassMaterialPreset::MicaAlt => window_vibrancy::NSVisualEffectMaterial::Sidebar,
            GlassMaterialPreset::Acrylic | GlassMaterialPreset::LiquidGlass => {
                window_vibrancy::NSVisualEffectMaterial::HudWindow
            }
        };
        window_vibrancy::apply_vibrancy(window, material, None, Some(16.0))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cosmic_theme::contrast_ratio;

    #[test]
    fn material_colors_generates_spec2025_hct_expressive_theme_meeting_wcag_aa() {
        let dark = os_theme_from_m3_seed(
            "m3-dyn-dark",
            "Material 3 Expressive Dynamic Dark",
            (66, 133, 244),
            true,
            M3DynamicVariant::Expressive,
            0.0,
        );
        let light = os_theme_from_m3_seed(
            "m3-dyn-light",
            "Material 3 Expressive Dynamic Light",
            (66, 133, 244),
            false,
            M3DynamicVariant::Expressive,
            0.0,
        );

        assert!(contrast_ratio(dark.colors.on_surface, dark.colors.surface) >= 4.5);
        assert!(contrast_ratio(dark.colors.on_primary, dark.colors.primary) >= 4.5);
        assert!(contrast_ratio(light.colors.on_surface, light.colors.surface) >= 4.5);
        assert!(contrast_ratio(light.colors.on_primary, light.colors.primary) >= 4.5);
    }

    #[test]
    fn md_icons_and_lucide_static_svg_provide_real_vector_svgs() {
        for name in [
            "home",
            "search",
            "settings",
            "palette",
            "notifications",
            "auto_awesome",
            "layers",
            "terminal",
            "folder",
            "tune",
        ] {
            let rounded = material_symbol_svg(name, MaterialSymbolStyle::Rounded);
            assert!(rounded.is_some_and(|svg| svg.contains("<svg") && svg.contains("</svg>")));
        }
        let lucide = lucide_icon_svg(lucide_static_svg::Icon::Sparkles);
        assert!(lucide.contains("<svg") && lucide.contains("viewBox=\"0 0 24 24\""));
    }

    #[test]
    fn backdrop_blur_core_resolves_dual_kawase_glass_plans() {
        let glass = GlassMaterialPreset::LiquidGlass.resolve_gpu_plan(2.0, 20.0);
        assert_eq!(glass.physical_radius_px, 56.0);
        assert!(glass.dual_kawase);
        assert!(glass.kawase_levels >= 2);
        assert!(glass.specular_intensity > 0.4);
    }
}
