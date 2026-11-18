---
name: bun-gpui
description: Bun integration in aphrody via the Aphrody Bun fork (aphrody-labs/bun). Use whenever working on TypeScript/TSX Native Shell (NSH) surfaces, typechecking (`bun check` / `nsh check`), formatting (`bun fmt` / `nsh fmt`), linting (`bun lint` / `nsh lint`), bundling (`Bun.build` / `bun build`), or reading upstream Bun documentation via `bun-brain` MCP / `aphrody-labs/bun`.
---

# Bun in `aphrody` (`aphrody-labs/bun` + Native Shell)

`aphrody` uses the installed Aphrody Bun runtime (`https://github.com/aphrody-labs/bun`, compacted offline in `bun-brain` MCP and `~/.gemini/brain/bun/`) for all TypeScript/TSX tooling across Native Shell (`nsh`) and repository scripts with zero npm/Node/Prettier/ESLint/TypeScript tarball dependencies.

## 1. Core Rules

1. **Zero npm tool dependencies**: Never use `npx`, `bunx`, `tsc`, `prettier`, `biome`, `eslint`, `oxlint`, `vitest`, `jest`, or `vite`.
2. **Offline documentation lookup**: Query `bun-brain` MCP (`bun_brain_search`, `bun_types_lookup`, `bun_api_matrix`) or `~/.gemini/brain/bun/` (`docs/runtime/check.mdx`, `docs/format/index.mdx`, `docs/lint/index.mdx`, `docs/bundler/index.mdx`, `docs/runtime/ffi.mdx` in `aphrody-labs/bun`) before guessing any Bun CLI flag or API.
3. **Root ignore configuration**: `bunfig.toml` at the repository root excludes `vendor/`, `target/`, and `**/.nsh/` from `bun fmt` and `bun lint`.
4. **Dot-directory test discovery (`bun test`)**: Pass explicit `./`-prefixed paths for test files inside hidden directories (e.g., `bun test tooling/bun ./.cloudflare/aphrody-worker/src/index.test.ts`); bare `.cloudflare/...` arguments are silently skipped by `bun test`.

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

Inside `gpui_ui` (`feature = "nsh-dev"`), `aphrody_nsh`, `gpui_app_shell` (`feature = "nsh"`), and `zed` (`feature = "nsh"` / `full`):
- Single-module `.tsx` buffers compile directly in memory via `nsh_bundler::compile_module` (pure `oxc` AST JSX compiler, **zero `rolldown` crates** in `gpui_ui`, `gpui_app_shell`, or `zed`).
- Opening **`aphrody_nsh: Open Live Tsx Preview`** splits the active `.tsx` editor and live-reloads the native `gpui` `AppView` surface on every keystroke, synchronized with `GlobalTheme` and supporting embedded `<GpuiCodeEditor>` (`gpui_code::Editor`) native components.
