#!/usr/bin/env bun
/**
 * Reproducible Rust toolchain, Cargo profile, linker, cache, and fork-plan auditor
 * tailored for the Aphrody / GPUI / Zed monorepo.
 *
 * Read-only diagnostics: never triggers `cargo build`, `cargo check`, or `cargo test`
 * so parallel builds never contend on Cargo workspace locks.
 *
 * Usage:
 *   bun tooling/bun/toolchain-audit.ts doctor [--strict]
 *   bun tooling/bun/toolchain-audit.ts profile-diff
 *   bun tooling/bun/toolchain-audit.ts fork-plan [--target all|cargo|clippy|wild|sccache]
 */
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { parseGitmodules } from "./submodules.ts";

export const repoRoot = resolve(import.meta.dir, "../..");

export interface ToolchainSpec {
  channel: string;
  profile: string;
  components: string[];
  targets: string[];
}

export interface CargoConfigSpec {
  rustcWrapper: string | null;
  buildDir: string | null;
  targetDir: string | null;
  buildRustflags: string[];
  autoCleanFrequency: string | null;
  featureUnification: string | null;
  unstable: Record<string, unknown>;
  aliases: Record<string, string | string[]>;
  env: Record<string, string>;
  targetRustflags: Record<string, string[]>;
  targetLinkers: Record<string, string>;
}

export interface ProfileSetting {
  inherits?: string;
  optLevel?: string | number;
  debug?: string | number | boolean;
  splitDebuginfo?: string;
  incremental?: boolean;
  codegenUnits?: number;
  lto?: string | boolean;
  strip?: string | boolean;
  panic?: string;
}

export interface OptimizationFinding {
  id: string;
  severity: "high" | "medium" | "low";
  scope: "Cargo.toml" | ".cargo/config.toml" | ".cargo/bundle-config.toml" | "rust-toolchain.toml" | "environment" | "lints";
  title: string;
  current: string;
  recommended: string;
  rationale: string;
}

export interface ProfileAnalysisReport {
  workspaceMembers: number;
  excludedCount: number;
  defaultMembers: number;
  profiles: Record<string, ProfileSetting>;
  devPackageOptLevel3Count: number;
  devPackageCodegen1Count: number;
  devPackageOverrides: string[];
  missingHeavyDevOverrides: string[];
  findings: OptimizationFinding[];
}

export interface LockDuplicateStats {
  totalEntries: number;
  uniqueCrates: number;
  duplicatedNames: number;
  topDuplicates: Array<{ name: string; versions: string[] }>;
  presentCrates: Record<string, string[]>;
}

export interface NshLintBoundaryItem {
  crateName: string;
  manifestPath: string;
  inheritsWorkspaceLints: boolean;
  dependsOnSmol: boolean;
  stdProcessCommandFiles: Array<{ file: string; line: number; text: string }>;
}

export interface WorkspaceHotspotReport {
  memberBreakdown: {
    total: number;
    crates: number;
    extensions: number;
    tooling: number;
    excluded: number;
    defaultMembers: number;
  };
  lockStats: LockDuplicateStats | null;
  featureUnificationHotspots: Array<{ crateName: string; variants: string[]; impact: string }>;
  nshLintBoundary: {
    clippyDisallowsStdProcessCommand: boolean;
    affectedCrates: NshLintBoundaryItem[];
    totalStdCommandSites: number;
  };
}

export interface ToolProbeResult {
  name: string;
  installed: boolean;
  path: string | null;
  version: string | null;
  latestCratesIo?: string;
  role: string;
}

export interface SccacheStatsSummary {
  available: boolean;
  version: string | null;
  compileRequestsExecuted: number | null;
  cacheHits: number | null;
  cacheHitRatePct: number | null;
  cacheLocation: string | null;
  baseDirs: string[];
  nonCacheableReasons: Record<string, number>;
  repoCoveredByBaseDirs: boolean;
}

export interface ForkTargetPlan {
  id: "cargo" | "clippy" | "wild" | "sccache";
  upstreamRepo: string;
  upstreamDefaultBranch: string;
  forkRepo: string;
  submodulePath: string;
  relativeSubmoduleUrl: string;
  buildTargetBinaries: string[];
  recommendedCherryPicks: Array<{ prOrIssue: string; status: string; summary: string }>;
  patchTableSnippet: string;
  buildCommand: string;
  verificationGates: string[];
}

export interface ForkPlanReport {
  orgConvention: {
    appForkRepo: string;
    upstreamAppRepo: string;
    vendorForkOrg: string;
    declaredSubmodules: number;
    usesRelativeUrls: boolean;
    syncWorkflow: string;
  };
  targets: ForkTargetPlan[];
}

export const RECOMMENDED_CRATE_VERSIONS: Record<string, string> = {
  "cargo-hakari": "0.9.39",
  "cargo-nextest": "0.9.152",
  "cargo-sweep": "0.8.0",
  "cargo-cache": "0.8.3",
  sccache: "0.19.0",
  "cargo-deny": "0.21.0",
  "cargo-machete": "0.10.1",
  "cargo-udeps": "0.1.61",
  "wild-linker": "0.11.0",
  "cargo-shear": "1.14.1",
  "cargo-dylint": "6.4.0",
};

export const HEAVY_DEV_OPT3_CANDIDATES = [
  "serde_derive",
  "thiserror-impl",
  "tokio-macros",
  "strum_macros",
  "clap_derive",
  "schemars_derive",
  "derive_more-impl",
  "zerocopy-derive",
  "async-trait",
  "pin-project-internal",
  "bytemuck_derive",
  "wasm-bindgen-macro",
  "naga",
  "wgpu-core",
  "flate2",
  "zstd-sys",
  "ring",
  "aws-lc-rs",
  "image",
  "pulldown-cmark",
] as const;

export const ENV_ALLOWLIST = [
  "RUSTC_WRAPPER",
  "RUSTC_BOOTSTRAP",
  "CARGO_HOME",
  "RUSTUP_HOME",
  "CARGO_TARGET_DIR",
  "CARGO_BUILD_BUILD_DIR",
  "CARGO_INCREMENTAL",
  "SCCACHE_BASEDIRS",
  "SCCACHE_CACHE_SIZE",
  "SCCACHE_ALLOW_INCREMENTAL",
  "SCCACHE_DIR",
  "SCCACHE_SERVER_PORT",
] as const;

export function parseToolchainToml(content: string): ToolchainSpec {
  const parsed = Bun.TOML.parse(content) as {
    toolchain?: {
      channel?: string;
      profile?: string;
      components?: unknown;
      targets?: unknown;
    };
  };
  const tc = parsed.toolchain ?? {};
  return {
    channel: typeof tc.channel === "string" ? tc.channel : "unknown",
    profile: typeof tc.profile === "string" ? tc.profile : "default",
    components: Array.isArray(tc.components) ? tc.components.map(String) : [],
    targets: Array.isArray(tc.targets) ? tc.targets.map(String) : [],
  };
}

