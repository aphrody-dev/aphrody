# `open` as an automated benchmark

## Requested

1. **Inventories**
   - Rust frameworks equivalent to Flutter, Jetpack Compose and Angular; likewise for Next.js, shadcn/ui, Base UI, Better Auth and Tailwind CSS.
   - The full Aphrody monorepo: third party, first party and documentation.
2. **Tools installed globally on every host (PC and VPS)**: tree-sitter, Helix, Magika, markitdown, Obscura, Lightpanda, Crawlee, Apify, hyperfine (fork), and the rest of the browser and crawl inventory.
3. **Benchmark**: every tool tested on its own site, on design.google and on m3.material.io.
4. **One results store**: SQLite, Redis, JSON and MDX; resumable; multiple hosts.
5. **Network and storage**: Cloudflare tunnel, R2 bucket, Worker, Pages, DNS; driven through CDP; streamed results; hyperfine measurements.
6. **Browser ↔ UI, both ways and recursively**: from a sitemap, generate a pixel-perfect full-stack application in a framework; render it, compare with the original, iterate.

## Requirements

- R17. Inventories are produced by scripts in `open` from queryable sources (GitHub API, registries, manifests, locks), with the provenance query; no hand-kept list.
- R18. Tool installation is reproducible and idempotent on every host.
- R19. Every benchmark run records tool, version, host, target, measurements and artefacts; it resumes where it stopped.
- R20. The sitemap → application generation is judged by pixel and DOM difference against the reference rendering (Chrome for Testing).
- R21. A tool with no build for a platform (e.g. Lightpanda on Windows): fork, patch, push, release, then distribute through `ay` and the Aphrody download with self-update. A tool that does not exist: create it following the practices of frameworks, the big clouds, the stores and the package managers.

## First R21 case: Lightpanda on Windows

Upstream PR lightpanda-io/browser#3753 ("native Windows support", 26 files, +1436/-108, open) fills the `WindowsIO` left empty in `Server.zig` and adapts `build.zig` (MSVC, Windows SDK, system libraries). Plan: apply it on `aphrody-labs/browser`, build, test, publish a Windows release, add it to `scripts/hosts/tools.json`.

## Added phases

| Phase | Deliverable |
|---|---|
| 7 | Inventories (frameworks, Aphrody monorepo) |
| 8 | Tools installed on the PC and the VPS, verified |
| 9 | Benchmark: shared results store, resumable, multi-host, Cloudflare |
| 10 | Browser ↔ UI loop: generation from a sitemap, pixel comparison, iteration |

## Open decisions

D15 (results store), D16 (target frameworks of the generation), D17 (where the benchmark runs): see [decisions.md](decisions.md).
