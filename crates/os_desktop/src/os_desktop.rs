//! COSMIC Epoch desktop shell, compositor canvas, and unified ArtCraft + Craft Anything studio binary (`aphrody-os`).

use std::path::PathBuf;

use gpui::{App, Bounds, KeyBinding, WindowBounds, WindowOptions, prelude::*, px, size};
use os_ui::AppearanceMode;
use ui::IconName;

pub mod applets;
pub mod shell;
pub mod studio_view;
pub mod windows;

pub use shell::{
    CloseOverlays, CycleLayout, FocusNext, FocusPrev, OsDesktopShell, ToggleDarkMode,
    ToggleInspectorDrawer, ToggleLauncher,
};

/// Launch parameters shared between the live `aphrody-os` desktop binary and headless visual tests.
#[derive(Debug, Clone)]
pub struct DesktopOptions {
    pub initial_theme: Option<String>,
    pub initial_mode: Option<AppearanceMode>,
    pub initial_app: Option<String>,
    pub inspect_path: Option<PathBuf>,
    pub width: f32,
    pub height: f32,
}

impl Default for DesktopOptions {
    fn default() -> Self {
        Self {
            initial_theme: None,
            initial_mode: None,
            initial_app: None,
            inspect_path: None,
            width: 1440.0,
            height: 900.0,
        }
    }
}

/// Resolves an icon string from declarative TOML catalogs dynamically via [`strum::EnumString`].
#[must_use]
pub fn parse_icon_name(raw: &str) -> IconName {
    raw.parse::<IconName>().unwrap_or(IconName::Blocks)
}

/// Registers standard keyboard shortcuts for [`OsDesktopShell`] across GPUI desktop & headless contexts.
pub fn bind_desktop_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-k", ToggleLauncher, None),
        KeyBinding::new("cmd-k", ToggleLauncher, None),
        KeyBinding::new("ctrl-l", CycleLayout, None),
        KeyBinding::new("cmd-l", CycleLayout, None),
        KeyBinding::new("ctrl-shift-t", ToggleDarkMode, None),
        KeyBinding::new("cmd-shift-t", ToggleDarkMode, None),
        KeyBinding::new("ctrl-i", ToggleInspectorDrawer, None),
        KeyBinding::new("escape", CloseOverlays, None),
        KeyBinding::new("tab", FocusNext, None),
        KeyBinding::new("shift-tab", FocusPrev, None),
    ]);
}

/// Launches the live native GPU window hosting [`OsDesktopShell`].
pub fn run_desktop_app(options: DesktopOptions) -> anyhow::Result<()> {
    let bounds_size = size(px(options.width), px(options.height));
    gpui_ui::application().run(move |cx: &mut App| {
        gpui_ui::init(cx);
        bind_desktop_keys(cx);

        let bounds = Bounds::centered(None, bounds_size, cx);
        if let Err(err) = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some(format!("{} OS — ArtCraft Studio", brand::APP_NAME).into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            move |window, cx| cx.new(|cx| OsDesktopShell::new(options, window, cx)),
        ) {
            log::error!("failed to open {} desktop window: {err:#}", brand::APP_NAME);
            cx.quit();
            return;
        }
        cx.activate(true);
    });
    Ok(())
}

#[cfg(feature = "shell-module")]
pub struct ArtCraftShellModule;

#[cfg(feature = "shell-module")]
impl gpui_app_shell::AppModule for ArtCraftShellModule {
    fn name(&self) -> &'static str {
        "artcraft-os"
    }

    fn init(
        &self,
        _app_state: &std::sync::Arc<gpui_app_shell::workspace::AppState>,
        _cx: &mut App,
    ) {
        if let Err(err) = os_craft::CraftCatalog::load_with_user_override() {
            log::warn!("ArtCraftShellModule catalog bootstrap warning: {err:#}");
        }
    }

    fn configure_workspace(
        &self,
        _workspace: &mut gpui_app_shell::workspace::Workspace,
        _window: &mut gpui::Window,
        _cx: &mut gpui_ui::Context<gpui_app_shell::workspace::Workspace>,
    ) {
    }
}
