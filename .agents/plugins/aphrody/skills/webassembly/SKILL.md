---
name: webassembly
description: Authoritative github.com/orgs/WebAssembly ecosystem reference (Binaryen, WABT, WASI P2, Component Model, Wasm Spec) mapped onto Aphrody LTO/AOT crates and gpui-wgpu CLIs.
---

# `orgs/WebAssembly` Unified Toolchain & Spec Guide

Aphrody mirrors the top 100 `WebAssembly/*` repositories directly into its native Rust/Bun Wasm toolchain ([webassembly_ecosystem.toml](../../../../assets/os/webassembly_ecosystem.toml)).

## Core Workflows

- **LTO Browser Bundles (`[profile.web-lto]`)**: `bun tooling/bun/web-build.ts build --lto` compiles with fat LTO + `codegen-units = 1` and optimizes with Binaryen `wasm-opt -Oz --enable-simd --enable-bulk-memory`.
- **WASI P2 + Wasmtime AOT (`[profile.aot]`)**: Build `--target wasm32-wasip2 --profile aot`, compile to native `.cwasm` via `gpui-wgpu wasm aot`, and run under `wasmtime run --allow-precompiled`.
- **Pure-Rust Inspection & Stripping**: `gpui-wgpu wasm inspect`, `validate`, `strip`, `compose`, `doctor --json`.
