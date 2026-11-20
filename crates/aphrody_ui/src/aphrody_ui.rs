//! Unified GPUI OS toolkit combining:
//! - [`cosmic_theme`]: Material 3 (`themes.json`) + COSMIC RON theme catalog, WCAG AA contrast checks, and `GlobalTheme` live sync.
//! - [`cosmic_shell`]: Libcosmic-inspired `OsConfigStore`, `PanelApplet` trait registry, `NavRail`, and `ContextDrawer`.
//! - [`reactive`]: Iced-style `ReactiveProgram` / `TaskCommand` / `SubscriptionSpec` hosted natively on GPUI entities.
//! - [`immediate`]: Egui-style `ImmediateUi` inspector widgets, `PropertyInspect`, and `gpui::canvas` telemetry charts.

pub mod cosmic_shell;
pub mod cosmic_theme;
pub mod google_design;
pub mod google_design_components;
pub mod google_design_mirror;
pub mod i18n;
pub mod immediate;
pub mod material_glass;
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
pub use material_glass::{
    GlassMaterialPreset, M3DynamicVariant, MaterialSymbolStyle, ResolvedGlassPlan,
    compute_m3_dynamic_scheme, lucide_icon_svg, m3_rgb_to_hsla, material_symbol_svg,
    os_theme_from_m3_seed,
};
#[cfg(any(target_os = "windows", target_os = "macos"))]
pub use material_glass::apply_native_window_glass;
pub use reactive::{
    FocusableActionChip, ProgressMeter, ReactiveHost, ReactiveProgram, SliderControl,
    SubscriptionSpec, TaskCommand, ToggleControl, focusable_action_chip, progress_meter,
    slider_control, toggle_control,
};
pub use google_design::{
    BUNDLED_GOOGLE_DESIGN_TOKENS_TOML, GoogleDesignTokens, GoogleSansFlexAxes, MirrorAccentPalette,
    ShapeScale, SpringSpec, StateLayerOpacities, WindowSizeClass,
};
pub use google_design_components::{
    CascadingPopoverCard, ExpressiveOmnibar, GroundingSourceCard, MobileBottomNavBar,
    MobileNavTab, PopoverMenuItem, RoadmapHorizonCard, SegmentedStudioBar, ThinkingAuraBadge,
};
pub use google_design_mirror::{
    GeminiSpectrumCard, GlowbarEntry, GlowbarShellCard, MATERIAL_COMPONENT_MAPPINGS,
    MaterialComponentMapping, MaterialComponentMatrix, QuireGlassNote, QuireStep,
    ReferenceGallery, ReferenceGalleryChange, ReferenceGalleryState, TransparentHudCard,
    YoutubeGradientScrubber, format_media_timestamp, seek_target_secs,
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
