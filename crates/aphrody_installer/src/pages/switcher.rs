use nsh_base::theme;
use nsh_core::prelude::*;

use crate::{
    store,
    ui::{choice_chip, section_card, status_banner},
};

pub(crate) fn switcher_view() -> impl IntoElement {
    let envs = store::environments();

    let mut rust_userlands = view().gap_2();
    let mut arch_des = view().gap_2();

    for env in envs {
        let id = env.id.clone();
        let id_cmp = id.clone();
        let title = format!("{} [{}]", env.display_name, env.toolkit.badge());
        let card = view()
            .gap_1()
            .child(choice_chip(
                title,
                move || store::config_snapshot().desktop == id_cmp,
                move || store::set_desktop(&id),
            ))
            .child(
                text(format!(
                    "{} · dm={} · exec=`{}` ({} pkgs)",
                    env.summary,
                    env.display_manager,
                    env.session_exec,
                    env.packages.len()
                ))
                .text_color(move || theme().muted_foreground),
            );

        if env.aphrody_userland {
            rust_userlands = rust_userlands.child(card);
        } else {
            arch_des = arch_des.child(card);
        }
    }

    scroll().flex_1().child(
        view()
            .p_3()
            .gap_3()
            .child(status_banner(store::status_line, store::progress_pct))
            .child(section_card(
                "Aphrody Rust Userland Stacks (GPUI, Iced COSMIC, egui+Bevy, WebGPU)",
                rust_userlands,
            ))
            .child(section_card(
                "Arch / CachyOS Classic Desktop Environments (GTK GNOME, Qt KDE, Hyprland, Sway, Niri)",
                arch_des,
            )),
    )
}
