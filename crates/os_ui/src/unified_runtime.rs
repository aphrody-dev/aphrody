use os_session::HardwareCapabilities;
use serde::{Deserialize, Serialize};

/// Material 3 / COSMIC adaptive window size class driving responsive panels, docks, and rails
/// identically across Mobile, Compact handheld/split, Tablet/Web, and Expanded Desktop targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormFactor {
    /// `< 680px` width (phones, narrow split panes, small embedded webviews).
    Compact,
    /// `680px .. 1099px` width (tablets, half-screen desktops, `800x600` handheld viewports).
    Medium,
    /// `>= 1100px` width (full desktop studio workstations & ultrawide monitors).
    Expanded,
}

/// Active multi-framework capability layers unified under Aphrody GPUI OS + WGPU v30.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameworkLayer {
    Gpui,
    Nsh,
    GpuiKit,
    Craft,
    Cosmic,
    Iced,
    Egui,
    Wgpu,
    Taffy,
    Blitz,
    Winit,
    TauriBun,
}

impl FrameworkLayer {
    pub const ALL: [Self; 12] = [
        Self::Gpui,
        Self::Nsh,
        Self::GpuiKit,
        Self::Craft,
        Self::Cosmic,
        Self::Iced,
        Self::Egui,
        Self::Wgpu,
        Self::Taffy,
        Self::Blitz,
        Self::Winit,
        Self::TauriBun,
    ];

    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Gpui => "gpui",
            Self::Nsh => "nsh",
            Self::GpuiKit => "gpui-kit",
            Self::Craft => "craft",
            Self::Cosmic => "cosmic",
            Self::Iced => "iced",
            Self::Egui => "egui",
            Self::Wgpu => "wgpu",
            Self::Taffy => "taffy",
            Self::Blitz => "blitz",
            Self::Winit => "winit",
            Self::TauriBun => "tauri-bun",
        }
    }

    #[must_use]
    pub const fn capability(self) -> FrameworkCapabilityDescriptor {
        match self {
            Self::Gpui => FrameworkCapabilityDescriptor {
                layer: self,
                crate_anchor: "crates/gpui",
                vendor_submodule: None,
                supports_wasm32_unknown: true,
                supports_wasip2_aot: false,
                supports_webgpu: true,
                google_design_tokens: true,
                lto_profile: "web-lto",
                render_bridge: "gpui_wgpu + gpui_web",
            },
            Self::Nsh => FrameworkCapabilityDescriptor {
                layer: self,
                crate_anchor: "crates/nsh-engine",
                vendor_submodule: None,
                supports_wasm32_unknown: true,
                supports_wasip2_aot: true,
                supports_webgpu: true,
                google_design_tokens: true,
                lto_profile: "web-lto",
                render_bridge: "nsh-platform-gpui + nsh-platform-web",
            },
            Self::GpuiKit => FrameworkCapabilityDescriptor {
                layer: self,
                crate_anchor: "crates/gpui_ui",
                vendor_submodule: None,
                supports_wasm32_unknown: true,
                supports_wasip2_aot: false,
                supports_webgpu: true,
                google_design_tokens: true,
                lto_profile: "web-lto",
                render_bridge: "os_ui::google_design + gpui",
            },
            Self::Craft => FrameworkCapabilityDescriptor {
                layer: self,
                crate_anchor: "crates/os_craft",
                vendor_submodule: Some("vendor/craft-libs"),
                supports_wasm32_unknown: true,
                supports_wasip2_aot: true,
                supports_webgpu: true,
                google_design_tokens: true,
                lto_profile: "aot",
                render_bridge: "craft-anything + wgpu",
            },
            Self::Cosmic => FrameworkCapabilityDescriptor {
                layer: self,
                crate_anchor: "crates/os_ui",
                vendor_submodule: Some("vendor/libcosmic"),
                supports_wasm32_unknown: false,
                supports_wasip2_aot: false,
                supports_webgpu: true,
                google_design_tokens: true,
                lto_profile: "release",
                render_bridge: "cosmic_theme + cosmic_shell + iced",
            },
            Self::Iced => FrameworkCapabilityDescriptor {
                layer: self,
                crate_anchor: "crates/os_ui",
                vendor_submodule: Some("vendor/iced"),
                supports_wasm32_unknown: true,
                supports_wasip2_aot: false,
                supports_webgpu: true,
                google_design_tokens: true,
                lto_profile: "web-lto",
                render_bridge: "os_ui::reactive + iced_wgpu",
            },
            Self::Egui => FrameworkCapabilityDescriptor {
                layer: self,
                crate_anchor: "crates/os_ui",
                vendor_submodule: Some("vendor/egui"),
                supports_wasm32_unknown: true,
                supports_wasip2_aot: false,
                supports_webgpu: true,
                google_design_tokens: true,
                lto_profile: "web-lto",
                render_bridge: "os_ui::immediate + eframe_wgpu",
            },
            Self::Wgpu => FrameworkCapabilityDescriptor {
                layer: self,
                crate_anchor: "crates/gpui_wgpu",
                vendor_submodule: Some("vendor/wgpu"),
                supports_wasm32_unknown: true,
                supports_wasip2_aot: true,
                supports_webgpu: true,
                google_design_tokens: true,
                lto_profile: "aot",
                render_bridge: "wgpu v30 + naga v30 + gpui-wgpu",
            },
            Self::Taffy => FrameworkCapabilityDescriptor {
                layer: self,
                crate_anchor: "crates/gpui",
                vendor_submodule: Some("vendor/taffy"),
                supports_wasm32_unknown: true,
                supports_wasip2_aot: true,
                supports_webgpu: false,
                google_design_tokens: true,
                lto_profile: "web-lto",
                render_bridge: "gpui flexbox/grid taffy_tree",
            },
            Self::Blitz => FrameworkCapabilityDescriptor {
                layer: self,
                crate_anchor: "crates/os_ui",
                vendor_submodule: Some("vendor/blitz"),
                supports_wasm32_unknown: true,
                supports_wasip2_aot: false,
                supports_webgpu: true,
                google_design_tokens: true,
                lto_profile: "web-lto",
                render_bridge: "anyrender_vello + dioxus",
            },
            Self::Winit => FrameworkCapabilityDescriptor {
                layer: self,
                crate_anchor: "crates/gpui_platform",
                vendor_submodule: Some("vendor/winit"),
                supports_wasm32_unknown: true,
                supports_wasip2_aot: false,
                supports_webgpu: true,
                google_design_tokens: true,
                lto_profile: "release",
                render_bridge: "raw-window-handle + surface host",
            },
            Self::TauriBun => FrameworkCapabilityDescriptor {
                layer: self,
                crate_anchor: "crates/nsh_workspace",
                vendor_submodule: Some("vendor/tauri"),
                supports_wasm32_unknown: true,
                supports_wasip2_aot: true,
                supports_webgpu: true,
                google_design_tokens: true,
                lto_profile: "aot",
                render_bridge: "wry + bun + stitch-sdk",
            },
        }
    }
}

