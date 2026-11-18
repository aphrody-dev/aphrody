# Cross-Compilation, LLVM Toolchain & Hermetic Native SDK (`depot_tools`) Guide

> Audited live on PC (`cargo-xwin 0.23.1`, `cargo-zigbuild 0.24.1`, `zig 0.17.1`, `clang 23.1.5`, `LLD 23.1.5`) and fleet node `fr1` (`clang 23.1.6`, `lld-22`, `mold`, `sccache`, `cargo-nextest 0.9.152`, `/srv/depot_tools`).

---

## 1. Codebase Audit & Toolchain Gaps

| Area | Current State in Repository | Concrete Gap / Opportunity |
| :--- | :--- | :--- |
| **`rust-toolchain.toml`** | [`rust-toolchain.toml`](../../../rust-toolchain.toml) pins `1.98.1` with only `wasm32-wasip2`, `wasm32-unknown-unknown`, and `x86_64-unknown-linux-musl`. | Missing standard desktop/mobile cross targets (`x86_64/aarch64-unknown-linux-gnu`, `x86_64/aarch64-pc-windows-msvc`, `aarch64/x86_64-apple-darwin`, `aarch64-linux-android`) unless installed ad-hoc. |
| **Linkers & `.cargo/config.toml`** | [`.cargo/config.toml`](../../../.cargo/config.toml) only sets `-fuse-ld=lld` for `aarch64-unknown-linux-gnu` and `+crt-static` on Windows. | Linux `x86_64` falls back to default `ld` instead of `rust-lld`/`mold`; Windows uses MSVC `link.exe` rather than `lld-link`/`rust-lld.exe`. |
| **Glibc Compatibility** | CI and bundler scripts spin up legacy Ubuntu 20.04 runners to guarantee glibc <= 2.31. | `cargo-zigbuild 0.24.1` compiles glibc-pinned Linux binaries (`x86_64-unknown-linux-gnu.2.28`) directly from any modern Ubuntu/Debian host. |
| **Native C/C++ Deps** | `webrtc-sys` (`livekit-rust-sdks`) downloads prebuilt `libwebrtc.a`; `cef-rs` downloads Spotify CDN tarballs + builds `libcef_dll_wrapper` via `cmake`/`ninja`; `wgpu` v30 is pure Rust + `naga`; `libsqlite3-sys` uses `bundled`; `ring` uses `cc`. | Avoids cloning 35 GB Chromium trees for normal builds (`depot_tools` on `fr1` at `/srv/depot_tools` is reserved only if building custom CEF/ANGLE/Chromium forks). Note `libgit2-dev` in [`script/linux`](../../../script/linux) is unused by pure-Rust `git`. |
| **Cargo Profiles ([`Cargo.toml`](../../../Cargo.toml))** | Already configures `[profile.release]` (`lto = "thin"`, `debug = "limited"`), `[profile.release-fast]` (`lto = false`), `[profile.dist]` (`lto = "fat"`, `strip = "symbols"`, `panic = "abort"`), and `[profile.web]` (`opt-level = "z"`, `lto = "thin"`, `strip = true`). | Add optional PGO (`profile-generate` -> `llvm-profdata merge` -> `profile-use`) and Linux BOLT (`-Wl,--emit-relocs`) pipelines for shipping binaries. |

---

## 2. How Chromium / `depot_tools` Overhead Is Eliminated

1. **Pure Rust Graphics (`wgpu` v30 + `naga`)**: Compiles WGSL directly to Vulkan SPIR-V, DX12 DXIL/DXBC, Metal MSL, and WebGPU without Chromium Dawn/ANGLE build trees.
2. **Prebuilt WebRTC (`webrtc-sys`)**: Fetches versioned release archives containing headers and static `libwebrtc.a` / `webrtc.lib`.
   - **ABI Caution**: Upstream prebuilt Linux `libwebrtc.a` links against GNU `libstdc++` (`-std=c++20`). When cross-compiling C++ crates with `cargo-zigbuild` (which defaults to LLVM `libc++`), pass `CXXFLAGS="-stdlib=libstdc++"` or run WebRTC links natively on Linux `fr1`.
