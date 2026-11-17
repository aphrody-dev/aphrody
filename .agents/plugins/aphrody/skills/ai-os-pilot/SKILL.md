---
name: ai-os-pilot
description: Control the entire Aphrody GPUI OS (COSMIC compositor, workspaces, ArtCraft studio, AccessKit UI trees) and external native applications (Windows UIA, Linux AT-SPI2/X11, macOS AXUIElement) through one unified MCP/CLI control plane (`aphrody-os pilot` / `aphrody_craft` MCP).
---

# AI OS Pilot (`aphrody_pilot` + `aphrody_craft` MCP + `aphrody-os`)

Unified AI computer-use and OS control engine migrating [`aphrody-labs/agent-desktop`](https://github.com/aphrody-labs/agent-desktop) into Zed/GPUI idioms without external C-ABI, Next.js, or legacy cursor-overlay bloat.

## Core Control Loop

1. **Preflight**: Run `desktop_status` and `desktop_permissions` to verify target (`gpui`, `native`, or `auto`), accessibility permission, and active windows.
2. **Inspect**: Run `desktop_snapshot` (`interactive: true`, `compact: true`) to capture the Accessibility Tree with stable refs (`@s<hex>:e<n>`), or use `desktop_find` directly by `role`, `name`, `name_contains`, or `identifier`.
3. **One-Shot Act + Verify (Recommended)**: Prefer `desktop_ai_act_verify` to observe, resolve locator/`@ref`, execute the action, poll readback postconditions (`checked`, `expanded`, `value`), and return the updated snapshot in a single roundtrip.
4. **Retry Discipline**: Check `error.disposition.retry`:
   - `safe`: transient timing error — safe to retry directly.
   - `reobserve`: UI shifted or ref went stale (`STALE_REF`) — run a fresh `snapshot` before retrying.
   - `do_not_retry`: capability/permission boundary (`PERM_DENIED`, `ELEVATION_REQUIRED`, `ELEMENT_DISABLED`).

## Shell Quoting Rules

- **PowerShell**: ALWAYS single-quote ref IDs (`'@s1a2b3c4:e5'`); unquoted `@token` is the PowerShell splatting operator.
- **POSIX shells**: `@s1a2b3c4:e5` works bare.

## Target Selection (`target`)

- `gpui` / `auto`: pilots Aphrody GPUI OS in-process via `accesskit::TreeUpdate` + `aphrody_session::CompositorState` (works natively on desktop, headless Linux CI, and `wasm32` WebGPU).
- `native`: pilots external OS windows via Windows `CUIAutomation` + `SendInput`, Linux `AT-SPI2` (`zbus`) + `x11rb`, or macOS `AXUIElement` + `CGEvent`.

## References

- [Complete Command & MCP Tool Matrix](references/commands-reference.md)
- [Platform Quirks, UIPI, Chromium & AT-SPI2](references/platforms-matrix.md)
- [Purge Manifest & Zed/GPUI Fusion Map](references/architecture-fusion.md)
