# Aphrody Labs Layout: `aphrody` in the Ordered Stack

The registry is `[repos]` in `ay/ay.toml` (live slugs, one `kind` each). The ordered stack is `fw` (1), `aphrody` (2), `ay` (3), `nie` (4), `shenron` (5); every other repository is a satellite or an upstream fork. Cross-repo Rust dependencies only point down the stack (`fw` <- `aphrody` <- `ay` ...).

## 1. Sibling Repositories (checked out under the shared workspace root, `../<key>`)

| Key | Slug | Kind | Role |
|---|---|---|---|
| `fw` | `aphrody-labs/fw` | toolchain | Toolchain and agent runtime; `plugins/aphrody` is the installed agent plugin and the canonical skill source |
| `aphrody` | `aphrody-labs/aphrody` | os | **This repository**: OS and desktop (Zed fork, GPUI, compositor, installer); owns the UI/GPU forks |
| `ay` | `aphrody-labs/ay` | sdk | SDK, cloud and edge (the single `ay-edge` Worker), workspace orchestration |
| `nie` | `aphrody-labs/nie` | game | Game engine and recompiler |
| `shenron` | `aphrody-labs/shenron` | site | dragonballfr.com |
| `wiki` | `aphrody-labs/wiki` | library | Rust workspace of shenron, reads `../shenron/data` |
| `browser` | `aphrody-labs/browser` | browser | One browser workspace (Blink headless, Servo/Obscura lite engine, agent-browser) |
| `fw-nt` | `aphrody-labs/fw-nt` | nt | NT operating system around `fw`; owns the Windows/NT forks |

Retired and not part of the layout: `rosegriffon` (stopped on 2026-11-19), `anu`, `ai-os`, `aydroid`, `open`, `aphrody-m3`, `claude`, `aynt`. There is no `C:\src\vercel` directory.

## 2. What `aphrody` Owns

| Area | Location |
|---|---|
| OS and desktop shell, session, installer | `crates/aphrody_desktop`, `crates/aphrody_session`, `crates/aphrody_installer` (`naming.toml` kind `os`) |
| GPUI, WGPU glue, UI toolkit, Native Shell | `crates/gpui*`, `crates/aphrody_ui`, `crates/nsh-*` |
| Studio editor (Zed fork) | `crates/zed` (binary `aphrody`) |
| Product identity | `product.toml`, `crates/brand` |
| UI/GPU upstream forks (`wgpu`, `taffy`, `egui`, `accesskit`, `cosmic-text`, `wry`, `mullvadvpn-app`, `craft-libs`, `craft-anything`, `kitesurf`, `browser`) | pinned git dependencies in `Cargo.toml` (`git = ..., tag = ...`), recorded in `tooling/bun/forks-manifest.json` |
| Docs | `docs/` (Zed mdBook) and `docs/aphrody/` (Aphrody architecture, design, GPU, NSH, WGPU, watch) |

## 3. Invariants

- Each upstream fork is checked out by exactly one repository; here forks are resolved as pinned Cargo git dependencies, there are no submodules and no `vendor/` checkouts.
- Cloud and edge code (Workers, routes, FinOps) lives in `ay`; `aphrody` does not define a second owner for `ay-edge` routes.
- Windows/NT forks live in `fw-nt`; toolchain and agent skills live in `fw`.
