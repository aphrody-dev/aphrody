//! Deterministic Material 3 Expressive (`GoogleDesignTokens`) & `ShadcnGpuiTheme` -> `egui::Visuals`
//! converter preserving WCAG 2.2 AA contrast and M3 corner geometry.

use anyhow::Result;
use ecolor::Color32;
use egui::{CornerRadius, Stroke, Style, Visuals};
use epaint::Shadow;
use gpui::{Hsla, Rgba};
use aphrody_ui::AppearanceMode;
use aphrody_ui::google_design::GoogleDesignTokens;
use aphrody_ui::starter_pack::ShadcnGpuiTheme;

/// Converts a `gpui::Hsla` color into an sRGB unpremultiplied `egui::Color32`.
#[must_use]
pub fn hsla_to_color32(color: Hsla) -> Color32 {
    let rgb: Rgba = color.to_rgb();
    let to_u8 = |channel: f32| (channel.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color32::from_rgba_unmultiplied(to_u8(rgb.r), to_u8(rgb.g), to_u8(rgb.b), to_u8(rgb.a))
}

fn parse_hex_color32(hex: &str, fallback: Color32) -> Color32 {
    let Some(clean) = hex.trim().strip_prefix('#') else {
        return fallback;
    };
    if clean.len() != 6 {
        return fallback;
    }
    let (Ok(r), Ok(g), Ok(b)) = (
        u8::from_str_radix(&clean[0..2], 16),
        u8::from_str_radix(&clean[2..4], 16),
        u8::from_str_radix(&clean[4..6], 16),
    ) else {
        return fallback;
    };
    Color32::from_rgb(r, g, b)
}

/// Maps `GoogleDesignTokens` (Light or Dark) into a complete `egui::Visuals` palette.
#[must_use]
pub fn google_design_to_egui_visuals(tokens: &GoogleDesignTokens, mode: AppearanceMode) -> Visuals {
    let mut visuals = match mode {
        AppearanceMode::Light => Visuals::light(),
        AppearanceMode::Dark => Visuals::dark(),
    };
    let palette = tokens.palette(mode);
    let color = |role: &str, fallback: Color32| {
        palette
            .get(role)
            .map_or(fallback, |hex| parse_hex_color32(hex, fallback))
    };

    let surface = color("surface", visuals.panel_fill);
    let surface_low = color("surface_container_low", surface);
    let surface_mid = color("surface_container", surface_low);
    let surface_high = color("surface_container_high", surface_mid);
    let surface_lowest = color("surface_container_lowest", visuals.extreme_bg_color);
    let on_surface = color("on_surface", visuals.text_color());
    let on_surface_variant = color("on_surface_variant", on_surface);
    let primary = color("primary", visuals.hyperlink_color);
    let primary_container = color("primary_container", visuals.selection.bg_fill);
    let on_primary_container = color("on_primary_container", on_surface);
    let outline_variant = color("outline_variant", visuals.window_stroke.color);
    let outline = color("outline", outline_variant);
    let error = color("error", visuals.error_fg_color);

    let radius_sm = CornerRadius::same(tokens.sys.shape.small_px.round().clamp(0.0, 64.0) as u8);
    let radius_md = CornerRadius::same(tokens.sys.shape.medium_px.round().clamp(0.0, 64.0) as u8);
    let radius_lg = CornerRadius::same(tokens.sys.shape.large_px.round().clamp(0.0, 64.0) as u8);
    let radius_xl = CornerRadius::same(
        tokens
            .sys
            .shape
            .large_increased_px
            .round()
            .clamp(0.0, 64.0) as u8,
    );

    visuals.dark_mode = matches!(mode, AppearanceMode::Dark);
    visuals.override_text_color = Some(on_surface);
    visuals.weak_text_color = Some(on_surface_variant);
    visuals.panel_fill = surface;
    visuals.window_fill = surface_low;
    visuals.faint_bg_color = surface_mid;
    visuals.extreme_bg_color = surface_lowest;
    visuals.text_edit_bg_color = Some(surface_lowest);
    visuals.hyperlink_color = primary;
    visuals.error_fg_color = error;
    visuals.window_corner_radius = radius_lg;
    visuals.menu_corner_radius = radius_xl;
    visuals.window_stroke = Stroke::new(1.0, outline_variant);
    visuals.popup_shadow = Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(if visuals.dark_mode { 110 } else { 36 }),
    };

    visuals.selection.bg_fill = primary_container;
    visuals.selection.stroke = Stroke::new(1.5, on_primary_container);

    visuals.widgets.noninteractive.bg_fill = surface_low;
    visuals.widgets.noninteractive.weak_bg_fill = surface;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, outline_variant);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, on_surface_variant);
    visuals.widgets.noninteractive.corner_radius = radius_md;

    visuals.widgets.inactive.bg_fill = surface_mid;
    visuals.widgets.inactive.weak_bg_fill = surface_low;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, outline_variant);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, on_surface);
    visuals.widgets.inactive.corner_radius = radius_sm;

    visuals.widgets.hovered.bg_fill = surface_high;
    visuals.widgets.hovered.weak_bg_fill = surface_mid;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.5, outline);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.5, on_surface);
    visuals.widgets.hovered.corner_radius = radius_sm;

    visuals.widgets.active.bg_fill = primary_container;
    visuals.widgets.active.weak_bg_fill = primary_container;
    visuals.widgets.active.bg_stroke = Stroke::new(2.0, primary);
    visuals.widgets.active.fg_stroke = Stroke::new(1.5, on_primary_container);
    visuals.widgets.active.corner_radius = radius_md;

    visuals.widgets.open.bg_fill = surface_high;
    visuals.widgets.open.weak_bg_fill = surface_mid;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, primary);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, on_surface);
    visuals.widgets.open.corner_radius = radius_md;

    visuals
}

