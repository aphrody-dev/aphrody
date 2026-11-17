use anyhow::Result;
use egui_gpui::{
    CraftComboMenu, CraftDockTab, CraftDockTabs, CraftImeBanner, CraftPropertyRow,
    CraftToolbarChip, EguiGpuiHost, ImeWireEvent, egui,
};
use aphrody_ui::{AppearanceMode, ShadcnGpuiTheme};

fn main() -> Result<()> {
    let mut host = EguiGpuiHost::new(AppearanceMode::Dark)?;
    host.push_wire_event(ImeWireEvent::Preedit {
        text: "Google Sans Flex".to_owned(),
        cursor_start: Some(0),
        cursor_end: Some(6),
    });

    let theme = ShadcnGpuiTheme::from_bundled(AppearanceMode::Dark)?;
    let tabs = vec![
        CraftDockTab::new("figcraft", "FigCraft").with_badge("Google"),
        CraftDockTab::new("anything", "Craft Anything").with_badge("Oxc"),
        CraftDockTab::new("unreal", "Unreal Bevy 5.8").with_badge("Nanite"),
    ];
    let mut active_tab = "figcraft".to_owned();
    let backends = vec![
        "Vulkan 1.3".to_owned(),
        "DirectX 12".to_owned(),
        "WebGPU".to_owned(),
    ];
    let mut selected_backend = 0usize;
    let mut spring_stiffness = 700.0f32;

    let artifact = host.run_frame(960, 540, 1.0, |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            CraftDockTabs::new(&tabs, &mut active_tab).show(ui, &theme);
            CraftComboMenu::new("backend", "GPU Backend", &backends, &mut selected_backend)
                .show(ui);
            let _active =
                CraftToolbarChip::new("Material 3 Neural Expressive", true).show(ui, &theme);
            CraftPropertyRow::new("Spatial Spring", &mut spring_stiffness, 100.0..=3800.0, "")
                .show(ui);
            CraftImeBanner::new("Google Sans Flex").show(ui, &theme);
        });
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&artifact.stats)?
    );
    Ok(())
}
