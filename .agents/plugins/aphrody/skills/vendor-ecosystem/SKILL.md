---
name: vendor-ecosystem
description: Reference catalog and architecture mapping for the 14 upstream third-party repositories that were consolidated out of vendor/ (zero direct Cargo dependency) into Aphrody native crates, skills, and release packages, plus the 10 kept Cargo-referenced vendor forks in vendor/.
---

# Aphrody Vendor & Third-Party Ecosystem Reference

`aphrody` keeps **only submodules with real Cargo `path =` or build-script dependencies** inside [`vendor/`](../../../../.gitmodules) (10 active forks, all synced with upstream and published under `aphrody-labs/*` GitHub Releases). Reference-only, architectural, and runtime-binary ecosystems live here as documented upstream pins rather than unbuilt Git checkouts.

## 1. Active `vendor/` Submodules (10 Kept — Real Cargo Dependencies)

| Submodule | Fork Repository | Pinned Release Tag | Justification (Direct Cargo / Build Consumer) |
| --- | --- | --- | --- |
| `vendor/wgpu` | `aphrody-labs/wgpu` | `v30.0.0-aphrody.20261118` | Patched `[patch.crates-io]` `wgpu`, `wgpu-core`, `wgpu-hal`, `wgpu-types`, `naga` (Metal probe + DX12 + WebGPU) across `gpui_wgpu`, `aphrody_cli`, `nsh-engine`, `egui_gpui`, `bevy_gpui` |
| `vendor/taffy` | `aphrody-labs/taffy` | `v0.15.1-aphrody.20261118` | Patched flexbox/grid layout engine in `gpui` & `nsh-core` |
| `vendor/cosmic-text` | `aphrody-labs/cosmic-text` | `v0.19.0-aphrody.20261118` | Patched text shaping in `gpui`, `nsh-core`, `egui_gpui` |
| `vendor/accesskit` | `aphrody-labs/accesskit` | `v0.25.2-aphrody.20261118` | Patched OS accessibility tree & adapters across `gpui`, `gpui_windows`, `gpui_linux`, `aphrody_pilot` |
| `vendor/egui` | `aphrody-labs/egui` | `v0.37.0-aphrody.20261118` | Immediate-mode inspector & WGSL WGPU v30 compat in `crates/egui_gpui` & `vendor/craft-anything` |
| `vendor/tauri/wry` | `aphrody-labs/wry` | `wry-v0.57.3-aphrody.20261118` | Native WebView engine in `crates/tauri_gpui` (`dirs 7` compatible) |
| `vendor/craft-libs` | `aphrody-labs/craft-libs` | `v0.1.0-aphrody.20261118` | 184-crate file format / CAD / PSD / PDF / Fig / ELF / MachO / PE / Wasm parser foundation unified via `[patch."https://github.com/storytold/craft-libs"]` in `aphrody_craft` & `craft-anything` |
| `vendor/craft-anything` | `aphrody-labs/craft-anything` | `v0.6.1-aphrody.20261118` | Clean-room JS/binary/disasm workbench powering `crates/aphrody_craft` |
| `vendor/mullvad` | `aphrody-labs/mullvadvpn-app` | `v2026.11-aphrody.1` | Embedded WireGuard VPN daemon SDK (`mullvad-*`) in `crates/aphrody_vpn` (`ayvpn-daemon`) |
| `vendor/browser` | `aphrody-labs/browser` | `aphrody-browser-20261118` | Headless/embedded browser engine & CDP harness for `crates/aphrody_browser` |

## 2. De-vendored Reference Ecosystems (14 Moved to Skills & Docs)

Every repository below had **zero `path = "vendor/..."` Cargo consumers** in `crates/` or `tooling/`. Their exact pinned commits remain preserved cleanly in upstream forks and documented here alongside the Aphrody native owners that replaced or absorbed them:

