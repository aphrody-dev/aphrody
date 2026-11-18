//! Compile-time generated Material 3 Expressive + Google Sans Flex design system
//! derived once from `assets/os/google_design_tokens.toml` (`https://design.google/`).
//!
//! All downstream UI, component, desktop, and design crates consume this module as
//! their single authoritative source of truth for palette tokens, typography scales,
//! corner shapes, state-layer opacities, variable font axis bounds, spring physics,
//! and WCAG 2.2 luminance/contrast calculations.

/// Full text of `assets/os/google_design_tokens.toml` embedded once for inspection or export.
pub const BUNDLED_GOOGLE_DESIGN_TOKENS_TOML: &str =
    include_str!("../../../assets/os/google_design_tokens.toml");

/// Minimum WCAG 2.2 AA contrast ratio for normal body text (`4.5:1`).
pub const WCAG_AA_MIN_CONTRAST: f32 = 4.5;

/// Minimum WCAG 2.2 AA contrast ratio for large text, icons, and UI focus rings (`3.0:1`).
pub const WCAG_AA_LARGE_CONTRAST: f32 = 3.0;

/// Four Material 3 Expressive surface modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThemeModeSlug {
    Light,
    Dark,
    NeuralObsidian,
    OledPitch,
}

impl ThemeModeSlug {
    /// Canonical TOML key under `[sys.color.*]`.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::NeuralObsidian => "neural_obsidian",
            Self::OledPitch => "oled_pitch",
        }
    }

    /// Parses mode aliases (`"light"`, `"dark"`, `"neural_obsidian"`, `"oled_pitch"`, etc.).
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "light" | "material_light" | "material-light" => Some(Self::Light),
            "dark" | "material_dark" | "material-dark" => Some(Self::Dark),
            "neural_obsidian" | "neural-obsidian" | "neural" | "obsidian" => {
                Some(Self::NeuralObsidian)
            }
            "oled_pitch" | "oled-pitch" | "oled" | "pitch" => Some(Self::OledPitch),
            _ => None,
        }
    }
}

/// Google Sans Flex variable font axis bounds from `[ref.variable_axes.*]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisSpec {
    pub tag: &'static str,
    pub name: &'static str,
    pub min: f32,
    pub default_value: f32,
    pub max: f32,
}

/// Material 3 Expressive corner shape scale from `[sys.shape]`.
#[derive(Debug, Clone, Copy, PartialEq)]
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

/// Material 3 Expressive interaction state-layer opacities from `[sys.state]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StateLayers {
    pub hover_opacity: f32,
    pub focus_opacity: f32,
    pub pressed_opacity: f32,
    pub dragged_opacity: f32,
    pub disabled_container_opacity: f32,
    pub disabled_content_opacity: f32,
    pub focus_ring_px: f32,
}

/// Material 3 Expressive density offsets and touch target constraints from `[sys.density]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DensityScale {
    pub default: i32,
    pub compact_1: i32,
    pub compact_2: i32,
    pub compact_3: i32,
    pub touch_target_min_px: f32,
    pub control_height_min_px: f32,
}

/// Responsive Material 3 window size breakpoint thresholds from `[sys.window_size_classes]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowSizeThresholds {
    pub compact_max_px: f32,
    pub medium_min_px: f32,
    pub expanded_min_px: f32,
    pub large_min_px: f32,
    pub extra_large_min_px: f32,
}

/// Responsive viewport size tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WindowSizeTier {
    Compact,
    Medium,
    Expanded,
    Large,
    ExtraLarge,
}

/// Spring oscillator parameters from `[sys.motion.springs.*]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpringTokenSpec {
    pub key: &'static str,
    pub stiffness: f32,
    pub damping_ratio: f32,
}

/// Single typography role from `[sys.typescale.*]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypeScaleEntry {
    pub key: &'static str,
    pub font: &'static str,
    pub size_sp: f32,
    pub line_height_sp: f32,
    pub weight: f32,
    pub tracking_sp: f32,
}

