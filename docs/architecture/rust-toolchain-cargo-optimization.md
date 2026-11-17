# Rust Toolchain, Cache, `fmt`, `check`, `clippy`, Linker & Cargo Optimization Guide

> End-to-end build velocity, linker, cache, lint-boundary, and toolchain-fork architecture tailored for the `aphrody` / GPUI / Zed monorepo (`302` workspace members, `2,069` resolved lockfile entries, `37` relative submodules). All tool versions, flags, crate releases, and upstream PRs documented here were verified live on `2026-11-16`.

---

## 1. Live Codebase & Host Baseline (`2026-11-16`)

### 1.1 Monorepo Graph Telemetry

Audited directly via [`tooling/bun/toolchain-audit.ts`](../../tooling/bun/toolchain-audit.ts) (`profile-diff` & `doctor`):

| Dimension | Live Observed State | Key Bottleneck / Action |
| :--- | :--- | :--- |
| **Workspace Scale** ([`Cargo.toml`](../../Cargo.toml)) | `302` members (`294` `crates/*`, `4` `extensions/*`, `4` `tooling/*`), `20` excluded `vendor/*`, `10` `default-members` | Switching between `gpui_ui`, `aphrody_desktop`, `nsh-cli`, and `zed` triggers feature-unification rebuilds without `cargo-hakari` or `[resolver] feature-unification`. |
| **`Cargo.lock` Footprint** ([`Cargo.lock`](../../Cargo.lock)) | `2,069` package entries across `1,804` unique crates; **`183` duplicated crate names** (`hashbrown` x5, `itertools` x5, `quick-xml` x5, `rand*` x5, `windows*` x5, `darling*` x4, `toml_edit` x4) | Deduplicate top offenders and unify features to cut duplicate LLVM bitcode & metadata emission. |
| **Pinned Toolchain** ([`rust-toolchain.toml`](../../rust-toolchain.toml)) | `1.98.1` (`rustc 1.98.1 48a229cea`, LLVM `22.1.8`, `cargo 1.98.1`, `clippy 0.1.98`, `rustfmt 1.9.0-stable`); locally `1.100.0` (LLVM `23.1.5`) & `1.102.0-nightly` are also installed | `1.98.1` accepts `-Zthreads=8`, while `1.100.0+` / `1.102.0-nightly` deprecates `-Zthreads` in favor of `--jobs-frontend=8`. System LLVM (`clang`/`lld-link` `23.1.5`) matches `1.100.0`. |
| **Active `.cargo/config.toml`** ([`.cargo/config.toml`](../../.cargo/config.toml)) | Sets `RUSTC_BOOTSTRAP = "1"`, `symbol-mangling-version=v0`, `--cfg tokio_unstable`, Windows `+crt-static`/`windows_slim_errors`, and `aarch64-unknown-linux-gnu` `-fuse-ld=lld` | Missing `x86_64-pc-windows-msvc` `lld`, `x86_64-unknown-linux-gnu` `mold`/`wild`, `build.build-dir` separation, `[cache] auto-clean-frequency`, and feature unification. |
| **Dev Profile** ([`Cargo.toml`](../../Cargo.toml)) | `[profile.dev]` sets `debug = "limited"`, `split-debuginfo = "unpacked"`, `codegen-units = 16`, `incremental = true`; `26` crates pinned at `opt-level = 3`, `37` single-file crates at `codegen-units = 1` | Upgrade `debug = "line-tables-only"` (`rust-lang/cargo#17518`), add `"*" = { debug = false }` under `[profile.dev.package]`, trim `build-override` debuginfo, and optimize `20` remaining heavy proc-macro/shader crates. |
| **Bundle Profile** ([`.cargo/bundle-config.toml`](../../.cargo/bundle-config.toml)) | Sets `opt-level = "z"` with core hot crates at `opt-level = 3` | Missing upstream `zed-industries/zed#63432` `-Z share-generics=y` (`-10.8%` / `-45.6 MB` binary size, `-36%` `drop_glue` instantiations). |
| **Local `sccache` State** | `sccache 0.19.0` active (`50 GiB` cap, `9 GiB` used), **`4.8%` hit rate** (`45` hits / `939` executed) | `SCCACHE_BASEDIRS=C:\labs;C:\aphrody` omits `C:\src\aphrody`; `incremental` (`523` calls) bypasses cache; `sccache 0.19.0` lacks `mozilla/sccache#3026` (`web-sys` Windows `@rsp` fix). |
| **Lint Boundary** ([`clippy.toml`](../../clippy.toml)) | `disallowed_methods = "deny"` bans `std::process::Command::*` in favor of `smol::process::Command::*` | **19 `std::process::Command::` sites** across `nsh-cli`, `nsh-core`, `nsh-platform-linux`, and `nsh-platform-macos` inherit `[lints] workspace = true` without depending on `smol`. |

