# Decisions

## Generalist rule (D21)

Aphrody is generalist: no framework, programming language, OS, architecture or human language is hand-picked. Every selection is a rule applied to the generated registries, re-evaluated when they change:

| Dimension | Registry | Rule |
|---|---|---|
| UI frameworks | [inventory/frameworks.json](../ideas/inventory/frameworks.json) (registries + GitHub) | every framework with an official or community Material implementation is a target; the official Google implementations are the reference |
| Programming languages | [inventory/targets.json](../ideas/inventory/targets.json) `languages` (Linguist) | every language with a C FFI consumes the shared C ABI |
| OS and architectures | `targets.json` `rust`, `go`, `zig`, `bun` | every target the toolchains of a component support |
| Human languages | `targets.json` `locales` (CLDR) | every CLDR locale, English as source |

The names in the decisions below are the first instances of these rules (where work starts), not limits.

## Taken

| Id | Question | Decision |
|---|---|---|
| D2 | Shape of the router | **Embedded library and service**: one core, one C ABI, a service on top |
| D3 | Web UI | **Angular** on a fork that follows upstream automatically, with aggressive Bun patches, AOT and LTO compilation, and a C ABI shared with the other frameworks (Rust, TypeScript, Zig, .NET, C++, ...) |
| D4 | Desktop, mobile, web | **Maximum coverage**: Flutter (desktop, mobile, web), Rust at the same level, Jetpack Compose Multiplatform as well |
| D8 | Inference | **Local first**; every inference type and every cloud provider |
| D12 | Build system | **Bazel everywhere** in Aphrody (thought of as a future Google project); just, Turborepo, Bun, Cargo and LLVM are called by Bazel targets |
| D13 | Gate levels | **Modelled on Bun's commands**: `check`, `fmt`, `lint`, `build`, `compile`, `test`, `webview`, `pm`; reuse `ay` and the Bun Rust crates already in Aphrody |
| D14 | Material spec sources | **Multi-source extraction**: Figma Material 3 kit; GitHub repositories of Google, Chrome and Chromium (UI views), Jetpack Compose, Angular components, Flutter; the full sites from A to Z; Google's design skills and MCP servers; community versions (MUI, ...) |
| D18 | Ecosystem inventory | **Very wide**: Material for GTK 4, Qt, COSMIC, Hyprland, egui, iced, Dioxus, gpui; taffy; shadcn, Tailwind, Base UI; React, Svelte, JS, TS; .NET; etc. |
| D3 status | Angular AOT | Angular's template compiler needs the TypeScript 6 compiler API (angular/angular#69704); TypeScript 7 exposes none before 7.1. No TypeScript 6 is shipped: Angular runs in JIT on Bun's transpiler until compiler-cli supports TypeScript 7 |
| D19 | M3 components in the Bun app | **Angular Material**, following D3 (Material Web removed from src as legacy) |
| D20 | Language | **English at the source**, multilingual through i18n and a translation stack, for every project |
| D21 | Selections | **Generalist**: rules over generated registries, see the top of this file |
| D22 | `open` and `auto` | **One codebase, two editions**: `open` is the open-source core, `auto` the paid hosted edition that consumes it ([vision.md](vision.md)) |
| D23 | Naming | Rules and arguments below |
| D24 | Site values | Rules and arguments below |
| D25 | Imports into `open` | Rules and arguments below |

## D23: naming