/// Converts a `ShadcnGpuiTheme` directly into `egui::Visuals`.
#[must_use]
pub fn shadcn_theme_to_egui_visuals(theme: &ShadcnGpuiTheme) -> Visuals {
    let mut visuals = match theme.mode {
        AppearanceMode::Light => Visuals::light(),
        AppearanceMode::Dark => Visuals::dark(),
    };
    visuals.dark_mode = matches!(theme.mode, AppearanceMode::Dark);
    visuals.panel_fill = hsla_to_color32(theme.background);
    visuals.window_fill = hsla_to_color32(theme.card);
    visuals.faint_bg_color = hsla_to_color32(theme.muted);
    visuals.extreme_bg_color = hsla_to_color32(theme.background);
    visuals.override_text_color = Some(hsla_to_color32(theme.foreground));
    visuals.weak_text_color = Some(hsla_to_color32(theme.muted_foreground));
    visuals.hyperlink_color = hsla_to_color32(theme.primary);
    visuals.error_fg_color = hsla_to_color32(theme.destructive);
    visuals.window_stroke = Stroke::new(1.0, hsla_to_color32(theme.border));
    visuals.selection.bg_fill = hsla_to_color32(theme.secondary);
    visuals.selection.stroke = Stroke::new(1.5, hsla_to_color32(theme.ring));
    visuals.window_corner_radius =
        CornerRadius::same(theme.radius_lg_px.round().clamp(0.0, 64.0) as u8);
    visuals.menu_corner_radius =
        CornerRadius::same(theme.radius_xl_px.round().clamp(0.0, 64.0) as u8);
    visuals
}

/// Applies bundled Google Design Material 3 Expressive tokens + comfortable M3 spacing to an `egui::Context`.
pub fn apply_google_design_theme(ctx: &egui::Context, mode: AppearanceMode) -> Result<()> {
    let tokens = GoogleDesignTokens::bundled()?;
    let visuals = google_design_to_egui_visuals(&tokens, mode);
    let mut style = Style {
        visuals,
        ..Style::default()
    };
    style.spacing.item_spacing = emath::vec2(8.0, 6.0);
    style.spacing.button_padding = emath::vec2(12.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(14);
    let egui_theme = match mode {
        AppearanceMode::Dark => egui::Theme::Dark,
        AppearanceMode::Light => egui::Theme::Light,
    };
    ctx.set_style_of(egui_theme, style);
    ctx.set_theme(egui_theme);
    Ok(())
}
