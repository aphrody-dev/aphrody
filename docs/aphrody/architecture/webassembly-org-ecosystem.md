# `orgs/WebAssembly` Ecosystem in Aphrody

Synced snapshot of [100 repositories](https://github.com/orgs/WebAssembly/repositories?q=sort%3Astars) sorted descending by stars and mapped onto Aphrody's native Wasm/WebGPU LTO + AOT stack.

| Repository | Stars | Category | Aphrody Integration |
| --- | ---: | --- | --- |
| [design](https://github.com/WebAssembly/design) | 11647 | `spec-and-testsuite` | wasmparser::Validator::new_with_features(WasmFeatures::all()) in crates/aphrody_cli |
| [binaryen](https://github.com/WebAssembly/binaryen) | 8680 | `toolchain-optimizer` | gpui-wgpu wasm optimize (-Oz + simd + bulk-memory) |
| [wabt](https://github.com/WebAssembly/wabt) | 8208 | `toolchain-optimizer` | wat2wasm / wasm2wat verified in shell-path-doctor & gpui-wgpu doctor |
| [WASI](https://github.com/WebAssembly/WASI) | 5848 | `component-model-wasi` | crates/extension_host (.cwasm AOT cache) & wasm32-wasip2 templates |
| [spec](https://github.com/WebAssembly/spec) | 3472 | `spec-and-testsuite` | wasmparser::Validator::new_with_features(WasmFeatures::all()) in crates/aphrody_cli |
| [wasi-sdk](https://github.com/WebAssembly/wasi-sdk) | 1664 | `toolchain-optimizer` | templates/wasm-wgpu/wasi-p2-aot-compute & emscripten skill |
| [component-model](https://github.com/WebAssembly/component-model) | 1389 | `component-model-wasi` | crates/extension_host (.cwasm AOT cache) & wasm32-wasip2 templates |
| [proposals](https://github.com/WebAssembly/proposals) | 1142 | `proposal` | tooling/bun/web-build.ts & crates/gpui_web |
| [gc](https://github.com/WebAssembly/gc) | 1056 | `spec-and-testsuite` | wasmparser::Validator::new_with_features(WasmFeatures::all()) in crates/aphrody_cli |
| [wasi-libc](https://github.com/WebAssembly/wasi-libc) | 1046 | `toolchain-optimizer` | templates/wasm-wgpu/wasi-p2-aot-compute & emscripten skill |
| [threads](https://github.com/WebAssembly/threads) | 773 | `runtime-and-api` | tooling/bun/web-build.ts & crates/gpui_web |
| [interface-types](https://github.com/WebAssembly/interface-types) | 643 | `component-model-wasi` | tooling/bun/web-build.ts & crates/gpui_web |
| [wasm-c-api](https://github.com/WebAssembly/wasm-c-api) | 612 | `runtime-and-api` | tooling/bun/web-build.ts & crates/gpui_web |
| [wasi-nn](https://github.com/WebAssembly/wasi-nn) | 562 | `component-model-wasi` | tooling/bun/web-build.ts & crates/gpui_web |
| [simd](https://github.com/WebAssembly/simd) | 557 | `spec-and-testsuite` | wasmparser::Validator::new_with_features(WasmFeatures::all()) in crates/aphrody_cli |
| [meetings](https://github.com/WebAssembly/meetings) | 548 | `proposal` | tooling/bun/web-build.ts & crates/gpui_web |
| [esm-integration](https://github.com/WebAssembly/esm-integration) | 457 | `runtime-and-api` | tooling/bun/web-build.ts & crates/gpui_web |
| [tool-conventions](https://github.com/WebAssembly/tool-conventions) | 374 | `toolchain-optimizer` | tooling/bun/web-build.ts & crates/gpui_web |
| [wasm-jit-prototype](https://github.com/WebAssembly/wasm-jit-prototype) | 314 | `proposal` | tooling/bun/web-build.ts & crates/gpui_web |
| [wasi-sockets](https://github.com/WebAssembly/wasi-sockets) | 296 | `component-model-wasi` | tooling/bun/web-build.ts & crates/gpui_web |
| [website](https://github.com/WebAssembly/website) | 289 | `proposal` | tooling/bun/web-build.ts & crates/gpui_web |
| [memory64](https://github.com/WebAssembly/memory64) | 270 | `component-model-wasi` | tooling/bun/web-build.ts & crates/gpui_web |
| [testsuite](https://github.com/WebAssembly/testsuite) | 249 | `spec-and-testsuite` | wasmparser::Validator::new_with_features(WasmFeatures::all()) in crates/aphrody_cli |
| [wasi-filesystem](https://github.com/WebAssembly/wasi-filesystem) | 236 | `component-model-wasi` | tooling/bun/web-build.ts & crates/gpui_web |
| [wasi-http](https://github.com/WebAssembly/wasi-http) | 223 | `component-model-wasi` | tooling/bun/web-build.ts & crates/gpui_web |

- Registry manifest: [webassembly_ecosystem.toml](../../../assets/os/webassembly_ecosystem.toml)
- Compact agent index: [webassembly-org-llms.txt](./webassembly-org-llms.txt)
- Toolchain skill: [.agents/plugins/aphrody/skills/webassembly/SKILL.md](../../../.agents/plugins/aphrody/skills/webassembly/SKILL.md)
