# wasm-bindgen Ecosystem & Aphrody WebAssembly Best Practices Guide

> Live architectural audit across all **8 repositories** in `github.com/wasm-bindgen`, `crates.io` upstream releases, GitIngest digests, and Aphrody's pinned `wasm-bindgen 0.2.129` + `v133` stack (`crates/gpui_web`, `crates/aphrody_desktop`, `tooling/bun/web-build.ts`).

---

## 1. Live Ingestion Telemetry & Workspace Summary

<!-- BEGIN WASM_BINDGEN_TELEMETRY -->
| Metric | Verified Live Value | Source |
| :--- | :--- | :--- |
| **`wasm-bindgen` Org Repositories** | `8` repositories audited & ingested (100% coverage) | `gh api --paginate orgs/wasm-bindgen/repos` |
| **Aphrody Pinned `wasm-bindgen`** | `Cargo.lock` `0.2.129` == CLI `0.2.129` (constraint `0.2.120`) | [`Cargo.lock`](../../Cargo.lock) + [`web-build.ts`](../../tooling/bun/web-build.ts) |
| **Binaryen Optimizer** | `v133` (`-Oz --enable-bulk-memory --enable-reference-types`) | Local PATH + [`web-build.ts`](../../tooling/bun/web-build.ts) |
| **`gpui_web` Minimized `web-sys`** | `52` explicit features (`crates/gpui_wgpu` uses `1`) | [`crates/gpui_web/Cargo.toml`](../../crates/gpui_web/Cargo.toml) |
| **GitIngest Endpoint & CLI** | `https://gitingest.com/api/ingest` reachable=`true` (HTTP 200); CLI `v0.3.1` | Live HTTP probe + `gitingest` |
| **Persisted Raw Digests** | `8` files under `target/gitingest/wasm-bindgen/` (`32.93 MB`, `5,528` extracted file sections) | [`wasm-bindgen-ingest.ts`](../../tooling/bun/wasm-bindgen-ingest.ts) |
<!-- END WASM_BINDGEN_TELEMETRY -->

---

## 2. All 8 `github.com/wasm-bindgen` Upstream Repositories

