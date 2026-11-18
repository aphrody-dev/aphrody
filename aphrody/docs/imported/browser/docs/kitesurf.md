# Kitesurf in this repository

`third_party/kitesurf` is the aphrody-labs fork of [cloudflare/kitesurf](https://github.com/cloudflare/kitesurf) (Apache-2.0), registered as a
submodule (the build host fetches it; the PC does not check it out). It is an experimental browser engine in Rust and WebAssembly: Blitz for the
DOM and layout, Stylo for CSS, Taffy for flex and grid, Parley for text, Vello for painting, usvg for SVG and Boa for JavaScript, behind a Chrome
DevTools Protocol endpoint, with one isolate per page. It runs on Cloudflare Workers (Dynamic Workers, Workers Paid) and locally under `wrangler dev`.

## What the patches are

`scripts/patches/` holds 193 patches on 24 vendored crates, applied to the crates.io sources. They are where Kitesurf's work is:

| Crate | Patches | Lines | What they add |
| :-- | --: | --: | :-- |
| blitz-dom | 75 | 20,615 | shadow DOM and slots, `::slotted`, flat-tree layout, CSS counters, anonymous table boxes, `:has()` invalidation, directionality and bidi, DOM selections and carets, incremental layout and damage tracking, lazy `@font-face`, form controls driven by the embedder |
| boa_engine, boa_parser, boa_ast, boa_gc, boa_interner | 44 | 10,934 | the JavaScript engine the pages run on |
| blitz-paint | 28 | 4,591 | painting of the new box kinds, re-anchored abspos, SVG fragments |
| stylo | 16 | 3,161 | relative-selector invalidation, animation state, the properties Gecko-only builds hide |
| parley, parley_engine | 9 | 3,100 | text layout |
| taffy | 7 | 1,106 | grid information, container size queries |
| usvg, jpeg-encoder, vello_cpu, vello_common, selectors, anyrender* | 16 | ~2,800 | images, SVG with the embedder's fonts, CPU painting |
| playwright, puppeteer | 2 | | the client patches that let the standard suites run |

## How close to Chrome it is, measured

Kitesurf measures itself with three gates (`tests/*-expectations.json`): the Web Platform Tests (a pinned corpus of 9,722 tests), the Puppeteer
suite and the Playwright suite. Its own expectations record 7,001 passes and 2,357 expected failures on the WPT corpus. By area:

| WPT area | Tests | Pass |
| :-- | --: | --: |
| css | 4,082 | 72 % |
| html | 1,888 | 77 % |
| encoding | 1,266 | 99 % |
| dom | 487 | 77 % |
| wasm | 357 | 83 % |
| fetch | 201 | 50 % |
| shadow-dom | 76 | 37 % |
| WebCryptoAPI | 97 | 14 % |
| editing | 120 | 8 % |

Chromium 160 passes nearly all of this corpus, so the gap is about a quarter of the tests, concentrated in CSS, fetch, editing, shadow DOM and
cryptography. The Puppeteer and Playwright expectations record 53 and 357 failures (261 skipped) against Chrome's own suites.

## What this repository does with it

1. **Build and test on the build host only.** The Rust core's host tests (`bazel test //packages/core:host_tests`) pass 5 of 5 from a cold Bazel
   cache in 287 s on the build host; nothing of Kitesurf is built on the PC. `bun ../ay/tools/sync-host.ts` puts a checkout there in seconds.
2. **Parity against the real thing.** The WPT corpus pinned by Kitesurf (`tests/wpt-revision.json`) runs on Chromium 160.0.8166.0 (`headless_shell`
   from `build/engines`, the same version `DEPS` pins) to give the reference expectations; `testing/acceptance.ts` compares the two per area. The
   parity number of an engine is the share of the tests Chromium passes that it passes too, not the share of the corpus.
3. **Work the failing areas by weight.** CSS (1,158 failures) and HTML (407) first, then fetch, shadow DOM and editing; each fix is a patch against
   the DioxusLabs crates, carried in the aphrody-labs forks of `blitz`, `taffy`, `anyrender` and `dioxus` rather than as vendored copies, and sent
   upstream when it is not Kitesurf-specific.
4. **Chromium itself.** Kitesurf is not a Chromium build and does not become one. Chromium 160 is built from its own sources (`build/engines`)
   with the build tools of `ay-tools` (siso, ninja, gn, depot_tools at the revisions its DEPS pins); Kitesurf is the lite, edge and Rust path.

## State of the integration (2026-11-17)

- **Patches absorbed into forks.** Each patched crate was cut from an exact upstream commit (`.cargo_vcs_info.json` of the published crate); the series
  is applied there as one commit per patch, with its real author, on the `kitesurf` branch of the aphrody-labs fork: parley, blitz, anyrender, taffy,
  stylo (with selectors), boa, resvg (usvg), vello, ratatui-image, jpeg-encoder. Four of those forks were created for this. `tools/kitesurf-absorb.ts`
  does it (the Cargo.toml hunks, which target the normalized published manifest, are replayed on the upstream form).
- **One prefixed workspace.** The submodule's `crates/` holds the 26 engine crates (the patched ones and the four that depend on them: glifo,
  stylo_dom, stylo_malloc_size_of, stylo_traits), named `browser-<crate>`, library names unchanged, siblings by path and version, `FEATURES.md` as the
  feature matrix. `tools/kitesurf-monorepo.ts` generates the layout from `cargo metadata` (the closure is computed, never listed). Proof on the build
  host: `cargo check` of the three wasm packages, 771 host tests of kitesurf-core passing, `cargo package` of a crate. The 26 `browser-*` names are free on crates.io; nothing is published.
- **Bun.** `bun.lock` was migrated from `pnpm-lock.yaml` and the developer scripts run on Bun. Bazel still reads `pnpm-lock.yaml`, the patch series and the
  node-run tests (`e2e-global-setup`, the lint rule tests), so those stay until the Bazel rules are moved to the workspace and to Bun.
- **Not done.** Bazel on `crates/`; the Chromium 160 WPT reference run and the per-area parity matrix; publishing.
