use std::sync::Arc;

use gpui::{App, ElementId, SharedString, Window, div, prelude::*, px};
use os_session::{TilingMode, WorkspaceId};
use os_ui::{
    AppearanceMode, AppletSlot, OsTheme, PanelApplet, StatusTone, focusable_action_chip,
    status_badge,
};
use ui::{IconName, Label, LabelCommon as _, LabelSize, h_flex};

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

    fn render_pill(
        &self,
        theme: &OsTheme,
        _window: &mut Window,
        _cx: &mut App,
    ) -> gpui::AnyElement {
        let mut bar = h_flex().gap_1().items_center();

        for (idx, ws) in self.workspaces.iter().enumerate() {
            let selected = ws.id == self.active_workspace;
            let ws_id = ws.id;
            let on_select = Arc::clone(&self.on_select_workspace);
            let badge =
                (ws.window_count > 0).then(|| SharedString::from(ws.window_count.to_string()));
            bar = bar.child(focusable_action_chip(
                ElementId::Name(format!("panel-ws-{ws_id}").into()),
                IconName::Screen,
                SharedString::from(format!("{} {}", ws.id, ws.name)),
                badge,
                SharedString::from(format!("Workspace {} {}", ws.id, ws.name)),
                selected,
                (idx + 1) as isize,
                theme,
                move |_, window, cx| on_select(ws_id, window, cx),
            ));
        }

        let on_cycle = Arc::clone(&self.on_cycle_mode);
        bar = bar.child(focusable_action_chip(
            "panel-cycle-layout",
            IconName::Split,
            SharedString::from(self.mode.label()),
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

    fn render_pill(
        &self,
        theme: &OsTheme,
        _window: &mut Window,
        _cx: &mut App,
    ) -> gpui::AnyElement {
        let on_toggle = Arc::clone(&self.on_toggle_launcher);
        h_flex()
            .gap_2()
            .items_center()
            .child(
                div()
                    .px_2p5()
                    .py_1()
                    .rounded(px(8.0))
                    .bg(theme.colors.surface_container_high)
                    .border_1()
                    .border_color(theme.colors.outline_variant)
                    .child(
                        Label::new(format!("{} OS · {}", self.brand_name, self.active_app))
                            .size(LabelSize::Small),
                    ),
            )
            .child(focusable_action_chip(
                "panel-open-launcher",
                IconName::Command,
                SharedString::from("Apps"),
                Some(SharedString::from("Ctrl+K")),
                SharedString::from("Toggle unified ArtCraft and system app launcher"),
                self.launcher_open,
                15,
                theme,
                move |_, window, cx| on_toggle(window, cx),
            ))
            .child(
                div()
                    .px_2()
                    .py_0p5()
                    .rounded(px(6.0))
                    .bg(theme.colors.surface_container)
                    .child(
                        Label::new(self.theme_name.clone())
                            .size(LabelSize::XSmall)
                            .color(ui::Color::Muted),
                    ),
            )
            .into_any_element()
    }
}

/// Right-slot COSMIC TopPanel applet showing active session units, Light/Dark mode switch, and Inspector Drawer toggle.
pub struct SystemTelemetryApplet {
    pub mode: AppearanceMode,
    pub unit_running: usize,
    pub unit_total: usize,
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

    fn render_pill(
        &self,
        theme: &OsTheme,
        _window: &mut Window,
        _cx: &mut App,
    ) -> gpui::AnyElement {
        let mode_label = match self.mode {
            AppearanceMode::Light => "Light",
            AppearanceMode::Dark => "Dark",
        };
        let unit_text = format!(
            "{}/{} units · {:.1}ms",
            self.unit_running, self.unit_total, self.latest_frame_ms
        );
        let unit_tone = if self.unit_running == self.unit_total {
            StatusTone::Success
        } else {
            StatusTone::Warning
        };
        let on_mode = Arc::clone(&self.on_toggle_mode);
        let on_drawer = Arc::clone(&self.on_toggle_drawer);

        h_flex()
            .gap_2()
            .items_center()
            .child(status_badge(IconName::Server, unit_text, unit_tone, theme))
            .child(focusable_action_chip(
                "panel-toggle-mode",
                IconName::Sparkle,
                SharedString::from(mode_label),
                Some(SharedString::from("Ctrl+Shift+T")),
                SharedString::from("Switch between Dark and Light OS theme polarity"),
                self.mode == AppearanceMode::Light,
                20,
                theme,
                move |_, window, cx| on_mode(window, cx),
            ))
            .child(focusable_action_chip(
                "panel-toggle-drawer",
                IconName::ListTree,
                SharedString::from("Inspector"),
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
