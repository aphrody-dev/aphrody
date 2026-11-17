# Full `agent-desktop` Purge, Migration & AI OS Pilotage Architecture

> Unified computer-use and whole-OS AI control layer migrating [`aphrody-labs/agent-desktop`](https://github.com/aphrody-labs/agent-desktop) into Zed/GPUI idioms inside Aphrody GPUI OS (`os_pilot` + `os_craft` + `os_desktop`).

## 1. Full Repository Audit & Purge Accounting

Scanning `aphrody-labs/agent-desktop@main` (`3,801` blobs across `~51.2 MB`) separated high-value OS Accessibility/Automation logic from Vercel-inherited site assets, multi-language FFI harnesses, historical probes, and anti-Zed file fragmentation:

| Category | Upstream Count | Upstream Size | Action | Aphrody Destination / Rationale |
|---|---:|---:|---|---|
| **Marketing Media (`assets/`)** | 7 files | 37,212 KB | **Purged** | Video/GIF bloat removed; GPUI visual smoke & PNG capture handle proof. |
| **Docs Web App (`apps/docs/`)** | 102 files | 1,162 KB | **Purged** | Next.js/Vercel Geist template replaced by [`docs/architecture/`](./README.md). |
| **Historical Probes (`probes/`)** | 345 files | 5,518 KB | **Purged & Distilled** | Exploratory PS1/Python heuristics baked directly into [`backend_windows.rs`](../../crates/os_pilot/src/backend_windows.rs) & [`platforms-matrix.md`](../../.agents/plugins/aphrody/skills/ai-os-pilot/references/platforms-matrix.md). |
| **C ABI & Cross-Lang Rigs (`crates/ffi/`, `tests/fixture-app*`, `tests/native/`, `tests/real-apps/`, `tests/ffi-python/`)** | 253 files | ~900 KB | **Purged** | Aphrody is pure Rust + Wasm + MCP/JSON-RPC; UI tests run natively on GPUI + `AccessKit` (`visual_smoke`, `os_pilot`). |
| **Standalone Wrapper & VM (`npm/`, `providers/macos/`)** | 12 files | ~144 KB | **Purged** | Replaced by [`gpui_store`](../../crates/gpui_store/src/packagers.rs) (22 packagers) & `fleet:verify`. |
| **Legacy Cursor Overlay (`cursor_overlay/`)** | 80+ files | ~250 KB | **Replaced** | Replaced by native GPU compositor HUD [`pilot_hud.rs`](../../crates/os_desktop/src/pilot_hud.rs) in `os_desktop`. |
| **Redundant Community/Brand Templates (legacy skills, `CONCEPTS.md`, `CHANGELOG.md`, external web links)** | 15+ files | ~180 KB | **Purged** | Replaced by Aphrody brand/docs and concise skill references. |
| **Core + Native Adapters + CLI (`crates/{core,windows,linux,macos}`, `src/`)** | 2,530+ fragmented files (`128 mod.rs`) | ~11.8 MB | **Migrated, Consolidated & Fused** | Consolidated into 11 Zed-style modules (zero `mod.rs`) in [`crates/os_pilot`](../../crates/os_pilot/src/os_pilot.rs) reusing workspace `windows 0.62`, `zbus`, `x11rb`, `core-graphics`, `accesskit`, `image`, `smol`. |

## 2. Zed / GPUI Architectural Convergence

Instead of bolting a separate binary & second MCP server onto `aphrody-os`, `os_pilot` fuses **four layers** into one cohesive control plane:

1. **In-Process GPUI + AccessKit Bridge ([`gpui_bridge.rs`](../../crates/os_pilot/src/gpui_bridge.rs))**:
   - Consumes `accesskit::TreeUpdate` directly from GPUI's `a11y.rs` tree builder AND synchronizes with [`os_session::CompositorState`](../../crates/os_session/src/compositor.rs).
   - Compiles identically on Desktop, Headless Linux CI, and `wasm32` WebGPU (using `web-time` + `image` PNG encoder).
   - Translates AI actions (`click`, `toggle`, `set_value`, `expand`, `focus`) back into `accesskit::ActionRequest` events processed live by [`OsDesktopShell`](../../crates/os_desktop/src/shell.rs).
2. **Real Host OS Backends (Zero Duplicate Crates)**:
   - **Windows** ([`backend_windows.rs`](../../crates/os_pilot/src/backend_windows.rs)): Direct `windows 0.62` COM `CUIAutomation` (`Invoke`, `Value`, `Toggle`, `ExpandCollapse`, `SelectionItem`, `Scroll`, `ScrollItem`), Chromium `Chrome_WidgetWin_1` `WM_GETOBJECT` wakeup, `SendMessageTimeoutW` hang guard, `TokenIntegrityLevel` UIPI elevation check, `EnumDisplayMonitors` + `GetDpiForMonitor`, `SendInput` Unicode/VK/mouse synthesis, Win32 clipboard, GDI `BitBlt` PNG capture.
   - **Linux** ([`backend_linux.rs`](../../crates/os_pilot/src/backend_linux.rs)): `zbus` AT-SPI2 D-Bus tree walker & `DoAction`/`GrabFocus`/`SetTextContents`, `x11rb` X11 windows + `xtest` input + `get_image` PNG capture, Wayland detection.
   - **macOS** ([`backend_macos.rs`](../../crates/os_pilot/src/backend_macos.rs)): ApplicationServices `AXUIElement`, CoreGraphics `CGEvent` + `CGDisplay`, TCC Accessibility & Screen Recording checks.
3. **Unified MCP Server ([`os_craft::mcp`](../../crates/os_craft/src/mcp.rs))**:
   - Exposes `os_*` (workspaces, tiling layouts, themes, session), `craft_*` (codecs & file sniffing), all 55+ `desktop_*` computer-use tools, and **`desktop_ai_act_verify`** (1-shot observe -> locate -> act -> 50ms readback verify -> post-snapshot diff) over stdio JSON-RPC 2.0 and hardened local HTTP (`/mcp`, `/.well-known/webmcp.json`, `/health`).
   - Automatically synchronizes compositor workspaces/windows/themes between `McpServerState` and `PilotEngine.gpui`.
4. **Live GPUI Shell HUD & Unified CLI ([`os_desktop`](../../crates/os_desktop/src/os_desktop.rs))**:
   - `aphrody-os pilot <op>` exposes every AI pilot operation in single `2.6` JSON envelopes (`aphrody-os pilot status`, `aphrody-os pilot snapshot`, `aphrody-os pilot click ...`, `aphrody-os pilot ai-act ...`).
   - [`pilot_hud.rs`](../../crates/os_desktop/src/pilot_hud.rs) renders AI Pilot readiness and recent verified actions inside `OsDesktopShell`'s top panel / inspector drawer.

## 3. Reproducible Audit & Verification

Run the automated purge-and-parity verifier:

```bash
bun run audit:agent-desktop
bun test tooling/bun/agent-desktop-audit.test.ts
cargo test -j 4 -p os_pilot -p os_craft -p os_desktop
```
