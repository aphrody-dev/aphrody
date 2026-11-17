import { describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import {
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

describe("gpui-vs-dioxus-audit pure parsers & classifiers", () => {
  test("parseSubmoduleStatusOutput parses clean, modified, uninitialized, and conflict rows", () => {
    const sample = [
      " 164e07f94d3cfe4e46430a98925b414b061ef4ea vendor/dioxus (heads/main)",
      "+7bade8ead71c7ea852042c0cf156241aca32cc4c vendor/blitz (heads/main)",
      "-92d2a9ea4f27075f840b6d36a42da8a1ffc1746f vendor/anyrender",
      "U2649e09d44ec4a1b928d957cbb0043ae1a3f56a5 vendor/taffy (heads/main)",
    ].join("\n");
    const parsed = parseSubmoduleStatusOutput(sample);
    expect(parsed).toEqual([
      {
        state: "clean",
        sha: "164e07f94d3cfe4e46430a98925b414b061ef4ea",
        path: "vendor/dioxus",
        refDesc: "heads/main",
      },
      {
        state: "modified",
        sha: "7bade8ead71c7ea852042c0cf156241aca32cc4c",
        path: "vendor/blitz",
        refDesc: "heads/main",
      },
      {
        state: "uninitialized",
        sha: "92d2a9ea4f27075f840b6d36a42da8a1ffc1746f",
        path: "vendor/anyrender",
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
      classifyForkDrift({
        localSha: "aaa",
        forkSha: "aaa",
        upstreamSha: "aaa",
        aheadBy: 0,
        behindBy: 0,
        localBehindFork: 0,
      }),
    ).toBe("identical");

    expect(
      classifyForkDrift({
        localSha: "bbb",
        forkSha: "bbb",
        upstreamSha: "aaa",
        aheadBy: 2,
        behindBy: 0,
        localBehindFork: 0,
      }),
    ).toBe("ahead (+2)");

    expect(
      classifyForkDrift({
        localSha: "aaa",
        forkSha: "aaa",
        upstreamSha: "bbb",
        aheadBy: 0,
        behindBy: 1,
        localBehindFork: 0,
      }),
    ).toBe("behind (-1)");

    expect(
      classifyForkDrift({
        localSha: "ccc",
        forkSha: "ccc",
        upstreamSha: "bbb",
        aheadBy: 1,
        behindBy: 291,
        localBehindFork: 0,
      }),
    ).toBe("diverged (+1/-291)");

    expect(
      classifyForkDrift({
        localSha: "aaa",
        forkSha: "bbb",
        upstreamSha: "bbb",
        aheadBy: 0,
        behindBy: 0,
        localBehindFork: 5,
      }),
    ).toBe("local-behind-fork (-5)");

    expect(
      classifyForkDrift({
        localSha: "aaa",
        forkSha: "bbb",
        upstreamSha: "bbb",
        aheadBy: null,
        behindBy: null,
        localBehindFork: null,
      }),
    ).toBe("local-diverged-from-fork");
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
    expect(summary.targetDepsSummary.length).toBe(1);
    expect(summary.targetDepsSummary[0]).toContain("windows");
    expect(summary.profiles).toEqual(["rg-wasm"]);
  });

  test("stripNullish removes null/undefined values before TOML serialization", () => {
    const cleaned = stripNullish({ a: 1, b: null, nested: [{ k: "x", missing: undefined }] });
    expect(cleaned).toEqual({ a: 1, nested: [{ k: "x" }] });
  });
});

describe("gpui-vs-dioxus-audit offline fixture workflow", () => {
  test("audits bridges, emits TOML + Mermaid doc, and preserves doc markers on update", async () => {
    const fixtureRoot = mkdtempSync(join(tmpdir(), "aphrody-gpui-dioxus-fixture-"));
    try {
      mkdirSync(join(fixtureRoot, "crates/gpui/src"), { recursive: true });
      mkdirSync(join(fixtureRoot, "crates/gpui_wgpu/src"), { recursive: true });
      mkdirSync(join(fixtureRoot, "crates/aphrody_ui/src"), { recursive: true });
      mkdirSync(join(fixtureRoot, "vendor/dioxus/packages/dioxus"), { recursive: true });
      mkdirSync(join(fixtureRoot, "vendor/blitz/examples/wasm_hello"), { recursive: true });
      mkdirSync(join(fixtureRoot, "vendor/anyrender"), { recursive: true });
      mkdirSync(join(fixtureRoot, "vendor/taffy"), { recursive: true });
      mkdirSync(join(fixtureRoot, "tooling/bun"), { recursive: true });

      writeFileSync(
        join(fixtureRoot, ".gitmodules"),
        [
          '[submodule "vendor/dioxus"]',
          "\tpath = vendor/dioxus",
          "\turl = ../../aphrody-labs/dioxus.git",
          "\tbranch = main",
          '[submodule "vendor/taffy"]',
          "\tpath = vendor/taffy",
          "\turl = ../../aphrody-labs/taffy.git",
          "\tbranch = main",
        ].join("\n"),
      );

      writeFileSync(
        join(fixtureRoot, "Cargo.toml"),
        `
[workspace]
members = ["crates/gpui", "crates/gpui_wgpu", "crates/aphrody_ui"]
exclude = ["vendor/dioxus", "vendor/blitz", "vendor/anyrender", "vendor/taffy"]

[workspace.package]
version = "0.5.0"

[patch.crates-io]
taffy = { path = "vendor/taffy" }
wgpu = { path = "vendor/wgpu/wgpu" }
`,
      );

      writeFileSync(
        join(fixtureRoot, "crates/gpui/Cargo.toml"),
        `
[package]
name = "gpui"
version = "0.2.2"

[features]
default = []
hot-reload = ["dep:subsecond"]

[dependencies]
taffy.workspace = true

[target.'cfg(not(target_family = "wasm"))'.dependencies]
subsecond = { version = "=0.7.10", optional = true }
`,
      );
      writeFileSync(
        join(fixtureRoot, "crates/gpui/src/hot_reload.rs"),
        "use dioxus_devtools_types::DevserverMsg;\nuse subsecond::JumpTable;\n",
      );
      writeFileSync(
        join(fixtureRoot, "crates/gpui_wgpu/Cargo.toml"),
        `[package]\nname = "gpui_wgpu"\nversion = "0.1.0"\n`,
      );
      writeFileSync(
        join(fixtureRoot, "crates/gpui_wgpu/src/wgpu_context.rs"),
        "/// bridges (`aphrody_ui`, `anyrender`) share one device.\npub fn shared_handles() {}\n",
      );
      writeFileSync(
        join(fixtureRoot, "crates/aphrody_ui/Cargo.toml"),
        `[package]\nname = "aphrody_ui"\nversion = "0.1.0"\n`,
      );
      writeFileSync(
        join(fixtureRoot, "crates/aphrody_ui/src/unified_runtime.rs"),
        "pub enum FrameworkLayer { Gpui, Blitz }\n",
      );
      writeFileSync(
        join(fixtureRoot, "vendor/dioxus/Cargo.toml"),
        `[workspace]\nmembers = ["packages/dioxus"]\n[workspace.package]\nversion = "0.8.0-alpha.3"\n[workspace.dependencies]\nanyrender = { version = "0.14.0" }\n`,
      );
      writeFileSync(
        join(fixtureRoot, "vendor/dioxus/packages/dioxus/Cargo.toml"),
        `[package]\nname = "dioxus"\nversion = { workspace = true }\n[features]\ndefault = []\nnative = []\n`,
      );
      writeFileSync(
        join(fixtureRoot, "vendor/blitz/Cargo.toml"),
        `[workspace]\nmembers = []\n[workspace.package]\nversion = "0.3.0-beta.4"\n[workspace.dependencies]\nanyrender = "0.15.0"\n`,
      );
      writeFileSync(
        join(fixtureRoot, "vendor/blitz/examples/wasm_hello/Cargo.toml"),
        `[package]\nname = "wasm_hello"\nversion = "0.0.0"\n`,
      );
      writeFileSync(
        join(fixtureRoot, "vendor/anyrender/Cargo.toml"),
        `[workspace]\nmembers = []\n[workspace.dependencies]\nanyrender_vello_gpu = { version = "0.12.0" }\n`,
      );
      writeFileSync(
        join(fixtureRoot, "vendor/taffy/Cargo.toml"),
        `[package]\nname = "taffy"\nversion = "0.15.1"\n[features]\ndefault = []\n`,
      );
      writeFileSync(
        join(fixtureRoot, "tooling/bun/web-build.ts"),
        `const specs = [{ id: "blitz", packageName: "wasm_hello" }];\n`,
      );

      const bridges = scanIntegrationBridges(fixtureRoot);
      expect(bridges.some((b) => b.id === "gpui-subsecond-hot-reload")).toBe(true);
      expect(bridges.some((b) => b.id === "verified-version-splits")).toBe(true);

      const report = await runAudit({ root: fixtureRoot, offline: true });
      expect(report.offlineMode).toBe(true);
      expect(report.submodules.length).toBe(TARGET_SUBMODULES.length);
      expect(report.featureMatrix.length).toBeGreaterThanOrEqual(5);

      const tomlText = serializeMatrixToml(report);
      const parsedToml = Bun.TOML.parse(tomlText) as Record<string, any>;
      expect(parsedToml.schema_version).toBe(1);
      expect(parsedToml.submodules.length).toBe(TARGET_SUBMODULES.length);

      const mermaid = buildArchitectureMermaid(report);
      expect(mermaid).toContain("```mermaid");
      expect(mermaid).toContain("flowchart LR");
      expect(mermaid).not.toContain("<br");

      const emitted = emitArtifacts(report, fixtureRoot);
      expect(existsSync(emitted.tomlPath)).toBe(true);
      expect(existsSync(emitted.docPath)).toBe(true);

      const docFirst = readFileSync(emitted.docPath, "utf8");
      const docSecond = renderArchitectureMarkdown(report, docFirst);
      expect(docSecond).toContain("<!-- BEGIN GPUI_DIOXUS_FORK_GRAPH -->");
      expect(docSecond).toContain("<!-- END GPUI_DIOXUS_UNIFICATION -->");
    } finally {
      rmSync(fixtureRoot, { recursive: true, force: true });
    }
  });
});

describe("live Aphrody GPUI vs Dioxus audit artifacts", () => {
  test("assets/os/gpui_vs_dioxus_matrix.toml validates schema, submodules, crates.io, and feature matrices", () => {
    const tomlPath = join(REPO_ROOT, MATRIX_TOML_REL);
    expect(existsSync(tomlPath)).toBe(true);
    const parsed = Bun.TOML.parse(readFileSync(tomlPath, "utf8")) as Record<string, any>;

    expect(parsed.schema_version).toBe(1);
    expect(parsed.offline_mode).toBe(false);
    expect(parsed.root_repo.fork_repo).toBe("Aphrody-Dev/aphrody");
    expect(parsed.root_repo.upstream_repo).toBe("zed-industries/zed");
    expect(parsed.root_repo.workspace_members_count).toBeGreaterThanOrEqual(300);

    expect(Array.isArray(parsed.submodules)).toBe(true);
    expect(parsed.submodules.length).toBe(TARGET_SUBMODULES.length);
    for (const expected of TARGET_SUBMODULES) {
      const found = parsed.submodules.find((s: Record<string, any>) => s.path === expected.path);
      expect(found).toBeDefined();
      expect(found.local_sha).toMatch(/^[0-9a-f]{40}$/);
      expect(found.fork_sha).toMatch(/^[0-9a-f]{40}$/);
      expect(found.upstream_sha).toMatch(/^[0-9a-f]{40}$/);
    }

    expect(Array.isArray(parsed.crates_io)).toBe(true);
    for (const crateName of CRATES_IO_TARGETS) {
      const c = parsed.crates_io.find((x: Record<string, any>) => x.name === crateName);
      expect(c).toBeDefined();
      expect(c.exists).toBe(true);
      expect(c.max_version).not.toBe("unknown");
    }

    expect(Array.isArray(parsed.feature_matrix)).toBe(true);
    for (const requiredManifest of [
      "crates/gpui/Cargo.toml",
      "crates/gpui_wgpu/Cargo.toml",
      "crates/gpui_web/Cargo.toml",
      "crates/gpui_ui/Cargo.toml",
      "vendor/dioxus/Cargo.toml",
      "vendor/dioxus/packages/dioxus/Cargo.toml",
      "vendor/blitz/Cargo.toml",
    ]) {
      const entry = parsed.feature_matrix.find((m: Record<string, any>) => m.manifest === requiredManifest);
      expect(entry).toBeDefined();
      expect(entry.feature_count).toBeGreaterThan(0);
    }

    const wgpuEntry = parsed.feature_matrix.find((m: Record<string, any>) => m.manifest === "vendor/wgpu/wgpu/Cargo.toml");
    expect(wgpuEntry?.version).toBe("30.0.0");

    expect(Array.isArray(parsed.integration_bridges)).toBe(true);
    expect(parsed.integration_bridges.some((b: Record<string, any>) => b.id === "gpui-subsecond-hot-reload")).toBe(true);
    expect(parsed.integration_bridges.some((b: Record<string, any>) => b.id === "gpui-wgpu-shared-handles")).toBe(true);
    expect(parsed.integration_bridges.some((b: Record<string, any>) => b.id === "verified-version-splits")).toBe(true);

    expect(Array.isArray(parsed.architecture_matrix)).toBe(true);
    expect(parsed.architecture_matrix.length).toBe(8);
  });

  test("docs/architecture/gpui-vs-dioxus-audit.md has complete Mermaid graph, markers, and valid relative links", () => {
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
    for (const upstream of [
      "zed-industries/zed",
      "DioxusLabs/dioxus",
      "DioxusLabs/blitz",
      "DioxusLabs/anyrender",
      "DioxusLabs/taffy",
      "gfx-rs/wgpu",
      "linebender/vello",
      "pop-os/cosmic-text",
      "AccessKit/accesskit",
      "DioxusLabs/winit",
    ]) {
      expect(md).toContain(upstream);
    }

    // Verify zero ephemeral leaks and all relative links resolve on disk.
    expect(md).not.toMatch(/\.gemini\/antigravity-cli\/brain/i);
    expect(md).not.toMatch(/AppData\/Local\/Temp/i);
    for (const match of md.matchAll(/\]\(([^)]+)\)/g)) {
      const rawTarget = match[1].split("#")[0].trim();
      if (!rawTarget || rawTarget.startsWith("http://") || rawTarget.startsWith("https://")) continue;
      const resolved = resolve(dirname(docPath), rawTarget);
      expect(existsSync(resolved) || isInsideUninitialisedSubmodule(resolved, REPO_ROOT)).toBe(true);
    }
  });

  test("target/gpui-vs-dioxus/audit.json matches audit report invariants when emitted", () => {
    const jsonPath = join(REPO_ROOT, AUDIT_JSON_REL);
    if (!existsSync(jsonPath)) return;
    const report = JSON.parse(readFileSync(jsonPath, "utf8")) as GpuiVsDioxusAuditReport;
    expect(report.schemaVersion).toBe(1);
    expect(report.submodules.length).toBe(9);
    expect(report.featureMatrix.length).toBeGreaterThanOrEqual(45);
    expect(report.bridges.length).toBeGreaterThanOrEqual(6);
    expect(report.architectureMatrix.length).toBe(8);
  });

  test("repo-hygiene passes cleanly with new files present", () => {
    const hygiene = checkRepoHygiene(REPO_ROOT);
    expect(hygiene.ephemeralLeaks).toEqual([]);
    expect(hygiene.brokenArchLinks).toEqual([]);
    expect(hygiene.ok).toBe(true);
  });
});
