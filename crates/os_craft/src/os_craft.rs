//! Unified ArtCraft Studio catalog, `storytold` 82-repo `craft-libs` & `craft-anything` pipeline, and OS MCP engine.

pub mod catalog;
pub mod mcp;
pub mod pipeline;
pub mod storytold_suite;

pub use catalog::{BUNDLED_CATALOG_TOML, CraftAppSpec, CraftCatalog, WorkspaceDefaults};
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
