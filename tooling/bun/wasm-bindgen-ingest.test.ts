import { describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { checkRepoHygiene } from "./repo-hygiene.ts";
import {
  DIGEST_DIR_REL,
  ECOSYSTEM_DOC_REL,
  EXPECTED_ORG_REPOS,
  extractLocalAphrodyState,
  INGEST_MANIFEST_REL,
  type IngestManifest,
  LLMS_TXT_REL,
  parseCargoManifestQuick,
  parseGitIngestSummaryMetadata,
  parsePaginatedJson,
  renderDirectoryTree,
  renderGitIngestDigest,
  REPO_ROOT,
  runEmitCommand,
  SCAN_OUT_REL,
  type ScanReport,
  selectKeyDigestFiles,
  SKILL_DIR_REL,
  updateEcosystemSummaryBlocks,
  validateGitIngestDigest,
  validateLlmsTxt,
  validateRelativeLinks,
  validateSkillFrontmatter,
} from "./wasm-bindgen-ingest.ts";

describe("wasm-bindgen-ingest pure parsers, serializers & validators", () => {
  test("parsePaginatedJson handles slurped, concatenated, and empty inputs", () => {
    expect(parsePaginatedJson("")).toEqual([]);
    expect(parsePaginatedJson('[{"name":"a"}][{"name":"b"}]').map((x: any) => x.name)).toEqual(["a", "b"]);
    expect(parsePaginatedJson('[[{"name":"x"}],[{"name":"y"}]]').map((x: any) => x.name)).toEqual(["x", "y"]);
  });

  test("parseCargoManifestQuick parses package metadata, MSRV, features, and deps", () => {
    const manifest = `
[workspace]
members = ["crates/cli", "crates/web-sys"]

[package]
name = "wasm-bindgen"
version = "0.2.130"
edition = "2021"
rust-version = "1.81"
license = "MIT OR Apache-2.0"

[features]
default = ["std"]
serde-serialize = ["serde", "serde_json", "std"]

[dependencies]
cfg-if = "1.0"
once_cell = "1.12"
`;
    const parsed = parseCargoManifestQuick(manifest);
    expect(parsed.packageName).toBe("wasm-bindgen");
    expect(parsed.version).toBe("0.2.130");
    expect(parsed.rustVersion).toBe("1.81");
    expect(parsed.workspaceMembers).toEqual(["crates/cli", "crates/web-sys"]);
    expect(parsed.features.default).toEqual(["std"]);
    expect(parsed.dependencies).toEqual(["cfg-if", "once_cell"]);
  });

  test("renderDirectoryTree and renderGitIngestDigest produce valid GitIngest format", () => {
    const paths = ["README.md", "Cargo.toml", "src/lib.rs", "src/passes/gc.rs"];
    const tree = renderDirectoryTree("wasm-bindgen-walrus", paths);
    expect(tree).toContain("Directory structure:");
    expect(tree).toContain("└── wasm-bindgen-walrus/");
    expect(tree).toContain("src/");
    expect(tree).toContain("Cargo.toml");

    const digest = renderGitIngestDigest("wasm-bindgen-walrus", paths, [
      { path: "README.md", content: "# Walrus" },
      { path: "Cargo.toml", content: '[package]\nname = "walrus"' },
    ]);
    const validation = validateGitIngestDigest(digest);
    expect(validation.ok).toBe(true);
    expect(validation.hasDirectoryStructure).toBe(true);
    expect(validation.fileCount).toBe(2);

    const invalid = validateGitIngestDigest("plain text without digest headers");
    expect(invalid.ok).toBe(false);
    expect(invalid.errors.length).toBeGreaterThanOrEqual(2);
  });

  test("selectKeyDigestFiles prioritizes manifests, README, and core crates", () => {
    const files = [
      "tests/huge/a.rs",
      "src/lib.rs",
      "Cargo.toml",
      "README.md",
      "crates/web-sys/Cargo.toml",
      "crates/js-sys/Cargo.toml",
      "crates/futures/Cargo.toml",
    ];
    const picked = selectKeyDigestFiles("wasm-bindgen", files, 10);
    expect(picked).toContain("README.md");
    expect(picked).toContain("Cargo.toml");
    expect(picked).toContain("crates/web-sys/Cargo.toml");
    expect(picked).toContain("crates/js-sys/Cargo.toml");
    expect(picked).toContain("crates/futures/Cargo.toml");
  });

  test("parseGitIngestSummaryMetadata extracts file counts and token estimates", () => {
    const parsed = parseGitIngestSummaryMetadata(
      "Repository: wasm-bindgen/walrus\nCommit: 58d65384\nFiles analyzed: 238\n\nEstimated tokens: 302.6k",
    );
    expect(parsed.filesAnalyzed).toBe(238);
    expect(parsed.estimatedTokens).toBe("302.6k");
  });

  test("validateSkillFrontmatter enforces agentskills.io naming and length rules", () => {
    expect(
      validateSkillFrontmatter("---\nname: wasm-bindgen\ndescription: Valid skill\n---\n# Title\n").ok,
    ).toBe(true);
    expect(
      validateSkillFrontmatter("---\nname: Invalid_Name\ndescription: Bad slug\n---\n").ok,
    ).toBe(false);
    expect(
      validateSkillFrontmatter(`---\nname: ${"a".repeat(65)}\ndescription: Too long\n---\n`).ok,
    ).toBe(false);
    expect(validateSkillFrontmatter("# Missing frontmatter").ok).toBe(false);
  });

  test("validateLlmsTxt validates llmstxt.org H1, blockquote, H2, and markdown link list", () => {
    const valid = `# Demo\n\n> Summary line\n\n## Docs\n\n- [Guide](./guide.md): Details\n`;
    const okRes = validateLlmsTxt(valid);
    expect(okRes.ok).toBe(true);
    expect(okRes.title).toBe("Demo");
    expect(okRes.hasBlockquote).toBe(true);
    expect(okRes.sections).toEqual(["Docs"]);
    expect(okRes.linkCount).toBe(1);

    expect(validateLlmsTxt("## Missing H1").ok).toBe(false);
    expect(validateLlmsTxt("# Title\n\nNo blockquote").ok).toBe(false);
  });
});

describe("wasm-bindgen-ingest Aphrody workspace & fixture emit", () => {
  test("extractLocalAphrodyState reads real Cargo.lock, Cargo.toml, gpui_web, and CLIs", () => {
    const local = extractLocalAphrodyState(REPO_ROOT);
    expect(local.cargoLockVersions.wasmBindgen).toBe("0.2.129");
    expect(local.cargoLockVersions.wasmBindgenFutures).toBe("0.4.79");
    expect(local.cargoLockVersions.jsSys).toBe("0.3.106");
    expect(local.cargoLockVersions.webSys).toBe("0.3.106");
    expect(local.gpuiWebFeaturesCount).toBeGreaterThanOrEqual(45);
    expect(local.gpuiWgpuWebSysFeatures).toEqual(["HtmlCanvasElement"]);
    // Host CLIs are optional on a clean checkout; when present they must match the pinned versions.
    if (local.cliVersions.wasmBindgen !== "missing") expect(local.cliVersions.wasmBindgen).toBe("0.2.129");
    if (local.cliVersions.wasmOpt !== "missing") expect(local.cliVersions.wasmOpt).toBe("v133");
    expect(local.profileWebSettings["opt-level"]).toBe("z");
    expect(local.profileWebSettings.panic).toBe("abort");
    expect(local.webBuildCohorts).toEqual(["baseline", "simd-baseline", "multithreaded"]);
  });

  test("runEmitCommand & updateEcosystemSummaryBlocks work deterministically in fixture", () => {
    const fixture = mkdtempSync(join(tmpdir(), "aphrody-wb-emit-"));
    try {
      mkdirSync(join(fixture, "crates/gpui_web/src"), { recursive: true });
      mkdirSync(join(fixture, "crates/gpui_wgpu"), { recursive: true });
      mkdirSync(join(fixture, "crates/os_desktop/examples"), { recursive: true });
      mkdirSync(join(fixture, "crates/nsh-platform-web/src/browser"), { recursive: true });
      mkdirSync(join(fixture, "tooling/bun"), { recursive: true });
      writeFileSync(join(fixture, "Cargo.toml"), "[package]\nname = 'root'");
      writeFileSync(join(fixture, "Cargo.lock"), "");
      writeFileSync(join(fixture, "crates/gpui_web/Cargo.toml"), "");
      writeFileSync(join(fixture, "crates/gpui_web/src/gpui_web.rs"), "");
      writeFileSync(join(fixture, "crates/gpui_web/src/events.rs"), "");
      writeFileSync(join(fixture, "crates/gpui_web/src/window.rs"), "");
      writeFileSync(join(fixture, "crates/gpui_web/src/dispatcher.rs"), "");
      writeFileSync(join(fixture, "crates/gpui_web/src/ime_mirror.rs"), "");
      writeFileSync(join(fixture, "crates/gpui_web/src/http_client.rs"), "");
      writeFileSync(join(fixture, "crates/gpui_web/src/platform.rs"), "");
      writeFileSync(join(fixture, "crates/gpui_wgpu/Cargo.toml"), "");
      writeFileSync(join(fixture, "crates/os_desktop/examples/web_os.rs"), "");
      writeFileSync(join(fixture, "crates/nsh-platform-web/Cargo.toml"), "");
      writeFileSync(join(fixture, "crates/nsh-platform-web/src/browser/values.rs"), "");
      writeFileSync(join(fixture, "tooling/bun/web-build.ts"), "");
      writeFileSync(join(fixture, "tooling/bun/web-verify.ts"), "");
      writeFileSync(join(fixture, "tooling/bun/wasm-bindgen-ingest.ts"), "");
      writeFileSync(join(fixture, "tooling/bun/wasm-bindgen-ingest.test.ts"), "");

      const scan: ScanReport = {
        schemaVersion: 1,
        generatedAt: "2026-11-16T22:45:00Z",
        org: "wasm-bindgen",
        repoCount: EXPECTED_ORG_REPOS.length,
        repos: EXPECTED_ORG_REPOS.map((name, idx) => ({
          name,
          fullName: `wasm-bindgen/${name}`,
          htmlUrl: `https://github.com/wasm-bindgen/${name}`,
          description: `${name} description`,
          language: "Rust",
          stars: 100 - idx,
          forks: 10,
          openIssuesAndPrs: 1,
          archived: false,
          defaultBranch: "main",
          pushedAt: "2026-11-16T00:00:00Z",
          latestCommitSha: `abcdef1234567890abcdef1234567890abcdef0${idx}`,
          latestCommitDate: "2026-11-16T00:00:00Z",
          latestCommitMessage: "Fixture commit",
          treeBlobCount: 10 + idx,
          treeTruncated: false,
          keyPathsPresent: ["README.md", "Cargo.toml"],
          subCargoVersions: [],
          roleInEcosystem: `Role for ${name}`,
        })),
        cratesIo: [
          {
            name: "wasm-bindgen",
            exists: true,
            maxVersion: "0.2.130",
            maxStableVersion: "0.2.130",
            downloads: 660000000,
            recentDownloads: 180000000,
            updatedAt: "2026-10-30T06:15:04Z",
            repository: "https://github.com/wasm-bindgen/wasm-bindgen",
          },
        ],
        localToolchain: extractLocalAphrodyState(REPO_ROOT),
        gitingestEndpointProbe: {
          url: "https://gitingest.com",
          reachable: true,
          apiRouteDiscovered: "/api/ingest",
          status: 200,
        },
      };

      const manifest: IngestManifest = {
        schemaVersion: 1,
        generatedAt: "2026-11-16T22:45:00Z",
        gitingestCliVersion: "0.3.1",
        remoteApiUrl: "https://gitingest.com/api/ingest",
        repoCount: EXPECTED_ORG_REPOS.length,
        totalDigestBytes: 8000,
        totalDigestFiles: 16,
        entries: EXPECTED_ORG_REPOS.map((name) => ({
          repo: name,
          fullName: `wasm-bindgen/${name}`,
          commitSha: "abcdef12",
          method: "gitingest.com-digest-url",
          digestPath: `target/gitingest/wasm-bindgen/${name}.txt`,
          digestBytes: 1000,
          digestLines: 40,
          digestFileCount: 2,
          digestSha256: "0000000000000000",
          filesAnalyzed: 10,
          estimatedTokens: "1.0k",
          summary: `Repository: wasm-bindgen/${name}`,
        })),
      };

      const res = runEmitCommand(scan, manifest, fixture);
      expect(res.skillFrontmatterOk).toBe(true);
      expect(res.llmsTxtOk).toBe(true);
      expect(res.brokenLinks).toEqual([]);
      expect(res.referencePaths.length).toBe(4);

      const customWrapper = [
        "# Header Keep",
        "<!-- BEGIN WASM_BINDGEN_TELEMETRY -->old<!-- END WASM_BINDGEN_TELEMETRY -->",
        "<!-- BEGIN WASM_BINDGEN_REPOS_TABLE -->old<!-- END WASM_BINDGEN_REPOS_TABLE -->",
        "<!-- BEGIN WASM_BINDGEN_CRATES_MATRIX -->old<!-- END WASM_BINDGEN_CRATES_MATRIX -->",
        "# Footer Keep",
      ].join("\n");
      const updated = updateEcosystemSummaryBlocks(customWrapper, scan, manifest);
      expect(updated).toContain("# Header Keep");
      expect(updated).toContain("# Footer Keep");
      expect(updated).not.toContain(">old<");
      expect(updated).toContain("wasm-bindgen/rust-externref-rfc");
    } finally {
      rmSync(fixture, { recursive: true, force: true });
    }
  }, 45_000);
});

describe("wasm-bindgen-ingest generated repo artifacts & live digests verification", () => {
  test("all 8 GitIngest digests, llms.txt, Skill bundle, ecosystem doc, and relative links are valid", () => {
    // The digests are gitignored output of a live org scan; verify them only where that scan has run.
    if (!existsSync(join(REPO_ROOT, SCAN_OUT_REL))) return;
    expect(existsSync(join(REPO_ROOT, INGEST_MANIFEST_REL))).toBe(true);

    const scan = JSON.parse(readFileSync(join(REPO_ROOT, SCAN_OUT_REL), "utf8")) as ScanReport;
    expect(scan.repoCount).toBe(8);
    const repoNames = new Set(scan.repos.map((r) => r.name));
    for (const expected of EXPECTED_ORG_REPOS) {
      expect(repoNames.has(expected)).toBe(true);
    }

    const manifest = JSON.parse(readFileSync(join(REPO_ROOT, INGEST_MANIFEST_REL), "utf8")) as IngestManifest;
    expect(manifest.repoCount).toBe(8);
    expect(manifest.entries.length).toBe(8);
    for (const entry of manifest.entries) {
      const digestAbs = join(REPO_ROOT, DIGEST_DIR_REL, `${entry.repo}.txt`);
      expect(existsSync(digestAbs)).toBe(true);
      const raw = readFileSync(digestAbs, "utf8");
      const check = validateGitIngestDigest(raw);
      expect(check.ok).toBe(true);
      expect(check.fileCount).toBeGreaterThanOrEqual(1);
    }

    const llmsAbs = join(REPO_ROOT, LLMS_TXT_REL);
    expect(existsSync(llmsAbs)).toBe(true);
    const llmsCheck = validateLlmsTxt(readFileSync(llmsAbs, "utf8"));
    expect(llmsCheck.ok).toBe(true);
    expect(llmsCheck.linkCount).toBeGreaterThanOrEqual(12);
    for (const expected of EXPECTED_ORG_REPOS) {
      expect(readFileSync(llmsAbs, "utf8")).toContain(`wasm-bindgen/${expected}`);
    }

    const skillAbs = join(REPO_ROOT, SKILL_DIR_REL, "SKILL.md");
    expect(existsSync(skillAbs)).toBe(true);
    const fm = validateSkillFrontmatter(readFileSync(skillAbs, "utf8"));
    expect(fm.ok).toBe(true);
    expect(fm.name).toBe("wasm-bindgen");

    const refFiles = [
      "repos-matrix.md",
      "js-web-sys-patterns.md",
      "walrus-weedle-tsgen.md",
      "aphrody-best-practices.md",
    ].map((f) => join(REPO_ROOT, SKILL_DIR_REL, "references", f));
    for (const rf of refFiles) {
      expect(existsSync(rf)).toBe(true);
      expect(readFileSync(rf, "utf8").length).toBeGreaterThan(1500);
    }

    const ecoAbs = join(REPO_ROOT, ECOSYSTEM_DOC_REL);
    expect(existsSync(ecoAbs)).toBe(true);
    const ecoMd = readFileSync(ecoAbs, "utf8");
    expect(ecoMd).toContain("0.2.129");
    expect(ecoMd).toContain("133");
    expect(ecoMd).toContain("gpui_web");
    expect(ecoMd).toContain("web-build.ts");

    const broken = validateRelativeLinks([llmsAbs, ecoAbs, skillAbs, ...refFiles]);
    expect(broken).toEqual([]);

    const hygiene = checkRepoHygiene(REPO_ROOT);
    expect(hygiene.ephemeralLeaks).toEqual([]);
    expect(hygiene.brokenArchLinks).toEqual([]);
    expect(hygiene.ok).toBe(true);
  });
});
