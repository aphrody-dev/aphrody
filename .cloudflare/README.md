# Aphrody Cloudflare Workers & R2 Edge Distribution

All Workers deploy to the `aphrody-labs` Cloudflare account via `bun tooling/bun/cloudflare-deploy.ts` (and CI `.github/workflows/aphrody-release.yml`).

## Workers

- `aphrody-releases`: Serves `releases.aphrody.com/*`, `downloads.aphrody.com/*`, and `aphrody.com/download*` backed by R2 bucket `aphrody-releases` (`/download`, `/install.sh`, `/install.ps1`, `/api/releases/:channel/latest`, `/api/channels`, `/assets/*`, `/healthz`).
- `docs-proxy`: Markdown/HTML documentation proxy worker (`docs.aphrody.com` fallback/mirror).
- `open-source-website-assets`: R2 asset passthrough backed by `aphrody-assets`.

## Local Dev & Deploy

```sh
bun run cf:dev:releases
bun run cf:audit
bun run cf:deploy
```
