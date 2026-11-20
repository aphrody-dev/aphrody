# aphrody Unified Plugin Rules

Single authoritative operating contract for agents working inside `<repo-root>` (`aphrody-labs/aphrody`).

## Architecture & Product Stack

- **Product**: `Aphrody`, the OS and desktop layer (Zed fork, GPUI): `product.toml`, `crates/brand`, binary `aphrody` from `crates/zed`, the desktop shell (`crates/aphrody_desktop`, `crates/aphrody_session`, `crates/aphrody_installer`), **GPUI**, **NSH** (`crates/nsh-*`), **gpui-kit**, and **WGPU/Naga v30.0.0** (`crates/gpui_wgpu`, `crates/aphrody_cli`) with **100% Zed ecosystem compatibility**.
- **Forks**: upstream forks (`wgpu`, `taffy`, `accesskit`, `cosmic-text`, `egui`, `wry`, `craft-libs`, `craft-anything`, `mullvadvpn-app`, `browser`) are pinned Cargo git dependencies (`git` + `tag`, listed in [`tooling/bun/forks-manifest.json`](../../../../tooling/bun/forks-manifest.json)); there are no submodules and no `vendor/` directory.
- **Modularity boundaries**:
  - Keep `gpui_ui` free of `settings_content`, `cloud_llm_client`, and `language_model_core`.
  - Keep `gpui_code --no-default-features` free of `buffer_diff`, `language`, `rpc`, and `proto`.
  - Always resolve WGPU/Naga from the pinned `aphrody-labs/wgpu` git dependency in the root `Cargo.toml` so validation and runtime semantics match identically.

## Coding Standards (`.rules` & Tooling)

1. **No `mod.rs`**: Put crate roots in `src/<crate_name>.rs` (`[lib] path = ...`) and submodules in `src/<module>.rs`.
2. **Error Handling**: Never use `.unwrap()`, `.expect()` (except documented invariant tests), or `let _ = <fallible>`. Return `anyhow::Result` or handle explicitly.
3. **README Line Guard**: Never remove lines 1–2 (`> [!IMPORTANT]` header) of root `README.md`.
4. **Bun First for JS/TS**: Use `bun` (`bun test`, `bun check`, `bun fmt`, `bun build`) for all TypeScript/TSX/hook tooling — never `node`, `npm`, `npx`, or `tsc`.
5. **Repo-Native Artifacts & Typed CLI Tooling Only**: Never store architecture plans, repeatable scripts, or guides inside `~/.gemini/antigravity-cli/brain/`, `%TEMP%`, or `/tmp`. Version architectural blueprints in [`docs/aphrody/architecture/README.md`](../../../../docs/aphrody/architecture/README.md) with repo-relative links, put repeatable workflows in [`tooling/bun/`](../../../../tooling/bun/) (exposed via root [`package.json`](../../../../package.json)) or `tooling/xtask/`, direct ephemeral outputs to gitignored `target/`, and enforce via `bun tooling/bun/repo-hygiene.ts`.
6. **Reproducible Brand & Google Design Source of Truth**: Never hardcode product strings or ad-hoc color/motion values; consume `brand::*` + `aphrody_ui::google_design` (`assets/os/google_design_tokens.toml`, anchored to `https://design.google/`) and run `cargo xtask brand check` + `bun run design:audit` after edits.
7. **GPU & Shader Pipeline**:
   - Validate WGSL with `gpui-wgpu validate` (`naga` v30) and test backends across Vulkan (`glslangValidator`, `spirv-val`), DirectX 12 (`dxc.exe` SM 6.x), and WebGPU.
   - Respect `COPY_BYTES_PER_ROW_ALIGNMENT = 256` on texture readbacks and D3D/Metal clip-space depth `[0, 1]`.

## Git & GitHub Workflow

- Work strictly on `main` (never `git stash`, `git reset --hard`, or ad-hoc branches locally).
- Follow skill routing in `.agents/plugins/aphrody/skills/` and central docs index at `.agents/plugins/aphrody/docs/INDEX.md`.
- Always prove changes with concrete build/test/CLI/screenshot verification.
