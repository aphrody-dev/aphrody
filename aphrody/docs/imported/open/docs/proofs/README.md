# Proofs

Run on a Linux build host with the release build of this workspace (`cargo build --release -p open --features tui`,
fat LTO), Chrome for Testing canary 160.0.8164.0 (`chrome-for-testing` last-known-good, 2026-11-15) and the public
OpenTunnel service. `cargo check --workspace --all-targets` and `cargo test --workspace` pass on the same host; the
Bun bindings pass `bun scripts/test-ts.ts`.

| Proof | Command | Result |
| :-- | :-- | :-- |
| Real crawl | `open crawl https://www.rust-lang.org/ --max-pages 25 --depth 2 --concurrency 6 --shard shard.db` | 25 pages, 0 failures, 592 links seen, 2.3 s; [pages](crawl-rust-lang.tsv), [stats](crawl-rust-lang.stats.json); `open search shard.db "cargo package manager"` ranks the pages that talk about Cargo |
| CDP browse session | `open browse demo https://www.rust-lang.org/` | tab opened over CDP, accessibility snapshot with refs, 1425x3490 full-page PNG ([session](browse-session.txt), [image](browse-rust-lang.png)) |
| Tunnel end to end | `open tunnel up --route web=127.0.0.1:3100` | `https://web.<id>.opentunnel.xyz/some/path` fetched from another machine returned the page served on the build host, with its random token; TLS ends in the `open` process |
| TUI | `open-tui` in xterm (Xvfb) with tmux, a local OpenAI-compatible server, Enter | streamed answer, offer and attempts in the status line ([screenshot](tui.png)) |
| Extension in Chrome for Testing | `open browse demo chrome-extension://<id>/onboarding.html --extension extension/dist` | service worker target of the extension present, Solid page rendered ([session](extension-session.txt), [screenshot](extension-onboarding.png)) |
| Extension with the Rust relay | `open browse relay`, `open browse pair --extension extension/dist`, then the extension on start | relay `status`: `connected: true`, `pairingState: connected`, protocol 3 compatible; `navigate` through the relay loaded the page in the browser |

The model behind the TUI proof is a deterministic local server, not a real model: it proves rendering, streaming and
the router's offer handling, not answer quality.
