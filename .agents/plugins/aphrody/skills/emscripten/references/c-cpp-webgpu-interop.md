# C/C++, JavaScript & WebGPU Interop (Emscripten 7.0.x & Aphrody)

> Covers Dawn's `emdawnwebgpu` port (`--use-port=emdawnwebgpu`), `EM_JS`/`EM_ASYNC_JS`, Embind, `ccall`/`cwrap`, Clang/LLVM 23 vs LLVM 24 linking, and Aphrody `gpui_web`/`wgpu`/`nsh-core` integration.

---

## 1. WebGPU in Emscripten: `--use-port=emdawnwebgpu`

Upstream Emscripten removed legacy `-sUSE_WEBGPU` in favor of **Emdawnwebgpu** (maintained in Google Dawn, implementing `<webgpu/webgpu.h>` and `<webgpu/webgpu_cpp.h>`, currently pinned at `v20261105.214757` in `tools/ports/emdawnwebgpu.py` for Emscripten 4.0.10+).

### Compile & Link Invocation

```sh
# Pass --use-port=emdawnwebgpu during BOTH compile and link; always add --closure=1 in release links!
em++ -std=c++20 -O3 -flto --use-port=emdawnwebgpu -c webgpu_renderer.cpp -o webgpu_renderer.o
em++ -O3 -flto --use-port=emdawnwebgpu --closure=1 \
  -sSTRICT -sEXPORT_ES6 -sMODULARIZE -sJSPI -sALLOW_MEMORY_GROWTH \
  webgpu_renderer.o -o dist/webgpu_renderer.mjs
```

- **Disable C++ wrappers if needed**: `--use-port=emdawnwebgpu:cpp_bindings=false` when supplying custom/pinned `webgpu_cpp.h`.
- **Pre-warm port cache**: `embuilder build emdawnwebgpu` (useful in CI Docker layers).
- **Local/offline port**: Extract `emdawnwebgpu_pkg-*.zip` from `google/dawn/releases` and pass `--use-port=path/to/emdawnwebgpu.port.py`.

### Minimal `<webgpu/webgpu_cpp.h>` Canvas Surface Setup

```cpp
#include <emscripten/emscripten.h>
#include <webgpu/webgpu_cpp.h>

static wgpu::Instance instance = wgpu::CreateInstance();

void init_surface(wgpu::Device device, const char* canvas_selector) {
  wgpu::EmscriptenSurfaceSourceCanvasHTMLSelector canvasDesc{};
  canvasDesc.selector = canvas_selector; // e.g. "#canvas"

  wgpu::SurfaceDescriptor surfDesc{};
  surfDesc.nextInChain = &canvasDesc;
  wgpu::Surface surface = instance.CreateSurface(&surfDesc);

  wgpu::SurfaceConfiguration cfg{};
  cfg.device = device;
  cfg.format = wgpu::TextureFormat::BGRA8Unorm;
  cfg.usage = wgpu::TextureUsages::RenderAttachment;
  cfg.width = 1280;
  cfg.height = 800;
  cfg.presentMode = wgpu::PresentMode::Fifo;
  surface.Configure(&cfg);
}
```

> **Aphrody alignment**: Keep WebGPU surface creation, 256-byte row alignment (`COPY_BYTES_PER_ROW_ALIGNMENT`), and 16-byte uniform struct padding identical to [`.agents/plugins/aphrody/skills/webgpu/SKILL.md`](../../webgpu/SKILL.md).

---

## 2. C/C++ <-> JavaScript Interop Patterns

### A. `EM_JS` & `EM_ASYNC_JS` (Typed Zero-Boilerplate JS Calls)

```cpp
#include <emscripten/emscripten.h>
#include <stdint.h>

// Preferred over EM_ASM: explicit C signature, safe with --closure=1 and separate compilation.
EM_JS(void, aphrody_submit_telemetry, (const char* tag_ptr, double ms), {
  const tag = UTF8ToString(tag_ptr);
  globalThis.__aphrodyMetrics?.record(tag, ms);
});

// With -sJSPI (recommended) or -sASYNCIFY, pause C/C++ cleanly across a JS Promise:
EM_ASYNC_JS(int, aphrody_has_webgpu_adapter, (), {
  if (!navigator.gpu) return 0;
  const adapter = await navigator.gpu.requestAdapter();
  return adapter ? 1 : 0;
});
```

