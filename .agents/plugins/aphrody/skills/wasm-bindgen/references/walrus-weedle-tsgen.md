# walrus, weedle, ts-gen, js-bindgen & Externref RFC

> Compiler, IR transformation, WebIDL/TypeScript codegen, and next-gen ABI architecture synthesized from the 8 `github.com/wasm-bindgen` repositories.

## 1. `walrus` (`0.28.0`) Transformation Passes Before `wasm-opt`

`wasm-bindgen-cli` embeds `wasm-bindgen-cli-support`, which parses the raw `rustc`/LLD `wasm32-unknown-unknown` artifact into a `walrus::Module` IR before Binaryen `wasm-opt` ever touches the file:

1. **`__wasm_bindgen_unstable` Custom Section Decoding**: `crates/cli-support/src/decode.rs` extracts compiler-emitted ABI descriptors describing every `#[wasm_bindgen]` import/export signature.
2. **WIT Adapter & Shim Lowering**: `crates/cli-support/src/wit/` builds incoming/outgoing JS adapter bindings and emits the companion ESM glue (`.js`) and optional TypeScript declarations (`.d.ts`).
3. **`externref` Table Transformation (`transforms/externref/mod.rs`)**: Replaces placeholder `externref` handle intrinsics with direct Wasm `table.get` / `table.set` operations on the blessed `__wbindgen_externref_table` (with `INITIAL_HEAP_OFFSET = 1024` reserving stack slots below `1024` and pre-populating `["undefined", "null", "true", "false"]`).
4. **ABI / Runtime Transforms**:
   - `transforms/multi_value`: lowers multi-return functions natively when `--target` supports Wasm multi-value.
   - `transforms/threads`: instruments `__wbindgen_thread_destroy` and TLS init for `+atomics,+bulk-memory` (`wasm32` and `wasm64`).
   - `transforms/jspi`: wraps marked async exports/imports for Wasm JS Promise Integration (JSPI).
   - `transforms/catch_handler` + `export_sp_restore`: translates legacy/modern Wasm exception handling (`Try` / `TryTable`) and restores the shadow stack pointer across trapped/caught calls.
5. **`walrus::passes::gc` & `walrus::passes::used`**:
   - **Dead-Symbol & Custom-Section Elimination**: Marks reachable functions, tables, globals, memories, and elements from surviving exports/start functions, deleting unreferenced `__wbindgen_describe_*` stubs and dead helper functions.
   - `Bindgen` options (`remove_name_section`, `remove_producers_section`, `split_debug_info`, `js_string_builtins` via `wasm:js/string-constants`) prune non-runtime metadata cleanly while `walrus` preserves DWARF via `gimli 0.34.0` when `--keep-debug` is enabled.
6. **Hand-off to Binaryen `wasm-opt` (`v133`)**:
   - `walrus` runs structural ABI lowering and dead-descriptor GC *first*; `wasm-opt -Oz` then performs peephole optimization, CFG simplification, dead-code elimination, inlining, and local coalescing on the finalized valid Wasm module (with matching feature flags `--enable-bulk-memory --enable-reference-types --enable-multivalue ...`). Running `wasm-opt` *before* `wasm-bindgen` breaks custom `__wasm_bindgen_unstable` descriptors and placeholder intrinsics!

## 2. `weedle` (`0.13.1`) & WebIDL -> `web-sys` Codegen

- `weedle` is a `nom 5.1.0` WebIDL grammar parser producing typed ASTs (`weedle::Definitions`, `Interface`, `Dictionary`, `Enum`, `Namespace`, `ExtendedAttributeList`).
- Upstream `crates/webidl` (`wasm-bindgen-webidl`) feeds all curated WHATWG/W3C `.webidl` files through `weedle::parse()`, builds a `FirstPassRecord` cross-interface inheritance & mixin table, and generates `crates/web-sys/src/features/` + `Cargo.toml` feature gates via `quote` + `syn`.
- When an unstable WebIDL overrides a stable attribute with a different type, `wasm-bindgen-webidl` emits dual `#[cfg(not(web_sys_unstable_apis))]` / `#[cfg(web_sys_unstable_apis)]` definitions so downstream crates opt in explicitly.

## 3. `ts-gen` & `js-bindgen`: TypeScript Contracts & Next-Gen Linker Interop

- **`ts-gen` (`wasm-bindgen/ts-gen`)**: Parses TypeScript `.d.ts` declarations using **`oxc_parser 0.154`** + `oxc_resolver`, populates a two-phase `TypeRegistry` (merging `var` + `interface` declarations and resolving inheritance chains), and prints idiomatic `#[wasm_bindgen]` Rust extern blocks via `syn 2` + `prettyplease`. Useful in Aphrody whenever wrapping Cloudflare Worker/npm TypeScript SDKs (`examples/cloudflare-worker`, `--strict-nullable` for exact `Option<T>` carriers) instead of hand-writing error-prone `extern "C"` blocks.
- **`js-bindgen` (`wasm-bindgen/js-bindgen`)**: Next-generation linker-driven `#![no_std]` research workspace (`embed_js`, `import_js`, `unsafe_global_wat`, custom `js-bindgen-ld`) testing real-generic `js-sys`, `wasm64-unknown-unknown`, and `bun` test runner integration without legacy placeholder descriptor tables.

## 4. `rust-externref-rfc`: Future Zero-Wrapper `Externref` ABI

- Upstream RFC 3987 proposed an unsized lang-type `__externref_t`, which conflicted with Rust's operational semantics requiring all values to be representable as bytes with provenance.
- `wasm-bindgen/rust-externref-rfc` reframes the solution around **boxed `Externref` pointers** (`c_externref_ptr` in `ptr addrspace(2)` matching the LLVM RFC): owned `Externref`, borrowed `ExternrefRef<'a>`, ABI signal `ByValue<T>`, and scoped `with_stack_slot(|tmp| ...)` so LLVM's optimizer eliminates `__externref_table_alloc`/`__externref_table_free` pairs wherever pointers don't escape.

## 5. `wasm-bindgen-bot` Org Automation

- Two-job GitHub Actions architecture (`start` -> `analyze` -> `post`) running Claude Code in `--bare` sandboxed mode via Cloudflare AI Gateway for automated PR review (`merge-ready` label), issue triage, and reproduction across all 8 `wasm-bindgen` repositories.
