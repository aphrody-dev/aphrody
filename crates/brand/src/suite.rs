//! Single canonical registry of Aphrody's consolidated fork additions over upstream Zed.
//!
//! Consumed identically by `crates/aphrody`, `crates/aphrody_nsh`, `crates/aphrody_desktop`,
//! `crates/aphrody_cli`, `crates/gpui_app_shell`, and `crates/zed` so IDE actions, OS panels,
//! WebGPU/Wasm bundles, CLI output, and MCP tools share one authoritative source of truth.

/// Runtime target surfaces supported by an Aphrody fork pillar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum AphrodySurface {
    Ide,
    DesktopOs,
    WebGpuWasm,
    Cli,
    Mcp,
}

impl AphrodySurface {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ide => "ide",
            Self::DesktopOs => "desktop_os",
            Self::WebGpuWasm => "webgpu_wasm",
            Self::Cli => "cli",
            Self::Mcp => "mcp",
        }
    }
}

/// Declarative descriptor for one consolidated Aphrody fork pillar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct AphrodyPillarSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub primary_crate: &'static str,
    pub cargo_feature: &'static str,
    pub summary: &'static str,
    pub surfaces: &'static [AphrodySurface],
}

/// All 12 consolidated Aphrody fork pillars unified under the `aphrody` feature architecture.
pub const APHRODY_PILLARS: &[AphrodyPillarSpec] = &[
    AphrodyPillarSpec {
        id: "brand-design",
        name: "Brand & Material 3 Expressive Tokens",
        primary_crate: "brand",
        cargo_feature: "minimal",
        summary: "Declarative product.toml identity, 4 ThemeModeSlug modes, WindowSizeTier, springs, and WCAG 2.2 AA contrast verification.",
        surfaces: &[
            AphrodySurface::Ide,
            AphrodySurface::DesktopOs,
            AphrodySurface::WebGpuWasm,
            AphrodySurface::Cli,
            AphrodySurface::Mcp,
        ],
    },
    AphrodyPillarSpec {
        id: "material3-kit",
        name: "GPUI Kit & Compose Material 3 1.5.0 Studio",
        primary_crate: "gpui-component",
        cargo_feature: "kit",
        summary: "16 M3 Expressive components, Compose 1.5.0 Stable widgets, 35 MaterialShapes WGSL SDFs, Parametric Prototyping, and Figma DTCG.",
        surfaces: &[
            AphrodySurface::Ide,
            AphrodySurface::DesktopOs,
            AphrodySurface::WebGpuWasm,
        ],
    },
    AphrodyPillarSpec {
        id: "nsh-studio",
        name: "Native Shell (NSH) & Live TSX Preview",
        primary_crate: "aphrody_nsh",
        cargo_feature: "gpui",
        summary: "In-memory TSX hot-reload preview, embedded Tree-sitter GpuiCodeEditor component, and Material 3 NSH kit.",
        surfaces: &[AphrodySurface::Ide, AphrodySurface::WebGpuWasm],
    },
    AphrodyPillarSpec {
        id: "design-stitch",
        name: "Google Design Mirror & Stitch DESIGN.md",
        primary_crate: "aphrody_design",
        cargo_feature: "design",
        summary: "Live design.google mirror, Google Stitch screen synthesis, DESIGN.md lint, and Chrome Modern Web Guidance / CWV audits.",
        surfaces: &[
            AphrodySurface::Ide,
            AphrodySurface::DesktopOs,
            AphrodySurface::Cli,
            AphrodySurface::Mcp,
        ],
    },
    AphrodyPillarSpec {
        id: "craft-suite",
        name: "Creative Studio, Canvas & Binary Inspector",
        primary_crate: "aphrody_craft",
        cargo_feature: "craft",
        summary: "Unified creative workbench, vector/UI canvas GPUI/egui parity, Unreal Bevy 5.8 viewport, and multi-format binary/SVG/Wasm inspection.",
        surfaces: &[
            AphrodySurface::Ide,
            AphrodySurface::DesktopOs,
            AphrodySurface::WebGpuWasm,
            AphrodySurface::Cli,
            AphrodySurface::Mcp,
        ],
    },
    AphrodyPillarSpec {
        id: "cosmic-os",
        name: "COSMIC Epoch GPUI Shell & WebGPU Runtime",
        primary_crate: "aphrody_desktop",
        cargo_feature: "desktop-shell",
        summary: "Tiling compositor, floating pill dock, reactive settings, and Fat-LTO slim pure-WebGPU Wasm cohorts on web.aphrody.com.",
        surfaces: &[
            AphrodySurface::DesktopOs,
            AphrodySurface::WebGpuWasm,
            AphrodySurface::Cli,
        ],
    },
    AphrodyPillarSpec {
        id: "pilot-mcp",
        name: "AI OS Pilot & Headless Zed MCP",
        primary_crate: "aphrody_pilot",
        cargo_feature: "pilot",
        summary: "Unified AccessKit/UIA/AT-SPI2/AXUIElement automation, headless Zed AST controller, and JSON-RPC 2.0 stdio MCP server.",
        surfaces: &[
            AphrodySurface::Ide,
            AphrodySurface::DesktopOs,
            AphrodySurface::Cli,
            AphrodySurface::Mcp,
        ],
    },
    AphrodyPillarSpec {
        id: "browser",
        name: "Native Chromium & WebGPU Browser Engine",
        primary_crate: "aphrody_browser",
        cargo_feature: "browser",
        summary: "Embedded GPUI browser views and WebGPU runtime bridge.",
        surfaces: &[AphrodySurface::DesktopOs, AphrodySurface::WebGpuWasm],
    },
    AphrodyPillarSpec {
        id: "store",
        name: "Awesome GPUI Store & 22-Target Packager",
        primary_crate: "aphrody_store",
        cargo_feature: "store",
        summary: "Live-synced GPUI app catalog, Zed extensions registry, upt package bridge, and slim R2-hydrated Wasm catalog mode.",
        surfaces: &[
            AphrodySurface::DesktopOs,
            AphrodySurface::WebGpuWasm,
            AphrodySurface::Cli,
        ],
    },
    AphrodyPillarSpec {
        id: "installer",
        name: "CachyOS / Arch Net-Installer & DE Switcher",
        primary_crate: "aphrody_installer",
        cargo_feature: "installer",
        summary: "Declarative system profiles and live desktop environment switching.",
        surfaces: &[AphrodySurface::DesktopOs, AphrodySurface::Cli],
    },
    AphrodyPillarSpec {
        id: "vpn",
        name: "Native WireGuard VPN Module",
        primary_crate: "aphrody_vpn",
        cargo_feature: "vpn",
        summary: "Integrated Mullvad WireGuard tunnel control for GPUI App Shell.",
        surfaces: &[AphrodySurface::DesktopOs],
    },
    AphrodyPillarSpec {
        id: "auto",
        name: "Federated Auto Runtime Integration",
        primary_crate: "aphrody_auto",
        cargo_feature: "auto",
        summary: "Shared Aphrody x Auto telemetry and application surface hooks.",
        surfaces: &[AphrodySurface::DesktopOs, AphrodySurface::WebGpuWasm],
    },
];

/// Looks up an [`AphrodyPillarSpec`] by canonical `id`.
#[must_use]
pub fn pillar_by_id(id: &str) -> Option<&'static AphrodyPillarSpec> {
    APHRODY_PILLARS.iter().find(|pillar| pillar.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn pillar_registry_is_unique_and_complete() {
        assert_eq!(APHRODY_PILLARS.len(), 12);
        let mut ids = HashSet::new();
        for pillar in APHRODY_PILLARS {
            assert!(ids.insert(pillar.id), "duplicate pillar id: {}", pillar.id);
            assert!(!pillar.name.is_empty());
            assert!(!pillar.primary_crate.is_empty());
            assert!(!pillar.surfaces.is_empty());
        }
        assert!(pillar_by_id("material3-kit").is_some());
        assert!(pillar_by_id("cosmic-os").is_some());
        assert!(pillar_by_id("pilot-mcp").is_some());
    }
}
