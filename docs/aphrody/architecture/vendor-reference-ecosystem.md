# Pinned Fork Dependencies & Reference Ecosystem

> Policy established across [`../../../Cargo.toml`](../../../Cargo.toml), [`../../../tooling/bun/forks-manifest.json`](../../../tooling/bun/forks-manifest.json), and `fw: plugins/aphrody/skills/vendor-ecosystem`: upstream forks are consumed as pinned Cargo git dependencies (`git = "https://github.com/aphrody-labs/<fork>.git", tag = "..."`) only when root workspace crates directly compile or patch them. The repository has no git submodules and no `vendor/` directory. Pure reference ecosystems are tracked via live GitHub / `crates.io` metadata and compact skills.

---

## 1. Pinned Forks (`10` Cargo/Build-Active Dependencies)

| Fork (`aphrody-labs/*`) | Pinned Tag (root `Cargo.toml`) | Primary Consumers in Workspace | Justification |
| :--- | :--- | :--- | :--- |
| `aphrody-labs/wgpu` | `v30.0.0-aphrody.20261120` | `gpui_wgpu`, `aphrody_cli`, `nsh-engine`, `egui_gpui`, `iced_gpui` | Redirected via `[patch.crates-io]` (`wgpu*`, `naga`) so GPUI, NSH, and egui share one GPU/Naga stack. |
| `aphrody-labs/taffy` | `v0.15.1-aphrody.20261120` | `gpui`, `nsh-core`, `aphrody_session` | Redirected via `[patch.crates-io]` for single-source Flexbox/Grid/Block layout. |
| `aphrody-labs/accesskit` | `v0.25.2-aphrody.20261120` | `gpui`, `gpui_{linux,macos,windows}`, `aphrody_pilot`, `nsh-platform-gpui` | Redirected via `[patch.crates-io]` for unified OS UIA/AT-SPI2/AX accessibility trees. |
| `aphrody-labs/cosmic-text` | `v0.19.0-aphrody.20261120` | `gpui_wgpu` | Redirected via `[patch.crates-io]` for HarfBuzz/Swash shaping and Google Sans Flex metrics. |
| `aphrody-labs/egui` | `v0.37.0-aphrody.20261120` | `crates/egui_gpui`, `crates/bevy_gpui`, `crates/aphrody_craft` | Embedded immediate-mode inspector compiled against shared `wgpu` v30 and `accesskit` 0.25. |
| `aphrody-labs/wry` | `v0.57.3-aphrody.20261120` | `crates/nsh-platform-gpui` | OS WebView host crate without needing the full Tauri tree. |
| `aphrody-labs/craft-libs` | `v0.1.0-aphrody.20261120.2` | `crates/aphrody_craft`, `crates/bevy_gpui` | 45+ format decoders plus the `[patch."https://github.com/storytold/craft-libs"]` redirections to the pinned tag. |
| `aphrody-labs/craft-anything` | `v0.6.1-aphrody.20261120` | `crates/aphrody_craft` | Binary/disasm/JS/evidence analysis crates used directly by `aphrody_craft`. |
| `aphrody-labs/mullvadvpn-app` | `v2026.11.20-aphrody.3` | `crates/ayvpn_daemon`, `crates/aphrody_vpn` | Embedded Talpid/WireGuard split-tunnel crates (`mullvad-daemon`, `mullvad-*`). |
| `aphrody-labs/browser` | `v2026.11.20-aphrody.3` | `crates/aphrody_browser`, `crates/nsh-platform-web` | Browser automation CDP daemon and SSRF/URL policy host. |

`aphrody-labs/kitesurf` is also pinned in [`forks-manifest.json`](../../../tooling/bun/forks-manifest.json) with the same release scheme. Tag, sha, and release URLs for every fork are recorded there.

---

## 2. Reference-Only Ecosystems (`14` Repositories Not Consumed by Cargo)

The `14` reference-only repositories (`bun`, `winit`, `vello`, `softbuffer`, `dioxus`, `blitz`, `anyrender`, `iced`, `tauri/tauri`, `tauri/cef-rs`, `libcosmic`, `cosmic-epoch`, `smithay`, `new-cli-installer`) are not part of this repository's tree or Cargo graph. They are documented with their pinned `aphrody-labs/*` commits and workspace replacements in vendor-ecosystem/SKILL.md (`fw: plugins/aphrody/skills/vendor-ecosystem`) and audited via [`../../../tooling/bun/gpui-vs-dioxus-audit.ts`](../../../tooling/bun/gpui-vs-dioxus-audit.ts). Bun is its own repository, `aphrody-labs/bun`.
