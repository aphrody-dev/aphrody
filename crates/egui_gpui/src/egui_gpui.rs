//! Native Zed / Aphrody GPUI bridge for `egui 0.37`, `epaint`, `egui_extras`, `egui-wgpu` (WGPU v30),
//! and Craft Studio immediate-mode controls styled with Google Design Material 3 Expressive tokens.
//!
//! Aligns `egui` onto the monorepo's pinned `aphrody-labs/wgpu` v30, `aphrody-labs/accesskit` 0.25, and
//! [`aphrody_ui::GoogleDesignTokens`] (`https://design.google/` Material 3 Expressive) without requiring
//! a secondary `winit` event loop inside GPUI windows.

pub mod craft_controls;
pub mod host;
pub mod theme_bridge;

pub use ecolor;
pub use egui;
#[cfg(feature = "wgpu")]
pub use egui_wgpu;
pub use egui_extras;
pub use emath;
pub use epaint;

pub use craft_controls::{
    CraftComboMenu, CraftDockTab, CraftDockTabs, CraftImeBanner, CraftPropertyRow,
    CraftToolbarChip,
};
pub use host::{
    EguiFrameArtifact, EguiFrameStats, EguiGpuiHost, ImeWireEvent, rasterize_primitives_rgba8,
};
pub use theme_bridge::{
    apply_google_design_theme, google_design_to_egui_visuals, hsla_to_color32,
    shadcn_theme_to_egui_visuals,
};