export function parseCargoConfigToml(content: string): CargoConfigSpec {
  const parsed = Bun.TOML.parse(content) as Record<string, any>;
  const build = (parsed.build ?? {}) as Record<string, unknown>;
  const cache = (parsed.cache ?? {}) as Record<string, unknown>;
  const resolver = (parsed.resolver ?? {}) as Record<string, unknown>;
  const target = (parsed.target ?? {}) as Record<string, Record<string, unknown>>;
  const targetRustflags: Record<string, string[]> = {};
  const targetLinkers: Record<string, string> = {};

  for (const [tripleOrCfg, cfg] of Object.entries(target)) {
    if (!cfg || typeof cfg !== "object") continue;
    if (Array.isArray(cfg.rustflags)) {
      targetRustflags[tripleOrCfg] = cfg.rustflags.map(String);
    } else if (typeof cfg.rustflags === "string") {
      targetRustflags[tripleOrCfg] = cfg.rustflags.split(/\s+/).filter(Boolean);
    }
    if (typeof cfg.linker === "string") {
      targetLinkers[tripleOrCfg] = cfg.linker;
    }
  }

  const buildRustflags = Array.isArray(build.rustflags)
    ? build.rustflags.map(String)
    : typeof build.rustflags === "string"
      ? build.rustflags.split(/\s+/).filter(Boolean)
      : [];

  return {
    rustcWrapper: typeof build["rustc-wrapper"] === "string" ? build["rustc-wrapper"] : null,
    buildDir: typeof build["build-dir"] === "string" ? build["build-dir"] : null,
    targetDir: typeof build["target-dir"] === "string" ? build["target-dir"] : null,
    buildRustflags,
    autoCleanFrequency: typeof cache["auto-clean-frequency"] === "string" ? cache["auto-clean-frequency"] : null,
    featureUnification: typeof resolver["feature-unification"] === "string" ? resolver["feature-unification"] : null,
    unstable: (parsed.unstable && typeof parsed.unstable === "object" ? parsed.unstable : {}) as Record<string, unknown>,
    aliases: (parsed.alias && typeof parsed.alias === "object" ? parsed.alias : {}) as Record<string, string | string[]>,
    env: Object.fromEntries(
      Object.entries((parsed.env ?? {}) as Record<string, unknown>).map(([k, v]) => [k, String(v)]),
    ),
    targetRustflags,
    targetLinkers,
  };
}

export function parseCargoLockDuplicates(lockContent: string): LockDuplicateStats {
  const pkgs: Array<{ name: string; version: string }> = [];
  for (const m of lockContent.matchAll(/\[\[package\]\]\r?\nname = "([^"]+)"\r?\nversion = "([^"]+)"/g)) {
    pkgs.push({ name: m[1], version: m[2] });
  }
  const counts = new Map<string, string[]>();
  for (const pkg of pkgs) {
    const list = counts.get(pkg.name) ?? [];
    list.push(pkg.version);
    counts.set(pkg.name, list);
  }
  const topDuplicates = [...counts.entries()]
    .filter(([, versions]) => versions.length > 1)
    .sort((a, b) => b[1].length - a[1].length || a[0].localeCompare(b[0]))
    .map(([name, versions]) => ({ name, versions }));

  const presentCrates: Record<string, string[]> = {};
  for (const [name, versions] of counts.entries()) {
    presentCrates[name] = versions;
  }

  return {
    totalEntries: pkgs.length,
    uniqueCrates: counts.size,
    duplicatedNames: topDuplicates.length,
    topDuplicates,
    presentCrates,
  };
}

function normalizeProfileEntry(raw: Record<string, any> | undefined): ProfileSetting {
  if (!raw) return {};
  return {
    inherits: typeof raw.inherits === "string" ? raw.inherits : undefined,
    optLevel: raw["opt-level"],
    debug: raw.debug,
    splitDebuginfo: typeof raw["split-debuginfo"] === "string" ? raw["split-debuginfo"] : undefined,
    incremental: typeof raw.incremental === "boolean" ? raw.incremental : undefined,
    codegenUnits: typeof raw["codegen-units"] === "number" ? raw["codegen-units"] : undefined,
    lto: raw.lto,
    strip: raw.strip,
    panic: typeof raw.panic === "string" ? raw.panic : undefined,
  };
}

function flattenRustflags(cfg: CargoConfigSpec | null): string[] {
  if (!cfg) return [];
  return [...cfg.buildRustflags, ...Object.values(cfg.targetRustflags).flat()];
}