### B. Embind (`<emscripten/bind.h>`) & Bulk Zero-Copy Buffer Passing

```cpp
#include <emscripten/bind.h>
#include <emscripten/val.h>
#include <vector>

struct FrameStats {
  uint32_t drawCalls;
  double frameMs;
};

void upload_vertices(uintptr_t ptr, size_t byte_len) {
  const auto* bytes = reinterpret_cast<const uint8_t*>(ptr);
  (void)bytes; (void)byte_len;
}

emscripten::val get_heap_slice(const std::vector<float>& buf) {
  // Zero-copy TypedArray view directly into Wasm linear memory (valid until memory growth!)
  return emscripten::val(emscripten::typed_memory_view(buf.size(), buf.data()));
}

EMSCRIPTEN_BINDINGS(aphrody_bridge) {
  emscripten::value_object<FrameStats>("FrameStats")
      .field("drawCalls", &FrameStats::drawCalls)
      .field("frameMs", &FrameStats::frameMs);
  emscripten::function("uploadVertices", &upload_vertices);
  emscripten::function("getHeapSlice", &get_heap_slice);
}
```

- Link with `-lembind`. In Emscripten **7.0.1+**, invokers are generated at link time (no runtime `eval`/`new Function`), preserving strict CSP (`-sDYNAMIC_EXECUTION=0`) compatibility!
- **Heap Safety Rule**: Never retain a `typed_memory_view` across calls that allocate (`_malloc`) when `-sALLOW_MEMORY_GROWTH` is enabled, because `memory.grow` detaches the underlying `ArrayBuffer`.

### C. `ccall` / `cwrap` for C Exports

```sh
emcc -O3 api.c -sEXPORTED_FUNCTIONS=_compute_hash,_malloc,_free -sEXPORTED_RUNTIME_METHODS=ccall,cwrap -o api.mjs
```

---

## 3. C/C++ Wasm Linking, Clang/LLVM 23 & Aphrody Crates

- **Rust <-> C/Emscripten in Aphrody**:
  - `crates/nsh-core/Cargo.toml` explicitly treats Emscripten as both `wasm32` and `unix` (`cfg(any(not(target_arch = "wasm32"), target_os = "emscripten"))` pulling `libc`).
  - `vendor/wgpu/wgpu-core/platform-deps/emscripten/Cargo.toml` provides `wgpu-core-deps-emscripten` forwarding `wgpu-hal/gles` for `target_os = "emscripten"`.
- **Host Clang (clang version 23.1.5 (https://github.com/llvm/llvm-project 392dfff97d29b59d77e5cac6d4b6ef2585fe7117)) vs Emscripten `emsdk` (LLVM 24)**:
  - Host `clang --target=wasm32-unknown-unknown -nostdlib -O3` builds freestanding C kernels directly via `lld` (`wasm-ld`) with zero `emsdk` dependency.
  - For `wasm32-unknown-emscripten`, **always link with `emcc` using `emsdk`'s matched LLVM 24 + `musl` sysroot + JS runtime libraries** rather than mixing LLVM 23 LTO bitcode with LLVM 24 `wasm-ld`.

---

## 4. Bun + `xh` Local Serving & COOP/COEP Verification

```ts
// Minimal Bun dev server verifying COOP/COEP + .wasm MIME + xh header checks
Bun.serve({
  hostname: "127.0.0.1",
  port: 4196,
  async fetch(req) {
    const path = new URL(req.url).pathname === "/" ? "/index.html" : new URL(req.url).pathname;
    const file = Bun.file(`./dist${path}`);
    if (!(await file.exists())) return new Response("Not Found", { status: 404 });
    return new Response(file, {
      headers: {
        "Cross-Origin-Opener-Policy": "same-origin",
        "Cross-Origin-Embedder-Policy": "require-corp",
        ...(path.endsWith(".wasm") ? { "Content-Type": "application/wasm" } : {}),
      },
    });
  },
});
```

```sh
xh --ignore-stdin --check-status --headers GET http://127.0.0.1:4196/engine.wasm
```
