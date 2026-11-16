//! Unified GPUI OS toolkit combining:
//! - [`cosmic_theme`]: Material 3 (`themes.json`) + COSMIC RON theme catalog, WCAG AA contrast checks, and `GlobalTheme` live sync.
//! - [`cosmic_shell`]: Libcosmic-inspired `OsConfigStore`, `PanelApplet` trait registry, `NavRail`, and `ContextDrawer`.
//! - [`reactive`]: Iced-style `ReactiveProgram` / `TaskCommand` / `SubscriptionSpec` hosted natively on GPUI entities.
//! - [`immediate`]: Egui-style `ImmediateUi` inspector widgets, `PropertyInspect`, and `gpui::canvas` telemetry charts.

pub mod cosmic_shell;
pub mod cosmic_theme;
pub mod immediate;
pub mod reactive;

pub use cosmic_shell::{
    AppletSlot, NavItem, OsConfigStore, PanelApplet, render_context_drawer, render_nav_rail,
};
pub use cosmic_theme::{
    AppearanceMode, BUNDLED_THEMES_JSON, OsColorTokens, OsLayoutTokens, OsTheme, OsThemeCatalog,
    OsThemeEntry, contrast_ratio, parse_cosmic_ron_palette, parse_hex_hsla,
};
pub use immediate::{
    ImmediateMemory, ImmediateUi, PropertyInspect, StatusTone, sparkline_chart, status_badge,
};
pub use reactive::{
    ReactiveHost, ReactiveProgram, SubscriptionSpec, TaskCommand, focusable_action_chip,
    progress_meter, slider_control, toggle_control,
};
