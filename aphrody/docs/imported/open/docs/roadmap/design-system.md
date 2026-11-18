# Design system spec

## Sources

| Layer | Official source |
|---|---|
| Rules | m3.material.io, design.google (reference archive) |
| Colour | material-color-utilities (dynamic scheme from the Aphrody source colour) |
| Tokens | Google's latest official set; Angular Material `--mat-sys-*` on the web |
| Web components | Angular Material (angular/components), on a fork |
| Flutter components | Flutter material (desktop, mobile, web) |
| Compose components | Jetpack Compose material3, in Compose Multiplatform |
| Icons | Material Symbols (google/material-design-icons) |
| Fonts | Google Sans Flex, Google Sans Code (googlefonts) |
| Mockups | Figma library "Material 3 Design Kit" |

## Coverage (decision D4: maximum)

| Target | Implementations |
|---|---|
| Web | Angular Material (fork), Flutter web, Compose Multiplatform (wasm) |
| Android | Flutter, Compose material3 |
| iOS | Flutter, Compose Multiplatform |
| Desktop (Windows, macOS, Linux) | Flutter desktop, Compose Multiplatform desktop, Rust framework (D7) |
| Terminal | open (D5) |

## Angular fork (decision D3)

- Forks of `angular/angular`, `angular/components` and `angular/angular-cli` in `aphrody-labs` (created), following upstream automatically: the Aphrody patch stack is replayed on every new upstream head, built, tested, then published; a conflict opens an issue instead of breaking the branch.
- Aggressive Bun patches: Bun replaces the Node tools (package manager, script runner, tests and bundling where possible).
- AOT and LTO compilation of the chain.

## Shared C ABI

A common core exposed through a C ABI, consumed by Angular (Bun FFI or WebAssembly), Flutter (`dart:ffi`), Compose (JNI or Kotlin/Native cinterop), Rust, Zig, .NET (`LibraryImport`) and C++. The AI router is the first module of this core.

## Aphrody identity

Source colour, scheme variant and font applied through each implementation's official theming (Angular `mat.theme`, Flutter `ColorScheme.fromSeed`, Compose `ColorScheme`). No other value: everything else comes from the official tokens.
