# Google Design, `DESIGN.md`, Modern Web Guidance & Stitch Architecture

`https://design.google/` (Material 3 Expressive, [*Code is a Design Material*](https://design.google/library/code-is-a-design-material), and [*Google Sans Flex*](https://design.google/library/google-sans-flex-font)) is the single visual design source of truth for the Aphrody monorepo and every application scaffolded with GPUI, GPUI Kit, NSH, or Wasm/WebGPU.

## 1. Source of Truth Chain

- **Product Manifest**: `[design]` in [product.toml](../../product.toml) sets `source_of_truth = "https://design.google/"`, `stitch_url = "https://stitch.withgoogle.com/"`, `sans_flex_family = "Google Sans Flex"`, `design_md = "DESIGN.md"`, and binds [google_design_tokens.toml](../../assets/os/google_design_tokens.toml) and [themes.json](../../assets/os/themes.json), enforced by `cargo xtask brand check` via [brand.rs](../../tooling/xtask/src/tasks/brand.rs) and `brand::DESIGN_*` constants in [crates/brand](../../crates/brand/src/brand.rs).
- **Root `DESIGN.md` & 3-Tier Design Tokens**: [DESIGN.md](../../DESIGN.md) (`google-labs-code/design.md` spec) and [google_design_tokens.toml](../../assets/os/google_design_tokens.toml) structure Reference (`[ref.palette]`, `[ref.typeface]`, `[ref.variable_axes]`), System (`[sys.*]`), and Component (`[comp.*]`) tokens, verified for WCAG 2.2 AA (`>= 4.5:1`) contrast in Light and Dark modes.
- **CLI Replacements (`xh` + `monolith`)**: Installed alongside native pure-Rust `ureq` + `roxmltree` + `html5ever` mirroring so both shell tooling and compiled binaries avoid legacy `curl`/`wget` subprocess overhead.

## 2. Paired Pure-Rust Full-Stack Mirror & Zed Integration

- **Fused Core Crate**: [crates/aphrody_design](../../crates/aphrody_design/src/aphrody_design.rs) combines pure-Rust live HTTP/sitemap mirroring (`http_mirror`), `DESIGN.md` parsing/linting/diff/export (`design_md`), `stitch-sdk` & `stitch.withgoogle.com` screen codegen (`stitch_client`), `GoogleChrome/modern-web-guidance-src` Baseline auditing (`modern_web_guidance`), and PNG screen captures (`screen_capture`).
- **OS UI Design API**: [google_design.rs](../../crates/aphrody_ui/src/google_design.rs) pairs `aphrody_design` with `aphrody_ui` for M3 Expressive analytical springs, `GoogleSansFlexAxes`, `WindowSizeClass`, CSS custom properties, and WGSL `GoogleDesignTheme` uniforms.
- **Native Zed / Aphrody IDE Panels**: [design_stitch_studio.rs](../../crates/aphrody_nsh/src/design_stitch_studio.rs) registers `OpenGoogleDesignMirror`, `OpenStitchStudio`, and `CaptureDesignScreen` directly inside `aphrody_nsh` (loaded by default in [crates/zed](../../crates/zed/src/main.rs)).

## 3. Sync & Audit Gates

- Regenerate tokens, sitemap snapshot & `DESIGN.md`: `bun tooling/bun/google-design-sync.ts sync` (`bun run design:sync`).
- Audit monorepo, themes & templates: `bun tooling/bun/google-design-sync.ts audit` (`bun run design:audit`).
- Test pure-Rust mirror, Stitch & UI crates: `cargo test -p brand -p aphrody_design -p aphrody_ui`.
