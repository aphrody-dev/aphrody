---
name: github-ops
description: >-
  GitHub repository automation for aphrody-dev/aphrody: Issues, Wiki sync, PR
  templates, Renovate, scheduled upstream syncs from zed-industries/zed,
  longbridge/nsh, longbridge/gpui-kit, gfx-rs/wgpu, aphrody-labs/bun,
  agent-browser, agent-desktop, and Kitesurf forks, automated PR approval and
  auto-merge, fleet/Cloudflare orchestration, and issue-fix workflows.
  Use when modifying .github/workflows, labels, Wiki pages, fleet CI, or sync ops.
---

# GitHub Automation & Upstream Sync for aphrody-dev/aphrody

Manages GitHub operations across standard hosted runners (`ubuntu-latest`, `windows-latest`, `macos-latest`), declarative fleet hosts (`assets/os/fleet_cluster.toml`), and native `GITHUB_TOKEN` (or optional GitHub App token `vars.APHRODY_APP_ID` / `secrets.APHRODY_APP_PRIVATE_KEY`).

## Repository Configuration Baseline

Verify live settings on `aphrody-dev/aphrody` with `gh api repos/aphrody-dev/aphrody`:
- `has_issues: true`, `has_wiki: true`, `has_discussions: true`
- `allow_auto_merge: true`, `allow_squash_merge: true`, `allow_rebase_merge: true`, `delete_branch_on_merge: true`
- Actions workflow permissions (`repos/aphrody-dev/aphrody/actions/permissions/workflow`): `default_workflow_permissions: "write"`, `can_approve_pull_request_reviews: true`.

## Workflow Suite (`.github/workflows/aphrody-*.yml`)

1. **`aphrody-ci.yml`**: PR, `main`, and `merge_group` verification across five jobs:
   - `plugin-and-config`: validates `product.toml`, plugin/hook JSON, workflow YAML, `bun run hygiene`, and `bun run test:tooling`.
   - `brand-and-sdk-linux`: `cargo xtask brand check`, `brand`/`xtask` tests, modular `gpui_ui` (`nsh-full`), `gpui_code --no-default-features`, `gpui_app_shell` (`nsh`), and `aphrody_cli`.
   - `wasm-webgpu-first`: `wasm32-unknown-unknown` + `wasm32-wasip2` checks (`bun run web:check`, `bun run web:build:release`) with pinned `wasm-bindgen` and Binaryen `version_133`.
   - `windows-aphrody-and-gpu`: Windows `aphrody` binary, `aphrody_cli`, and `cli` checks.
   - `cloudflare-and-framework`: `bun tooling/bun/cloudflare-deploy.ts audit --dry-run` plus `cargo clippy` and `cargo test --lib` across `aphrody*`, `cosmic_gpui`, `iced_gpui`, `egui_gpui`, `bevy_gpui`, and `tauri_gpui`.
2. **`aphrody-upstream-sync.yml`**: Daily (`0 4 * * *`) and `workflow_dispatch` sync via `tooling/bun/upstream-sync.ts` across `all`, `zed`, `nsh`, `gpui-kit`, `bun`, `wgpu`, `store`, `agent-browser`, and `agent-desktop`, updating `assets/os/upstream_sync_lock.toml`, checking `repo-hygiene.ts`, and opening `sync/upstreams-auto` PRs labeled `upstream-sync,auto-merge,zed,gpui-kit,nsh`. Also respects Kitesurf-absorbed vendor forks (`taffy`, `parley`, `stylo`, `blitz`, `vello`, `anyrender`, `boa`, `resvg`, `jpeg-encoder`, `ratatui-image`) under `vendor/*` and `vendor/browser/third_party`.
3. **`aphrody-fleet-orchestrator.yml`**: 15-minute `schedule`, `push` (`main` fleet/Worker paths), `workflow_dispatch` (`audit`, `probe`, `schedule`, `beacon`, `rebalance`), and `repository_dispatch` (`fleet_rebalance`, `push_main`, `cloudflare_webhook`). Validates `assets/os/fleet_cluster.toml` on Linux/Windows/macOS, runs `fleet-lb` + `.cloudflare/aphrody-worker` tests, verifies `bun run fleet:lb` subcommands, and performs secret-gated SSH probing, `/api/fleet/beacon` sync, and Worker deployment.
4. **`aphrody-release.yml`**: Multi-platform release builds (`cargo-zigbuild` Linux `x86_64`/`aarch64`, Windows `x64`, macOS `arm64`) across `lts`/`stable`/`preview`/`canary`/`nightly`, Sigstore build provenance attestations, GitHub Releases, R2 `aphrody-releases` uploads, and `.cloudflare/aphrody-worker` deployment.
5. **`aphrody-auto-merge.yml`**: Approves and enables squash auto-merge for `auto-merge`, `upstream-sync`, `issue-fix`, `dependencies`, Renovate, and Dependabot PRs once `aphrody CI` succeeds.
6. **`aphrody-issue-fix.yml`**: Domain-labels issues/PRs (`gpu/vulkan`, `gpu/directx`, `gpu/webgpu`, `wgpu`, `nsh`, `gpui-kit`, `zed`, `issue-fix`), handles `/autofix` (`cargo fmt --all` + `cargo xtask brand apply`), and runs `zed-wasm-wgpu-issues.ts audit`.
7. **`aphrody-wiki-sync.yml`**: Mirrors `README.md`, `docs/wgpu/`, `docs/gpu/`, `docs/nsh/`, and `.agents/plugins/aphrody/` docs/skills to `aphrody-dev/aphrody.wiki.git`.
8. **`aphrody-janitor.yml`** & **`aphrody-kernel-watch.yml`**: Weekly stale triage and merged `sync|bot|wt|autofix/*` branch pruning; 5-minute `tooling/kernel-watch` polling.

## Manual Operations Cheat Sheet

```powershell
# Check upstream sync status and lockfile locally
bun run sync:status

# Trigger upstream sync or fleet orchestration on GitHub Actions
gh workflow run aphrody-upstream-sync.yml --repo aphrody-dev/aphrody
gh workflow run aphrody-fleet-orchestrator.yml --repo aphrody-dev/aphrody -f action=probe -f job_kind=native_rust -f dry_run=true

# Trigger Wiki sync
gh workflow run aphrody-wiki-sync.yml --repo aphrody-dev/aphrody

# Enable auto-merge on a verified PR
gh pr merge <PR_NUMBER> --repo aphrody-dev/aphrody --auto --squash --delete-branch
```
