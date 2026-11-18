import type { WorkerEnv } from "./types.ts";

export async function handleAssetsRequest(request: Request, env: WorkerEnv, keyPrefixStrip = ""): Promise<Response> {
  const url = new URL(request.url);
  let rawPath = url.pathname;
  if (keyPrefixStrip && rawPath.startsWith(keyPrefixStrip)) {
    rawPath = rawPath.slice(keyPrefixStrip.length);
  }
  const key = rawPath.replace(/^\/+/, "");
  if (!key || key.includes("..") || key.includes("\0") || key.includes("\\")) {
    return new Response("Not Found", {
      status: 404,
      headers: { "Cache-Control": "no-store", "X-Content-Type-Options": "nosniff" },
    });
  }

  const bucket = env.ASSETS_BUCKET ?? env.OPEN_SOURCE_WEBSITE_ASSETS_BUCKET;
  const object = await bucket?.get(key);
  if (!object) {
    return new Response("Not Found", {
      status: 404,
      headers: { "Cache-Control": "no-store", "X-Content-Type-Options": "nosniff" },
    });
  }

  const headers = new Headers({
    "X-Content-Type-Options": "nosniff",
    "Access-Control-Allow-Origin": "*",
  });
  object.writeHttpMetadata?.(headers);
  if (object.httpEtag) {
    headers.set("etag", object.httpEtag);
    if (request.headers.get("if-none-match") === object.httpEtag) {
      return new Response(null, { status: 304, headers });
    }
  }
  if (!headers.has("cache-control")) {
    headers.set(
      "cache-control",
      key.startsWith("v1/") ? "public, max-age=31536000, immutable" : "public, max-age=3600",
    );
  }

  return new Response(request.method === "HEAD" ? null : object.body, { headers });
}
