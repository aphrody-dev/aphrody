# Ubuntu

Archive kernels per series (Launchpad) and the mainline build PPA.

Generated from the watch store as of 2026-11-17T05:00Z. Do not edit; run `bun tooling/kernel-watch/cli.ts build`.

## Kernel versions

| Rank | Kernel | Channel | Version | Behind mainline rc | Behind stable | First seen |
| --: | :-- | :-- | :-- | :-- | :-- | :-- |
| 1 | Ubuntu | mainline-ppa | 7.4-rc3 | at rc | 1 cycle ahead | 2026-11-17T04:48Z |
| 2 | Ubuntu | stonking/proposed | 7.3.0-11.12 | 1 cycle | 4 patches | 2026-11-17T04:48Z |
| 3 | Ubuntu | tenacious/release | 7.3.0-9.9 | 1 cycle | 4 patches | 2026-11-17T04:48Z |
| 4 | Ubuntu | resolute/security | 7.0.0-43.43 | 4 cycles | 3 cycles | 2026-11-17T04:48Z |
| 5 | Ubuntu | noble/proposed | 6.8.0-151.151 | 16 cycles | 15 cycles | 2026-11-17T04:48Z |
| 6 | Ubuntu | jammy/security | 5.15.0-203.213 | 29 cycles | 28 cycles | 2026-11-17T04:48Z |

## Activity

| Window | Commits | Releases | Issues opened | PRs opened | PRs merged | Discussions | Mail | Tags |
| :-- | --: | --: | --: | --: | --: | --: | --: | --: |
| 24h | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 7d | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

A leading ≥ marks a lower bound: the polled feed page does not reach back to the start of the window.

## Sites and organisations

| Name | Kind | URL |
| :-- | :-- | :-- |
| wiki | docs | https://wiki.ubuntu.com/Kernel |
| launchpad | forge | https://launchpad.net/ubuntu/+source/linux |
| mainline-builds | site | https://kernel.ubuntu.com/mainline/ |

## Sources

| Source | Kind | Yields | Interval | Verified |
| :-- | :-- | :-- | --: | :-- |
| [ubuntu-launchpad](https://api.launchpad.net/devel/ubuntu/+archive/primary?ws.op=getPublishedSources&source_name=linux&exact_match=true&order_by_date=true&ws.size=30) | package-index | package, version | 30m | 200 |
| [ubuntu-mainline-builds](https://kernel.ubuntu.com/mainline/) | html-index | build, version | 60m | 200 |
