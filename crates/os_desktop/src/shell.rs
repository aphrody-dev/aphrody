use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use gpui::{
    AnyElement, App, Context, ElementId, Entity, FocusHandle, Focusable, KeyDownEvent, Render,
    Role, SharedString, Subscription, Window, actions, div, prelude::*, px,
};
use os_craft::CraftCatalog;
use os_session::{
    AppEntry, CompositorState, HardwareCapabilities, LayerAnchor, LayerKind, LayerSurface,
    PlacedWindow, Rect, SessionManifest, SessionSupervisor, TilingMode, ToplevelId, UnitPhase,
    WorkspaceId, discover_system_apps,
};
use os_ui::{
    AppearanceMode, AppletSlot, ImmediateUi, OsConfigStore, OsTheme, OsThemeCatalog, PanelApplet,
    ReactiveHost, StatusTone, contrast_ratio, focusable_action_chip, render_context_drawer,
    status_badge,
};
use ui::{Icon, IconName, IconSize, Label, LabelCommon as _, LabelSize, h_flex, v_flex};

use crate::applets::{BrandLauncherApplet, SystemTelemetryApplet, WorkspaceApplet, WorkspacePill};
use crate::studio_view::{StudioCallbacks, StudioState, render_studio_window};
use crate::windows::{
    DesktopPrefs, FilesState, SettingsFlags, SettingsMessage, SettingsProgram, WindowKind,
    render_files_window, render_monitor_window,
};
use crate::{DesktopOptions, parse_icon_name};

const TOP_PANEL_HEIGHT: f32 = 44.0;
const BOTTOM_DOCK_HEIGHT: f32 = 64.0;
const DRAWER_WIDTH: f32 = 320.0;

actions!(
    os_desktop,
    [
        ToggleLauncher,
        CycleLayout,
        ToggleDarkMode,
        ToggleInspectorDrawer,
        CloseOverlays,
        FocusNext,
        FocusPrev,
    ]
);