export function analyzeProfiles(
  cargoTomlContent: string,
  cargoConfigContent?: string,
  lockStats?: LockDuplicateStats | null,
  bundleConfigContent?: string,
): ProfileAnalysisReport {
  const parsed = Bun.TOML.parse(cargoTomlContent) as Record<string, any>;
  const ws = (parsed.workspace ?? {}) as Record<string, unknown>;
  const members = Array.isArray(ws.members) ? ws.members.map(String) : [];
  const exclude = Array.isArray(ws.exclude) ? ws.exclude.map(String) : [];
  const defaultMembers = Array.isArray(ws["default-members"]) ? ws["default-members"].map(String) : [];

  const rawProfiles = (parsed.profile ?? {}) as Record<string, Record<string, any>>;
  const profiles: Record<string, ProfileSetting> = {};
  for (const name of ["dev", "dbg", "release", "release-fast", "dist", "web"]) {
    if (rawProfiles[name]) {
      profiles[name] = normalizeProfileEntry(rawProfiles[name]);
    }
  }
  if (rawProfiles.dev?.["build-override"]) {
    profiles["dev.build-override"] = normalizeProfileEntry(rawProfiles.dev["build-override"]);
  }

  const devPkgMap = (rawProfiles.dev?.package ?? {}) as Record<string, Record<string, any>>;
  const devPackageOverrides = Object.keys(devPkgMap);
  let devPackageOptLevel3Count = 0;
  let devPackageCodegen1Count = 0;
  for (const entry of Object.values(devPkgMap)) {
    if (entry?.["opt-level"] === 3) devPackageOptLevel3Count++;
    if (entry?.["codegen-units"] === 1) devPackageCodegen1Count++;
  }

  const missingHeavyDevOverrides = HEAVY_DEV_OPT3_CANDIDATES.filter((crateName) => {
    if (devPkgMap[crateName]) return false;
    if (!lockStats) return true;
    return Boolean(lockStats.presentCrates[crateName]);
  });

  const cfg = cargoConfigContent ? parseCargoConfigToml(cargoConfigContent) : null;
  const bundleCfg = bundleConfigContent ? parseCargoConfigToml(bundleConfigContent) : null;
  const allFlags = flattenRustflags(cfg);
  const allFlagStr = allFlags.join(" ");
  const findings: OptimizationFinding[] = [];

  const devDebug = profiles.dev?.debug;
  if (devDebug !== "line-tables-only" && devDebug !== false && devDebug !== 0 && devDebug !== "none") {
    findings.push({
      id: "profile.dev.debug-line-tables",
      severity: "high",
      scope: "Cargo.toml",
      title: "Switch [profile.dev] debug from \"limited\" to \"line-tables-only\"",
      current: `debug = ${JSON.stringify(devDebug ?? "full")}`,
      recommended: 'debug = "line-tables-only"',
      rationale:
        'Cargo PR #17518 (merged 2026-11-14) switches default dev debuginfo to "line-tables-only" (MSRV 1.71+), shrinking build/target artifacts and speeding up codegen & linking while keeping panic backtraces.',
    });
  }

  if (!devPkgMap["*"] || devPkgMap["*"].debug === undefined) {
    findings.push({
      id: "profile.dev.deps-no-debug",
      severity: "high",
      scope: "Cargo.toml",
      title: "Disable debuginfo for third-party dependencies in dev builds",
      current: '"*" = { debug = false } missing under [profile.dev.package]',
      recommended: '[profile.dev.package]\n"*" = { debug = false }',
      rationale:
        "Recommended by Cargo's Optimizing Build Performance guide: avoids generating DWARF/PDB payloads for ~1,800 upstream crates while keeping [profile.dbg] at debug = \"full\" when attaching a debugger.",
    });
  }

  const buildOverrideDebug = profiles["dev.build-override"]?.debug;
  if (buildOverrideDebug !== false && buildOverrideDebug !== 0 && buildOverrideDebug !== "none" && buildOverrideDebug !== "line-tables-only") {
    findings.push({
      id: "profile.dev.build-override-debug",
      severity: "medium",
      scope: "Cargo.toml",
      title: "Trim [profile.dev.build-override] debuginfo and optimize proc-macros/build scripts",
      current: `debug = ${JSON.stringify(buildOverrideDebug ?? "default")}`,
      recommended: 'debug = false (or "line-tables-only") + opt-level = 3 for heavy proc-macro deps',
      rationale:
        "Build scripts and proc-macros rarely require module-level debug tables; keeping build-override opt-level aligned avoids double compilation while reducing PDB/DWARF writes.",
    });
  }

  if (missingHeavyDevOverrides.length > 0) {
    findings.push({
      id: "profile.dev.heavy-proc-macros-opt3",
      severity: "medium",
      scope: "Cargo.toml",
      title: "Add opt-level = 3 dev overrides for unoptimized proc-macro and shader/compression crates",
      current: `${devPackageOptLevel3Count} crates pinned at opt-level = 3; ${missingHeavyDevOverrides.length} heavy lockfile crates unpinned`,
      recommended: `${missingHeavyDevOverrides.slice(0, 8).join(", ")} (+${Math.max(0, missingHeavyDevOverrides.length - 8)} more) = { opt-level = 3 }`,
      rationale:
        "Optimizing derive proc-macros (`serde_derive`, `thiserror-impl`, `tokio-macros`, `zerocopy-derive`, `wasm-bindgen-macro`) and shader/compression engines (`naga`, `wgpu-core`, `zstd-sys`, `ring`, `image`) speeds macro expansion, tests, and asset pipelines.",
    });
  }

  if (cfg) {
    const winFlags = (cfg.targetRustflags['cfg(target_os = "windows")'] ?? []).concat(
      cfg.targetRustflags["x86_64-pc-windows-msvc"] ?? [],
    );
    const hasWinLld =
      Boolean(cfg.targetLinkers["x86_64-pc-windows-msvc"]?.includes("lld")) ||
      Boolean(cfg.targetLinkers['cfg(target_os = "windows")']?.includes("lld")) ||
      winFlags.some((f) => f.includes("lld") || f.includes("linker-features=+lld"));
    if (!hasWinLld) {
      findings.push({
        id: "config.windows.lld-linker",
        severity: "high",
        scope: ".cargo/config.toml",
        title: "Enable LLD linker on x86_64-pc-windows-msvc",
        current: "Uses MSVC link.exe by default",
        recommended: '[target.x86_64-pc-windows-msvc] linker = "lld-link" (or rustflags = ["-C", "linker-features=+lld"])',
        rationale:
          "LLD 23.1.5 and bundled rust-lld.exe are installed locally and link large GPUI/Zed PDB binaries substantially faster than MSVC link.exe.",
      });
    }

    const linuxX64Flags = (cfg.targetRustflags["x86_64-unknown-linux-gnu"] ?? []).concat(
      cfg.targetRustflags['cfg(target_os = "linux")'] ?? [],
    );
    const hasFastLinuxX64Linker = linuxX64Flags.some((f) => /fuse-ld=(mold|wild|lld)/.test(f));
    if (!hasFastLinuxX64Linker) {
      findings.push({
        id: "config.linux.fast-linker",
        severity: "high",
        scope: ".cargo/config.toml",
        title: "Enable mold/wild/lld linker on x86_64-unknown-linux-gnu",
        current: "Only aarch64-unknown-linux-gnu specifies -fuse-ld=lld; x86_64 falls back to GNU ld",
        recommended: '[target.x86_64-unknown-linux-gnu] rustflags = ["-C", "link-arg=-fuse-ld=mold"] (or wild / lld)',
        rationale:
          "GNU ld is single-threaded and dominates incremental relinks of `zed` and `aphrody_desktop` on Linux hosts (`fr1`/`fr2`).",
      });
    }

    if (!cfg.buildDir) {
      findings.push({
        id: "config.build-dir-separation",
        severity: "medium",
        scope: ".cargo/config.toml",
        title: "Configure [build] build-dir separation for intermediate artifacts",
        current: "build.build-dir unset (intermediates mixed in target-dir)",
        recommended: 'build-dir = "{workspace-root}/target/build" (or CARGO_BUILD_BUILD_DIR on tmpfs/Dev Drive)',
        rationale:
          "Cargo's stabilized build-dir template separates heavy intermediate crates from final output directories and prepares for cross-workspace artifact deduplication (rust-lang/cargo#17512).",
      });
    }

    if (!cfg.autoCleanFrequency) {
      findings.push({
        id: "config.cache-auto-clean",
        severity: "low",
        scope: ".cargo/config.toml",
        title: "Set explicit [cache] auto-clean-frequency",
        current: "Not explicitly declared in .cargo/config.toml",
        recommended: '[cache]\nauto-clean-frequency = "1 day"',
        rationale:
          "Prevents unbounded ~/.cargo registry/git cache growth on CI/fleet hosts (`fr1`/`fr2`) alongside `cargo-sweep` and `-Zgc`.",
      });
    }

    if (!cfg.featureUnification) {
      findings.push({
        id: "config.feature-unification",
        severity: "high",
        scope: ".cargo/config.toml",
        title: "Unify workspace features via cargo-hakari or [resolver] feature-unification = \"workspace\"",
        current: `No workspace-hack crate and no [resolver] feature-unification across ${members.length} workspace members`,
        recommended: "Generate `crates/workspace-hack` via `cargo hakari` (or enable `[resolver] feature-unification = \"workspace\"` with RUSTC_BOOTSTRAP=1)",
        rationale:
          "Without unified features, switching between `-p gpui_ui`, `-p aphrody_desktop`, `-p nsh-cli`, and `-p zed` re-compiles shared crates (`tokio`, `syn`, `taffy`, `wgpu`, `windows`) under differing feature bitsets.",
      });
    }

    if (!allFlagStr.includes("-Zthreads") && !allFlagStr.includes("--jobs-frontend")) {
      findings.push({
        id: "config.parallel-frontend",
        severity: "medium",
        scope: ".cargo/config.toml",
        title: "Document version-aware parallel frontend flag (-Zthreads=8 vs --jobs-frontend=8)",
        current: "Not set in repo .cargo/config.toml (user global ~/.cargo/config.toml passes deprecated -Zthreads=8)",
        recommended: "Use `-Zthreads=8` on rustc 1.98.1 (with RUSTC_BOOTSTRAP=1) and `--jobs-frontend=8` on rustc >= 1.100.0 / nightly",
        rationale:
          "Live probe confirmed rustc 1.98.1 accepts `-Zthreads=8`, while rustc 1.100.0+ and 1.102.0-nightly deprecate `-Zthreads` in favor of `--jobs-frontend`.",
      });
    }
  }

  const bundleFlags = flattenRustflags(bundleCfg);
  if (bundleConfigContent !== undefined && !bundleFlags.some((f) => f.includes("share-generics"))) {
    findings.push({
      id: "bundle.share-generics",
      severity: "medium",
      scope: ".cargo/bundle-config.toml",
      title: "Restore -Zshare-generics=y in .cargo/bundle-config.toml",
      current: "share-generics missing from .cargo/bundle-config.toml",
      recommended: '[env]\nRUSTC_BOOTSTRAP = "1"\n[build]\nrustflags = ["-Z", "share-generics=y"]',
      rationale:
        "Upstream zed-industries/zed#63432 cut release binary size by 10.8% (-45.6 MB) and drop_glue instantiations by 36% (101,778 -> 65,041).",
    });
  }

  return {
    workspaceMembers: members.length,
    excludedCount: exclude.length,
    defaultMembers: defaultMembers.length,
    profiles,
    devPackageOptLevel3Count,
    devPackageCodegen1Count,
    devPackageOverrides,
    missingHeavyDevOverrides,
    findings,
  };
}