<!-- BEGIN WASM_BINDGEN_REPOS_TABLE -->
| Repository | Branch & HEAD SHA | Stars | Language | Blob Files | Digest Method & Size | Primary Ecosystem Role |
| :--- | :--- | ---: | :--- | ---: | :--- | :--- |
| [wasm-bindgen/wasm-bindgen](https://github.com/wasm-bindgen/wasm-bindgen) | `main` @ `ce77fe15` (2026-11-12) | 9,231 | Rust | 4,346 | `gitingest.com-digest-url` (20.72 MB) | Core Rust <-> JS ABI generator, macro crate, CLI (`wasm-bindgen-cli`), `js-sys`, `web-sys`, `wasm-bindgen-futures`, and `wasm-bindgen-webidl`. |
| [wasm-bindgen/wasm-pack](https://github.com/wasm-bindgen/wasm-pack) | `master` @ `c262c330` (2026-11-11) | 7,309 | Rust | 157 | `gitingest.com-digest-url` (574.1 KB) | Legacy Rust -> Wasm npm/webpack workflow orchestrator (`0.15.1`); superseded in Aphrody by `tooling/bun/web-build.ts`. |
| [wasm-bindgen/walrus](https://github.com/wasm-bindgen/walrus) | `main` @ `58d65384` (2026-11-16) | 533 | Rust | 241 | `gitingest.com-digest-url` (1.14 MB) | WebAssembly IR transformation & dead-symbol/DWARF/GC library powering `wasm-bindgen-cli-support` optimization passes before Binaryen `wasm-opt`. |
| [wasm-bindgen/weedle](https://github.com/wasm-bindgen/weedle) | `master` @ `dfb9fef6` (2026-10-29) | 54 | WebIDL | 28 | `gitingest.com-digest-url` (254.4 KB) | Nom-based WebIDL parser turning WHATWG/W3C `.webidl` specifications into ASTs consumed by `wasm-bindgen-webidl` for `web-sys`. |
| [wasm-bindgen/ts-gen](https://github.com/wasm-bindgen/ts-gen) | `main` @ `b6086739` (2026-11-07) | 21 | Rust | 558 | `gitingest.com-digest-url` (9.37 MB) | oxc_parser + syn TypeScript `.d.ts` -> Rust `#[wasm_bindgen]` extern generator for zero-drift JS/TS SDK contracts. |
| [wasm-bindgen/js-bindgen](https://github.com/wasm-bindgen/js-bindgen) | `main` @ `679a992a` (2026-07-15) | 15 | Rust | 277 | `gitingest.com-digest-url` (710.3 KB) | Next-generation linker-driven `#![no_std]` JS FFI (`embed_js`/`import_js`/`js-bindgen-ld`) with `wasm32` + `wasm64` and real-generic `js-sys` R&D. |
| [wasm-bindgen/rust-externref-rfc](https://github.com/wasm-bindgen/rust-externref-rfc) | `main` @ `9950a98f` (2026-10-13) | 0 | Markdown/Spec | 2 | `gitingest.com-digest-url` (23.4 KB) | Draft Rust language/LLVM proposal for boxed `Externref`/`ExternrefRef<'a>`/`ByValue<T>` (`ptr addrspace(2)`) removing table marshalling overhead. |
| [wasm-bindgen/wasm-bindgen-bot](https://github.com/wasm-bindgen/wasm-bindgen-bot) | `main` @ `9608f75e` (2026-11-13) | 0 | JavaScript | 38 | `gitingest.com-digest-url` (175.2 KB) | Cloudflare AI Gateway + sandboxed GitHub Actions review/triage automation maintaining all 8 `wasm-bindgen` org repositories. |
<!-- END WASM_BINDGEN_REPOS_TABLE -->

### Repository Architecture Breakdown

- **`wasm-bindgen/wasm-bindgen`**: Monorepo hosting `wasm-bindgen`, `wasm-bindgen-macro`, `wasm-bindgen-macro-support`, `wasm-bindgen-shared`, `wasm-bindgen-futures`, `js-sys`, `web-sys`, `wasm-bindgen-webidl`, `wasm-bindgen-cli-support`, and `wasm-bindgen-cli`.
- **`wasm-bindgen/walrus`**: Wasm IR transformation library (`wasmparser` + `wasm-encoder` + `gimli` DWARF + Wasm GC) powering `wasm-bindgen-cli-support`'s dead-symbol GC, `externref` table lowering, JSPI, multi-value, and threads passes *before* `wasm-opt`.
- **`wasm-bindgen/weedle`**: `nom`-based WebIDL parser turning W3C/WHATWG specs into ASTs for `wasm-bindgen-webidl`, which code-generates `crates/web-sys` with `#[cfg(web_sys_unstable_apis)]` dual-override guards.
- **`wasm-bindgen/ts-gen`**: `oxc_parser 0.154` + `syn 2` + `prettyplease` compiler turning TypeScript `.d.ts` declarations into `#[wasm_bindgen]` Rust bindings (supporting `--strict-nullable` and Cloudflare Worker TypeScript contracts).
- **`wasm-bindgen/js-bindgen`**: Next-gen `#![no_std]` linker-driven JS interop (`js-bindgen-ld`, `embed_js`, `import_js`, `unsafe_global_wat`) exploring real-generic `js-sys`, `wasm64-unknown-unknown`, and `bun` testing.
- **`wasm-bindgen/rust-externref-rfc`**: Draft proposal for boxed `Externref` / `ExternrefRef<'a>` / `ByValue<T>` / `with_stack_slot` over `ptr addrspace(2)`, solving RFC 3987's operational-semantics objection while letting LLVM fold away `__externref_table_alloc`/`free` pairs.
- **`wasm-bindgen/wasm-pack`**: Legacy npm/webpack packager (`0.15.1`); deliberately omitted from Aphrody in favor of [`tooling/bun/web-build.ts`](../../tooling/bun/web-build.ts).
- **`wasm-bindgen/wasm-bindgen-bot`**: Sandboxed Cloudflare AI Gateway PR reviewer & issue triage bot maintaining the `wasm-bindgen` organization.

---

## 3. `crates.io` Upstream Releases vs Aphrody Workspace Pins

<!-- BEGIN WASM_BINDGEN_CRATES_MATRIX -->
| Crate | `crates.io` Latest | Aphrody `Cargo.toml` | Aphrody `Cargo.lock` | Total Downloads | Updated (UTC) | Status in Aphrody |
| :--- | :--- | :--- | :--- | ---: | :--- | :--- |
| `wasm-bindgen` | `0.2.130` | `0.2.120` | `0.2.129` | 664,674,044 | 2026-10-30 | Core Wasm/JS runtime (`Cargo.lock` == CLI `0.2.129`; 1 patch behind `0.2.130`) |
| `wasm-bindgen-futures` | `0.4.80` | `0.4` | `0.4.79` | 413,056,825 | 2026-10-30 | Async Promise <-> Future bridge in `gpui_web` (`0.4.79` vs `0.4.80`) |
| `js-sys` | `0.3.107` | `0.3` | `0.3.106` | 618,679,099 | 2026-10-30 | ECMAScript built-ins (`Uint8Array`, `Reflect`, `Atomics`, `Promise`) |
| `web-sys` | `0.3.107` | `0.3` | `0.3.106` | 476,510,563 | 2026-10-30 | Minimized WebIDL bindings (`52` features in `gpui_web`, `1` in `gpui_wgpu`) |
| `walrus` | `0.28.0` | `internal to wasm-bindgen-cli` | `not-in-workspace` | 18,218,835 | 2026-11-03 | Powers `wasm-bindgen-cli` IR transforms (`externref`, `gc`, DWARF) pre-`wasm-opt` |
| `weedle` | `0.13.1` | `upstream codegen-only` | `not-in-workspace` | 1,778,072 | 2026-02-26 | WebIDL parser driving upstream `wasm-bindgen-webidl` -> `web-sys` codegen |
| `wasm-pack` | `0.15.1` | `replaced by web-build.ts` | `not-in-workspace` | 6,916,348 | 2026-10-30 | Legacy npm/webpack wrapper superseded by Aphrody `cargo build + wasm-bindgen + wasm-opt` |
<!-- END WASM_BINDGEN_CRATES_MATRIX -->

### Version Pinning & Upgrade Policy

- **Exact Lockstep Rule**: [`tooling/bun/web-build.ts`](../../tooling/bun/web-build.ts) runs `verifyBindgenVersion()` to guarantee CLI `wasm-bindgen` (0.2.129) strictly equals `Cargo.lock` (0.2.129). Never update `Cargo.lock` to `0.2.130` without installing matching `wasm-bindgen-cli@0.2.130`.
- **`wasm_thread` Fork Rationale**: Root [`Cargo.toml`](../../Cargo.toml) pins `wasm_thread` to `zed-industries/wasm_thread` rev `0cf96c7708df` (`features = ["es_modules"]`) because upstream `crates.io` `wasm_thread` invokes deprecated positional `wasm-bindgen` init arguments (deprecated since `0.2.93`).

---

## 4. Deep Best Practices Grounded in `crates/gpui_web` & `web-build.ts`

### 4.1 `ExternRef` & `Closure` Cleanup Discipline

1. **RAII Unregistration Over `.forget()`**: [`crates/gpui_web/src/events.rs`](../../crates/gpui_web/src/events.rs) wraps every DOM event listener in `EventListenerHandle`, whose `Drop` implementation calls `target.remove_event_listener_with_callback(...)` before dropping the `Closure`. Simply dropping a `Closure` without removing the listener triggers *"closure invoked after being dropped"*; calling `.forget()` leaks Wasm heap/table entries on view teardown.
2. **Owner Lifetime + `Weak` Back-References**: [`crates/gpui_web/src/window.rs`](../../crates/gpui_web/src/window.rs) retains `_raf_closure`, `_resize_observer_closure`, and `_safe_area_observer_closure` directly on `WebWindow`, using `Rc::downgrade(&inner)` on observers to eliminate retain cycles.
3. **One-Shot Closures**: [`crates/gpui_web/src/dispatcher.rs`](../../crates/gpui_web/src/dispatcher.rs) and [`ime_mirror.rs`](../../crates/gpui_web/src/ime_mirror.rs) use `Closure::once_into_js` for `setTimeout`, `queueMicrotask`, and `requestIdleCallback`.
4. **Reentrancy Safety Under `panic = "abort"`**: `WebWindowInner::with_callback` moves callbacks out via `.take()` before execution so nested DOM events cannot trigger fatal `RefCell` double-borrow aborts.

### 4.2 Zero-Copy `&[u8]` / `Uint8Array` Views vs Heap-Safe Copies

- **`unsafe { js_sys::Uint8Array::view(slice) }`** exposes Wasm linear memory without copying, but **any allocation (`Vec`, `Box`, `String`, Rust re-entry) can execute `memory.grow` and detach the underlying `ArrayBuffer`**, invalidating the JS view immediately.
- Therefore, `crates/gpui_web` reserves direct memory views strictly for synchronous fixed-memory operations (such as `Int32Array::new_with_byte_offset_and_length` on shared `AtomicI32` mailbox signals in `dispatcher.rs`) and uses owned copies (`Uint8Array::from(bytes.as_slice())` and `Uint8Array::new(&buffer).to_vec()`) across async Fetch streams (`http_client.rs`) and Clipboard Blob reads (`platform.rs`).

### 4.3 `wasm-bindgen-futures`, Microtasks & Worker Concurrency

- `wasm_bindgen_futures::JsFuture::from(promise).await` bridges JS Promises to Rust `Future`s on the microtask queue.
- `http_client.rs` pumps `ReadableStreamDefaultReader::read()` inside `wasm_bindgen_futures::spawn_local` over `async_channel::bounded(8)` rather than `futures::mpsc` (whose internal mutex can park the browser main thread when workers pull body chunks).
- `dispatcher.rs` pairs `SharedArrayBuffer` + `Atomics.waitAsync` -> `JsFuture` on the main thread because browsers disallow synchronous `Atomics.wait` on the UI thread.

### 4.4 `web-sys` Feature Minimization & Dynamic Feature Probing

- `crates/gpui_web/Cargo.toml` enables only **52 exact WebIDL features**; `crates/gpui_wgpu/Cargo.toml` enables only **1 (`HtmlCanvasElement`)**.
- Dynamic APIs (`navigator.clipboard`, `requestIdleCallback`, `PointerEvent.getPredictedEvents`, `Atomics.waitAsync`) are probed via `js_sys::Reflect::has` / `Reflect::get` before invoking `web-sys` bindings to prevent `wasm-bindgen` aborts in non-secure contexts or Safari.
- WebIDL dictionaries lacking runtime class prototypes (`ReadableStreamReadResult`) use `.unchecked_into()`; DOM interfaces use checked `.dyn_into()`.

### 4.5 `walrus` + `wasm-opt` Optimization & Why Aphrody Rejects `wasm-pack`

1. **Compiler & Optimizer Pipeline**: `rustc --profile web` (`opt-level = "z"`, `lto = "thin"`, `codegen-units = 1`, `strip = true`, `panic = "abort"`) -> `wasm-bindgen` CLI (`walrus` `externref` lowering, descriptor decoding, dead-stub `passes::gc`) -> Binaryen `v133` (`-Oz --enable-bulk-memory --enable-bulk-memory-opt --enable-nontrapping-float-to-int --enable-sign-ext --enable-reference-types --enable-multivalue --strip-debug`).
2. **Why Not `wasm-pack`?** Legacy `wasm-pack 0.15.1` enforces single-target `npm`/`webpack` outputs, hits Windows `sccache` `CreateProcess` 32 KB argv overflow on `web-sys`, cannot build Aphrody's 3 runtime-selected cohorts (`baseline`, `simd-baseline`, opt-in COI `multithreaded`), and skips native Wasm section validation + `--brotli-budget` checks implemented in [`tooling/bun/web-build.ts`](../../tooling/bun/web-build.ts).

---

## 5. Linked Artifacts & Progressive-Disclosure Skill

- **`llms.txt` Index**: [`docs/architecture/wasm-bindgen-llms.txt`](./wasm-bindgen-llms.txt)
- **Agent Skill**: [`.agents/plugins/aphrody/skills/wasm-bindgen/SKILL.md`](../../.agents/plugins/aphrody/skills/wasm-bindgen/SKILL.md)
- **Skill References**:
  - [`repos-matrix.md`](../../.agents/plugins/aphrody/skills/wasm-bindgen/references/repos-matrix.md)
  - [`js-web-sys-patterns.md`](../../.agents/plugins/aphrody/skills/wasm-bindgen/references/js-web-sys-patterns.md)
  - [`walrus-weedle-tsgen.md`](../../.agents/plugins/aphrody/skills/wasm-bindgen/references/walrus-weedle-tsgen.md)
  - [`aphrody-best-practices.md`](../../.agents/plugins/aphrody/skills/wasm-bindgen/references/aphrody-best-practices.md)
- **Automation CLI & Test Suite**: [`tooling/bun/wasm-bindgen-ingest.ts`](../../tooling/bun/wasm-bindgen-ingest.ts), [`tooling/bun/wasm-bindgen-ingest.test.ts`](../../tooling/bun/wasm-bindgen-ingest.test.ts)
- **Gitignored Digest Cache**: `target/gitingest/wasm-bindgen/*.txt` + `target/gitingest/wasm-bindgen/ingest-manifest.json`
