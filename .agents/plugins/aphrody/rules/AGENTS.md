# aphrody Unified Plugin Rules

Single authoritative operating contract for agents working inside `<repo-root>` (`aphrody-dev/aphrody`).

## Architecture & Product Stack

- **Product**: `Aphrody` (`product.toml`, `crates/brand`, binary `aphrody` from `crates/zed`) uniting **GPUI**, **NSH** (`crates/nsh-*`), **gpui-kit**, **WGPU/Naga v30.0.0** (`vendor/wgpu`, `crates/gpui_wgpu`, `crates/wgpu_cli`), and **Bun** (`vendor/bun`) with **100% Zed ecosystem compatibility**.
- **Modularity boundaries**:
  - Keep `gpui_ui` free of `settings_content`, `cloud_llm_client`, and `language_model_core`.
  - Keep `gpui_code --no-default-features` free of `buffer_diff`, `language`, `rpc`, and `proto`.
  - Always resolve WGPU/Naga from `vendor/wgpu` (rev `cddb2f5d80dc0009b2ef6767635140cacf90a007`) so validation and runtime semantics match identically.

## Coding Standards (`.rules` & Tooling)

1. **No `mod.rs`**: Put crate roots in `src/<crate_name>.rs` (`[lib] path = ...`) and submodules in `src/<module>.rs`.
2. **Error Handling**: Never use `.unwrap()`, `.expect()` (except documented invariant tests), or `let _ = <fallible>`. Return `anyhow::Result` or handle explicitly.
3. **README Line Guard**: Never remove lines 1–2 (`> [!IMPORTANT]` header) of root `README.md`.
4. **Bun First for JS/TS**: Use `bun` (`bun test`, `bun check`, `bun fmt`, `bun build`) for all TypeScript/TSX/hook tooling — never `node`, `npm`, `npx`, or `tsc`.
5. **Reproducible Brand**: Never hardcode product strings; consume `brand::*` or run `cargo xtask brand apply` + `cargo xtask brand check` after editing `product.toml` or SVG icons.
6. **GPU & Shader Pipeline**:
   - Validate WGSL with `gpui-wgpu validate` (`naga` v30) and test backends across Vulkan (`glslangValidator`, `spirv-val`), DirectX 12 (`dxc.exe` SM 6.x), and WebGPU.
   - Respect `COPY_BYTES_PER_ROW_ALIGNMENT = 256` on texture readbacks and D3D/Metal clip-space depth `[0, 1]`.

## Git & GitHub Workflow

- Work strictly on `main` (never `git stash`, `git reset --hard`, or ad-hoc branches locally).
- Follow skill routing in `.agents/plugins/aphrody/skills/` and central docs index at `.agents/plugins/aphrody/docs/INDEX.md`.
- Always prove changes with concrete build/test/CLI/screenshot verification.
