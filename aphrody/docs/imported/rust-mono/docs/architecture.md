# Architecture

## Shape

rust-mono is a Git superproject. Its tree contains tooling (`tools/`), documentation (`docs/`), the manifest (`forks.toml`), the sync history (`ledger/`) and 51 gitlinks. Each gitlink points at one commit of one fork in the same GitHub organization; `.gitmodules` uses relative URLs (`../<name>.git`), so a clone of the superproject from any host or mirror resolves the forks next to it. Nothing about the organization is written in a script: it is `[meta] org` in `forks.toml`.

```text
rust-mono/
  forks.toml  .gitmodules  ledger/        manifest, generated submodule list, sync history
  tools/  docs/  .github/                 Bun scripts + tests, these pages, CI and the sync workflow
  compiler/    rust, rust-lang-llvm-project, llvm-project, gcc, enzyme, tpde, miri, rustc_codegen_cranelift, rustc_codegen_gcc
  devtools/    cargo, rustup, rust-analyzer, rust-clippy, rustfmt, mdBook
  std/         stdarch, compiler-builtins, backtrace-rs, libc, hashbrown, cc-rs
  registry/    crates.io, docs.rs
  books/       reference, book, rust-by-example, nomicon, edition-guide, rustc-dev-guide, rust-embedded-book
  governance/  rfcs, team, goals, compiler-team, rust-forge, blog.rust-lang.org, this-week-in-rust, www.rust-lang.org
  infra/       rustc-perf, crater, bors, triagebot, simpleinfra
  wasm/        wasmtime, wasm-tools, wit-bindgen, binaryen, wasm-bindgen, wasi-sdk
  consumers/   zed, bun
```

Group names avoid `docs/` and `tools/`, which belong to the superproject itself. A group is a directory of one concern; the role (`compiler`, `backend`, `codegen`, `std`, `package-manager`, `installer`, `ide`, `devtool`, `registry`, `docs`, `governance`, `infra`, `wasm`, `consumer`) is finer and lives in `forks.toml`. The path is always `<group>/<fork name>`, enforced by the manifest validator.

## Why submodules and not one merged tree

- **History and upstream relations survive.** A fork keeps its full history, its branches and its upstream parent, so `merge-upstream`, compare, blame and upstream PRs still work. A squashed or subtree copy loses all of that.
- **Size.** On GitHub, rust-lang/rust is 1.0 GB, the LLVM it pins 3.4 GB, llvm/llvm-project 4.4 GB, bun 1.6 GB, zed 0.5 GB. One working tree of everything is not something a workstation holds; a pinned submodule is cloned only when someone needs it, with `--depth 1` or `--filter=blob:none`.
- **Reproducibility is the gitlink.** One superproject commit names the exact commit of every project. Check it out and you have the same inputs, anywhere.

## One checkout each

The owner's rule: a fork is cloned once. Two things follow.

1. rust-mono never copies a fork's files.
2. rust-lang/rust itself carries cargo, backtrace-rs, rustc-perf, the books, LLVM, gcc, enzyme and tpde as submodules, and miri, clippy, rust-analyzer, rustfmt, rustc_codegen_cranelift, rustc_codegen_gcc, stdarch, compiler-builtins and rustc-dev-guide as josh subtree copies. The monorepo pins the standalone fork once (for example `devtools/cargo`) and `forks.toml` records where rust uses it (`rust_submodule`, `rust_subtree`). A build of rust therefore does not initialise `compiler/rust/src/tools/cargo` separately: the build wrapper (phase 5) points the nested path at the monorepo's checkout, with `git submodule update --reference` or a bind of the existing directory.

   Pins are independent of what rust pins. The sync moves each fork to its own upstream head, so `devtools/cargo` may be newer than the cargo commit `compiler/rust` expects. Builds that need the matching commit read it with `git ls-tree <rust-sha> src/tools/cargo` (the gitlink of the nested submodule) and check that out; this is the same information as the `rust_submodule` field.

## The LLVM question

rust-lang/llvm-project is the LLVM that rustc pins: its default branch is `landing-page`, the work is on `rustc/<llvm-version>-<date>` branches. The monorepo tracks the branch rust's gitlink points into (`rustc/23.1-2026-07-22`, whose head contains the commit rust pins: verified with the compare API, 44 commits ahead of it). When rust moves to a new `rustc/*` branch, the `branch` of `compiler/rust-lang-llvm-project` changes in `forks.toml`; the sync does not discover new branches by itself. The unmodified upstream is `compiler/llvm-project` (llvm/llvm-project); the two have different names because the organization already forked llvm/llvm-project as `llvm-project` and the naming scheme forbids numeric suffixes.

## Not forked

rust-lang/crates.io-index: 427 MB of one-commit-per-publish history. Clients use the sparse index (index.crates.io); a fork would mirror a cache. rust-lang/crates.io (the registry code) is forked.

## Naming

The repository is `rust-mono`, kind `research` in aphrody-labs (`naming.toml`; `bun naming/check.ts` of the organization's `.github` repository reports 0 errors, 0 warnings). It has to live in aphrody-labs: the relative submodule URLs resolve to sibling repositories of the same owner, and the forks are in aphrody-labs. No scheme kind fits a toolchain aggregate better; `research` is the one that accepts a free kebab-case id there. Forks keep their upstream names (`rust-lang-llvm-project` and `rust-embedded-book` follow the collision rule).

## Standard

Settings, labels, rulesets and the security profile follow the organization standard. The repository is mostly vendored pointers, so it carries the topic `aphrody-vendored` (forks security profile: alerts on, security updates off). Dependabot covers only GitHub Actions: the submodules are the sync workflow's job.
