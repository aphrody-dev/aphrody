//! Headless NSH end-to-end UI flow tests for `aphrody_installer`.

#![cfg(all(feature = "nsh-ui", not(target_family = "wasm")))]

use aphrody_installer::InstallerApp;
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;

fn start() -> Headless {
    let mut app = Headless::start(EngineConfig::new(Root::of::<InstallerApp>())).expect("start NSH");
    app.wait_for("Guided Install").expect("tab bar loads");
    app
}

#[test]
fn switches_between_aphrody_rust_userland_and_arch_des_via_nsh_ui() {
    let mut app = start();
    app.press("Switch DE").expect("switch to DE tab");
    app.wait_for("COSMIC Epoch (Iced / libcosmic) [Iced / COSMIC]")
        .expect("Iced COSMIC visible");
    app.press("role:button[GNOME (GTK4 / libadwaita) [GTK4 / Adwaita]]")
        .expect("select GNOME GTK4");
    app.wait_for("DE: gnome").expect("active DE updates to gnome");

    app.press("role:button[Craft Studio Workstation (egui + Bevy 5.8) [egui / Bevy]]")
        .expect("switch to egui Craft Workstation");
    app.wait_for("DE: egui-craft").expect("active DE updates to egui-craft");
}

#[test]
fn guided_install_generates_dry_run_plan_and_updates_progress() {
    let mut app = start();
    app.press("role:button[Generate & Execute Dry-Run Plan]")
        .expect("execute dry-run plan");
    app.wait_for("Dry-run plan ready: 11 steps, 155 packages, target=aphrody-gpui")
        .expect("dry-run completion banner updates");
}
