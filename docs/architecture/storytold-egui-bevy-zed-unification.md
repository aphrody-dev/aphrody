# Storytold 82-Repository Ecosystem, `egui 0.37` & Bevy Unreal 5.8 Unification in Zed

> Canonical architecture blueprint unifying all **82 `https://github.com/orgs/storytold/repositories`** (`vendor/craft-libs` 165 crates + `vendor/craft-anything` 13 crates + 18 Studio apps), **`egui 0.37`** (`vendor/egui` -> [`crates/egui_gpui`](../../crates/egui_gpui/src/egui_gpui.rs)), **Bevy + `bevy_egui` Unreal Engine 5.8** ([`crates/bevy_gpui`](../../crates/bevy_gpui/src/bevy_gpui.rs)), and **Google Design + Stitch Studio** ([`crates/craft_studio`](../../crates/craft_studio/src/craft_studio.rs), [`docs/design/`](../design/manifest.json)) inside the single `aphrody` Zed monorepo.

---

## 1. Upstream Cargo Manifest & UI Architecture Comparison

### 1.1 `storytold/craft-libs/crates/ui/Cargo.toml` (`craft-ui`) vs Zed Upstream `gpui` + `ui`

| Dimension | `storytold/craft-libs/crates/ui` (`craft-ui`) | Zed / Aphrody `crates/gpui` + `crates/ui` + `crates/os_ui` | Unification Strategy in [`crates/egui_gpui`](../../crates/egui_gpui/src/egui_gpui.rs) |
| --- | --- | --- | --- |
| Paradigm | Immediate-mode (`&mut egui::Ui` per frame) | Retained Entity + `Render` / `RenderOnce` + Taffy Flexbox/Grid | Both run side-by-side (`GpuiGoogleDesign`, `EguiImmediate`, `SplitSideBySide`) |
| GPU & Text Stack | `egui 0.34..0.37`, `epaint` font atlas, `egui-wgpu` | `vendor/wgpu` v30, `cosmic-text`, platform DirectWrite/CoreText/Blade | Single `vendor/wgpu` v30 (`SharedWgpuHandles`) + `GoogleDesignTokens` bridge |
| Accessibility | `accesskit` via `egui::Context` | `vendor/accesskit` 0.25.2 + `os_pilot` | Unified `[patch.crates-io]` -> `vendor/accesskit` 0.25.2 |
| Reusable Controls | Dock tabs, combo menus, property rows, IME wire composer | `StarterButton`, `ExpressiveOmnibar`, `CascadingPopoverCard`, `SegmentedStudioBar` | [`craft_controls.rs`](../../crates/egui_gpui/src/craft_controls.rs) natively on `egui 0.37` |
| Styling Source | Ad-hoc dark theme structs | 3-tier `GoogleDesignTokens` (`assets/os/google_design_tokens.toml`) | `google_design_to_egui_visuals()` + `shadcn_theme_to_egui_visuals()` |

### 1.2 Zed Upstream `crates/gpui/Cargo.toml` vs `emilk/egui/Cargo.toml` (`vendor/egui`)

| Manifest Aspect | `crates/gpui/Cargo.toml` (Zed Layout) | `vendor/egui/Cargo.toml` (`emilk/egui` 0.37) | Monorepo Resolution |
| --- | --- | --- | --- |
| Workspace Resolver | `resolver = "2"`, centralized `[workspace.dependencies]`, `[lints] workspace = true` | `resolver = "2"`, 15 subcrates (`ecolor`, `emath`, `epaint`, `egui`, `egui-wgpu`, `egui_extras`, …) | Core 7 `egui` crates registered in root `[workspace.dependencies]` + `[patch.crates-io]` |
| Crate Placement | First-party crates under `crates/<name>/src/<name>.rs` (zero `mod.rs`) | `vendor/egui/crates/*` | Bridge placed at [`crates/egui_gpui/src/egui_gpui.rs`](../../crates/egui_gpui/src/egui_gpui.rs) matching `crates/gpui` |
| WGPU Version | `wgpu = "30.0.0"` (`vendor/wgpu`) | `egui-wgpu` upgraded to `wgpu = "30.0.0"` | Zero duplicate `wgpu` or `naga` versions in `Cargo.lock` |
| Tooling & Lints | `cargo xtask package-conformity`, `cargo xtask licenses`, `deny.toml` | `cargo deny`, `typos`, `wasm32` checks | Validated by `cargo xtask package-conformity` + `bun tooling/bun/web-build.ts check` |

---

## 2. All 82 `storytold` Repositories Mapped in One Monorepo

Tracked declaratively in [`assets/os/storytold_ecosystem.toml`](../../assets/os/storytold_ecosystem.toml) and audited via [`tooling/bun/storytold-unify.ts`](../../tooling/bun/storytold-unify.ts) (`bun run storytold:audit`):

