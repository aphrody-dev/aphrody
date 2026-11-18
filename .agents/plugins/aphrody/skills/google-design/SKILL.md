---
name: google-design
description: Authoritative design system specification anchored to https://design.google/ (Material 3 Expressive, Neural Expressive Nov 2026, Code is a Design Material, Google Sans Flex) across the entire Aphrody monorepo, GPUI, egui_gpui, bevy_gpui, aphrody_craft, gpui-kit, NSH, Stitch Studio, and Wasm/WebGPU applications.
---

# Google Design Source of Truth (`https://design.google/`)

Every UI surface, theme, component token table, shader palette, and studio workspace in Aphrody derives from **`https://design.google/`** and the canonical [`docs/design/`](../../../../../docs/design/manifest.json) package:

- **Material 3 Expressive & Neural Expressive (Nov 2026)** 3-tier tokens (`assets/os/google_design_tokens.toml`, [`docs/design/stylesheet.css`](../../../../../docs/design/stylesheet.css), `aphrody_ui::google_design`)
- ***Code is a Design Material*** (`https://design.google/library/code-is-a-design-material`): live Rust/WGSL/TypeScript code + shaders as expressive design material
- ***Google Sans Flex*** (`https://design.google/library/google-sans-flex-font`): 6 variable axes (`opsz`, `wght`, `wdth`, `GRAD`, `slnt`, `ROND`) paired with `JetBrains Mono` for code & hex telemetry
- Root [`DESIGN.md`](../../../../../DESIGN.md), [`docs/design/DESIGN.md`](../../../../../docs/design/DESIGN.md), Figma W3C DTCG kit ([`docs/design/kits/figma-design-kit.json`](../../../../../docs/design/kits/figma-design-kit.json)), and Google Stitch kit ([`docs/design/kits/stitch-design-kit.json`](../../../../../docs/design/kits/stitch-design-kit.json))
- Dual-GUI & 3D Studio Stack ([`references/gpui-and-egui-components.md`](./references/gpui-and-egui-components.md)):
  - **Unified Single GPUI Retained Mode**: `crates/gpui` (`0.2.2`, `Path<T = Pixels>`, `taffy 0.15.1`, `wgpu v30`) + `aphrody_ui::google_design_components` (`ExpressiveOmnibar`, `CascadingPopoverCard`, `GroundingSourceCard`, `SegmentedStudioBar`, `ThinkingAuraBadge`) + `aphrody_ui::starter_pack`
  - **Native `gpui-component` / `gpui-kit` `0.10.0` M3 Kit**: [`crates/gpui-component/src/material3.rs`](../../../../../crates/gpui-component/src/material3.rs) (`gpui_kit::material3`) implements 4 `M3AppearanceMode` themes (`Light`, `Dark`, `NeuralObsidian`, `OledPitch`), `M3SpringToken`, WCAG AA verification (`m3_contrast_ratio`, `m3_meets_wcag_aa`), and 16 `RenderOnce` components (`M3FilledButton`, `M3TonalButton`, `M3OutlinedButton`, `M3Fab`, `M3ExtendedFab`, `M3SegmentedButtonBar`, `M3FilterChip`, `M3ElevatedCard`, `M3FilledCard`, `M3OutlinedCard`, `M3TopAppBar`, `M3NavigationRail`, `M3NavigationBar`, `M3SearchBar`, `M3ExpressiveSwitch`, `M3DialogSheet`), bridged with GPUI themes in [`crates/gpui_ui/src/kit.rs`](../../../../../crates/gpui_ui/src/kit.rs) (`apply_kit_m3_theme`, `sync_kit_with_gpui_theme`)
  - **Native `nsh-base` / `aphrody_nsh` `0.6.0` M3 Kit**: [`crates/nsh-base/src/material3.rs`](../../../../../crates/nsh-base/src/material3.rs) (`M3Mode`, `M3_SPRINGS`, `m3_theme`, `m3_adaptive_theme`) syncs via [`crates/gpui_ui/src/nsh.rs`](../../../../../crates/gpui_ui/src/nsh.rs) (`sync_app_view_with_gpui_m3_theme`) into [`crates/aphrody_nsh/src/material3_kit.rs`](../../../../../crates/aphrody_nsh/src/material3_kit.rs) (12 builders: `m3_surface`, `m3_filled_button`, `m3_tonal_button`, `m3_fab`, `m3_chip`, `m3_card`, `m3_navigation_rail`, `m3_top_app_bar`, `m3_search_omnibar`, `m3_segmented_tabs`, `m3_switch`, `m3_aura_badge` + `export_m3_nsh_tsx_kit()`)
  - **Immediate-Mode `egui 0.37` Bridge**: `egui_gpui` (`google_design_to_egui_visuals`, `CraftDockTabs`, `CraftComboMenu`, `CraftToolbarChip`, `CraftPropertyRow`, `CraftImeBanner`)
  - **Unreal Bevy 5.8 Viewport & MaterialX**: `bevy_gpui` (Nanite 64v/124t meshlets, OpenPBR/MaterialX -> Naga v30 WGSL, Solari GI, OpenUSD Crate LZ4)
  - **Full-GPUI FigCraft & Craft Studio (`aphrody_craft`)**: [`dual_frontend.rs`](../../../../../crates/aphrody_craft/src/dual_frontend.rs) defaults to `ParityRenderMode::GpuiGoogleDesign` alongside `EguiNative037` and `SplitSideBySide`; [`figcraft_gpui.rs`](../../../../../crates/aphrody_craft/src/figcraft_gpui.rs) renders `FigCraftStudioView` (`Render + Focusable + EventEmitter<FigCraftStudioEvent>`); [`figcraft_core.rs`](../../../../../crates/aphrody_craft/src/figcraft_core.rs) adds multi-page `FigCraftDocument` (`FigCraftPage`, `PageKind`), `UndoRedoHistory` (cap 128), `ComponentVariantDefinition`, 24 starter templates (`m3_expressive_starter_templates()`), and 4 deterministic exporters (`export_to_gpui_kit_rs`, `export_to_nsh_rs_tsx`, `export_to_gpui_rs`, `export_to_lighthouse100_html`)

