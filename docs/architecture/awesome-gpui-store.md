# Awesome GPUI Store — Live Catalog, NSH + GPUI Cross-Platform UI & 22-Target Packager

> Reproducible architecture specification transforming upstream [`zed-industries/awesome-gpui`](https://github.com/zed-industries/awesome-gpui) into a live-synced catalog, cross-platform NSH + GPUI Store UI, unified `upt` package-manager resolver, and 22-target store/distro packaging pipeline.

---

## 1. Architecture Overview

```text
https://github.com/zed-industries/awesome-gpui (README.md + gh api graphql)
        │
        ▼  bun run tooling/bun/awesome-gpui-store.ts sync
assets/os/awesome_gpui_store.toml   (bundled via include_str! in aphrody_store)
        │
        ├──▶ crates/aphrody_store/src/aphrody_store.rs  (AwesomeGpuiCatalog + upt 0.9.0 OS resolver)
        ├──▶ crates/aphrody_store/src/nsh_store.rs   (STORE_NSH_TSX + StoreNshRoot + aphrody_ui/GPUI view)
        └──▶ crates/aphrody_store/src/packagers.rs & tooling/bun/awesome-gpui-store.ts
                 └── 22 Target Store, Distro, Mobile, Browser & OCI Manifests
```

- **Live Catalog**: [`assets/os/awesome_gpui_store.toml`](../../assets/os/awesome_gpui_store.toml) merges first-party identity metadata from [`product.toml`](../../product.toml) (`aphrody`, `aphrody-os`, `gpui-store`) with every entry parsed from live `zed-industries/awesome-gpui` and enriched via batched `gh api graphql` queries (stars, SPDX license, default branch, topics, archive state, and `HEAD:Cargo.toml` crate/version/binary inspection).
- **Bun Automation CLI**: [`tooling/bun/awesome-gpui-store.ts`](../../tooling/bun/awesome-gpui-store.ts) provides `sync`, `list`, `package`, and `verify`, backed by unit/integration tests in [`tooling/bun/awesome-gpui-store.test.ts`](../../tooling/bun/awesome-gpui-store.test.ts).
- **Rust Store Crate**: [`crates/aphrody_store/Cargo.toml`](../../crates/aphrody_store/Cargo.toml) (`aphrody_store`) runs natively on Windows/macOS/Linux desktop, `wasm32-unknown-unknown` WebGPU, and iOS/Android mobile surfaces.

---

## 2. Catalog Schema (`assets/os/awesome_gpui_store.toml`)

Top-level header fields:

- `schema_version` (`1`), `source_repo` (`"zed-industries/awesome-gpui"`), `source_url`, `synced_at` (ISO-8601 UTC), `upstream_stars`, `total_entries`.

Each `[[entries]]` table defines:

| Field | Type / Values | Purpose |
| :--- | :--- | :--- |
| `id`, `name`, `tagline`, `description` | `String` | Stable slug, display title, short summary (WinGet/Snap compliant), full description |
| `category`, `subcategory` | `app` \| `library` \| `showcase` \| `tool` \| `resource` | Store rail grouping plus `UI Kits`, `Developer Tools`, `Productivity`, `Media`, `System Utilities`, `Demos & Examples` |
| `repo`, `homepage`, `icon_url` | `String` | Canonical source repository, project site, and icon URL |
| `publisher`, `publisher_id`, `app_id` | `String` | Publisher display name, PascalCase WinGet ID prefix, and reverse-DNS `app_id` |
| `crate_name`, `npm_package`, `bin_name` | `String` | Live-extracted Cargo package name, scoped npm package, and binary name |
| `version`, `license`, `stars`, `status` | `String` / `u64` / enum | Normalized semver, SPDX license, live star count, and `active` / `quiet` / `dormant` / `archived` / `unavailable` |
| `platforms` | `windows`, `macos`, `linux`, `freebsd`, `web`, `ios`, `android` | Runtime platform compatibility vector |
| `targets` | Array of 22 target slugs | Enabled distribution ecosystems |
| `tags`, `first_party` | `Vec<String>`, `bool` | Topic tags and first-party flag |

---

## 3. All 22 Store, Distro, Mobile, Browser & OCI Packaging Targets

Generated deterministically by both [`crates/aphrody_store/src/packagers.rs`](../../crates/aphrody_store/src/packagers.rs) and [`tooling/bun/awesome-gpui-store.ts`](../../tooling/bun/awesome-gpui-store.ts):

| Target Slug | Ecosystem | Generated Artifacts |
| :--- | :--- | :--- |
| `winget` | Windows Package Manager | `<Publisher>.<Pkg>.yaml`, `.installer.yaml`, `.locale.en-US.yaml` (ManifestVersion `1.9.0`) |
| `msstore` | Microsoft Store (MSIX) | `AppxManifest.xml` (4-part MSIX version + `runFullTrust`) |
| `scoop` | Scoop (Windows) | `bucket/<slug>.json` (x64 + arm64 + `checkver`/`autoupdate`) |
| `choco` | Chocolatey (Windows) | `<slug>.nuspec` + `tools/chocolateyinstall.ps1` |
| `flatpak` | Flatpak / Flathub | `<app_id>.yml` (`org.freedesktop.Sdk.Extension.rust-stable`) + `<app_id>.metainfo.xml` |
| `snap` | Ubuntu Snap | `snap/snapcraft.yaml` (`core24`, strict confinement, `<=78` char summary) |
| `arch` | Arch Linux `pacman` / AUR | `PKGBUILD` + `.SRCINFO` |
| `nix` | Nix / NixOS / nix-darwin | `flake.nix` (`rustPlatform.buildRustPackage`, Wayland/Vulkan deps) |
| `apk` | Alpine Linux `apk` | `APKBUILD` (`abuild` recipe with Wayland/Vulkan/fontconfig deps) |
| `rpm` | Fedora / RHEL / openSUSE `rpm` | `<slug>.spec` + `rpm-metadata.toml` (`[package.metadata.generate-rpm]`) |
| `deb` | Debian / Ubuntu `.deb` (`apt`) | `DEBIAN/control` + `deb-metadata.toml` (`[package.metadata.deb]`) |
| `xbps` | Void Linux `xbps` | `srcpkgs/<slug>/template` (`build_style=cargo`) |
| `gentoo` | Gentoo `emerge` | `<slug>-<ver>.ebuild` (`EAPI=8`, `inherit cargo`) |
| `freebsd` | FreeBSD `pkg` | `Makefile` (`USES=cargo pkgconfig`) + `pkg-plist` |
| `brew` | Homebrew | `Formula/<slug>.rb` (`std_cargo_args`) |
| `appimage` | Linux AppImage | `AppImageBuilder.yml` |
| `oci` | Docker / OCI Container | Multi-stage `Dockerfile` (`distroless/cc-debian12` + OCI labels) |
| `playstore` | Google Play Store (AAB) | `AndroidManifest.xml` + `build.gradle.kts` (`cargo-ndk` ABI build recipe) |
| `appstore` | Apple App Store (macOS/iOS) | `Info.plist` + `<Bin>.entitlements` + `ExportOptions.plist` |
| `cargo` | crates.io / Cargo | `Cargo.toml` (`cdylib` + `rlib`, `[package.metadata.deb]`, `[package.metadata.generate-rpm]`, `[profile.web]`) |
| `npm` | npm Dual Distribution | `package.json` (`.`, `./wasm`, `./nsh` exports + `nsh` metadata) |
| `chrome` | Chrome Web Store (MV3) | `manifest.json` (`'wasm-unsafe-eval'` CSP), `popup.html`, `host.js` (WebGPU), `service_worker.js` |

---

## 4. Unified Rust Package-Manager & Packaging Crate Comparison

Live versions verified against `crates.io`:

| Crate | Verified Version | Role in Aphrody Store Ecosystem |
| :--- | :--- | :--- |
| **`upt`** ([`sigoden/upt`](https://github.com/sigoden/upt)) | `0.9.0` | **Wired directly into `aphrody_store`** (`upt.workspace = true`). Lightweight zero-bloat runtime command translator (`upt::{detect_os, detect_vendor, init_vendor, Vendor}`) mapping `install`/`remove`/`upgrade`/`search`/`info`/`update`/`list` cleanly across 28 OS vendors (`apk`, `apt`, `brew`, `cards`, `choco`, `dnf`, `emerge`, `eopkg`, `flatpak`, `guix`, `nala`, `nix-env`, `opkg`, `pacman`, `pkg`, `pkg(termux)`, `pkgman`, `prt-get`, `scoop`, `slackpkg`, `snap`, `upt`, `urpm`, `winget`, `xbps`, `yay`, `yum`, `zypper`). |
| **`metapac`** ([`ripytide/metapac`](https://github.com/ripytide/metapac)) | `0.10.3` | Declarative multi-package-manager sync CLI/tool useful for reproducible workstation provisioning alongside `upt`. |
| **`cargo-packager`** ([`crabnebula-dev/cargo-packager`](https://github.com/crabnebula-dev/cargo-packager)) | `0.11.8` | End-to-end Rust binary bundler emitting DMG, NSIS/WiX MSI, AppImage, `.deb`, and Pacman archives directly from compiled binaries (complements [`cross-compile-and-depot-tools.md`](./cross-compile-and-depot-tools.md) and [`production-guide.md`](./production-guide.md)). |
| **`rpm`** ([`rpm-rs/rpm`](https://github.com/rpm-rs/rpm)) | `0.30.2` | Pure-Rust RPM archive reader/builder library without needing host `rpmbuild`. |
| **`cargo-generate-rpm`** ([`cat-in-136/cargo-generate-rpm`](https://github.com/cat-in-136/cargo-generate-rpm)) | `0.21.1` | Binary `.rpm` packager consuming `[package.metadata.generate-rpm]` (generated in our `cargo` & `rpm` targets). |
| **`cargo-deb`** ([`kornelski/cargo-deb`](https://github.com/kornelski/cargo-deb)) | `3.8.0` | Pure-Rust Debian `.deb` packager consuming `[package.metadata.deb]` (generated in our `cargo` & `deb` targets). |

---

## 5. NSH + GPUI Cross-Platform Store UI

Implemented in [`crates/aphrody_store/src/nsh_store.rs`](../../crates/aphrody_store/src/nsh_store.rs):

- **`STORE_NSH_TSX`**: Portable NSH TSX application using `"nsh"` (`signal`, `computed`, `host`, `mount`, `View`, `Text`, `Scroll`), `"nsh/base"` (`Button`, `Input`), global `<Each>` / `<If>`, and `env(safe-area-inset-*)` mobile/desktop/web padding. Compiles cleanly via `nsh_bundler::compile_module` under `nsh-dev`.
- **`StoreNshRoot`**: Native Rust `#[component]` reactive tree built with `nsh_core` + `nsh_base` (`button`, `theme`, `input`, `scroll`, `each`), mountable via `store_nsh_root()` or desktop `mount_store_nsh_surface` (`gpui_ui::nsh::AppView` with live GPUI theme sync and `package_preview` / `upt_command` host handlers).
- **`render_store_window`**: Adaptive `gpui` + `aphrody_ui` Store interface using `AdaptiveViewport` (`Compact` / `Medium` / `Expanded`), `render_nav_rail`, category & 22-target filter chips, live `upt` host detection badge, compatibility matrix, and `ImmediateUi` manifest preview/export inspector.

---

## 6. CLI Verification & Workspace Integration

```sh
# 1. Sync live awesome-gpui README + gh api metadata into assets/os/awesome_gpui_store.toml
bun run tooling/bun/awesome-gpui-store.ts sync

# 2. List catalog counts by category and target
bun run tooling/bun/awesome-gpui-store.ts list --category library

# 3. Generate all 22 targets for aphrody (or any upstream entry)
bun run tooling/bun/awesome-gpui-store.ts package --id aphrody --target all --out target/gpui-store-dist

# 4. Structurally verify every generated XML/YAML/JSON/TOML/spec/control/PKGBUILD/flake/Dockerfile
bun run tooling/bun/awesome-gpui-store.ts verify --dir target/gpui-store-dist

# 5. Run unit + packaging test suite
bun test tooling/bun/awesome-gpui-store.test.ts
```

Coordinator wiring for root `Cargo.toml`:
- `[workspace].members`: add `"crates/aphrody_store"`
- `[workspace.dependencies]`: add `aphrody_store = { path = "crates/aphrody_store" }` and `upt = "0.9.0"`
