# `agent-desktop` Purge Manifest & Zed/GPUI Fusion Map

## Purged Categories (Dropped from `aphrody-labs/agent-desktop`)

| Source Path | Blob Count / Size | Reason Purged | Aphrody Replacement |
|---|---|---|---|
| `assets/` | 7 files (~37.2 MB) | Heavy marketing MP4/GIF/PNG assets | Live GPUI `visual_smoke` PNGs + `screenshot` |
| `apps/docs/` | 102 files (~1.16 MB) | Vercel Next.js Geist docs website | `docs/aphrody/architecture/*.md` + GitIngest/skills |
| `probes/` | 345 files (~5.52 MB) | Historical exploratory PowerShell/Python scripts | Distilled into `backend_*.rs` + `platforms-matrix.md` |
| `crates/ffi/` + `tests/ffi-python/` | 201 files (~648 KB) | External C ABI (`cdylib`, `cbindgen`, Python `ctypes`) | Pure Rust `aphrody_pilot` + MCP JSON-RPC + Bun TS |
| `tests/fixture-app*`, `tests/native/`, `tests/real-apps/`, `tests/linux-dist/` | ~65 files (~315 KB) | Multi-language C#/Swift/ObjC/C test rigs | Headless GPUI `AccessKit` tests + `cargo test -p aphrody_pilot` |
| `npm/`, `providers/macos/` | 12 files (~144 KB) | Standalone npm wrapper + VM guest images | `crates/aphrody_store` (22 package targets) + `fleet` |
| `crates/*/cursor_overlay*` | 80+ files | Legacy GDI/Cocoa/X11 floating cursor overlay windows | `aphrody_desktop::pilot_hud` native GPUI compositor HUD |
| `128 x mod.rs` micro-files | ~1,400 tiny files | Anti-Zed `mod.rs` nesting & duplicate `redundant_clone` | Clean `aphrody_pilot.rs` + 10 cohesive Zed-style modules |

## Fused Capabilities in `aphrody-os`

- **Single MCP Server (`crates/aphrody_craft/src/mcp.rs`)**: Combines `os_*` workspace/compositor/session + `craft_*` codecs/sniffing + 55+ `desktop_*` & `desktop_ai_act_verify` computer-use tools.
- **Zed Dependency Alignment**: Reuses Aphrody's existing `windows 0.62`, `windows-sys 0.61`, `zbus 5.13`, `x11rb 0.13`, `core-graphics`, `accesskit 0.25`, `image 0.25`, `smol 2.0`, and `web-time 1.1` with zero duplicate crates.
