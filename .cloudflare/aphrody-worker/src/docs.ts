import { MONOGRAM_SVG, type WorkerEnv } from "./types.ts";

export const LLMS_TXT = `# Aphrody Documentation

> Open-source Rust operating system, GPU-native studio, Craft apps, Wayland WASM compositor, and ay SDK/CLI.

## Surfaces
- [Aphrody Home](https://aphrody.com/): Main landing, Wayland WASM demo, and catalog
- [Aphrody Launcher](https://aphrody.app/): WebGPU launcher and edge publishing platform
- [Aphrody Releases](https://releases.aphrody.com/): Multi-channel binaries and installers
- [Aphrody Assets](https://assets.aphrody.app/): Content-addressed R2 design, font, and showcase media
- [Aphrody API](https://api.aphrody.com/v1/): OpenAPI catalog and artifact endpoints
- [Aphrody MCP](https://mcp.aphrody.com/): Streamable HTTP MCP and A2A gateway

## Quick Install
- POSIX: \`curl -fsSL https://releases.aphrody.com/install.sh | sh\`
- Windows: \`irm https://releases.aphrody.com/install.ps1 | iex\`

## Modules
- **Aphrody OS**: Pure Rust OS, kernel/Linux tooling, GPUI + WGPU v30 compositor + Material 3 GPUI Kit & NSH
- **ay SDK**: C ABI, OpenAPI, design tokens (Material 3 Expressive), and quota-aware fleet tools
- **Craft Apps**: FigCraft (Full GPUI), ArtCraft, PhotoCraft, VectorCraft, FilmCraft, SoundCraft, WordCraft
`;

const DOCS_PAGES: Record<string, { title: string; desc: string; h1: string; lead: string; md: string }> = {
  "/": {
    title: "Aphrody Docs — OS, SDK, CLI & Craft Studio",
    desc: "Official Aphrody documentation: Aphrody OS, ay SDK, GPU-native GPUI/WGPU studio, Craft Apps, and zero-egress R2 releases.",
    h1: "Aphrody Documentation",
    lead: "Pure Rust OS, GPU-native GPUI/WGPU v30 studio, Craft Apps, and the ay SDK/CLI.",
    md: LLMS_TXT,
  },
  "/docs": {
    title: "Getting Started — Aphrody Docs",
    desc: "Install Aphrody, explore the ay SDK, and launch the browser Wayland/WebGPU studio.",
    h1: "Getting Started with Aphrody",
    lead: "Install the Aphrody CLI/studio across Windows, macOS, and Linux, or run in the browser via WebGPU.",
    md: `# Getting Started with Aphrody\n\n## Install\n\`\`\`sh\ncurl -fsSL https://releases.aphrody.com/install.sh | sh\n\`\`\`\n\n## Channels\n- \`stable\`, \`preview\`, \`canary\`, \`nightly\`, \`lts\`, \`dev\` at https://releases.aphrody.com/\n`,
  },
};