### 1.2 Live Ecosystem Version Matrix (`crates.io` & GitHub)

| Tool / Crate | Local Host (`win32`) | Latest Live (`crates.io` / GitHub) | Role in Aphrody Pipeline |
| :--- | :--- | :--- | :--- |
| **`sccache`** | `0.19.0` | `0.19.0` (`mozilla/sccache` `v0.19.0`; `#3026` merged `2026-11-14`) | Local disk + Cloudflare R2 compilation cache |
| **`cargo-nextest`** | `0.9.151` | `0.9.152` (`nextest-rs/nextest`, `2026-11-13`) | Parallel partitioned test runner ([`.config/nextest.toml`](../../.config/nextest.toml)) |
| **`cargo-hakari`** | Not installed | `0.9.39` (`guppy-rs/guppy`, `2026-09-19`) | Stable workspace-hack feature unification generator |
| **`cargo-sweep`** | `0.8.0` | `0.8.0` (`holmgr/cargo-sweep`, `2025-10-11`) | Time/toolchain-based `target/` artifact GC |
| **`cargo-cache`** | Not installed | `0.8.3` (`2022-09-11`) | Inspects/prunes `~/.cargo` registry/git caches |
| **`cargo-deny`** | `0.20.2` | `0.21.0` (`EmbarkStudios/cargo-deny`, `2026-11-16`) | License, CVE, duplicate & source origin checks |
| **`cargo-machete`** | `0.10.0` | `0.10.1` (`bnjbvr/cargo-machete`, `2026-11-15`) | Sub-second heuristic unused dependency scanner |
| **`cargo-udeps`** | Not installed | `0.1.61` (`est31/cargo-udeps`, `2026-04-29`) | Nightly compiler-accurate unused dependency audit |
| **`cargo-shear`** | `1.14.1` | `1.14.1` (`2026-11-13`) | Workspace unused dep detector used in [`script/clippy`](../../script/clippy) |
| **`cargo-dylint`** | Not installed | `6.4.0` (`2026-11-10`) | Runs custom GPUI lints in [`tooling/lints/`](../../tooling/lints/Cargo.toml) |
| **`wild-linker`** | Not installed | `0.11.0` (`wild-linker/wild`, `2026-10-24`; COFF `#2975` `2026-11-15`) | Ultra-fast Linux ELF & `wasm32` linker (COFF WIP on `main`) |
| **`mold`** | Linux (`fr1`/`fr2`) | `v3.0.0` (`rui314/mold`, `2026-10-05`) | Production-ready fast parallel ELF linker for Linux |
| **`lld-link` / `rust-lld`** | `23.1.5` + bundled | LLVM `23.1.5` / sysroot `rust-lld.exe` | Default fast COFF linker for Windows MSVC |
| **`rustc_codegen_cranelift`** | Installed on `nightly` | `dev` (`rust-lang/rustc_codegen_cranelift`, `2026-11-13`) | Fast debug machine-code backend (primary: Linux; experimental: Windows MSVC) |

---

## 2. Cargo Profiles, Feature Unification, and IR Bloat Engineering

### 2.1 Profile Tuning (`line-tables-only`, Zero Dep Debuginfo, Proc-Macro `opt-level = 3`)

1. **`debug = "line-tables-only"` (Cargo PR `#17518`, merged `2026-11-14`)**:
   - `rustc 1.98.1` supports `-C debuginfo=line-tables-only` (MSRV `1.71`), generating minimal file/line tables for panic backtraces while skipping module/type DWARF and PDB tables emitted by `"limited"`.
   - Keep `[profile.dbg]` (`inherits = "dev"`, `debug = "full"`) for interactive lldb/WinDbg/CodeLLDB sessions.
2. **Strip Third-Party Debuginfo (`"*" = { debug = false }`)**:
   - Cargo's official build performance guide (`doc/book/src/guide/build-performance.md`) explicitly pairs `debug = "line-tables-only"` with `[profile.dev.package] "*" = { debug = false }` so Cargo avoids writing debug info for ~1,800 upstream crates while keeping backtrace lines for workspace crates.
   - Set `[profile.dev.build-override] debug = false` so proc-macros and `build.rs` scripts stop emitting unused debug symbols.
3. **Complete Proc-Macro & Heavy Core `opt-level = 3` Overrides**:
   - [`Cargo.toml`](../../Cargo.toml) already optimizes `gpui_macros`, `ui_macros`, `nsh-macros`, `syn`, `quote`, `proc-macro2`, `taffy`, `resvg`, `wasmtime`, and `cranelift-codegen`.
   - Profiling `Cargo.lock` shows 20 additional heavy proc-macros, shader compilers, compression libraries, and crypto crates still compiled at `opt-level = 0` in `dev` builds:
     - Derive macros: `serde_derive`, `thiserror-impl`, `tokio-macros`, `strum_macros`, `clap_derive`, `schemars_derive`, `derive_more-impl`, `zerocopy-derive`, `async-trait`, `pin-project-internal`, `bytemuck_derive`, `wasm-bindgen-macro`
     - Heavy runtime/build libs: `naga`, `wgpu-core`, `flate2`, `zstd-sys`, `ring`, `aws-lc-rs`, `image`, `pulldown-cmark`

