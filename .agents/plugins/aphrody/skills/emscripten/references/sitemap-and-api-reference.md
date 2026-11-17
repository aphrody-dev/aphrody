# `emscripten.org` Sitemap & API Reference (7.0.1-git)

> Live crawl summary: **92** unique same-origin HTML routes discovered (**80** fetched in bounded BFS, **90** indexed in `/searchindex.js`; `/sitemap.xml` returned `HTTP 404` on GitHub Pages Sphinx).

---

## 1. Crawl Methodology & `xh` Transport Findings

1. **Why `/sitemap.xml` Fails Upstream**: `GET https://emscripten.org/sitemap.xml` currently returns `HTTP/2.0 404 Not Found` from GitHub Pages/Varnish (historically `404`/`503`).
2. **Authoritative Discovery via `/searchindex.js` + BFS**:
   - `https://emscripten.org/searchindex.js` returns `HTTP 200` JSON inside `Search.setIndex({...})` listing all `docnames`, `titles`, and `filenames`.
   - Concurrent BFS from `https://emscripten.org/` discovers root pages (`/index.html`, `/genindex.html`, `/search.html`), verifies HTTP 200 status codes, and extracts cross-origin specification links.
   - Core pages are cross-verified with `xh --ignore-stdin --check-status --follow --timeout=15 GET <url>`.

---

## 2. Core C/C++ Headers & JS Runtime APIs

