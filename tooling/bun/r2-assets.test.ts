import { describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  buildR2Manifest,
  isKeepLocalBoot,
  pruneLocalOffloadedAssets,
  pushAssetsToR2,
  R2_MANIFEST_REL,
  serializeR2ManifestToml,
} from "./r2-assets.ts";

describe("r2-assets content-addressed migration", () => {
  test("classifies compile-time assets as keepLocalBoot and offloads runtime media", () => {
    expect(isKeepLocalBoot("assets/fonts/lilex/Lilex-Regular.ttf")).toBe(true);
    expect(isKeepLocalBoot("assets/images/aphrody_logo.svg")).toBe(true);
    expect(isKeepLocalBoot("examples/nsh/gallery/fonts/Pacifico-Regular.ttf")).toBe(true);
    expect(isKeepLocalBoot("crates/gpui/examples/image/black-cat-typing.gif")).toBe(false);
    expect(isKeepLocalBoot("docs/design/gemini-home.png")).toBe(false);
    expect(isKeepLocalBoot("docs/design/Material 3 Design Kit (Community).zip")).toBe(false);
  });

  test("builds deterministic SHA-256 R2 manifest and safely prunes only non-boot blobs", () => {
    const dir = mkdtempSync(join(tmpdir(), "r2-assets-test-"));
    try {
      mkdirSync(join(dir, "assets", "fonts", "lilex"), { recursive: true });
      mkdirSync(join(dir, "docs", "design"), { recursive: true });
      writeFileSync(join(dir, "assets", "fonts", "lilex", "Lilex-Regular.ttf"), "ttf-bytes");
      writeFileSync(join(dir, "docs", "design", "gemini-home.png"), "png-bytes");

      const manifest = buildR2Manifest(dir);
      expect(manifest.totalAssets).toBe(2);
      const png = manifest.assets.find((a) => a.logicalPath === "docs/design/gemini-home.png")!;
      expect(png.keepLocalBoot).toBe(false);
      expect(png.r2Key).toMatch(/^v1\/design-reference\/[0-9a-f]{64}\.png$/);
      expect(png.publicUrl).toBe(`https://assets.aphrody.app/${png.r2Key}`);

      const toml = serializeR2ManifestToml(manifest);
      expect(toml).toContain('bucket = "aphrody-assets"');

      const dryPush = pushAssetsToR2(dir, true);
      expect(dryPush.ok).toBe(true);
      expect(dryPush.uploaded.length).toBe(2);

      const pruned = pruneLocalOffloadedAssets(dir, false);
      expect(pruned.prunedPaths).toEqual(["docs/design/gemini-home.png"]);
      expect(existsSync(join(dir, "docs", "design", "gemini-home.png"))).toBe(false);
      expect(existsSync(join(dir, "assets", "fonts", "lilex", "Lilex-Regular.ttf"))).toBe(true);
      expect(readFileSync(join(dir, R2_MANIFEST_REL), "utf8")).toContain("gemini-home.png");
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});
