# Contributing to Aphrody (`aphrody`)

Thank you for contributing to **Aphrody** (`aphrody-dev/aphrody`), unifying GPUI, Native Shell (`crates/nsh-*`), `gpui-kit`, WGPU/Naga v30 (`vendor/wgpu`, `crates/gpui_wgpu`, `crates/wgpu_cli`), and Bun (`vendor/bun`) with full Zed ecosystem compatibility.

## Before You Start

- **Use Discussions for architecture proposals**: Open a thread in [aphrody Discussions](https://github.com/aphrody-dev/aphrody/discussions) or check existing [Issues](https://github.com/aphrody-dev/aphrody/issues) before large cross-crate refactors.
- **Automated upstream syncs**: Synchronization with `zed-industries/zed`, `longbridge/nsh`, `longbridge/gpui-kit`, `gfx-rs/wgpu`, and `aphrody-labs/bun` is managed through [`.github/workflows/aphrody-upstream-sync.yml`](./.github/workflows/aphrody-upstream-sync.yml).
- **Plugin, coding rules & design guides**: Read [`.agents/plugins/aphrody/rules/AGENTS.md`](./.agents/plugins/aphrody/rules/AGENTS.md) and [`.agents/plugins/aphrody/docs/INDEX.md`](./.agents/plugins/aphrody/docs/INDEX.md).

## Engineering Standards

1. **No `mod.rs`**: Keep crate roots in `src/<crate_name>.rs` and submodules in `src/<module>.rs`.
2. **Explicit Error Handling**: Avoid `.unwrap()`, `.expect()` (outside tests), and `let _ = <fallible>` in core crates.
3. **Brand Consistency**: Never hardcode product strings (`Zed`/`Aphrody`); read `brand::*` or run `cargo xtask brand check` (and `cargo xtask brand apply` after editing `product.toml` or icons).
4. **Bun for TypeScript/TSX & Hooks**: Use `bun` (`bun test`, `bun check`, `bun fmt`, `bun build`), never `node`, `npm`, or `npx`.
5. **GPU & Shader Verification**: Validate WGSL with `gpui-wgpu validate` and verify backend changes across Vulkan, DirectX 12, and WebGPU (`examples/gpu/`).
6. **Modular Boundaries**:
   - Keep `gpui_ui` free of `settings_content`, `cloud_llm_client`, and `language_model_core`.
   - Keep `gpui_code --no-default-features` free of `buffer_diff`, `language`, `rpc`, and `proto`.

## Verification Checklist

Follow the [pull request template](./.github/pull_request_template.md?plain=1) and run relevant checks before pushing:

```sh
cargo xtask brand check
cargo test -j 4 -p brand -p xtask --bins
cargo check -j 4 -p gpui_ui --features nsh-full
cargo check -j 4 -p gpui_code --no-default-features
cargo test -j 4 -p wgpu_cli --lib
```

For UI changes, see [`docs/src/development/ui-checklist.md`](./docs/src/development/ui-checklist.md) and attach rendered screenshots when applicable.


## Autonomous Worktrees, Cross-Compilation, and Release Channels

1. **Out-of-tree isolated worktrees** (keep the root checkout cleanly on `main`):
   - `bun run worktree:create -- --id <task>` allocates `../worktrees/aphrody-<task>` paired with `CARGO_TARGET_DIR=../target-worktrees/aphrody-<task>`.
   - `bun run worktree:list` / `bun run worktree:prune -- --id <task>`.
2. **Cross-compilation toolchain**:
   - `bun run cross:doctor` inspects `cargo-xwin`, `cargo-zigbuild`, `zig`, LLVM (`clang`, `lld-link`, `llvm-objdump`), Wasm tools, and `rustup` targets.
   - `bun run cross:matrix` and `bun run cross:check -- --target <triple> --package <crate>`.
3. **Six release tracks**: `lts`, `stable`, `preview`, `canary`, `nightly`, and `dev` (configured in `product.toml`, `crates/release_channel`, and `.github/workflows/aphrody-release.yml`).
4. **Repo hygiene & automated gates**: run `bun run hygiene && bun run test:tooling` before committing; never store durable scripts or plans outside the repository.
