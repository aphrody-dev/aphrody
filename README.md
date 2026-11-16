> [!IMPORTANT]
> Remove this line to confirm you've reviewed this PR before submitting.

# Aphrody — GPUI OS (`gpui-os`)

Modular, local-first, ultra-fast Rust **GPU UI Framework**, **Code Engine**, **Native Shell (TSX/Rust)**, **WGPU Toolkit**, **Application Starter Pack**, and **Aphrody Studio** — with **100% ecosystem compatibility with Zed**.

`gpui-os` unifies the GPUI desktop/mobile/web engine, Native Shell (`nsh`), `vendor/wgpu` (v30.0.0), and `vendor/bun` into composable Rust libraries plus the **Aphrody** studio binary (`aphrody`), driven by a single declarative identity manifest ([`product.toml`](./product.toml)).

---

## Architecture Layers

| Crate / Path | Purpose | Heavy Bloat in Default Build |
| :--- | :--- | :--- |
| **`crates/brand`** + **[`product.toml`](./product.toml)** | Single-source-of-truth product identity (`Aphrody`, `aphrody://`, `.aphrody/`, app IDs, icons) with transparent Zed compatibility (`.zed/`, `zed://`, `ZED_*`, `zed:extension`). | **Zero** (0 runtime deps) |
| **`crates/gpui_ui`** | GPU-accelerated UI engine (`gpui`, `gpui_platform`, `Application::run_embedded`), design system (`ui`, `ui_input`, `component`), themes, icons, and optional Native Shell (`--features nsh-full`). | **Zero** (No WebRTC, No Wasmtime, No LLM SDKs, No `rolldown`) |
| **`crates/gpui_code`** | Composable Rust code engine: pure text core (`rope`, `sum_tree`, `streaming_diff`, `imara-diff`, `fuzzy`) + opt-in Tree-sitter `syntax`/`grammars`, `buffer_diff`, `lsp`, `git`, `worktree`, `terminal` PTY, `editor`, and `vim`. | **Zero** WebRTC / Wasmtime / LLM SDKs (`--no-default-features` builds zero `rpc`/`proto`) |
| **`crates/nsh-*` & `crates/nsh_workspace`** | Absorbed Native Shell v0.5.0 unified on `gpui 0.2.2` & `wgpu 30.0.0`: Solid-style signals, 35+ controls, live in-memory `.tsx` hot-reload preview, and `<GpuiCodeEditor>` native embedding. | **Zero** `gpui-fast` duplicate, **zero** npm TypeScript tarballs |
| **`crates/gpui_app_shell`** | Production multi-window docked application starter pack (`Workspace`, `Dock`, `Pane`, Command Palette, Keymaps, JSON Settings, SQLite `db`, `AppModule` trait, `NshModule`). | **Zero** WebRTC / Wasmtime |
| **`crates/wgpu_cli` (`gpui-wgpu`)** | GPU adapter inspector, Naga WGSL validator/reflector, cross-compiler (HLSL/MSL/GLSL/SPIR-V), headless PNG renderer, compute runner, and GPU benchmark CLI built on `vendor/wgpu`. | Standalone Apache-2.0 CLI |
| **`crates/zed` (`bin = "aphrody"`)** | **Aphrody Studio**: **Local-First Lite** by default (`default = []`), with opt-in Cargo features (`nsh`, `extensions`, `debugger`, `repl`, `collab`, `ai`, `updater`, `crash-reporting`, `full`). | **Zero** by default; 100% opt-in via `--features full` |

---

## Quick Start

### 1. Check Default SDK Crates (`gpui_ui`, `gpui_code`, `gpui_app_shell`)

```sh
cargo check
```

### 2. Run & Build Aphrody Studio

```sh
# Ultra-fast Local-First Aphrody (LSP, Git, Worktree, Terminal, Multi-buffer, Vim, Search, Tasks):
cargo run -p zed

# Aphrody with Native Shell live TSX surfaces + Wasm Extensions + Debugger:
cargo run -p zed --features "nsh,extensions,debugger"

# Full Aphrody with every subsystem enabled:
cargo run -p zed --features full

# Maximum-optimization LLVM FatLTO distribution build:
cargo build -p zed --profile dist --features "nsh,extensions,debugger"
```

### 3. Reproducible Product Branding (`product.toml` -> `cargo xtask brand`)