### 2.2 Workspace Feature Unification (`cargo-hakari` vs `[resolver] feature-unification`)

Without feature unification across 302 workspace members, Cargo recomputes dependency feature sets per command (`gpui_ui`, `aphrody_desktop`, `nsh-cli`, `zed`, `--tests`), repeatedly rebuilding `nsh-core`, `gpui`, `tokio`, and `windows`/`windows-sys`:

- **Strategy A — Native `[resolver] feature-unification = "workspace"` (`-Zfeature-unification`, Cargo `#15157`)**:
  - Works immediately because [`.cargo/config.toml`](../../.cargo/config.toml) already sets `RUSTC_BOOTSTRAP = "1"`.
  - Zero edits to 302 member `Cargo.toml` files; unifies features workspace-wide for every `cargo check`, `cargo build`, and `cargo test`.
  - **Caveat**: A feature enabled by crate A can satisfy crate B's missing feature declaration locally; retain per-crate `--no-default-features` / `--features` CI checks in [`.github/workflows/aphrody-ci.yml`](../../.github/workflows/aphrody-ci.yml) (`gpui_code --no-default-features`, `gpui_ui --features nsh-full`).
- **Strategy B — `cargo-hakari 0.9.39` (`crates/workspace-hack`)**:
  - Works on pure stable Cargo without `RUSTC_BOOTSTRAP = "1"`.
  - Initialize via:
    ```sh
    cargo install --locked cargo-hakari --version 0.9.39
    cargo hakari init crates/workspace-hack
    cargo hakari generate
    cargo hakari manage-deps
    ```
  - **Windows Note**: Ensure `.gitattributes` keeps `crates/workspace-hack/Cargo.toml text eol=lf` (`cargo-hakari` rejects CRLF on Windows).

### 2.3 Binary Size & LLVM IR Bloat Reduction

- **Restore `-Z share-generics=y` in [`.cargo/bundle-config.toml`](../../.cargo/bundle-config.toml)**:
  - Upstream `zed-industries/zed#63432` proved `-Z share-generics=y` shrinks release bundles by **10.8% (`-45.6 MB`)** and cuts `drop_glue` instantiations from `101,778` to `65,041` (**-36%**).
- **Measure & Refactor Monomorphization Hotspots (`zed#63232`, `#63426`, `#63882`, `#62059`)**:
  - Inspect IR line counts before refactoring generic UI/settings builders:
    ```sh
    cargo install --locked cargo-llvm-lines
    cargo llvm-lines -p gpui_ui | head -n 35
    cargo build -p zed --timings
    node script/cargo-timing-info.js target/cargo-timings/cargo-timing-*.html
    ```
  - Use non-generic inner functions + `#[inline(never)]` boundaries behind thin generic wrappers in `gpui_ui` / `aphrody_ui` / `settings_content`.

### 2.4 `build-dir` Separation, Cache GC & Fine-Grain Locking

- **Separate Intermediates from Final Outputs (`build.build-dir`)**:
  - Cargo stabilizes `[build] build-dir` with `{workspace-root}`, `{cargo-cache-home}`, and `{workspace-path-hash}` templates:
    ```toml
    [build]
    build-dir = "{workspace-root}/target/build"
    ```
  - Keeps `target/debug` and `target/release` uncluttered for packagers while making it trivial to redirect `CARGO_BUILD_BUILD_DIR` onto a Dev Drive (`Windows`) or `tmpfs`/NVMe scratch (`/srv/xbuild/cargo-build/{workspace-path-hash}` on `fr1`).
  - Watch for upstream `rust-lang/cargo#17512` (cross-workspace shared storage under `$CARGO_HOME/shared-storage`) and `#17183` (long `build-dir` path edge case fixed before stabilization).
- **Automatic Cache Garbage Collection**:
  - Set `[cache] auto-clean-frequency = "1 day"` in [`.cargo/config.toml`](../../.cargo/config.toml).
  - Run `cargo sweep --time 14` on Windows and `fr1`/`fr2` weekly (or `cargo sweep --installed` after toolchain bumps) instead of wiping `target/` wholesale unless crossing CI disk caps (`script/clear-target-dir-if-larger-than.ps1`).
  - With `RUSTC_BOOTSTRAP = "1"`, CI fleet builders can also pass `CARGO_UNSTABLE_FINE_GRAIN_LOCKING=true` (`rust-lang/cargo#16659`) and `CARGO_UNSTABLE_GC=true` (`rust-lang/cargo#17524`).

