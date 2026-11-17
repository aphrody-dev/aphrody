# Host kernel probe

Generated from the recorded probes in `data/probe/`; do not edit. Each probe is a read-only run of `bun tooling/kernel-watch/cli.ts probe <id>` against an Arch host (see [PLAN.md](PLAN.md)). Packages are ranked by closeness to the mainline rc, the same way as [versions.md](versions.md).

## production: production Arch host (read-only)

Probed 2026-11-17T05:33Z. Reference: mainline 7.4-rc3, stable 7.3.4.

| | |
| :-- | :-- |
| Running kernel | 7.2.14-arch1-1 (7.2.14) |
| Build | #1 SMP PREEMPT_DYNAMIC Fri, 30 Oct 2026 14:32:40 +0000 |
| Behind mainline rc | 2 cycles |
| Behind stable | 1 cycle |
| Newest in Arch repositories | linux 7.3.4.arch1-2 (core) |
| Newest in testing repositories | none newer observed |
| Newest in the AUR | linux-mainline 7.4rc3-1 (aur) |
| Newest in CachyOS repositories | linux-cachyos-rc 7.4.rc2-1 (cachyos-znver4) |
| Running kernel behind newest Arch repository kernel | 1 cycle |
| Pending updates (all packages) | 94 |
| Host database sync | 2026-11-14T07:18Z |
| Repositories | core, extra, multilib, omarchy |

| Rank | Origin | Package | Version | Where | Behind mainline rc | Behind stable |
| --: | :-- | :-- | :-- | :-- | :-- | :-- |
| 1 | aur | [linux-mainline](https://aur.archlinux.org/packages/linux-mainline) | 7.4rc3-1 | aur | at rc | 1 ahead |
| 2 | aur | [linux-cachyos-rc](https://aur.archlinux.org/packages/linux-cachyos-rc) | 7.4.rc2-1 | aur (flagged out of date) | 1 rc behind | 1 ahead |
| 3 | cachyos | [linux-cachyos-rc](https://mirror.cachyos.org/repo/x86_64_v4/cachyos-znver4/linux-cachyos-rc-7.4.rc2-1-x86_64_v4.pkg.tar.zst) | 7.4.rc2-1 | cachyos-znver4 | 1 rc behind | 1 ahead |
| 4 | cachyos | [linux-cachyos-rc](https://mirror.cachyos.org/repo/x86_64/cachyos/linux-cachyos-rc-7.4.rc1-2-x86_64.pkg.tar.zst) | 7.4.rc1-2 | cachyos | 2 rc behind | 1 ahead |
| 5 | cachyos | [linux-cachyos-rc](https://mirror.cachyos.org/repo/x86_64_v3/cachyos-v3/linux-cachyos-rc-7.4.rc1-2-x86_64_v3.pkg.tar.zst) | 7.4.rc1-2 | cachyos-v3 | 2 rc behind | 1 ahead |
| 6 | repo | [linux](https://archlinux.org/packages/core/x86_64/linux/) | 7.3.4.arch1-2 | core | 1 cycle | current |
| 7 | host-repo | linux | 7.3.4.arch1-1 | core (host sync db) | 1 cycle | current |
| 8 | pending | linux | 7.3.4.arch1-2 | pending update from 7.2.14.arch1-1 | 1 cycle | current |
| 9 | cachyos | [linux-cachyos](https://mirror.cachyos.org/repo/x86_64_v3/cachyos-v3/linux-cachyos-7.3.4-1-x86_64_v3.pkg.tar.zst) | 7.3.4-1 | cachyos-v3 | 1 cycle | current |
| 10 | cachyos | [linux-cachyos](https://mirror.cachyos.org/repo/x86_64_v4/cachyos-v4/linux-cachyos-7.3.4-1-x86_64_v4.pkg.tar.zst) | 7.3.4-1 | cachyos-v4 | 1 cycle | current |
| 11 | cachyos | [linux-cachyos](https://mirror.cachyos.org/repo/x86_64_v4/cachyos-znver4/linux-cachyos-7.3.4-1-x86_64_v4.pkg.tar.zst) | 7.3.4-1 | cachyos-znver4 | 1 cycle | current |
| 12 | cachyos | [linux-cachyos-rc](https://mirror.cachyos.org/repo/x86_64_v4/cachyos-v4/linux-cachyos-rc-7.3.rc6-2-x86_64_v4.pkg.tar.zst) | 7.3.rc6-2 | cachyos-v4 | 1 cycle | rc of next |
| 13 | aur | [linux-xanmod](https://aur.archlinux.org/packages/linux-xanmod) | 7.3.4-1 | aur | 1 cycle | current |
| 14 | repo | [linux-zen](https://archlinux.org/packages/extra/x86_64/linux-zen/) | 7.3.4.zen2-2 | extra | 1 cycle | current |
| 15 | host-repo | linux-zen | 7.3.4.zen2-1 | extra (host sync db) | 1 cycle | current |
| 16 | cachyos | [linux-cachyos](https://mirror.cachyos.org/repo/x86_64/cachyos/linux-cachyos-7.3.3-1-x86_64.pkg.tar.zst) | 7.3.3-1 | cachyos | 1 cycle | 1 patch |
| 17 | running | running kernel | 7.2.14-arch1-1 | uname -r | 2 cycles | 1 cycle |
| 18 | installed | linux | 7.2.14.arch1-1 | local db | 2 cycles | 1 cycle |
| 19 | repo | [linux-hardened](https://archlinux.org/packages/extra/x86_64/linux-hardened/) | 7.2.15.hardened1-3 | extra | 2 cycles | 1 cycle |
| 20 | host-repo | linux-hardened | 7.2.15.hardened1-2 | extra (host sync db) | 2 cycles | 1 cycle |
| 21 | repo | [linux-rt](https://archlinux.org/packages/extra/x86_64/linux-rt/) | 7.2.15.rt3.arch1-4 | extra | 2 cycles | 1 cycle |
| 22 | host-repo | linux-rt | 7.2.15.rt3.arch1-3 | extra (host sync db) | 2 cycles | 1 cycle |
| 23 | aur | [linux-git](https://aur.archlinux.org/packages/linux-git) | 7.0.r11782.c1f49dea2b8f-1 | aur | 4 cycles | 3 cycles |
| 24 | repo | [linux-lts](https://archlinux.org/packages/core/x86_64/linux-lts/) | 6.18.63-3 | core | 6 cycles | 5 cycles |
| 25 | host-repo | linux-lts | 6.18.63-2 | core (host sync db) | 6 cycles | 5 cycles |
| 26 | repo | [linux-rt-lts](https://archlinux.org/packages/extra/x86_64/linux-rt-lts/) | 6.18.63.rt9.arch1-3 | extra | 6 cycles | 5 cycles |
| 27 | host-repo | linux-rt-lts | 6.18.63.rt9.arch1-2 | extra (host sync db) | 6 cycles | 5 cycles |

