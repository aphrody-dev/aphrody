//! Native Shell (`nsh`) integration for `gpui_ui`.
//!
//! Unifies `longbridge/nsh` with `gpui-os` on a single `gpui` engine:
//! - Reactive SolidJS-style signals, memos, effects, and Tailwind layout (`nsh_core`)
//! - 35+ rich controls (`Plot` charts, `RichEditor`, `Calendar`/`DatePicker`, `ColorPicker`,
//!   `DataTable`, `EmojiPicker`, `TextView` Markdown + KaTeX math, WGSL `Shader`s) (`nsh_base`)
//! - Native GPUI surface embedding (`nsh_platform_gpui::AppView`) and `NativeComponent` bridge
//! - Optional QuickJS JIT TSX/JS sandbox (`nsh-js`), hot-reload compiler (`nsh-dev`), and
//!   Ratatui terminal grid renderer (`nsh-tui`)
//! - Live theme, appearance, and font-scale synchronization from `gpui_ui::Theme`.

use std::sync::Once;

use gpui::App;
use nsh_core::element::AdaptiveColor;
use nsh_core::platform::Appearance as NshAppearance;
use theme::{ActiveTheme as _, Appearance as GpuiAppearance, Theme as GpuiTheme};

pub use nsh_base;
#[cfg(feature = "nsh-dev")]
pub use nsh_bundler;
pub use nsh_core;
pub use nsh_engine;
pub use nsh_macros;
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
pub use nsh_platform_gpui;
#[cfg(feature = "nsh-tui")]
pub use nsh_platform_tui;

#[cfg(any(target_os = "linux", target_os = "macos", windows))]
pub use nsh_platform_gpui::{
    AppOptions, AppSource, AppView, AssetBase, CloseRequest, ComponentContext, ComponentFactory,
    Components, NativeComponent, WindowDisplay, open_app_window, run_app,
};

static NSH_UI_SETS_INIT: Once = Once::new();

/// Register the complete `nsh/base` component & token registry for all GPUI-embedded
/// `AppView` instances (idempotent).
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
pub fn ensure_nsh_ui_sets_registered() {
    NSH_UI_SETS_INIT.call_once(|| {
        nsh_platform_gpui::set_ui_sets(vec![nsh_base::registry()]);
    });
}

/// Convert a `gpui::Hsla` color into `nsh_core::Hsla`.
pub const fn gpui_hsla_to_nsh(color: gpui::Hsla) -> nsh_core::Hsla {
    nsh_core::color::hsla(color.h, color.s, color.l, color.a)
}

/// Convert an `nsh_core::Hsla` color into `gpui::Hsla`.
pub const fn nsh_hsla_to_gpui(color: nsh_core::Hsla) -> gpui::Hsla {
    gpui::Hsla {
        h: color.h,
        s: color.s,
        l: color.l,
        a: color.a,
    }
}

/// Build an `AdaptiveColor` using the same `gpui::Hsla` value for both appearances
/// (used when syncing the currently active `gpui_ui::Theme` directly into an `AppView`).
pub fn fixed_adaptive_color(color: gpui::Hsla) -> AdaptiveColor {
    let converted = gpui_hsla_to_nsh(color);
    AdaptiveColor::new(converted, converted)
}

/// Convert `gpui_ui`'s [`GpuiAppearance`] into NSH's [`NshAppearance`].
pub const fn nsh_appearance_from_gpui(appearance: GpuiAppearance) -> NshAppearance {
    match appearance {
        GpuiAppearance::Light => NshAppearance::Light,
        GpuiAppearance::Dark => NshAppearance::Dark,
    }
}