## Spatial vs Effects Physics Springs

- **Spatial springs** (`damping_ratio = 0.9`): size, layout, position, and shape morphing (`spatial_fast = 1400`, `spatial_default = 700`, `spatial_slow = 300`).
- **Effects springs** (`damping_ratio = 1.0`): opacity, color, and elevation transitions (`effects_fast = 3800`, `effects_default = 1600`, `effects_slow = 800`).

## Automation & Native Commands

- Build & validate complete `docs/design/` asset/kit/MDX/schema package: `bun run design:pack`
- Sync tokens, sitemap snapshot & `DESIGN.md`: `bun run design:sync`
- Audit monorepo & templates: `bun run design:audit`
- Lint any `DESIGN.md`: `cargo run -p aphrody_cli -- design design-md lint docs/design/DESIGN.md`
- Open Studio & Parity workspaces inside Aphrody IDE ([`crates/aphrody_nsh/src/design_stitch_studio.rs`](../../../../../crates/aphrody_nsh/src/design_stitch_studio.rs)):
  - `aphrody_nsh::OpenGoogleDesignMirror`
  - `aphrody_nsh::OpenStitchStudio`
  - `aphrody_nsh::OpenGpuiStarterPack`
  - `aphrody_nsh::OpenGoogleFigCraftStudio` (mounts `FigCraftStudioView`)
  - `aphrody_nsh::OpenGoogleCraftAnythingStudio`
  - `aphrody_nsh::OpenUnrealBevyStudio`
  - `aphrody_nsh::ToggleStudioGuiParity`
