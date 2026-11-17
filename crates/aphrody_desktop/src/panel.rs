//! COSMIC Epoch `cosmic-panel` Top Bar & Floating Dock cleanly decoupled from shell orchestration.

use aphrody_session::TilingMode;
use aphrody_ui::{OsTheme, focusable_action_chip};
use gpui::{AnyElement, App, FontWeight, SharedString, div, prelude::*, px};
use ui::{IconName, Label, LabelCommon as _, LabelSize, h_flex};

use crate::apps::WindowKind;

pub struct DockCallbacks {
    pub on_open_window: Box<dyn Fn(WindowKind, &mut App) + 'static>,
    pub on_select_craft: Box<dyn Fn(String, &mut App) + 'static>,
    pub on_toggle_launcher: Box<dyn Fn(&mut App) + 'static>,
    pub on_toggle_overview: Box<dyn Fn(&mut App) + 'static>,
    pub on_toggle_screenshot: Box<dyn Fn(&mut App) + 'static>,
    pub on_toggle_lock: Box<dyn Fn(&mut App) + 'static>,
}

/// Renders the `cosmic-panel` floating bottom dock across all 8 COSMIC/Aphrody native apps
/// plus Overview, App Library/Launcher, Screenshot, and Greeter Lock triggers.
pub fn render_cosmic_dock(
    compact_width: bool,
    theme: &OsTheme,
    focused_kind: Option<WindowKind>,
    overview_open: bool,
    launcher_open: bool,
    screenshot_open: bool,
    dock_h: f32,
    cb: DockCallbacks,
) -> AnyElement {
    let on_open = std::rc::Rc::new(cb.on_open_window);
    let on_overview = std::rc::Rc::new(cb.on_toggle_overview);
    let on_launcher = std::rc::Rc::new(cb.on_toggle_launcher);
    let on_shot = std::rc::Rc::new(cb.on_toggle_screenshot);
    let on_lock = std::rc::Rc::new(cb.on_toggle_lock);

    let mut dock_row = h_flex()
        .h(px(dock_h))
        .px_2p5()
        .gap_1p5()
        .items_center()
        .rounded(px(theme.layout.window_radius + 4.0))
        .border_1()
        .border_color(theme.colors.outline_variant)
        .bg(theme.colors.surface_container.opacity(0.92));

    let items: &[(WindowKind, &str)] = if compact_width {
        &[
            (WindowKind::Studio, "Studio"),
            (WindowKind::Term, "Term"),
            (WindowKind::Files, "Files"),
            (WindowKind::Settings, "Settings"),
            (WindowKind::Store, "Store"),
            (WindowKind::Installer, "DE"),
        ]
    } else {
        &[
            (WindowKind::Studio, "Studio"),
            (WindowKind::Files, "Files"),
            (WindowKind::Settings, "Settings"),
            (WindowKind::Monitor, "Monitor"),
            (WindowKind::Term, "Term"),
            (WindowKind::Edit, "Edit"),
            (WindowKind::Store, "Store"),
            (WindowKind::Installer, "Installer"),
        ]
    };

    for (idx, &(kind, short_label)) in items.iter().enumerate() {
        let cb_open = std::rc::Rc::clone(&on_open);
        let active = focused_kind == Some(kind);
        dock_row = dock_row.child(focusable_action_chip(
            gpui::ElementId::Name(format!("dock-{}", kind.app_id()).into()),
            kind.icon(),
            if compact_width {
                SharedString::default()
            } else {
                SharedString::from(short_label)
            },
            None,
            SharedString::from(kind.default_title()),
            active,
            (40 + idx) as isize,
            theme,
            move |_ev, _w, cx| cb_open(kind, cx),
        ));
    }

    dock_row = dock_row
        .child(
            div()
                .w(px(1.0))
                .h(px(22.0))
                .mx_1()
                .bg(theme.colors.outline_variant),
        )
        .child(focusable_action_chip(
            "dock-workspaces-overview",
            IconName::Server,
            if compact_width { "" } else { "Workspaces" },
            None,
            "Toggle COSMIC Workspaces Overview",
            overview_open,
            55,
            theme,
            move |_ev, _w, cx| on_overview(cx),
        ))
        .child(focusable_action_chip(
            "dock-app-library",
            IconName::MagnifyingGlass,
            if compact_width { "" } else { "Library" },
            None,
            "Toggle Pop Launcher & COSMIC App Library",
            launcher_open,
            56,
            theme,
            move |_ev, _w, cx| on_launcher(cx),
        ))
        .child(focusable_action_chip(
            "dock-screenshot",
            IconName::Image,
            if compact_width { "" } else { "Shot" },
            None,
            "Toggle COSMIC Screenshot Utility",
            screenshot_open,
            57,
            theme,
            move |_ev, _w, cx| on_shot(cx),
        ))
        .child(focusable_action_chip(
            "dock-lock-greeter",
            IconName::Lock,
            if compact_width { "" } else { "Lock" },
            None,
            "Lock Session (COSMIC Greeter)",
            false,
            58,
            theme,
            move |_ev, _w, cx| on_lock(cx),
        ));

    h_flex()
        .w_full()
        .justify_center()
        .py_1()
        .child(dock_row)
        .into_any_element()
}

/// Renders compact top status badge strip for COSMIC active hint + tiling + active workspace.
#[must_use]
pub fn render_cosmic_status_strip(
    _theme: &OsTheme,
    tiling: TilingMode,
    power_label: &str,
    stack_count: usize,
) -> AnyElement {
    h_flex()
        .gap_2()
        .items_center()
        .child(
            Label::new(format!(
                "{} · {} · {} stack(s)",
                tiling.label(),
                power_label,
                stack_count
            ))
            .size(LabelSize::XSmall)
            .weight(FontWeight::MEDIUM)
            .color(ui::Color::Muted),
        )
        .into_any_element()
}