---

## 3. Platform Recipes: Windows, Linux (`fr1`/`fr2`), and macOS

### 3.1 Windows (`x86_64-pc-windows-msvc`)

1. **Fast Linker (`rust-lld.exe` / `lld-link 23.1.5`)**:
   - Prefer Cargo's built-in `-C linker-features=+lld` (uses sysroot `rust-lld.exe` cleanly with zero external PATH dependency) or explicit `linker = "lld-link"` (`C:\Program Files\LLVM\bin\lld-link.exe` `23.1.5`), preserving `--cfg windows_slim_errors` and `-C target-feature=+crt-static` required by `livekit`/`webrtc-sys`.
2. **ReFS Dev Drive + Defender Performance Mode**:
   - The local Windows volume `C:` is currently `NTFS` (`54.5 GB` free). Even with Defender path exclusions (`C:\src`, `C:\Users\aphro\.cargo`, `C:\Users\aphro\.rustup`, `C:\Users\aphro\AppData\Local\Mozilla\sccache`), creating a ReFS Dev Drive (e.g., `D:` VHDX >= `120 GB`) and relocating `CARGO_HOME`, `CARGO_TARGET_DIR`, `CARGO_BUILD_BUILD_DIR`, and `SCCACHE_DIR` onto `D:` speeds up Cargo's metadata/rlib I/O and enables ReFS copy-on-write cloning:
     ```powershell
     # PowerShell (Admin) — mount a 150 GB Dev Drive VHDX on D: and enable Dev Drive Defender async mode
     New-VHD -Path C:\devdrive.vhdx -SizeBytes 150GB -Dynamic | Mount-VHD -Passthru | Initialize-Disk -PartitionStyle GPT -PassThru | New-Partition -UseMaximumSize -DriveLetter D | Format-Volume -FileSystem ReFS -DevDrive -NewFileSystemLabel "AphrodyDev" -Confirm:$false
     fsutil devdrv trust D:
     ```
3. **Fixing Local `sccache` Hit Rate (`4.8%` -> `> 80%`) & `web-sys` Windows Limit**:
   - **Root Cause 1 (`SCCACHE_BASEDIRS`)**: Global env currently sets `SCCACHE_BASEDIRS=C:\labs;C:\aphrody`, excluding `C:\src\aphrody`! Update user env to include `C:\src`:
     ```powershell
     [Environment]::SetEnvironmentVariable("SCCACHE_BASEDIRS", "C:\src;C:\labs;C:\aphrody", "User")
     [Environment]::SetEnvironmentVariable("SCCACHE_ALLOW_INCREMENTAL", "1", "User")
     [Environment]::SetEnvironmentVariable("SCCACHE_BASEDIRS_VERBATIM_ENV_DEPS", "CARGO_MANIFEST_DIR,CARGO_MANIFEST_PATH", "User")
     sccache --stop-server
     ```
   - **Root Cause 2 (`incremental = true` vs CI `CARGO_INCREMENTAL=0`)**: Setting `SCCACHE_ALLOW_INCREMENTAL=1` lets `sccache` cache all upstream registry/git crates while skipping incremental workspace members locally; in CI / cold builds, export `CARGO_INCREMENTAL=0` so workspace crates also hit L0 disk + Cloudflare R2 L1 (`zed#61108`).
   - **Root Cause 3 (`web-sys` 32 KiB `CreateProcessW` `os error 206`)**: `sccache 0.19.0` expands `@rsp` response files before spawning `rustc`, overflowing Windows' `32,767`-char command-line cap when compiling `web-sys` (~`1,500` features = `43,897` chars). **Keep `RUSTC_WRAPPER=""` for `wasm32-unknown-unknown`** in [`tooling/bun/web-build.ts`](../../tooling/bun/web-build.ts) and [`tooling/xtask/src/tasks/web_examples.rs`](../../tooling/xtask/src/tasks/web_examples.rs) until deploying `sccache` built from `mozilla/sccache` commit `91d34f1094` (PR `#3026`, merged `2026-11-14`) or `v0.20.0+`.
4. **Global `~/.cargo/config.toml` Hygiene**:
   - Remove `-C target-cpu=native` (breaks binary portability & poisons shared R2 `sccache` keys) and `-Zthreads=8` (deprecated on `rustc 1.100.0+` / `nightly`) from user-global `~/.cargo/config.toml`. Apply parallel frontend flags conditionally per toolchain version (`-Zthreads=8` on `1.98.1`, `--jobs-frontend=8` on `1.100.0+`).

### 3.2 Linux Fleet (`fr1` Builds, `fr2` Tests/Bench, `ubuntu-latest` CI)

