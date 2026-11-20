# Contributing to Aphrody (`aphrody`)

Thank you for helping make **Aphrody** (`aphrody-labs/aphrody`) better! Aphrody is the OS and desktop layer (Zed fork, GPUI): the desktop shell, session and installer (`crates/aphrody_desktop`, `crates/aphrody_session`, `crates/aphrody_installer`), GPUI, Native Shell (`crates/nsh-*`), `gpui-kit`, and WGPU/Naga v30 (`crates/gpui_wgpu`, `crates/aphrody_cli`, with the `aphrody-labs/wgpu` fork pinned as a Cargo git dependency), with 100% Zed ecosystem compatibility driven by [`product.toml`](./product.toml).

## Before You Start

- **Keep no more than three PRs open at a time.**
  This helps keep reviews responsive and gives every contributor a fair share of reviewing bandwidth. Start with one PR and see it through to merge; stacking open PRs increases rebase churn as `main` advances.
- **Discuss features before you start writing code.**
  If there is not already a confirmed GitHub Issue, start a [GitHub Discussion](https://github.com/aphrody-labs/aphrody/discussions) rather than opening a premature PR or duplicate issue. This is especially important for large cross-crate refactors and changes to the `zed:extension` Extension API (`crates/extension_api`).
  - Before proposing a moderate-to-large feature, read [Aphrody's Feature Process](./docs/src/development/feature-process.md) for the motivation, integration points, and design checklist to cover.
- **Automated upstream syncs**: Synchronization with `zed-industries/zed`, `longbridge/nsh`, `longbridge/gpui-kit`, and `gfx-rs/wgpu` is managed through [`.github/workflows/aphrody-upstream-sync.yml`](./.github/workflows/aphrody-upstream-sync.yml).
- **Code of Conduct**: All participation across issues, discussions, and pull requests is governed by [`CODE_OF_CONDUCT.md`](./CODE_OF_CONDUCT.md).
- **Plugin, coding rules & architecture guides**: Read [`AGENTS.md`](./AGENTS.md), [`.agents/plugins/aphrody/rules/AGENTS.md`](./.agents/plugins/aphrody/rules/AGENTS.md), [`.agents/plugins/aphrody/docs/INDEX.md`](./.agents/plugins/aphrody/docs/INDEX.md), and [`docs/aphrody/architecture/README.md`](./docs/aphrody/architecture/README.md).

### Things We Will (Probably) Not Merge

There are few rigid rules, but we typically decline:

- **Changes that can be provided by an extension**, such as new languages, themes, snippets, debuggers, or MCP servers. See [Developing Extensions](./docs/src/extensions/developing-extensions.md).
- **Extension API (`crates/extension_api`) changes without prior discussion**, or changes that break `zed:extension` ecosystem compatibility.
- **Unreviewed off-the-shelf file or app icons.** Follow the [icon design guidelines](./crates/icons/README.md) for UI icons; app and channel icons are generated from [`product.toml`](./product.toml) and [`assets/images/aphrody_logo.svg`](./assets/images/aphrody_logo.svg) via `cargo xtask brand apply`.
- **Features whose complexity outweighs their benefit** for the maintenance surface they introduce.
- **Giant refactorings** without prior architectural alignment.
- **Non-trivial changes without tests** (or without GPU/CLI/UI verification artifacts).
- **Style-only code changes that do not alter logic.** Reducing allocations, removing `.unwrap()`, and fixing typos are welcome; churn-only reformatting or renaming is not.
- **Unverified or LLM-generated work the author does not understand.**

### Ways to Contribute (We Love PRs That...)

- **Fix or extend documentation**: browse [`documentation` issues](https://github.com/aphrody-labs/aphrody/issues?q=is%3Aissue+state%3Aopen+label%3Adocumentation).
- **Pick up curated starter or community issues**: browse [`good first issue`](https://github.com/aphrody-labs/aphrody/issues?q=is%3Aissue+state%3Aopen+label%3A%22good+first+issue%22) and [`help wanted`](https://github.com/aphrody-labs/aphrody/issues?q=is%3Aissue+state%3Aopen+label%3A%22help+wanted%22).
- **Fix bugs**: browse open [`bug` issues](https://github.com/aphrody-labs/aphrody/issues?q=is%3Aissue+state%3Aopen+label%3Abug) or filter by subsystem [labels](https://github.com/aphrody-labs/aphrody/labels) (`zed`, `nsh`, `gpui-kit`, `wgpu`, `gpu/vulkan`, `gpu/directx`, `gpu/webgpu`, `accessibility`).
- **Make existing features work across more platforms**, windowing backends, and GPUs (Wayland/X11, Windows DX12/Vulkan, macOS Metal, Wasm/WebGPU).
- **Add small, focused features**, such as keybindings, actions, NSH controls, or `gpui-wgpu` diagnostics.

## Reporting Bugs & Crashes

Before filing an issue, search existing [Issues](https://github.com/aphrody-labs/aphrody/issues). For feature ideas or design feedback, use [Feature Discussions](https://github.com/aphrody-labs/aphrody/discussions/new?category=feature-requests) ([template](./.github/DISCUSSION_TEMPLATE/feature-requests.yml?plain=1)).

Use the structured issue templates:

- [Bug Report](./.github/ISSUE_TEMPLATE/10_bug_report.yml?plain=1)
- [Crash Report](./.github/ISSUE_TEMPLATE/11_crash_report.yml?plain=1)
- [Upstream Sync / Issue Fix](./.github/ISSUE_TEMPLATE/20_feature_or_issue_fix.yml?plain=1)

Include minimal reproduction steps from a clean setup, `aphrody: copy system specs into clipboard` output, relevant redacted logs (`Aphrody.log` via `aphrody: open log` or `aphrody --foreground` — redact any tokens/API keys), redacted `settings.json` / `keymap.json`, and for rendering/shader bugs the output of `cargo run -p aphrody_cli -- adapters --json`.

## AI Policy

We welcome AI/LLM coding tools, and we hold a high quality bar for every contribution: **every author must genuinely understand, build, and verify the work submitted on their behalf**.

- **No unattended external PR spam**: unreviewed or untested output from autonomous bots will be closed. Repo-native automation (upstream syncs, Renovate, labeled `issue-fix` workflows) must pass full CI, hygiene, and test gates.
- **Communicate clearly in your own words**: write concise, factual PR descriptions, review replies, and issue comments yourself rather than pasting raw model-generated prose.
- **Translations**: non-native English speakers using an LLM to translate messages are encouraged to place the translation in a quote block (`>`) followed by the original text in their native language.
- **Disclosing LLM chat context**: if sharing relevant context from an AI session helps review, quote only the relevant excerpt (`>`), mark it as AI-generated, and add your own commentary explaining why it is relevant.
- **Git & repo-hygiene rules**: never add AI attribution trailers (`Co-Authored-By`, etc.) to commits, never commit files or links referencing external scratch paths (`~/.gemini/antigravity-cli/brain/`, `%TEMP%`, `/tmp`), and never remove lines 1–2 of [`README.md`](./README.md) (`> [!IMPORTANT]` self-review guard) before final human review.

This policy is adapted from Zed's and [ripgrep's AI policy](https://github.com/BurntSushi/ripgrep/blob/f0cec341ab95c25c691ad3d5754d4bd9eedde21f/AI_POLICY.md).

## Building from Source

### 1. Forks & Toolchain

Upstream forks (`wgpu`, `taffy`, `accesskit`, `cosmic-text`, `egui`, `wry`, `craft-libs`, `craft-anything`, `mullvadvpn-app`, `browser`) are not submodules and there is no `vendor/` directory: Cargo resolves each as a pinned git dependency (`git = "https://github.com/aphrody-labs/<fork>.git", tag = "..."`) recorded in [`tooling/bun/forks-manifest.json`](./tooling/bun/forks-manifest.json). Ensure the pinned Rust toolchain ([`rust-toolchain.toml`](./rust-toolchain.toml)) is active; `cargo` fetches the forks on first build.

> Note: Root [`Cargo.toml`](./Cargo.toml) sets `default-members` to the modular SDK/OS crates (`gpui_ui`, `gpui_code`, `aphrody_session`, `aphrody_craft`, `aphrody_ui`, `aphrody_desktop`, `aphrody_cli`, and others). Pass `-p zed` to build/run the **Aphrody Studio** binary (`aphrody`), or `--workspace` for workspace-wide tests.

### 2. Platform Guides

- **Linux**: [`docs/src/development/linux.md`](./docs/src/development/linux.md) — install OS deps with `./script/linux`, run `cargo run -p zed` (or `--no-default-features` for core studio, `cargo run -p aphrody_desktop --bin aphrody-os` for the GPUI OS shell), install locally with `./script/install-linux`, or force X11 on Wayland with `WAYLAND_DISPLAY=''`.
- **macOS**: [`docs/src/development/macos.md`](./docs/src/development/macos.md) — requires `rustup`, Xcode + Command Line Tools, and `cmake`; run `cargo run -p zed` and optional visual tests (`cargo run -p zed --bin zed-visual-test-runner --features visual-tests`).
- **Windows**: [`docs/src/development/windows.md`](./docs/src/development/windows.md) — requires `rustup`, MSVC Build Tools + Spectre-mitigated libs, Windows 10/11 SDK, `cmake`, and Git/Windows long-path support. Do not override `.cargo/config.toml` `RUSTFLAGS` via environment variables (see [`.cargo/config.toml`](./.cargo/config.toml)); run `cargo run -p zed`.
- **FreeBSD (experimental)**: [`docs/src/development/freebsd.md`](./docs/src/development/freebsd.md) — `./script/freebsd`.
- **WebAssembly & WebGPU (`wasm32-unknown-unknown` / `wasm32-wasip2`)**:
  ```sh
  bun run web:matrix
  bun run web:check
  bun run web:build
  bun run web:verify
  cargo xtask web-examples
  ```
  Extension crates compile to `wasm32-wasip2` (pre-listed in [`rust-toolchain.toml`](./rust-toolchain.toml)); see [`docs/src/extensions/developing-extensions.md`](./docs/src/extensions/developing-extensions.md).

## Engineering & Coding Standards

- **Formatting**: Configured in [`rustfmt.toml`](./rustfmt.toml) (`edition = "2024"`, `style_edition = "2024"`). Format Rust with `cargo fmt --all` and verify with `cargo fmt --all -- --check`. Markdown/JSONC formatting uses [`.prettierrc`](./.prettierrc) (`./script/prettier`).
- **Rust & GPUI Rules ([`AGENTS.md`](./AGENTS.md))**:
  1. **No `mod.rs`**: Keep crate roots in `src/<crate_name>.rs` (`[lib] path = ...`) and submodules in `src/<module>.rs`.
  2. **Explicit Error Handling**: Avoid `.unwrap()`, `.expect()` (outside documented invariant tests), and silent `let _ = <fallible>` in core crates; propagate with `?`, `.log_err()`, or explicit `match`/`if let`.
  3. **Clippy & Disallowed APIs ([`clippy.toml`](./clippy.toml), [`Cargo.toml`](./Cargo.toml))**: Workspace lints deny `dbg_macro`, `todo`, `declare_interior_mutable_const`, `redundant_clone`, and `disallowed_methods` (e.g., use `smol::process::Command` instead of blocking `std::process::Command`, `gpui::BackgroundExecutor::timer` instead of `smol::Timer::after`, and `serde_json::from_slice` instead of `serde_json::from_reader`).
  4. **GPUI Concurrency & Hygiene**: Use full-word variable names, shadow clones cleanly in async blocks, never drop a `Task` without `.await`, `.detach()`, `.detach_and_log_err(cx)`, or storing it, and write comments only to explain non-obvious *why*.
  5. **Brand Consistency**: Never hardcode product strings (`Zed`/`Aphrody`); consume `brand::*` or run `cargo xtask brand check` (and `cargo xtask brand apply` after editing [`product.toml`](./product.toml) or [`assets/images/aphrody_logo.svg`](./assets/images/aphrody_logo.svg)).
  6. **Bun for TypeScript/TSX & Hooks**: Use `bun` (`bun test`, `bun check`, `bun fmt`, `bun build`) or `cargo run -p nsh-cli -- ...`, never `node`, `npm`, or `npx`.
  7. **GPU & Shader Verification**: Validate WGSL with the pinned Naga v30 (`cargo run -p aphrody_cli -- validate ...`) and verify backend changes across Vulkan, DirectX 12, and WebGPU ([`examples/gpu/`](./examples/gpu), [`docs/aphrody/gpu/`](./docs/aphrody/gpu)).
  8. **Modular Boundaries**:
     - Keep `gpui_ui` free of `settings_content`, `cloud_llm_client`, and `language_model_core`.
     - Keep `gpui_code --no-default-features` free of `buffer_diff`, `language`, `rpc`, and `proto`.
  9. **Workspace Conformity & New Crates**: Use workspace-inherited dependencies (`Cargo.toml [workspace.dependencies]`) and `[lints] workspace = true` (checked via `cargo xtask package-conformity`). Scaffold new crates with `./script/new-crate <name> [--apache]` so license symlinks and root module layout match workspace rules.

## Testing, Clippy, `xtask` & Verification Checklist

Run Clippy with warnings denied via the repo wrappers (which also run `cargo-shear`, `typos --config .config/typos.toml`, and `buf` locally on Unix when available):

```sh
# Linux / macOS:
./script/clippy            # or ./script/clippy -p <crate>

# Windows (PowerShell):
pwsh ./script/clippy.ps1   # or cargo xtask clippy [-p <crate>]
```

Run tests with Cargo or `cargo-nextest` ([`.config/nextest.toml`](./.config/nextest.toml)):

```sh
cargo test --workspace
# or:
cargo nextest run --workspace --no-fail-fast
```

Run core Aphrody checks before opening a PR:

```sh
cargo fmt --all -- --check
cargo xtask brand check
cargo xtask package-conformity
cargo test -j 4 -p brand -p xtask --bins
cargo check -j 4 -p gpui_ui --features nsh-full
cargo check -j 4 -p gpui_code --no-default-features
cargo check -j 4 -p gpui_app_shell --features nsh
cargo test -j 4 -p aphrody_cli --bins
cargo check -j 4 -p aphrody_cli --examples
bun run hygiene && bun run test:tooling
```

Additional repository checks: `cargo xtask licenses` / `./script/check-licenses` (requires Unix symlinks / `core.symlinks = true`), `./script/check-todos`, `./script/check-keymaps`, `./script/check-links`, and [`package.json`](./package.json) shortcuts (`bun run fmt:check`, `bun run clippy`, `bun run brand:check`, `bun run conformity`, `bun run licenses`).

## Debugging & Performance Profiling

- **Frame timings**: Set `ZED_MEASUREMENTS=1` in release mode to log frame durations to stderr, and compare captures with `./script/histogram version-a version-b`.
- **Microbenchmarks (`#[perf]`)**: Annotate unit benchmarks with `util_macros::perf` (`#[perf]`) and run `cargo perf-test -p <crate>` or `cargo perf-compare` (aliases in [`.cargo/config.toml`](./.cargo/config.toml)).
- **Debuggers & crashes**: Build with `[profile.dbg]` or `cargo run -p zed --config 'profile.release.debug="full"'` and attach `rust-gdb` / `rust-lldb`; see [`docs/src/development/debuggers.md`](./docs/src/development/debuggers.md) and [`docs/src/development/debugging-crashes.md`](./docs/src/development/debugging-crashes.md). On Windows, ETW profiles can be captured via `zed: record etw trace` (`crates/etw_tracing`).
- **Keychain bypass in dev builds**: Development builds skip system keychain prompts by default; set `ZED_DEVELOPMENT_USE_KEYCHAIN=1` when testing keychain integration.

## Pull Request Process

- Follow the [pull request template](./.github/pull_request_template.md?plain=1).
- **Build, run locally, and manually test your changes** before opening a PR; document tested platforms and concise reviewer verification steps.
- **Keep each PR about one thing**, leaving unrelated features and cleanups for separate PRs.
- **Visual & non-visual artifacts**: For UI changes, follow [`docs/src/development/ui-checklist.md`](./docs/src/development/ui-checklist.md) and attach before/after screenshots or video. For performance, GPU, or CLI changes, attach benchmarks, `gpui-wgpu` reports, or test output.
- **PR Titles & Release Notes** ([`docs/src/development/release-notes.md`](./docs/src/development/release-notes.md)):
  - Use a clear, capitalized, imperative PR title without conventional-commit prefixes (`fix:`, `feat:`, `docs:`, etc.) or trailing punctuation (optional crate scope prefix, e.g. `git_ui: Add history view`).
  - Include a final `Release Notes:` section with a blank line followed by `- Added ...`, `- Fixed ...`, `- Improved ...`, or `- N/A`. Any PR touching `crates/gpui*` must include a `- [GPUI] ...` entry (or `- [GPUI] N/A`).
- **README Self-Review Guard**: Leave lines 1–2 of [`README.md`](./README.md) intact during edits; human authors remove line 2 manually only to confirm final diff self-review.
- **Review etiquette**: Opening a PR does not guarantee a merge, and pinging maintainers by username or email does not raise review priority.

## Licensing & Contributor Agreement

Aphrody maintains the upstream Zed split license boundary:

- Application and editor crates are licensed under **GNU GPL v3.0-or-later** ([`LICENSE-GPL`](./LICENSE-GPL)).
- Reusable UI, SDK and WGPU CLI crates (`gpui`, `gpui_ui`, `aphrody_cli`, etc.) are licensed under **Apache-2.0** ([`LICENSE-APACHE`](./LICENSE-APACHE)) where their `Cargo.toml` says so; the desktop, session and installer crates are GPL-3.0-or-later.
- First-party crates must never use AGPL and must include a relative symlink (`../../LICENSE-GPL` or `../../LICENSE-APACHE`) matching `package.license` in `Cargo.toml` (enforced by `./script/check-licenses`, `cargo xtask licenses`, and `cargo xtask gpui`).

Contributions are accepted under an **inbound=outbound** policy (under the license of the modified crate) with **Developer Certificate of Origin (DCO)** sign-off (`git commit -s` / `Signed-off-by: Name <email>`), attesting that you have the right to submit the code under [`LICENSE-GPL`](./LICENSE-GPL) and [`LICENSE-APACHE`](./LICENSE-APACHE) without needing an external CLA portal.

## Cross-Compilation, Fleet & Release Channels

1. **Cross-compilation toolchain**:
   - `bun run cross:doctor` inspects `cargo-xwin`, `cargo-zigbuild`, `zig`, LLVM (`clang`, `lld-link`, `llvm-objdump`), Wasm tools, and `rustup` targets.
   - `bun run cross:matrix` and `bun run cross:check -- --target <triple> --package <crate>`.
2. **Fleet sync & verification**:
   - `bun run fleet:sync` and `bun run fleet:verify` synchronize and verify remote fleet builds (`fr1`) and tests/benchmarks (`fr2`).
3. **Six release tracks**: `lts`, `stable`, `preview`, `canary`, `nightly`, and `dev` (configured in [`product.toml`](./product.toml), [`crates/release_channel`](./crates/release_channel), [`.github/workflows/aphrody-release.yml`](./.github/workflows/aphrody-release.yml), and [`.cloudflare/aphrody-releases/`](./.cloudflare/aphrody-releases)).
4. **Repo hygiene & automated gates**: run `bun run hygiene && bun run test:tooling` before committing; never store durable scripts or plans outside the repository.

## Resources

- **Codebase overview**: [`docs/src/development.md`](./docs/src/development.md), [`docs/src/development/glossary.md`](./docs/src/development/glossary.md), [`README.md`](./README.md), and [`docs/aphrody/architecture/README.md`](./docs/aphrody/architecture/README.md)
- **UI/UX checklist & icons**: [`docs/src/development/ui-checklist.md`](./docs/src/development/ui-checklist.md) and [`crates/icons/README.md`](./crates/icons/README.md)
- **Platform build & Linux packaging notes**: [`docs/src/development/linux.md#notes-for-packaging-zed`](./docs/src/development/linux.md#notes-for-packaging-zed), [`macos.md`](./docs/src/development/macos.md), [`windows.md`](./docs/src/development/windows.md), [`freebsd.md`](./docs/src/development/freebsd.md)
- **Extensions**: [`docs/src/extensions/developing-extensions.md`](./docs/src/extensions/developing-extensions.md)
