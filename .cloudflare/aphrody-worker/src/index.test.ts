import { describe, expect, test } from "bun:test";
import worker, {
  classify,
  detectPlatform,
  FLEET_TELEMETRY_KEY,
  LEARN_BUNDLE_KEY,
  LEARN_MANIFEST_KEY,
  renderDownloadPortal,
  routeSurface,
  signFleetPayload,
  verify,
  type WorkerEnv,
} from "./index.ts";

async function signGithub(secret: string, body: string): Promise<string> {
  const key = await crypto.subtle.importKey(
    "raw",
    new TextEncoder().encode(secret),
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"],
  );
  const sig = new Uint8Array(await crypto.subtle.sign("HMAC", key, new TextEncoder().encode(body)));
  return `sha256=${[...sig].map((b) => b.toString(16).padStart(2, "0")).join("")}`;
}

describe("unified aphrody-worker (releases + docs + r2-assets + github)", () => {
  const sentEvents: unknown[] = [];
  const env: WorkerEnv = {
    GITHUB_WEBHOOK_SECRET: "test-secret-key",
    RELEASES_BUCKET: {
      async get(key: string) {
        if (key === "channels/stable/latest.json") {
          return { body: JSON.stringify({ version: "1.30.0" }) };
        }
        if (key === "scripts/install.sh") {
          return { body: "#!/usr/bin/env sh\necho aphrody\n" };
        }
        return null;
      },
    },
    ASSETS_BUCKET: {
      async get(key: string) {
        if (key === "v1/design-reference/abc.png") {
          return { body: "png-data", httpEtag: '"etag-abc"' };
        }
        return null;
      },
    },
    EVENTS: {
      async send(msg) {
        sentEvents.push(msg);
      },
    },
  };

  test("routes all 6 surfaces cleanly by hostname and path", () => {
    expect(routeSurface(new URL("https://releases.aphrody.com/download"))).toBe("releases");
    expect(routeSurface(new URL("https://docs.aphrody.com/"))).toBe("docs");
    expect(routeSurface(new URL("https://assets.aphrody.app/v1/design-reference/abc.png"))).toBe("assets");
    expect(routeSurface(new URL("https://aphrody.com/r2-assets/v1/design-reference/abc.png"))).toBe("assets");
    expect(routeSurface(new URL("https://aphrody.com/github"))).toBe("github");
    expect(routeSurface(new URL("https://web.aphrody.com/"))).toBe("web");
    expect(routeSurface(new URL("https://aphrody.com/web/baseline/web_os_bg.wasm"))).toBe("web");
    expect(routeSurface(new URL("https://learn.aphrody.com/api/learn/status"))).toBe("learn");
  });

  test("releases portal, channels, and healthz work identically", async () => {
    expect(detectPlatform("Mozilla/5.0 (Windows NT 10.0; Win64; x64)").asset).toBe("Aphrody-Setup-x64.exe");
    const portal = await renderDownloadPortal(new Request("https://releases.aphrody.com/download?channel=dev"));
    expect(portal.status).toBe(200);
    expect(await portal.text()).toContain("APHRODY_CHANNEL=dev");

    const health = await worker.fetch(new Request("https://releases.aphrody.com/healthz"), env);
    expect(health.status).toBe(200);
    const json = (await health.json()) as { service: string; surfaces: string[] };
    expect(json.service).toBe("aphrody-worker");
    expect(json.surfaces).toEqual(["releases", "docs", "assets", "github", "learn", "web"]);
  });

  test("docs surface negotiates markdown for AI agents", async () => {
    const mdRes = await worker.fetch(
      new Request("https://docs.aphrody.com/", { headers: { Accept: "text/markdown" } }),
      env,
    );
    expect(mdRes.status).toBe(200);
    expect(mdRes.headers.get("content-type")).toContain("text/markdown");
    expect(await mdRes.text()).toContain("# Aphrody Documentation");
  });

  test("r2 assets surface enforces path traversal checks, immutable caching, and 304 ETag", async () => {
    const bad = await worker.fetch(new Request("https://assets.aphrody.app/../etc/passwd"), env);
    expect(bad.status).toBe(404);

    const hit = await worker.fetch(new Request("https://assets.aphrody.app/v1/design-reference/abc.png"), env);
    expect(hit.status).toBe(200);
    expect(hit.headers.get("cache-control")).toBe("public, max-age=31536000, immutable");
    expect(hit.headers.get("etag")).toBe('"etag-abc"');

    const notModified = await worker.fetch(
      new Request("https://assets.aphrody.app/v1/design-reference/abc.png", {
        headers: { "If-None-Match": '"etag-abc"' },
      }),
      env,
    );
    expect(notModified.status).toBe(304);
  });

  test("github webhook verifies HMAC SHA-256 and classifies owner push_main events", async () => {
    const payload = JSON.stringify({
      ref: "refs/heads/main",
      after: "1234567890abcdef",
      repository: { full_name: "aphrody-dev/aphrody", default_branch: "main", owner: { login: "aphrody-dev" } },
      head_commit: { message: "feat: consolidate workers" },
    });
    const sig = await signGithub("test-secret-key", payload);
    expect(await verify("test-secret-key", new TextEncoder().encode(payload).buffer, sig)).toBe(true);
    expect(classify("push", "deliv-1", JSON.parse(payload))?.kind).toBe("push_main");

    const badSig = await worker.fetch(
      new Request("https://aphrody.com/github", { method: "POST", body: payload, headers: { "x-hub-signature-256": "sha256=" + "0".repeat(64) } }),
      env,
    );
    expect(badSig.status).toBe(401);

    const ok = await worker.fetch(
      new Request("https://aphrody.com/github", {
        method: "POST",
        body: payload,
        headers: { "x-hub-signature-256": sig, "x-github-event": "push", "x-github-delivery": "deliv-1" },
      }),
      env,
    );
    expect(ok.status).toBe(202);
    expect(sentEvents.length).toBe(1);
  });

  test("fleet endpoints return deterministic defaults when R2 telemetry is absent", async () => {
    const statusRes = await worker.fetch(new Request("https://releases.aphrody.com/api/fleet/status"), {});
    expect(statusRes.status).toBe(200);
    const statusJson = (await statusRes.json()) as {
      ok: boolean;
      source: string;
      totalNodes: number;
      nodes: unknown[];
    };
    expect(statusJson.ok).toBe(true);
    expect(statusJson.source).toBe("default");
    expect(statusJson.totalNodes).toBe(0);
    expect(statusJson.nodes).toEqual([]);

    const routeRes = await worker.fetch(
      new Request("https://releases.aphrody.com/api/fleet/route?job=native_rust&shards=2"),
      {},
    );
    expect(routeRes.status).toBe(200);
    const routeJson = (await routeRes.json()) as {
      ok: boolean;
      selected: unknown;
      shards: unknown[];
      reason: string;
    };
    expect(routeJson.ok).toBe(true);
    expect(routeJson.selected).toBeNull();
    expect(routeJson.shards).toEqual([]);
    expect(routeJson.reason).toBe("no_matching_telemetry_nodes");
  });

  test("fleet beacon authenticates with Bearer or HMAC-SHA256, persists to R2, and routes lightest capable nodes", async () => {
    const store = new Map<string, string>();
    const fleetEnv: WorkerEnv = {
      FLEET_BEACON_TOKEN: "fleet-secret-token",
      ASSETS_BUCKET: {
        async get(key: string) {
          const val = store.get(key);
          return val ? { body: val } : null;
        },
        async put(key: string, value: string | Uint8Array | ArrayBuffer | ReadableStream) {
          store.set(key, typeof value === "string" ? value : new TextDecoder().decode(value as Uint8Array));
        },
      },
    };

    const telemetryBody = JSON.stringify({
      sourceHost: "orchestrator",
      ttlSec: 300,
      nodes: [
        {
          alias: "fr1",
          region: "eu-west",
          tier: "builder",
          reachable: true,
          healthy: true,
          cpus: 16,
          load1: 8.0,
          memTotalGb: 64,
          memAvailGb: 40,
          workAvailGb: 180,
          activeJobs: 1,
          rttMs: 14,
          capabilities: ["xfs_reflink", "native_rust", "wasm"],
          score: 68,
        },
        {
          alias: "fr2",
          region: "eu-west",
          tier: "verifier",
          reachable: true,
          healthy: true,
          cpus: 16,
          load1: 1.2,
          memTotalGb: 64,
          memAvailGb: 56,
          workAvailGb: 200,
          activeJobs: 0,
          rttMs: 18,
          capabilities: ["xfs_reflink", "wasm", "native_rust"],
          score: 91,
        },
        {
          alias: "prd",
          region: "us-edge",
          tier: "edge",
          reachable: true,
          healthy: true,
          cpus: 8,
          load1: 0.4,
          memTotalGb: 32,
          memAvailGb: 28,
          workAvailGb: 220,
          activeJobs: 0,
          rttMs: 25,
          capabilities: ["btrfs_reflink", "edge_proxy", "wasm"],
          score: 94,
        },
      ],
    });

    const unauthRes = await worker.fetch(
      new Request("https://releases.aphrody.com/api/fleet/beacon", {
        method: "POST",
        body: telemetryBody,
      }),
      fleetEnv,
    );
    expect(unauthRes.status).toBe(401);

    const sig = await signFleetPayload("fleet-secret-token", telemetryBody);
    const hmacRes = await worker.fetch(
      new Request("https://releases.aphrody.com/api/fleet/beacon", {
        method: "POST",
        body: telemetryBody,
        headers: { "x-fleet-signature-256": sig, "content-type": "application/json" },
      }),
      fleetEnv,
    );
    expect(hmacRes.status).toBe(200);
    const hmacJson = (await hmacRes.json()) as {
      ok: boolean;
      persisted: boolean;
      authMode: string;
      totalNodes: number;
    };
    expect(hmacJson.ok).toBe(true);
    expect(hmacJson.persisted).toBe(true);
    expect(hmacJson.authMode).toBe("hmac-sha256");
    expect(hmacJson.totalNodes).toBe(3);
    expect(store.has(FLEET_TELEMETRY_KEY)).toBe(true);

    const statusRes = await worker.fetch(new Request("https://releases.aphrody.com/api/fleet/status"), fleetEnv);
    expect(statusRes.status).toBe(200);
    const statusJson = (await statusRes.json()) as { source: string; healthyNodes: number };
    expect(statusJson.source).toBe("r2");
    expect(statusJson.healthyNodes).toBe(3);

    const rustRoute = await worker.fetch(
      new Request("https://releases.aphrody.com/api/fleet/route?job=native_rust&shards=2"),
      fleetEnv,
    );
    expect(rustRoute.status).toBe(200);
    const rustJson = (await rustRoute.json()) as {
      selected: { alias: string } | null;
      fallbackChain: string[];
      shards: Array<{ host: string }>;
    };
    expect(rustJson.selected?.alias).toBe("fr2");
    expect(rustJson.fallbackChain).toEqual(["fr1"]);
    expect(rustJson.shards.map((s) => s.host)).toEqual(["fr2", "fr1"]);

    const excludeRoute = await worker.fetch(
      new Request("https://releases.aphrody.com/api/fleet/route?job=native_rust&exclude=fr2"),
      fleetEnv,
    );
    const excludeJson = (await excludeRoute.json()) as { selected: { alias: string } | null };
    expect(excludeJson.selected?.alias).toBe("fr1");

    const bearerUpdate = await worker.fetch(
      new Request("https://releases.aphrody.com/api/fleet/beacon", {
        method: "POST",
        body: JSON.stringify({
          merge: true,
          alias: "fr1",
          reachable: true,
          healthy: true,
          cpus: 16,
          load1: 0.2,
          memTotalGb: 64,
          memAvailGb: 60,
          workAvailGb: 180,
          capabilities: ["xfs_reflink", "native_rust", "wasm"],
          score: 98,
        }),
        headers: { authorization: "Bearer fleet-secret-token" },
      }),
      fleetEnv,
    );
    expect(bearerUpdate.status).toBe(200);
    const bearerJson = (await bearerUpdate.json()) as { authMode: string; totalNodes: number };
    expect(bearerJson.authMode).toBe("bearer");
    expect(bearerJson.totalNodes).toBe(3);

    const edgeRoute = await worker.fetch(
      new Request("https://releases.aphrody.com/api/fleet/route?job=edge_proxy"),
      fleetEnv,
    );
    const edgeJson = (await edgeRoute.json()) as { selected: { alias: string } | null };
    expect(edgeJson.selected?.alias).toBe("prd");
  });

  test("eliminates 404s across /api/releases, /r2-assets/, and GET /github", async () => {
    const store = new Map<string, string>([["v1/design-reference/abc.png", "png"]]);
    const liveEnv: WorkerEnv = {
      RELEASES_BUCKET: {
        async get() {
          return null;
        },
      },
      ASSETS_BUCKET: {
        async get(k: string) {
          const val = store.get(k);
          return val ? { body: val } : null;
        },
        async list() {
          return { objects: [...store.keys()].map((key) => ({ key })), truncated: false };
        },
      },
      EVENTS: env.EVENTS,
    };

    const releasesIndex = await worker.fetch(new Request("https://releases.aphrody.com/api/releases"), liveEnv);
    expect(releasesIndex.status).toBe(200);
    const releasesJson = (await releasesIndex.json()) as { ok: boolean; channels: Array<{ channel: string }> };
    expect(releasesJson.ok).toBe(true);
    expect(releasesJson.channels.length).toBe(6);

    const stableLatest = await worker.fetch(
      new Request("https://releases.aphrody.com/api/releases/stable/latest"),
      liveEnv,
    );
    expect(stableLatest.status).toBe(200);
    const stableJson = (await stableLatest.json()) as { channel: string; status: string };
    expect(stableJson.channel).toBe("stable");

    const assetsRoot = await worker.fetch(new Request("https://aphrody.com/r2-assets/"), liveEnv);
    expect(assetsRoot.status).toBe(200);
    const assetsJson = (await assetsRoot.json()) as { ok: boolean; keys: string[] };
    expect(assetsJson.ok).toBe(true);
    expect(assetsJson.keys).toContain("v1/design-reference/abc.png");

    const githubGet = await worker.fetch(new Request("https://aphrody.com/github"), liveEnv);
    expect(githubGet.status).toBe(200);
    const githubJson = (await githubGet.json()) as { ok: boolean; accepts: string };
    expect(githubJson.ok).toBe(true);
    expect(githubJson.accepts).toBe("POST");

    const autoLatest = await worker.fetch(
      new Request("https://releases.aphrody.com/api/releases/auto/latest"),
      liveEnv,
    );
    expect(autoLatest.status).toBe(200);
    const autoJson = (await autoLatest.json()) as { product: string; prefix: string };
    expect(autoJson.product).toBe("auto");
    expect(autoJson.prefix).toBe("auto/releases/");

    const topologyRes = await worker.fetch(
      new Request("https://releases.aphrody.com/api/cloudflare/topology"),
      liveEnv,
    );
    expect(topologyRes.status).toBe(200);
    const topoJson = (await topologyRes.json()) as { ok: boolean; workers: Array<{ name: string }> };
    expect(topoJson.ok).toBe(true);
    expect(topoJson.workers.map((w) => w.name)).toEqual(["aphrody-worker", "aphrody-learn", "auto"]);
  });

  test("serverless autonomous learn ingests events, distills via cron, promotes without human approval, and serves conditional bundles", async () => {
    const r2 = new Map<string, string>();
    const learnEvents: unknown[] = [];
    const learnEnv: WorkerEnv = {
      LEARN_SYNC_TOKEN: "learn-secret-token",
      ASSETS_BUCKET: {
        async get(key: string) {
          const val = r2.get(key);
          return val !== undefined ? { body: val, httpEtag: `"etag-${key}"` } : null;
        },
        async put(key: string, value: string | Uint8Array | ArrayBuffer | ReadableStream) {
          r2.set(key, typeof value === "string" ? value : new TextDecoder().decode(value as Uint8Array));
        },
        async list(opts?: { prefix?: string; limit?: number }) {
          const prefix = opts?.prefix ?? "";
          const keys = [...r2.keys()].filter((k) => k.startsWith(prefix)).slice(0, opts?.limit ?? 100);
          return { objects: keys.map((key) => ({ key })), truncated: false };
        },
        async delete(keys: string | string[]) {
          for (const k of Array.isArray(keys) ? keys : [keys]) r2.delete(k);
        },
      },
      EVENTS: {
        async send(msg) {
          learnEvents.push(msg);
        },
      },
    };

    expect(routeSurface(new URL("https://learn.aphrody.com/api/learn/status"))).toBe("learn");

    const unauthPost = await worker.fetch(
      new Request("https://learn.aphrody.com/api/learn/events", { method: "POST", body: "{}" }),
      learnEnv,
    );
    expect(unauthPost.status).toBe(401);

    const eventsBody = JSON.stringify({
      harness: "claude",
      pcId: "win-ws-1",
      mcpKeep: ["relay", "bun-brain"],
      mcpDisable: ["vercel", "rea"],
      entries: [
        {
          at: "2026-11-18T15:30:00Z",
          harness: "claude",
          pcId: "win-ws-1",
          kind: "correction",
          tool: "session_inbox",
          summary: "Always drain relay session_inbox on every swarm_wait wakeup.",
        },
        {
          at: "2026-11-18T15:31:00Z",
          harness: "agy",
          pcId: "linux-fr1",
          kind: "recovery",
          tool: "run_command",
          summary: "Recovered bun test ./.cloudflare/aphrody-worker/src/index.test.ts with ./ prefix.",
        },
      ],
    });
    const postRes = await worker.fetch(
      new Request("https://learn.aphrody.com/api/learn/events?distill=1", {
        method: "POST",
        body: eventsBody,
        headers: {
          authorization: "Bearer learn-secret-token",
          "content-type": "application/json",
        },
      }),
      learnEnv,
    );
    expect(postRes.status).toBe(200);
    const postJson = (await postRes.json()) as {
      ok: boolean;
      accepted: number;
      distilled: { revision: number; promotedTopics: number };
    };
    expect(postJson.ok).toBe(true);
    expect(postJson.accepted).toBe(2);
    expect(postJson.distilled.revision).toBe(1);
    expect(postJson.distilled.promotedTopics).toBe(2);
    expect(r2.has(LEARN_BUNDLE_KEY)).toBe(true);
    expect(r2.has(LEARN_MANIFEST_KEY)).toBe(true);
    expect(learnEvents.length).toBe(1);

    const bundleRes = await worker.fetch(
      new Request("https://learn.aphrody.com/api/learn/bundle.json", {
        headers: { authorization: "Bearer learn-secret-token" },
      }),
      learnEnv,
    );
    expect(bundleRes.status).toBe(200);
    const bundleJson = (await bundleRes.json()) as {
      revision: number;
      sha256: string;
      sources: { harnesses: string[]; pcs: string[] };
      memory: { topics: Record<string, { bullets: string[] }> };
      skills: { deltas: Record<string, { notes: string[] }> };
      mcp: { profile: { keepBaseline: string[]; disableRecommended: string[] } };
    };
    expect(bundleJson.sources.harnesses).toContain("claude");
    expect(bundleJson.sources.pcs).toContain("win-ws-1");
    expect(bundleJson.memory.topics["cross-session"]?.bullets.length).toBe(1);
    expect(bundleJson.skills.deltas["bun-gpui"]?.notes.length).toBe(1);
    expect(bundleJson.mcp.profile.keepBaseline).toEqual(["relay", "bun-brain"]);

    const notModifiedRes = await worker.fetch(
      new Request("https://learn.aphrody.com/api/learn/manifest.json?since=1", {
        headers: { authorization: "Bearer learn-secret-token" },
      }),
      learnEnv,
    );
    expect(notModifiedRes.status).toBe(304);

    // Cron scheduled() drains any new inbox shard without human approval.
    r2.set(
      "learn/inbox/manual-shard.json",
      JSON.stringify({
        shardId: "manual-shard",
        receivedAt: "2026-11-18T15:35:00Z",
        harness: "codex",
        agent: "codex",
        pcId: "linux-fr2",
        entries: [
          {
            at: "2026-11-18T15:35:00Z",
            harness: "codex",
            pcId: "linux-fr2",
            kind: "correction",
            tool: "wrangler",
            summary: "Never hardcode account_id or API tokens inside wrangler.toml.",
          },
        ],
      }),
    );
    await worker.scheduled?.({ cron: "*/15 * * * *" }, learnEnv);
    expect(r2.has("learn/inbox/manual-shard.json")).toBe(false);
    const statusAfter = await worker.fetch(new Request("https://learn.aphrody.com/api/learn/status"), learnEnv);
    const statusJson = (await statusAfter.json()) as {
      revision: number;
      counts: { memoryTopics: number; harnesses: number };
    };
    expect(statusJson.revision).toBe(2);
    expect(statusJson.counts.memoryTopics).toBeGreaterThanOrEqual(3);
  });

  test("web surface serves Wasm/WebGPU cohorts with Brotli negotiation, MIME types, caching, and conditional COOP/COEP", async () => {
    const webObjects = new Map<string, { body: string; httpEtag: string }>([
      ["web/aphrody_desktop/index.html", { body: "<!doctype html><html></html>", httpEtag: '"idx-1"' }],
      ["web/aphrody_desktop/simd-baseline/web_os_bg.wasm.br", { body: "br-wasm-bytes", httpEtag: '"wasm-br-1"' }],
      ["web/aphrody_desktop/simd-baseline/web_os_bg.wasm", { body: "raw-wasm-bytes", httpEtag: '"wasm-raw-1"' }],
      ["web/aphrody_desktop/multithreaded/web_os_bg.wasm.br", { body: "mt-br-wasm", httpEtag: '"mt-br-1"' }],
    ]);
    const webEnv: WorkerEnv = {
      ASSETS_BUCKET: {
        async get(key: string) {
          return webObjects.get(key) ?? null;
        },
      },
    };

    const badTraversal = await worker.fetch(new Request("https://web.aphrody.com/../secret"), webEnv);
    expect(badTraversal.status).toBe(404);

    const wasmBr = await worker.fetch(
      new Request("https://web.aphrody.com/simd-baseline/web_os_bg.wasm", {
        headers: { "Accept-Encoding": "gzip, br" },
      }),
      webEnv,
    );
    expect(wasmBr.status).toBe(200);
    expect(wasmBr.headers.get("content-type")).toBe("application/wasm");
    expect(wasmBr.headers.get("content-encoding")).toBe("br");
    expect(wasmBr.headers.get("vary")).toBe("Accept-Encoding");
    expect(wasmBr.headers.get("cache-control")).toBe("public, max-age=31536000, immutable");
    expect(wasmBr.headers.get("cross-origin-opener-policy")).toBeNull();
    expect(await wasmBr.text()).toBe("br-wasm-bytes");

    const notMod = await worker.fetch(
      new Request("https://web.aphrody.com/simd-baseline/web_os_bg.wasm", {
        headers: { "Accept-Encoding": "br", "If-None-Match": '"wasm-br-1"' },
      }),
      webEnv,
    );
    expect(notMod.status).toBe(304);

    const mtRes = await worker.fetch(
      new Request("https://aphrody.com/web/multithreaded/web_os_bg.wasm", {
        headers: { "Accept-Encoding": "br" },
      }),
      webEnv,
    );
    expect(mtRes.status).toBe(200);
    expect(mtRes.headers.get("cross-origin-opener-policy")).toBe("same-origin");
    expect(mtRes.headers.get("cross-origin-embedder-policy")).toBe("require-corp");

    const coiIndex = await worker.fetch(new Request("https://web.aphrody.com/?coi=1"), webEnv);
    expect(coiIndex.status).toBe(200);
    expect(coiIndex.headers.get("content-type")).toBe("text/html; charset=utf-8");
    expect(coiIndex.headers.get("cross-origin-opener-policy")).toBe("same-origin");
    expect(coiIndex.headers.get("cross-origin-embedder-policy")).toBe("require-corp");

    const missing = await worker.fetch(new Request("https://web.aphrody.com/"), { ASSETS_BUCKET: { async get() { return null; } } });
    expect(missing.status).toBe(503);
  });
});