export function scanRustFilesForPattern(dir: string, pattern: RegExp): Array<{ file: string; line: number; text: string }> {
  const matches: Array<{ file: string; line: number; text: string }> = [];
  if (!existsSync(dir)) return matches;
  const stack = [dir];
  while (stack.length > 0) {
    const current = stack.pop()!;
    for (const entry of readdirSync(current, { withFileTypes: true })) {
      const full = join(current, entry.name);
      if (entry.isDirectory()) {
        if (entry.name === "target") continue;
        stack.push(full);
      } else if (entry.isFile() && entry.name.endsWith(".rs")) {
        const lines = readFileSync(full, "utf8").split(/\r?\n/);
        for (let i = 0; i < lines.length; i++) {
          if (pattern.test(lines[i])) {
            matches.push({ file: full.replaceAll("\\", "/"), line: i + 1, text: lines[i].trim() });
          }
        }
      }
    }
  }
  return matches;
}

export function auditWorkspaceHotspots(root = repoRoot): WorkspaceHotspotReport {
  const cargoToml = readFileSync(join(root, "Cargo.toml"), "utf8");
  const parsed = Bun.TOML.parse(cargoToml) as Record<string, any>;
  const ws = (parsed.workspace ?? {}) as Record<string, unknown>;
  const members = Array.isArray(ws.members) ? ws.members.map(String) : [];
  const exclude = Array.isArray(ws.exclude) ? ws.exclude.map(String) : [];
  const defaultMembers = Array.isArray(ws["default-members"]) ? ws["default-members"].map(String) : [];

  const lockPath = join(root, "Cargo.lock");
  const lockStats = existsSync(lockPath) ? parseCargoLockDuplicates(readFileSync(lockPath, "utf8")) : null;

  const clippyTomlPath = join(root, "clippy.toml");
  const clippyToml = existsSync(clippyTomlPath) ? readFileSync(clippyTomlPath, "utf8") : "";
  const clippyDisallowsStdProcessCommand = clippyToml.includes("std::process::Command::");

  const affectedCrates: NshLintBoundaryItem[] = [];
  let totalStdCommandSites = 0;
  const cratesDir = join(root, "crates");
  if (existsSync(cratesDir)) {
    for (const entry of readdirSync(cratesDir, { withFileTypes: true })) {
      if (!entry.isDirectory() || !entry.name.startsWith("nsh-")) continue;
      const crateDir = join(cratesDir, entry.name);
      const manifestPath = join(crateDir, "Cargo.toml");
      if (!existsSync(manifestPath)) continue;
      const manifest = readFileSync(manifestPath, "utf8");
      const inheritsWorkspaceLints = /\[lints\]\s*\r?\n\s*workspace\s*=\s*true/.test(manifest);
      const dependsOnSmol = /\bsmol\b/.test(manifest);
      const hits = scanRustFilesForPattern(join(crateDir, "src"), /\bstd::process::Command::/);
      totalStdCommandSites += hits.length;
      if (hits.length > 0 || (inheritsWorkspaceLints && !dependsOnSmol)) {
        if (hits.length > 0) {
          affectedCrates.push({
            crateName: entry.name,
            manifestPath: manifestPath.replaceAll("\\", "/"),
            inheritsWorkspaceLints,
            dependsOnSmol,
            stdProcessCommandFiles: hits.map((h) => ({
              ...h,
              file: h.file.startsWith(root.replaceAll("\\", "/"))
                ? h.file.slice(root.replaceAll("\\", "/").length + 1)
                : h.file,
            })),
          });
        }
      }
    }
  }

  const featureUnificationHotspots: Array<{ crateName: string; variants: string[]; impact: string }> = [
    {
      crateName: "nsh-core",
      variants: ["default=[] (apps/gpui_ui)", "features=[declarations,websocket,updater] (nsh-cli)", "features=[shader] (dev-deps)"],
      impact: "Rebuilds nsh-core + optional naga/resvg/rustls trees when switching between aphrody_desktop, gpui_ui, and nsh-cli",
    },
    {
      crateName: "gpui",
      variants: ["default=[font-kit,wayland,x11,windows-manifest]", "test-support", "profiler/bench-support"],
      impact: "test-support drags proptest, postage, futures-concurrency across editor/workspace/zed test targets",
    },
    {
      crateName: "tokio",
      variants: ["rt + time (nsh-core)", "rt-multi-thread + net + io-util + sync + macros", "full (workspace/zed)"],
      impact: "Without hakari or workspace feature-unification, tokio feature flags flip across Tier 1-4 checks",
    },
    {
      crateName: "windows / windows-sys",
      variants: lockStats?.presentCrates["windows"] ?? ["0.56.0", "0.57.0", "0.58.0", "0.61.3", "0.62.2"],
      impact: "5 concurrent major lines + dynamic per-crate Win32 feature unions re-emit massive metadata",
    },
  ];

  return {
    memberBreakdown: {
      total: members.length,
      crates: members.filter((m) => m.startsWith("crates/")).length,
      extensions: members.filter((m) => m.startsWith("extensions/")).length,
      tooling: members.filter((m) => m.startsWith("tooling/")).length,
      excluded: exclude.length,
      defaultMembers: defaultMembers.length,
    },
    lockStats,
    featureUnificationHotspots,
    nshLintBoundary: {
      clippyDisallowsStdProcessCommand,
      affectedCrates,
      totalStdCommandSites,
    },
  };
}

