import { MONOGRAM_SVG, type WorkerEnv } from "./types.ts";

export const VALID_CHANNELS = new Set(["lts", "stable", "preview", "canary", "nightly", "dev"]);

export function detectPlatform(userAgent: string): {
  label: string;
  os: "windows" | "macos" | "linux";
  arch: "x86_64" | "aarch64";
  asset: string;
} {
  if (/Macintosh|Mac OS X/i.test(userAgent)) {
    const arm = /Arm|Apple|Silicon/i.test(userAgent);
    return {
      label: arm ? "macOS (Apple Silicon)" : "macOS (Intel)",
      os: "macos",
      arch: arm ? "aarch64" : "x86_64",
      asset: arm ? "Aphrody-arm64.dmg" : "Aphrody-x64.dmg",
    };
  }
  if (/Linux/i.test(userAgent)) {
    const arm = /aarch64|arm64/i.test(userAgent);
    return {
      label: arm ? "Linux (ARM64)" : "Linux (x86_64)",
      os: "linux",
      arch: arm ? "aarch64" : "x86_64",
      asset: arm ? "aphrody-aarch64.tar.gz" : "aphrody-x86_64.tar.gz",
    };
  }
  const armWin = /ARM64|WOA/i.test(userAgent);
  return {
    label: armWin ? "Windows (ARM64)" : "Windows (x64)",
    os: "windows",
    arch: armWin ? "aarch64" : "x86_64",
    asset: armWin ? "Aphrody-Setup-arm64.exe" : "Aphrody-Setup-x64.exe",
  };
}

