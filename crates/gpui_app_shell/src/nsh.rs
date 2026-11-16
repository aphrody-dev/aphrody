//! Native Shell (`nsh`) module for `gpui_app_shell`.
//!
//! Enables any `gpui_app_shell` application to mount reactive NSH surfaces
//! (Rust or TSX/QuickJS, live `.tsx` hot-reload previews, signed `.nshpkg`
//! packages, and `ws://` dev-server sessions) with embedded `GpuiCodeEditor`
//! support via `.with_module(NshModule::default())`.

use std::sync::Arc;

use gpui_ui::gpui::{App, AppContext as _, Context, Window};
pub use nsh_workspace::{
    ConnectDevServer, GpuiCodeEditorComponent, NshSurfaceItem, OpenLiveTsxPreview,
    OpenShowcaseSurface,
};
use workspace::{AppState, Workspace};

use crate::AppModule;

/// Pluggable [`AppModule`] registering Native Shell (`nsh`) actions and surface
/// capabilities into a `gpui_app_shell` application.
#[derive(Debug, Default, Clone, Copy)]
pub struct NshModule {
    /// Automatically open the interactive NSH + `GpuiCodeEditor` showcase tab on startup.
    pub open_showcase_on_launch: bool,
}

impl NshModule {
    /// Create an [`NshModule`] that opens the showcase surface when a workspace opens.
    pub const fn with_showcase() -> Self {
        Self {
            open_showcase_on_launch: true,
        }
    }
}

impl AppModule for NshModule {
    fn name(&self) -> &'static str {
        "nsh"
    }

    fn init(&self, _app_state: &Arc<AppState>, _cx: &mut App) {
        gpui_ui::nsh::ensure_nsh_ui_sets_registered();
    }

    fn configure_workspace(
        &self,
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        NshSurfaceItem::register(workspace, window, cx);
        if self.open_showcase_on_launch {
            let languages = workspace.app_state().languages.clone();
            let item = cx.new(|cx| NshSurfaceItem::showcase(languages, window, cx));
            workspace.add_item_to_active_pane(Box::new(item), None, true, window, cx);
        }
    }
}
