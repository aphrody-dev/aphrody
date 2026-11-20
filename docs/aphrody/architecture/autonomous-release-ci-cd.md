# 24/7 Zero-Human-In-The-Loop Dev, CI/CD, Release & Download Architecture

> Reproducible end-to-end engineering design across GitHub Actions (`checkout@v7.0.1`, `github-script@v9.0.0`, `attest-build-provenance@v4.2.2`, `create-github-app-token@v3.3.0`, `stale@v11.0.0`), 6-tier release channels (`lts`, `stable`, `preview`, `canary`, `nightly`, `dev`), SHA-256 hardened auto-updates, and $0 egress Cloudflare R2 download portal.

---

## 1. Live-Verified Actions, CI/CD & Merge Queue Security

| Workflow / Config | Finding Resolved | Production Configuration |
| :--- | :--- | :--- |
| [`.github/workflows/aphrody-ci.yml`](../../../.github/workflows/aphrody-ci.yml) | Merged `merge_group` missing explicit `types: [checks_requested]`; added `bun run hygiene` + `bun run test:tooling`. | Pins `actions/checkout@v7`, `oven-sh/setup-bun@v2`, `Swatinem/rust-cache@v2`; every `actions/checkout` runs with `submodules: false` because forks are pinned Cargo git dependencies. |
| [`.github/workflows/aphrody-auto-merge.yml`](../../../.github/workflows/aphrody-auto-merge.yml) | Replaced `python3` inline script with `bun`; supports `APHRODY_APP_ID`/`APHRODY_APP_PRIVATE_KEY` via `actions/create-github-app-token@v3` (required by GitHub branch protection so bot approvals satisfy required reviews and re-trigger workflows) and adds `merge_group`. | Runs continuous squash/auto-merge for green labeled/bot PRs cleanly 24/7. |
| [`.github/workflows/aphrody-issue-fix.yml`](../../../.github/workflows/aphrody-issue-fix.yml) | `/autofix` comment handler lacked commenter permission & fork validation before `gh pr checkout`. | Gates `/autofix` to `OWNER`/`MEMBER`/`COLLABORATOR` and same-repo head refs before running `cargo fmt` + `cargo xtask brand apply`. |
| [`.github/workflows/aphrody-release.yml`](../../../.github/workflows/aphrody-release.yml) | Unified 6-channel release, SHA-256 manifest, Sigstore attestation (`actions/attest-build-provenance@v4`), GitHub Releases + optional R2 sync. | Replaces hardcoded Zed DO Spaces (`script/upload-nightly`) and Zippy credentials cleanly. |
| [`.github/workflows/aphrody-janitor.yml`](../../../.github/workflows/aphrody-janitor.yml) | Daily stale PR/branch janitor (`actions/stale@v11` + merged `sync/*` / `bot/*` branch pruning). | Prevents stale branch buildup across autonomous cycles. |
| [`.github/renovate.json`](../../../.github/renovate.json) | Enabled `:semanticCommitsEnabled` and `"schedule": ["at any time"]` (preserving the `wgpu`/`naga` exclusion: the `aphrody-labs/wgpu` fork is pinned by git tag). | Continuous dependency hygiene feeding autonomous CI + merge queue. |
| [`.github/CODEOWNERS`](../../../.github/CODEOWNERS) | Explicit subsystem ownership for automated PR routing. | Pairs cleanly with Merge Queue rulesets. |

---

## 2. Six-Tier Channel Lifecycle & Tagging Model

| Channel (`ReleaseTrack`) | Cadence | Tagging Convention | Promotion & Rollback |
| :--- | :--- | :--- | :--- |
| **`lts`** | Semi-annual (`6m`) | `vX.Y.Z-lts.N` (immutable) | Promoted from vetted `stable`; rollback re-points `channels/lts/latest.json` |
| **`stable`** | Bi-weekly | `vX.Y.Z` (immutable) | Promoted from `preview`; hotfix patch or manifest repoint |
| **`preview`** | Weekly | `vX.Y.Z-pre.N` (immutable) | Cut from `main`; feeds `stable` soak |
| **`canary`** | Per `main` merge | `canary` + `vX.Y.Z-canary.YYYYMMDDHHMMSS` | Continuous build & attestation on green `main` push |
| **`nightly`** | 2x Daily (02:00/14:00 UTC) | `nightly` + `vX.Y.Z-nightly.YYYYMMDD` | Scheduled cron snapshot from `main` |
| **`dev`** | Local / PR | Git SHA | Local workspace & [`fleet-sync.ts`](../../../tooling/bun/fleet-sync.ts) dev builds |

---

## 3. Auto-Update Integrity & $0 Egress Download Portal

1. **In-App Auto-Updater ([`crates/auto_update/src/auto_update.rs`](../../../crates/auto_update/src/auto_update.rs))**:
   - Replaces legacy `github.com/zed-industries/zed/commits/...` URLs with `brand::REPOSITORY_URL`, uses `brand::APP_SLUG` temp prefixes (`aphrody-auto-update`), derives brand asset names (`Aphrody.dmg`, `aphrody.tar.gz`, `Aphrody-Setup.exe`) from `brand::APP_NAME`/`brand::APP_SLUG`, and performs streaming **SHA-256 verification** (`sha2::Sha256`) on `ReleaseAsset { sha256 }` before invoking OS installers.
   - CLI tools integrate `axoupdater` (`0.10.3`) against the same GitHub/R2 JSON feed; every release asset carries Sigstore provenance verifiable via `gh attestation verify <artifact> --owner aphrody-dev`.
2. **Cloudflare R2 Zero-Egress Portal ([`.cloudflare/aphrody-worker/`](../../../.cloudflare/aphrody-worker/src/releases.ts))**:
   - Routes `releases.aphrody.com/*` and `aphrody.com/download*`:
     - `GET /download`: UA-aware HTML portal selecting Windows x64/ARM64, macOS Apple Silicon/Intel, Linux x86_64/aarch64 installers + channel selector (`stable`, `lts`, `preview`, `canary`, `nightly`) + SHA-256/Attestation copy commands.
     - `GET /api/releases/:channel/latest` & `GET /releases/:channel/:version/asset`: Drop-in JSON feed served from R2 `manifests/`.
     - `GET /install.sh` & `GET /install.ps1`: OS bootstrap installers with SHA-256 verification.
