use anyhow::Result;
use bevy_gpui::{
    SolariGiSettings, UnrealWorld, ViewportShadingMode, compile_materialx_to_wgsl,
    default_openpbr_materialx_xml, render_bevy_egui_inspector, render_viewport_rgba8,
};
use egui_gpui::EguiGpuiHost;
use aphrody_ui::AppearanceMode;
use serde_json::json;

fn main() -> Result<()> {
    let mut world = UnrealWorld::default_showcase_scene();
    let mut gi = SolariGiSettings::default();
    let mut shading = ViewportShadingMode::LitPbrSolari;

    let shader = compile_materialx_to_wgsl(default_openpbr_materialx_xml().as_bytes(), &gi)?;
    world.step(1.0 / 60.0);
    let (rgba, viewport_stats) = render_viewport_rgba8(&world, &gi, shading, 320, 180);

    let mut egui_host = EguiGpuiHost::new(AppearanceMode::Dark)?;
    let inspector = render_bevy_egui_inspector(
        &mut egui_host,
        &mut world,
        &mut gi,
        &mut shading,
        420,
        540,
    )?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": shader.naga_validated,
            "materialx_dialect": shader.dialect,
            "wgsl_bytes": shader.wgsl_source.len(),
            "viewport_rgba_bytes": rgba.len(),
            "viewport_stats": viewport_stats,
            "bevy_egui_inspector": inspector,
        }))?
    );
    Ok(())
}
