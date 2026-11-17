# Linux: structure report

Generated on the build host by `tooling/kernel-watch/graph.ts` from blob-less shallow clones; each section names the commit it was built from. Structure and dependency level: tree, languages, subsystem inventory, dependency graph, enforced style. There is no symbol-level code graph.

## linux

Built from 666539eecdf4 (https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git) on `master`, committed 2026-11-16T09:35:38-08:00. 97,371 tracked files, 1599.3 MiB, 41,798,629 lines of code.

### Top-level tree

| Entry | Files | Size | Code lines | Main language |
| :-- | :-- | :-- | :-- | :-- |
| `drivers/` | 38,199 | 1111.5 MiB | 26,963,063 | C |
| `arch/` | 18,951 | 121.4 MiB | 4,830,688 | C |
| `tools/` | 10,035 | 80.7 MiB | 1,914,971 | C |
| `Documentation/` | 11,611 | 56.5 MiB | 7,672 | Documentation |
| `sound/` | 3,029 | 50.1 MiB | 1,728,497 | C |
| `fs/` | 2,365 | 46.2 MiB | 1,681,777 | C |
| `include/` | 6,778 | 46.2 MiB | 1,493,608 | C |
| `net/` | 1,897 | 33.1 MiB | 1,317,245 | C |
| `kernel/` | 754 | 15.6 MiB | 596,521 | C |
| `lib/` | 921 | 9.5 MiB | 339,159 | C |
| `mm/` | 208 | 6.2 MiB | 235,725 | C |
| `rust/` | 710 | 6.1 MiB | 174,170 | Rust |
| `scripts/` | 800 | 3.4 MiB | 95,912 | C |
| `security/` | 318 | 3.4 MiB | 132,885 | C |
| `crypto/` | 184 | 3.3 MiB | 106,915 | C |
| `block/` | 103 | 2.0 MiB | 74,656 | C |
| `(root files)` | 18 | 1.2 MiB | 2,454 | Make |
| `samples/` | 292 | 1.1 MiB | 43,042 | C |
| `io_uring/` | 91 | 0.7 MiB | 28,957 | C |
| `virt/` | 21 | 0.3 MiB | 11,693 | C |
| `ipc/` | 13 | 0.2 MiB | 9,892 | C |
| `LICENSES/` | 23 | 0.2 MiB | 0 | Other |
| `init/` | 17 | 0.2 MiB | 7,076 | C |
| `certs/` | 12 | 0.0 MiB | 1,354 | C |
| `usr/` | 21 | 0.0 MiB | 697 | Make |

### Languages

| Language | Files | Size | Code lines |
| :-- | :-- | :-- | :-- |
| C | 65,080 | 1415.4 MiB | 38,243,392 |
| Device tree | 6,646 | 49.3 MiB | 2,227,544 |
| Documentation | 4,130 | 31.2 MiB | 0 |
| JSON | 1,084 | 27.2 MiB | 0 |
| Other | 4,634 | 17.9 MiB | 0 |
| YAML | 5,972 | 17.0 MiB | 0 |
| Assembly | 1,362 | 9.2 MiB | 378,871 |
| Rust | 568 | 7.0 MiB | 210,514 |
| Kconfig | 1,964 | 6.9 MiB | 237,254 |
| Shell | 1,271 | 6.0 MiB | 233,288 |
| Python | 455 | 4.8 MiB | 135,266 |
| Text | 896 | 3.5 MiB | 0 |
| Make | 3,248 | 2.9 MiB | 89,336 |
| Perl | 46 | 1.0 MiB | 40,688 |

### MAINTAINERS subsystems

3,406 subsystems; status Maintained 2288, Supported 822, Orphan 147, Odd 105, unspecified 39, Obsolete 2, Orphaned 1, Externally 1, Buried 1. Maintainer names are not copied here.

