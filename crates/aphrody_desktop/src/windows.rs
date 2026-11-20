use std::path::PathBuf;
use std::sync::Arc;

use gpui::{AnyElement, App, Context, ElementId, SharedString, Window, div, prelude::*, px};
use aphrody_craft::{CraftCatalog, InspectionReport, inspect_path};
use aphrody_session::{
    DistBundleSpec, HardwareCapabilities, SessionEnvironment, SessionSupervisor, TilingMode,
    UnitPhase,
};
use aphrody_ui::{
    AppearanceMode, GlassMaterialPreset, ImmediateUi, M3DynamicVariant, OsConfigStore,
    OsI18nCatalog, OsTheme, OsThemeCatalog, ReactiveHost, ReactiveProgram, StatusTone,
    SubscriptionSpec, TaskCommand, UnifiedPlatformProfile, focusable_action_chip, progress_meter,
    slider_control, sparkline_chart, status_badge, toggle_control,
};
use serde::{Deserialize, Serialize};
use ui::{IconName, Label, LabelCommon as _, LabelSize, h_flex, rems_from_px, v_flex};

/// Window payload identity inside [`aphrody_session::CompositorState`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WindowKind {
    Studio,
    Browser,
    Vpn,
    Files,
    Settings,
    Monitor,
    Term,
    Edit,
    Store,
    Installer,
    Design,
}

impl WindowKind {
    pub const ALL: [Self; 11] = [
        Self::Studio,
        Self::Browser,
        Self::Vpn,
        Self::Files,
        Self::Settings,
        Self::Monitor,
        Self::Term,
        Self::Edit,
        Self::Store,
        Self::Installer,
        Self::Design,
    ];

    #[must_use]
    pub const fn app_id(self) -> &'static str {
        match self {
            Self::Studio => "artcraft-studio",
            Self::Browser => "aphrody-browser",
            Self::Vpn => "aphrody-vpn",
            Self::Files => "cosmic-files",
            Self::Settings => "cosmic-settings",
            Self::Monitor => "system-monitor",
            Self::Term => "cosmic-term",
            Self::Edit => "cosmic-edit",
            Self::Store => "cosmic-store",
            Self::Installer => "aphrody-installer",
            Self::Design => "design-gallery",
        }
    }

    #[must_use]
    pub const fn default_title(self) -> &'static str {
        match self {
            Self::Studio => "Studio",
            Self::Browser => "Aphrody Browser",
            Self::Vpn => "WireGuard VPN",
            Self::Files => "COSMIC Files",
            Self::Settings => "COSMIC Settings",
            Self::Monitor => "System Monitor",
            Self::Term => "COSMIC Terminal",
            Self::Edit => "COSMIC Edit",
            Self::Store => "COSMIC Store",
            Self::Installer => "OS Installer & DE Switcher",
            Self::Design => "Design & Hub",
        }
    }

    #[must_use]
    pub const fn icon(self) -> IconName {
        match self {
            Self::Studio => IconName::Image,
            Self::Browser => IconName::ToolWeb,
            Self::Vpn => IconName::VpnShield,
            Self::Files => IconName::FolderOpen,
            Self::Settings => IconName::Settings,
            Self::Monitor => IconName::Server,
            Self::Term => IconName::Terminal,
            Self::Edit => IconName::FileCode,
            Self::Store => IconName::Download,
            Self::Installer => IconName::Code,
            Self::Design => IconName::Sparkle,
        }
    }
}

/// Persistent user preferences stored atomically via `OsConfigStore` (Pop!_OS `cosmic-config` pattern).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DesktopPrefs {
    pub theme_id: String,
    pub mode: AppearanceMode,
    pub layout_mode: TilingMode,
    pub window_gap: f32,
    pub reduced_motion: bool,
    #[serde(default = "default_locale_pref")]
    pub locale: String,
    #[serde(default)]
    pub glass_preset: GlassMaterialPreset,
    #[serde(default)]
    pub m3_seed_rgb: Option<[u8; 3]>,
    #[serde(default)]
    pub m3_variant: M3DynamicVariant,
}

fn default_locale_pref() -> String {
    aphrody_session::detect_process_locale().as_str().to_owned()
}