Per fleet role separation ([`tooling/bun/fleet-sync.ts`](../../tooling/bun/fleet-sync.ts) and [Cross-Compilation Guide](./cross-compile-and-depot-tools.md)):
- **`fr1` (Heavy Builds & Bundling)**:
  - Install `mold 3.0.0` + `wild 0.11.0` (`cargo install --locked wild-linker --version 0.11.0` or system package):
    ```toml
    [target.x86_64-unknown-linux-gnu]
    rustflags = ["-C", "link-arg=-fuse-ld=mold"]
    # Or experimental ultra-fast Wild linker on Linux x86_64/aarch64:
    # rustflags = ["-C", "linker=clang", "-C", "link-arg=--ld-path=wild"]
    ```
  - Mount fast NVMe or `tmpfs` build dir for ephemeral CI/fleet builds (`CARGO_BUILD_BUILD_DIR=/srv/xbuild/cargo-build/{workspace-path-hash}`), paired with `sccache` local NVMe disk + R2 L1 (`SCCACHE_CACHE_SIZE=100G`).
  - Enable parallel frontend (`RUSTFLAGS="-Zthreads=8"` on `1.98.1`, or `--jobs-frontend=8` on `1.100.0+`) for multi-core EPYC/Xeon nodes (`-Zthreads=8` is the Cargo-recommended ceiling for diminishing returns).
- **`fr2` (Partitioned Tests & Benchmarks)**:
  - Run `cargo nextest run --workspace --profile ci` (honors sequential DB locks and slow test priorities in [`.config/nextest.toml`](../../.config/nextest.toml)) and `cargo perf-test` / `cargo perf-compare` for runtime & compile regression gates.
  - Note: `wild 0.11.0` supports `x86_64`/`aarch64`/`riscv64`/`loongarch64`/`ppc64le` Linux ELF and `wasm32`, while Windows COFF (`#2975`, `#2979`) and macOS Mach-O are still under active development on `wild-linker/wild` `main`. Use `lld-link`/`rust-lld` on Windows and `mold` (production) or `wild` (iterative ELF/Wasm) on Linux.

### 3.3 macOS (`aarch64-apple-darwin` & `x86_64-apple-darwin`)

- Retain `MACOSX_DEPLOYMENT_TARGET = "12.0"` in [`.cargo/config.toml`](../../.cargo/config.toml) and `split-debuginfo = "unpacked"` in [`Cargo.toml`](../../Cargo.toml) (skips expensive `dsymutil` runs during local dev).
- Xcode 15+/26+ ships Apple's `ld_prime` by default (substantially faster than legacy `ld64`); optionally evaluate Homebrew LLVM `ld64.lld` (`-C link-arg=-fuse-ld=/opt/homebrew/opt/llvm/bin/ld64.lld`) for pure Rust test binaries, keeping Apple `ld` for final `.app` notarization & Swift/Metal system framework links.
- Set `RUST_LIB_BACKTRACE=0` in test runs to avoid DWARF symbolication stalls in GPUI test harnesses (matching [`tooling/xtask/src/tasks/workflows/run_tests.rs`](../../tooling/xtask/src/tasks/workflows/run_tests.rs)).

---

## 4. Fast `fmt`, `check`, `clippy`, `dylint`, and `rustc_codegen_cranelift`

### 4.1 Sub-Second `fmt` & Targeted `check`

- **Formatter ([`rustfmt.toml`](../../rustfmt.toml))**: Pins `edition = "2024"` and `style_edition = "2024"`. For inner-loop iteration, format only modified Rust files instead of scanning all 302 crates:
  ```sh
  # Changed files vs main (Unix/Git Bash)
  git diff --name-only --diff-filter=AM origin/main -- "*.rs" | xargs -r rustfmt --edition 2024 --check
  # Full CI check (matches package.json "fmt:check")
  cargo fmt --all -- --check
  ```
- **Targeted `cargo check` & Parallel Worktrees**:
  - Bare `cargo check` uses the 10 `default-members` (`gpui_wgpu`, `gpui_web`, `gpui_ui`, `gpui_code`, `aphrody_store`, `aphrody_session`, `aphrody_craft`, `aphrody_ui`, `aphrody_desktop`, `aphrody_cli`) instead of all 302 members.
  - Use [`tooling/bun/worktree.ts`](../../tooling/bun/worktree.ts) (`bun run worktree:create -- --id <name>`) so parallel agents get separate `CARGO_TARGET_DIR` paths (`../target-worktrees/aphrody-<id>`) and never stall on Cargo file locks (`rust-lang/cargo#16804`).

### 4.2 Fast `clippy` & the `nsh-core` / `nsh-*` Lint Boundary

