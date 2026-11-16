---
name: aphrody-brand
description: >-
  Reproducible declarative identity pipeline for aphrody driven by product.toml,
  crates/brand, and `cargo xtask brand apply|check`. Use whenever changing product
  names, domains, URLs, app IDs, SVG/PNG/ICO icons, installer manifests, or
  verifying brand & Zed compatibility alignment.
---

# Aphrody Declarative Brand Pipeline

All product identity metadata flows from a single source of truth: [`product.toml`](../../../../product.toml). Never scatter hardcoded product strings across crates by hand.

## Components

1. **`product.toml`** (root): Defines product name (`Aphrody`), slug (`aphrody`), reverse-DNS ID, organization (`Aphrody Labs`), domains (`aphrody.com`), URL schemes (`aphrody`, compat `zed`), legacy config fallbacks, and icon source (`assets/images/aphrody_logo.svg`).
2. **`crates/brand`** (`crates/brand/build.rs`, `src/brand.rs`): Zero-dependency compile-time parser generating `APP_NAME`, `APP_SLUG`, `ORG_NAME`, `HOMEPAGE_URL`, `URL_SCHEME`, `COMPAT_URL_SCHEME`, `DOCK_ACTION_SCHEME_PREFIX`, `CLI_URL_SCHEME_PREFIX`, `CONFIG_DIR_NAME`, `LEGACY_CONFIG_DIR_NAME`, etc. Tested via `cargo test -p brand`.
3. **`tooling/xtask/src/tasks/brand.rs`**:
   - `cargo xtask brand apply`: Reads `product.toml` + `assets/images/aphrody_logo.svg` (rendered via `resvg` + `tiny-skia` + `image`), regenerates Windows `.ico` / `.png` app icons across channels, and synchronizes `Cargo.toml` / `.desktop.in` / InnoSetup manifests deterministically.
   - `cargo xtask brand check`: Fails non-zero if any generated manifest or icon drifts out of sync with `product.toml`.

## Verification Checklist

1. After editing `product.toml` or `assets/images/aphrody_logo.svg`, run `cargo xtask brand apply`.
2. Verify zero drift before committing or in CI:
   ```powershell
   cargo xtask brand check
   cargo test -p brand
   cargo test -p xtask --bins
   ```
3. Verify `zed.exe`/`zed://`/`.zed` fallbacks remain intact so Zed extensions, settings, keymaps, and themes keep working seamlessly.
