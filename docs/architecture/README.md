# Aphrody Architecture & Production Documentation

> Canonical, Git-versioned architectural specifications, reproducible automation CLIs, and production readiness plans for the `aphrody` monorepo. Every document and script here resolves exclusively within this repository without external scratch state.

## Architecture Guides

- [Production Readiness & Monorepo Hardening Guide](./production-guide.md): Workspace layering (6 tiers), `Cargo.lock` deduplication, Wasm/WebGPU shrinking & reliability, runtime sandboxing, P0/P1/P2 roadmap, testing pyramid, release packaging, and Cloudflare Workers edge operations.
- [Skills, MCP, ACP, A2A & AI-Ready Ecosystem Plan](./skills-mcp-acp-a2a-plan.md): Unifying `os_craft` MCP, `context_server`, `rmcp 3.10.0`, `brontes 0.7.3`, `agent-client-protocol-rmcp 5.3.0`, `a2a-mcp 0.11.3`, WebMCP, WIT plugins, Tauri v3, and AI-Ready pipelines (`crate-to-skill.ts`, `cli -> mcp`, `cli -> skills`, `app -> plugin`, `pkg -> ai-ready`).
- [Code -> Markdown -> Docs / OpenAPI / `llms.txt` -> Website Plan](./code-to-docs-website-plan.md): Replacing legacy `mdbook` / missing `docs_preprocessor` with `cargo xtask docs` IR extraction, Fumadocs v16 + React Router v7 static prerender, Pagefind/Orama hybrid search, `llms.txt`, `/.well-known/agent-skills/index.json`, and Cloudflare Workers Static Assets ($0 Vercel).
- [Cross-Compilation, LLVM & Hermetic Native SDK (`depot_tools`) Guide](./cross-compile-and-depot-tools.md): `cargo-xwin`, `cargo-zigbuild` glibc pinning, LLVM 23/`lld`/`mold`/ThinLTO/PGO/BOLT, WebRTC & CEF prebuilt caches avoiding 35 GB Chromium checkouts.
- [24/7 Autonomous Dev, Worktree, CI/CD, Channels & Download Architecture](./autonomous-release-ci-cd.md): Out-of-tree isolated worktrees, Merge Queue + GitHub App auto-merge, 6-tier channels (`lts`/`stable`/`preview`/`canary`/`nightly`/`dev`), Sigstore attestations, SHA-256 `auto_update`, and Cloudflare R2 download portal.

## Reproducible Tooling CLI (`package.json`)

- [`tooling/bun/submodules.ts`](../../tooling/bun/submodules.ts): Parallel submodule inspection and `--depth 1 --jobs 8` sync driven by [`.gitmodules`](../../.gitmodules).
- [`tooling/bun/worktree.ts`](../../tooling/bun/worktree.ts): Out-of-tree parallel Git worktree & isolated `CARGO_TARGET_DIR` manager preserving a clean `main` checkout.
- [`tooling/bun/cross-build.ts`](../../tooling/bun/cross-build.ts): Cross-compilation doctor and runner across `cargo-xwin`, `cargo-zigbuild`, `llvm`/`clang`/`lld`, `mold`, and `wasm32`.
- [`tooling/bun/web-build.ts`](../../tooling/bun/web-build.ts): Unified multi-cohort Wasm + WebGPU builder/auditor across all 8 Rust UI/rendering frameworks.
- [`tooling/bun/web-verify.ts`](../../tooling/bun/web-verify.ts): Headless Chrome/Edge CDP DOM/ARIA/WebGPU verifier saving screenshots under gitignored `target/web-screenshots/`.
- [`tooling/bun/crate-to-skill.ts`](../../tooling/bun/crate-to-skill.ts): Converts any workspace or `docs.rs` Rust crate into a portable `SKILL.md` + `references/` bundle.
- [`tooling/bun/fleet-sync.ts`](../../tooling/bun/fleet-sync.ts): SSH fleet delta sync, remote build/test execution, and binary screenshot retrieval into gitignored `target/os_screenshots/`.
- [`tooling/bun/repo-hygiene.ts`](../../tooling/bun/repo-hygiene.ts): Enforces `README.md` guard, zero ephemeral non-repo path leaks, relative doc link integrity, and manifest validity.
