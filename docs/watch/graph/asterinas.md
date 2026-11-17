# Asterinas: structure report

Generated on the build host by `tooling/kernel-watch/graph.ts` from blob-less shallow clones; each section names the commit it was built from. Structure and dependency level: tree, languages, subsystem inventory, dependency graph, enforced style. There is no symbol-level code graph.

## asterinas

Built from [f691f8ecccce](https://github.com/asterinas/asterinas/commit/f691f8ecccce1973120cabd66a13d5e8f92f33e6) on `main`, committed 2026-11-16T21:00:01-08:00. 2,972 tracked files, 11.8 MiB, 333,287 lines of code.

### Top-level tree

| Entry | Files | Size | Code lines | Main language |
| :-- | :-- | :-- | :-- | :-- |
| `kernel/` | 1,255 | 6.5 MiB | 207,703 | Rust |
| `ostd/` | 362 | 1.9 MiB | 57,569 | Rust |
| `test/` | 844 | 1.5 MiB | 49,230 | C |
| `book/` | 210 | 0.6 MiB | 408 | Documentation |
| `.agents/` | 46 | 0.4 MiB | 2,422 | YAML |
| `osdk/` | 103 | 0.4 MiB | 10,620 | Rust |
| `tools/` | 47 | 0.2 MiB | 4,739 | Rust |
| `(root files)` | 27 | 0.2 MiB | 551 | Documentation |
| `.github/` | 39 | 0.1 MiB | 0 | YAML |
| `distro/` | 35 | 0.1 MiB | 45 | Shell |
| `.vscode/` | 1 | 0.0 MiB | 0 | JSON |
| `.devcontainer/` | 1 | 0.0 MiB | 0 | JSON |
| `.claude/` | 2 | 0.0 MiB | 0 | JSON |

### Languages

| Language | Files | Size | Code lines |
| :-- | :-- | :-- | :-- |
| Rust | 1,570 | 8.8 MiB | 279,026 |
| C | 253 | 1.0 MiB | 39,422 |
| Documentation | 175 | 0.7 MiB | 0 |
| Other | 331 | 0.5 MiB | 0 |
| YAML | 160 | 0.3 MiB | 0 |
| Shell | 197 | 0.3 MiB | 9,006 |
| Assembly | 40 | 0.1 MiB | 3,610 |
| TOML | 135 | 0.1 MiB | 0 |
| Text | 9 | 0.1 MiB | 0 |
| Make | 90 | 0.1 MiB | 1,871 |
| Python | 3 | 0.0 MiB | 352 |
| JSON | 9 | 0.0 MiB | 0 |

### Cargo workspace

59 workspace crates, 225 packages in the resolved graph, 203 workspace-internal dependency edges.

| Crate | Path | Version | Edition | Workspace deps | External deps |
| :-- | :-- | :-- | :-- | :-- | :-- |
| `asterinas` | `kernel` | 0.1.0 | 2024 | aster-core, aster-drm, aster-i8042, aster-simpledrm, aster-uart, ostd | 0 |
| `aster-drm` | `kernel/comps/drm` | 0.1.0 | 2024 | aster-core, aster-util, atomic-integer-wrapper, device-id, int-to-c-enum, ostd, ostd-pod, sparse-id-alloc | 5 |
| `aster-i8042` | `kernel/comps/i8042` | 0.1.0 | 2024 | aster-cmdline, aster-core, aster-input, component, ostd | 2 |
| `aster-simpledrm` | `kernel/comps/simpledrm` | 0.1.0 | 2024 | aster-core, aster-drm, aster-framebuffer, component, ostd | 0 |
| `aster-uart` | `kernel/comps/uart` | 0.1.0 | 2024 | aster-console, aster-util, component, fdt-util, ostd | 3 |
| `aster-core` | `kernel/core` | 0.1.0 | 2024 | align_ext, aster-bigtcp, aster-block, aster-cmdline, aster-console, aster-device, aster-framebuffer, aster-fuse, aster-input, aster-logger, aster-network, aster-nvme, aster-rights, aster-rights-proc, aster-softirq, aster-systree, aster-time, aster-util, aster-virtio, atomic-integer-wrapper, component, cpio-decoder, device-id, id-alloc, int-to-c-enum, io-util, jhash, keyable-arc, logo-ascii-art, osdk-frame-allocator, osdk-heap-allocator, ostd, ostd-pod, ring-buffer, short-vis-path, sparse-id-alloc, typeflags, typeflags-util, xarray | 23 |
| `aster-block` | `kernel/core/comps/block` | 0.1.0 | 2024 | align_ext, aster-util, component, device-id, dma-pool, id-alloc, int-to-c-enum, io-util, ostd, ostd-pod | 2 |
| `aster-cmdline` | `kernel/core/comps/cmdline` | 0.1.0 | 2024 | component, ostd | 2 |
| `aster-console` | `kernel/core/comps/console` | 0.1.0 | 2024 | component, ostd | 1 |
| `aster-device` | `kernel/core/comps/device` | 0.1.0 | 2024 | aster-systree, aster-util, component, device-id, ostd | 1 |
| `aster-framebuffer` | `kernel/core/comps/framebuffer` | 0.1.0 | 2024 | align_ext, component, int-to-c-enum, ostd | 3 |
| `aster-input` | `kernel/core/comps/input` | 0.1.0 | 2024 | component, ostd, ostd-pod | 4 |
| `aster-logger` | `kernel/core/comps/logger` | 0.1.0 | 2024 | aster-console, component, ostd | 1 |
| `aster-mlsdisk` | `kernel/core/comps/mlsdisk` | 0.1.0 | 2024 | aster-block, component, device-id, ostd, ostd-pod | 10 |
| `aster-network` | `kernel/core/comps/network` | 0.1.0 | 2024 | aster-bigtcp, aster-softirq, component, ostd | 1 |
| `aster-nvme` | `kernel/core/comps/nvme` | 0.1.0 | 2024 | aster-block, aster-pci, aster-util, component, device-id, io-util, ostd, ostd-pod | 2 |
| `aster-pci` | `kernel/core/comps/pci` | 0.1.0 | 2024 | align_ext, component, fdt-util, int-to-c-enum, ostd | 3 |
| `aster-softirq` | `kernel/core/comps/softirq` | 0.1.0 | 2024 | aster-util, component, ostd | 2 |
| `aster-systree` | `kernel/core/comps/systree` | 0.1.0 | 2024 | aster-util, component, id-alloc, ostd | 3 |
| `aster-time` | `kernel/core/comps/time` | 0.1.0 | 2024 | aster-util, component, fdt-util, ostd | 4 |
| `aster-virtio` | `kernel/core/comps/virtio` | 0.1.0 | 2024 | aster-bigtcp, aster-block, aster-cmdline, aster-console, aster-fuse, aster-input, aster-network, aster-pci, aster-rights, aster-softirq, aster-util, component, device-id, dma-pool, fdt-util, id-alloc, int-to-c-enum, io-util, ostd, ostd-pod, short-vis-path, typeflags-util | 5 |
| `aster-bigtcp` | `kernel/libs/aster-bigtcp` | 0.1.0 | 2024 | aster-softirq, aster-util, dma-pool, int-to-c-enum, jhash, ostd, ostd-pod | 5 |
| `aster-fuse` | `kernel/libs/aster-fuse` | 0.1.0 | 2024 | int-to-c-enum, ostd, ostd-pod | 2 |
| `aster-rights` | `kernel/libs/aster-rights` | 0.1.0 | 2024 | typeflags, typeflags-util | 1 |
| `aster-rights-proc` | `kernel/libs/aster-rights-proc` | 0.1.0 | 2024 | - | 3 |
| `aster-util` | `kernel/libs/aster-util` | 0.1.0 | 2024 | aster-rights, aster-rights-proc, osdk-heap-allocator, ostd, ostd-pod, typeflags-util | 3 |
| `atomic-integer-wrapper` | `kernel/libs/atomic-integer-wrapper` | 0.1.0 | 2024 | - | 3 |
| `component` | `kernel/libs/comp-sys/component` | 0.1.0 | 2024 | component-macro | 3 |
| `component-macro` | `kernel/libs/comp-sys/component-macro` | 0.1.0 | 2024 | - | 6 |
| `cpio-decoder` | `kernel/libs/cpio-decoder` | 0.1.0 | 2024 | int-to-c-enum | 1 |
| `device-id` | `kernel/libs/device-id` | 0.1.0 | 2024 | aster-util | 1 |
| `dma-pool` | `kernel/libs/dma-pool` | 0.1.0 | 2024 | aster-softirq, aster-util, ostd | 1 |
| `fdt-util` | `kernel/libs/fdt-util` | 0.1.0 | 2024 | ostd | 3 |
| `io-util` | `kernel/libs/io-util` | 0.1.0 | 2024 | - | 1 |
| `jhash` | `kernel/libs/jhash` | 0.1.0 | 2024 | - | 0 |
| `keyable-arc` | `kernel/libs/keyable-arc` | 0.1.0 | 2024 | - | 0 |
| `logo-ascii-art` | `kernel/libs/logo-ascii-art` | 0.1.0 | 2024 | - | 1 |
| `ring-buffer` | `kernel/libs/ring-buffer` | 0.1.0 | 2024 | ostd, ostd-pod | 1 |
| `short-vis-path` | `kernel/libs/short-vis-path` | 0.1.0 | 2024 | - | 3 |
| `sparse-id-alloc` | `kernel/libs/sparse-id-alloc` | 0.1.0 | 2024 | - | 0 |
| `typeflags` | `kernel/libs/typeflags` | 0.1.0 | 2024 | - | 4 |
| `typeflags-util` | `kernel/libs/typeflags-util` | 0.1.0 | 2024 | - | 0 |
| `xarray` | `kernel/libs/xarray` | 0.1.0 | 2024 | ostd | 0 |
| `osdk-frame-allocator` | `osdk/deps/frame-allocator` | 0.19.0 | 2024 | ostd | 1 |
| `osdk-heap-allocator` | `osdk/deps/heap-allocator` | 0.19.0 | 2024 | ostd | 1 |
| `osdk-test-kernel` | `osdk/deps/test-kernel` | 0.19.0 | 2024 | ostd | 1 |
| `ostd` | `ostd` | 0.19.0 | 2024 | align_ext, id-alloc, int-to-c-enum, linux-boot-params, ostd-macros, ostd-pod, ostd-test | 28 |
| `align_ext` | `ostd/libs/align_ext` | 0.1.0 | 2024 | - | 0 |
| `id-alloc` | `ostd/libs/id-alloc` | 0.1.0 | 2024 | - | 1 |
| `int-to-c-enum` | `ostd/libs/int-to-c-enum` | 0.1.0 | 2024 | int-to-c-enum-derive | 0 |
| `int-to-c-enum-derive` | `ostd/libs/int-to-c-enum/derive` | 0.1.0 | 2024 | - | 3 |
| `linux-boot-params` | `ostd/libs/linux-bzimage/boot-params` | 0.19.0 | 2024 | - | 0 |
| `linux-bzimage-builder` | `ostd/libs/linux-bzimage/builder` | 0.19.0 | 2024 | align_ext, ostd-pod | 6 |
| `linux-bzimage-setup` | `ostd/libs/linux-bzimage/setup` | 0.19.0 | 2024 | linux-boot-params | 8 |
| `ostd-macros` | `ostd/libs/ostd-macros` | 0.19.0 | 2024 | - | 4 |
| `ostd-pod` | `ostd/libs/ostd-pod` | 0.4.0 | 2024 | ostd-pod-macros, padding-struct | 1 |
| `ostd-pod-macros` | `ostd/libs/ostd-pod/macros` | 0.4.0 | 2024 | - | 3 |
| `ostd-test` | `ostd/libs/ostd-test` | 0.19.0 | 2024 | - | 0 |
| `padding-struct` | `ostd/libs/padding-struct` | 0.2.0 | 2024 | - | 3 |

Most used external crates: spin (20), bitflags (14), zerocopy (13), proc-macro2 (9), quote (9), syn (9), bitvec (7), inherit-methods-macro (7), fdt (4), log (4), smallvec (4), tdx-guest (4), hashbrown (3), inventory (3), lending-iterator (3).

### Linux syscall coverage

The book documents supported syscalls in System Call Matching Language files: 244 syscalls described (162 fully covered without flag-level caveats, 82 with flag-level patterns).

| Category | SCML files | Fully covered | Flag-level |
| :-- | :-- | :-- | :-- |
| file-and-directory-operations | 10 | 60 | 11 |
| file-descriptor-and-io-control | 10 | 18 | 13 |
| file-systems-and-mount-control | 9 | 8 | 10 |
| inter-process-communication | 5 | 1 | 4 |
| memory-management | 6 | 1 | 6 |
| namespaces-cgroups-and-security | 5 | 2 | 5 |
| networking-and-sockets | 9 | 4 | 13 |
| process-and-thread-management | 7 | 42 | 8 |
| signals-and-timers | 6 | 19 | 5 |
| system-information-and-misc | 8 | 7 | 7 |

### Enforced style

Configuration files present: `.editorconfig`, `rustfmt.toml`, `clippy.toml`, `rust-toolchain.toml`, `CODEOWNERS`.

- rustfmt.toml: `{"imports_granularity":"Crate","group_imports":"StdExternalCrate","reorder_imports":true,"skip_macro_invocations":["chmod","mkmod","ioc"]}`
- clippy.toml: `{"check-inconsistent-struct-field-initializers":true}`
- rust-toolchain.toml: `{"toolchain":{"channel":"nightly-2026-07-21","components":["rust-src","rustc-dev","llvm-tools-preview"],"targets":["x86_64-unknown-none","riscv64imac-unknown-none-elf","loongarch64-unknown-none-softfloat","aarch64-unknown-none-softfloat"]}}`
- workspace lints: `{"rust":{"redundant_imports":"warn","unexpected_cfgs":{"level":"allow","check-cfg":["cfg(ktest)"]},"unreachable_pub":"warn","unused_qualifications":"warn"},"clippy":{"allow_attributes":"warn","inconsistent_struct_constructor":"warn","redundant_test_prefix":"warn"}}`
- .editorconfig: `{"*":{"root":"true","end_of_line":"lf","insert_final_newline":"true"}}`
- CI workflows: benchmark_x86.yml, benchmark_x86_tdx.yml, check_licenses.yml, close_stale_issues_and_prs.yml, invoke_aster_code_review.yml, publish_api_docs.yml, publish_aster_nixos.yml, publish_docker_images.yml, publish_osdk_and_ostd.yml, publish_sctrace.yml, publish_website.yml, push_cachix_dev.yml, push_cachix_prebuilt.yml, test_aarch64.yml, test_asterinas_vsock.yml, test_cargo_component.yml, test_devcontainer.yml, test_iso_image.yml, test_loongarch.yml, test_nix_flake.yml, test_nixos_full.yml, test_nixos_minimal.yml, test_riscv.yml, test_vmx.yml, test_x86.yml, test_x86_firecracker.yml, test_x86_tdx.yml, test_xfstests_full.yml, validate_scmls.yml

