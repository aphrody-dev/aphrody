//! Unified ArtCraft Studio catalog, `craft-libs` multi-format pipeline, and Craft Anything MCP engine.

pub mod catalog;
pub mod mcp;
pub mod pipeline;

pub use catalog::{BUNDLED_CATALOG_TOML, CraftAppSpec, CraftCatalog, WorkspaceDefaults};
pub use mcp::{McpServerState, McpToolDescriptor, handle_json_rpc_line, serve_http, serve_stdio};
#[cfg(feature = "codecs")]
pub use pipeline::render_svg_rgba;
pub use pipeline::{BinaryInspection, InspectionReport, LayerSummary, inspect_bytes, inspect_path};
