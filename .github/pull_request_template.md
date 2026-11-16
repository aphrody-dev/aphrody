## Summary

What problem does this solve, how, and why does this change matter?
Use `Closes #123` for each issue resolved so GitHub closes and links it automatically.
Include artifacts produced when testing (screenshots, CLI JSON reports, benchmarks).

## Testing

- [ ] `cargo xtask brand check`
- [ ] `cargo check -p gpui_ui --features nsh-full` / `cargo check -p gpui_code --no-default-features`
- [ ] Backend / shader / UI tests or screenshots verified

## Self-review

- [ ] Diff follows `.rules` (no `mod.rs`, no `.unwrap()` on fallible calls, Clippy clean)
- [ ] UI changes follow [docs/src/development/ui-checklist.md](./docs/src/development/ui-checklist.md)
- [ ] Tests cover the new or changed behavior

Release Notes:

- N/A or Added/Fixed/Improved ...