/// Convert a `gpui_ui` [`GpuiTheme`] (plus active font settings in `cx`) into an
/// [`nsh_base::Theme`] so embedded NSH surfaces match the host application's colors,
/// status tokens, chart palette, and font families.
pub fn nsh_theme_from_gpui(gpui_theme: &GpuiTheme, cx: &App) -> nsh_base::Theme {
    let colors = gpui_theme.colors();
    let status = gpui_theme.status();

    let mut theme = nsh_base::Theme::default();

    theme.set_color("background", fixed_adaptive_color(colors.background));
    theme.set_color("foreground", fixed_adaptive_color(colors.text));

    theme.set_color("card", fixed_adaptive_color(colors.surface_background));
    theme.set_color("card-foreground", fixed_adaptive_color(colors.text));

    theme.set_color(
        "popover",
        fixed_adaptive_color(colors.elevated_surface_background),
    );
    theme.set_color("popover-foreground", fixed_adaptive_color(colors.text));

    theme.set_color("primary", fixed_adaptive_color(colors.text_accent));
    theme.set_color(
        "primary-foreground",
        fixed_adaptive_color(colors.background),
    );

    theme.set_color("secondary", fixed_adaptive_color(colors.element_background));
    theme.set_color("secondary-foreground", fixed_adaptive_color(colors.text));

    theme.set_color("muted", fixed_adaptive_color(colors.panel_background));
    theme.set_color(
        "muted-foreground",
        fixed_adaptive_color(colors.text_muted),
    );

    theme.set_color("accent", fixed_adaptive_color(colors.element_hover));
    theme.set_color("accent-foreground", fixed_adaptive_color(colors.text));

    theme.set_color("destructive", fixed_adaptive_color(status.error));
    theme.set_color("border", fixed_adaptive_color(colors.border));
    theme.set_color("input", fixed_adaptive_color(colors.border_variant));
    theme.set_color("ring", fixed_adaptive_color(colors.border_focused));
    theme.set_color(
        "selection",
        fixed_adaptive_color(colors.element_selection_background),
    );

    // Chart palette derived from semantic status/accent colors
    theme.set_color("chart-1", fixed_adaptive_color(colors.text_accent));
    theme.set_color("chart-2", fixed_adaptive_color(status.success));
    theme.set_color("chart-3", fixed_adaptive_color(status.warning));
    theme.set_color("chart-4", fixed_adaptive_color(status.info));
    theme.set_color("chart-5", fixed_adaptive_color(status.modified));

    // Sidebar tokens mapped to panel surfaces
    theme.set_color("sidebar", fixed_adaptive_color(colors.panel_background));
    theme.set_color("sidebar-foreground", fixed_adaptive_color(colors.text));
    theme.set_color("sidebar-primary", fixed_adaptive_color(colors.text_accent));
    theme.set_color(
        "sidebar-primary-foreground",
        fixed_adaptive_color(colors.background),
    );
    theme.set_color(
        "sidebar-accent",
        fixed_adaptive_color(colors.ghost_element_hover),
    );
    theme.set_color(
        "sidebar-accent-foreground",
        fixed_adaptive_color(colors.text),
    );
    theme.set_color(
        "sidebar-border",
        fixed_adaptive_color(colors.border_variant),
    );
    theme.set_color("sidebar-ring", fixed_adaptive_color(colors.border_focused));

    let settings_provider = theme::theme_settings(cx);
    let ui_font_family = settings_provider.ui_font(cx).family.to_string();
    let buffer_font_family = settings_provider.buffer_font(cx).family.to_string();
    if !ui_font_family.starts_with('.') {
        theme.set_font(nsh_core::element::FONT_SANS, ui_font_family);
    }
    if !buffer_font_family.starts_with('.') {
        theme.set_font(nsh_core::element::FONT_MONO, buffer_font_family);
    }

    theme
}

/// Synchronize an embedded [`AppView`] with the active `gpui_ui` global theme,
/// light/dark appearance, and UI font scale.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
pub fn sync_app_view_with_gpui_theme(view: &AppView, cx: &mut App) {
    ensure_nsh_ui_sets_registered();
    let active_theme = cx.theme().clone();
    let appearance = nsh_appearance_from_gpui(active_theme.appearance());
    let mapped_theme = nsh_theme_from_gpui(&active_theme, cx);
    let ui_font_px = f32::from(theme::theme_settings(cx).ui_font_size(cx));
    let font_scale = (ui_font_px / 14.0).clamp(0.5, 3.0);

    view.set_appearance(appearance, cx);
    view.set_theme(Some(mapped_theme), cx);
    view.set_font_scale(Some(font_scale), cx);
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    fn test_gpui_to_nsh_theme_bridge(cx: &mut TestAppContext) {
        cx.update(|cx| {
            crate::init(cx);
            let active_theme = cx.theme().clone();
            let mapped = nsh_theme_from_gpui(&active_theme, cx);
            let expected_bg = gpui_hsla_to_nsh(active_theme.colors().background);
            assert_eq!(mapped.background.dark, expected_bg);
            assert_eq!(mapped.background.light, expected_bg);
        });
    }
}
