//! Unified GPUI OS toolkit combining:
//! - [`cosmic_theme`]: Material 3 (`themes.json`) + COSMIC RON theme catalog, WCAG AA contrast checks, and `GlobalTheme` live sync.
//! - [`cosmic_shell`]: Libcosmic-inspired `OsConfigStore`, `PanelApplet` trait registry, `NavRail`, and `ContextDrawer`.
//! - [`reactive`]: Iced-style `ReactiveProgram` / `TaskCommand` / `SubscriptionSpec` hosted natively on GPUI entities.
//! - [`immediate`]: Egui-style `ImmediateUi` inspector widgets, `PropertyInspect`, and `gpui::canvas` telemetry charts.

pub mod cosmic_shell;
pub mod cosmic_theme;
pub mod google_design;
pub mod i18n;
pub mod immediate;
pub mod reactive;
pub mod starter_pack;
pub mod unified_runtime;

pub use cosmic_shell::{
    AppletSlot, ContextDrawer, NavItem, NavRail, OsConfigStore, PanelApplet, render_context_drawer,
    render_nav_rail,
};
pub use cosmic_theme::{
    AppearanceMode, BUNDLED_THEMES_JSON, OsColorTokens, OsLayoutTokens, OsTheme, OsThemeCatalog,
    OsThemeEntry, contrast_ratio, parse_cosmic_ron_palette, parse_hex_hsla,
};
pub use i18n::{
    BUNDLED_OS_LOCALES_TOML, LocaleEntry, OsI18nCatalog, PluralCategory, format_iso_utc,
    plural_category,
};
pub use immediate::{
    ColorSwatchRow, ImmediateMemory, ImmediateUi, KeyValueRow, PropertyInspect, SparklineChart,
    StatusBadge, StatusTone, sparkline_chart, status_badge,
};
pub use reactive::{
    FocusableActionChip, ProgressMeter, ReactiveHost, ReactiveProgram, SliderControl,
    SubscriptionSpec, TaskCommand, ToggleControl, focusable_action_chip, progress_meter,
    slider_control, toggle_control,
};
pub use google_design::{
    BUNDLED_GOOGLE_DESIGN_TOKENS_TOML, GoogleDesignTokens, GoogleSansFlexAxes, ShapeScale,
    SpringSpec, StateLayerOpacities, WindowSizeClass,
};
pub use starter_pack::{
    AgentSlotMeta, BUNDLED_SUBMODULE_PARITY_TOML, CoreWebVitalsBudgetSpec, MonorepoAuditSummary,
    ShadcnGpuiTheme, StarterAlert, StarterBadge, StarterButton, StarterCard, StarterKpiCard,
    StarterPackCatalog, StarterPrimitiveSpec, StarterSize, StarterSkeleton, StarterVariant,
    SubmoduleParityEntry, SubmoduleParityMatrix,
};
pub use unified_runtime::{
    AdaptiveViewport, FormFactor, FrameworkCapabilityDescriptor, FrameworkLayer,
    UnifiedPlatformProfile, UnifiedRuntime,
};