| Former Path | Fork Repo (`aphrody-labs/*`) | Pinned SHA | Upstream Origin | Why Removed from `vendor/` & Aphrody Replacement |
| --- | --- | --- | --- | --- |
| `vendor/bun` | `aphrody-labs/bun` (`main`) | `4407879a5f5a42535c0852292b935b5e73146a7f` | `oven-sh/bun` | Consumed as installed runtime binary (`bun`, `bun check`, `bun fmt`, `bun lint`, `bun test`) + compacted in [`bun-gpui`](../bun-gpui/SKILL.md) and `bun-brain` MCP — no Rust Cargo crate compiles `vendor/bun` |
| `vendor/winit` | `aphrody-labs/winit` (`main`) | `0c7372808c59c0d46ae777959e65e773381dc9e9` | `rust-windowing/winit` | GPUI uses native platform backends (`gpui_windows`, `gpui_macos`, `gpui_linux`, `gpui_web`, `gpui_android`) without winit |
| `vendor/vello` | `aphrody-labs/vello` (`main`) | `71109540002c9a2c6bb7c000ce8c829ba261304b` | `linebender/vello` | GPUI paints directly through `gpui_wgpu` / `wgpu` v30 scene pipelines; zero `vendor/vello` Cargo refs |
| `vendor/softbuffer` | `aphrody-labs/softbuffer` (`master`) | `81883e7b940891f6d5d78061e85b913b3ca3f20d` | `rust-windowing/softbuffer` | Software fallback handled inside `gpui_linux` (X11/SHM) and `gpui_web` Canvas2D/WebGL2 |
| `vendor/dioxus` | `aphrody-labs/dioxus` (`main`) | `164e07f94d3cfe4e46430a98925b414b061ef4ea` | `DioxusLabs/dioxus` | Architecture benchmark reference (`aphrody_ui::unified_runtime` + `subsecond` crates.io); GPUI + NSH own reactive UI natively |
| `vendor/blitz` | `aphrody-labs/blitz` (`main`) | `7bade8ead71c7ea852042c0cf156241aca32cc4c` | `DioxusLabs/blitz` | HTML/CSS DOM benchmark reference; NSH (`nsh-dom`, `nsh-bundler`, `nsh-core`) + `aphrody_browser` cover DOM/CSS natively |
| `vendor/anyrender` | `aphrody-labs/anyrender` (`main`) | `92d2a9ea4f27075f840b6d36a42da8a1ffc1746f` | `DioxusLabs/anyrender` | Reference 2D paint trait; GPUI `PaintScene` (`crates/gpui`) + `gpui_wgpu` fill this role directly |
| `vendor/iced` | `aphrody-labs/iced` (`master`) | `e5c350f271a8046f875c6662d452fde9e462d486` | `iced-rs/iced` | Elm message/update pattern implemented purely in `crates/iced_gpui` and `crates/aphrody_ui::reactive` with zero `vendor/iced` build dependency |
| `vendor/tauri/tauri` | `aphrody-labs/tauri` (`v3`) | `4cd56c8890126dfdcfcd3f863820936732119310` | `tauri-apps/tauri` | Only `vendor/tauri/wry` is compiled by `crates/tauri_gpui`; full Tauri CLI/mobile templates are reference-only |
| `vendor/tauri/cef-rs` | `aphrody-labs/cef-rs` (`dev`) | `36d823c8fd5937efa667995f59cccaf665953cce` | `tauri-apps/cef-rs` | Unwired CEF bindings; `crates/aphrody_browser` (`vendor/browser`) and `vendor/tauri/wry` provide browser & webview engines |
| `vendor/libcosmic` | `aphrody-labs/libcosmic` (`master`) | `12ae9ef3a8b9e36e6fb8459444341b680c2ccc7a` | `pop-os/libcosmic` | COSMIC theme/RON & shell ergonomics implemented natively in `crates/cosmic_gpui` and `crates/aphrody_ui::{cosmic_theme,cosmic_shell}` |
| `vendor/cosmic-epoch` | `aphrody-labs/cosmic-epoch` (`master`) | `e98f8289a22f9e612feee7d4c6ba197782db154b` | `pop-os/cosmic-epoch` | Desktop session architecture mirrored natively in `crates/aphrody_desktop` + `crates/aphrody_pilot` (`AccessKit`) |
| `vendor/smithay` | `aphrody-labs/smithay` (`master`) | `33323a917af13676875c27fb7748ffdaea5f4c2f` | `Smithay/smithay` | Wayland compositor reference; `crates/gpui_linux` + `crates/aphrody_desktop` run as native clients/shells |
| `vendor/new-cli-installer` | `aphrody-labs/New-Cli-Installer` (`master`) | `2037b04f9ce1e62ce34b2191fb58a22075e428a7` | `aphrody-labs/New-Cli-Installer` | Installer UX absorbed into `crates/aphrody_installer` (`src/`) & `crates/aphrody_store` (`upt`) |

See [`docs/aphrody/architecture/vendor-reference-ecosystem.md`](../../../../../docs/aphrody/architecture/vendor-reference-ecosystem.md) for architecture notes and upstream sync commands.
