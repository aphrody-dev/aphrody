# Layout against Chromium

Chromium's top level set against this repository. The live comparison (Chromium's current top level against this tree) is generated:
`bun testing/acceptance.ts docs` writes `.local/docs/chromium-layout.md`. Chromium is a C++ product tree; this one is a Rust
workspace around imported engines, so what has no counterpart is listed with the reason.

## Directories

| Chromium | Here | Note |
|---|---|---|
| `base/` | `third_party/bun/src/bun_core`, ... | the foundation library of the runtime is Bun's, in its submodule |
| `build/`, `buildtools/`, `build_overrides/` | `build/engines/` | the GN overlay (V8, Skia, Blink from the Chromium checkout); the toolchains are the Chromium checkout's own |
| `components/` | `components/` | first-party crates (`browser-detect`, `browser-index`, `browser-engine` with `process`; next: `cdp`, `v8-sys`, `skia-sys`) |
| `docs/` | `docs/` | this file, the inventory, [../styleguide/bun](../styleguide/bun/README.md) is the style guide |
| `styleguide/` | `styleguide/bun/` | JavaScript and TypeScript style with its proof; Rust follows `rustfmt` and the workspace lints |
| `testing/` | `testing/` | the acceptance tool: engines x live corpus, benchmarks, dashboard |
| `third_party/` | `third_party/<upstream>/<crate>` | one directory per upstream, `README.chromium` each (generated); Chromium's own checkout is `third_party/chromium`, ignored |
| `tools/` | `tools/` | layout, importer, flattener, pins |
| `infra/` | none yet | CI lives in the organisation's workflows |
| `headless/`, `content/`, `chrome/`, `net/`, `url/`, `skia/`, `v8/`, `ui/`, `gpu/`, ... | the Chromium checkout, through `build/engines` | built, never imported: `//headless:headless_shell`, `//v8:v8_monolith`, `//skia:skia` |
| `ios/`, `android_webview/`, `ash/`, `chromeos/`, `fuchsia_web/`, ... | not used | other products |

## Root files

| Chromium | Here |
|---|---|
| `DEPS` | `DEPS` (the Chromium checkout, pinned); imports are pinned in `tools/upstreams.lock.json` |
| `PRESUBMIT.py` | `PRESUBMIT.ts` (`bun PRESUBMIT.ts`) |
| `README.md`, `LICENSE` | `README.md`, `LICENSE` (Apache-2.0, the licence the first-party crates declare) |
| `.gitattributes`, `.gitignore`, `.gitmodules`, `.git-blame-ignore-revs` | `.gitattributes`, `.gitignore`, `.git-blame-ignore-revs` (forks are submodules with relative URLs when they appear) |
| `package.json` | `package.json` (Bun workspaces for the JS tooling) |
| `.clang-format`, `.rustfmt.toml`, `.ruff.toml` | `bunfig.toml` `[fmt]` for TypeScript; no C++ or Python here |
| `.gn`, `BUILD.gn` | `build/engines/BUILD.gn`, linked into the Chromium checkout as `//browser` by `build/engines/build.sh` |
| `OWNERS`, `*_OWNERS`, `DIR_METADATA`, `WATCHLISTS` | not adopted: ownership is the organisation's, in GitHub |
| `AUTHORS`, `CODE_OF_CONDUCT.md`, `.mailmap`, `.gitallowed`, `codereview.settings`, `vpython.toml` | not adopted |

## Third party, vendored and first party

- **First party:** `components/<crate>` (new crates are `browser-<unit>`), `testing/`, `tools/`, `styleguide/`, `.claude/`.
- **Third party, imported with history:** `third_party/<upstream>/`: Servo and its organisation's crates, Mozilla (`mozjs`, `mozangle`),
  HarfBuzz/FreeType/platform bindings, `lol-html`, `rust-argon2`, `hyperfine`, our forks `obscura` and `agent-browser`. Local
  modifications are listed in each `README.chromium`.
- **Runtime:** `third_party/bun`, a submodule of aphrody-labs/bun (rebased on oven-sh/bun) with a relative URL: its own workspace and build, as a Chromium dependency has.
- **Not imported:** the Chromium checkout, JetStream (a pin in `tools/upstreams.lock.json`), the 16 GB of Chromium history.
