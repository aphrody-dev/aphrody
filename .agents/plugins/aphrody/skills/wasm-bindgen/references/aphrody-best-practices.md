# Aphrody Best Practices: WebAssembly & `wasm-bindgen` Pipeline

> Grounded directly in [`Cargo.toml`](../../../../../../Cargo.toml), [`tooling/bun/web-build.ts`](../../../../../../tooling/bun/web-build.ts), [`crates/gpui_web/`](../../../../../../crates/gpui_web/Cargo.toml), [`crates/aphrody_desktop/examples/web_os.rs`](../../../../../../crates/aphrody_desktop/examples/web_os.rs), and [`crates/nsh-platform-web/`](../../../../../../crates/nsh-platform-web/Cargo.toml).

## 1. Why Aphrody Uses `cargo build + wasm-bindgen CLI + wasm-opt` Over Legacy `wasm-pack`

| Capability | Legacy `wasm-pack 0.15.1` | Aphrody `tooling/bun/web-build.ts` |
| :--- | :--- | :--- |
| **Target Ecosystem** | Assumes single-bundle `npm` / `webpack` packaging (`pkg/` + `package.json`) | Produces multi-cohort web distributions (`baseline`, `simd-baseline`, opt-in `multithreaded`) + runtime WebGPU/SIMD probe HTML |
| **CLI <-> Lockfile Lock** | Can download arbitrary `wasm-bindgen` binaries via `binary-install` | `verifyBindgenVersion()` fails fast unless `wasm-bindgen` CLI (0.2.129) strictly equals `Cargo.lock` (0.2.129) |
| **Windows `web-sys` Build Fix** | Inherits `~/.cargo/config.toml` `sccache`, overflowing Windows `CreateProcess` 32 KB argv limit | Explicitly clears `RUSTC_WRAPPER=""` for `wasm32` builds (`web-build.ts:173`) while preserving native `sccache` |
| **COOP / COEP Isolation** | Single build flag forces either zero threads or broken third-party embeds | Keeps `baseline` + `simd-baseline` free of shared memory (verified via `WebAssembly.Module.imports`) and gates `+atomics` behind opt-in `--coi` / `--threads` |
| **Release Profile & Binaryen** | Default release profile; separate wasm-opt step | Enforces `[profile.web]` (`opt-level = "z"`, `lto = "thin"`, `codegen-units = 1`, `strip = true`, `panic = "abort"`) + `wasm-opt v133 -Oz` + Brotli budget gates (`--brotli-budget`) |

## 2. `gpui_web` Architecture & Runtime Invariants

- **Single Top-Level Window & Lifecycle State Machine**: `WebPlatform` enforces `WebWindowLifecycle::{Available, Open, Closed, Unavailable}`; graphics initialization runs asynchronously via `wasm_bindgen_futures::spawn_local` inside `Platform::run`, trying WebGPU first and tearing down/rebuilding a clean `<canvas>` for WebGL2 fallback if WebGPU fails (`platform.rs:230-305`).
- **Reentrancy-Safe Callbacks**: Because `[profile.web]` uses `panic = "abort"`, any double `RefCell::borrow_mut()` aborts the Wasm module irreversibly. `WebWindowInner::with_callback` and `with_input_handler` move callbacks out via `.take()` before invoking user/GPUI handlers, then restore them afterwards (`window.rs:438-454`, `window.rs:576-584`).
- **Mobile IME & Viewport Coalescing**: [`ime_mirror.rs`](../../../../../../crates/gpui_web/src/ime_mirror.rs) maintains a hidden `<textarea>` (`font-size: 16px` to defeat iOS/Android auto-zoom, `FocusOptions::set_prevent_scroll(true)`) and coalesces DOM writes via `Closure::once_into_js` + `setTimeout(0)` so mid-gesture writes never reset mobile software keyboards.
- **Font Loading & Missing Glyph Streaming**: `WebTextSystem` pairs `CosmicTextSystem::new_without_system_fonts` with `OffscreenCanvasRenderingContext2d` emoji/CJK glyph fallback (`canvas_fallback.rs`, `canvas_text.rs`) instead of embedding megabytes of fallback fonts in Wasm `.rodata`.

## 3. Dual Browser Strategy: `gpui_web` vs `nsh-platform-web`

Aphrody consciously maintains two complementary Wasm bridge architectures:

1. **`crates/gpui_web` (`wasm-bindgen` + `web-sys` + `gpui_wgpu`)**: Full Rust-rendered GPU UI (`aphrody_desktop`, `zed`, `gpui`) targeting WebGPU/WebGL2 canvases, browser Fetch streams, Clipboard API, ResizeObserver, and optional `wasm_thread` workers.
2. **`crates/nsh-platform-web` (Custom Slot Bridge [`src/browser/values.rs`](../../../../../../crates/nsh-platform-web/src/browser/values.rs))**: Compact `#[link(wasm_import_module = "nsh_browser")]` integer-op FFI (`Value(pub u32)` + packed binary buffers) built for NSH DOM/QuickJS TSX payloads where `nsh package` performs custom Wasm symbol dead-code pruning across `linkable` artifacts without shipping full `web-sys` bindings.

## 4. Cohort Verification Checklist

1. Confirm `wasm-bindgen --version` matches `Cargo.lock` (`0.2.129`).
2. Run `bun tooling/bun/web-build.ts check` across `gpui_web`, `gpui_wgpu`, `aphrody_session`, `aphrody_craft`, `aphrody_ui`, `aphrody_desktop`.
3. Build release cohorts with `bun tooling/bun/web-build.ts build --package aphrody_desktop --release --opt --brotli-budget 6500000`.
4. Verify `manifest.json` confirms `baseline.importsSharedMemory === false` and verify rendered canvas / DOM semantics using [`tooling/bun/web-verify.ts`](../../../../../../tooling/bun/web-verify.ts).
