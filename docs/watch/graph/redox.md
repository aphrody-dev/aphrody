# Redox: structure report

Generated on the build host by `tooling/kernel-watch/graph.ts` from blob-less shallow clones; each section names the commit it was built from. Structure and dependency level: tree, languages, subsystem inventory, dependency graph, enforced style. There is no symbol-level code graph.

## redox-kernel

Built from [8f0e965e9985](https://gitlab.redox-os.org/redox-os/kernel/-/commit/8f0e965e99856fc6215294663fa1a9d1018a7eb2) on `master`, committed 2026-11-14T22:36:03+09:00. 267 tracked files, 1.6 MiB, 51,076 lines of code.

### Top-level tree

| Entry | Files | Size | Code lines | Main language |
| :-- | :-- | :-- | :-- | :-- |
| `src/` | 197 | 1.3 MiB | 43,661 | Rust |
| `syscall/` | 21 | 0.1 MiB | 4,114 | Rust |
| `rmm/` | 24 | 0.1 MiB | 3,057 | Rust |
| `(root files)` | 14 | 0.1 MiB | 244 | Documentation |
| `linkers/` | 4 | 0.0 MiB | 0 | Other |
| `res/` | 1 | 0.0 MiB | 0 | Other |
| `targets/` | 4 | 0.0 MiB | 0 | JSON |
| `.helix/` | 2 | 0.0 MiB | 0 | TOML |

### Languages

| Language | Files | Size | Code lines |
| :-- | :-- | :-- | :-- |
| Rust | 231 | 1.5 MiB | 50,514 |
| Other | 14 | 0.0 MiB | 0 |
| Documentation | 4 | 0.0 MiB | 0 |
| Assembly | 2 | 0.0 MiB | 328 |
| TOML | 7 | 0.0 MiB | 0 |
| Make | 1 | 0.0 MiB | 124 |
| JSON | 4 | 0.0 MiB | 0 |
| YAML | 1 | 0.0 MiB | 0 |
| Device tree | 2 | 0.0 MiB | 103 |
| Shell | 1 | 0.0 MiB | 7 |

### Cargo workspace

3 workspace crates, 72 packages in the resolved graph, 2 workspace-internal dependency edges.

| Crate | Path | Version | Edition | Workspace deps | External deps |
| :-- | :-- | :-- | :-- | :-- | :-- |
| `kernel` | `.` | 0.5.12 | 2024 | redox_syscall, rmm | 16 |
| `rmm` | `rmm` | 0.1.0 | 2024 | - | 1 |
| `redox_syscall` | `syscall` | 0.10.0 | 2021 | - | 2 |

Most used external crates: bitflags (3), arrayvec (1), bitfield (1), cc (1), fdt (1), hashbrown (1), linked_list_allocator (1), object (1), raw-cpuid (1), rustc-demangle (1), rustc-std-workspace-core (1), sbi-rt (1), slab (1), smallvec (1), spin (1).

### Enforced style

Configuration files present: `rustfmt.toml`, `rust-toolchain.toml`.

- rustfmt.toml: `{"blank_lines_lower_bound":0,"blank_lines_upper_bound":1,"brace_style":"SameLineWhere","disable_all_formatting":false,"edition":"2024","style_edition":"2015","empty_item_single_line":true,"fn_single_line":false,"force_explicit_abi":true,"format_strings":false,"hard_tabs":false,"show_parse_errors":true,"imports_granularity":"Crate","imports_indent":"Block","imports_layout":"Mixed","indent_style":"Block","max_width":100,"newline_style":"Unix","skip_children":false,"tab_spaces":4,"trailing_comma":"Vertical","where_single_line":false}`
- rust-toolchain.toml: `{"toolchain":{"channel":"nightly-2026-05-24","components":["rust-src"]}}`
- lints: `{"clippy":{"arithmetic_side_effects":"warn","cast_ptr_alignment":"warn","identity_op":"allow","if_same_then_else":"allow","indexing_slicing":"warn","many_single_char_names":"allow","module_inception":"allow","new_without_default":"allow","not_unsafe_ptr_arg_deref":"deny","or_fun_call":"allow","precedence":"deny","ptr_cast_constness":"deny","too_many_arguments":"allow","significant_drop_in_scrutinee":"deny","unwrap_used":"warn"},"rust":{"static_mut_refs":"warn","unreachable_patterns":"deny","unused_must_use":"deny"}}`

## redox-drivers

Built from [20ffe4d7f4a8](https://gitlab.redox-os.org/redox-os/drivers/-/commit/20ffe4d7f4a85b7cc1f59495d7e6e355fed4cb06) on `master`, committed 2025-11-29T09:22:25-07:00. 244 tracked files, 1.9 MiB, 44,050 lines of code.

### Top-level tree

| Entry | Files | Size | Code lines | Main language |
| :-- | :-- | :-- | :-- | :-- |
| `storage/` | 45 | 0.7 MiB | 7,145 | Rust |
| `usb/` | 35 | 0.3 MiB | 9,257 | Rust |
| `net/` | 29 | 0.3 MiB | 7,196 | Rust |
| `graphics/` | 28 | 0.1 MiB | 3,814 | Rust |
| `pcid/` | 15 | 0.1 MiB | 2,756 | Rust |
| `audio/` | 16 | 0.1 MiB | 3,249 | Rust |
| `input/` | 13 | 0.1 MiB | 2,840 | Rust |
| `acpid/` | 7 | 0.1 MiB | 2,510 | Rust |
| `(root files)` | 10 | 0.1 MiB | 10 | Documentation |
| `virtio-core/` | 12 | 0.1 MiB | 1,545 | Rust |
| `common/` | 9 | 0.0 MiB | 1,158 | Rust |
| `inputd/` | 3 | 0.0 MiB | 801 | Rust |
| `amlserde/` | 2 | 0.0 MiB | 475 | Rust |
| `executor/` | 2 | 0.0 MiB | 396 | Rust |
| `vboxd/` | 4 | 0.0 MiB | 381 | Rust |
| `hwd/` | 7 | 0.0 MiB | 242 | Rust |
| `rtcd/` | 3 | 0.0 MiB | 173 | Rust |
| `.gitlab/` | 2 | 0.0 MiB | 0 | Documentation |
| `pcid-spawner/` | 2 | 0.0 MiB | 102 | Rust |

### Languages

| Language | Files | Size | Code lines |
| :-- | :-- | :-- | :-- |
| Rust | 163 | 1.3 MiB | 44,040 |
| Other | 17 | 0.6 MiB | 0 |
| TOML | 57 | 0.0 MiB | 0 |
| Documentation | 5 | 0.0 MiB | 0 |
| YAML | 1 | 0.0 MiB | 0 |
| Shell | 1 | 0.0 MiB | 10 |

### Cargo workspace

43 workspace crates, 196 packages in the resolved graph, 92 workspace-internal dependency edges.

| Crate | Path | Version | Edition | Workspace deps | External deps |
| :-- | :-- | :-- | :-- | :-- | :-- |
| `acpid` | `acpid` | 0.1.0 | 2018 | amlserde, common | 16 |
| `amlserde` | `amlserde` | 0.0.1 | 2021 | - | 3 |
| `ac97d` | `audio/ac97d` | 0.1.0 | 2021 | common, pcid | 8 |
| `ihdad` | `audio/ihdad` | 0.1.0 | 2021 | common, pcid | 8 |
| `sb16d` | `audio/sb16d` | 0.1.0 | 2021 | common | 8 |
| `common` | `common` | 0.1.0 | 2021 | - | 4 |
| `executor` | `executor` | 0.1.0 | 2021 | - | 3 |
| `bgad` | `graphics/bgad` | 0.1.0 | 2021 | common, inputd, pcid | 6 |
| `console-draw` | `graphics/console-draw` | 0.1.0 | 2021 | graphics-ipc | 2 |
| `driver-graphics` | `graphics/driver-graphics` | 0.1.0 | 2021 | common, graphics-ipc, inputd | 4 |
| `fbbootlogd` | `graphics/fbbootlogd` | 0.1.0 | 2021 | console-draw, graphics-ipc, inputd | 7 |
| `fbcond` | `graphics/fbcond` | 0.1.0 | 2021 | common, console-draw, graphics-ipc, inputd | 8 |
| `graphics-ipc` | `graphics/graphics-ipc` | 0.1.0 | 2021 | common | 2 |
| `vesad` | `graphics/vesad` | 0.1.0 | 2021 | common, driver-graphics, graphics-ipc, inputd | 6 |
| `virtio-gpud` | `graphics/virtio-gpud` | 0.1.0 | 2021 | common, driver-graphics, graphics-ipc, inputd, pcid, virtio-core | 10 |
| `hwd` | `hwd` | 0.1.0 | 2018 | amlserde, common | 4 |
| `ps2d` | `input/ps2d` | 0.1.0 | 2018 | common, inputd | 8 |
| `usbhidd` | `input/usbhidd` | 0.1.0 | 2018 | common, inputd, xhcid | 5 |
| `inputd` | `inputd` | 0.1.0 | 2021 | common | 7 |
| `alxd` | `net/alxd` | 0.1.0 | 2018 | common | 6 |
| `driver-network` | `net/driver-network` | 0.1.0 | 2021 | - | 4 |
| `e1000d` | `net/e1000d` | 0.1.0 | 2018 | common, driver-network, pcid | 6 |
| `ixgbed` | `net/ixgbed` | 1.0.0 | 2021 | common, driver-network, pcid | 5 |
| `rtl8139d` | `net/rtl8139d` | 0.1.0 | 2018 | common, driver-network, pcid | 6 |
| `rtl8168d` | `net/rtl8168d` | 0.1.0 | 2018 | common, driver-network, pcid | 6 |
| `virtio-netd` | `net/virtio-netd` | 0.1.0 | 2021 | common, driver-network, pcid, virtio-core | 6 |
| `pcid` | `pcid` | 0.1.0 | 2021 | common | 12 |
| `pcid-spawner` | `pcid-spawner` | 0.1.0 | 2021 | common, pcid | 6 |
| `rtcd` | `rtcd` | 0.1.0 | 2018 | common | 1 |
| `ahcid` | `storage/ahcid` | 0.1.0 | 2021 | common, driver-block, pcid | 7 |
| `bcm2835-sdhcid` | `storage/bcm2835-sdhcid` | 0.1.0 | 2021 | common, driver-block | 5 |
| `driver-block` | `storage/driver-block` | 0.1.0 | 2021 | executor, partitionlib | 6 |
| `ided` | `storage/ided` | 0.1.0 | 2021 | common, driver-block, pcid | 5 |
| `lived` | `storage/lived` | 0.1.0 | 2021 | driver-block | 5 |
| `nvmed` | `storage/nvmed` | 0.1.0 | 2021 | common, driver-block, executor, partitionlib, pcid | 10 |
| `partitionlib` | `storage/partitionlib` | 0.1.0 | 2021 | - | 3 |
| `usbscsid` | `storage/usbscsid` | 0.1.0 | 2021 | driver-block, xhcid | 7 |
| `virtio-blkd` | `storage/virtio-blkd` | 0.1.0 | 2021 | common, driver-block, pcid, virtio-core | 10 |
| `usbctl` | `usb/usbctl` | 0.1.0 | 2018 | xhcid | 1 |
| `usbhubd` | `usb/usbhubd` | 0.1.0 | 2018 | common, xhcid | 2 |
| `xhcid` | `usb/xhcid` | 0.1.0 | 2018 | common, pcid | 18 |
| `vboxd` | `vboxd` | 0.1.0 | 2018 | common, pcid | 5 |
| `virtio-core` | `virtio-core` | 0.1.0 | 2021 | common, pcid | 9 |

Most used external crates: redox_syscall (35), libredox (33), redox-daemon (30), log (29), redox_event (25), redox-scheme (15), bitflags (14), orbclient (10), futures (7), anyhow (6), serde (5), spin (5), thiserror (5), plain (4), ransid (4).

### Enforced style

Configuration files present: `rust-toolchain.toml`.

- rust-toolchain.toml: `{"toolchain":{"channel":"nightly-2025-10-03","components":["rust-src"]}}`

## redox-base

Built from [145309f44eac](https://gitlab.redox-os.org/redox-os/base/-/commit/145309f44eacb9b3db512b980c5deeb1b5045996) on `main`, committed 2026-11-13T10:13:39-07:00. 487 tracked files, 3.4 MiB, 88,225 lines of code.

### Top-level tree

| Entry | Files | Size | Code lines | Main language |
| :-- | :-- | :-- | :-- | :-- |
| `drivers/` | 283 | 2.3 MiB | 56,783 | Rust |
| `selftests/` | 34 | 0.3 MiB | 7,989 | Rust |
| `bootstrap/` | 19 | 0.2 MiB | 4,927 | Rust |
| `netstack/` | 19 | 0.2 MiB | 4,983 | Rust |
| `(root files)` | 11 | 0.1 MiB | 310 | Make |
| `udsd/` | 7 | 0.1 MiB | 2,621 | Rust |
| `ramfs/` | 4 | 0.1 MiB | 2,015 | Rust |
| `ptyd/` | 17 | 0.1 MiB | 1,908 | Rust |
| `ipcd/` | 14 | 0.0 MiB | 1,433 | Rust |
| `initfs/` | 11 | 0.0 MiB | 1,270 | Rust |
| `scheme-utils/` | 5 | 0.0 MiB | 1,017 | Rust |
| `init/` | 6 | 0.0 MiB | 874 | Rust |
| `dhcpd/` | 3 | 0.0 MiB | 514 | Rust |
| `audiod/` | 4 | 0.0 MiB | 337 | Rust |
| `getty/` | 2 | 0.0 MiB | 350 | Rust |
| `randd/` | 2 | 0.0 MiB | 293 | Rust |
| `logd/` | 3 | 0.0 MiB | 297 | Rust |
| `daemon/` | 2 | 0.0 MiB | 140 | Rust |
| `.gitlab/` | 2 | 0.0 MiB | 0 | Documentation |
| `init.initfs.d/` | 21 | 0.0 MiB | 0 | Other |
| `zerod/` | 3 | 0.0 MiB | 124 | Rust |
| `init.d/` | 13 | 0.0 MiB | 0 | Other |
| `config/` | 2 | 0.0 MiB | 40 | Rust |

### Languages

| Language | Files | Size | Code lines |
| :-- | :-- | :-- | :-- |
| Rust | 325 | 2.7 MiB | 87,663 |
| Other | 60 | 0.6 MiB | 0 |
| TOML | 83 | 0.0 MiB | 0 |
| Documentation | 9 | 0.0 MiB | 0 |
| Shell | 5 | 0.0 MiB | 379 |
| Make | 1 | 0.0 MiB | 183 |
| YAML | 2 | 0.0 MiB | 0 |
| Text | 2 | 0.0 MiB | 0 |

### Cargo workspace

65 workspace crates, 310 packages in the resolved graph, 164 workspace-internal dependency edges.

| Crate | Path | Version | Edition | Workspace deps | External deps |
| :-- | :-- | :-- | :-- | :-- | :-- |
| `audiod` | `audiod` | 0.1.0 | 2021 | daemon, scheme-utils | 7 |
| `config` | `config` | 0.0.0 | 2024 | - | 0 |
| `daemon` | `daemon` | 0.0.0 | 2024 | - | 4 |
| `dhcpd` | `dhcpd` | 0.0.0 | 2024 | - | 0 |
| `acpid` | `drivers/acpid` | 0.1.0 | 2024 | amlserde, common, daemon, scheme-utils | 15 |
| `amlserde` | `drivers/amlserde` | 0.0.1 | 2021 | - | 3 |
| `ac97d` | `drivers/audio/ac97d` | 0.1.0 | 2021 | audiod, common, daemon, pcid, scheme-utils | 6 |
| `ihdad` | `drivers/audio/ihdad` | 0.1.0 | 2021 | audiod, common, daemon, pcid, scheme-utils | 8 |
| `sb16d` | `drivers/audio/sb16d` | 0.1.0 | 2021 | common, daemon, scheme-utils | 7 |
| `common` | `drivers/common` | 0.1.0 | 2021 | - | 4 |
| `executor` | `drivers/executor` | 0.1.0 | 2024 | - | 4 |
| `console-draw` | `drivers/graphics/console-draw` | 0.1.0 | 2021 | graphics-ipc | 5 |
| `drawbench` | `drivers/graphics/drawbench` | 0.1.0 | 2021 | graphics-ipc, inputd | 1 |
| `driver-graphics` | `drivers/graphics/driver-graphics` | 0.1.0 | 2024 | common, graphics-ipc, inputd, scheme-utils | 8 |
| `fbbootlogd` | `drivers/graphics/fbbootlogd` | 0.1.0 | 2021 | console-draw, daemon, graphics-ipc, inputd | 5 |
| `fbcond` | `drivers/graphics/fbcond` | 0.1.0 | 2021 | common, console-draw, daemon, graphics-ipc, inputd, scheme-utils | 9 |
| `graphics-ipc` | `drivers/graphics/graphics-ipc` | 0.1.0 | 2021 | - | 2 |
| `ihdgd` | `drivers/graphics/ihdgd` | 0.1.0 | 2021 | common, daemon, driver-graphics, ihdgd_macros, pcid | 13 |
| `ihdgd_macros` | `drivers/graphics/ihdgd/ihdgd_macros` | 0.1.0 | 2024 | - | 3 |
| `vesad` | `drivers/graphics/vesad` | 0.1.0 | 2021 | common, daemon, driver-graphics | 3 |
| `virtio-gpud` | `drivers/graphics/virtio-gpud` | 0.1.0 | 2021 | common, daemon, driver-graphics, pcid, virtio-core | 7 |
| `hwd` | `drivers/hwd` | 0.1.0 | 2018 | amlserde, common, daemon | 5 |
| `ps2d` | `drivers/input/ps2d` | 0.1.0 | 2018 | common, daemon, inputd | 7 |
| `usbhidd` | `drivers/input/usbhidd` | 0.1.0 | 2018 | common, inputd, xhcid | 8 |
| `inputd` | `drivers/inputd` | 0.1.0 | 2021 | common, daemon, scheme-utils | 6 |
| `driver-network` | `drivers/net/driver-network` | 0.1.0 | 2021 | daemon, scheme-utils | 3 |
| `e1000d` | `drivers/net/e1000d` | 0.1.0 | 2018 | common, daemon, driver-network, pcid | 5 |
| `ixgbed` | `drivers/net/ixgbed` | 1.0.0 | 2021 | common, daemon, driver-network, pcid | 4 |
| `rtl8139d` | `drivers/net/rtl8139d` | 0.1.0 | 2018 | common, daemon, driver-network, pcid | 5 |
| `rtl8168d` | `drivers/net/rtl8168d` | 0.1.0 | 2018 | common, daemon, driver-network, pcid | 5 |
| `virtio-netd` | `drivers/net/virtio-netd` | 0.1.0 | 2021 | common, daemon, driver-network, pcid, virtio-core | 5 |
| `pcid` | `drivers/pcid` | 0.1.0 | 2021 | common, daemon, scheme-utils | 11 |
| `pcid-spawner` | `drivers/pcid-spawner` | 0.1.0 | 2021 | common, config, daemon, pcid | 7 |
| `redoxerd` | `drivers/redoxerd` | 0.1.0 | 2018 | common | 8 |
| `rtcd` | `drivers/rtcd` | 0.1.0 | 2018 | common | 1 |
| `ahcid` | `drivers/storage/ahcid` | 0.1.0 | 2021 | common, daemon, driver-block, pcid | 5 |
| `bcm2835-sdhcid` | `drivers/storage/bcm2835-sdhcid` | 0.1.0 | 2021 | common, daemon, driver-block | 4 |
| `driver-block` | `drivers/storage/driver-block` | 0.1.0 | 2021 | common, daemon, executor, partitionlib, scheme-utils | 8 |
| `ided` | `drivers/storage/ided` | 0.1.0 | 2021 | common, daemon, driver-block, pcid | 4 |
| `lived` | `drivers/storage/lived` | 0.1.0 | 2021 | daemon, driver-block | 4 |
| `nbdd` | `drivers/storage/nbdd` | 0.1.0 | 2021 | daemon, driver-block | 4 |
| `nvmed` | `drivers/storage/nvmed` | 0.1.0 | 2021 | common, daemon, driver-block, executor, partitionlib, pcid | 9 |
| `partitionlib` | `drivers/storage/partitionlib` | 0.1.0 | 2021 | - | 3 |
| `usbscsid` | `drivers/storage/usbscsid` | 0.1.0 | 2021 | daemon, driver-block, xhcid | 6 |
| `virtio-blkd` | `drivers/storage/virtio-blkd` | 0.1.0 | 2021 | common, daemon, driver-block, pcid, virtio-core | 9 |
| `virtio-fsd` | `drivers/storage/virtio-fsd` | 0.1.0 | 2024 | common, daemon, pcid, scheme-utils, virtio-core | 13 |
| `usbctl` | `drivers/usb/usbctl` | 0.1.0 | 2018 | common, xhcid | 1 |
| `usbhubd` | `drivers/usb/usbhubd` | 0.1.0 | 2018 | common, xhcid | 2 |
| `xhcid` | `drivers/usb/xhcid` | 0.1.0 | 2018 | common, config, daemon, pcid, scheme-utils | 16 |
| `vboxd` | `drivers/vboxd` | 0.1.0 | 2018 | common, daemon, pcid | 4 |
| `virtio-core` | `drivers/virtio-core` | 0.1.0 | 2021 | common, pcid | 9 |
| `getty` | `getty` | 0.1.0 | 2024 | - | 4 |
| `init` | `init` | 0.1.0 | 2024 | config | 7 |
| `redox-initfs` | `initfs` | 0.2.0 | 2024 | - | 1 |
| `redox-initfs-tools` | `initfs/tools` | 0.2.0 | 2021 | redox-initfs | 7 |
| `ipcd` | `ipcd` | 0.1.0 | 2018 | daemon, scheme-utils | 5 |
| `logd` | `logd` | 0.1.0 | 2021 | daemon, scheme-utils | 3 |
| `netstack` | `netstack` | 0.1.0 | 2021 | daemon, scheme-utils | 8 |
| `ptyd` | `ptyd` | 0.1.0 | 2021 | daemon, scheme-utils | 6 |
| `ramfs` | `ramfs` | 0.1.0 | 2018 | daemon, scheme-utils | 10 |
| `randd` | `randd` | 0.1.0 | 2021 | daemon, scheme-utils | 7 |
| `scheme-utils` | `scheme-utils` | 0.0.0 | 2024 | - | 3 |
| `virtio-fsd-selftest` | `selftests/virtio-fsd` | 0.1.0 | 2024 | - | 4 |
| `udsd` | `udsd` | 0.1.0 | 2018 | daemon, scheme-utils | 6 |
| `zerod` | `zerod` | 0.1.0 | 2021 | daemon, scheme-utils | 3 |

Most used external crates: redox_syscall (51), libredox (49), redox_event (34), log (32), redox-scheme (25), anyhow (14), bitflags (12), libc (11), plain (9), futures (8), orbclient (7), serde (7), thiserror (6), toml (5), clap (4).

### Enforced style

Configuration files present: `rustfmt.toml`, `rust-toolchain.toml`.

- rustfmt.toml: `{"group_imports":"StdExternalCrate","imports_granularity":"Module"}`
- rust-toolchain.toml: `{"toolchain":{"channel":"nightly-2026-05-24","components":["rust-src"]}}`
- workspace lints: `{"rust":{"missing_docs":"allow"},"clippy":{"cast_lossless":"warn","missing_safety_doc":"warn","perf":{"level":"deny","priority":-1},"precedence":"deny","unused_io_amount":"warn"}}`

