# How an agent finds something

Start from `forks.toml`: every fork has a `path`, an `upstream`, a `role` and, when rust carries the same project, `rust_submodule` or `rust_subtree`.

| You want | Do |
| :-- | :-- |
| Which repository holds X (cargo, the reference, bors, the LLVM rustc uses) | `grep -n -B1 -A8 'name = "cargo"' forks.toml`, or `bun tools/forks.ts --json` and filter on `path` |
| The exact commit in use | `git ls-files -s <path>` (mode 160000), or `git ls-tree HEAD <path>` |
| How far a fork is behind upstream, live | `bun tools/forks.ts --only cargo` |
| What changed upstream and when it was merged | `ledger/<YYYY-MM>.jsonl`, one JSON object per line, filter `fork` or `upstream` |
| The file, without cloning everything | `gh api repos/<org>/<name>/contents/<file>?ref=<sha>` using `<org>` from `forks.toml [meta]` and `<sha>` from the pin |
| A working tree | `git submodule update --init --depth 1 -- <path>` (blobless alternative: `git -C <path> fetch --filter=blob:none` after init) |
| Where rust uses this project | `rust_submodule` / `rust_subtree` of the entry; the gitlink rust expects: `git ls-tree <rust-sha> <rust_submodule>` |
| Issues, PRs, RFCs, release notes | not stored here; read them from the upstream with `gh` until the knowledge feed exists ([knowledge-feed.md](knowledge-feed.md)) |

Rules that keep this safe: no write on an upstream (no PR, issue or comment on rust-lang, llvm, bytecodealliance...), no compilation on a workstation, no full clone of `compiler/rust` or the LLVM forks on a workstation (use `--depth 1` or `--filter=blob:none`; register pins with `git update-index --add --cacheinfo`, as `tools/submodules.ts` does).

Group meaning: `compiler` (rustc, LLVM, backends, miri), `devtools` (cargo, rustup, rust-analyzer, clippy, rustfmt, mdBook), `std` (library crates), `registry` (crates.io, docs.rs), `books`, `governance` (RFCs, teams, goals, forge, blog, newsletter), `infra` (perf, crater, bors, triagebot), `wasm`, `consumers`.
