//! OS session management, compositor geometry, and application discovery for Aphrody GPUI OS.
//!
//! Pure Rust and UI-toolkit agnostic: usable identically from native GPUI shells, headless
//! CLI/daemon supervisors, MCP control servers, and Wasm environments.

pub mod compositor;
pub mod discovery;
pub mod session;

pub use compositor::{
    CompositorState, LayerAnchor, LayerKind, LayerSurface, PlacedWindow, Rect, TilingMode,
    ToplevelId, ToplevelState, WorkspaceId, WorkspaceState, layout_windows,
};
pub use discovery::{AppEntry, AppOrigin, DistBundleSpec, DistFormat, discover_system_apps};
pub use session::{
    RestartPolicy, SessionEnvironment, SessionError, SessionManifest, SessionSettings,
    SessionSupervisor, SessionUnitConfig, SessionUnitStatus, UnitKind, UnitPhase,
};
