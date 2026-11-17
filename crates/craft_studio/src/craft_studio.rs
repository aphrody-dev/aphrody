//! Unified Google Design + Stitch Studio Core (`craft_studio`) uniting:
//! - [`figcraft_core`]: Clean-room FigCraft Google Edition (`.fig` Kiwi/ZIP, `.sketch`, `.penpot`,
//!   `DESIGN.md` token binding, `data-stitch-id` round-trip, and NSH TSX / GPUI Rust / Baseline HTML codegen).
//! - [`craft_anything_core`]: Clean-room Craft Anything Google Edition (`craft-anything-disasm` X64/A64
//!   CFG/disassembly, `craft-anything-js` `oxc` AST/bundle analyzer, `craft-anything-evidence` SHA-256 ledger).
//! - [`dual_frontend`]: Shared GUI core (`CraftStudioSession`) rendered simultaneously through
//!   **Google Design GPUI (`os_ui`)** and **Migrated `egui 0.37` / `bevy_egui` (`egui_gpui` + `bevy_gpui`)**
//!   for live visual & performance parity benchmarking across all 18 Storytold apps.

pub mod craft_anything_core;
pub mod dual_frontend;
pub mod figcraft_core;

pub use craft_anything_core::{
    CraftAnythingWorkbench, DisassembledInsnRow, EvidenceLedgerEntry, JsAstReport,
};
pub use dual_frontend::{
    CraftStudioSession, GuiParityBenchmark, ParityRenderMode, StudioPillar,
    render_gpui_studio_panel,
};
pub use figcraft_core::{
    AutoLayoutAxis, FigCraftDocument, FigCraftFrameNode, FigTokenBinding,
    export_figcraft_baseline_html, export_figcraft_gpui_rust, export_figcraft_nsh_tsx,
};
