use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use web_time::Instant;

use gpui::{
    AnyElement, App, Context, ElementId, Entity, FocusHandle, Focusable, KeyDownEvent, Render,
    Role, SharedString, Subscription, Window, actions, div, prelude::*, px,
};
use aphrody_craft::CraftCatalog;
use aphrody_session::{
    AppEntry, CompositorState, HardwareCapabilities, LayerAnchor, LayerKind, LayerSurface,
    PlacedWindow, Rect, SessionManifest, SessionSupervisor, TilingMode, ToplevelId, UnitPhase,
    WorkspaceId, discover_system_apps,
};
use aphrody_ui::{
    AdaptiveViewport, AppearanceMode, AppletSlot, OsConfigStore, OsTheme, OsThemeCatalog,
    PanelApplet, ReactiveHost, StatusTone, focusable_action_chip, status_badge,
};
use ui::{
    Icon, IconName, IconSize, Label, LabelCommon as _, LabelSize, h_flex, rems_from_px,
    v_flex,
};

use crate::DesktopOptions;
use crate::applets::{BrandLauncherApplet, SystemTelemetryApplet, WorkspaceApplet, WorkspacePill};
use crate::apps::cosmic_settings::{
    CosmicSettingsCallbacks, SettingsPage, render_cosmic_settings_window,
};
use crate::apps::edit::{CosmicEditState, render_cosmic_edit_window};
use crate::apps::installer::render_installer_window;
use crate::apps::store::render_cosmic_store_window;
use crate::apps::term::{CosmicTermState, render_cosmic_term_window};
use crate::comp_viewport::render_workspaces_overview;
use crate::launcher::{LauncherFolder, render_pop_launcher_and_app_library};
use crate::overlays::{
    render_context_drawer_pane, render_cosmic_greeter_lock, render_cosmic_notifications_stack,
    render_cosmic_osd_pill, render_cosmic_screenshot_toolbar, render_launcher_modal,
};
use crate::studio_view::{StudioCallbacks, StudioState, render_studio_window};
use crate::windows::{
    DesktopPrefs, FilesState, SettingsFlags, SettingsMessage, SettingsProgram, WindowKind,
    render_files_window, render_monitor_window,
};

const TOP_PANEL_HEIGHT: f32 = 44.0;
const BOTTOM_DOCK_HEIGHT: f32 = 64.0;
const DRAWER_WIDTH: f32 = 320.0;

actions!(
    aphrody_desktop,
    [
        ToggleLauncher,
        CycleLayout,
        ToggleDarkMode,
        ToggleInspectorDrawer,
        CloseOverlays,
        FocusNext,
        FocusPrev,
        ToggleOverview,
        ToggleScreenshot,
        ToggleLockScreen,
        StackFocusedWindow,
        CycleStackTab,
        ToggleFloatingWindow
    ]
);

/// Top-level GPUI COSMIC Epoch desktop shell combining TopPanel applets, Compositor tiling,
/// floating DockBar, searchable unified Launcher, and modular reactive/immediate windows.
pub struct OsDesktopShell {
    focus_handle: FocusHandle,
    pub(crate) theme_catalog: OsThemeCatalog,
    pub(crate) theme: OsTheme,
    prefs_store: OsConfigStore<DesktopPrefs>,
    pub(crate) craft_catalog: CraftCatalog,
    supervisor: SessionSupervisor,
    pub(crate) compositor: CompositorState,
    pub pilot: aphrody_pilot::PilotEngine,
    pub(crate) discovered_apps: Vec<AppEntry>,
    pub(crate) window_kinds: BTreeMap<ToplevelId, WindowKind>,
    pub(crate) studio: StudioState,
    files: FilesState,
    files_scan_seq: u64,
    settings_host: Entity<ReactiveHost<SettingsProgram>>,
    _settings_obs: Subscription,
    pub settings_daemon: aphrody_session::CosmicSettingsDaemon,
    pub notifications: aphrody_session::NotificationCenter,
    pub osd: Option<aphrody_session::OsdBanner>,
    pub overview_open: bool,
    pub app_library_open: bool,
    pub screenshot_open: bool,
    pub lock_screen_open: bool,
    pub settings_page: SettingsPage,
    pub term_state: CosmicTermState,
    pub edit_state: CosmicEditState,
    pub store_catalog: aphrody_store::AwesomeGpuiCatalog,
    pub store_category: Option<aphrody_store::StoreCategory>,
    pub installer_catalog: aphrody_installer::InstallerProfileCatalog,
    pub selected_de_id: String,
    pub launcher_folder: LauncherFolder,
    pub launcher_open: bool,
    pub launcher_query: String,
    pub drawer_open: bool,
    hardware: HardwareCapabilities,
    last_render_at: Instant,
    frame_samples: Vec<f32>,
}

