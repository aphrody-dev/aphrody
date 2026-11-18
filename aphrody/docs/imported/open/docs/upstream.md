# Upstream

Generated from the workspace manifest (pins) and from `open.lock.json` as it stood before the restructure (ledger).
Nothing below is vendored: pins resolve through cargo from crates.io or git. The removed copies and their lock (`open.lock.json`) are in the history before commit `470c0eff1d` (`git show 470c0eff1d^:open.lock.json`).

## Pins (live: `bun scripts/upstream.ts`)

<!-- pins:begin -->
| Crates | Source | Pin | Head |
| :-- | :-- | :-- | :-- |
| browse-capture, browse-core, browse-page, browse-relay, browse-script | anomalyco/browse | `7a277ed8f9` | at head |
| opentui | anomalyco/opentui | `e477ffa3ab` | at head |
| rune, rune-kit, rune-motion, winit, wgpu-hal, parley_engine, accesskit_windows, accesskit_macos | anomalyco/rune | `631e8a67cb` | at head |
| adblock | crates.io | `0.13.5` | |
| opentunnel | crates.io | `0.6.2` | |
<!-- pins:end -->

## Ledger of the copies removed from the tree (100 entries)

| Name | Upstream | Last imported rev | Disposition |
| :-- | :-- | :-- | :-- |
| agents-benchmark | anomalyco/agents-benchmark | `ad9e4ba43f` | purged: upstream project, not a part of the framework |
| astro-sst | anomalyco/astro-sst | `4b86ab38d1` | purged: upstream project, not a part of the framework |
| bank | anomalyco/bank | `99f6ce1ae1` | purged: upstream project, not a part of the framework |
| bellhop | anomalyco/bellhop | `ace0d77086` | purged: upstream project, not a part of the framework |
| blackout | anomalyco/blackout | `569ed3a962` | purged: upstream project, not a part of the framework |
| browse | anomalyco/browse | `e3a45fd944` | replaced: `browse-*` crates (git pin) drive the browser; `extension/` is the ported MV3 extension |
| browser-control | anomalyco/browser-control | `9ea31dc454` | purged: superseded by browse (relay + extension) |
| bun-ffi-structs | anomalyco/bun-ffi-structs | `b0bcb13536` | purged: upstream project, not a part of the framework |
| bun-yoga | anomalyco/bun-yoga | `8fafeeeec0` | purged: upstream project, not a part of the framework |
| clip | anomalyco/clip | `c7e1d9b1bd` | purged: upstream project, not a part of the framework |
| computer-use | anomalyco/computer-use | `3889692301` | purged: upstream project, not a part of the framework |
| deadbolt | anomalyco/deadbolt | `11a56948b4` | purged: upstream project, not a part of the framework |
| demo-ai-app | anomalyco/demo-ai-app | `94b4dfdea1` | purged: upstream project, not a part of the framework |
| demo-notes-app | anomalyco/demo-notes-app | `308c0312a0` | purged: upstream project, not a part of the framework |
| effect-http-recorder | anomalyco/effect-http-recorder | `89e1b85f7c` | purged: upstream project, not a part of the framework |
| extension | anomalyco/extension | `f6edc2ac4e` | purged: upstream project, not a part of the framework |
| fiend | anomalyco/fiend | `263ff7db3c` | purged: upstream project, not a part of the framework |
| flicks | anomalyco/flicks | `8f64cea7a8` | purged: upstream project, not a part of the framework |
| freenextjsforjim.biz | anomalyco/freenextjsforjim.biz | `85141c7c1f` | purged: upstream project, not a part of the framework |
| ghostty-web | anomalyco/ghostty-web | `6e24d04045` | purged: upstream project, not a part of the framework |
| grim | anomalyco/grim | `9a18ccea80` | purged: upstream project, not a part of the framework |
| guide | anomalyco/guide | `273b6c75d5` | purged: upstream project, not a part of the framework |
| harbor | anomalyco/harbor | `f03db62fd2` | purged: upstream project, not a part of the framework |
| haunt | anomalyco/haunt | `e5ff494f48` | purged: upstream project, not a part of the framework |
| hex | anomalyco/hex | `31feb6f0e8` | purged: upstream project, not a part of the framework |
| hokum | anomalyco/hokum | `e2e31b1467` | purged: upstream project, not a part of the framework |
| homebrew-tap | anomalyco/homebrew-tap | `bace1e2c4d` | purged: upstream project, not a part of the framework |
| ideal-stack-preview | anomalyco/ideal-stack-preview | `8b4542ac93` | purged: upstream project, not a part of the framework |
| identity | anomalyco/identity | `a140a9672b` | purged: upstream project, not a part of the framework |
| ion | anomalyco/ion | `d5c4855b26` | purged: upstream project, not a part of the framework |
| isocast | anomalyco/isocast | `dd2d1038f1` | purged: upstream project, not a part of the framework |
| isserverlessready | anomalyco/isserverlessready | `b461c5b2ce` | purged: upstream project, not a part of the framework |
| jobmail | anomalyco/jobmail | `5866c459ef` | purged: upstream project, not a part of the framework |
| justice | anomalyco/justice | `86fdb29686` | purged: upstream project, not a part of the framework |
| kysely-data-api | anomalyco/kysely-data-api | `c78be516f1` | purged: upstream project, not a part of the framework |
| lerna-yarn-starter | anomalyco/lerna-yarn-starter | `e9255d339f` | purged: upstream project, not a part of the framework |
| models.dev | anomalyco/models.dev | `aeba6448ae` | purged: ai-router reads the live catalog (models.dev API) at run time |
| monorepo-template | anomalyco/monorepo-template | `f9c5b5a1c6` | purged: upstream project, not a part of the framework |
| net-tools | anomalyco/net-tools | `d97459d168` | purged: upstream project, not a part of the framework |
| nightmail | anomalyco/nightmail | `5f1bcb5473` | purged: upstream project, not a part of the framework |
| niri | anomalyco/niri | `aee25c9efc` | purged: upstream project, not a part of the framework |
| oc-plugin-rainbow | anomalyco/oc-plugin-rainbow | `fd0de201d1` | purged: upstream project, not a part of the framework |
| openauth | anomalyco/openauth | `98dc59625e` | purged: upstream project, not a part of the framework |
| openclaw | openclaw/openclaw | `92016ce978` | purged: upstream project, not a part of the framework |
| opencode | anomalyco/opencode | `38bd73789a` | purged: upstream project, not a part of the framework |
| opencode-bench | anomalyco/opencode-bench | `49f0f70d30` | purged: upstream project, not a part of the framework |
| opencode-beta | anomalyco/opencode-beta | `2d6329ef83` | purged: upstream project, not a part of the framework |
| opencode-console-cli | anomalyco/opencode-console-cli | `54d35bb2bb` | purged: upstream project, not a part of the framework |
| opencode-drive | anomalyco/opencode-drive | `3db7ca3df6` | purged: upstream project, not a part of the framework |
| opencode-github-copilot | anomalyco/opencode-github-copilot | `5fefaf5b58` | purged: upstream project, not a part of the framework |
| opencode-plugin-lane | anomalyco/opencode-plugin-lane | `161c20b88d` | purged: upstream project, not a part of the framework |
| opencode-pty | anomalyco/opencode-pty | `2f3364a226` | purged: OpenTUI `pty` covers it |
| opencode-sdk-go | anomalyco/opencode-sdk-go | `1f8f6faf2b` | purged: upstream project, not a part of the framework |
| opencode-voice | anomalyco/opencode-voice | `130f75fb7f` | purged: upstream project, not a part of the framework |
| opencode-zed-extension | anomalyco/opencode-zed-extension | `b0c9958e2d` | purged: upstream project, not a part of the framework |
| opentui | anomalyco/opentui | `0c48678ddd` | replaced: `opentui` crate (git pin), native library from the npm platform package (`scripts/opentui-native.ts`) |
| opentui-index | anomalyco/opentui-index | `2b0012ab44` | purged: upstream project, not a part of the framework |
| opentunnel | anomalyco/opentunnel | `6e757aa2c7` | replaced: `opentunnel` crate (crates.io) |
| phonecall | anomalyco/phonecall | `066352836d` | purged: upstream project, not a part of the framework |
| provider | anomalyco/provider | `8f7e6ba270` | purged: upstream project, not a part of the framework |
| pulumi-namecheap | anomalyco/pulumi-namecheap | `394f58fb20` | purged: upstream project, not a part of the framework |
| pulumi-planetscale | anomalyco/pulumi-planetscale | `5f6352517e` | purged: upstream project, not a part of the framework |
| pulumi-railway | anomalyco/pulumi-railway | `bbdcf4dd23` | purged: upstream project, not a part of the framework |
| pulumi-sst | anomalyco/pulumi-sst | `8932dfed46` | purged: upstream project, not a part of the framework |
| pulumi-supabase | anomalyco/pulumi-supabase | `5b3443568d` | purged: upstream project, not a part of the framework |
| rift | anomalyco/rift | `7077f47ae1` | purged: upstream project, not a part of the framework |
| rune | anomalyco/rune | `bd4d42b12c` | replaced: `rune`, `rune-kit`, `rune-motion` (git pin) behind the `ui` feature |
| scoop-bucket | anomalyco/scoop-bucket | `48eba9281c` | purged: upstream project, not a part of the framework |
| serverless-stack-resources-sample | anomalyco/serverless-stack-resources-sample | `d79591c2e5` | purged: upstream project, not a part of the framework |
| slack-support | anomalyco/slack-support | `6c16daf429` | purged: upstream project, not a part of the framework |
| social-cards | anomalyco/social-cards | `924edceea7` | purged: upstream project, not a part of the framework |
| sqr | anomalyco/sqr | `817fa07fd0` | purged: upstream project, not a part of the framework |
| sst | anomalyco/sst | `6bb81a17bc` | purged: upstream project, not a part of the framework |
| sst-start-demo | anomalyco/sst-start-demo | `069dfea77d` | purged: upstream project, not a part of the framework |
| sst-weekly-repos | anomalyco/sst-weekly-repos | `9635eaedf4` | purged: upstream project, not a part of the framework |
| strobe | anomalyco/strobe | `cf75375dcf` | purged: upstream project, not a part of the framework |
| telemetry | anomalyco/telemetry | `4974f4a662` | purged: upstream project, not a part of the framework |
| terminal-control | anomalyco/terminal-control | `6c255aa481` | purged: OpenTUI `TerminalView` and `pty` cover it |
| textjs | anomalyco/textjs | `e021286a89` | purged: upstream project, not a part of the framework |
| torpedo | anomalyco/torpedo | `8c5b47ab03` | purged: upstream project, not a part of the framework |
| transcribe.cpp | anomalyco/transcribe.cpp | `5700f127e3` | purged: upstream project, not a part of the framework |
| tree-sitter-clojure | anomalyco/tree-sitter-clojure | `14d6493956` | purged: upstream project, not a part of the framework |
| tree-sitter-vue | anomalyco/tree-sitter-vue | `6e3fd7f0f6` | purged: upstream project, not a part of the framework |
| tutorial-links-app | anomalyco/tutorial-links-app | `66a04a4e01` | purged: upstream project, not a part of the framework |
| v1-conf | anomalyco/v1-conf | `ceb5870e84` | purged: upstream project, not a part of the framework |
| v2 | anomalyco/v2 | `0341899c67` | purged: upstream project, not a part of the framework |
| zed-extensions | anomalyco/zed-extensions | `b853a426bb` | purged: upstream project, not a part of the framework |
| codex | openai/codex | `81b995a6c9` | purged |
| openai-node | openai/openai-node | `a0719fc6c8` | purged |
| openai-agents-js | openai/openai-agents-js | `8aeac0eb25` | purged |
| openai-openapi | openai/openai-openapi | `f437d0eccb` | purged |
| OpenAPI-Specification | OAI/OpenAPI-Specification | `4f86429f13` | purged |
| hey-api | hey-api/hey-api | `d6ba11fd9a` | purged |
| huggingface-skills | huggingface/skills | `25910b3d0b` | purged |
| angular-material | aphrody-labs/components | `9187233788` | purged: upstream project, not a part of the framework |
| material-web | aphrody-labs/material-web | `37aa14b843` | purged: upstream project, not a part of the framework |
| material-color-utilities | aphrody-labs/material-color-utilities | `194144d75e` | purged: upstream project, not a part of the framework |
| material-symbols | aphrody-labs/material-design-icons | `10ecc14790` | purged: upstream project, not a part of the framework |
| flutter-material | aphrody-labs/flutter | `e8ac49ab53` | purged: upstream project, not a part of the framework |
| compose-material3 | aphrody-labs/androidx | `e70f5406a4` | purged: upstream project, not a part of the framework |
