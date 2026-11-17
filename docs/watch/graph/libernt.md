# LiberNT: structure report

Generated on the build host by `tooling/kernel-watch/graph.ts` from blob-less shallow clones; each section names the commit it was built from. Structure and dependency level: tree, languages, subsystem inventory, dependency graph, enforced style. There is no symbol-level code graph.

## libernt

Built from [9680fba1001d](https://github.com/LiberNT/LiberNT/commit/9680fba1001d71e7da6e23c766baef2b9e7a0f69) on `main-nt10`, committed 2026-11-17T13:55:34+09:00. 38,462 tracked files, 715.2 MiB, 13,781,536 lines of code.

### Top-level tree

| Entry | Files | Size | Code lines | Main language |
| :-- | :-- | :-- | :-- | :-- |
| `dll/` | 9,909 | 186.5 MiB | 3,448,565 | C |
| `sdk/` | 8,112 | 127.4 MiB | 3,783,164 | C |
| `modules/` | 6,139 | 120.0 MiB | 2,589,104 | C |
| `media/` | 1,912 | 98.5 MiB | 5,164 | Text |
| `drivers/` | 4,520 | 76.9 MiB | 1,998,787 | C |
| `base/` | 4,850 | 57.0 MiB | 699,031 | C |
| `subsystems/` | 1,166 | 20.7 MiB | 615,674 | C |
| `ntos/` | 801 | 11.8 MiB | 387,076 | C |
| `boot/` | 441 | 9.5 MiB | 103,623 | C |
| `hal/` | 176 | 3.3 MiB | 61,880 | C |
| `submodules/` | 222 | 1.6 MiB | 40,677 | C |
| `bootloader/` | 128 | 1.3 MiB | 45,980 | C |
| `(root files)` | 36 | 0.4 MiB | 1,731 | Documentation |
| `translations/` | 36 | 0.3 MiB | 0 | Documentation |
| `scripts/` | 7 | 0.0 MiB | 1,080 | Python |
| `.github/` | 4 | 0.0 MiB | 0 | YAML |
| `.theia/` | 3 | 0.0 MiB | 0 | JSON |

### Languages

| Language | Files | Size | Code lines |
| :-- | :-- | :-- | :-- |
| C | 19,945 | 390.5 MiB | 12,079,913 |
| Other | 6,349 | 217.3 MiB | 0 |
| Resource | 5,375 | 41.5 MiB | 0 |
| C++ | 2,846 | 35.1 MiB | 1,214,353 |
| Text | 358 | 13.9 MiB | 0 |
| IDL | 671 | 7.7 MiB | 240,881 |
| Assembly | 503 | 4.3 MiB | 127,460 |
| CMake | 2,075 | 2.0 MiB | 63,137 |
| JavaScript | 27 | 0.9 MiB | 21,986 |
| Python | 112 | 0.9 MiB | 22,414 |
| Documentation | 103 | 0.5 MiB | 0 |
| Shell | 54 | 0.2 MiB | 7,128 |
| C# | 13 | 0.1 MiB | 2,166 |
| Perl | 1 | 0.0 MiB | 1,310 |

### CMake modules (NT components)

1,583 modules declared with set_module_type.

| Module type | Modules |
| :-- | :-- |
| win32cui | 560 |
| win32dll | 548 |
| kernelmodedriver | 269 |
| win32gui | 114 |
| kmdfdriver | 28 |
| kerneldll | 25 |
| cpl | 21 |
| module | 16 |
| nativedll | 6 |
| nativecui | 4 |
| win32ocx | 3 |
| wdmdriver | 2 |
| kernel | 1 |

| Top-level directory | Modules |
| :-- | :-- |
| `modules/` | 549 |
| `dll/` | 519 |
| `drivers/` | 256 |
| `base/` | 217 |
| `subsystems/` | 46 |
| `sdk/` | 4 |
| `boot/` | 2 |
| `hal/` | 2 |
| `media/` | 1 |
| `ntos/` | 1 |

Most linked modules: kernel32 (1218), msvcrt (1165), ntdll (859), user32 (541), advapi32 (499), ole32 (310), ntoskrnl (299), hal (283), gdi32 (234), oleaut32 (168), shell32 (149), comctl32 (122), shlwapi (120), setupapi (89), combase (82).

### Enforced style

Configuration files present: `.clang-format`, `.editorconfig`, `.gitattributes`, `CONTRIBUTING.md`, `CODEOWNERS`, `CODE_OF_CONDUCT.md`.

- .clang-format: BasedOnStyle Microsoft, IndentWidth 4, UseTab Never, BreakBeforeBraces Allman, AlignAfterOpenBracket AlwaysBreak
- .editorconfig: `{"*":{"root":"true","insert_final_newline":"true","charset":"utf-8","indent_size":"4","indent_style":"space","trim_trailing_whitespace":"true"},"*.{json,yml,md}":{"indent_size":"2"}}`
- CI workflows: build.yml, pr-cache-cleanup.yml