function renderDocsHtml(pathname: string): string {
  const page = DOCS_PAGES[pathname === "/docs/" ? "/docs" : pathname] ?? DOCS_PAGES["/"];
  const canonical = `https://docs.aphrody.com${pathname === "/docs" || pathname === "/docs/" ? "/docs/" : "/"}`;
  return `<!DOCTYPE html>
<html lang="en" dir="ltr" data-theme="neural">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>${page.title}</title>
  <meta name="description" content="${page.desc}" />
  <link rel="canonical" href="${canonical}" />
  <link rel="icon" href="/icon.svg" type="image/svg+xml" />
  <link rel="icon" href="/favicon.ico" sizes="any" />
  <link rel="apple-touch-icon" href="https://aphrody.com/apple-touch-icon.png" />
  <meta property="og:type" content="website" />
  <meta property="og:site_name" content="Aphrody Docs" />
  <meta property="og:title" content="${page.title}" />
  <meta property="og:description" content="${page.desc}" />
  <meta property="og:url" content="${canonical}" />
  <meta property="og:image" content="https://aphrody.com/og-image.png" />
  <meta name="twitter:card" content="summary_large_image" />
  <style>
    :root {
      color-scheme: dark light;
      --md-ref-typeface-brand: "Google Sans Flex", "IBM Plex Sans", system-ui, sans-serif;
      --md-ref-typeface-mono: "Google Sans Code", "JetBrains Mono", monospace;
      --md-sys-color-background: #0B0E14;
      --md-sys-color-surface-container-low: #13161F;
      --md-sys-color-surface-container: #181C26;
      --md-sys-color-surface-container-lowest: #06080C;
      --md-sys-color-on-surface: #E4EAF4;
      --md-sys-color-on-surface-variant: #BAC4D6;
      --md-sys-color-outline-variant: #384152;
      --md-sys-color-primary: #8FD0FF;
      --md-sys-color-on-primary: #003352;
      --md-sys-color-primary-container: #004B75;
      --md-sys-color-on-primary-container: #CEE5FF;
    }
    @media (prefers-color-scheme: light) {
      :root {
        --md-sys-color-background: #F7F9FF;
        --md-sys-color-surface-container-low: #EEF3FA;
        --md-sys-color-surface-container: #E7EDF6;
        --md-sys-color-surface-container-lowest: #FFFFFF;
        --md-sys-color-on-surface: #161C24;
        --md-sys-color-on-surface-variant: #3D4754;
        --md-sys-color-outline-variant: #BEC7D4;
        --md-sys-color-primary: #006399;
        --md-sys-color-on-primary: #FFFFFF;
        --md-sys-color-primary-container: #CEE5FF;
        --md-sys-color-on-primary-container: #004972;
      }
    }
    * { box-sizing: border-box; }
    body {
      margin: 0; font-family: var(--md-ref-typeface-brand);
      background: var(--md-sys-color-background); color: var(--md-sys-color-on-surface);
      padding: 24px; min-height: 100vh; display: grid; place-items: center;
    }
    main {
      max-width: 880px; width: 100%; background: var(--md-sys-color-surface-container-low);
      border: 1px solid var(--md-sys-color-outline-variant); border-radius: 28px; padding: 32px;
      display: flex; flex-direction: column; gap: 20px;
    }
    header { display: flex; align-items: center; justify-content: space-between; gap: 12px; flex-wrap: wrap; }
    .brand { display: inline-flex; align-items: center; gap: 12px; font-weight: 700; font-size: 18px; color: inherit; text-decoration: none; }
    .pill {
      font-size: 12px; padding: 6px 12px; border-radius: 9999px; font-weight: 600;
      background: var(--md-sys-color-primary-container); color: var(--md-sys-color-on-primary-container);
    }
    h1 { margin: 0; font-size: clamp(1.7rem, 3vw, 2.3rem); }
    p { margin: 0; color: var(--md-sys-color-on-surface-variant); line-height: 1.55; }
    .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 12px; }
    .card {
      padding: 16px; border-radius: 16px; background: var(--md-sys-color-surface-container);
      border: 1px solid var(--md-sys-color-outline-variant); text-decoration: none; color: inherit;
      display: flex; flex-direction: column; gap: 6px;
    }
    .card strong { color: var(--md-sys-color-primary); }
    pre {
      margin: 0; padding: 16px; border-radius: 14px; overflow-x: auto; font-size: 12.5px;
      background: var(--md-sys-color-surface-container-lowest); border: 1px solid var(--md-sys-color-outline-variant);
      font-family: var(--md-ref-typeface-mono);
    }
    footer { display: flex; justify-content: space-between; flex-wrap: wrap; gap: 8px; font-size: 12px; color: var(--md-sys-color-on-surface-variant); }
    footer a { color: var(--md-sys-color-primary); }
  </style>
</head>
<body>
  <main>
    <header>
      <a class="brand" href="/">
        ${MONOGRAM_SVG.replace('width="96" height="96"', 'width="32" height="32"')}
        <span>Aphrody Docs</span>
      </a>
      <span class="pill">Material 3 Neural Expressive &bull; Accept: text/markdown</span>
    </header>
    <h1>${page.h1}</h1>
    <p>${page.lead}</p>
    <section class="grid" aria-label="Documentation modules">
      <a class="card" href="https://aphrody.com/wayland/"><strong>Aphrody OS &amp; Wayland</strong><span>Pure Rust OS, WGPU v30, and in-browser Wayland WASM compositor.</span></a>
      <a class="card" href="https://releases.aphrody.com/"><strong>Releases &amp; Installers</strong><span>Zero-egress R2 downloads across stable, preview, canary, and nightly.</span></a>
      <a class="card" href="https://aphrody.app/"><strong>Launcher &amp; Edge Platform</strong><span>WebGPU launcher, preview deploys, and real-user Web Vitals.</span></a>
      <a class="card" href="https://api.aphrody.com/v1/"><strong>OpenAPI &amp; ay SDK</strong><span>C ABI, OpenAPI catalogue, Design Tokens, and fleet quota router.</span></a>
    </section>
    <pre># Install Aphrody CLI &amp; Studio\ncurl -fsSL https://releases.aphrody.com/install.sh | sh\n\n# Agent Markdown discovery\ncurl -H "Accept: text/markdown" https://docs.aphrody.com/\ncurl https://docs.aphrody.com/llms.txt</pre>
    <footer>
      <span><a href="https://aphrody.com/">aphrody.com</a> &middot; <a href="/llms.txt">/llms.txt</a> &middot; <a href="/docs/">/docs/</a></span>
      <span><a href="/healthz">/healthz</a></span>
    </footer>
  </main>
</body>
</html>`;
}

