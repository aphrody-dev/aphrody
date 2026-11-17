use anyhow::Result;
use aphrody_craft::{CraftStudioSession, StudioPillar, export_figcraft_baseline_html};
use aphrody_design::design_md::load_bundled_root_design_md;
use aphrody_design::web_standards::audit_html_standards;
use aphrody_ui::AppearanceMode;
use serde_json::json;

fn main() -> Result<()> {
    let mut session = CraftStudioSession::bootstrap(AppearanceMode::Dark)?;
    let mut pillars = Vec::new();
    for pillar in [
        StudioPillar::FigCraftGoogle,
        StudioPillar::CraftAnythingGoogle,
        StudioPillar::UnrealBevy58,
        StudioPillar::StorytoldSuiteCatalog,
    ] {
        session.active_pillar = pillar;
        pillars.push(session.run_parity_benchmark(960, 540)?);
    }

    let design_md = load_bundled_root_design_md()?;
    let html = export_figcraft_baseline_html(&session.figcraft, &design_md);
    let lighthouse = audit_html_standards("figcraft.html", &html, &design_md, Some(64_000));

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": lighthouse.ok && lighthouse.scores.overall == 100,
            "storytold_total_repos": session.storytold_ledger.total_repositories,
            "craft_catalog_apps": session.catalog.apps.len(),
            "figcraft_nodes": session.figcraft.total_node_count(),
            "craft_anything_disasm_rows": session.craft_anything.disasm_rows.len(),
            "lighthouse_overall": lighthouse.scores.overall,
            "pillars": pillars,
        }))?
    );
    Ok(())
}
