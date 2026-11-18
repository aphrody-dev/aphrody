# Fleet, Edge & Asset Orchestration Architecture

Reproducible multi-VPS build scheduling, Cloudflare edge distribution, R2 asset offloading, `vendor/browser/third_party` pinning, and GitHub workflow automation for `aphrody`. All configuration resolves from repository manifests, `~/.ssh/config`, and environment variables without hardcoded IP addresses, usernames, account IDs, or machine-local paths.

---

## 1. Declarative Fleet Manifest & Dynamic Load Balancing

[`assets/os/fleet_cluster.toml`](../../../assets/os/fleet_cluster.toml) defines the cluster contract consumed by [`tooling/bun/fleet-lb.ts`](../../../tooling/bun/fleet-lb.ts) (exposed as `bun run fleet:lb` in [`package.json`](../../../package.json)) and verified by [`tooling/bun/fleet-lb.test.ts`](../../../tooling/bun/fleet-lb.test.ts).

### Node Discovery & Capabilities

- **SSH Discovery**: [`fleet-lb.ts`](../../../tooling/bun/fleet-lb.ts) parses `~/.ssh/config` (overridable via `APHRODY_SSH_CONFIG` or `--ssh-config`) while ignoring wildcard/git entries (`*`, `git`, `github.com`, `*.github.com`, `localhost`), matching `ssh_aliases` declared in [`fleet_cluster.toml`](../../../assets/os/fleet_cluster.toml), and honoring `APHRODY_FLEET_HOSTS`. When SSH config is absent or `--local` is passed, local host telemetry is included via `include_local_fallback = true`.
- **Node Tiers & Capabilities**:

| Node | Region | Tier | Work Dir | Capabilities |
| :--- | :--- | :--- | :--- | :--- |
| `fr1` | `eu-west` | `builder` | `/srv/xbuild` | `xfs_reflink`, `native_rust`, `wasm` |
| `fr2` | `eu-west` | `verifier` | `/srv/xbuild` | `xfs_reflink`, `wasm`, `native_rust` |
| `prd` | `us-edge` | `edge` | `/srv/xbuild` | `btrfs_reflink`, `edge_proxy`, `wasm` |

- **Host OS Profiles**: OS-level kernel sysctl, KVM VirtIO block udev rules, `sccache`, and XFS/Btrfs storage tiering live in [`assets/os/ci_host_profiles.toml`](../../../assets/os/ci_host_profiles.toml) and [`fleet-sync.ts`](../../../tooling/bun/fleet-sync.ts) (see [Multi-VPS XFS & Cloudflare Deploy](./fleet-xfs-cloudflare-deploy.md)).

### Live Probing, Scoring & Work-Stealing Dispatch

- **Async Probing (`probe`)**: Runs `ssh -o BatchMode=yes` with `tr -d '\r' | bash -s` across remote Linux hosts to read `/proc/loadavg`, `/proc/meminfo`, `nproc`, `df -Pk`, filesystem reflink support, and active Cargo/runner processes. Local Windows, macOS, and Linux hosts are sampled with `node:os` (`loadavg`, `cpus`, `freemem`) and `statfsSync`.
- **Dynamic Score (`[0, 100]`)**: Configured under `[scoring]` in [`fleet_cluster.toml`](../../../assets/os/fleet_cluster.toml) using `load_ratio` (`load1 / cpus`, weight `42.0`, cap `1.5`), available memory ratio (weight `25.0`), work-disk availability (weight `18.0`, floor `min_work_avail_gb = 25.0`), active jobs (`8.0` per job), probe RTT (`7.0` scaled by `3000 ms`), plus `role_affinity_bonus` (`10.0`), `region_affinity_bonus` (`4.0`), and per-node `priority_bias`. Nodes exceeding `max_load_per_cpu = 1.35` or below `min_free_mem_gb = 2.0` / `min_free_disk_gb = 10.0` are marked saturated/unhealthy.
- **Job Profiles (`schedule`)**: Declares required/preferred capabilities, `default_shards`, and minimum memory/disk thresholds for `native_rust`, `wasm`, `edge_proxy`, `verify`, and `release`. Shards spread across eligible ranked candidates with fallback chains.
- **Work-Stealing Dispatch (`dispatch`)**: Executes commands on the highest-scoring eligible host and automatically retries against the next host in the fallback chain up to `work_stealing_max_retries = 2` if an attempt fails (`stolenFrom` recorded per attempt).

