# Aphrody naming and independence, 1.0.0

Machine-readable form: `naming.toml`. Checkers: `check.ts` (one repository), `repos.ts` (the organisations, live). Date: 2026-11-17.

Owner's rule: every repository of aphrody-dev, aphrody-labs and rose-griffon, local and remote, is independent of the others, with coherent prefixes, namespaces and scopes, non-blocking, flexible and interchangeable.

## 1. Principles

1. A repository names itself. Its names come from its own `naming.toml` (id, kind, org, optional family), never from another repository's files. This file only holds the rules every declaration is checked against.
2. One prefix per repository, the same everywhere: GitHub name, crates, npm, PyPI, NuGet, Go, GHCR, Cloudflare, environment variables, binaries. The prefix is the repository id.
3. A repository builds, tests and releases alone from a clean clone, with no sibling checkout, no other repository's branch, no network access to another repository.
4. Cross-repository use goes through a published version (registry) or a pinned SHA, and is always optional (feature flag, fallback, or skip). No repository's CI waits for, triggers or reads another's.
5. Names are the sole coupling surface and registries never free a name: a published name stays, is documented as legacy, and new code uses the scheme name.

## 2. Orgs and kinds

| Org | Holds | npm scope | GHCR |
|---|---|---|---|
| aphrody-labs | OS (`aphrody`), SDK (`ay`), applications, runtime, crafts, research, configuration, forks and mirrors | `@aphrody` | `ghcr.io/aphrody-labs` |
| rose-griffon | the Rose Griffon site and tools (`rosegriffon`) | `@rosegriffon` | `ghcr.io/rose-griffon` |

Repository name per kind (all lowercase kebab-case, `[a-z0-9-]`, no dots, underscores or uppercase for originals):

| Kind | Rule | Examples |
|---|---|---|
| os | exactly `aphrody` | aphrody |
| sdk, nt, tool | one short word, 2 to 12 characters | ay, aynt |
| app, browser, craft, game, research, runtime | kebab-case product id | bun, nie, craft-anything, shenron, auto, open, ay-cf, ay-media, browser, msys, r2b |
| design | `aphrody-<short>` | aphrody-m3 |
| family | `<family>` for the root, `<family>-<unit>` for members, listed under `[families]` | yolo, yolo-ui, yolo-re |
| config | `.github`, `claude`, `gemini`, or `<id>-config` | shenron-config |
| fork, mirror | the upstream name; on a collision `<upstream-owner>-<name>` (or a declared alias such as `gfw-`); never a numeric suffix | zig, uutils-coreutils |
| archive | frozen as it was, never renamed or published from | legacy, ai-os, anu, ay-tools, aysh, ayvpn |

A fork that became a product is registered under the product's kind and follows that kind (aphrody started as a Zed fork and is kind `os`).

## 3. Names per registry

`{id}` is the repository id (the repository name unless `repo` differs), `{short}` is the id without a leading `aphrody-` (`aphrody-m3` gives `m3`, `aphrody` gives nothing), `{ID}` the id upper-cased with `_`, `{unit}` the package inside the repository. Across `aphrody-labs`, npm packages use the `@aphrody` scope and new non-exception Rust crates use the `ay-` prefix (`aphrody-` is reserved for `aphrody` and absorbed features in `ay`/`aphrody`), while `shenron`, `nie`, and `rosegriffon` keep their own prefixes and scopes.

