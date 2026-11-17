# Android Common Kernel

android-mainline and GKI branches of the Android Common Kernel.

Generated from the watch store as of 2026-11-17T05:00Z. Do not edit; run `bun tooling/kernel-watch/cli.ts build`.

## Kernel versions

| Rank | Kernel | Channel | Version | Behind mainline rc | Behind stable | First seen |
| --: | :-- | :-- | :-- | :-- | :-- | :-- |
| 1 | Android Common Kernel | android-mainline | 7.3.0 | 1 cycle | 4 patches | 2026-11-17T04:59Z |
| 2 | Android Common Kernel | gki (android18-7.3) | 7.3.0 | 1 cycle | 4 patches | 2026-11-17T04:59Z |

## Activity

| Window | Commits | Releases | Issues opened | PRs opened | PRs merged | Discussions | Mail | Tags |
| :-- | --: | --: | --: | --: | --: | --: | --: | --: |
| 24h | 4 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 7d | 7 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

A leading ≥ marks a lower bound: the polled feed page does not reach back to the start of the window.

## Notable (7d)

- 2026-11-16T16:12Z commit: [ANDROID: gki_defconfig: Remove CONFIG_DM_SNAPSHOT](https://android.googlesource.com/kernel/common/+/244c1268ab72dffe5792e78bb09302ec1c5ba286)
- 2026-11-16T11:28Z commit: [ANDROID: ARM64: gki_defconfig: Enable PKVM guest driver](https://android.googlesource.com/kernel/common/+/83c15a41357516c11fe500cf3e4c094c645bd9a4)
- 2026-11-16T06:56Z commit: [ANDROID: Remove ioremap_guard from the GKI and microdroid cmdlines](https://android.googlesource.com/kernel/common/+/c0d3ec4cfcca46c43e0263c63e12ea02f29bbdd7)
- 2026-11-16T06:52Z commit: [ANDROID: microdroid: add system dma heap module](https://android.googlesource.com/kernel/common/+/7d9575e8d1fd2f522f62a8c6f34590d173666395)
- 2026-11-12T13:21Z commit: [ANDROID: selftests/mm: Move MAP_DROPPABLE fallback to a compat header](https://android.googlesource.com/kernel/common/+/f3a85265e02b0a54ffd7f18cf317d090203405f8)

## Sites and organisations

| Name | Kind | URL |
| :-- | :-- | :-- |
| source-android | docs | https://source.android.com/docs/core/architecture/kernel/android-common |
| googlesource | forge | https://android.googlesource.com/kernel/common |

## Sources

| Source | Kind | Yields | Interval | Verified |
| :-- | :-- | :-- | --: | :-- |
| [android-gki-version](https://android.googlesource.com/kernel/common/+refs/heads?format=JSON) | gitiles | version | 360m | 200 |
| [android-mainline-log](https://android.googlesource.com/kernel/common/+log/refs/heads/android-mainline?format=JSON&n=50) | gitiles | commit | 30m | 200 |
| [android-mainline-version](https://android.googlesource.com/kernel/common/+/refs/heads/android-mainline/Makefile?format=TEXT) | gitiles | version | 30m | 200 |
