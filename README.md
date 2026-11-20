> [!IMPORTANT]
> Remove this line to confirm you've reviewed this PR before submitting.

# Aphrody — OS and desktop (Zed fork, GPUI) (`aphrody`)

Local-first Rust **OS and desktop** layer, a fork of [Zed](https://github.com/zed-industries/zed): the `aphrody-os` GPUI desktop shell and compositor (`aphrody_desktop`, `aphrody_session`), the installer (`aphrody_installer`), the **Aphrody Studio** editor (`aphrody` binary), and the GPU UI stack (GPUI, WGPU, Taffy layout, Native Shell TSX/Rust, AccessKit, Cosmic-Text, design tokens), with full Zed ecosystem compatibility. It owns the UI/GPU upstream forks. The toolchain and agent runtime live in [`fw`](../fw); the SDK, cloud and edge live in [`ay`](../ay). Dependencies only point down the stack (`fw` <- `aphrody` <- `ay`).

`aphrody` unifies the GPUI desktop/mobile/web engine, Native Shell (`nsh`), pinned [`aphrody-labs/*`](./tooling/bun/forks-manifest.json) GitHub Releases (`wgpu` v30.0.0, `taffy` + Cloudflare Kitesurf patches, `egui`, `accesskit`, `cosmic-text`, `wry`, `mullvadvpn-app`, `browser`, `kitesurf`, `craft-libs`, `craft-anything`), and `ghcr.io/aphrody-labs/aphrody-forks:20261120` into composable Rust libraries plus the **Aphrody** studio binary (`aphrody`), driven by a single declarative identity manifest ([`product.toml`](./product.toml)).

---

## Architecture Layers

| Crate / Path | Purpose | Heavy Bloat in Default Build |
| :--- | :--- | :--- |
| **`crates/brand`** + **[`product.toml`](./product.toml)** | Single-source-of-truth product identity (`Aphrody`, `aphrody://`, `.aphrody/`, app IDs, icons) with transparent Zed compatibility (`.zed/`, `zed://`, `ZED_*`, `zed:extension`). | **Zero** (0 runtime deps) |
| **`crates/gpui_ui`** | GPU-accelerated UI engine (`gpui`, `gpui_platform`, `Application::run_embedded`), design system (`ui`, `ui_input`, `component`), themes, icons, and optional Native Shell (`--features nsh-full`). | **Zero** (No WebRTC, No Wasmtime, No LLM SDKs, No `rolldown`) |
| **`crates/gpui_code`** | Composable Rust code engine: pure text core (`rope`, `sum_tree`, `streaming_diff`, `imara-diff`, `fuzzy`) + opt-in Tree-sitter `syntax`/`grammars`, `buffer_diff`, `lsp`, `git`, `worktree`, `terminal` PTY, `editor`, and `vim`. | **Zero** WebRTC / Wasmtime / LLM SDKs (`--no-default-features` builds zero `rpc`/`proto`) |
| **`crates/nsh-*` & `crates/aphrody_nsh`** | Absorbed Native Shell v0.6.0 unified on `gpui 0.2.2` & `wgpu 30.0.0`: Solid-style signals, 35+ controls, live in-memory `.tsx` hot-reload preview, and `<GpuiCodeEditor>` native embedding. | **Zero** `gpui-fast` duplicate, **zero** npm TypeScript tarballs |
| **`crates/gpui_app_shell`** | Production multi-window docked application starter pack (`Workspace`, `Dock`, `Pane`, Command Palette, Keymaps, JSON Settings, SQLite `db`, `AppModule` trait, `NshModule`). | **Zero** WebRTC / Wasmtime |
| **`crates/aphrody_cli` (`gpui-wgpu`)** | Unified WGPU/Naga v30 adapter, shader validator/transpiler, headless PNG renderer, compute/bench, Wasm LTO/AOT toolchain, 5 standalone `templates/wasm-wgpu/` starters, `DESIGN.md` / Lighthouse CLI, and 24-tool JSON-RPC MCP server. | Standalone Apache-2.0 CLI + MCP |
| **`crates/aphrody_design`** | Pure-Rust `https://design.google/` HTTP/HTML5/sitemap mirror, `DESIGN.md` Tailwind v4/W3C/WGSL exporter, Google Stitch SDK + PNG capture, and Core Web Vitals / Lighthouse / W3C / Vercel auditor. | Zero subprocess `curl`/`wget`, `wasm32`-ready |
| **`crates/aphrody_session`** | Wayland/X11/Windows/macOS desktop session supervisor, topological unit startup (`assets/os/session.toml`), `taffy`-backed LayerShell/Dwindle/CosmicGrid/NiriScroll compositor geometry, XDG/StartMenu app discovery, and `.deb`/`.desktop`/Flatpak descriptors. | Pure Rust (`wasm32`-ready) |
| **`crates/aphrody_ui`** | Unified GPUI OS toolkit combining real Material 3 HCT dynamic colors (`material-colors 0.5` Spec2025), Google Material Symbols (`md-icons 0.4`) & Lucide (`lucide-static-svg 1.76`) vector SVGs, frosted/liquid glass (`backdrop-blur-core 0.2` + `window-vibrancy 0.8`), COSMIC RON theming (`assets/os/themes.json` + WCAG AA checks), [`google_design`](./crates/aphrody_ui/src/google_design.rs) tokens/springs, [`starter_pack`](./crates/aphrody_ui/src/starter_pack.rs), `PanelApplet`/`NavRail`, `iced`-style `ReactiveProgram`, and `egui`-style `ImmediateUi`. | Pure GPUI (`ui` + `theme`) |
| **`crates/aphrody_craft`** | Unified creative studio & binary inspector (`assets/os/craft_catalog.toml`), pure-Rust decoders (`craft-sniff`, `craft-psd`, `craft-exr`, `craft-plist`, `image`, `usvg`/`resvg`), `object`/`wasmparser` inspector, and stdio/HTTP JSON-RPC 2.0 MCP server. | Opt-in codecs (`--no-default-features` skips raster/native/Wasm) |
| **`crates/aphrody_desktop` (`bin = "aphrody-os"`)** | COSMIC Epoch GPU desktop shell (`TopPanel`, compositor tiling stage, Liquid Glass `DockBar`, searchable `Launcher`, Studio, Native Browser, WireGuard VPN, Files, Reactive Settings, System Monitor, Design & Hub, Store, Installer, and live WebGPU Wasm at `https://web.aphrody.com/`). | `default = ["desktop"]` |
| **`crates/zed` (`bin = "aphrody"`)** | **Aphrody Studio**: consolidated `aphrody` profile by default (`default = ["aphrody"]`), with `--no-default-features` stripping `nsh`, `extensions`, `debugger`, `repl`, `collab`, `ai`, `updater`, and `crash-reporting` down to the local-first editor core. | `--no-default-features` trims optional subsystems |

---

## Quick Start

### 1. Check the Default Workspace Members (`crates/aphrody`, `gpui_wgpu`, `gpui_web`, `gpui_ui`, `gpui_code`, `aphrody_session`, `aphrody_installer`, `aphrody_desktop`, `aphrody_cli`, and the other `default-members` of [`Cargo.toml`](./Cargo.toml))

```sh
cargo check
```

### 2. Run & Build Aphrody Studio

```sh
# Local-First Core Aphrody Studio (no collab, updater, telemetry, or extensions):
cargo run -p zed --no-default-features

# Full Aphrody Studio (default profile):
cargo run -p zed

# Aphrody GPUI OS Shell, Studio & Binary Inspector:
cargo run -p aphrody_desktop --bin aphrody-os
cargo run -p aphrody_desktop --bin aphrody-os -- doctor --json
cargo run -p aphrody_desktop --bin aphrody-os -- catalog
cargo run -p aphrody_desktop --bin aphrody-os -- inspect assets/images/aphrody_logo.svg --json
cargo run -p aphrody_desktop --bin aphrody-os -- mcp

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

### 5. Native Shell (`nsh`) × `bun` Toolchain Workflow

Driven by Aphrody Bun runtime (see [`.agents/plugins/aphrody/skills/bun-gpui/SKILL.md`](./.agents/plugins/aphrody/skills/bun-gpui/SKILL.md)) for TypeScript checking, formatting, Tailwind sorting, and linting with zero npm downloads:

```sh
cargo run -p nsh-cli -- check examples/nsh/counter
cargo run -p nsh-cli -- fmt examples/nsh/counter
cargo run -p nsh-cli -- lint examples/nsh/counter
cargo run -p nsh-cli -- test examples/nsh/counter
```

### 6. WGPU v30.0.0 CLI (`gpui-wgpu`), GPU Backends & Unified Plugin

Backed by pinned [`aphrody-labs/wgpu`](./tooling/bun/forks-manifest.json) (`v30.0.0-aphrody.20261120`), [`aphrody/docs/wgpu/llms.txt`](./aphrody/docs/wgpu/llms.txt), [`aphrody/docs/wgpu/llms-full.txt`](./aphrody/docs/wgpu/llms-full.txt), [`aphrody/docs/gpu/`](./aphrody/docs/gpu), and the unified agent plugin in [`.agents/plugins/aphrody/`](./.agents/plugins/aphrody/plugin.json) (bundling all skills, hooks, rules, and docs for Zed, GPUI, WGPU, Vulkan, DirectX 12, WebGPU, NSH, gpui-kit, Bun, Brand, and GitHub Ops):

```sh
cargo run -p aphrody_cli -- adapters
cargo run -p aphrody_cli -- validate crates/gpui_wgpu/src/shaders.wgsl
cargo run -p aphrody_cli -- transpile path/to/shader.wgsl --target hlsl
cargo run -p aphrody_cli -- render path/to/shader.wgsl -o render.png --width 800 --height 600
cargo run -p aphrody_cli -- bench --iterations 25
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
- Legacy SaaS server (`crates/collab`) and `aws-sdk-s3` / `aws-sdk-kinesis` / `sqlx` / `sea-orm` dependencies are purged (`collab_ui` remains strictly behind the `collab` feature in `crates/zed`).

---

## Licensing

Derived from [Zed](https://github.com/zed-industries/zed) and [Native Shell](https://github.com/longbridge/nsh), with pinned GitHub Releases in [`tooling/bun/forks-manifest.json`](./tooling/bun/forks-manifest.json) (`aphrody-labs/wgpu`, `aphrody-labs/taffy`, `aphrody-labs/egui`, `aphrody-labs/accesskit`, `aphrody-labs/cosmic-text`, `aphrody-labs/wry`, `aphrody-labs/mullvadvpn-app`, `aphrody-labs/browser`, `aphrody-labs/kitesurf`, `aphrody-labs/craft-libs`, `aphrody-labs/craft-anything`) and OCI package `ghcr.io/aphrody-labs/aphrody-forks:20261120`. Workspace crates default to Apache-2.0 (`[workspace.package]`) and each crate declares its own `license` in its `Cargo.toml`: Apache-2.0 for `gpui*`, `brand`, `aphrody_design`, `aphrody_cli` and `nsh-*`; GPL-3.0-or-later for the Zed-derived crates (including `zed`, `aphrody`, `aphrody_session`, `aphrody_ui` and `aphrody_craft`); `Apache-2.0 OR GPL-3.0-or-later` for `aphrody_pilot`, `aphrody_google` and `aphrody_media`.