export function parseSccacheStatsOutput(raw: string, targetRepoRoot = repoRoot): SccacheStatsSummary {
  const getNum = (label: string): number | null => {
    const m = raw.match(new RegExp(`^${label}\\s+(\\d+)`, "m"));
    return m ? Number(m[1]) : null;
  };
  const rateMatch = raw.match(/^Cache hits rate\s+([\d.]+)\s*%/m);
  const locMatch = raw.match(/^Cache location\s+(.+)$/m);
  const baseMatch = raw.match(/^Base directories\s+(.+)$/m);
  const baseDirs = baseMatch
    ? baseMatch[1]
        .split(",")
        .map((s) => s.trim())
        .filter(Boolean)
    : [];

  const nonCacheableReasons: Record<string, number> = {};
  const reasonsIdx = raw.indexOf("Non-cacheable reasons:");
  if (reasonsIdx >= 0) {
    const tail = raw.slice(reasonsIdx).split(/\r?\n/).slice(1);
    for (const line of tail) {
      if (!line.trim()) break;
      const rm = line.match(/^(.+?)\s{2,}(\d+)\s*$/);
      if (rm) nonCacheableReasons[rm[1].trim()] = Number(rm[2]);
    }
  }

  const normRepo = targetRepoRoot.replaceAll("\\", "/").toLowerCase().replace(/\/+$/, "");
  const repoCoveredByBaseDirs = baseDirs.some((bd) => {
    const normBd = bd.replaceAll("\\", "/").toLowerCase().replace(/\/+$/, "");
    return normRepo === normBd || normRepo.startsWith(`${normBd}/`);
  });

  return {
    available: true,
    version: null,
    compileRequestsExecuted: getNum("Compile requests executed"),
    cacheHits: getNum("Cache hits"),
    cacheHitRatePct: rateMatch ? Number(rateMatch[1]) : null,
    cacheLocation: locMatch ? locMatch[1].trim() : null,
    baseDirs,
    nonCacheableReasons,
    repoCoveredByBaseDirs,
  };
}

