set dotenv-load

just := just_executable()

# List available Pop!_OS / COSMIC-style workspace recipes
default:
    @{{ just }} --list

# Verify repository hygiene, submodules, starter-pack parity matrix, and Bun tooling tests
check: hygiene submodules-check standards tooling-test

# Enforce repository hygiene and README guard
hygiene:
    bun tooling/bun/repo-hygiene.ts

# Check that all declared vendor submodules are initialized and clean
submodules-check:
    bun tooling/bun/submodules.ts check

# Sync Cargo-referenced submodules in parallel
submodules-sync jobs="8":
    bun tooling/bun/submodules.ts sync --jobs {{ jobs }}

# Regenerate starter-pack & submodule parity matrix
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
