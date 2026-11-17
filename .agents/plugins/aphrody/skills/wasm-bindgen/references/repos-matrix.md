# wasm-bindgen Organization 8-Repository Matrix

> Live snapshot generated `2026-11-16T22:51:51.396Z` from `gh api --paginate orgs/wasm-bindgen/repos`, GitHub Git Trees, `crates.io`, and `https://gitingest.com/api/ingest`.

## 1. All 8 Org Repositories

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

## 2. Sub-Crate & Manifest Inventory

### `wasm-bindgen/wasm-bindgen`

- **Latest Commit**: `ce77fe15277cd4ae050e8027247a9bd76fb0400a` (2026-11-12T01:40:16Z) — *String returns for small strings, JSON::stringify and Intl; Date::new_with_value; ArrayBuffer bytes (#5446)*
- **Artifacts / Crates**: `wasm-bindgen-cli-support@0.2.130` (`crates/cli-support/Cargo.toml`, MSRV 1.86), `wasm-bindgen-cli@0.2.130` (`crates/cli/Cargo.toml`, MSRV 1.86), `wasm-bindgen-futures@0.4.80` (`crates/futures/Cargo.toml`, MSRV 1.81), `js-sys@0.3.107` (`crates/js-sys/Cargo.toml`, MSRV 1.81), `wasm-bindgen-macro-support@0.2.130` (`crates/macro-support/Cargo.toml`, MSRV 1.81), `wasm-bindgen-macro@0.2.130` (`crates/macro/Cargo.toml`, MSRV 1.81), `wasm-bindgen-shared@0.2.130` (`crates/shared/Cargo.toml`, MSRV 1.81), `wasm-bindgen-test-macro@0.3.80` (`crates/test-macro/Cargo.toml`, MSRV 1.81), `wasm-bindgen-test-shared@0.2.130` (`crates/test-shared/Cargo.toml`, MSRV 1.81), `wasm-bindgen-test@0.3.80` (`crates/test/Cargo.toml`, MSRV 1.81), `typescript-tests@0.0.0` (`crates/typescript-tests/Cargo.toml`), `web-sys@0.3.107` (`crates/web-sys/Cargo.toml`, MSRV 1.81), `webidl-tests@0.0.0` (`crates/webidl-tests/Cargo.toml`), `wasm-bindgen-webidl@0.0.0` (`crates/webidl/Cargo.toml`)
- **Key Upstream Files Indexed**: `README.md`, `Cargo.toml`, `src/lib.rs`, `crates/cli-support/Cargo.toml`, `crates/cli/Cargo.toml`, `crates/futures/Cargo.toml`, `crates/js-sys/Cargo.toml`, `crates/macro-support/Cargo.toml`, `crates/macro/Cargo.toml`, `crates/shared/Cargo.toml`
- **GitIngest Status**: Local digest: `target/gitingest/wasm-bindgen/wasm-bindgen.txt` (`20.72 MB`, `529,299` lines, SHA-256 `269113dd1b33...`, method `gitingest.com-digest-url`)

### `wasm-bindgen/wasm-pack`

- **Latest Commit**: `c262c3307d95c842f40221df8e2dc119b4136f7b` (2026-11-11T05:15:08Z) — *Install the native Apple Silicon build in init.sh (#1623)*
- **Artifacts / Crates**: `wasm-pack@0.15.1` (edition 2021)
- **Key Upstream Files Indexed**: `README.md`, `Cargo.toml`, `src/lib.rs`, `src/main.rs`
- **GitIngest Status**: Local digest: `target/gitingest/wasm-bindgen/wasm-pack.txt` (`574.1 KB`, `16,371` lines, SHA-256 `ae662535cdae...`, method `gitingest.com-digest-url`)

### `wasm-bindgen/walrus`

- **Latest Commit**: `58d6538468f29e4efdb6fb0f19ec62853cd7a602` (2026-11-16T18:53:49Z) — *fix(dwarf): derive DWARF from where each instruction was emitted (#340)*
- **Artifacts / Crates**: `walrus-fuzz-utils@0.1.0` (`crates/fuzz-utils/Cargo.toml`), `walrus-macro@0.26.0` (`crates/macro/Cargo.toml`), `walrus-tests-utils@0.1.0` (`crates/tests-utils/Cargo.toml`), `walrus-tests@0.1.0` (`crates/tests/Cargo.toml`)
- **Key Upstream Files Indexed**: `README.md`, `Cargo.toml`, `src/lib.rs`, `crates/fuzz-utils/Cargo.toml`, `crates/macro/Cargo.toml`, `crates/tests-utils/Cargo.toml`, `crates/tests/Cargo.toml`, `src/passes/gc.rs`, `src/passes/mod.rs`, `src/passes/used.rs`
- **GitIngest Status**: Local digest: `target/gitingest/wasm-bindgen/walrus.txt` (`1.14 MB`, `36,251` lines, SHA-256 `6e83dd8a32ec...`, method `gitingest.com-digest-url`)

### `wasm-bindgen/weedle`

- **Latest Commit**: `dfb9fef65352d1232f48a6f65986ac823140b268` (2026-10-29T23:22:46Z) — *Require wasm-bindgen-bot's author-ready label for auto-merge (#82)*
- **Artifacts / Crates**: `weedle@0.13.1` (edition 2018)
- **Key Upstream Files Indexed**: `README.md`, `Cargo.toml`, `src/lib.rs`, `.github/workflows/bot.yml`, `.github/workflows/rust.yml`, `.travis.yml`, `AGENTS.md`, `LICENSE.md`, `src/argument.rs`, `src/attribute.rs`
- **GitIngest Status**: Local digest: `target/gitingest/wasm-bindgen/weedle.txt` (`254.4 KB`, `8,564` lines, SHA-256 `32649714e28f...`, method `gitingest.com-digest-url`)

### `wasm-bindgen/ts-gen`

- **Latest Commit**: `b6086739b7d398e17a5b1d395651bfd71ae1fcf1` (2026-11-07T18:36:18Z) — *Reads are Option<T>; --strict-nullable keeps the exact carriers (#202)*
- **Artifacts / Crates**: `ts-gen@0.1.0` (edition 2021)
- **Key Upstream Files Indexed**: `README.md`, `Cargo.toml`, `src/lib.rs`, `src/main.rs`
- **GitIngest Status**: Local digest: `target/gitingest/wasm-bindgen/ts-gen.txt` (`9.37 MB`, `246,574` lines, SHA-256 `f1f0598580bf...`, method `gitingest.com-digest-url`)

### `wasm-bindgen/js-bindgen`

- **Latest Commit**: `679a992a07e8803bf77d7f45512b13ede2cb9811` (2026-07-15T06:01:14Z) — *Add `bun` runner (#67)*
- **Artifacts / Crates**: `placeholder@workspace` (`client/Cargo.toml`), `js-bindgen@workspace` (`client/js-bindgen/Cargo.toml`), `js-sys@workspace` (`client/js-sys/Cargo.toml`), `js-bindgen-test@workspace` (`client/test/Cargo.toml`, MSRV 1.87), `web-sys@workspace` (`client/web-sys/Cargo.toml`)
- **Key Upstream Files Indexed**: `README.md`, `client/Cargo.toml`, `client/js-bindgen/Cargo.toml`, `client/js-bindgen/src/lib.rs`
- **GitIngest Status**: Local digest: `target/gitingest/wasm-bindgen/js-bindgen.txt` (`710.3 KB`, `26,988` lines, SHA-256 `0db22c9217ae...`, method `gitingest.com-digest-url`)

### `wasm-bindgen/rust-externref-rfc`

- **Latest Commit**: `9950a98f80892d7d5b03f584ae6e32584e06bb97` (2026-10-13T15:35:12Z) — *Edits*
- **Artifacts / Crates**: Spec / action repository (no Cargo or npm manifest)
- **Key Upstream Files Indexed**: `README.md`, `externref-proposal.md`
- **GitIngest Status**: Local digest: `target/gitingest/wasm-bindgen/rust-externref-rfc.txt` (`23.4 KB`, `625` lines, SHA-256 `a23e372b3bcd...`, method `gitingest.com-digest-url`)

### `wasm-bindgen/wasm-bindgen-bot`

- **Latest Commit**: `9608f75e0cb8ae686c2bbb6556869a857bf95e64` (2026-11-13T19:40:24Z) — *Give the model the whole diff with an index of its files*
- **Artifacts / Crates**: Spec / action repository (no Cargo or npm manifest)
- **Key Upstream Files Indexed**: `README.md`, `analyze/action.yml`, `post/action.yml`, `scripts/context.mjs`, `scripts/post.mjs`, `scripts/run.mjs`
- **GitIngest Status**: Local digest: `target/gitingest/wasm-bindgen/wasm-bindgen-bot.txt` (`175.2 KB`, `4,177` lines, SHA-256 `9c2395c1cc72...`, method `gitingest.com-digest-url`)

## 3. `crates.io` vs Aphrody Workspace Pin Parity

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

### Workspace & CLI Alignment Notes

- **Pinned CLI vs Lockfile**: `tooling/bun/web-build.ts` checks `wasm-bindgen --version` (0.2.129) against `Cargo.lock` (0.2.129). Upstream `0.2.130` added small-string return intrinsics and `ArrayBuffer`/Intl helpers; upgrading requires bumping both `Cargo.lock` (`cargo update -p wasm-bindgen --precise 0.2.130`) and installing `wasm-bindgen-cli@0.2.130` atomically.
- **`wasm_thread` Fork Pin**: Root `Cargo.toml` patches `wasm_thread` to `https://github.com/zed-industries/wasm_thread` rev `0cf96c7708df` (`es_modules` feature) because crates.io `wasm_thread` passes deprecated positional arguments removed/deprecated after `wasm-bindgen 0.2.93`.
- **Binaryen**: Aphrody uses `v133` directly rather than `wasm-pack`'s bundled installer.