1. **Why `cargo xtask clippy` Is Slow**:
   - Both [`tooling/xtask/src/tasks/clippy.rs`](../../tooling/xtask/src/tasks/clippy.rs) and [`script/clippy`](../../script/clippy) / [`script/clippy.ps1`](../../script/clippy.ps1) run:
     ```sh
     cargo clippy --workspace --release --all-targets --all-features -- --deny warnings
     ```
   - Passing `--release --all-targets --all-features` across 302 members forces LLVM release codegen/optimization passes for proc-macros, tests, benchmarks, and optional feature matrices simultaneously.
   - **Fast Local Inner Loop** (check modified crates without re-linting 1,800 upstream crates):
     ```sh
     cargo clippy -p gpui_ui -p aphrody_desktop --no-deps --all-targets -- -D warnings
     cargo shear --locked --deny-warnings
     ```
2. **Audited `nsh-core` / `nsh-*` Lint Boundary Conflict**:
   - Root [`Cargo.toml`](../../Cargo.toml) sets `[workspace.lints.clippy] disallowed_methods = "deny"`, paired with [`clippy.toml`](../../clippy.toml) banning `std::process::Command::spawn`, `output`, `status`, `stdin`, `stdout`, and `stderr` with replacement `smol::process::Command::*`.
   - Every `nsh-*` crate (including [`crates/nsh-core/Cargo.toml`](../../crates/nsh-core/Cargo.toml) and [`crates/nsh-cli/Cargo.toml`](../../crates/nsh-cli/Cargo.toml)) sets `[lints] workspace = true`, yet **none** of the 30 `crates/nsh-*` crates depend on `smol` (they use `tokio` and `std::thread`).
   - Live AST scan found **19 `std::process::Command::` call sites** across `nsh-cli` (`10` sites in `bindgen.rs`, `package.rs`, `rust_app.rs`, `self_update.rs`), `nsh-core` (`2` test/runtime sites in `process.rs` & `websocket/native.rs`), `nsh-platform-linux` (`5` sites), and `nsh-platform-macos` (`2` sites).
   - **Clean Boundary Fix**:
     - For `nsh-core` / `nsh-cli` / `nsh-platform-*` sites that run inside `tokio` or synchronous CLI/test harnesses without `smol`, annotate with `#[allow(clippy::disallowed_methods, reason = "nsh uses tokio/std, not smol")]` (following the exact pattern used in [`tooling/xtask/src/tasks/clippy.rs`](../../tooling/xtask/src/tasks/clippy.rs)), OR group `nsh-cli` tests under crate-level `#![cfg_attr(test, allow(clippy::disallowed_methods))]`.
3. **Custom Dylint Workspace ([`tooling/lints/`](../../tooling/lints/Cargo.toml))**:
   - Kept deliberately outside root `[workspace]` because `clippy_utils` pins `nightly-2026-03-21` ([`tooling/lints/rust-toolchain.toml`](../../tooling/lints/rust-toolchain.toml)).
   - Run independently:
     ```sh
     cargo install --locked cargo-dylint dylint-link
     cargo dylint --all
     cargo test --manifest-path tooling/lints/Cargo.toml
     ```

### 4.3 `rustc_codegen_cranelift` (`cg_clif`) Hybrid Dev Workflow

`rustc-codegen-cranelift-x86_64-pc-windows-msvc` is already installed on the host's `nightly` toolchain (`1.102.0-nightly`):

- **Linux (`fr1` / `x86_64` & `aarch64`) — Recommended Fast Dev Iteration**:
  ```sh
  rustup component add rustc-codegen-cranelift-preview --toolchain nightly
  CARGO_PROFILE_DEV_CODEGEN_BACKEND=cranelift \
    cargo +nightly build -Zcodegen-backend -p gpui_ui -p aphrody_desktop
  ```
- **Hybrid Profile (Cranelift for Workspace, LLVM for `opt-level = 3` Deps)**:
  ```toml
  [unstable]
  codegen-backend = true

  [profile.dev]
  codegen-backend = "cranelift"

  [profile.dev.package."*"]
  # Keep heavy math/shader/proc-macro deps on LLVM when opt-level = 3 is active
  ```
- **Windows MSVC Caveats**: While COFF TLS and stack probing landed upstream, `cg_clif` on `x86_64-pc-windows-msvc` remains less battle-tested for panic unwinding and MSVC inline assembly (`ring`/`webrtc-sys`). Keep `llvm` as default on Windows and gate `cg_clif` to opt-in local checks or Linux `fr1` builds.

---

## 5. Forking & Upstream Auto-Integration (`cargo`, `rust-clippy`, `wild`, `sccache`)

### 5.1 Aphrody Org & Relative Submodule Convention

