use nsh_base::theme;
use nsh_core::prelude::*;

use crate::{
    planner::InstallTargetMode,
    store,
    ui::{action_button, choice_chip, section_card, step_row, switch_row},
};

pub(crate) fn advanced_view() -> impl IntoElement {
    let srv_profiles = store::server_profiles();
    let groups = store::netinstall_groups();

    let mut srv_col = view().gap_2();
    for srv in srv_profiles {
        let sid = srv.id.clone();
        let sid_cmp = sid.clone();
        srv_col = srv_col.child(choice_chip(
            format!("{} — {}", srv.name, srv.description),
            move || {
                let cfg = store::config_snapshot();
                cfg.target_mode == InstallTargetMode::Server && cfg.server_profile == sid_cmp
            },
            move || store::set_server_profile(&sid),
        ));
    }

    let mut grp_col = view().gap_2();
    for grp in groups {
        let gname = grp.name.clone();
        let gname_cmp = gname.clone();
        grp_col = grp_col.child(choice_chip(
            format!("{} ({} pkgs)", grp.name, grp.packages.len()),
            move || {
                store::config_snapshot()
                    .optional_groups
                    .iter()
                    .any(|g| g == &gname_cmp)
            },
            move || store::toggle_optional_group(&gname),
        ));
    }

    scroll().flex_1().child(
        view()
            .p_3()
            .gap_3()
            .child(section_card(
                "Install Target Mode (Desktop vs CachyOS Server Edition)",
                view()
                    .gap_2()
                    .child(
                        view()
                            .flex_row()
                            .gap_2()
                            .child(choice_chip(
                                "Desktop Edition",
                                || store::config_snapshot().target_mode == InstallTargetMode::Desktop,
                                || store::set_target_mode(InstallTargetMode::Desktop),
                            ))
                            .child(choice_chip(
                                "Server Edition",
                                || store::config_snapshot().target_mode == InstallTargetMode::Server,
                                || store::set_target_mode(InstallTargetMode::Server),
                            )),
                    )
                    .child(srv_col),
            ))
            .child(section_card("Optional Netinstall Bundles", grp_col))
            .child(section_card(
                "System Tweaks & Autologin",
                view()
                    .gap_2()
                    .child(switch_row(
                        "Enable Display Manager Autologin",
                        || store::config_snapshot().autologin,
                        store::set_autologin,
                    ))
                    .child(action_button("Execute Advanced Plan", true, store::run_dry_install)),
            ))
            .child(section_card(
                "Execution Log Stream",
                view().gap_1().child(each(
                    store::logs,
                    |line| line.clone(),
                    |line, _| {
                        let txt = line.get_untracked();
                        step_row("LOG", txt)
                    },
                )),
            ))
            .child(
                text(move || {
                    format!(
                        "Current mode: {:?}, optional bundles: {}",
                        store::config_snapshot().target_mode,
                        store::config_snapshot().optional_groups.len()
                    )
                })
                .text_color(move || theme().muted_foreground),
            ),
    )
}
