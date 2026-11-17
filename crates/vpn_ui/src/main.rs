//! `aphrody-vpn`: the VPN client as a standalone app on the Aphrody app shell.

use gpui_app_shell::AppShellBuilder;
use vpn_ui::VpnModule;

fn main() {
    let mut builder = AppShellBuilder::new("Aphrody VPN")
        .with_code_panels(false)
        .with_module(VpnModule::new().open_on_launch(true));
    if let Some(data_dir) = dirs::data_dir() {
        builder = builder.data_dir(data_dir.join("aphrody-vpn"));
    }
    #[cfg(feature = "nsh")]
    let builder = builder.with_module(gpui_app_shell::nsh::NshModule::default());
    builder.run();
}