export function renderDownloadPortal(request: Request): Response {
  const url = new URL(request.url);
  const rawChannel = (url.searchParams.get("channel") ?? "stable").toLowerCase();
  const channel = VALID_CHANNELS.has(rawChannel) ? rawChannel : "stable";
  const detected = detectPlatform(request.headers.get("User-Agent") ?? "");
  const assetUrl = `/assets/${channel}/latest/${detected.asset}`;
  const title = `Download Aphrody (${channel}) — GPU-Native Studio`;
  const desc = `Download Aphrody (${channel}) for Windows, macOS, Linux, iOS, and Android via zero-egress Cloudflare R2 with Sigstore provenance.`;

  const html = `<!DOCTYPE html>
<html lang="en" dir="ltr" data-theme="neural">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>${title}</title>
  <meta name="description" content="${desc}" />
  <link rel="canonical" href="https://releases.aphrody.com/" />
  <link rel="icon" href="/icon.svg" type="image/svg+xml" />
  <link rel="icon" href="/favicon.ico" sizes="any" />
  <link rel="apple-touch-icon" href="https://aphrody.com/apple-touch-icon.png" />
  <meta property="og:type" content="website" />
  <meta property="og:site_name" content="Aphrody Releases" />
  <meta property="og:title" content="${title}" />
  <meta property="og:description" content="${desc}" />
  <meta property="og:url" content="https://releases.aphrody.com/" />
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
      --md-sys-brand-bolt: #FFB300;
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
      margin: 0;
      font-family: var(--md-ref-typeface-brand);
      background: var(--md-sys-color-background);
      background-image: radial-gradient(circle at 50% 20%, rgba(59, 130, 246, 0.18), transparent 60%);
      color: var(--md-sys-color-on-surface);
      display: grid;
      place-items: center;
      min-height: 100vh;
      padding: 24px;
    }
    main {
      max-width: 780px;
      width: 100%;
      background: var(--md-sys-color-surface-container-low);
      border: 1px solid var(--md-sys-color-outline-variant);
      border-radius: 28px;
      padding: 32px;
      display: flex;
      flex-direction: column;
      gap: 18px;
    }
    header { display: flex; align-items: center; justify-content: space-between; gap: 12px; flex-wrap: wrap; }
    .brand { display: inline-flex; align-items: center; gap: 12px; font-weight: 700; font-size: 18px; }
    .pill {
      display: inline-block; font-size: 12px; padding: 6px 12px; border-radius: 9999px;
      background: var(--md-sys-color-primary-container); color: var(--md-sys-color-on-primary-container);
      font-weight: 600;
    }
    h1 { margin: 4px 0 0; font-size: clamp(1.6rem, 3vw, 2.2rem); }
    p { margin: 0; color: var(--md-sys-color-on-surface-variant); }
    .cta {
      display: flex; align-items: center; justify-content: center; min-height: 48px;
      text-decoration: none; background: var(--md-sys-color-primary); color: var(--md-sys-color-on-primary);
      font-weight: 700; padding: 12px 22px; border-radius: 9999px;
    }
    .channels { display: flex; flex-wrap: wrap; gap: 8px; font-size: 13px; }
    .channels a {
      min-height: 36px; display: inline-flex; align-items: center; color: var(--md-sys-color-on-surface);
      text-decoration: none; padding: 6px 14px; border-radius: 9999px;
      border: 1px solid var(--md-sys-color-outline-variant); background: var(--md-sys-color-surface-container);
    }
    .channels a[aria-current="true"] {
      background: var(--md-sys-color-primary); color: var(--md-sys-color-on-primary);
      border-color: var(--md-sys-color-primary); font-weight: 700;
    }
    .surfaces { display: grid; grid-template-columns: repeat(auto-fit, minmax(210px, 1fr)); gap: 12px; }
    .card {
      padding: 14px; border-radius: 16px; background: var(--md-sys-color-surface-container);
      border: 1px solid var(--md-sys-color-outline-variant); font-size: 13px;
    }
    .card strong { display: block; margin-bottom: 4px; color: var(--md-sys-color-on-surface); }
    pre {
      margin: 0; background: var(--md-sys-color-surface-container-lowest); padding: 16px;
      border-radius: 14px; overflow-x: auto; font-size: 12.5px;
      font-family: var(--md-ref-typeface-mono); border: 1px solid var(--md-sys-color-outline-variant);
    }
    footer { font-size: 12px; color: var(--md-sys-color-on-surface-variant); display: flex; justify-content: space-between; flex-wrap: wrap; gap: 8px; }
  </style>
</head>
<body>
  <main>
    <header>
      <div class="brand">
        ${MONOGRAM_SVG.replace('width="96" height="96"', 'width="32" height="32"')}
        <span>Aphrody Releases</span>
      </div>
      <span class="pill">Zero-Egress Cloudflare R2 &bull; Sigstore Provenance &bull; Channel: ${channel}</span>
    </header>
    <h1>Download Aphrody</h1>
    <p>GPU-native studio (120 FPS GPUI, NSH, ArtCraft, WGPU v30, 100% Zed compatible). Detected: <strong>${detected.label}</strong></p>
    <a class="cta" href="${assetUrl}">Download ${detected.asset} (${channel})</a>
    <nav class="channels" aria-label="Release channels">
      ${["lts", "stable", "preview", "canary", "nightly", "dev"]
        .map((c) => `<a href="/download?channel=${c}" aria-current="${c === channel}">${c}</a>`)
        .join("")}
    </nav>
    <section class="surfaces" aria-label="Desktop, Mobile & Package Targets">
      <div class="card"><strong>Desktop Suites</strong>Windows (.exe / winget), macOS (.dmg / brew), Linux (deb / rpm / flatpak / nix / AUR)</div>
      <div class="card"><strong>Mobile Companion</strong>iOS Metal (TestFlight / .xcframework) &amp; Android Vulkan (.aar / APK / Play Store)</div>
      <div class="card"><strong>22-Target Store</strong><code>aphrody_store</code> + 1,590 Zed extensions &amp; Wasm/WebGPU templates</div>
    </section>
    <pre># POSIX (macOS / Linux)\ncurl -fsSL https://releases.aphrody.com/install.sh | APHRODY_CHANNEL=${channel} sh\n\n# Windows PowerShell\n$env:APHRODY_CHANNEL="${channel}"; irm https://releases.aphrody.com/install.ps1 | iex\n\n# Verify Sigstore attestation\ngh attestation verify ${detected.asset} --owner aphrody-dev</pre>
    <footer><span>Material 3 Neural Expressive &middot; design.google</span><span>/api/channels &middot; /api/design</span></footer>
  </main>
</body>
</html>`;
  return new Response(html, { headers: { "Content-Type": "text/html; charset=utf-8", "X-Content-Type-Options": "nosniff" } });
}

