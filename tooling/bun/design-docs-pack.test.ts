import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "../..");

describe("Aphrody Figma, Stitch, Themes & Roadmap Design Pack", () => {
  test("manifest.json validates with 20+ SVGs, 12 maquettes, and WCAG AA primary contrast", async () => {
    const manifest = JSON.parse(readFileSync(resolve(repoRoot, "docs/design/manifest.json"), "utf8"));
    const { validateDesignManifest } = await import("../../docs/design/schema.ts");
    const check = validateDesignManifest(manifest);
    expect(check.ok).toBe(true);
    expect(manifest.svgs.length).toBeGreaterThanOrEqual(20);
    expect(manifest.maquettes.length).toBe(12);
    expect(manifest.wcagContrastDarkPrimary).toBeGreaterThanOrEqual(4.5);
    expect(manifest.wcagContrastLightPrimary).toBeGreaterThanOrEqual(4.5);
    for (const s of manifest.svgs as Array<{ path: string }>) {
      const svgText = readFileSync(resolve(repoRoot, "docs/design", s.path.replace("./", "")), "utf8");
      expect(svgText).not.toMatch(/&(?!amp;|lt;|gt;|quot;|apos;|#\d+;|#x[0-9a-fA-F]+;)/);
    }
  });

  test("Figma Design Kit v3 and Code Connect cover all 6 collections and 20 component sets", () => {
    const figma = JSON.parse(readFileSync(resolve(repoRoot, "docs/design/kits/figma-design-kit.json"), "utf8")) as {
      version: string;
      collections: Record<string, unknown>;
      componentSets: Array<{ id: string }>;
    };
    const codeConnect = JSON.parse(readFileSync(resolve(repoRoot, "docs/design/kits/figma-code-connect.json"), "utf8")) as {
      mappings: Array<{ figmaComponentSet: string }>;
    };
    expect(figma.version).toBe("3.0.0");
    expect(Object.keys(figma.collections).length).toBe(6);
    expect(figma.componentSets.length).toBe(20);
    expect(codeConnect.mappings.length).toBe(20);
    expect(codeConnect.mappings.map((m) => m.figmaComponentSet)).toEqual(figma.componentSets.map((c) => c.id));
  });

  test("Aphrody Zed theme family defines all 4 modes with full syntax tokens", () => {
    const aphrodyTheme = JSON.parse(readFileSync(resolve(repoRoot, "assets/themes/aphrody/aphrody.json"), "utf8")) as {
      name: string;
      themes: Array<{ name: string; appearance: string; style: Record<string, unknown> }>;
    };
    expect(aphrodyTheme.name).toBe("Aphrody");
    expect(aphrodyTheme.themes.map((t) => t.name)).toEqual([
      "Aphrody Neural Obsidian",
      "Aphrody Material Dark",
      "Aphrody Material Light",
      "Aphrody OLED Pitch",
    ]);
    for (const variant of aphrodyTheme.themes) {
      expect(typeof variant.style["editor.background"]).toBe("string");
      expect(typeof variant.style["syntax"]).toBe("object");
    }
  });

  test("aphrody_labs_roadmap.toml parses with 10 domains and H0-H3 horizons", () => {
    const raw = readFileSync(resolve(repoRoot, "assets/os/aphrody_labs_roadmap.toml"), "utf8");
    const parsed = Bun.TOML.parse(raw) as {
      domains: Array<{ id: string }>;
      horizons: Array<{ id: string }>;
    };
    expect(parsed.domains.length).toBe(10);
    expect(parsed.horizons.map((h) => h.id)).toEqual(["H0", "H1", "H2", "H3"]);
  });
});