3. **Prebuilt CEF / WebView (`aphrody-labs/cef-rs` ref + [`vendor/tauri/wry`](../../../vendor/tauri/wry))**: CEF reference moved to [`vendor-reference-ecosystem.md`](./vendor-reference-ecosystem.md) (`CEF_PATH` / `CEF_DOWNLOAD_URL` avoids 35 GB Chromium checkouts), while `vendor/tauri/wry` remains vendored for `nsh-platform-gpui`. Keep `/srv/depot_tools` on `fr1` exclusively for custom Chromium/ANGLE source rebuilds.
4. **Windows CRT Caution**: `+crt-static` resolves `livekit` static lib conflicts in standalone executables, but dynamic C-API plugins/extensions (e.g. external CEF DLL boundaries) require matching CRT linkage across DLL boundaries.

---

## 3. Target Cross-Compilation Matrix

| Target Triple | Preferred Host / Fleet | Tooling Driver | Linker / Sysroot Notes |
| :--- | :--- | :--- | :--- |
| `x86_64-unknown-linux-gnu.2.28` | Linux `fr1` / CI | `cargo zigbuild` / `clang` | Pins glibc 2.28 without legacy Ubuntu 20.04 VMs; `-fuse-ld=lld` / `mold` |
| `aarch64-unknown-linux-gnu.2.28` | Linux `fr1` / CI | `cargo zigbuild` | Cross-compiles ARM64 Linux Desktop + Wayland (`-fuse-ld=lld`) |
| `x86_64-unknown-linux-musl` | Linux `fr1` / CI | `cargo zigbuild` / `musl-gcc` | Fully static binaries (`remote_server`, bundled `bwrap`) |
| `x86_64-pc-windows-msvc` | Win PC or Linux `fr1` | Native MSVC / `cargo xwin 0.23.1` | `clang-cl` + `lld-link` against cached `XWIN_CACHE_DIR` WinSDK CRT |
| `aarch64-pc-windows-msvc` | Win PC or Linux `fr1` | `cargo xwin 0.23.1` | Cross-compiles Windows on ARM64 cleanly from Linux `fr1` or x64 Windows |
| `x86_64/aarch64-apple-darwin` | macOS runner / `fr1` (`osxcross`/SDK) | Native `clang` / `cargo zigbuild` | `MACOSX_DEPLOYMENT_TARGET=12.0`, `ld64` / `ld64.lld` + `lipo` universal binary |
| `aarch64-linux-android` | Win PC (NDK 27/29/30) / `fr1` | `cargo ndk` / `cargo check` | Android Vulkan/SurfaceView GPUI & `os_*` crates |
| `wasm32-unknown-unknown` | Any host | [`tooling/bun/web-build.ts`](../../../tooling/bun/web-build.ts) | `[profile.web]`, `RUSTC_WRAPPER=""`, `wasm-bindgen 0.2.129`, `wasm-opt -Oz` |
| `wasm32-wasip2` | Any host | `cargo` (`wit-bindgen`) | Zed Wasmtime `48.0.7` sandboxed extensions |

---

## 4. Hermetic Caching & Reproducible CLI (`tooling/bun/cross-build.ts`)

Use [`tooling/bun/cross-build.ts`](../../../tooling/bun/cross-build.ts) (`bun run cross:doctor` / `bun run cross:matrix`) to inspect installed cross-compilers, LLVM (`clang`/`lld`/`llvm-profdata`), Zig, XWin, and `/srv/depot_tools`, and emit exact reproducible cross-build invocations:

```sh
# Warm cargo-xwin CRT/WinSDK cache offline on Linux build host (fr1)
export XWIN_CACHE_DIR=/var/cache/xwin
export XWIN_ARCH=x86_64,aarch64

# Cross-check OS crates from Windows or Linux targeting glibc 2.28 / Windows MSVC
bun run cross:doctor
bun run cross:check -- --target x86_64-unknown-linux-gnu --package aphrody_session
```
