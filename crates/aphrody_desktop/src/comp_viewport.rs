//! COSMIC Compositor (`cosmic-comp`) Viewport, Pop Shell Stacking Tabs & Active Hint,
//! `cosmic-bg` gradient backdrop, and `cosmic-workspaces-epoch` Live Overview Grid.

use std::rc::Rc;

use aphrody_session::{CompositorState, PlacedWindow, StackId, TilingMode, ToplevelId, WorkspaceId};
use aphrody_ui::{OsTheme, StatusTone, status_badge};
use gpui::{AnyElement, App, FontWeight, Window, div, prelude::*, px};
use ui::{Icon, IconName, IconSize, Label, LabelCommon as _, LabelSize, h_flex, v_flex};

#[derive(Clone)]
pub struct ViewportWindowCallbacks {
    pub on_focus: Rc<dyn Fn(ToplevelId, &mut Window, &mut App) + 'static>,
    pub on_toggle_maximize: Rc<dyn Fn(ToplevelId, &mut Window, &mut App) + 'static>,
    pub on_toggle_floating: Rc<dyn Fn(ToplevelId, &mut Window, &mut App) + 'static>,
    pub on_stack_window: Rc<dyn Fn(ToplevelId, &mut Window, &mut App) + 'static>,
    pub on_select_stack_tab: Rc<dyn Fn(StackId, ToplevelId, &mut Window, &mut App) + 'static>,
    pub on_unstack_window: Rc<dyn Fn(ToplevelId, &mut Window, &mut App) + 'static>,
    pub on_close_window: Rc<dyn Fn(ToplevelId, &mut Window, &mut App) + 'static>,
}