| Subsystem | Status | Maintainers | Lists | File patterns |
| :-- | :-- | :-- | :-- | :-- |
| ARM/APPLE MACHINE SUPPORT | Maintained | 2 | 2 | 68 |
| NETWORKING [IPv4/IPv6] | Maintained | 2 | 1 | 50 |
| MEMORY MANAGEMENT - CORE | Maintained | 1 | 1 | 46 |
| NETWORKING [GENERAL] | Maintained | 4 | 1 | 45 |
| DIALOG SEMICONDUCTOR DRIVERS | Supported | 1 | 0 | 40 |
| WOLFSON MICROELECTRONICS DRIVERS | Supported | 0 | 1 | 36 |
| ARM/QUALCOMM MAILING LIST |  | 0 | 1 | 35 |
| MEMORY MANAGEMENT - MISC | Maintained | 2 | 1 | 35 |
| RISC-V MICROCHIP SUPPORT | Supported | 1 | 1 | 35 |
| Hyper-V/Azure CORE AND DRIVERS | Supported | 3 | 1 | 34 |
| DRIVER CORE, KOBJECTS, DEBUGFS AND SYSFS | Supported | 4 | 1 | 31 |
| BITMAP API | Maintained | 1 | 0 | 30 |
| ROHM POWER MANAGEMENT IC DEVICE DRIVERS | Supported | 1 | 0 | 30 |
| BPF [GENERAL] (Safe Dynamic Programs and Tools) | Supported | 5 | 1 | 29 |
| ETHERNET PHY LIBRARY | Maintained | 2 | 1 | 29 |
| OMAP2+ SUPPORT | Maintained | 5 | 1 | 29 |
| I2C/SMBUS CONTROLLER DRIVERS FOR PC | Maintained | 1 | 1 | 27 |
| TEXAS INSTRUMENTS AUDIO (ASoC/HDA) DRIVERS | Maintained | 4 | 1 | 27 |
| ARM/SAMSUNG S3C, S5P AND EXYNOS ARM ARCHITECTURES | Maintained | 2 | 2 | 26 |
| ARM/STI ARCHITECTURE | Maintained | 1 | 1 | 26 |
| MEMORY MANAGEMENT - PAGE ALLOCATOR | Maintained | 2 | 1 | 26 |
| ARM/ACTIONS SEMI ARCHITECTURE | Maintained | 2 | 2 | 25 |
| INGENIC JZ47xx SoCs | Maintained | 1 | 1 | 25 |
| MEMORY MAPPING | Maintained | 3 | 1 | 25 |
| ARM/NOMADIK/Ux500 ARCHITECTURES | Maintained | 1 | 1 | 23 |

### drivers/

144 directories under drivers/, largest by code lines (full list in the JSON).

