# Import review: packages/chromium-bun into packages/browser-engines and packages/crawl

Unit moved from the Aphrody monorepo (37 commits replayed, merge 2ac05afaf), then checked against decision D25.

| Check | Finding | Outcome |
|---|---|---|
| Provenance | every replayed commit carries `Moved from aphrody-labs/aphrody <commit>` | kept |
| Verification | 59 tests passed in `open` before the review, against a topology file of the source repository | replaced: 13 tests in `browser-engines`, 4 in `crawl` (1 end-to-end test skipped without a browser), `bun check` clean in both |
| Purification | host names of the source fleet in the engine registry (build hosts), its private mesh subnet in the search code, local paths in a generated report, machine-specific measurements and French text in the headless profile, the source repository's `third_party/` layout and `SRC_ROOT` as defaults, pinned installer versions | registry entries name their upstream URL only; a source build reads its output directory from `BROWSER_ENGINES_CHROMIUM_OUT`; installers run at their latest version; browser cache in the platform cache directory under `BROWSER_ENGINES_CACHE`; profile rewritten in English without host facts; private-data gate clean |
| Purge | modules specific to the source repository or to one deployment: a search cluster over three named hosts, a product subdomain catalog with seed documents, a grounding verifier, an end-to-end runner against one production site, a Chromium tree auditor and purge matrix of the source repository, a "Sovereign layers" description and a benchmark "winner picker" built on it, a WebUI proof of concept, a fork list, the upstream release trackers that read and write the source repository's `third_party/` tree | removed from `open`; the trackers belong to the repository whose tree they maintain |
| Reorganisation | one package mixed engines, CDP, crawling, search, benchmarks, audits and QA | `packages/browser-engines` (engine registry, install, launch, backend choice, CDP client, launcher, headless profile, content shields, SSRF guard) and `packages/crawl` (crawler, search shards on bun:sqlite FTS5 with BM25, network probe) |
| Naming (D23) | `ChromiumBunCdpClient`, `ChromiumBunCrawler`, `SovereignSearchShardDb`, a hash named BLAKE3 that computed BLAKE2b, brand user agent `AphrodyBot` with a site URL, `APHRODY_*` variables | `CdpClient`, `Crawler`, `SearchShard`, `contentHash` (BLAKE2b, as computed); the crawler's user agent is required from the caller; variables `BROWSER_ENGINES_*`, `CRAWL_*` |

Interface changes for consumers:

| Before | After |
|---|---|
| `packages/chromium-bun/src/cli.ts engines ...` | `packages/browser-engines/src/cli.ts engines ...` (bin `browser-engines`) |
| `chromium-bun trackers`, `search`, `cluster`, `crawl`, `bench`, `inspect`, `audit` | removed (see Purge) |
| `@aphrody/chromium-bun` exports | `@aphrody/browser-engines` (engines, CDP, shields, SSRF), `@aphrody/crawl` (crawler, shards, net probe) |
| `CHROMIUM_PATH` for a source build | `BROWSER_ENGINES_CHROMIUM_OUT` (directory holding the built binary) |
| `APHRODY_BROWSERS` | `BROWSER_ENGINES_CACHE` |
| `APHRODY_NET_LIB`, `APHRODY_CDP_URL` | `CRAWL_NET_PROBE_LIB`, `CRAWL_CDP_URL` |
| `CHROMIUM_BUN_TOPOLOGY`, `SRC_ROOT`, `CHROMIUM_ROOT`, `DEPOT_TOOLS_ROOT` | not read any more |
