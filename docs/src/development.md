---
title: Developing Aphrody
description: "Guide to building and developing Aphrody from source."
---

# Developing Aphrody

See the platform-specific instructions for building Aphrody from source:

- [macOS](./development/macos.md)
- [Linux](./development/linux.md)
- [Windows](./development/windows.md)
- [FreeBSD](./development/freebsd.md)

## Bird's-eye view of Aphrody {#birds-eye-view-of-zed}

Keep the [Aphrody glossary](./development/glossary.md) handy when starting out. It explains structures and terms you'll see throughout the codebase.

Aphrody is composed of modular crates. Here are the primary ones you're most likely to work with:

- [`brand`](https://github.com/aphrody-dev/aphrody/tree/main/crates/brand) derives single-source-of-truth product identity (`product.toml`, app names, URL schemes, env prefixes, paths, icons) while keeping 100% Zed ecosystem compatibility.
- [`gpui`](https://github.com/aphrody-dev/aphrody/tree/main/crates/gpui) is the GPU-accelerated UI framework providing reactive entities, layout, and rendering primitives.
- [`gpui_ui`](https://github.com/aphrody-dev/aphrody/tree/main/crates/gpui_ui) re-exports the lightweight UI engine (`gpui`, `ui`, `theme`, `component`, optional `nsh-full`) without heavy studio/LLM/WebRTC dependencies.
- [`gpui_code`](https://github.com/aphrody-dev/aphrody/tree/main/crates/gpui_code) bundles the composable Rust code engine (`rope`, `sum_tree`, `editor`, `vim`, `lsp`, `language`, `git`, `worktree`, `terminal`).
- [`gpui_app_shell`](https://github.com/aphrody-dev/aphrody/tree/main/crates/gpui_app_shell) provides a multi-window docked application starter pack (`Workspace`, `Dock`, `Pane`, keymaps, settings, `db`).
- [`wgpu_cli`](https://github.com/aphrody-dev/aphrody/tree/main/crates/wgpu_cli) (`gpui-wgpu`) validates/transpiles WGSL shaders on vendored `vendor/wgpu` / Naga v30 and inspects GPU adapters across Vulkan, DirectX 12, and WebGPU.
- [`nsh_workspace`](https://github.com/aphrody-dev/aphrody/tree/main/crates/nsh_workspace) and `crates/nsh-*` integrate the Native Shell (TSX/Rust) runtime powered by `vendor/bun`.
- [`os_session`](https://github.com/aphrody-dev/aphrody/tree/main/crates/os_session), [`os_ui`](https://github.com/aphrody-dev/aphrody/tree/main/crates/os_ui), [`os_craft`](https://github.com/aphrody-dev/aphrody/tree/main/crates/os_craft), and [`os_desktop`](https://github.com/aphrody-dev/aphrody/tree/main/crates/os_desktop) (`aphrody-os`) implement the Aphrody GPUI OS shell and ArtCraft workbench.
- [`editor`](https://github.com/aphrody-dev/aphrody/tree/main/crates/editor) contains the core `Editor` type that drives code buffers and input fields, plus display layers for inlay hints and completions.
- [`project`](https://github.com/aphrody-dev/aphrody/tree/main/crates/project) manages worktrees, file navigation, and LSP/debugger client orchestration.
- [`workspace`](https://github.com/aphrody-dev/aphrody/tree/main/crates/workspace) handles window/pane serialization and project grouping.
- [`vim`](https://github.com/aphrody-dev/aphrody/tree/main/crates/vim) implements Vim modal editing over `editor`.
- [`lsp`](https://github.com/aphrody-dev/aphrody/tree/main/crates/lsp) handles communication with external Language Server Protocol servers.
- [`language`](https://github.com/aphrody-dev/aphrody/tree/main/crates/language) drives syntax highlighting, symbol indexing, and language configuration.
- [`theme`](https://github.com/aphrody-dev/aphrody/tree/main/crates/theme) defines themes, icon themes, and color tokens.
- [`ui`](https://github.com/aphrody-dev/aphrody/tree/main/crates/ui) provides shared UI components and design patterns.
- [`cli`](https://github.com/aphrody-dev/aphrody/tree/main/crates/cli) invokes the `aphrody` CLI binary.
- [`zed`](https://github.com/aphrody-dev/aphrody/tree/main/crates/zed) is the main `aphrody` studio binary entry point.

## Keychain access

Aphrody stores secrets in the system keychain.

However, when running a development build on macOS (and similar platforms) trying to access the keychain results in repeated password prompts because development binaries do not have a stable signed identity.

By default, when running a `dev` build of Aphrody an alternative credential provider is used to bypass the system keychain.

> **Note:** This is **only** the case for development builds. For non-development release channels the system keychain is always used.

If you need to test the real system keychain in a development build, set:

```sh
ZED_DEVELOPMENT_USE_KEYCHAIN=1
```

## Performance Measurements

Aphrody includes a frame time measurement system to profile how long each frame takes to render.

### Using `ZED_MEASUREMENTS`

To enable performance measurements, set `ZED_MEASUREMENTS=1`:

```sh
export ZED_MEASUREMENTS=1
```

When enabled, frame rendering timing information is printed to stderr.

### Performance Comparison Workflow

1. **Enable measurements:**

   ```sh
   export ZED_MEASUREMENTS=1
   ```

2. **Test the first version:**

   - Checkout the baseline commit
   - Run Aphrody Studio in release mode for 5–10 seconds: `cargo run -p zed --release &> version-a`

3. **Test the second version:**

   - Checkout the candidate commit
   - Run Aphrody Studio in release mode for 5–10 seconds: `cargo run -p zed --release &> version-b`

4. **Generate comparison:**

   ```sh
   script/histogram version-a version-b
   ```

### Using `util_macros::perf`

For benchmarking unit tests, annotate them with `#[perf]` from `util_macros` and run `cargo perf-test -p $CRATE` (see `.cargo/config.toml`, `crates/util_macros`, and `tooling/perf`).

## ETW Profiling on Windows

Aphrody supports Event Tracing for Windows (ETW) to capture CPU, GPU, memory, disk, and file I/O traces into `.etl` files.

ETW recordings may contain personally identifiable or security-sensitive data (file paths, registry keys, process names); inspect traces before sharing.

### Recording a trace

Open the command palette and run:

- `zed: record etw trace`: records CPU, GPU, memory, and I/O activity
- `zed: record etw trace with heap tracing`: includes heap allocation data

Administrator permission is requested before recording starts.

### Saving or canceling

While recording, open the command palette and run:

- `zed: save etw trace`: stops recording and saves the trace to disk
- `zed: cancel etw trace`: stops recording without saving

## Contributor links

- [CONTRIBUTING.md](https://github.com/aphrody-dev/aphrody/blob/main/CONTRIBUTING.md)
- [Debugging Crashes](./development/debugging-crashes.md)
- [Code of Conduct](https://github.com/aphrody-dev/aphrody/blob/main/CODE_OF_CONDUCT.md)
- [Licensing & DCO](https://github.com/aphrody-dev/aphrody/blob/main/CONTRIBUTING.md#licensing--contributor-agreement)
