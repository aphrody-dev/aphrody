# Phases

Each phase ends according to the definition of done in [spec.md](spec.md).

| Phase | Deliverable | Depends on | Status |
|---|---|---|---|
| 0 | Decisions taken; spec and design system agreed | — | D2, D3, D4, D8, D12, D13, D14, D18 taken; D1, D5, D6, D7, D9 to D11, D15 to D17 open |
| 1 | Healthy monorepo: cross-platform clone, `bun.lock`, Bun AOT and Rust LTO builds green | D1 | Windows clone fixed (310 files dropped); `bun.lock` and builds remain |
| 1b | Toolchain (Bazel + just, Turborepo, Bun, Cargo, LLVM); validation gates as Bun and Cargo plugins | D12, D13 | to start |
| 2 | Angular forks (angular, components, angular-cli) following upstream, Bun patches, AOT and LTO | D3 | forks created in aphrody-labs; follow automation and patches to do |
| 3 | Common C ABI core and AI router (library + service, local first) | D2, D8 to D11 | in progress: `crates/ai-router` (library, C ABI `include/ai_router.h`, CLI, OpenAI- and Anthropic-compatible service), `packages/ai-router` (Bun binding), CI on Linux, Windows and macOS |
| 4 | Aphrody theme on Angular Material, Flutter material, Compose material3, Rust framework | D4, D7 | to start |
| 5 | Router surfaces on every target (web, Android, iOS, desktop, terminal, serverless, self-hosted) | phases 2 to 4, D5 | to start |
| 6 | Reference archive of design.google + m3.material.io | D6 | to start |
| 7 | Inventories (frameworks, Aphrody monorepo) | — | done (842 projects; Aphrody 521 third party, 216 crates, 51 packages, 1462 docs) |
| 8 | Tools installed on the PC and the VPS, verified | — | PC done (9 tools); VPS to do; Lightpanda Windows via R21 |
| 9 | Benchmark: shared resumable results store, multi-host, Cloudflare | D15, D17 | to start |
| 10 | Browser ↔ UI loop: generation from a sitemap, pixel comparison, iteration | D16 | to start |
| G | Goal deliverable: Bun fullstack × Material 3 ([goal-bun-m3.md](goal-bun-m3.md)) | — | skills, API surface, theme, Figma kit done; app, AOT build, coverage remain |
| L | English at the source and i18n everywhere (R22) | — | docs translated; i18n stack to wire |
