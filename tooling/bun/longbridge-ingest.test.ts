import { describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  ARCH_DOC_REL,
  auditCentralization,
  buildGraphifyDocument,
  buildMermaidTopology,
  type CargoMetadataRaw,
  classifyRepo,
  extractCargoManifestSummary,
  extractFeatureMatrix,
  extractHtmlTitle,
  extractInternalLinks,
  GRAPH_OUT_REL,
  type GraphifyDocument,
  INVENTORY_TOML_REL,
  isCoreFeatureMatrixCrate,
  isExtensionPackage,
  loadLocalAphrodyIndex,
  mapWithConcurrency,
  normalizeVersionMajorMinor,
  parsePaginatedJson,
  parsePngMetadata,
  parseSitemapXml,
  REPO_ROOT,
  type ScanReport,
  stripNullish,
  updateArchitectureDoc,
  writeInventoryToml,
} from "./longbridge-ingest.ts";

describe("longbridge-ingest pure parsers & classifiers", () => {
  test("parsePaginatedJson handles both --slurp arrays and concatenated JSON arrays", () => {
    const slurped = JSON.stringify([[{ name: "a" }, { name: "b" }], [{ name: "c" }]]);
    expect(parsePaginatedJson<{ name: string }>(slurped).map((x) => x.name)).toEqual(["a", "b", "c"]);

    const concatenated = '[{"name":"x"}][{"name":"y"},{"name":"z"}]';
    expect(parsePaginatedJson<{ name: string }>(concatenated).map((x) => x.name)).toEqual(["x", "y", "z"]);
    expect(parsePaginatedJson("")).toEqual([]);
  });

  test("normalizeVersionMajorMinor normalizes prerelease and build metadata", () => {
    expect(normalizeVersionMajorMinor("0.15.2")).toBe("0.15");
    expect(normalizeVersionMajorMinor("1.0.229-alpha.1+meta")).toBe("1.0");
    expect(normalizeVersionMajorMinor("2.14.0")).toBe("2.14");
  });

  test("extractCargoManifestSummary extracts package, workspace members, features, and deps", () => {
    const sample = `
[workspace]
members = ["crates/kit", "crates/webview"]

[package]
name = "gpui-kit"
version = "0.10.0"
edition = "2024"
license = "Apache-2.0"

[features]
default = ["webview"]
webview = ["dep:wry"]
inspector = []

[dependencies]
gpui = { workspace = true }
taffy = "0.15"
`;
    const parsed = extractCargoManifestSummary(sample);
    expect(parsed.packageName).toBe("gpui-kit");
    expect(parsed.version).toBe("0.10.0");
    expect(parsed.workspaceMembers).toEqual(["crates/kit", "crates/webview"]);
    expect(parsed.features.default).toEqual(["webview"]);
    expect(parsed.features.webview).toEqual(["dep:wry"]);
    expect(parsed.dependencies).toEqual(["gpui", "taffy"]);
  });

  test("classifyRepo accurately separates absorbed, merge candidates, showcase/bench, and infra repos", () => {
    const localIndex = loadLocalAphrodyIndex(REPO_ROOT);

    const makeRepo = (overrides: Partial<Parameters<typeof classifyRepo>[0]>) => ({
      name: "demo",
      fullName: "longbridge/demo",
      owner: "longbridge",
      description: "",
      homepage: "",
      htmlUrl: "https://github.com/longbridge/demo",
      language: "Rust",
      stars: 10,
      forks: 1,
      openIssuesAndPrs: 0,
      archived: false,
      isFork: false,
      defaultBranch: "main",
      pushedAt: "2026-10-01T00:00:00Z",
      topics: [],
      ...overrides,
    });

    const nsh = classifyRepo(makeRepo({ name: "nsh", fullName: "longbridge/nsh" }), localIndex, "longbridge");
    expect(nsh.classification).toBe("ABSORBED_FLATTENED");
    expect(nsh.localMatches.some((m) => m.startsWith("crates/nsh"))).toBe(true);

    const quickjs = classifyRepo(
      makeRepo({ name: "quickjs-jit", fullName: "longbridge/quickjs-jit" }),
      localIndex,
      "longbridge",
    );
    expect(quickjs.classification).toBe("ABSORBED_FLATTENED");

    const zed = classifyRepo(
      makeRepo({ name: "zed", fullName: "zed-industries/zed", owner: "zed-industries" }),
      localIndex,
      "zed-upstream",
    );
    expect(zed.classification).toBe("ABSORBED_FLATTENED");

    const gpuiFast = classifyRepo(
      makeRepo({ name: "gpui-fast", fullName: "longbridge/gpui-fast", stars: 600 }),
      localIndex,
      "longbridge",
    );
    expect(gpuiFast.classification).toBe("MERGE_CANDIDATE");
    expect(gpuiFast.targetCrates).toContain("crates/gpui/src/taffy.rs");

    const bench = classifyRepo(
      makeRepo({ name: "nsh-perf-bench", fullName: "longbridge/nsh-perf-bench" }),
      localIndex,
      "longbridge",
    );
    expect(bench.classification).toBe("REFERENCE_BENCH_SHOWCASE");

    const infra = classifyRepo(
      makeRepo({ name: "trino", fullName: "longbridge/trino", language: "Java" }),
      localIndex,
      "longbridge",
    );
    expect(infra.classification).toBe("INFRA_OUT_OF_SCOPE");

    const community = classifyRepo(
      makeRepo({ name: "gpuix", fullName: "remorses/gpuix", owner: "remorses", stars: 2900 }),
      localIndex,
      "community-gpui",
    );
    expect(community.classification).toBe("MERGE_CANDIDATE");
  });

  test("parseSitemapXml handles urlset and sitemapindex XML cleanly", () => {
    const urlset = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url><loc>https://nsh.dev/</loc></url>
  <url><loc>https://nsh.dev/apps?tab=all&amp;lang=en</loc></url>
  <url><loc>https://nsh.dev/</loc></url>
</urlset>`;
    const parsedUrlset = parseSitemapXml(urlset);
    expect(parsedUrlset.urls).toEqual(["https://nsh.dev/", "https://nsh.dev/apps?tab=all&lang=en"]);
    expect(parsedUrlset.nestedSitemaps).toEqual([]);

    const indexXml = `<?xml version="1.0" encoding="UTF-8"?>
<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <sitemap><loc>https://zed.dev/sitemap-0.xml</loc></sitemap>
</sitemapindex>`;
    const parsedIndex = parseSitemapXml(indexXml);
    expect(parsedIndex.urls).toEqual([]);
    expect(parsedIndex.nestedSitemaps).toEqual(["https://zed.dev/sitemap-0.xml"]);
  });

  test("extractInternalLinks and extractHtmlTitle normalize same-origin links", () => {
    const html = `
      <html>
        <head><title>  NSH &mdash; Docs </title></head>
        <body>
          <a href="/apps">Apps</a>
          <a href="https://nsh.dev/guide#intro">Guide</a>
          <a href="https://external.example.com/">External</a>
          <a href="mailto:hello@example.com">Mail</a>
        </body>
      </html>`;
    expect(extractHtmlTitle(html)).toBe("NSH &mdash; Docs");
    expect(extractInternalLinks(html, "https://nsh.dev/")).toEqual(["https://nsh.dev/apps", "https://nsh.dev/guide"]);
  });

  test("parsePngMetadata validates PNG signature, IHDR dimensions, and byte size", () => {
    const buf = new Uint8Array(2048);
    buf.set([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a], 0);
    const view = new DataView(buf.buffer);
    view.setUint32(16, 1280, false);
    view.setUint32(20, 800, false);
    expect(parsePngMetadata(buf)).toEqual({ valid: true, width: 1280, height: 800 });

    const tiny = buf.slice(0, 64);
    expect(parsePngMetadata(tiny).valid).toBe(false);

    const invalidSig = new Uint8Array(2048);
    expect(parsePngMetadata(invalidSig).valid).toBe(false);
  });

  test("stripNullish removes null and undefined fields recursively for Bun.TOML.stringify", () => {
    const cleaned = stripNullish({
      a: 1,
      b: null,
      c: undefined,
      arr: [{ k: "v", empty: null, missing: undefined }],
    });
    expect(cleaned).toEqual({ a: 1, arr: [{ k: "v" }] });
    const serialized = Bun.TOML.stringify(cleaned);
    expect(Bun.TOML.parse(serialized)).toEqual({ a: 1, arr: [{ k: "v" }] });
  });

  test("mapWithConcurrency respects concurrency ceiling and preserves output order", async () => {
    let active = 0;
    let maxSeen = 0;
    const out = await mapWithConcurrency([1, 2, 3, 4, 5, 6], 2, async (n) => {
      active++;
      maxSeen = Math.max(maxSeen, active);
      await Bun.sleep(10);
      active--;
      return n * 10;
    });
    expect(maxSeen).toBeLessThanOrEqual(2);
    expect(out).toEqual([10, 20, 30, 40, 50, 60]);
  });

  test("isExtensionPackage and isCoreFeatureMatrixCrate match expected workspace conventions", () => {
    expect(isExtensionPackage("C:/src/aphrody/extensions/glsl/Cargo.toml", "glsl")).toBe(true);
    expect(isExtensionPackage("C:/src/aphrody/crates/extension_api/Cargo.toml", "zed_extension_api")).toBe(true);
    expect(isExtensionPackage("C:/src/aphrody/crates/gpui/Cargo.toml", "gpui")).toBe(false);

    expect(isCoreFeatureMatrixCrate("gpui_ui")).toBe(true);
    expect(isCoreFeatureMatrixCrate("nsh-core")).toBe(true);
    expect(isCoreFeatureMatrixCrate("aphrody_desktop")).toBe(true);
    expect(isCoreFeatureMatrixCrate("unrelated_util")).toBe(false);
  });
});

describe("longbridge-ingest offline Cargo centralization & Graphify fixture", () => {
  test("audits workspace lints, top/target non-workspace deps, duplicate graph crates, TOML, and docs", () => {
    const fixtureRoot = mkdtempSync(join(tmpdir(), "aphrody-ingest-fixture-"));
    try {
      mkdirSync(join(fixtureRoot, "crates/gpui"), { recursive: true });
      mkdirSync(join(fixtureRoot, "crates/nsh-core"), { recursive: true });
      mkdirSync(join(fixtureRoot, "extensions/html"), { recursive: true });
      mkdirSync(join(fixtureRoot, "docs/aphrody/architecture"), { recursive: true });

      mkdirSync(join(fixtureRoot, "tooling/bun"), { recursive: true });
      writeFileSync(
        join(fixtureRoot, "tooling/bun/forks-manifest.json"),
        JSON.stringify({
          forks: [
            {
              name: "taffy",
              repo: "aphrody-labs/taffy",
              tag: "v0.15.1-aphrody.20261120",
              gitUrl: "https://github.com/aphrody-labs/taffy.git",
            },
          ],
        }),
      );
      writeFileSync(
        join(fixtureRoot, "Cargo.toml"),
        `
[workspace]
members = ["crates/gpui", "crates/nsh-core", "extensions/html"]

[workspace.dependencies]
gpui = { path = "crates/gpui" }
serde = "1.0"
custom_git = { git = "https://github.com/example/custom" }

[patch.crates-io]
taffy = { git = "https://github.com/aphrody-labs/taffy.git", tag = "v0.15.1-aphrody.20261120" }
async-task = { git = "https://github.com/smol-rs/async-task", rev = "abc123" }
`,
      );
      writeFileSync(
        join(fixtureRoot, "crates/gpui/Cargo.toml"),
        `
[package]
name = "gpui"
version = "0.2.2"
license = "Apache-2.0"

[lints]
workspace = true

[features]
default = ["wayland"]
wayland = []

[dependencies]
serde.workspace = true
num_cpus = "1.13"

[target.'cfg(windows)'.dependencies]
winreg = "0.52"
`,
      );
      writeFileSync(
        join(fixtureRoot, "crates/nsh-core/Cargo.toml"),
        `
[package]
name = "nsh-core"
version = "0.5.0"
license = "Apache-2.0"

[dependencies]
gpui.workspace = true
`,
      );
      writeFileSync(
        join(fixtureRoot, "extensions/html/Cargo.toml"),
        `
[package]
name = "html"
version = "0.1.0"

[dependencies]
zed_extension_api = "0.2.0"
`,
      );

      const docPath = join(fixtureRoot, "docs/aphrody/architecture/longbridge-gpui-unification.md");
      writeFileSync(
        docPath,
        `# Test Doc\n<!-- BEGIN LONGBRIDGE_INGEST_METRICS -->\nold\n<!-- END LONGBRIDGE_INGEST_METRICS -->\n<!-- BEGIN LONGBRIDGE_INGEST_MERMAID -->\nold\n<!-- END LONGBRIDGE_INGEST_MERMAID -->\n`,
      );

      const mockMeta: CargoMetadataRaw = {
        workspace_root: fixtureRoot,
        target_directory: join(fixtureRoot, "target"),
        workspace_members: ["gpui-id", "nsh-core-id", "ext-html-id"],
        packages: [
          {
            id: "gpui-id",
            name: "gpui",
            version: "0.2.2",
            source: null,
            license: "Apache-2.0",
            manifest_path: join(fixtureRoot, "crates/gpui/Cargo.toml"),
            features: { default: ["wayland"], wayland: [] },
            dependencies: [{ name: "serde", kind: null }],
          },
          {
            id: "nsh-core-id",
            name: "nsh-core",
            version: "0.5.0",
            source: null,
            license: "Apache-2.0",
            manifest_path: join(fixtureRoot, "crates/nsh-core/Cargo.toml"),
            features: { svg: [] },
            dependencies: [{ name: "gpui", kind: null }],
          },
          {
            id: "ext-html-id",
            name: "html",
            version: "0.1.0",
            source: null,
            license: "MIT",
            manifest_path: join(fixtureRoot, "extensions/html/Cargo.toml"),
            features: {},
            dependencies: [],
          },
          {
            id: "hashbrown-14",
            name: "hashbrown",
            version: "0.14.5",
            source: "registry+https://github.com/rust-lang/crates.io-index",
            manifest_path: "/registry/hashbrown-0.14.5/Cargo.toml",
          },
          {
            id: "hashbrown-15",
            name: "hashbrown",
            version: "0.15.2",
            source: "registry+https://github.com/rust-lang/crates.io-index",
            manifest_path: "/registry/hashbrown-0.15.2/Cargo.toml",
          },
        ],
        resolve: {
          nodes: [
            { id: "gpui-id", dependencies: ["hashbrown-14"] },
            { id: "nsh-core-id", dependencies: ["gpui-id", "hashbrown-15"] },
            { id: "ext-html-id", dependencies: [] },
            { id: "hashbrown-14", dependencies: [] },
            { id: "hashbrown-15", dependencies: [] },
          ],
        },
      };

      const audit = auditCentralization(mockMeta, fixtureRoot);
      expect(audit.workspaceMemberCount).toBe(3);
      expect(audit.auditedMemberCount).toBe(2);
      expect(audit.extensionCountSkipped).toBe(1);
      expect(audit.missingWorkspaceLints).toEqual(["html", "nsh-core"]);
      expect(audit.nonWorkspaceTopLevelCount).toBe(1);
      expect(audit.nonWorkspaceTargetLevelCount).toBe(1);
      expect(audit.uniqueNonWorkspaceDepNames).toEqual(["num_cpus", "winreg"]);
      expect(audit.workspaceDependencyBreakdown.pathCount).toBe(1);
      expect(audit.workspaceDependencyBreakdown.gitCount).toBe(1);
      expect(audit.workspaceDependencyBreakdown.registryCount).toBe(1);
      expect(audit.patchBreakdown.pathCount).toBe(0);
      expect(audit.patchBreakdown.gitCount).toBe(2);
      expect(audit.duplicatePackageCount).toBe(1);
      expect(audit.duplicateMajorMinorCount).toBe(1);
      expect(audit.duplicatePackages[0]?.majorMinorVersions).toEqual(["0.14", "0.15"]);

      const featureMatrix = extractFeatureMatrix(mockMeta, audit, fixtureRoot);
      expect(Object.keys(featureMatrix)).toEqual(["gpui", "nsh-core"]);
      expect(featureMatrix["nsh-core"].directWorkspaceDeps).toEqual(["gpui"]);

      const localIndex = loadLocalAphrodyIndex(fixtureRoot);
      const fakeScan: ScanReport = {
        schemaVersion: 1,
        generatedAt: "2026-11-16T00:00:00Z",
        longbridgeTotalRepos: 1,
        countsByClassification: {
          ABSORBED_FLATTENED: 0,
          MERGE_CANDIDATE: 1,
          REFERENCE_BENCH_SHOWCASE: 0,
          INFRA_OUT_OF_SCOPE: 0,
        },
        localIndexSummary: {
          pinnedForksCount: 1,
          crateDirsCount: 2,
          workspaceMembersCount: 3,
          workspaceDepsCount: 3,
          patchCount: 2,
        },
        longbridgeRepos: [
          {
            name: "gpui-fast",
            fullName: "longbridge/gpui-fast",
            owner: "longbridge",
            description: "Fast layout",
            homepage: "",
            htmlUrl: "https://github.com/longbridge/gpui-fast",
            language: "Rust",
            stars: 636,
            forks: 10,
            openIssuesAndPrs: 2,
            archived: false,
            isFork: false,
            defaultBranch: "main",
            pushedAt: "2026-11-01T00:00:00Z",
            topics: ["gpui"],
            sourceOrg: "longbridge",
            classification: "MERGE_CANDIDATE",
            category: "gpui-layout-perf",
            relevanceScore: 97,
            localMatches: [],
            targetCrates: ["crates/gpui/src/taffy.rs"],
            rationale: "Incremental TaffyTree",
          },
        ],
        upstreamZed: {
          name: "zed",
          fullName: "zed-industries/zed",
          owner: "zed-industries",
          description: "Zed",
          homepage: "https://zed.dev",
          htmlUrl: "https://github.com/zed-industries/zed",
          language: "Rust",
          stars: 95000,
          forks: 1000,
          openIssuesAndPrs: 100,
          archived: false,
          isFork: false,
          defaultBranch: "main",
          pushedAt: "2026-11-16T00:00:00Z",
          topics: [],
          sourceOrg: "zed-upstream",
          classification: "ABSORBED_FLATTENED",
          category: "core-gpui-editor",
          relevanceScore: 100,
          localMatches: ["crates/gpui"],
          targetCrates: ["crates/gpui"],
          rationale: "Base workspace",
        },
        communityGpuiRepos: [],
      };

      const graph = buildGraphifyDocument(fakeScan, mockMeta, audit, featureMatrix, localIndex, [], [], fixtureRoot);
      expect(graph.nodes.some((n) => n.id === "gh:longbridge/gpui-fast")).toBe(true);
      expect(graph.edges.some((e) => e.from === "gh:longbridge/gpui-fast" && e.to === "crate:gpui" && e.kind === "MERGE_TARGET")).toBe(
        true,
      );
      expect(graph.mergeCandidates.length).toBe(1);
      expect(graph.nodes.some((n) => n.id === "fork:aphrody-labs/taffy" && n.kind === "pinned_fork")).toBe(true);
      expect(graph.edges.some((e) => e.from === "crate:gpui" && e.to === "fork:aphrody-labs/taffy" && e.kind === "PATCHED_VIA")).toBe(true);

      const tomlPath = join(fixtureRoot, "inventory.toml");
      writeInventoryToml(fakeScan, graph, [], [], tomlPath);
      const parsedToml = Bun.TOML.parse(readFileSync(tomlPath, "utf8")) as Record<string, any>;
      expect(parsedToml.schema_version).toBe(1);
      expect(parsedToml.summary.longbridge_total_repos).toBe(1);

      const mermaid = buildMermaidTopology(graph);
      expect(mermaid).toContain("```mermaid");
      updateArchitectureDoc(graph, docPath);
      const updatedDoc = readFileSync(docPath, "utf8");
      expect(updatedDoc).toContain("gpui-fast");
      expect(updatedDoc).toContain("```mermaid");
    } finally {
      rmSync(fixtureRoot, { recursive: true, force: true });
    }
  });
});

