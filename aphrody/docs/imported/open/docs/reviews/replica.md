# Review: tools/replica and the ai-router multi-modality pass

Gemini Carbon review of the replica workflow (Bun orchestration, Python steps) and of the ai-router diff (native Anthropic and Google streaming, non-chat pass-through), chat `bc86f9b083060065`. 24 findings, each checked against the code.

## Import (D25): obscura render diagnostics

| Check | Finding | Outcome |
|---|---|---|
| Provenance | `tools/replica/py/replica/obscura_check.py` is `render-repros/check.py` of h4ckf0r0day/obscura at `0d9a2f764ffdb097269da048002c593865e83a9e` (Apache-2.0), header line naming it | kept unchanged below the header; `open.lock.json` re-exports whole repositories, so the pin lives in that header |
| Verification | `pair_metrics` called by `visual.compare` on original and clone screenshots of equal size | kept |
| Purification, purge | none needed: no site values; its own `main` (fixture checks) stays, unused | kept |

## Findings

| # | Finding | Location | Outcome |
|---|---|---|---|
| 1 | stdout read before stderr: a chatty agent blocks | `agent.ts` runStage | fixed: both pipes drained together |
| 2 | same in the Python runner | `ml.ts` py | fixed: stderr drained while stdout streams |
| 3 | `pyghidra.start()` twice in one process fails | `rea.py` ghidra_facts | fixed: `pyghidra.started()` guard. Rejected in part: `open_project`, `program_loader`, `program_context`, `analyze` and `task_monitor` are top-level functions of pyghidra 3.2 (its PyPI description lists them under API) |
| 4 | reports of two binaries with one name overwrite each other | `rea.py` report | fixed: sanitised name plus a hash of the resolved path |
| 5 | request bodies read without a limit | `server.rs` serve | fixed: every POST body read once through `take`, 413 above `AI_ROUTER_MAX_BODY` (256 MiB default) |
| 6 | multipart parsed as lossy UTF-8; delimiters matched without CRLF | `server.rs`, `transport.rs` | fixed: one byte-level parser (`parts`) with RFC 2046 delimiters; only part headers and the model value are decoded; test with binary content holding `--boundary` |
| 7 | source downloads follow redirects to any host, ignore robots.txt, read without a cap | `convert.py` gather | fixed: redirect handler refusing other origins, `urllib.robotparser`, `REPLICA_SOURCE_MAX_BYTES` (64 MiB) |
| 8 | clone captured at the requested route, not the redirected one; PNG-only decoding | `visual.py` | fixed: `final` route; `tf.io.decode_image`. The clone URL is the user's own argument: not an SSRF surface |
| 9 | `preprocess_input` lookup can fail on Keras 3 | `visual.py` embedder | fixed: lookup kept, identity fallback when no submodule publishes it |
| 10 | shape quota spent by pages that failed | `crawl.py` | fixed: quota returned on failure |
| 11 | seeds already captured recorded twice | `crawl.py` | fixed: captured pages are crawled for links only; continuation seeds come from the frontier of unvisited links |
| 12 | robots.txt percent-encoding and tie lengths | `crawl.ts` robots | rejected: empty `Disallow` is allow-all by RFC 9309, `$` and `*` are handled, tie-break favours Allow; percent-encoding normalisation left as a known limit of the agent-browser engine (crawl4ai, the default, has its own parser) |
| 13 | desktop viewport not restored when the mobile capture fails | `crawl.ts` | fixed: `finally` restores it. The first page is resized before its screenshot, so its layout is the requested one |
| 14 | `--agent` split on spaces; paths unquoted in the prompt | `agent.ts` | fixed: quote-aware split, quoted paths. `AGENT_BROWSER_ENGINE` set once per process for the chosen engine: by design |
| 15 | failed agent stages ignored | `agent.ts`, `cli.ts` | fixed: a non-zero exit stops the run with the log path |
| 16 | "skip with a reason" in the clone column scored as no | `parity.ts`, prompt | fixed: any value starting with skip is skipped; the prompt asks for the reason in notes |
| 17 | origin checked by prefix | `cli.ts`, `recon.ts`, `crawl.ts`, `crawl.py`, `convert.py` | fixed: origin equality (`sameOrigin`, `same_origin`) |
| 18 | musllinux wheels counted for glibc hosts | `ml.ts` pythonVersion | fixed: manylinux only; `REPLICA_PYTHON` overrides |
| 19 | index persisted over an older one | `rag.py` | rejected: `persist` rewrites the store files whole; the model is recorded with the index |
| 20 | no role in the first Anthropic chunk; tool without arguments streams `""` | `transport.rs` anthropic_event | fixed: role on `message_start`, `{}` at `content_block_stop`, arguments given at start sent whole; test |
| 21 | multi-line SSE data; Google safety finishes | `transport.rs` stream | fixed: events assembled from all their `data:` lines; safety reasons map to `content_filter`. `tool_calls` as finish when calls were made matches OpenAI |
| 22 | empty credential headers | `transport.rs` stream | rejected: offers without a credential are never candidates (`Offer::available`); the service ends every stream with `[DONE]` |
| 23 | chat models with audio or image output picked for speech and images | `policy.rs` kind_fits | fixed: generators only (no text output); transcription by model name |
| 24 | long `X-Router` header | `server.rs` | fixed: offer and attempt count only |

Tests after the fixes: `cargo test -p ai-router` 11 passed on the test host; `bun test tools/replica` 4 passed; `bun check tools/replica` clean.
