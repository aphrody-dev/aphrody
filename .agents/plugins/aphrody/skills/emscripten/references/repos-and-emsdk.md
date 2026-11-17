# Repositories & `emsdk` Reference (`emscripten-core`)

> Complete inventory of all 12 repositories in `https://github.com/orgs/emscripten-core/repositories` plus `emsdk` provisioning, CI setup, Bazel rules, and host toolchain diagnostics.

---

## 1. Host Toolchain Diagnostic (`probeEmcc()`)

| Tool / Variable | Host State (`win32/x64`) | Notes |
| :--- | :--- | :--- |
| `emcc` / `em++` | **Absent on host** | Required for C/C++ -> `wasm32-unknown-emscripten` linking & JS glue generation |
| `emsdk` CLI & Env | `emsdk`: Absent, `EMSDK=unset` | Provision via `git clone https://github.com/emscripten-core/emsdk` |
| `clang` | `clang version 23.1.5 (https://github.com/llvm/llvm-project 392dfff97d29b59d77e5cac6d4b6ef2585fe7117)` (`C:\Program Files\LLVM\bin\clang.exe`) | Host LLVM 23 vs Emscripten 7.0.1-git `EXPECTED_LLVM_VERSION = 24` |
| `lld` | `LLD 23.1.5 (https://github.com/llvm/llvm-project 392dfff97d29b59d77e5cac6d4b6ef2585fe7117)` | Host `lld-link`; Emscripten bundles `wasm-ld` inside `emsdk` `upstream/bin/` |
| `wasm-opt` | `wasm-opt version 133 (version_133)` | Binaryen optimizer shared with `tooling/bun/web-build.ts` |
| `wasm-bindgen` | `wasm-bindgen 0.2.129` | Rust `wasm32-unknown-unknown` binding generator |
| `xh` / `gh` | `xh`: `xh 0.26.3`; `gh`: `gh version 2.103.3 (2026-10-26)` | Used by `tooling/bun/emscripten-crawl.ts` |

---

## 2. Complete `emscripten-core` Organization Matrix (12 Repos)

| Repository | State | Language | Stars | Head SHA | Ingest Digest | Role & Integration Notes |
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

## 3. `emsdk` Toolchain Architecture (`emsdk_manifest.json`)

1. **Upstream Binary Waterfall**:
   - `emsdk_manifest.json` fetches prebuilt `wasm-binaries.{tar.xz,zip}` from `storage.googleapis.com/webassembly/emscripten-releases-builds/` containing aligned **LLVM + Clang + `wasm-ld` + Binaryen + Node + Python** under `upstream/`.
   - Sets `LLVM_ROOT`, `BINARYEN_ROOT`, `EMSCRIPTEN_ROOT`, and generates `.emscripten` config automatically on `emsdk activate`.
2. **Standard Installation & Pinning**:
   ```sh
   git clone https://github.com/emscripten-core/emsdk.git
   cd emsdk
   ./emsdk install latest   # or pin exact release e.g. 7.0.0
   ./emsdk activate latest
   source ./emsdk_env.sh    # Windows cmd: emsdk_env.bat
   emcc --version
   ```
3. **LLVM Version Matching Invariant**:
   - Live `tools/shared.py` in `emscripten-core/emscripten` enforces `EXPECTED_LLVM_VERSION = 24` for `7.0.1-git`.
   - When using a standalone host Clang (e.g. LLVM `23.1.5` on Windows), `clang --target=wasm32-unknown-emscripten -c` emits object files, **but** final `emcc` linking and LTO bitcode require matching LLVM major versions (or pre-built `emsdk` LLVM 24). Never pass mismatched `-flto` bitcode between Clang 23 and LLVM 24 `wasm-ld`.
4. **GitHub Actions (`emscripten-core/setup-emsdk` & `emscripten-ci`)**:
   - Use `emscripten-core/setup-emsdk` (or `mymindstorm/setup-emsdk`) with pinned `version: 7.0.0` and Actions cache enabled instead of `latest` on production release lanes.
   - Reference `emscripten-core/emscripten-ci` `Dockerfile` when reproducing upstream Linux CI containers on fleet node `fr1`.
5. **Bazel (`emscripten-core/bazel-central-registry`) & Legacy Archives**:
   - Bzlmod projects pull `emsdk` via `bazel_dep(name = "emsdk", ...)` maintained in `emscripten-core/bazel-central-registry/modules/emsdk/`.
   - **Do NOT use** `emscripten-fastcomp`, `emscripten-fastcomp-clang` (archived 2021 asm.js backend), or `emscripten-core/terser` (archived fork; upstream `terser` is used directly).
