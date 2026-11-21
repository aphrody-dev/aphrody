# Layout 2026: aphrody

The target layout of this repository. The rules, the conventions with their sources, the proof of relocation safety and the upstream-or-in-house decisions are in
`docs/architecture/layout-2026.md` of aphrody-labs/fw; this page is the decision for every tracked path of this repository, generated from `layout.toml`
(`bun ../fw/tools/layout.ts doc`) and `git ls-files`. A path that is not in the map fails `bun ../fw/tools/layout.ts check` in CI.

aphrody is the OS and desktop layer, a Zed fork: it keeps the upstream Zed layout (`crates/`, `assets/`, `extensions/`, `script/`, `tooling/`) so merges from upstream stay mechanical; the Aphrody additions live in `crates/aphrody_*`, `crates/nsh-*`, `tooling/bun`, `site/`, `templates/` and `examples/`. It depends on fw only; direction fw <- aphrody <- ay, pinned git dependencies, no submodules, no vendor/.

## Decision for every tracked path

<!-- layout:begin generated (bun tools/layout.ts doc, from layout.toml and git ls-files of 9754 paths) -->

### Every tracked path, by area

| area | files | decision | reason |
| --- | ---: | --- | --- |
| `.agents` | 111 | keep | agent plugin manifests read from .agents at the root |
| `.cargo` | 3 | keep | Cargo reads .cargo/*.toml from the workspace root |
| `.cloudflare` | 12 | keep | Cloudflare worker project for aphrody.com |
| `.config` | 3 | keep | lychee, nextest and typos discover their config under .config |
| `.dockerignore` | 1 | keep | Docker tooling reads these from the build context root |
| `.editorconfig` | 1 | keep | editors and search/lint tools discover their config upward from the root |
| `.git-blame-ignore-revs` | 1 | keep | Git reads these from the repository root |
| `.gitattributes` | 1 | keep | Git reads these from the repository root |
| `.github` | 22 | keep | GitHub reads workflows, actions and templates from .github |
| `.gitignore` | 1 | keep | Git reads these from the repository root |
| `.mailmap` | 1 | keep | Git reads these from the repository root |
| `.prettierrc` | 1 | keep | Bun workspace root: install, lockfile, runtime config and TypeScript base config are read from the root |
| `.stitch` | 4 | keep | Google Stitch design system files (DESIGN.md, SITE.md) read from .stitch |
| `.zed` | 3 | keep | Zed project settings and tasks |
| `AGENTS.md` | 1 | keep | agent rules and the style contract; agents load them from the root |
| `assets` | 513 | keep | upstream Zed layout: embedded assets (icons, keymaps, themes, fonts); kept so upstream merges stay mechanical |
| `bunfig.toml` | 1 | keep | Bun workspace root: install, lockfile, runtime config and TypeScript base config are read from the root |
| `Cargo.lock` | 1 | keep | Cargo and the Rust toolchain read these from the workspace root |
| `Cargo.toml` | 1 | keep | Cargo and the Rust toolchain read these from the workspace root |
| `clippy.toml` | 1 | keep | Cargo and the Rust toolchain read these from the workspace root |
| `CODE_OF_CONDUCT.md` | 1 | keep | repository front matter and legal files, read by GitHub and package registries at the root |
| `CONTRIBUTING.md` | 1 | keep | repository front matter and legal files, read by GitHub and package registries at the root |
| `corgi.toml` | 1 | keep | Cargo and the Rust toolchain read these from the workspace root |
| `crates` | 7660 | keep | upstream Zed layout: flat crates/* (Cargo convention); kept so upstream merges stay mechanical |
| `default.nix` | 1 | keep | Nix flakes and legacy entry points are looked up at the root |
| `deny.toml` | 1 | keep | Cargo and the Rust toolchain read these from the workspace root |
| `DESIGN.md` | 1 | keep | repository front matter and legal files, read by GitHub and package registries at the root |
| `docker-bake.hcl` | 1 | keep | Docker tooling reads these from the build context root |
| `Dockerfile-distros` | 1 | keep | Docker tooling reads these from the build context root |
| `Dockerfile-distros.dockerignore` | 1 | keep | Docker tooling reads these from the build context root |
| `docs` | 388 | keep | upstream Zed docs tree (mdbook) plus aphrody docs |
| `examples` | 543 | keep | runnable examples (gpu, nsh), outside the build workspace |
| `extensions` | 49 | keep | upstream Zed layout: built-in extensions as workspace members |
| `flake.lock` | 1 | keep | Nix flakes and legacy entry points are looked up at the root |
| `flake.nix` | 1 | keep | Nix flakes and legacy entry points are looked up at the root |
| `justfile` | 1 | keep | just discovers the justfile at the root |
| `layout.toml` | 1 | keep | declarative product, naming, release and layout configuration: the single root source other files are derived from |
| `legal` | 5 | keep | terms and policies shipped with the product |
| `LICENSE-APACHE` | 1 | keep | repository front matter and legal files, read by GitHub and package registries at the root |
| `LICENSE-GPL` | 1 | keep | repository front matter and legal files, read by GitHub and package registries at the root |
| `naming.toml` | 1 | keep | declarative product, naming, release and layout configuration: the single root source other files are derived from |
| `nix` | 14 | keep | Nix build and module definitions referenced by flake.nix |
| `package.json` | 1 | keep | Bun workspace root: install, lockfile, runtime config and TypeScript base config are read from the root |
| `product.toml` | 1 | keep | declarative product, naming, release and layout configuration: the single root source other files are derived from |
| `README.md` | 1 | keep | repository front matter and legal files, read by GitHub and package registries at the root |
| `rust-toolchain.toml` | 1 | keep | Cargo and the Rust toolchain read these from the workspace root |
| `rustfmt.toml` | 1 | keep | Cargo and the Rust toolchain read these from the workspace root |
| `script` | 76 | keep | upstream Zed layout: release and developer scripts |
| `shell.nix` | 1 | keep | Nix flakes and legacy entry points are looked up at the root |
| `site` | 5 | keep | aphrody.com site generator and public assets |
| `templates` | 36 | keep | project templates (m3-gpui-nsh, wasm-wgpu) |
| `tooling` | 273 | keep | upstream Zed layout: repository tooling (xtask, compliance, lints, perf) plus Bun tooling |

### Move table (old path -> new path)

| old path | new path | reason | status |
| --- | --- | --- | --- |
| (none) | | | |

<!-- layout:end generated -->