Verified via `gh repo list Aphrody-Dev`, `gh repo list aphrody-labs`, and [`.gitmodules`](../../.gitmodules):
- Primary monorepo: `aphrody-dev/aphrody` (fork of `zed-industries/zed`).
- Upstream dependency & tool forks live under `https://github.com/aphrody-labs/<tool>` and mount as **relative submodules** (`url = ../../aphrody-labs/<tool>.git`) under `vendor/*` (excluded from root `[workspace]` in [`Cargo.toml`](../../Cargo.toml) and fetched selectively with `--depth 1 --jobs 8` via [`tooling/bun/submodules.ts`](../../tooling/bun/submodules.ts) and [`.github/actions/vendor-submodules/action.yml`](../../.github/actions/vendor-submodules/action.yml)).
- None of `cargo`, `rust-clippy`, `wild`, or `sccache` are forked in `aphrody-labs` yet.

### 5.2 Declarative Toolchain Fork Plan

Run `bun tooling/bun/toolchain-audit.ts fork-plan` for live JSON specs:

| Fork (`aphrody-labs/*`) | Upstream & Default Branch | Submodule Path | Priority Cherry-Picks / Patches |
| :--- | :--- | :--- | :--- |
| **`aphrody-labs/sccache`** | `mozilla/sccache` (`main`) | `vendor/toolchain/sccache` | **`#3026`** (`2026-11-14`, `@rsp` response file spawning fixing `web-sys` Windows `os error 206`) + **`#3022`** (`SCCACHE_BASEDIRS_VERBATIM_ENV_DEPS`) |
| **`aphrody-labs/cargo`** | `rust-lang/cargo` (`master`) | `vendor/toolchain/cargo` | **`#17518`** (`debug=line-tables-only` default), **`#17512`** (`shared-storage` cross-workspace artifact dedup), **`#17524`** (`cargo clean gc --max-target-age`), **`#16659`** (parallel `-Zfine-grain-locking`) |
| **`aphrody-labs/wild`** | `wild-linker/wild` (`main`) | `vendor/toolchain/wild` | **`#2975` & `#2979`** (`2026-11-15`, Windows PE/COFF `.CRT$` linking) + **`#2980` & `#2977`** (`wasm32` `--soname` / `--import-table`) |
| **`aphrody-labs/rust-clippy`** | `rust-lang/rust-clippy` (`main`) | `vendor/toolchain/rust-clippy` | Pin `clippy_utils` cleanly for [`tooling/lints/`](../../tooling/lints/Cargo.toml) alongside `nsh-*` lint exemptions |

### 5.3 Automated Upstream Rebase / Cherry-Pick & Regression Gate Pipeline

Extend [`.github/workflows/aphrody-upstream-sync.yml`](../../.github/workflows/aphrody-upstream-sync.yml) with a scheduled `sync-toolchain-forks` job (`cron: '0 4 * * *'`):

1. **Fork Bootstrap (one-time per tool)**:
   ```sh
   gh repo fork mozilla/sccache --org aphrody-labs --clone=false
   git submodule add -b main ../../aphrody-labs/sccache.git vendor/toolchain/sccache
   ```
   Add `vendor/toolchain/cargo`, `vendor/toolchain/rust-clippy`, `vendor/toolchain/wild`, and `vendor/toolchain/sccache` to `[workspace.exclude]` in [`Cargo.toml`](../../Cargo.toml).
2. **Automated Sync & Cherry-Pick Loop** (matching [`aphrody-auto-merge.yml`](../../.github/workflows/aphrody-auto-merge.yml)):
   - Fetch `upstream/<default-branch>`, reset/rebase `aphrody/patched` onto `upstream`, and apply declarative `cherry-pick` commit lists (or open an `upstream-sync` issue automatically on merge conflicts, matching the existing `sync-zed-and-wgpu-digest` pattern).
   - Build tool binaries on `fr1` / CI (`cargo build --manifest-path vendor/toolchain/<tool>/Cargo.toml --release`) and publish internal artifacts / R2 cache drops.
3. **Mandatory Verification & Perf Regression Gates**:
   - Repository hygiene & doc links: `bun run hygiene` ([`tooling/bun/repo-hygiene.ts`](../../tooling/bun/repo-hygiene.ts))
   - Tooling tests: `bun run test:tooling`
   - Brand & config checks: `cargo xtask brand check`
   - Runtime & compile benchmarks ([`tooling/perf/src/main.rs`](../../tooling/perf/src/main.rs)):
     ```sh
     cargo perf-test -p gpui -- --json=baseline --quiet
     # Apply toolchain/profile update, then compare:
     cargo perf-test -p gpui -- --json=candidate --quiet
     cargo perf-compare candidate baseline
     ```
   - Link & incremental compile timing gate: `cargo check --timings` + `hyperfine` before merging any toolchain PR.

---

## 6. Recommended Patches for Coordinator Review