1. **Core Engines & Monorepos (4)**: `craft-libs` (`vendor/craft-libs`, 165 crates), `craft-anything` (`vendor/craft-anything`, 13 crates), `egui` (`vendor/egui` + `crates/egui_gpui`), `artcraft`.
2. **18 Integrated Studio Apps (`assets/os/craft_catalog.toml` + `crates/craft_studio`)**: `figcraft`, `cadcraft`, `meshcraft`, `anicraft`, `pixelcraft`, `vectorcraft`, `gradecraft`, `rawcraft`, `drawcraft`, `cutcraft`, `texcraft`, `sfxLab`, `typecraft`, `writecraft`, `gridcraft`, `deckcraft`, `pdfcraft`, and `unrealcraft` (Unreal Bevy 5.8).
3. **Bevy 3D / CAD / Simulation Stack (11 -> `crates/bevy_gpui`)**: `bevy-cad`, `bevy-mesh`, `bevy-sculpt`, `bevy-skeleton`, `bevy-mocap`, `bevy-cloth`, `bevy-hair`, `bevy-2d-animator`, `bevy-timeline`, `bevy-gpu-Readback`, `bevy-xsens-mocap`.
4. **Pure-Rust Codecs, Parsers & Renderers (38 -> `vendor/craft-libs/crates/*` & `crates/os_craft`)**: `.fig` (`craft-fig`, `craft-kiwi`), `.sketch` (`craft-sketch`), `.penpot` (`craft-penpot`), `.mtlx` (`craft-materialx`), `.usdc` (`craft-usd`), `.psd`/`.ora`/`.kra`/`.xcf`/`.exr`/`.hdr`/`.dds`/`.tga`, `.docx`/`.xlsx`/`.pptx`/`.csv`, `.dxf`/`.gcode`, `.midi`/`.sf2`, ELF/PE/Mach-O/archive readers.
5. **AI, Reverse-Engineering & GPU Compute (11 -> `crates/craft_studio` & `crates/os_craft` MCP)**: `craft-anything` Oxc JS/TS AST + x86_64/ARM64/Wasm disassembler + SHA-256 evidence store, exposed via `storytold_ecosystem_status` and `craft_anything_js_analyze` MCP tools.

---

## 3. Unreal Engine 5.8 Equivalent (`crates/bevy_gpui`)

[`crates/bevy_gpui`](../../crates/bevy_gpui/src/bevy_gpui.rs) brings an Unreal Engine 5.8-grade 3D pipeline into Zed without duplicating `winit`, `wgpu`, or `accesskit`:

- **Archetype ECS World (`ecs_world.rs`)**: Deterministic entity/component storage (`Transform3d`, `CameraRig3d`, `DirectionalSunLight`, `PointLight3d`, `MocapTrackComponent`, `MeshAssetComponent`, `MaterialSurfaceComponent`).
- **Nanite-Style Virtual Geometry (`virtual_geometry.rs`)**: Automatic 64-vertex / 124-triangle meshlet builder (`build_virtual_geometry`), bounding spheres, normal cones, and per-frame frustum + backface-cone culling (`cull_meshlets`).
- **MaterialX / OpenPBR -> Naga v30 WGSL + Solari GI (`material_solari.rs`)**: Parses `craft-materialx` XML documents (`OpenPBR`, `StandardSurface`, `GltfPbr`, `UsdPreviewSurface`), compiles them to WGSL shaders, and validates every shader through `naga::front::wgsl` + `naga::valid::Validator`.
- **Viewport & `bevy_egui` Inspector (`viewport.rs`)**: Software + WGPU-compatible RGBA8 GGX PBR + soft-shadow + Nanite meshlet-heatmap renderer paired with an immediate-mode `bevy_egui` 0.37 outliner & telemetry overlay.

---

## 4. Google Design + Stitch Studio + Dual-GUI Parity (`crates/craft_studio`)

[`crates/craft_studio`](../../crates/craft_studio/src/craft_studio.rs) hosts a single shared document state (`CraftStudioSession`) across:
- **FigCraft — Google Design Edition** (`figcraft_core.rs`): Live `.fig` / Kiwi / SVG / W3C DTCG token bridge synced to `GoogleDesignTokens` and [`docs/design/`](../design/manifest.json).
- **Craft Anything — Google Stitch Edition** (`craft_anything_core.rs`): Clean-room binary + Oxc JS/TS reverse-engineering into Google Design / Stitch studio cards.
- **Unreal Bevy 5.8 Studio**: Live 3D procedural scene + MaterialX shader + Nanite meshlet stats.
- **Dual-Frontend Parity (`dual_frontend.rs`)**: Renders the exact same `CraftStudioSession` through **GPUI** (`render_gpui_studio_card`) and **`egui 0.37`** (`render_egui_studio_frame`), wired into [`crates/nsh_workspace/src/design_stitch_studio.rs`](../../crates/nsh_workspace/src/design_stitch_studio.rs) via `OpenGoogleFigCraftStudio`, `OpenGoogleCraftAnythingStudio`, `OpenUnrealBevyStudio`, and `ToggleStudioGuiParity`.