impl OsDesktopShell {
    pub fn new(options: DesktopOptions, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut theme_catalog = OsThemeCatalog::bundled().unwrap_or_else(|err| {
            log::warn!("failed to load bundled themes JSON: {err:#}");
            OsThemeCatalog::default()
        });
        theme_catalog.load_user_overrides();

        let mut prefs_store = OsConfigStore::<DesktopPrefs>::load_or_default("desktop", "prefs");
        if let Some(theme_id) = &options.initial_theme {
            prefs_store.value.theme_id = theme_id.clone();
        }
        if let Some(mode) = options.initial_mode {
            prefs_store.value.mode = mode;
        }

        let theme = theme_catalog
            .resolve(&prefs_store.value.theme_id, Some(prefs_store.value.mode))
            .or_else(|| theme_catalog.resolve(brand::APP_SLUG, Some(prefs_store.value.mode)))
            .or_else(|| {
                let ids = theme_catalog.ids();
                ids.first().and_then(|id| theme_catalog.resolve(id, None))
            })
            .unwrap_or_else(|| {
                log::warn!("OsThemeCatalog is empty, using default theme");
                OsTheme::default()
            });
        theme.apply_to_gpui(cx);

        let craft_catalog = CraftCatalog::load_with_user_override().unwrap_or_else(|err| {
            log::warn!("failed to load CraftCatalog: {err:#}");
            CraftCatalog {
                schema_version: 1,
                workspaces: aphrody_craft::WorkspaceDefaults {
                    default_names: vec!["Studio".to_owned(), "Code".to_owned()],
                    default_layout: "dwindle".to_owned(),
                },
                apps: Vec::new(),
            }
        });

        let manifest = SessionManifest::load_with_user_override().unwrap_or_else(|err| {
            log::warn!("failed to load SessionManifest: {err:#}");
            SessionManifest {
                schema_version: 1,
                ..SessionManifest::default()
            }
        });
        let mut supervisor = SessionSupervisor::new(manifest).unwrap_or_else(|err| {
            log::warn!("failed to plan session topology: {err:#}");
            SessionSupervisor::default()
        });
        supervisor.activate_session();

        let ws_names: Vec<&str> = craft_catalog
            .workspaces
            .default_names
            .iter()
            .map(String::as_str)
            .collect();
        let mut compositor = CompositorState::new(
            Rect::new(0.0, 0.0, options.width, options.height),
            &ws_names,
            prefs_store.value.layout_mode,
        );
        compositor.gap = prefs_store.value.window_gap;
        compositor.layers.set_surface(LayerSurface {
            id: 1,
            name: "top-panel".to_owned(),
            kind: LayerKind::Top,
            anchor: LayerAnchor::TOP | LayerAnchor::LEFT | LayerAnchor::RIGHT,
            exclusive_zone: TOP_PANEL_HEIGHT,
        });
        compositor.layers.set_surface(LayerSurface {
            id: 2,
            name: "bottom-dock".to_owned(),
            kind: LayerKind::Top,
            anchor: LayerAnchor::BOTTOM | LayerAnchor::LEFT | LayerAnchor::RIGHT,
            exclusive_zone: BOTTOM_DOCK_HEIGHT,
        });

        let mut window_kinds = BTreeMap::new();
        let studio_win = compositor.open_window("ArtCraft Studio", WindowKind::Studio.app_id());
        window_kinds.insert(studio_win, WindowKind::Studio);
        compositor.focus_window(studio_win);

        if compositor.switch_workspace(2) {
            let files_win = compositor.open_window("COSMIC Files", WindowKind::Files.app_id());
            window_kinds.insert(files_win, WindowKind::Files);
        }
        if compositor.switch_workspace(4) {
            let settings_win =
                compositor.open_window("COSMIC Settings", WindowKind::Settings.app_id());
            window_kinds.insert(settings_win, WindowKind::Settings);
            let mon_win = compositor.open_window("System Monitor", WindowKind::Monitor.app_id());
            window_kinds.insert(mon_win, WindowKind::Monitor);
        }
        compositor.switch_workspace(1);

        let mut studio = StudioState::new(&craft_catalog, options.initial_app.as_deref());
        if let Some(inspect_target) = &options.inspect_path {
            studio.inspect_local_file(inspect_target, &craft_catalog);
        }

        let files = FilesState::scan_initial(&craft_catalog);
        let discovered_apps = discover_system_apps(28);

        let settings_flags = SettingsFlags {
            prefs: prefs_store.value.clone(),
            theme_ids: theme_catalog.ids(),
        };
        let settings_host = cx.new(|cx| ReactiveHost::<SettingsProgram>::new(settings_flags, cx));
        let settings_obs = cx.observe(&settings_host, |shell, host, cx| {
            let new_prefs = host.read(cx).state.prefs.clone();
            shell.apply_preferences(new_prefs, cx);
        });

        let hardware = HardwareCapabilities::detect();
        let baseline_ms = (hardware.frame_budget_ms * 0.62).max(2.0);
        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle, cx);

        let pilot = aphrody_pilot::PilotEngine::new();
        pilot.gpui.sync_compositor(
            compositor.clone(),
            &theme.id,
            theme.mode == AppearanceMode::Dark,
        );

        let settings_daemon = aphrody_session::CosmicSettingsDaemon::default();
        let mut notifications = aphrody_session::NotificationCenter::default();
        notifications.push(
            "aphrody-session",
            "COSMIC Epoch + Pop!_OS Shell Active",
            "Stacking, Overview, Launcher, OSD, RandR & DE Switcher ready.",
            aphrody_session::NotificationUrgency::Normal,
        );
        let store_catalog = aphrody_store::AwesomeGpuiCatalog::bundled().unwrap_or_else(|_| {
            aphrody_store::AwesomeGpuiCatalog {
                schema_version: 1,
                source_repo: "aphrody-dev/aphrody".to_owned(),
                source_url: "https://aphrody.com".to_owned(),
                synced_at: String::new(),
                upstream_stars: 0,
                total_entries: 0,
                entries: Vec::new(),
            }
        });
        let installer_catalog =
            aphrody_installer::InstallerProfileCatalog::bundled().unwrap_or_default();

