//! OS session management, compositor geometry, and application discovery for Aphrody GPUI OS.
//!
//! Pure Rust and UI-toolkit agnostic: usable identically from native GPUI shells, headless
//! CLI/daemon supervisors, MCP control servers, and Wasm environments.

pub mod compositor;
pub mod discovery;
pub mod i18n;
pub mod session;
pub mod wasm_hal;

pub use compositor::{
    CompositorState, LayerAnchor, LayerKind, LayerSurface, PlacedWindow, Rect, TilingMode,
    ToplevelId, ToplevelState, WorkspaceId, WorkspaceState, layout_windows,
};
pub use discovery::{AppEntry, AppOrigin, DistBundleSpec, DistFormat, discover_system_apps};
pub use i18n::{
    Bcp47Tag, BidiHazard, BidiHazardKind, LocalizedString, NegotiatedLocale, TextDir,
    detect_process_locale, is_rtl_language, negotiate_locale, scan_bidi_hazards, strong_direction,
    validate_utf8_no_bom, wrap_bidi_isolate,
};
pub use session::{
    HardwareCapabilities, RestartPolicy, SessionEnvironment, SessionError, SessionManifest,
    SessionSettings, SessionSupervisor, SessionUnitConfig, SessionUnitStatus, UnitKind, UnitPhase,
};
pub use wasm_hal::{
    CallingConvention, IsaKind, IsaProfile, PAGE_SIZE_4K, PAGE_SIZE_16K, WASM_BINARY_VERSION,
    WASM_CORE2_NOOP_WAT, WASM_CORE2_SIMD_PROBE_BYTES, WASM_CORE2_SIMD_PROBE_WAT,
    WASM_LINEAR_PAGE_BYTES, WASM_MAGIC, WasmBinaryError, WasmCore2Header, WasmSectionHeader,
    WasmSectionId, WasmValueType, blit_rgba_alpha_add_scalar, blit_rgba_alpha_add_simd128,
    decode_uleb128_u32, encode_uleb128_u32, encode_wasm_core2_noop_module,
    parse_custom_section_name, parse_wasm_core2_header, read_timestamp_ticks, spin_wait_hint,
    wasm_linear_memory_pages, xor_checksum_words, xor_checksum_words_scalar,
};
