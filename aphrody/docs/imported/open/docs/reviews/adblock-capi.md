# Import review: crates/adblock-capi

Unit moved from the Aphrody monorepo (5 commits replayed across its two former paths, merge ace8ad2e7), then checked against decision D25.

| Check | Finding | Outcome |
|---|---|---|
| Provenance | every replayed commit carries `Moved from aphrody-labs/aphrody <commit>` | kept |
| Verification | `cargo check` and `cargo test -p aphrody-adblock-capi` in the `open` workspace: 3 tests pass, no warning | kept |
| Purification | test URLs on the project's production site; lint comments that belonged to unrelated crates of the source workspace | tests use the reserved `example.com` domain (RFC 2606); comments removed; private-data gate clean |
| Purge | none: the crate is a thin glue over Brave's `adblock` crate and its C ABI | kept whole |
| Reorganisation | member of `open`'s Cargo workspace under `crates/`; `[workspace.package]` and `[workspace.dependencies]` added for it | kept |
| Naming (D23) | crate `aphrody-adblock-capi`, cdylib `aphrody_adblock`, symbols `aphrody_adblock_*` | kept: the `aphrody_` prefix is the namespace of the shared C ABI it implements (`aphrody-core-capi`, D23 rule 3); the crate name is the registry coordinate of that ABI family |
