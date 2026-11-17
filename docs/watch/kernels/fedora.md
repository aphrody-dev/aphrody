# Fedora

Rawhide and stable Fedora kernels (Bodhi) plus the upstream-tracking kernel-ark tree.

Generated from the watch store as of 2026-11-17T06:00Z. Do not edit; run `bun tooling/kernel-watch/cli.ts build`.

## Kernel versions

| Rank | Kernel | Channel | Version | Behind mainline rc | Behind stable | First seen |
| --: | :-- | :-- | :-- | :-- | :-- | :-- |
| 1 | Fedora | F46 | 7.4.0-0.rc3.26.fc46 | at rc | 1 cycle ahead | 2026-11-17T04:59Z |
| 2 | Fedora | F43 | 7.3.4-100.fc43 | 1 cycle | current | 2026-11-17T04:59Z |
| 3 | Fedora | F44 | 7.3.4-200.fc44 | 1 cycle | current | 2026-11-17T04:59Z |
| 4 | Fedora | F45 | 7.3.4-300.fc45 | 1 cycle | current | 2026-11-17T04:59Z |
| 5 | Fedora | kernel-ark | 7.3.0-0.rc7.2f2f7eeef8b0.62 | 1 cycle | rc of next | 2026-11-17T04:59Z |

## Activity

| Window | Commits | Releases | Issues opened | PRs opened | PRs merged | Discussions | Mail | Tags |
| :-- | --: | --: | --: | --: | --: | --: | --: | --: |
| 24h | 14 | 0 | 0 | 2 | 10 | 0 | 0 | 1 |
| 7d | ≥150 | 0 | 0 | 13 | 19 | 0 | 0 | 12 |

A leading ≥ marks a lower bound: the polled feed page does not reach back to the start of the window.

## Version changes (7d)

- 2026-11-17T04:59Z kernel-ark: 7.3.0-0.rc7.2f2f7eeef8b0.62
- 2026-11-17T04:59Z kernel-ark: 7.4.0-0.rc3.26

## Notable (7d)

- 2026-11-16T16:13Z commit: [Merge branch 'makefile_cleanups_1' into 'os-build'](https://gitlab.com/cki-project/kernel-ark/-/commit/37c8448bd5ffa198037fcc3cbe4100c12cad5c69)
- 2026-11-16T16:13Z commit: [Merge branch 'configs/os-build/2026-10-29/drivers/platform' into 'os-build'](https://gitlab.com/cki-project/kernel-ark/-/commit/4b0c57960c8eb98b86e3aefab7769572c003d074)
- 2026-11-16T16:12Z commit: [Merge branch 'configs/os-build/2026-10-28/lib/Kconfig.debug' into 'os-build'](https://gitlab.com/cki-project/kernel-ark/-/commit/0174ba5aa9f7b10ec677a973fd7d3055a0582e5a)
- 2026-11-16T16:12Z commit: [Merge branch 'configs/os-build/2026-10-26/drivers/mtd' into 'os-build'](https://gitlab.com/cki-project/kernel-ark/-/commit/d1494a985e079cf5b15e6780e4025aaf7739fcb1)
- 2026-11-16T16:11Z commit: [Merge branch 'configs/os-build/2026-10-23/lib/Kconfig.debug' into 'os-build'](https://gitlab.com/cki-project/kernel-ark/-/commit/cada737df9cc79ebf8719c73c2d86ed0a8ba06fe)
- 2026-11-16T11:23Z tag: [kernel-7.4.0-0.rc3.26](https://gitlab.com/cki-project/kernel-ark/-/tags/kernel-7.4.0-0.rc3.26)
- 2026-11-16T11:07Z pr: [redhat/Makefile: restore variables lost by the EXPORTED_VARS conversion](https://gitlab.com/cki-project/kernel-ark/-/merge_requests/4892)
- 2026-11-15T22:30Z tag: [v7.4-rc3](https://gitlab.com/cki-project/kernel-ark/-/tags/v7.4-rc3)
- 2026-11-15T14:50Z pr: [Revert "phy: qcom: qmp-pcie: Skip PHY reset if already up"](https://gitlab.com/cki-project/kernel-ark/-/merge_requests/4891)
- 2026-11-15T11:21Z tag: [kernel-7.4.0-0.rc2.23c62304dde2.25](https://gitlab.com/cki-project/kernel-ark/-/tags/kernel-7.4.0-0.rc2.23c62304dde2.25)
- 2026-11-14T11:22Z tag: [kernel-7.4.0-0.rc2.97d73c825122.24](https://gitlab.com/cki-project/kernel-ark/-/tags/kernel-7.4.0-0.rc2.97d73c825122.24)
- 2026-11-14T09:04Z pr: [firmware: raspberrypi: Add bcm2712 revD compatible check](https://gitlab.com/cki-project/kernel-ark/-/merge_requests/4889)
- 2026-11-13T11:23Z tag: [kernel-7.4.0-0.rc2.7e1392a42ea2.23](https://gitlab.com/cki-project/kernel-ark/-/tags/kernel-7.4.0-0.rc2.7e1392a42ea2.23)
- 2026-11-12T16:44Z pr: [redhat: Tighten the default image policy in the UKI](https://gitlab.com/cki-project/kernel-ark/-/merge_requests/4887)
- 2026-11-11T06:33Z pr: [Backport for ipu-bridge and intel_cvs drivers](https://gitlab.com/cki-project/kernel-ark/-/merge_requests/4884)

## Sites and organisations

| Name | Kind | URL |
| :-- | :-- | :-- |
| dist-git | forge | https://src.fedoraproject.org/rpms/kernel |
| kernel-ark | gitlab org | https://gitlab.com/cki-project/kernel-ark |

## Sources

| Source | Kind | Yields | Interval | Verified |
| :-- | :-- | :-- | --: | :-- |
| [fedora-ark-commits](https://gitlab.com/api/v4/projects/cki-project%2Fkernel-ark/repository/commits?per_page=50) | forge-api | commit | 30m | 200 |
| [fedora-ark-merge-requests](https://gitlab.com/api/v4/projects/cki-project%2Fkernel-ark/merge_requests?state=all&order_by=updated_at&per_page=50) | forge-api | pr | 30m | 200 |
| [fedora-ark-tags](https://gitlab.com/api/v4/projects/cki-project%2Fkernel-ark/repository/tags?per_page=30&order_by=updated) | forge-api | tag, version | 30m | 200 |
| [fedora-bodhi](https://bodhi.fedoraproject.org/updates/?packages=kernel&status=stable&rows_per_page=40) | package-index | package, version | 30m | 200 |
