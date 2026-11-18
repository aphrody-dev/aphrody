import { describe, expect, test } from "bun:test";
import worker, { detectPlatform, renderDownloadPortal } from "../../.cloudflare/aphrody-worker/src/index.ts";

describe("aphrody-worker consolidated Cloudflare portal", () => {
  const mockEnv = {
    RELEASES_BUCKET: {
      async get() {
        return null;
      },
    },
  };

  test("detectPlatform resolves Windows, macOS, and Linux binaries cleanly", () => {
    expect(detectPlatform("Mozilla/5.0 (Windows NT 10.0; Win64; x64)").asset).toBe("Aphrody-Setup-x64.exe");
    expect(detectPlatform("Mozilla/5.0 (Windows NT 10.0; ARM64)").asset).toBe("Aphrody-Setup-arm64.exe");
    expect(detectPlatform("Mozilla/5.0 (Macintosh; Intel Mac OS X)").asset).toBe("Aphrody-x64.dmg");
    expect(detectPlatform("Mozilla/5.0 (X11; Linux aarch64)").asset).toBe("aphrody-aarch64.tar.gz");
  });

  test("renderDownloadPortal includes Neural Expressive tokens, mobile targets, and all 6 channels", async () => {
    const resp = renderDownloadPortal(
      new Request("https://releases.aphrody.com/download?channel=dev", {
        headers: { "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)" },
      }),
    );
    expect(resp.status).toBe(200);
    const html = await resp.text();
    expect(html).toContain("--md-sys-color-primary");
    expect(html).toContain("Google Sans Flex");
    expect(html).toContain("/download?channel=dev\" aria-current=\"true\"");
    expect(html).toContain("Mobile Companion");
    expect(html).toContain("22-Target Store");
  });

  test("/api/channels, /api/design, and /healthz return structured JSON contracts", async () => {
    const chResp = await worker.fetch(new Request("https://releases.aphrody.com/api/channels"), mockEnv);
    expect(chResp.status).toBe(200);
    const chJson = (await chResp.json()) as { channels: string[] };
    expect(chJson.channels).toContain("dev");
    expect(chJson.channels.length).toBe(6);

    const designResp = await worker.fetch(new Request("https://releases.aphrody.com/api/design"), mockEnv);
    expect(designResp.status).toBe(200);
    const designJson = (await designResp.json()) as {
      source_of_truth: string;
      modes: string[];
      store_targets: number;
    };
    expect(designJson.source_of_truth).toBe("https://design.google/");
    expect(designJson.modes).toEqual(["neural", "dark", "light", "oled"]);
    expect(designJson.store_targets).toBe(22);

    const healthResp = await worker.fetch(new Request("https://releases.aphrody.com/healthz"), mockEnv);
    expect(healthResp.status).toBe(200);
  });
});
