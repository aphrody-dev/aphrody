## Summary

What problem does this solve, how, and why does this change matter?
Use `Closes #123` for each issue resolved so GitHub closes and links it automatically.
Include artifacts produced when testing (before/after screenshots, GIFs, videos, CLI JSON reports, benchmarks).

## Testing

Manually test your changes; report results, mention platforms tested, and include setup/reproduction steps for reviewers.

- [ ] `cargo xtask brand check`
- [ ] `cargo check -p gpui_ui --features nsh-full` / `cargo check -p gpui_code --no-default-features`
- [ ] `./script/clippy` (or `pwsh ./script/clippy.ps1`) / `cargo fmt --all -- --check`
- [ ] `bun run hygiene && bun run test:tooling`
- [ ] Backend / shader / UI tests or screenshots verified

## Self-review

- [ ] I've reviewed my diff for quality, security, reliability, and performance.
- [ ] Diff follows [`AGENTS.md`](https://github.com/aphrody-dev/aphrody/blob/main/AGENTS.md) (no `mod.rs`, no `.unwrap()` on fallible calls, Clippy clean, `README.md` guard respected).
- [ ] UI changes follow [`docs/src/development/ui-checklist.md`](https://github.com/aphrody-dev/aphrody/blob/main/docs/src/development/ui-checklist.md).
- [ ] Tests cover the new or changed behavior.

Release Notes:

- N/A or Added/Fixed/Improved ... (if modifying `crates/gpui*`, include `- [GPUI] ...`)
