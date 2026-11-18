import { describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import {
  analyzeLockfileConflicts,
  ARCH_DOC_REL,
  AUDIT_JSON_REL,
  buildArchitectureMermaid,
  classifyForkDrift,
  CRATES_IO_TARGETS,
  emitArtifacts,
  extractManifestSummary,
  type GpuiVsDioxusAuditReport,
  MATRIX_TOML_REL,
  parseSubmoduleStatusOutput,
  renderArchitectureMarkdown,
  REPO_ROOT,
  runAudit,
  scanIntegrationBridges,
  serializeMatrixToml,
  stripNullish,
  TARGET_SUBMODULES,
} from "./gpui-vs-dioxus-audit.ts";
import { checkRepoHygiene, isInsideUninitialisedSubmodule } from "./repo-hygiene.ts";

describe("gpui-vs-dioxus-audit pure parsers & conflict analyzer", () => {
  test("parseSubmoduleStatusOutput parses clean, modified, uninitialized, and conflict rows", () => {
    const sample = [
      " 2798f45cb4c4e2f53cdf25c4e526283efb5b2e59 vendor/wgpu (heads/main)",
      "+fd26cf0d676495d7856dc64b3f10cf5447ad9885 vendor/egui (heads/main)",
      "-62f7a7e4777d9b60f55a18a8865c73bb12297f38 vendor/craft-libs",
      "U2649e09d44ec4a1b928d957cbb0043ae1a3f56a5 vendor/taffy (heads/main)",
    ].join("\n");
    expect(parseSubmoduleStatusOutput(sample)).toEqual([
      {
        state: "clean",
        sha: "2798f45cb4c4e2f53cdf25c4e526283efb5b2e59",
        path: "vendor/wgpu",
        refDesc: "heads/main",
      },
      {
        state: "modified",
        sha: "fd26cf0d676495d7856dc64b3f10cf5447ad9885",
        path: "vendor/egui",
        refDesc: "heads/main",
      },
      {
        state: "uninitialized",
        sha: "62f7a7e4777d9b60f55a18a8865c73bb12297f38",
        path: "vendor/craft-libs",
        refDesc: "",
      },
      {
        state: "conflict",
        sha: "2649e09d44ec4a1b928d957cbb0043ae1a3f56a5",
        path: "vendor/taffy",
        refDesc: "heads/main",
      },
    ]);
  });

  test("classifyForkDrift accurately distinguishes identical, ahead, behind, diverged, and local lag", () => {
    expect(
      classifyForkDrift({ localSha: "aaa", forkSha: "aaa", upstreamSha: "aaa", aheadBy: 0, behindBy: 0, localBehindFork: 0 }),
    ).toBe("identical");
    expect(
      classifyForkDrift({ localSha: "bbb", forkSha: "bbb", upstreamSha: "aaa", aheadBy: 2, behindBy: 0, localBehindFork: 0 }),
    ).toBe("ahead (+2)");
    expect(
      classifyForkDrift({ localSha: "aaa", forkSha: "aaa", upstreamSha: "bbb", aheadBy: 0, behindBy: 1, localBehindFork: 0 }),
    ).toBe("behind (-1)");
    expect(
      classifyForkDrift({ localSha: "ccc", forkSha: "ccc", upstreamSha: "bbb", aheadBy: 1, behindBy: 291, localBehindFork: 0 }),
    ).toBe("diverged (+1/-291)");
    expect(
      classifyForkDrift({ localSha: "aaa", forkSha: "bbb", upstreamSha: "bbb", aheadBy: 0, behindBy: 0, localBehindFork: 5 }),
    ).toBe("local-behind-fork (-5)");
  });

  test("analyzeLockfileConflicts identifies unified foundations and multi-version splits", () => {
    const sampleLock = `
[[package]]
name = "wgpu"
version = "30.0.0"

[[package]]
name = "ropey"
version = "1.6.1"
source = "registry+https://github.com/rust-lang/crates.io-index"

[[package]]
name = "ropey"
version = "2.0.0-beta.1"
source = "registry+https://github.com/rust-lang/crates.io-index"

[[package]]
name = "nsh-base"
version = "0.6.0"
dependencies = [
 "ropey 1.6.1",
]

[[package]]
name = "gpui-base"
version = "0.10.0"
dependencies = [
 "ropey 2.0.0-beta.1",
]
`;
    const sampleCargo = `
[patch.crates-io]
wgpu = { path = "vendor/wgpu/wgpu" }

[patch."https://github.com/storytold/craft-libs"]
craft-sniff = { path = "vendor/craft-libs/crates/sniff" }
`;
    const report = analyzeLockfileConflicts(sampleLock, sampleCargo);
    expect(report.totalLockfilePackages).toBe(5);
    expect(report.multiVersionCrateCount).toBe(1);
    expect(report.patchCraftLibsCount).toBe(1);
    const ropeyConflict = report.conflicts.find((c) => c.crateName === "ropey");
    expect(ropeyConflict?.status).toBe("lockfile-multi-version");
    expect(ropeyConflict?.versions).toEqual(["1.6.1", "2.0.0-beta.1"]);
    const wgpuConflict = report.conflicts.find((c) => c.crateName === "wgpu");
    expect(wgpuConflict?.status).toBe("unified-by-patch");
  });

  test("extractManifestSummary extracts features, workspace-inherited versions, target deps, and profiles", () => {
    const sample = `
[package]
name = "gpui"
version = { workspace = true }
edition = "2024"
license = "Apache-2.0"

[features]
default = ["wayland", "x11"]
hot-reload = ["dep:subsecond", "dep:dioxus-cli-config"]

[dependencies]
taffy = { workspace = true }
accesskit = { workspace = true }

[target.'cfg(target_os = "windows")'.dependencies]
windows = { workspace = true }

[profile.rg-wasm]
opt-level = "z"
`;
    const summary = extractManifestSummary(
      sample,
      { manifestRel: "crates/gpui/Cargo.toml", ecosystem: "aphrody-gpui", role: "Core GPUI" },
      "0.2.2",
    );
    expect(summary.packageName).toBe("gpui");
    expect(summary.version).toBe("0.2.2");
    expect(summary.featureCount).toBe(2);
    expect(summary.features["hot-reload"]).toEqual(["dep:subsecond", "dep:dioxus-cli-config"]);
    expect(summary.dependencies).toEqual(["accesskit", "taffy"]);
    expect(summary.targetDepsSummary[0]).toContain("windows");
    expect(summary.profiles).toEqual(["rg-wasm"]);
  });

  test("stripNullish removes null/undefined values before TOML serialization", () => {
    expect(stripNullish({ a: 1, b: null, nested: [{ k: "x", missing: undefined }] })).toEqual({
      a: 1,
      nested: [{ k: "x" }],
    });
  });
});

describe("gpui-vs-dioxus multi-ecosystem offline fixture workflow", () => {
  test("audits kept submodules, emits TOML + Mermaid doc, and preserves doc markers", async () => {
    const fixtureRoot = mkdtempSync(join(tmpdir(), "aphrody-ecosystem-fixture-"));
    try {
      mkdirSync(join(fixtureRoot, "crates/gpui/src"), { recursive: true });
      mkdirSync(join(fixtureRoot, "crates/egui_gpui"), { recursive: true });
      mkdirSync(join(fixtureRoot, "crates/aphrody_craft/src"), { recursive: true });
      mkdirSync(join(fixtureRoot, "crates/iced_gpui/src"), { recursive: true });
      mkdirSync(join(fixtureRoot, "vendor/taffy"), { recursive: true });
      mkdirSync(join(fixtureRoot, "vendor/egui/crates/egui"), { recursive: true });

      writeFileSync(
        join(fixtureRoot, ".gitmodules"),
        [
          '[submodule "vendor/taffy"]',
          "\tpath = vendor/taffy",
          "\turl = ../../aphrody-labs/taffy.git",
          "\tbranch = main",
          '[submodule "vendor/egui"]',
          "\tpath = vendor/egui",
          "\turl = ../../aphrody-labs/egui.git",
          "\tbranch = main",
        ].join("\n"),
      );

      writeFileSync(
        join(fixtureRoot, "Cargo.toml"),
        `
[workspace]
members = ["crates/gpui", "crates/egui_gpui", "crates/aphrody_craft", "crates/iced_gpui"]
exclude = ["vendor/egui"]

[workspace.package]
version = "0.5.0"

[patch.crates-io]
taffy = { path = "vendor/taffy" }
wgpu = { path = "vendor/wgpu/wgpu" }
`,
      );
      writeFileSync(join(fixtureRoot, "Cargo.lock"), `[[package]]\nname = "wgpu"\nversion = "30.0.0"\n`);
      writeFileSync(join(fixtureRoot, "crates/gpui/Cargo.toml"), `[package]\nname = "gpui"\nversion = "0.2.2"\n[features]\ndefault = []\n`);
      writeFileSync(join(fixtureRoot, "crates/gpui/src/hot_reload.rs"), "use subsecond::JumpTable;\n");
      writeFileSync(join(fixtureRoot, "crates/egui_gpui/Cargo.toml"), `[package]\nname = "egui_gpui"\nversion = "0.37.0"\n`);
      writeFileSync(join(fixtureRoot, "crates/aphrody_craft/Cargo.toml"), `[package]\nname = "aphrody_craft"\nversion = "0.1.0"\n`);
      writeFileSync(join(fixtureRoot, "crates/iced_gpui/Cargo.toml"), `[package]\nname = "iced_gpui"\nversion = "0.14.0"\n`);
      writeFileSync(join(fixtureRoot, "vendor/taffy/Cargo.toml"), `[package]\nname = "taffy"\nversion = "0.15.1"\n[features]\ndefault = []\n`);
      writeFileSync(join(fixtureRoot, "vendor/egui/crates/egui/Cargo.toml"), `[package]\nname = "egui"\nversion = "0.37.0"\n[features]\ndefault = []\n`);

      const bridges = scanIntegrationBridges(fixtureRoot);
      expect(bridges.some((b) => b.id === "gpui-subsecond-hot-reload")).toBe(true);
      expect(bridges.some((b) => b.id === "egui-craft-bevy-bridge")).toBe(true);
      expect(bridges.some((b) => b.id === "iced-libcosmic-gpui-bridge")).toBe(true);

      const report = await runAudit({ root: fixtureRoot, offline: true });
      expect(report.offlineMode).toBe(true);
      expect(report.submodules.length).toBe(TARGET_SUBMODULES.length);
      expect(report.featureMatrix.length).toBeGreaterThanOrEqual(5);

      const parsedToml = Bun.TOML.parse(serializeMatrixToml(report)) as Record<string, any>;
      expect(parsedToml.schema_version).toBe(2);
      expect(parsedToml.submodules.length).toBe(TARGET_SUBMODULES.length);

      const mermaid = buildArchitectureMermaid(report);
      expect(mermaid).toContain("```mermaid");
      expect(mermaid).not.toContain("<br");

      const emitted = emitArtifacts(report, fixtureRoot);
      expect(existsSync(emitted.tomlPath)).toBe(true);
      expect(existsSync(emitted.docPath)).toBe(true);
      const docSecond = renderArchitectureMarkdown(report, readFileSync(emitted.docPath, "utf8"));
      expect(docSecond).toContain("<!-- BEGIN GPUI_DIOXUS_FORK_GRAPH -->");
      expect(docSecond).toContain("<!-- END GPUI_DIOXUS_UNIFICATION -->");
    } finally {
      rmSync(fixtureRoot, { recursive: true, force: true });
    }
  });
});

describe("live Aphrody multi-ecosystem audit artifacts", () => {
  test("assets/os/gpui_vs_dioxus_matrix.toml validates submodules, crates.io, feature matrix, and conflicts", () => {
    const tomlPath = join(REPO_ROOT, MATRIX_TOML_REL);
    expect(existsSync(tomlPath)).toBe(true);
    const parsed = Bun.TOML.parse(readFileSync(tomlPath, "utf8")) as Record<string, any>;

    expect(parsed.schema_version).toBe(2);
    expect(parsed.root_repo.fork_repo).toBe("Aphrody-Dev/aphrody");
    expect(parsed.root_repo.upstream_repo).toBe("zed-industries/zed");
    expect(parsed.root_repo.workspace_members_count).toBeGreaterThanOrEqual(300);

    expect(Array.isArray(parsed.submodules)).toBe(true);
    expect(parsed.submodules.length).toBe(TARGET_SUBMODULES.length);
    for (const expected of TARGET_SUBMODULES) {
      const found = parsed.submodules.find((s: Record<string, any>) => s.path === expected.path);
      expect(found).toBeDefined();
      expect(found.local_sha).toMatch(/^[0-9a-f]{40}$/);
    }

    if (!parsed.offline_mode && Array.isArray(parsed.crates_io) && parsed.crates_io.length > 0) {
      for (const crateName of CRATES_IO_TARGETS) {
        const c = parsed.crates_io.find((x: Record<string, any>) => x.name === crateName);
        expect(c).toBeDefined();
        expect(c.exists).toBe(true);
      }
    }

    expect(Array.isArray(parsed.feature_matrix)).toBe(true);
    for (const requiredManifest of [
      "crates/gpui/Cargo.toml",
      "crates/gpui_wgpu/Cargo.toml",
      "crates/egui_gpui/Cargo.toml",
      "crates/aphrody_craft/Cargo.toml",
      "crates/iced_gpui/Cargo.toml",
      "vendor/egui/crates/egui/Cargo.toml",
      "vendor/craft-libs/crates/sniff/Cargo.toml",
    ]) {
      const entry = parsed.feature_matrix.find((m: Record<string, any>) => m.manifest === requiredManifest);
      expect(entry).toBeDefined();
    }

    expect(Array.isArray(parsed.dependency_conflicts)).toBe(true);
    expect(parsed.dependency_conflicts.some((c: Record<string, any>) => c.crate_name === "wgpu" && c.status === "unified-by-patch")).toBe(true);
    expect(parsed.dependency_conflicts.some((c: Record<string, any>) => c.crate_name === "ropey")).toBe(true);
    expect(Array.isArray(parsed.architecture_matrix)).toBe(true);
    expect(parsed.architecture_matrix.length).toBe(6);
  });

  test("aphrody/docs/architecture/gpui-vs-dioxus-audit.md has Mermaid graph, markers, and valid relative links", () => {
    const docPath = join(REPO_ROOT, ARCH_DOC_REL);
    expect(existsSync(docPath)).toBe(true);
    const md = readFileSync(docPath, "utf8");

    for (const marker of [
      "GPUI_DIOXUS_METRICS",
      "GPUI_DIOXUS_FORK_GRAPH",
      "GPUI_DIOXUS_SUBMODULES",
      "GPUI_DIOXUS_CRATES_IO",
      "GPUI_DIOXUS_FEATURES",
      "GPUI_DIOXUS_BRIDGES",
      "GPUI_DIOXUS_ECOSYSTEM_MATRIX",
      "GPUI_DIOXUS_UNIFICATION",
    ]) {
      expect(md).toContain(`<!-- BEGIN ${marker} -->`);
      expect(md).toContain(`<!-- END ${marker} -->`);
    }

    expect(md).toContain("```mermaid");
    expect(md).not.toMatch(/<br\s*\/?>/i);
    expect(md).not.toMatch(/\.gemini\/antigravity-cli\/brain/i);

    for (const match of md.matchAll(/\]\(([^)]+)\)/g)) {
      const rawTarget = match[1].split("#")[0].trim();
      if (!rawTarget || rawTarget.startsWith("http://") || rawTarget.startsWith("https://")) continue;
      const resolved = resolve(dirname(docPath), rawTarget);
      expect(existsSync(resolved) || isInsideUninitialisedSubmodule(resolved, REPO_ROOT)).toBe(true);
    }
  });

  test("target/gpui-vs-dioxus/audit.json matches audit schema when emitted", () => {
    const jsonPath = join(REPO_ROOT, AUDIT_JSON_REL);
    if (!existsSync(jsonPath)) return;
    const report = JSON.parse(readFileSync(jsonPath, "utf8")) as GpuiVsDioxusAuditReport;
    expect(report.schemaVersion).toBe(2);
    expect(report.submodules.length).toBe(TARGET_SUBMODULES.length);
    expect(report.featureMatrix.length).toBeGreaterThanOrEqual(25);
    expect(report.architectureMatrix.length).toBe(6);
  });

  test("repo-hygiene passes cleanly", () => {
    const hygiene = checkRepoHygiene(REPO_ROOT);
    expect(hygiene.ephemeralLeaks).toEqual([]);
    expect(hygiene.brokenArchLinks).toEqual([]);
    expect(hygiene.ok).toBe(true);
  });
});
