# COSMIC Epoch + Pop!_OS Desktop Architecture (`aphrody_desktop`)

> Modular refactoring of `aphrody_desktop` (`aphrody-os`) mirroring `pop-os/pop` and `pop-os/cosmic-epoch` across `aphrody_session`, `cosmic_gpui`, and `aphrody_desktop` without `mod.rs` boilerplate.

## 1. Subsystem Mapping

| `pop-os/cosmic-epoch` Repo | Aphrody Module | Responsibility |
|---|---|---|
| `cosmic-comp` + `pop-os/shell` | [`comp_viewport.rs`](../../../crates/aphrody_desktop/src/comp_viewport.rs) & [`compositor.rs`](../../../crates/aphrody_session/src/compositor.rs) | Dwindle / CosmicGrid / NiriScroll / Floating layout, active-hint focus ring (`ActiveHintConfig`), tabbed stacking (`WindowStackGroup`), directional focus, and workspace switching. |
| `cosmic-workspaces-epoch` | [`comp_viewport.rs`](../../../crates/aphrody_desktop/src/comp_viewport.rs) | Interactive live `WorkspacesOverview` grid displaying per-workspace window thumbnails and tiling mode selector. |
| `cosmic-panel` + `cosmic-applets` | [`panel.rs`](../../../crates/aphrody_desktop/src/panel.rs) & [`applets.rs`](../../../crates/aphrody_desktop/src/applets.rs) | Configurable Top Bar + Floating Bottom Dock across native apps, Craft suite, Overview, App Library, Screenshot, and Greeter Lock. |
| `pop-launcher` + `cosmic-app-library` | [`launcher.rs`](../../../crates/aphrody_desktop/src/launcher.rs) & [`epoch_daemon.rs`](../../../crates/aphrody_session/src/epoch_daemon.rs) | `=calc`, `:cmd`, `sh `, and `/` query prefixes paired with Pop! Folder categorized application grid. |
| `cosmic-settings` + `cosmic-settings-daemon` + `cosmic-randr` | [`apps/cosmic_settings.rs`](../../../crates/aphrody_desktop/src/apps/cosmic_settings.rs) | 5-page `NavRail` Control Center (Appearance, Desktop & Tiling, Displays RandR VRR, Power & Backlight Daemon, W3C i18n Language). |
| `cosmic-osd`, `cosmic-notifications`, `cosmic-screenshot`, `cosmic-greeter` | [`overlays.rs`](../../../crates/aphrody_desktop/src/overlays.rs) | Bottom OSD pill, dismissible notification stack, screenshot capture mode toolbar, and full-screen session lock overlay. |
| `cosmic-term`, `cosmic-edit`, `cosmic-files`, `cosmic-store` | [`apps/*.rs`](../../../crates/aphrody_desktop/src/apps.rs) | Integrated terminal workbench, Tree-sitter outline editor, async file browser, and 22-target Awesome GPUI Store. |
| `cachyos/new-cli-installer` (NSH Rewrite) | [`apps/installer.rs`](../../../crates/aphrody_desktop/src/apps/installer.rs) & [`aphrody_installer`](../../../crates/aphrody_installer/src/aphrody_installer.rs) | Guided/Advanced/Rescue net-installer + live Desktop Environment Switcher across Rust Userlands and Arch DEs. |

## 2. Exposure Across CLI, MCP, Web & Desktop

Every COSMIC and Storytold Craft subsystem is exposed uniformly through [`cosmic_gpui`](../../../crates/cosmic_gpui/src/cosmic_gpui.rs), [`aphrody_craft::expose`](../../../crates/aphrody_craft/src/expose.rs), [`aphrody_cli`](../../../crates/aphrody_cli/src/unified.rs), and headless MCP:

- CLI: `aphrody-cli cosmic status|expose`, `aphrody-cli craft expose|catalog|sniff`, `aphrody-cli install profiles|plan|switch-de`
- MCP: `cosmic_epoch_matrix`, `craft_expose_manifest`, `installer_switch_de_plan`, `os_workspace_*`, `desktop_*`, `browser_*`, `zed_headless_*`
- Web: `/cosmic/*` and `/craft/*` compiled via `wasm32-unknown-unknown` (`--features web`)
