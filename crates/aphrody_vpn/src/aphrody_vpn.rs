//! A native GPUI client for the Aphrody VPN daemon.
//!
//! [`VpnModule`] adds a dock panel, a pane item and their actions to any `gpui_app_shell`
//! application. The daemon is reached through `ayvpn-daemon` (the `vendor/ayvpn` submodule); the
//! view only knows the [`VpnBackend`] trait, so tests drive it with a fake.

mod backend;
mod daemon_backend;
mod panel;

#[cfg(test)]
mod panel_tests;

use std::sync::Arc;

use gpui_app_shell::AppModule;
use gpui_ui::gpui::{App, AppContext as _, KeyBinding};
use gpui_ui::{Context, Window};
use workspace::{AppState, Workspace};

pub use backend::{
    AccountStatus, City, ConnectionState, Country, ExitLocation, LocationSelection, Toggle,
    VpnBackend, VpnCommand, VpnSnapshot,
};
pub use daemon_backend::DaemonBackend;
pub use panel::{
    OpenVpn, PrimaryAction, StatusPresentation, ToggleConnection, ToggleFocus, VpnPane, VpnPanel,
    VpnView, normalize_account, present, time_left,
};

/// Registers the VPN panel, pane item and actions in every workspace window.
pub struct VpnModule {
    backend: Arc<dyn VpnBackend>,
    open_on_launch: bool,
}

impl VpnModule {
    /// A module backed by the local daemon.
    pub fn new() -> Self {
        Self::with_backend(Arc::new(DaemonBackend::new()))
    }

    pub fn with_backend(backend: Arc<dyn VpnBackend>) -> Self {
        Self {
            backend,
            open_on_launch: false,
        }
    }

    /// Opens the VPN view as a tab when a workspace opens, for an app that is only the VPN client.
    pub fn open_on_launch(mut self, open: bool) -> Self {
        self.open_on_launch = open;
        self
    }
}

impl Default for VpnModule {
    fn default() -> Self {
        Self::new()
    }
}

impl AppModule for VpnModule {
    fn name(&self) -> &'static str {
        "vpn"
    }

    fn init(&self, _app_state: &Arc<AppState>, cx: &mut App) {
        cx.bind_keys([KeyBinding::new("ctrl-alt-v", ToggleFocus, None)]);
    }

    fn configure_workspace(
        &self,
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        let backend = self.backend.clone();
        let view = cx.new(|cx| VpnView::new(backend, window, cx));
        let panel = cx.new(|_| VpnPanel::new(view.clone()));
        workspace.add_panel(panel, window, cx);
        panel::register_actions(workspace, view.clone());
        if self.open_on_launch {
            let pane = cx.new(|_| VpnPane::new(view));
            workspace.add_item_to_active_pane(Box::new(pane), None, true, window, cx);
        }
    }
}
