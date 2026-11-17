# Monorepo WebAssembly & WebGPU First Architecture

Aphrody treats **WebAssembly (`wasm32-unknown-unknown`, `wasm32-wasip2`)** and **WebGPU (`wgpu` / WGSL v30 + `naga`)** as primary targets alongside native desktop and mobile shells, rather than secondary portability layers.

## Core Invariants

1. **WebGPU First (`WebBackendPreference::Auto`)**:
   - [`WgpuContext::new_web_with_backend`](../../crates/gpui_wgpu/src/wgpu_context.rs) constructs instances with `wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL` via `wgpu::util::new_instance_with_webgpu_detection`, probing `high-performance` and compatibility WebGPU adapters before falling back to WebGL2.
   - Desktop Linux/Windows/macOS renderers share the same WGSL shaders (`shaders.wgsl`, `shaders_storage.wgsl`, `shaders_subpixel.wgsl`) validated by [`aphrody_cli`](../../crates/aphrody_cli/src/main.rs).
2. **Non-Wasm Side Effects Isolated via `cfg`**:
   - Core crates in `[workspace].default-members` (`gpui`, `gpui_wgpu`, `gpui_web`, `gpui_ui`, `gpui_code`, `aphrody_session`, `aphrody_craft`, `aphrody_ui`, `aphrody_desktop`, `aphrody_store`, `aphrody_cli`) gate OS-only process/filesystem/native-package-manager crates (such as `upt` in [`aphrody_store`](../../crates/aphrody_store/Cargo.toml)) behind `[target.'cfg(not(target_family = "wasm"))'.dependencies]`.
   - Core hardware/compute primitives in [`aphrody_session::wasm_hal`](../../crates/aphrody_session/src/wasm_hal.rs) run from one codebase on both bare-metal native ISAs (`core::arch::asm!`) and Wasm Core 2.0 (`v128` SIMD + linear memory).
3. **Three-Cohort Browser Delivery (`tooling/bun/web-build.ts`)**:
   - `baseline`: universal browser support (`wasm32-unknown-unknown`), strictly verified not to import shared memory or `__wasm_init_tls`.
   - `simd-baseline`: `-C target-feature=+simd128` for 128-bit vector throughput without Cross-Origin Isolation headers.
   - `multithreaded` (`--coi`): opt-in `-C target-feature=+atomics,+bulk-memory,+mutable-globals` with COOP/COEP headers for threaded Wasm workers.
4. **WASI Preview 2 (`wasm32-wasip2`), LTO/AOT Profiles & Starter Templates**:
   - Pure logic crates (`aphrody_session`, `aphrody_craft`, `release_channel`, `brand`) cross-check cleanly on `wasm32-wasip2`, matching the host workspace's Wasmtime P2 AOT runtime (`.aot-cache/*.cwasm` in [`crates/extension_host`](../../crates/extension_host/src/wasm_host.rs)).
   - `[profile.web-lto]` (browser fat-LTO + `-Oz` + `wasm-opt`) and `[profile.aot]` (native `.cwasm` AOT) back `bun tooling/bun/web-build.ts build --lto|--aot`, `gpui-wgpu app|wasm|design|mcp`, and the 5 standalone starter templates documented in [`awesome-wasm-wgpu-gpui.md`](./awesome-wasm-wgpu-gpui.md).

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