export function buildForkPlan(options?: {
  root?: string;
  gitmodulesContent?: string;
  targetFilter?: string;
}): ForkPlanReport {
  const root = options?.root ?? repoRoot;
  const gitmodulesText =
    options?.gitmodulesContent ??
    (existsSync(join(root, ".gitmodules")) ? readFileSync(join(root, ".gitmodules"), "utf8") : "");
  const submodules = gitmodulesText ? parseGitmodules(gitmodulesText) : [];
  const usesRelativeUrls =
    submodules.length > 0
      ? submodules.every((s) => s.url.startsWith("../"))
      : existsSync(join(root, "tooling/bun/forks-manifest.json"));

  const allTargets: ForkTargetPlan[] = [
    {
      id: "cargo",
      upstreamRepo: "rust-lang/cargo",
      upstreamDefaultBranch: "master",
      forkRepo: "aphrody-labs/cargo",
      submodulePath: "vendor/toolchain/cargo",
      relativeSubmoduleUrl: "../../aphrody-labs/cargo.git",
      buildTargetBinaries: ["cargo"],
      recommendedCherryPicks: [
        {
          prOrIssue: "rust-lang/cargo#17518",
          status: "merged 2026-11-14",
          summary: "Switch default [profile.dev] debuginfo to line-tables-only and add built-in [profile.debug].",
        },
        {
          prOrIssue: "rust-lang/cargo#17512",
          status: "open PR (build-dir shared storage)",
          summary: "Cross-workspace shared storage blobs & unit-output deduplication under $CARGO_HOME/shared-storage.",
        },
        {
          prOrIssue: "rust-lang/cargo#17524",
          status: "open PR (FCP)",
          summary: "Track workspace target-dir & build-dir in global cache DB and clean stale dirs via `cargo clean gc --max-target-age`.",
        },
        {
          prOrIssue: "rust-lang/cargo#16659",
          status: "merged 2026-02-21",
          summary: "Fix parallel locking contention when -Zfine-grain-locking is enabled.",
        },
      ],
      patchTableSnippet: `# Used only when workspace crates depend on cargo / cargo-util crates:\n# cargo = { path = "vendor/toolchain/cargo" }`,
      buildCommand: "cargo build --manifest-path vendor/toolchain/cargo/Cargo.toml --release --bin cargo",
      verificationGates: [
        "bun run hygiene",
        "bun run test:tooling",
        "cargo xtask brand check",
        "cargo perf-test -p gpui -- --quiet",
      ],
    },
    {
      id: "clippy",
      upstreamRepo: "rust-lang/rust-clippy",
      upstreamDefaultBranch: "main",
      forkRepo: "aphrody-labs/rust-clippy",
      submodulePath: "vendor/toolchain/rust-clippy",
      relativeSubmoduleUrl: "../../aphrody-labs/rust-clippy.git",
      buildTargetBinaries: ["clippy-driver", "cargo-clippy"],
      recommendedCherryPicks: [
        {
          prOrIssue: "tooling/lints/Cargo.toml clippy_utils pin",
          status: "repo boundary",
          summary: "Align `clippy_utils` rev with custom Aphrody dylint rules (`tooling/lints`) and nsh-core lint exemptions.",
        },
      ],
      patchTableSnippet: `# In tooling/lints/Cargo.toml (outside root workspace):\nclippy_utils = { path = "../../vendor/toolchain/rust-clippy/clippy_utils" }`,
      buildCommand: "cargo +nightly build --manifest-path vendor/toolchain/rust-clippy/Cargo.toml --release",
      verificationGates: [
        "cargo test --manifest-path tooling/lints/Cargo.toml",
        "cargo xtask clippy -p gpui",
        "bun run hygiene",
      ],
    },
    {
      id: "wild",
      upstreamRepo: "wild-linker/wild",
      upstreamDefaultBranch: "main",
      forkRepo: "aphrody-labs/wild",
      submodulePath: "vendor/toolchain/wild",
      relativeSubmoduleUrl: "../../aphrody-labs/wild.git",
      buildTargetBinaries: ["wild"],
      recommendedCherryPicks: [
        {
          prOrIssue: "wild-linker/wild#2975 & #2979",
          status: "merged 2026-11-15",
          summary: "COFF `.CRT$` section merging & CI cross-tests advancing Windows PE/COFF parity alongside Linux ELF / wasm32.",
        },
        {
          prOrIssue: "wild-linker/wild#2980 & #2977",
          status: "merged 2026-11-15",
          summary: "Wasm `--soname` and `--import-table` support for WebGPU/WASI linking experiments.",
        },
      ],
      patchTableSnippet: `# Binary-only tool (excluded from root workspace via [workspace.exclude]):\n# Build on fr1 / Linux & drop ` + "`wild`" + ` in ~/.cargo/bin for -C link-arg=--ld-path=wild`,
      buildCommand: "cargo build --manifest-path vendor/toolchain/wild/Cargo.toml --release --bin wild",
      verificationGates: [
        "cargo test --manifest-path vendor/toolchain/wild/Cargo.toml",
        "hyperfine link time benchmark on `zed` / `aphrody_desktop` (fr1)",
      ],
    },
    {
      id: "sccache",
      upstreamRepo: "mozilla/sccache",
      upstreamDefaultBranch: "main",
      forkRepo: "aphrody-labs/sccache",
      submodulePath: "vendor/toolchain/sccache",
      relativeSubmoduleUrl: "../../aphrody-labs/sccache.git",
      buildTargetBinaries: ["sccache"],
      recommendedCherryPicks: [
        {
          prOrIssue: "mozilla/sccache#3026",
          status: "merged 2026-11-14 (post-0.19.0)",
          summary: "Spawns long rustc command lines through @response files, fixing Windows CreateProcessW 32 KiB os error 206 on `web-sys`.",
        },
        {
          prOrIssue: "mozilla/sccache#3022",
          status: "merged 2026-11-14 (post-0.19.0)",
          summary: "Adds `SCCACHE_BASEDIRS_VERBATIM_ENV_DEPS=CARGO_MANIFEST_DIR,CARGO_MANIFEST_PATH` so multi-worktree builds share deps safely.",
        },
      ],
      patchTableSnippet: `# Standalone binary wrapper (excluded from root workspace via [workspace.exclude]):\n# Eliminates Windows RUSTC_WRAPPER="" workaround once #3026 build is deployed`,
      buildCommand: "cargo build --manifest-path vendor/toolchain/sccache/Cargo.toml --release --bin sccache",
      verificationGates: [
        "bun run web:check",
        "sccache --show-stats (verify hit rate & zero os error 206)",
      ],
    },
  ];

  const filter = options?.targetFilter && options.targetFilter !== "all" ? options.targetFilter : null;
  const targets = filter ? allTargets.filter((t) => t.id === filter) : allTargets;
  if (filter && targets.length === 0) {
    throw new Error(`Unknown fork target '${filter}'. Expected one of: all, cargo, clippy, wild, sccache`);
  }

  return {
    orgConvention: {
      appForkRepo: "aphrody-dev/aphrody",
      upstreamAppRepo: "zed-industries/zed",
      vendorForkOrg: "aphrody-labs",
      declaredSubmodules: submodules.length,
      usesRelativeUrls,
      syncWorkflow: ".github/workflows/aphrody-upstream-sync.yml",
    },
    targets,
  };
}

function probeBinary(bin: string, versionArg = "--version"): ToolProbeResult {
  const latestCratesIo = RECOMMENDED_CRATE_VERSIONS[bin] ?? RECOMMENDED_CRATE_VERSIONS[`${bin}-linker`];
  let binPath: string | null = null;
  try {
    const whichCmd = process.platform === "win32" ? ["where.exe", bin] : ["which", bin];
    const loc = Bun.spawnSync(whichCmd, { stdout: "pipe", stderr: "pipe" });
    binPath =
      loc.exitCode === 0
        ? new TextDecoder().decode(loc.stdout).split(/\r?\n/).map((s) => s.trim()).find(Boolean) ?? null
        : null;
  } catch {
    binPath = null;
  }

  if (!binPath) {
    return {
      name: bin,
      installed: false,
      path: null,
      version: null,
      latestCratesIo,
      role: toolRole(bin),
    };
  }

  try {
    const res = Bun.spawnSync([binPath, versionArg], { stdout: "pipe", stderr: "pipe" });
    const out = new TextDecoder().decode(res.stdout).trim() || new TextDecoder().decode(res.stderr).trim();
    const version = out.split(/\r?\n/)[0]?.trim() || "present";
    return {
      name: bin,
      installed: res.exitCode === 0 || Boolean(out),
      path: binPath,
      version,
      latestCratesIo,
      role: toolRole(bin),
    };
  } catch {
    return {
      name: bin,
      installed: true,
      path: binPath,
      version: "present",
      latestCratesIo,
      role: toolRole(bin),
    };
  }
}

function toolRole(bin: string): string {
  const roles: Record<string, string> = {
    rustc: "Rust compiler",
    cargo: "Cargo package & workspace driver",
    "clippy-driver": "Rust linter driver",
    rustfmt: "Canonical code formatter",
    "lld-link": "LLVM COFF linker (Windows)",
    "rust-lld": "Bundled Rust LLD linker",
    clang: "LLVM C/C++ compiler & linker driver",
    mold: "Fast Unix ELF linker",
    wild: "Ultra-fast incremental-oriented linker (Linux/Wasm)",
    sccache: "Shared local & R2/S3 Rust/C++ compilation cache",
    "cargo-nextest": "Fast parallel Rust test runner",
    "cargo-hakari": "Workspace-hack generator unifying feature sets",
    "cargo-sweep": "Prunes stale Cargo target artifacts by age/toolchain",
    "cargo-cache": "Inspects & cleans ~/.cargo registry/git caches",
    "cargo-deny": "License, advisory, duplicate & source policy gate",
    "cargo-machete": "Fast regex/heuristic unused dependency detector",
    "cargo-udeps": "Nightly compiler-accurate unused dep detector",
    "cargo-shear": "Workspace unused dependency auto-cleaner",
    "cargo-dylint": "Custom GPUI/Zed/Aphrody lint runner",
    git: "Version control & relative submodule engine",
  };
  return roles[bin] ?? "Toolchain component";
}

