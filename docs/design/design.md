# Google Design System (`design.google`) — Aphrody Contract

> Source: `https://design.google/?home` & `https://design.google/library/code-is-a-design-material`

## 1. Core Principles
- **Code is a Design Material**: prototypes, tokens, and production GPUI/NSH components share a single typed contract ([schema.ts](./schema.ts), [manifest.json](./manifest.json)).
- **Material 3 Expressive**: semantic surface hierarchy, adaptive light/dark tokens ([tokens.css](./tokens.css)), expressive spring motion (`cubic-bezier(0.22, 1, 0.36, 1)`), and `Google Sans Flex` / `JetBrains Mono` typography.
- **Zero Ad-Hoc Styling**: all screens, assets, and features consume semantic tokens from `tokens.css` / `os_ui::google_design` (`assets/os/google_design_tokens.toml`).

## 2. Included Assets & Kits
- **Optimized SVGs**: `docs/design/svg/design-google-1.svg` .. `design-google-6.svg`
- **Guidelines (MDX)**: `docs/design/guidelines/code-is-a-design-material.mdx`
- **Design Kits**: `docs/design/kits/figma-design-kit.json`, `docs/design/kits/stitch-design-kit.json`
- **GPUI Components**: mapped in `manifest.json` (`os_ui`, `gpui_kit`, `nsh_base`).
