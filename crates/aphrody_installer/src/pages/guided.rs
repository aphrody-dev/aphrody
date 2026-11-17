use nsh_base::{input, theme};
use nsh_core::prelude::*;

use crate::{
    planner::InstallTargetMode,
    profiles::{BootloaderKind, FilesystemKind, SUPPORTED_KERNELS},
    store,
    ui::{action_button, choice_chip, section_card, status_banner, step_row, switch_row},
};

pub(crate) fn guided_view() -> impl IntoElement {
    let envs = store::environments();
    let plan_rows = move || store::execution_plan().steps;

    let mut de_list = view().gap_1();
    for env in envs.into_iter().take(7) {
        let id = env.id.clone();
        let label = format!("{} [{}]", env.display_name, env.toolkit.badge());
        let id_sel = id.clone();
        de_list = de_list.child(choice_chip(
            label,
            move || {
                let cfg = store::config_snapshot();
                cfg.target_mode == InstallTargetMode::Desktop && cfg.desktop == id_sel
            },
            move || store::set_desktop(&id),
        ));
    }

    let mut kernel_row = view().gap_1();
    for kernel in SUPPORTED_KERNELS.iter().take(4) {
        let kid = kernel.id;
        kernel_row = kernel_row.child(choice_chip(
            format!("{} · {}", kernel.label, kernel.scheduler),
            move || store::config_snapshot().kernel == kid,
            move || store::set_kernel(kid),
        ));
    }

    let mut fs_row = view().flex_row().gap_2();
    for fs in FilesystemKind::ALL.into_iter().take(3) {
        fs_row = fs_row.child(choice_chip(
            fs.slug(),
            move || store::config_snapshot().fs_name == fs.slug(),
            move || store::set_filesystem(fs),
        ));
    }

    let mut bl_row = view().flex_row().gap_2();
    for bl in BootloaderKind::ALL {
        bl_row = bl_row.child(choice_chip(
            bl.slug(),
            move || store::config_snapshot().bootloader == bl.slug(),
            move || store::set_bootloader(bl),
        ));
    }

    scroll().flex_1().child(
        view()
            .p_3()
            .gap_3()
            .child(status_banner(store::status_line, store::progress_pct))
            .child(section_card(
                "Target Device & Host Identity",
                view()
                    .gap_2()
                    .child(
                        input()
                            .value(move || store::config_snapshot().device)
                            .on_change(|v: &str| store::set_device(v))
                            .placeholder("/dev/nvme0n1")
                            .px_3()
                            .py_2()
                            .border_1()
                            .border_color(move || theme().input)
                            .rounded(move || theme().radius)
                            .text_color(move || theme().foreground),
                    )
                    .child(
                        input()
                            .value(move || store::config_snapshot().hostname)
                            .on_change(|v: &str| store::set_hostname(v))
                            .placeholder("aphrody-os")
                            .px_3()
                            .py_2()
                            .border_1()
                            .border_color(move || theme().input)
                            .rounded(move || theme().radius)
                            .text_color(move || theme().foreground),
                    ),
            ))
            .child(section_card("Desktop Environment / Rust Userland", de_list))
            .child(section_card("Kernel & Bootloader", view().gap_2().child(kernel_row).child(fs_row).child(bl_row)))
            .child(section_card(
                "Disk Encryption, ZRAM & Hardware Drivers",
                view()
                    .gap_2()
                    .child(switch_row(
                        "LUKS2 Full-Disk Encryption",
                        || store::config_snapshot().encrypt_luks,
                        store::set_encrypt_luks,
                    ))
                    .child(switch_row(
                        "ZRAM + System Tweaks",
                        || store::config_snapshot().zram_swap,
                        store::set_zram_swap,
                    ))
                    .child(switch_row(
                        "CHWD Hardware Drivers (NVIDIA/AMD/Intel/Wi-Fi)",
                        || store::config_snapshot().chwd_auto_drivers,
                        store::set_chwd_drivers,
                    ))
                    .child(action_button("Generate & Execute Dry-Run Plan", true, store::run_dry_install)),
            ))
            .child(section_card(
                "Execution Plan Preview",
                view().gap_1().child(each(
                    plan_rows,
                    |s| s.id.clone(),
                    |step, _| {
                        let st = step.get_untracked();
                        step_row(st.phase, st.command_preview)
                    },
                )),
            )),
    )
}
