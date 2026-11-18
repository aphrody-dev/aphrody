import { describe, expect, test } from "bun:test";
import worker, { classify, detectPlatform, renderDownloadPortal, routeSurface, verify, type WorkerEnv } from "./index.ts";

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

  test("routes all 4 surfaces cleanly by hostname and path", () => {
    expect(routeSurface(new URL("https://releases.aphrody.com/download"))).toBe("releases");
    expect(routeSurface(new URL("https://docs.aphrody.com/"))).toBe("docs");
    expect(routeSurface(new URL("https://assets.aphrody.app/v1/design-reference/abc.png"))).toBe("assets");
    expect(routeSurface(new URL("https://aphrody.com/r2-assets/v1/design-reference/abc.png"))).toBe("assets");
    expect(routeSurface(new URL("https://aphrody.com/github"))).toBe("github");
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
    expect(json.surfaces).toEqual(["releases", "docs", "assets", "github"]);
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
});
