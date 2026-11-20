set dotenv-load

just := just_executable()

# List available workspace recipes
default:
    @{{ just }} --list

# Verify repository hygiene, starter-pack parity matrix, and Bun tooling tests
check: hygiene standards tooling-test

# Enforce repository hygiene and README guard
hygiene:
    bun tooling/bun/repo-hygiene.ts

# Regenerate starter-pack & fork parity matrix
standards:
    bun tooling/bun/prod-standards-starterpack.ts all

# Run Bun tooling test suite
tooling-test:
    bun run test:tooling

# Build the WebGPU OS (ai-os + cloudflare-os + nysh + Chromium Views GPUI) Wasm target
web-os:
    bun tooling/bun/web-build.ts build --package aphrody_desktop --opt

# Sync workspace to the remote build host
sync-host:
    bun tooling/bun/fleet-sync.ts sync

# Dry-run Cloudflare Workers & Pages deployment audit
deploy-check:
    bun tooling/bun/cloudflare-deploy.ts audit --dry-run

# Clean ephemeral WebGPU proof and store distribution directories
clean:
    bun -e "import { rmSync } from 'node:fs'; for (const p of ['target/gpui-store-dist', 'target/wasm-wgpu-proof']) rmSync(p, { recursive: true, force: true });"
