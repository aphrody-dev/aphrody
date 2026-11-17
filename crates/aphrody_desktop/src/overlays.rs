use gpui::{AnyElement, Context, ElementId, Role, SharedString, div, prelude::*, px};
use aphrody_ui::{ImmediateUi, OsTheme, contrast_ratio, focusable_action_chip, render_context_drawer};
use ui::{Icon, IconName, IconSize, Label, LabelCommon as _, LabelSize, h_flex, v_flex};

use crate::parse_icon_name;
use crate::shell::OsDesktopShell;
use crate::windows::WindowKind;

pub(crate) fn render_context_drawer_pane(
    shell: &OsDesktopShell,
    cx: &mut Context<OsDesktopShell>,
) -> AnyElement {
    let mut ui = ImmediateUi::new(&shell.theme);
    ui.heading("Desktop Tokens & Contrast");
    let on_bg = contrast_ratio(
        shell.theme.colors.on_background,
        shell.theme.colors.background,
    );
    let on_primary = contrast_ratio(shell.theme.colors.on_primary, shell.theme.colors.primary);
    ui.kv_row(
        "Theme ID",
        format!("{} ({:?})", shell.theme.id, shell.theme.mode),
    );
    ui.kv_row("AA Background", format!("{on_bg:.2}:1"));
    ui.kv_row("AA Primary", format!("{on_primary:.2}:1"));
    ui.color_swatch("Primary Seed", shell.theme.colors.primary, "Interactive");
    ui.color_swatch("Secondary", shell.theme.colors.secondary, "Accent");
    ui.color_swatch(
        "Surface Low",
        shell.theme.colors.surface_container_low,
        "Chrome",
    );

    ui.section("Quick Themes", |sec| {
        for tid in shell.theme_catalog.ids() {
            let entity = cx.entity();
            let label = tid.clone();
            let selected = tid == shell.theme.id;
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

    crate::pilot_hud::append_pilot_drawer_section(
        &mut ui,
        shell.pilot.active_adapter().backend_name(),
        &shell.pilot.recent_hud_activity,
    );

    let close_entity = cx.entity();
    render_context_drawer(
        "Inspector Drawer",
        ui.finish(),
        &shell.theme,
        move |_w, cx| {
            close_entity.update(cx, |shell, cx| shell.toggle_drawer(cx));
        },
    )
}

pub(crate) fn render_launcher_modal(
    shell: &OsDesktopShell,
    vw: f32,
    vh: f32,
    cx: &mut Context<OsDesktopShell>,
) -> AnyElement {
    let query = shell.launcher_query.trim().to_ascii_lowercase();
    let close_entity = cx.entity();

    let mut app_list = v_flex().w_full().gap_1p5().pb_2();
    let mut shown = 0_usize;

    for app in &shell.craft_catalog.apps {
        if !query.is_empty()
            && !app.name.to_ascii_lowercase().contains(&query)
            && !app.tagline.to_ascii_lowercase().contains(&query)
            && !app.category.to_ascii_lowercase().contains(&query)
        {
            continue;
        }
        let entity = cx.entity();
        let app_id = app.id.clone();
        let selected = app_id == shell.studio.active_app_id;
        app_list = app_list.child(focusable_action_chip(
            ElementId::Name(format!("launcher-craft-{app_id}").into()),
            parse_icon_name(&app.icon),
            SharedString::from(format!("{} · {}", app.name, app.category)),
            Some(SharedString::from(format!("{} ext", app.extensions.len()))),
            SharedString::from(format!("Launch {}", app.name)),
            selected,
            (500 + shown) as isize,
            &shell.theme,
            move |_, _w, cx| {
                entity.update(cx, |shell, cx| shell.select_craft_app(&app_id, cx));
            },
        ));
        shown += 1;
        if shown >= 12 {
            break;
        }
    }

    for sys_app in &shell.discovered_apps {
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
            &shell.theme,
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
                .bg(shell.theme.colors.surface_container)
                .border_2()
                .border_color(shell.theme.colors.primary)
                .child(
                    h_flex()
                        .w_full()
                        .min_w_0()
                        .gap_2()
                        .justify_between()
                        .items_center()
                        .child(
                            h_flex()
                                .flex_1()
                                .min_w_0()
                                .gap_2()
                                .items_center()
                                .overflow_hidden()
                                .child(Icon::new(IconName::MagnifyingGlass).size(IconSize::Small))
                                .child(
                                    div().flex_1().min_w_0().overflow_hidden().child(
                                        Label::new(if shell.launcher_query.is_empty() {
                                            "Type to filter 17 ArtCraft apps + system desktop entries…"
                                                .to_owned()
                                        } else {
                                            format!("Filter: {}", shell.launcher_query)
                                        })
                                        .size(LabelSize::Default)
                                        .truncate(),
                                    ),
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
                            &shell.theme,
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

/// Renders `cosmic-osd` floating bottom indicator pill (`OsdBanner`: volume, backlight, power profile, layout).
#[must_use]
pub fn render_cosmic_osd_pill(
    theme: &OsTheme,
    banner: &aphrody_session::OsdBanner,
) -> AnyElement {
    use aphrody_ui::{StatusTone, status_badge};
    use ui::{IconName, Label, LabelCommon as _, LabelSize, h_flex, v_flex};
    use gpui::{div, prelude::*, px};

    let ratio = f32::from(banner.level_percent.unwrap_or(100).min(100)) / 100.0;
    v_flex()
        .id("cosmic-osd-pill")
        .p_3()
        .w(px(360.0))
        .gap_1p5()
        .overflow_hidden()
        .rounded(px(theme.layout.window_radius))
        .border_1()
        .border_color(theme.colors.outline_variant)
        .bg(theme.colors.surface_container_high)
        .child(
            h_flex()
                .gap_2()
                .justify_between()
                .items_center()
                .overflow_hidden()
                .child(status_badge(
                    IconName::Settings,
                    banner.title.clone(),
                    StatusTone::Accent,
                    theme,
                ))
                .child(
                    div().flex_1().min_w_0().overflow_hidden().child(
                        Label::new(banner.subtitle.clone())
                            .size(LabelSize::XSmall)
                            .truncate(),
                    ),
                ),
        )
        .child(
            div()
                .w_full()
                .h(px(6.0))
                .rounded(px(999.0))
                .bg(theme.colors.surface_container_high)
                .child(
                    div()
                        .h_full()
                        .w(gpui::relative(ratio))
                        .rounded(px(999.0))
                        .bg(theme.colors.primary),
                ),
        )
        .into_any_element()
}

/// Renders interactive `cosmic-notifications` toast stack in the top-right corner.
pub fn render_cosmic_notifications_stack(
    theme: &OsTheme,
    center: &aphrody_session::NotificationCenter,
    on_dismiss: Box<dyn Fn(u64, &mut gpui::App) + 'static>,
    on_clear_all: Box<dyn Fn(&mut gpui::App) + 'static>,
) -> AnyElement {
    use gpui::{div, prelude::*, px};
    use ui::{Label, LabelCommon as _, LabelSize, h_flex, v_flex};

    let cb_dismiss = std::rc::Rc::new(on_dismiss);
    let mut stack = v_flex()
        .id("cosmic-notifications-stack")
        .w(px(320.0))
        .gap_2()
        .child(
            h_flex()
                .justify_between()
                .items_center()
                .child(
                    Label::new(format!("Notifications ({})", center.items.len()))
                        .size(LabelSize::XSmall),
                )
                .child(
                    div()
                        .id("cosmic-notif-clear-all")
                        .cursor_pointer()
                        .px_2()
                        .py_0p5()
                        .rounded(px(6.0))
                        .bg(theme.colors.surface_container_high)
                        .on_click(move |_ev, _w, cx| on_clear_all(cx))
                        .child(Label::new("Clear all").size(LabelSize::XSmall)),
                ),
        );

    for (idx, item) in center.items.iter().take(4).enumerate() {
        let nid = item.id;
        let dismiss = std::rc::Rc::clone(&cb_dismiss);
        stack = stack.child(
            v_flex()
                .p_2p5()
                .gap_1()
                .rounded(px(theme.layout.window_radius))
                .border_1()
                .border_color(theme.colors.outline_variant)
                .bg(theme.colors.surface_container.opacity(0.95))
                .child(
                    h_flex()
                        .justify_between()
                        .child(Label::new(item.summary.clone()).size(LabelSize::XSmall))
                        .child(
                            div()
                                .id(("cosmic-notif-dismiss", idx))
                                .cursor_pointer()
                                .px_1p5()
                                .rounded(px(4.0))
                                .on_click(move |_ev, _w, cx| dismiss(nid, cx))
                                .child(Label::new("×").size(LabelSize::XSmall)),
                        ),
                )
                .child(
                    Label::new(item.body.clone())
                        .size(LabelSize::XSmall)
                        .color(ui::Color::Muted),
                ),
        );
    }

    stack.into_any_element()
}

/// Renders `cosmic-screenshot` selection/window/screen toolbar overlay.
pub fn render_cosmic_screenshot_toolbar(
    theme: &OsTheme,
    on_capture_mode: Box<dyn Fn(&'static str, &mut gpui::App) + 'static>,
    on_close: Box<dyn Fn(&mut gpui::App) + 'static>,
) -> AnyElement {
    use gpui::{div, prelude::*, px};
    use ui::{Label, LabelCommon as _, LabelSize, h_flex};

    let cb = std::rc::Rc::new(on_capture_mode);
    let cb_close = std::rc::Rc::new(on_close);
    let mut bar = h_flex()
        .id("cosmic-screenshot-toolbar")
        .p_2p5()
        .gap_2()
        .items_center()
        .rounded(px(999.0))
        .border_1()
        .border_color(theme.colors.primary)
        .bg(theme.colors.surface_container.opacity(0.96));

    for (idx, (mode, label)) in [
        ("screen", "Entire Screen"),
        ("window", "Active Window"),
        ("selection", "Region Selection"),
    ]
    .into_iter()
    .enumerate()
    {
        let capture = std::rc::Rc::clone(&cb);
        let dismiss = std::rc::Rc::clone(&cb_close);
        bar = bar.child(
            div()
                .id(("cosmic-shot-mode", idx))
                .cursor_pointer()
                .px_3()
                .py_1()
                .rounded(px(999.0))
                .bg(theme.colors.primary_container)
                .on_click(move |_ev, _w, cx| {
                    capture(mode, cx);
                    dismiss(cx);
                })
                .child(Label::new(label).size(LabelSize::XSmall)),
        );
    }
    bar.into_any_element()
}

/// Renders `cosmic-greeter` full-screen lock / session-unlock overlay.
pub fn render_cosmic_greeter_lock(
    theme: &OsTheme,
    user_label: &str,
    on_unlock: Box<dyn Fn(&mut gpui::App) + 'static>,
) -> AnyElement {
    use gpui::{FontWeight, div, prelude::*, px};
    use ui::{Icon, IconName, IconSize, Label, LabelCommon as _, LabelSize, v_flex};

    v_flex()
        .id("cosmic-greeter-overlay")
        .size_full()
        .justify_center()
        .items_center()
        .gap_3()
        .bg(theme.colors.background.opacity(0.96))
        .child(Icon::new(IconName::Lock).size(IconSize::Medium))
        .child(
            Label::new(format!("{} Session Locked", brand::APP_NAME))
                .size(LabelSize::Large)
                .weight(FontWeight::BOLD),
        )
        .child(
            Label::new(format!("User: {user_label} · Wayland / GPUI Compositor"))
                .size(LabelSize::Small)
                .color(ui::Color::Muted),
        )
        .child(
            div()
                .id("cosmic-greeter-unlock")
                .cursor_pointer()
                .px_4()
                .py_2()
                .rounded(px(999.0))
                .bg(theme.colors.primary)
                .on_click(move |_ev, _w, cx| on_unlock(cx))
                .child(
                    Label::new("Unlock Session")
                        .size(LabelSize::Small)
                        .color(ui::Color::Custom(theme.colors.on_primary)),
                ),
        )
        .into_any_element()
}
