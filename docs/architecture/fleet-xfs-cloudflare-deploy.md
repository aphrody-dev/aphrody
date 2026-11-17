# Parallel Multi-VPS XFS Build, CI/CD & Cloudflare Edge Deployment

## Fleet Topology

| Alias | OS / Kernel | Resources | Storage / CoW | Role |
| :--- | :--- | :--- | :--- | :--- |
| `fr1` | Ubuntu 26.10 (`7.3.0-10-generic`) | 12c / 45.9 GiB | `/srv/xbuild` XFS LVM (`reflink=1`), NVMe `sccache` | Primary XFS/NVMe builder (`aphrody-os`, `aphrody-cli`, `gpui-wgpu`, `zed`), `nativelink-server`, GitHub Actions Runner (`build.slice`) |
| `fr2` | Ubuntu 26.10 (`7.3.0-10-generic`) | 6c / 11.4 GiB | `/srv/xbuild` XFS (`reflink=1`) | Secondary parallel verifier (Wasm32, WASI P2, Bun tooling tests, `nativelink-worker`) |
| `prd` | Arch Linux (`7.2.14-arch1-1`) | 8c / 31.3 GiB | Btrfs CoW (`reflink`, ~232 GiB free) | Production edge proxy (`nginx`, Cloudflare Tunnels, Docker stacks, `/opt/aphrody/bin` live edge deploy) |

## Parallel Build & Deploy Commands

```sh
# Optimize all VPS builders (KVM udev rotational=0, Kernel sysctl, sccache, CoW scratch dirs)
bun run host:audit:all
bun run host:apply:all

# Sync workspace to all builders concurrently & run sharded parallel builds
bun run fleet:sync:all
bun run fleet:parallel

# Instant O(1) XFS/Btrfs CoW snapshot for concurrent CI jobs
bun tooling/bun/fleet-sync.ts xfs-clone --host fr1 --job ci-pr-123

# End-to-end release build + multi-VPS edge deploy + Cloudflare R2/Worker deploy
bun run cf:audit
bun run fleet:deploy
bun tooling/bun/cloudflare-deploy.ts smoke
```

## Cloudflare Workers & R2

- [`cloudflare-deploy.ts`](../../tooling/bun/cloudflare-deploy.ts) manages R2 bucket `aphrody-releases`, proxied DNS `releases.aphrody.com`, installer scripts ([`aphrody-install.sh`](../../script/aphrody-install.sh), [`aphrody-install.ps1`](../../script/aphrody-install.ps1)), channel JSON manifests, and Worker deployments under [`.cloudflare/`](../../.cloudflare/README.md).
- [`aphrody-ci.yml`](../../.github/workflows/aphrody-ci.yml) validates `cloudflare-and-framework` alongside Linux/Windows/Wasm CI matrices, and [`aphrody-release.yml`](../../.github/workflows/aphrody-release.yml) publishes signed artifacts and updates R2 + Cloudflare Workers automatically.