export async function handleReleasesRequest(request: Request, env: WorkerEnv): Promise<Response> {
  const url = new URL(request.url);
  const path = url.pathname;
  const bucket = env.RELEASES_BUCKET;

  if (path === "/icon.svg" || path === "/favicon.svg") {
    return new Response(MONOGRAM_SVG, {
      headers: {
        "Content-Type": "image/svg+xml; charset=utf-8",
        "Cache-Control": "public, max-age=86400",
        "X-Content-Type-Options": "nosniff",
      },
    });
  }
  if (path === "/favicon.ico") return Response.redirect("https://aphrody.com/favicon.ico", 301);
  if (path === "/apple-touch-icon.png") return Response.redirect("https://aphrody.com/apple-touch-icon.png", 301);
  if (path === "/robots.txt") {
    return new Response("User-agent: *\nAllow: /\n", {
      headers: { "Content-Type": "text/plain; charset=utf-8", "Cache-Control": "public, max-age=3600" },
    });
  }
  if (path === "/api/channels") {
    return Response.json(
      { channels: [...VALID_CHANNELS], latest_endpoint: "/api/releases/{channel}/latest" },
      { headers: { "Cache-Control": "public, max-age=60" } },
    );
  }
  if (path === "/api/design") {
    return Response.json(
      {
        source_of_truth: "https://design.google/",
        system: "Material 3 Expressive + Neural Obsidian",
        modes: ["neural", "dark", "light", "oled"],
        channels: [...VALID_CHANNELS],
        mobile: { ios: "gpui_ios + NativeShell.xcframework", android: "nsh-platform-android Vulkan .aar" },
        store_targets: 22,
      },
      { headers: { "Cache-Control": "public, max-age=300" } },
    );
  }
  if (path === "/" || path === "/download" || path === "/download/") {
    return renderDownloadPortal(request);
  }
  if (path === "/install.sh") {
    const obj = await bucket?.get("scripts/install.sh");
    return obj
      ? new Response(obj.body, {
          headers: { "Content-Type": "text/x-shellscript; charset=utf-8", "Cache-Control": "public, max-age=300" },
        })
      : new Response("Installer unavailable - run deploy to populate scripts/install.sh\n", { status: 404 });
  }
  if (path === "/install.ps1") {
    const obj = await bucket?.get("scripts/install.ps1");
    return obj
      ? new Response(obj.body, {
          headers: { "Content-Type": "text/plain; charset=utf-8", "Cache-Control": "public, max-age=300" },
        })
      : new Response("Installer unavailable - run deploy to populate scripts/install.ps1\n", { status: 404 });
  }

  const assetMatch = /^\/releases\/([^/]+)\/([^/]+)\/asset$/.exec(path);
  if (assetMatch) {
    const [, rawChannel, version] = assetMatch;
    const channel = rawChannel.toLowerCase();
    if (!VALID_CHANNELS.has(channel)) return new Response("Invalid channel", { status: 400 });
    const os = url.searchParams.get("os") ?? "windows";
    const arch = url.searchParams.get("arch") ?? "x86_64";
    const obj = await bucket?.get(`manifests/${channel}/${version}/${os}-${arch}.json`);
    return obj
      ? new Response(obj.body, { headers: { "Content-Type": "application/json; charset=utf-8" } })
      : new Response("Manifest not found", { status: 404 });
  }

  const latestMatch = /^\/api\/releases\/([^/]+)\/latest$/.exec(path);
  if (latestMatch) {
    const channel = latestMatch[1].toLowerCase();
    if (!VALID_CHANNELS.has(channel)) return new Response("Invalid channel", { status: 400 });
    const obj = await bucket?.get(`channels/${channel}/latest.json`);
    return obj
      ? new Response(obj.body, { headers: { "Content-Type": "application/json; charset=utf-8" } })
      : new Response("Channel manifest not found", { status: 404 });
  }

  if (path.startsWith("/assets/")) {
    const key = path.slice("/assets/".length);
    const obj = await bucket?.get(key);
    if (!obj) return new Response("Asset not found", { status: 404 });
    const headers = new Headers({ "Cache-Control": "public, max-age=31536000, immutable" });
    obj.writeHttpMetadata?.(headers);
    if (obj.httpEtag) headers.set("etag", obj.httpEtag);
    return new Response(request.method === "HEAD" ? null : obj.body, { headers });
  }

  return new Response("Not Found", { status: 404 });
}
