# Design system: Material 3 from Google's official sources

Direction: re-implement nothing; fork and absorb what Google ships (Material 3 on Flutter, Jetpack Compose and Angular).

## Official implementations and status

| Platform | Repository | Status |
|---|---|---|
| Web | angular/components (Angular Material) | active; Google recommends it for the web |
| Web | material-components/material-web | in maintenance, "pending new maintainers" (README); the owner considers it legacy |
| Android | androidx/androidx (Compose material3) | active |
| Android (Views) | material-components/material-components-android | active |
| Cross-platform | flutter/flutter (material) | active |
| iOS | material-components/material-components-ios | archived |
| Colour | material-foundation/material-color-utilities | active |
| Icons | google/material-design-icons (Material Symbols) | active |
| Fonts | google/fonts, googlefonts/googlesans, googlesans-flex, googlesans-code | active |
| Tokens | material-foundation/material-tokens, material-theme-builder | archived |

Reference sources for the rules: m3.material.io and design.google (crawl, see [browser-and-crawl.md](browser-and-crawl.md)); the community Figma library "Material 3 Design Kit", already connected to the account.

## Tokens

- Google's latest official token set ("Google Material 3", version 34.x, `tokens/versions/latest/sass` in the `@material/web` package) moved to plain Sass variables. Compared with the old `v0_192` it brings a focus opacity of 0.10 (was 0.12) and a "disabled" state opacity of 0.38.
- Angular Material exposes its own `--mat-sys-*` tokens: the source to follow on the web.

## Colour (HCT)

A freestanding C port of material-color-utilities (CAM16, HctSolver, 2021 dynamic schemes) matched the MCU 0.4.0 reference to 1e-12 and a full scheme bit for bit, native and in wasm with no import. Since removed; the lesson: colour science ports exactly, but the source stays MCU.

## Rust

Google does not ship M3 for Rust. Options checked: FerroUI (Avalonia port, Material 3 Expressive theme with native components), egui-material3 (standard M3), a Slint fork (plan only). The generated inventory ([inventory/frameworks.md](inventory/frameworks.md), category `material-rust`) lists the rest.
