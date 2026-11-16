---
name: zed
description: >-
  Architecture, full Zed ecosystem compatibility invariants, extension WIT ABI,
  settings/keymap/theme schemas, xtask conformity, and upstream cherry-pick/sync
  procedures for gpui-os. Use when modifying Zed-derived crates, extensions,
  settings schemas, onboarding, CLI compatibility, or upstream syncs.
---

# Zed Ecosystem Architecture & Compatibility in gpui-os

`gpui-os` compiles the full modular studio as `aphrody` (`crates/zed`, package name `zed`, binary `aphrody`) while preserving **100% compatibility** with Zed settings, extensions, keymaps, themes, schemas, and upstream merges.

## Non-Negotiable Compatibility Invariants

1. **Extension WIT ABI & Store API**:
   - Keep `package zed:extension;` (`crates/extension/wit/`) unchanged so all existing Zed WASM/native extensions work unmodified.
   - Preserve `https://api.zed.dev` resolution for extension registry/downloads in `crates/http_client/src/http_client.rs` (`build_zed_api_url`).
2. **Directories & Config Fallbacks**:
   - Primary project directory `.aphrody/` with `.zed/` fallback across `worktree`, `project`, `task_inventory`, and `crates/zed`.
   - User config/data dirs resolve cleanly through `crates/paths` and `crates/brand`.
3. **Schemes, Fonts, Keymaps, Env Vars**:
   - Accept both `aphrody://` and `zed://` URLs and JSON schemas (`open_listener`, `json_schema_store`, `cli`).
   - Support `.AphrodySans` / `.AphrodyMono` plus `.ZedSans` / `.ZedMono` aliases (`gpui`, `settings`).
   - Keep `BaseKeymap::Zed` with `#[serde(alias = "Aphrody")]`.
   - Read `APHRODY_*` env/task variables with `ZED_*` fallbacks and dual exports (`crates/zed_env_vars`, `crates/task`, `crates/terminal`).
4. **CLI & Binary Dispatch**:
   - `crates/cli` discovers and spawns `aphrody.exe` / `aphrody` with `zed.exe` / `zed` fallback; `crates/zed/src/main.rs` enforces `paths::APP_NAME_LOWERCASE == env!("CARGO_BIN_NAME")`.

## Hard Repository Rules (`.rules`)

- Never create `mod.rs` files; use explicit `[lib] path = "..."` and sibling `foo.rs` modules.
- Never use `.unwrap()` or `let _ = <fallible>` in production code.
- Preserve lines 1–2 of root `README.md` (`> [!IMPORTANT]` + note) until public mirror readiness.
- Never remove doc comments when fixing warnings; keep Clippy clean via `./script/clippy` / `cargo clippy`.

## Verification & Conformity Commands

- `cargo xtask brand check` — validates `product.toml` against manifests & icons.
- `cargo xtask package-conformity` — checks workspace dependency inheritance & lints.
- `cargo check -p zed --bin aphrody --features full` — validates full feature compilation.
- `cargo build -p zed --bin aphrody --features nsh` — builds desktop executable with NSH workspace.

## References

- [Zed Architecture & Dev Docs](../../docs/zed/SUMMARY.md)
- [UI Checklist](../../docs/zed/development/ui-checklist.md)
- [Extension Development](../../docs/zed/extensions.md)
- [Brand Conformance](../../docs/conventions/CONVENTIONS.md)
- Also see sibling skills: `zed-cherry-pick`, `aphrody-brand`, `gpui`, `gpui-test`, `gpui-bench`, `lint-creator`.
