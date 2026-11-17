//! Aphrody OS Installer & Desktop Environment Switcher rewritten in pure Rust + NSH
//! from `vendor/new-cli-installer` (`cachyos/new-cli-installer`).
//!
//! Works both as a standalone Wasm-safe library crate (`aphrody_installer`) AND as a full NSH
//! native application (`InstallerApp`, `#[native_module("aphrody-os/installer")]`) when the
//! `nsh-ui` feature is enabled.

pub mod planner;
pub mod profiles;

#[cfg(feature = "nsh-ui")]
pub mod pages;
#[cfg(feature = "nsh-ui")]
pub mod store;
#[cfg(feature = "nsh-ui")]
pub mod ui;

pub use planner::{
    BtrfsSubvolume, DesktopSwitchPlan, InstallExecutionPlan, InstallStepSpec, InstallTargetMode,
    InstallerConfig, PartitionEntry, PreflightReport, RESCUE_TASKS, RescueTaskSpec,
    default_btrfs_subvolumes, plan_desktop_switch, plan_installation, validate_config,
};
pub use profiles::{
    BUNDLED_NET_PROFILES_TOML, BUNDLED_SERVER_PROFILES_TOML, BootloaderKind,
    DesktopEnvironmentSpec, EnvironmentToolkit, FilesystemKind, InstallerProfileCatalog,
    KernelSpec, NetinstallGroupSpec, ProfileError, SUPPORTED_KERNELS, ServerProfileSpec,
    ServiceUnitSpec,
};

#[cfg(feature = "nsh-ui")]
use nsh_base::{nav_page, nav_stack, tab, tabs, theme};
#[cfg(feature = "nsh-ui")]
use nsh_core::prelude::*;

#[cfg(feature = "nsh-ui")]
#[native_module("aphrody-os/installer")]
fn native(registry: &mut Registry) {
    registry.register::<InstallerApp>();
}

#[cfg(feature = "nsh-ui")]
#[component]
pub fn InstallerApp() -> impl IntoElement {
    let tab_bar = tabs()
        .flex_row()
        .gap_2()
        .px_3()
        .py_2()
        .indicator_style(|s| s.bottom(px(0.0)).h(px(2.0)))
        .child(tab_button("Guided Install", store::InstallerTab::Guided))
        .child(tab_button("Switch DE", store::InstallerTab::SwitchDe))
        .child(tab_button("Advanced & Server", store::InstallerTab::Advanced))
        .child(tab_button("System Rescue", store::InstallerTab::Rescue));

    let header = view()
        .flex_col()
        .border_b_1()
        .border_b_color(move || theme().border)
        .bg(move || theme().card)
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .px_4()
                .pt_3()
                .child(
                    text(format!("{} OS Installer & DE Switcher", brand::APP_NAME))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(move || theme().foreground),
                )
                .child(
                    text(move || format!("DE: {}", store::config_snapshot().desktop))
                        .text_color(move || theme().primary),
                ),
        )
        .child(tab_bar);

    let body = view()
        .flex_1()
        .child(when(
            || store::active_tab() == store::InstallerTab::Guided,
            pages::guided::guided_view,
        ))
        .child(when(
            || store::active_tab() == store::InstallerTab::SwitchDe,
            pages::switcher::switcher_view,
        ))
        .child(when(
            || store::active_tab() == store::InstallerTab::Advanced,
            pages::advanced::advanced_view,
        ))
        .child(when(
            || store::active_tab() == store::InstallerTab::Rescue,
            pages::rescue::rescue_view,
        ));

    view().flex_1().bg(move || theme().background).child(
        nav_stack()
            .flex_1()
            .child(nav_page().title("Aphrody OS Installer").header(header).child(body)),
    )
}

#[cfg(feature = "nsh-ui")]
fn tab_button(label: &'static str, kind: store::InstallerTab) -> impl IntoElement {
    tab()
        .selected(move || store::active_tab() == kind)
        .on_press(move || store::set_tab(kind))
        .px_3()
        .py_2()
        .child(
            text(label)
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || {
                    if store::active_tab() == kind {
                        theme().primary
                    } else {
                        theme().muted_foreground
                    }
                }),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cachyos_profiles_parse_with_aphrody_rust_userlands_and_arch_des() {
        let catalog = InstallerProfileCatalog::bundled().expect("bundled installer TOMLs parse");
        assert!(catalog.environment("aphrody-gpui").is_some());
        assert!(catalog.environment("iced-cosmic").is_some());
        assert!(catalog.environment("egui-craft").is_some());
        assert!(catalog.environment("webgpu-os").is_some());
        assert!(catalog.environment("gnome").is_some());
        assert!(catalog.environment("kde").is_some());
        assert!(catalog.environment("hyprland").is_some());

        let plan = plan_installation(&InstallerConfig::default(), &catalog);
        assert!(plan.preflight.valid);
        assert!(plan.steps.len() >= 9);
        assert!(!plan.btrfs_subvolumes.is_empty());

        let switched =
            plan_desktop_switch("aphrody-gpui", "kde", &catalog).expect("switch to KDE plans");
        assert_eq!(switched.display_manager, "plasma-login-manager");
        assert!(switched.install_packages.iter().any(|p| p == "plasma-desktop"));
    }
}
