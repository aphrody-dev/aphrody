# js-sys, web-sys & wasm-bindgen-futures Patterns

> Verified against upstream `wasm-bindgen` (0.2.130), `crates/gpui_web`, and `crates/gpui_wgpu`.

## 1. `Closure` Lifetime Hygiene & Cleanup

JavaScript garbage collectors cannot invoke Rust `Drop` automatically when an event listener goes unused; conversely, calling `Closure::forget()` on dynamic DOM listeners leaks heap table slots and Wasm closure allocations permanently.

| Pattern | When to Use | Aphrody Implementation |
| :--- | :--- | :--- |
| **RAII `EventListenerHandle`** | DOM `addEventListener` (`pointer*`, `key*`, `paste`, `visibilitychange`) | [`crates/gpui_web/src/events.rs`](../../../../../../crates/gpui_web/src/events.rs) stores `target`, `event_name`, and `Closure<dyn FnMut(JsValue)>`; `impl Drop` calls `remove_event_listener_with_callback` before dropping the closure |
| **Struct-Owned `Closure` + `Weak`** | Long-lived frame loops (`requestAnimationFrame`) & `ResizeObserver` | [`crates/gpui_web/src/window.rs`](../../../../../../crates/gpui_web/src/window.rs) holds `_raf_closure`, `_resize_observer_closure`, `_safe_area_observer_closure` with `Rc::downgrade(&inner)` to break reference cycles |
| **`Closure::once_into_js`** | Guaranteed single-fire browser APIs (`setTimeout`, `queueMicrotask`, `requestIdleCallback`) | [`crates/gpui_web/src/dispatcher.rs`](../../../../../../crates/gpui_web/src/dispatcher.rs) & [`ime_mirror.rs`](../../../../../../crates/gpui_web/src/ime_mirror.rs) auto-drop Rust closure state after the single JS call |
| **Non-Passive Event Listeners** | Wheel / touch events requiring `preventDefault()` | `EventListenerHandle::add_non_passive` constructs `{ passive: false }` via `js_sys::Reflect::set` and binds `addEventListener.call3` |

```rust
// Correct Aphrody pattern (crates/gpui_web/src/events.rs):
impl Drop for EventListenerHandle {
    fn drop(&mut self) {
        self.target
            .remove_event_listener_with_callback(
                self.event_name,
                self.closure.as_ref().unchecked_ref(),
            )
            .ok();
    }
}
```

## 2. Zero-Copy `&[u8]` / `Uint8Array::view` vs Safe Copies

Upstream `crates/js-sys/src/lib.rs` marks `Uint8Array::view(rust: &[u8])` `unsafe` for a critical reason: Wasm linear memory grows via `memory.grow`, which **detaches** the existing `ArrayBuffer` in JS!

- **Use Safe Copy (`Uint8Array::from` / `.to_vec()`) Across Async or Re-entrant Boundaries**:
  - [`crates/gpui_web/src/http_client.rs`](../../../../../../crates/gpui_web/src/http_client.rs) copies outgoing request bytes via `js_sys::Uint8Array::from(bytes.as_slice())` before handing them to `RequestInit::set_body` and incoming stream chunks via `.dyn_into::<js_sys::Uint8Array>().map(|b| b.to_vec())`.
  - [`crates/gpui_web/src/platform.rs`](../../../../../../crates/gpui_web/src/platform.rs) reads clipboard blobs with `js_sys::Uint8Array::new(&buffer).to_vec()`.
- **When `Uint8Array::view` Is Safe**:
  - Only for **pure synchronous leaf calls** where JS consumes/copies the typed array immediately before returning and **zero Rust allocations or JS->Rust callbacks occur** while the view lives (e.g., synchronous `WebGL2RenderingContext.texSubImage2D` or synchronous `TextDecoder.decode(view)`).
  - For `Atomics` shared-memory signaling where a fixed `AtomicI32` address stays valid on shared `WebAssembly.Memory`, [`dispatcher.rs`](../../../../../../crates/gpui_web/src/dispatcher.rs) slices directly via `js_sys::Int32Array::new_with_byte_offset_and_length(&memory.buffer(), byte_offset, 1)`.

## 3. `wasm-bindgen-futures`, Microtasks & Worker Pumping

- **Promise -> Future**: `wasm_bindgen_futures::JsFuture::from(promise).await` schedules continuation on the microtask queue without blocking the browser event loop.
- **Fire-and-Forget UI Tasks**: `wasm_bindgen_futures::spawn_local(async move { ... })` drives top-level async startup (`initialize_graphics` in `platform.rs`) and streaming body readers (`pump_response_body` in `http_client.rs`).
- **Avoid Main-Thread Mutex Deadlocks**: `http_client.rs` explicitly uses `async_channel::bounded(8)` instead of `futures::mpsc` because `futures::mpsc`'s internal lock can park the browser main thread when a worker thread pulls body bytes.
- **`Atomics.waitAsync` Main-Thread Mailbox**: `dispatcher.rs` uses non-blocking `js_sys::Atomics::wait_async` -> `JsFuture` in `spawn_local` because browser main threads throw `TypeError` on synchronous `Atomics.wait`.
- **Feature Flags**: Upstream `wasm-bindgen-futures 0.4.80` supports `futures-core-03-stream` (forwarding `js-sys/futures-core-03-stream` for `JsStream` / `AsyncIterator`) and optional Emscripten Tokio integration under `--cfg wasm_bindgen_unstable_tokio`.

## 4. `web-sys` Feature Minimization & Dynamic Feature Probing

- **Strict Minimal Features**: `web-sys` contains over 3,000 WebIDL types; enabling `"all"` explodes compile times and trips Windows `CreateProcess` argv limits. Aphrody enables **52 exact features** in [`crates/gpui_web/Cargo.toml`](../../../../../../crates/gpui_web/Cargo.toml) and **1 feature (`HtmlCanvasElement`)** in [`crates/gpui_wgpu/Cargo.toml`](../../../../../../crates/gpui_wgpu/Cargo.toml).
- **Unstable WebIDL APIs**: Features gated by `#[cfg(web_sys_unstable_apis)]` in `web-sys` (such as newer WebGPU / WebHID / EyeDropper drafts) require `RUSTFLAGS="--cfg=web_sys_unstable_apis"`; keep `check-cfg` clean and prefer `js_sys::Reflect` probing where browser support varies.
- **Probe Optional APIs Before Dispatching**:
  - Calling a `web-sys` method when the JS property is `undefined` (like `navigator.clipboard` on HTTP non-secure contexts, or `requestIdleCallback` on Safari, or `PointerEvent.getPredictedEvents` on older browsers) causes a `wasm-bindgen` runtime abort!
  - Always probe via `js_sys::Reflect::has` / `Reflect::get` first (verified in `platform.rs:625`, `dispatcher.rs:401`, `events.rs:1426`).
- **Dictionaries vs Interfaces**: WebIDL dictionaries (such as `web_sys::ReadableStreamReadResult`) have no JS constructor prototype for `instanceof`; cast with `.unchecked_into()` rather than `.dyn_into()` (`http_client.rs:292`), and use `.dyn_into::<T>()` on real DOM interfaces (`HtmlCanvasElement`, `Response`, `Uint8Array`).
