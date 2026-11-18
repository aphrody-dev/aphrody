# GPUI Starter Pack, Web Standards & Submodule Parity Matrix

> Authoritative cross-stack specification unifying **Core Web Vitals**, **Lighthouse**, **W3C** (HTML5, WCAG 2.2 AA, WAI-ARIA, CharMod, BCP 47, DTCG), **Google Style Guides**, **Vercel Web Interface Guidelines**, and **shadcn/ui + Base UI + Tailwind v4** directly into Aphrody GPUI, NSH, and WebGPU.

## Core Web Vitals & GPUI Frame Budgets

| Metric | Web Budget (Good) | GPUI / WebGPU / Wasm Contract |
| --- | --- | --- |
| LCP / First Surface Paint | `<= 2500 ms` | Non-blocking module bootstrap, zero sync `document.write()` or CSS `@import` |
| INP / Dispatch Latency | `<= 200 ms` | GPUI action dispatch `<= 8 ms` |
| CLS / Layout Stability | `<= 0.1` | Explicit canvas/media sizing & Taffy constraints |
| GPU Frame Cadence | `<= 16.67 ms` (60Hz) / `<= 8.33 ms` (120Hz) | Shared WGSL Naga v30 pipelines + M3 Expressive damped springs |
| Critical Wasm Payload | `<= 358400 B` Brotli | `[profile.web-lto]` + Binaryen `wasm-opt -Oz` cohorts |

## Fork Submodule Parity Matrix

| Stack / Submodule | Patched | Wasm32 | WASI P2 | WGPU v30 | Taffy | AccessKit | M3 + Tailwind | shadcn/Base UI | MCP |
| --- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| Aphrody GPUI Core + Starter Pack | YES | YES | YES | YES | YES | YES | YES | YES | YES |
| WGPU v30 + Naga (aphrody-labs/wgpu) | YES | YES | YES | YES | - | - | - | - | YES |
| Taffy Layout (aphrody-labs/taffy) | YES | YES | YES | - | YES | - | - | - | - |
| AccessKit (aphrody-labs/accesskit) | YES | YES | - | - | - | YES | - | - | YES |
| Cosmic Text (aphrody-labs/cosmic-text) | YES | YES | YES | - | - | - | - | - | - |
| Egui (aphrody-labs/egui) | YES | YES | - | YES | - | YES | - | - | - |
| WRY WebView (aphrody-labs/wry) | - | - | - | - | - | YES | - | - | - |
| Craft Libs (aphrody-labs/craft-libs) | - | YES | YES | - | - | - | - | - | YES |
| Craft Anything (aphrody-labs/craft-anything) | - | YES | - | YES | - | YES | - | - | YES |
| Mullvad VPN (aphrody-labs/mullvadvpn-app) | - | - | - | - | - | - | - | - | YES |
| Aphrody Browser (aphrody-labs/browser) | - | YES | - | YES | - | YES | - | - | YES |

## Deterministic Micro-Benchmarks (Bun Native)

| Benchmark | Iterations | Throughput (ops/sec) | p50 (us) | p95 (us) |
| --- | ---: | ---: | ---: | ---: |
| `design_md_frontmatter_parse` | 200 | 400000 | 2.4 | 4.6 |
| `shadcn_tailwind_v4_css_emit` | 200 | 147558 | 5.6 | 9.5 |
| `cwv_lighthouse_w3c_html_audit` | 200 | 1310616 | 0.4 | 3.1 |
| `submodule_parity_graph_resolve` | 50 | 56491 | 16.5 | 25.1 |

## Monorepo TS -> Rust AI-Agent Readiness Audit

- TypeScript files checked via `Bun.Transpiler`: **66** (syntax errors: 0, explicit `any` warnings: 10)
- Pure-Rust core crate files checked: **56** (`mod.rs` violations: 0, production `.unwrap()`: 0)
- Machine path leaks across TS + Rust: **0**

## Key Source Files

- [Root `DESIGN.md`](../../../DESIGN.md)
- [`assets/os/submodule_parity_matrix.toml`](../../../assets/os/submodule_parity_matrix.toml)
- [`crates/aphrody_design`](../../../crates/aphrody_design/src/aphrody_design.rs)
- [`crates/aphrody_ui`](../../../crates/aphrody_ui/src/aphrody_ui.rs)
- [`crates/ui`](../../../crates/ui/src/ui.rs)
- [`crates/nsh-base`](../../../crates/nsh-base/src/lib.rs)
- [`crates/aphrody_cli`](../../../crates/aphrody_cli/src/main.rs)
- [`tooling/bun/prod-standards-starterpack.ts`](../../../tooling/bun/prod-standards-starterpack.ts)