> **Note**: Apply these exact diffs after active workspace Cargo build locks clear. Verify TOML/JSON syntax anytime via `bun tooling/bun/toolchain-audit.ts profile-diff`.

### 6.1 [`.cargo/config.toml`](../../.cargo/config.toml)

```diff
+[build]
+build-dir = "{workspace-root}/target/build"
+
+[cache]
+auto-clean-frequency = "1 day"
+
+[resolver]
+# Requires RUSTC_BOOTSTRAP = "1" (already set under [env]); use cargo-hakari on pure stable
+feature-unification = "workspace"
+
 [target.'cfg(all())']
 # v0 mangling scheme provides more detailed backtraces around closures
 rustflags = ["-C", "symbol-mangling-version=v0", "--cfg", "tokio_unstable"]
@@ -14,6 +24,13 @@ rustflags = [
     "windows_slim_errors",        # This cfg will reduce the size of `windows::core::Error` from 16 bytes to 4 bytes
     "-C",
     "target-feature=+crt-static", # This fixes the linking issue when compiling livekit on Windows
+    "-C",
+    "linker-features=+lld",       # Use bundled rust-lld.exe on Windows MSVC for faster PDB links
+]
+
+[target.x86_64-unknown-linux-gnu]
+rustflags = [
+    "-C",
+    "link-arg=-fuse-ld=mold",     # Use parallel mold linker on Linux x86_64 (fr1/fr2/CI)
 ]
```

### 6.2 [`.cargo/bundle-config.toml`](../../.cargo/bundle-config.toml)

```diff
+[env]
+RUSTC_BOOTSTRAP = "1"
+
+[build]
+rustflags = ["-Z", "share-generics=y"]
+
 [profile.release]
 opt-level = "z"
```

### 6.3 [`Cargo.toml`](../../Cargo.toml) (`[profile.dev*]` & `[workspace.exclude]`)

```diff
 [profile.dev]
 split-debuginfo = "unpacked"
 incremental = true
 codegen-units = 16
-debug = "limited"
+debug = "line-tables-only"
 
 # mirror configuration for crates compiled for the build platform
 # (without this cargo will compile ~400 crates twice)
 [profile.dev.build-override]
 codegen-units = 16
 split-debuginfo = "unpacked"
-debug = "limited"
+debug = false
 
 # "debug" is a reserved profile name.
 [profile.dbg]
@@ -1552,6 +1552,7 @@ debug = "full"
 debug = "full"
 
 [profile.dev.package]
+"*" = { debug = false }
 # proc-macros start
 gpui_macros = { opt-level = 3 }
 derive_refineable = { opt-level = 3 }
@@ -1564,6 +1565,25 @@ quote = { opt-level = 3 }
 syn = { opt-level = 3 }
 proc-macro2 = { opt-level = 3 }
 # proc-macros end
+serde_derive = { opt-level = 3 }
+thiserror-impl = { opt-level = 3 }
+tokio-macros = { opt-level = 3 }
+strum_macros = { opt-level = 3 }
+clap_derive = { opt-level = 3 }
+schemars_derive = { opt-level = 3 }
+derive_more-impl = { opt-level = 3 }
+zerocopy-derive = { opt-level = 3 }
+async-trait = { opt-level = 3 }
+pin-project-internal = { opt-level = 3 }
+bytemuck_derive = { opt-level = 3 }
+wasm-bindgen-macro = { opt-level = 3 }
+naga = { opt-level = 3 }
+wgpu-core = { opt-level = 3 }
+flate2 = { opt-level = 3 }
+zstd-sys = { opt-level = 3 }
+ring = { opt-level = 3 }
+aws-lc-rs = { opt-level = 3 }
+image = { opt-level = 3 }
+pulldown-cmark = { opt-level = 3 }
```

### 6.4 [`rust-toolchain.toml`](../../rust-toolchain.toml) (Optional Upgrade Decision)

```diff
 [toolchain]
-channel = "1.98.1"
+# Verified locally installed with LLVM 23.1.5 matching system clang/lld-link 23.1.5;
+# keep "1.98.1" if tracking zed-industries/zed verbatim without divergence.
+channel = "1.100.0"
```

### 6.5 [`package.json`](../../package.json) & [`docs/architecture/README.md`](README.md)

```diff
     "hygiene": "bun tooling/bun/repo-hygiene.ts",
+    "toolchain:doctor": "bun tooling/bun/toolchain-audit.ts doctor",
+    "toolchain:profile-diff": "bun tooling/bun/toolchain-audit.ts profile-diff",
+    "toolchain:fork-plan": "bun tooling/bun/toolchain-audit.ts fork-plan",
+    "clippy:fast": "cargo clippy -p gpui_ui -p gpui_code -p aphrody_desktop --no-deps --all-targets -- -D warnings",
     "test:tooling": "bun test tooling/bun"
```
