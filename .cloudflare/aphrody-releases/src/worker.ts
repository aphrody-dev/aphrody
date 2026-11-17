export interface R2ObjectLike {
  body: ReadableStream | string | Uint8Array;
  httpEtag?: string;
  writeHttpMetadata?: (headers: Headers) => void;
}

export interface R2BucketLike {
  get(key: string): Promise<R2ObjectLike | null>;
}

export interface Env {
  RELEASES_BUCKET: R2BucketLike;
}

const VALID_CHANNELS = new Set(["lts", "stable", "preview", "canary", "nightly"]);

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

  const html = `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Download Aphrody (${channel}) — GPU-Native Studio</title>
  <style>
    :root { color-scheme: dark; }
    * { box-sizing: border-box; }
    body { margin: 0; font-family: system-ui, -apple-system, sans-serif; background: #090d16; color: #e2e8f0; display: grid; place-items: center; min-height: 100vh; padding: 24px; }
    main { max-width: 680px; width: 100%; background: #0f172a; border: 1px solid #1e293b; border-radius: 16px; padding: 32px; }
    .pill { display: inline-block; font-size: 12px; padding: 4px 10px; border-radius: 999px; background: #1d4ed826; color: #60a5fa; border: 1px solid #2563eb4d; }
    h1 { margin: 14px 0 8px; font-size: 2rem; }
    .cta { display: block; text-align: center; text-decoration: none; background: #2563eb; color: #fff; font-weight: 600; padding: 14px 18px; border-radius: 10px; margin: 20px 0 12px; }
    .channels { display: flex; flex-wrap: wrap; gap: 8px; margin-bottom: 18px; font-size: 13px; }
    .channels a { color: #93c5fd; text-decoration: none; padding: 4px 10px; border-radius: 6px; border: 1px solid #1e293b; }
    .channels a[aria-current="true"] { background: #1e40af; color: #fff; border-color: #3b82f6; }
    pre { background: #020617; padding: 14px; border-radius: 10px; overflow-x: auto; font-size: 12px; border: 1px solid #1e293b; }
  </style>
</head>
<body>
  <main>
    <span class="pill">Zero-Egress Cloudflare R2 &bull; Sigstore Provenance &bull; Channel: ${channel}</span>
    <h1>Download Aphrody</h1>
    <p>GPU-native studio (120 FPS GPUI, NSH, ArtCraft, WGPU v30, 100% Zed compatible). Detected: <strong>${detected.label}</strong></p>
    <a class="cta" href="${assetUrl}">Download ${detected.asset} (${channel})</a>
    <nav class="channels" aria-label="Release channels">
      ${["lts", "stable", "preview", "canary", "nightly"]
        .map((c) => `<a href="/download?channel=${c}" aria-current="${c === channel}">${c}</a>`)
        .join("")}
    </nav>
    <pre># POSIX (macOS / Linux)\ncurl -fsSL https://releases.aphrody.com/install.sh | APHRODY_CHANNEL=${channel} sh\n\n# Windows PowerShell\n$env:APHRODY_CHANNEL="${channel}"; irm https://releases.aphrody.com/install.ps1 | iex\n\n# Verify Sigstore attestation\ngh attestation verify ${detected.asset} --owner aphrody-dev</pre>
  </main>
</body>
</html>`;
  return new Response(html, { headers: { "Content-Type": "text/html; charset=utf-8" } });
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    const path = url.pathname;

    if (path === "/healthz" || path === "/api/health") {
      return Response.json(
        {
          ok: true,
          service: "aphrody-releases",
          channels: [...VALID_CHANNELS],
          r2_bucket: "aphrody-releases",
        },
        { headers: { "Cache-Control": "no-store" } },
      );
    }
    if (path === "/api/channels") {
      return Response.json(
        {
          channels: [...VALID_CHANNELS],
          latest_endpoint: "/api/releases/{channel}/latest",
        },
        { headers: { "Cache-Control": "public, max-age=60" } },
      );
    }
    if (path === "/" || path === "/download" || path === "/download/") {
      return renderDownloadPortal(request);
    }
    if (path === "/install.sh") {
      const obj = await env.RELEASES_BUCKET.get("scripts/install.sh");
      return obj
        ? new Response(obj.body, {
            headers: {
              "Content-Type": "text/x-shellscript; charset=utf-8",
              "Cache-Control": "public, max-age=300",
            },
          })
        : new Response("Installer unavailable — run deploy to populate scripts/install.sh\n", {
            status: 404,
          });
    }
    if (path === "/install.ps1") {
      const obj = await env.RELEASES_BUCKET.get("scripts/install.ps1");
      return obj
        ? new Response(obj.body, {
            headers: {
              "Content-Type": "text/plain; charset=utf-8",
              "Cache-Control": "public, max-age=300",
            },
          })
        : new Response("Installer unavailable — run deploy to populate scripts/install.ps1\n", {
            status: 404,
          });
    }

    const assetMatch = /^\/releases\/([^/]+)\/([^/]+)\/asset$/.exec(path);
    if (assetMatch) {
      const [, rawChannel, version] = assetMatch;
      const channel = rawChannel.toLowerCase();
      if (!VALID_CHANNELS.has(channel)) return new Response("Invalid channel", { status: 400 });
      const os = url.searchParams.get("os") ?? "windows";
      const arch = url.searchParams.get("arch") ?? "x86_64";
      const obj = await env.RELEASES_BUCKET.get(`manifests/${channel}/${version}/${os}-${arch}.json`);
      return obj
        ? new Response(obj.body, { headers: { "Content-Type": "application/json; charset=utf-8" } })
        : new Response("Manifest not found", { status: 404 });
    }

    const latestMatch = /^\/api\/releases\/([^/]+)\/latest$/.exec(path);
    if (latestMatch) {
      const channel = latestMatch[1].toLowerCase();
      if (!VALID_CHANNELS.has(channel)) return new Response("Invalid channel", { status: 400 });
      const obj = await env.RELEASES_BUCKET.get(`channels/${channel}/latest.json`);
      return obj
        ? new Response(obj.body, { headers: { "Content-Type": "application/json; charset=utf-8" } })
        : new Response("Channel manifest not found", { status: 404 });
    }

    if (path.startsWith("/assets/")) {
      const key = path.slice("/assets/".length);
      const obj = await env.RELEASES_BUCKET.get(key);
      if (!obj) return new Response("Asset not found", { status: 404 });
      const headers = new Headers({ "Cache-Control": "public, max-age=31536000, immutable" });
      obj.writeHttpMetadata?.(headers);
      if (obj.httpEtag) headers.set("etag", obj.httpEtag);
      return new Response(obj.body, { headers });
    }

    return new Response("Not Found", { status: 404 });
  },
};