| Registry | Rule | Example (id `ay`) | Example (id `aphrody`) |
|---|---|---|---|
| GitHub | `{org}/{id}` | aphrody-labs/ay | aphrody-labs/aphrody |
| crates.io | `ay-{unit}` (satellites), `aphrody-*` (`ay`/`aphrody`); `shenron-*`/`koharu-*`/`nie-*`/`rg-*` keep their own | ay-brand, ay-cf-edge, ay-auto-modele | aphrody-kernel |
| npm | `@aphrody/{short}-{unit}` (except `@rosegriffon/*` and `shenron`/`nie`); main package `@aphrody/{short}` | @aphrody/ay-chromium, @aphrody/auto-domain | @aphrody/kernel |
| PyPI | `{id}-{unit}`; import package `{id_snake}_{unit}` | ay-tools, ay_tools | aphrody-sdk |
| NuGet | `Aphrody.{Short}.{Unit}` | Aphrody.Ay.Interop | Aphrody.Kernel |
| Go | module path is the repository URL: `github.com/{org}/{repo}/{unit}` | github.com/aphrody-labs/ay/tools | github.com/aphrody-labs/aphrody |
| Docker | `ghcr.io/{org}/{repo}-{unit}`; GHCR only | ghcr.io/aphrody-labs/ay-tools | ghcr.io/aphrody-labs/aphrody |
| Cloudflare | `{id}-{unit}-{stage}`, stage in `prod`, `stg`, `dev`; workers, pages, D1, R2, KV, queues | ay-docs-prod | aphrody-api-stg |
| Environment | `{ID}_{NAME}`; platform and upstream variables (`CI`, `GITHUB_*`, `CARGO_*`, `BUN_*` ...) keep their own | AY_HOME | APHRODY_HOME |
| CLI binary | `{id}`; more binaries `{id}-{unit}` | ay, ay-docs | aphrody |

Exceptions that follow from the kind:
- fork and mirror keep every upstream name, byte for byte, in every registry (nothing of theirs is published by us);
- runtime (`bun`) keeps the upstream runtime's names as a drop-in (`bun`, `bun-types`, `BUN_*`); Aphrody-only units of it use `@aphrody/bun-<unit>` and `bun-<unit>` crates;
- family members may use the family prefix (`yolo-*`) instead of their own id;
- unpublished units (`publish = false`, `"private": true`) are internal and not checked for registry names.

Legacy names, kept because registries never free a name, are listed in `naming.toml` `[[legacy]]` and in the owning repository's `naming.toml` `[legacy]`; the checker reports them as notes, not warnings, and new code never adds one.

Crates.io today (365 crates owned by aphrody-dev, read live on 2026-11-17): `aphrody-*` 173, `bun-*` and `bun_*` 36 (browser, git, host, os units), `*craft-*` 130 (filmcraft, effectcraft, figcraft, deckcraft, cadcraft, designcraft, pdfcraft), `craft-*` 7, `koharu-*` 5, `bunos-*` 4, `yolo-*` 3, `shenron-*` 3, `obscura-*` 3, `dragon-pixel-engine`. Each belongs to the repository whose id it should carry; the mapping is in `[[legacy]]`. A rename is a new crate published by the owning repository's own release pipeline; nothing is published from here.

## 4. Declaration

`naming.toml` at the repository root:

```toml
id = "ay"                 # defaults to the repository name
kind = "sdk"              # a key of [kinds] in the scheme
org = "aphrody-dev"
# repo = "ay"             # only when the GitHub name differs from the id
# family = "yolo"         # members of a family

[names]                   # optional overrides of the org defaults
# npm_scope = "@aphrody"
# crate_prefix = "ay-"
# env_prefix = "AY_"
# upstream = true          # the workspace keeps an upstream's crate and package names (a fork that became a product)

[legacy]                  # names that exist in registries or on disk and stay until their pipeline replaces them
# crates = ["aphrody-"]
# npm = ["@aphrody/chromium"]
# env = ["APHRODY_"]
# repos = ["ANU"]

[exceptions]              # rule id = reason; the finding is shown as a note, not a warning
# I090 = "dev-only helper that is skipped when the checkout is absent"
```

