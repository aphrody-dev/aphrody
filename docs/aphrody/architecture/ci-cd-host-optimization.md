# CI/CD & Build Host Optimization (`fr1` Ubuntu 26.10, Kernel 7.3, KVM, XFS, `sccache`)

> Grounded directly in Zed's existing infrastructure ([`script/setup-sccache`](../../../script/setup-sccache), [`script/linux`](../../../script/linux), [`script/bootstrap`](../../../script/bootstrap), [`script/clear-target-dir-if-larger-than`](../../../script/clear-target-dir-if-larger-than), [`tooling/xtask/src/tasks/workflows/runners.rs`](../../../tooling/xtask/src/tasks/workflows/runners.rs)) and live `fio` / `xfs_info` / `sysctl` measurements on `fr1`.

## Architecture & Storage Tier Routing

| Tier | Mount / Device | Measured Characteristics (`fr1`) | Optimal Workload Placement |
| :--- | :--- | :--- | :--- |
| **Tier 1 (RAM)** | `/tmp` (`tmpfs`, 23 GiB, 17 GiB free) | ~4,596 MiB/s read · ~1,970 MiB/s write | Linker temp (`TMPDIR=/tmp`), proc-macro scratch, `wasm-opt` / test sandboxes |
| **Tier 2 (NVMe)** | `/` (`/dev/sda1` `ext4`, `none` sched, `rota=0`) | ~199 MiB/s 4K randread · ~85.4 MiB/s randwrite (~51k IOPS) | `sccache` small-object cache (`/var/cache/aphrody/sccache`, 40 GiB cap), active `CARGO_TARGET_DIR`, `.cargo/registry` |
| **Tier 3 (XFS LVM)** | `/srv/xbuild` (`xbuild-data` XFS `reflink=1`, `agcount=12`, 568 GiB free) | Instant CoW reflinks (`cp --reflink=always` ~8–15 ms), ~4 MiB/s buffered Ceph tier | Cold mirrors (`/srv/xbuild/cold/third_party-store`), Docker/NativeLink CAS, multi-GB repo snapshots via `--reflink=auto` |

Placing small-object `sccache` (~40K random 4–64K reads/writes) on Ceph `/srv/xbuild` bottlenecks compilation at ~115–260 IOPS (`p99.95 > 2.7 s`), whereas keeping cold checkouts off `/` frees >34 GiB of local NVMe (`77% -> ~65%`) so `sccache` and active builds run at >190x higher random write IOPS.

## Zed Baseline Extensions

- **`script/cargo`**: Updated [`script/cargo`](../../../script/cargo) to recognize [`product.toml`](../../../product.toml) and `aphrody-labs/aphrody` remotes in addition to upstream `zed-industries/zed`.
- **Reproducible Profiles**: Declarative TOML profiles in [`assets/os/ci_host_profiles.toml`](../../../assets/os/ci_host_profiles.toml).
- **Live Optimizer CLI**: [`tooling/bun/ci-host-optimize.ts`](../../../tooling/bun/ci-host-optimize.ts) (`bun run host:audit`, `bun run host:apply`, `bun run host:bench`):
  - **Kernel 7.3 & XFS sysctl (`/etc/sysctl.d/96-aphrody-ci-kernel73.conf`)**: Enables unprivileged `io_uring`, sets `vm.vfs_cache_pressure = 50`, `vm.compaction_proactiveness = 20`, `vm.dirty_background_bytes = 256M`, `vm.dirty_bytes = 1G`, `inotify`, and BBR + fq alongside `90-fr1-chromium-host.conf`.
  - **KVM VirtIO Block Rule (`/etc/udev/rules.d/67-aphrody-ci-block.rules`)**: Clears erroneous `rotational=1` HDD flags across virtual `sd[a-z]/vd[a-z]`, raises `nr_requests=512`, `read_ahead_kb=1024`, keeps `none` on NVMe `sda`, and preserves cgroup-weighted `bfq` on secondary volumes.
  - **`sccache` + cgroups v2 Runner Drop-in**: Exports `RUSTC_WRAPPER=/usr/local/bin/sccache`, `SCCACHE_DIR=/var/cache/aphrody/sccache`, `SCCACHE_BASEDIRS=/srv/xbuild:/home/ubuntu:/srv`, `TMPDIR=/tmp`, and moves `actions.runner.*` services out of `system.slice` into `build.slice` (`MemoryHigh=34G`, `MemoryMax=38G`, `CPUWeight=100`).
- **Fleet Delta Runner**: [`tooling/bun/fleet-sync.ts`](../../../tooling/bun/fleet-sync.ts) exports `RUSTC_WRAPPER` and `SCCACHE_BASEDIRS`, runs `aphrody_pilot` + `aphrody_craft` + `aphrody_desktop` + `aphrody_ui` + `aphrody_store` + `brand`, and verifies headless Linux desktop PNGs cleanly.