/// All 15 Material 3 typography scale roles from `[sys.typescale.*]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypeScaleSet {
    pub display_large: TypeScaleEntry,
    pub display_medium: TypeScaleEntry,
    pub display_small: TypeScaleEntry,
    pub headline_large: TypeScaleEntry,
    pub headline_medium: TypeScaleEntry,
    pub headline_small: TypeScaleEntry,
    pub title_large: TypeScaleEntry,
    pub title_medium: TypeScaleEntry,
    pub title_small: TypeScaleEntry,
    pub body_large: TypeScaleEntry,
    pub body_medium: TypeScaleEntry,
    pub body_small: TypeScaleEntry,
    pub label_large: TypeScaleEntry,
    pub label_medium: TypeScaleEntry,
    pub label_small: TypeScaleEntry,
}

/// Complete 30-role Material 3 semantic palette in canonical `#RRGGBB` hex strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorPaletteHex {
    pub background: &'static str,
    pub on_background: &'static str,
    pub surface: &'static str,
    pub surface_dim: &'static str,
    pub surface_bright: &'static str,
    pub surface_container_lowest: &'static str,
    pub surface_container_low: &'static str,
    pub surface_container: &'static str,
    pub surface_container_high: &'static str,
    pub surface_container_highest: &'static str,
    pub on_surface: &'static str,
    pub on_surface_variant: &'static str,
    pub outline: &'static str,
    pub outline_variant: &'static str,
    pub primary: &'static str,
    pub on_primary: &'static str,
    pub primary_container: &'static str,
    pub on_primary_container: &'static str,
    pub secondary: &'static str,
    pub on_secondary: &'static str,
    pub secondary_container: &'static str,
    pub on_secondary_container: &'static str,
    pub tertiary: &'static str,
    pub on_tertiary: &'static str,
    pub tertiary_container: &'static str,
    pub on_tertiary_container: &'static str,
    pub error: &'static str,
    pub on_error: &'static str,
    pub error_container: &'static str,
    pub on_error_container: &'static str,
}

/// Complete 30-role Material 3 semantic palette packed as `0x00RRGGBB` integers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorPaletteRgb {
    pub background: u32,
    pub on_background: u32,
    pub surface: u32,
    pub surface_dim: u32,
    pub surface_bright: u32,
    pub surface_container_lowest: u32,
    pub surface_container_low: u32,
    pub surface_container: u32,
    pub surface_container_high: u32,
    pub surface_container_highest: u32,
    pub on_surface: u32,
    pub on_surface_variant: u32,
    pub outline: u32,
    pub outline_variant: u32,
    pub primary: u32,
    pub on_primary: u32,
    pub primary_container: u32,
    pub on_primary_container: u32,
    pub secondary: u32,
    pub on_secondary: u32,
    pub secondary_container: u32,
    pub on_secondary_container: u32,
    pub tertiary: u32,
    pub on_tertiary: u32,
    pub tertiary_container: u32,
    pub on_tertiary_container: u32,
    pub error: u32,
    pub on_error: u32,
    pub error_container: u32,
    pub on_error_container: u32,
}

include!(concat!(env!("OUT_DIR"), "/design_tokens_constants.rs"));

const fn eq_bytes(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut index = 0;
    while index < a.len() {
        if a[index] != b[index] {
            return false;
        }
        index += 1;
    }
    true
}

/// Const lookup for `[ref.palette]` `0x00RRGGBB` integers by key, returning `None` if absent.
#[must_use]
pub const fn reference_palette_u32_const(key: &str) -> Option<u32> {
    let target = key.as_bytes();
    let mut index = 0;
    while index < REFERENCE_PALETTE.len() {
        if eq_bytes(REFERENCE_PALETTE[index].0.as_bytes(), target) {
            return Some(REFERENCE_PALETTE[index].2);
        }
        index += 1;
    }
    None
}

/// Const lookup for `[ref.palette]` `0x00RRGGBB` integers with a compile-time fallback.
#[must_use]
pub const fn reference_palette_u32_or(key: &str, fallback: u32) -> u32 {
    match reference_palette_u32_const(key) {
        Some(value) => value,
        None => fallback,
    }
}

/// Const lookup for `[sys.motion.springs.*]` presets by key.
#[must_use]
pub const fn spring_spec_const(key: &str) -> Option<SpringTokenSpec> {
    let target = key.as_bytes();
    let mut index = 0;
    while index < SPRINGS.len() {
        if eq_bytes(SPRINGS[index].key.as_bytes(), target) {
            return Some(SPRINGS[index]);
        }
        index += 1;
    }
    None
}