impl Default for DesktopPrefs {
    fn default() -> Self {
        Self {
            theme_id: brand::APP_SLUG.to_owned(),
            mode: AppearanceMode::Dark,
            layout_mode: TilingMode::Dwindle,
            window_gap: 8.0,
            reduced_motion: false,
            locale: default_locale_pref(),
            glass_preset: GlassMaterialPreset::LiquidGlass,
            m3_seed_rgb: None,
            m3_variant: M3DynamicVariant::Expressive,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SettingsFlags {
    pub prefs: DesktopPrefs,
    pub theme_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SettingsState {
    pub prefs: DesktopPrefs,
    pub theme_ids: Vec<String>,
    pub saved_note: String,
    pub version: u64,
}

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    SelectTheme(String),
    SetMode(AppearanceMode),
    SetLayout(TilingMode),
    SetGap(f32),
    SetReducedMotion(bool),
    SetLocale(String),
}

/// `iced`-style reactive COSMIC Settings application hosted on a GPUI `ReactiveHost`.
pub struct SettingsProgram;

impl ReactiveProgram for SettingsProgram {
    type State = SettingsState;
    type Message = SettingsMessage;
    type Flags = SettingsFlags;

    fn init(flags: Self::Flags) -> (Self::State, TaskCommand<Self::Message>) {
        let note = OsConfigStore::<DesktopPrefs>::load_or_default("desktop", "prefs")
            .path
            .display()
            .to_string();
        (
            SettingsState {
                prefs: flags.prefs,
                theme_ids: flags.theme_ids,
                saved_note: format!("Store: {note}"),
                version: 0,
            },
            TaskCommand::none(),
        )
    }

    fn update(state: &mut Self::State, message: Self::Message) -> TaskCommand<Self::Message> {
        match message {
            SettingsMessage::SelectTheme(id) => state.prefs.theme_id = id,
            SettingsMessage::SetMode(mode) => state.prefs.mode = mode,
            SettingsMessage::SetLayout(layout) => state.prefs.layout_mode = layout,
            SettingsMessage::SetGap(gap) => state.prefs.window_gap = gap.clamp(0.0, 24.0),
            SettingsMessage::SetReducedMotion(flag) => state.prefs.reduced_motion = flag,
            SettingsMessage::SetLocale(locale) => {
                state.prefs.locale = aphrody_session::Bcp47Tag::parse(&locale)
                    .map_or_else(|| "en".to_owned(), |t| t.as_str().to_owned());
            }
        }
        state.version += 1;

        let mut store = OsConfigStore::load_or_default("desktop", "prefs");
        store.value = state.prefs.clone();
        match store.save() {
            Ok(()) => state.saved_note = format!("Saved atomically (rev {})", state.version),
            Err(err) => state.saved_note = format!("Save deferred: {err}"),
        }
        TaskCommand::none()
    }

    fn subscription(_state: &Self::State) -> SubscriptionSpec<Self::Message> {
        SubscriptionSpec::None
    }

    fn view(
        state: &Self::State,
        _window: &mut Window,
        cx: &mut Context<ReactiveHost<Self>>,
    ) -> AnyElement {
        let theme = match OsTheme::try_global(cx).cloned() {
            Some(active) => active,
            None => match OsThemeCatalog::bundled()
                .ok()
                .and_then(|c| c.resolve(&state.prefs.theme_id, Some(state.prefs.mode)))
            {
                Some(resolved) => resolved,
                None => return gpui::div().into_any_element(),
            },
        };

        let i18n = OsI18nCatalog::load_with_user_override().unwrap_or_default();
        let active_locale = i18n.resolve_tag(&state.prefs.locale);
        let active_dir = i18n.direction(&active_locale);

        let entity = cx.entity();
        let mut ui = ImmediateUi::new(&theme);
        ui.heading(i18n.translate(&active_locale, "settings_title"));
        ui.label(state.saved_note.clone());

        let mut theme_chips = h_flex().w_full().gap_1p5().flex_wrap();
        for (idx, theme_id) in state.theme_ids.iter().enumerate() {
            let selected = theme_id == &state.prefs.theme_id;
            let tid = theme_id.clone();
            let target = entity.clone();
            theme_chips = theme_chips.child(focusable_action_chip(
                ElementId::Name(format!("settings-theme-{tid}").into()),
                IconName::Sparkle,
                SharedString::from(tid.clone()),
                None,
                SharedString::from(format!("Apply theme {tid}")),
                selected,
                (200 + idx) as isize,
                &theme,
                move |_, _window, cx| {
                    target.update(cx, |host, cx| {
                        host.dispatch(SettingsMessage::SelectTheme(tid.clone()), cx);
                    });
                },
            ));
        }

        let target_mode = entity.clone();
        let is_light = state.prefs.mode == AppearanceMode::Light;
        let mode_toggle = toggle_control(
            "settings-polarity",
            "Light Polarity",
            is_light,
            &theme,
            move |next_light, _window, cx| {
                let next_mode = if next_light {
                    AppearanceMode::Light
                } else {
                    AppearanceMode::Dark
                };
                target_mode.update(cx, |host, cx| {
                    host.dispatch(SettingsMessage::SetMode(next_mode), cx);
                });
            },
        );

        let mut layout_chips = h_flex().w_full().gap_1p5().flex_wrap();
        for (idx, layout) in TilingMode::ALL.into_iter().enumerate() {
            let selected = layout == state.prefs.layout_mode;
            let target = entity.clone();
            layout_chips = layout_chips.child(focusable_action_chip(
                ElementId::Name(format!("settings-layout-{}", layout.slug()).into()),
                IconName::Split,
                SharedString::from(layout.label()),
                None,
                SharedString::from(format!("Use {} workspace tiling", layout.label())),
                selected,
                (230 + idx) as isize,
                &theme,
                move |_, _window, cx| {
                    target.update(cx, |host, cx| {
                        host.dispatch(SettingsMessage::SetLayout(layout), cx);
                    });
                },
            ));
        }

        let target_gap = entity.clone();
        let gap_slider = slider_control(
            "settings-gap",
            "Window Gap",
            state.prefs.window_gap,
            0.0,
            24.0,
            "px",
            &theme,
            move |gap, _window, cx| {
                target_gap.update(cx, |host, cx| {
                    host.dispatch(SettingsMessage::SetGap(gap), cx);
                });
            },
        );

        let mut locale_chips = h_flex().w_full().gap_1p5().flex_wrap();
        for (idx, tag) in i18n.available_locales().into_iter().enumerate() {
            let selected = tag.eq_ignore_ascii_case(&active_locale);
            let target = entity.clone();
            let chosen = tag.clone();
            let label = i18n
                .entry(&tag)
                .map_or_else(|| tag.clone(), |e| format!("{} ({tag})", e.native_name));
            let dir_hint = i18n.direction(&tag);
            locale_chips = locale_chips.child(focusable_action_chip(
                ElementId::Name(format!("settings-locale-{tag}").into()),
                IconName::Font,
                SharedString::from(label),
                None,
                SharedString::from(format!("BCP 47 {tag} ({dir_hint})")),
                selected,
                (260 + idx) as isize,
                &theme,
                move |_, _window, cx| {
                    target.update(cx, |host, cx| {
                        host.dispatch(SettingsMessage::SetLocale(chosen.clone()), cx);
                    });
                },
            ));
        }

        let target_motion = entity;
        let motion_toggle = toggle_control(
            "settings-reduced-motion",
            i18n.translate(&active_locale, "reduced_motion"),
            state.prefs.reduced_motion,
            &theme,
            move |flag, _window, cx| {
                target_motion.update(cx, |host, cx| {
                    host.dispatch(SettingsMessage::SetReducedMotion(flag), cx);
                });
            },
        );

        ui.section(i18n.translate(&active_locale, "theme_catalog"), |sec| {
            sec.push_element(mode_toggle);
            sec.push_element(theme_chips.into_any_element());
            sec.color_swatch("Primary", sec.theme.colors.primary, "M3 Seed");
            sec.color_swatch("Surface", sec.theme.colors.surface_container, "Container");
        });

        ui.section(i18n.translate(&active_locale, "language_locale"), |sec| {
            sec.kv_row(
                i18n.translate(&active_locale, "text_direction"),
                format!("{active_locale} | dir={active_dir}"),
            );
            sec.push_element(locale_chips.into_any_element());
        });

        ui.section(
            i18n.translate(&active_locale, "compositor_geometry"),
            |sec| {
                sec.push_element(layout_chips.into_any_element());
                sec.push_element(gap_slider);
                sec.push_element(motion_toggle);
            },
        );

        v_flex()
            .id("settings-scroll")
            .size_full()
            .p_3()
            .overflow_y_scroll()
            .bg(theme.colors.surface)
            .child(ui.finish())
            .into_any_element()
    }
}

#[derive(Debug, Clone)]
pub struct FileRow {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub bytes: u64,
    pub report: Option<InspectionReport>,
}

/// Interactive COSMIC Files state inspecting directory entries live via `craft-sniff` & `aphrody_craft`.
#[derive(Debug, Clone)]
pub struct FilesState {
    pub current_dir: PathBuf,
    pub entries: Vec<FileRow>,
    pub selected_idx: Option<usize>,
}

impl FilesState {
    pub fn scan_initial(catalog: &CraftCatalog) -> Self {
        let dir = std::env::current_dir()
            .ok()
            .map(|cwd| {
                let candidate = cwd.join("assets").join("os");
                if candidate.is_dir() { candidate } else { cwd }
            })
            .unwrap_or_else(|| PathBuf::from("."));
        let mut state = Self {
            current_dir: dir.clone(),
            entries: Vec::new(),
            selected_idx: None,
        };
        state.rescan(&dir, catalog);
        state
    }

    /// Reads directory entries and runs bounded `craft-sniff` inspection on a thread-safe slice.
    #[must_use]
    pub fn scan_directory(dir: &std::path::Path, catalog: &CraftCatalog) -> Vec<FileRow> {
        let mut entries = Vec::new();
        match std::fs::read_dir(dir) {
            Ok(read_dir) => {
                for entry in read_dir.flatten() {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if name.starts_with('.') {
                        continue;
                    }
                    let meta = entry.metadata().ok();
                    let is_dir = meta.as_ref().is_some_and(|m| m.is_dir());
                    let bytes = meta.as_ref().map_or(0, |m| m.len());
                    let report = if !is_dir && bytes <= 512 * 1024 {
                        match inspect_path(&path, Some(catalog)) {
                            Ok(report) => Some(report),
                            Err(err) => {
                                log::debug!(
                                    "skipping file inspection for '{}': {err:#}",
                                    path.display()
                                );
                                None
                            }
                        }
                    } else {
                        None
                    };
                    entries.push(FileRow {
                        path,
                        name,
                        is_dir,
                        bytes,
                        report,
                    });
                    if entries.len() >= 24 {
                        break;
                    }
                }
            }
            Err(err) => {
                log::warn!("failed to list directory '{}': {err}", dir.display());
            }
        }
        entries.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        entries
    }

    pub fn apply_scan_results(&mut self, dir: PathBuf, entries: Vec<FileRow>) {
        self.current_dir = dir;
        self.entries = entries;
        self.selected_idx = (!self.entries.is_empty()).then_some(0);
    }

    pub fn rescan(&mut self, dir: &PathBuf, catalog: &CraftCatalog) {
        let entries = Self::scan_directory(dir, catalog);
        self.apply_scan_results(dir.clone(), entries);
    }
}

pub fn render_files_window(
    files: &FilesState,
    theme: &OsTheme,
    on_select: Arc<dyn Fn(usize, &mut Window, &mut App) + Send + Sync>,
    on_open_in_studio: Arc<dyn Fn(PathBuf, &mut Window, &mut App) + Send + Sync>,
    on_up_dir: Arc<dyn Fn(&mut Window, &mut App) + Send + Sync>,
) -> AnyElement {
    let selected = files.selected_idx.and_then(|idx| files.entries.get(idx));

    let mut rows_col = v_flex().w_full().gap_1();
    for (idx, row) in files.entries.iter().enumerate() {
        let is_sel = files.selected_idx == Some(idx);
        let on_sel = Arc::clone(&on_select);
        let badge = if row.is_dir {
            Some(SharedString::from("DIR"))
        } else {
            row.report
                .as_ref()
                .map(|r| SharedString::from(r.format_name.clone()))
        };
        let icon = if row.is_dir {
            IconName::FolderOpen
        } else {
            IconName::FileGeneric
        };
        rows_col = rows_col.child(focusable_action_chip(
            ElementId::Name(format!("file-row-{idx}").into()),
            icon,
            SharedString::from(row.name.clone()),
            badge,
            SharedString::from(row.path.display().to_string()),
            is_sel,
            (300 + idx) as isize,
            theme,
            move |_, window, cx| on_sel(idx, window, cx),
        ));
    }

    let detail = if let Some(sel) = selected {
        let mut ui = ImmediateUi::new(theme);
        ui.heading(sel.name.clone());
        ui.kv_row("Bytes", format!("{} B", sel.bytes));
        if let Some(report) = &sel.report {
            ui.kv_row("Format", report.format_name.clone());
            ui.kv_row("MIME", report.mime_type.clone());
            ui.kv_row("Entropy", format!("{:.3}", report.shannon_entropy));
            if let Some(app) = &report.recommended_app {
                ui.kv_row("Recommended", app.clone());
            }
        }
        if !sel.is_dir {
            let path = sel.path.clone();
            let open_cb = Arc::clone(&on_open_in_studio);
            ui.button(
                "files-open-in-studio",
                IconName::ArrowUpRight,
                "Open in Studio",
                true,
                move |_, window, cx| open_cb(path.clone(), window, cx),
            );
        }
        ui.finish()
    } else {
        Label::new("Empty folder")
            .size(LabelSize::Small)
            .into_any_element()
    };

    v_flex()
        .size_full()
        .p_3()
        .gap_2()
        .bg(theme.colors.surface)
        .child(
            h_flex()
                .w_full()
                .min_w_0()
                .justify_between()
                .items_center()
                .gap_2()
                .child(
                    div().flex_1().min_w_0().overflow_hidden().child(
                        Label::new(files.current_dir.display().to_string())
                            .size(LabelSize::XSmall)
                            .color(ui::Color::Muted)
                            .truncate_middle(),
                    ),
                )
                .child(focusable_action_chip(
                    "files-up-dir",
                    IconName::ArrowUp,
                    SharedString::from("Up"),
                    None,
                    SharedString::from("Go to parent directory"),
                    false,
                    299,
                    theme,
                    move |_, window, cx| on_up_dir(window, cx),
                )),
        )
        .child(
            h_flex()
                .flex_1()
                .w_full()
                .min_w_0()
                .min_h(px(0.0))
                .gap_3()
                .child(
                    v_flex()
                        .id("files-list-scroll")
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .overflow_y_scroll()
                        .child(rows_col),
                )
                .child(
                    v_flex()
                        .id("files-detail-scroll")
                        .flex_shrink_0()
                        .w(rems_from_px(230.0_f32))
                        .h_full()
                        .p_2()
                        .pb_4()
                        .overflow_y_scroll()
                        .rounded(rems_from_px(8.0_f32))
                        .bg(theme.colors.surface_container)
                        .border_1()
                        .border_color(theme.colors.outline_variant)
                        .child(detail),
                ),
        )
        .into_any_element()
}

pub fn render_monitor_window(
    supervisor: &SessionSupervisor,
    hardware: &HardwareCapabilities,
    frame_samples: &[f32],
    theme: &OsTheme,
    on_activate_session: Arc<dyn Fn(&mut Window, &mut App) + Send + Sync>,
) -> AnyElement {
    let snapshot = supervisor.snapshot();
    let env_info = SessionEnvironment::detect(&supervisor.manifest);
    let dist = DistBundleSpec::from_brand(env!("CARGO_PKG_VERSION"));

    let mut ui = ImmediateUi::new(theme);
    ui.heading("Session Supervisor & 120 FPS Hardware Profile");
    ui.button(
        "monitor-activate-session",
        IconName::PlayFilled,
        "Reconcile Session Units",
        false,
        move |_, window, cx| on_activate_session(window, cx),
    );

    let running = snapshot
        .iter()
        .filter(|u| u.phase == UnitPhase::Running)
        .count();
    let total = snapshot.len().max(1);
    ui.push_element(progress_meter(
        format!("Active units ({running}/{total})"),
        running as f32 / total as f32,
        theme,
    ));

    ui.section("Adaptive Hardware, CPU/GPU & 120 FPS", |sec| {
        sec.kv_row(
            "Target OS / Arch",
            format!("{} / {}", hardware.os, hardware.arch),
        );
        sec.kv_row(
            "CPU Threads / Jobs",
            format!(
                "{} cores ({} parallel)",
                hardware.cpu_threads, hardware.recommended_build_jobs
            ),
        );
        sec.kv_row(
            "Frame Budget",
            format!(
                "{} FPS ({:.2} ms)",
                hardware.target_fps, hardware.frame_budget_ms
            ),
        );
        sec.kv_row("GPU Backends", hardware.gpu_backend_hint.clone());
        sec.kv_row("Present Mode", hardware.present_mode_hint.clone());
        let tools = if hardware.detected_toolchains.is_empty() {
            "none detected".to_owned()
        } else {
            hardware.detected_toolchains.join(", ")
        };
        sec.kv_row("Toolchains", tools);
    });

    ui.section("Environment & Pop!_OS Dist Bundle", |sec| {
        let session_type = env_info
            .vars
            .get("XDG_SESSION_TYPE")
            .map_or("gpui-native", String::as_str);
        let display_name = env_info
            .vars
            .get("WAYLAND_DISPLAY")
            .or_else(|| env_info.vars.get("DISPLAY"))
            .map_or("native-gpu", String::as_str);
        sec.kv_row("Platform", std::env::consts::OS);
        sec.kv_row("Session", format!("{session_type} ({display_name})"));
        sec.kv_row("Package", format!("{} v{}", dist.app_slug, dist.version));
        sec.kv_row("Format Spec", "deb / flatpak / appimage / desktop");
    });

    let unified = UnifiedPlatformProfile::detect();
    let layer_list = unified
        .active_layers
        .iter()
        .map(|layer| layer.slug())
        .collect::<Vec<_>>()
        .join(" · ");
    ui.section("Unified Stack (WebGPU / Desktop / Mobile)", |sec| {
        sec.kv_row("Target Family", unified.target_family.clone());
        sec.kv_row(
            "Surface Cohorts",
            format!(
                "webgpu:{} desktop:{} mobile:{}",
                unified.wasm_webgpu_ready, unified.desktop_ready, unified.mobile_ready
            ),
        );
        sec.kv_row(
            format!("Layers ({})", unified.active_layers.len()),
            layer_list,
        );
    });

    ui.section("Supervised Units", |sec| {
        for unit in &snapshot {
            let (icon, text, tone) = match unit.phase {
                UnitPhase::Running => (IconName::Check, "RUNNING", StatusTone::Success),
                UnitPhase::Starting => (IconName::RefreshTitle, "STARTING", StatusTone::Accent),
                UnitPhase::Degraded => (IconName::Warning, "DEGRADED", StatusTone::Warning),
                UnitPhase::Failed => (IconName::Warning, "FAILED", StatusTone::Error),
                UnitPhase::Stopped => (IconName::Dash, "STOPPED", StatusTone::Neutral),
            };
            sec.push_element(
                h_flex()
                    .w_full()
                    .min_w_0()
                    .justify_between()
                    .items_center()
                    .gap_2()
                    .child(
                        div().flex_1().min_w_0().overflow_hidden().child(
                            Label::new(unit.config.id.clone())
                                .size(LabelSize::XSmall)
                                .truncate(),
                        ),
                    )
                    .child(status_badge(icon, text, tone, sec.theme))
                    .into_any_element(),
            );
        }
    });

    ui.push_element(sparkline_chart(
        "Compositor Frame Budget",
        "ms",
        frame_samples,
        theme,
    ));

    v_flex()
        .id("monitor-scroll")
        .size_full()
        .p_3()
        .pb_6()
        .overflow_y_scroll()
        .bg(theme.colors.surface)
        .child(ui.finish())
        .into_any_element()
}
