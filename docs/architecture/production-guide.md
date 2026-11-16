# Aphrody Production-Ready Monorepo Architecture & Hardening Guide

> Synthesized from codebase telemetry (`C:\src\aphrody` root workspace, `Cargo.lock`, `.gitmodules`), live cross-platform test verification (Windows, Linux Lavapipe, Chrome Headless WebGPU, Android `aarch64-linux-android` check), and primary upstream references (2025–2026).

---

## 1. Verified Monorepo Baseline & Telemetry

| Dimension | Current Observed State | Production Target |
| :--- | :--- | :--- |
| **Workspace Scale** | `301` member crates, `20` excluded `vendor/*` directories, `default-members = 7` | Strict 6-tier one-way DAG validated via `cargo xtask check-layers` |
| **Submodules** | `37` relative submodules (`../../aphrody-labs/*.git`) in [`.gitmodules`](../../.gitmodules); `6` Cargo path-referenced (`taffy`, `wgpu`, `cosmic-text`, `accesskit`, `craft-libs`, `tauri/wry`) | Parallel `--jobs 8` shallow/blobless checkout via [`tooling/bun/submodules.ts`](../../tooling/bun/submodules.ts) |
| **`Cargo.lock` Graph** | `2,063` package entries across `1,801` unique crates; **`180` duplicated crate names** (`hashbrown`, `itertools`, `quick-xml`, `rand*`, `windows*` x5; `darling*`, `toml_edit` x4) | Eliminate top-8 duplicates + unify features with `cargo-hakari` |
| **Converged Core Stack** | `wgpu 30.0.0`, `taffy 0.15.1`, `cosmic-text 0.19.0`, `accesskit 0.25.2`, `wry 0.57.2`, `wasm-bindgen 0.2.129`, `wasmtime 48.0.7`, `rustls 0.23.46`, `quickjs-jit 0.14.2`, `agent-client-protocol 5.3.0` | Automated `cargo-deny` + `cargo-semver-checks` on public SDK crates |
| **Web (`wasm32`) Footprint** | `os_desktop` (`web_os`, debug + `wasm-opt -Oz`): `23,452,112` B raw (`simd-baseline`), **`6,388,025` B Brotli** | `< 4.5 MB` raw / **`<= 1.5 MB` Brotli** (`[profile.web]` + decoupled WOFF2 fonts & audio) |
| **120 FPS Frame Loop** | `DEFAULT_TARGET_FPS = 120` (`8.33 ms` budget), `InputRateTracker`, Mesa Lavapipe / D3D12 / `BrowserWebGpu` verified | `wgpu::PipelineCache` + non-blocking `DeviceLost` recovery + atlas LRU eviction |

---

## 2. Six-Tier Workspace Architecture & Dependency Invariants

```text
Tier 5: Cloudflare Edge & Distribution (.cloudflare/, docs.aphrody.com, releases.aphrody.com)
   └── Tier 4: Heavy IDE & AI Apps (crates/zed, editor, collab, workspace, agent, acp_thread)
         └── Tier 3: NSH Polyglot Shell (crates/nsh-core, nsh-base, nsh-js, nsh-cli)
               └── Tier 2: Aphrody OS Creative Suite (crates/os_session, os_craft, os_ui, os_desktop)
                     └── Tier 1: GPU Core SDK (crates/gpui, gpui_ui, gpui_code, gpui_app_shell, gpui_wgpu, gpui_web)
                           └── Tier 0: Kernel & Tooling (crates/brand, util, tooling/xtask, lints, perf, bun)
```

### Key Build, Linker & Hygiene Upgrades
1. **Top `Cargo.lock` Duplicate Consolidation (`180` duplicates -> minimal)**:
   - Align `hashbrown` (5x), `itertools` (5x), `quick-xml` (5x), `rand`/`rand_core` (5x), `windows*` (5x), `darling*` (4x), and `syn` (3x) via root [`Cargo.toml`](../../Cargo.toml) `[patch.crates-io]` and upstream dependency bumps.
2. **Feature Unification (`cargo-hakari`) & Unused Dep Pruning (`cargo-machete`)**:
   - Add a workspace-hack crate (`cargo-hakari`) so compiling `os_desktop`, `gpui_app_shell`, `nsh-cli`, and `zed` shares identical dependency feature flags without recompilation storms.
   - Adopt workspace-inherited dependencies in remaining crates flagged by `cargo xtask package-conformity` (`perf`, `gpui`, `title_bar`, `vim`, `fs`) and enforce `[lints] workspace = true` uniformly (including [`crates/gpui_web/examples/hello_web/Cargo.toml`](../../crates/gpui_web/examples/hello_web/Cargo.toml)).
