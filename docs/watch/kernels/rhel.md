# RHEL / CentOS Stream

CentOS Stream kernel source-git, the public upstream of RHEL kernels (RHEL itself publishes no kernel API).

Generated from the watch store as of 2026-11-17T05:00Z. Do not edit; run `bun tooling/kernel-watch/cli.ts build`.

## Kernel versions

| Rank | Kernel | Channel | Version | Behind mainline rc | Behind stable | First seen |
| --: | :-- | :-- | :-- | :-- | :-- | :-- |
| 1 | RHEL / CentOS Stream | centos-stream-10 | 6.12.0 | 12 cycles | 11 cycles | 2026-11-17T04:59Z |
| 2 | RHEL / CentOS Stream | centos-stream-9 | 5.14.0 | 30 cycles | 29 cycles | 2026-11-17T04:59Z |

## Activity

| Window | Commits | Releases | Issues opened | PRs opened | PRs merged | Discussions | Mail | Tags |
| :-- | --: | --: | --: | --: | --: | --: | --: | --: |
| 24h | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 7d | 30 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

A leading ≥ marks a lower bound: the polled feed page does not reach back to the start of the window.

## Notable (7d)

- 2026-11-11T09:11Z commit: [[redhat] kernel-5.14.0-765.el9](https://gitlab.com/redhat/centos-stream/src/kernel/centos-stream-9/-/commit/c6faf147119cf8cb37e71922b3afdacafb6f73ab)
- 2026-11-11T09:10Z commit: [Merge: CVE-2026-99032: net/smc: protect clcsock lifetime in smc_getname](https://gitlab.com/redhat/centos-stream/src/kernel/centos-stream-9/-/commit/3ab2f3cfd4b24ad1775915e8e19a86d417692944)
- 2026-11-11T09:10Z commit: [Merge: CVE-2026-68451: s390/zcrypt: Validate length for CCA ECC private key requests](https://gitlab.com/redhat/centos-stream/src/kernel/centos-stream-9/-/commit/c938a4b154e1e8e06e2624b8832904140811c3d7)
- 2026-11-11T09:10Z commit: [Merge: CVE-2026-68452: s390/zcrypt: Validate length for CCA AES cipher key requests](https://gitlab.com/redhat/centos-stream/src/kernel/centos-stream-9/-/commit/6974d67c51b389d30c70029fcbec66554e930fa9)
- 2026-11-11T09:10Z commit: [Merge: scsi: qla2xxx: Derive physical port from real INTx pin on Hyper-V DDA guests](https://gitlab.com/redhat/centos-stream/src/kernel/centos-stream-9/-/commit/904417c83c5a397fc418c0a37b90cfe62a5ac7a4)

## Sites and organisations

| Name | Kind | URL |
| :-- | :-- | :-- |
| centos-stream-10 | gitlab org | https://gitlab.com/redhat/centos-stream/src/kernel/centos-stream-10 |

## Sources

| Source | Kind | Yields | Interval | Verified |
| :-- | :-- | :-- | --: | :-- |
| [rhel-c10s-commits](https://gitlab.com/api/v4/projects/redhat%2Fcentos-stream%2Fsrc%2Fkernel%2Fcentos-stream-10/repository/commits?per_page=30) | forge-api | commit, version | 60m | 200 |
| [rhel-c9s-commits](https://gitlab.com/api/v4/projects/redhat%2Fcentos-stream%2Fsrc%2Fkernel%2Fcentos-stream-9/repository/commits?per_page=30) | forge-api | commit, version | 60m | 200 |
