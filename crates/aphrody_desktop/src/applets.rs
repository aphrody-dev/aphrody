use std::sync::Arc;

use gpui::{App, ElementId, SharedString, Window, div, prelude::*};
use aphrody_session::{TilingMode, WorkspaceId};
use aphrody_ui::{
    AdaptiveViewport, AppearanceMode, AppletSlot, OsTheme, PanelApplet, StatusTone,
    focusable_action_chip, status_badge,
};
use ui::{IconName, Label, LabelCommon as _, LabelSize, h_flex, rems_from_px};

#[derive(Debug, Clone)]
pub struct WorkspacePill {
    pub id: WorkspaceId,
    pub name: SharedString,
    pub window_count: usize,
}

type WorkspaceCallback = Arc<dyn Fn(WorkspaceId, &mut Window, &mut App) + Send + Sync>;
type ActionCallback = Arc<dyn Fn(&mut Window, &mut App) + Send + Sync>;

/// Left-slot COSMIC TopPanel applet rendering live workspaces and the active tiling mode switcher.
pub struct WorkspaceApplet {
    pub workspaces: Vec<WorkspacePill>,
    pub active_workspace: WorkspaceId,
    pub mode: TilingMode,
    pub on_select_workspace: WorkspaceCallback,
    pub on_cycle_mode: ActionCallback,
}

impl PanelApplet for WorkspaceApplet {
    fn id(&self) -> &'static str {
        "os.panel.workspaces"
    }

    fn title(&self) -> SharedString {
        SharedString::from("Workspaces")
    }

    fn slot(&self) -> AppletSlot {
        AppletSlot::Left
    }

    fn order(&self) -> i32 {
        0
    }

    fn render_pill(&self, theme: &OsTheme, window: &mut Window, _cx: &mut App) -> gpui::AnyElement {
        let vp_size = window.viewport_size();
        let vp = AdaptiveViewport::from_size(f32::from(vp_size.width), f32::from(vp_size.height));
        let narrow = vp.narrow_panel_labels;
        let ultra_narrow = vp.icon_only_panel_controls;
        let mut bar = h_flex().gap_1().items_center();

        for (idx, ws) in self.workspaces.iter().enumerate() {
            let selected = ws.id == self.active_workspace;
            let ws_id = ws.id;
            let on_select = Arc::clone(&self.on_select_workspace);
            let badge = (!ultra_narrow && ws.window_count > 0)
                .then(|| SharedString::from(ws.window_count.to_string()));
            let label = if ultra_narrow && !selected {
                SharedString::from(format!("{ws_id}"))
            } else if narrow {
                SharedString::from(format!("W{ws_id}"))
            } else {
                SharedString::from(format!("{} {}", ws.id, ws.name))
            };
            bar = bar.child(focusable_action_chip(
                ElementId::Name(format!("panel-ws-{ws_id}").into()),
                IconName::Screen,
                label,
                badge,
                SharedString::from(format!("Workspace {} {}", ws.id, ws.name)),
                selected,
                (idx + 1) as isize,
                theme,
                move |_, window, cx| on_select(ws_id, window, cx),
            ));
        }

        let on_cycle = Arc::clone(&self.on_cycle_mode);
        let mode_label = if ultra_narrow {
            SharedString::default()
        } else {
            SharedString::from(self.mode.label())
        };
        bar = bar.child(focusable_action_chip(
            "panel-cycle-layout",
            IconName::Split,
            mode_label,
            None,
            SharedString::from(format!(
                "Cycle compositor tiling mode ({})",
                self.mode.label()
            )),
            false,
            10,
            theme,
            move |_, window, cx| on_cycle(window, cx),
        ));

        bar.into_any_element()
    }
}

/// Center-slot COSMIC TopPanel applet showing the OS brand pill, active Studio app, and Ctrl+K Launcher button.
pub struct BrandLauncherApplet {
    pub brand_name: SharedString,
    pub active_app: SharedString,
    pub theme_name: SharedString,
    pub launcher_open: bool,
    pub on_toggle_launcher: ActionCallback,
}

