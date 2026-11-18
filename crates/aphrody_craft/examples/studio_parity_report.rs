use std::hint::black_box;
use std::time::Instant;

use anyhow::Result;
use aphrody_craft::{
    CraftStudioSession, StudioPillar, export_figcraft_baseline_html, inspect_bytes,
};
use aphrody_design::design_md::load_bundled_root_design_md;
use aphrody_design::web_standards::audit_html_standards;
use aphrody_ui::{AppearanceMode, TaskCommand, parse_cosmic_ron_palette};
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

    // Craft-Libs pure-Rust container identification benchmark (ELF, PE, Wasm, PNG, JSON x 100)
    let sample_elf: &[u8] = b"\x7fELF\x02\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00";
    let sample_wasm: &[u8] = b"\x00asm\x01\x00\x00\x00\x01\x04\x01`\x00\x00";
    let sample_pe: &[u8] = b"MZ\x90\x00\x03\x00\x00\x00\x04\x00\x00\x00";
    let sample_png: &[u8] = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR";
    let sample_json: &[u8] = b"{\"schema\":1,\"engine\":\"aphrody\"}";
    let craft_started = Instant::now();
    let mut craft_formats = 0usize;
    for iter in 0..100usize {
        for (name, payload) in [
            ("bin.elf", sample_elf),
            ("mod.wasm", sample_wasm),
            ("app.exe", sample_pe),
            ("frame.png", sample_png),
            ("meta.json", sample_json),
        ] {
            let report = inspect_bytes(name, payload, None, Some(&session.catalog));
            if !report.format_name.is_empty() && iter == 0 {
                craft_formats += 1;
            }
            black_box(report);
        }
    }
    let craft_sniff_100x_us = craft_started.elapsed().as_micros() as u64;

    // Libcosmic RON theme parse + WCAG conversion benchmark (x 100)
    let cosmic_ron = r#"(
        bg_color: Srgba::new(0.055, 0.078, 0.106, 1.0),
        primary_container_bg: Srgba::new(0.086, 0.114, 0.145, 1.0),
        accent: Srgba::new(0.478, 0.816, 1.0, 1.0),
    )"#;
    let cosmic_started = Instant::now();
    for _ in 0..100 {
        black_box(parse_cosmic_ron_palette("bench-cosmic", cosmic_ron));
    }
    let libcosmic_ron_100x_us = cosmic_started.elapsed().as_micros() as u64;

    // Iced-style Elm TaskCommand batch reduction benchmark (x 1000)
    let iced_started = Instant::now();
    let mut acc = 0u64;
    for i in 0..1000u64 {
        let mut stack = vec![TaskCommand::batch([
            TaskCommand::done(i),
            TaskCommand::done(i.wrapping_add(1)),
            TaskCommand::none(),
        ])];
        while let Some(cmd) = stack.pop() {
            match cmd {
                TaskCommand::None | TaskCommand::Perform(_) => {}
                TaskCommand::Message(v) => acc = acc.wrapping_add(v),
                TaskCommand::Batch(items) => stack.extend(items),
            }
        }
    }
    black_box(acc);
    let iced_elm_1000x_us = iced_started.elapsed().as_micros() as u64;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": lighthouse.ok && lighthouse.scores.overall == 100,
            "storytold_total_repos": session.storytold_ledger.total_repositories,
            "craft_catalog_apps": session.catalog.apps.len(),
            "figcraft_nodes": session.figcraft.total_node_count(),
            "craft_anything_disasm_rows": session.craft_anything.disasm_rows.len(),
            "lighthouse_overall": lighthouse.scores.overall,
            "craft_formats_verified": craft_formats,
            "craft_sniff_100x_us": craft_sniff_100x_us,
            "libcosmic_ron_100x_us": libcosmic_ron_100x_us,
            "iced_elm_1000x_us": iced_elm_1000x_us,
            "pillars": pillars,
        }))?
    );
    Ok(())
}
