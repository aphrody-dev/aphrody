# Full `agent-desktop` & `agent-browser` Fusion into AI OS Pilotage Architecture

> Unified computer-use, browser automation, and whole-OS AI control layer fusing upstream `agent-desktop` (`1.3.1`) and `agent-browser` (`0.43.0`) into Zed/GPUI idioms across [`crates/aphrody_browser`](../../../crates/aphrody_browser/src/aphrody_browser.rs), [`crates/aphrody_pilot`](../../../crates/aphrody_pilot/src/aphrody_pilot.rs), [`crates/aphrody_craft`](../../../crates/aphrody_craft/src/mcp.rs), and [`crates/aphrody_desktop`](../../../crates/aphrody_desktop/src/main.rs).

## 1. Full Repository Audit & Purge Accounting

Auditing `aphrody-labs/agent-desktop@main` (`3,801` blobs across `~51.2 MB`) separated OS Accessibility, Automation, and browser-control logic from site media, multi-language FFI harnesses, historical probes, and fragmented `mod.rs` layouts:

| Category | Upstream Count | Upstream Size | Action | Aphrody Destination / Rationale |
|---|---:|---:|---|---|
| **Marketing Media (`assets/`)** | 7 files | 37,212 KB | **Purged** | Video/GIF bloat removed; GPUI visual smoke & PNG capture handle verification. |
| **Docs Web App (`apps/docs/`)** | 102 files | 1,162 KB | **Purged** | Upstream web docs template replaced by [`aphrody/docs/architecture/`](./README.md). |
| **Historical Probes (`probes/`)** | 345 files | 5,518 KB | **Purged & Distilled** | Exploratory PS1/Python heuristics folded into [`backend_windows.rs`](../../../crates/aphrody_pilot/src/backend_windows.rs) & [`platforms-matrix.md`](../../../.agents/plugins/aphrody/skills/ai-os-pilot/references/platforms-matrix.md). |
| **C ABI & Cross-Lang Rigs (`crates/ffi/`, `tests/fixture-app*`, `tests/native/`, `tests/real-apps/`, `tests/ffi-python/`)** | 253 files | ~900 KB | **Purged** | Aphrody is pure Rust + Wasm + MCP/JSON-RPC; UI tests run natively on GPUI + `AccessKit`. |
| **Standalone Wrapper & VM (`npm/`, `providers/macos/`)** | 12 files | ~144 KB | **Purged** | Replaced by [`aphrody_store`](../../../crates/aphrody_store/src/packagers.rs) (22 packagers) & `fleet:verify`. |
| **Legacy Cursor Overlay (`cursor_overlay/`)** | 80+ files | ~250 KB | **Replaced** | Replaced by native GPU compositor HUD [`pilot_hud.rs`](../../../crates/aphrody_desktop/src/pilot_hud.rs) in `aphrody_desktop`. |
| **Redundant Community/Brand Templates (legacy skills, `CONCEPTS.md`, `CHANGELOG.md`, external web links)** | 15+ files | ~180 KB | **Purged** | Replaced by Aphrody brand/docs and concise skill references. |
| **Core + Native Adapters + CLI (`crates/{core,windows,linux,macos}`, `src/`)** | 2,530+ fragmented files (`128 mod.rs`) | ~11.8 MB | **Migrated, Consolidated & Fused** | Consolidated into Zed-style modules (zero `mod.rs`) across [`crates/aphrody_pilot`](../../../crates/aphrody_pilot/src/aphrody_pilot.rs), [`crates/aphrody_browser`](../../../crates/aphrody_browser/src/aphrody_browser.rs), and [`crates/aphrody_desktop`](../../../crates/aphrody_desktop/src/main.rs), reusing workspace `windows 0.62`, `zbus`, `x11rb`, `core-graphics`, `accesskit`, `image`, `smol`. |

## 2. Zed / GPUI Architectural Convergence

`aphrody_pilot`, `aphrody_browser`, and `aphrody_desktop` fuse **five layers** into one cohesive control plane:

1. **In-Process GPUI + AccessKit Bridge ([`gpui_bridge.rs`](../../../crates/aphrody_pilot/src/gpui_bridge.rs))**:
   - Consumes `accesskit::TreeUpdate` directly from GPUI's `a11y.rs` tree builder AND synchronizes with [`aphrody_session::CompositorState`](../../../crates/aphrody_session/src/compositor.rs).
   - Compiles identically on Desktop, Headless Linux CI, and `wasm32` WebGPU (using `web-time` + `image` PNG encoder).
   - Translates AI actions (`click`, `toggle`, `set_value`, `expand`, `focus`) back into `accesskit::ActionRequest` events processed live by [`OsDesktopShell`](../../../crates/aphrody_desktop/src/shell.rs).