/// Const lookup for `[sys.variable_axes.*]` specs by tag.
#[must_use]
pub const fn axis_spec_const(tag: &str) -> Option<AxisSpec> {
    let target = tag.as_bytes();
    let mut index = 0;
    while index < VARIABLE_AXES.len() {
        if eq_bytes(VARIABLE_AXES[index].tag.as_bytes(), target) {
            return Some(VARIABLE_AXES[index]);
        }
        index += 1;
    }
    None
}

/// Looks up a `[ref.palette]` hex string by key (e.g. `"gemini_blue"`).
#[must_use]
pub fn reference_palette_hex(key: &str) -> Option<&'static str> {
    REFERENCE_PALETTE
        .iter()
        .find(|(k, _, _)| *k == key)
        .map(|(_, hex, _)| *hex)
}

/// Looks up a `[ref.palette]` `0x00RRGGBB` integer by key.
#[must_use]
pub fn reference_palette_u32(key: &str) -> Option<u32> {
    REFERENCE_PALETTE
        .iter()
        .find(|(k, _, _)| *k == key)
        .map(|(_, _, rgb)| *rgb)
}

/// Looks up a `[ref.variable_axes.*]` specification by tag (`"opsz"`, `"wght"`, `"wdth"`, `"GRAD"`, `"slnt"`, `"ROND"`).
#[must_use]
pub fn axis_spec(tag: &str) -> Option<AxisSpec> {
    VARIABLE_AXES.iter().copied().find(|axis| axis.tag == tag)
}

/// Looks up a `[sys.motion.springs.*]` preset by TOML key (e.g. `"spatial_fast"`).
#[must_use]
pub fn spring_spec(key: &str) -> Option<SpringTokenSpec> {
    SPRINGS.iter().copied().find(|spring| spring.key == key)
}

/// Looks up a `[comp.<component>.<key>]` numeric token by component and field key.
#[must_use]
pub fn comp_number(component: &str, key: &str) -> Option<f32> {
    COMP_NUMBERS
        .iter()
        .find(|(c, k, _)| *c == component && *k == key)
        .map(|(_, _, v)| *v)
}

/// Returns the semantic hex palette for `mode`.
#[must_use]
pub const fn palette_hex(mode: ThemeModeSlug) -> &'static ColorPaletteHex {
    match mode {
        ThemeModeSlug::Light => &PALETTE_LIGHT_HEX,
        ThemeModeSlug::Dark => &PALETTE_DARK_HEX,
        ThemeModeSlug::NeuralObsidian => &PALETTE_NEURAL_OBSIDIAN_HEX,
        ThemeModeSlug::OledPitch => &PALETTE_OLED_PITCH_HEX,
    }
}

/// Returns the semantic packed-RGB palette for `mode`.
#[must_use]
pub const fn palette_rgb(mode: ThemeModeSlug) -> &'static ColorPaletteRgb {
    match mode {
        ThemeModeSlug::Light => &PALETTE_LIGHT_RGB,
        ThemeModeSlug::Dark => &PALETTE_DARK_RGB,
        ThemeModeSlug::NeuralObsidian => &PALETTE_NEURAL_OBSIDIAN_RGB,
        ThemeModeSlug::OledPitch => &PALETTE_OLED_PITCH_RGB,
    }
}

/// Classifies viewport width in logical pixels against `[sys.window_size_classes]`.
#[must_use]
pub fn classify_window_width(width_px: f32) -> WindowSizeTier {
    if width_px < WINDOW_SIZE_CLASSES.medium_min_px {
        WindowSizeTier::Compact
    } else if width_px < WINDOW_SIZE_CLASSES.expanded_min_px {
        WindowSizeTier::Medium
    } else if width_px < WINDOW_SIZE_CLASSES.large_min_px {
        WindowSizeTier::Expanded
    } else if width_px < WINDOW_SIZE_CLASSES.extra_large_min_px {
        WindowSizeTier::Large
    } else {
        WindowSizeTier::ExtraLarge
    }
}

