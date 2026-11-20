//! # Aphrody — Unified Full-Stack Rust OS, GUI & AI Framework
//!
//! `aphrody` organizes the Aphrody + Zed + GPUI + COSMIC + Storytold ecosystem into a single
//! composable library facade (modeled after `dioxus`), enabling clean minimal binaries on Web/Wasm,
//! Headless MCP servers, Native Desktop/Mobile apps, and multi-backend UI stacks.

pub mod suite;

pub use brand;
pub use suite::{AphrodyPillarStatus, AphrodySuiteProfile};

#[cfg(feature = "auto")]
pub use aphrody_auto as auto;

#[cfg(feature = "desktop-shell")]
pub use aphrody_desktop as desktop_shell;

#[cfg(feature = "app-shell")]
pub use gpui_app_shell as app_shell;

#[cfg(feature = "kit")]
pub use gpui_kit as kit;

#[cfg(feature = "kit")]
pub use gpui_kit::material3 as m3;

#[cfg(feature = "session")]
pub use aphrody_session as session;

#[cfg(feature = "design")]
pub use aphrody_design as design;

#[cfg(feature = "ui")]
pub use aphrody_ui as ui;

#[cfg(feature = "craft")]
pub use aphrody_craft as craft;

#[cfg(feature = "pilot")]
pub use aphrody_pilot as pilot;

#[cfg(feature = "store")]
pub use aphrody_store as store;

#[cfg(feature = "installer")]
pub use aphrody_installer as installer;

#[cfg(feature = "cosmic")]
pub use cosmic_gpui as cosmic;

#[cfg(feature = "vpn")]
pub use aphrody_vpn as vpn;

#[cfg(feature = "browser")]
pub use aphrody_browser as browser;

/// Unified GUI & compositor backends placed cleanly alongside `crates/gpui`.
pub mod backends {
    #[cfg(feature = "gpui")]
    pub use gpui;
    #[cfg(feature = "gpui")]
    pub use gpui_ui;
    #[cfg(feature = "gpui")]
    pub use aphrody_nsh as gpui_nsh;
    #[cfg(feature = "iced")]
    pub use iced_gpui as iced;
    #[cfg(feature = "egui")]
    pub use egui_gpui as egui;
    #[cfg(feature = "bevy")]
    pub use bevy_gpui as bevy;
    #[cfg(feature = "tauri")]
    pub use tauri_gpui as tauri;
    #[cfg(feature = "cosmic")]
    pub use cosmic_gpui as cosmic;
    #[cfg(feature = "nsh")]
    pub mod nsh {
        pub use nsh_base as base;
        pub use nsh_core as core;
    }
}

/// Distributed Zed IDE / Workspace / Tree-sitter crates unified as modular Rust libraries.
#[cfg(any(feature = "editor", feature = "tree-sitter"))]
pub mod zed {
    #[cfg(feature = "editor")]
    pub use editor;
    #[cfg(feature = "editor")]
    pub use gpui_code;
    #[cfg(any(feature = "editor", feature = "tree-sitter"))]
    pub use language;
    #[cfg(feature = "editor")]
    pub use project;
    #[cfg(feature = "tree-sitter")]
    pub use tree_sitter;
    #[cfg(feature = "editor")]
    pub use workspace;
}

pub mod prelude {
    pub use brand::{APP_NAME, APP_SLUG};
    #[cfg(feature = "craft")]
    pub use aphrody_craft::{CraftCatalog, StorytoldEcosystemLedger};
    #[cfg(feature = "installer")]
    pub use aphrody_installer::{
        DesktopSwitchPlan, InstallerConfig, InstallerProfileCatalog, plan_desktop_switch,
        plan_installation,
    };
    #[cfg(feature = "pilot")]
    pub use aphrody_pilot::{PilotEngine, TargetBackend};
    #[cfg(feature = "session")]
    pub use aphrody_session::{CompositorState, CosmicSettingsDaemon, SessionSupervisor, TilingMode};
    #[cfg(feature = "store")]
    pub use aphrody_store::{AwesomeGpuiCatalog, StoreTarget};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facade_brand_identity_and_suite_profile_are_consistent() -> Result<(), serde_json::Error> {
        assert!(!crate::brand::APP_NAME.is_empty());
        assert!(!crate::brand::APP_SLUG.is_empty());
        let profile = AphrodySuiteProfile::current();
        assert_eq!(profile.total_pillars, 12);
        assert!(profile.has_pillar("brand-design"));
        assert_eq!(profile.theme_modes.len(), 4);
        assert!((profile.wcag_aa_min_contrast - 4.5).abs() < f32::EPSILON);
        let serialized = serde_json::to_string(&profile)?;
        assert!(serialized.contains("material3-kit"));
        assert!(serialized.contains("cosmic-os"));
        Ok(())
    }
}
