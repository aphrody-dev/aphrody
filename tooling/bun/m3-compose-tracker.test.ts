import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import {
  CATALOG_JSON_REL,
  FIGMA_DTCG_REL,
  MATERIAL_SHAPES_35,
  REPO_ROOT,
  SOURCES,
  TEMPLATE_DIR_REL,
  TRACKER_LOCK_REL,
  WGSL_SHADER_REL,
  computeLockStamp,
  parseMavenVersions,
  runConvert,
} from "./m3-compose-tracker.ts";

describe("m3-compose-tracker", () => {
  test("parses Google Maven XML metadata into stable vs latest", () => {
    const sampleXml = `
      <metadata>
        <versioning>
          <latest>1.6.0-alpha01</latest>
          <release>1.6.0-alpha01</release>
          <versions>
            <version>1.4.0</version>
            <version>1.5.0-beta01</version>
            <version>1.5.0-rc01</version>
            <version>1.5.0</version>
            <version>1.6.0-alpha01</version>
          </versions>
        </versioning>
      </metadata>`;
    const parsed = parseMavenVersions(sampleXml);
    expect(parsed.stable).toBe("1.5.0");
    expect(parsed.latest).toBe("1.6.0-alpha01");
  });

  test("lock stamp is deterministic and tracks all 35 MaterialShapes", () => {
    expect(MATERIAL_SHAPES_35.length).toBe(35);
    const core = {
      material3_stable: "1.5.0",
      material3_latest: "1.6.0-alpha01",
      material3_ripple_stable: "1.5.0",
      material3_window_size_stable: "1.5.0",
      compose_bom_latest: "2026.11.01",
      jetbrains_compose_m3_latest: "1.13.0-rc01",
      figma_kit_file_id: SOURCES.figmaKitFileId,
      compose_blog_url: SOURCES.composeBlog,
      parametric_blog_url: SOURCES.parametricBlog,
    };
    expect(computeLockStamp(core)).toBe(computeLockStamp(core));
  });

  test("converted GPU WGSL, GPUI Kit, NSH TSX, and Figma DTCG artifacts have zero drift", () => {
    const res = runConvert(REPO_ROOT, true);
    expect(res.drifted).toEqual([]);
    expect(res.shapesCount).toBe(35);
    expect(res.componentCount).toBeGreaterThanOrEqual(16);

    expect(existsSync(join(REPO_ROOT, TRACKER_LOCK_REL))).toBe(true);
    expect(existsSync(join(REPO_ROOT, CATALOG_JSON_REL))).toBe(true);
    expect(existsSync(join(REPO_ROOT, FIGMA_DTCG_REL))).toBe(true);
    expect(existsSync(join(REPO_ROOT, WGSL_SHADER_REL))).toBe(true);
    expect(existsSync(join(REPO_ROOT, TEMPLATE_DIR_REL, "nsh-m3-kit.tsx"))).toBe(true);
    expect(existsSync(join(REPO_ROOT, TEMPLATE_DIR_REL, "src/main.rs"))).toBe(true);

    const dtcg = JSON.parse(readFileSync(join(REPO_ROOT, FIGMA_DTCG_REL), "utf8"));
    expect(dtcg.figmaCommunityFileId).toBe("1035203688168086460");
    expect(dtcg.composeMaterial3Version).toBe("1.5.0");
  });
});
