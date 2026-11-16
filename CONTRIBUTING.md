# Contributing to Aphrody (`gpui-os`)

Thank you for contributing to **Aphrody** (`aphrody-dev/gpui-os`), unifying GPUI, Native Shell (`crates/nsh-*`), `gpui-kit`, WGPU/Naga v30 (`vendor/wgpu`, `crates/gpui_wgpu`, `crates/wgpu_cli`), and Bun (`vendor/bun`) with full Zed ecosystem compatibility.

## Before You Start

- **Use Discussions for architecture proposals**: Open a thread in [gpui-os Discussions](https://github.com/aphrody-dev/gpui-os/discussions) or check existing [Issues](https://github.com/aphrody-dev/gpui-os/issues) before large cross-crate refactors.
- **Automated upstream syncs**: Synchronization with `zed-industries/zed`, `longbridge/nsh`, `longbridge/gpui-component`, `gfx-rs/wgpu`, and `aphrody-labs/bun` is managed through [`.github/workflows/gpui-os-upstream-sync.yml`](./.github/workflows/gpui-os-upstream-sync.yml).
- **Plugin, coding rules & design guides**: Read [`.agents/plugins/gpui-os/rules/AGENTS.md`](./.agents/plugins/gpui-os/rules/AGENTS.md) and [`.agents/plugins/gpui-os/docs/INDEX.md`](./.agents/plugins/gpui-os/docs/INDEX.md).

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
