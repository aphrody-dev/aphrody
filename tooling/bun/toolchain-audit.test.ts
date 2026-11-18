import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import {
  analyzeProfiles,
  auditWorkspaceHotspots,
  buildForkPlan,
  buildRecommendedPatches,
  parseCargoConfigToml,
  parseCargoLockDuplicates,
  parseSccacheStatsOutput,
  parseToolchainToml,
  runDoctor,
} from "./toolchain-audit.ts";

const repoRoot = resolve(import.meta.dir, "../..");

describe("aphrody Rust toolchain & Cargo optimization suite", () => {
  test("parseToolchainToml parses pinned repo toolchain and targets", () => {
    const raw = readFileSync(join(repoRoot, "rust-toolchain.toml"), "utf8");
    const tc = parseToolchainToml(raw);
    expect(tc.channel).toBe("1.100.0");
    expect(tc.profile).toBe("minimal");
    expect(tc.components).toContain("clippy");
    expect(tc.components).toContain("rustfmt");
    expect(tc.targets).toContain("wasm32-unknown-unknown");
    expect(tc.targets).toContain("wasm32-wasip2");
    expect(tc.targets).toContain("x86_64-unknown-linux-musl");
  });

  test("parseCargoConfigToml reads repo .cargo/config.toml aliases, env, and target flags", () => {
    const raw = readFileSync(join(repoRoot, ".cargo/config.toml"), "utf8");
    const cfg = parseCargoConfigToml(raw);
    expect(cfg.env.RUSTC_BOOTSTRAP).toBe("1");
    expect(cfg.aliases.xtask).toBe("run --package xtask --");
    expect(Array.isArray(cfg.aliases["perf-test"])).toBe(true);
    expect(cfg.targetRustflags["aarch64-unknown-linux-gnu"]).toContain("link-arg=-fuse-ld=lld");
    expect(cfg.targetRustflags['cfg(target_os = "windows")']).toContain("windows_slim_errors");
  });

  test("parseCargoLockDuplicates aggregates real lockfile duplicates cleanly", () => {
    const sample = `
[[package]]
name = "hashbrown"
version = "0.14.5"

[[package]]
name = "hashbrown"
version = "0.17.1"

[[package]]
name = "serde"
version = "1.0.228"
`;
    const sampleStats = parseCargoLockDuplicates(sample);
    expect(sampleStats.totalEntries).toBe(3);
    expect(sampleStats.uniqueCrates).toBe(2);
    expect(sampleStats.duplicatedNames).toBe(1);
    expect(sampleStats.topDuplicates[0]).toEqual({ name: "hashbrown", versions: ["0.14.5", "0.17.1"] });

    const lockPath = join(repoRoot, "Cargo.lock");
    if (existsSync(lockPath)) {
      const realStats = parseCargoLockDuplicates(readFileSync(lockPath, "utf8"));
      expect(realStats.totalEntries).toBeGreaterThan(1500);
      expect(realStats.duplicatedNames).toBeGreaterThan(100);
      expect(realStats.presentCrates.serde_derive?.length).toBeGreaterThanOrEqual(1);
    }
  });

  test("analyzeProfiles flags real gaps in repo manifests and clears when recommended patches are applied", () => {
    const cargoToml = readFileSync(join(repoRoot, "Cargo.toml"), "utf8");
    const cargoConfig = readFileSync(join(repoRoot, ".cargo/config.toml"), "utf8");
    const bundleConfig = readFileSync(join(repoRoot, ".cargo/bundle-config.toml"), "utf8");
    const lockStats = parseCargoLockDuplicates(readFileSync(join(repoRoot, "Cargo.lock"), "utf8"));

    const liveReport = analyzeProfiles(cargoToml, cargoConfig, lockStats, bundleConfig);
    expect(liveReport.workspaceMembers).toBeGreaterThanOrEqual(300);
    expect(liveReport.excludedCount).toBeGreaterThanOrEqual(10);
    expect(liveReport.defaultMembers).toBeGreaterThanOrEqual(7);
    expect(liveReport.profiles.dev?.debug).toBe("line-tables-only");

    // Baseline unoptimized manifests must still trigger all 7 optimization findings.
    const unoptimizedCargo = `
[workspace]
members = ["crates/a"]
exclude = []
default-members = ["crates/a"]
[profile.dev]
debug = "limited"
`;
    const baselineReport = analyzeProfiles(unoptimizedCargo, "[build]\njobs = 4\n", lockStats, "[build]\n");
    const ids = new Set(baselineReport.findings.map((f) => f.id));
    expect(ids.has("profile.dev.debug-line-tables")).toBe(true);
    expect(ids.has("profile.dev.deps-no-debug")).toBe(true);
    expect(ids.has("config.windows.lld-linker")).toBe(true);
    expect(ids.has("config.linux.fast-linker")).toBe(true);
    expect(ids.has("config.build-dir-separation")).toBe(true);
    expect(ids.has("config.feature-unification")).toBe(true);
    expect(ids.has("bundle.share-generics")).toBe(true);

    const patches = buildRecommendedPatches();
    const tunedCargoToml = `
[workspace]
members = ["crates/a"]
exclude = []
default-members = ["crates/a"]
${patches.cargoToml}
`;
    const tunedConfigToml = `
${patches.cargoConfigToml}
[target.'cfg(all())']
rustflags = ["--jobs-frontend=8"]
`;
    const tunedReport = analyzeProfiles(tunedCargoToml, tunedConfigToml, null, patches.bundleConfigToml);
    expect(tunedReport.findings).toEqual([]);
  });

  test("recommended patch snippets parse cleanly as valid TOML and JSON", () => {
    const patches = buildRecommendedPatches();
    expect(() => Bun.TOML.parse(patches.cargoToml)).not.toThrow();
    expect(() => Bun.TOML.parse(patches.cargoConfigToml)).not.toThrow();
    expect(() => Bun.TOML.parse(patches.bundleConfigToml)).not.toThrow();
    expect(() => Bun.TOML.parse(patches.rustToolchainToml)).not.toThrow();
    const pkg = JSON.parse(patches.packageJsonScripts) as Record<string, string>;
    expect(pkg["toolchain:doctor"]).toContain("toolchain-audit.ts doctor");
  });

  test("auditWorkspaceHotspots detects exact workspace counts and nsh-core lint boundary", () => {
    const hotspots = auditWorkspaceHotspots(repoRoot);
    expect(hotspots.memberBreakdown.total).toBeGreaterThanOrEqual(300);
    expect(hotspots.memberBreakdown.crates).toBeGreaterThanOrEqual(290);
    expect(hotspots.memberBreakdown.extensions).toBe(4);
    expect(hotspots.memberBreakdown.tooling).toBe(4);
    expect(hotspots.nshLintBoundary.clippyDisallowsStdProcessCommand).toBe(true);
    expect(hotspots.nshLintBoundary.totalStdCommandSites).toBeGreaterThan(0);
    const affectedNames = hotspots.nshLintBoundary.affectedCrates.map((c) => c.crateName);
    expect(affectedNames).toContain("nsh-core");
    expect(affectedNames).toContain("nsh-cli");
    for (const item of hotspots.nshLintBoundary.affectedCrates) {
      expect(item.inheritsWorkspaceLints).toBe(true);
      expect(item.dependsOnSmol).toBe(false);
    }
  });

  test("parseSccacheStatsOutput extracts hit rate, baseDirs coverage, and non-cacheable reasons", () => {
    const sample = `
Compile requests                      0
Compile requests executed           936
Cache hits                           45
Cache misses                        889
Cache hits rate                    4.82 %
Cache location                  Local disk: "C:\\\\Users\\\\aphro\\\\AppData\\\\Local\\\\Mozilla\\\\sccache\\\\cache"
Base directories                c:/labs/, c:/aphrody/
Version (client)                0.19.0

Non-cacheable reasons:
incremental                         520
multiple input files                440
crate-type (bin)                     47
`;
    const uncovered = parseSccacheStatsOutput(sample, "C:\\src\\aphrody");
    expect(uncovered.compileRequestsExecuted).toBe(936);
    expect(uncovered.cacheHits).toBe(45);
    expect(uncovered.cacheHitRatePct).toBe(4.82);
    expect(uncovered.baseDirs).toEqual(["c:/labs/", "c:/aphrody/"]);
    expect(uncovered.nonCacheableReasons.incremental).toBe(520);
    expect(uncovered.repoCoveredByBaseDirs).toBe(false);

    const covered = parseSccacheStatsOutput(sample, "C:\\aphrody");
    expect(covered.repoCoveredByBaseDirs).toBe(true);
  });

  test("buildForkPlan honors Aphrody relative submodule conventions and upstream branches", () => {
    const fullPlan = buildForkPlan({ root: repoRoot });
    expect(fullPlan.orgConvention.appForkRepo).toBe("aphrody-dev/aphrody");
    expect(fullPlan.orgConvention.vendorForkOrg).toBe("aphrody-labs");
    expect(fullPlan.orgConvention.usesRelativeUrls).toBe(true);
    expect(fullPlan.targets.map((t) => t.id)).toEqual(["cargo", "clippy", "wild", "sccache"]);

    const cargoPlan = fullPlan.targets.find((t) => t.id === "cargo")!;
    expect(cargoPlan.upstreamDefaultBranch).toBe("master");
    expect(cargoPlan.relativeSubmoduleUrl).toBe("../../aphrody-labs/cargo.git");
    expect(cargoPlan.recommendedCherryPicks.some((p) => p.prOrIssue.includes("17518"))).toBe(true);

    const sccacheOnly = buildForkPlan({ root: repoRoot, targetFilter: "sccache" });
    expect(sccacheOnly.targets.length).toBe(1);
    expect(sccacheOnly.targets[0].recommendedCherryPicks.some((p) => p.prOrIssue.includes("3026"))).toBe(true);

    expect(() => buildForkPlan({ root: repoRoot, targetFilter: "nonexistent" })).toThrow();
  });

  test("runDoctor and CLI subcommands emit valid JSON cleanly without workspace build locks", () => {
    const report = runDoctor(repoRoot);
    expect(report.rustToolchainToml.channel).toBe("1.100.0");
    expect(report.tools.some((t) => t.name === "rustc" && t.installed)).toBe(true);
    expect(report.tools.some((t) => t.name === "cargo" && t.installed)).toBe(true);
    expect(report.findings.length).toBeGreaterThan(0);

    for (const args of [["profile-diff"], ["fork-plan", "--target", "wild"]]) {
      const run = Bun.spawnSync(["bun", "tooling/bun/toolchain-audit.ts", ...args], {
        cwd: repoRoot,
        stdout: "pipe",
        stderr: "pipe",
      });
      expect(run.exitCode).toBe(0);
      const parsed = JSON.parse(new TextDecoder().decode(run.stdout));
      expect(parsed).toBeTruthy();
    }
  }, 20_000);
});