1. **A name says what the thing does, never who owns it or where it runs.** `open` is a library for any user and any deployment: an owner, brand or host name means nothing to a user, breaks on another deployment and leaks private infrastructure. This covers directories, packages, crates, modules, types, functions, environment variables, flags and comments. Marketing words ("Sovereign", "ultimate") are not names.
2. **Registry coordinates are not code.** An npm scope or a crates.io name exists to avoid collisions in a shared registry; it appears only in the manifest `name` field. npm packages keep the scope the project already publishes under (`@aphrody/`), because changing a published scope is a registry migration with no effect on the code. Rust crates take a functional name and add a prefix only when crates.io already has that name, checked at publication.
3. **One symbol prefix per C ABI.** C has a single global symbol namespace per process, so every exported symbol needs a prefix that no other library uses. The prefix belongs to the ABI, not to the crate that implements it: crates implementing the shared ABI of `aphrody-core-capi` export `aphrody_*`, as that ABI defines.
4. **Environment variables belong to their component**: `<COMPONENT>_<SETTING>`, the component being the package or crate name in upper snake case. No project-wide prefix: a variable is read by one component and documented with it.
5. **Spelling**: kebab-case for directories, files, packages and crates; one concern per module; tests in `test/` next to `src/`; English source.

## D24: site values

1. **Hosts, addresses, users, accounts and folders are resolved when the code runs, never stored**, in the repository or in a file it points to. The caller's own configuration is the input: an ssh destination is resolved by `ssh -G` from the caller's `~/.ssh/config`; a runtime comes from the target's runtime manager; a tool is started by package name (`bun x --bun <package>`), not by install path.
2. **Defaults are the platform's**: data in the user state directory (`LOCALAPPDATA`, `XDG_STATE_HOME` or `~/.local/state`), temporary files in the platform temp directory; never a path that exists only on one machine.
3. **The gate** `scripts/check/private.ts` reads this machine's private terms at run time (user, home, the folders the checkout lives in, host name, git e-mail, ssh aliases and host names, the networks of a fleet file given as input) and fails on any tracked first-party file that contains one. Generators that quote another repository's text redact the same terms (`scripts/check/private-terms.ts`).
4. **Documents state requirements and decisions, not who said them**, and quote no person.

## D25: imports into `open`

An import lands only after six checks, recorded in `docs/reviews/<unit>.md`:

1. **Provenance**: history replayed with a `Moved from <repository> <commit>` trailer per commit, or a lock entry for upstream code.
2. **Verification**: its tests run green in `open`.
3. **Purification**: site values follow D24; the private-data gate is clean.
4. **Purge**: code that belongs to another product, to one deployment or to nobody (dead code, demonstrations, fake outputs) stays in its repository or is deleted.
5. **Reorganisation**: split by concern into units named by D23.
6. **Review**: each point checked against the code, outcome written.

The rule was applied to the units already moved: `packages/chromium-bun` became `packages/browser-engines` and `packages/crawl` ([review](../reviews/browser-engines.md)), `crates/adblock-capi` ([review](../reviews/adblock-capi.md)).

## Still open

| Id | Question | Options | Recommendation |
|---|---|---|---|
| D1 | What stays in `open`? | everything (93 sources + ecosystem); only what the products use | keep the full lock, build only what the products use |
| D5 | Terminal | opentui + M3 palette; other | opentui |
| D6 | Reference archive | tools of [browser-and-crawl](../ideas/browser-and-crawl.md); storage | Crawlee + Chrome for Testing + browsertrix (WACZ); storage to define |
| D7 | Rust native UI framework | catalog; common base; choose one | with D4 (Rust at Flutter's level), pick the one that carries M3 best after comparison |
| D9 | Scope of "every inference type" | text, vision, audio (STT/TTS), image, video, embeddings, rerank, realtime | priority order to set |
| D10 | Cloud providers | every discovered provider (models.dev, OpenRouter, Hugging Face, Cloudflare and Vercel gateways) | catalog discovered at run time, no hardcoded list |
| D11 | Local engines | llama.cpp, Ollama, ONNX Runtime, OpenVINO, MLX, WebGPU (browser) | per target hardware |
| D15 | Results store | SQLite as source, Redis as cache and queue, JSON and MDX as exports | as stated |
| D16 | Target frameworks of the full-stack generation | Angular, Next.js, Flutter web, Leptos, Dioxus, ... | to set |
| D17 | Where the benchmark runs | PC only, VPS, or both with distribution | both |
