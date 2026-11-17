---
name: github-ops
description: >-
  GitHub repository automation for aphrody-dev/aphrody: Issues, Wiki sync, PR
  templates, Renovate, scheduled upstream syncs from zed-industries/zed,
  longbridge/nsh, longbridge/gpui-kit, gfx-rs/wgpu, and aphrody-labs/bun,
  automated PR approval and auto-merge, and automated issue-fix/triage workflows.
  Use when modifying .github/workflows, labels, Wiki pages, or triage/sync ops.
---

# GitHub Automation & Upstream Sync for aphrody-dev/aphrody

Manages end-to-end GitHub operations using standard GitHub-hosted runners (`ubuntu-latest`, `windows-latest`) and native `GITHUB_TOKEN` (with repo workflow write + PR approval permissions enabled).

## Repository Configuration Baseline

Verify live settings on `aphrody-dev/aphrody` with `gh api repos/aphrody-dev/aphrody`:
- `has_issues: true`, `has_wiki: true`, `has_discussions: true`
- `allow_auto_merge: true`, `allow_squash_merge: true`, `allow_rebase_merge: true`, `delete_branch_on_merge: true`
- Actions workflow permissions (`repos/aphrody-dev/aphrody/actions/permissions/workflow`): `default_workflow_permissions: "write"`, `can_approve_pull_request_reviews: true`.

## Workflow Suite (`.github/workflows/aphrody-*.yml`)

1. **`aphrody-ci.yml`**: Fast verification on PRs and `main` pushes (`cargo xtask brand check`, `cargo test -p brand -p xtask --bins -p aphrody_cli`, modular SDK checks `gpui_ui`, `gpui_code`, `gpui_app_shell`, Windows `aphrody` bin check, plugin/hook JSON & Bun validation).
2. **`aphrody-upstream-sync.yml`**: Scheduled (`cron`) & `workflow_dispatch` upstream sync tracking:
   - `zed-industries/zed` (git merge/cherry-pick candidates & commit digest / PR creation)
   - `longbridge/nsh` (`crates/nsh-*`, `docs/nsh`)
   - `longbridge/gpui-kit` (`skills/gpui-kit*`, `docs/gpui-kit`)
   - `gfx-rs/wgpu` (upstream revision check against `vendor/wgpu`)
   - `aphrody-labs/bun` (`vendor/bun` submodule fast-forward PR)
   - Automatically labels sync PRs with `upstream-sync` + `auto-merge`.
3. **`aphrody-auto-merge.yml`**: Runs on PRs labeled `auto-merge` (or Renovate/trusted sync bot PRs), approves PR via `gh pr review --approve`, enables `gh pr merge --auto --squash --delete-branch` once checks pass, and auto-closes linked issues (`Closes #...`).
4. **`aphrody-issue-fix.yml`**: Triages newly opened issues & PRs with domain labels (`zed`, `nsh`, `gpui-kit`, `wgpu`, `gpu/vulkan`, `gpu/directx`, `gpu/webgpu`, `issue-fix`), links matching PRs closing issues, and supports `workflow_dispatch` / comment-triggered automated formatting/clippy/brand autofix commits.
5. **`aphrody-wiki-sync.yml`**: Synchronizes repository markdown docs (`README.md`, `docs/wgpu/`, `docs/gpu/`, `docs/nsh/`, plugin docs/skills) directly to `aphrody-dev/aphrody.wiki.git` (`Home.md`, `_Sidebar.md`, topic pages).

## Manual Operations Cheat Sheet

```powershell
# Trigger upstream sync on GitHub Actions
gh workflow run aphrody-upstream-sync.yml --repo aphrody-dev/aphrody

# Trigger Wiki sync
gh workflow run aphrody-wiki-sync.yml --repo aphrody-dev/aphrody

# Enable auto-merge on a verified PR
gh pr merge <PR_NUMBER> --repo aphrody-dev/aphrody --auto --squash --delete-branch
```