export function runDoctor(root = repoRoot) {
  const tcText = readFileSync(join(root, "rust-toolchain.toml"), "utf8");
  const cfgText = readFileSync(join(root, ".cargo/config.toml"), "utf8");
  const bundleCfgPath = join(root, ".cargo/bundle-config.toml");
  const bundleCfgText = existsSync(bundleCfgPath) ? readFileSync(bundleCfgPath, "utf8") : undefined;
  const cargoTomlText = readFileSync(join(root, "Cargo.toml"), "utf8");

  const toolchainToml = parseToolchainToml(tcText);
  const cargoConfig = parseCargoConfigToml(cfgText);
  const hotspots = auditWorkspaceHotspots(root);
  const profiles = analyzeProfiles(cargoTomlText, cfgText, hotspots.lockStats, bundleCfgText);

  const rustcVv = new TextDecoder()
    .decode(Bun.spawnSync(["rustc", "-Vv"], { cwd: root, stdout: "pipe", stderr: "pipe" }).stdout)
    .trim();
  const sysroot = new TextDecoder()
    .decode(Bun.spawnSync(["rustc", "--print", "sysroot"], { cwd: root, stdout: "pipe", stderr: "pipe" }).stdout)
    .trim();
  const hostLine = rustcVv.split(/\r?\n/).find((l) => l.startsWith("host:"))?.slice(5).trim() ?? "";
  const rustLldFile = process.platform === "win32" ? "rust-lld.exe" : "rust-lld";
  const bundledRustLldPath = hostLine && sysroot ? join(sysroot, "lib", "rustlib", hostLine, "bin", rustLldFile) : "";
  const hasBundledRustLld = Boolean(bundledRustLldPath && existsSync(bundledRustLldPath));

  const componentsRaw = new TextDecoder()
    .decode(Bun.spawnSync(["rustup", "component", "list", "--installed"], { cwd: root, stdout: "pipe", stderr: "pipe" }).stdout)
    .split(/\r?\n/)
    .map((s) => s.trim())
    .filter(Boolean);
  const nightlyCranelift = new TextDecoder()
    .decode(
      Bun.spawnSync(["rustup", "component", "list", "--installed", "--toolchain", "nightly"], {
        stdout: "pipe",
        stderr: "pipe",
      }).stdout,
    )
    .split(/\r?\n/)
    .some((c) => c.startsWith("rustc-codegen-cranelift"));

  const bins = [
    "rustc",
    "cargo",
    "clippy-driver",
    "rustfmt",
    "lld-link",
    "clang",
    "mold",
    "wild",
    "sccache",
    "cargo-nextest",
    "cargo-hakari",
    "cargo-sweep",
    "cargo-cache",
    "cargo-deny",
    "cargo-machete",
    "cargo-udeps",
    "cargo-shear",
    "cargo-dylint",
    "git",
  ];
  const tools = bins.map((b) => probeBinary(b));

  let sccacheStats: SccacheStatsSummary = {
    available: false,
    version: null,
    compileRequestsExecuted: null,
    cacheHits: null,
    cacheHitRatePct: null,
    cacheLocation: null,
    baseDirs: [],
    nonCacheableReasons: {},
    repoCoveredByBaseDirs: false,
  };
  try {
    const sccacheRes = Bun.spawnSync(["sccache", "--show-stats"], { stdout: "pipe", stderr: "pipe" });
    if (sccacheRes.exitCode === 0) {
      sccacheStats = {
        ...parseSccacheStatsOutput(new TextDecoder().decode(sccacheRes.stdout), root),
        version: tools.find((t) => t.name === "sccache")?.version ?? null,
      };
    }
  } catch {
    // sccache absent on PATH
  }

  const envSummary: Record<string, string | null> = {};
  for (const key of ENV_ALLOWLIST) {
    envSummary[key] = process.env[key] ?? null;
  }

  let windowsStorage: { devDriveDetected: boolean; volumes: string[] } | null = null;
  if (process.platform === "win32") {
    const volRes = Bun.spawnSync(
      [
        "powershell",
        "-NoProfile",
        "-Command",
        "Get-Volume | Where-Object DriveLetter | ForEach-Object { \"$($_.DriveLetter): $($_.FileSystemType) freeGB=$([math]::Round($_.SizeRemaining/1GB,1))\" }",
      ],
      { stdout: "pipe", stderr: "pipe" },
    );
    const vols = new TextDecoder()
      .decode(volRes.stdout)
      .split(/\r?\n/)
      .map((s) => s.trim())
      .filter(Boolean);
    windowsStorage = {
      devDriveDetected: vols.some((v) => v.includes("ReFS")),
      volumes: vols,
    };
  }

  const extraFindings: OptimizationFinding[] = [...profiles.findings];
  if (sccacheStats.available && !sccacheStats.repoCoveredByBaseDirs) {
    extraFindings.push({
      id: "env.sccache.basedir-missing-repo",
      severity: "high",
      scope: "environment",
      title: "SCCACHE_BASEDIRS does not cover the active repository checkout",
      current: `Base directories: ${sccacheStats.baseDirs.join(", ") || "<none>"}`,
      recommended: `Include ${root.replaceAll("\\", "/")} (or C:\\src) in SCCACHE_BASEDIRS and set SCCACHE_BASEDIRS_VERBATIM_ENV_DEPS=CARGO_MANIFEST_DIR,CARGO_MANIFEST_PATH`,
      rationale:
        "Without covering C:\\src in SCCACHE_BASEDIRS, cross-worktree cache hits fail (`sccache` measured hit rate is low and workspace incremental crates bypass cache).",
    });
  }
  if (windowsStorage && !windowsStorage.devDriveDetected) {
    extraFindings.push({
      id: "env.windows.dev-drive",
      severity: "low",
      scope: "environment",
      title: "Windows host uses NTFS rather than ReFS Dev Drive for Cargo target/build cache",
      current: windowsStorage.volumes.join("; "),
      recommended: "Create a ReFS Dev Drive (VHDX or partition) for CARGO_TARGET_DIR & CARGO_HOME with Microsoft Defender performance mode",
      rationale: "ReFS Dev Drive + copy-on-write accelerates Cargo's tens of thousands of tiny metadata/rlib writes by ~20-30% on Windows.",
    });
  }
  if (hotspots.nshLintBoundary.totalStdCommandSites > 0) {
    extraFindings.push({
      id: "lints.nsh-core-command-boundary",
      severity: "medium",
      scope: "lints",
      title: "clippy.toml disallowed_methods conflicts with std::process::Command inside nsh-core / nsh-cli",
      current: `${hotspots.nshLintBoundary.totalStdCommandSites} std::process::Command call site(s) across ${hotspots.nshLintBoundary.affectedCrates.map((c) => c.crateName).join(", ")} (all inherit [lints] workspace = true without depending on smol)`,
      recommended: "Annotate non-smol nsh test/CLI invocations with #[allow(clippy::disallowed_methods, reason = \"nsh uses tokio/std, not smol\")] or scope clippy.toml",
      rationale:
        "Workspace `cargo xtask clippy` runs `--release --all-targets --all-features -- --deny warnings`, failing on nsh crates where smol::process::Command is unavailable.",
    });
  }

  return {
    generatedAt: new Date().toISOString(),
    host: {
      platform: process.platform,
      arch: process.arch,
      rustcHost: hostLine,
      sysroot,
      bundledRustLldAvailable: hasBundledRustLld,
      bundledRustLldPath: hasBundledRustLld ? bundledRustLldPath : null,
      nightlyCraneliftInstalled: nightlyCranelift,
      windowsStorage,
    },
    rustToolchainToml: toolchainToml,
    activeComponents: componentsRaw,
    cargoConfig,
    tools,
    sccache: sccacheStats,
    env: envSummary,
    workspaceScale: hotspots.memberBreakdown,
    lockStats: hotspots.lockStats
      ? {
          totalEntries: hotspots.lockStats.totalEntries,
          uniqueCrates: hotspots.lockStats.uniqueCrates,
          duplicatedNames: hotspots.lockStats.duplicatedNames,
          top5Duplicates: hotspots.lockStats.topDuplicates.slice(0, 5),
        }
      : null,
    findings: extraFindings,
    status: extraFindings.some((f) => f.severity === "high") ? "needs-tuning" : "optimal",
  };
}

