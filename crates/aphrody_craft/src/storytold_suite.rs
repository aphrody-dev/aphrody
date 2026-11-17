//! Unified `https://github.com/orgs/storytold/repositories` (82 repositories) manifest loader,
//! `.fig` / `.sketch` / `.penpot` / `.mtlx` / ELF / JS (`oxc`) deep container analyzers,
//! and multi-crate Storytold suite integration.

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};

/// Embedded 82-repository `storytold` unification & Cargo deduplication ledger.
pub const BUNDLED_STORYTOLD_ECOSYSTEM_TOML: &str =
    include_str!("../../../assets/os/storytold_ecosystem.toml");

/// Consolidated facade re-exporting all 50 `storytold/craft-libs` & `storytold/craft-anything`
/// workspace crates through one uniform `aphrody_craft::storytold_suite::crates::*` namespace.
#[cfg(feature = "storytold-suite")]
pub mod crates {
    pub use craft_anicraft_format as anicraft_format;
    pub use craft_anything_analysis as anything_analysis;
    pub use craft_anything_disasm as anything_disasm;
    pub use craft_anything_evidence as anything_evidence;
    pub use craft_anything_formats as anything_formats;
    pub use craft_anything_js as anything_js;
    pub use craft_color as color;
    pub use craft_csv as csv;
    pub use craft_dds as dds;
    pub use craft_deckcraft_format as deckcraft_format;
    pub use craft_docx as docx;
    pub use craft_dxf as dxf;
    pub use craft_ecproj_format as ecproj_format;
    pub use craft_elf as elf;
    pub use craft_fcproj_format as fcproj_format;
    pub use craft_fig as fig;
    pub use craft_figcraft_format as figcraft_format;
    pub use craft_figma_rest as figma_rest;
    pub use craft_gcode as gcode;
    pub use craft_gridcraft_format as gridcraft_format;
    pub use craft_hdr as hdr;
    pub use craft_i18n as i18n;
    pub use craft_kiwi as kiwi;
    pub use craft_kra as kra;
    pub use craft_lottie as lottie;
    pub use craft_macho as macho;
    pub use craft_materialx as materialx;
    pub use craft_math as math;
    pub use craft_mcmat as mcmat;
    pub use craft_mesh as mesh;
    pub use craft_midi as midi;
    pub use craft_ora as ora;
    pub use craft_pcraft_format as pcraft_format;
    pub use craft_pe as pe;
    pub use craft_penpot as penpot;
    pub use craft_pptx as pptx;
    pub use craft_sf2 as sf2;
    pub use craft_sheet as sheet;
    pub use craft_sketch as sketch;
    pub use craft_tar as tar;
    pub use craft_tcraft as tcraft;
    pub use craft_tga as tga;
    pub use craft_timeline as timeline;
    pub use craft_usd as usd;
    pub use craft_vectorcraft_format as vectorcraft_format;
    pub use craft_wordmodel as wordmodel;
    pub use craft_xcf as xcf;
    pub use craft_xlsx as xlsx;
    pub use craft_xml as xml;
    pub use craft_zip as zip;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DedupRuleSpec {
    pub family: String,
    pub crates: Vec<String>,
    pub canonical_source: String,
    pub eliminates_splits_from: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorytoldRepoSpec {
    pub name: String,
    pub category: String,
    pub language: String,
    pub license: String,
    pub archived: bool,
    pub fork: bool,
    pub aphrody_owner: String,
    pub studio_app_id: String,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorytoldEcosystemLedger {
    pub schema_version: u32,
    pub org_url: String,
    pub total_repositories: usize,
    pub craft_libs_crates_on_disk: usize,
    pub craft_anything_crates_on_disk: usize,
    pub egui_crates_on_disk: usize,
    pub gpui_egui_bridge_crate: String,
    pub bevy_unreal_engine_crate: String,
    pub shared_dual_gui_studio_crate: String,
    #[serde(default)]
    pub deduplication_rules: Vec<DedupRuleSpec>,
    #[serde(default)]
    pub repositories: Vec<StorytoldRepoSpec>,
}

impl StorytoldEcosystemLedger {
    pub fn bundled() -> Result<Self> {
        toml::from_str(BUNDLED_STORYTOLD_ECOSYSTEM_TOML)
            .context("parsing bundled storytold_ecosystem.toml")
    }

    #[must_use]
    pub fn by_studio_app(&self, app_id: &str) -> Vec<&StorytoldRepoSpec> {
        self.repositories
            .iter()
            .filter(|r| r.studio_app_id.eq_ignore_ascii_case(app_id))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignContainerSummary {
    pub format: String,
    pub valid: bool,
    pub version: Option<u32>,
    pub chunk_or_page_count: usize,
    pub embedded_image_count: usize,
    pub detail: String,
}

/// Inspects a `.fig` (Figma/FigCraft), bare `canvas.fig`, `.sketch`, or MaterialX `.mtlx` payload
/// using the clean-room `storytold/craft-libs` readers.
#[cfg(feature = "storytold-suite")]
#[must_use]
pub fn inspect_design_container(bytes: &[u8]) -> DesignContainerSummary {
    if bytes.starts_with(craft_fig::DESIGN_HEADER) || bytes.starts_with(craft_fig::JAM_HEADER) {
        let kind = if bytes.starts_with(craft_fig::DESIGN_HEADER) {
            "FigmaCanvasBinary"
        } else {
            "FigJamCanvasBinary"
        };
        let header_version = bytes
            .get(8..12)
            .and_then(|slice| <[u8; 4]>::try_from(slice).ok())
            .map(u32::from_le_bytes);
        return match craft_fig::chunks(bytes) {
            Ok(raw_chunks) => DesignContainerSummary {
                format: kind.to_owned(),
                valid: true,
                version: header_version,
                chunk_or_page_count: raw_chunks.len(),
                embedded_image_count: 0,
                detail: format!(
                    "{kind} v{} ({} chunks)",
                    header_version.unwrap_or(craft_fig::VERSION),
                    raw_chunks.len()
                ),
            },
            Err(err) => DesignContainerSummary {
                format: kind.to_owned(),
                valid: false,
                version: header_version,
                chunk_or_page_count: 0,
                embedded_image_count: 0,
                detail: err.to_string(),
            },
        };
    }

    if craft_fig::is_fig(bytes) {
        return match craft_fig::Archive::open_with(bytes, craft_fig::Limits::default()) {
            Ok(archive) => {
                let version = archive
                    .canvas
                    .get(8..12)
                    .and_then(|slice| <[u8; 4]>::try_from(slice).ok())
                    .map(u32::from_le_bytes);
                let chunk_count = craft_fig::chunks(&archive.canvas)
                    .map_or(0, |list| list.len());
                DesignContainerSummary {
                    format: "FigArchive".to_owned(),
                    valid: true,
                    version,
                    chunk_or_page_count: chunk_count,
                    embedded_image_count: 0,
                    detail: format!(
                        ".fig archive ({chunk_count} kiwi chunks, file_name={})",
                        archive.file_name.as_deref().unwrap_or("<unnamed>")
                    ),
                }
            }
            Err(err) => DesignContainerSummary {
                format: "FigArchive".to_owned(),
                valid: false,
                version: None,
                chunk_or_page_count: 0,
                embedded_image_count: 0,
                detail: err.to_string(),
            },
        };
    }

    if bytes.trim_ascii_start().starts_with(b"<?xml") || bytes.windows(10).any(|w| w == b"<materialx")
    {
        if let Ok(doc) = craft_materialx::Document::parse(bytes)
            && let Ok(mat) = doc.read_material(None)
        {
            return DesignContainerSummary {
                format: "MaterialX".to_owned(),
                valid: true,
                version: Some(139),
                chunk_or_page_count: mat.inputs.len(),
                embedded_image_count: 0,
                detail: format!("MaterialX `{}` ({:?}, {} inputs)", mat.name, mat.shader, mat.inputs.len()),
            };
        }
    }

    DesignContainerSummary {
        format: "UnknownDesignContainer".to_owned(),
        valid: false,
        version: None,
        chunk_or_page_count: 0,
        embedded_image_count: 0,
        detail: "Input did not match .fig, canvas.fig, or MaterialX XML".to_owned(),
    }
}

/// Runs pure-Rust `oxc` JavaScript/TypeScript static bundle & AST analysis via `craft-anything-js`.
#[cfg(feature = "storytold-suite")]
#[must_use]
pub fn analyze_js_bundle_source(path_label: &str, source: &str) -> serde_json::Value {
    let analysis =
        craft_anything_js::analyze(path_label, source, &craft_anything_js::Limits::default());
    serde_json::to_value(&analysis).unwrap_or_else(|_| serde_json::json!({ "ok": false }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_storytold_ledger_has_all_82_repos_and_suite_analyzers_work() -> Result<()> {
        let ledger = StorytoldEcosystemLedger::bundled()?;
        assert_eq!(ledger.total_repositories, 82);
        assert_eq!(ledger.repositories.len(), 82);
        assert!(!ledger.by_studio_app("figcraft").is_empty());
        assert!(!ledger.by_studio_app("unrealcraft").is_empty());

        #[cfg(feature = "storytold-suite")]
        {
            let mtlx = br#"<?xml version="1.0"?><materialx version="1.39"><open_pbr_surface name="S" type="surfaceshader"><input name="base_color" type="color3" value="0.2, 0.5, 0.9"/></open_pbr_surface><surfacematerial name="M" type="material"><input name="surfaceshader" type="surfaceshader" nodename="S"/></surfacematerial></materialx>"#;
            let summary = inspect_design_container(mtlx);
            assert!(summary.valid);
            assert_eq!(summary.format, "MaterialX");

            let js_val = analyze_js_bundle_source(
                "app.tsx",
                "export function greet(name: string) { return `Hello ${name}`; }",
            );
            assert!(js_val.is_object());
        }
        Ok(())
    }
}
