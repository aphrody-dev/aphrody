---
name: kernel-watch
description: >-
  Live state of the Linux mainline rc and stable kernels, Asterinas, Redox, LiberNT
  and the kernels shipped by Android, CachyOS, ChromiumOS, Alpine, NixOS, Fedora,
  Ubuntu, Arch and RHEL/CentOS Stream: versions and their distance to the mainline
  rc, recent commits, releases, issues, PRs and discussions, satellite sites and
  repositories. Use when a task depends on which kernel version a distribution
  ships, what changed upstream recently, or how Asterinas, Redox or LiberNT are
  moving; for the test lanes (read-only probe of the production Arch host, rc kernels booted in
  QEMU on the build host, the WSL2 kernel build); also for adding or fixing a watched source.
---

# Kernel watch

A scheduled workflow polls the kernel sources every 5 minutes (GitHub cron: minutes of latency) into a cache-only store; nothing generated is committed. Generated pages and records live in the git-ignored private store `.private/kernel-watch/watch/` (`KERNEL_WATCH_PRIVATE_DIR` overrides it); build them locally with `bun cli.ts build` after a poll. Paths below are relative to that directory unless they start with `docs/` or `tooling/`.

## Where the answer is

1. `state-of-kernels.md`: condensed context (mainline rc, stable, longterm, distributions closest to the rc, one line per kernel). Start here.
2. `versions.md`: every observed channel with cycles behind the mainline rc, rc steps, and distance to stable.
3. `kernels/<id>.md`: per kernel, 24h and 7d activity, notable items, latest releases, organisation repositories, sites, sources. Ids: linux, asterinas, redox, libernt, android, cachyos, chromiumos, alpine, nixos, fedora, ubuntu, arch, rhel.
4. `data/*.json`: the same data for programs (`versions.json`, `digests.json`, `kernels.json`, `sources.json`).
5. `aphrody/docs/watch/PLAN.md` (tracked, hand-written): what is deliberately not covered (site deployment, sub-minute webhooks, full mailing-list ingestion, Phase B structure reports) and, under "Test lanes", what each lane runs on, how to rerun it, what it proves and what it does not.
6. `probe.md`, `lane.md`, `wsl.md`: the lane pages (host probe of the production Arch host, rc kernels booted in QEMU with their smoke results, WSL2 kernel build and merge experiment). `versions.md` links each kernel to its lane result in a Test lane column.

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

Never commit generated output; it belongs in the private store.

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

## Test lanes: what runs where

| Lane | Host | Rerun |
| :-- | :-- | :-- |
| Host probe (running kernel, pending updates, newest Arch, testing, AUR and CachyOS kernels) | production Arch host over ssh, read-only; run it from the build host or a workstation, never from CI | `bun cli.ts probe production` (alias from `lanes.toml`; `--record DIR` saves the raw bodies as test fixtures) |
| rc lane (Arch baseline, AUR linux-mainline built at the rc tag, linux-cachyos-rc; boot in QEMU/KVM; smoke suite) | build host only (docker, /dev/kvm); the PC and production never build or boot | on the build host `bun lane-host.ts run --root DIR [--ids arch-mainline] [--if-new]`, `bun lane-host.ts install-timer --root DIR`; then anywhere `bun cli.ts lane-sync --root DIR` and `bun cli.ts build` |
| WSL2 kernel (Microsoft tree, config-wsl, tuned build, boot test, merge experiment) | build host only | `wsl/build.sh ROOT REF MARCH`, `wsl/merge-test.sh ROOT MSFT_TAG STABLE_TAG UPSTREAM_TAG`; install with `bun wsl/install-wsl-kernel.ts --from <dir|url>` (dry run unless `--apply`) |

Rules that hold for every lane:

- Production (the probe target) is read-only: no package install or sync of the live database, no unit, no write under /etc, /boot or /usr, never a build or test host. If a lane seems to need a write there, stop and report it.
- The PC runs no builds or VMs; kernel builds, containers and QEMU run on the build host, long jobs in the background.
- Nothing is written to an upstream (no issue, PR or comment on kernel.org trees, Arch, CachyOS or Microsoft); forks go to the organisation.
- Hosts are ssh aliases read from `lanes.toml` at run time; the lane root is passed as `--root`. No address, account or host path belongs in a tracked file.
- Tests are hermetic: recorded fixtures under `fixtures/probe` and `fixtures/lane`, no ssh, no QEMU, no network.
- Read `aphrody/docs/watch/PLAN.md` ("Test lanes") before claiming anything about a kernel: it lists what each lane proves and what it cannot (no real hardware, no hv_sock or dxgkrnl in the QEMU boot test, CachyOS repository packages lag its PKGBUILD).
