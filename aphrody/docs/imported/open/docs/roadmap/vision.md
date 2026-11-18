# Long-term vision

`open` gathers the open-source ecosystem Aphrody needs into a single repository built end to end, and derives three products from it:

1. **A free AI router**: multi-provider, local first, then free offers, then the cheapest paid ones; compatible with OpenAI and Anthropic; exposed as an embeddable library through a C ABI and as a self-hostable service.
2. **Authentic Material 3 interfaces**: Google's official implementations (Angular Material, Compose, Flutter) plus Rust at the same level, themed with the Aphrody identity, on web, desktop, mobile and terminal.
3. **A living source of truth**: reference sites and repositories (design.google, m3.material.io, M3 implementations, the "open" ecosystem) archived, indexed and linked to the code.

Editions: `open` is the open-source edition and `auto` (`apps/auto` in aphrody-labs/aphrody) is its paid hosted edition. One vision, one codebase, one feature set, in the relation Crawlee has to Apify, n8n to n8n Cloud, Chromium to Chrome:

- `open` is a library and holds the product core: workflow automation (`automate`), agents, ingestion, store, client, domain model, UI, the mail and personal-assistant connectors, crawl and browser engines.
- The Aphrody monorepo (aphrody-labs/aphrody) holds what already exists there once: i18n (MessageFormat 2), secrets at rest (`src/vault`), native document formats (auto's FFI crate merged with `src/craft` behind the C ABI), video and media (`src/tv`).
- `auto` consumes both and keeps only the hosted layer: tenancy, billing, the hosted part of services, administration, API and worker deployment, store listings.
- auto's packages move with their history once their English and i18n pass in src has landed (decision D22).

Principles:

- Absorb rather than rewrite; provenance (lock) for everything that enters.
- No hardcoded values: data read from a source, versions resolved at run time.
- Deliver complete: every step ends built, running, tested and proven.
- English at the source, multilingual through i18n.
- Bazel everywhere; Bun for JS/TS (AOT), Rust with fat LTO, C for the ABI, Zig where upstream uses it.
