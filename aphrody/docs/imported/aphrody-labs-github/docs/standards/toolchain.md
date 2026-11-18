# Toolchain standard

One toolchain for every repository. `templates/toolchain.toml` is the source; `tools/toolchain.ts` reads it. The shape is Zed's
(`rust-toolchain.toml`, `.cargo/config.toml`, profiles, `clippy.toml`, nextest, typos, sccache in CI); the version is the latest
stable, where Zed keeps an older pin on purpose.

| Need | Rule | Where |
| :-- | :-- | :-- |
| Rust version | `rust-toolchain.toml` in every Cargo repository: latest stable, `profile = "minimal"`, `rustfmt clippy rust-analyzer rust-src`. A nightly is pinned by date, with the CI image. | `[rust]`, `[repo.<name>]` |
| Switching | rustup selects the toolchain from the file; `bun tools/toolchain.ts host <ssh-host> install` installs every channel the checkouts pin, with `llvm-tools`. | `host install` |
| Cargo tools | Pinned versions, installed with `cargo binstall --locked name@version` (prebuilt binaries, no compile). `update` moves them to the latest stable release. | `[tools]` |
| Compilation cache | sccache on every build host and in CI (`RUSTC_WRAPPER=sccache`, `CARGO_INCREMENTAL=0`; incremental builds cannot be cached). A remote cache is the environment's business (R2 through its S3 API); no key is written to a file. | `[sccache]`, `ci-rust.yml` |
| Profiles | Zed's dev profile (split debug info, 16 codegen units, limited debug, the same for build scripts and proc macros, which otherwise compile twice). Release is a per-repository choice; `check` reports it. | `templates/cargo-profiles.toml` |
| Unused dependencies | `cargo shear` in CI (hygiene job) and on demand: `bun tools/toolchain.ts host <ssh-host> unused <repo>`. | `ci-rust.yml` |
| Supply chain | `cargo deny` when the repository has a `deny.toml`; `typos` on every push. | `ci-rust.yml` |
| Bun | `linker = "isolated"`, `globalStore = true`, `exact`, `saveTextLockfile`, `minimumReleaseAge = 259200`. A warm install is one symlink per package (29 ms against 460 ms for a 17-package workspace). | `[bun]` |

Everything heavy (cargo builds, shear on a large workspace) runs on a build host, never on the PC.

## Commands

```sh
bun tools/toolchain.ts check [repo ...]   # drift of every checkout against the policy
bun tools/toolchain.ts apply [repo ...]   # rust-toolchain.toml, dev profiles, bunfig [install]; a file with uncommitted changes is skipped
bun tools/toolchain.ts update             # latest stable Rust and cargo tools, live
bun tools/toolchain.ts host <ssh-host> install
bun tools/toolchain.ts host <ssh-host> unused <repo>
```

## Cargo tools by purpose

- **Build and test**: cargo-nextest (tests; doctests stay on `cargo test --doc`), bacon (background checker, replaces cargo-watch), cargo-llvm-cov, cargo-mutants.
- **Dependencies**: cargo-shear (unused), cargo-deny and cargo-audit (licenses, advisories), cargo-hakari (workspace-hack), cargo-semver-checks, cargo-about.
- **Binaries**: cargo-binutils (`cargo size`, `cargo objdump`, `cargo strip`, through the `llvm-tools` component), cargo-bloat, cargo-zigbuild (cross builds).
- **Space and CI**: cargo-sweep (old artifacts of `target/`), cargo-chef (Docker layer caching), cargo-binstall, cargo-dist and cargo-release (releases).
- **Text and files**: typos-cli, taplo-cli (TOML), just.
