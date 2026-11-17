---
name: google-design
description: Authoritative design system specification anchored to https://design.google/ (Material 3 Expressive, Code is a Design Material, Google Sans Flex) across the entire Aphrody monorepo, GPUI, gpui-kit, NSH, Stitch Studio, and all Wasm/WebGPU applications.
---

# Google Design Source of Truth (`https://design.google/`)

Every UI surface, theme, component token table, shader palette, and app template in Aphrody derives from **`https://design.google/`** — uniting:
- **Material 3 Expressive** 3-tier tokens (`assets/os/google_design_tokens.toml`, `os_ui::google_design`)
- [*Code is a Design Material*](https://design.google/library/code-is-a-design-material): live code + shaders as expressive design material
- [*Google Sans Flex*](https://design.google/library/google-sans-flex-font): 6 variable axes (`opsz`, `wght`, `wdth`, `GRAD`, `slnt`, `ROND`) paired with monospace fonts (`JetBrains Mono` / `.AphrodyMono`) for code
- Root [`DESIGN.md`](../../../../DESIGN.md), `GoogleChrome/modern-web-guidance-src`, and `stitch.withgoogle.com` natively mirrored in `google_design_stitch` and `nsh_workspace`

## Spatial vs Effects Physics Springs

- **Spatial springs** (`damping_ratio = 0.9`): size, layout, position, and shape morphing (`spatial_fast = 1400`, `spatial_default = 700`, `spatial_slow = 300`).
- **Effects springs** (`damping_ratio = 1.0`): opacity, color, and elevation transitions (`effects_fast = 3800`, `effects_default = 1600`, `effects_slow = 800`).

## Automation & Native Commands

- Sync tokens, sitemap snapshot & `DESIGN.md`: `bun run design:sync`
- Audit monorepo & templates: `bun run design:audit`
- Live Rust HTTP/sitemap mirror (`curl`/`wget`/`monolith` fusion): `gpui-wgpu design mirror https://design.google/`
- Studio & capture inside Aphrody IDE: `nsh_workspace::OpenGoogleDesignMirror`, `nsh_workspace::OpenStitchStudio`, `nsh_workspace::CaptureDesignScreen`