```sh
bun run fleet:lb probe --all --json
bun run fleet:lb schedule --job native_rust --shards 2 --json
bun run fleet:lb dispatch --job verify --cmd "cargo test -p brand" --json
bun run fleet:lb beacon --all --dry-run --json
```

---

## 2. Consolidated Cloudflare Worker & Fleet API

All Cloudflare edge surfaces are consolidated into a single Worker at [`.cloudflare/aphrody-worker/`](../../../.cloudflare/README.md) ([`wrangler.toml`](../../../.cloudflare/aphrody-worker/wrangler.toml)), eliminating legacy split workers (`aphrody-releases`, `docs-proxy`, `open-source-website-assets`, `aphrody-github`).

| Surface / Route | Handler | Backing Binding |
| :--- | :--- | :--- |
| `releases.aphrody.com/*`, `downloads.aphrody.com/*`, `aphrody.com/download*` | [`releases.ts`](../../../.cloudflare/aphrody-worker/src/releases.ts) | `RELEASES_BUCKET` (`aphrody-releases` R2) |
| `docs.aphrody.com/*`, `/llms.txt`, `/docs*` (`Accept: text/markdown`) | [`docs.ts`](../../../.cloudflare/aphrody-worker/src/docs.ts) | Optional `DOCS_ORIGIN_URL` + built-in fallback |
| `assets.aphrody.app/*`, `/r2-assets/*` | [`assets.ts`](../../../.cloudflare/aphrody-worker/src/assets.ts) | `ASSETS_BUCKET` (`aphrody-assets` R2, immutable `v1/*` cache) |
| `aphrody.com/github` | [`github.ts`](../../../.cloudflare/aphrody-worker/src/github.ts) | HMAC SHA-256 verification -> `EVENTS` (`github-events` Queue) |
| `/healthz`, `/health`, `/api/health` | [`index.ts`](../../../.cloudflare/aphrody-worker/src/index.ts) | Worker health + channel/bucket metadata |
| `/api/fleet/status`, `/api/fleet/route`, `/api/fleet/beacon` | [`fleet.ts`](../../../.cloudflare/aphrody-worker/src/fleet.ts) | R2 `fleet/telemetry.json` (`ASSETS_BUCKET` / `RELEASES_BUCKET`) |

### Edge Fleet Telemetry & Routing Endpoints

- **`POST /api/fleet/beacon`**: Accepts single-node or multi-node telemetry snapshots (full replacement or `merge: true` upsert) published by `fleet-lb beacon`. When `FLEET_BEACON_TOKEN` is configured in [`types.ts`](../../../.cloudflare/aphrody-worker/src/types.ts), requests authenticate via `Authorization: Bearer <token>` or HMAC-SHA256 `X-Fleet-Signature-256` (`sha256=<hex>`) using constant-time comparison, and persist state to R2 key `fleet/telemetry.json`.
- **`GET /api/fleet/status`**: Returns normalized node telemetry with per-node `stale` flags computed against `staleTtlSec` (default `300s`), returning a deterministic empty snapshot (`source: "default"`) when R2 telemetry is absent.
- **`GET|POST /api/fleet/route`**: Filters reachable, healthy, non-stale nodes by `job` (`native_rust`, `wasm`, `edge_proxy`, `verify`, `release`, `any`), explicit `capabilities`, `region`, and `exclude`, then returns the `selected` node, `fallbackChain`, ranked `candidates`, and balanced `shards`.

Audit and dry-run verification run via [`tooling/bun/cloudflare-deploy.ts`](../../../tooling/bun/cloudflare-deploy.ts):

```sh
bun run cf:audit
bun test tooling/bun/fleet-lb.test.ts ./.cloudflare/aphrody-worker/src/index.test.ts tooling/bun/fleet-deploy.test.ts
```

---

## 3. Content-Addressed R2 Asset Offloading & Local Disk Hygiene

