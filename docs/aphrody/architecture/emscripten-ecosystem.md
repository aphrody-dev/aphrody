# Emscripten Ecosystem, Live Org Ingestion & Aphrody Wasm Architecture

> Live-verified synthesis of `https://emscripten.org/` (**Emscripten 7.0.1-git**), all **12** repositories in `github.com/orgs/emscripten-core`, and integration patterns across Aphrody's `wasm32-unknown-unknown`, `wasm32-wasip2`, and `wasm32-unknown-emscripten` pipelines.

---

## 1. Live Host Probe & Crawl/Ingest Telemetry

| Probe / Pipeline Dimension | Observed Value | Artifact / Verification Source |
| :--- | :--- | :--- |
| **Host `emcc` / `emsdk` (`probeEmcc()`)** | **Absent on host PC** (`emcc`, `em++`, `emsdk` not on `PATH`; `EMSDK`/`EM_CONFIG`/`EMSCRIPTEN` unset) | Live `Bun.which` probe on `win32/x64` |
| **Host Native LLVM & Wasm Tools** | `clang`: `clang version 23.1.5 (https://github.com/llvm/llvm-project 392dfff97d29b59d77e5cac6d4b6ef2585fe7117)`; `lld`: `LLD 23.1.5 (https://github.com/llvm/llvm-project 392dfff97d29b59d77e5cac6d4b6ef2585fe7117)`; `wasm-opt`: `wasm-opt version 133 (version_133)`; `wasm-bindgen`: `wasm-bindgen 0.2.129` | Cross-checked against Emscripten 7.0.1-git `EXPECTED_LLVM_VERSION = 24` |
| **CLI Transport & GitHub Tooling** | `xh`: `xh 0.26.3`; `gh`: `gh version 2.103.3 (2026-10-26)`; `gitingest`: `present` | [`tooling/bun/emscripten-crawl.ts`](../../../tooling/bun/emscripten-crawl.ts) |
| **`emscripten.org` Sitemap Probe** | `/sitemap.xml` returned `HTTP 404`; `/searchindex.js` returned `HTTP 200` (**90** indexed Sphinx docs) | `target/emscripten-crawl/sitemap.json`, `target/emscripten-crawl/sitemap.xml` |
| **BFS + SearchIndex Crawl** | **92** unique same-origin HTML routes discovered (**80** fetched, **80** HTTP 2xx, `maxPages=80`) | `bun tooling/bun/emscripten-crawl.ts crawl` |
| **`emscripten-core` Org Ingestion** | **12** repositories (**9** active, **3** archived) ingested into `.txt` digests + JSON manifest | `target/gitingest/emscripten-core/*.txt`, `org-manifest.json` |

### `xh` HTTP Transport Verification Sample

| Route | Status | Bytes | Latency | Notes |
| :--- | :--- | :--- | :--- | :--- |
| `/sitemap.xml` | `HTTP 404` | 659 B | 459 ms | GitHub Pages Sphinx returned `HTTP/2.0 404 Not Found`; bypassed via `searchindex.js` + BFS |
| `/` | `HTTP 200` | 9.9 KiB | 360 ms | `xh --check-status` passed |
| `/docs/index.html` | `HTTP 200` | 30.8 KiB | 523 ms | `xh --check-status` passed |
| `/docs/introducing_emscripten/release_notes.html` | `HTTP 200` | 23.4 KiB | 524 ms | `xh --check-status` passed |
| `/docs/tools_reference/emcc.html` | `HTTP 200` | 71.8 KiB | 380 ms | `xh --check-status` passed |
| `/docs/tools_reference/emsdk.html` | `HTTP 200` | 54.4 KiB | 534 ms | `xh --check-status` passed |
| `/docs/tools_reference/settings_reference.html` | `HTTP 200` | 220.6 KiB | 513 ms | `xh --check-status` passed |
| `/docs/compiling/best_practices.html` | `HTTP 200` | 32.8 KiB | 350 ms | `xh --check-status` passed |
| `/docs/compiling/Modularized-Output.html` | `HTTP 200` | 37.3 KiB | 331 ms | `xh --check-status` passed |
| `/docs/porting/multimedia_and_graphics/WebGPU-support.html` | `HTTP 200` | 24.0 KiB | 278 ms | `xh --check-status` passed |
| `/docs/porting/asyncify.html` | `HTTP 200` | 88.0 KiB | 278 ms | `xh --check-status` passed |
| `/docs/porting/pthreads.html` | `HTTP 200` | 43.8 KiB | 279 ms | `xh --check-status` passed |
| `/docs/porting/simd.html` | `HTTP 200` | 132.0 KiB | 262 ms | `xh --check-status` passed |
| `/docs/porting/connecting_cpp_and_javascript/embind.html` | `HTTP 200` | 192.1 KiB | 277 ms | `xh --check-status` passed |
| `/docs/porting/connecting_cpp_and_javascript/Interacting-with-code.html` | `HTTP 200` | 115.3 KiB | 259 ms | `xh --check-status` passed |

