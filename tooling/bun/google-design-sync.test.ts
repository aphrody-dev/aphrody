import { describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  auditDesignCompliance,
  buildCanonicalDesignTokens,
  checkPaletteWcagAa,
  contrastRatioHex,
  DESIGN_SOURCE_URL,
  parseSitemapXmlUrls,
  repoRoot,
  serializeDesignTokensToml,
  syncGoogleDesignTokens,
} from "./google-design-sync.ts";

describe("google-design-sync", () => {
  test("3-tier tokens, Google Sans Flex axes, and sitemap parser work deterministically", () => {
    const doc = buildCanonicalDesignTokens("2026-11-17T00:00:00Z");
    expect(doc.source_of_truth).toBe(DESIGN_SOURCE_URL);
    expect(doc.sans_flex_family).toBe("Google Sans Flex");
    expect(doc.ref.variable_axes.opsz.max).toBe(144);
    expect(doc.ref.variable_axes.wght.max).toBe(1000);
    expect(doc.ref.variable_axes.ROND.max).toBe(100);
    expect(checkPaletteWcagAa("light", doc.sys.color.light)).toEqual([]);
    expect(checkPaletteWcagAa("dark", doc.sys.color.dark)).toEqual([]);
    expect(contrastRatioHex("#FFFFFF", "#006399")).toBeGreaterThanOrEqual(4.5);

    const parsed = Bun.TOML.parse(serializeDesignTokensToml(doc)) as typeof doc;
    expect(parsed.sys.motion.springs.spatial_fast.damping_ratio).toBe(0.9);
    expect(parsed.sys.motion.springs.effects_default.damping_ratio).toBe(1.0);
    expect(parsed.sys.shape.extra_large_px).toBe(28);
    expect(Object.keys(parsed.comp).length).toBeGreaterThanOrEqual(7);

    const urls = parseSitemapXmlUrls(
      `<urlset><url><loc>https://design.google/library/code-is-a-design-material</loc><lastmod>2026-01-01</lastmod><priority>0.9</priority></url></urlset>`,
    );
    expect(urls.length).toBe(1);
    expect(urls[0].loc).toContain("code-is-a-design-material");
  });

  test("offline sync writes tokens, snapshot, DESIGN.md and workspace audit passes", async () => {
    const dir = mkdtempSync(join(tmpdir(), "google-design-test-"));
    try {
      const target = join(dir, "google_design_tokens.toml");
      const snapshot = join(dir, "snapshot.toml");
      const designMd = join(dir, "DESIGN.md");
      const res = await syncGoogleDesignTokens(target, true, snapshot, designMd);
      expect(res.source).toBe(DESIGN_SOURCE_URL);
      const parsed = Bun.TOML.parse(readFileSync(target, "utf8")) as {
        source_of_truth: string;
        comp: { button: { container_height_px: number } };
      };
      expect(parsed.source_of_truth).toBe(DESIGN_SOURCE_URL);
      expect(parsed.comp.button.container_height_px).toBe(40);
      expect(readFileSync(designMd, "utf8")).toContain("Google Sans Flex");
      expect(readFileSync(snapshot, "utf8")).toContain("stitch_routes");
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }

    const report = auditDesignCompliance(repoRoot);
    expect(report.sourceOfTruth).toBe(DESIGN_SOURCE_URL);
    expect(report.wcagFailures).toEqual([]);
    expect(report.errors).toEqual([]);
    expect(report.templateFailures).toEqual([]);
    expect(report.ok).toBe(true);
  });
});
