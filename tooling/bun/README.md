# Aphrody Reproducible Tooling (`cargo xtask` + `tooling/bun/cli.ts`)

Aligned with `zed-industries/zed`'s `tooling/xtask` architecture:

1. **Primary Rust Entrypoint (`cargo xtask`)**:
   - `cargo xtask clippy` / `./script/clippy`
   - `cargo xtask package-conformity`
   - `cargo xtask licenses`
   - `cargo xtask brand check|apply`
   - `cargo xtask new-app <name> --template <minimal|code|studio|nsh-studio|craft-studio>`
   - `cargo xtask web-examples`
   - `cargo xtask studio <status|check|parity|session>` / `./script/studio`

2. **Unified Bun Domain Router ([`cli.ts`](./cli.ts))**:
   Consolidates every TypeScript/Web/Design/Storytold/Fleet utility behind one typed dispatcher:
   - `bun tooling/bun/cli.ts list [--json]`
   - `bun tooling/bun/cli.ts check [--json]`
   - `bun tooling/bun/cli.ts workspace <hygiene|vendor-doctor|i18n|standards|agy-audit>`
     (upstream forks are pinned Cargo git dependencies, `git` + `tag`, recorded in [`forks-manifest.json`](./forks-manifest.json); there are no submodules and no `vendor/` checkouts)
   - `bun tooling/bun/cli.ts design <sync|audit|pack>`
   - `bun tooling/bun/cli.ts storytold <sync|audit>`
   - `bun tooling/bun/cli.ts web <matrix|check|build|verify|cross|issues>`
   - `bun tooling/bun/cli.ts session <serve|snapshot|export|verify>`
   - `bun tooling/bun/cli.ts ingest <store|longbridge|wasm-bindgen|emscripten|webassembly>`
   - `bun tooling/bun/cli.ts fleet <sync|verify|host-audit|toolchain>`
