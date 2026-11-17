import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, test } from "bun:test";
import { DESIGN_DOCS_DIR, buildDesignDocsPackage } from "./design-docs-pack.ts";

describe("Google Design docs/design complete asset, kit, MDX & contract pack", () => {
  test("builds and validates manifest, SVGs, CSS, DESIGN.md, Figma & Stitch kits, and 6 MDX guides", async () => {
    const manifest = buildDesignDocsPackage();
    const { validateDesignManifest } = await import(join(DESIGN_DOCS_DIR, "schema.ts"));
    const res = validateDesignManifest(manifest);
    expect(res.ok).toBe(true);
    expect(res.errors).toEqual([]);
    expect(manifest.svgs.length).toBeGreaterThanOrEqual(8);
    expect(manifest.guidelines.length).toBe(6);
    expect(manifest.components.length).toBeGreaterThanOrEqual(8);
    expect(manifest.referenceCaptures.length).toBe(5);

    const css = readFileSync(join(DESIGN_DOCS_DIR, "stylesheet.css"), "utf8");
    expect(css).toContain("--md-sys-color-primary:");
    expect(css).toContain("--md-sys-motion-spring-spatial-fast:");
    expect(existsSync(join(DESIGN_DOCS_DIR, "DESIGN.md"))).toBe(true);
    expect(existsSync(join(DESIGN_DOCS_DIR, "kits/figma-design-kit.json"))).toBe(true);
    expect(existsSync(join(DESIGN_DOCS_DIR, "kits/stitch-design-kit.json"))).toBe(true);
  });
});
