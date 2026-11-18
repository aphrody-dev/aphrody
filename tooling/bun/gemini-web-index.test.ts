import { expect, test } from "bun:test";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  parseSitemapLocales,
  queryGeminiIndex,
  syncGeminiIndex,
} from "./gemini-web-index.ts";

test("parseSitemapLocales extracts hreflang entries", () => {
  const xml = `<url><xhtml:link rel="alternate" hreflang="fr" href="https://gemini.google.com/?hl=fr"/></url>`;
  const locales = parseSitemapLocales(xml, "2026-11-18T00:00:00Z");
  expect(locales.length).toBe(1);
  expect(locales[0]?.hreflang).toBe("fr");
});

test("syncGeminiIndex writes sqlite db, map.json, doc.md and supports query", async () => {
  const dir = mkdtempSync(join(tmpdir(), "gemini-web-test-"));
  const dbPath = join(dir, "index.db");
  const mapPath = join(dir, "map.json");
  const docPath = join(dir, "sitemap.md");
  try {
    const res = await syncGeminiIndex({ dbPath, mapPath, docPath, offline: true });
    expect(res.status).toBe("ok");
    expect(res.routes).toBeGreaterThanOrEqual(17);
    expect(res.controls).toBeGreaterThanOrEqual(4);

    const hits = queryGeminiIndex("customize", dbPath) as Array<{ path: string }>;
    expect(hits.some((h) => h.path === "/customize")).toBe(true);
    expect(await Bun.file(mapPath).exists()).toBe(true);
    expect(await Bun.file(docPath).exists()).toBe(true);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
