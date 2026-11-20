import { describe, expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { checkRepoHygiene } from "./repo-hygiene.ts";
import { CROSS_MATRIX } from "./cross-build.ts";
import releaseWorker, { detectPlatform, renderDownloadPortal } from "../../.cloudflare/aphrody-worker/src/index.ts";

const repoRoot = resolve(import.meta.dir, "../..");

describe("aphrody repo-native bun tooling", () => {
  test("validates pinned aphrody-labs forks manifest and tags", () => {
    const manifest = JSON.parse(
      readFileSync(resolve(repoRoot, "tooling/bun/forks-manifest.json"), "utf8"),
    ) as { forks: Array<{ repo: string; tag: string; sha: string; gitUrl: string }> };
    expect(manifest.forks.length).toBe(11);
    for (const fork of manifest.forks) {
      expect(fork.repo.startsWith("aphrody-labs/")).toBe(true);
      expect(fork.sha).toMatch(/^[0-9a-f]{40}$/);
      expect(fork.tag.length).toBeGreaterThan(0);
    }
  });

  test("post-edit-check allows repo paths and flags brain/temp paths", () => {
    const okRun = Bun.spawnSync(["bun", ".agents/plugins/aphrody/hooks/post-edit-check.ts"], {
      cwd: repoRoot,
      stdin: Buffer.from(JSON.stringify({ toolCall: { name: "write_file", args: { Path: "crates/example/src/lib.rs" } } })),
    });
    expect(okRun.exitCode).toBe(0);
    expect(JSON.parse(new TextDecoder().decode(okRun.stdout))).toEqual({});

    const brainRun = Bun.spawnSync(["bun", ".agents/plugins/aphrody/hooks/post-edit-check.ts"], {
      cwd: repoRoot,
      stdin: Buffer.from(
        JSON.stringify({
          toolCall: {
            name: "write_file",
            args: { Path: "C:/Users/example/.gemini/antigravity-cli/brain/123/plan.md" },
          },
        }),
      ),
    });
    expect(brainRun.exitCode).toBe(0);
    const msg = JSON.parse(new TextDecoder().decode(brainRun.stdout)) as { systemMessage?: string };
    expect(msg.systemMessage).toContain("docs/aphrody/architecture/");
  });

  test("CROSS_MATRIX covers cargo-xwin, cargo-zigbuild, android, wasm32, and wasip2", () => {
    const drivers = new Set(CROSS_MATRIX.map((s) => s.driver));
    expect(drivers.has("cargo-xwin")).toBe(true);
    expect(drivers.has("cargo-zigbuild")).toBe(true);
    expect(CROSS_MATRIX.some((s) => s.target === "wasm32-unknown-unknown")).toBe(true);
    expect(CROSS_MATRIX.some((s) => s.target === "wasm32-wasip2")).toBe(true);
  });

  test("cloudflare release portal routes download, latest channel manifest, and assets", async () => {
    const mac = detectPlatform("Mozilla/5.0 (Macintosh; Apple Silicon Mac OS X)");
    expect(mac.os).toBe("macos");
    expect(mac.asset).toBe("Aphrody-arm64.dmg");

    const portalResp = await renderDownloadPortal(
      new Request("https://releases.aphrody.com/download?channel=canary", {
        headers: { "User-Agent": "Mozilla/5.0 (X11; Linux x86_64)" },
      }),
    );
    const html = await portalResp.text();
    expect(portalResp.status).toBe(200);
    expect(html).toContain("/assets/canary/latest/aphrody-x86_64.tar.gz");
    expect(html).toContain("APHRODY_CHANNEL=canary");

    const mockEnv = {
      RELEASES_BUCKET: {
        async get(key: string) {
          if (key === "channels/lts/latest.json") {
            return { body: JSON.stringify({ version: "1.0.0", url: "https://releases.aphrody.com/assets/lts/1.0.0/Aphrody-Setup-x64.exe" }) };
          }
          if (key === "manifests/canary/2026.11.16/windows-x86_64.json") {
            return { body: JSON.stringify({ version: "2026.11.16", sha256: "abc123" }) };
          }
          return null;
        },
      },
    };

    const latestResp = await releaseWorker.fetch(new Request("https://releases.aphrody.com/api/releases/lts/latest"), mockEnv);
    expect(latestResp.status).toBe(200);
    expect((await latestResp.json()) as { version: string }).toEqual({
      version: "1.0.0",
      url: "https://releases.aphrody.com/assets/lts/1.0.0/Aphrody-Setup-x64.exe",
    });

    const assetManifestResp = await releaseWorker.fetch(
      new Request("https://releases.aphrody.com/releases/canary/2026.11.16/asset?os=windows&arch=x86_64"),
      mockEnv,
    );
    expect(assetManifestResp.status).toBe(200);
    expect((await assetManifestResp.json()) as { sha256: string }).toEqual({
      version: "2026.11.16",
      sha256: "abc123",
    });
  });

  test("all Aphrody GitHub workflows parse cleanly as YAML", () => {
    const workflowDir = resolve(repoRoot, ".github/workflows");
    const files = readdirSync(workflowDir).filter((f) => f.startsWith("aphrody-") && (f.endsWith(".yml") || f.endsWith(".yaml")));
    expect(files.length).toBeGreaterThanOrEqual(5);
    for (const file of files) {
      const parsed = Bun.YAML.parse(readFileSync(resolve(workflowDir, file), "utf8"));
      expect(parsed).toBeTruthy();
    }
  });

  test("repo hygiene passes with zero ephemeral brain references or broken doc links", () => {
    const report = checkRepoHygiene(repoRoot);
    expect(report.readmeGuardOk).toBe(true);
    expect(report.ephemeralLeaks).toEqual([]);
    expect(report.brokenArchLinks).toEqual([]);
    expect(report.invalidManifests).toEqual([]);
    expect(report.ok).toBe(true);
  });
});
