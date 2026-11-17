# Platform Quirks & Battle-Tested Safeguards

Migrated and distilled from `agent-desktop` platform adapters & runtime investigations:

## Windows (`backend_windows.rs`)
- **COM MTA + Per-Monitor DPI**: Initialized via `COINIT_MULTITHREADED` + `GetDpiForMonitor(MDT_EFFECTIVE_DPI)`; never trust unverified display scale assumptions.
- **Hung-Window Protection**: Before invoking UI Automation on an HWND, `SendMessageTimeoutW(WM_NULL, SMTO_ABORTIFHUNG, 600ms)` fails fast with `APP_UNRESPONSIVE` instead of wedging the UIA thread.
- **Chromium / Electron Activation**: Windows with class `Chrome_WidgetWin_1` receive `WM_GETOBJECT(0, OBJID_CLIENT)` before `ElementFromHandle` so lazy renderers materialize full DOM accessibility nodes.
- **UIPI Elevation Gate**: `OpenProcessToken(TOKEN_QUERY)` + `GetTokenInformation(TokenIntegrityLevel)` compares caller and target integrity RIDs and returns `ELEVATION_REQUIRED` when injecting toward higher-integrity windows.

## Linux (`backend_linux.rs`)
- **AT-SPI2 D-Bus Discovery**: Connects via `zbus` to session `org.a1y.Bus` (`GetAddress`) before walking `org.a1y.atspi.Registry`.
- **X11 vs Wayland**: Uses `x11rb` (`xtest`, geometry, `get_image` PNG) under X11 and reports `WAYLAND_DISPLAY` / portal availability cleanly when Wayland input restriction applies.
- **AT-SPI2 Error Caution**: Never assume an AT-SPI action return alone proves UI state mutation — rely on `postcondition_satisfied` readback verification.

## macOS (`backend_macos.rs`)
- **TCC Permissions**: Probes `AXIsProcessTrusted()` for Accessibility and `CGPreflightScreenCaptureAccess()` for screenshots; surfaces `PERM_DENIED` cleanly without silent hangs.
- **AX Memory Safety**: Every `AXUIElementRef` is RAII-managed with `CFRelease` on `Drop`.

## GPUI AccessKit + COSMIC Compositor (`gpui_bridge.rs`)
- Compiles on **every target including `wasm32-unknown-unknown` and `wasm32-wasip2`**, allowing full AI OS loops in headless CI and WebGPU browser builds.
