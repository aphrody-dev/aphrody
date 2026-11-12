// SPDX-License-Identifier: Apache-2.0
import { describe, expect, it } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { server } from "../server";
import { ALL_APPS } from "../src/data/apps";

const APP_ROOT = join(import.meta.dir, "..");

describe("Aphrody Unified Sovereign Web App — Zero Stubs & Zero External Google Redirects", () => {
  it("purges all external *.google.com links from apps registry and UI components", () => {
    for (const app of ALL_APPS) {
      expect(app.url).not.toMatch(/google\./i);
    }

    for (const rel of [
      "src/components/logo.ts",
      "src/components/header.ts",
      "src/components/apps-popover.ts",
      "src/components/search-bar.ts",
      "src/components/lens-modal.ts",
      "src/components/footer.ts",
    ]) {
      const code = readFileSync(join(APP_ROOT, rel), "utf8");
      expect(code).not.toMatch(/https?:\/\/([a-z0-9-]+\.)*google\./i);
      expect(code).not.toContain("doodles.google");
    }
  });

  it("serves 3-VPS RRF Datacenter Search (/api/search) and WireGuard cluster status (/api/cluster/status)", async () => {
    const sRes = await server.fetch(
      new Request("http://localhost:3000/api/search?q=craft+wasm"),
    );
    expect(sRes.status).toBe(200);
    const sJson = (await sRes.json()) as {
      rrfConstantK: number;
      routedVia: {
        ingress: { id: string };
        crawlerReplica: { id: string };
        googleGrounding: { id: string };
      };
      knowledgeCard: { subdomain?: string } | null;
      hits: Array<{ rrfScore: number }>;
    };
    expect(sJson.rrfConstantK).toBe(60);
    expect(sJson.routedVia.ingress.id).toBe("fr1");
    expect(sJson.routedVia.crawlerReplica.id).toBe("fr2");
    expect(sJson.routedVia.googleGrounding.id).toBe("usa");
    expect(sJson.knowledgeCard?.subdomain).toBe("craft.aphrody.app");
    expect(sJson.hits.length).toBeGreaterThan(0);

    const cRes = await server.fetch(
      new Request("http://localhost:3000/api/cluster/status"),
    );
    expect(cRes.status).toBe(200);
    const cJson = (await cRes.json()) as {
      meshSubnet: string;
      nodes: unknown[];
    };
    expect(cJson.meshSubnet).toBe("10.200.77.0/24");
    expect(cJson.nodes.length).toBe(3);
  });

  it("executes real Craft Creative Cloud WASM RGBA, SVG Optimizer, RIFF WAV, and WAT->WASM endpoints", async () => {
    // 1. WASM RGBA invert
    const rawPx = Buffer.from([10, 20, 30, 255]).toString("base64");
    const rgbaRes = await server.fetch(
      new Request("http://localhost:3000/api/craft/rgba", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ rgbaBase64: rawPx, mode: "invert" }),
      }),
    );
    expect(rgbaRes.status).toBe(200);
    const rgbaJson = (await rgbaRes.json()) as { rgbaBase64: string };
    const outPx = [...Uint8Array.fromBase64(rgbaJson.rgbaBase64)];
    expect(outPx).toEqual([245, 235, 225, 255]);

    // 2. VectorCraft SVG optimizer
    const svgRes = await server.fetch(
      new Request("http://localhost:3000/api/craft/svg-optimize", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          svg: "<svg><!-- comment -->  <path d='M10.12345 20.98765'/> </svg>",
        }),
      }),
    );
    expect(svgRes.status).toBe(200);
    const svgJson = (await svgRes.json()) as {
      optimized: string;
      savedPercent: number;
    };
    expect(svgJson.optimized).not.toContain("comment");
    expect(svgJson.savedPercent).toBeGreaterThan(0);

    // 3. AudioCraft PCM16 WAV
    const wavRes = await server.fetch(
      new Request("http://localhost:3000/api/craft/wav?freq=440&ms=120"),
    );
    expect(wavRes.status).toBe(200);
    expect(wavRes.headers.get("content-type")).toBe("audio/wav");
    const wavBytes = new Uint8Array(await wavRes.arrayBuffer());
    expect(new TextDecoder().decode(wavBytes.subarray(0, 4))).toBe("RIFF");
    expect(new TextDecoder().decode(wavBytes.subarray(8, 12))).toBe("WAVE");

    // 4. CodeCraft WAT -> WASM compiler
    const watRes = await server.fetch(
      new Request("http://localhost:3000/api/craft/wat", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          wat: '(module (func (export "mul") (param i32 i32) (result i32) local.get 0 local.get 1 i32.mul))',
          args: [6, 7],
        }),
      }),
    );
    expect(watRes.status).toBe(200);
    const watJson = (await watRes.json()) as {
      result: number;
      exportName: string;
    };
    expect(watJson.exportName).toBe("mul");
    expect(watJson.result).toBe(42);
  });

  it("executes Fluent 2 Office WordCraft OOXML .docx export and GridCraft zero-eval formulas", async () => {
    const docxRes = await server.fetch(
      new Request("http://localhost:3000/api/office/docx", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          title: "Sovereign Spec",
          body: "Line 1\nLine 2",
        }),
      }),
    );
    expect(docxRes.status).toBe(200);
    const docxBytes = new Uint8Array(await docxRes.arrayBuffer());
    // PKZIP magic 0x50 0x4B 0x03 0x04
    expect([docxBytes[0], docxBytes[1], docxBytes[2], docxBytes[3]]).toEqual([
      0x50, 0x4b, 0x03, 0x04,
    ]);
    expect(new TextDecoder().decode(docxBytes)).toContain(
      "[Content_Types].xml",
    );

    const fRes = await server.fetch(
      new Request("http://localhost:3000/api/office/formula", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          cells: {
            B1: "48",
            B2: "64",
            B3: "24",
            B4: "=SUM(B1:B3)",
            B5: "=ROUND(AVG(B1:B3), 2)",
            B6: "=IF(B4>100, 1, 0)",
          },
        }),
      }),
    );
    expect(fRes.status).toBe(200);
    const fJson = (await fRes.json()) as { evaluated: Record<string, number> };
    expect(fJson.evaluated.B4).toBe(136);
    expect(fJson.evaluated.B5).toBe(45.33);
    expect(fJson.evaluated.B6).toBe(1);
  });

  it("executes Spacedrive VDFS CAS, M3 Workspace (.ics/.vcf/Argon2id), and Locally Uncensored Studio", async () => {
    const sdRes = await server.fetch(
      new Request("http://localhost:3000/api/spacedrive/files"),
    );
    expect(sdRes.status).toBe(200);
    const sdJson = (await sdRes.json()) as {
      files: Array<{ blake3Hash: string }>;
    };
    expect(sdJson.files.length).toBeGreaterThanOrEqual(4);
    expect(sdJson.files[0]?.blake3Hash.length).toBe(64);

    const icsRes = await server.fetch(
      new Request("http://localhost:3000/api/workspace/events.ics"),
    );
    expect(icsRes.status).toBe(200);
    expect(await icsRes.text()).toContain("BEGIN:VCALENDAR");

    const vcfRes = await server.fetch(
      new Request("http://localhost:3000/api/workspace/contacts.vcf"),
    );
    expect(vcfRes.status).toBe(200);
    expect(await vcfRes.text()).toContain("BEGIN:VCARD");

    const email = `test-${Date.now()}@aphrody.com`;
    const regRes = await server.fetch(
      new Request("http://localhost:3000/api/workspace/account", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          email,
          password: "correct-horse-battery-staple",
        }),
      }),
    );
    expect(regRes.status).toBe(200);
    expect(
      ((await regRes.json()) as { authenticated: boolean }).authenticated,
    ).toBe(true);

    const badRes = await server.fetch(
      new Request("http://localhost:3000/api/workspace/account", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email, password: "wrong-password" }),
      }),
    );
    expect(
      ((await badRes.json()) as { authenticated: boolean }).authenticated,
    ).toBe(false);

    const stRes = await server.fetch(
      new Request("http://localhost:3000/api/studio/complete", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ prompt: "spacedrive vdfs" }),
      }),
    );
    expect(stRes.status).toBe(200);
    const stJson = (await stRes.json()) as { output: string };
    expect(stJson.output.length).toBeGreaterThan(20);
  });
});
