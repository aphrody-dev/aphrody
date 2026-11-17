//! Embedded `aphrody_installer` view inside COSMIC OS for CachyOS / Arch Installation
//! Planning & Live Desktop Environment / Rust Userland Switching.

use aphrody_installer::{
    InstallerConfig, InstallerProfileCatalog, plan_desktop_switch, plan_installation,
};
use aphrody_ui::{KeyValueRow, OsTheme, StatusTone, status_badge};
use gpui::{AnyElement, App, div, prelude::*, px};
use ui::{IconName, Label, LabelCommon as _, LabelSize, h_flex, v_flex};

use crate::apps::section_heading;

pub fn render_installer_window(
    theme: &OsTheme,
    catalog: &InstallerProfileCatalog,
    active_env_id: &str,
    on_select_env: Box<dyn Fn(&str, &mut App) + 'static>,
) -> AnyElement {
    let cb = std::rc::Rc::new(on_select_env);
    let mut cards = v_flex().gap_1p5();
    for (idx, env) in catalog.environments.iter().take(10).enumerate() {
        let eid = env.id.clone();
        let active = eid == active_env_id;
        let on_sel = std::rc::Rc::clone(&cb);
        cards = cards.child(
            div()
                .id(("cosmic-inst-env", idx))
                .cursor_pointer()
                .p_2()
                .rounded(px(8.0))
                .border_1()
                .border_color(if active {
                    theme.colors.primary
                } else {
                    theme.colors.outline_variant
                })
                .bg(if active {
                    theme.colors.primary_container
                } else {
                    theme.colors.surface_container_high
                })
                .on_click(move |_ev, _w, cx| on_sel(&eid, cx))
                .child(
                    h_flex()
                        .justify_between()
                        .child(Label::new(env.display_name.clone()).size(LabelSize::XSmall))
                        .child(status_badge(
                            IconName::Code,
                            env.toolkit.badge(),
                            if env.aphrody_userland {
                                StatusTone::Success
                            } else {
                                StatusTone::Neutral
                            },
                            theme,
                        )),
                ),
        );
    }

    let mut cfg = InstallerConfig::default();
    cfg.desktop = active_env_id.to_owned();
    let plan = plan_installation(&cfg, catalog);
    let switch = plan_desktop_switch("aphrody-gpui", active_env_id, catalog).ok();

    let mut plan_col = v_flex().flex_1().gap_1p5();
    plan_col = plan_col
        .child(section_heading(theme, "CachyOS Dry-Run Execution Plan"))
        .child(KeyValueRow::new(
            "Total Packages",
            plan.packages.len().to_string(),
        ))
        .child(KeyValueRow::new("Steps", plan.steps.len().to_string()));
    if let Some(sw) = switch {
        plan_col = plan_col
            .child(KeyValueRow::new("Display Manager", sw.display_manager))
            .child(KeyValueRow::new("Session Command", sw.session_exec));
    }
    for step in plan.steps.iter().take(5) {
        plan_col = plan_col.child(KeyValueRow::new(
            step.title.clone(),
            step.command_preview.clone(),
        ));
    }

    h_flex()
        .size_full()
        .gap_3()
        .child(
            v_flex()
                .w(px(300.0))
                .gap_2()
                .child(section_heading(theme, "Rust Userlands & Arch DEs"))
                .child(cards),
        )
        .child(plan_col)
        .into_any_element()
}
