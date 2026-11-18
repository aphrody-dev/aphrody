# Aphrody Labs standard

Version 1.0.0, 2026-11-17. Machine-readable twin: [standard.toml](standard.toml) (labels, required files, security settings, ruleset JSON, pinned actions, Cloudflare policy). When the two disagree, fix the prose to match the TOML. A repository deviates only by writing the deviation and its reason under **Deviations** in its README.

Scope: the `aphrody-labs` organization (free plan), after consolidating former `aphrody-dev` repositories under `aphrody-labs`. Repositories owned by another agent, protected (`aphrody`, `ay`), or preserved as archives are listed in `scope.skip_repos` and are read-only for the apply phase.

- [1. Repository classes](#1-repository-classes)
- [2. Layout](#2-layout)
- [3. README and docs](#3-readme-and-docs)
- [4. Naming](#4-naming)
- [5. Labels](#5-labels)
- [6. Issue forms, pull request template, CODEOWNERS, community files](#6-issue-forms-pull-request-template-codeowners-community-files)
- [7. Rulesets](#7-rulesets)
- [8. Security features](#8-security-features)
- [9. Actions conventions](#9-actions-conventions)
- [10. Reusable workflows](#10-reusable-workflows)
- [11. Dependabot](#11-dependabot)
- [12. Release and publishing](#12-release-and-publishing)
- [13. Pages, wiki, discussions](#13-pages-wiki-discussions)
- [14. Cloudflare](#14-cloudflare)
- [15. GitHub Apps, OAuth apps, tokens, secrets](#15-github-apps-oauth-apps-tokens-secrets)
- [16. Organization profile](#16-organization-profile)
- [17. Apply order and verification](#17-apply-order-and-verification)
- [18. Verified facts](#18-verified-facts)
- [19. Code, commits, pull requests and reviews](#19-code-commits-pull-requests-and-reviews)

## 1. Repository classes

Classes come from the API, first match wins (`[classes]`).

| Class | Rule | What the standard requires |
| :-- | :-- | :-- |
| Private | `visibility == private` | Dependabot alerts and security updates; no wiki; no secrets in tree. Secret scanning and CodeQL need GitHub Advanced Security, not available on the free plan. |
| Fork | `fork == true` | Keep upstream name, license and README. Issues, wiki and projects off. Inherited upstream workflows disabled. Secret scanning, push protection, Dependabot alerts. Nothing else. |
| Original | not a fork, not archived | Everything below. |
| Archived | `archived == true` | Frozen. Archive instead of delete; never unarchive without a reason in the commit log. |

A duplicate of an upstream (two forks of one upstream, a detached copy of a fork) is archived, not deleted, after the submodule and README references are checked. Dead originals (no push for 30 days, no workflows, issues or pull requests, superseded by another repository) are archived the same way.

## 2. Layout

Every original has, at the root:

```text
README.md                 template in section 3
LICENSE                   Apache-2.0 unless the repository says otherwise
AGENTS.md                 recommended: instructions for coding agents (build, test, conventions)
.editorconfig  .gitattributes  .gitignore
.markdownlint-cli2.jsonc  templates/markdownlint-cli2.jsonc
.github/
  CODEOWNERS              templates/CODEOWNERS (not inherited from the organization)
  dependabot.yml          templates/dependabot.yml
  release.yml             published repositories only (release notes categories)
  workflows/
    ci.yml                caller of a reusable CI workflow, job id "ci"
    release.yml           published repositories only (tag triggered)
docs/                     Markdown, or MDX for published sites
src/ or crates/ or packages/ or cmd/    code, by stack
```

Inherited from `aphrody-labs/.github` when a repository has none: `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `SUPPORT.md`, the issue forms and the pull request template. A repository-level `.github/ISSUE_TEMPLATE/` replaces the whole inherited set, so keep all five files or none.

Never committed: `.env*`, keys and certificates (`*.pem`, `*.p12`, `id_*`), `.dev.vars`, build output, vendored copies that a package manager can fetch. Vendored upstream code lives in a git submodule with a relative URL, one checkout each; forks are submodules, not copies.

## 3. README and docs

README, CONTRIBUTING, SECURITY and every community file are GitHub-flavored Markdown (GitHub does not render MDX). Published documentation sites use MDX under `docs/**/*.mdx`.

README rules (`[readme]`):

1. Title is the repository name, 60 characters at most, followed by one sentence that equals the repository description.
2. At most five badges, one row: CI, license, and at most three more that carry information.
3. Required sections in this order: **Why**, **Install**, **Usage**, **Documentation**, **Development**, **License**. A section may be one line; it may not be absent.
4. English first (`readme_language = "en"`); other languages come from the repository's i18n catalogs, never as the only README.
5. Commands are copy-pasteable and tested; versions come from the manifest, not from prose.

Template: [templates/README.template.md](templates/README.template.md).

MDX rules (`[docs]`):

1. Front matter has `title` and `description`.
2. Pages read as Markdown if components are stripped; at most a handful of typed components, imported explicitly.
3. One H1 per page, matching `title`; headings are sentence case.
4. CI runs `markdownlint-cli2`, a link check (`lychee`) and an MDX compile (`@mdx-js/mdx`) through the reusable `docs.yml`.

Template: [templates/doc.template.mdx](templates/doc.template.mdx).

Repository metadata: description (required), homepage (required when a site or package page exists), 3 to 20 lowercase kebab topics including exactly one role topic `aphrody-<runtime|craft|games|unix|platform|design|tools>`. The role topic drives the organization profile grouping.

Licenses: originals default to Apache-2.0; add a `LICENSE` file wherever the API reports none or `NOASSERTION`. Forks and mirrors keep the upstream license untouched.

## 4. Naming

| Thing | Rule |
| :-- | :-- |
| Repository | `^[a-z0-9][a-z0-9._-]*$`, at most 50 characters, no `-1` suffixes for duplicates (archive the duplicate instead). Forks keep the upstream name. |
| Default branch | `main`. Forks keep upstream's branch. |
| Tags | `vMAJOR.MINOR.PATCH[-prerelease]` (semver). Release tags are immutable. |
| Commits | `area: summary` (Bun's shape), imperative, 72 characters at most; `type(scope): summary` is accepted (types: feat, fix, docs, chore, refactor, perf, test, build, ci, revert). Breaking: a `BREAKING CHANGE:` footer. No AI attribution trailers on new commits. Full rules and the checker: section 19. |
| Workflow files | `ci.yml`, `release.yml`, `docs.yml`, `pages.yml`; the workflow `name:` equals the file stem. Reusable ones live only in this repository. |
| Topics | lowercase kebab, at most 20, one role topic. |
| Labels | section 5. |

## 5. Labels

One set for every original, defined in `[[labels]]` of standard.toml and synced by the apply phase (create or update; unknown labels are reported, not deleted). GitHub's default `enhancement`, `documentation` and `invalid` are renamed to `feature`, `docs` and `wontfix` (`[labels_rename]`).

| Group | Labels |
| :-- | :-- |
| Type | `bug`, `feature`, `docs`, `chore`, `security`, `question` |
| Triage | `needs-triage`, `blocked`, `duplicate`, `wontfix`, `good first issue`, `help wanted` |
| Priority | `p0` production down or data loss, `p1` major breakage, `p2` normal, `p3` low |
| Release | `breaking`, `skip-changelog` |
| Dependabot | `dependencies`, `github_actions`, `bun`, `rust` |

Area labels (`area:cli`, `area:docs`) are per repository and prefixed `area:`.

## 6. Issue forms, pull request template, CODEOWNERS, community files

- Issue forms (YAML, `.github/ISSUE_TEMPLATE/`): `bug.yml`, `feature.yml`, `docs.yml`, `crash.yml`, plus `config.yml` with `blank_issues_enabled: false` and contact links to the security policy and SUPPORT. Forms apply the type label and `needs-triage`.
- Pull request template: `.github/pull_request_template.md` (what changed and why, how it was verified with commands and output, risk, rollback, breaking changes, checklist). Checks mirror CONTRIBUTING.md and section 19.
- CODEOWNERS is not inherited: each original carries `.github/CODEOWNERS` from the template. Owners are real GitHub users or teams with write access; the solo setup uses the owner account. Code-owner review is not required in rulesets (a solo maintainer cannot approve their own pull request).
- `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `SUPPORT.md` live in this repository and apply everywhere by default.
- Security reports use private vulnerability reporting plus contact@aphrody.com; never public issues.

## 7. Rulesets

Context: one maintainer plus autonomous agents who commit to `main` directly. So the integrity rules are absolute (nobody rewrites history), and the review rules are present but bypassable by repository admins. Org-level rulesets are not available on the free plan, so rulesets are created per repository (`POST /repos/{owner}/{repo}/rulesets`) from the JSON in standard.toml.

| Ruleset | Target | Rules | Bypass | Applies to |
| :-- | :-- | :-- | :-- | :-- |
| `main-integrity` | default branch | `deletion`, `non_fast_forward`, `required_linear_history` | none | every original (and private repository) |
| `tags-immutable` | `refs/tags/v*` | `deletion`, `update`, `non_fast_forward` | none | every original |
| `main-gated` | default branch | `pull_request` (0 approvals, threads resolved, squash or rebase), `required_status_checks` (`ci / gate`) | repository role admin, always | gated tier |

Tiers (`rulesets.tiers`): `solo` is the default (integrity and tags). `gated` adds `main-gated` for published packages, production deployers and repositories with outside contributors; membership is `rulesets.gated_repos` plus the topic `aphrody-gated`.

Why these choices:

- Required status checks block direct pushes of unchecked commits, which would stop the agents, so the solo tier relies on post-push CI plus alerts; the gated tier gates the pull request path and lets admins bypass.
- Zero required approvals: a solo maintainer cannot approve their own pull request, and a required approval would force a second account.
- The required check name is `ci / gate`: caller job id `ci` plus the reusable workflow's final job `gate`. Confirm the exact name from the first run's check runs before enforcing.
- Merge queue: the rule type `merge_queue` exists. Add it to `main-gated` only if the API accepts it for the repository; callers already trigger on `merge_group`, which a merge queue requires.
- Forks get no ruleset. The legacy repository keeps its own `main-no-rewrite` ruleset; it is read-only.

Repository merge settings: squash and rebase allowed, merge commits off, delete branch on merge, auto-merge on, update branch on (`[repo_settings.original]`).

## 8. Security features

Targets from `[security]`. Applied through organization security configurations (`POST /orgs/{org}/code-security/configurations`, then `/attach`), not repository by repository.

| Feature | Public originals | Forks | Private |
| :-- | :-- | :-- | :-- |
| Dependency graph | on | on | on |
| Dependabot alerts | on | on | on |
| Dependabot security updates | on | off | on |
| Secret scanning | on | on | n/a on free plan |
| Push protection | on | on | n/a on free plan |
| CodeQL default setup (default suite) | on | not set | n/a |
| Private vulnerability reporting | on | off | n/a |

- Two configurations replace the unattached "GitHub recommended" (id 17, left untouched): `aphrody-originals` attaches to public originals; `aphrody-forks` attaches to public forks and is the default for new public repositories. A new original is promoted to `aphrody-originals` when it is created.
- CodeQL default setup languages come from `GET /repos/{owner}/{repo}/code-scanning/default-setup` (currently offered: actions, javascript-typescript, python, rust, and others by detection); forks are not scanned because their upstream is.
- Organization: two-factor authentication required, default repository permission read, members cannot create repositories, at most two admins, no pending admin invitation unless the owner confirms the invitee.
- Actions org settings: default workflow permissions `read`, workflows cannot approve pull requests, allowed actions `selected` (GitHub-owned, the `patterns_allowed` list, plus every third-party action found in the workflows of non-fork originals, derived by the apply phase), approval required for all outside contributors' workflow runs.
- Secrets: none in the tree; `[secrets]` lists which org secrets are visible to all repositories (target: none except the two app keys) and which move to environment scope.

## 9. Actions conventions

Enforced by `tools/lint-standard.ts` and the org policy above.

1. **Pin third-party actions to a full commit SHA** with the version as a trailing comment (`uses: actions/checkout@3d3c…90b1 # v7.0.1`). First-party reusable workflows (`aphrody-labs/.github/...`) are referenced by `@main`: same owner, same review, no pin churn. Current pins are in `[actions.pinned_versions]`, resolved live with `gh api` on the date above; Dependabot's `github-actions` ecosystem keeps them current.
2. **Least privilege**: `permissions: { contents: read }` at the top of every workflow; grant extra scopes per job (`contents: write`, `pages: write`, `id-token: write`, `attestations: write`, `packages: write`) only where needed.
3. **Concurrency**: every caller sets `concurrency.group: ${{ github.workflow }}-${{ github.ref }}`; `cancel-in-progress` is true for pull requests, false for the default branch and for releases and deploys.
4. **Timeouts**: every job has `timeout-minutes` (default 30).
5. **Checkout**: `persist-credentials: false` unless the job pushes.
6. **Untrusted input**: never interpolate `github.event.*` (titles, branch names, bodies) into `run:`; pass through `env:`. `pull_request_target` never checks out pull request code.
7. **Runners**: `ubuntu-latest` by default; heavy builds (Chromium, LLVM, Bun fork) run on the build host (`fr1`, self-hosted label) and never on a developer machine.
8. **One gate job**: every reusable CI ends with a job named `gate` that fails if any prerequisite failed, so the required check is one stable name.
9. **Workflows disabled on forks** unless the fork's own workflow is its purpose; dead upstream workflows never run on a schedule here.
10. **Environments** (`release`, `github-pages`, `production`) carry deployment secrets and protection rules; repository secrets are for CI only.

## 10. Reusable workflows

Located in `.github/workflows/` of this repository, called as `aphrody-labs/.github/.github/workflows/<file>@main`. Starter workflows (thin callers) live in `workflow-templates/` and appear under **Actions, New workflow** in every original.

| File | Purpose | Notes |
| :-- | :-- | :-- |
| `ci-bun.yml` | Bun and TypeScript: install frozen, lint, typecheck, test, build | `bun run --if-present` so missing scripts are skipped |
| `ci-rust.yml` | Cargo: fmt, clippy `-D warnings`, test; matrix over `os` | `Swatinem/rust-cache`, `dtolnay/rust-toolchain` |
| `ci-c-llvm.yml` | C and C++: LLVM install, CMake + Ninja, ctest, optional clang-format | requires `llvm-version` |
| `ci-go.yml` | Go: vet, `-race` tests, golangci-lint | reads `go.mod` |
| `docs.yml` | markdownlint, lychee, MDX compile | MDX check needs `title` and `description` |
| `pages.yml` | Pages deploy through Actions | environment `github-pages`; caller grants `pages: write`, `id-token: write` |
| `release.yml` | GitHub Release from a semver tag, generated notes, provenance attestations | `--verify-tag` |
| `publish-npm.yml` | npm publish with trusted publishing | section 12 |
| `publish-crates.yml` | crates.io publish with trusted publishing | section 12 |

Caller example (`.github/workflows/ci.yml`):

```yaml
name: ci
on:
  push:
    branches: [main]
  pull_request:
  merge_group:
permissions:
  contents: read
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}
jobs:
  ci:
    uses: aphrody-labs/.github/.github/workflows/ci-bun.yml@main
```

A reusable workflow in a public repository is callable by other public repositories; that is what makes this work for the whole organization on the free plan.

## 11. Dependabot

`templates/dependabot.yml` is the baseline; the apply phase writes one `updates` entry per manifest directory it finds and always adds `github-actions`.

- Ecosystems: `github-actions`, `bun`, `cargo`, `gomod`, `gitsubmodule`, `docker`, `nuget`, `uv`. (`bun` supports version updates, not security updates or vendoring.)
- Weekly on Monday; `open-pull-requests-limit: 5`.
- **Grouped updates**: one group `minor-and-patch` (`applies-to: version-updates`, `update-types: [minor, patch]`) and one `security` group (`applies-to: security-updates`). Majors arrive as individual pull requests.
- **Cooldown**: `default-days: 3`, `semver-major-days: 7`, so a freshly published release is not adopted the minute it appears. Security updates are not delayed.
- **`exclude-paths`**: `third_party/**`, `vendor/**`, `vendored/**`, `upstream/**`, `fixtures/**`, `testdata/**`.
- Labels `dependencies` plus the ecosystem label. Dependabot pull requests are squash-merged by the owner or automation when `ci / gate` is green.
- Open backlogs to clear first: `aphrody-m3` (207 alerts), `yolo` (44), `antigravity-sdk-python` (37), `shenron` (4).

## 12. Release and publishing

Tag-driven, one path for every stack:

1. Land the change on `main` (`ci / gate` green).
2. Push `vX.Y.Z`. `release.yml` (caller) runs the reusable `release.yml`: semver check, optional attestations (`actions/attest-build-provenance`), `gh release create --generate-notes --verify-tag`; tags containing `-` are prereleases.
3. The same caller runs the publish job for the stack.

Release notes come from `.github/release.yml` categories (Breaking, Features, Fixes, Documentation, Maintenance, Other) keyed on the labels in section 5. GitHub Releases are the changelog; a `CHANGELOG.md` exists only when users read the package outside GitHub. For multi-package monorepos that need version PRs, a release-PR tool may be added per repository as a documented deviation.

### npm: trusted publishing (OIDC), no `NPM_TOKEN`

- Requirements (npm docs): GitHub-hosted runner, npm CLI 11.5.1 or newer, Node 22.14.0 or newer, `id-token: write` in the caller and the reusable workflow, `package.json` `repository.url` identical to the GitHub URL.
- On npmjs.com, configure a Trusted Publisher per package: owner, repository, workflow filename of the **caller** (`release.yml`), optional environment. npm does not validate on save; the first publish must happen within 2 days or the configuration expires.
- Provenance is generated automatically for public repositories and public packages.
- `publish-npm.yml` publishes with the npm CLI (`npm publish --access public --tag <tag>`), after Bun builds the package.
- Until the trusted publisher exists, a granular token with expiry may live in an environment secret; the org-wide `NPM_TOKEN` (visible to all repositories today) is replaced and removed.

### crates.io: trusted publishing (OIDC), no `CARGO_REGISTRY_TOKEN`

- The crate must already exist (first publish with an API token, once). Then, in the crate settings, add a Trusted Publisher: owner, repository, workflow filename (`release.yml`), optional environment (`release`).
- `publish-crates.yml` uses `rust-lang/crates-io-auth-action` (pinned) with `id-token: write`; the exchanged token lives 30 minutes and is revoked at job end.

### Other stacks

Go: the tag is the release (module proxy). C and C++: release assets from the build job, attested. Containers: `ghcr.io` with `GITHUB_TOKEN`, `packages: write` on that job only.

## 13. Pages, wiki, discussions

- **Pages**: only when a repository has a built site. Source is **GitHub Actions** (`build_type: workflow`) through the reusable `pages.yml` (`configure-pages`, `upload-pages-artifact`, `deploy-pages`); never a `gh-pages` branch or `/docs` source. HTTPS enforced. Custom domains follow section 14. Today only `google-bun` has Pages, and its workflow fails: fix it or disable Pages.
- **Wiki**: off everywhere (21 empty wikis to disable). Documentation lives in the repository and is reviewed like code.
- **Discussions**: off by default; on for repositories with an outside audience (`discussions.enable_on`), with categories Announcements, Q&A, Ideas, Show and tell. Without Discussions, questions use the `question` label.
- **Issues**: on for originals, off for forks. **Projects**: off; planning lives in issues and the organization's task tooling.

## 14. Cloudflare

Policy (`[cloudflare]`), written from the 2026-11-17 inventory (1 account, 4 zones, 11 Workers, 10 R2 buckets, 2 queues, 4 tunnels, 10 Access apps, 7 account tokens; KV, D1, Pages and Hyperdrive empty).

1. **One resource per purpose.** No second Worker or bucket that serves the same purpose under another name. Duplicates are consolidated into one parameterized resource, not copied per hostname.
2. **Naming**: `<product>-<purpose>[-<env>]`, lowercase kebab (`aphrody-dist`, `shenron-cdn`, `github-events`). Add `-<env>` only when more than one environment exists.
3. **Roles**: Workers are edge compute and routing. R2 holds static assets, release artifacts, backups and large objects. Nothing else is added (KV, D1, Pages, Hyperdrive stay empty unless a purpose needs one).
4. **Config in the repository**: `wrangler.jsonc` beside the code, deployed from CI with `cloudflare/wrangler-action` (pinned) and a scoped API token in a GitHub environment secret (Cloudflare offers no GitHub OIDC for wrangler). Dashboard-only changes are drift.
5. **One server per hostname**: a hostname is served by an R2 custom domain (static) or a Worker route or custom domain (dynamic), never both. The current assets, dist, video and bun-lib hostnames violate this and are resolved at the domain-role stage.
6. **Exposure**: `workers.dev` disabled once a custom domain serves the Worker; `r2.dev` public access disabled (shenron-cdn after confirming its Worker is the sole reader).
7. **Tunnels**: one per host group; every ingress rule has a DNS record, or it is removed.
8. **Access**: one application per hostname; migration leftovers (`mig-prd-admin`, `mig-prd-ssh`, policy `mig-owner-only`) are removed only after the owner confirms the migration is finished.
9. **Tokens**: account-owned, one purpose each, name `<system>-<purpose>-<yyyy-mm>`, expiry 90 days at most, revoked after 30 idle days. The non-expiring `aphrody-full-2026-11-14` is replaced by scoped, expiring tokens and then rotated.
10. **Deletion rule**: delete only a resource proven duplicate or orphaned (no route, no binding, no traffic in the last 30 days); record id, name and reason in the commit or ledger entry that deletes it. Prefer disable or archive first. Dangling bindings (for example the missing KV and D1 ids of `aphrody-app`) are fixed or removed, not recreated.
11. **Writers to deleted buckets** (aphrody-archive and nine others still receive requests): find and stop the writers; recreate nothing.
12. **Cost**: R2 above 10 GB, Workers paid usage and any new resource are recorded in the budget ledger.
13. **Frozen**: no DNS, zone, route or custom-domain change on `aphrody.com` or `aphrody.app` until the owner decides the per-domain roles; both are inventory only, and the `aphrody` monorepo (agent `agy`) owns them.

## 15. GitHub Apps, OAuth apps, tokens, secrets

- **Apps**: install only what is used; `repository_selection` is `selected`, never `all` (the organization's own app is the single exception). Only the organization's own app may hold administration permission. An app with no event, check run or commit in 90 days is removed. Review quarterly. First review: cursor, neon-postgres, better-i18n (all hold broad permissions on all repositories), then claude, chatgpt-codex-connector and figma.
- **OAuth apps**: enable third-party application access restrictions in the organization settings and approve each application explicitly. This setting has no REST endpoint; it is a one-time settings change.
- **Tokens**: prefer GitHub App installation tokens (short-lived) over personal access tokens; fine-grained PATs only, expiry 90 days at most, one repository set per token. No classic PAT in CI.
- **Secrets**: organization secrets are visible to selected repositories only. `NPM_TOKEN`, `CARGO_REGISTRY_TOKEN` and `HOSTINGER_API_TOKEN` move from all to selected (today any fork workflow can read them); the first two are replaced by OIDC. Cloudflare credentials are environment secrets. Rotate every 90 days. Secrets are never printed or committed, in code, logs or issue text.
- **Members**: two-factor required; at most two admins; review members who do not push; pending admin invitations are confirmed by the owner or revoked.

## 16. Organization profile

`profile/README.md` is generated, not edited. `profile/generate.ts` reads the live repository lists of `aphrody-labs` and `aphrody-dev` through the GitHub API, drops private and archived repositories, and groups the rest by role using `[profile]` in standard.toml:

| Group | Source |
| :-- | :-- |
| Runtime and platform, Craft and design, Games and engines, Unix and userland, Tools and satellites | originals, by explicit repository list, name pattern, or the `aphrody-<role>` topic |
| Forks | forks, by family patterns, as collapsed lists of names |

Run `bun profile/generate.ts` (or `--check` in CI). `.github/workflows/readme.yml` runs it daily and commits when the output changed. The previous generator (`scripts/catalog.ts` of the retired `ay` repository) and its stale links are gone.

## 17. Apply order and verification

Order, each step reversible and recorded:

1. Land this repository (community files, forms, templates, reusable workflows).
2. Organization: security configurations created and attached, default permissions to `read`, 2FA, member and invitation review, apps restricted, secrets narrowed.
3. Per original: labels, settings, topics and description, `CODEOWNERS`, `dependabot.yml`, `ci.yml` caller, `LICENSE`, README gaps, wiki off, rulesets (tier), CodeQL.
4. Archive duplicates and dead originals (list in the inventory) after reference checks.
5. Forks: disable inherited workflows on the failing ones, issues off.
6. Cloudflare: delete only proven duplicates or orphans; the rest after the domain-role decision.

Steps 2 to 5 are `bun tools/apply.ts` (dry run by default, `--apply` to write, `--only=org,archive,settings,labels,security,rulesets,features,readme,forks,needs`, `--repo=owner/name`). Every step reads the live state and writes only the difference, so a dry run right after an apply must print no `would` line. Run on 2026-11-17: the free plan has no private-repository rulesets, auto-merge, secret scanning, non-provider patterns or validity checks, and `main-gated` stays deferred until the `ci / gate` check has run on the default branch of a repository.

Verification is by API, never by status code alone: `GET` the setting back and compare with standard.toml; screenshots or DOM for anything rendered (profile, Pages); the first green run of each reusable workflow; `bun tools/lint-standard.ts` for this repository.

## 18. Verified facts

Checked in the session of 2026-11-17 against live sources.

| Fact | Source |
| :-- | :-- |
| Ruleset rule types include `deletion`, `non_fast_forward`, `required_linear_history`, `pull_request`, `required_status_checks`, `merge_queue`, `required_signatures`, `workflows`, `code_scanning`; `bypass_mode` is `always`, `pull_request` or `exempt` | REST API, repository rules |
| A merge queue needs the `merge_group` event in required workflows | docs, managing a merge queue |
| Dependabot supports `groups` (`applies-to`, `patterns`, `update-types`), `cooldown` (`default-days`, `semver-*-days`), `exclude-paths`, `multi-ecosystem-groups`; `bun` ecosystem exists, without security updates | docs, Dependabot options reference and supported ecosystems |
| npm trusted publishing: npm 11.5.1+, Node 22.14.0+, `id-token: write`, caller workflow filename, GitHub-hosted runners only | docs.npmjs.com, trusted publishers |
| crates.io trusted publishing: crate must exist, 30 minute token, `rust-lang/crates-io-auth-action` | crates.io docs source (`rust-lang/crates.io`) |
| Default setup languages offered for a repository (`actions`, `javascript-typescript`, `python`, `rust`, ...) | `GET /repos/{owner}/{repo}/code-scanning/default-setup` |
| Security configuration fields and the unattached "GitHub recommended" (id 17) | `GET /orgs/aphrody-labs/code-security/configurations/17` |
| Action versions and SHAs in `[actions.pinned_versions]` | `gh api repos/<action>/releases/latest` and `commits/<tag>` |

Not verified here (the apply phase must try and record the HTTP status): availability of the `merge_queue` rule on this plan, security configuration attach on private repositories, and the exact `ci / gate` check name before enforcement.

## 19. Code, commits, pull requests and reviews

One standard for TypeScript and JavaScript on Bun, Rust (userland, kernel-facing, browser-facing), tests, comments, commits, pull requests and reviews: [docs/standards/code-style.md](docs/standards/code-style.md). Every rule carries a source tag, a section and a status (adopted, adapted, rejected); every source is fetched live and recorded with URL, retrieval date, sha256 and size in [docs/standards/sources.toml](docs/standards/sources.toml); disagreements between sources are decided in [docs/standards/conflicts.md](docs/standards/conflicts.md). Machine-readable keys: `[code_style]` and `[[files.templates]]` in standard.toml.

| Part | Where |
| :-- | :-- |
| Commit message format, template | [docs/standards/commit-format.md](docs/standards/commit-format.md), [templates/gitmessage](templates/gitmessage) |
| Commit and pull request title checker | `bun tools/check-commit.ts`, tests with `bun test tools` |
| Pull request template, issue forms | `.github/pull_request_template.md`, `.github/ISSUE_TEMPLATE/` |
| Review checklist | [templates/REVIEW.md](templates/REVIEW.md) |
| Bun formatter and linter settings | [templates/bunfig.standard.toml](templates/bunfig.standard.toml) |
| rustfmt, Cargo lints, clippy | [templates/rustfmt.toml](templates/rustfmt.toml), [templates/rustfmt.browser.toml](templates/rustfmt.browser.toml), [templates/rust-lints.toml](templates/rust-lints.toml), [templates/clippy.toml](templates/clippy.toml) |
| Source fetcher and measurement | `bun tools/standards-sources.ts`, `bun tools/bun-practice.ts` |

Precedence: a repository's own AGENTS.md and the conventions of a fork or vendored tree, then the output of the native tools, then the standard. For Bun-run code, `bun fmt` and `bun lint` win over any style guide.