### Discovered `emscripten.org` Route Hierarchy

- **Root Routes** (3): `/genindex.html`, `/index.html`, `/search.html`
- **`docs/api_reference/`** (17 routes, 17 crawled): [`advanced-apis.html`](https://emscripten.org/docs/api_reference/advanced-apis.html), [`bind.h.html`](https://emscripten.org/docs/api_reference/bind.h.html), [`console.h.html`](https://emscripten.org/docs/api_reference/console.h.html), [`emscripten.h.html`](https://emscripten.org/docs/api_reference/emscripten.h.html), [`fetch.html`](https://emscripten.org/docs/api_reference/fetch.html)
- **`docs/building_from_source/`** (4 routes, 4 crawled): [`configuring_emscripten_settings.html`](https://emscripten.org/docs/building_from_source/configuring_emscripten_settings.html), [`index.html`](https://emscripten.org/docs/building_from_source/index.html), [`toolchain_what_is_needed.html`](https://emscripten.org/docs/building_from_source/toolchain_what_is_needed.html), [`verify_emscripten_environment.html`](https://emscripten.org/docs/building_from_source/verify_emscripten_environment.html)
- **`docs/compiling/`** (11 routes, 11 crawled): [`best_practices.html`](https://emscripten.org/docs/compiling/best_practices.html), [`Building-Projects.html`](https://emscripten.org/docs/compiling/Building-Projects.html), [`Contrib-Ports.html`](https://emscripten.org/docs/compiling/Contrib-Ports.html), [`CrossOriginStorage.html`](https://emscripten.org/docs/compiling/CrossOriginStorage.html), [`Deploying-Pages.html`](https://emscripten.org/docs/compiling/Deploying-Pages.html)
- **`docs/contributing/`** (4 routes, 4 crawled): [`AUTHORS.html`](https://emscripten.org/docs/contributing/AUTHORS.html), [`contributing.html`](https://emscripten.org/docs/contributing/contributing.html), [`developers_guide.html`](https://emscripten.org/docs/contributing/developers_guide.html), [`index.html`](https://emscripten.org/docs/contributing/index.html)
- **`docs/debugging/`** (1 routes, 1 crawled): [`Sanitizers.html`](https://emscripten.org/docs/debugging/Sanitizers.html)
- **`docs/getting_started/`** (6 routes, 6 crawled): [`bug_reports.html`](https://emscripten.org/docs/getting_started/bug_reports.html), [`downloads.html`](https://emscripten.org/docs/getting_started/downloads.html), [`FAQ.html`](https://emscripten.org/docs/getting_started/FAQ.html), [`index.html`](https://emscripten.org/docs/getting_started/index.html), [`test-suite.html`](https://emscripten.org/docs/getting_started/test-suite.html)
- **`docs/introducing_emscripten/`** (6 routes, 6 crawled): [`about_emscripten.html`](https://emscripten.org/docs/introducing_emscripten/about_emscripten.html), [`community.html`](https://emscripten.org/docs/introducing_emscripten/community.html), [`emscripten_license.html`](https://emscripten.org/docs/introducing_emscripten/emscripten_license.html), [`index.html`](https://emscripten.org/docs/introducing_emscripten/index.html), [`release_notes.html`](https://emscripten.org/docs/introducing_emscripten/release_notes.html)
- **`docs/optimizing/`** (4 routes, 4 crawled): [`Module-Splitting.html`](https://emscripten.org/docs/optimizing/Module-Splitting.html), [`Optimizing-Code.html`](https://emscripten.org/docs/optimizing/Optimizing-Code.html), [`Optimizing-WebGL.html`](https://emscripten.org/docs/optimizing/Optimizing-WebGL.html), [`Profiling-Toolchain.html`](https://emscripten.org/docs/optimizing/Profiling-Toolchain.html)
- **`docs/porting/`** (27 routes, 22 crawled (+4 subsections)): [`asyncify.html`](https://emscripten.org/docs/porting/asyncify.html), [`Audio.html`](https://emscripten.org/docs/porting/Audio.html), [`Debugging.html`](https://emscripten.org/docs/porting/Debugging.html), [`emscripten-runtime-environment.html`](https://emscripten.org/docs/porting/emscripten-runtime-environment.html), [`exceptions.html`](https://emscripten.org/docs/porting/exceptions.html)
- **`docs/site/`** (3 routes, 0 crawled): [`about.html`](https://emscripten.org/docs/site/about.html), [`glossary.html`](https://emscripten.org/docs/site/glossary.html), [`index.html`](https://emscripten.org/docs/site/index.html)
- **`docs/tools_reference/`** (5 routes, 3 crawled): [`emcc.html`](https://emscripten.org/docs/tools_reference/emcc.html), [`emcmdprompt.html`](https://emscripten.org/docs/tools_reference/emcmdprompt.html), [`emsdk.html`](https://emscripten.org/docs/tools_reference/emsdk.html), [`index.html`](https://emscripten.org/docs/tools_reference/index.html), [`settings_reference.html`](https://emscripten.org/docs/tools_reference/settings_reference.html)

---

## 2. All 12 Repositories in `emscripten-core`

| Repository | State | Language | Stars | Head SHA | Ingest Summary | Architectural Role |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| [`emscripten`](https://github.com/emscripten-core/emscripten) | Active | `C++` | 27,780 | `6102fc29` | `gitingest-api` (10000 files, 78.32 MiB) | Core LLVM-to-WebAssembly compiler frontend (`emcc`/`em++`), JS runtime (`library_*.js`), ports (including `emdawnwebgpu`), and test suite. |
| [`emsdk`](https://github.com/emscripten-core/emsdk) | Active | `Python` | 3,698 | `708dcbcc` | `gitingest-api` (133 files, 682.9 KiB) | Cross-platform Emscripten SDK installer/activator (`emsdk.py`, `emsdk_manifest.json`) managing pinned LLVM, Binaryen, Node, and Python toolchains. |
| [`emscripten-fastcomp`](https://github.com/emscripten-core/emscripten-fastcomp) | Archived | `LLVM` | 185 | `9bdc7bb4` | `github-tree-fallback` (10 files, 171.8 KiB) | Archived legacy asm.js LLVM backend superseded by the upstream LLVM WebAssembly backend. |
| [`setup-emsdk`](https://github.com/emscripten-core/setup-emsdk) | Active | `TypeScript` | 123 | `0822153d` | `gitingest-api` (12 files, 58.4 KiB) | Official GitHub Action (`TypeScript`) installing and caching Emscripten SDK versions in CI workflows. |
| [`emscripten-fastcomp-clang`](https://github.com/emscripten-core/emscripten-fastcomp-clang) | Archived | `C++` | 79 | `a21700cd` | `gitingest-api` (10000 files, 35.10 MiB) | Archived legacy Fastcomp Clang frontend superseded by upstream LLVM Clang. |
| [`posixtestsuite`](https://github.com/emscripten-core/posixtestsuite) | Active | `C` | 67 | `85555325` | `gitingest-api` (2867 files, 6.63 MiB) | Emscripten-adapted Open POSIX Test Suite validating pthread, signal, semaphore, and libc compliance. |
| [`emscripten-ci`](https://github.com/emscripten-core/emscripten-ci) | Active | `Dockerfile` | 9 | `02b09b1e` | `gitingest-api` (3 files, 5.7 KiB) | Docker environment specification (`Dockerfile`) used to build and test Emscripten's own continuous integration. |
| [`llvm-project`](https://github.com/emscripten-core/llvm-project) | Active Fork | `LLVM` | 9 | `caf695ff` | `github-tree-fallback` (10 files, 168.2 KiB) | Upstream-synced LLVM fork supporting Emscripten Wasm backend/Clang integration and release validation. |
| [`musl`](https://github.com/emscripten-core/musl) | Active | `C` | 8 | `85e0e351` | `gitingest-api` (2605 files, 3.64 MiB) | Emscripten fork of musl libc providing C standard library headers and runtime primitives for `wasm32-unknown-emscripten`. |
| [`terser`](https://github.com/emscripten-core/terser) | Archived | `—` | 2 | `b47c3e68` | `gitingest-api` (288 files, 2.64 MiB) | Archived downstream Terser fork; Emscripten now consumes upstream Terser directly. |
| [`bazel-central-registry`](https://github.com/emscripten-core/bazel-central-registry) | Active Fork | `—` | 0 | `40d0113b` | `gitingest-api` (10000 files, 10.65 MiB) | Emscripten-maintained Bazel Central Registry fork publishing the Bzlmod `emsdk` module and C/C++ Wasm toolchain rules. |
| [`emsdk-arm64-build-infra`](https://github.com/emscripten-core/emsdk-arm64-build-infra) | Active | `—` | 0 | `empty` | `empty-repo` (0 files, 450 B) | ARM64 Emscripten SDK build infrastructure repository (currently initialized empty upstream). |

---

## 3. `wasm32-unknown-emscripten` vs `wasm32-unknown-unknown` vs `wasm32-wasip2` in Aphrody

Aphrody targets three complementary WebAssembly ABIs documented across [`rust-toolchain.toml`](../../../rust-toolchain.toml), [`tooling/bun/web-build.ts`](../../../tooling/bun/web-build.ts), and [`docs/aphrody/architecture/cross-compile-and-depot-tools.md`](./cross-compile-and-depot-tools.md):

| Dimension | `wasm32-unknown-unknown` (Aphrody Browser UI) | `wasm32-wasip2` (Aphrody Extensions) | `wasm32-unknown-emscripten` (C/C++ POSIX & Emscripten) |
| :--- | :--- | :--- | :--- |
| **Primary Aphrody Crates** | [`gpui_web`](../../../crates/gpui_web/Cargo.toml), `aphrody_desktop`, `nsh-platform-web`, `gpui_wgpu` | [`extension_host`](../../../crates/extension_host/Cargo.toml) (Wasmtime `48.0.7`), `extension_api` (WIT) | [`nsh-core`](../../../crates/nsh-core/Cargo.toml#L99-L100) (`target_os = "emscripten"`), `wgpu-core-deps-emscripten` (inside the pinned `aphrody-labs/wgpu` tag) |
| **Standard C Library (`libc`)** | None (pure Rust `std` stub + JS bindings) | WASI Preview 2 component model + `wasi-libc` | Full POSIX/musl `libc` (`emscripten-core/musl`), `cfg(unix)`, signals, pthreads, virtual FS |
| **JS / Host Bindings** | `wasm-bindgen 0.2.129` + `web-sys` / `js-sys` | Canonical ABI / `wit-bindgen` host imports | Embind (`bind.h` link-time invokers in 7.0.1), `EM_JS`, `EM_ASM`, `ccall`/`cwrap`, `WebIDL Binder` |
| **WebGPU API Bridge** | Browser `navigator.gpu` via `wgpu` -> `web-sys` | Host-mediated WIT GPU/UI capability | **Emdawnwebgpu** (`--use-port=emdawnwebgpu`, `<webgpu/webgpu.h>`, replacing removed `-sUSE_WEBGPU`) |
| **Async / Blocking Bridge** | `wasm-bindgen-futures` Promise poll | Wasmtime async host calls + epoch interruption | **JSPI** (`-sJSPI`, zero size overhead) or **Asyncify** (`-sASYNCIFY`) + `emscripten_set_main_loop()` |
| **Threads & Shared Memory** | Opt-in `wasm_thread` (`target-feature=+atomics`, COOP/COEP) | Host thread pool outside guest sandbox | `-pthread -sPTHREAD_POOL_SIZE=...` (COOP/COEP `SharedArrayBuffer`) or `-sWASM_WORKERS` |
| **Linker & Optimizer** | `rust-lld` + `wasm-bindgen` + Binaryen `wasm-opt -Oz` | `wasm-tools component` + Wasmtime Cranelift | `emcc` -> `wasm-ld` (LLVM `24`) + Binaryen `wasm-opt` + Closure (`--closure=1`) |

> **When to choose Emscripten in Aphrody**: Reach for Emscripten only when integrating C/C++ libraries that require POSIX `libc`, C++ exceptions/RTTI, `pthreads`, virtual POSIX filesystems (`MEMFS`/`IDBFS`), or native C/C++ `webgpu.h` libraries (via Emdawnwebgpu). Keep pure-Rust Aphrody UI/apps (`aphrody_desktop`, `gpui_web`, `nsh-platform-web`) on `wasm32-unknown-unknown` via [`tooling/bun/web-build.ts`](../../../tooling/bun/web-build.ts) and sandboxed IDE plugins on `wasm32-wasip2`.

---

## 4. Production C/C++ -> WebAssembly Rules (Emscripten 7.0.x Verified)

1. **WebGPU via Emdawnwebgpu (Not `-sUSE_WEBGPU`)**:
   - Emscripten 4.0.10+ through 7.0.1-git delegates `<webgpu/webgpu.h>` and `<webgpu/webgpu_cpp.h>` to Dawn's `emdawnwebgpu` port (currently pinned upstream at `v20261105.214757`).
   - Pass `--use-port=emdawnwebgpu` at **both** compile and link stages, and pass `--closure=1` during release linking to strip JS glue size.
2. **Modular ES6 Output Without Global `Module` Pollution**:
   - Prefer `-sMODULARIZE -sEXPORT_ES6` (or emit `.mjs`, which implies `-sEXPORT_ES6` -> `-sMODULARIZE`) so instantiating C/C++ Wasm modules works cleanly inside Bun, Vite, Workers, and Cloudflare Static Assets.
3. **Modern Minimal CLI Flags (`docs/compiling/best_practices.html`)**:
   - Omit redundant defaults like `-sWASM=1`; omit `=1` on boolean flags (`-sSTRICT`, `-sALLOW_MEMORY_GROWTH`, `-sMODULARIZE`, `-sEXPORT_ES6`).
   - Prefer standard driver flags (`-pthread`, `-m64`, `-O3`/`-Os`/`-Oz`, `-flto`) and `@response_file` lists for long exports.
4. **Async & Blocking I/O (`JSPI` vs `Asyncify`)**:
   - Prefer `emscripten_set_main_loop()` for frame loops; use `-sJSPI` where V8/browser JSPI is available (zero binary expansion) or `-sASYNCIFY` with `ASYNCIFY_ONLY` allowlists for universal fallback.
5. **Multithreading & COOP/COEP Headers**:
   - `-pthread` requires `SharedArrayBuffer`, which browsers gate on `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp` (or `credentialless`). Configure these headers selectively in dev/edge servers (matching Aphrody's `--coi` / `/play/*` policy).
6. **SIMD & Clang/LLVM Version Alignment**:
   - Pass `-msimd128` at compile time to enable LLVM Wasm autovectorization plus `<wasm_simd128.h>`, x86 SSE–AVX2, and ARM NEON intrinsics.
   - Note that Aphrody's local host Clang (`clang version 23.1.5 (https://github.com/llvm/llvm-project 392dfff97d29b59d77e5cac6d4b6ef2585fe7117)`) targets `wasm32-unknown-unknown` and `wasm32-unknown-emscripten` object emission (`clang -c --target=wasm32-unknown-emscripten`), whereas Emscripten `7.0.1-git` `tools/shared.py` enforces `EXPECTED_LLVM_VERSION = 24` for its bundled `emsdk` `llvm-project` sysroot/JS-linker pass.
7. **Embind & Emscripten 7.0.0–7.0.1 Linker Upgrades**:
   - Emscripten 7.0.1 generates Embind JS invokers at **link time** instead of runtime `new Function`, making `-sDYNAMIC_EXECUTION=0` fast without `-sEMBIND_AOT`.
   - Emscripten 7.0.0 moved program-wide builtins out of `libclang_rt.builtins` into `libemcc` (link `-lemcc` when passing `-nostdlib`) and removed `EMCC_ONLY_FORCED_STDLIBS`.

---

## 5. Bun + `xh` Reproducible Automation & CI Verification

```sh
# 1. Full pipeline: crawl emscripten.org + ingest all 12 emscripten-core repos + emit docs/skills
bun tooling/bun/emscripten-crawl.ts all --max-pages 80 --concurrency 6

# 2. Run offline unit, parser, XML, frontmatter, and link-integrity tests
bun test tooling/bun/emscripten-crawl.test.ts

# 3. Verify zero ephemeral path leaks and relative doc link integrity workspace-wide
bun tooling/bun/repo-hygiene.ts

# 4. Probe local host toolchain & serve COOP/COEP-enabled Wasm artifacts via Bun + xh
xh --ignore-stdin --check-status --headers --follow GET https://emscripten.org/
```

### Companion Artifacts

- **`llms.txt` Index**: [`docs/aphrody/architecture/emscripten-llms.txt`](./emscripten-llms.txt)
- **Agent Skill**: `fw: plugins/aphrody/skills/emscripten`
- **Cross-Compilation Context**: [`docs/aphrody/architecture/cross-compile-and-depot-tools.md`](./cross-compile-and-depot-tools.md) & [`docs/aphrody/architecture/production-guide.md`](./production-guide.md)