export function buildRecommendedPatches() {
  return {
    cargoConfigToml: `[build]
build-dir = "{workspace-root}/target/build"

[cache]
auto-clean-frequency = "1 day"

[resolver]
# Requires RUSTC_BOOTSTRAP = "1" on 1.98.1 (or use cargo-hakari workspace-hack on stable without -Z)
feature-unification = "workspace"

[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "linker-features=+lld"]

[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=mold"]`,
    cargoToml: `[profile.dev]
split-debuginfo = "unpacked"
incremental = true
codegen-units = 16
debug = "line-tables-only"

[profile.dev.build-override]
codegen-units = 16
split-debuginfo = "unpacked"
debug = false

[profile.dev.package]
"*" = { debug = false }
serde_derive = { opt-level = 3 }
thiserror-impl = { opt-level = 3 }
tokio-macros = { opt-level = 3 }
strum_macros = { opt-level = 3 }
clap_derive = { opt-level = 3 }
schemars_derive = { opt-level = 3 }
derive_more-impl = { opt-level = 3 }
zerocopy-derive = { opt-level = 3 }
async-trait = { opt-level = 3 }
pin-project-internal = { opt-level = 3 }
bytemuck_derive = { opt-level = 3 }
wasm-bindgen-macro = { opt-level = 3 }
naga = { opt-level = 3 }
wgpu-core = { opt-level = 3 }
flate2 = { opt-level = 3 }
zstd-sys = { opt-level = 3 }
ring = { opt-level = 3 }
aws-lc-rs = { opt-level = 3 }
image = { opt-level = 3 }
pulldown-cmark = { opt-level = 3 }`,
    bundleConfigToml: `[env]
RUSTC_BOOTSTRAP = "1"

[build]
rustflags = ["-Z", "share-generics=y"]`,
    rustToolchainToml: `# Optional upgrade (verified installed locally with LLVM 23.1.5 matching system clang/lld-link 23.1.5):
[toolchain]
channel = "1.100.0" # or retain "1.98.1" for strict upstream zed-industries/zed parity`,
    packageJsonScripts: `{
  "toolchain:doctor": "bun tooling/bun/toolchain-audit.ts doctor",
  "toolchain:profile-diff": "bun tooling/bun/toolchain-audit.ts profile-diff",
  "toolchain:fork-plan": "bun tooling/bun/toolchain-audit.ts fork-plan",
  "clippy:fast": "cargo clippy -p gpui_ui -p gpui_code -p aphrody_desktop --no-deps --all-targets -- -D warnings",
  "test:nextest": "cargo nextest run --workspace"
}`,
  };
}

function opt(flag: string): string | undefined {
  const args = Bun.argv.slice(2);
  const idx = args.indexOf(flag);
  if (idx >= 0) return args[idx + 1];
  const eq = args.find((a) => a.startsWith(`${flag}=`));
  return eq ? eq.slice(flag.length + 1) : undefined;
}

if (import.meta.main) {
  const args = Bun.argv.slice(2);
  const cmd = args[0] ?? "doctor";

  if (cmd === "doctor") {
    const report = runDoctor(repoRoot);
    console.log(JSON.stringify(report, null, 2));
    if (args.includes("--strict") && report.status !== "optimal") {
      process.exit(1);
    }
  } else if (cmd === "profile-diff") {
    const cargoTomlText = readFileSync(join(repoRoot, "Cargo.toml"), "utf8");
    const cfgText = readFileSync(join(repoRoot, ".cargo/config.toml"), "utf8");
    const bundlePath = join(repoRoot, ".cargo/bundle-config.toml");
    const bundleText = existsSync(bundlePath) ? readFileSync(bundlePath, "utf8") : undefined;
    const hotspots = auditWorkspaceHotspots(repoRoot);
    const profiles = analyzeProfiles(cargoTomlText, cfgText, hotspots.lockStats, bundleText);
    const compactHotspots = {
      ...hotspots,
      lockStats: hotspots.lockStats
        ? {
            totalEntries: hotspots.lockStats.totalEntries,
            uniqueCrates: hotspots.lockStats.uniqueCrates,
            duplicatedNames: hotspots.lockStats.duplicatedNames,
            topDuplicates: hotspots.lockStats.topDuplicates.slice(0, 15),
          }
        : null,
    };
    console.log(
      JSON.stringify(
        {
          profiles,
          hotspots: compactHotspots,
          recommendedPatches: buildRecommendedPatches(),
        },
        null,
        2,
      ),
    );
  } else if (cmd === "fork-plan") {
    const target = opt("--target") ?? "all";
    const plan = buildForkPlan({ root: repoRoot, targetFilter: target });
    console.log(JSON.stringify(plan, null, 2));
  } else {
    console.error(
      "Usage: bun tooling/bun/toolchain-audit.ts [doctor [--strict] | profile-diff | fork-plan [--target all|cargo|clippy|wild|sccache]]",
    );
    process.exit(1);
  }
}