        Self {
            focus_handle,
            theme_catalog,
            theme,
            prefs_store,
            craft_catalog,
            supervisor,
            compositor,
            pilot,
            discovered_apps,
            window_kinds,
            studio,
            files,
            files_scan_seq: 0,
            settings_host,
            _settings_obs: settings_obs,
            settings_daemon,
            notifications,
            osd: None,
            overview_open: false,
            app_library_open: false,
            screenshot_open: false,
            lock_screen_open: false,
            settings_page: SettingsPage::default(),
            term_state: CosmicTermState::default(),
            edit_state: CosmicEditState::default(),
            store_catalog,
            store_category: None,
            installer_catalog,
            selected_de_id: "aphrody-gpui".to_owned(),
            launcher_folder: LauncherFolder::default(),
            launcher_open: false,
            launcher_query: String::new(),
            drawer_open: false,
            hardware,
            last_render_at: Instant::now(),
            frame_samples: vec![baseline_ms; 8],
        }
    }

    pub fn apply_preferences(&mut self, prefs: DesktopPrefs, cx: &mut Context<Self>) {
        self.prefs_store.value = prefs.clone();
        self.compositor.gap = prefs.window_gap;
        self.compositor.set_active_mode(prefs.layout_mode);
        if let Some(resolved) = self
            .theme_catalog
            .resolve(&prefs.theme_id, Some(prefs.mode))
        {
            self.theme = resolved;
            self.theme.apply_to_gpui(cx);
        }
        self.pilot.gpui.sync_compositor(
            self.compositor.clone(),
            &self.theme.id,
            self.theme.mode == AppearanceMode::Dark,
        );
        cx.notify();
    }

    /// Executes an AI OS Pilot tool step against the live GPUI `OsDesktopShell`, syncing the
    /// COSMIC compositor, AccessKit tree, and AI Pilot HUD activity trail in one transaction.
    pub fn pilot_execute_step(
        &mut self,
        tool: &str,
        args: serde_json::Value,
        target: aphrody_pilot::TargetBackend,
        cx: &mut Context<Self>,
    ) -> Result<serde_json::Value, aphrody_pilot::PilotError> {
        let prev_ws = self.compositor.active_workspace;
        let was_dark = self.theme.mode == AppearanceMode::Dark;
        self.pilot
            .gpui
            .sync_compositor(self.compositor.clone(), &self.theme.id, was_dark);
        let ctx = aphrody_pilot::CommandContext {
            target,
            ..Default::default()
        };
        let out = self.pilot.execute_tool(tool, &args, &ctx)?;
        self.compositor = self.pilot.gpui.export_compositor();
        if self.compositor.active_workspace != prev_ws {
            self.sync_chrome_layers(self.compositor.screen.w, self.compositor.screen.h);
        }
        cx.notify();
        Ok(out)
    }

    pub fn open_or_focus_window(&mut self, kind: WindowKind, title: &str, cx: &mut Context<Self>) {
        let active_ws = self.compositor.active_workspace;
        let existing_in_ws = self.compositor.workspaces.get(&active_ws).and_then(|ws| {
            ws.window_ids
                .iter()
                .find(|id| self.window_kinds.get(id) == Some(&kind))
                .copied()
        });

        if let Some(id) = existing_in_ws {
            self.compositor.focus_window(id);
        } else {
            let id = self.compositor.open_window(title, kind.app_id());
            self.window_kinds.insert(id, kind);
        }
        self.launcher_open = false;
        cx.notify();
    }

    pub fn select_craft_app(&mut self, app_id: &str, cx: &mut Context<Self>) {
        self.studio.select_app(app_id, &self.craft_catalog);
        self.open_or_focus_window(WindowKind::Studio, "ArtCraft Studio", cx);
    }

    pub fn toggle_mode(&mut self, cx: &mut Context<Self>) {
        let next_mode = match self.prefs_store.value.mode {
            AppearanceMode::Dark => AppearanceMode::Light,
            AppearanceMode::Light => AppearanceMode::Dark,
        };
        self.settings_host.update(cx, |host, cx| {
            host.dispatch(SettingsMessage::SetMode(next_mode), cx);
        });
    }

    pub fn select_theme(&mut self, theme_id: &str, cx: &mut Context<Self>) {
        let tid = theme_id.to_owned();
        self.settings_host.update(cx, |host, cx| {
            host.dispatch(SettingsMessage::SelectTheme(tid), cx);
        });
    }

    pub fn set_layout(&mut self, mode: TilingMode, cx: &mut Context<Self>) {
        self.compositor.set_active_mode(mode);
        self.prefs_store.value.layout_mode = mode;
        cx.notify();
    }

    pub fn cycle_layout(&mut self, cx: &mut Context<Self>) {
        let mode = self.compositor.cycle_active_mode();
        self.prefs_store.value.layout_mode = mode;
        cx.notify();
    }

    pub fn toggle_launcher(&mut self, cx: &mut Context<Self>) {
        self.launcher_open = !self.launcher_open;
        if !self.launcher_open {
            self.launcher_query.clear();
        }
        cx.notify();
    }

    pub fn toggle_drawer(&mut self, cx: &mut Context<Self>) {
        self.drawer_open = !self.drawer_open;
        let screen = self.compositor.screen;
        self.sync_chrome_layers(screen.w, screen.h);
        cx.notify();
    }

    pub fn toggle_overview(&mut self, cx: &mut Context<Self>) {
        self.overview_open = !self.overview_open;
        self.launcher_open = false;
        self.app_library_open = false;
        cx.notify();
    }

    pub fn toggle_app_library(&mut self, cx: &mut Context<Self>) {
        self.app_library_open = !self.app_library_open;
        self.launcher_open = false;
        self.overview_open = false;
        cx.notify();
    }

    pub fn toggle_screenshot(&mut self, cx: &mut Context<Self>) {
        self.screenshot_open = !self.screenshot_open;
        cx.notify();
    }

    pub fn toggle_lock_screen(&mut self, cx: &mut Context<Self>) {
        self.lock_screen_open = !self.lock_screen_open;
        self.launcher_open = false;
        self.overview_open = false;
        cx.notify();
    }

    pub fn stack_focused_window(&mut self, cx: &mut Context<Self>) {
        if let Some(sid) = self.compositor.stack_with_focused() {
            self.trigger_osd(
                aphrody_session::OsdKind::TilingLayout,
                "Pop Shell Stack",
                &format!("Stacked windows into group #{sid}"),
                100,
                cx,
            );
        }
        cx.notify();
    }

    pub fn cycle_active_stack(&mut self, delta: i32, cx: &mut Context<Self>) {
        if let Some(focused) = self.compositor.focused_toplevel
            && let Some(sid) = self.compositor.active_stack_for(focused).map(|s| s.id)
            && self.compositor.cycle_stack_tab(sid, delta).is_none()
        {
            log::debug!("stack {sid} had no active tab to cycle");
        }
        cx.notify();
    }

    pub fn toggle_floating_focused(&mut self, cx: &mut Context<Self>) {
        if let Some(focused) = self.compositor.focused_toplevel
            && self.compositor.toggle_floating(focused)
        {
            self.trigger_osd(
                aphrody_session::OsdKind::TilingLayout,
                "Floating Mode Toggled",
                "Switched active window tile/float state",
                100,
                cx,
            );
        }
        cx.notify();
    }

    pub fn trigger_osd(
        &mut self,
        kind: aphrody_session::OsdKind,
        title: &str,
        subtitle: &str,
        pct: u8,
        cx: &mut Context<Self>,
    ) {
        self.osd = Some(aphrody_session::OsdBanner {
            kind,
            title: title.to_owned(),
            subtitle: subtitle.to_owned(),
            level_percent: Some(pct),
        });
        cx.notify();
    }

    fn panel_heights(viewport_h: f32) -> (f32, f32) {
        let vp = AdaptiveViewport::from_size(1440.0, viewport_h);
        (vp.top_panel_height, vp.dock_height)
    }

    fn sync_chrome_layers(&mut self, viewport_w: f32, viewport_h: f32) -> (f32, f32) {
        let (top_h, dock_h) = Self::panel_heights(viewport_h);
        self.compositor.layers.set_surface(LayerSurface {
            id: 1,
            name: "top-panel".to_owned(),
            kind: LayerKind::Top,
            anchor: LayerAnchor::TOP | LayerAnchor::LEFT | LayerAnchor::RIGHT,
            exclusive_zone: top_h,
        });
        self.compositor.layers.set_surface(LayerSurface {
            id: 2,
            name: "bottom-dock".to_owned(),
            kind: LayerKind::Top,
            anchor: LayerAnchor::BOTTOM | LayerAnchor::LEFT | LayerAnchor::RIGHT,
            exclusive_zone: dock_h,
        });
        if self.drawer_open {
            let drawer_w = (viewport_w * 0.36).clamp(220.0, DRAWER_WIDTH);
            self.compositor.layers.set_surface(LayerSurface {
                id: 3,
                name: "context-drawer".to_owned(),
                kind: LayerKind::Top,
                anchor: LayerAnchor::RIGHT | LayerAnchor::TOP | LayerAnchor::BOTTOM,
                exclusive_zone: drawer_w,
            });
        } else {
            self.compositor.layers.remove_surface(3);
        }
        (top_h, dock_h)
    }

    fn record_frame_sample(&mut self) {
        let elapsed_ms = self.last_render_at.elapsed().as_secs_f32() * 1_000.0;
        self.last_render_at = Instant::now();
        if (0.2..=40.0).contains(&elapsed_ms) {
            self.frame_samples.push(elapsed_ms);
            if self.frame_samples.len() > 32 {
                let overflow = self.frame_samples.len() - 32;
                self.frame_samples.drain(..overflow);
            }
        }
    }

    fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.launcher_open {
            return;
        }
        if let Some(ch) = &event.keystroke.key_char
            && !event.keystroke.modifiers.control
            && !event.keystroke.modifiers.platform
        {
            self.launcher_query.push_str(ch);
            cx.notify();
            return;
        }
        if event.keystroke.key == "backspace" {
            self.launcher_query.pop();
            cx.notify();
        }
    }

    fn render_top_panel(
        &self,
        top_h: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let entity = cx.entity();
        let active_ws = self.compositor.active_workspace;
        let ws_mode = self
            .compositor
            .workspaces
            .get(&active_ws)
            .map_or(TilingMode::Dwindle, |ws| ws.mode);

        let workspaces: Vec<WorkspacePill> = self
            .compositor
            .workspaces
            .values()
            .map(|ws| WorkspacePill {
                id: ws.id,
                name: SharedString::from(ws.name.clone()),
                window_count: ws.window_ids.len(),
            })
            .collect();

        let ws_entity = entity.clone();
        let cycle_entity = entity.clone();
        let ws_applet = WorkspaceApplet {
            workspaces,
            active_workspace: active_ws,
            mode: ws_mode,
            on_select_workspace: Arc::new(move |id: WorkspaceId, _w, cx| {
                ws_entity.update(cx, |shell, cx| {
                    shell.compositor.switch_workspace(id);
                    cx.notify();
                });
            }),
            on_cycle_mode: Arc::new(move |_w, cx| {
                cycle_entity.update(cx, |shell, cx| shell.cycle_layout(cx));
            }),
        };

        let active_app_name = self
            .craft_catalog
            .by_id(&self.studio.active_app_id)
            .map_or("ArtCraft", |app| app.name.as_str());
        let launch_entity = entity.clone();
        let brand_applet = BrandLauncherApplet {
            brand_name: SharedString::from(brand::APP_NAME),
            active_app: SharedString::from(active_app_name.to_owned()),
            theme_name: SharedString::from(self.theme.name.clone()),
            launcher_open: self.launcher_open,
            on_toggle_launcher: Arc::new(move |_w, cx| {
                launch_entity.update(cx, |shell, cx| shell.toggle_launcher(cx));
            }),
        };

        let snapshot = self.supervisor.snapshot();
        let unit_running = snapshot
            .iter()
            .filter(|u| u.phase == UnitPhase::Running)
            .count();
        let latest_frame = *self.frame_samples.last().unwrap_or(&5.6);
        let mode_entity = entity.clone();
        let drawer_entity = entity;
        let telemetry_applet = SystemTelemetryApplet {
            mode: self.theme.mode,
            unit_running,
            unit_total: snapshot.len(),
            target_fps: self.hardware.target_fps,
            latest_frame_ms: latest_frame,
            drawer_open: self.drawer_open,
            on_toggle_mode: Arc::new(move |_w, cx| {
                mode_entity.update(cx, |shell, cx| shell.toggle_mode(cx));
            }),
            on_toggle_drawer: Arc::new(move |_w, cx| {
                drawer_entity.update(cx, |shell, cx| shell.toggle_drawer(cx));
            }),
        };

        let applets: Vec<&dyn PanelApplet> = vec![&ws_applet, &brand_applet, &telemetry_applet];
        let mut left_pills = Vec::new();
        let mut center_pills = Vec::new();
        let mut right_pills = Vec::new();
        for applet in applets {
            let pill = applet.render_pill(&self.theme, window, cx);
            match applet.slot() {
                AppletSlot::Left => left_pills.push(pill),
                AppletSlot::Center => center_pills.push(pill),
                AppletSlot::Right => right_pills.push(pill),
            }
        }
        let compact_pilot = f32::from(window.viewport_size().width) < 1400.0;
        right_pills.insert(
            0,
            crate::pilot_hud::render_pilot_status_pill(
                self.pilot.active_adapter().backend_name(),
                self.compositor.arrange_active_workspace().len(),
                &self.pilot.recent_hud_activity,
                compact_pilot,
                &self.theme,
            ),
        );

        h_flex()
            .id("os-top-panel")
            .role(Role::Toolbar)
            .aria_label("Desktop Top Panel")
            .w_full()
            .min_w_0()
            .h(rems_from_px(top_h))
            .px_3()
            .gap_2()
            .justify_between()
            .items_center()
            .bg(self.theme.colors.surface_container_low)
            .border_b_1()
            .border_color(self.theme.colors.outline_variant)
            .child(
                h_flex()
                    .flex_shrink_0()
                    .min_w_0()
                    .overflow_hidden()
                    .gap_1p5()
                    .items_center()
                    .children(left_pills),
            )
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .justify_center()
                    .gap_2()
                    .items_center()
                    .children(center_pills),
            )
            .child(
                h_flex()
                    .flex_shrink_0()
                    .min_w_0()
                    .overflow_hidden()
                    .gap_2()
                    .items_center()
                    .children(right_pills),
            )
            .into_any_element()
    }

    fn render_placed_window(
        &self,
        placed: &PlacedWindow,
        top_h: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let kind = self
            .window_kinds
            .get(&placed.id)
            .copied()
            .unwrap_or(WindowKind::Studio);
        let title = self
            .compositor
            .toplevels
            .get(&placed.id)
            .map_or_else(|| "Window".to_owned(), |w| w.title.clone());
        let is_maximized = self
            .compositor
            .toplevels
            .get(&placed.id)
            .is_some_and(|w| w.maximized);

        let focus_entity = cx.entity();
        let max_entity = cx.entity();
        let close_entity = cx.entity();
        let win_id = placed.id;

        let border_color = if self.compositor.active_hint.enabled && placed.focused {
            self.theme.colors.primary
        } else {
            self.theme.colors.outline_variant
        };
        let active_stack = self.compositor.active_stack_for(win_id).cloned();
        let stack_strip = if self.compositor.active_hint.show_stack_tabs
            && let Some(stack) = active_stack
        {
            let stack_id = stack.id;
            let mut row = h_flex()
                .w_full()
                .px_2p5()
                .py_1()
                .gap_1()
                .bg(self.theme.colors.surface_container_low)
                .border_b_1()
                .border_color(self.theme.colors.outline_variant);
            for (tab_idx, &tid) in stack.tabs.iter().enumerate() {
                let tab_title = self
                    .compositor
                    .toplevels
                    .get(&tid)
                    .map_or_else(|| format!("Window {tid}"), |w| w.title.clone());
                let is_tab_active = stack.active_toplevel() == Some(tid);
                let sel_ent = cx.entity();
                row = row.child(
                    div()
                        .id(ElementId::Name(format!("pop-tab-{stack_id}-{tab_idx}").into()))
                        .cursor_pointer()
                        .px_2()
                        .py_0p5()
                        .rounded(px(6.0))
                        .bg(if is_tab_active {
                            self.theme.colors.primary
                        } else {
                            self.theme.colors.surface_container_high
                        })
                        .on_click(move |_, _w, cx| {
                            sel_ent.update(cx, |s, cx| {
                                if s.compositor.select_stack_tab(stack_id, tid) {
                                    cx.notify();
                                }
                            });
                        })
                        .child(
                            Label::new(tab_title)
                                .size(LabelSize::XSmall)
                                .color(if is_tab_active {
                                    ui::Color::Custom(self.theme.colors.on_primary)
                                } else {
                                    ui::Color::Default
                                }),
                        ),
                );
            }
            let unstack_ent = cx.entity();
            row = row.child(
                div()
                    .id(ElementId::Name(format!("pop-unstack-{win_id}").into()))
                    .cursor_pointer()
                    .px_1p5()
                    .py_0p5()
                    .rounded(px(6.0))
                    .bg(self.theme.colors.surface_container)
                    .on_click(move |_, _w, cx| {
                        unstack_ent.update(cx, |s, cx| {
                            if s.compositor.unstack_window(win_id) {
                                cx.notify();
                            }
                        });
                    })
                    .child(Label::new("Unstack").size(LabelSize::XSmall)),
            );
            Some(row)
        } else {
            None
        };
        let header_bg = if placed.focused {
            self.theme.colors.surface_container_high
        } else {
            self.theme.colors.surface_container
        };

        let body = match kind {
            WindowKind::Studio => self.build_studio_body(placed.rect.w, cx),
            WindowKind::Files => self.build_files_body(cx),
            WindowKind::Settings => {
                let entity_page = cx.entity();
                let entity_theme = cx.entity();
                let entity_mode = cx.entity();
                let entity_layout = cx.entity();
                let entity_hint = cx.entity();
                let entity_power = cx.entity();
                let entity_bright = cx.entity();
                let entity_switch = cx.entity();
                render_cosmic_settings_window(
                    self.settings_page,
                    &self.theme,
                    &self.theme_catalog,
                    &self.theme.id,
                    self.compositor
                        .workspaces
                        .get(&self.compositor.active_workspace)
                        .map(|w| w.mode)
                        .unwrap_or_default(),
                    self.compositor.active_hint.enabled,
                    self.compositor.active_hint.width_px,
                    &self.settings_daemon,
                    CosmicSettingsCallbacks {
                        on_select_page: Box::new(move |page, cx| {
                            entity_page.update(cx, |s, cx| {
                                s.settings_page = page;
                                cx.notify();
                            });
                        }),
                        on_select_theme: Box::new(move |id, cx| {
                            entity_theme.update(cx, |s, cx| s.select_theme(id, cx));
                        }),
                        on_toggle_mode: Box::new(move |cx| {
                            entity_mode.update(cx, |s, cx| s.toggle_mode(cx));
                        }),
                        on_set_layout: Box::new(move |mode, cx| {
                            entity_layout.update(cx, |s, cx| s.set_layout(mode, cx));
                        }),
                        on_toggle_active_hint: Box::new(move |cx| {
                            entity_hint.update(cx, |s, cx| {
                                let next = !s.compositor.active_hint.enabled;
                                s.compositor.set_active_hint(next, 2.0, true);
                                s.settings_daemon.active_hint = next;
                                cx.notify();
                            });
                        }),
                        on_set_power_profile: Box::new(move |prof, cx| {
                            entity_power.update(cx, |s, cx| {
                                s.settings_daemon.power_profile = prof;
                                s.trigger_osd(
                                    aphrody_session::OsdKind::PowerProfile,
                                    "Power Profile",
                                    prof.label(),
                                    100,
                                    cx,
                                );
                            });
                        }),
                        on_adjust_brightness: Box::new(move |delta, cx| {
                            entity_bright.update(cx, |s, cx| {
                                let next =
                                    (s.settings_daemon.brightness_percent as i16 + i16::from(delta))
                                        .clamp(5, 100) as u8;
                                s.settings_daemon.brightness_percent = next;
                                s.trigger_osd(
                                    aphrody_session::OsdKind::Brightness,
                                    "Display Backlight",
                                    &format!("{next}%"),
                                    next,
                                    cx,
                                );
                            });
                        }),
                        on_open_de_switcher: Box::new(move |cx| {
                            entity_switch.update(cx, |s, cx| {
                                s.open_or_focus_window(
                                    WindowKind::Installer,
                                    "OS Installer & DE Switcher",
                                    cx,
                                );
                            });
                        }),
                    },
                )
            }
            WindowKind::Monitor => self.build_monitor_body(cx),
            WindowKind::Term => {
                let entity_term = cx.entity();
                render_cosmic_term_window(
                    &self.theme,
                    &self.term_state,
                    Box::new(move |cmd, cx| {
                        entity_term.update(cx, |s, cx| {
                            s.term_state.push_command(cmd);
                            cx.notify();
                        });
                    }),
                )
            }
            WindowKind::Edit => {
                let entity_edit = cx.entity();
                render_cosmic_edit_window(
                    &self.theme,
                    &self.edit_state,
                    Box::new(move |lang, cx| {
                        entity_edit.update(cx, |s, cx| {
                            s.edit_state.load_sample(lang);
                            cx.notify();
                        });
                    }),
                )
            }
            WindowKind::Store => {
                let entity_store = cx.entity();
                render_cosmic_store_window(
                    &self.theme,
                    &self.store_catalog,
                    self.store_category,
                    Box::new(move |cat, cx| {
                        entity_store.update(cx, |s, cx| {
                            s.store_category = cat;
                            cx.notify();
                        });
                    }),
                )
            }
            WindowKind::Installer => {
                let entity_inst = cx.entity();
                render_installer_window(
                    &self.theme,
                    &self.installer_catalog,
                    &self.selected_de_id,
                    Box::new(move |de_id, cx| {
                        let target = de_id.to_owned();
                        entity_inst.update(cx, |s, cx| {
                            s.selected_de_id = target.clone();
                            s.settings_daemon.active_de_env = target.clone();
                            s.notifications.push(
                                "aphrody-installer",
                                "DE Target Selected",
                                format!("Prepared switch profile for {target}"),
                                aphrody_session::NotificationUrgency::Normal,
                            );
                            cx.notify();
                        });
                    }),
                )
            }
        };

        v_flex()
            .id(ElementId::Name(format!("win-{win_id}").into()))
            .role(Role::Region)
            .aria_label(SharedString::from(title.clone()))
            .absolute()
            .left(px(placed.rect.x))
            .top(px((placed.rect.y - top_h).max(0.0)))
            .w(px(placed.rect.w))
            .h(px(placed.rect.h))
            .rounded(px(self.theme.layout.window_radius))
            .overflow_hidden()
            .bg(self.theme.colors.surface)
            .border_2()
            .border_color(border_color)
            .on_mouse_down(gpui::MouseButton::Left, move |_, _w, cx| {
                focus_entity.update(cx, |shell, cx| {
                    shell.compositor.focus_window(win_id);
                    cx.notify();
                });
            })
            .child(
                h_flex()
                    .w_full()
                    .min_w_0()
                    .h(rems_from_px(36.0_f32))
                    .px_3()
                    .gap_2()
                    .justify_between()
                    .items_center()
                    .bg(header_bg)
                    .border_b_1()
                    .border_color(self.theme.colors.outline_variant)
                    .child(
                        h_flex()
                            .flex_1()
                            .min_w_0()
                            .gap_2()
                            .items_center()
                            .overflow_hidden()
                            .child(Icon::new(kind.icon()).size(IconSize::Small))
                            .child(
                                div()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .child(Label::new(title).size(LabelSize::Small).truncate()),
                            )
                            .when(placed.focused, |el| {
                                el.child(status_badge(
                                    IconName::Check,
                                    "Focused",
                                    StatusTone::Accent,
                                    &self.theme,
                                ))
                            }),
                    )
                    .child({
                        let stack_ent = cx.entity();
                        let float_ent = cx.entity();
                        h_flex()
                            .gap_1()
                            .items_center()
                            .child(focusable_action_chip(
                                ElementId::Name(format!("win-stack-{win_id}").into()),
                                IconName::Code,
                                SharedString::default(),
                                None,
                                SharedString::from("Stack window with neighbor"),
                                self.compositor.active_stack_for(win_id).is_some(),
                                398,
                                &self.theme,
                                move |_, _w, cx| {
                                    stack_ent.update(cx, |shell, cx| {
                                        shell.compositor.focus_window(win_id);
                                        shell.stack_focused_window(cx);
                                    });
                                },
                            ))
                            .child(focusable_action_chip(
                                ElementId::Name(format!("win-float-{win_id}").into()),
                                IconName::Image,
                                SharedString::default(),
                                None,
                                SharedString::from("Toggle floating state"),
                                placed.floating,
                                399,
                                &self.theme,
                                move |_, _w, cx| {
                                    float_ent.update(cx, |shell, cx| {
                                        if shell.compositor.toggle_floating(win_id) {
                                            cx.notify();
                                        }
                                    });
                                },
                            ))
                            .child(focusable_action_chip(
                                ElementId::Name(format!("win-max-{win_id}").into()),
                                if is_maximized {
                                    IconName::Minimize
                                } else {
                                    IconName::Maximize
                                },
                                SharedString::default(),
                                None,
                                SharedString::from("Toggle maximize window"),
                                is_maximized,
                                400,
                                &self.theme,
                                move |_, _w, cx| {
                                    max_entity.update(cx, |shell, cx| {
                                        shell.compositor.toggle_maximize(win_id);
                                        cx.notify();
                                    });
                                },
                            ))
                            .child(focusable_action_chip(
                                ElementId::Name(format!("win-close-{win_id}").into()),
                                IconName::Close,
                                SharedString::default(),
                                None,
                                SharedString::from("Close window"),
                                false,
                                401,
                                &self.theme,
                                move |_, _w, cx| {
                                    close_entity.update(cx, |shell, cx| {
                                        shell.compositor.close_window(win_id);
                                        shell.window_kinds.remove(&win_id);
                                        cx.notify();
                                    });
                                },
                            ))
                    }),
            )
            .when_some(stack_strip, |el, strip| el.child(strip))
            .child(body)
            .into_any_element()
    }

    fn build_studio_body(&self, width: f32, cx: &mut Context<Self>) -> AnyElement {
        let e1 = cx.entity();
        let e2 = cx.entity();
        let e3 = cx.entity();
        let e4 = cx.entity();
        let e5 = cx.entity();
        let e6 = cx.entity();
        let e7 = cx.entity();
        let e8 = cx.entity();
        let e9 = cx.entity();
        let e10 = cx.entity();
        let e11 = cx.entity();

        render_studio_window(
            &self.studio,
            &self.craft_catalog,
            &self.frame_samples,
            width,
            &self.theme,
            StudioCallbacks {
                on_select_app: Arc::new(move |app_id, _w, cx| {
                    e1.update(cx, |shell, cx| {
                        shell.studio.select_app(&app_id, &shell.craft_catalog);
                        cx.notify();
                    });
                }),
                on_select_tool: Arc::new(move |idx, _w, cx| {
                    e2.update(cx, |shell, cx| {
                        shell.studio.active_tool_idx = idx;
                        cx.notify();
                    });
                }),
                on_zoom: Arc::new(move |val, _w, cx| {
                    e3.update(cx, |shell, cx| {
                        shell.studio.zoom_percent = val;
                        cx.notify();
                    });
                }),
                on_exposure: Arc::new(move |val, _w, cx| {
                    e4.update(cx, |shell, cx| {
                        shell.studio.exposure_ev = val;
                        cx.notify();
                    });
                }),
                on_grid_size: Arc::new(move |val, _w, cx| {
                    e5.update(cx, |shell, cx| {
                        shell.studio.grid_size_px = val;
                        cx.notify();
                    });
                }),
                on_toggle_snap: Arc::new(move |flag, _w, cx| {
                    e6.update(cx, |shell, cx| {
                        shell.studio.snap_to_grid = flag;
                        cx.notify();
                    });
                }),
                on_toggle_guides: Arc::new(move |flag, _w, cx| {
                    e7.update(cx, |shell, cx| {
                        shell.studio.high_contrast_guides = flag;
                        cx.notify();
                    });
                }),
                on_sample_svg: Arc::new(move |_w, cx| {
                    e8.update(cx, |shell, cx| {
                        shell.studio.load_svg_sample(&shell.craft_catalog);
                        cx.notify();
                    });
                }),
                on_sample_wasm: Arc::new(move |_w, cx| {
                    e9.update(cx, |shell, cx| {
                        shell.studio.load_wasm_sample(&shell.craft_catalog);
                        cx.notify();
                    });
                }),
                on_sample_self_bin: Arc::new(move |_w, cx| {
                    e10.update(cx, |shell, cx| {
                        shell.studio.load_self_binary_sample(&shell.craft_catalog);
                        cx.notify();
                    });
                }),
                on_toggle_section: Arc::new(move |sec, _w, cx| {
                    e11.update(cx, |shell, cx| {
                        shell.studio.memory.toggle_section(&sec);
                        cx.notify();
                    });
                }),
            },
        )
    }

    /// Scans `dir` on GPUI's background pool and updates [`FilesState`] without stalling UI frames.
    pub fn open_directory_async(&mut self, dir: PathBuf, cx: &mut Context<Self>) {
        self.files_scan_seq = self.files_scan_seq.wrapping_add(1);
        let seq = self.files_scan_seq;
        let catalog = self.craft_catalog.clone();
        let target_dir = dir.clone();
        cx.spawn(async move |this, cx| {
            let entries = cx
                .background_spawn(async move { FilesState::scan_directory(&dir, &catalog) })
                .await;
            if let Err(err) = this.update(cx, |shell, cx| {
                if shell.files_scan_seq == seq {
                    shell.files.apply_scan_results(target_dir, entries);
                    cx.notify();
                }
            }) {
                log::debug!("files rescan finished after shell dropped: {err:#}");
            }
        })
        .detach();
    }

    fn build_files_body(&self, cx: &mut Context<Self>) -> AnyElement {
        let e1 = cx.entity();
        let e2 = cx.entity();
        let e3 = cx.entity();
        render_files_window(
            &self.files,
            &self.theme,
            Arc::new(move |idx, _w, cx| {
                e1.update(cx, |shell, cx| {
                    if let Some(row) = shell.files.entries.get(idx).cloned() {
                        if row.is_dir {
                            shell.open_directory_async(row.path, cx);
                        } else {
                            shell.files.selected_idx = Some(idx);
                            cx.notify();
                        }
                    }
                });
            }),
            Arc::new(move |path: PathBuf, _w, cx| {
                e2.update(cx, |shell, cx| {
                    shell.studio.inspect_local_file(&path, &shell.craft_catalog);
                    shell.open_or_focus_window(WindowKind::Studio, "ArtCraft Studio", cx);
                });
            }),
            Arc::new(move |_w, cx| {
                e3.update(cx, |shell, cx| {
                    if let Some(parent) = shell.files.current_dir.parent().map(Path::to_path_buf) {
                        shell.open_directory_async(parent, cx);
                    }
                });
            }),
        )
    }

    fn build_monitor_body(&self, cx: &mut Context<Self>) -> AnyElement {
        let e = cx.entity();
        render_monitor_window(
            &self.supervisor,
            &self.hardware,
            &self.frame_samples,
            &self.theme,
            Arc::new(move |_w, cx| {
                e.update(cx, |shell, cx| {
                    shell.supervisor.activate_session();
                    cx.notify();
                });
            }),
        )
    }

    fn render_dock(&self, dock_h: f32, vw: f32, cx: &mut Context<Self>) -> AnyElement {
        crate::dock::render_cosmic_dock(self, &self.theme, dock_h, vw, cx)
    }
}

