# Aphrody Labs & Ecosystem Deduplication Ledger

> Machine-readable TOML ledger: [`assets/os/aphrody_labs_dedup.toml`](../../../assets/os/aphrody_labs_dedup.toml). Orchestrated dynamically across `agy` (parallel cluster subagents via `run_workflow`) and live peer Claude Code (`PID 11592`, `aphrody-labs/.github`).

## 1. Live Organization Snapshot (`aphrody-labs`)

| Metric | Live Count | Notes |
| --- | ---: | --- |
| Total repositories (`aphrody-dev` + `aphrody-labs` + `rose-griffon`) | 1,180 | Deduplicated live (35 duplicate forks, `-1` suffixes & dead slice archives deleted; 23 superseded repos archived) |
| Archived repositories | 25 | `legacy`, `bun-os`, `bun-distribution`, `ayvpn`, `ay-tools`, `yolo*`, `wii-os`, `google-bun`, `gemini-super-app` |
| Active upstream forks & mirrors | 1,132 | Canonical `<upstream>` / `<owner>-<name>` forks (`24` pinned in [`.gitmodules`](../../../.gitmodules), `38` in `ay/submodules.toml`) |
| Active original repositories | 23 | Flagship products (`aphrody`, `ay`, `anu`, `ai-os`, `aydroid`, `aynt`, `aysh`, `bun`, `browser`, `nie`, `shenron`, `open`, `auto`, `aphrody-m3`, `aphrody-git`, `rust-mono`, `r2b`, `msys`, `.github`) |

## 2. Canonical Monorepo Owners

| Monorepo / Repo | Primary Scope | Consolidated / Absorbed Upstream Clusters |
| --- | --- | --- |
| `aphrody-dev/aphrody` ([root](../../../Cargo.toml)) | Unified Rust OS, GPUI, Craft, NSH, WebGPU/Wasm, W3C i18n, Tooling | `agent-desktop`, `artcraft`, `artcraftx`, `anicraft`, `autocraft`, `dioxus-monorepo`, standalone `cosmic-*` & `craft-*` UI crates, `gpui-component` / `nsh` merges |
| `aphrody-dev/ai-os` | AI-OS runtime, Relay (MCP/ACP/A2A/HTTP/CLI), Automate | `relay`, `automate`, `env-kit`, `gnu-ts`, `omniu`, `wck` |
| `aphrody-dev/ay` | Core SDK & machine setup | `packages/machine`, fleet installers |
| `aphrody-labs/open` | AI Router, TUI, Browser automation, Tunnel | `browse`, `opentui`, `opentunnel`, `models.dev` (0 ahead of upstream) |
| `aphrody-labs/.github` | Org standard, CI workflows, W3C `i18n/` checker | Org-wide `STANDARD.md`, `standard.toml`, `i18n/I18N.md`, `i18n/check.ts` |

## 3. Repository Clusters Audited

| Family / Cluster | Count | Strategy |
| --- | ---: | --- |
| `other-ecosystem-forks` | 764 | Consolidate active forks via [`.gitmodules`](../../../.gitmodules) & archive superseded 0-ahead copies |
| `agents-mcp-acp-a2a` | 47 | Consolidate active forks via [`.gitmodules`](../../../.gitmodules) & archive superseded 0-ahead copies |
| `craft-studio-forks` | 39 | Consolidate active forks via [`.gitmodules`](../../../.gitmodules) & archive superseded 0-ahead copies |
| `cosmic-desktop-suite` | 33 | Consolidate active forks via [`.gitmodules`](../../../.gitmodules) & archive superseded 0-ahead copies |
| `browser-crawler-scrape` | 19 | Consolidate active forks via [`.gitmodules`](../../../.gitmodules) & archive superseded 0-ahead copies |
| `yolo-legacy-experiments` | 19 | Consolidate active forks via [`.gitmodules`](../../../.gitmodules) & archive superseded 0-ahead copies |
| `dioxus-blitz-layout` | 16 | Consolidate active forks via [`.gitmodules`](../../../.gitmodules) & archive superseded 0-ahead copies |
| `compilers-cargo-wasm` | 11 | Consolidate active forks via [`.gitmodules`](../../../.gitmodules) & archive superseded 0-ahead copies |
| `zed-gpui-core` | 8 | Consolidate active forks via [`.gitmodules`](../../../.gitmodules) & archive superseded 0-ahead copies |
| `bun-runtime-and-pkgs` | 6 | Consolidate active forks via [`.gitmodules`](../../../.gitmodules) & archive superseded 0-ahead copies |
| `tauri-webview-stack` | 5 | Consolidate active forks via [`.gitmodules`](../../../.gitmodules) & archive superseded 0-ahead copies |

## 4. Deduplication & Consolidation Actions Executed

- **Duplicate forks eliminated (`DELETE`)**: `anomalyco-browse` (synced into `aphrody-labs/browse`), `cloudflare-boringtun`, `cloudflare-pingora`, `mullvad-gotatun`, `mullvad-mullvadvpn-app`, `base-ui-1`, `tailwindcss-1`, `next.js-1`, `LiberNT` (commits preserved in `aphrody-dev/aynt`), `storytold-winit`, `storytold-wgpu`, `gfw-build-extra`, `gfw-MINGW-packages`, detached `antigravity-sdk-python` (replaced by canonical fork), and archived `longbridge-nsh` (active fork renamed to `aphrody-labs/longbridge-nsh`).
- **Dead slice & cross-org collision archives eliminated (`DELETE`)**: `aphrody-labs/ai-os`, `agent-desktop`, `dioxus-monorepo`, `carbon`, `yolo-bun`, `yolo-hub`, `bench`, `depot`, `env-kit`, `gamescope-rs`, `gaming`, `infra`, `nt`, `ring3-agent`, `rust-stack`, `steam-depot`, `unix`, `wayland`, `wck`, `xtask` (`bun-os` and `bun-distribution` preserved for `crates.io` tarballs).
- **Thin/duplicate products absorbed & archived (`PATCH archived=true`)**: `aphrody-dev/ayvpn` -> [`crates/ayvpn_daemon`](../../../crates/ayvpn_daemon/src/lib.rs) + [`crates/aphrody_vpn`](../../../crates/aphrody_vpn/src/aphrody_vpn.rs) + `browser-vpn`; `aphrody-dev/ay-tools` -> `ay/crates/ay-deps`; `aphrody-labs/legacy`, `yolo` + 16 `yolo-*` monorepos, `wii-os`, `google-bun`, and `gemini-super-app`.
