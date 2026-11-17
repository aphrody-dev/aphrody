# Monorepo WebAssembly & WebGPU First Architecture

Aphrody treats **WebAssembly (`wasm32-unknown-unknown`, `wasm32-wasip2`)** and **WebGPU (`wgpu` / WGSL v30 + `naga`)** as primary targets alongside native desktop and mobile shells, rather than secondary portability layers.

## Core Invariants

1. **WebGPU First (`WebBackendPreference::Auto`)**:
   - [`WgpuContext::new_web_with_backend`](../../crates/gpui_wgpu/src/wgpu_context.rs) constructs instances with `wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL` via `wgpu::util::new_instance_with_webgpu_detection`, probing `high-performance` and compatibility WebGPU adapters before falling back to WebGL2.
   - Desktop Linux/Windows/macOS renderers share the same WGSL shaders (`shaders.wgsl`, `shaders_storage.wgsl`, `shaders_subpixel.wgsl`) validated by [`wgpu_cli`](../../crates/wgpu_cli/src/main.rs).
2. **Non-Wasm Side Effects Isolated via `cfg`**:
   - Core crates in `[workspace].default-members` (`gpui`, `gpui_wgpu`, `gpui_web`, `gpui_ui`, `gpui_code`, `os_session`, `os_craft`, `os_ui`, `os_desktop`, `gpui_store`, `wgpu_cli`) gate OS-only process/filesystem/native-package-manager crates (such as `upt` in [`gpui_store`](../../crates/gpui_store/Cargo.toml)) behind `[target.'cfg(not(target_family = "wasm"))'.dependencies]`.
   - Core hardware/compute primitives in [`os_session::wasm_hal`](../../crates/os_session/src/wasm_hal.rs) run from one codebase on both bare-metal native ISAs (`core::arch::asm!`) and Wasm Core 2.0 (`v128` SIMD + linear memory).
3. **Three-Cohort Browser Delivery (`tooling/bun/web-build.ts`)**:
   - `baseline`: universal browser support (`wasm32-unknown-unknown`), strictly verified not to import shared memory or `__wasm_init_tls`.
   - `simd-baseline`: `-C target-feature=+simd128` for 128-bit vector throughput without Cross-Origin Isolation headers.
   - `multithreaded` (`--coi`): opt-in `-C target-feature=+atomics,+bulk-memory,+mutable-globals` with COOP/COEP headers for threaded Wasm workers.
4. **WASI Preview 2 (`wasm32-wasip2`) & Host Runtime (`wasmtime 48`)**:
   - Pure logic crates (`os_session`, `os_craft`, `release_channel`, `brand`) cross-check cleanly on `wasm32-wasip2`, matching the host workspace's pinned `wasmtime = "48.0.7"` + `wasmtime-wasi` (`p2`) runtime.

## Standard Verification Commands

```sh
# 1. Full monorepo wasm32-unknown-unknown + wasm32-wasip2 check
bun run web:check

# 2. Build optimized baseline + simd128 WebGPU cohorts (wasm-bindgen 0.2.129 + wasm-opt)
bun run web:build:release

# 3. Real Chromium headless WebGPU screenshot & zero-exception DOM check
bun run web:verify

# 4. Inspect unified cross-framework WebGPU matrix
bun run web:matrix
```

In CI, [`.github/workflows/aphrody-ci.yml`](../../.github/workflows/aphrody-ci.yml) runs `wasm-webgpu-first` as an independent required check alongside Linux (`nsh-full`) and Windows checks.
