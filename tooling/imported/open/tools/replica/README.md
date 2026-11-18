# replica

One recursive workflow from the recon of a site to a built clone, assembled from upstream tools and libraries.

```sh
bun tools/replica/cli.ts env                                   # Python environment, devices
bun tools/replica/cli.ts recon https://example.org --out replica/example
bun tools/replica/cli.ts ask replica/example "how are files uploaded?"
bun tools/replica/cli.ts run https://example.org --out replica/example --project <dir> --brief brief.md --clone-url http://127.0.0.1:8080
```

## Stages

| Stage | What runs | Output in the replica directory |
|---|---|---|
| Crawl | [crawl4ai](https://github.com/unclecode/crawl4ai) deep crawl (default), or [agent-browser](https://github.com/vercel-labs/agent-browser) with Chrome, [Obscura](https://github.com/h4ckf0r0day/obscura) or Lightpanda (`--engine`); robots.txt honoured, one page at a time with a delay, item pages of one route capped (`--per-shape`) | `screens.jsonl`, `screens/`, `pages/*.md` |
| Sources | published OpenAPI descriptions and linked documents, typed by [Magika](https://github.com/google/magika), converted by [docling](https://github.com/docling-project/docling) (PDF) or [markitdown](https://github.com/microsoft/markitdown) | `sources/`, `sources.json` |
| Binaries | [LIEF](https://github.com/lief-project/LIEF) and [pyghidra](https://pypi.org/project/pyghidra/) (Ghidra headless) on binaries given with `--binaries`, for files the user may analyse | `rea/*.md` |
| Map | recon map and feature matrix in the format of the `replica-*` skills; the matrix is merged, never overwritten | `recon.md`, `features.csv` |
| Index | [LlamaIndex](https://github.com/run-llama/llama_index) over every Markdown file, embeddings by a Hub sentence-similarity model on PyTorch (CUDA when present) | `index/` |
| Agent stages | architect, design, backend, build, test, diff: a coding agent CLI (`claude`, `codex`, `opencode`, `gemini`, or `REPLICA_AGENT`) follows each `replica-*` skill, searching the index with `ask` | `architecture.md`, `stages/` and the project |
| Parity | feature parity (must 3, should 2, could 1); with `--clone-url`, layout parity: TensorFlow image embeddings of original and clone screens, plus obscura's pixel and structural-edge diagnostics (`render-repros/check.py`, absorbed) | `parity.md`, `visual.json`, `clone-screens/` |
| Recursion | while parity is under `--target`, a deeper recon of the three weakest areas, re-indexing, then build, test and diff again, for `--rounds` | `workflow.log` |

Screenshots, page text and downloaded sources are the site's content: the replica directory's `.gitignore` keeps them out of version control; `recon.md`, `features.csv` and the stage notes are the analysis.

## Python environment

`ml.ts` creates it with uv in the user state directory (`LOCALAPPDATA`, `XDG_STATE_HOME` or `~/.local/state`, then `replica/py`): the newest CPython that both TensorFlow and PyTorch publish wheels for on the platform (read from PyPI), PyTorch through `uv pip install --torch-backend auto` (CUDA build when an NVIDIA driver answers, CPU build otherwise), TensorFlow with its `and-cuda` extra on Linux with a driver and its CPU build elsewhere (TensorFlow publishes no GPU build for native Windows or macOS), then crawl4ai's browser setup. A marker keyed on `py/pyproject.toml` and the GPU skips the work when nothing changed.

## Settings

| Variable | Default |
|---|---|
| `REPLICA_AGENT` | first of `claude`, `codex`, `opencode`, `gemini` on PATH; a command line with `{prompt}` |
| `REPLICA_SKILLS` | nearest `.agents/skills` holding `replica-recon` |
| `REPLICA_EMBED_MODEL` | the Hub's most downloaded sentence-similarity model, resolved when the index is built |
| `REPLICA_BACKBONE` | `EfficientNetV2B0` (any `keras.applications` model) |
| `REPLICA_BROWSER` | `agent-browser` |
| `HF_ENDPOINT` | `https://huggingface.co` (model search for the embedding model) |
