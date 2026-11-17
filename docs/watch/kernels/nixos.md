# NixOS

nixpkgs kernel pins (kernels-org.json): testing (rc) plus every maintained series.

Generated from the watch store as of 2026-11-17T06:00Z. Do not edit; run `bun tooling/kernel-watch/cli.ts build`.

## Kernel versions

| Rank | Kernel | Channel | Version | Behind mainline rc | Behind stable | First seen |
| --: | :-- | :-- | :-- | :-- | :-- | :-- |
| 1 | NixOS | testing | 7.4-rc3 | at rc | 1 cycle ahead | 2026-11-17T04:59Z |
| 2 | NixOS | 7.3 | 7.3.4 | 1 cycle | current | 2026-11-17T04:59Z |
| 3 | NixOS | 6.18 lts | 6.18.63 | 6 cycles | 5 cycles | 2026-11-17T04:59Z |
| 4 | NixOS | 6.12 lts | 6.12.117 | 12 cycles | 11 cycles | 2026-11-17T04:59Z |
| 5 | NixOS | 6.6 lts | 6.6.162 | 18 cycles | 17 cycles | 2026-11-17T04:59Z |
| 6 | NixOS | 6.1 lts | 6.1.192 | 23 cycles | 22 cycles | 2026-11-17T04:59Z |

## Activity

| Window | Commits | Releases | Issues opened | PRs opened | PRs merged | Discussions | Mail | Tags |
| :-- | --: | --: | --: | --: | --: | --: | --: | --: |
| 24h | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 7d | 12 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

A leading ≥ marks a lower bound: the polled feed page does not reach back to the start of the window.

## Notable (7d)

- 2026-11-16T17:39Z commit: [linux_testing: 7.4-rc2 -> 7.4-rc3](https://github.com/NixOS/nixpkgs/commit/00aad0872e52b1690aeabc9eb2466d1eb70bc94c)
- 2026-11-11T17:37Z commit: [linux_zen: 7.3.4 -> 7.3.4](https://github.com/NixOS/nixpkgs/commit/7c68e22c2704fd87d2f2d1f442e0141ca6a9e60d)
- 2026-11-11T08:17Z commit: [linux_zen: 7.2.15 -> 7.3.4 (#586563)](https://github.com/NixOS/nixpkgs/commit/f049638e6b90672a12ceb3e8590c874640dbbfcf)
- 2026-11-11T06:22Z commit: [linux_xanmod, linux_xanmod_latest: 2026-11-10 (#586560)](https://github.com/NixOS/nixpkgs/commit/789789e55727b3995f598537c159bab9f774eb7e)
- 2026-11-11T05:14Z commit: [linux_zen: 7.2.15 -> 7.3.4](https://github.com/NixOS/nixpkgs/commit/c8075e55cf6a6ecc10d178b3abc3a1766fd0d441)

## Sites and organisations

| Name | Kind | URL |
| :-- | :-- | :-- |
| search | site | https://search.nixos.org/packages |
| nixpkgs | github org | https://github.com/NixOS/nixpkgs |

## Sources

| Source | Kind | Yields | Interval | Verified |
| :-- | :-- | :-- | --: | :-- |
| [nixos-kernel-commits](https://api.github.com/repos/NixOS/nixpkgs/commits?path=pkgs/os-specific/linux/kernel&per_page=50) | forge-api | commit | 30m | 200 |
| [nixos-kernels](https://raw.githubusercontent.com/NixOS/nixpkgs/master/pkgs/os-specific/linux/kernel/kernels-org.json) | package-index | version | 15m | 200 |
