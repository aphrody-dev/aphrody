use std::collections::BTreeMap;
use std::sync::Arc;

use gpui::{App, Hsla, Rgba, SharedString, WindowBackgroundAppearance, hsla};
use palette::{FromColor, Srgba};
use serde::{Deserialize, Serialize};
use syntax_theme::SyntaxTheme;
use theme::{
    AccentColors, Appearance, GlobalTheme, PlayerColors, StatusColors, SystemColors, Theme,
    ThemeColors, ThemeStyles, try_parse_color,
};

/// Embedded Material 3 + COSMIC + Omarchy + OLED theme catalog (`assets/os/themes.json`).
pub const BUNDLED_THEMES_JSON: &str = include_str!("../../../assets/os/themes.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppearanceMode {
    Light,
    #[default]
    Dark,
}

impl From<AppearanceMode> for Appearance {
    fn from(value: AppearanceMode) -> Self {
        match value {
            AppearanceMode::Light => Self::Light,
            AppearanceMode::Dark => Self::Dark,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OsLayoutTokens {
    #[serde(default = "default_gap")]
    pub gap: f32,
    #[serde(default = "default_section_gap")]
    pub section_gap: f32,
    #[serde(default = "default_focus_ring", rename = "focus-ring")]
    pub focus_ring: f32,
    #[serde(default = "default_window_radius", rename = "window-radius")]
    pub window_radius: f32,
    #[serde(default = "default_min_touch_target")]
    pub min_touch_target: f32,
    #[serde(default = "default_min_control_height")]
    pub min_control_height: f32,
}

fn default_gap() -> f32 {
    8.0
}
fn default_section_gap() -> f32 {
    16.0
}
fn default_focus_ring() -> f32 {
    2.0
}
fn default_window_radius() -> f32 {
    12.0
}
fn default_min_touch_target() -> f32 {
    44.0
}
fn default_min_control_height() -> f32 {
    36.0
}

impl Default for OsLayoutTokens {
    fn default() -> Self {
        Self {
            gap: default_gap(),
            section_gap: default_section_gap(),
            focus_ring: default_focus_ring(),
            window_radius: default_window_radius(),
            min_touch_target: default_min_touch_target(),
            min_control_height: default_min_control_height(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RawThemesDocument {
    #[serde(default)]
    pub layout: OsLayoutTokens,
    pub themes: BTreeMap<String, RawThemeEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RawThemeEntry {
    pub name: String,
    #[serde(default)]
    pub seed: String,
    #[serde(default)]
    pub variant: String,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    pub light: BTreeMap<String, String>,
    pub dark: BTreeMap<String, String>,
}

/// Resolved semantic color roles compatible with both Material 3 and Pop!_OS `libcosmic`.
#[derive(Debug, Clone)]
pub struct OsColorTokens {
    pub background: Hsla,
    pub on_background: Hsla,
    pub surface: Hsla,
    pub surface_dim: Hsla,
    pub surface_bright: Hsla,
    pub surface_container_lowest: Hsla,
    pub surface_container_low: Hsla,
    pub surface_container: Hsla,
    pub surface_container_high: Hsla,
    pub surface_container_highest: Hsla,
    pub on_surface: Hsla,
    pub on_surface_variant: Hsla,
    pub outline: Hsla,
    pub outline_variant: Hsla,
    pub primary: Hsla,
    pub primary_dim: Hsla,
    pub on_primary: Hsla,
    pub primary_container: Hsla,
    pub on_primary_container: Hsla,
    pub secondary: Hsla,
    pub on_secondary: Hsla,
    pub secondary_container: Hsla,
    pub on_secondary_container: Hsla,
    pub tertiary: Hsla,
    pub on_tertiary: Hsla,
    pub error: Hsla,
    pub on_error: Hsla,
    pub error_container: Hsla,
    pub on_error_container: Hsla,
}

#[derive(Debug, Clone)]
pub struct OsTheme {
    pub id: String,
    pub name: String,
    pub mode: AppearanceMode,
    pub colors: OsColorTokens,
    pub layout: OsLayoutTokens,
}

#[derive(Debug, Clone)]
pub struct OsThemeEntry {
    pub id: String,
    pub name: String,
    pub note: String,
    pub default_mode: AppearanceMode,
    pub light: OsColorTokens,
    pub dark: OsColorTokens,
}

/// Runtime catalog of all loaded Material 3 + COSMIC + user override themes.
#[derive(Debug, Clone)]
pub struct OsThemeCatalog {
    pub layout: OsLayoutTokens,
    pub entries: BTreeMap<String, OsThemeEntry>,
}

impl OsThemeCatalog {
    pub fn bundled() -> anyhow::Result<Self> {
        let mut catalog = Self::from_json_str(BUNDLED_THEMES_JSON)?;
        catalog.load_user_overrides();
        Ok(catalog)
    }

    pub fn from_json_str(json: &str) -> anyhow::Result<Self> {
        let parsed: RawThemesDocument = serde_json::from_str(json)?;
        let mut entries = BTreeMap::new();
        for (id, raw) in parsed.themes {
            let light = parse_token_map(&raw.light, AppearanceMode::Light);
            let dark = parse_token_map(&raw.dark, AppearanceMode::Dark);
            let default_mode = match raw.default.as_deref() {
                Some("light") => AppearanceMode::Light,
                _ => AppearanceMode::Dark,
            };
            entries.insert(
                id.clone(),
                OsThemeEntry {
                    id,
                    name: raw.name,
                    note: raw.note.unwrap_or_default(),
                    default_mode,
                    light,
                    dark,
                },
            );
        }
        Ok(Self {
            layout: parsed.layout,
            entries,
        })
    }

    /// Scans `$XDG_CONFIG_HOME/aphrody/themes` for `.ron` (COSMIC palette) or `.json` custom themes.
    pub fn load_user_overrides(&mut self) {
        let Some(cfg) = dirs::config_dir() else {
            return;
        };
        let theme_dir = cfg.join(brand::APP_SLUG).join("themes");
        let Ok(read_dir) = std::fs::read_dir(&theme_dir) else {
            return;
        };
        for item in read_dir.flatten() {
            let path = item.path();
            let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
                continue;
            };
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            if ext.eq_ignore_ascii_case("ron") {
                if let Some(entry) = parse_cosmic_ron_palette(
                    path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("cosmic-custom"),
                    &content,
                ) {
                    self.entries.insert(entry.id.clone(), entry);
                }
            } else if ext.eq_ignore_ascii_case("json")
                && let Ok(doc) = Self::from_json_str(&content)
            {
                self.entries.extend(doc.entries);
            }
        }
    }

    #[must_use]
    pub fn resolve(&self, id: &str, mode: Option<AppearanceMode>) -> Option<OsTheme> {
        let entry = self
            .entries
            .get(id)
            .or_else(|| self.entries.values().next())?;
        let chosen_mode = mode.unwrap_or(entry.default_mode);
        let colors = match chosen_mode {
            AppearanceMode::Light => entry.light.clone(),
            AppearanceMode::Dark => entry.dark.clone(),
        };
        Some(OsTheme {
            id: entry.id.clone(),
            name: entry.name.clone(),
            mode: chosen_mode,
            colors,
            layout: self.layout.clone(),
        })
    }

    #[must_use]
    pub fn ids(&self) -> Vec<String> {
        self.entries.keys().cloned().collect()
    }
}

impl OsTheme {
    /// Converts this `OsTheme` into a full Zed/GPUI [`Theme`] while preserving the active syntax theme if present.
    #[must_use]
    pub fn to_gpui_theme(&self, existing_syntax: Option<Arc<SyntaxTheme>>) -> Arc<Theme> {
        let c = &self.colors;
        let is_dark = matches!(self.mode, AppearanceMode::Dark);
        let mut tc = if is_dark {
            ThemeColors::dark()
        } else {
            ThemeColors::light()
        };

        tc.background = c.background;
        tc.surface_background = c.surface_container_low;
        tc.elevated_surface_background = c.surface_container_high;
        tc.panel_background = c.surface_container;
        tc.panel_overlay_background = c.surface_container_high;
        tc.title_bar_background = c.surface_container_lowest;
        tc.title_bar_inactive_background = c.surface_dim;
        tc.status_bar_background = c.surface_container_lowest;
        tc.toolbar_background = c.surface_container_low;
        tc.tab_bar_background = c.surface_container_low;
        tc.tab_inactive_background = c.surface_container;
        tc.tab_active_background = c.surface_container_highest;
        tc.element_background = c.surface_container_high;
        tc.element_hover = c.surface_container_highest;
        tc.element_active = c.primary_container;
        tc.element_selected = c.primary_container;
        tc.element_selection_background = c.primary_container.opacity(0.6);
        tc.ghost_element_hover = c.surface_container_high;
        tc.ghost_element_active = c.surface_container_highest;
        tc.ghost_element_selected = c.primary_container;
        tc.text = c.on_surface;
        tc.text_muted = c.on_surface_variant;
        tc.text_placeholder = c.outline;
        tc.text_accent = c.primary;
        tc.icon = c.on_surface;
        tc.icon_muted = c.on_surface_variant;
        tc.icon_accent = c.primary;
        tc.border = c.outline;
        tc.border_variant = c.outline_variant;
        tc.border_focused = c.primary;
        tc.border_selected = c.primary;
        tc.pane_focused_border = c.primary;
        tc.panel_focused_border = c.primary;

        let mut status = if is_dark {
            StatusColors::dark()
        } else {
            StatusColors::light()
        };
        status.error = c.error;
        status.error_background = c.error_container;
        status.error_border = c.error;
        status.info = c.primary;
        status.info_background = c.primary_container;
        status.info_border = c.primary;

        Arc::new(Theme {
            id: format!("os-{}-{}", self.id, if is_dark { "dark" } else { "light" }),
            name: SharedString::from(format!(
                "{} ({})",
                self.name,
                if is_dark { "Dark" } else { "Light" }
            )),
            appearance: self.mode.into(),
            styles: ThemeStyles {
                window_background_appearance: WindowBackgroundAppearance::Opaque,
                system: SystemColors::default(),
                accents: if is_dark {
                    AccentColors::dark()
                } else {
                    AccentColors::light()
                },
                colors: tc,
                status,
                player: if is_dark {
                    PlayerColors::dark()
                } else {
                    PlayerColors::light()
                },
                syntax: existing_syntax.unwrap_or_else(|| Arc::new(SyntaxTheme::default())),
            },
        })
    }

    /// Applies this OS theme directly to GPUI's global active `Theme` and stores [`OsTheme`] as a GPUI `Global`.
    pub fn apply_to_gpui(&self, cx: &mut App) {
        cx.set_global(self.clone());
        let existing_syntax = cx
            .try_global::<GlobalTheme>()
            .map(|_| GlobalTheme::theme(cx).styles.syntax.clone());
        let gpui_theme = self.to_gpui_theme(existing_syntax);
        if cx.has_global::<GlobalTheme>() {
            GlobalTheme::update_theme(cx, gpui_theme);
        } else {
            let icon_themes = theme::ThemeRegistry::global(cx);
            if let Ok(icon_theme) = icon_themes.default_icon_theme() {
                cx.set_global(GlobalTheme::new(gpui_theme, icon_theme));
            }
        }
    }

    #[must_use]
    pub fn try_global(cx: &App) -> Option<&Self> {
        cx.try_global::<Self>()
    }
}

impl gpui::Global for OsTheme {}

/// Parses `#RRGGBB` or `#RRGGBBAA` into [`Hsla`].
#[must_use]
pub fn parse_hex_hsla(raw: &str, fallback: Hsla) -> Hsla {
    try_parse_color(raw.trim()).unwrap_or(fallback)
}

fn token_color(map: &BTreeMap<String, String>, key: &str, fallback: Hsla) -> Hsla {
    map.get(key)
        .map(|raw| parse_hex_hsla(raw, fallback))
        .unwrap_or(fallback)
}

fn parse_token_map(map: &BTreeMap<String, String>, mode: AppearanceMode) -> OsColorTokens {
    let base_surface = match mode {
        AppearanceMode::Light => hsla(210.0 / 360.0, 0.2, 0.98, 1.0),
        AppearanceMode::Dark => hsla(210.0 / 360.0, 0.3, 0.08, 1.0),
    };
    let base_fg = match mode {
        AppearanceMode::Light => hsla(210.0 / 360.0, 0.25, 0.12, 1.0),
        AppearanceMode::Dark => hsla(210.0 / 360.0, 0.2, 0.90, 1.0),
    };
    let base_accent = hsla(200.0 / 360.0, 0.8, 0.55, 1.0);
    let base_error = hsla(0.0, 0.72, 0.52, 1.0);

    OsColorTokens {
        background: token_color(map, "background", base_surface),
        on_background: token_color(map, "on-background", base_fg),
        surface: token_color(map, "surface", base_surface),
        surface_dim: token_color(map, "surface-dim", base_surface),
        surface_bright: token_color(map, "surface-bright", base_surface),
        surface_container_lowest: token_color(map, "surface-container-lowest", base_surface),
        surface_container_low: token_color(map, "surface-container-low", base_surface),
        surface_container: token_color(map, "surface-container", base_surface),
        surface_container_high: token_color(map, "surface-container-high", base_surface),
        surface_container_highest: token_color(map, "surface-container-highest", base_surface),
        on_surface: token_color(map, "on-surface", base_fg),
        on_surface_variant: token_color(map, "on-surface-variant", base_fg),
        outline: token_color(map, "outline", base_fg),
        outline_variant: token_color(map, "outline-variant", base_fg),
        primary: token_color(map, "primary", base_accent),
        primary_dim: token_color(map, "primary-dim", base_accent),
        on_primary: token_color(map, "on-primary", base_surface),
        primary_container: token_color(map, "primary-container", base_surface),
        on_primary_container: token_color(map, "on-primary-container", base_fg),
        secondary: token_color(map, "secondary", base_accent),
        on_secondary: token_color(map, "on-secondary", base_surface),
        secondary_container: token_color(map, "secondary-container", base_surface),
        on_secondary_container: token_color(map, "on-secondary-container", base_fg),
        tertiary: token_color(map, "tertiary", base_accent),
        on_tertiary: token_color(map, "on-tertiary", base_surface),
        error: token_color(map, "error", base_error),
        on_error: token_color(map, "on-error", base_surface),
        error_container: token_color(map, "error-container", base_surface),
        on_error_container: token_color(map, "on-error-container", base_fg),
    }
}

/// Lightweight parser for COSMIC `.ron` palette expressions containing hex (`"#..."`) or `Srgba::new(r, g, b, a)` fields.
#[must_use]
pub fn parse_cosmic_ron_palette(id: &str, ron_text: &str) -> Option<OsThemeEntry> {
    let bg = extract_ron_color(ron_text, "bg_color")
        .or_else(|| extract_ron_color(ron_text, "background"))?;
    let on_bg = extract_ron_color(ron_text, "text_tint")
        .or_else(|| extract_ron_color(ron_text, "on_bg"))
        .unwrap_or_else(|| hsla(0.0, 0.0, 0.94, 1.0));
    let accent = extract_ron_color(ron_text, "accent")
        .or_else(|| extract_ron_color(ron_text, "primary"))
        .unwrap_or_else(|| hsla(185.0 / 360.0, 0.65, 0.62, 1.0));
    let is_dark = ron_text.contains("is_dark: true") || relative_luminance(bg) < 0.35;

    let tokens = OsColorTokens {
        background: bg,
        on_background: on_bg,
        surface: bg,
        surface_dim: bg,
        surface_bright: bg,
        surface_container_lowest: bg,
        surface_container_low: bg,
        surface_container: bg,
        surface_container_high: bg,
        surface_container_highest: bg,
        on_surface: on_bg,
        on_surface_variant: on_bg.opacity(0.84),
        outline: on_bg.opacity(0.45),
        outline_variant: on_bg.opacity(0.24),
        primary: accent,
        primary_dim: accent,
        on_primary: if relative_luminance(accent) > 0.4 {
            hsla(0.0, 0.0, 0.08, 1.0)
        } else {
            hsla(0.0, 0.0, 0.98, 1.0)
        },
        primary_container: accent.opacity(0.22),
        on_primary_container: on_bg,
        secondary: accent,
        on_secondary: bg,
        secondary_container: accent.opacity(0.16),
        on_secondary_container: on_bg,
        tertiary: accent,
        on_tertiary: bg,
        error: hsla(358.0 / 360.0, 0.78, 0.56, 1.0),
        on_error: hsla(0.0, 0.0, 1.0, 1.0),
        error_container: hsla(358.0 / 360.0, 0.72, 0.25, 1.0),
        on_error_container: hsla(0.0, 0.0, 0.96, 1.0),
    };

    Some(OsThemeEntry {
        id: id.to_owned(),
        name: format!("COSMIC {id}"),
        note: "Parsed from COSMIC RON palette".to_owned(),
        default_mode: if is_dark {
            AppearanceMode::Dark
        } else {
            AppearanceMode::Light
        },
        light: tokens.clone(),
        dark: tokens,
    })
}

fn extract_ron_color(text: &str, key: &str) -> Option<Hsla> {
    let idx = text.find(key)?;
    let slice = &text[idx + key.len()..];
    if let Some(q1) = slice.find('"')
        && let Some(q2) = slice[q1 + 1..].find('"')
    {
        let candidate = &slice[q1 + 1..q1 + 1 + q2];
        if candidate.starts_with('#') {
            return try_parse_color(candidate).ok();
        }
    }
    let start = slice.find("new(")? + 4;
    let end = slice[start..].find(')')? + start;
    let nums: Vec<f32> = slice[start..end]
        .split(',')
        .filter_map(|p| p.trim().parse::<f32>().ok())
        .collect();
    if nums.len() >= 3 {
        let srgba = Srgba::new(
            nums[0],
            nums[1],
            nums[2],
            nums.get(3).copied().unwrap_or(1.0),
        );
        let pal_hsla = palette::Hsla::from_color(srgba);
        return Some(hsla(
            pal_hsla.hue.into_positive_degrees() / 360.0,
            pal_hsla.saturation,
            pal_hsla.lightness,
            pal_hsla.alpha,
        ));
    }
    None
}

/// Computes W3C WCAG 2.2 relative luminance for an opaque/blended [`Hsla`] color.
#[must_use]
pub fn relative_luminance(color: Hsla) -> f32 {
    let rgb: Rgba = color.to_rgb();
    let lin = |c: f32| {
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(rgb.r) + 0.7152 * lin(rgb.g) + 0.0722 * lin(rgb.b)
}

/// Computes WCAG 2.2 contrast ratio `(L1 + 0.05) / (L2 + 0.05)` in range `1.0..=21.0`.
#[must_use]
pub fn contrast_ratio(fg: Hsla, bg: Hsla) -> f32 {
    let l1 = relative_luminance(fg);
    let l2 = relative_luminance(bg);
    let (hi, lo) = if l1 >= l2 { (l1, l2) } else { (l2, l1) };
    (hi + 0.05) / (lo + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_bundled_themes_meet_wcag_aa_contrast() {
        let catalog = OsThemeCatalog::bundled().expect("themes.json must load cleanly");
        assert!(catalog.entries.contains_key("aphrody"));
        assert!(catalog.entries.contains_key("cosmic"));

        for (id, _) in &catalog.entries {
            for mode in [AppearanceMode::Light, AppearanceMode::Dark] {
                let theme = catalog
                    .resolve(id, Some(mode))
                    .expect("theme resolves for both modes");
                let text_bg = contrast_ratio(theme.colors.on_surface, theme.colors.surface);
                let muted_card = contrast_ratio(
                    theme.colors.on_surface_variant,
                    theme.colors.surface_container,
                );
                let primary_btn = contrast_ratio(theme.colors.on_primary, theme.colors.primary);
                assert!(
                    text_bg >= 4.5,
                    "Theme {id} ({mode:?}) on_surface/surface contrast {text_bg:.2} < 4.5"
                );
                assert!(
                    muted_card >= 4.5,
                    "Theme {id} ({mode:?}) on_surface_variant/surface_container contrast {muted_card:.2} < 4.5"
                );
                assert!(
                    primary_btn >= 4.5,
                    "Theme {id} ({mode:?}) on_primary/primary contrast {primary_btn:.2} < 4.5"
                );
            }
        }
    }

    #[test]
    fn parses_cosmic_ron_palette_and_converts_to_gpui_theme() {
        let sample = r##"(
            is_dark: true,
            bg_color: Srgba::new(0.11, 0.11, 0.12, 1.0),
            text_tint: "#F0F4F8",
            accent: "#63D0DF",
        )"##;
        let parsed = parse_cosmic_ron_palette("sample-ron", sample).expect("RON parses");
        let theme = OsTheme {
            id: parsed.id,
            name: parsed.name,
            mode: parsed.default_mode,
            colors: parsed.dark,
            layout: OsLayoutTokens::default(),
        };
        let gpui_theme = theme.to_gpui_theme(None);
        assert_eq!(gpui_theme.appearance, Appearance::Dark);
    }
}
