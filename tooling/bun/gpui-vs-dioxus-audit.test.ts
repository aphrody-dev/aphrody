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
  FORKS_MANIFEST_REL,
  forkTagVersion,
  type GpuiVsDioxusAuditReport,
  isForkPinnedInCargo,
  MATRIX_TOML_REL,
  readForksManifest,
  renderArchitectureMarkdown,
  REPO_ROOT,
  runAudit,
  scanIntegrationBridges,
  serializeMatrixToml,
  stripNullish,
  TARGET_FORKS,
} from "./gpui-vs-dioxus-audit.ts";
import { checkRepoHygiene } from "./repo-hygiene.ts";

describe("gpui-vs-dioxus-audit pure parsers & conflict analyzer", () => {
  test("readForksManifest, isForkPinnedInCargo and forkTagVersion derive pins from forks-manifest.json", () => {
    const manifest = readForksManifest(REPO_ROOT);
    for (const spec of TARGET_FORKS) {
      const pin = manifest.find((f) => f.name === spec.name);
      expect(pin).toBeDefined();
      expect(pin?.repo).toBe(spec.forkRepo);
      expect(pin?.sha).toMatch(/^[0-9a-f]{40}$/);
    }
    const fork = { gitUrl: "https://github.com/aphrody-labs/taffy.git", tag: "v0.15.1-aphrody.20261120" };
    const pinned = `taffy = { git = "${fork.gitUrl}", tag = "${fork.tag}", default-features = false }`;
    expect(isForkPinnedInCargo(pinned, fork)).toBe(true);
    expect(isForkPinnedInCargo(pinned.replace(fork.tag, "v0.0.0"), fork)).toBe(false);
    expect(isForkPinnedInCargo('taffy = { path = "crates/taffy" }', fork)).toBe(false);
    expect(forkTagVersion("v30.0.0-aphrody.20261120")).toBe("30.0.0");
    expect(forkTagVersion("v0.1.0-aphrody.20261120.2")).toBe("0.1.0");
  });

  test("classifyForkDrift accurately distinguishes identical, ahead, behind, diverged, and pin lag", () => {
    expect(
      classifyForkDrift({ pinnedSha: "aaa", forkSha: "aaa", upstreamSha: "aaa", aheadBy: 0, behindBy: 0, pinBehindFork: 0 }),
    ).toBe("identical");
    expect(
      classifyForkDrift({ pinnedSha: "bbb", forkSha: "bbb", upstreamSha: "aaa", aheadBy: 2, behindBy: 0, pinBehindFork: 0 }),
    ).toBe("ahead (+2)");
    expect(
      classifyForkDrift({ pinnedSha: "aaa", forkSha: "aaa", upstreamSha: "bbb", aheadBy: 0, behindBy: 1, pinBehindFork: 0 }),
    ).toBe("behind (-1)");
    expect(
      classifyForkDrift({ pinnedSha: "ccc", forkSha: "ccc", upstreamSha: "bbb", aheadBy: 1, behindBy: 291, pinBehindFork: 0 }),
    ).toBe("diverged (+1/-291)");
    expect(
      classifyForkDrift({ pinnedSha: "aaa", forkSha: "bbb", upstreamSha: "bbb", aheadBy: 0, behindBy: 0, pinBehindFork: 5 }),
    ).toBe("pin-behind-fork (-5)");
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
wgpu = { git = "https://github.com/aphrody-labs/wgpu.git", tag = "v30.0.0-aphrody.20261120" }

[patch."https://github.com/storytold/craft-libs"]
craft-sniff = { git = "https://github.com/aphrody-labs/craft-libs.git", tag = "v0.1.0-aphrody.20261120.2" }
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
  test("audits pinned forks, emits TOML + Mermaid doc, and preserves doc markers", async () => {
    const fixtureRoot = mkdtempSync(join(tmpdir(), "aphrody-ecosystem-fixture-"));
    try {
      mkdirSync(join(fixtureRoot, "crates/gpui/src"), { recursive: true });
      mkdirSync(join(fixtureRoot, "crates/egui_gpui"), { recursive: true });
      mkdirSync(join(fixtureRoot, "crates/aphrody_craft/src"), { recursive: true });
      mkdirSync(join(fixtureRoot, "crates/iced_gpui/src"), { recursive: true });
      mkdirSync(join(fixtureRoot, "crates/gpui_wgpu"), { recursive: true });
      mkdirSync(join(fixtureRoot, "crates/aphrody_ui"), { recursive: true });
      mkdirSync(join(fixtureRoot, "tooling/bun"), { recursive: true });

      const fixtureForks = [
        { name: "taffy", upstream: "DioxusLabs/taffy", tag: "v0.15.1-aphrody.20261120", sha: "7".repeat(40) },
        { name: "egui", upstream: "emilk/egui", tag: "v0.37.0-aphrody.20261120", sha: "e".repeat(40) },
      ].map((f) => ({
        ...f,
        repo: `aphrody-labs/${f.name}`,
        branch: "main",
        gitUrl: `https://github.com/aphrody-labs/${f.name}.git`,
      }));
      writeFileSync(join(fixtureRoot, FORKS_MANIFEST_REL), JSON.stringify({ schemaVersion: 1, forks: fixtureForks }));

      writeFileSync(
        join(fixtureRoot, "Cargo.toml"),
        `
[workspace]
members = ["crates/gpui", "crates/egui_gpui", "crates/aphrody_craft", "crates/iced_gpui"]
exclude = ["crates/bevy_gpui"]

[workspace.package]
version = "0.5.0"

[workspace.dependencies]
taffy = { git = "https://github.com/aphrody-labs/taffy.git", tag = "v0.15.1-aphrody.20261120" }
egui = { git = "https://github.com/aphrody-labs/egui.git", tag = "v0.37.0-aphrody.20261120" }
`,
      );
      writeFileSync(join(fixtureRoot, "Cargo.lock"), `[[package]]\nname = "wgpu"\nversion = "30.0.0"\n`);
      writeFileSync(join(fixtureRoot, "crates/gpui/Cargo.toml"), `[package]\nname = "gpui"\nversion = "0.2.2"\n[features]\ndefault = []\n`);
      writeFileSync(join(fixtureRoot, "crates/gpui/src/hot_reload.rs"), "use subsecond::JumpTable;\n");
      writeFileSync(join(fixtureRoot, "crates/egui_gpui/Cargo.toml"), `[package]\nname = "egui_gpui"\nversion = "0.37.0"\n`);
      writeFileSync(join(fixtureRoot, "crates/aphrody_craft/Cargo.toml"), `[package]\nname = "aphrody_craft"\nversion = "0.1.0"\n`);
      writeFileSync(join(fixtureRoot, "crates/iced_gpui/Cargo.toml"), `[package]\nname = "iced_gpui"\nversion = "0.14.0"\n`);
      writeFileSync(join(fixtureRoot, "crates/gpui_wgpu/Cargo.toml"), `[package]\nname = "gpui_wgpu"\nversion = "0.2.2"\n[features]\ndefault = []\n`);
      writeFileSync(join(fixtureRoot, "crates/aphrody_ui/Cargo.toml"), `[package]\nname = "aphrody_ui"\nversion = "0.5.0"\n[features]\ndefault = []\n`);

      const bridges = scanIntegrationBridges(fixtureRoot);
      expect(bridges.some((b) => b.id === "gpui-subsecond-hot-reload")).toBe(true);
      expect(bridges.some((b) => b.id === "egui-craft-bevy-bridge")).toBe(true);
      expect(bridges.some((b) => b.id === "iced-libcosmic-gpui-bridge")).toBe(true);

      const report = await runAudit({ root: fixtureRoot, offline: true });
      expect(report.offlineMode).toBe(true);
      expect(report.forks.length).toBe(TARGET_FORKS.length);
      const pinnedNames = report.forks.filter((f) => f.pinnedInCargo).map((f) => f.name).sort();
      expect(pinnedNames).toEqual(["egui", "taffy"]);
      expect(report.forks.find((f) => f.name === "taffy")?.pinnedTag).toBe("v0.15.1-aphrody.20261120");
      expect(report.forks.find((f) => f.name === "wgpu")?.pinnedInCargo).toBe(false);
      expect(report.rootRepo.pinnedForkCount).toBe(2);
      expect(report.summary.pinnedInCargoCount).toBe(2);
      expect(report.featureMatrix.length).toBe(6);

      const parsedToml = Bun.TOML.parse(serializeMatrixToml(report)) as Record<string, any>;
      expect(parsedToml.schema_version).toBe(2);
      expect(parsedToml.forks.length).toBe(TARGET_FORKS.length);
      expect(parsedToml.forks.find((f: Record<string, any>) => f.name === "egui").pinned_tag).toBe("v0.37.0-aphrody.20261120");
      expect(parsedToml.submodules).toBeUndefined();

      const mermaid = buildArchitectureMermaid(report);
      expect(mermaid).toContain("```mermaid");
      expect(mermaid).not.toContain("<br");
      expect(mermaid).toContain("aphrody-labs/taffy@v0.15.1-aphrody.20261120");
      expect(mermaid).not.toContain("vendor/");

      const emitted = emitArtifacts(report, fixtureRoot);
      expect(existsSync(emitted.tomlPath)).toBe(true);
      expect(existsSync(emitted.docPath)).toBe(true);
      const docSecond = renderArchitectureMarkdown(report, readFileSync(emitted.docPath, "utf8"));
      expect(docSecond).toContain("<!-- BEGIN GPUI_DIOXUS_FORK_GRAPH -->");
      expect(docSecond).toContain("<!-- END GPUI_DIOXUS_UNIFICATION -->");
      expect(docSecond).toContain("<!-- BEGIN GPUI_DIOXUS_FORKS -->");
      expect(docSecond).not.toMatch(/submodule|vendor\//i);
    } finally {
      rmSync(fixtureRoot, { recursive: true, force: true });
    }
  });
});

describe("live Aphrody multi-ecosystem audit artifacts", () => {
  test("assets/os/gpui_vs_dioxus_matrix.toml validates pinned forks, crates.io, feature matrix, and conflicts", () => {
    const tomlPath = join(REPO_ROOT, MATRIX_TOML_REL);
    expect(existsSync(tomlPath)).toBe(true);
    const parsed = Bun.TOML.parse(readFileSync(tomlPath, "utf8")) as Record<string, any>;

    expect(parsed.schema_version).toBe(2);
    expect(parsed.root_repo.fork_repo).toBe("Aphrody-Dev/aphrody");
    expect(parsed.root_repo.upstream_repo).toBe("zed-industries/zed");
    expect(parsed.root_repo.workspace_members_count).toBeGreaterThanOrEqual(300);

    expect(Array.isArray(parsed.forks)).toBe(true);
    expect(parsed.forks.length).toBe(TARGET_FORKS.length);
    const manifest = readForksManifest(REPO_ROOT);
    for (const expected of TARGET_FORKS) {
      const found = parsed.forks.find((f: Record<string, any>) => f.name === expected.name);
      expect(found).toBeDefined();
      expect(found.pinned_sha).toMatch(/^[0-9a-f]{40}$/);
      expect(found.pinned_tag).toBe(manifest.find((f) => f.name === expected.name)?.tag);
      expect(found.pinned_in_cargo).toBe(true);
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

  test("docs/aphrody/architecture/gpui-vs-dioxus-audit.md has Mermaid graph, markers, and valid relative links", () => {
    const docPath = join(REPO_ROOT, ARCH_DOC_REL);
    expect(existsSync(docPath)).toBe(true);
    const md = readFileSync(docPath, "utf8");

    for (const marker of [
      "GPUI_DIOXUS_METRICS",
      "GPUI_DIOXUS_FORK_GRAPH",
      "GPUI_DIOXUS_FORKS",
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
    expect(md).not.toMatch(/submodule|vendor\//i);

    for (const match of md.matchAll(/\]\(([^)]+)\)/g)) {
      const rawTarget = match[1].split("#")[0].trim();
      if (!rawTarget || rawTarget.startsWith("http://") || rawTarget.startsWith("https://")) continue;
      const resolved = resolve(dirname(docPath), rawTarget);
      expect(existsSync(resolved)).toBe(true);
    }
  });

  test("target/gpui-vs-dioxus/audit.json matches audit schema when emitted", () => {
    const jsonPath = join(REPO_ROOT, AUDIT_JSON_REL);
    if (!existsSync(jsonPath)) return;
    const report = JSON.parse(readFileSync(jsonPath, "utf8")) as GpuiVsDioxusAuditReport;
    expect(report.schemaVersion).toBe(2);
    expect(report.forks.length).toBe(TARGET_FORKS.length);
    expect(report.featureMatrix.length).toBeGreaterThanOrEqual(20);
    expect(report.architectureMatrix.length).toBe(6);
  });

  test("repo-hygiene passes cleanly", () => {
    const hygiene = checkRepoHygiene(REPO_ROOT);
    expect(hygiene.ephemeralLeaks).toEqual([]);
    expect(hygiene.brokenArchLinks).toEqual([]);
    expect(hygiene.ok).toBe(true);
  });
});