/// Renders one composited tiled/floating/stacked window frame with Pop Shell active-hint ring.
pub fn render_composited_window_frame(
    _idx: usize,
    placed: &PlacedWindow,
    compositor: &CompositorState,
    origin_x: f32,
    origin_y: f32,
    top_bar_h: f32,
    theme: &OsTheme,
    body: AnyElement,
    cb: &ViewportWindowCallbacks,
) -> AnyElement {
    let win_id = placed.id;
    let hint_enabled = compositor.active_hint.enabled && placed.focused;
    let border_w = if hint_enabled {
        compositor.active_hint.width_px.max(1.5)
    } else {
        1.0
    };
    let border_color = if hint_enabled {
        theme.colors.primary
    } else {
        theme.colors.outline_variant
    };

    let active_stack = compositor.active_stack_for(win_id);
    let has_stack_tabs = compositor.active_hint.show_stack_tabs && active_stack.is_some();

    let mut stack_strip = h_flex().gap_1().px_2().py_1().bg(theme.colors.surface_container_low);
    if let Some(stack) = active_stack {
        let stack_id = stack.id;
        for (tab_idx, &tid) in stack.tabs.iter().enumerate() {
            let tab_title = compositor
                .toplevels
                .get(&tid)
                .map(|t| t.title.clone())
                .unwrap_or_else(|| format!("Window {tid}"));
            let active = stack.active_toplevel() == Some(tid);
            let on_sel = Rc::clone(&cb.on_select_stack_tab);
            stack_strip = stack_strip.child(
                div()
                    .id(("pop-stack-tab", win_id as usize * 32 + tab_idx))
                    .cursor_pointer()
                    .px_2()
                    .py_0p5()
                    .rounded(px(6.0))
                    .bg(if active {
                        theme.colors.primary
                    } else {
                        theme.colors.surface_container_high
                    })
                    .on_click(move |_ev, window, cx| on_sel(stack_id, tid, window, cx))
                    .child(
                        Label::new(tab_title)
                            .size(LabelSize::XSmall)
                            .color(ui::Color::Custom(if active {
                                theme.colors.on_primary
                            } else {
                                theme.colors.on_surface
                            })),
                    ),
            );
        }
        let on_unstack = Rc::clone(&cb.on_unstack_window);
        stack_strip = stack_strip.child(
            div()
                .id(("pop-stack-unstack", win_id as usize))
                .cursor_pointer()
                .px_1p5()
                .py_0p5()
                .rounded(px(6.0))
                .bg(theme.colors.surface_container_high)
                .on_click(move |_ev, window, cx| on_unstack(win_id, window, cx))
                .child(Label::new("Unstack").size(LabelSize::XSmall)),
        );
    }

    let on_focus = Rc::clone(&cb.on_focus);
    let on_stack = Rc::clone(&cb.on_stack_window);
    let on_float = Rc::clone(&cb.on_toggle_floating);
    let on_max = Rc::clone(&cb.on_toggle_maximize);
    let on_close = Rc::clone(&cb.on_close_window);

    let title_row = h_flex()
        .id(("cosmic-win-bar", win_id as usize))
        .h(px(32.0))
        .px_2p5()
        .justify_between()
        .items_center()
        .bg(if placed.focused {
            theme.colors.surface_container_high
        } else {
            theme.colors.surface_container
        })
        .border_b_1()
        .border_color(theme.colors.outline_variant)
        .on_click(move |_ev, window, cx| on_focus(win_id, window, cx))
        .child(
            h_flex()
                .gap_1p5()
                .items_center()
                .child(Icon::new(IconName::Settings).size(IconSize::XSmall))
                .child(
                    Label::new(placed.title.clone())
                        .size(LabelSize::XSmall)
                        .weight(FontWeight::SEMIBOLD),
                )
                .when(placed.floating, |el| {
                    el.child(status_badge(
                        IconName::Image,
                        "Float",
                        StatusTone::Accent,
                        theme,
                    ))
                }),
        )
        .child(
            h_flex()
                .gap_1()
                .items_center()
                .child(
                    div()
                        .id(("win-stack", win_id as usize))
                        .cursor_pointer()
                        .px_1p5()
                        .rounded(px(4.0))
                        .bg(theme.colors.surface_container)
                        .on_click(move |_ev, window, cx| on_stack(win_id, window, cx))
                        .child(Label::new("Stack").size(LabelSize::XSmall)),
                )
                .child(
                    div()
                        .id(("win-float", win_id as usize))
                        .cursor_pointer()
                        .px_1p5()
                        .rounded(px(4.0))
                        .bg(theme.colors.surface_container)
                        .on_click(move |_ev, window, cx| on_float(win_id, window, cx))
                        .child(Label::new("Float").size(LabelSize::XSmall)),
                )
                .child(
                    div()
                        .id(("win-max", win_id as usize))
                        .cursor_pointer()
                        .px_1p5()
                        .rounded(px(4.0))
                        .bg(theme.colors.surface_container)
                        .on_click(move |_ev, window, cx| on_max(win_id, window, cx))
                        .child(Label::new(if placed.maximized { "Restore" } else { "Max" }).size(LabelSize::XSmall)),
                )
                .child(
                    div()
                        .id(("win-close", win_id as usize))
                        .cursor_pointer()
                        .px_1p5()
                        .rounded(px(4.0))
                        .bg(theme.colors.surface_container)
                        .on_click(move |_ev, window, cx| on_close(win_id, window, cx))
                        .child(Label::new("×").size(LabelSize::XSmall)),
                ),
        );

    let mut frame = v_flex()
        .id(("cosmic-comp-win", win_id as usize))
        .absolute()
        .left(px(placed.rect.x - origin_x))
        .top(px(placed.rect.y - origin_y - top_bar_h))
        .w(px(placed.rect.w.max(120.0)))
        .h(px(placed.rect.h.max(90.0)))
        .rounded(px(theme.layout.window_radius))
        .border(px(border_w))
        .border_color(border_color)
        .bg(theme.colors.surface_container)
        .overflow_hidden()
        .child(title_row);

    if has_stack_tabs {
        frame = frame.child(stack_strip);
    }

    frame
        .child(div().flex_1().min_h_0().p_2().overflow_hidden().child(body))
        .into_any_element()
}

