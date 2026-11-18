# A free, open, multi-provider AI router

Idea: one router, compatible with OpenAI and Anthropic, that discovers models at run time and fails over from one offer to the next until one answers; usable from any language through a C ABI and from every surface (CLI, TUI, desktop, mobile, serverless, self-hosted).

## Existing base

Aphrody's `ai-os` router (TypeScript, Bun) has the logic: catalog joined from models.dev, the litellm price table, OpenRouter `/models`, the Hugging Face router and the Arena leaderboard; policies `auto`, `auto:cheap`, `auto:fast`, `auto:code`, `auto:long`, `arena`; failover on 429, 5xx, timeout, quota, model-specific 4xx; HTTP `/v1/chat/completions`, `/v1/messages`, `/v1/models`.

## Prototypes built, then removed

Everything below was built, tested, then purged. The facts remain useful:

| Prototype | Measured result |
|---|---|
| C ABI (5 calls: `open`, `request`, `chat`, `models`, `close`), BSD sockets / Winsock | OK on Linux against a fake OpenAI server: plain and chunked replies, bearer, JSON escaping |
| HTTPS through the OS TLS stack: WinHTTP on Windows, libcurl loaded with `dlopen` elsewhere | OK live on Windows and Linux, no link dependency |
| Transport hook `ay_router_set_transport`: the platform supplies HTTPS (Java `HttpURLConnection` on Android, URLSession on iOS) | written, never compiled |
| `auto:free` policy: zero-priced offers first (best Arena score), then the cheapest paid ones | 21 tests green |
| OpenAI-compatible Cloudflare Worker, offers discovered at run time (Workers AI, Gemini free tier) | `glm-5.3-flash` refused ("not available on the Workers Free plan"), automatic failover to `glm-4.7-flash` which answered; attempt trace returned in the response |
| Rust binding (`build.rs` compiles the C) | live test green |
| CLI, line-based terminal chat, Win32 desktop chat | real answers obtained |

Failures: the Gemini keys in the vault were refused by Google (403 "invalid API key"); hand-made interfaces judged ugly (CSS and Win32 instead of official components); no mobile surface and no full-screen TUI.

## Market

Open-source gateways are proxies, not libraries: LiteLLM (Python, MIT, also a Rust build), Bifrost (Go, Apache-2.0), Portkey (TypeScript, Workers), LLM Gateway (AGPLv3), Agent Router (formerly Envoy AI Gateway). None ships a C ABI: that is the gap.

## Open points for the spec

- "Free and unlimited": no free offer is unlimited (Workers AI has a daily allocation; free provider plans are capped). Unlimited means local models (llama.cpp, Ollama) as the last resort; the owner chose local first (D8).
- Library and service (D2 decided: both).
- UI: Google's official components (see [design-system.md](design-system.md)).