| Directory | Files | Code lines | Kconfig symbols | Subdirs |
| :-- | :-- | :-- | :-- | :-- |
| `drivers/gpu` | 8,489 | 8,747,808 | 598 | 7 |
| `drivers/net` | 6,959 | 5,388,257 | 1454 | 31 |
| `drivers/media` | 3,142 | 1,594,809 | 870 | 17 |
| `drivers/scsi` | 869 | 1,054,568 | 195 | 37 |
| `drivers/clk` | 1,490 | 883,784 | 751 | 58 |
| `drivers/pinctrl` | 678 | 615,630 | 518 | 35 |
| `drivers/usb` | 867 | 586,872 | 483 | 26 |
| `drivers/infiniband` | 639 | 445,057 | 43 | 4 |
| `drivers/iio` | 903 | 441,476 | 681 | 31 |
| `drivers/accel` | 566 | 384,803 | 9 | 6 |
| `drivers/video` | 535 | 297,696 | 323 | 4 |
| `drivers/crypto` | 598 | 270,825 | 143 | 25 |
| `drivers/platform` | 492 | 244,429 | 283 | 12 |
| `drivers/staging` | 989 | 238,405 | 95 | 11 |
| `drivers/input` | 480 | 223,703 | 434 | 10 |
| `drivers/hwmon` | 356 | 223,166 | 359 | 3 |
| `drivers/md` | 297 | 216,143 | 61 | 4 |
| `drivers/mtd` | 325 | 193,326 | 229 | 10 |
| `drivers/tty` | 229 | 187,388 | 265 | 4 |
| `drivers/pci` | 269 | 179,308 | 198 | 7 |
| `drivers/acpi` | 363 | 178,982 | 98 | 10 |
| `drivers/dma` | 248 | 169,381 | 126 | 19 |
| `drivers/hid` | 321 | 168,617 | 179 | 7 |
| `drivers/phy` | 403 | 161,838 | 201 | 35 |
| `drivers/s390` | 234 | 138,388 | 36 | 7 |
| `drivers/spi` | 189 | 138,059 | 168 | 1 |
| `drivers/mmc` | 177 | 136,197 | 109 | 2 |
| `drivers/comedi` | 207 | 130,649 | 143 | 2 |
| `drivers/regulator` | 238 | 126,334 | 212 | 0 |
| `drivers/mfd` | 276 | 124,602 | 227 | 0 |
| `drivers/power` | 199 | 121,580 | 181 | 3 |
| `drivers/i2c` | 195 | 116,802 | 171 | 3 |
| `drivers/firmware` | 261 | 111,552 | 130 | 16 |
| `drivers/soc` | 311 | 110,753 | 218 | 35 |
| `drivers/misc` | 268 | 107,455 | 97 | 19 |
| `drivers/iommu` | 160 | 103,085 | 81 | 7 |
| `drivers/gpio` | 233 | 98,749 | 226 | 0 |
| `drivers/block` | 110 | 98,209 | 58 | 8 |
| `drivers/ata` | 142 | 96,432 | 134 | 1 |
| `drivers/char` | 215 | 92,534 | 148 | 6 |

### Kconfig dependency graph

22,159 config symbols in 1,964 Kconfig files, grouped by directory (two levels under arch, drivers, fs, net, sound, security, crypto, tools). Edges are select, imply and depends-on references that cross groups (4,379 distinct edges; the JSON keeps the heaviest 400).

| From | To | Kind | Edges |
| :-- | :-- | :-- | :-- |
| `drivers/clk` | `init` | depends | 379 |
| `sound/soc` | `drivers/i2c` | depends | 325 |
| `drivers/media` | `drivers/i2c` | depends | 316 |
| `drivers/crypto` | `crypto` | select | 294 |
| `drivers/net` | `drivers/pci` | depends | 267 |
| `drivers/pinctrl` | `init` | depends | 265 |
| `drivers/iio` | `drivers/i2c` | depends | 263 |
| `drivers/clk` | `arch/arm64` | depends | 244 |
| `drivers/iio` | `drivers/base` | select | 239 |
| `drivers/net` | `lib` | select | 219 |
| `sound/soc` | `drivers/base` | select | 218 |
| `drivers/mfd` | `drivers/base` | select | 203 |
| `drivers/iio` | `drivers/spi` | depends | 201 |
| `drivers/irqchip` | `kernel` | select | 154 |
| `sound/soc` | `init` | depends | 149 |
| `drivers/media` | `init` | depends | 148 |
| `arch/x86` | `arch` | select | 147 |
| `drivers/net` | `init` | depends | 141 |
| `drivers/phy` | `init` | depends | 137 |
| `drivers/clk` | `arch/arm` | depends | 136 |
| `drivers/platform` | `drivers/acpi` | depends | 132 |
| `drivers/gpu` | `drivers/video` | depends | 126 |
| `drivers/net` | `arch/arm` | depends | 126 |
| `drivers/hwmon` | `drivers/i2c` | depends | 125 |
| `drivers/phy` | `arch/arm` | depends | 125 |
| `drivers/pinctrl` | `arch/arm64` | depends | 124 |
| `drivers/watchdog` | `init` | depends | 124 |
| `arch/arm` | `drivers/clocksource` | select | 117 |
| `drivers/mfd` | `drivers/i2c` | depends | 116 |
| `arch/arm` | `drivers/irqchip` | select | 115 |
| `drivers/pinctrl` | `drivers/gpio` | select | 111 |
| `drivers/media` | `arch/arm` | depends | 109 |
| `drivers/net` | `lib` | depends | 109 |
| `drivers/pinctrl` | `arch/arm` | depends | 109 |
| `arch/arm` | `drivers/pinctrl` | select | 106 |
| `drivers/spi` | `init` | depends | 105 |
| `arch/powerpc` | `arch` | select | 104 |
| `drivers/gpu` | `init` | depends | 102 |
| `drivers/net` | `drivers/base` | select | 102 |
| `drivers/input` | `drivers/i2c` | depends | 101 |