/// Evaluates normalized `0.0 -> 1.0` displacement at `t_secs` for a unit-mass damped harmonic oscillator.
#[must_use]
pub fn sample_spring_position(stiffness: f32, damping_ratio: f32, t_secs: f32) -> f32 {
    if t_secs <= 0.0 || stiffness <= 0.0 {
        return 0.0;
    }
    let omega0 = stiffness.sqrt();
    let zeta = damping_ratio.max(0.05);
    if (zeta - 1.0).abs() < 1e-4 {
        let decay = (-omega0 * t_secs).exp();
        1.0 - decay * (1.0 + omega0 * t_secs)
    } else if zeta < 1.0 {
        let root = (1.0 - zeta * zeta).sqrt();
        let omega_d = omega0 * root;
        let decay = (-zeta * omega0 * t_secs).exp();
        let ratio = zeta / root;
        1.0 - decay * ((omega_d * t_secs).cos() + ratio * (omega_d * t_secs).sin())
    } else {
        let root = (zeta * zeta - 1.0).sqrt();
        let r1 = -omega0 * (zeta - root);
        let r2 = -omega0 * (zeta + root);
        let denom = r1 - r2;
        if denom.abs() < 1e-6 {
            1.0
        } else {
            1.0 - (r2 * (r1 * t_secs).exp() - r1 * (r2 * t_secs).exp()) / -denom
        }
    }
}

/// Parses `#RRGGBB` into `(r, g, b)` octets.
#[must_use]
pub fn parse_hex_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let clean = hex.trim().strip_prefix('#')?;
    if clean.len() != 6 || !clean.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&clean[0..2], 16).ok()?;
    let g = u8::from_str_radix(&clean[2..4], 16).ok()?;
    let b = u8::from_str_radix(&clean[4..6], 16).ok()?;
    Some((r, g, b))
}

/// Parses `#RRGGBB` or `#RRGGBBAA` into normalized `[r, g, b, a]` channels in `0.0..=1.0`.
#[must_use]
pub fn parse_hex_rgba_f32(hex: &str) -> Option<[f32; 4]> {
    let clean = hex.trim().strip_prefix('#')?;
    if (clean.len() != 6 && clean.len() != 8) || !clean.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    let r = f32::from(u8::from_str_radix(&clean[0..2], 16).ok()?) / 255.0;
    let g = f32::from(u8::from_str_radix(&clean[2..4], 16).ok()?) / 255.0;
    let b = f32::from(u8::from_str_radix(&clean[4..6], 16).ok()?) / 255.0;
    let a = if clean.len() == 8 {
        f32::from(u8::from_str_radix(&clean[6..8], 16).ok()?) / 255.0
    } else {
        1.0
    };
    Some([r, g, b, a])
}

/// Parses `#RRGGBB` into `0x00RRGGBB`.
#[must_use]
pub fn hex_to_u32_rgb(hex: &str) -> Option<u32> {
    let (r, g, b) = parse_hex_rgb(hex)?;
    Some((u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b))
}

