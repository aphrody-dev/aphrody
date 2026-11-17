---
name: emscripten
description: >-
  Authoritative Emscripten 7.0.1-git toolchain, emsdk, and C/C++-to-WebAssembly engineering
  skill for aphrody. Use whenever compiling C/C++ with emcc/em++, targeting
  wasm32-unknown-emscripten alongside wasm32-unknown-unknown and wasm32-wasip2,
  wiring Emdawnwebgpu (--use-port=emdawnwebgpu), configuring -sMODULARIZE -sEXPORT_ES6,
  JSPI/Asyncify, Embind/EM_JS/EM_ASM, pthreads with COOP/COEP, or -msimd128.
license: Apache-2.0
compatibility: >-
  Compliant with agentskills.io, Claude Code, and Aphrody plugin discovery.
  Requires emsdk/emcc for C/C++ Wasm compilation; Bun, xh, and gh for live doc/repo sync.
metadata:
  upstream-version: "7.0.1-git"
  crawled-routes: "92"
  ingested-repos: "12"
  generator: "tooling/bun/emscripten-crawl.ts"
---

# Emscripten (7.0.1-git) & C/C++ WebAssembly Engineering

> Progressive-disclosure skill synthesized from live `emscripten.org` (92 routes) and all 12 `github.com/emscripten-core` repositories. Local `emcc`/`emsdk` is NOT installed on this Windows host (host has `clang` `clang version 23.1.5 (https://github.com/llvm/llvm-project 392dfff97d29b59d77e5cac6d4b6ef2585fe7117)`, `lld` `LLD 23.1.5 (https://github.com/llvm/llvm-project 392dfff97d29b59d77e5cac6d4b6ef2585fe7117)`, `wasm-opt` `wasm-opt version 133 (version_133)`); install via `emsdk` or `setup-emsdk` before compiling C/C++ Emscripten targets.

## Quick Reference Map

Read only the targeted reference file needed for your task:

| Focus Area | Reference Document | When to Read |
| :--- | :--- | :--- |
| **All 12 Repos & `emsdk` Setup** | [references/repos-and-emsdk.md](references/repos-and-emsdk.md) | Installing `emsdk`, CI via `setup-emsdk`, Bazel `emsdk`, `musl`, LLVM 24 vs host Clang 23, or checking repo roles |
| **Docs Sitemap & C/JS APIs** | [references/sitemap-and-api-reference.md](references/sitemap-and-api-reference.md) | Locating official `emscripten.org` pages (`searchindex.js` index), C headers (`emscripten.h`, `html5.h`, `fetch.h`), or FS/Module APIs |
| **`emcc` Flags & Wasm Best Practices** | [references/emcc-wasm-best-practices.md](references/emcc-wasm-best-practices.md) | Selecting `-sSTRICT`, `-sMODULARIZE -sEXPORT_ES6`, `-sJSPI` vs `-sASYNCIFY`, `-pthread` + COOP/COEP, `-msimd128`, or `wasm32-unknown-unknown`/`wasip2` tradeoffs |
| **C/C++ & WebGPU Interop** | [references/c-cpp-webgpu-interop.md](references/c-cpp-webgpu-interop.md) | Integrating `--use-port=emdawnwebgpu`, `EM_JS`/`EM_ASYNC_JS`, Embind (7.0.1 link-time invokers), `ccall`/`cwrap`, or Aphrody `gpui_web`/`nsh-core` bridges |

## Non-Negotiable Directives & Traps to Avoid

1. **Use Emdawnwebgpu (`--use-port=emdawnwebgpu`), Never `-sUSE_WEBGPU`**:
   Emscripten's legacy built-in WebGPU bindings were replaced by Dawn's external port. Pass `--use-port=emdawnwebgpu` at **both** compile and link steps, and add `--closure=1` when linking release builds.
2. **Follow Modern `emcc` Flag Hygiene (`7.0.x`)**:
   - Omit `-sWASM=1` (always default) and omit `=1` on boolean flags (`-sSTRICT`, `-sALLOW_MEMORY_GROWTH`, `-sMODULARIZE`, `-sEXPORT_ES6`).
   - Prefer standard compiler flags: `-pthread` (not `-sUSE_PTHREADS`), `-m64` (not `-sMEMORY64`), `-O3`/`-Os`/`-Oz`, `-flto`.
   - Use `.mjs` or `-sEXPORT_ES6` (implies `-sMODULARIZE`) to prevent polluting global `Module`.
3. **Avoid High-Frequency Wasm <-> JS Boundary Calls**:
   Batch pixels, vertices, and audio frames in linear memory views (`HEAPU8` / `HEAPF32` / `typed_memory_view`) instead of per-item callbacks.
4. **Gate `-pthread` Behind COOP/COEP Headers**:
   Browser `SharedArrayBuffer` requires `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp` (or `credentialless`). Test locally with Bun + `xh` header verification before shipping.
5. **Respect Aphrody Target Boundaries**:
   - Keep pure-Rust browser UI (`aphrody_desktop`, `gpui_web`, `nsh-platform-web`) on `wasm32-unknown-unknown` via `tooling/bun/web-build.ts`.
   - Keep sandboxed extensions on `wasm32-wasip2` (`wasmtime 48.0.7`).
   - Use `wasm32-unknown-emscripten` only for C/C++ POSIX, pthreads, virtual FS, or native `webgpu.h` workloads (`nsh-core` `cfg(target_os = "emscripten")`).
6. **Emscripten 7.0.0–7.0.1 ABI & Linker Changes**:
   - Emscripten 7.0.1 generates Embind invokers at link time (`-sDYNAMIC_EXECUTION=0` fast without `-sEMBIND_AOT`).
   - Emscripten 7.0.0 removed `EMCC_ONLY_FORCED_STDLIBS` and introduced `libemcc` (`-lemcc` when linking with `-nostdlib`).
   - Upstream `7.0.1-git` checks `EXPECTED_LLVM_VERSION = 24`; always use `emsdk`'s bundled LLVM/Binaryen for final `emcc` links rather than mixing host Clang 23 bitcode.

## Verification & Sync Commands

```sh
# Refresh live crawl (emscripten.org) + all 12 emscripten-core repos + docs/skills
bun tooling/bun/emscripten-crawl.ts all --max-pages 80 --concurrency 6

# Verify offline unit tests, XML/frontmatter invariants, and link integrity
bun test tooling/bun/emscripten-crawl.test.ts

# Check workspace-wide docs & skill hygiene
bun tooling/bun/repo-hygiene.ts
```

- Architecture overview: [`docs/architecture/emscripten-ecosystem.md`](../../../../../docs/architecture/emscripten-ecosystem.md)
- LLM compact index: [`docs/architecture/emscripten-llms.txt`](../../../../../docs/architecture/emscripten-llms.txt)
