# Aphrody Vendor, Third-Party Supply-Chain, and WireGuard LAN Plan

Single source of truth for Zed contributor conformity, split licensing, vendor/third-party submodule parity, third-party supply-chain policy, and the WireGuard LAN build/verification fleet.

## 1. Zed Contributor Guide & Workspace Conformity

- Repository guidelines follow [CONTRIBUTING.md](../../CONTRIBUTING.md), root [AGENTS.md](../../AGENTS.md), and [docs/AGENTS.md](../../docs/AGENTS.md).
- Workspace consistency is enforced by [package_conformity.rs](../../tooling/xtask/src/tasks/package_conformity.rs) (`cargo xtask package-conformity`) and [licenses.rs](../../tooling/xtask/src/tasks/licenses.rs) (`cargo xtask licenses`):
  - Internal crates inherit dependencies from `[workspace.dependencies]` in [Cargo.toml](../../Cargo.toml).
  - Tooling crate `perf` is explicitly permitted to retain its standalone stricter Clippy table.
  - Every crate in the workspace owns a mode `120000` git symlink (`LICENSE-APACHE` or `LICENSE-GPL`) matching Zed's split Apache-2.0 / GPL-3.0-or-later license boundary, plus explicit manifest `license = "..."` fields for reproducible `cargo-deny` validation across Linux and Windows (`core.symlinks = false`).

## 2. Third-Party Supply-Chain Policy (`deny.toml`)

- [deny.toml](../../deny.toml) enforces:
  - Approved SPDX license allowlist (including `Apache-2.0`, `MIT`, `BSD-2-Clause`, `BSD-3-Clause`, `GPL-3.0-or-later`, `ISC`, `MPL-2.0`, `Zlib`, `Unicode-3.0`, `bzip2-1.0.6`, `WTFPL`).
  - `unknown-registry = "deny"` and `unknown-git = "deny"`, with an explicit allowlist matching every pinned git source in `Cargo.lock`.
- Verification command: `cargo deny check licenses sources --hide-inclusion-graph`.

## 3. Vendor & Nested Submodule Remediation

- Top-level `.gitmodules` ([.gitmodules](../../.gitmodules)) declares 38 submodules (20 referenced directly by `Cargo.toml`, plus runtime/OS tooling submodules including `vendor/bun` and `vendor/ayvpn` tracking `branch = main`).
- Nested submodules required by vendored Cargo manifests (`vendor/libcosmic/iced` and `vendor/ayvpn/mullvad/mullvadvpn-app`) are automatically discovered via `listNestedReferencedPaths` in [submodules.ts](../../tooling/bun/submodules.ts) and initialized across:
  - CI action: [.github/actions/vendor-submodules/action.yml](../../.github/actions/vendor-submodules/action.yml)
  - Remote Linux hosts: [fleet-sync.ts](../../tooling/bun/fleet-sync.ts)
  - `[patch.crates-io]` overrides in [Cargo.toml](../../Cargo.toml) (`taffy`, `cosmic-text`, `wgpu*`, `naga`, `accesskit*`, plus Zed's upstream patches).
- **WGPU Pinning Decision (`vendor/wgpu`)**: `vendor/wgpu` remains unified on `wgpu 30.0.0` / `naga 30.0.0` with Aphrody's custom Metal patch so `gpui_wgpu`, `aphrody_cli`, `nsh-*`, and `vendor/vello` resolve against a single `wgpu`/`naga` graph without dual-version conflicts vs `gfx-rs/wgpu` `v30.0.2` / `trunk`.

## 4. WireGuard LAN Fleet & `aphrody_vpn` Local Network Sharing

- Heavy Linux workspace builds and integration tests offload from the Windows workstation to the WireGuard `wg1` mesh (`10.200.77.0/24` across `fr1`, `fr2`, and `prd`) via SSH aliases in [fleet-sync.ts](../../tooling/bun/fleet-sync.ts).
- Local-network split tunneling inside `crates/aphrody_vpn` is wired through `Toggle::AllowLan` in [backend.rs](../../crates/aphrody_vpn/src/backend.rs) and enforced in [daemon_backend.rs](../../crates/aphrody_vpn/src/daemon_backend.rs) so local WireGuard/LAN traffic stays reachable alongside the tunnel.
- Automated full-stack audit: [vendor-lan-doctor.ts](../../tooling/bun/vendor-lan-doctor.ts) (`bun run vendor:doctor`).