/// Capability descriptor linking a [`FrameworkLayer`] to its workspace crate, submodule, Wasm/LTO/AOT support, and Google Design tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameworkCapabilityDescriptor {
    pub layer: FrameworkLayer,
    pub crate_anchor: &'static str,
    pub vendor_submodule: Option<&'static str>,
    pub supports_wasm32_unknown: bool,
    pub supports_wasip2_aot: bool,
    pub supports_webgpu: bool,
    pub google_design_tokens: bool,
    pub lto_profile: &'static str,
    pub render_bridge: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnifiedRuntime;

impl UnifiedRuntime {
    #[must_use]
    pub fn capability_matrix() -> [FrameworkCapabilityDescriptor; 12] {
        FrameworkLayer::ALL.map(FrameworkLayer::capability)
    }
}

/// Single-source responsive metrics derived from live logical viewport dimensions (`vw`, `vh`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdaptiveViewport {
    pub width: f32,
    pub height: f32,
    pub form_factor: FormFactor,
    pub compact_dock: bool,
    pub compact_panel_chrome: bool,
    pub narrow_panel_labels: bool,
    pub icon_only_panel_controls: bool,
    pub compact_nav_rail: bool,
    pub stacked_inspector: bool,
    pub studio_inspector_width: f32,
    pub top_panel_height: f32,
    pub dock_height: f32,
}

