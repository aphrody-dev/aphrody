---
name: ai-os-pilot
description: Control the entire Aphrody GPUI OS (COSMIC compositor, workspaces, ArtCraft studio, AccessKit UI trees) and external native applications (Windows UIA, Linux AT-SPI2/X11, macOS AXUIElement) through one unified MCP/CLI control plane (`aphrody-os pilot` / `aphrody_craft` MCP).
---

# AI OS Pilot (`aphrody_pilot` + `aphrody_browser` + `aphrody_craft` MCP + `aphrody-os`)

Unified AI computer-use, browser automation, and OS control engine fusing upstream `agent-desktop` (`1.3.1`) and `agent-browser` (`0.43.0`) into Zed/GPUI idioms across `crates/aphrody_pilot`, `crates/aphrody_browser`, and `crates/aphrody_desktop`.

## Core Control Loop

1. **Preflight**: Run `desktop_status` and `desktop_permissions` to verify target (`gpui`, `native`, or `auto`), accessibility permission, and active windows.
2. **Inspect**: Run `desktop_snapshot` (`interactive: true`, `compact: true`) to capture the Accessibility Tree with stable refs (`@s<hex>:e<n>`), or `desktop_find` by `role`, `name`, `name_contains`, or `identifier`. For browser tabs inside GPUI Chromium, use `browser_snapshot` and `browser_diff_snapshot` (`SnapshotDiff`, `SemanticLocator`, `AnnotatedElementBox`).
3. **Act + Verify (Recommended)**: Prefer `desktop_act_verify` or `desktop_ai_act_verify` (`act_verify` -> readback `PostconditionVerdict` -> post-snapshot diff) in one roundtrip.
4. **Retry & Batch Discipline**: Check `error.disposition.retry` (`RetryDisposition`):
   - `safe`: transient timing error; retry directly.
   - `reobserve`: UI shifted or ref went stale (`STALE_REF`); capture a fresh snapshot first.
   - `do_not_retry`: capability or permission boundary (`PERM_DENIED`, `ELEVATION_REQUIRED`, `ELEMENT_DISABLED`).
   - Browser/UI batches track `BatchExecutionState` (`BrowserBatchReport`) and support `--stop-on-error`.

## Browser & Tab Controls (`agent-browser 0.43.0` Fusion)

- [`crates/aphrody_browser`](../../../../../crates/aphrody_browser/src/aphrody_browser.rs) embeds `AgentBrowserCore` inside `ChromiumBrowserView` (`agent_core`, `apply_omnibox_intent`).
- [`crates/aphrody_pilot/src/browser.rs`](../../../../../crates/aphrody_pilot/src/browser.rs) exposes 27 `browser_*` MCP tools backed by `TabSessionManager`, `BrowserActionPolicy`, `NetworkRouteTable`, `HarRingBuffer`, `BrowserEventRingBuffer`, and `BrowserStorageState`.
- `aphrody-os pilot` CLI supports `--session`, `--timeout-ms`, `--app`, `--window-id`, and `--stop-on-error`, surfaced in the shell HUD via `pilot_hud_summary` ([`pilot_hud.rs`](../../../../../crates/aphrody_desktop/src/pilot_hud.rs)).

## Shell Quoting Rules

- **PowerShell**: Always single-quote ref IDs (`'@s1a2b3c4:e5'`); unquoted `@token` triggers PowerShell splatting.
- **POSIX shells**: `@s1a2b3c4:e5` works unquoted.

## Target Selection (`target`)

- `gpui` / `auto`: pilots Aphrody GPUI OS in-process via `accesskit::TreeUpdate` + `aphrody_session::CompositorState` (Desktop, Headless Linux CI, and `wasm32` WebGPU).
- `native`: pilots external OS windows via Windows `CUIAutomation` + `SendInput`, Linux `AT-SPI2` (`zbus`) + `x11rb`, or macOS `AXUIElement` + `CGEvent`.

## References

- [Complete Command & MCP Tool Matrix](references/commands-reference.md)
- [Platform Quirks, UIPI, Chromium & AT-SPI2](references/platforms-matrix.md)
- [Purge Manifest & Zed/GPUI Fusion Map](references/architecture-fusion.md)
