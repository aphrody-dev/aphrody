# Unified code style

Version 1.0.0, 2026-11-17. One standard for TypeScript and JavaScript on Bun, Rust, tests, comments, commits, pull requests and reviews. It extends [STANDARD.md](../../STANDARD.md) (section 4 naming, section 6 forms and templates) and does not replace it.

Built from five families of sources, all fetched live and recorded in [sources.toml](sources.toml) (URL, retrieval date, sha256, size): Bun (the fork's review rules, formatter and linter, and a measurement of 100 recent merged pull requests of its creator), Google (TypeScript, JavaScript, Rust style guides; engineering practices for change descriptions and reviews), Rust for Linux (kernel Rust guidelines, patch rules), Rust for Chromium (Rust docs and style guide), and the official Rust style guide. Rules are summarized in our own words; read the linked source for the full text. Disagreements between sources are decided in [conflicts.md](conflicts.md).

## How to read

- Precedence: the repository's own AGENTS.md and upstream conventions of forks and vendored trees; then the output of the native tools (`bun fmt`, `bun lint`, `rustfmt`, `clippy`); then this document; then the other guides as tie-breakers. For Bun-run code the output of `bun fmt` and `bun lint` always wins, because both are native and mandatory in this estate.
- Rows are `ID | rule | source | status | enforced by`. The source column is `TAG Section`, where the tag is defined below and the section names the heading in that source. Status is `adopted` (taken as is), `adapted` (taken with a stated change) or `rejected` (not taken, reason in the rule or in a conflict row).
- Enforced by: `fmt` is `bun fmt` or `rustfmt`, `lint` is `bun lint`, `clippy` is clippy or rustc lints, `commit` is `tools/check-commit.ts`, `review` is a human or agent reviewer with [REVIEW.md](../../templates/REVIEW.md), `ci` is a CI gate.
- `C-nn` points to a conflict row.

Profiles of Rust code: **userland** is the default; **kernel-facing** is code that is built into or talks directly to a kernel (drivers, syscall and ioctl wrappers); **browser-facing** is code that embeds or extends a browser engine or parses untrusted web content.

| Tag | Source family | Detail in sources.toml |
| :-- | :-- | :-- |
| BUN | Bun review rules, repository rules, landing guide | `bun-review`, `bun-claude`, `bun-landing-prs` |
| BUNCFG | Bun formatter, linter, rustfmt, clippy and toolchain configuration | `bun-*` config sources |
| BUNTPL | Bun pull request and issue templates | `bun-pr-template`, `bun-issue-*` |
| MEAS | Measurement of the last 100 merged pull requests of Jarred-Sumner in oven-sh/bun | measurement `bun-pr-practice` |
| GTS, GJS, GRS | Google TypeScript, JavaScript, Rust style guides | `google-tsguide`, `google-jsguide`, `google-rustguide` |
| GEP | Google engineering practices (change descriptions, small changes, reviewing) | `google-eng-*` |
| RFL, KMK, KPS, KAI | Rust for Linux guidelines, kernel Makefile lint flags, patch submission, AI and generated-content rules | `linux-*`, `kernel-docs-*` |
| CRS, CUN, CCL, CTP, CCR | Chromium Rust: style and design, unsafe and unstable policy, clippy, third-party crates, commit and review policy | `chromium-*` |
| RSG, RFM, CLP | Rust style guide, rustfmt options, clippy options | `rust-style-*`, `rustfmt-configurations`, `clippy-lint-configuration` |

## 1. TypeScript and JavaScript on Bun

| ID | Rule | Source | Status | Enforced |
| :-- | :-- | :-- | :-- | :-- |
| TS-01 | The formatter's output is the style: run `bun fmt`; never hand-format, never reformat untouched code in a focused change. | BUNCFG Formatter docs; GTS Consistency | adopted | fmt |
| TS-02 | Quotes, semicolons and trailing commas are whatever `bun fmt` prints (double quotes, semicolons, trailing commas everywhere). Google's single quotes are not used. | BUNCFG Formatter docs; GTS String literals | rejected (C-01) | fmt |
| TS-03 | Print width is 100 in `[fmt]` of bunfig.toml; indentation 2 spaces from the formatter; `.editorconfig` supplies charset, LF, final newline. | BUNCFG .prettierrc, Formatter docs; GJS Column limit | adapted (C-02) | fmt |
| TS-04 | `const` by default, `let` when reassigned, never `var`; one declaration per statement. | GTS Local variable declarations | adopted | lint |
| TS-05 | Strict equality; `== null` is the one allowed loose comparison. | GTS Equality checks | adopted | lint |
| TS-06 | ES modules only: no namespaces, no triple-slash references, no `import x = require()`. `require()` stays legal where Bun allows it (CommonJS files, built-in modules). | GTS Use modules not namespaces | adapted (C-06) | lint |
| TS-07 | Named exports; no default export except where the runtime or a tool reads one (Bun.serve entry objects, `*.config.*`). No `export let`; export only what other modules use. | GTS Exports | adapted (C-04) | lint |
| TS-08 | Do not use `any`; use `unknown` and narrow, or a precise type. A justified `any` carries a one-line reason. | GTS any Type | adopted | lint |
| TS-09 | No `@ts-ignore` or `@ts-nocheck`; `@ts-expect-error` only with a description and only where the test or boundary needs it. | GTS @ts-ignore | adapted (C-05) | lint |
| TS-10 | Type assertions and `!` need an obvious reason; prefer a runtime check that narrows. A cast where the type is statically known is a design smell: change the signature. | GTS Type and non-nullability assertions; BUN API design | adopted | lint |
| TS-11 | `interface` for object shapes, `type` for unions and aliases; `T[]` for simple element types, `Array<T>` for complex ones. | GTS Prefer interfaces over type literal aliases, Array<T> Type | adopted | lint |
| TS-12 | No wrapper objects (`new String`), no `const enum`, no `with`, no `eval` or `Function(string)`, no `debugger`, no changes to built-in prototypes, no ASI reliance. | GTS Disallowed features | adopted | lint |
| TS-13 | Private state: `#private` fields are allowed in Bun-run code (native emit, no downleveling); the Google ban on them is not used. Never signal privacy with an underscore prefix in public API. | GTS No #private fields; BUN API design | rejected (C-03) | review |
| TS-14 | Throw `new Error(...)` (or a subclass) only, never a primitive; keep the thrown object when rethrowing; try blocks cover only the code that can throw. | GTS Exception handling, Keep try blocks focused; BUN Error handling | adopted | review |
| TS-15 | Never swallow a failure or report success on one: no empty `catch`, no catch-and-return-default, no ignored I/O result. Map only the specific expected error code to the benign path. | BUN Error handling | adopted | lint, review |
| TS-16 | User-reachable failures are catchable errors, never crashes; exit nonzero after printing an error. | BUN Error handling | adopted | review |
| TS-17 | Control blocks always use braces; a one-line `if` may omit them. Prefer `for...of` over `forEach` and `for...in`. | GTS Control flow statements and blocks | adopted | lint |
| TS-18 | Template literals over concatenation; no line continuations in strings. | GTS Template literals, No line continuations | adopted | lint |
| TS-19 | Naming: UpperCamelCase for types, classes, enums; lowerCamelCase for values and functions; CONSTANT_CASE for module-level constants and enum members; no trailing underscore; descriptive names, no letter-deletion abbreviations. | GTS Naming | adopted | review |
| TS-20 | Booleans in public APIs: use a two-value enum or an options object when a bare `true` at the call site is unreadable; otherwise a `/* name= */` comment. | BUN API design; GTS Comments when calling a function | adopted | review |
| TS-21 | Small focused functions; consider splitting past about 40 lines or several nesting levels. | GTS Function Length | adopted | review |
| TS-22 | Match the file's local conventions (import placement, namespace aliases, parameter names, error-path order) before introducing a new pattern; magic numbers become derived named constants. | BUN Code style and idioms; GTS Consistency | adopted | review |
| TS-23 | Look for the in-tree helper before hand-writing one; one implementation in the module that owns the concept, never a copy between modules. | BUN Code style and idioms, Architecture and layering | adopted | review |
| TS-24 | Built-in runtime modules that ship inside Bun (`src/js` of the fork) are hot-path code: tamper-resistant intrinsics, lazy heavy requires, platform checks that tree-shake. Applies to that tree only. | BUN Code style and idioms | adopted | review |
| TS-25 | Lint settings of the estate: Bun's default rules plus the explicit rules in templates/bunfig.standard.toml; a stricter local rule is allowed, a weaker one needs a comment naming the reason. | BUNCFG Linter docs, oxlint.json | adopted | lint |

## 2. Rust

### 2.1 All profiles

| ID | Rule | Source | Status | Enforced |
| :-- | :-- | :-- | :-- | :-- |
| RS-01 | Format with rustfmt; the default style is the style. No hand formatting; `#[rustfmt::skip]` only for manual alignment and generated code. | RSG Motivation, The default Rust style; GRS Opting out of automatic formatting; RFL Style and formatting | adopted | fmt |
| RS-02 | Four spaces, 100 columns, block indent, trailing commas, LF endings (the rustfmt default and the Rust style guide). | RSG Indentation and line width, Trailing commas; RFM max_width | adopted | fmt |
| RS-03 | `rustfmt.toml` keeps stable options only: `edition = "2024"`, `style_edition = "2024"`, `newline_style = "Unix"`, `hex_literal_case = "Lower"`. Nightly-only keys appear only in repositories that pin a nightly toolchain. | RFM edition, style_edition, newline_style, hex_literal_case, Stable flags; GRS Use the appropriate formatting tool | adapted (C-08, C-14) | fmt |
| RS-04 | Do not set `use_small_heuristics = "Max"`, `wrap_comments` or other style overrides in the baseline. | CRS .rustfmt.toml; RSG The default Rust style | rejected (C-08) | fmt |
| RS-05 | Naming follows the Rust API guidelines: `as_`, `to_`, `into_` with the matching receiver, `new` and `try_new`, `Ext` suffix for extension traits, descriptive lifetimes, no letter-deletion abbreviations. | GRS Naming; RFL Naming | adopted | clippy, review |
| RS-06 | Every `unsafe` block, impl or call site has a `// SAFETY:` comment that says why the code is sound; every `unsafe fn` or `unsafe trait` has a `# Safety` doc section stating what callers must guarantee. | RFL Comments, Code documentation; CUN Safety comments; GRS Unsafe code; BUN Code style and idioms | adopted (C-24) | clippy |
| RS-07 | Make unsafe operations explicit even inside `unsafe fn` (`unsafe_op_in_unsafe_fn`); warn on comments and docs that claim safety where there is none. | KMK rust_common_flags | adopted | clippy |
| RS-08 | Keep unsafe blocks minimal: compute safe values first, wrap only the unsafe call. Expose a safe API over unsafe internals; justify unsafe for speed with a benchmark. | CUN Usage Guidelines; GRS Unsafe code; CRS (Un)Safety | adopted | clippy, review |
| RS-09 | Prefer `#[expect(lint, reason = "...")]` over `#[allow]`, so a stale suppression warns. `allow` stays for conditional compilation, macro expansions that vary, and per-architecture casts. | RFL Lints; CCL Chromium policy | adopted (C-25) | clippy |
| RS-10 | Recoverable failure is `Result`, an obvious absence is `Option`, a violated invariant is a panic. A panic is never reachable from user input, network bytes, files or the environment. Prefer `expect("reason")` over `unwrap()`; `unwrap` and `expect` are fine in tests. | GRS Reporting errors, Prefer expect over unwrap; RFL Code documentation (Panics); BUN Error handling | adopted (C-10) | clippy, review |
| RS-11 | No `dbg!`, `todo!` or `unimplemented!` in committed code (tests may use them behind `allow-*-in-tests`). | BUNCFG clippy.toml, Cargo lints | adopted | clippy |
| RS-12 | Pointer casts use `.cast()`, `.cast_mut()`, `.cast_const()`, `ptr::from_ref`, `ptr::from_mut`, not `as`; lossless numeric conversions use `From`, not `as _`. | KMK rust_common_flags; BUNCFG Cargo lints | adopted | clippy |
| RS-13 | Every acquisition is released at the acquisition site by a guard armed before the next fallible call; an allocation has exactly one named owner and one allocator; borrowed slices never outlive their memory. | BUN Native code: memory safety | adopted | review |
| RS-14 | Enums over bare booleans for non-obvious parameters; named structs over tuples; options structs implement `Default`. | GRS Prefer enum over bool in APIs, Types | adopted | review |
| RS-15 | Prefer unique ownership and borrowing over `Rc` and `Arc`; use shared ownership when the alternative is unreasonably complex, and say why. | GRS Avoid Rc and Arc | adopted | clippy, review |
| RS-16 | No glob imports except `use super::*` in test modules, enum variants in methods of that enum, and `pub use` re-exports; no preludes of our own. | GRS Don't use glob imports | adopted | review |
| RS-17 | Import layout in userland code is the rustfmt default. Kernel-facing and browser-facing profiles override it (RK-01, RB-03). | RSG Imports; RFL Imports; GRS A use statement should import items from one module | adapted (C-07) | fmt |
| RS-18 | Libraries document every public item (`missing_docs`) and avoid `pub` for items no other crate uses (`unreachable_pub`). | KMK rust_common_flags; GRS When and how to write documentation; BUNCFG Cargo lints | adopted (C-28, C-30) | clippy |
| RS-19 | Identifiers are ASCII; no bidirectional control characters in source. | KMK rust_common_flags; STANDARD i18n rules | adopted | clippy |
| RS-20 | Stable Rust and the pinned toolchain. An unstable feature needs a comment with its tracking issue and what replaces it when it stabilizes. | GRS No unstable features; CUN Unstable Rust usage; RFM Stable flags | adopted (C-14) | review |
| RS-21 | The toolchain is pinned in `rust-toolchain.toml` and CI installs exactly it; `cargo fetch --locked` and `Cargo.lock` are committed for binaries. | BUNCFG rust-toolchain.toml; STANDARD CONTRIBUTING | adopted | ci |
| RS-22 | Do not rely on `catch_unwind` for correctness; code must be sound with `panic = "abort"`. | GRS Don't call panic::catch_unwind | adopted | review |
| RS-23 | Lint levels are `warn` in Cargo.toml and CI runs `-D warnings`, so local iteration is cheap and the gate is strict. | RFL Lints; CCL Chromium policy; BUNCFG Cargo lints | adapted (C-09) | ci |
| RS-24 | `clippy.toml` sets `allow-unwrap-in-tests`, `allow-expect-in-tests`, `allow-dbg-in-tests` and `check-private-items`, so test code is not nagged and the documentation lints also cover private unsafe items. | CLP allow-unwrap-in-tests, check-private-items; RFL .clippy.toml | adopted | clippy |

### 2.2 Kernel-facing

| ID | Rule | Source | Status | Enforced |
| :-- | :-- | :-- | :-- | :-- |
| RK-01 | Imports are vertical: one item per line, braces as soon as a list has two items, and a trailing empty `//` comment on the last item keeps rustfmt from re-condensing. This makes rebases and patch series conflict less. | RFL Imports | adopted (C-07) | fmt |
| RK-02 | C types come from the kernel prelude (`c_int`, `c_char`), never from `core::ffi`. Wrapped C concepts keep the C name, with Rust casing and without repeating the module in the item name. | RFL C FFI types, Naming | adopted | review |
| RK-03 | `//` comments and `///` docs are Markdown; sentences start with a capital and end with a period, including `// SAFETY:` and `// TODO:`. Comments are for implementers, docs for users; a documented private item still uses `///`. | RFL Comments | adopted (C-22, C-23) | review |
| RK-04 | The first doc paragraph is one sentence; `# Safety`, `# Panics` (rare, a fallible path is preferred), `# Errors` and `# Examples` sections where they apply; items are linked with intra-doc links; examples are tests. | RFL Code documentation, Testing | adopted | clippy, review |
| RK-05 | Lints of the kernel profile: unsafe-op-in-unsafe-fn denied, `missing_docs`, `rust_2018_idioms`, `unreachable_pub`, `undocumented_unsafe_blocks`, `unnecessary_safety_comment`, `unnecessary_safety_doc`, `as_underscore`, `cast_lossless`, the pointer-cast lints, `ignored_unit_patterns`, `mut_mut`, `needless_bitwise_bool`, `no_mangle_with_rust_abi`, plus rustdoc `missing_crate_level_docs` and `unescaped_backticks`. | KMK rust_common_flags | adopted | clippy |
| RK-06 | Forbid or redirect hostile APIs with `disallowed-methods` and `disallowed-macros`, each with a `reason` and, where one exists, a `replacement`. | RFL .clippy.toml; BUNCFG clippy.toml | adopted | clippy |
| RK-07 | Rust that runs in a kernel panics almost never: return `Result`; allocation is fallible. | RFL Code documentation (Panics), Error handling | adopted | review |

### 2.3 Browser-facing

| ID | Rule | Source | Status | Enforced |
| :-- | :-- | :-- | :-- | :-- |
| RB-01 | Follow the Google Rust style guide, the Rust style guide and the Rust API guidelines; API design priorities are: promote use, be performant, be ergonomic, be idiomatic, minimize misuse with security impact. | CRS Rust Style Guide, Core Design Goals | adopted | review |
| RB-02 | Prefer small crates, ideally one abstraction; keep API implementations in leaf modules and only declarations and re-exports in non-leaf modules; name leaf modules after their primary item. The flat file layout of Chromium is not used with Cargo. | GRS Prefer small crates, Name source files and leaf modules, Don't implement APIs in non-leaf modules | adapted (C-13) | review |
| RB-03 | One module per `use` line (`imports_granularity = "Module"`), locally scoped trait imports, qualified calls to free functions, types imported unqualified. The granularity key needs nightly rustfmt, so the overlay file is used only where a nightly toolchain is pinned. | GRS A use statement should import items from one module, Prefer locally-scoped trait imports, Qualify calls to top-level functions; RFM imports_granularity | adapted (C-07) | fmt |
| RB-04 | FFI: bindgen for C, `cxx` for C++ (Crubit when it is available for the direction needed); wrap every FFI function in a safe Rust API; keep the `cxx` surface minimal to ease a later migration. | CRS C/C++ Interop via FFI; CRS Which tool should I use | adopted | review |
| RB-05 | `unsafe` is allowed with safety comments and without extra approval, but unsafe code gets the extra scrutiny owners give high-risk code; do not require callers to write `unsafe`; never expose raw pointers when a reference works. | CUN When to use unsafe, How to use unsafe; CRS (Un)Safety | adopted (C-29) | review |
| RB-06 | Third-party crates: one manifest lists the directly used crates with versions and features, transitive crates are derived, new crates are reviewed for licence and for `unsafe`, and a crate with `unsafe` that fails review stays out of privileged processes. | CTP Importing new crates, Security | adapted | review, ci |
| RB-07 | Clippy: enable the default set as warnings plus `missing_safety_doc` and `undocumented_unsafe_blocks`; a suppression is local, carries a short reason, and a suppression of an unsafe-related lint asks the security owner first. | CCL Chromium policy; CCL default_clippy_lints | adopted | clippy |
| RB-08 | No `async`/`.await` and no `std::thread` in code that runs inside the browser's own task system. | GRS Concurrency [chrome] | rejected (C-12) | review |
| RB-09 | Tests are plain `cargo test`; the GN-only `gtest` integration is not used. | CRS Getting Started FAQ | rejected (C-11) | ci |

## 3. Tests

| ID | Rule | Source | Status | Enforced |
| :-- | :-- | :-- | :-- | :-- |
| TE-01 | Every behavioral change ships an automated test in the same change. Manual verification and a benchmark do not count. Exceptions are docs, dependency or toolchain bumps, CI and build configuration, each named in the description. | BUN Tests reviewers reject; MEAS tests row | adopted | review |
| TE-02 | Prove the test fails for the right reason: it fails on the unfixed code, and deleting any load-bearing clause of the fix breaks a test. State this in the description. | BUN Tests reviewers reject; BUN CLAUDE Writing Tests | adopted | review |
| TE-03 | Await the observable condition; wire every failure event to reject; poll with a deadline, never sleep-then-check; never raise a timeout to hide slowness. | BUN Tests reviewers reject | adopted | review |
| TE-04 | Hermetic tests: no external network or live registry, ports from the OS (`port: 0`), temp directories from the harness, resources released by `using` or `finally` registered before the assertions. | BUN Tests reviewers reject; BUN CLAUDE Writing Tests | adopted | review |
| TE-05 | Assert the strongest invariant: exact values, exact error class and code, no vacuous assertions (un-awaited `.rejects`, expects in callbacks that may never run, loops over empty sets). | BUN Tests reviewers reject | adopted | lint, review |
| TE-06 | Cover the variant matrix: sibling entry points, both states of each flag, boundaries (at the limit passes, one past fails), error paths, every platform CI runs. Add variants beside existing tests, never by mutating them. | BUN Tests reviewers reject; BUN Cross-platform | adopted | review |
| TE-07 | Never weaken, skip or delete a test or safety net silently: each removal has a stated reason or replacement, each skip a comment with the observed failure. No committed `.only`. | BUN Tests reviewers reject, PR process | adopted | lint |
| TE-08 | Subprocess tests drain stdout, stderr and exit together and assert one combined object, stdout before exit code. | BUN Tests reviewers reject; BUN CLAUDE Writing Tests | adopted | review |
| TE-09 | Keep tests fast: about one second per test, ten per file; independent subprocess tests run concurrently. | BUN CLAUDE Writing Tests; BUN Tests reviewers reject | adopted | review |
| TE-10 | A test goes into the existing file for the code it covers; a new file only when no home exists. A regression file is named after a real issue number and only for true regressions. | BUN CLAUDE Test Organization | adopted | review |
| TE-11 | Rust: unit tests sit beside the code, doctests are the usage examples and count as tests, integration tests live under `tests/`. | RFL Testing; GRS Use the right per-codebase test framework | adopted | ci |

## 4. Comments and documentation

| ID | Rule | Source | Status | Enforced |
| :-- | :-- | :-- | :-- | :-- |
| CM-01 | Comment only what the code cannot say, in one line; never restate the code, never narrate the change. Prefer a link to an issue over prose. | BUN Code style and idioms; GTS Make comments that actually add information | adopted | review |
| CM-02 | A paragraph-long justification for a workaround means the code is wrong; fix the code. | BUN CLAUDE Important Development Notes | adopted | review |
| CM-03 | TypeScript: `/** */` documents API, `//` explains implementation; `@param` and `@returns` only when they add information beyond the name and type; no type annotations in JSDoc. | GTS JSDoc versus comments, Make comments that actually add information | adopted | review |
| CM-04 | Rust: `///` and `//!` are Markdown, public items are documented, first paragraph is one sentence; ordinary `//` comments are complete sentences with a capital and a period. | GRS When and how to write documentation; RSG Comments; RFL Comments | adopted | clippy |
| CM-05 | Documentation ships with the feature: types (`.d.ts`), help text, docs page and both surfaces (CLI flag and API) in the same change. | BUN API design | adopted | review |
| CM-06 | Error messages are reviewed like code: name the failed resource and the rejected value, give the cause and a remedy, no "Please", recovery hint on its own line, to stderr. | BUN Error handling | adopted | review |
| CM-07 | Delete dead code in the change that kills it and name the deletions in the description; never comment code out. | BUN Code style and idioms, PR process | adopted | review |
| CM-08 | Deprecations say how to migrate in the deprecation text. | GTS Deprecation | adopted | review |

## 5. Commits

Checked by `bun tools/check-commit.ts` (default style `area`, set by `code_style.commit_style` in standard.toml). Template: [../../templates/gitmessage](../../templates/gitmessage). Format reference: [commit-format.md](commit-format.md).

| ID | Rule | Source | Status | Enforced |
| :-- | :-- | :-- | :-- | :-- |
| CO-01 | Subject is `area: summary`, Bun's shape: the area is the noun of what changes (install, http, css, docs, ci, tls), lowercase unless it is an acronym or a proper name. `type(scope): summary` with the types feat, fix, docs, chore, refactor, perf, test, build, ci, revert is accepted (`--style=conventional` makes it the only form); a repository that feeds release notes from commit types says so in its AGENTS.md. | MEAS title row; BUN PR process; STANDARD section 4; GEP First Line; KPS Subject Line | adapted (C-15) | commit |
| CO-02 | The summary is imperative, specific, at most 72 characters in all, with no trailing period; it must stand alone in a history listing. "Fix bug", "Phase 1", "Update" are not summaries. | GEP First Line, Bad CL Descriptions; KPS Describe your changes; MEAS title row | adopted (C-16, C-26) | commit |
| CO-03 | A blank line separates subject and body. The body leads with the problem and its user-visible impact, then what changes and why this approach, then how it was verified; it is self-contained. | KPS Describe your changes; GEP Body is Informative; BUN PR process | adopted | commit, review |
| CO-04 | Quantify claims: a performance, memory or size improvement cites before and after numbers and names the non-obvious cost. | KPS Describe your changes; BUN Performance | adopted | review |
| CO-05 | Name the root cause and make the fixing line identifiable apart from refactoring. | BUN PR process | adopted | review |
| CO-06 | References: `Fixes #N` or `Closes #N` for the issue; for a regression, `Fixes: <12 hex> ("subject of the culprit")`. Cite a commit by 12 hex characters and its subject. | KPS Describe your changes, Using Reported-by and Fixes | adopted | commit |
| CO-07 | One logical change per commit; tests and docs travel with the code; every commit builds and passes, so history bisects. | KPS Separate your changes; GEP Small CLs; BUN One concern per PR | adopted | review |
| CO-08 | A breaking change has a `BREAKING CHANGE:` footer saying what to do instead; with `!` after the area or scope when the subject carries one. | STANDARD section 4 | adopted | commit |
| CO-09 | A revert uses type `revert`, quotes the reverted subject and states why. | STANDARD section 4; MEAS example 44891 | adapted | commit |
| CO-10 | No AI or tool attribution in new commits: no `Co-authored-by` for an assistant or bot, no `Generated by`, `Assisted-by`, robot emoji footers. Human co-authors are welcome. History already on a shared branch is not rewritten; the checker is run on new ranges only. | STANDARD section 4; KAI Guidelines; MEAS squash trailers | rejected (C-17, C-27) | commit |
| CO-11 | `Signed-off-by` appears only when the repository's README asks for a DCO, and only as a human's own sign-off, never an agent's. | KPS Sign your work; KAI Signed-off-by; CONTRIBUTING | adapted (C-18) | commit |
| CO-12 | Hard-wrap bodies at 72 columns in direct commits; a squash message built from a pull request body is Markdown and is not wrapped. | KPS The canonical patch format; STANDARD squash_merge_commit_message | adapted (C-19) | commit |
| CO-13 | Messages carry no secrets, no absolute paths of a developer machine, no private hostnames. | STANDARD CONTRIBUTING; owner privacy rule | adopted | commit |
| CO-14 | History on the default branch is linear; no force push, no deletion, no fixup or squash commits left behind. | STANDARD section 7 | adopted | commit, ci |

## 6. Pull requests

Template: [.github/pull_request_template.md](../../.github/pull_request_template.md).

| ID | Rule | Source | Status | Enforced |
| :-- | :-- | :-- | :-- | :-- |
| PR-01 | The title is the squash subject and follows CO-01 and CO-02; GitHub appends the number as `(#N)` after a space, which is not counted. | STANDARD squash_merge_commit_title; MEAS squash row | adopted | commit |
| PR-02 | One concern per pull request, narrow enough to review in one sitting; about 100 changed lines is comfortable and 1000 is usually too many; deleting a file counts as one line; generated or mechanical changes may be larger and say so. Fixing every instance of the same bug class is one concern. | GEP Small CLs; BUN One concern per PR, Fix the whole class | adopted (C-20) | review |
| PR-03 | The description has: what changed and why, how it was verified, risk, rollback, breaking changes. It is the permanent squash message, so keep it true after every rework. | STANDARD section 6; BUNTPL pull_request_template; GEP CL descriptions; BUN PR process | adapted | review |
| PR-04 | "How it was verified" lists the exact commands run and the observed result, and names what was not run. A change that could not be built or tested says so. | BUN CLAUDE Important Development Notes; KAI Procedure for finding and fixing bugs | adopted | review |
| PR-05 | Behavior changes show a before and after table or output for the cases that matter. | MEAS body rows (examples 45190, 46486) | adopted | review |
| PR-06 | Re-read the whole diff before asking for review; remove debug prints, scratch files, `.only`, formatter churn, lockfile and submodule ride-alongs, unrelated deletions; open as draft until done. | BUN PR process; CCR Commit Checklist | adopted | review |
| PR-07 | Green CI on every required job is a precondition for review; a failure on the branch but not on main is the author's to root-cause. | BUN PR process; CONTRIBUTING | adopted | ci |
| PR-08 | A bug found in review that the change does not touch is acknowledged, scoped and tracked in an issue; a bug of the exact class the change claims to fix is fixed in the same change. | BUN PR process | adopted | review |
| PR-09 | New behavior-changing options default off; renames keep deprecated aliases; the user-observable surface does not break silently. | BUN API design | adopted | review |
| PR-10 | Merge by squash or rebase only; the default branch stays linear. | STANDARD section 7 | adopted | ci |
| PR-11 | A change bound for an upstream follows that upstream's own contribution policy as read at submission time; nothing is written to an upstream without the owner's decision. | owner brief; KAI Guidelines | adopted | review |

## 7. Reviews

Checklist: [REVIEW.md](../../templates/REVIEW.md).

| ID | Rule | Source | Status | Enforced |
| :-- | :-- | :-- | :-- | :-- |
| RV-01 | Approve when the change definitely improves the health of the code base, even if it is not perfect; seek continuous improvement, not polish. A feature the owner does not want can still be refused. | GEP The Standard of Code Review | adopted (C-21) | review |
| RV-02 | Respond within one business day; agents respond in the same session. | GEP Speed of Code Reviews | adapted | review |
| RV-03 | Comment on the code, not the person; explain why; label severity: `Nit:` minor, `Optional:` or `Consider:` suggestion, `FYI:` information, unlabeled means required. | GEP How to write code review comments | adopted | review |
| RV-04 | Reply to every thread with evidence: `Fixed in <short sha>: <one line>`, `Added in <short sha>: ...`, or `Keeping X: <reason>`. A bare dismissal does not close a thread. | MEAS review rows (examples 46486, 45139, 45720); BUN PR process | adopted | review |
| RV-05 | Treat every suggestion, especially a bot's, as an unverified hypothesis: reproduce or check it, then apply or decline with checkable evidence; never bulk-apply and never bulk-resolve. | BUN PR process | adopted | review |
| RV-06 | When a reviewer flags a pattern once, sweep and fix every instance. | BUN PR process | adopted | review |
| RV-07 | Review against the checklist categories: design, tests, memory and unsafe, correctness of the whole bug class, error handling, public surface, performance claims, cross-platform, security, dependencies, docs. | BUN REVIEW; GEP What to look for in a code review; CUN How to use unsafe | adopted | review |
| RV-08 | Unsafe code, FFI and parsers of untrusted input get the extra scrutiny of a second pair of eyes familiar with the area. | CUN When to use unsafe; BUN Security | adopted | review |
| RV-09 | A reviewer may reject a change for size alone and ask for a series. | GEP Small CLs | adopted | review |
| RV-10 | Handle pushback by evidence, not authority: either side may change position when shown a fact; unresolved design disputes go to the owner. | GEP Handling pushback in code reviews | adopted | review |
| RV-11 | When several reviewers are asked, say what each is expected to check; a reviewer aims for actionable feedback several times per working day, an author for the same speed of reply. | CCR Code review policies, Expectations for all reviewers | adapted | review |

## 8. Machine-checkable subset

| What | File | Check |
| :-- | :-- | :-- |
| Bun formatter and linter | [templates/bunfig.standard.toml](../../templates/bunfig.standard.toml) | `bun fmt --check`, `bun lint` |
| rustfmt baseline and browser overlay | [templates/rustfmt.toml](../../templates/rustfmt.toml), [templates/rustfmt.browser.toml](../../templates/rustfmt.browser.toml) | `cargo fmt --check` |
| Cargo lints and clippy options | [templates/rust-lints.toml](../../templates/rust-lints.toml), [templates/clippy.toml](../../templates/clippy.toml) | `cargo clippy --all-targets -- -D warnings` |
| Commit and pull request title | [tools/check-commit.ts](../../tools/check-commit.ts) | `bun test tools` |
| This document | [tools/lint-code-style.ts](../../tools/lint-code-style.ts) | every rule has a known tag, a status and an existing conflict |