A file may start with a comment line `naming-ignore-file` to be skipped (used by the checker's own sources and fixtures).

## 5. Independence rules

Severity: error = a clean clone or another repository's state decides the result; warn = needs an optional switch or a pin; info = allowed, shown.

| Rule | Detects | Severity |
|---|---|---|
| I001 | a submodule of another original repository of the family | error |
| I002 | a submodule inside the family (fork or mirror): must be pinned by gitlink and optional for the build | warn |
| I003 | relative submodule URL (`../x`), fine for forks and mirrors because they follow the org; shown | info |
| I004 | a submodule that tracks a branch (error for originals, warn for forks) | error, warn |
| I010 I013 | Cargo `path` dependency, patch or workspace member outside the repository | error |
| I011 I012 | Cargo `git` dependency on the family without `rev` (error) or required (warn) | error, warn |
| I020 I021 I022 | npm `file:`/`link:`/workspace outside the repository; install from GitHub of the family | error, warn |
| I030 I040 I041 I050 | Go `replace`, uv `path`/unpinned `git`, .NET `ProjectReference` outside the repository | error |
| I060 | image of another repository: pin a digest and keep a fallback | warn |
| I061 | Cloudflare service binding to another repository's Worker | error |
| I070 | workflow `uses` of another repository pinned to a SHA: allowed, keep it optional | info |
| I071 | workflow `uses` of another repository at a branch or tag (warn for the org's own reusable workflows in `.github`, which STANDARD.md calls at `@main`: pin a SHA to be fully independent) | error, warn |
| I072 | CI checks out another repository (`repository:`) | error |
| I073 | CI triggers a workflow or dispatch of another repository | error |
| I080 | a file of another repository fetched at a branch (`raw.githubusercontent.com/.../main/`) | error |
| I081 I082 | clone or release download of another repository at run time | warn |
| I090 | a path that reaches a sibling checkout (`../x` beyond the root, `$WORKSPACE_ROOT/x`, `C:\src\x`); error in code, note in docs and in `checkout =` hints | error, info |
| I091 | a shared `WORKSPACE_ROOT` layout; accept it only as an optional override | info |
| I100 | another repository's slug hardcoded: read it from this repository's identity file | info |
| I110 | a file generated from another repository: own the source or consume a published artifact | warn |
| I120 | reads an environment variable owned by another repository of the family | warn |

How to use another repository without coupling: (1) its published package at an exact version, resolved from the registry, in a clean clone; (2) a pinned SHA that CI fetches by itself into its own cache directory inside the repository (never next to it), behind a feature or a skip; (3) a documented path or URL read from an environment variable, whose absence skips the feature. A local sibling checkout is a developer convenience that may override (2), never a requirement.

Name conventions that keep repositories interchangeable: a repository never imports another's identity file or its `ay.toml`-style registry of siblings; where a tool needs the list of family repositories it takes it from `naming.toml` of this organisation or from the GitHub API.

## 6. Checking

```
bun naming/check.ts [repoDir]                 # report; exit 0 (a naming check never blocks)
bun naming/check.ts [repoDir] --strict        # exit 1 when an error is found
bun naming/check.ts [repoDir] --json | --all | --inventory
bun naming/repos.ts [--org aphrody-labs]      # GitHub repository names, live
bun test naming/check.test.ts
```

The checker reads the repository's tracked files only (`git ls-files`) and skips vendored trees (`node_modules`, `target`, `third_party`, `vendor`, `upstream`, `dist`, fixtures). Without a declaration it derives id, kind and org from the directory and the `[repos]` registry.

Optional CI step for a repository (no required job, no moving ref):

```yaml
- name: naming check
  continue-on-error: true
  run: |
    d="$RUNNER_TEMP/naming"; mkdir -p "$d"
    base="https://raw.githubusercontent.com/aphrody-labs/.github/<SHA>/naming"
    curl -fsSL "$base/check.ts" -o "$d/check.ts" && curl -fsSL "$base/naming.toml" -o "$d/naming.toml" || { echo "naming checker unavailable, skipped"; exit 0; }
    bun "$d/check.ts" . --strict
```