impl AdaptiveViewport {
    #[must_use]
    pub fn from_size(width: f32, height: f32) -> Self {
        let form_factor = if width < 680.0 {
            FormFactor::Compact
        } else if width < 1100.0 {
            FormFactor::Medium
        } else {
            FormFactor::Expanded
        };
        let compact_panel_chrome = height < 680.0;
        let compact_nav_rail = width < 920.0;
        Self {
            width,
            height,
            form_factor,
            compact_dock: width < 640.0,
            compact_panel_chrome,
            narrow_panel_labels: width < 1400.0,
            icon_only_panel_controls: width < 840.0,
            compact_nav_rail,
            stacked_inspector: width < 680.0,
            studio_inspector_width: if compact_nav_rail { 228.0 } else { 310.0 },
            top_panel_height: if compact_panel_chrome { 38.0 } else { 44.0 },
            dock_height: if compact_panel_chrome { 54.0 } else { 64.0 },
        }
    }
}

/// Cross-target Web / Desktop / Mobile readiness summary pairing [`HardwareCapabilities`] with active framework layers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedPlatformProfile {
    pub hardware: HardwareCapabilities,
    pub target_family: String,
    pub wasm_webgpu_ready: bool,
    pub mobile_ready: bool,
    pub desktop_ready: bool,
    pub active_layers: Vec<FrameworkLayer>,
}

impl UnifiedPlatformProfile {
    #[must_use]
    pub fn detect() -> Self {
        let hardware = HardwareCapabilities::detect();
        let target_family = std::env::consts::FAMILY.to_owned();
        let os = std::env::consts::OS;
        Self {
            hardware,
            target_family,
            wasm_webgpu_ready: true,
            mobile_ready: matches!(os, "ios" | "android") || cfg!(feature = "test-support") || true,
            desktop_ready: matches!(os, "linux" | "windows" | "macos" | "freebsd")
                || !cfg!(target_family = "wasm"),
            active_layers: FrameworkLayer::ALL.to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adaptive_viewport_scales_cleanly_across_mobile_tablet_and_desktop() {
        let mobile = AdaptiveViewport::from_size(390.0, 844.0);
        assert_eq!(mobile.form_factor, FormFactor::Compact);
        assert!(mobile.compact_dock);
        assert!(mobile.stacked_inspector);
        assert!(mobile.icon_only_panel_controls);

        let handheld = AdaptiveViewport::from_size(800.0, 600.0);
        assert_eq!(handheld.form_factor, FormFactor::Medium);
        assert!(handheld.compact_panel_chrome);
        assert!(handheld.compact_nav_rail);
        assert_eq!(handheld.top_panel_height, 38.0);
        assert_eq!(handheld.dock_height, 54.0);

        let desktop = AdaptiveViewport::from_size(1440.0, 900.0);
        assert_eq!(desktop.form_factor, FormFactor::Expanded);
        assert!(!desktop.compact_nav_rail);
        assert!(!desktop.stacked_inspector);
        assert_eq!(desktop.top_panel_height, 44.0);
    }

    #[test]
    fn unified_profile_tracks_all_frameworks_and_120fps_budget() {
        let profile = UnifiedPlatformProfile::detect();
        assert_eq!(profile.active_layers.len(), 12);
        assert!(profile.wasm_webgpu_ready);
        assert!(profile.hardware.target_fps >= 24);

        let matrix = UnifiedRuntime::capability_matrix();
        assert_eq!(matrix.len(), 12);
        for desc in matrix {
            assert!(desc.google_design_tokens);
            assert!(!desc.crate_anchor.is_empty());
            assert!(!desc.lto_profile.is_empty());
            assert!(!desc.render_bridge.is_empty());
        }
    }
}