3. **Linkers, Caching & Fleet Dispatch ([`.cargo/config.toml`](../../.cargo/config.toml), [`fleet-sync.ts`](../../tooling/bun/fleet-sync.ts))**:
   - Enable `rust-lld.exe` on `x86_64-pc-windows-msvc` and `mold` on `x86_64-unknown-linux-gnu`.
   - Maintain `sccache` natively while letting [`web-build.ts`](../../tooling/bun/web-build.ts) and [`web_examples.rs`](../../tooling/xtask/src/tasks/web_examples.rs) override `RUSTC_WRAPPER=""` for Windows `web-sys` builds.
   - Offload heavy builds to `fr1`, `cargo nextest` partitioned tests/benchmarks to `fr2`, and keep `prd` strictly for signed runtime deployments.
4. **Workspace Lints & `deny.toml` Policy**:
   - Upgrade `[workspace.lints.rust]` `unexpected_cfgs` to `"warn"` with explicit `check-cfg`, and add `clippy::await_holding_lock = "deny"` and `clippy::clone_on_ref_ptr = "warn"`.
   - Configure `deny.toml` to match Aphrody's split license boundary: explicitly allow `GPL-3.0-or-later` alongside permissive licenses (`MIT`, `Apache-2.0`, `BSD-*`, `ISC`, `Zlib`) on IDE/app crates, while gating permissive-only licenses on `publish = true` SDK crates; ban `openssl` in favor of [`rustls`](../../Cargo.toml) and pin allowed git origins.

---

## 3. Graphics Engine, WGPU v30, 120 FPS & WebAssembly Hardening

| Area | Current Codebase Gap | Production Hardening Action |
| :--- | :--- | :--- |
| **WebAssembly Payload Size** | [`crates/assets/src/assets.rs`](../../crates/assets/src/assets.rs) embeds raw `.ttf` fonts (~1.6 MB), `.wav` sounds, and defaults into Wasm `.rodata`; legacy Trunk HTML sets `data-wasm-opt="0"`. | Default web releases to [`[profile.web]`](../../Cargo.toml) via `web-build.ts --release`; split `assets` behind `web-assets-split` keeping only tiny core SVG icons inline; stream subsetted WOFF2 fonts via `on_missing_glyphs` ([`dynamic_fonts.rs`](../../crates/gpui_web/examples/hello_web/dynamic_fonts.rs)); swap `dlmalloc` for `talc` (`WasmDynamicTalc`; `wee_alloc` stays banned). |
| **Shader Pipeline Cache** | `crates/gpui_wgpu` has zero `device.create_pipeline_cache` calls. | Key `wgpu::PipelineCache` by `(adapter_info, driver_version, shader_hash)` with atomic `Write -> fs::rename` persistence on native and `createRenderPipelineAsync` on WebGPU. |
| **Async `DeviceLost` Recovery** | Native [`wgpu_renderer.rs`](../../crates/gpui_wgpu/src/wgpu_renderer.rs) calls blocking `std::thread::sleep(350ms)`; Wasm stops permanently asking user to reload. | Drive exponential backoff (`50/100/200ms`) on GPUI's foreground executor without blocking OS event processing; on WebGPU, asynchronously re-acquire adapter/device or fall back to WebGL2 cleanly. |
| **Texture Atlas Eviction** | [`wgpu_atlas.rs`](../../crates/gpui_wgpu/src/wgpu_atlas.rs) only frees a texture when `live_atlas_keys == 0`. | Track per-tile `last_frame_used: u64`, enforce a per-kind texture pool cap, and sweep tiles unused for >600 frames before allocating additional GPU textures. |
| **Web A11y & Mobile Keyboard** | [`crates/gpui_web/src/window.rs`](../../crates/gpui_web/src/window.rs) sets `<canvas role="application">`; [`viewport.rs`](../../crates/gpui_web/src/viewport.rs) leaves `WindowInsets.ime` at zero. | Bridge `accesskit` updates into a synchronized hidden shadow DOM tree mirroring interactive focusable nodes; populate `insets.ime.bottom = (layout_height - visual_viewport.height()).max(0.0)`. |

---

## 4. Polyglot Runtimes, Sandboxing & Security Hardening (Zero-Trust)

