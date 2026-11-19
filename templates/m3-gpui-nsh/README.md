# Material 3 1.5.0 -> GPU / GPUI Kit / NSH Starter Template

Generated deterministically by `bun run m3:convert` from:
- Compose Material 3 `1.5.0` Stable (`compose-bom:2026.11.01`): https://m3.material.io/blog/compose-material-3-is-stable
- Material 3 Parametric Prototyping: https://m3.material.io/blog/enhance-your-component-designs-with-parametric-prototyping
- Official Material 3 Figma Design Kit (`1035203688168086460`): https://www.figma.com/community/file/1035203688168086460/material-3-design-kit

## Contents

- `src/main.rs` — Native GPUI Kit app launching `M3ParametricPrototyperView` with `M3ExpressiveThemeScope`
- `nsh-m3-kit.tsx` — NSH TSX library with Compose M3 `1.5.0` components, 35 `MATERIAL_SHAPES_35`, and 4-mode palettes
- `m3-tokens.css` — 4-mode CSS custom property token sheet (`light`, `dark`, `neural_obsidian`, `oled_pitch`)
- `shaders/m3_expressive.wgsl` — Pure WebGPU / Naga v30 WGSL shader (35 MaterialShapes SDF morphing, wavy progress, inset focus rings, progressive blur)
- `m3-parametric-preset.json` — Parametric Prototyping preset synced with Figma Design Kit `1035203688168086460`

## Commands

```sh
bun run m3:track
bun run m3:convert
bun run m3:prove
cargo run -p gpui_ui --features material3 --example m3_compose15_showcase
```
