---
name: kernel-watch
description: >-
  Live state of the Linux mainline rc and stable kernels, Asterinas, Redox, LiberNT
  and the kernels shipped by Android, CachyOS, ChromiumOS, Alpine, NixOS, Fedora,
  Ubuntu, Arch and RHEL/CentOS Stream: versions and their distance to the mainline
  rc, recent commits, releases, issues, PRs and discussions, satellite sites and
  repositories. Use when a task depends on which kernel version a distribution
  ships, what changed upstream recently, or how Asterinas, Redox or LiberNT are
  moving; also for adding or fixing a watched source.
---

# Kernel watch

A scheduled workflow polls the kernel sources every 5 minutes (GitHub cron: minutes of latency, runs can be late or skipped) and commits regenerated pages under `docs/watch/`. Read those first; poll locally only when you need fresher data or full-text search.

## Where the answer is

1. `docs/watch/state-of-kernels.md`: condensed context (mainline rc, stable, longterm, distributions closest to the rc, one line per kernel). Start here.
2. `docs/watch/versions.md`: every observed channel with cycles behind the mainline rc, rc steps, and distance to stable.
3. `docs/watch/kernels/<id>.md`: per kernel, 24h and 7d activity, notable items, latest releases, organisation repositories, sites, sources. Ids: linux, asterinas, redox, libernt, android, cachyos, chromiumos, alpine, nixos, fedora, ubuntu, arch, rhel.
4. `docs/watch/data/*.json`: the same data for programs (`versions.json`, `digests.json`, `kernels.json`, `sources.json`).
5. `docs/watch/PLAN.md`: what is deliberately not covered (site deployment, sub-minute webhooks, full mailing-list ingestion, Phase B structure reports).

A `≥` before a commit count means the polled feed page did not reach back to the start of the window; treat it as a lower bound.

## Query the store

The store lives in the Actions cache, not in git. Build a local one in under a minute (public rate limits apply; `GITHUB_TOKEN` or `GH_TOKEN` is used when set):

```sh
cd tooling/kernel-watch
bun cli.ts poll                                   # conditional and resumable; run again later to update
bun cli.ts versions                               # newest kernel per distribution, closest to mainline rc first
bun cli.ts search "scheduler" --kernel asterinas  # FTS5; add --kind commit|issue|pr|release|discussion|mail|tag|package
bun cli.ts status                                 # per-source health, failures, next allowed poll
bun cli.ts build --out /tmp/watch                 # regenerate the pages somewhere else
```

Never commit a locally generated `docs/watch`; the workflow owns those files.

## Reading the versions

- Versions are ordered with Bun.semver after normalising (`7.4-rc3` < `7.4` < `7.4.1`); distro strings (`7.4.0-0.rc3.26.fc46`, `7.4rc3-1`, `6.12.0-284.el10`) are parsed to their kernel version.
- "Behind mainline rc" counts mainline cycles using the tag list of the mainline tree, so a major-version bump costs one cycle like any other.
- Alpine has no rc kernel (`linux-edge` is absent from aports); Arch's rc is the AUR `linux-mainline`; RHEL is read from CentOS Stream source-git; Ubuntu "mainline-ppa" is the mainline build archive, not what the archive ships.

## Change a source

Everything lives in `tooling/kernel-watch/sources.toml` (kernels, sites, organisations, sources). Add the entry, request the URL once and read the body before choosing an adapter (the adapter list is `ADAPTERS` in `adapters.ts`), then:

```sh
bun cli.ts validate
bun cli.ts verify --write     # live status and shape of every URL into fixtures/verified.json
bun test                      # hermetic, local Bun.serve replays fixtures/
```

Sources are read-only: no issues, PRs or comments are ever written upstream. Send no personal contact in the user agent.
