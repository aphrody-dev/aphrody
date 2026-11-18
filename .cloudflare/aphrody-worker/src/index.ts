import { handleAssetsRequest } from "./assets.ts";
import { handleDocsRequest, LLMS_TXT, markdownPathFor } from "./docs.ts";
import { classify, handleGithubRequest, OWNERS, verify } from "./github.ts";
import { detectPlatform, handleReleasesRequest, renderDownloadPortal, VALID_CHANNELS } from "./releases.ts";
import type { WorkerEnv } from "./types.ts";

export { classify, detectPlatform, LLMS_TXT, markdownPathFor, OWNERS, renderDownloadPortal, VALID_CHANNELS, verify };
export type { WorkerEnv };

export type SurfaceKind = "releases" | "docs" | "assets" | "github";

export function routeSurface(url: URL): SurfaceKind {
  const host = url.hostname.toLowerCase();
  const path = url.pathname;
  if (path === "/github") return "github";
  if (path.startsWith("/r2-assets/") || host === "assets.aphrody.com" || host === "assets.aphrody.app") {
    return "assets";
  }
  if (host === "docs.aphrody.com" || path === "/llms.txt" || path === "/docs/llms.txt" || path === "/docs.md" || path === "/docs" || path.startsWith("/docs/")) {
    return "docs";
  }
  return "releases";
}

export default {
  async fetch(request: Request, env: WorkerEnv): Promise<Response> {
    const url = new URL(request.url);
    const path = url.pathname;
    const surface = routeSurface(url);

    if (path === "/healthz" || path === "/health" || path === "/api/health") {
      return Response.json(
        {
          ok: true,
          service: "aphrody-worker",
          surface,
          surfaces: ["releases", "docs", "assets", "github"],
          channels: [...VALID_CHANNELS],
          buckets: { releases: "aphrody-releases", assets: "aphrody-assets" },
        },
        { headers: { "Cache-Control": "no-store" } },
      );
    }

    switch (surface) {
      case "github":
        return handleGithubRequest(request, env);
      case "assets":
        return handleAssetsRequest(request, env, path.startsWith("/r2-assets/") ? "/r2-assets" : "");
      case "docs":
        return handleDocsRequest(request, env);
      case "releases":
        return handleReleasesRequest(request, env);
    }
  },
};
