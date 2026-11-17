# `aphrody` — Full-Stack Rust OS, GUI & AI Framework

Modeled directly after `DioxusLabs/dioxus` (`packages/dioxus`), `aphrody` is the unified Rust library facade uniting Zed, GPUI, Iced/COSMIC Epoch, `egui 0.37`, Bevy UE5.8, Tauri v3, Native Shell (NSH), Storytold Craft, Agent-Browser CDP automation, Tree-sitter, and Headless Rust MCP under one coherent feature matrix.

## Feature Architecture

- **Targets**: `desktop`, `web`, `mobile`, `server`, `headless`, `fullstack`
- **Profiles**: `minimal` (session + design + installer), `lib` (+ store + craft + cosmic), `full` (entire stack)
- **Backends (placed cleanly alongside `gpui`)**: `gpui`, `iced` (`iced_gpui`), `egui` (`egui_gpui`), `bevy` (`bevy_gpui`), `tauri` (`tauri_gpui`), `cosmic` (`cosmic_gpui`), `nsh` (`nsh-core`/`nsh-base`)
- **Capabilities**: `craft` / `craft-studio`, `pilot` / `browser` (absorbed `agent-browser` + UIA/AT-SPI2/AX), `editor` / `tree-sitter`, `store` (22 targets), `installer` (CachyOS/Arch/Aphrody DE Switcher), `mcp`, `vpn`

## Binary & Workspace Optimization

Use `[profile.bun-release]` (`opt-level = "z"`, `lto = "fat"`, `codegen-units = 1`, `strip = "symbols"`, `panic = "abort"`) via:

```sh
cargo build --profile bun-release -p aphrody_cli --bin aphrody-cli
```
