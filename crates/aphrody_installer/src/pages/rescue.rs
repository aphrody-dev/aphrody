use nsh_base::theme;
use nsh_core::prelude::*;

use crate::{
    planner::RESCUE_TASKS,
    store,
    ui::{action_button, section_card, status_banner, step_row},
};

pub(crate) fn rescue_view() -> impl IntoElement {
    let mut rescue_list = view().gap_2();
    for task in RESCUE_TASKS {
        let tid = task.id;
        rescue_list = rescue_list.child(
            view()
                .p_2()
                .gap_1()
                .rounded(move || theme().radius)
                .bg(move || theme().muted)
                .child(
                    text(task.title)
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(move || theme().foreground),
                )
                .child(
                    text(task.description)
                        .text_color(move || theme().muted_foreground),
                )
                .child(step_row("CMD", task.command_preview))
                .child(action_button("Run Rescue Task", false, move || store::trigger_rescue(tid))),
        );
    }

    scroll().flex_1().child(
        view()
            .p_3()
            .gap_3()
            .child(status_banner(store::status_line, store::progress_pct))
            .child(section_card(
                "System Rescue (Drivers, Bootloader, Btrfs Snapshots, Keyring, Journal)",
                rescue_list,
            )),
    )
}
