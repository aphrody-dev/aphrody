---
name: google-design
description: Authoritative design system specification anchored to https://design.google/ (Material 3 Expressive, Neural Expressive Nov 2026, Code is a Design Material, Google Sans Flex) across the entire Aphrody monorepo, GPUI, egui_gpui, bevy_gpui, craft_studio, gpui-kit, NSH, Stitch Studio, and Wasm/WebGPU applications.
---

# Google Design Source of Truth (`https://design.google/`)

Every UI surface, theme, component token table, shader palette, and studio workspace in Aphrody derives from **`https://design.google/`** and the canonical [`docs/design/`](../../../../docs/design/manifest.json) package:

- **Material 3 Expressive & Neural Expressive (Nov 2026)** 3-tier tokens (`assets/os/google_design_tokens.toml`, [`docs/design/stylesheet.css`](../../../../docs/design/stylesheet.css), `os_ui::google_design`)
- [*Code is a Design Material*](https://design.google/library/code-is-a-design-material): live Rust/WGSL/TypeScript code + shaders as expressive design material
- [*Google Sans Flex*](https://design.google/library/google-sans-flex-font): 6 variable axes (`opsz`, `wght`, `wdth`, `GRAD`, `slnt`, `ROND`) paired with `JetBrains Mono` for code & hex telemetry
- Root [`DESIGN.md`](../../../../DESIGN.md), [`docs/design/DESIGN.md`](../../../../docs/design/DESIGN.md), Figma W3C DTCG kit ([`docs/design/kits/figma-design-kit.tokens.json`](../../../../docs/design/kits/figma-design-kit.tokens.json)), and Google Stitch kit ([`docs/design/kits/stitch-design-kit.json`](../../../../docs/design/kits/stitch-design-kit.json))
- Dual-GUI & 3D Studio Stack ([`references/gpui-and-egui-components.md`](./references/gpui-and-egui-components.md)):
  - **GPUI Retained Mode**: `os_ui::google_design_components` (`ExpressiveOmnibar`, `CascadingPopoverCard`, `GroundingSourceCard`, `SegmentedStudioBar`, `ThinkingAuraBadge`) + `os_ui::starter_pack`
  - **Immediate-Mode `egui 0.37` Bridge**: `egui_gpui` (`google_design_to_egui_visuals`, `CraftDockTabs`, `CraftComboMenu`, `CraftToolbarChip`, `CraftPropertyRow`, `CraftImeBanner`)
  - **Unreal Bevy 5.8 Viewport & MaterialX**: `bevy_gpui` (Nanite 64v/124t meshlets, OpenPBR/MaterialX -> Naga v30 WGSL, Solari GI, OpenUSD Crate LZ4)
  - **Shared Studio Core**: `craft_studio` (Google FigCraft + Google Craft Anything + Unreal Bevy 5.8 + 18 Storytold Craft Apps in `GpuiGoogleDesign`, `EguiImmediate`, or `SplitSideBySide` parity mode)

## Spatial vs Effects Physics Springs

- **Spatial springs** (`damping_ratio = 0.9`): size, layout, position, and shape morphing (`spatial_fast = 1400`, `spatial_default = 700`, `spatial_slow = 300`).
- **Effects springs** (`damping_ratio = 1.0`): opacity, color, and elevation transitions (`effects_fast = 3800`, `effects_default = 1600`, `effects_slow = 800`).

## Automation & Native Commands

- Build & validate complete `docs/design/` asset/kit/MDX/schema package: `bun run design:pack`
- Sync tokens, sitemap snapshot & `DESIGN.md`: `bun run design:sync`
- Audit monorepo & templates: `bun run design:audit`
- Lint any `DESIGN.md`: `cargo run -p wgpu_cli -- design design-md lint docs/design/DESIGN.md`
- Open Studio & Parity workspaces inside Aphrody IDE (`crates/nsh_workspace/src/design_stitch_studio.rs`):
  - `nsh_workspace::OpenGoogleDesignMirror`
  - `nsh_workspace::OpenStitchStudio`
  - `nsh_workspace::OpenGpuiStarterPack`
  - `nsh_workspace::OpenGoogleFigCraftStudio`
  - `nsh_workspace::OpenGoogleCraftAnythingStudio`
  - `nsh_workspace::OpenUnrealBevyStudio`
  - `nsh_workspace::ToggleStudioGuiParity`
