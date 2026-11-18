---
name: gpui
description: >-
  Core GPUI UI framework engineering skill for aphrody covering Entity state,
  Render views, Element custom painting, Taffy Flexbox/Grid layout, FocusHandle,
  Actions, Async spawning, and multi-backend rendering (gpui_wgpu, gpui_windows,
  gpui_macos, gpui_linux, gpui_web, gpui_android). Use before designing or editing
  native UI, elements, windows, events, or GPUI crate internals.
---

# GPUI Core Framework in aphrody

GPUI (`crates/gpui`) is the hybrid immediate/retained-mode GPU UI framework powering Aphrody (`crates/zed`), `gpui-kit` components, and `NSH` Native Shell surfaces across Windows, macOS, Linux, Android, and Web/WASM.

## Workspace Crate Map

- `crates/gpui` — Core platform-independent reactive state (`App`, `Entity<T>`, `WeakEntity<T>`, `Context<T>`, `Window`), `Render`, `IntoElement`, `Element`, Taffy layout engine, text system, actions, animations.
- `crates/gpui_wgpu` — Cross-platform WGPU/Naga GPU UI renderer (9 pipelines, reverse-Z depth, Oklab gradients, dual-source subpixel text, headless PNG capture).
- `crates/gpui_windows` — Windows Win32 windowing, DWrite/D2D/D3D11/DX12 platform integration, AMD AGS, DirectComposition.
- `crates/gpui_macos`, `crates/gpui_linux`, `crates/gpui_web`, `crates/gpui_android`, `crates/gpui_platform` — OS windowing, clipboard, display, input, and WebGPU/WebGL2 browser targets.
- `crates/gpui_ui`, `crates/gpui_code`, `crates/gpui_app_shell` — Clean modular SDK facades decoupling reusable UI/code/app-shell primitives from full editor/LLM transitive bloat.

## Essential Patterns

1. **Entities & Borrowing**: Read/update models via `entity.read(cx)` / `entity.update(cx, |this, cx| ...)`. Hold only `WeakEntity<T>` inside closures (`cx.listener`, `cx.spawn`, `observe`) and check `.upgrade()` inside async blocks to avoid leaks or panics.
2. **State Updates & Repaint**: Mutate state cleanly via `entity.update(cx, ...)` and call `cx.notify()` only when state changes; keep `render()`/`prepaint()`/`paint()` pure of per-frame global writes or gratuitous notifications so GPUI Fast retention (`gpui-fast` skill) reuses/splices subtrees cleanly.
3. **Elements & Layout**: Prefer composable `div().flex().flex_col().gap_2()` chains (`taffy` 0.15) with `ScrollHandle`/`ListState` for scrolling; use custom `Element` (`request_layout` -> `prepaint` -> `paint`), `Path<T = Pixels>` (`Path::translate`), and 1-arg `window.paint_layer(|window| ...)` for virtualized canvases, GPU shader quads, or hitboxes (`HitboxBehavior`).
4. **Boundary Architecture**: Never leak `settings_content`, `cloud_llm_client`, `rpc`, or `proto` into `gpui_ui` or `--no-default-features` `gpui_code`.
5. **Testing & Benchmarking**: Pair every change with `gpui::test` deterministic scheduler tests (see `gpui-test`) and Criterion benchmarks via `gpui::bench` (see `gpui-bench`).

## Deep References

- Complete API guides in `.agents/plugins/aphrody/skills/gpui-kit/references/gpui/`:
  - `entity.md`, `entity-api.md`, `entity-patterns.md`, `entity-best-practices.md`
  - `element.md`, `element-api.md`, `element-patterns.md`, `element-best-practices.md`
  - `context.md`, `action.md`, `event.md`, `focus-handle.md`, `global.md`, `async.md`, `layout-style.md`
  - `test.md`, `test-reference.md`, `test-examples.md`
- GPU renderers & shader stack: see `wgpu`, `vulkan`, `directx`, `webgpu`, `nsh`, `gpui-kit`, `gpui-fast`.
- Full indexed brain: `~/.gemini/brain/gpui/INDEX.md` (`bun ~/.gemini/scripts/learn.ts search "<terms>"`).
