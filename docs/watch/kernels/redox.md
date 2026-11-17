# Redox

Rust microkernel and userland, forge on GitLab.

Generated from the watch store as of 2026-11-17T05:00Z. Do not edit; run `bun tooling/kernel-watch/cli.ts build`.

## Activity

| Window | Commits | Releases | Issues opened | PRs opened | PRs merged | Discussions | Mail | Tags |
| :-- | --: | --: | --: | --: | --: | --: | --: | --: |
| 24h | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 7d | 2 | 0 | 0 | 2 | 2 | 0 | 0 | 0 |

A leading ≥ marks a lower bound: the polled feed page does not reach back to the start of the window.

| Metric | Now | 24h delta | 7d delta |
| :-- | --: | --: | --: |
| forks | 97 | n/a | n/a |
| open_issues | 66 | n/a | n/a |
| open_prs | 16 | n/a | n/a |
| stars | 37 | n/a | n/a |

## Notable (7d)

- 2026-11-14T13:36Z commit: [refactor: Use args() and defaults for Sqe initialization](https://gitlab.redox-os.org/redox-os/kernel/-/commit/8f0e965e99856fc6215294663fa1a9d1018a7eb2)
- 2026-11-14T13:34Z pr: [refactor: Use args() and defaults for Sqe initialization](https://gitlab.redox-os.org/redox-os/kernel/-/merge_requests/776)
- 2026-11-10T12:31Z commit: [Use node-local kernel stacks](https://gitlab.redox-os.org/redox-os/kernel/-/commit/f31b7512dc381b9fed09b9a641da717092d83e65)
- 2026-11-06T12:33Z pr: [Local Kernel Stacks](https://gitlab.redox-os.org/redox-os/kernel/-/merge_requests/768)

## Organisation repositories

| Repository | Stars | Pushed | Description |
| :-- | --: | :-- | :-- |
| [redox-os/relibc](https://gitlab.redox-os.org/redox-os/relibc) | 41 | 2026-11-17 | C Library in Rust for Redox and Linux (WIP) |
| [redox-os/redox](https://gitlab.redox-os.org/redox-os/redox) | 136 | 2026-11-17 | Redox: A Rust Operating System |
| [redox-os/liborbital](https://gitlab.redox-os.org/redox-os/liborbital) | 0 | 2026-11-16 | C bindings to the Orbital client library |
| [redox-os/base](https://gitlab.redox-os.org/redox-os/base) | 2 | 2026-11-16 | Essential system components and drivers. |
| [redox-os/test-results](https://gitlab.redox-os.org/redox-os/test-results) | 1 | 2026-11-16 |  |
| [redox-os/games](https://gitlab.redox-os.org/redox-os/games) | 4 | 2026-11-16 | Games for Redox! |
| [redox-os/pkgutils](https://gitlab.redox-os.org/redox-os/pkgutils) | 12 | 2026-11-16 | Redox Packaging Utilities |
| [redox-os/pkgar](https://gitlab.redox-os.org/redox-os/pkgar) | 4 | 2026-11-15 | Package Archive |
| [redox-os/cargo](https://gitlab.redox-os.org/redox-os/cargo) | 0 | 2026-11-15 | The Rust package manager |
| [redox-os/backtrace-rs](https://gitlab.redox-os.org/redox-os/backtrace-rs) | 0 | 2026-11-15 |  |
| [redox-os/kernel](https://gitlab.redox-os.org/redox-os/kernel) | 37 | 2026-11-15 | The Redox microkernel |
| [redox-os/book](https://gitlab.redox-os.org/redox-os/book) | 15 | 2026-11-14 | The Redox book - https://doc.redox-os.org/book/ |
| [redox-os/website](https://gitlab.redox-os.org/redox-os/website) | 12 | 2026-11-14 | The Redox website (blog, news, feed, etc.) |
| [redox-os/orbclient](https://gitlab.redox-os.org/redox-os/orbclient) | 5 | 2026-11-14 | The Orbital Client Library. Compatible with Redox and SDL2. |
| [redox-os/orbutils](https://gitlab.redox-os.org/redox-os/orbutils) | 3 | 2026-11-13 | The Orbital Utilities. Compatible with Redox and SDL2. |

## Sites and organisations

| Name | Kind | URL |
| :-- | :-- | :-- |
| site | site | https://www.redox-os.org/ |
| book | book | https://doc.redox-os.org/book/ |
| rfcs | rfc | https://gitlab.redox-os.org/redox-os/rfcs |
| redox-os | gitlab org | https://gitlab.redox-os.org/redox-os |

## Sources

| Source | Kind | Yields | Interval | Verified |
| :-- | :-- | :-- | --: | :-- |
| [redox-commits](https://gitlab.redox-os.org/api/v4/projects/redox-os%2Fkernel/repository/commits?per_page=100) | forge-api | commit | 10m | 200 |
| [redox-counts](https://gitlab.redox-os.org/api/v4/projects/redox-os%2Fkernel/merge_requests?state=opened&per_page=1) | forge-api | metric | 30m | 200 |
| [redox-group-projects](https://gitlab.redox-os.org/api/v4/groups/redox-os/projects?per_page=100&order_by=last_activity_at) | forge-api | repo | 720m | 200 |
| [redox-issues](https://gitlab.redox-os.org/api/v4/projects/redox-os%2Fkernel/issues?state=all&order_by=updated_at&per_page=100) | forge-api | issue | 10m | 200 |
| [redox-merge-requests](https://gitlab.redox-os.org/api/v4/projects/redox-os%2Fkernel/merge_requests?state=all&order_by=updated_at&per_page=100) | forge-api | pr | 10m | 200 |
| [redox-news](https://www.redox-os.org/news/index.xml) | feed | news | 360m | 200 |
| [redox-project](https://gitlab.redox-os.org/api/v4/projects/redox-os%2Fkernel) | forge-api | metric | 60m | 200 |
| [redox-releases](https://gitlab.redox-os.org/api/v4/projects/redox-os%2Fkernel/releases?per_page=30) | forge-api | release | 60m | 200 |
| [redox-sitemap](https://www.redox-os.org/sitemap.xml) | sitemap | page | 720m | 200 |
| [redox-tags](https://gitlab.redox-os.org/api/v4/projects/redox-os%2Fkernel/repository/tags?per_page=50) | forge-api | tag | 60m | 200 |