Heavy non-boot binary assets (reference PNG captures, showcase GIFs/WebPs, sound WAVs, optional fonts) are decoupled from Git checkouts via [`tooling/bun/r2-assets.ts`](../../../tooling/bun/r2-assets.ts) and [`assets/os/r2_assets_manifest.toml`](../../../assets/os/r2_assets_manifest.toml):

- **Content-Addressed Keys**: Each asset is hashed (`sha256`) into `v1/<category>/<sha256>.<ext>` inside R2 bucket `aphrody-assets` and served at `https://assets.aphrody.app/<r2_key>` (and Worker `/r2-assets/<r2_key>`) with `Cache-Control: public, max-age=31536000, immutable` and `ETag` `304` handling.
- **Boot Asset Guard (`keep_local_boot`)**: Compile-time `include_bytes!`, `cargo xtask brand check` assets (`assets/images/*`), and bundled core fonts (`assets/fonts/lilex/`, `ibm-plex-sans/`, `GoogleSansCode-Variable.ttf`, `JetBrainsMono-Regular.ttf`, `crates/gpui/examples/image/app-icon.png`) stay tracked on disk (`2,917,160` bytes kept local; `9,115,541` bytes across offloadable entries in the 50-asset manifest).
- **On-Demand Hydration & Local Slimming**: `bun run r2:hydrate` verifies SHA-256 digests into `target/r2-cache/` (or `--working-tree`); `bun run r2:prune` and [`tooling/bun/disk-slim.ts`](../../../tooling/bun/disk-slim.ts) (`bun run disk:slim`) reclaim `target/*` build caches and multi-pack submodule `.git` history safely.

---

## 4. Slimmed `vendor/browser/third_party` Pinning

Inside the `vendor/browser` submodule ([`vendor/browser/third_party/README.md`](../../../vendor/browser/third_party/README.md)), third-party trees are decoupled from checked-in copies:

- **Zero-Byte Gitlink Mountpoints**: `third_party/browse`, `third_party/bun`, `third_party/kitesurf`, and `third_party/mullvad` are tracked as gitlinks (`160000`) initialized on demand rather than vendored source duplicates.
- **Live Upstream Revision Pinning (`mode = "pin"`)**: [`vendor/browser/tools/upstreams.toml`](../../../vendor/browser/tools/upstreams.toml) and [`vendor/browser/tools/upstreams.lock.json`](../../../vendor/browser/tools/upstreams.lock.json) record exact branch/tag SHAs (with `crates = {}`) resolved via `git ls-remote` across Servo crates, `stylo`, `webrender`, `mozjs`, `html5ever`, `obscura`, `agent-browser`, `lol-html`, `hyperfine`, and `JetStream`. Heavy Chromium checkouts stay external on build hosts via `CHROMIUM_SRC` and `DEPS`.

---

## 5. GitHub Orchestrator & Release Automation

- **[`.github/workflows/aphrody-fleet-orchestrator.yml`](../../../.github/workflows/aphrody-fleet-orchestrator.yml)**: Triggers on `workflow_dispatch` (`audit`, `probe`, `schedule`, `beacon`, `rebalance`), 15-minute `schedule`, `push` to `main` touching fleet/Worker files, and `repository_dispatch` (`fleet_rebalance`, `push_main`, `cloudflare_webhook`). Validates [`fleet_cluster.toml`](../../../assets/os/fleet_cluster.toml) across `ubuntu-latest`, `windows-latest`, and `macos-latest`, runs Bun fleet and Worker test suites, verifies `fleet:lb` CLI subcommands, audits `.cloudflare/aphrody-worker`, and performs secret-gated SSH probing, `/api/fleet/beacon` updates, and Worker deploys when `FLEET_SSH_CONFIG_B64`, `FLEET_SSH_KEY`, `FLEET_BEACON_TOKEN`, and Cloudflare credentials are configured.
- **Companion Workflows**: [`aphrody-ci.yml`](../../../.github/workflows/aphrody-ci.yml) runs `cloudflare-and-framework` checks alongside Linux, Windows, and WebGPU/Wasm CI jobs; [`aphrody-release.yml`](../../../.github/workflows/aphrody-release.yml) publishes signed multi-platform binaries with Sigstore attestations, mirrors channel manifests to R2 `aphrody-releases`, and deploys `.cloudflare/aphrody-worker`.

See [Architecture Index](./README.md) for all sibling subsystem guides.
