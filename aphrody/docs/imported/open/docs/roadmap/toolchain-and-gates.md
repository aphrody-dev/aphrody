# Toolchain and validation gates

## Monorepo toolchain (decisions D12, D13)

| Role | Tool |
|---|---|
| Build graph, caching, remote execution | Bazel (everywhere) |
| Commands | just |
| JS/TS task orchestration and cache | Turborepo |
| JS/TS: packages, scripts, tests, bundling, AOT | Bun |
| Rust | Cargo, release profile with fat LTO |
| Native compilation, cross-language C/C++/Rust LTO | LLVM (clang, lld) |

## Validation gates

Nothing compiles without passing the gates. They run as a build plugin:
- Bun side: `Bun.plugin` / `bun build` plugin (`onLoad`, `onResolve`) validating every loaded file;
- Cargo side: shared `build.rs` or a `cargo` subcommand run before `cargo build`.

Levels follow Bun's commands (D13): `check`, `fmt`, `lint`, `build`, `compile`, `test`, `webview`, `pm`, reusing `ay` and the Bun Rust crates in Aphrody.

| Gate | What it checks | Source of truth |
|---|---|---|
| Modern web practices | APIs and patterns, Baseline features | GoogleChrome/modern-web-guidance, web-platform-dx/web-features |
| HTML | W3C conformance | validator/validator (Nu Html Checker) |
| CSS | W3C conformance | w3c/css-validator |
| Accessibility | WCAG | dequelabs/axe-core, w3c/wcag |
| Material Design | components, tokens, states, layout, M3 accessibility | m3.material.io, design.google (reference archive), specs extracted to JSON |
| Design system | no hardcoded value: colours, fonts, radii, spacing come from the official tokens | generated tokens |
| Performance | Core Web Vitals (LCP, INP, CLS) | GoogleChrome/lighthouse, GoogleChrome/web-vitals |
| Docs and roadmap | timeless docs, requirements tied to their spec, valid links | Google developer documentation style guide, Google eng-practices review guide |
| Hygiene | pinned versions and dates, absolute paths, hardcoded lists, slop, non-English source text | hygiene scanner |
| TypeScript toolchain | type check, lint, format, native compile | `bun check`, oxc, TypeScript 7, scriptc, Bun AOT (see [../ideas/compilers.md](../ideas/compilers.md)) |

Limit: Core Web Vitals are measured on an executed rendering. That gate runs right after compilation and blocks release, not compilation itself. The other gates are static and block before compilation.
