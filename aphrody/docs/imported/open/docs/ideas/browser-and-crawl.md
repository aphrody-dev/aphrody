# Browsers, engines and crawl

Goal: capture and fully mirror design.google and m3.material.io (HTML, CSS, JS, fonts, images, video, network calls), with a screenshot of every page, as a real source.

## Why a real browser

On `m3.material.io/components/buttons/overview`, `curl` receives 62 KB with 9 visible words; the rendered DOM has 460. The pages are built by JavaScript. A first pass also saw 276 `fetch` and 425 `xhr` requests.

Sitemaps: m3.material.io publishes one (375 URLs); so does design.google. robots.txt: design.google disallows `/api/`; m3 disallows `/archive`, `/search.html` and search or tracking URLs.

## Real dependencies (read in manifests and code)

| Tool | JS engine | Graphics rendering | CDP client | Browser driven |
|---|---|---|---|---|
| Playwright | the browser's | the browser's | internal | Chromium = Chrome for Testing build served by `cdn.playwright.dev`; Firefox and WebKit in Playwright-specific revisions |
| Puppeteer | same | same | internal | Chrome for Testing (`storage.googleapis.com/chrome-for-testing-public`) |
| agent-browser | same | same | own Rust client (`tokio-tungstenite`), no Playwright | Chrome for Testing; Lightpanda and Obscura modules |
| Obscura | V8 through `deno_core` | own engine (html5ever, taffy, tiny-skia), embedded fonts | own CDP server | none |
| Lightpanda | V8 (zig-v8 fork); network curl + BoringSSL + nghttp2 | none: PNG capture is a text rendering; CSS domain limited to 2 methods | CDP server (port 9222) | none |
| Crawlee (JS) | — | — | — | `playwright` as a peer dependency; remote browser through `remoteBrowser` |
| Crawlee (Python) | — | — | — | `playwright` extra, fingerprints `browserforge`, HTTP `impit` / `curl-cffi` |
| anomalyco/browse | — | — | — | drives tabs with Playwright; MCP server + Chromium extension |

Playwright docs: `connectOverCDP` works only with Chromium and is "significantly lower fidelity" than Playwright's own protocol.

## Full inventory

- Engines: Chrome for Testing, Chromium, WebKit, Firefox, Servo (classic WebDriver only), Blitz, Obscura, Lightpanda, Ladybird, Brave, ungoogled-chromium.
- Drivers: Playwright, Puppeteer, Selenium, WebDriver BiDi, Chrome DevTools Protocol, agent-browser, chrome-devtools-mcp, playwright-mcp, browser-use, Stagehand.
- Crawlers: Crawlee (JS, Python), Scrapy (+ scrapy-playwright), crawl4ai, Firecrawl, katana, Apify (CLI, SDK, actors).
- Archiving: browsertrix-crawler (WARC/WACZ), pywb (replay), single-file-cli, monolith, httrack, wget2.
- Network: mitmproxy (HAR, API).
- Extraction: markitdown, trafilatura, readability, Jina Reader.
- Services: Cloudflare Browser Run `/crawl` (discovery, rendering, HTML/Markdown/JSON), browserless, steel-browser.

## Proposed pipeline

1. Discovery: sitemaps, katana, Crawlee (queue, retries, robots limits).
2. Reference rendering: Chrome for Testing (agent-browser); full network trace.
3. Faithful archive: browsertrix-crawler to WACZ, replayed with pywb; single-file per page.
4. Comparison: Obscura, Lightpanda, WebKit on the same batch (DOM, network).
5. Content: markitdown, trafilatura.
6. Code link: every M3 component tied to its implementation (angular/components, androidx, flutter) and to the Figma kit.

Mistake not to repeat: a first mirror was written on Playwright alone, without justification, with a single engine.
