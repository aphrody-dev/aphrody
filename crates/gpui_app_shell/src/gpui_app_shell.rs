//! # `gpui_app_shell` — Production-Ready GPUI Application Starter Pack
//!
//! Provides a complete, customizable, domain-agnostic application shell built on
//! [`gpui_ui`] and [`gpui_code`]:
//! - Multi-window & multi-workspace docking (`Workspace`, left/right/bottom `Dock`, split `Pane`s, tabs)
//! - Command Palette (`Ctrl+Shift+P` / `Cmd+Shift+P`), Keymaps, and `which_key`
//! - JSON `SettingsStore`, Themes & Icon Themes, SQLite `AppDatabase` + `KeyValueStore`
//! - Pluggable [`AppModule`] trait so any domain (DB client, RE workbench, audio DAW,
//!   CAD tool, terminal multiplexer, or custom IDE) can register panels, items, actions,
//!   and status bar widgets without modifying core crates.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context as _, Result};
use client::{Client, UserStore};
use db::AppDatabase;
use db::kvp::KeyValueStore;
use fs::{Fs, RealFs};
use gpui_code::languages;
use gpui_code::{Editor, LanguageRegistry, Project, project};
use gpui_tokio::Tokio;
use gpui_ui::gpui::{
    App, AppContext as _, Application, AsyncWindowContext, Bounds, KeyBinding, Point,
    SharedString, TaskExt as _, TitlebarOptions, WeakEntity, Window, WindowBounds, WindowKind,
    WindowOptions, point, px, size,
};
use gpui_ui::{ActiveTheme as _, Assets};
use node_runtime::NodeRuntime;
use release_channel::ReleaseChannel;
use reqwest_client::ReqwestClient;
use semver::Version;
use session::{AppSession, Session};
use settings::{Settings as _, SettingsStore};
use theme_settings::ThemeSettings;
use util::ResultExt as _;
use uuid::Uuid;
use workspace::{AppState, MultiWorkspace, Panel, Workspace, WorkspaceSettings, WorkspaceStore};

pub use activity_indicator;
pub use breadcrumbs;
pub use command_palette;
pub use command_palette_hooks;
pub use db;
pub use gpui_code;
pub use gpui_ui;
pub use keymap_editor;
pub use notifications;
pub use paths;
pub use picker;
pub use platform_title_bar;
pub use recent_projects;
pub use session;
pub use settings;
pub use task;
pub use theme_selector;
pub use theme_settings;
pub use title_bar;
pub use ui_prompt;
pub use which_key;
pub use workspace;
pub use zed_actions;

#[cfg(feature = "panels")]
pub use file_finder;
#[cfg(feature = "panels")]
pub use go_to_line;
#[cfg(feature = "panels")]
pub use outline_panel;
#[cfg(feature = "panels")]
pub use project_panel;
#[cfg(feature = "panels")]
pub use tab_switcher;
#[cfg(feature = "panels")]
pub use tasks_ui;

#[cfg(feature = "nsh")]
pub mod nsh;

/// Pluggable module / extension contract for `gpui_app_shell` applications.
///
/// External crates can implement `AppModule` to register custom GPUI actions,
/// settings, dock panels, pane items, status-bar items, or language adapters.
pub trait AppModule: 'static {
    /// Unique identifier for logging and diagnostics.
    fn name(&self) -> &'static str;

    /// Called once during application startup after `AppState` and `SettingsStore` are ready.
    fn init(&self, _app_state: &Arc<AppState>, _cx: &mut App) {}

    /// Called whenever a new [`Workspace`] is created in a window, allowing the module
    /// to attach dock panels, status bar widgets, toolbar items, or workspace actions.
    fn configure_workspace(
        &self,
        _workspace: &mut Workspace,
        _window: &mut Window,
        _cx: &mut gpui_ui::Context<Workspace>,
    ) {
    }
}

/// Configuration & builder for launching a production GPUI application.
pub struct AppShellBuilder {
    app_name: SharedString,
    version: Version,
    data_dir: Option<PathBuf>,
    initial_paths: Vec<PathBuf>,
    load_all_themes: bool,
    enable_code_panels: bool,
    modules: Vec<Box<dyn AppModule>>,
    key_bindings: Vec<KeyBinding>,
    on_ready: Option<Box<dyn FnOnce(Arc<AppState>, &mut App) + 'static>>,
}

impl AppShellBuilder {
    /// Create a new application shell with the given display name.
    pub fn new(app_name: impl Into<SharedString>) -> Self {
        Self {
            app_name: app_name.into(),
            version: Version::new(0, 1, 0),
            data_dir: None,
            initial_paths: Vec::new(),
            load_all_themes: true,
            enable_code_panels: cfg!(feature = "panels"),
            modules: Vec::new(),
            key_bindings: Vec::new(),
            on_ready: None,
        }
    }

