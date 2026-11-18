# Aphrody Consolidated Cloudflare Worker & R2 Edge Distribution

All Aphrody edge surfaces (releases, docs markdown proxy, content-addressed R2 assets, and GitHub webhook queue ingestion) are consolidated into **one** Worker in `.cloudflare/aphrody-worker/` deployed via `bun tooling/bun/cloudflare-deploy.ts` (and CI `.github/workflows/aphrody-release.yml`).

## Unified Worker (`.cloudflare/aphrody-worker/`)

- **`releases`** (`releases.aphrody.com/*`, `downloads.aphrody.com/*`, `aphrody.com/download*`): Zero-egress binary portal, `/install.sh`, `/install.ps1`, `/api/releases/:channel/latest`, `/api/channels`, `/assets/*` backed by R2 bucket `aphrody-releases`.
- **`docs`** (`docs.aphrody.com/*`): Markdown (`Accept: text/markdown`) + HTML documentation proxy, `/llms.txt`, `/sitemap.xml`.
- **`assets`** (`https://assets.aphrody.app` R2 Custom Domain + `/r2-assets/*`): Immutable `v1/*` design/media/font blobs backed by R2 bucket `aphrody-assets` and indexed in `assets/os/r2_assets_manifest.toml`.
- **`github`** (`aphrody.com/github`): HMAC SHA-256 verified webhook intake publishing to queue `github-events`.

## Local Dev, R2 Assets & Deploy

```sh
bun run r2:manifest
bun run r2:push
bun run cf:dev
bun run cf:audit
bun run cf:deploy
```
