# `emcc` & WebAssembly Best Practices (Emscripten 7.0.x)

> Grounded directly in `https://emscripten.org/docs/compiling/best_practices.html`, `Modularized-Output.html`, `asyncify.html`, `pthreads.html`, `simd.html`, and `ChangeLog.md` (7.0.0 / 7.0.1-git).

---

## 1. Official `emcc` Command-Line Hygiene

- **Omit enabled-by-default settings**: Do not pass `-sWASM=1`.
- **Omit `=1` on boolean `-s` flags**: Write `-sSTRICT`, `-sALLOW_MEMORY_GROWTH`, `-sMODULARIZE`, `-sEXPORT_ES6` rather than `=1`.
- **Prefer standard driver flags**: Use `-pthread` (not `-sUSE_PTHREADS`), `-m64` (not `-sMEMORY64`), `-Werror -Wall`.
- **Use comma-separated lists & response files**: Write `-sEXPORTED_FUNCTIONS=_main,_malloc` (avoid JSON array quoting) and move huge flag lists into `@flags.rsp`.
- **Separate compilation**: Compile `emcc -c foo.cpp -o foo.o` per translation unit before linking `emcc foo.o bar.o -o out.mjs`.
- **Release vs Debug profiles**:
  - **Release**: `-O3` (max speed) or `-Os`/`-Oz` (min size) + `-flto` at **both** compile and link + `--closure=1` at link (unless an external bundler minifies JS) + `-sENVIRONMENT=web` (or `web,worker`).
  - **Debug**: `-g` at compile + `-g`, `-gline-tables-only`, or `-gsource-map` at link; omit `-O2`/`-O3`/`-flto`.

---

## 2. Modularized ES6 Output (`-sMODULARIZE` & `-sEXPORT_ES6`)

```sh
# Emitting .mjs automatically enables -sEXPORT_ES6 (which implies -sMODULARIZE).
emcc -O3 -flto -sSTRICT -sENVIRONMENT=web,worker \
  -sMODULARIZE -sEXPORT_ES6 -sEXPORT_NAME=CreateEngine \
  --closure=1 engine.o -o dist/engine.mjs
```

```ts
// Clean TypeScript / Bun ESM consumption with zero global Module pollution
import CreateEngine from "./dist/engine.mjs";

const engine = await CreateEngine({
  locateFile: (path: string) => new URL(path, import.meta.url).href,
  print: (line: string) => console.info("[wasm]", line),
});
engine._engine_step(16.6);
```

- **`-sMODULARIZE=instance` (experimental)**: Exports a single shared instance initializer rather than a multi-instance factory (implicitly enables `EXPORT_ES6`).
- **`-sWASM_ESM_INTEGRATION` / Source Phase Imports**: Experimental ES Module Integration binding Wasm imports/exports directly via native ESM phase imports.

---

## 3. Asynchronous Execution: Main Loop, JSPI, and Asyncify

Never block the browser main thread in an infinite C/C++ `while (true)` loop:

| Mechanism | Flag / API | Binary Overhead | Best For |
| :--- | :--- | :--- | :--- |
| **Event-Loop Callback** | `emscripten_set_main_loop(fn, 0, true)` | Zero | Frame-driven rendering / WebGPU games / UI loops |
| **JSPI (Recommended Async)** | `-sJSPI` + `EM_ASYNC_JS` | **Zero code bloat** | Modern Chromium/V8 & JSPI-capable engines awaiting Promises synchronously from C/C++ |
| **Asyncify (Universal)** | `-sASYNCIFY` + `-sASYNCIFY_ONLY=[...]` | Significant without allowlist | Older browsers without JSPI when porting legacy blocking C/C++ call stacks |

> **Tip**: Always scope `-sASYNCIFY_ONLY` (or `-sASYNCIFY_IGNORE_INDIRECT`) when using `-sASYNCIFY` so Binaryen does not instrument unrelated hot loops.

---

## 4. Pthreads (`SharedArrayBuffer`), Wasm Workers, and COOP/COEP

```sh
emcc -O3 -pthread -c worker_math.cpp -o worker_math.o
emcc -O3 -pthread -sPTHREAD_POOL_SIZE=navigator.hardwareConcurrency worker_math.o -o dist/worker_math.mjs
```

- **Mandatory HTTP Response Headers** (browser gate on `SharedArrayBuffer`):
  - `Cross-Origin-Opener-Policy: same-origin`
  - `Cross-Origin-Embedder-Policy: require-corp` (or `credentialless` when embedding uncredentialed third-party assets)
- Guard C/C++ code portably with `#ifdef __EMSCRIPTEN_PTHREADS__`.
- **Single-binary note**: A `-pthread` build requires `SharedArrayBuffer` (COOP/COEP) and will not silently run in a non-COI context; build separate `baseline` vs `coi` cohorts (matching Aphrody's `tooling/bun/web-build.ts` `--coi` cohort rule!).
- **Lightweight alternative**: `-sWASM_WORKERS` creates dedicated Wasm Workers with `<emscripten/wasm_worker.h>` and smaller startup overhead than full POSIX pthreads.

---

## 5. WebAssembly SIMD (`-msimd128`) & Exceptions

- Pass `-msimd128` at **compile time** to enable:
  1. LLVM SLP/loop autovectorization (disable with `-fno-vectorize -fno-slp-vectorize` if unwanted).
  2. GCC/Clang vector extensions (`__attribute__((vector_size(16)))`).
  3. Portable Wasm intrinsics (`#include <wasm_simd128.h>`).
  4. Automatic translation of x86 `*mmintrin.h` (SSE through AVX2/FMA) and ARM `<arm_neon.h>`.
- Supported across Chrome >= 91, Firefox >= 89, Safari >= 16.4, Node >= 16.4, and Bun.
- Prefer native Wasm EH (`-fwasm-exceptions`) over legacy JS `try/catch` invoke wrappers when targeting modern runtimes.

---

## 6. Emscripten 7.0.0 & 7.0.1-git Breaking Changes Checklist

- **7.0.1-git**: Embind emits JS invokers at **link time** (no `new Function` at runtime; `-sDYNAMIC_EXECUTION=0` runs fast without needing `-sEMBIND_AOT`).
- **7.0.0**: `libclang_rt.builtins` and `libclang_rt.profile` now link into side modules directly; new `libemcc` holds program-wide symbols (`-lemcc` required if using `-nostdlib`); `EMCC_ONLY_FORCED_STDLIBS` removed.
