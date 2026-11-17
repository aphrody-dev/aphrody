# Aphrody Labs & Ecosystem Deduplication Ledger

> Machine-readable TOML ledger: [`assets/os/aphrody_labs_dedup.toml`](../../assets/os/aphrody_labs_dedup.toml). Orchestrated dynamically across `agy` (parallel cluster subagents via `run_workflow`) and live peer Claude Code (`PID 11592`, `aphrody-labs/.github`).

## 1. Live Organization Snapshot (`aphrody-labs`)

| Metric | Live Count | Notes |
| --- | ---: | --- |
| Total repositories audited | 967 | Retrieved live from GitHub `aphrody-labs` API |
| Already archived on GitHub | 21 | Superseded mirrors/experiments marked `isArchived: true` |
| Active upstream forks | 894 | Relative-URL submodules (`vendor/*`, `37` pinned in [`.gitmodules`](../../.gitmodules)) + monorepo mirrors |
| Active original repositories | 52 | Flagship monorepos (`aphrody`, `ai-os`, `ay`, `ANU`, `open`, `auto`, `shenron`, `nie`, `.github`) + candidates for archiving |

## 2. Canonical Monorepo Owners

| Monorepo / Repo | Primary Scope | Consolidated / Absorbed Upstream Clusters |
| --- | --- | --- |
| `aphrody-dev/aphrody` ([root](../../Cargo.toml)) | Unified Rust OS, GPUI, Craft, NSH, WebGPU/Wasm, W3C i18n, Tooling | `agent-desktop`, `artcraft`, `artcraftx`, `anicraft`, `autocraft`, `dioxus-monorepo`, standalone `cosmic-*` & `craft-*` UI crates, `gpui-component` / `nsh` merges |
| `aphrody-dev/ai-os` | AI-OS runtime, Relay (MCP/ACP/A2A/HTTP/CLI), Automate | `relay`, `automate`, `env-kit`, `gnu-ts`, `omniu`, `wck` |
| `aphrody-dev/ay` | Core SDK & machine setup | `packages/machine`, fleet installers |
| `aphrody-labs/open` | AI Router, TUI, Browser automation, Tunnel | `browse`, `opentui`, `opentunnel`, `models.dev` (0 ahead of upstream) |
| `aphrody-labs/.github` | Org standard, CI workflows, W3C `i18n/` checker | Org-wide `STANDARD.md`, `standard.toml`, `i18n/I18N.md`, `i18n/check.ts` |

## 3. Repository Clusters Audited

| Family / Cluster | Count | Strategy |
| --- | ---: | --- |
| `other-ecosystem-forks` | 764 | Consolidate active forks via [`.gitmodules`](../../.gitmodules) & archive superseded 0-ahead copies |
| `agents-mcp-acp-a2a` | 47 | Consolidate active forks via [`.gitmodules`](../../.gitmodules) & archive superseded 0-ahead copies |
| `craft-studio-forks` | 39 | Consolidate active forks via [`.gitmodules`](../../.gitmodules) & archive superseded 0-ahead copies |
| `cosmic-desktop-suite` | 33 | Consolidate active forks via [`.gitmodules`](../../.gitmodules) & archive superseded 0-ahead copies |
| `browser-crawler-scrape` | 19 | Consolidate active forks via [`.gitmodules`](../../.gitmodules) & archive superseded 0-ahead copies |
| `yolo-legacy-experiments` | 19 | Consolidate active forks via [`.gitmodules`](../../.gitmodules) & archive superseded 0-ahead copies |
| `dioxus-blitz-layout` | 16 | Consolidate active forks via [`.gitmodules`](../../.gitmodules) & archive superseded 0-ahead copies |
| `compilers-cargo-wasm` | 11 | Consolidate active forks via [`.gitmodules`](../../.gitmodules) & archive superseded 0-ahead copies |
| `zed-gpui-core` | 8 | Consolidate active forks via [`.gitmodules`](../../.gitmodules) & archive superseded 0-ahead copies |
| `bun-runtime-and-pkgs` | 6 | Consolidate active forks via [`.gitmodules`](../../.gitmodules) & archive superseded 0-ahead copies |
| `tauri-webview-stack` | 5 | Consolidate active forks via [`.gitmodules`](../../.gitmodules) & archive superseded 0-ahead copies |

## 4. High-Confidence Archive Candidates Verified

- `aphrody-labs/agent-desktop`: 100% migrated into [`crates/aphrody_pilot`](../../crates/aphrody_pilot/src/aphrody_pilot.rs), [`crates/aphrody_craft`](../../crates/aphrody_craft/src/aphrody_craft.rs), and [`crates/aphrody_desktop`](../../crates/aphrody_desktop/src/aphrody_desktop.rs) (see [`agent-desktop-ai-os.md`](./agent-desktop-ai-os.md)).
- `aphrody-labs/browse`, `aphrody-labs/opentui`, `aphrody-labs/opentunnel`, `aphrody-labs/models.dev`: absorbed into `aphrody-labs/open` (0 custom commits ahead).
- Standalone legacy `artcraft`, `artcraftx`, `anicraft`, `autocraft`, `brandcraft`, `cadcraft`: consolidated into `vendor/craft-anything`, `vendor/craft-libs`, and `crates/aphrody_craft`.
- Standalone `apify-client-*`, `apify-sdk-*` unreferenced 0-ahead forks: consolidated via `aphrody-labs/open` / npm on `bun`.
