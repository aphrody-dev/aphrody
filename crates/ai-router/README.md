# ai-router

Multi-provider AI router: a library with a C ABI, a CLI and an HTTP service compatible with OpenAI and Anthropic clients.

- **Offers discovered at run time**: the [models.dev](https://models.dev) catalog (providers, base URLs, prices, limits, modalities, the variables that hold credentials) and the OpenAI-compatible servers running on this machine (Ollama, llama.cpp server, LM Studio, or `AI_ROUTER_LOCAL`). No model list is written in the code.
- **Order**: local servers first, then offers at zero cost for the caller, then the cheapest. An offer whose credential variable is not set is never tried. Policies: `auto`, `auto:local`, `auto:free`, `auto:cheap`, or an offer id `<provider>/<model>` followed by the `auto` order.
- **Failover**: on any failure (rate limit, quota, server error, timeout, refused model) the next offer is tried; the answer carries `router.offer` and `router.attempts`.
- **Protocols spoken upstream**: OpenAI-compatible, Anthropic messages, Google generateContent; each streamed in its own protocol (server-sent events of chat completions, Anthropic message events, Google `streamGenerateContent`) and delivered as chat-completion chunks.
- **Kinds of inference**: chat; embeddings, image generation, speech and transcription go to offers whose catalog modalities (or, for local servers, model names) fit the kind, passed through to their OpenAI-compatible endpoint with failover.
- **Free and unlimited**: zero-cost offers and local models come first; local models have no quota, remote free offers do.

## Surfaces

| Surface | How |
|---|---|
| CLI | `ai-router models [policy] [--json]`, `ai-router chat [--model <policy or id>] <prompt>` |
| Service | `ai-router serve [--addr host:port]` (beyond loopback it needs `AI_ROUTER_TOKENS`, comma-separated bearer tokens, or `AI_ROUTER_OPEN=1`): `POST /v1/chat/completions`, `POST /v1/messages`, `POST /v1/embeddings`, `POST /v1/images/generations`, `POST /v1/audio/speech`, `POST /v1/audio/transcriptions`, `GET /v1/models`, `GET /health` |
| C ABI | `include/ai_router.h`: `aphrody_router_open`, `_chat`, `_models`, `_close`, `_last_error`; strings freed with `aphrody_free` (crate `c-abi`) |
| Bun | `packages/ai-router` (bun:ffi), with streaming |
| Terminal | `crates/ai-router-tui`: full-screen chat, streamed answers, policy switch |
| GUI, desktop, mobile web | `apps/ai-router`: Angular Material chat, one Bun process (`--open` starts it in the system browser), installable web manifest |
| Container (serverless, cloud, self-hosted) | `deploy/Containerfile`: static binary in an empty image, published as `ghcr.io/aphrody-labs/ai-router` by the workflow after a health check |
| Agent | `apps/router-agent`: the Hugging Face MCP agent loop with the router as its model and the Hugging Face skills |

## Verified

- Tests (`cargo test -p ai-router`): failover after a rate limit, offers without credentials never tried, Anthropic and Google answers translated to chat completions, local server first through the C ABI, the service answering OpenAI and Anthropic clients, native Anthropic and Google streaming (text and tool calls), embeddings and transcriptions routed to models of that kind (library and service), multipart model rewrite. CI builds and tests on Linux, Windows and macOS (`.github/workflows/ai-router.yml`), including the Bun binding against each build.
- Live embeddings: on Windows, the CI build's service answered `POST /v1/embeddings` with `model: auto:local` through the local embedding model LM Studio served (two inputs, 768-dimension vectors), the chat models of the same servers left out.
- Live: on Windows, the CI build found three local servers (Ollama, llama.cpp server, LM Studio) and over 8,000 catalog offers, answered through a local model as CLI and as a service (OpenAI and Anthropic endpoints).

## Not yet

Native mobile apps (the web app is installable on mobile; a native build over the C ABI with a platform HTTPS transport is not done); non-chat kinds through the C ABI and the Bun binding (the service has them); embeddings, images and audio from Anthropic or Google upstreams (only OpenAI-compatible offers serve non-chat kinds).