| Severity | Audit Location | Mitigation & Architecture Fix |
| :--- | :--- | :--- |
| **P0** | [`crates/os_craft/src/mcp.rs`](../../crates/os_craft/src/mcp.rs) | **Hardened in part now**: strict loopback `Host` header check (blocking DNS rebinding), cross-origin `Origin` block, optional `APHRODY_MCP_TOKEN` bearer auth, and `Cache-Control: no-store`. **Next**: migrate onto feature-gated `rmcp 3.10.0` + `brontes 0.7.3` with workspace root confinement on `craft_sniff_file`. |
| **P0** | [`crates/crashes/src/crashes.rs`](../../crates/crashes/src/crashes.rs), [`crates/zed/src/reliability.rs`](../../crates/zed/src/reliability.rs) | Disable unredacted binary `.dmp` minidump uploads by default (raw heap/stack dumps leak buffers and credentials); submit symbolicated traces only or scrub memory buffers locally. |
| **P0** | [`crates/crashes/src/crashes.rs`](../../crates/crashes/src/crashes.rs), [`crates/extension_host/src/wasm_host.rs`](../../crates/extension_host/src/wasm_host.rs) | Replace `std::process::abort()` on non-critical background/applet panics with `std::panic::catch_unwind` isolation boundaries and graceful panel recovery banners. |
| **P0** | [`crates/extension_host/src/wasm_host.rs`](../../crates/extension_host/src/wasm_host.rs) | Attach `wasmtime::ResourceLimiter` (`StoreLimits`: cap linear memory e.g. 512 MB, tables, and nested instances) alongside `epoch_interruption(true)`; pipe WASI stdio to bounded `MemoryOutputPipe`/`zlog` rather than `inherit_stdio()`. |
| **P1** | [`crates/nsh-js/src/host.rs`](../../crates/nsh-js/src/host.rs), [`crates/nsh-js/src/sandbox.rs`](../../crates/nsh-js/src/sandbox.rs) | Mark `eval_bytecode` `unsafe` (or require HMAC/Ed25519 signature + header validation before `Module::load`); replace Windows wall-clock `Instant::now()` CPU accounting with `GetThreadTimes` / `QueryThreadCycleTime`; freeze `(async function*(){}).constructor` and `Reflect`/`Object.setPrototypeOf`. |
| **P1** | [`crates/sandbox/src/windows_wsl.rs`](../../crates/sandbox/src/windows_wsl.rs), [`linux_bubblewrap.rs`](../../crates/sandbox/src/linux_bubblewrap.rs) | Remove hardcoded `cloud.zed.dev/releases` WSL downloader; adopt native Windows Job Objects (`win32job`) + AppContainer low-box tokens and Linux unprivileged `landlock` + `seccomp-bpf` so sandboxing never falls back unsandboxed. |
| **P1** | [`crates/nsh-platform-gpui/src/native/web.rs`](../../crates/nsh-platform-gpui/src/native/web.rs) | Restrict `always_allowed` `data:` wildcard navigation in production builds and validate `request.uri()` against per-window capability ACLs inside `with_ipc_handler`. |
| **P2** | [`crates/db/src/db.rs`](../../crates/db/src/db.rs), [`crates/ztracing/src/lib.rs`](../../crates/ztracing/src/lib.rs) | Raise `PRAGMA busy_timeout` from `500` to `5000` ms and quarantine corrupted `db.sqlite*` files to `*.corrupt.<ts>` before retrying; add a lock-free `4,096`-slot atomic render-loop ring buffer dumping traces only when frame time exceeds `16 ms`. |

---

## 5. Testing Pyramid, Release Pipeline & Edge Operations

1. **Deterministic & Visual Testing**:
   - Enforce `#[gpui::test(iterations = N)]` + `SEED=<u64>` + `PENDING_TRACES=1`.
   - Upgrade [`crates/os_desktop/tests/visual_smoke.rs`](../../crates/os_desktop/tests/visual_smoke.rs) from bucket counts (`assert_non_blank_png`) to perceptual **MSSIM** comparison (`image-compare`, threshold `>= 0.992`) under Linux Mesa Lavapipe and Windows D3D12 WARP.
   - Automate [`tooling/bun/web-verify.ts`](../../tooling/bun/web-verify.ts) headless Chromium WebGPU verification across PRs, add `proptest` suites on `os_craft`/`nsh-core`/`text`, and gate PRs with custom [`tooling/lints/`](../../tooling/lints/src/lib.rs) checks and [`tooling/bun/repo-hygiene.ts`](../../tooling/bun/repo-hygiene.ts).
2. **Release Engineering & Updater Autonomy**:
   - Pair `release-plz` with `cargo-dist` across Windows (MSIX + NSIS with Azure Trusted Signing), macOS (notarized Universal DMG), and Linux (portable archive/AppImage/Flatpak with bundled `bwrap`), producing Sigstore/OIDC `actions/attest-build-provenance` attestations and CycloneDX SBOMs.
   - Migrate [`crates/auto_update`](../../crates/auto_update/src/auto_update.rs) endpoints from legacy domains to `https://releases.aphrody.com` backed by Cloudflare R2 + `axoupdater` for CLI tools.
3. **Edge Infrastructure ($0 Vercel — Cloudflare Workers Static Assets + R2 + Durable Objects)**:
   - Replace stale `.cloudflare/` Workers proxies and `deploy_docs.rs` legacy targets with unified `wrangler.jsonc` Static Assets (`./dist/client`), HTTP `Accept: text/markdown` content negotiation, selective COOP/COEP headers restricted to `/play/*` and `/studio/*` routes (preserving documentation embeds), and SQLite-backed Durable Objects with WebSocket hibernation (`ctx.acceptWebSocket()`) for real-time sync.
   - Target production SLOs: TTI `< 1.2s` p95, Wasm cold start `< 400ms` p95, frame time `<= 8.33ms` p99 (120Hz), and Wasm release Brotli bundle `<= 2.0 MB` (`<= 6.5 MB` intermediate debug/assets ceiling).
