use gpui::{AnyElement, Context, ElementId, Role, SharedString, div, prelude::*, px};
use os_ui::{ImmediateUi, contrast_ratio, focusable_action_chip, render_context_drawer};
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
                        .justify_between()
                        .items_center()
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(Icon::new(IconName::MagnifyingGlass).size(IconSize::Small))
                                .child(
                                    Label::new(if shell.launcher_query.is_empty() {
                                        "Type to filter 17 ArtCraft apps + system desktop entries…"
                                            .to_owned()
                                    } else {
                                        format!("Filter: {}", shell.launcher_query)
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