    /// Set the semantic version reported by the application.
    pub fn version(mut self, version: Version) -> Self {
        self.version = version;
        self
    }

    /// Isolate all application config, SQLite state, and logs inside a custom directory
    /// so custom GPUI apps never collide with Zed's user directory.
    pub fn data_dir(mut self, dir: impl AsRef<Path>) -> Self {
        self.data_dir = Some(dir.as_ref().to_path_buf());
        self
    }

    /// Open one or more local directories or files when the window launches.
    pub fn open_paths<I, P>(mut self, paths: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        self.initial_paths.extend(paths.into_iter().map(Into::into));
        self
    }

    /// Toggle whether default IDE panels (project tree, outline, file finder, terminal, git)
    /// are automatically wired into workspaces.
    pub fn with_code_panels(mut self, enabled: bool) -> Self {
        self.enable_code_panels = enabled;
        self
    }

    /// Register a modular plugin/extension into the application lifecycle.
    pub fn with_module(mut self, module: impl AppModule) -> Self {
        self.modules.push(Box::new(module));
        self
    }

    /// Add custom global keybindings on startup.
    pub fn with_key_bindings(mut self, bindings: impl IntoIterator<Item = KeyBinding>) -> Self {
        self.key_bindings.extend(bindings);
        self
    }

    /// Run a custom callback once the shell and `AppState` are initialized,
    /// instead of (or in addition to) opening the default workspace window.
    pub fn on_ready(
        mut self,
        callback: impl FnOnce(Arc<AppState>, &mut App) + 'static,
    ) -> Self {
        self.on_ready = Some(Box::new(callback));
        self
    }

    /// Build and run the native GPUI event loop.
    pub fn run(self) {
        zlog::init();
        zlog::init_output_stderr();

        if let Some(custom_dir) = &self.data_dir
            && let Some(dir_str) = custom_dir.to_str()
        {
            paths::set_custom_data_dir(dir_str);
        }

        let app = gpui_ui::application();
        self.run_with_application(app);
    }

    /// Run using a caller-supplied [`Application`] (for example headless or custom platform).
    pub fn run_with_application(self, app: Application) {
        let Self {
            app_name,
            version,
            initial_paths,
            load_all_themes,
            enable_code_panels,
            modules,
            key_bindings,
            on_ready,
            ..
        } = self;

        app.run(move |cx| {
            let app_state = init_app_state(
                &app_name,
                version,
                load_all_themes,
                enable_code_panels,
                cx,
            );

            if !key_bindings.is_empty() {
                cx.bind_keys(key_bindings);
            }

            let modules: Arc<[Box<dyn AppModule>]> = Arc::from(modules);
            for module in modules.iter() {
                log::info!("Initializing module: {}", module.name());
                module.init(&app_state, cx);
            }

            {
                let modules = modules.clone();
                cx.observe_new(move |workspace: &mut Workspace, window, cx| {
                    if let Some(window) = window {
                        for module in modules.iter() {
                            module.configure_workspace(workspace, window, cx);
                        }
                    }
                })
                .detach();
            }

            if let Some(callback) = on_ready {
                callback(app_state, cx);
                return;
            }

            open_starter_window(app_name, app_state, initial_paths, cx);
        });
    }
}

/// Default window options respecting client/server decorations and theme background.
pub fn default_window_options(display_uuid: Option<Uuid>, cx: &mut App) -> WindowOptions {
    let display = display_uuid.and_then(|uuid| {
        cx.displays()
            .into_iter()
            .find(|display| display.uuid().ok() == Some(uuid))
    });
    let app_id = ReleaseChannel::try_global(cx)
        .map(|channel| channel.app_id())
        .unwrap_or("dev.gpui.App");
    let window_decorations = match WorkspaceSettings::get_global(cx).window_decorations {
        settings::WindowDecorations::Server => gpui_ui::gpui::WindowDecorations::Server,
        settings::WindowDecorations::Client => gpui_ui::gpui::WindowDecorations::Client,
    };

    WindowOptions {
        titlebar: Some(TitlebarOptions {
            title: None,
            appears_transparent: true,
            traffic_light_position: Some(point(px(9.0), px(9.0))),
        }),
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: Point {
                x: px(120.0),
                y: px(80.0),
            },
            size: size(px(1280.0), px(820.0)),
        })),
        focus: true,
        show: true,
        kind: WindowKind::Normal,
        is_movable: true,
        app_owns_titlebar_drag: true,
        display_id: display.map(|display| display.id()),
        window_background: cx.theme().window_background_appearance(),
        app_id: Some(app_id.to_owned()),
        window_decorations: Some(window_decorations),
        window_min_size: Some(size(px(360.0), px(240.0))),
        ..Default::default()
    }
}