2. **Real Host OS Backends (Zero Duplicate Crates)**:
   - **Windows** ([`backend_windows.rs`](../../../crates/aphrody_pilot/src/backend_windows.rs)): Direct `windows 0.62` COM `CUIAutomation` (`Invoke`, `Value`, `Toggle`, `ExpandCollapse`, `SelectionItem`, `Scroll`, `ScrollItem`), Chromium `Chrome_WidgetWin_1` `WM_GETOBJECT` wakeup, `SendMessageTimeoutW` hang guard, `TokenIntegrityLevel` UIPI elevation check, `EnumDisplayMonitors` + `GetDpiForMonitor`, `SendInput` Unicode/VK/mouse synthesis, Win32 clipboard, GDI `BitBlt` PNG capture.
   - **Linux** ([`backend_linux.rs`](../../../crates/aphrody_pilot/src/backend_linux.rs)): `zbus` AT-SPI2 D-Bus tree walker & `DoAction`/`GrabFocus`/`SetTextContents`, `x11rb` X11 windows + `xtest` input + `get_image` PNG capture, Wayland detection.
   - **macOS** ([`backend_macos.rs`](../../../crates/aphrody_pilot/src/backend_macos.rs)): ApplicationServices `AXUIElement`, CoreGraphics `CGEvent` + `CGDisplay`, TCC Accessibility & Screen Recording checks.
3. **Fused `agent-browser 0.43.0` + `agent-desktop 1.3.1` Engine ([`browser.rs`](../../../crates/aphrody_pilot/src/browser.rs), [`agent_browser_core.rs`](../../../crates/aphrody_browser/src/agent_browser_core.rs), [`chromium_gpui.rs`](../../../crates/aphrody_browser/src/chromium_gpui.rs))**:
   - [`ChromiumBrowserView`](../../../crates/aphrody_browser/src/chromium_gpui.rs) embeds `AgentBrowserCore` (`agent_core`, `apply_omnibox_intent`) inside GPUI Chromium `//ui/views` alongside Kitesurf escalation and per-tab VPN routing.
   - [`crates/aphrody_pilot/src/browser.rs`](../../../crates/aphrody_pilot/src/browser.rs) provides `SnapshotDiff`, `SemanticLocator`, `AnnotatedElementBox`, `NetworkRouteTable`, `HarRingBuffer`, `BrowserEventRingBuffer`, `BrowserStorageState`, `TabSessionManager`, `BrowserActionPolicy`, and `BrowserBatchReport`.
   - [`action.rs`](../../../crates/aphrody_pilot/src/action.rs), [`locator.rs`](../../../crates/aphrody_pilot/src/locator.rs), [`model.rs`](../../../crates/aphrody_pilot/src/model.rs), and [`policy.rs`](../../../crates/aphrody_pilot/src/policy.rs) enforce `act_verify`, `PostconditionVerdict`, `RetryDisposition`, and `BatchExecutionState`.
4. **Unified MCP Server ([`aphrody_craft::mcp`](../../../crates/aphrody_craft/src/mcp.rs))**:
   - Exposes `os_*` (workspaces, tiling layouts, themes, session), `craft_*` (codecs & file sniffing), all `desktop_*` computer-use tools, **`desktop_act_verify`** and **`desktop_ai_act_verify`** (1-shot observe -> locate -> act -> readback verify -> post-snapshot diff), and all **27 `browser_*` tools** (`browser_batch`, `browser_diff_snapshot`, `browser_diff_screenshot`, `browser_annotate`, `browser_find`, `browser_tab_*`, `browser_route_*`, `browser_har_*`, `browser_storage_*`, `browser_console`, `browser_errors`, `browser_wait`, `browser_confirm`, `browser_deny`, `browser_highlight`, `browser_policy_set`, etc.) over stdio JSON-RPC 2.0 and hardened local HTTP (`/mcp`, `/.well-known/webmcp.json`, `/health`).
   - Automatically synchronizes compositor workspaces/windows/themes between `McpServerState` and `PilotEngine.gpui`.
5. **Live GPUI Shell HUD & Unified CLI ([`aphrody_desktop`](../../../crates/aphrody_desktop/src/aphrody_desktop.rs))**:
   - `aphrody-os pilot <op>` exposes AI pilot operations in single JSON envelopes (`status`, `snapshot`, `click`, `act-verify`, `ai-act`, plus `--session`, `--timeout-ms`, `--app`, `--window-id`, and `--stop-on-error`).
   - [`pilot_hud.rs`](../../../crates/aphrody_desktop/src/pilot_hud.rs) renders `render_pilot_status_pill`, `append_pilot_drawer_section`, and `pilot_hud_summary` inside `OsDesktopShell`.

## 3. Reproducible Audit & Verification

Run the automated purge-and-parity verifier:

```bash
bun run audit:agent-desktop
bun tooling/bun/agent-desktop-audit.ts --offline
bun test tooling/bun/agent-desktop-audit.test.ts
cargo test -j 4 -p aphrody_pilot -p aphrody_browser -p aphrody_craft -p aphrody_desktop
```
