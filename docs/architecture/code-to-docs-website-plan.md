# Code -> Markdown -> Docs / OpenAPI / `llms.txt` -> Website Automation Plan

> Blueprint for transforming Aphrody Rust crates, `schemars` JSON Schemas, WIT interfaces, Tree-sitter grammars, `.nsh/types` declarations, CLI manuals, and `assets/os/*` manifests into deterministic Markdown, OpenAPI 3.1, Agent Skills, `llms.txt` feeds, and a Cloudflare-hosted static website ($0 Vercel).

---

## 1. Diagnosis of Current Assets & Broken Legacy Residue

### Broken Upstream Residue to Retire
- [`docs/book.toml`](../../docs/book.toml) invokes `cargo run -p docs_preprocessor --` and expects `crates/docs_preprocessor/actions.json`, **which does not exist** in this workspace.
- [`tooling/xtask/src/tasks/workflows/deploy_docs.rs`](../../tooling/xtask/src/tasks/workflows/deploy_docs.rs) and [`.cloudflare/`](../../.cloudflare/docs-proxy/wrangler.toml) still reference `mdbook 0.4.37`, `zed-industries/zed`, and `zed.dev` zones.

### Ready Internal Sources to Wire Directly
- **Brand & URLs**: [`product.toml`](../../product.toml) (`https://aphrody.com`, `https://docs.aphrody.com`, `brand check`).
- **Rust & Schema Truth**: [`crates/schema_generator`](../../crates/schema_generator/Cargo.toml), [`crates/json_schema_store`](../../crates/json_schema_store/Cargo.toml) (`schemars 1.0.4`), `zed --dump-all-actions`, [`crates/extension_api/wit/since_v0.8.0/`](../../crates/extension_api/wit/since_v0.8.0/extension.wit) (11 WIT interfaces), [`crates/grammars/`](../../crates/grammars/Cargo.toml) (`tree-sitter`), [`crates/os_craft/src/mcp.rs`](../../crates/os_craft/src/mcp.rs) (`builtin_tools()`), and [`assets/os/`](../../assets/os/craft_catalog.toml).
- **TypeScript & Skill Generators**: [`crates/nsh-js/src/declarations.rs`](../../crates/nsh-js/src/declarations.rs) (parity-tested `.nsh/types/*.d.ts`), [`tooling/bun/crate-to-skill.ts`](../../tooling/bun/crate-to-skill.ts) (`cargo-doc-md` / `rustdoc-md`), [`tooling/bun/web-build.ts`](../../tooling/bun/web-build.ts) (live WebGPU demo bundles), and [`.cloudflare/docs-proxy/src/worker.js`](../../.cloudflare/docs-proxy/src/worker.js) (`Accept: text/markdown` worker logic).

---

## 2. Recommended Toolchain & Website Framework ($0 Vercel, Bun-First)

| Pipeline Stage | Selected Stack (2026 Verified) | Role |
| :--- | :--- | :--- |
| **Rust API -> MD** | `cargo rustdoc` (`rustc 1.98.1` + `RUSTC_BOOTSTRAP=1`) + `cargo-doc-md 0.7.3` / `rustdoc-types 0.61.0` + `cargo-public-api 0.52.0` | Generate Tier-1 multi-page MDX + changelog API diffs without parsing all 301 crates on every PR; Tier-2 generates manifest/README index pages. |
| **Rust OpenAPI & WIT** | `schemars 1.0.4` + `aide 0.15.1` (`axum 0.8.9`), `wit-parser 0.256.0` | Derive OpenAPI 3.1 directly from `schemars` types (`cloud_api_types`) and convert all 11 WIT files into API Markdown. |
| **Rust -> TS & Grammar Highlight** | `wasm-bindgen 0.2.129` `.d.ts`, `ts-rs 12.0.1`, `tree-sitter-highlight 0.27.2` | Project Rust NSH/OS/Web types into TS cleanly and render custom DSL/WGSL/RON/WIT syntax blocks. |
| **TS Docs & Twoslash** | `typedoc 0.28.20` + `typedoc-plugin-markdown 4.14.1` + `ts-morph 28.0.0`, `twoslash 0.3.9` + `shiki 4.6.0` | Turn `.nsh/types/*.d.ts` + `wasm-bindgen` `.d.ts` into API docs and type-check every TSX snippet at docs build time. |
| **Static Site Framework** | **Fumadocs v16** (`fumadocs-core 16.18.1`, `ui 16.19.3`, `mdx 15.6.2`, `openapi 12.5.5`, `twoslash 4.0.4`) + **React Router v7 (`ssr: false`, `prerender`)** + **Orama `3.1.18` / Pagefind `1.5.2`** | Pure static `dist/client/` ($0 Vercel, zero Next.js lock-in, Bun-fast), native `llms.txt` / per-page `.md` generation, Scalar `@scalar/api-reference 1.75.1` / `@scalar/openapi-to-markdown 1.7.4`, and interactive WebGPU canvas mounts. |

---

## 3. Automated DAG Workflow (`cargo xtask docs` -> `docs-pipeline.ts` -> Edge)

```text
Stage 0: Sources (product.toml · Cargo.toml · docs/src · wit/ · grammars/ · nsh-js · assets/os)
   │
   ├──▶ Stage 1: `cargo xtask docs` -> `target/docs-ir/`
   │       actions.json, schemas/*.json, wit/*.md, nsh-types/*.d.ts, grammars.json, os-catalog.json, Tier-1 rustdoc MD
   │
   ├──▶ Stage 2: `tooling/bun/docs-pipeline.ts [--check]`
   │       Hydrate {#kb}/{#action} tags, rewrite stale URLs via product.toml, run TypeDoc + Scalar OpenAPI -> MD,
   │       package Tier-1 SKILL.md + CLI skills + /.well-known/agent-skills/index.json (0.2.0) + context7.json
   │
   ├──▶ Stage 3: Static Site Prerender (`apps/docs/`)
   │       Fumadocs Twoslash check, static prerender -> dist/client/, Orama + Pagefind index,
   │       /llms.txt, scoped */llms-full.txt, per-page .md mirrors, /openapi.json
   │
   └──▶ Stage 4: Verification & Cloudflare Workers Static Assets Deploy (`wrangler deploy`)
           --check drift gate + lychee + Playwright/Chrome check + Worker `Accept: text/markdown` + R2 assets.aphrody.com
```

### Target Agent Discovery Contracts
1. **`/llms.txt` & Scoped `*/llms-full.txt`**: `llmstxt.org` v2 index linking Core Guides, GPUI SDK, NSH, OS, WGPU (`docs/wgpu/llms.txt`), WIT Extensions, Schemas, OpenAPI, and Skills; keep subsystem `llms-full.txt` modular to avoid multi-MB root dumps.
2. **HTTP `Accept: text/markdown` + `<link rel="alternate" type="text/markdown">`**: Clean Markdown twin rendered beside every route and negotiated at the Cloudflare Worker edge with `Vary: Accept`.
3. **`/.well-known/agent-skills/index.json` (`0.2.0`) & `/context7.json`**: Deterministic `.zip` skill archives with `sha256:` digests matching `discovery/0.2.0` schema plus `context7.json` indexing rules excluding `vendor/` and `target/`.

### Verification Gates
- `cargo xtask brand check` (zero stale upstream branding)
- `bun tooling/bun/repo-hygiene.ts` & `bun tooling/bun/docs-pipeline.ts --check` (zero drift)
- Static site Twoslash build + link check (`lychee`) + edge contract test (`Accept: text/markdown`, `index.json` SHA-256 verification, headless Chromium check)
