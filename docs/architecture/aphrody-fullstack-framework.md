# Aphrody Full-Stack Rust Framework Architecture (`aphrody_*`)

> Unified Dioxus-style modular Rust OS, UI backends, headless Zed + Tree-sitter, absorbed `agent-browser` CDP, Storytold Craft, and CachyOS/Aphrody DE Switcher architecture across `C:/src/aphrody`.

## 1. Crate Hierarchy & `aphrody_*` Naming Convention

| Crate | Path | Role & Parity Scope |
|---|---|---|
| [`aphrody`](../../crates/aphrody/src/aphrody.rs) | `crates/aphrody` | Dioxus-style top-level Rust library facade (`minimal`, `lib`, `desktop`, `web`, `mobile`, `server`, `headless`, `fullstack`, `full`). |
| [`aphrody_cli`](../../crates/aphrody_cli/src/main.rs) | `crates/aphrody_cli` | Unified developer CLI (`aphrody-cli` + `gpui-wgpu`) uniting `build`, `deploy`, `dev ast/matrix`, `craft`, `cosmic`, `zed headless/mcp`, `install`, `wasm`, `design`, `mcp`. |
| [`aphrody_session`](../../crates/aphrody_session/src/aphrody_session.rs) | `crates/aphrody_session` | `CompositorState` (Dwindle / CosmicGrid / NiriScroll / Floating + Pop Shell `WindowStackGroup` & `WorkspaceOverviewSnapshot`), `CosmicSettingsDaemon`, `NotificationCenter`, `OsdBanner`, `query_pop_launcher`, W3C i18n, Wasm HAL. |
| [`aphrody_craft`](../../crates/aphrody_craft/src/aphrody_craft.rs) | `crates/aphrody_craft` | Consolidated ArtCraft studio + Storytold 82-repo suite (`craft-libs` + `craft-anything`), `CraftExposureManifest` (CLI/MCP/Web/Desktop/build/deploy), and MCP server. |
| [`aphrody_pilot`](../../crates/aphrody_pilot/src/aphrody_pilot.rs) | `crates/aphrody_pilot` | Cross-platform low-level OS pilot (Win UIA, Linux AT-SPI2/X11, macOS AX), absorbed `agent-browser` CDP & SSRF guard ([`browser.rs`](../../crates/aphrody_pilot/src/browser.rs)), and Headless Zed + Tree-sitter MCP ([`zed_headless.rs`](../../crates/aphrody_pilot/src/zed_headless.rs)). |
| [`aphrody_installer`](../../crates/aphrody_installer/src/aphrody_installer.rs) | `crates/aphrody_installer` | Pure-Rust + NSH rewrite of `vendor/new-cli-installer` (`cachyos/new-cli-installer`) with Btrfs/LUKS2 planner and live DE Switcher (`aphrody-gpui`, `iced-cosmic`, `egui-craft`, `webgpu-os`, GNOME, KDE, Hyprland, Sway, Niri). |
| [`aphrody_store`](../../crates/aphrody_store/src/aphrody_store.rs) | `crates/aphrody_store` | Awesome GPUI Store + 22-target universal deploy packagers (`winget`, `flatpak`, `arch`, `nix`, `oci`, etc.). |
| [`aphrody_design`](../../crates/aphrody_design/src/aphrody_design.rs) | `crates/aphrody_design` | `design.google` mirror, `DESIGN.md` WCAG linter, Stitch studio SDK & CWV/Lighthouse audits. |
| [`aphrody_ui`](../../crates/aphrody_ui/src/aphrody_ui.rs) | `crates/aphrody_ui` | Material 3 Expressive + `libcosmic` RON theme bridge, `NavRail`, `ContextDrawer`, `StarterButton`/`StarterCard`, immediate helpers. |
| [`aphrody_desktop`](../../crates/aphrody_desktop/src/aphrody_desktop.rs) | `crates/aphrody_desktop` | Modular COSMIC Epoch + Pop!_OS GPUI desktop shell (`aphrody-os`). |
| [`aphrody_nsh`](../../crates/aphrody_nsh/src/aphrody_nsh.rs) | `crates/aphrody_nsh` | Native Shell workspace studio panes + Stitch studio bridge. |
| [`aphrody_vpn`](../../crates/aphrody_vpn/src/aphrody_vpn.rs) | `crates/aphrody_vpn` | GPUI VPN client over embedded Mullvad/ayvpn daemon. |

## 2. Aligned Multi-UI Backends Alongside `crates/gpui`

All five UI/compositor adapters sit symmetrically next to [`crates/gpui`](../../crates/gpui/Cargo.toml), [`crates/gpui_wgpu`](../../crates/gpui_wgpu/Cargo.toml), and [`crates/gpui_web`](../../crates/gpui_web/Cargo.toml) sharing workspace `wgpu v30`, `taffy`, `accesskit 0.25`, `cosmic-text`, and `serde`:

- [`iced_gpui`](../../crates/iced_gpui/src/iced_gpui.rs): Iced Elm `Message`/`CommandSpec` + COSMIC `libcosmic` RON palette bridge.
- [`egui_gpui`](../../crates/egui_gpui/src/egui_gpui.rs): Immediate-mode `egui 0.37` context over GPUI canvas.
- [`bevy_gpui`](../../crates/bevy_gpui/src/bevy_gpui.rs): Bevy `0.20` ECS + UE 5.8 Solari/Nanite/MaterialX pipeline over `gpui_wgpu`.
- [`tauri_gpui`](../../crates/tauri_gpui/src/tauri_gpui.rs): Tauri v3 ACL IPC capability router + 22-target bundler alignment.
- [`cosmic_gpui`](../../crates/cosmic_gpui/src/cosmic_gpui.rs): COSMIC Epoch subsystem exposure across CLI, MCP, Web, and Desktop.

## 3. Headless Zed (`--headless --mcp`), `agent-browser` & `[profile.bun-release]`

- Launching `aphrody --headless --mcp` or `aphrody-cli zed mcp` runs a stdio JSON-RPC 2.0 MCP server with zero GUI window creation, serving `zed_headless_*` (workspace buffers, Tree-sitter AST/symbol analysis across Rust, TypeScript, TSX, Python, and JSON), `browser_*` (CDP discovery, WebSocket commands, SSRF IP/DNS guard absorbed from `agent-browser`), and `desktop_*` (Windows UIA / Linux AT-SPI2 / macOS AXUIElement / GPUI AccessKit).
- `[profile.bun-release]` in [`Cargo.toml`](../../Cargo.toml) applies `opt-level = "z"`, `lto = "fat"`, `codegen-units = 1`, `strip = "symbols"`, and `panic = "abort"` for compact final distribution binaries.