impl PanelApplet for BrandLauncherApplet {
    fn id(&self) -> &'static str {
        "os.panel.brand_launcher"
    }

    fn title(&self) -> SharedString {
        SharedString::from("Brand & Launcher")
    }

    fn slot(&self) -> AppletSlot {
        AppletSlot::Center
    }

    fn order(&self) -> i32 {
        0
    }

    fn render_pill(&self, theme: &OsTheme, window: &mut Window, _cx: &mut App) -> gpui::AnyElement {
        let vp_size = window.viewport_size();
        let vp = AdaptiveViewport::from_size(f32::from(vp_size.width), f32::from(vp_size.height));
        let narrow = vp.narrow_panel_labels;
        let ultra_narrow = vp.icon_only_panel_controls;
        let on_toggle = Arc::clone(&self.on_toggle_launcher);

        let title_pill = (!ultra_narrow).then(|| {
            let text = if narrow {
                self.active_app.to_string()
            } else {
                format!("{} OS · {}", self.brand_name, self.active_app)
            };
            div()
                .min_w_0()
                .overflow_hidden()
                .px_2p5()
                .py_1()
                .rounded(rems_from_px(8.0_f32))
                .bg(theme.colors.surface_container_high)
                .border_1()
                .border_color(theme.colors.outline_variant)
                .child(Label::new(text).size(LabelSize::Small).truncate())
        });

        let compact = f32::from(vp_size.width) < 1520.0;
        let shortcut_badge = (!narrow && !compact).then(|| SharedString::from("Ctrl+K"));
        let theme_chip = (!narrow).then(|| {
            div()
                .flex_shrink_0()
                .px_2()
                .py_0p5()
                .rounded(rems_from_px(6.0_f32))
                .bg(theme.colors.surface_container)
                .child(
                    Label::new(self.theme_name.clone())
                        .size(LabelSize::XSmall)
                        .color(ui::Color::Muted)
                        .single_line(),
                )
        });

        h_flex()
            .gap_1p5()
            .items_center()
            .children(title_pill)
            .child(focusable_action_chip(
                "panel-open-launcher",
                IconName::Command,
                SharedString::from("Apps"),
                shortcut_badge,
                SharedString::from("Toggle unified application launcher"),
                self.launcher_open,
                15,
                theme,
                move |_, window, cx| on_toggle(window, cx),
            ))
            .children(theme_chip)
            .into_any_element()
    }
}

/// Right-slot COSMIC TopPanel applet showing active session units, Light/Dark mode switch, and Inspector Drawer toggle.
pub struct SystemTelemetryApplet {
    pub mode: AppearanceMode,
    pub unit_running: usize,
    pub unit_total: usize,
    pub target_fps: u32,
    pub latest_frame_ms: f32,
    pub drawer_open: bool,
    pub on_toggle_mode: ActionCallback,
    pub on_toggle_drawer: ActionCallback,
}

impl PanelApplet for SystemTelemetryApplet {
    fn id(&self) -> &'static str {
        "os.panel.telemetry"
    }

    fn title(&self) -> SharedString {
        SharedString::from("Session & Theme")
    }

    fn slot(&self) -> AppletSlot {
        AppletSlot::Right
    }

    fn order(&self) -> i32 {
        0
    }

    fn render_pill(&self, theme: &OsTheme, window: &mut Window, _cx: &mut App) -> gpui::AnyElement {
        let vp_size = window.viewport_size();
        let vp = AdaptiveViewport::from_size(f32::from(vp_size.width), f32::from(vp_size.height));
        let narrow = vp.narrow_panel_labels;
        let ultra_narrow = vp.icon_only_panel_controls;

        let mode_label = if ultra_narrow {
            SharedString::default()
        } else {
            SharedString::from(match self.mode {
                AppearanceMode::Light => "Light",
                AppearanceMode::Dark => "Dark",
            })
        };
        let unit_text = if ultra_narrow {
            format!("{}Hz", self.target_fps)
        } else if narrow {
            format!(
                "{}/{} · {}Hz",
                self.unit_running, self.unit_total, self.target_fps
            )
        } else {
            format!(
                "{}/{} · {}Hz · {:.1}ms",
                self.unit_running, self.unit_total, self.target_fps, self.latest_frame_ms
            )
        };
        let unit_tone = if self.unit_running == self.unit_total {
            StatusTone::Success
        } else {
            StatusTone::Warning
        };
        let on_mode = Arc::clone(&self.on_toggle_mode);
        let on_drawer = Arc::clone(&self.on_toggle_drawer);

        h_flex()
            .gap_1p5()
            .items_center()
            .child(status_badge(IconName::Server, unit_text, unit_tone, theme))
            .child(focusable_action_chip(
                "panel-toggle-mode",
                IconName::Sparkle,
                mode_label,
                if f32::from(vp_size.width) >= 1520.0 {
                    Some(SharedString::from("Ctrl+Shift+T"))
                } else {
                    None
                },
                SharedString::from("Switch between Dark and Light OS theme polarity"),
                self.mode == AppearanceMode::Light,
                20,
                theme,
                move |_, window, cx| on_mode(window, cx),
            ))
            .child(focusable_action_chip(
                "panel-toggle-drawer",
                IconName::ListTree,
                if ultra_narrow {
                    SharedString::default()
                } else {
                    SharedString::from("Inspector")
                },
                None,
                SharedString::from("Toggle contextual M3 token and session drawer"),
                self.drawer_open,
                21,
                theme,
                move |_, window, cx| on_drawer(window, cx),
            ))
            .into_any_element()
    }
}
