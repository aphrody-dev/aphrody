# Aphrody Labs Consolidation into `aphrody` (Final Product) and `ay` (SDK / libc)

Canonical consolidation map eliminating duplication across `aphrody-labs/*` repositories while keeping `rosegriffon`, `nie`, and `shenron` isolated.

- **`aphrody`** (`aphrody-dev/aphrody`): **Final Product** (IDE, OS, GPUI, NSH, Craft, Desktop, Web).
- **`ay`** (`aphrody-dev/ay`): **SDK / libc** (low-level runtime primitives, Cloudflare/edge, media, Next/Turbopack, automation).

## 1. Consolidated into `aphrody` (`aphrody-dev/aphrody`)

| Source Repo (`aphrody-labs/*`) | Canonical Owner in `aphrody` | Notes |
|---|---|---|
| `aphrody-labs/.github` | `.agents/plugins/aphrody/` & `aphrody/docs/` | Org standards (`STANDARD.md`, `standard.toml`, `naming/`) |
| `aphrody-labs/aphrody-git` | `crates/git*` & `tooling/bun/` | Git/GH/ingest automation |
| `aphrody-labs/browser` | `crates/gpui_web` & `crates/aphrody_pilot` | Browser / WebGPU / CDP surface |
| `aphrody-labs/open` | `crates/aphrody_cli` & `tooling/` | Open/launcher & target-dir detection |
| `aphrody-labs/rust-mono` | `vendor/` & `rust-toolchain.toml` | Rust toolchain & governance |

## 2. Consolidated into `ay` (`aphrody-dev/ay`)

| Source Repo (`aphrody-labs/*`) | Canonical Owner in `ay` | Notes |
|---|---|---|
| `aphrody-labs/auto` | `apps/` & `crates/` | Pure Bun 1.4.5 automation & locales |
| `aphrody-labs/ay-cf` | `workers/` & `cf/` | Cloudflare Workers, FinOps, `http-scan` |
| `aphrody-labs/ay-media` | `crates/ay-media*` | Media processing crates |
| `aphrody-labs/ay-vercel` | `crates/ay-vercel-next` | Pure-Rust Next.js + Turbopack |

## 3. Zero-Duplication Invariants

- Single `gpui` / `gpui_wgpu` / `gpui_kit` / `nsh` stack in `aphrody` (`vendor/wgpu` v30.0.0).
- Single `ay` edge/cloud/media/next stack in `ay`.
- Ephemeral/empty folders (`C:\src\vercel`) removed; `rosegriffon`, `nie`, and `shenron` untouched.
