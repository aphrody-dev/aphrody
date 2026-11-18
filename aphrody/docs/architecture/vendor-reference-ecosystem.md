# Vendor Submodule Reduction & Reference Ecosystem (`vendor/`)

> Policy established across [`../../../.gitmodules`](../../../.gitmodules), [`../../../Cargo.toml`](../../../Cargo.toml), and [`../../../.agents/plugins/aphrody/skills/vendor-ecosystem/SKILL.md`](../../../.agents/plugins/aphrody/skills/vendor-ecosystem/SKILL.md): keep submodules in `vendor/` **only** when root workspace crates directly compile or patch them via Cargo, and track pure reference ecosystems via live GitHub / `crates.io` metadata and compact skills.

---

## 1. Kept Submodules (`10` Cargo/Build-Active Forks)

| Submodule Path | Fork (`aphrody-labs/*`) | Primary Consumers in Workspace | Justification |
| :--- | :--- | :--- | :--- |
| [`vendor/wgpu`](../../../vendor/wgpu) | `aphrody-labs/wgpu` (`v30.0.0`) | `gpui_wgpu`, `aphrody_cli`, `nsh-engine`, `egui-wgpu`, `iced_gpui` | Patched via `[patch.crates-io]` so GPUI, NSH, and egui share one GPU/Naga stack. |
| [`vendor/taffy`](../../../vendor/taffy) | `aphrody-labs/taffy` (`v0.15.1`) | `gpui`, `nsh-core`, `aphrody_session` | Patched via `[patch.crates-io]` for single-source Flexbox/Grid/Block layout. |
| [`vendor/accesskit`](../../../vendor/accesskit) | `aphrody-labs/accesskit` (`v0.25.2`) | `gpui`, `gpui_{linux,macos,windows}`, `aphrody_pilot`, `egui` | Patched via `[patch.crates-io]` for unified OS UIA/AT-SPI2/AX accessibility trees. |
| [`vendor/cosmic-text`](../../../vendor/cosmic-text) | `aphrody-labs/cosmic-text` (`v0.19.0`) | `gpui_wgpu` | Patched via `[patch.crates-io]` for HarfBuzz/Swash shaping and Google Sans Flex metrics. |
| [`vendor/egui`](../../../vendor/egui) | `aphrody-labs/egui` (`v0.37.0`) | `crates/egui_gpui`, `crates/bevy_gpui`, `crates/aphrody_craft` | Embedded immediate-mode inspector compiled against shared `wgpu` v30 and `accesskit` 0.25. |
| [`vendor/tauri/wry`](../../../vendor/tauri/wry) | `aphrody-labs/wry` (`dev`) | `crates/nsh-platform-gpui` | Patched OS WebView host crate without needing full `vendor/tauri/tauri`. |
| [`vendor/craft-libs`](../../../vendor/craft-libs) | `aphrody-labs/craft-libs` | `crates/aphrody_craft`, `crates/bevy_gpui` | 45+ format decoders + 17 `[patch."https://github.com/storytold/craft-libs"]` path overrides. |
| [`vendor/craft-anything`](../../../vendor/craft-anything) | `aphrody-labs/craft-anything` (`v0.6.1`) | `crates/aphrody_craft` | Binary/disasm/JS/evidence analysis crates used directly by `aphrody_craft`. |
| [`vendor/mullvad`](../../../vendor/mullvad) | `aphrody-labs/mullvadvpn-app` | `crates/aphrody_vpn` | Embedded Talpid/WireGuard split-tunnel crates (`mullvad-daemon`, `talpid-*`). |
| [`vendor/browser`](../../../vendor/browser) | `aphrody-labs/browser` | `crates/aphrody_browser`, `crates/aphrody_pilot` | Browser automation CDP daemon and SSRF/URL policy host. |

---

## 2. De-Vendored Reference Ecosystems (`14` Removed Submodules)

All `14` reference-only repositories (`bun`, `winit`, `vello`, `softbuffer`, `dioxus`, `blitz`, `anyrender`, `iced`, `tauri/tauri`, `tauri/cef-rs`, `libcosmic`, `cosmic-epoch`, `smithay`, `new-cli-installer`) are documented with their pinned `aphrody-labs/*` commits and workspace replacements in [`vendor-ecosystem/SKILL.md`](../../../.agents/plugins/aphrody/skills/vendor-ecosystem/SKILL.md) and audited via [`../../../tooling/bun/gpui-vs-dioxus-audit.ts`](../../../tooling/bun/gpui-vs-dioxus-audit.ts).
