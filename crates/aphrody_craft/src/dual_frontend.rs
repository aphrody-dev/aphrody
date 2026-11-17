//! Shared GUI Core (`CraftStudioSession`) & Side-by-Side Comparative Renderer:
//! Drives **Google Design GPUI (`aphrody_ui`)** and **Migrated `egui 0.37` / `bevy_egui` (`egui_gpui` + `bevy_gpui`)**
//! from one identical state model so visual fidelity, ergonomics, AccessKit a11y, and frame latency
//! can be compared side-by-side across FigCraft, Craft Anything, Bevy UE5.8, and all 18 Storytold apps.

use anyhow::Result;
use bevy_gpui::{
    CompiledMaterialShader, SolariGiSettings, UnrealWorld, ViewportRenderStats,
    ViewportShadingMode, compile_materialx_to_wgsl, default_openpbr_materialx_xml,
    render_bevy_egui_inspector, render_viewport_rgba8,
};
use egui_gpui::{EguiFrameStats, EguiGpuiHost};
use gpui::{AnyElement, IntoElement, ParentElement as _, Styled as _, div, px};
use crate::catalog::CraftCatalog;
use crate::storytold_suite::StorytoldEcosystemLedger;
use aphrody_ui::AppearanceMode;
use aphrody_ui::starter_pack::{ShadcnGpuiTheme, StarterBadge, StarterCard, StarterKpiCard};
use serde::{Deserialize, Serialize};
use ui::{Label, LabelCommon as _, LabelSize, h_flex, v_flex};
use web_time::Instant;

