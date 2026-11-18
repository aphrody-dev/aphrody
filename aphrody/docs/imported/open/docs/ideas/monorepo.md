# The `open` workspace

## Contents

- One Cargo workspace (`crates/`: open, browse, crawl, browser-engines, tunnel, tui, ai-router, c-abi, adblock-capi, hub) and Bun workspaces (`packages/*`, `apps/*`, `extension`); the layout table is in the [README](../../README.md).
- No vendored upstream: crates come from crates.io or from a git revision pinned in the root `Cargo.toml`; the table of pins and the ledger of the copies removed from the tree are in [`../upstream.md`](../upstream.md).

## How it is built

- Rust: `rust-toolchain.toml` names the nightly; `.cargo/config.toml` links with lld through clang on Linux x86_64 and points `OPENTUI_LIB_DIR` at `target/opentui-native` (`bun scripts/opentui-native.ts` fills it).
- Bun: root `package.json` workspaces, `bunfig.toml` isolated linker; tests run per package with `bun scripts/test-ts.ts` (never at the root).
- CI: `.github/workflows/ci.yml` runs `cargo check`, `cargo test`, the extension build and its tests; `ai-router.yml` builds and tests the router on three operating systems and publishes its image; `libraries.yml` attaches the libraries to a release.

## Missing (observed)

- No AOT binary and no Rust LTO release build produced in CI.
- No site or entry documentation beyond `README.md`.