describe("live Aphrody repository inventory, architecture doc & Graphify integrity", () => {
  test("assets/os/longbridge_gpui_inventory.toml parses cleanly as TOML with expected baseline", () => {
    const invPath = join(REPO_ROOT, INVENTORY_TOML_REL);
    expect(existsSync(invPath)).toBe(true);
    const parsed = Bun.TOML.parse(readFileSync(invPath, "utf8")) as Record<string, any>;
    expect(parsed.schema_version).toBe(1);
    expect(parsed.summary.longbridge_total_repos).toBeGreaterThanOrEqual(100);
    expect(parsed.summary.workspace_crate_count).toBeGreaterThanOrEqual(250);
    expect(Array.isArray(parsed.longbridge_repos)).toBe(true);
    expect(parsed.longbridge_repos.length).toBe(parsed.summary.longbridge_total_repos);
  });

  test("docs/aphrody/architecture/longbridge-gpui-unification.md contains valid markers and Mermaid topology", () => {
    const docPath = join(REPO_ROOT, ARCH_DOC_REL);
    expect(existsSync(docPath)).toBe(true);
    const md = readFileSync(docPath, "utf8");
    expect(md).toContain("<!-- BEGIN LONGBRIDGE_INGEST_METRICS -->");
    expect(md).toContain("<!-- END LONGBRIDGE_INGEST_METRICS -->");
    expect(md).toContain("<!-- BEGIN LONGBRIDGE_INGEST_MERMAID -->");
    expect(md).toContain("<!-- END LONGBRIDGE_INGEST_MERMAID -->");
    expect(md).toContain("```mermaid");
  });

  test("target/longbridge-ingest/graph.json validates Graphify schema when generated", () => {
    const graphPath = join(REPO_ROOT, GRAPH_OUT_REL);
    if (!existsSync(graphPath)) return;
    const graph = JSON.parse(readFileSync(graphPath, "utf8")) as GraphifyDocument;
    expect(graph.schemaVersion).toBe(1);
    expect(graph.nodes.length).toBeGreaterThan(50);
    expect(graph.edges.length).toBeGreaterThan(50);
    expect(graph.duplicatePackages.length).toBeGreaterThan(0);
    expect(graph.mergeCandidates.length).toBeGreaterThanOrEqual(5);
    expect(Object.keys(graph.featureMatrix).length).toBeGreaterThanOrEqual(30);
  });
});