/// Linearizes a normalized sRGB channel (`0.0..=1.0`) per IEC 61966-2-1 / WCAG 2.2.
#[must_use]
pub fn srgb_channel_to_linear(channel: f32) -> f32 {
    if channel <= 0.03928 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

/// Computes W3C WCAG 2.2 relative luminance (`0.0..=1.0`) from normalized sRGB channels.
#[must_use]
pub fn relative_luminance_srgb(r: f32, g: f32, b: f32) -> f32 {
    0.2126 * srgb_channel_to_linear(r)
        + 0.7152 * srgb_channel_to_linear(g)
        + 0.0722 * srgb_channel_to_linear(b)
}

/// Computes W3C WCAG 2.2 relative luminance from `u8` RGB channels.
#[must_use]
pub fn relative_luminance_u8(r: u8, g: u8, b: u8) -> f32 {
    relative_luminance_srgb(
        f32::from(r) / 255.0,
        f32::from(g) / 255.0,
        f32::from(b) / 255.0,
    )
}

/// Computes W3C WCAG 2.2 relative luminance from packed `0x00RRGGBB`.
#[must_use]
pub fn relative_luminance_u32(rgb: u32) -> f32 {
    relative_luminance_u8(
        ((rgb >> 16) & 0xff) as u8,
        ((rgb >> 8) & 0xff) as u8,
        (rgb & 0xff) as u8,
    )
}

/// Computes WCAG 2.2 contrast ratio `(L1 + 0.05) / (L2 + 0.05)` from precomputed luminances.
#[must_use]
pub fn contrast_ratio_from_luminances(l1: f32, l2: f32) -> f32 {
    let (hi, lo) = if l1 >= l2 { (l1, l2) } else { (l2, l1) };
    (hi + 0.05) / (lo + 0.05)
}

/// Computes WCAG 2.2 contrast ratio (`1.0..=21.0`) for opaque sRGB `[r, g, b]` colors.
#[must_use]
pub fn contrast_ratio_srgb(fg: [f32; 3], bg: [f32; 3]) -> f32 {
    contrast_ratio_from_luminances(
        relative_luminance_srgb(fg[0], fg[1], fg[2]),
        relative_luminance_srgb(bg[0], bg[1], bg[2]),
    )
}

/// Computes WCAG 2.2 contrast ratio for `[r, g, b, a]` foreground alpha-composited over opaque `bg`.
#[must_use]
pub fn contrast_ratio_rgba(fg: [f32; 4], bg: [f32; 3]) -> f32 {
    let alpha = fg[3].clamp(0.0, 1.0);
    let blended = if alpha < 1.0 {
        [
            fg[0] * alpha + bg[0] * (1.0 - alpha),
            fg[1] * alpha + bg[1] * (1.0 - alpha),
            fg[2] * alpha + bg[2] * (1.0 - alpha),
        ]
    } else {
        [fg[0], fg[1], fg[2]]
    };
    contrast_ratio_srgb(blended, bg)
}

/// Computes WCAG 2.2 contrast ratio between packed `0x00RRGGBB` foreground and background.
#[must_use]
pub fn contrast_ratio_u32(fg: u32, bg: u32) -> f32 {
    contrast_ratio_from_luminances(relative_luminance_u32(fg), relative_luminance_u32(bg))
}

/// Computes WCAG 2.2 contrast ratio between two `#RRGGBB` / `#RRGGBBAA` strings.
#[must_use]
pub fn contrast_ratio_hex(fg: &str, bg: &str) -> Option<f32> {
    let fg_rgba = parse_hex_rgba_f32(fg)?;
    let bg_rgba = parse_hex_rgba_f32(bg)?;
    Some(contrast_ratio_rgba(fg_rgba, [bg_rgba[0], bg_rgba[1], bg_rgba[2]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_design_tokens_match_spec_and_meet_wcag_aa() {
        assert_eq!(SOURCE_OF_TRUTH, "https://design.google/");
        assert_eq!(SYSTEM_NAME, "Material 3 Expressive");
        assert_eq!(SCHEMA_VERSION, 1);
        assert!(TYPEFACE_BRAND.contains("Google Sans Flex"));
        assert!(reference_palette_hex("gemini_blue") == Some("#4285F4"));
        assert_eq!(reference_palette_u32("glimmer_void"), Some(0x000000));
        assert!(axis_spec("opsz").is_some());
        assert_eq!(SPRINGS.len(), 11);
        assert_eq!(TYPESCALES.len(), 15);
        assert_eq!(classify_window_width(500.0), WindowSizeTier::Compact);
        assert_eq!(classify_window_width(1024.0), WindowSizeTier::Expanded);

        for mode in [
            ThemeModeSlug::Light,
            ThemeModeSlug::Dark,
            ThemeModeSlug::NeuralObsidian,
            ThemeModeSlug::OledPitch,
        ] {
            let p = palette_rgb(mode);
            for (label, fg, bg) in [
                ("on_background/background", p.on_background, p.background),
                ("on_surface/surface", p.on_surface, p.surface),
                ("on_surface_variant/surface_container", p.on_surface_variant, p.surface_container),
                ("on_primary/primary", p.on_primary, p.primary),
                ("on_secondary/secondary", p.on_secondary, p.secondary),
                ("on_tertiary/tertiary", p.on_tertiary, p.tertiary),
                ("on_error/error", p.on_error, p.error),
            ] {
                let ratio = contrast_ratio_u32(fg, bg);
                assert!(
                    ratio >= WCAG_AA_MIN_CONTRAST,
                    "{mode:?} {label} contrast {ratio:.2} < {WCAG_AA_MIN_CONTRAST}"
                );
            }
        }
    }
}