All product identity metadata, bundle identifiers, OS desktop/installer files, and multi-channel icons (`.png`, `@2x.png`, multi-size Windows `.ico`) are generated deterministically from [`product.toml`](./product.toml) and [`assets/images/aphrody_logo.svg`](./assets/images/aphrody_logo.svg):

```sh
# Regenerate all manifests and icons from product.toml:
cargo xtask brand apply

# Verify zero brand drift in CI (read-only):
cargo xtask brand check
```

### 4. Scaffold a New Standalone GPUI / NSH Application

```sh
cargo xtask new-app my-nsh-studio --template nsh-studio
cargo xtask new-app my-studio --template studio
cargo xtask new-app my-code-tool --template code
cargo xtask new-app my-widget --template minimal
```

### 5. Native Shell (`nsh`) × `vendor/bun` Zero-Dependency Workflow

[`vendor/bun`](./vendor/bun) (`aphrody-labs/bun` Git submodule) powers TypeScript checking, formatting, Tailwind sorting, and linting with zero npm downloads:

```sh
cargo run -p nsh-cli -- check examples/nsh/counter
cargo run -p nsh-cli -- fmt examples/nsh/counter
cargo run -p nsh-cli -- lint examples/nsh/counter
cargo run -p nsh-cli -- test examples/nsh/counter
```

### 6. WGPU v30.0.0 CLI (`gpui-wgpu`), GPU Backends & Unified Plugin

Backed by [`vendor/wgpu`](./vendor/wgpu), [`docs/wgpu/llms.txt`](./docs/wgpu/llms.txt), [`docs/wgpu/llms-full.txt`](./docs/wgpu/llms-full.txt), [`docs/gpu/`](./docs/gpu), and the unified agent plugin in [`.agents/plugins/gpui-os/`](./.agents/plugins/gpui-os/plugin.json) (bundling all skills, hooks, rules, and docs for Zed, GPUI, WGPU, Vulkan, DirectX 12, WebGPU, NSH, gpui-kit, Bun, Brand, and GitHub Ops):

```sh
cargo run -p wgpu_cli -- adapters
cargo run -p wgpu_cli -- validate crates/gpui_wgpu/src/shaders.wgsl
cargo run -p wgpu_cli -- transpile path/to/shader.wgsl --target hlsl
cargo run -p wgpu_cli -- render path/to/shader.wgsl -o render.png --width 800 --height 600
cargo run -p wgpu_cli -- bench --iterations 25
```

---

## Full Zed Ecosystem Compatibility

Aphrody isolates its user directories (`%APPDATA%\Aphrody` / `%LOCALAPPDATA%\Aphrody` on Windows, `~/.config/aphrody` / `~/.local/share/aphrody` on Linux, `~/Library/Application Support/Aphrody` on macOS) so it can run side-by-side with Zed while keeping 100% compatibility:
- **Wasm Extensions**: Preserves the `zed:extension` WIT ABI (`zed_extension_api`) and connects directly to `https://api.zed.dev` so every community language, theme, icon, debugger, and MCP extension works unmodified.
- **Project Configs**: Automatically discovers and watches both `.aphrody/{settings,tasks,debug}.json` and `.zed/{settings,tasks,debug}.json` in any repository.
- **URLs, Schemas, Keymaps & Fonts**: Supports both `aphrody://` and `zed://` deep links and JSON schemas, all `zed::*` keymap action bindings, both `$APHRODY_*` and `$ZED_*` task/env variables, and both `.AphrodySans`/`.AphrodyMono` and `.ZedSans`/`.ZedMono` virtual font aliases.

---

## Privacy & Local-First Defaults

- `telemetry.diagnostics = false` and `telemetry.metrics = false`
- `auto_update = false`
- `title_bar.show_sign_in = false` and `title_bar.show_user_menu = false`
- Legacy SaaS server (`crates/collab`) and `aws-sdk-s3` / `aws-sdk-kinesis` / `sqlx` / `sea-orm` dependencies are purged.

---

## Licensing

Derived from [Zed](https://github.com/zed-industries/zed) and [Native Shell](https://github.com/longbridge/nsh), with [`wgpu`](https://github.com/gfx-rs/wgpu) (Apache-2.0 / MIT) in `vendor/wgpu` and [`bun`](https://github.com/aphrody-labs/bun) as a submodule in `vendor/bun`. Source code is licensed under GPL-3.0-or-later and Apache-2.0 / MIT where marked (including `gpui`, `brand`, `wgpu_cli`, and `nsh-*`).
