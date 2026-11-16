---
name: bun-gpui
description: Bun integration in aphrody via the `vendor/bun` submodule (aphrody-labs/bun). Use whenever working on TypeScript/TSX Native Shell (NSH) surfaces, typechecking (`bun check` / `nsh check`), formatting (`bun fmt` / `nsh fmt`), linting (`bun lint` / `nsh lint`), bundling (`Bun.build` / `bun build`), or reading upstream Bun documentation offline in `vendor/bun/docs/`.
---

# Bun in `aphrody` (`vendor/bun` + Native Shell)

`aphrody` vendors `https://github.com/aphrody-labs/bun` as a Git submodule at `vendor/bun` (`.gitmodules`), providing offline upstream documentation (`vendor/bun/docs/`) and powering all TypeScript/TSX tooling for Native Shell (`nsh`) with zero npm/Node/Prettier/ESLint/TypeScript tarball dependencies.

## 1. Core Rules

1. **Zero npm tool dependencies**: Never use `npx`, `bunx`, `tsc`, `prettier`, `biome`, `eslint`, `oxlint`, `vitest`, `jest`, or `vite`.
2. **Offline source of truth**: Read `vendor/bun/docs/` directly before assuming any Bun CLI flag or API:
   - Type checking: `vendor/bun/docs/runtime/check.mdx`
   - Formatting & Tailwind class sorting: `vendor/bun/docs/format/index.mdx`
   - Linting (1,000+ built-in rules): `vendor/bun/docs/lint/index.mdx`
   - Bundler & universal plugins: `vendor/bun/docs/bundler/index.mdx`, `vendor/bun/docs/bundler/plugins.mdx`
   - Native FFI (`bun:ffi`): `vendor/bun/docs/runtime/ffi.mdx`
3. **Root ignore configuration**: `bunfig.toml` at the repository root excludes `vendor/`, `target/`, and `**/.nsh/` from `bun fmt` and `bun lint`.

## 2. Unified NSH × Bun Commands

`crates/nsh-cli` (`nsh`) delegates TypeScript verification, formatting, and linting directly to Bun:

| Action | NSH Command | Underlying Bun Command | Notes |
| --- | --- | --- | --- |
| Typecheck Rust + TSX | `nsh check [dir]` | `bun check --no-script -p <dir>` | Runs `cargo check --all-targets` first on Rust crates, refreshes `.nsh/types` + `.nsh/tsconfig.json`, then checks TSX across all CPU cores using Bun's built-in `lib.*.d.ts`. |
| Format + sort Tailwind | `nsh fmt [path] [--check]` | `bun fmt [--check]` | Formats `.ts`, `.tsx`, `.json`, `.md` and sorts imports + Tailwind CSS classes. |
| Lint TSX/TS | `nsh lint [path] [--fix]` | `bun lint [--fix]` | Applies built-in TypeScript and JSX lint rules with zero npm packages. |
|Override binary | `--bun <path>` or `NSH_BUN` | `<path>` | Custom `bun` binary path when needed. |

## 3. Zero-Dependency `package.json` & `tsconfig.json`

`nsh init` scaffolds TSX and mixed apps with:
- `tsconfig.json` extending `./.nsh/tsconfig.json` (`"moduleResolution": "Bundler"`, `"jsx": "preserve"`, `"jsxImportSource": "nsh"`, `"types": []`).
- `package.json` with `"type": "module"`, `"dependencies": {}`, `"devDependencies": {}`, and scripts wired to `nsh` and `bun` (`"check": "bun check --no-script"`, `"fmt": "bun fmt"`, `"lint": "bun lint"`).

## 4. In-Memory Live TSX Preview inside `gpui_app_shell` and `zed`

Inside `gpui_ui` (`feature = "nsh-dev"`), `nsh_workspace`, `gpui_app_shell` (`feature = "nsh"`), and `zed` (`feature = "nsh"` / `full`):
- Single-module `.tsx` buffers compile directly in memory via `nsh_bundler::compile_module` (pure `oxc` AST JSX compiler, **zero `rolldown` crates** in `gpui_ui`, `gpui_app_shell`, or `zed`).
- Opening **`nsh_workspace: Open Live Tsx Preview`** splits the active `.tsx` editor and live-reloads the native `gpui` `AppView` surface on every keystroke, synchronized with `GlobalTheme` and supporting embedded `<GpuiCodeEditor>` (`gpui_code::Editor`) native components.
