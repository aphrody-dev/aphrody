# Awesome Wasm, WebGPU & GPUI Unified Toolchain

> Complete LTO / AOT WebAssembly (`wasm32-unknown-unknown`, `wasm32-wasip2`) and WGPU / Naga v30 ecosystem guide anchored to [`https://design.google/`](https://design.google/), [`DESIGN.md`](../../DESIGN.md), and [`crates/aphrody_ui/src/starter_pack.rs`](../../crates/aphrody_ui/src/starter_pack.rs).

## Cargo Profiles (`[profile.web-lto]` & `[profile.aot]`)

- `[profile.web-lto]` (`Cargo.toml`): `inherits = "web"`, `opt-level = "z"`, `lto = "fat"`, `codegen-units = 1`, `strip = "debuginfo"` — used cleanly with `bun tooling/bun/web-build.ts build --lto` and Binaryen `wasm-opt -Oz`.
- `[profile.aot]` (`Cargo.toml`): `inherits = "release"`, `opt-level = 3`, `lto = "fat"`, `codegen-units = 1` — paired with `wasmtime compile` (`.cwasm`) for zero-startup host execution in [`crates/extension_host`](../../crates/extension_host/src/wasm_host.rs) and `gpui-wgpu wasm aot`.

## Unified CLI & 24-Tool MCP Server (`crates/aphrody_cli`)

- `gpui-wgpu wasm inspect|validate|optimize|aot|bindgen|compose|strip`: Binary inspection (`wasmparser` + gzip/SHA-256), strict validation, `wasm-opt -Oz`, Wasmtime AOT `.cwasm`, lockstep `wasm-bindgen`, component composition (`wac`), and pure-Rust section stripping (`wasm-encoder`).
- `gpui-wgpu doctor`: Probes installed `wasm32` Rust targets, Binaryen/Wasmtime/SH/DXC/SPIR-V tools, and active GPU adapters.
- `gpui-wgpu app list|new|build|tokens`: Scaffolds and builds the 5 standalone starter templates under [`assets/os/awesome_wasm_wgpu.toml`](../../assets/os/awesome_wasm_wgpu.toml).
- `gpui-wgpu design mirror|sitemap|design-md|stitch|web-guidance|lighthouse|parity-matrix`: Live pure-Rust `https://design.google/` mirror, Stitch TSX/PNG synthesis, Tailwind v4 `@theme` / `components.json` export, CWV + 4-category Lighthouse scoring, and fork submodule parity matrix queries.
- `gpui-wgpu mcp`: 24-tool stdio JSON-RPC 2.0 MCP server exposing WGPU/Naga, Wasm LTO/AOT, `DESIGN.md`, Stitch capture, Core Web Vitals/Lighthouse, shadcn/Tailwind, GPUI Starter Pack catalog, and submodule parity queries.

## Standalone Templates (`templates/wasm-wgpu/*`)

- `wgpu-compute-particles`: WGSL compute + fullscreen M3 Expressive particle shader (`shaders/particles.wgsl`)
- `gpui-web-studio`: Browser WebGPU + GPUI studio viewport (`shaders/studio_surface.wgsl`)
- `nsh-webgpu-shader`: NSH reactive springs + procedural WGSL wave shader (`shaders/expressive_wave.wgsl`)
- `wasi-p2-aot-compute`: `wasm32-wasip2` AOT-ready deterministic WCAG contrast compute kernel
- `gpui-shadcn-starterpack`: CWV `>= 95`, Lighthouse, W3C WCAG AA, Vercel UI & shadcn/Base UI/Tailwind v4 starter pack (`components.json`, `styles/shadcn-tailwind.css`)

## Related Guides

- [`docs/architecture/webgpu-wasm-first.md`](./webgpu-wasm-first.md)
- [`docs/architecture/webassembly-org-ecosystem.md`](./webassembly-org-ecosystem.md)
- [`docs/architecture/google-design-system.md`](./google-design-system.md)
- [`docs/architecture/gpui-starterpack-and-parity-matrix.md`](./gpui-starterpack-and-parity-matrix.md)