/// Renders `cosmic-workspaces-epoch` live interactive workspace overview grid.
pub fn render_workspaces_overview(
    compositor: &CompositorState,
    theme: &OsTheme,
    on_select_workspace: Box<dyn Fn(WorkspaceId, &mut App) + 'static>,
    on_close_overview: Box<dyn Fn(&mut App) + 'static>,
    on_set_mode: Box<dyn Fn(TilingMode, &mut App) + 'static>,
) -> AnyElement {
    let on_ws = Rc::new(on_select_workspace);
    let on_close = Rc::new(on_close_overview);
    let on_mode = Rc::new(on_set_mode);

    let overviews = compositor.workspace_overviews();
    let mut cards = h_flex().gap_3().flex_wrap().justify_center();

    for ws in overviews {
        let wid = ws.workspace_id;
        let ws_cb = Rc::clone(&on_ws);
        let close_cb = Rc::clone(&on_close);
        let mut mini_wins = v_flex().gap_1().flex_1();
        for p in ws.placed.iter().take(4) {
            mini_wins = mini_wins.child(
                div()
                    .px_2()
                    .py_1()
                    .rounded(px(6.0))
                    .bg(theme.colors.surface_container_high)
                    .child(Label::new(p.title.clone()).size(LabelSize::XSmall)),
            );
        }
        if ws.placed.is_empty() {
            mini_wins = mini_wins.child(
                Label::new("Empty workspace")
                    .size(LabelSize::XSmall)
                    .color(ui::Color::Muted),
            );
        }

        cards = cards.child(
            v_flex()
                .id(("cosmic-overview-ws", wid as usize))
                .cursor_pointer()
                .w(px(260.0))
                .h(px(176.0))
                .p_3()
                .gap_2()
                .rounded(px(theme.layout.window_radius))
                .border(px(if ws.active { 2.0 } else { 1.0 }))
                .border_color(if ws.active {
                    theme.colors.primary
                } else {
                    theme.colors.outline_variant
                })
                .bg(if ws.active {
                    theme.colors.primary_container
                } else {
                    theme.colors.surface_container
                })
                .on_click(move |_ev, _w, cx| {
                    ws_cb(wid, cx);
                    close_cb(cx);
                })
                .child(
                    h_flex()
                        .justify_between()
                        .child(
                            Label::new(format!("{} · {}", ws.workspace_id, ws.name))
                                .size(LabelSize::Small)
                                .weight(FontWeight::SEMIBOLD),
                        )
                        .child(status_badge(
                            IconName::Server,
                            ws.mode.label(),
                            if ws.active {
                                StatusTone::Success
                            } else {
                                StatusTone::Neutral
                            },
                            theme,
                        )),
                )
                .child(mini_wins),
        );
    }

    let mut mode_bar = h_flex().gap_2();
    for mode in TilingMode::ALL {
        let cb_m = Rc::clone(&on_mode);
        mode_bar = mode_bar.child(
            div()
                .id(("cosmic-overview-mode", mode as usize))
                .cursor_pointer()
                .px_2p5()
                .py_1()
                .rounded(px(8.0))
                .bg(theme.colors.surface_container_high)
                .on_click(move |_ev, _w, cx| cb_m(mode, cx))
                .child(Label::new(mode.label()).size(LabelSize::XSmall)),
        );
    }

    let dismiss = Rc::clone(&on_close);
    v_flex()
        .id("cosmic-workspaces-overview-modal")
        .size_full()
        .p_6()
        .gap_4()
        .justify_center()
        .items_center()
        .bg(theme.colors.background.opacity(0.92))
        .child(
            h_flex()
                .w_full()
                .max_w(px(900.0))
                .justify_between()
                .items_center()
                .child(
                    Label::new("COSMIC Workspaces Overview")
                        .size(LabelSize::Large)
                        .weight(FontWeight::BOLD),
                )
                .child(
                    h_flex()
                        .gap_2()
                        .child(mode_bar)
                        .child(
                            div()
                                .id("cosmic-overview-close")
                                .cursor_pointer()
                                .px_3()
                                .py_1()
                                .rounded(px(8.0))
                                .bg(theme.colors.primary)
                                .on_click(move |_ev, _w, cx| dismiss(cx))
                                .child(
                                    Label::new("Done")
                                        .size(LabelSize::XSmall)
                                        .color(ui::Color::Custom(theme.colors.on_primary)),
                                ),
                        ),
                ),
        )
        .child(cards)
        .into_any_element()
}