/// Initialize the full local-first `AppState` and core workspace services inside `cx`.
pub fn init_app_state(
    app_name: &str,
    version: Version,
    load_all_themes: bool,
    enable_code_panels: bool,
    cx: &mut App,
) -> Arc<AppState> {
    zed_actions::init();
    release_channel::init(version.clone(), cx);
    gpui_tokio::init(cx);

    let app_db = AppDatabase::new();
    let kvp = KeyValueStore::from_app_db(&app_db);
    cx.set_global(app_db);

    settings::init(cx);
    zlog_settings::init(cx);
    zed_credentials_provider::init_global(cx);
    command_palette_hooks::init(cx);
    feature_flags::FeatureFlagStore::init(cx);

    if let Err(err) = Assets.load_fonts(cx) {
        log::warn!("Failed to load embedded fonts: {err:#}");
    }

    let themes_mode = if load_all_themes {
        gpui_ui::theme::LoadThemes::All(Box::new(Assets))
    } else {
        gpui_ui::theme::LoadThemes::JustBase
    };
    theme_settings::init(themes_mode, cx);
    let _theme_settings = ThemeSettings::get_global(cx);

    let fs: Arc<dyn Fs> = RealFs::new(None, cx.background_executor().clone());
    <dyn Fs>::set_global(fs.clone(), cx);

    let user_agent = format!(
        "{app_name}/{version} ({}; {})",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let http = {
        let _guard = Tokio::handle(cx).enter();
        ReqwestClient::user_agent(&user_agent).expect("failed to initialize Reqwest HTTP client")
    };
    cx.set_http_client(Arc::new(http));

    let client = Client::production(cx);
    Client::set_global(client.clone(), cx);

    let mut language_registry = LanguageRegistry::new(cx.background_executor().clone());
    language_registry.set_language_server_download_dir(paths::languages_dir().clone());
    let languages = Arc::new(language_registry);

    let (_node_tx, node_rx) = watch::channel(None);
    let node_runtime = NodeRuntime::new(client.http_client(), None, node_rx);

    languages::init(languages.clone(), fs.clone(), node_runtime.clone(), cx);
    gpui_ui::ui::observe_scrollbar_settings::<SettingsStore>(cx);

    let user_store = cx.new(|cx| UserStore::new(client.clone(), cx));
    let workspace_store = cx.new(|cx| WorkspaceStore::new(client.clone(), cx));
    Project::init(&client, cx);

    let session_id = Uuid::new_v4().to_string();
    let session = cx.foreground_executor().block_on(Session::new(session_id, kvp));
    let app_session = cx.new(|cx| AppSession::new(session, cx));

    let app_state = Arc::new(AppState {
        languages,
        client: client.clone(),
        user_store: user_store.clone(),
        workspace_store,
        fs,
        build_window_options: default_window_options,
        node_runtime,
        session: app_session,
    });
    AppState::set_global(app_state.clone(), cx);

    command_palette::init(cx);
    gpui_code::editor::init(cx);
    gpui_code::diagnostics::init(cx);
    gpui_code::search::init(cx);
    workspace::init(app_state.clone(), cx);
    ui_prompt::init(cx);
    title_bar::init(cx);
    recent_projects::init(cx);
    theme_selector::init(cx);
    keymap_editor::init(cx);
    which_key::init(cx);
    notifications::init(client, user_store, cx);

    cx.set_global(workspace::PaneSearchBarCallbacks {
        setup_search_bar: |languages, toolbar, window, cx| {
            let search_bar =
                cx.new(|cx| gpui_code::search::BufferSearchBar::new(languages, window, cx));
            toolbar.update(cx, |toolbar, cx| {
                toolbar.add_item(search_bar, window, cx);
            });
        },
        wrap_div_with_search_actions:
            gpui_code::search::buffer_search::register_pane_search_actions,
    });

    if enable_code_panels {
        register_default_code_panels(cx);
    }

    if let Some(key_bindings) = settings::KeymapFile::load_asset_allow_partial_failure(
        settings::DEFAULT_KEYMAP_PATH,
        cx,
    )
    .log_err()
    {
        cx.bind_keys(key_bindings);
    }

    app_state
}

fn register_default_code_panels(cx: &mut App) {
    gpui_code::terminal_view::init(cx);
    gpui_code::git_ui::init(cx);
    gpui_code::vim::init(cx);
    gpui_code::json_schema_store::init(cx);

    #[cfg(feature = "panels")]
    {
        project_panel::init(cx);
        outline_panel::init(cx);
        file_finder::init(cx);
        go_to_line::init(cx);
        tab_switcher::init(cx);
        tasks_ui::init(cx);

        cx.observe_new(|workspace: &mut Workspace, window, cx| {
            let Some(window) = window else {
                return;
            };

            let panels_task = cx.spawn_in(window, async move |workspace_handle, cx| {
                let project_panel =
                    project_panel::ProjectPanel::load(workspace_handle.clone(), cx.clone());
                let outline_panel =
                    outline_panel::OutlinePanel::load(workspace_handle.clone(), cx.clone());
                let terminal_panel =
                    gpui_code::terminal_view::terminal_panel::TerminalPanel::load(
                        workspace_handle.clone(),
                        cx.clone(),
                    );
                let git_panel = gpui_code::git_ui::git_panel::GitPanel::load(
                    workspace_handle.clone(),
                    cx.clone(),
                );

                async fn add_panel_when_ready<P: Panel>(
                    panel_task: impl std::future::Future<Output = Result<gpui_ui::Entity<P>>> + 'static,
                    workspace_handle: WeakEntity<Workspace>,
                    mut cx: AsyncWindowContext,
                ) {
                    if let Some(panel) = panel_task.await.context("failed to load panel").log_err()
                    {
                        workspace_handle
                            .update_in(&mut cx, |workspace, window, cx| {
                                workspace.add_panel(panel, window, cx);
                            })
                            .log_err();
                    }
                }

                futures::join!(
                    add_panel_when_ready(project_panel, workspace_handle.clone(), cx.clone()),
                    add_panel_when_ready(outline_panel, workspace_handle.clone(), cx.clone()),
                    add_panel_when_ready(terminal_panel, workspace_handle.clone(), cx.clone()),
                    add_panel_when_ready(git_panel, workspace_handle.clone(), cx.clone()),
                );

                workspace_handle.update(cx, |workspace, cx| {
                    workspace.finish_dock_restoration(cx);
                })?;

                anyhow::Ok(())
            });

            workspace.set_panels_task(panels_task);
        })
        .detach();
    }
}

fn open_starter_window(
    app_name: SharedString,
    app_state: Arc<AppState>,
    initial_paths: Vec<PathBuf>,
    cx: &mut App,
) {
    let mut options = (app_state.build_window_options)(None, cx);
    if let Some(titlebar) = options.titlebar.as_mut() {
        titlebar.title = Some(app_name);
    }

    cx.spawn(async move |cx| {
        if initial_paths.is_empty() {
            let project = cx.update(|cx| {
                Project::local(
                    app_state.client.clone(),
                    app_state.node_runtime.clone(),
                    app_state.user_store.clone(),
                    app_state.languages.clone(),
                    app_state.fs.clone(),
                    None,
                    project::LocalProjectFlags::default(),
                    cx,
                )
            });

            cx.open_window(options, |window, cx| {
                let workspace = cx.new(|cx| {
                    Workspace::new(None, project.clone(), app_state.clone(), window, cx)
                });
                cx.new(|cx| MultiWorkspace::new(workspace, window, cx))
            })?;
        } else {
            cx.update(|cx| {
                workspace::open_paths(
                    &initial_paths,
                    app_state,
                    workspace::OpenOptions::default(),
                    cx,
                )
            })
            .await?;
        }
        anyhow::Ok(())
    })
    .detach_and_log_err(cx);
}

/// Helper to create a standalone code [`Editor`] view for a string with optional syntax highlighting.
pub fn create_code_editor(
    initial_text: impl Into<String>,
    language_name: Option<&str>,
    languages: &Arc<LanguageRegistry>,
    window: &mut Window,
    cx: &mut App,
) -> gpui_ui::Entity<Editor> {
    let text = initial_text.into();
    let buffer = cx.new(|cx| gpui_code::Buffer::local(text, cx));
    if let Some(language_name) = language_name {
        let languages = languages.clone();
        let language_name = language_name.to_owned();
        let buffer = buffer.clone();
        cx.spawn(async move |cx| {
            if let Some(language) = languages.language_for_name(&language_name).await.log_err() {
                buffer.update(cx, |buffer, cx| buffer.set_language(Some(language), cx));
            }
        })
        .detach();
    }
    cx.new(|cx| Editor::for_buffer(buffer, None, window, cx))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDomainModule;

    impl AppModule for TestDomainModule {
        fn name(&self) -> &'static str {
            "test-domain-module"
        }
    }

    #[test]
    fn test_app_shell_builder_configuration() {
        let builder = AppShellBuilder::new("GPUI Test Studio")
            .version(Version::new(1, 2, 3))
            .with_code_panels(false)
            .with_module(TestDomainModule);

        assert_eq!(builder.app_name.as_ref(), "GPUI Test Studio");
        assert_eq!(builder.version, Version::new(1, 2, 3));
        assert!(!builder.enable_code_panels);
        assert_eq!(builder.modules.len(), 1);
        assert_eq!(builder.modules[0].name(), "test-domain-module");
    }
}