/// Top-level GPUI COSMIC Epoch desktop shell combining TopPanel applets, Compositor tiling,
/// floating DockBar, searchable unified Launcher, and modular reactive/immediate windows.
pub struct OsDesktopShell {
    focus_handle: FocusHandle,
    theme_catalog: OsThemeCatalog,
    theme: OsTheme,
    prefs_store: OsConfigStore<DesktopPrefs>,
    craft_catalog: CraftCatalog,
    supervisor: SessionSupervisor,
    compositor: CompositorState,
    discovered_apps: Vec<AppEntry>,
    window_kinds: BTreeMap<ToplevelId, WindowKind>,
    studio: StudioState,
    files: FilesState,
    settings_host: Entity<ReactiveHost<SettingsProgram>>,
    _settings_obs: Subscription,
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
            .or_else(|| theme_catalog.resolve("aphrody", Some(prefs_store.value.mode)))
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
                workspaces: os_craft::WorkspaceDefaults {
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

        Self {
            focus_handle,
            theme_catalog,
            theme,
            prefs_store,
            craft_catalog,
            supervisor,
            compositor,
            discovered_apps,
            window_kinds,
            studio,
            files,
            settings_host,
            _settings_obs: settings_obs,
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
        cx.notify();
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

    fn panel_heights(viewport_h: f32) -> (f32, f32) {
        if viewport_h < 680.0 {
            (38.0, 54.0)
        } else {
            (TOP_PANEL_HEIGHT, BOTTOM_DOCK_HEIGHT)
        }
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

        h_flex()
            .id("os-top-panel")
            .role(Role::Toolbar)
            .aria_label("Desktop Top Panel")
            .w_full()
            .h(px(top_h))
            .px_3()
            .justify_between()
            .items_center()
            .bg(self.theme.colors.surface_container_low)
            .border_b_1()
            .border_color(self.theme.colors.outline_variant)
            .child(h_flex().gap_1p5().items_center().children(left_pills))
            .child(h_flex().gap_2().items_center().children(center_pills))
            .child(h_flex().gap_2().items_center().children(right_pills))
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

        let border_color = if placed.focused {
            self.theme.colors.primary
        } else {
            self.theme.colors.outline_variant
        };
        let header_bg = if placed.focused {
            self.theme.colors.surface_container_high
        } else {
            self.theme.colors.surface_container
        };

        let body = match kind {
            WindowKind::Studio => self.build_studio_body(placed.rect.w, cx),
            WindowKind::Files => self.build_files_body(cx),
            WindowKind::Settings => div()
                .flex_1()
                .w_full()
                .min_h(px(0.0))
                .child(self.settings_host.clone())
                .into_any_element(),
            WindowKind::Monitor => self.build_monitor_body(cx),
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
                    .h(px(36.0))
                    .px_3()
                    .justify_between()
                    .items_center()
                    .bg(header_bg)
                    .border_b_1()
                    .border_color(self.theme.colors.outline_variant)
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(Icon::new(kind.icon()).size(IconSize::Small))
                            .child(Label::new(title).size(LabelSize::Small))
                            .when(placed.focused, |el| {
                                el.child(status_badge(
                                    IconName::Check,
                                    "Focused",
                                    StatusTone::Accent,
                                    &self.theme,
                                ))
                            }),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
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
                            )),
                    ),
            )
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
                            shell.files.rescan(&row.path, &shell.craft_catalog);
                        } else {
                            shell.files.selected_idx = Some(idx);
                        }
                        cx.notify();
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
                        shell.files.rescan(&parent, &shell.craft_catalog);
                        cx.notify();
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

    fn render_context_drawer_pane(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut ui = ImmediateUi::new(&self.theme);
        ui.heading("Desktop Tokens & Contrast");
        let on_bg = contrast_ratio(
            self.theme.colors.on_background,
            self.theme.colors.background,
        );
        let on_primary = contrast_ratio(self.theme.colors.on_primary, self.theme.colors.primary);
        ui.kv_row(
            "Theme ID",
            format!("{} ({:?})", self.theme.id, self.theme.mode),
        );
        ui.kv_row("AA Background", format!("{on_bg:.2}:1"));
        ui.kv_row("AA Primary", format!("{on_primary:.2}:1"));
        ui.color_swatch("Primary Seed", self.theme.colors.primary, "Interactive");
        ui.color_swatch("Secondary", self.theme.colors.secondary, "Accent");
        ui.color_swatch(
            "Surface Low",
            self.theme.colors.surface_container_low,
            "Chrome",
        );

        ui.section("Quick Themes", |sec| {
            for tid in self.theme_catalog.ids() {
                let entity = cx.entity();
                let label = tid.clone();
                let selected = tid == self.theme.id;
                sec.button(
                    ElementId::Name(format!("drawer-theme-{tid}").into()),
                    IconName::Sparkle,
                    label,
                    selected,
                    move |_, _w, cx| {
                        entity.update(cx, |shell, cx| shell.select_theme(&tid, cx));
                    },
                );
            }
        });

        let close_entity = cx.entity();
        render_context_drawer(
            "Inspector Drawer",
            ui.finish(),
            &self.theme,
            move |_w, cx| {
                close_entity.update(cx, |shell, cx| shell.toggle_drawer(cx));
            },
        )
    }

    fn render_launcher_modal(&self, vw: f32, vh: f32, cx: &mut Context<Self>) -> AnyElement {
        let query = self.launcher_query.trim().to_ascii_lowercase();
        let close_entity = cx.entity();

        let mut app_list = v_flex().w_full().gap_1p5();
        let mut shown = 0_usize;

        for app in &self.craft_catalog.apps {
            if !query.is_empty()
                && !app.name.to_ascii_lowercase().contains(&query)
                && !app.tagline.to_ascii_lowercase().contains(&query)
                && !app.category.to_ascii_lowercase().contains(&query)
            {
                continue;
            }
            let entity = cx.entity();
            let app_id = app.id.clone();
            let selected = app_id == self.studio.active_app_id;
            app_list = app_list.child(focusable_action_chip(
                ElementId::Name(format!("launcher-craft-{app_id}").into()),
                parse_icon_name(&app.icon),
                SharedString::from(format!("{} · {}", app.name, app.category)),
                Some(SharedString::from(format!("{} ext", app.extensions.len()))),
                SharedString::from(format!("Launch {}", app.name)),
                selected,
                (500 + shown) as isize,
                &self.theme,
                move |_, _w, cx| {
                    entity.update(cx, |shell, cx| shell.select_craft_app(&app_id, cx));
                },
            ));
            shown += 1;
            if shown >= 12 {
                break;
            }
        }

        for sys_app in &self.discovered_apps {
            if shown >= 16 {
                break;
            }
            if !query.is_empty() && !sys_app.name.to_ascii_lowercase().contains(&query) {
                continue;
            }
            let entity = cx.entity();
            let app_name = sys_app.name.clone();
            app_list = app_list.child(focusable_action_chip(
                ElementId::Name(format!("launcher-sys-{shown}").into()),
                IconName::Terminal,
                SharedString::from(format!("{} ({:?})", sys_app.name, sys_app.origin)),
                None,
                SharedString::from(format!("Open system entry {}", sys_app.name)),
                false,
                (530 + shown) as isize,
                &self.theme,
                move |_, _w, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.open_or_focus_window(WindowKind::Files, &app_name, cx);
                    });
                },
            ));
            shown += 1;
        }

        let modal_w = (vw - 32.0).clamp(280.0, 580.0);
        let modal_h = (vh - 108.0).clamp(240.0, 620.0);
        let modal_top = if vh < 680.0 { 44.0 } else { 72.0 };

        div()
            .id("launcher-backdrop")
            .role(Role::Dialog)
            .aria_label("Unified Application Launcher")
            .absolute()
            .inset_0()
            .flex()
            .items_start()
            .justify_center()
            .pt(px(modal_top))
            .bg(gpui::Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.0,
                a: 0.45,
            })
            .child(
                v_flex()
                    .w(px(modal_w))
                    .max_h(px(modal_h))
                    .p_4()
                    .gap_3()
                    .rounded(px(14.0))
                    .bg(self.theme.colors.surface_container)
                    .border_2()
                    .border_color(self.theme.colors.primary)
                    .child(
                        h_flex()
                            .w_full()
                            .justify_between()
                            .items_center()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(Icon::new(IconName::MagnifyingGlass).size(IconSize::Small))
                                    .child(
                                        Label::new(if self.launcher_query.is_empty() {
                                            "Type to filter 17 ArtCraft apps + system desktop entries…"
                                                .to_owned()
                                        } else {
                                            format!("Filter: {}", self.launcher_query)
                                        })
                                        .size(LabelSize::Default),
                                    ),
                            )
                            .child(focusable_action_chip(
                                "launcher-close",
                                IconName::Close,
                                SharedString::from("Esc"),
                                None,
                                SharedString::from("Close launcher modal"),
                                false,
                                499,
                                &self.theme,
                                move |_, _w, cx| {
                                    close_entity.update(cx, |shell, cx| {
                                        shell.launcher_open = false;
                                        cx.notify();
                                    });
                                },
                            )),
                    )
                    .child(
                        v_flex()
                            .id("launcher-scroll")
                            .flex_1()
                            .overflow_y_scroll()
                            .child(app_list),
                    ),
            )
            .into_any_element()
    }

    fn render_dock(&self, dock_h: f32, vw: f32, cx: &mut Context<Self>) -> AnyElement {
        let compact_dock = vw < 640.0;
        let items: Vec<(
            &'static str,
            IconName,
            SharedString,
            Option<WindowKind>,
            Option<&'static str>,
        )> = if compact_dock {
            vec![
                (
                    "dock-launcher",
                    IconName::Command,
                    SharedString::from("Launcher"),
                    None,
                    None,
                ),
                (
                    "dock-studio",
                    IconName::Image,
                    SharedString::from("ArtCraft Studio"),
                    Some(WindowKind::Studio),
                    None,
                ),
                (
                    "dock-craft-anything",
                    IconName::Binary,
                    SharedString::from("Craft Anything"),
                    None,
                    Some("craft-anything"),
                ),
                (
                    "dock-files",
                    IconName::FolderOpen,
                    SharedString::from("Files"),
                    Some(WindowKind::Files),
                    None,
                ),
                (
                    "dock-settings",
                    IconName::Settings,
                    SharedString::from("Settings"),
                    Some(WindowKind::Settings),
                    None,
                ),
                (
                    "dock-monitor",
                    IconName::Server,
                    SharedString::from("Monitor"),
                    Some(WindowKind::Monitor),
                    None,
                ),
            ]
        } else {
            vec![
                (
                    "dock-launcher",
                    IconName::Command,
                    SharedString::from("Launcher"),
                    None,
                    None,
                ),
                (
                    "dock-studio",
                    IconName::Image,
                    SharedString::from("ArtCraft Studio"),
                    Some(WindowKind::Studio),
                    None,
                ),
                (
                    "dock-photocraft",
                    IconName::Sparkle,
                    SharedString::from("PhotoCraft"),
                    None,
                    Some("photocraft"),
                ),
                (
                    "dock-vectorcraft",
                    IconName::Pencil,
                    SharedString::from("VectorCraft"),
                    None,
                    Some("vectorcraft"),
                ),
                (
                    "dock-craft-anything",
                    IconName::Binary,
                    SharedString::from("Craft Anything"),
                    None,
                    Some("craft-anything"),
                ),
                (
                    "dock-files",
                    IconName::FolderOpen,
                    SharedString::from("Files"),
                    Some(WindowKind::Files),
                    None,
                ),
                (
                    "dock-settings",
                    IconName::Settings,
                    SharedString::from("Settings"),
                    Some(WindowKind::Settings),
                    None,
                ),
                (
                    "dock-monitor",
                    IconName::Server,
                    SharedString::from("Monitor"),
                    Some(WindowKind::Monitor),
                    None,
                ),
            ]
        };

        let tile_size = if compact_dock {
            38.0
        } else {
            self.theme.layout.min_touch_target
        };

        let active_kind = self
            .compositor
            .focused_toplevel
            .and_then(|id| self.window_kinds.get(&id).copied());

        let mut dock_row = h_flex()
            .px_3()
            .py_1p5()
            .gap_2()
            .items_center()
            .rounded(px(16.0))
            .bg(self.theme.colors.surface_container_high)
            .border_1()
            .border_color(self.theme.colors.outline_variant);

        for (idx, (id, icon, label, kind_opt, craft_id)) in items.into_iter().enumerate() {
            let is_active = match (kind_opt, craft_id) {
                (Some(kind), _) => active_kind == Some(kind),
                (None, Some(cid)) => {
                    self.studio.active_app_id == cid && active_kind == Some(WindowKind::Studio)
                }
                (None, None) => self.launcher_open,
            };
            let entity = cx.entity();
            let ring = self.theme.colors.primary;
            let bg = if is_active {
                self.theme.colors.primary_container
            } else {
                self.theme.colors.surface_container
            };
            let fg = if is_active {
                self.theme.colors.on_primary_container
            } else {
                self.theme.colors.on_surface
            };
            let border = if is_active {
                self.theme.colors.primary
            } else {
                self.theme.colors.outline_variant
            };

            dock_row = dock_row.child(
                div()
                    .id(id)
                    .min_w(px(tile_size))
                    .min_h(px(tile_size))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_0p5()
                    .rounded(px(12.0))
                    .bg(bg)
                    .text_color(fg)
                    .border_1()
                    .border_color(border)
                    .cursor_pointer()
                    .focusable()
                    .tab_index((700 + idx) as isize)
                    .tab_stop(true)
                    .role(Role::Button)
                    .aria_label(label.clone())
                    .focus_visible(move |s| s.border_2().border_color(ring))
                    .on_click(move |_, _w, cx| {
                        entity.update(cx, |shell, cx| {
                            if let Some(kind) = kind_opt {
                                shell.open_or_focus_window(kind, label.as_ref(), cx);
                            } else if let Some(cid) = craft_id {
                                shell.select_craft_app(cid, cx);
                            } else {
                                shell.toggle_launcher(cx);
                            }
                        });
                    })
                    .child(Icon::new(icon).size(IconSize::Medium))
                    .when(is_active, |el| {
                        el.child(
                            div()
                                .w(px(10.0))
                                .h(px(2.5))
                                .rounded(px(2.0))
                                .bg(self.theme.colors.primary),
                        )
                    }),
            );
        }

        h_flex()
            .id("os-bottom-dock")
            .role(Role::Toolbar)
            .aria_label("Desktop Application Dock")
            .w_full()
            .h(px(dock_h))
            .justify_center()
            .items_center()
            .bg(self.theme.colors.background)
            .child(dock_row)
            .into_any_element()
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
                        el.child(self.render_context_drawer_pane(cx))
                    }),
            )
            .child(dock)
            .when(self.launcher_open, |el| {
                el.child(self.render_launcher_modal(vw, vh, cx))
            })
    }
}