### Makefile composition

332 group-to-group build descents (obj-y and obj-$(CONFIG_X) of a subdirectory).

| From | To | Descents |
| :-- | :-- | :-- |
| `drivers` | `drivers/usb` | 6 |
| `drivers` | `drivers/hwtracing` | 4 |
| `drivers` | `drivers/input` | 3 |
| `drivers` | `drivers/vhost` | 3 |
| `drivers` | `drivers/block` | 2 |
| `drivers` | `drivers/char` | 2 |
| `drivers` | `drivers/video` | 2 |
| `(root)` | `arch/$(SRCARCH)` | 1 |
| `(root)` | `block` | 1 |
| `(root)` | `certs` | 1 |
| `(root)` | `crypto` | 1 |
| `(root)` | `drivers` | 1 |
| `(root)` | `fs` | 1 |
| `(root)` | `include` | 1 |
| `(root)` | `init` | 1 |
| `(root)` | `io_uring` | 1 |
| `(root)` | `ipc` | 1 |
| `(root)` | `kernel` | 1 |
| `(root)` | `mm` | 1 |
| `(root)` | `net` | 1 |
| `(root)` | `rust` | 1 |
| `(root)` | `samples` | 1 |
| `(root)` | `security` | 1 |
| `(root)` | `sound` | 1 |
| `(root)` | `usr` | 1 |

### CMake modules (NT components)

0 modules declared with set_module_type.

| Module type | Modules |
| :-- | :-- |

| Top-level directory | Modules |
| :-- | :-- |

Most linked modules: .

### Enforced style

Configuration files present: `.clang-format`, `.editorconfig`, `.rustfmt.toml`, `.clippy.toml`, `.pylintrc`, `.gitattributes`, `scripts/checkpatch.pl`, `Documentation/process/coding-style.rst`, `Documentation/rust/coding-guidelines.rst`.

- .clang-format: ColumnLimit 80, IndentWidth 8, TabWidth 8, UseTab Always, BreakBeforeBraces Custom, AllowShortIfStatementsOnASingleLine false, AlignAfterOpenBracket Align, SortIncludes false
- .rustfmt.toml: `{"edition":"2021","newline_style":"Unix"}`
- .clippy.toml: `{"msrv":"1.85.0","check-private-items":true,"disallowed-macros":[{"path":"kernel::dbg","reason":"the `dbg!` macro is intended as a debugging tool","allow-invalid":true}],"disallowed-methods":[{"path":"core::ffi::CStr::as_ptr","replacement":"kernel::prelude::CStrExt::as_char_ptr","reason":"kernel's `char` is always unsigned, use `as_char_ptr` instead"},{"path":"core::ffi::CStr::from_ptr","replacement":"kernel::prelude::CStrExt::from_char_ptr","reason":"kernel's `char` is always unsigned, use `from_char_ptr` instead"}]}`
- scripts/checkpatch.pl: max line length 100, --strict available
- .editorconfig: `{"{*.{awk,c,dts,dtsi,dtso,h,mk,rst,s,S},Kconfig,Makefile,Makefile.*}":{"charset":"utf-8","end_of_line":"lf","insert_final_newline":"true","indent_style":"tab","indent_size":"8"},"*.{json,py,rs}":{"charset":"utf-8","end_of_line":"lf","insert_final_newline":"true","indent_style":"space","indent_size":"4"},"tools/{perf,power,rcu,testing/kunit}/**.py,":{"indent_style":"tab","indent_size":"8"},"*.yaml"`

