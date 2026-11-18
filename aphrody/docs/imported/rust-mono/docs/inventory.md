# Inventory of the forks (2026-11-17)

Snapshot taken the day the forks were created, with `bun tools/forks.ts` against the live GitHub API. Ahead is the number of commits the fork has that upstream lacks; behind is the reverse. `forks.toml` is the source of truth; this page is history. Current numbers: run `bun tools/forks.ts`.

Forks of the organization before this work: 10 of the 51 below (marked **existing**). The rest were created with `gh repo fork <upstream> --org <org> --clone=false`, all branches included (the rust LLVM fork needs its `rustc/*` branches).

| Path | Fork | Upstream | Tracked branch | Role | Origin | Ahead | Behind | In rust-lang/rust |
| :-- | :-- | :-- | :-- | :-- | :-- | --: | --: | :-- |
| compiler/rust | rust | rust-lang/rust | main | compiler | existing | 0 | 531 | no |
| compiler/rust-lang-llvm-project | rust-lang-llvm-project | rust-lang/llvm-project | rustc/23.1-2026-07-22 | backend | created | 0 | 0 | submodule `src/llvm-project` |
| compiler/llvm-project | llvm-project | llvm/llvm-project | main | backend | existing | 0 | 347 | no |
| compiler/gcc | gcc | rust-lang/gcc | master | backend | created | 0 | 0 | submodule `src/gcc` |
| compiler/enzyme | enzyme | rust-lang/enzyme | main | backend | created | 0 | 0 | submodule `src/tools/enzyme` |
| compiler/tpde | tpde | rust-lang/tpde | main | backend | created | 0 | 0 | submodule `src/tpde` |
| compiler/miri | miri | rust-lang/miri | master | compiler | created | 0 | 0 | subtree `src/tools/miri` |
| compiler/rustc_codegen_cranelift | rustc_codegen_cranelift | rust-lang/rustc_codegen_cranelift | main | codegen | created | 0 | 0 | subtree `compiler/rustc_codegen_cranelift` |
| compiler/rustc_codegen_gcc | rustc_codegen_gcc | rust-lang/rustc_codegen_gcc | master | codegen | created | 0 | 0 | subtree `compiler/rustc_codegen_gcc` |
| devtools/cargo | cargo | rust-lang/cargo | master | package-manager | existing | 0 | 86 | submodule `src/tools/cargo` |
| devtools/rustup | rustup | rust-lang/rustup | main | installer | created | 0 | 0 | no |
| devtools/rust-analyzer | rust-analyzer | rust-lang/rust-analyzer | master | ide | created | 0 | 0 | subtree `src/tools/rust-analyzer` |
| devtools/rust-clippy | rust-clippy | rust-lang/rust-clippy | main | devtool | created | 0 | 0 | subtree `src/tools/clippy` |
| devtools/rustfmt | rustfmt | rust-lang/rustfmt | main | devtool | created | 0 | 0 | subtree `src/tools/rustfmt` |
| devtools/mdBook | mdBook | rust-lang/mdBook | main | devtool | created | 0 | 0 | no |
| std/stdarch | stdarch | rust-lang/stdarch | main | std | created | 0 | 0 | subtree `library/stdarch` |
| std/compiler-builtins | compiler-builtins | rust-lang/compiler-builtins | main | std | created | 0 | 0 | subtree `library/compiler-builtins` |
| std/backtrace-rs | backtrace-rs | rust-lang/backtrace-rs | main | std | created | 0 | 0 | submodule `library/backtrace` |
| std/libc | libc | rust-lang/libc | main | std | created | 0 | 0 | no |
| std/hashbrown | hashbrown | rust-lang/hashbrown | main | std | created | 0 | 0 | no |
| std/cc-rs | cc-rs | rust-lang/cc-rs | main | std | created | 0 | 0 | no |
| registry/crates.io | crates.io | rust-lang/crates.io | main | registry | existing | 0 | 42 | no |
| registry/docs.rs | docs.rs | rust-lang/docs.rs | main | registry | created | 0 | 0 | no |
| books/reference | reference | rust-lang/reference | master | docs | created | 0 | 0 | submodule `src/doc/reference` |
| books/book | book | rust-lang/book | main | docs | created | 0 | 0 | submodule `src/doc/book` |
| books/rust-by-example | rust-by-example | rust-lang/rust-by-example | master | docs | created | 0 | 0 | submodule `src/doc/rust-by-example` |
| books/nomicon | nomicon | rust-lang/nomicon | master | docs | created | 0 | 0 | submodule `src/doc/nomicon` |
| books/edition-guide | edition-guide | rust-lang/edition-guide | master | docs | created | 0 | 0 | submodule `src/doc/edition-guide` |
| books/rustc-dev-guide | rustc-dev-guide | rust-lang/rustc-dev-guide | main | docs | created | 0 | 0 | subtree `src/doc/rustc-dev-guide` |
| books/rust-embedded-book | rust-embedded-book | rust-embedded/book | master | docs | created | 0 | 0 | submodule `src/doc/embedded-book` |
| governance/rfcs | rfcs | rust-lang/rfcs | main | governance | created | 0 | 0 | no |
| governance/team | team | rust-lang/team | main | governance | created | 0 | 0 | no |
| governance/goals | goals | rust-lang/goals | main | governance | created | 0 | 0 | no |
| governance/compiler-team | compiler-team | rust-lang/compiler-team | main | governance | created | 0 | 0 | no |
| governance/rust-forge | rust-forge | rust-lang/rust-forge | main | governance | created | 0 | 0 | no |
| governance/blog.rust-lang.org | blog.rust-lang.org | rust-lang/blog.rust-lang.org | main | governance | created | 0 | 0 | no |
| governance/this-week-in-rust | this-week-in-rust | rust-lang/this-week-in-rust | main | governance | created | 0 | 0 | no |
| governance/www.rust-lang.org | www.rust-lang.org | rust-lang/www.rust-lang.org | main | governance | created | 0 | 0 | no |
| infra/rustc-perf | rustc-perf | rust-lang/rustc-perf | main | infra | created | 0 | 0 | submodule `src/tools/rustc-perf` |
| infra/crater | crater | rust-lang/crater | main | infra | created | 0 | 0 | no |
| infra/bors | bors | rust-lang/bors | main | infra | created | 0 | 0 | no |
| infra/triagebot | triagebot | rust-lang/triagebot | main | infra | created | 0 | 0 | no |
| infra/simpleinfra | simpleinfra | rust-lang/simpleinfra | master | infra | created | 0 | 0 | no |
| wasm/wasmtime | wasmtime | bytecodealliance/wasmtime | main | wasm | existing | 0 | 38 | no |
| wasm/wasm-tools | wasm-tools | bytecodealliance/wasm-tools | main | wasm | created | 0 | 0 | no |
| wasm/wit-bindgen | wit-bindgen | bytecodealliance/wit-bindgen | main | wasm | created | 0 | 0 | no |
| wasm/binaryen | binaryen | WebAssembly/binaryen | main | wasm | existing | 0 | 6 | no |
| wasm/wasm-bindgen | wasm-bindgen | wasm-bindgen/wasm-bindgen | main | wasm | existing | 0 | 0 | no |
| wasm/wasi-sdk | wasi-sdk | WebAssembly/wasi-sdk | main | wasm | existing | 0 | 0 | no |
| consumers/zed | zed | zed-industries/zed | main | consumer | existing | 1 | 28 | no |
| consumers/bun | bun | oven-sh/bun | main | consumer | existing | 4482 | 1 | no |