export function markdownPathFor(pathname: string): string {
  if (pathname === "/docs" || pathname === "/docs/") return "/docs/getting-started.md";
  if (pathname.endsWith("/index.md")) return pathname.replace(/\/index\.md$/, "/getting-started.md");
  if (pathname.endsWith(".md")) return pathname;
  if (pathname.endsWith(".html")) return pathname.replace(/\.html$/, ".md");
  if ((pathname.split("/").pop() ?? "").includes(".")) return pathname;
  if (pathname.endsWith("/")) return `${pathname}getting-started.md`;
  return `${pathname}.md`;
}

export async function handleDocsRequest(request: Request, env: WorkerEnv): Promise<Response> {
  const url = new URL(request.url);
  const path = url.pathname;

  if (path === "/icon.svg" || path === "/favicon.svg") {
    return new Response(MONOGRAM_SVG, {
      headers: { "Content-Type": "image/svg+xml; charset=utf-8", "Cache-Control": "public, max-age=86400", "X-Content-Type-Options": "nosniff" },
    });
  }
  if (path === "/favicon.ico") return Response.redirect("https://aphrody.com/favicon.ico", 301);
  if (path === "/apple-touch-icon.png") return Response.redirect("https://aphrody.com/apple-touch-icon.png", 301);
  if (path === "/robots.txt") {
    return new Response("User-agent: *\nAllow: /\nSitemap: https://docs.aphrody.com/sitemap.xml\n", {
      headers: { "Content-Type": "text/plain; charset=utf-8", "Cache-Control": "public, max-age=3600" },
    });
  }
  if (path === "/sitemap.xml") {
    return new Response(
      `<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><url><loc>https://docs.aphrody.com/</loc></url><url><loc>https://docs.aphrody.com/docs/</loc></url></urlset>\n`,
      { headers: { "Content-Type": "application/xml; charset=utf-8", "Cache-Control": "public, max-age=3600" } },
    );
  }
  if (path === "/llms.txt" || path === "/docs/llms.txt" || path === "/docs.md") {
    return new Response(LLMS_TXT, {
      headers: { "Content-Type": "text/markdown; charset=utf-8", "Cache-Control": "public, max-age=300", "X-Content-Type-Options": "nosniff" },
    });
  }

  const acceptHeader = request.headers.get("Accept") ?? "";
  const wantsMarkdown = acceptHeader
    .split(",")
    .map((m) => m.split(";")[0]?.trim().toLowerCase() ?? "")
    .includes("text/markdown");

  if (env.DOCS_ORIGIN_URL) {
    const upstream = new URL(env.DOCS_ORIGIN_URL);
    url.hostname = upstream.hostname;
    url.protocol = upstream.protocol;
    if (wantsMarkdown) url.pathname = markdownPathFor(url.pathname);
    const res = await fetch(url, request);
    if (res.status !== 404) return res;
  }

  if (wantsMarkdown || path.endsWith(".md")) {
    const key = path === "/docs" || path === "/docs/" ? "/docs" : "/";
    return new Response(DOCS_PAGES[key].md, {
      headers: { "Content-Type": "text/markdown; charset=utf-8", "Cache-Control": "public, max-age=300", "X-Content-Type-Options": "nosniff" },
    });
  }

  if (path === "/" || path === "/docs" || path === "/docs/" || path.startsWith("/docs/")) {
    return new Response(renderDocsHtml(path), {
      headers: { "Content-Type": "text/html; charset=utf-8", "Cache-Control": "public, max-age=300", "X-Content-Type-Options": "nosniff" },
    });
  }

  return new Response("Not Found", { status: 404, headers: { "Cache-Control": "no-store" } });
}
