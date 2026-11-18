---
name: gpui-fast
description: Engineering reference for GPUI Fast retained-mode frame reuse, view splicing, dependency tracking, persistent Taffy layout keys, text measurement Carry/Replay, GPU tile scroll layers, dynamic GPU textures, and gpui-kit compatibility in Aphrody.
---

# GPUI Fast in Aphrody

Authoritative reference for GPUI Fast (`longbridge/gpui-fast`) retained-mode mechanics and how `crates/gpui` + `crates/gpui-kit` + `crates/nsh-*` interoperate on `main`:

1. **Retained Subtrees & Splicing**: Every `Entity<V: Render>` / `AnyView` tracks `dependencies` vs `own_dependencies` (entities, globals, `ScrollHandle`/`ListState`, ambient pointer/modifier state, hover hitboxes) to **Reuse**, **Splice** around dirty nested views, or **Build** without altering GPUI's public API.
2. **Retained Layout & Text Carry/Replay**: Persistent Taffy nodes keyed by `GlobalElementId`/sibling path skip dirtying Taffy when style/child fingerprints match; unchanged text carries measurements and changed text replays prior constraint logs.
3. **Scroll Layers & Dynamic Textures**: Opaque scrolling containers/lists composite cached GPU tiles (2-viewport overscan, 64 MiB LRU, 100 ms `hover_freeze`, automatic `Bypass` on gradients/paths/focused inputs/animations); video/map surfaces use `DynamicTexture` / `WgpuDynamicTexture`.
4. **Mandatory View Discipline**: Keep `render()`/`prepaint()`/`paint()` free of per-frame global writes or gratuitous `cx.notify()`, update state via `entity.update(cx, ...)`, scroll via `ScrollHandle`/`ListState`, and use domain-derived `ElementId`s. Under `test-support`, `GPUI_VIEW_RETENTION=0` and `GPUI_SCROLL_LAYERS=0` force from-scratch draws (`window.refresh()` before `window.draw(cx)` in tests needing full rebuild).
5. **Compatibility Seam**: `gpui-base`, `gpui-component`, and `gpui-kit` enable `default = ["gpui-fast"]` to align cleanly with `crates/gpui`'s blanket `impl<T: Element> IntoElement for T`, `Path<T = Pixels>`, and 1-argument `Window::paint_layer(|window| ...)`.

Consult `~/.gemini/brain/gpui/GPUI_FAST.md` and `~/.gemini/brain/gpui/UNIFICATION_MATRIX.md` (or `bun ~/.gemini/scripts/learn.ts search "<topic>"`) for module-by-module internals.
