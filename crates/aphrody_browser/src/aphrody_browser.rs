//! Complete Aphrody Browser + VPN + Chromium `//ui/views` in GPUI + Full WebGPU OS fusion.
//!
//! Unifies:
//! - pinned `aphrody-labs/browser` (`browser-engine`, `browser-vpn`, `browser-browse`, `browser-views`),
//! - Chromium `//ui/views` & `//chrome/browser/ui/views` rendered natively on GPUI (`chromium_gpui`),
//! - Fused `ayvpn` per-tab / per-context Mullvad/WireGuard/MASQUE tunnel shield,
//! - Absorbed `anomalyco/browse` real-browser extension pairing (`127.0.0.1:19988`), `@e1` ARIA snapshots, and WebMCP,
//! - Full WebGPU OS (`webgpu_os`) uniting `ai-os` + `cloudflare-os` (Gadgets/Gatekeepers/Workshop) + `nysh` (`nsh` + `aysh`).

pub mod agent_browser_core;
pub mod webgpu_os;

#[cfg(feature = "gpui-views")]
pub mod chromium_gpui;

pub use agent_browser_core::{
    AgentBrowserCore, AnnotatedElementBox, AnnotatedScreenshot, BrowserActionPolicy,
    BrowserBatchReport, BrowserBatchStep, BrowserBatchStepResult, BrowserControllerState,
    BrowserCookie, BrowserEventRingBuffer, BrowserLaunchOptions, BrowserPolicyDecision,
    BrowserStorageState, BrowserTabSession, BrowserUrlCheck, ConsoleLogEntry, DiffLine,
    DiffLineKind, DomainFilter, HarContentMode, HarEntry, HarRingBuffer, NetworkRoute,
    NetworkRouteTable, OriginStorage, PageErrorEntry, RouteDisposition, RouteResponse,
    SemanticLocator, SemanticLocatorKind, SnapshotDiff, StorageEntry, TabRef, TabSessionManager,
    diff_snapshot_text, filter_non_overlapping_annotations, find_ax_nodes_by_locator,
    project_annotation_boxes, redact_header_map, route_url_matches, snapshot_comparison_text,
    validate_navigation_url,
};
pub use browser_browse as browse;
pub use browser_detect as detect;
pub use browser_engine as engine;
pub use browser_views as views;
pub use browser_vpn as vpn;

#[cfg(feature = "gpui-views")]
pub use chromium_gpui::{
    ChromiumBrowserView, CloseActiveBrowserTab, CycleTabVpnExit, EscalateBrowserEngine,
    NewBrowserTab, ToggleBrowserSidePanel,
};
pub use webgpu_os::{
    AiOsKernelTask, CloudflareGatekeeperKind, CloudflareOsGadget, NyshShellCell, NyshTerminalGrid,
    WebGpuOsRuntime,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_browser_vpn_and_webgpu_os_pipeline_integrates() {
        let mut os = WebGpuOsRuntime::new_default();
        assert_eq!(os.browser_model.tabs.len(), 1);
        let jp_tab = os.open_isolated_tab(
            "Shenron Crawl JP",
            "https://example.jp/releases",
            vpn::RouteMode::PinnedExit("jp".into()),
        );
        assert!(jp_tab >= 2);
        let proxy = os
            .vpn_router
            .resolve_context_proxy(&vpn::RouteMode::PinnedExit("jp".into()))
            .expect("jp proxy");
        assert_eq!(proxy.exit_id.as_deref(), Some("jp"));
        os.exec_nysh_line("ay vpn status --json");
        assert!(!os.nysh_terminal.transcript().is_empty());
    }
}
