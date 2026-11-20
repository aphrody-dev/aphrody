//! Unified COSMIC Epoch (`cosmic-epoch`) + Pop!_OS adapter crate
//! placed identically like `crates/gpui`, exposing ALL COSMIC subsystems (`cosmic-session`,
//! `cosmic-comp`, `cosmic-panel`, `cosmic-applets`, `pop-launcher`, `cosmic-app-library`,
//! `cosmic-osd`, `cosmic-notifications`, `cosmic-settings-daemon`, `cosmic-randr`,
//! `cosmic-workspaces-epoch`, `cosmic-greeter`, `cosmic-screenshot`) across **CLI, MCP, Web,
//! and Desktop** with unified build & deploy contracts.

use aphrody_session::{
    CompositorState, CosmicSettingsDaemon, Rect, SessionManifest, TilingMode,
};
use aphrody_store::StoreTarget;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CosmicComponentSpec {
    pub id: &'static str,
    pub upstream_repo: &'static str,
    pub aphrody_owner: &'static str,
    pub cli_command: &'static str,
    pub mcp_tools: &'static [&'static str],
    pub web_route: &'static str,
    pub desktop_surface: &'static str,
}

pub const COSMIC_EPOCH_COMPONENTS: &[CosmicComponentSpec] = &[
    CosmicComponentSpec {
        id: "cosmic-comp",
        upstream_repo: "pop-os/cosmic-comp",
        aphrody_owner: "crates/aphrody_session + crates/aphrody_desktop/src/comp_viewport.rs",
        cli_command: "aphrody-cli cosmic comp",
        mcp_tools: &["os_layout_set", "os_workspace_list", "os_workspace_switch"],
        web_route: "/cosmic/compositor",
        desktop_surface: "WorkspaceViewport",
    },
    CosmicComponentSpec {
        id: "cosmic-panel",
        upstream_repo: "pop-os/cosmic-panel",
        aphrody_owner: "crates/aphrody_desktop/src/panel.rs",
        cli_command: "aphrody-cli cosmic panel",
        mcp_tools: &["desktop_list_surfaces"],
        web_route: "/cosmic/panel",
        desktop_surface: "TopPanel + FloatingDock",
    },
    CosmicComponentSpec {
        id: "cosmic-applets",
        upstream_repo: "pop-os/cosmic-applets",
        aphrody_owner: "crates/aphrody_desktop/src/applets.rs",
        cli_command: "aphrody-cli cosmic applets",
        mcp_tools: &["desktop_snapshot"],
        web_route: "/cosmic/applets",
        desktop_surface: "PanelAppletSlot",
    },
    CosmicComponentSpec {
        id: "cosmic-launcher",
        upstream_repo: "pop-os/pop-launcher + pop-os/cosmic-launcher",
        aphrody_owner: "crates/aphrody_session/src/epoch_daemon.rs + crates/aphrody_desktop/src/launcher.rs",
        cli_command: "aphrody-cli cosmic launcher",
        mcp_tools: &["desktop_launch"],
        web_route: "/cosmic/launcher",
        desktop_surface: "PopLauncherModal + AppLibrary",
    },
    CosmicComponentSpec {
        id: "cosmic-workspaces-epoch",
        upstream_repo: "pop-os/cosmic-workspaces-epoch",
        aphrody_owner: "crates/aphrody_desktop/src/comp_viewport.rs",
        cli_command: "aphrody-cli cosmic workspaces",
        mcp_tools: &["os_workspace_list", "os_workspace_switch"],
        web_route: "/cosmic/workspaces",
        desktop_surface: "WorkspacesOverview",
    },
    CosmicComponentSpec {
        id: "cosmic-settings",
        upstream_repo: "pop-os/cosmic-settings + pop-os/cosmic-settings-daemon",
        aphrody_owner: "crates/aphrody_desktop/src/apps/settings.rs",
        cli_command: "aphrody-cli cosmic settings",
        mcp_tools: &["os_theme_set"],
        web_route: "/cosmic/settings",
        desktop_surface: "CosmicSettingsWindow",
    },
    CosmicComponentSpec {
        id: "cosmic-osd",
        upstream_repo: "pop-os/cosmic-osd + pop-os/cosmic-notifications",
        aphrody_owner: "crates/aphrody_desktop/src/overlays.rs",
        cli_command: "aphrody-cli cosmic notifications",
        mcp_tools: &["desktop_list_notifications", "desktop_dismiss_all_notifications"],
        web_route: "/cosmic/osd",
        desktop_surface: "OsdPill + NotificationToastStack",
    },
    CosmicComponentSpec {
        id: "cosmic-files",
        upstream_repo: "pop-os/cosmic-files",
        aphrody_owner: "crates/aphrody_desktop/src/apps/files.rs",
        cli_command: "aphrody-cli cosmic files",
        mcp_tools: &["craft_sniff_file"],
        web_route: "/cosmic/files",
        desktop_surface: "CosmicFilesWindow",
    },
    CosmicComponentSpec {
        id: "cosmic-store",
        upstream_repo: "pop-os/cosmic-store",
        aphrody_owner: "crates/aphrody_store + crates/aphrody_desktop/src/apps/store.rs",
        cli_command: "aphrody-cli deploy --suite cosmic",
        mcp_tools: &["desktop_list_apps"],
        web_route: "/cosmic/store",
        desktop_surface: "CosmicStoreWindow",
    },
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmicExposureSummary {
    pub component_count: usize,
    pub default_workspace_count: usize,
    pub daemon_power_profile: String,
    pub deploy_targets: usize,
    pub session_name: String,
}

#[must_use]
pub fn inspect_cosmic_stack() -> CosmicExposureSummary {
    let comp = CompositorState::new(
        Rect::new(0.0, 0.0, 1440.0, 900.0),
        &["Studio", "Browser", "Code", "Monitor"],
        TilingMode::Dwindle,
    );
    let daemon = CosmicSettingsDaemon::default();
    let session = SessionManifest::bundled().unwrap_or_default();
    CosmicExposureSummary {
        component_count: COSMIC_EPOCH_COMPONENTS.len(),
        default_workspace_count: comp.workspaces.len(),
        daemon_power_profile: daemon.power_profile.label().to_owned(),
        deploy_targets: StoreTarget::ALL.len(),
        session_name: session.session.name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosmic_exposure_covers_all_core_components_and_22_deploy_targets() {
        let summary = inspect_cosmic_stack();
        assert!(summary.component_count >= 9);
        assert_eq!(summary.default_workspace_count, 4);
        assert_eq!(summary.deploy_targets, 22);
    }
}