## Decisions

- **Not forked: rust-lang/crates.io-index.** 427 MB of one-commit-per-publish history; the sparse index (index.crates.io) replaces it for every client and a fork would only mirror a registry cache.
- **Name collision.** The organization already forks llvm/llvm-project as `llvm-project`, so rust-lang/llvm-project (the LLVM that rustc pins, default branch `landing-page`, work on `rustc/*` branches) is `rust-lang-llvm-project` and rust-embedded/book is `rust-embedded-book` (rust-lang/book is `book`), following naming/NAMING.md (`<upstream-owner>-<name>`, never a numeric suffix).
- **Moved upstream.** rustwasm/wasm-bindgen now lives at wasm-bindgen/wasm-bindgen; the existing fork's parent is the new owner.
- **Beyond the requested minimum.** rust-lang/gcc, enzyme and tpde (rust submodules), rust-embedded/book (rust submodule), goals, compiler-team, simpleinfra, www.rust-lang.org and mdBook: governance and infrastructure an agent needs next to the compiler.
- **Not in the list on purpose.** WebKit (a dependency of bun, not of Rust), and any crate on crates.io: crates come from the registry, not from forks.
- **Same project, two homes.** rust-lang/rust carries cargo, backtrace-rs, rustc-perf, the books, LLVM, gcc, enzyme and tpde as git submodules and miri, clippy, rust-analyzer, rustfmt, rustc_codegen_*, stdarch, compiler-builtins and rustc-dev-guide as josh subtrees. The monorepo pins the standalone fork, once; see docs/architecture.md.
