# Requirements

## Scope

| In | Out (proposed) |
|---|---|
| The `open` monorepo: absorbed sources, lock, Bun and Cargo workspaces | Areas the rest of Aphrody keeps |
| AI router: library, C ABI, service, surfaces | Model training |
| Official Material 3 UI for the router surfaces | Home-made design |
| Reference archive of design.google + m3.material.io | Public republication of Google content |

## Requirements

### Monorepo
- R1. Every source has its entry in `open.lock.json` (url, rev, license, paths).
- R2. `bun install` at the root produces a `bun.lock`; `bun test` passes per package.
- R3. Rust crates build in release with fat LTO; Bun CLIs compile AOT.
- R4. The repository clones on Windows, Linux and macOS (no invalid path). Done.

### Router
- R5. OpenAI-compatible API (`/v1/chat/completions`, `/v1/models`) and Anthropic (`/v1/messages`).
- R6. Catalog discovered at run time; no hardcoded model list.
- R7. Default policy: local first, then free, automatic failover with a trace of attempts.
- R8. Stable C ABI; HTTPS through the OS TLS stack or a transport supplied by the platform.
- R9. Surfaces: CLI, full-screen TUI, desktop, mobile, serverless, self-hosted.
- R9a. Shipped as both an embedded library and a service (D2).
- R9b. Local first; every inference type and every cloud provider supported (D8).
- R9c. The C ABI is consumed by Angular, Flutter, Compose, Rust, Zig, .NET and C++ without rewriting the core.

State of R5 to R9c: `crates/ai-router` serves `/v1/chat/completions`, `/v1/models` and `/v1/messages` (R5); offers come from the models.dev catalog and the local OpenAI-compatible servers found on loopback, with credentials read from the variables the catalog names (R6); policies `auto`, `auto:local`, `auto:free`, `auto:cheap`, failover with an attempt trace (R7); C ABI in the shared `aphrody_*` ABI, HTTPS through rustls (R8, platform transport not yet); CLI, service, full-screen terminal, Angular Material app for desktop and mobile web, container image for serverless and self-hosted platforms (R9; native mobile not yet); library and service (R9a); text chat only (R9b: other inference types not yet); Bun binding (R9c: other languages not yet). "Free and unlimited": zero-cost offers first and local models without a quota; no remote free offer is unlimited.

### UI
- R10. Google's official components and tokens only; Aphrody identity through official theming.
- R10a. Maximum coverage: Angular (fork), Flutter (desktop, mobile, web), Compose Multiplatform, a Rust framework at the same level (D3, D4).
- R10b. Angular fork following upstream automatically, Bun patches, AOT and LTO compilation.
- R11. Light, dark, medium and high contrast; M3 accessibility (48 dp targets, contrast, focus).

### Reference archive
- R12. Every page of both sites rendered in a real browser: screenshot, DOM, network, WACZ archive.
- R13. robots.txt respected; internal use.

### Toolchain and gates
- R14. Toolchain: Bazel everywhere (D12), calling just, Turborepo, Bun, Cargo, LLVM.
- R15. Validation gates run as a Bun and Cargo plugin before any compilation, by level like Bun's commands (D13). Details: [toolchain-and-gates.md](toolchain-and-gates.md).
- R16. Core Web Vitals (Lighthouse, web-vitals) measured on the rendering, blocking before release.

### Language
- R22. English at the source everywhere; every user-facing string goes through i18n; translations produced by the translation stack.

## Definition of done (every deliverable)

Built without error, running, tested (automated tests green), proven by a screenshot or real output, secret scan clean, Gemini Carbon High review archived.