impl Focusable for OsDesktopShell {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for OsDesktopShell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.record_frame_sample();
        let viewport = window.viewport_size();
        let vw = f32::from(viewport.width).max(320.0);
        let vh = f32::from(viewport.height).max(240.0);
        if f32::from(viewport.width) > 100.0 && f32::from(viewport.height) > 100.0 {
            self.compositor.screen = Rect::new(0.0, 0.0, vw, vh);
        }
        let (top_h, dock_h) = self.sync_chrome_layers(vw, vh);

        let placed_windows = self.compositor.arrange_active_workspace();
        let stage_children: Vec<AnyElement> = placed_windows
            .iter()
            .map(|placed| self.render_placed_window(placed, top_h, cx))
            .collect();

        let top_panel = self.render_top_panel(top_h, window, cx);
        let dock = self.render_dock(dock_h, vw, cx);

        v_flex()
            .id("os-desktop-root")
            .role(Role::Application)
            .aria_label(SharedString::from(format!(
                "{} OS Desktop",
                brand::APP_NAME
            )))
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|this, _: &ToggleLauncher, _w, cx| this.toggle_launcher(cx)))
            .on_action(cx.listener(|this, _: &CycleLayout, _w, cx| this.cycle_layout(cx)))
            .on_action(cx.listener(|this, _: &ToggleDarkMode, _w, cx| this.toggle_mode(cx)))
            .on_action(cx.listener(|this, _: &ToggleOverview, _w, cx| this.toggle_overview(cx)))
            .on_action(cx.listener(|this, _: &ToggleScreenshot, _w, cx| this.toggle_screenshot(cx)))
            .on_action(cx.listener(|this, _: &ToggleLockScreen, _w, cx| this.toggle_lock_screen(cx)))
            .on_action(cx.listener(|this, _: &StackFocusedWindow, _w, cx| this.stack_focused_window(cx)))
            .on_action(cx.listener(|this, _: &CycleStackTab, _w, cx| this.cycle_active_stack(1, cx)))
            .on_action(cx.listener(|this, _: &ToggleFloatingWindow, _w, cx| this.toggle_floating_focused(cx)))
            .on_action(cx.listener(|this, _: &ToggleInspectorDrawer, _w, cx| {
                this.toggle_drawer(cx);
            }))
            .on_action(cx.listener(|this, _: &CloseOverlays, _w, cx| {
                this.launcher_open = false;
                this.drawer_open = false;
                let screen = this.compositor.screen;
                this.sync_chrome_layers(screen.w, screen.h);
                cx.notify();
            }))
            .on_action(cx.listener(|_, _: &FocusNext, window, cx| window.focus_next(cx)))
            .on_action(cx.listener(|_, _: &FocusPrev, window, cx| window.focus_prev(cx)))
            .on_key_down(cx.listener(Self::handle_key_down))
            .size_full()
            .bg(self.theme.colors.background)
            .text_color(self.theme.colors.on_surface)
            .child(top_panel)
            .child(
                h_flex()
                    .flex_1()
                    .w_full()
                    .min_h(px(0.0))
                    .overflow_hidden()
                    .child(
                        div()
                            .id("workspace-stage")
                            .relative()
                            .flex_1()
                            .h_full()
                            .overflow_hidden()
                            .bg(self.theme.colors.surface_dim)
                            .children(stage_children),
                    )
                    .when(self.drawer_open, |el| {
                        el.child(render_context_drawer_pane(self, cx))
                    }),
            )
            .child(dock)
            .when(self.launcher_open, |el| {
                el.child(render_launcher_modal(self, vw, vh, cx))
            })
            .when(self.app_library_open, |el| {
                let f_ent = cx.entity();
                let l_ent = cx.entity();
                let c_ent = cx.entity();
                el.child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .justify_center()
                        .items_center()
                        .bg(self.theme.colors.background.opacity(0.65))
                        .child(render_pop_launcher_and_app_library(
                            &self.theme,
                            &self.launcher_query,
                            self.launcher_folder,
                            &self.craft_catalog,
                            &self.discovered_apps,
                            Box::new(move |folder, cx| {
                                f_ent.update(cx, |s, cx| {
                                    s.launcher_folder = folder;
                                    cx.notify();
                                });
                            }),
                            Box::new(move |item, cx| {
                                l_ent.update(cx, |s, cx| match item.action {
                                    aphrody_session::LauncherActionKind::CraftApp(app_id) => {
                                        s.select_craft_app(&app_id, cx);
                                    }
                                    aphrody_session::LauncherActionKind::ShellCommand(cmd) => {
                                        s.term_state.push_command(&cmd);
                                        s.open_or_focus_window(
                                            WindowKind::Term,
                                            "COSMIC Terminal",
                                            cx,
                                        );
                                    }
                                    aphrody_session::LauncherActionKind::CompositorCommand(cmd) => {
                                        match cmd.as_str() {
                                            ":overview" => s.toggle_overview(cx),
                                            ":stack" => s.stack_focused_window(cx),
                                            ":tile" => s.cycle_layout(cx),
                                            ":float" => s.toggle_floating_focused(cx),
                                            ":dark" => s.toggle_mode(cx),
                                            ":shot" => s.toggle_screenshot(cx),
                                            ":lock" => s.toggle_lock_screen(cx),
                                            ":term" => s.open_or_focus_window(
                                                WindowKind::Term,
                                                "COSMIC Terminal",
                                                cx,
                                            ),
                                            ":edit" => s.open_or_focus_window(
                                                WindowKind::Edit,
                                                "COSMIC Edit",
                                                cx,
                                            ),
                                            ":store" => s.open_or_focus_window(
                                                WindowKind::Store,
                                                "COSMIC Store",
                                                cx,
                                            ),
                                            ":installer" => s.open_or_focus_window(
                                                WindowKind::Installer,
                                                "OS Installer & DE Switcher",
                                                cx,
                                            ),
                                            ":settings" => s.open_or_focus_window(
                                                WindowKind::Settings,
                                                "COSMIC Settings",
                                                cx,
                                            ),
                                            ":monitor" => s.open_or_focus_window(
                                                WindowKind::Monitor,
                                                "System Monitor",
                                                cx,
                                            ),
                                            ":files" => s.open_or_focus_window(
                                                WindowKind::Files,
                                                "COSMIC Files",
                                                cx,
                                            ),
                                            _ => cx.notify(),
                                        }
                                    }
                                    aphrody_session::LauncherActionKind::FilePath(_) => {
                                        s.open_or_focus_window(
                                            WindowKind::Files,
                                            "COSMIC Files",
                                            cx,
                                        );
                                    }
                                    aphrody_session::LauncherActionKind::DesktopApp(_)
                                    | aphrody_session::LauncherActionKind::CalculatorResult(_) => {
                                        s.open_or_focus_window(
                                            WindowKind::Store,
                                            "COSMIC Store",
                                            cx,
                                        );
                                    }
                                });
                            }),
                            Box::new(move |cx| {
                                c_ent.update(cx, |s, cx| {
                                    s.app_library_open = false;
                                    cx.notify();
                                });
                            }),
                        )),
                )
            })
            .when(self.overview_open, |el| {
                let ws_ent = cx.entity();
                let cl_ent = cx.entity();
                let mode_ent = cx.entity();
                el.child(
                    div()
                        .absolute()
                        .inset_0()
                        .child(render_workspaces_overview(
                            &self.compositor,
                            &self.theme,
                            Box::new(move |wid, cx| {
                                ws_ent.update(cx, |s, cx| {
                                    if s.compositor.switch_workspace(wid) {
                                        cx.notify();
                                    }
                                });
                            }),
                            Box::new(move |cx| {
                                cl_ent.update(cx, |s, cx| {
                                    s.overview_open = false;
                                    cx.notify();
                                });
                            }),
                            Box::new(move |mode, cx| {
                                mode_ent.update(cx, |s, cx| s.set_layout(mode, cx));
                            }),
                        )),
                )
            })
            .when(self.screenshot_open, |el| {
                let cap_ent = cx.entity();
                let cl_ent = cx.entity();
                el.child(
                    div()
                        .absolute()
                        .bottom(px(82.0))
                        .left(px(0.0))
                        .right(px(0.0))
                        .flex()
                        .justify_center()
                        .child(render_cosmic_screenshot_toolbar(
                            &self.theme,
                            Box::new(move |mode, cx| {
                                cap_ent.update(cx, |s, cx| {
                                    s.notifications.push(
                                        "cosmic-screenshot",
                                        "Screenshot Ready",
                                        format!("Captured {mode} frame cleanly"),
                                        aphrody_session::NotificationUrgency::Low,
                                    );
                                    s.trigger_osd(
                                        aphrody_session::OsdKind::Screenshot,
                                        "Screenshot Captured",
                                        mode,
                                        100,
                                        cx,
                                    );
                                });
                            }),
                            Box::new(move |cx| {
                                cl_ent.update(cx, |s, cx| {
                                    s.screenshot_open = false;
                                    cx.notify();
                                });
                            }),
                        )),
                )
            })
            .when_some(self.osd.as_ref(), |el, banner| {
                el.child(
                    div()
                        .absolute()
                        .bottom(px(88.0))
                        .left(px(0.0))
                        .right(px(0.0))
                        .flex()
                        .justify_center()
                        .child(render_cosmic_osd_pill(&self.theme, banner)),
                )
            })
            .when(!self.notifications.items.is_empty(), |el| {
                let d_ent = cx.entity();
                let c_ent = cx.entity();
                let drawer_w = if self.drawer_open {
                    (vw * 0.36).clamp(220.0, DRAWER_WIDTH)
                } else {
                    0.0
                };
                el.child(
                    div()
                        .absolute()
                        .top(px(54.0))
                        .right(px(drawer_w + 16.0))
                        .child(render_cosmic_notifications_stack(
                            &self.theme,
                            &self.notifications,
                            Box::new(move |nid, cx| {
                                d_ent.update(cx, |s, cx| {
                                    s.notifications.dismiss(nid);
                                    cx.notify();
                                });
                            }),
                            Box::new(move |cx| {
                                c_ent.update(cx, |s, cx| {
                                    s.notifications.clear();
                                    cx.notify();
                                });
                            }),
                        )),
                )
            })
            .when(self.lock_screen_open, |el| {
                let u_ent = cx.entity();
                el.child(
                    div()
                        .absolute()
                        .inset_0()
                        .child(render_cosmic_greeter_lock(
                            &self.theme,
                            "aphrody",
                            Box::new(move |cx| {
                                u_ent.update(cx, |s, cx| {
                                    s.lock_screen_open = false;
                                    cx.notify();
                                });
                            }),
                        )),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adaptive_panel_heights_compact_below_threshold() {
        assert_eq!(OsDesktopShell::panel_heights(600.0), (38.0, 54.0));
        assert_eq!(
            OsDesktopShell::panel_heights(900.0),
            (TOP_PANEL_HEIGHT, BOTTOM_DOCK_HEIGHT)
        );
    }

    #[test]
    fn files_scan_directory_lists_sorted_entries_without_panicking() {
        let dir = tempfile::tempdir().expect("create temp dir");
        std::fs::create_dir_all(dir.path().join("z_folder")).expect("create subdir");
        std::fs::write(dir.path().join("sample.svg"), b"<svg/>").expect("write svg");
        let catalog = CraftCatalog::bundled().expect("catalog parses");
        let rows = FilesState::scan_directory(dir.path(), &catalog);
        assert_eq!(rows.len(), 2);
        assert!(rows[0].is_dir);
        assert_eq!(rows[0].name, "z_folder");
        assert_eq!(rows[1].name, "sample.svg");
    }
}
