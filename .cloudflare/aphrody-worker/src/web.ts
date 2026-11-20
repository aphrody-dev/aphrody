import type { WorkerEnv } from "./types.ts";

const CONTENT_TYPES: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".wasm": "application/wasm",
  ".json": "application/json; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".ttf": "font/ttf",
  ".woff2": "font/woff2",
};

const COMPRESSIBLE_EXTS = new Set([".wasm", ".js", ".mjs", ".html", ".json", ".css", ".svg"]);
const COHORT_DIRS = new Set(["baseline", "simd-baseline", "multithreaded", "coi"]);
const KNOWN_APPS = new Set(["aphrody_desktop", "aphrody_store", "gpui_web", "gpui_kit", "nsh"]);
const DEFAULT_WEB_APP = "aphrody_desktop";

function extensionOf(path: string): string {
  const idx = path.lastIndexOf(".");
  return idx >= 0 ? path.slice(idx).toLowerCase() : "";
}

export function resolveWebR2Key(pathname: string, stripPrefix = ""): { app: string; relPath: string; r2Key: string } | null {
  let raw = pathname;
  if (stripPrefix && raw.startsWith(stripPrefix)) {
    raw = raw.slice(stripPrefix.length);
  }
  const clean = raw.replace(/^\/+/, "");
  if (clean.includes("..") || clean.includes("\0") || clean.includes("\\")) {
    return null;
  }
  if (!clean) {
    return { app: DEFAULT_WEB_APP, relPath: "index.html", r2Key: `web/${DEFAULT_WEB_APP}/index.html` };
  }
  const segments = clean.split("/").filter(Boolean);
  const first = segments[0] ?? "";
  if (KNOWN_APPS.has(first)) {
    const rest = segments.slice(1).join("/") || "index.html";
    return { app: first, relPath: rest, r2Key: `web/${first}/${rest}` };
  }
  return { app: DEFAULT_WEB_APP, relPath: clean, r2Key: `web/${DEFAULT_WEB_APP}/${clean}` };
}

export function shouldEnableCrossOriginIsolation(url: URL, relPath: string): boolean {
  if (url.searchParams.get("coi") === "1" || url.searchParams.get("threads") === "1") {
    return true;
  }
  const first = relPath.split("/")[0] ?? "";
  return first === "multithreaded" || first === "coi";
}

export async function handleWebRequest(request: Request, env: WorkerEnv, stripPrefix = ""): Promise<Response> {
  if (request.method !== "GET" && request.method !== "HEAD") {
    return new Response("Method Not Allowed", {
      status: 405,
      headers: { Allow: "GET, HEAD", "Cache-Control": "no-store" },
    });
  }

  const url = new URL(request.url);
  const resolved = resolveWebR2Key(url.pathname, stripPrefix);
  if (!resolved) {
    return new Response("Not Found", {
      status: 404,
      headers: { "Cache-Control": "no-store", "X-Content-Type-Options": "nosniff" },
    });
  }

  const bucket = env.ASSETS_BUCKET ?? env.OPEN_SOURCE_WEBSITE_ASSETS_BUCKET;
  if (!bucket) {
    return Response.json(
      { ok: false, error: "web_bucket_unbound", app: resolved.app, key: resolved.r2Key },
      { status: 503, headers: { "Cache-Control": "no-store" } },
    );
  }

  const ext = extensionOf(resolved.relPath);
  const acceptEncoding = (request.headers.get("accept-encoding") ?? "").toLowerCase();
  const candidates: Array<{ key: string; encoding?: "br" | "gzip" }> = [];
  if (COMPRESSIBLE_EXTS.has(ext)) {
    if (acceptEncoding.includes("br")) {
      candidates.push({ key: `${resolved.r2Key}.br`, encoding: "br" });
    }
    if (acceptEncoding.includes("gzip")) {
      candidates.push({ key: `${resolved.r2Key}.gz`, encoding: "gzip" });
    }
  }
  candidates.push({ key: resolved.r2Key });

  let matchedObject: Awaited<ReturnType<typeof bucket.get>> = null;
  let matchedEncoding: "br" | "gzip" | undefined;
  for (const candidate of candidates) {
    const obj = await bucket.get(candidate.key);
    if (obj) {
      matchedObject = obj;
      matchedEncoding = candidate.encoding;
      break;
    }
  }

  if (!matchedObject) {
    if (resolved.relPath === "index.html" || resolved.relPath === "manifest.json") {
      return Response.json(
        {
          ok: false,
          error: "web_bundle_not_published",
          app: resolved.app,
          expectedKey: resolved.r2Key,
          hint: "Run `bun tooling/bun/web-build.ts build --package aphrody_desktop --release --opt --slim --webgpu-only` then `bun tooling/bun/cloudflare-deploy.ts publish-web`.",
        },
        { status: 503, headers: { "Cache-Control": "no-store" } },
      );
    }
    return new Response("Not Found", {
      status: 404,
      headers: { "Cache-Control": "no-store", "X-Content-Type-Options": "nosniff" },
    });
  }

  const headers = new Headers({
    "Content-Type": CONTENT_TYPES[ext] ?? "application/octet-stream",
    "X-Content-Type-Options": "nosniff",
    "Access-Control-Allow-Origin": "*",
    "Cross-Origin-Resource-Policy": "cross-origin",
    Vary: "Accept-Encoding",
  });
  matchedObject.writeHttpMetadata?.(headers);
  // Ensure canonical MIME types even if R2 object metadata defaulted to octet-stream.
  if (CONTENT_TYPES[ext]) {
    headers.set("Content-Type", CONTENT_TYPES[ext]);
  }
  if (matchedEncoding) {
    headers.set("Content-Encoding", matchedEncoding);
  }
  if (shouldEnableCrossOriginIsolation(url, resolved.relPath)) {
    headers.set("Cross-Origin-Opener-Policy", "same-origin");
    headers.set("Cross-Origin-Embedder-Policy", "require-corp");
  }

  const firstSeg = resolved.relPath.split("/")[0] ?? "";
  const isImmutableCohortAsset = COHORT_DIRS.has(firstSeg) && (ext === ".wasm" || ext === ".js" || ext === ".mjs");
  headers.set(
    "Cache-Control",
    isImmutableCohortAsset ? "public, max-age=31536000, immutable" : "public, max-age=60, must-revalidate",
  );

  if (matchedObject.httpEtag) {
    headers.set("etag", matchedObject.httpEtag);
    if (request.headers.get("if-none-match") === matchedObject.httpEtag) {
      return new Response(null, { status: 304, headers });
    }
  }

  const init: ResponseInit & { encodeBody?: "automatic" | "manual" } = { headers };
  if (matchedEncoding) {
    init.encodeBody = "manual";
  }
  return new Response(request.method === "HEAD" ? null : matchedObject.body, init);
}
