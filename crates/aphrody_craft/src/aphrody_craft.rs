//! Unified ArtCraft Studio catalog, `storytold` 82-repo `craft-libs` & `craft-anything` pipeline,
//! FigCraft Google Edition, Craft Anything Disasm/JS/Evidence, Dual GPUI vs `egui` parity,
//! unified CLI/MCP/Web/Desktop exposure, and OS MCP engine.

pub mod catalog;
#[cfg(feature = "craft-anything")]
pub mod craft_anything_core;
#[cfg(feature = "studio")]
pub mod dual_frontend;
pub mod expose;
#[cfg(feature = "figcraft")]
pub mod figcraft_core;
#[cfg(feature = "studio")]
pub mod figcraft_gpui;
pub mod mcp;
pub mod pipeline;
pub mod storytold_suite;

pub use catalog::{BUNDLED_CATALOG_TOML, CraftAppSpec, CraftCatalog, WorkspaceDefaults};
#[cfg(feature = "craft-anything")]
pub use craft_anything_core::{
    CraftAnythingWorkbench, DisassembledInsnRow, EvidenceLedgerEntry, JsAstReport,
};
#[cfg(feature = "studio")]
pub use dual_frontend::{
    CraftStudioSession, GuiParityBenchmark, ParityRenderMode, StudioPillar,
    render_gpui_studio_panel, spawn_figcraft_studio_view,
};
pub use expose::{CraftExposureEntry, CraftExposureManifest, CraftSurfaceChannel};
#[cfg(feature = "figcraft")]
pub use figcraft_core::{
    AutoLayoutAxis, AutoLayoutMode, ComponentVariantDefinition, ComponentVariantSpec,
    FigCraftCommand, FigCraftDesignMode, FigCraftDocument, FigCraftExportTarget,
    FigCraftFrameNode, FigCraftNode, FigCraftNodeKind, FigCraftPage, FigCraftSnapshot,
    FigTokenBinding, LayoutAlignment, PaddingEdges, PageKind, SizingConstraint, SnapAxis,
    SnapGuideLine, SnapGuideResult, StarterWidgetTemplate, TemplateCategory, UndoRedoHistory,
    export_figcraft_baseline_html, export_figcraft_gpui_rust, export_figcraft_nsh_tsx,
    export_to_gpui_kit_rs, export_to_gpui_rs, export_to_lighthouse100_html,
    export_to_lighthouse100_html_bundled, export_to_nsh_rs_tsx, m3_expressive_starter_templates,
};
#[cfg(feature = "studio")]
pub use figcraft_gpui::{FigCraftInspectorTab, FigCraftStudioEvent, FigCraftStudioView};
pub use mcp::{McpServerState, McpToolDescriptor, handle_json_rpc_line};
#[cfg(not(target_family = "wasm"))]
pub use mcp::{serve_http, serve_stdio};
#[cfg(feature = "codecs")]
pub use pipeline::render_svg_rgba;
pub use pipeline::{BinaryInspection, InspectionReport, LayerSummary, inspect_bytes, inspect_path};
#[cfg(feature = "storytold-suite")]
pub use storytold_suite::{analyze_js_bundle_source, inspect_design_container};
pub use storytold_suite::{
    BUNDLED_STORYTOLD_ECOSYSTEM_TOML, DedupRuleSpec, DesignContainerSummary,
    StorytoldEcosystemLedger, StorytoldRepoSpec,
};