| Header / Runtime Surface | Purpose | Key Docs Route |
| :--- | :--- | :--- |
| `<emscripten.h>` | `emscripten_set_main_loop`, `EM_JS`, `EM_ASM`, `emscripten_sleep`, `emscripten_run_script` | [`emscripten.h`](https://emscripten.org/docs/api_reference/emscripten.h.html) |
| `<emscripten/html5.h>` | DOM callbacks (keyboard, mouse, pointer, touch, wheel, resize, fullscreen, WebGL context) | [`html5.h`](https://emscripten.org/docs/api_reference/html5.h.html) |
| `<emscripten/bind.h>` & `<emscripten/val.h>` | Embind C++ classes/functions/smart pointers and dynamic JS `val` handles | [`bind.h`](https://emscripten.org/docs/api_reference/bind.h.html), [`val.h`](https://emscripten.org/docs/api_reference/val.h.html) |
| `<emscripten/fetch.h>` | Async/sync `XMLHttpRequest` / IndexedDB caching C API | [`fetch.h`](https://emscripten.org/docs/api_reference/fetch.html) |
| `<emscripten/proxying.h>` & `wasm_workers` | Lock-free worker task proxying and lightweight Wasm Workers without full pthreads | [`proxying.h`](https://emscripten.org/docs/api_reference/proxying.h.html), [`wasm_workers`](https://emscripten.org/docs/api_reference/wasm_workers.html) |
| `<emscripten/fiber.h>` & `stack.h` | User-space Asyncify fibers and stack inspection/bounds checking | [`fiber.h`](https://emscripten.org/docs/api_reference/fiber.h.html), [`stack.h`](https://emscripten.org/docs/api_reference/stack.h.html) |
| `<webgpu/webgpu.h>` (`emdawnwebgpu`) | Dawn-compatible C/C++ WebGPU API layered over browser `navigator.gpu` | [`WebGPU-support`](https://emscripten.org/docs/porting/multimedia_and_graphics/WebGPU-support.html) |
| `FS` (`MEMFS`, `IDBFS`, `NODEFS`, `WORKERFS`) | Virtual POSIX filesystem APIs, preloading (`--preload-file`), and persistent IndexedDB mounts | [`Filesystem-API`](https://emscripten.org/docs/api_reference/Filesystem-API.html) |
| `Module` & `preamble.js` | Factory lifecycle hooks (`onRuntimeInitialized`, `locateFile`, `print`, `ccall`, `cwrap`) | [`module`](https://emscripten.org/docs/api_reference/module.html), [`preamble.js`](https://emscripten.org/docs/api_reference/preamble.js.html) |

---

## 3. Complete Crawled Route Index by Section

### `docs/api_reference/` (17 routes)

- [`/docs/api_reference/advanced-apis.html`](https://emscripten.org/docs/api_reference/advanced-apis.html) — Advanced APIs - Emscripten 7.0.1-git (dev) documentation
- [`/docs/api_reference/bind.h.html`](https://emscripten.org/docs/api_reference/bind.h.html) — bind.h (under-construction) - Emscripten 7.0.1-git (dev) documentation
- [`/docs/api_reference/console.h.html`](https://emscripten.org/docs/api_reference/console.h.html) — console.h - Emscripten 7.0.1-git (dev) documentation
- [`/docs/api_reference/emscripten.h.html`](https://emscripten.org/docs/api_reference/emscripten.h.html) — emscripten.h - Emscripten 7.0.1-git (dev) documentation
- [`/docs/api_reference/fetch.html`](https://emscripten.org/docs/api_reference/fetch.html) — Fetch API - Emscripten 7.0.1-git (dev) documentation
- [`/docs/api_reference/fiber.h.html`](https://emscripten.org/docs/api_reference/fiber.h.html) — fiber.h - Emscripten 7.0.1-git (dev) documentation
- [`/docs/api_reference/Filesystem-API.html`](https://emscripten.org/docs/api_reference/Filesystem-API.html) — File System API - Emscripten 7.0.1-git (dev) documentation
- [`/docs/api_reference/html5.h.html`](https://emscripten.org/docs/api_reference/html5.h.html) — html5.h - Emscripten 7.0.1-git (dev) documentation
- [`/docs/api_reference/index.html`](https://emscripten.org/docs/api_reference/index.html) — API Reference - Emscripten 7.0.1-git (dev) documentation
- [`/docs/api_reference/module.html`](https://emscripten.org/docs/api_reference/module.html) — Module object - Emscripten 7.0.1-git (dev) documentation
- [`/docs/api_reference/preamble.js.html`](https://emscripten.org/docs/api_reference/preamble.js.html) — preamble.js - Emscripten 7.0.1-git (dev) documentation
- [`/docs/api_reference/proxying.h.html`](https://emscripten.org/docs/api_reference/proxying.h.html) — proxying.h - Emscripten 7.0.1-git (dev) documentation
- [`/docs/api_reference/stack.h.html`](https://emscripten.org/docs/api_reference/stack.h.html) — stack.h - Emscripten 7.0.1-git (dev) documentation
- [`/docs/api_reference/trace.h.html`](https://emscripten.org/docs/api_reference/trace.h.html) — trace.h - Emscripten 7.0.1-git (dev) documentation
- [`/docs/api_reference/val.h.html`](https://emscripten.org/docs/api_reference/val.h.html) — val.h - Emscripten 7.0.1-git (dev) documentation
- [`/docs/api_reference/wasm_audio_worklets.html`](https://emscripten.org/docs/api_reference/wasm_audio_worklets.html) — Wasm Audio Worklets API - Emscripten 7.0.1-git (dev) documentation
- [`/docs/api_reference/wasm_workers.html`](https://emscripten.org/docs/api_reference/wasm_workers.html) — Wasm Workers API - Emscripten 7.0.1-git (dev) documentation

### `docs/building_from_source/` (4 routes)

- [`/docs/building_from_source/configuring_emscripten_settings.html`](https://emscripten.org/docs/building_from_source/configuring_emscripten_settings.html) — Configuring Emscripten Settings when Manually Building from Source - Emscripten 7.0.1-git (dev) documentation
- [`/docs/building_from_source/index.html`](https://emscripten.org/docs/building_from_source/index.html) — Building Emscripten from Source - Emscripten 7.0.1-git (dev) documentation
- [`/docs/building_from_source/toolchain_what_is_needed.html`](https://emscripten.org/docs/building_from_source/toolchain_what_is_needed.html) — Emscripten Toolchain Requirements - Emscripten 7.0.1-git (dev) documentation
- [`/docs/building_from_source/verify_emscripten_environment.html`](https://emscripten.org/docs/building_from_source/verify_emscripten_environment.html) — Verifying the Emscripten Development Environment - Emscripten 7.0.1-git (dev) documentation

### `docs/compiling/` (11 routes)

- [`/docs/compiling/best_practices.html`](https://emscripten.org/docs/compiling/best_practices.html) — Best Practices - Emscripten 7.0.1-git (dev) documentation
- [`/docs/compiling/Building-Projects.html`](https://emscripten.org/docs/compiling/Building-Projects.html) — Building Projects - Emscripten 7.0.1-git (dev) documentation
- [`/docs/compiling/Contrib-Ports.html`](https://emscripten.org/docs/compiling/Contrib-Ports.html) — Emscripten Contrib Ports - Emscripten 7.0.1-git (dev) documentation
- [`/docs/compiling/CrossOriginStorage.html`](https://emscripten.org/docs/compiling/CrossOriginStorage.html) — Cross-Origin Storage (COS) - Emscripten 7.0.1-git (dev) documentation
- [`/docs/compiling/Deploying-Pages.html`](https://emscripten.org/docs/compiling/Deploying-Pages.html) — Deploying Emscripten Compiled Pages - Emscripten 7.0.1-git (dev) documentation
- [`/docs/compiling/Dynamic-Linking.html`](https://emscripten.org/docs/compiling/Dynamic-Linking.html) — Dynamic Linking - Emscripten 7.0.1-git (dev) documentation
- [`/docs/compiling/GitLab.html`](https://emscripten.org/docs/compiling/GitLab.html) — Building and Deploying on GitLab - Emscripten 7.0.1-git (dev) documentation
- [`/docs/compiling/index.html`](https://emscripten.org/docs/compiling/index.html) — Compiling and Running Projects - Emscripten 7.0.1-git (dev) documentation
- [`/docs/compiling/Modularized-Output.html`](https://emscripten.org/docs/compiling/Modularized-Output.html) — Modularized Output - Emscripten 7.0.1-git (dev) documentation
- [`/docs/compiling/Running-html-files-with-emrun.html`](https://emscripten.org/docs/compiling/Running-html-files-with-emrun.html) — Running HTML files with emrun - Emscripten 7.0.1-git (dev) documentation
- [`/docs/compiling/WebAssembly.html`](https://emscripten.org/docs/compiling/WebAssembly.html) — Building to WebAssembly - Emscripten 7.0.1-git (dev) documentation

### `docs/contributing/` (4 routes)

- [`/docs/contributing/AUTHORS.html`](https://emscripten.org/docs/contributing/AUTHORS.html) — AUTHORS - Emscripten 7.0.1-git (dev) documentation
- [`/docs/contributing/contributing.html`](https://emscripten.org/docs/contributing/contributing.html) — Contributing - Emscripten 7.0.1-git (dev) documentation
- [`/docs/contributing/developers_guide.html`](https://emscripten.org/docs/contributing/developers_guide.html) — Developer’s Guide - Emscripten 7.0.1-git (dev) documentation
- [`/docs/contributing/index.html`](https://emscripten.org/docs/contributing/index.html) — Contributing to Emscripten - Emscripten 7.0.1-git (dev) documentation

### `docs/debugging/` (1 routes)

- [`/docs/debugging/Sanitizers.html`](https://emscripten.org/docs/debugging/Sanitizers.html) — Debugging with Sanitizers - Emscripten 7.0.1-git (dev) documentation

### `docs/getting_started/` (6 routes)

- [`/docs/getting_started/bug_reports.html`](https://emscripten.org/docs/getting_started/bug_reports.html) — Bug Reporting - Emscripten 7.0.1-git (dev) documentation
- [`/docs/getting_started/downloads.html`](https://emscripten.org/docs/getting_started/downloads.html) — Download and install - Emscripten 7.0.1-git (dev) documentation
- [`/docs/getting_started/FAQ.html`](https://emscripten.org/docs/getting_started/FAQ.html) — FAQ - Emscripten 7.0.1-git (dev) documentation
- [`/docs/getting_started/index.html`](https://emscripten.org/docs/getting_started/index.html) — Getting Started - Emscripten 7.0.1-git (dev) documentation
- [`/docs/getting_started/test-suite.html`](https://emscripten.org/docs/getting_started/test-suite.html) — Emscripten Test Suite - Emscripten 7.0.1-git (dev) documentation
- [`/docs/getting_started/Tutorial.html`](https://emscripten.org/docs/getting_started/Tutorial.html) — Emscripten Tutorial - Emscripten 7.0.1-git (dev) documentation

### `docs/introducing_emscripten/` (6 routes)

- [`/docs/introducing_emscripten/about_emscripten.html`](https://emscripten.org/docs/introducing_emscripten/about_emscripten.html) — About Emscripten - Emscripten 7.0.1-git (dev) documentation
- [`/docs/introducing_emscripten/community.html`](https://emscripten.org/docs/introducing_emscripten/community.html) — Community - Emscripten 7.0.1-git (dev) documentation
- [`/docs/introducing_emscripten/emscripten_license.html`](https://emscripten.org/docs/introducing_emscripten/emscripten_license.html) — Open Source License - Emscripten 7.0.1-git (dev) documentation
- [`/docs/introducing_emscripten/index.html`](https://emscripten.org/docs/introducing_emscripten/index.html) — Introducing Emscripten - Emscripten 7.0.1-git (dev) documentation
- [`/docs/introducing_emscripten/release_notes.html`](https://emscripten.org/docs/introducing_emscripten/release_notes.html) — Release Notes - Emscripten 7.0.1-git (dev) documentation
- [`/docs/introducing_emscripten/Talks-and-Publications.html`](https://emscripten.org/docs/introducing_emscripten/Talks-and-Publications.html) — Talks and Publications - Emscripten 7.0.1-git (dev) documentation

### `docs/optimizing/` (4 routes)

- [`/docs/optimizing/Module-Splitting.html`](https://emscripten.org/docs/optimizing/Module-Splitting.html) — Module Splitting - Emscripten 7.0.1-git (dev) documentation
- [`/docs/optimizing/Optimizing-Code.html`](https://emscripten.org/docs/optimizing/Optimizing-Code.html) — Optimizing Code - Emscripten 7.0.1-git (dev) documentation
- [`/docs/optimizing/Optimizing-WebGL.html`](https://emscripten.org/docs/optimizing/Optimizing-WebGL.html) — Optimizing WebGL - Emscripten 7.0.1-git (dev) documentation
- [`/docs/optimizing/Profiling-Toolchain.html`](https://emscripten.org/docs/optimizing/Profiling-Toolchain.html) — Profiling the Toolchain - Emscripten 7.0.1-git (dev) documentation

### `docs/porting/` (27 routes)

- [`/docs/porting/asyncify.html`](https://emscripten.org/docs/porting/asyncify.html) — Asynchronous Code - Emscripten 7.0.1-git (dev) documentation
- [`/docs/porting/Audio.html`](https://emscripten.org/docs/porting/Audio.html) — Audio - Emscripten 7.0.1-git (dev) documentation
- [`/docs/porting/connecting_cpp_and_javascript/embind.html`](https://emscripten.org/docs/porting/connecting_cpp_and_javascript/embind.html) — Embind - Emscripten 7.0.1-git (dev) documentation
- [`/docs/porting/connecting_cpp_and_javascript/index.html`](https://emscripten.org/docs/porting/connecting_cpp_and_javascript/index.html) — Connecting C++ and JavaScript - Emscripten 7.0.1-git (dev) documentation
- [`/docs/porting/connecting_cpp_and_javascript/Interacting-with-code.html`](https://emscripten.org/docs/porting/connecting_cpp_and_javascript/Interacting-with-code.html) — Interacting with code - Emscripten 7.0.1-git (dev) documentation
- [`/docs/porting/connecting_cpp_and_javascript/WebIDL-Binder.html`](https://emscripten.org/docs/porting/connecting_cpp_and_javascript/WebIDL-Binder.html) — WebIDL Binder - Emscripten 7.0.1-git (dev) documentation
- [`/docs/porting/Debugging.html`](https://emscripten.org/docs/porting/Debugging.html) — Debugging - Emscripten 7.0.1-git (dev) documentation
- [`/docs/porting/emscripten-runtime-environment.html`](https://emscripten.org/docs/porting/emscripten-runtime-environment.html) — Emscripten Runtime Environment - Emscripten 7.0.1-git (dev) documentation
- [`/docs/porting/exceptions.html`](https://emscripten.org/docs/porting/exceptions.html) — C++ Exceptions Support - Emscripten 7.0.1-git (dev) documentation
- [`/docs/porting/files/file_systems_overview.html`](https://emscripten.org/docs/porting/files/file_systems_overview.html) — File System Overview - Emscripten 7.0.1-git (dev) documentation
- [`/docs/porting/files/index.html`](https://emscripten.org/docs/porting/files/index.html) — Files and File Systems - Emscripten 7.0.1-git (dev) documentation
- [`/docs/porting/files/packaging_files.html`](https://emscripten.org/docs/porting/files/packaging_files.html) — Packaging Files - Emscripten 7.0.1-git (dev) documentation
- [`/docs/porting/files/Synchronous-Virtual-XHR-Backed-File-System-Usage.html`](https://emscripten.org/docs/porting/files/Synchronous-Virtual-XHR-Backed-File-System-Usage.html) — Synchronous Virtual XHR Backed File System Usage - Emscripten 7.0.1-git (dev) documentation
- [`/docs/porting/guidelines/api_limitations.html`](https://emscripten.org/docs/porting/guidelines/api_limitations.html) — API Limitations - Emscripten 7.0.1-git (dev) documentation
- [`/docs/porting/guidelines/browser_limitations.html`](https://emscripten.org/docs/porting/guidelines/browser_limitations.html) — Specific Browser Limitations - Emscripten 7.0.1-git (dev) documentation
- [`/docs/porting/guidelines/function_pointer_issues.html`](https://emscripten.org/docs/porting/guidelines/function_pointer_issues.html) — Function Pointer Issues - Emscripten 7.0.1-git (dev) documentation
- [`/docs/porting/guidelines/index.html`](https://emscripten.org/docs/porting/guidelines/index.html) — Code Portability and Limitations - Emscripten 7.0.1-git (dev) documentation
- [`/docs/porting/guidelines/portability_guidelines.html`](https://emscripten.org/docs/porting/guidelines/portability_guidelines.html) — Portability Guidelines - Emscripten 7.0.1-git (dev) documentation
- [`/docs/porting/index.html`](https://emscripten.org/docs/porting/index.html) — Porting - Emscripten 7.0.1-git (dev) documentation
- [`/docs/porting/multimedia_and_graphics/EGL-Support-in-Emscripten.html`](https://emscripten.org/docs/porting/multimedia_and_graphics/EGL-Support-in-Emscripten.html) — EGL Support in Emscripten
- [`/docs/porting/multimedia_and_graphics/index.html`](https://emscripten.org/docs/porting/multimedia_and_graphics/index.html) — Multimedia and Graphics
- [`/docs/porting/multimedia_and_graphics/OpenGL-support.html`](https://emscripten.org/docs/porting/multimedia_and_graphics/OpenGL-support.html) — OpenGL support in Emscripten
- [`/docs/porting/multimedia_and_graphics/WebGPU-support.html`](https://emscripten.org/docs/porting/multimedia_and_graphics/WebGPU-support.html) — Using WebGPU in Emscripten - Emscripten 7.0.1-git (dev) documentation
- [`/docs/porting/networking.html`](https://emscripten.org/docs/porting/networking.html) — Networking
- [`/docs/porting/pthreads.html`](https://emscripten.org/docs/porting/pthreads.html) — Pthreads support - Emscripten 7.0.1-git (dev) documentation
- [`/docs/porting/setjmp-longjmp.html`](https://emscripten.org/docs/porting/setjmp-longjmp.html) — C setjmp-longjmp Support
- [`/docs/porting/simd.html`](https://emscripten.org/docs/porting/simd.html) — Using SIMD with WebAssembly - Emscripten 7.0.1-git (dev) documentation

### `docs/site/` (3 routes)

- [`/docs/site/about.html`](https://emscripten.org/docs/site/about.html) — About this site
- [`/docs/site/glossary.html`](https://emscripten.org/docs/site/glossary.html) — Glossary
- [`/docs/site/index.html`](https://emscripten.org/docs/site/index.html) — Site Links

### `docs/tools_reference/` (5 routes)

- [`/docs/tools_reference/emcc.html`](https://emscripten.org/docs/tools_reference/emcc.html) — Emscripten Compiler Frontend (emcc) - Emscripten 7.0.1-git (dev) documentation
- [`/docs/tools_reference/emcmdprompt.html`](https://emscripten.org/docs/tools_reference/emcmdprompt.html) — Emscripten Windows Command Prompt (emcmdprompt.bat)
- [`/docs/tools_reference/emsdk.html`](https://emscripten.org/docs/tools_reference/emsdk.html) — Emscripten SDK (emsdk) - Emscripten 7.0.1-git (dev) documentation
- [`/docs/tools_reference/index.html`](https://emscripten.org/docs/tools_reference/index.html) — Tools Reference
- [`/docs/tools_reference/settings_reference.html`](https://emscripten.org/docs/tools_reference/settings_reference.html) — Emscripten Compiler Settings - Emscripten 7.0.1-git (dev) documentation