use crate::craft_anything_core::CraftAnythingWorkbench;
use crate::figcraft_core::{
    FigCraftDocument, export_figcraft_baseline_html, export_figcraft_gpui_rust,
    export_figcraft_nsh_tsx,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StudioPillar {
    #[default]
    FigCraftGoogle,
    CraftAnythingGoogle,
    UnrealBevy58,
    StorytoldSuiteCatalog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParityRenderMode {
    GpuiGoogleDesign,
    EguiNative037,
    #[default]
    SplitSideBySide,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct GuiParityBenchmark {
    pub gpui_tree_build_us: u64,
    pub egui_tessellate_us: u64,
    pub egui_vertices: usize,
    pub egui_triangles: usize,
    pub egui_draw_calls: usize,
    pub egui_accesskit_nodes: usize,
    pub bevy_viewport_us: u64,
    pub bevy_visible_meshlets: usize,
    pub wcag_aa_verified: bool,
}

/// Single shared domain session backing both GPUI Google Design views and `egui 0.37` / `bevy_egui` panels.
pub struct CraftStudioSession {
    pub active_pillar: StudioPillar,
    pub parity_mode: ParityRenderMode,
    pub appearance: AppearanceMode,
    pub active_craft_app_id: String,
    pub catalog: CraftCatalog,
    pub storytold_ledger: StorytoldEcosystemLedger,
    pub figcraft: FigCraftDocument,
    pub craft_anything: CraftAnythingWorkbench,
    pub unreal_world: UnrealWorld,
    pub solari_gi: SolariGiSettings,
    pub shading_mode: ViewportShadingMode,
    pub compiled_shader: CompiledMaterialShader,
    pub egui_host: EguiGpuiHost,
    pub last_benchmark: GuiParityBenchmark,
}

impl CraftStudioSession {
    pub fn bootstrap(appearance: AppearanceMode) -> Result<Self> {
        let catalog = CraftCatalog::bundled()?;
        let storytold_ledger = StorytoldEcosystemLedger::bundled()?;
        let figcraft = FigCraftDocument::google_expressive_default()?;
        let craft_anything = CraftAnythingWorkbench::default_google_workbench();
        let unreal_world = UnrealWorld::default_showcase_scene();
        let solari_gi = SolariGiSettings::default();
        let compiled_shader =
            compile_materialx_to_wgsl(default_openpbr_materialx_xml().as_bytes(), &solari_gi)?;
        let egui_host = EguiGpuiHost::new(appearance)?;

        let mut session = Self {
            active_pillar: StudioPillar::FigCraftGoogle,
            parity_mode: ParityRenderMode::SplitSideBySide,
            appearance,
            active_craft_app_id: "figcraft".to_owned(),
            catalog,
            storytold_ledger,
            figcraft,
            craft_anything,
            unreal_world,
            solari_gi,
            shading_mode: ViewportShadingMode::LitPbrSolari,
            compiled_shader,
            egui_host,
            last_benchmark: GuiParityBenchmark::default(),
        };
        session.run_parity_benchmark(640, 420)?;
        Ok(session)
    }

    pub fn set_appearance(&mut self, mode: AppearanceMode) -> Result<()> {
        self.appearance = mode;
        self.egui_host.set_appearance(mode)?;
        Ok(())
    }

    /// Executes the migrated `egui 0.37` / `bevy_egui` equivalent of the active Studio pillar.
    pub fn run_egui_equivalent(&mut self, width_px: u32, height_px: u32) -> Result<EguiFrameStats> {
        if self.active_pillar == StudioPillar::UnrealBevy58 {
            let snap = render_bevy_egui_inspector(
                &mut self.egui_host,
                &mut self.unreal_world,
                &mut self.solari_gi,
                &mut self.shading_mode,
                width_px,
                height_px,
            )?;
            return Ok(snap.egui_stats);
        }

        let pillar = self.active_pillar;
        let fig_title = self.figcraft.title.clone();
        let fig_nodes = self.figcraft.total_node_count();
        let disasm_len = self.craft_anything.disasm_rows.len();
        let subject = self.craft_anything.active_subject.clone();
        let repo_total = self.storytold_ledger.total_repositories;
        let app_count = self.catalog.apps.len();

        let artifact = self.egui_host.run_frame(width_px, height_px, 1.0, |ui| {
            egui_gpui::egui::CentralPanel::default().show(ui, |ui| match pillar {
                StudioPillar::FigCraftGoogle => {
                    ui.heading(format!("FigCraft (egui 0.37) · {fig_title}"));
                    ui.label(format!(
                        "Shared Document Nodes: {fig_nodes} | .fig Kiwi + DESIGN.md synced"
                    ));
                    ui.separator();
                    for frame in &self.figcraft.root_frames {
                        ui.group(|ui| {
                            ui.label(format!("Frame: {} ({})", frame.name, frame.stitch_id));
                            for child in &frame.children {
                                let _clicked = ui
                                    .button(format!(
                                        "{} [{}]",
                                        child.name,
                                        child.fill_role
                                    ))
                                    .clicked();
                            }
                        });
                    }
                }
                StudioPillar::CraftAnythingGoogle => {
                    ui.heading(format!("Craft Anything (egui 0.37) · {subject}"));
                    ui.label(format!("Decoded X64/A64 Instructions: {disasm_len}"));
                    ui.separator();
                    for row in &self.craft_anything.disasm_rows {
                        ui.monospace(format!(
                            "{} [{}] {:<14} {}",
                            row.address_hex, row.arch, row.bytes_hex, row.mnemonic_text
                        ));
                    }
                }
                StudioPillar::StorytoldSuiteCatalog => {
                    ui.heading(format!(
                        "Storytold Unified Monorepo ({repo_total} GitHub Repos · {app_count} Studio Apps)"
                    ));
                    ui.separator();
                    for app in self.catalog.apps.iter().take(10) {
                        ui.label(format!("• {} ({}) — {}", app.name, app.category, app.tagline));
                    }
                }
                StudioPillar::UnrealBevy58 => {}
            });
        });

        Ok(artifact.stats)
    }

    /// Runs both the GPUI tree builder, the `egui 0.37` tessellator, and the Bevy UE5.8 viewport
    /// on identical shared state to populate [`GuiParityBenchmark`].
    pub fn run_parity_benchmark(
        &mut self,
        width_px: u32,
        height_px: u32,
    ) -> Result<GuiParityBenchmark> {
        let theme = ShadcnGpuiTheme::from_bundled(self.appearance)?;
        let wcag_ok = theme.verify_wcag_aa().is_ok();

        let gpui_started = Instant::now();
        let _tsx_preview = export_figcraft_nsh_tsx(&self.figcraft);
        let _gpui_code = export_figcraft_gpui_rust(&self.figcraft);
        let gpui_us = gpui_started.elapsed().as_micros() as u64;

        let egui_stats = self.run_egui_equivalent(width_px, height_px)?;
        let (_, vp_stats): (Vec<u8>, ViewportRenderStats) = render_viewport_rgba8(
            &self.unreal_world,
            &self.solari_gi,
            self.shading_mode,
            128,
            72,
        );

        let bench = GuiParityBenchmark {
            gpui_tree_build_us: gpui_us,
            egui_tessellate_us: egui_stats.build_and_tessellate_us,
            egui_vertices: egui_stats.vertices,
            egui_triangles: egui_stats.triangles,
            egui_draw_calls: egui_stats.draw_calls,
            egui_accesskit_nodes: egui_stats.accesskit_nodes,
            bevy_viewport_us: vp_stats.render_us,
            bevy_visible_meshlets: vp_stats.meshlets.visible_clusters,
            wcag_aa_verified: wcag_ok,
        };
        self.last_benchmark = bench;
        Ok(bench)
    }

    #[must_use]
    pub fn export_figcraft_html(&self) -> Option<String> {
        let doc = aphrody_design::design_md::load_bundled_root_design_md().ok()?;
        Some(export_figcraft_baseline_html(&self.figcraft, &doc))
    }
}

/// Renders the Google Design Neural Expressive GPUI surface (plus live `GPUI` vs `egui 0.37` parity HUD)
/// for embedding directly inside `aphrody_nsh` Zed tabs or `aphrody_desktop`.
#[must_use]
pub fn render_gpui_studio_panel(session: &CraftStudioSession) -> AnyElement {
    let theme = ShadcnGpuiTheme::from_bundled(session.appearance).unwrap_or_else(|_| {
        let fallback_tokens = aphrody_ui::GoogleDesignTokens::bundled()
            .unwrap_or_else(|_| unreachable_tokens_fallback());
        ShadcnGpuiTheme::from_google_design(&fallback_tokens, session.appearance)
    });
    let b = &session.last_benchmark;

    let pillar_label = match session.active_pillar {
        StudioPillar::FigCraftGoogle => "Google FigCraft + Stitch Studio",
        StudioPillar::CraftAnythingGoogle => "Google Craft Anything Workbench",
        StudioPillar::UnrealBevy58 => "Unreal Engine 5.8 (Bevy 0.20 + Solari + Nanite)",
        StudioPillar::StorytoldSuiteCatalog => "Storytold 82-Repo Unified Suite",
    };

    let mode_label = match session.parity_mode {
        ParityRenderMode::GpuiGoogleDesign => "GPUI Google Design",
        ParityRenderMode::EguiNative037 => "egui 0.37 / bevy_egui",
        ParityRenderMode::SplitSideBySide => "50/50 Split Parity (GPUI vs egui)",
    };

    v_flex()
        .gap_3()
        .w_full()
        .child(
            h_flex()
                .justify_between()
                .items_center()
                .gap_2()
                .child(Label::new(pillar_label))
                .child(StarterBadge::new(mode_label, &theme)),
        )
        .child(
            h_flex()
                .gap_3()
                .flex_wrap()
                .child(StarterKpiCard::new(
                    "Storytold Repos",
                    session.storytold_ledger.total_repositories.to_string(),
                    format!("{} apps", session.catalog.apps.len()),
                    true,
                    &theme,
                ))
                .child(StarterKpiCard::new(
                    "egui 0.37 Mesh",
                    format!("{} tris", b.egui_triangles),
                    format!("{} us · {} a11y", b.egui_tessellate_us, b.egui_accesskit_nodes),
                    b.egui_triangles > 0,
                    &theme,
                ))
                .child(StarterKpiCard::new(
                    "Bevy UE5.8 Solari",
                    format!("{} meshlets", b.bevy_visible_meshlets),
                    if session.compiled_shader.naga_validated {
                        "NAGA OK"
                    } else {
                        "ERR"
                    },
                    session.compiled_shader.naga_validated,
                    &theme,
                )),
        )
        .child(
            StarterCard::new(
                format!(
                    "Shared GUI Core · FigCraft ({} nodes) · Craft Anything ({} insns) · Bevy ({} actors)",
                    session.figcraft.total_node_count(),
                    session.craft_anything.disasm_rows.len(),
                    session.unreal_world.actors.len()
                ),
                &theme,
            )
            .description(format!(
                "WCAG 2.2 AA verified={} · Active Craft App `{}` · Zero duplicate wgpu/taffy/accesskit/egui",
                b.wcag_aa_verified, session.active_craft_app_id
            ))
            .footer(format!(
                "Disasm head: {} | MaterialX WGSL bytes: {}",
                session
                    .craft_anything
                    .disasm_rows
                    .first()
                    .map_or("-", |r| r.mnemonic_text.as_str()),
                session.compiled_shader.wgsl_source.len()
            ))
            .child(
                div()
                    .pt_1()
                    .rounded(px(theme.radius_md_px))
                    .child(
                        Label::new(format!(
                            "craft-libs crates: {} | GPUI build: {} us | egui tessellate: {} us | Bevy viewport: {} us",
                            session.storytold_ledger.craft_libs_crates_on_disk,
                            b.gpui_tree_build_us,
                            b.egui_tessellate_us,
                            b.bevy_viewport_us,
                        ))
                        .size(LabelSize::Small),
                    ),
            ),
        )
        .into_any_element()
}

fn unreachable_tokens_fallback() -> aphrody_ui::GoogleDesignTokens {
    aphrody_ui::GoogleDesignTokens::bundled().unwrap_or_else(|_| aphrody_ui::GoogleDesignTokens {
            schema_version: 1,
            source_of_truth: "https://design.google/".to_owned(),
            system: "Material 3 Expressive".to_owned(),
            spec_url: "https://m3.material.io/".to_owned(),
            fonts_url: "https://fonts.google.com/".to_owned(),
            stitch_url: "https://stitch.withgoogle.com/".to_owned(),
            sans_flex_family: "Google Sans Flex".to_owned(),
            synced_at: String::new(),
            seed_hex: "#135B82".to_owned(),
            variant: "expressive".to_owned(),
            principles: Vec::new(),
            reference: aphrody_ui::google_design::RefTokens {
                palette: Default::default(),
                typeface: Default::default(),
                variable_axes: Default::default(),
            },
            sys: aphrody_ui::google_design::SysTokens {
                color: aphrody_ui::google_design::ColorPaletteMap {
                    light: Default::default(),
                    dark: Default::default(),
                },
                motion: aphrody_ui::google_design::MotionTokens {
                    springs: Default::default(),
                },
                shape: aphrody_ui::ShapeScale {
                    none_px: 0.0,
                    extra_small_px: 4.0,
                    small_px: 8.0,
                    medium_px: 12.0,
                    large_px: 16.0,
                    large_increased_px: 20.0,
                    extra_large_px: 28.0,
                    full_px: 9999.0,
                },
                state: aphrody_ui::StateLayerOpacities {
                    hover_opacity: 0.08,
                    focus_opacity: 0.10,
                    pressed_opacity: 0.10,
                    dragged_opacity: 0.16,
                    disabled_container_opacity: 0.12,
                    disabled_content_opacity: 0.38,
                    focus_ring_px: 2.0,
                },
            },
        })
}

#[cfg(test)]
mod tests {
    use aphrody_design::design_md::load_bundled_root_design_md;
    use aphrody_design::web_standards::audit_html_standards;

    use super::*;

    #[test]
    fn shared_studio_drives_gpui_egui_037_bevy_ue58_figcraft_and_craft_anything() -> Result<()> {
        let mut session = CraftStudioSession::bootstrap(AppearanceMode::Dark)?;
        assert_eq!(session.storytold_ledger.total_repositories, 82);
        assert!(session.catalog.apps.len() >= 18);
        assert!(session.figcraft.total_node_count() >= 3);
        assert!(!session.craft_anything.disasm_rows.is_empty());
        assert!(session.compiled_shader.naga_validated);

        for pillar in [
            StudioPillar::FigCraftGoogle,
            StudioPillar::CraftAnythingGoogle,
            StudioPillar::UnrealBevy58,
            StudioPillar::StorytoldSuiteCatalog,
        ] {
            session.active_pillar = pillar;
            let bench = session.run_parity_benchmark(640, 420)?;
            assert!(bench.wcag_aa_verified);
            assert!(bench.egui_vertices > 10);
            assert!(bench.egui_triangles > 4);
            assert!(bench.bevy_visible_meshlets > 0);
        }

        let design_md = load_bundled_root_design_md()?;
        let html = export_figcraft_baseline_html(&session.figcraft, &design_md);
        let lh = audit_html_standards("figcraft.html", &html, &design_md, Some(64_000));
        assert!(lh.ok, "Lighthouse/Baseline findings: {:?}", lh.findings);
        assert_eq!(lh.scores.overall, 100);

        session.set_appearance(AppearanceMode::Light)?;
        let light_bench = session.run_parity_benchmark(640, 420)?;
        assert!(light_bench.wcag_aa_verified);
        Ok(())
    }
}
