import { handleAssetsRequest } from "./assets.ts";
import { handleDocsRequest, LLMS_TXT, markdownPathFor } from "./docs.ts";
import { FLEET_TELEMETRY_KEY, handleFleetRequest, signFleetPayload, verifyFleetBeaconAuth } from "./fleet.ts";
import { classify, handleGithubRequest, OWNERS, verify } from "./github.ts";
import {
  buildLearnManifest,
  defaultLearnBundle,
  distillLearnState,
  foldShardsIntoBundle,
  handleLearnRequest,
  LEARN_BUNDLE_KEY,
  LEARN_MANIFEST_KEY,
} from "./learn.ts";
import { detectPlatform, handleReleasesRequest, renderDownloadPortal, VALID_CHANNELS } from "./releases.ts";
import type { ExecutionContextLike, ScheduledControllerLike, WorkerEnv } from "./types.ts";
import { handleWebRequest, resolveWebR2Key, shouldEnableCrossOriginIsolation } from "./web.ts";

export {
  buildLearnManifest,
  classify,
  defaultLearnBundle,
  detectPlatform,
  distillLearnState,
  FLEET_TELEMETRY_KEY,
  foldShardsIntoBundle,
  LEARN_BUNDLE_KEY,
  LEARN_MANIFEST_KEY,
  LLMS_TXT,
  markdownPathFor,
  OWNERS,
  renderDownloadPortal,
  resolveWebR2Key,
  shouldEnableCrossOriginIsolation,
  signFleetPayload,
  VALID_CHANNELS,
  verify,
  verifyFleetBeaconAuth,
};
export type { ExecutionContextLike, ScheduledControllerLike, WorkerEnv };

export type SurfaceKind = "releases" | "docs" | "assets" | "github" | "learn" | "web";

export function routeSurface(url: URL): SurfaceKind {
  const host = url.hostname.toLowerCase();
  const path = url.pathname;
  if (path === "/github") return "github";
  if (
    host === "web.aphrody.com" ||
    host === "web.aphrody.app" ||
    path === "/web" ||
    path.startsWith("/web/")
  ) {
    return "web";
  }
  if (
    host === "learn.aphrody.com" ||
    host === "learn.aphrody.app" ||
    path === "/learn" ||
    path.startsWith("/api/learn")
  ) {
    return "learn";
  }
  if (path.startsWith("/r2-assets/") || path === "/r2-assets" || host === "assets.aphrody.com" || host === "assets.aphrody.app") {
    return "assets";
  }
  if (host === "docs.aphrody.com" || path === "/llms.txt" || path === "/docs/llms.txt" || path === "/docs.md" || path === "/docs" || path.startsWith("/docs/")) {
    return "docs";
  }
  return "releases";
}

export default {
  async fetch(request: Request, env: WorkerEnv, ctx?: ExecutionContextLike): Promise<Response> {
    const url = new URL(request.url);
    const path = url.pathname;
    const surface = routeSurface(url);

    if (path === "/healthz" || path === "/health" || path === "/api/health") {
      return Response.json(
        {
          ok: true,
          service: "aphrody-worker",
          surface,
          surfaces: ["releases", "docs", "assets", "github", "learn", "web"],
          channels: [...VALID_CHANNELS],
          buckets: { releases: "aphrody-releases", assets: "aphrody-assets" },
        },
        { headers: { "Cache-Control": "no-store" } },
      );
    }

    if (path === "/api/fleet/status" || path === "/api/fleet/route" || path === "/api/fleet/beacon") {
      return handleFleetRequest(request, env);
    }

    if (path === "/api/cloudflare/topology") {
      return Response.json(
        {
          ok: true,
          zone: "aphrody.com",
          workers: [
            { name: "aphrody-worker", surfaces: ["releases", "docs", "assets", "github", "fleet", "learn", "web"], cron: "*/15 * * * *" },
            { name: "aphrody-learn", routes: ["learn.aphrody.com/*"], cron: "*/15 * * * *" },
            { name: "auto", routes: ["auto.aphrody.com"], cron: "0 6 * * *" },
          ],
          buckets: {
            "aphrody-assets": ["v1/", "web/", "fleet/", "learn/", "auto/assets/"],
            "aphrody-releases": ["scripts/", "channels/", "manifests/", "auto/releases/"],
            "aphrody-backups": ["auto/archives/"],
          },
        },
        { headers: { "Cache-Control": "public, max-age=60" } },
      );
    }

    if (surface === "learn") {
      return handleLearnRequest(request, env, ctx);
    }

    switch (surface) {
      case "github":
        return handleGithubRequest(request, env);
      case "web":
        return handleWebRequest(
          request,
          env,
          path === "/web" || path.startsWith("/web/") ? "/web" : "",
        );
      case "assets":
        return handleAssetsRequest(
          request,
          env,
          path === "/r2-assets" || path.startsWith("/r2-assets/") ? "/r2-assets" : "",
        );
      case "docs":
        return handleDocsRequest(request, env);
      case "releases":
        return handleReleasesRequest(request, env);
    }
  },

  async scheduled(_controller: ScheduledControllerLike, env: WorkerEnv, ctx?: ExecutionContextLike): Promise<void> {
    const work = distillLearnState(env);
    if (ctx?.waitUntil) {
      ctx.waitUntil(work);
      return;
    }
    await work;
  },
};
