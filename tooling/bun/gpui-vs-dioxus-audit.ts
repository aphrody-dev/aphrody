#!/usr/bin/env bun
/**
 * Code-verified, live-upstream, and fork-audited GPUI vs Dioxus Architecture Graph,
 * Ecosystem Feature Matrix, Cargo Tooling & Integration Engine for Aphrody.
 *
 * Usage:
 *   bun tooling/bun/gpui-vs-dioxus-audit.ts audit [--offline] [--json]
 *   bun tooling/bun/gpui-vs-dioxus-audit.ts emit [--json]
 *   bun tooling/bun/gpui-vs-dioxus-audit.ts all [--offline] [--json]
 */
import { existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join, relative, resolve } from "node:path";
import { parseGitmodules, type SubmoduleEntry } from "./submodules.ts";

export const REPO_ROOT = resolve(import.meta.dir, "../..");
export const AUDIT_JSON_REL = "target/gpui-vs-dioxus/audit.json";
export const MATRIX_TOML_REL = "assets/os/gpui_vs_dioxus_matrix.toml";
export const ARCH_DOC_REL = "docs/architecture/gpui-vs-dioxus-audit.md";

const USER_AGENT = "AphrodyGpuiDioxusAudit/1.0 (+https://github.com/Aphrody-Dev/aphrody)";

export interface SubmoduleAuditSpec {
  path: string;
  forkRepo: string;
  defaultUpstream: string;
  rootUpstream?: string;
  role: string;
}

export const TARGET_SUBMODULES: SubmoduleAuditSpec[] = [
  {
    path: "vendor/dioxus",
    forkRepo: "aphrody-labs/dioxus",
    defaultUpstream: "DioxusLabs/dioxus",
    role: "Dioxus VDOM/Signals/RSX/Fullstack/dx CLI/Subsecond/Manganis workspace",
  },
  {
    path: "vendor/blitz",
    forkRepo: "aphrody-labs/blitz",
    defaultUpstream: "DioxusLabs/blitz",
    role: "Blitz HTML5/CSS DOM engine (Stylo + stylo_taffy + Parley + AnyRender)",
  },
  {
    path: "vendor/anyrender",
    forkRepo: "aphrody-labs/anyrender",
    defaultUpstream: "DioxusLabs/anyrender",
    role: "AnyRender 2D scene abstraction (Vello, vello_gpu, vello_cpu, Skia, wgpu_context)",
  },
  {
    path: "vendor/taffy",
    forkRepo: "aphrody-labs/taffy",
    defaultUpstream: "DioxusLabs/taffy",
    role: "Shared Flexbox/Grid/Block layout engine (patched directly into root Cargo.toml)",
  },
  {
    path: "vendor/wgpu",
    forkRepo: "aphrody-labs/wgpu",
    defaultUpstream: "gfx-rs/wgpu",
    role: "Shared WGPU v30 GPU HAL/Naga shader stack (patched directly into root Cargo.toml)",
  },
  {
    path: "vendor/vello",
    forkRepo: "aphrody-labs/vello",
    defaultUpstream: "linebender/vello",
    role: "Linebender 2D compute & sparse-strips vector renderer (vello, vello_gpu, vello_cpu)",
  },
  {
    path: "vendor/cosmic-text",
    forkRepo: "aphrody-labs/cosmic-text",
    defaultUpstream: "pop-os/cosmic-text",
    role: "Shared multi-line text shaping & glyph layout (patched directly into root Cargo.toml)",
  },
  {
    path: "vendor/accesskit",
    forkRepo: "aphrody-labs/accesskit",
    defaultUpstream: "AccessKit/accesskit",
    role: "Shared OS accessibility tree & platform adapters (patched directly into root Cargo.toml)",
  },
  {
    path: "vendor/winit",
    forkRepo: "aphrody-labs/winit",
    defaultUpstream: "DioxusLabs/winit",
    rootUpstream: "rust-windowing/winit",
    role: "Cross-platform windowing/input event loop (Blitz & Dioxus Native shell)",
  },
];

export const CRATES_IO_TARGETS = [
  "dioxus",
  "dioxus-cli",
  "dioxus-native",
  "subsecond",
  "manganis",
  "blitz-dom",
  "blitz-html",
  "anyrender",
  "gpui",
  "wgpu",
  "taffy",
  "vello",
  "cosmic-text",
  "accesskit",
  "winit",
] as const;

export interface ManifestTargetSpec {
  manifestRel: string;
  ecosystem: "aphrody-gpui" | "aphrody-nsh" | "dioxus" | "blitz-anyrender" | "shared-foundation";
  role: string;
}

export const FEATURE_MANIFEST_TARGETS: ManifestTargetSpec[] = [
  { manifestRel: "crates/gpui/Cargo.toml", ecosystem: "aphrody-gpui", role: "Core GPUI UI framework + Taffy layout + Subsecond hot-reload" },
  { manifestRel: "crates/gpui_wgpu/Cargo.toml", ecosystem: "aphrody-gpui", role: "Shared WGPU v30 GPUI renderer (Linux + Web + Headless)" },
  { manifestRel: "crates/gpui_web/Cargo.toml", ecosystem: "aphrody-gpui", role: "GPUI wasm32 browser platform (WebGPU + WebGL2 + ARIA)" },
  { manifestRel: "crates/gpui_ui/Cargo.toml", ecosystem: "aphrody-gpui", role: "Unified GPUI UI engine + NSH AppView theme bridge" },
  { manifestRel: "crates/gpui_platform/Cargo.toml", ecosystem: "aphrody-gpui", role: "Cross-platform GPUI host selector (Win/Mac/Linux/iOS/Web)" },
  { manifestRel: "crates/gpui_windows/Cargo.toml", ecosystem: "aphrody-gpui", role: "Native Windows D3D11 + DirectComposition + DirectWrite backend" },
  { manifestRel: "crates/gpui_macos/Cargo.toml", ecosystem: "aphrody-gpui", role: "Native macOS Metal + CAMetalDisplayLink backend" },
  { manifestRel: "crates/gpui_linux/Cargo.toml", ecosystem: "aphrody-gpui", role: "Native Wayland/X11 + gpui_wgpu Linux backend" },
  { manifestRel: "crates/gpui_ios/Cargo.toml", ecosystem: "aphrody-gpui", role: "Native iOS UIKit + Metal backend" },
  { manifestRel: "crates/gpui_store/Cargo.toml", ecosystem: "aphrody-gpui", role: "Awesome GPUI Store + upt 0.9.0 + 22-target packager" },
  { manifestRel: "crates/gpui_app_shell/Cargo.toml", ecosystem: "aphrody-gpui", role: "Modular GPUI app starter shell (docks, panes, palette)" },
  { manifestRel: "crates/os_ui/Cargo.toml", ecosystem: "aphrody-gpui", role: "Adaptive viewport + 12-layer unified runtime profile" },
  { manifestRel: "crates/os_desktop/Cargo.toml", ecosystem: "aphrody-gpui", role: "Aphrody OS desktop shell + ArtCraft workbench" },
  { manifestRel: "crates/nsh-core/Cargo.toml", ecosystem: "aphrody-nsh", role: "NSH platform-independent shadow tree + Taffy layout core" },
  { manifestRel: "crates/nsh-platform-gpui/Cargo.toml", ecosystem: "aphrody-nsh", role: "NSH desktop GPUI host embedding bridge" },
  { manifestRel: "crates/nsh-platform-web/Cargo.toml", ecosystem: "aphrody-nsh", role: "NSH wasm32 browser DOM + QuickJS/WebGL host" },
  { manifestRel: "vendor/dioxus/Cargo.toml", ecosystem: "dioxus", role: "Dioxus workspace root + virtual examples manifest" },
  { manifestRel: "vendor/dioxus/packages/dioxus/Cargo.toml", ecosystem: "dioxus", role: "Top-level Dioxus crate + platform feature gates" },
  { manifestRel: "vendor/dioxus/packages/core/Cargo.toml", ecosystem: "dioxus", role: "Dioxus core VDOM + macro/signals/hooks/SSR switches" },
  { manifestRel: "vendor/dioxus/packages/native/Cargo.toml", ecosystem: "dioxus", role: "Dioxus Native Blitz/AnyRender host crate" },
  { manifestRel: "vendor/dioxus/packages/dx/Cargo.toml", ecosystem: "dioxus", role: "Dioxus CLI (dx) build/serve/bundle/hotpatch toolchain" },
  { manifestRel: "vendor/dioxus/packages/web/Cargo.toml", ecosystem: "dioxus", role: "Dioxus browser DOM/hydration renderer" },
  { manifestRel: "vendor/dioxus/packages/desktop/Cargo.toml", ecosystem: "dioxus", role: "Dioxus Wry/Tao OS WebView desktop renderer" },
  { manifestRel: "vendor/dioxus/packages/fullstack/Cargo.toml", ecosystem: "dioxus", role: "Dioxus isomorphic server functions + hydration" },
  { manifestRel: "vendor/dioxus/packages/liveview/Cargo.toml", ecosystem: "dioxus", role: "Dioxus Axum WebSocket server-driven UI" },
  { manifestRel: "vendor/dioxus/packages/subsecond/subsecond/Cargo.toml", ecosystem: "dioxus", role: "Subsecond jump-table Rust hot-patching runtime" },
  { manifestRel: "vendor/dioxus/packages/manganis/manganis/Cargo.toml", ecosystem: "dioxus", role: "Manganis compile-time asset collection/optimization" },
  { manifestRel: "vendor/blitz/Cargo.toml", ecosystem: "blitz-anyrender", role: "Blitz workspace root + [profile.rg-wasm] optimization" },
  { manifestRel: "vendor/blitz/packages/blitz/Cargo.toml", ecosystem: "blitz-anyrender", role: "High-level Blitz HTML/CSS renderer facade" },
  { manifestRel: "vendor/blitz/packages/blitz-dom/Cargo.toml", ecosystem: "blitz-anyrender", role: "Blitz DOM + Stylo CSS + Taffy + Parley + AccessKit" },
  { manifestRel: "vendor/blitz/packages/blitz-html/Cargo.toml", ecosystem: "blitz-anyrender", role: "Blitz HTML5ever/XML5ever parser" },
  { manifestRel: "vendor/blitz/packages/blitz-paint/Cargo.toml", ecosystem: "blitz-anyrender", role: "Blitz display-list painter emitting AnyRender commands" },
  { manifestRel: "vendor/blitz/packages/blitz-shell/Cargo.toml", ecosystem: "blitz-anyrender", role: "Blitz Winit event loop + AccessKit + CDP devtools shell" },
  { manifestRel: "vendor/blitz/packages/stylo_taffy/Cargo.toml", ecosystem: "blitz-anyrender", role: "Servo Stylo -> Taffy layout style translator" },
  { manifestRel: "vendor/blitz/packages/dioxus-native/Cargo.toml", ecosystem: "blitz-anyrender", role: "Blitz-side Dioxus Native runner (vello-gpu / winit 0.31-beta.4)" },
  { manifestRel: "vendor/blitz/examples/wasm_hello/Cargo.toml", ecosystem: "blitz-anyrender", role: "Blitz Wasm WebGPU example tracked by web-build.ts" },
  { manifestRel: "vendor/anyrender/crates/anyrender/Cargo.toml", ecosystem: "blitz-anyrender", role: "AnyRender 2D PaintScene trait" },
  { manifestRel: "vendor/anyrender/crates/anyrender_vello/Cargo.toml", ecosystem: "blitz-anyrender", role: "AnyRender Vello GPU compute backend" },
  { manifestRel: "vendor/anyrender/crates/anyrender_vello_gpu/Cargo.toml", ecosystem: "blitz-anyrender", role: "AnyRender vello_gpu hybrid backend" },
  { manifestRel: "vendor/anyrender/crates/anyrender_vello_cpu/Cargo.toml", ecosystem: "blitz-anyrender", role: "AnyRender CPU sparse-strips raster backend" },
  { manifestRel: "vendor/anyrender/crates/anyrender_skia/Cargo.toml", ecosystem: "blitz-anyrender", role: "AnyRender Skia CPU/Vulkan backend" },
  { manifestRel: "vendor/taffy/Cargo.toml", ecosystem: "shared-foundation", role: "Patched shared Taffy layout crate (0.15.1)" },
  { manifestRel: "vendor/wgpu/wgpu/Cargo.toml", ecosystem: "shared-foundation", role: "Patched shared WGPU v30 crate" },
  { manifestRel: "vendor/vello/research/vello_research/Cargo.toml", ecosystem: "shared-foundation", role: "Vello compute renderer crate (0.12.0)" },
  { manifestRel: "vendor/vello/vello_gpu/Cargo.toml", ecosystem: "shared-foundation", role: "Vello sparse-strips GPU renderer crate (0.4.0)" },
  { manifestRel: "vendor/vello/vello_cpu/Cargo.toml", ecosystem: "shared-foundation", role: "Vello sparse-strips CPU renderer crate (0.4.0)" },
  { manifestRel: "vendor/cosmic-text/Cargo.toml", ecosystem: "shared-foundation", role: "Patched shared Cosmic Text shaper (0.19.0)" },
  { manifestRel: "vendor/accesskit/accesskit/Cargo.toml", ecosystem: "shared-foundation", role: "Patched shared AccessKit core (0.25.2)" },
  { manifestRel: "vendor/winit/winit/Cargo.toml", ecosystem: "shared-foundation", role: "Vendored Winit 0.31.0-beta.4 windowing crate" },
];

export interface SubmoduleStatusRow {
  state: "clean" | "modified" | "uninitialized" | "conflict";
  sha: string;
  path: string;
  refDesc: string;
}

export interface ManifestSummary {
  manifestRel: string;
  ecosystem: ManifestTargetSpec["ecosystem"];
  role: string;
  packageName: string;
  version: string;
  workspaceVersion?: string;
  edition?: string;
  license?: string;
  rustVersion?: string;
  featureCount: number;
  features: Record<string, string[]>;
  dependencies: string[];
  devDependencies: string[];
  targetDepsSummary: string[];
  profiles: string[];
}

export interface SubmoduleForkAudit {
  path: string;
  gitmodulesUrl: string;
  gitmodulesBranch: string;
  localSha: string;
  localState: SubmoduleStatusRow["state"];
  localCommitDate: string;
  localCommitSubject: string;
  forkRepo: string;
  forkDefaultBranch: string;
  forkTargetBranch: string;
  forkSha: string;
  upstreamRepo: string;
  upstreamDefaultBranch: string;
  upstreamSha: string;
  upstreamStars: number;
  upstreamPushedAt: string;
  upstreamLicense: string;
  rootUpstreamRepo?: string;
  rootUpstreamSha?: string;
  localBehindFork: number | null;
  forkAheadUpstream: number | null;
  forkBehindUpstream: number | null;
  driftStatus: string;
  customCommits: Array<{ sha: string; date: string; subject: string }>;
  patchedInRootCargo: boolean;
  excludedFromRootWorkspace: boolean;
  role: string;
}

export interface CratesIoSnapshot {
  name: string;
  maxVersion: string;
  maxStableVersion: string;
  downloads: number;
  recentDownloads: number;
  updatedAt: string;
  repository: string;
  exists: boolean;
  error?: string;
}

export interface HostToolSnapshot {
  name: string;
  command: string;
  installed: boolean;
  version: string;
}

export interface BridgeFinding {
  id: string;
  kind: "active-code-bridge" | "cargo-patch-unification" | "version-split" | "tooling-cohort" | "runtime-enum";
  files: string[];
  symbols: string[];
  summary: string;
  verifiedLines: Array<{ file: string; line: number; text: string }>;
}

export interface EcosystemComparisonRow {
  dimension: string;
  title: string;
  gpuiAphrody: string;
  dioxusBlitzAnyrender: string;
  nshBridge: string;
  unificationPath: string;
}

export interface GpuiVsDioxusAuditReport {
  schemaVersion: number;
  generatedAt: string;
  offlineMode: boolean;
  rootRepo: {
    localSha: string;
    localCommitDate: string;
    localCommitSubject: string;
    forkRepo: string;
    forkSha: string;
    upstreamRepo: string;
    upstreamStars: number;
    upstreamSha: string;
    aheadBy: number | null;
    behindBy: number | null;
    driftStatus: string;
    workspaceMembersCount: number;
    workspaceExcludedCount: number;
    gitmodulesCount: number;
    patchCratesIoCount: number;
    gpuiCrateVersion: string;
  };
  submodules: SubmoduleForkAudit[];
  cratesIo: CratesIoSnapshot[];
  hostTools: {
    tools: HostToolSnapshot[];
    rustupTargets: string[];
    webBuildFrameworks: string[];
  };
  featureMatrix: ManifestSummary[];
  bridges: BridgeFinding[];
  architectureMatrix: EcosystemComparisonRow[];
  summary: {
    submoduleCount: number;
    forkAheadSubmoduleCount: number;
    localBehindForkSubmoduleCount: number;
    patchedSubmoduleCount: number;
    cratesIoCheckedCount: number;
    featureMatrixManifestCount: number;
    totalFeaturesCounted: number;
    activeBridgeCount: number;
    versionSplitCount: number;
    installedHostToolCount: number;
  };
}

// ---------------------------------------------------------------------------
// Generic & Pure Helpers
// ---------------------------------------------------------------------------

export function stripNullish<T>(value: T): T {
  if (Array.isArray(value)) {
    return value.filter((item) => item !== undefined && item !== null).map((item) => stripNullish(item)) as unknown as T;
  }
  if (value !== null && typeof value === "object") {
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(value as Record<string, unknown>)) {
      if (v === undefined || v === null) continue;
      out[k] = stripNullish(v);
    }
    return out as T;
  }
  return value;
}

export function tomlStr(val: string): string {
  return JSON.stringify(val);
}

export function tomlArr(items: readonly string[]): string {
  return `[${items.map((item) => tomlStr(item)).join(", ")}]`;
}

export function parseSubmoduleStatusOutput(stdout: string): SubmoduleStatusRow[] {
  const rows: SubmoduleStatusRow[] = [];
  for (const rawLine of stdout.split(/\r?\n/)) {
    if (!rawLine.trim()) continue;
    const prefix = rawLine[0];
    const match = /^[ +-U]?([0-9a-f]+)\s+(\S+)(?:\s+\(([^)]+)\))?/.exec(rawLine);
    if (!match) continue;
    const state: SubmoduleStatusRow["state"] =
      prefix === "-"
        ? "uninitialized"
        : prefix === "+"
          ? "modified"
          : prefix === "U"
            ? "conflict"
            : "clean";
    rows.push({
      state,
      sha: match[1],
      path: match[2].replace(/\\/g, "/"),
      refDesc: match[3] ?? "",
    });
  }
  return rows;
}

export function classifyForkDrift(params: {
  localSha: string;
  forkSha: string;
  upstreamSha: string;
  aheadBy: number | null;
  behindBy: number | null;
  localBehindFork: number | null;
}): string {
  const { localSha, forkSha, upstreamSha, aheadBy, behindBy, localBehindFork } = params;
  if (localBehindFork !== null && localBehindFork > 0) {
    return `local-behind-fork (-${localBehindFork})`;
  }
  if (localSha && forkSha && localSha !== forkSha && localBehindFork === null) {
    return "local-diverged-from-fork";
  }
  if (aheadBy !== null && behindBy !== null) {
    if (aheadBy === 0 && behindBy === 0) return "identical";
    if (aheadBy > 0 && behindBy === 0) return `ahead (+${aheadBy})`;
    if (aheadBy === 0 && behindBy > 0) return `behind (-${behindBy})`;
    return `diverged (+${aheadBy}/-${behindBy})`;
  }
  if (forkSha && upstreamSha && forkSha === upstreamSha) return "identical";
  return "unknown";
}

export function extractManifestSummary(
  tomlText: string,
  spec: ManifestTargetSpec,
  workspaceVersionFallback?: string,
): ManifestSummary {
  const parsed = Bun.TOML.parse(tomlText) as Record<string, any>;
  const pkg = (parsed.package ?? {}) as Record<string, any>;
  const ws = (parsed.workspace ?? {}) as Record<string, any>;
  const wsPkg = (ws.package ?? {}) as Record<string, any>;
  const wsVersion = typeof wsPkg.version === "string" ? wsPkg.version : workspaceVersionFallback;

  let pkgVersion = "0.0.0";
  if (typeof pkg.version === "string") {
    pkgVersion = pkg.version;
  } else if (pkg.version && typeof pkg.version === "object" && pkg.version.workspace === true && wsVersion) {
    pkgVersion = wsVersion;
  } else if (wsVersion) {
    pkgVersion = wsVersion;
  }

  let edition: string | undefined;
  if (typeof pkg.edition === "string") edition = pkg.edition;
  else if (typeof wsPkg.edition === "string") edition = wsPkg.edition;

  let license: string | undefined;
  if (typeof pkg.license === "string") license = pkg.license;
  else if (typeof wsPkg.license === "string") license = wsPkg.license;

  let rustVersion: string | undefined;
  if (typeof pkg["rust-version"] === "string") rustVersion = pkg["rust-version"];
  else if (typeof wsPkg["rust-version"] === "string") rustVersion = wsPkg["rust-version"];

  const rawFeatures = (parsed.features ?? {}) as Record<string, unknown>;
  const features: Record<string, string[]> = {};
  for (const [k, v] of Object.entries(rawFeatures)) {
    features[k] = Array.isArray(v) ? v.map(String) : [];
  }

  const dependencies = Object.keys((parsed.dependencies ?? {}) as Record<string, unknown>).sort();
  const devDependencies = Object.keys((parsed["dev-dependencies"] ?? {}) as Record<string, unknown>).sort();

  const targetDepsSummary: string[] = [];
  const rawTarget = (parsed.target ?? {}) as Record<string, Record<string, unknown>>;
  for (const [cfg, sections] of Object.entries(rawTarget)) {
    const depKeys = Object.keys((sections?.dependencies ?? {}) as Record<string, unknown>);
    if (depKeys.length > 0) {
      targetDepsSummary.push(`${cfg}: ${depKeys.join(", ")}`);
    }
  }

  const profiles = Object.keys((parsed.profile ?? {}) as Record<string, unknown>).sort();
  const packageName =
    typeof pkg.name === "string"
      ? pkg.name
      : spec.manifestRel.split("/").slice(-2, -1)[0] ?? spec.manifestRel;

  return {
    manifestRel: spec.manifestRel,
    ecosystem: spec.ecosystem,
    role: spec.role,
    packageName,
    version: pkgVersion,
    workspaceVersion: wsVersion,
    edition,
    license,
    rustVersion,
    featureCount: Object.keys(features).length,
    features,
    dependencies,
    devDependencies,
    targetDepsSummary,
    profiles,
  };
}

function runCommand(cmd: string[], cwd = REPO_ROOT): { ok: boolean; stdout: string; stderr: string } {
  try {
    const proc = Bun.spawnSync(cmd, { cwd, stdout: "pipe", stderr: "pipe" });
    return {
      ok: proc.exitCode === 0,
      stdout: proc.stdout ? new TextDecoder().decode(proc.stdout) : "",
      stderr: proc.stderr ? new TextDecoder().decode(proc.stderr) : "",
    };
  } catch (err) {
    return { ok: false, stdout: "", stderr: String(err) };
  }
}

async function ghJsonAsync<T>(endpoint: string): Promise<T | null> {
  try {
    const proc = Bun.spawn(["gh", "api", endpoint], { stdout: "pipe", stderr: "pipe" });
    const [code, out] = await Promise.all([proc.exited, new Response(proc.stdout).text()]);
    if (code !== 0) return null;
    return JSON.parse(out) as T;
  } catch {
    return null;
  }
}

function walkRustAndTomlFiles(dir: string, out: string[] = []): string[] {
  if (!existsSync(dir)) return out;
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.name === "target" || entry.name === ".git" || entry.name === "node_modules") continue;
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      walkRustAndTomlFiles(full, out);
    } else if (entry.name.endsWith(".rs") || entry.name === "Cargo.toml") {
      out.push(full);
    }
  }
  return out;
}

export function scanIntegrationBridges(root = REPO_ROOT): BridgeFinding[] {
  const findings: BridgeFinding[] = [];
  const cratesDir = join(root, "crates");
  const needle = /\b(dioxus|subsecond|blitz|anyrender)\b/i;

  const hitsByFile = new Map<string, Array<{ line: number; text: string }>>();
  for (const file of walkRustAndTomlFiles(cratesDir)) {
    const rel = relative(root, file).replace(/\\/g, "/");
    const lines = readFileSync(file, "utf8").split(/\r?\n/);
    for (let i = 0; i < lines.length; i++) {
      if (needle.test(lines[i])) {
        const list = hitsByFile.get(rel) ?? [];
        list.push({ line: i + 1, text: lines[i].trim() });
        hitsByFile.set(rel, list);
      }
    }
  }

  // 1. GPUI Subsecond / Dioxus Devserver Hot-Reload Bridge
  const gpuiHotFiles = [
    "crates/gpui/Cargo.toml",
    "crates/gpui/build.rs",
    "crates/gpui/src/gpui.rs",
    "crates/gpui/src/hot_reload.rs",
    "crates/gpui/src/view.rs",
  ].filter((f) => existsSync(join(root, f)));
  const gpuiHotLines: Array<{ file: string; line: number; text: string }> = [];
  for (const f of gpuiHotFiles) {
    for (const h of hitsByFile.get(f) ?? []) {
      gpuiHotLines.push({ file: f, ...h });
    }
  }
  findings.push({
    id: "gpui-subsecond-hot-reload",
    kind: "active-code-bridge",
    files: gpuiHotFiles,
    symbols: [
      "gpui::hot_reload",
      "subsecond::HotFn::current",
      "subsecond::apply_patch",
      "subsecond::aslr_reference",
      "dioxus_cli_config::devserver_raw_addr",
      "dioxus_devtools_types::DevserverMsg",
    ],
    summary:
      "crates/gpui implements native hot-reload behind `feature = \"hot-reload\"` (enabled in debug assertions on macOS/Linux/Windows/iOS via crates/gpui/build.rs `gpui_hot_reload`), connecting directly to `dx serve --hot-patch` over WebSocket and detouring View/AnyView renders via `subsecond::HotFn::current`.",
    verifiedLines: gpuiHotLines.slice(0, 20),
  });

  // 2. Shared WGPU v30 + Secondary UI Context Bridge
  const wgpuCtxFile = "crates/gpui_wgpu/src/wgpu_context.rs";
  const wgpuLines = (hitsByFile.get(wgpuCtxFile) ?? []).map((h) => ({ file: wgpuCtxFile, ...h }));
  findings.push({
    id: "gpui-wgpu-shared-handles",
    kind: "active-code-bridge",
    files: [wgpuCtxFile, "crates/gpui_wgpu/Cargo.toml", "Cargo.toml"].filter((f) => existsSync(join(root, f))),
    symbols: ["SharedWgpuHandles", "WgpuContext::shared_handles", "wgpu v30.0.0"],
    summary:
      "gpui_wgpu::WgpuContext::shared_handles exports cloned WGPU v30 Instance/Adapter/Device/Queue handles explicitly documented to share one GPU device with `os_ui`, `egui-wgpu`, `iced_wgpu`, `anyrender`, and `cef` OSR.",
    verifiedLines: wgpuLines,
  });

  // 3. OS UI / OS Desktop Multi-Framework Layer Registry
  const osUiFile = "crates/os_ui/src/unified_runtime.rs";
  const visualSmokeFile = "crates/os_desktop/tests/visual_smoke.rs";
  const osLines = [
    ...(hitsByFile.get(osUiFile) ?? []).map((h) => ({ file: osUiFile, ...h })),
    ...(hitsByFile.get(visualSmokeFile) ?? []).map((h) => ({ file: visualSmokeFile, ...h })),
  ];
  findings.push({
    id: "os-ui-framework-layer-blitz",
    kind: "runtime-enum",
    files: [osUiFile, visualSmokeFile].filter((f) => existsSync(join(root, f))),
    symbols: ["FrameworkLayer::Blitz", "FrameworkLayer::Taffy", "FrameworkLayer::Wgpu", "UnifiedPlatformProfile"],
    summary:
      "os_ui::unified_runtime::FrameworkLayer models 12 unified UI/render layers including Gpui, Nsh, GpuiKit, Craft, Cosmic, Iced, Egui, Wgpu, Taffy, Blitz, Winit, and TauriBun.",
    verifiedLines: osLines,
  });

  // 4. Web Build Pipeline (Blitz wasm_hello tracked in tooling/bun/web-build.ts)
  const webBuildPath = join(root, "tooling/bun/web-build.ts");
  if (existsSync(webBuildPath)) {
    const wbLines = readFileSync(webBuildPath, "utf8").split(/\r?\n/);
    const blitzHits: Array<{ file: string; line: number; text: string }> = [];
    for (let i = 0; i < wbLines.length; i++) {
      if (/blitz|wasm_hello|anyrender/i.test(wbLines[i])) {
        blitzHits.push({ file: "tooling/bun/web-build.ts", line: i + 1, text: wbLines[i].trim() });
      }
    }
    findings.push({
      id: "web-build-blitz-cohort",
      kind: "tooling-cohort",
      files: ["tooling/bun/web-build.ts", "vendor/blitz/examples/wasm_hello/Cargo.toml"].filter((f) =>
        existsSync(join(root, f)),
      ),
      symbols: ["FRAMEWORK_SPECS[id=blitz]", "wasm_hello", "anyrender_vello"],
      summary:
        "tooling/bun/web-build.ts registers `vendor/blitz/examples/wasm_hello` (`wasm_hello`) alongside `os_desktop`, `gpui_store`, `gpui_web`, `nsh`, `craft`, `egui`, `iced`, and `wgpu` for unified Wasm/WebGPU cohort audits.",
      verifiedLines: blitzHits,
    });
  }

  // 5. Cargo [patch.crates-io] Unification across Root + Submodules
  const rootCargoPath = join(root, "Cargo.toml");
  const rootTomlText = existsSync(rootCargoPath) ? readFileSync(rootCargoPath, "utf8") : "";
  const rootParsed = rootTomlText ? (Bun.TOML.parse(rootTomlText) as Record<string, any>) : {};
  const patchCrates = ((rootParsed.patch ?? {}) as Record<string, any>)["crates-io"] ?? {};
  const patchedShared = ["taffy", "cosmic-text", "wgpu", "naga", "accesskit"].filter((k) => k in patchCrates);
  const patchLines: Array<{ file: string; line: number; text: string }> = [];
  if (rootTomlText) {
    const lines = rootTomlText.split(/\r?\n/);
    for (let i = 0; i < lines.length; i++) {
      if (/^(taffy|cosmic-text|wgpu|naga|accesskit)\s*=/.test(lines[i].trim())) {
        patchLines.push({ file: "Cargo.toml", line: i + 1, text: lines[i].trim() });
      }
    }
  }
  findings.push({
    id: "cargo-patch-shared-foundation",
    kind: "cargo-patch-unification",
    files: ["Cargo.toml", ".gitmodules"],
    symbols: patchedShared.map((p) => `[patch.crates-io].${p}`),
    summary: `Root Cargo.toml [patch.crates-io] unifies ${patchedShared.join(", ")} directly onto local vendor/* submodule checkouts, while vendor/{dioxus,blitz,anyrender,vello,winit} remain in [workspace.exclude] to prevent multi-workspace Cargo resolver conflicts.`,
    verifiedLines: patchLines.slice(0, 16),
  });

  // 6. Verified Version Splits (Code/Manifest Drifts Found Directly in Repo)
  const gpuiTomlPath = join(root, "crates/gpui/Cargo.toml");
  const dioxusTomlPath = join(root, "vendor/dioxus/Cargo.toml");
  const blitzTomlPath = join(root, "vendor/blitz/Cargo.toml");
  const anyrenderTomlPath = join(root, "vendor/anyrender/Cargo.toml");
  const taffyTomlPath = join(root, "vendor/taffy/Cargo.toml");

  const splitLines: Array<{ file: string; line: number; text: string }> = [];
  const checkPattern = (relFile: string, re: RegExp) => {
    const p = join(root, relFile);
    if (!existsSync(p)) return;
    const lines = readFileSync(p, "utf8").split(/\r?\n/);
    for (let i = 0; i < lines.length; i++) {
      if (re.test(lines[i])) {
        splitLines.push({ file: relFile, line: i + 1, text: lines[i].trim() });
      }
    }
  };
  checkPattern("crates/gpui/Cargo.toml", /(subsecond|dioxus-cli-config|dioxus-devtools-types)\s*=/);
  checkPattern("vendor/dioxus/Cargo.toml", /^(version|anyrender|anyrender_vello_hybrid|vello|winit|blitz-dom)\s*=/);
  checkPattern("vendor/blitz/Cargo.toml", /^(version|anyrender|anyrender_vello_gpu|vello|vello_gpu|winit)\s*=/);
  checkPattern("vendor/anyrender/Cargo.toml", /^(anyrender|anyrender_vello_gpu|vello_gpu)\s*=/);
  checkPattern("vendor/taffy/Cargo.toml", /^version\s*=/);

  const gpuiText = existsSync(gpuiTomlPath) ? readFileSync(gpuiTomlPath, "utf8") : "";
  const dioxusText = existsSync(dioxusTomlPath) ? readFileSync(dioxusTomlPath, "utf8") : "";
  const blitzText = existsSync(blitzTomlPath) ? readFileSync(blitzTomlPath, "utf8") : "";
  const anyrenderText = existsSync(anyrenderTomlPath) ? readFileSync(anyrenderTomlPath, "utf8") : "";
  const taffyText = existsSync(taffyTomlPath) ? readFileSync(taffyTomlPath, "utf8") : "";

  const gpuiDioxusPin = gpuiText.match(/subsecond\s*=\s*\{\s*version\s*=\s*"([^"]+)"/)?.[1] ?? "unknown";
  const dioxusWsVer = dioxusText.match(/\[workspace\.package\]\r?\nversion\s*=\s*"([^"]+)"/)?.[1] ?? "unknown";
  const blitzAnyrenderVer = blitzText.match(/anyrender\s*=\s*"([^"]+)"/)?.[1] ?? "unknown";
  const dioxusAnyrenderVer = dioxusText.match(/anyrender\s*=\s*\{\s*version\s*=\s*"([^"]+)"/)?.[1] ?? "unknown";
  const taffyVer = taffyText.match(/^version\s*=\s*"([^"]+)"/m)?.[1] ?? "unknown";
  const hasRenamedVelloGpu = anyrenderText.includes("anyrender_vello_gpu");

  findings.push({
    id: "verified-version-splits",
    kind: "version-split",
    files: [
      "crates/gpui/Cargo.toml",
      "vendor/dioxus/Cargo.toml",
      "vendor/blitz/Cargo.toml",
      "vendor/anyrender/Cargo.toml",
      "vendor/taffy/Cargo.toml",
    ].filter((f) => existsSync(join(root, f))),
    symbols: [
      `gpui->subsecond(${gpuiDioxusPin})`,
      `vendor/dioxus(${dioxusWsVer})`,
      `dioxus->anyrender(${dioxusAnyrenderVer})`,
      `blitz->anyrender(${blitzAnyrenderVer})`,
      `taffy(${taffyVer})`,
    ],
    summary: `Live manifest inspection reveals 3 concrete version splits: (1) crates/gpui hot-reload pins subsecond/dioxus-cli-config/dioxus-devtools-types at '${gpuiDioxusPin}' (matching host dx CLI) while vendor/dioxus is already '${dioxusWsVer}'; (2) vendor/dioxus pins anyrender '${dioxusAnyrenderVer}' + anyrender_vello_hybrid + winit 0.31.0-beta.3 while vendor/blitz + vendor/anyrender already migrated to anyrender '${blitzAnyrenderVer}' + ${hasRenamedVelloGpu ? "anyrender_vello_gpu" : "anyrender_vello_hybrid"} + vello 0.12 + winit 0.31.0-beta.4; (3) vendor/taffy carries fork bump '${taffyVer}' so [patch.crates-io] outranks crates.io.`,
    verifiedLines: splitLines.slice(0, 24),
  });

  return findings;
}

export function buildArchitectureComparison(report: Pick<GpuiVsDioxusAuditReport, "bridges" | "submodules" | "hostTools" | "featureMatrix"> & { rootRepo?: Partial<GpuiVsDioxusAuditReport["rootRepo"]> }): EcosystemComparisonRow[] {
  const gpuiFeatCount = report.featureMatrix.find((m) => m.manifestRel === "crates/gpui/Cargo.toml")?.featureCount ?? 0;
  const dioxusFeatCount =
    report.featureMatrix.find((m) => m.manifestRel === "vendor/dioxus/packages/dioxus/Cargo.toml")?.featureCount ?? 0;
  const nativeFeatCount =
    report.featureMatrix.find((m) => m.manifestRel === "vendor/dioxus/packages/native/Cargo.toml")?.featureCount ?? 0;
  const blitzDomFeatCount =
    report.featureMatrix.find((m) => m.manifestRel === "vendor/blitz/packages/blitz-dom/Cargo.toml")?.featureCount ?? 0;

  const byPath = new Map(report.submodules.map((s) => [s.path, s]));
  const subLabel = (path: string) => {
    const s = byPath.get(path);
    return s ? `${path} (${s.driftStatus})` : path;
  };
  const taffyVer = report.featureMatrix.find((m) => m.manifestRel === "vendor/taffy/Cargo.toml")?.version ?? "0.15.1";
  const wgpuVer = report.featureMatrix.find((m) => m.manifestRel === "vendor/wgpu/wgpu/Cargo.toml")?.version ?? "30.0.0";
  const gpuiSubsecondDep =
    report.bridges.find((b) => b.id === "verified-version-splits")?.symbols.find((s) => s.startsWith("gpui->subsecond")) ??
    "gpui->subsecond";
  const dioxusWsVer =
    report.featureMatrix.find((m) => m.manifestRel === "vendor/dioxus/Cargo.toml")?.workspaceVersion ??
    report.featureMatrix.find((m) => m.manifestRel === "vendor/dioxus/packages/dioxus/Cargo.toml")?.version ??
    "0.8.0-alpha.3";
  const wsMembers = report.rootRepo?.workspaceMembersCount ?? 302;

  return [
    {
      dimension: "reactivity_state",
      title: "Reactivity & State Model",
      gpuiAphrody: `Entity<T> slotmap arena + Context<T> observers + retained views + immediate Element trees (${gpuiFeatCount} features in crates/gpui). Zero virtual DOM diffing.`,
      dioxusBlitzAnyrender: `GenerationalBox Signal<T> / Memo / Store<T> + VirtualDom + rsx! template macro (${dioxusFeatCount} features in dioxus). Fine-grained signal subscriptions re-render components into template mutations.`,
      nshBridge: "crates/nsh-core provides SolidJS-style reactive signals/computed/effects with slab shadow tree bridged into gpui_ui::nsh::AppView.",
      unificationPath: "Keep GPUI Entity<T> as native OS/IDE host state; embed NSH TSX/reactive panels via gpui_ui::nsh and inspect Dioxus Signal patterns for fine-grained invalidation.",
    },
    {
      dimension: "layout_engine",
      title: "Layout & Styling Engine",
      gpuiAphrody: "Direct taffy::TaffyTree in crates/gpui/src/taffy.rs with Tailwind-like fluent Styled builder (currently clears TaffyTree per frame; merge candidate longbridge/gpui-fast adds dirty tracking).",
      dioxusBlitzAnyrender: `Full HTML5/CSS cascade via Servo stylo -> stylo_taffy -> taffy + Parley inline text layout inside blitz-dom (${blitzDomFeatCount} features) or browser CSS in dioxus-web/desktop.`,
      nshBridge: "nsh-core translates Tailwind/CSS utility classes directly to shared vendor/taffy without Servo Stylo overhead.",
      unificationPath: `Both GPUI, Blitz, and NSH converge on single patched vendor/taffy (${taffyVer}); use direct Taffy for 120 FPS OS chrome and Blitz DOM only when full HTML/CSS rendering is needed.`,
    },
    {
      dimension: "gpu_rendering",
      title: "GPU & Rendering Pipeline",
      gpuiAphrody: `Hybrid native + WGPU: Metal/CAMetalDisplayLink (macOS/iOS), D3D11/DirectComposition (Windows), gpui_wgpu on wgpu ${wgpuVer} (Linux Wayland/X11 + WebGPU/WebGL2). Zero Blade remaining.`,
      dioxusBlitzAnyrender: `AnyRender 2D scene abstraction over vello compute, vello_gpu hybrid, vello_cpu sparse strips, or Skia (${nativeFeatCount} features in dioxus-native), plus Wry WebView in dioxus-desktop.`,
      nshBridge: "gpui_wgpu::WgpuContext::shared_handles() explicitly exports WGPU Instance/Adapter/Device/Queue for secondary renderers including anyrender.",
      unificationPath: `Zero-copy compose AnyRender/Vello scenes inside GPUI surfaces using WgpuContext::shared_handles() on unified vendor/wgpu (${wgpuVer}).`,
    },
    {
      dimension: "hot_reload",
      title: "Hot-Reload & Dev Server",
      gpuiAphrody: `crates/gpui/src/hot_reload.rs + view.rs wire Dioxus subsecond jump-table patching (${gpuiSubsecondDep}) + dioxus-cli-config + dioxus-devtools-types over \`dx serve --hot-patch\` WebSocket.`,
      dioxusBlitzAnyrender: `dx serve provides dual RSX literal/template hot-reload (dioxus-rsx-hotreload) + Subsecond Rust function jump-table patching + ThinLinking (${dioxusWsVer}).`,
      nshBridge: "nsh-dev + nsh-bundler provide Oxc/Rolldown TSX HMR for embedded NSH panels.",
      unificationPath: `Align ${gpuiSubsecondDep} pin with vendor/dioxus (${dioxusWsVer}) once dx 0.8.0 stabilizes so both Rust GPUI and NSH TSX hot-patch from one workflow.`,
    },
    {
      dimension: "asset_pipeline",
      title: "Asset Bundling & Resolution",
      gpuiAphrody: "Compile-time rust-embed 8.11 (crates/assets) implementing gpui::AssetSource plus brand/theme TOML catalogs (include_str!).",
      dioxusBlitzAnyrender: "manganis::asset!() macro embeds link-section asset metadata consumed by dx CLI (lightningcss, grass, mozjpeg, imagequant, brotli) + dioxus-asset-resolver.",
      nshBridge: "nsh-core asset loader + gpui_store embedded TOML catalog.",
      unificationPath: "Keep rust-embed/include_str! for zero-external-CLI Cargo builds of Aphrody OS; use Manganis only inside Dioxus/dx-managed sub-apps.",
    },
    {
      dimension: "multi_platform",
      title: "Multi-Platform & Windowing",
      gpuiAphrody: "gpui_platform selects custom per-OS native crates (gpui_macos, gpui_windows, gpui_linux, gpui_ios, gpui_web) without Winit overhead.",
      dioxusBlitzAnyrender: "Unified dioxus crate feature-gates web, desktop (Wry/Tao), mobile, native (Blitz + Winit 0.31), liveview (Axum WS), ssr, and fullstack.",
      nshBridge: "nsh spans GPUI, Web DOM, TUI (Ratatui), Android, iOS, macOS, Linux, and Windows; os_ui tracks all 12 layers.",
      unificationPath: "Use GPUI native platforms for OS/IDE windows (avoiding Winit beta drift) and optionally mount Blitz/Dioxus Native documents as embedded HTML/CSS panes.",
    },
    {
      dimension: "bundle_cross_compile",
      title: "Bundle Size & Cross-Compile Tooling",
      gpuiAphrody: "tooling/bun/web-build.ts builds 3 Wasm cohorts (baseline, simd128, --coi) with [profile.web] + wasm-opt -Oz; tooling/bun/cross-build.ts drives cargo-xwin/cargo-zigbuild.",
      dioxusBlitzAnyrender: "dx bundle + wasm-split-cli + [profile.wasm-split-release] / [profile.rg-wasm] (opt-level=z, fat LTO, strip) across Web, Desktop, Android, and iOS.",
      nshBridge: "nsh-platform-web linkable Wasm + tooling/bun/web-build.ts tracks blitz wasm_hello alongside os_desktop and gpui_web.",
      unificationPath: "Port Blitz/Taffy [profile.rg-wasm] insights into Aphrody WebGPU CI budgets and reuse wasm-split patterns in tooling/bun/web-build.ts.",
    },
    {
      dimension: "fork_unification",
      title: "Aphrody Fork & Workspace Unification",
      gpuiAphrody: `Aphrody-Dev/aphrody (${report.rootRepo?.driftStatus ?? "diverged"} vs zed-industries/zed) centralizes ${wsMembers} workspace crates and patches taffy, cosmic-text, wgpu, accesskit via [patch.crates-io].`,
      dioxusBlitzAnyrender: `Fork status: ${subLabel("vendor/dioxus")}, ${subLabel("vendor/blitz")}, ${subLabel("vendor/taffy")}, ${subLabel("vendor/anyrender")}, ${subLabel("vendor/wgpu")}.`,
      nshBridge: "All .gitmodules use relative ../../aphrody-labs/*.git URLs with single checkouts in vendor/*.",
      unificationPath: "Reconcile vendor/dioxus anyrender 0.14/vello 0.11 vs vendor/blitz anyrender 0.15/vello 0.12 and track vendor/wgpu main vs trunk before promoting Blitz into root workspace.",
    },
  ];
}

// ---------------------------------------------------------------------------
// Live Inspection Runners
// ---------------------------------------------------------------------------

async function fetchCratesIoSnapshots(): Promise<CratesIoSnapshot[]> {
  const results = await Promise.all(
    CRATES_IO_TARGETS.map(async (name): Promise<CratesIoSnapshot> => {
      try {
        const res = await fetch(`https://crates.io/api/v1/crates/${encodeURIComponent(name)}`, {
          headers: { "User-Agent": USER_AGENT },
        });
        if (!res.ok) {
          return {
            name,
            maxVersion: "unknown",
            maxStableVersion: "unknown",
            downloads: 0,
            recentDownloads: 0,
            updatedAt: "",
            repository: "",
            exists: false,
            error: `HTTP ${res.status}`,
          };
        }
        const payload = (await res.json()) as { crate: Record<string, any> };
        const c = payload.crate;
        return {
          name,
          maxVersion: String(c.max_version ?? "unknown"),
          maxStableVersion: String(c.max_stable_version ?? "unknown"),
          downloads: Number(c.downloads ?? 0),
          recentDownloads: Number(c.recent_downloads ?? 0),
          updatedAt: String(c.updated_at ?? ""),
          repository: String(c.repository ?? ""),
          exists: true,
        };
      } catch (err) {
        return {
          name,
          maxVersion: "unknown",
          maxStableVersion: "unknown",
          downloads: 0,
          recentDownloads: 0,
          updatedAt: "",
          repository: "",
          exists: false,
          error: String(err),
        };
      }
    }),
  );
  return results;
}

function inspectHostTools(root = REPO_ROOT): GpuiVsDioxusAuditReport["hostTools"] {
  const probes: Array<{ name: string; command: string; args: string[] }> = [
    { name: "dx", command: "dx", args: ["--version"] },
    { name: "cargo", command: "cargo", args: ["--version"] },
    { name: "rustc", command: "rustc", args: ["--version"] },
    { name: "wasm-bindgen", command: "wasm-bindgen", args: ["--version"] },
    { name: "wasm-opt", command: "wasm-opt", args: ["--version"] },
    { name: "bun", command: "bun", args: ["--version"] },
  ];
  const tools: HostToolSnapshot[] = probes.map((p) => {
    const res = runCommand([p.command, ...p.args], root);
    const out = (res.stdout || res.stderr).trim().split(/\r?\n/)[0] ?? "";
    return {
      name: p.name,
      command: `${p.command} ${p.args.join(" ")}`,
      installed: res.ok,
      version: res.ok ? out : "missing",
    };
  });

  const targetsRes = runCommand(["rustup", "target", "list", "--installed"], root);
  const rustupTargets = targetsRes.ok
    ? targetsRes.stdout
        .split(/\r?\n/)
        .map((s) => s.trim())
        .filter(Boolean)
    : [];

  const wbPath = join(root, "tooling/bun/web-build.ts");
  const webBuildFrameworks: string[] = [];
  if (existsSync(wbPath)) {
    const wbText = readFileSync(wbPath, "utf8");
    for (const m of wbText.matchAll(/id:\s*"([^"]+)"[\s\S]*?packageName:\s*"([^"]+)"/g)) {
      webBuildFrameworks.push(`${m[1]}(${m[2]})`);
    }
  }

  return { tools, rustupTargets, webBuildFrameworks };
}

async function inspectSubmodulesAndForks(
  root: string,
  gitmodules: SubmoduleEntry[],
  patchKeys: Set<string>,
  excludedList: Set<string>,
  offline: boolean,
): Promise<SubmoduleForkAudit[]> {
  const subPaths = TARGET_SUBMODULES.map((s) => s.path);
  const statusRes = runCommand(["git", "-C", root, "submodule", "status", ...subPaths], root);
  const statusRows = parseSubmoduleStatusOutput(statusRes.stdout);
  const statusMap = new Map(statusRows.map((r) => [r.path, r]));
  const gmMap = new Map(gitmodules.map((g) => [g.path.replace(/\\/g, "/"), g]));

  return Promise.all(
    TARGET_SUBMODULES.map(async (spec) => {
      const gm = gmMap.get(spec.path);
      const st = statusMap.get(spec.path);
      const subDir = join(root, spec.path);

      const headShaRes = existsSync(subDir) ? runCommand(["git", "-C", subDir, "rev-parse", "HEAD"], root) : { ok: false, stdout: "" };
      const localSha = (st?.sha || headShaRes.stdout.trim() || "").trim();
      const logRes = existsSync(subDir)
        ? runCommand(["git", "-C", subDir, "log", "-1", "--format=%cI%n%s"], root)
        : { ok: false, stdout: "" };
      const [localCommitDate = "", localCommitSubject = ""] = logRes.stdout.trim().split(/\r?\n/);

      const gitmodulesBranch = gm?.branch ?? "main";
      let forkDefaultBranch = gitmodulesBranch;
      let forkTargetBranch = gitmodulesBranch;
      let forkSha = localSha;
      let upstreamRepo = spec.defaultUpstream;
      let upstreamDefaultBranch = "main";
      let upstreamSha = "";
      let upstreamStars = 0;
      let upstreamPushedAt = "";
      let upstreamLicense = "";
      let rootUpstreamSha: string | undefined;
      let localBehindFork: number | null = null;
      let forkAheadUpstream: number | null = null;
      let forkBehindUpstream: number | null = null;
      const customCommits: Array<{ sha: string; date: string; subject: string }> = [];

      if (!offline) {
        const forkMeta = await ghJsonAsync<Record<string, any>>(`repos/${spec.forkRepo}`);
        if (forkMeta) {
          forkDefaultBranch = String(forkMeta.default_branch ?? gitmodulesBranch);
          if (forkMeta.parent?.full_name) {
            upstreamRepo = String(forkMeta.parent.full_name);
          }
        }
        forkTargetBranch = gm?.branch ?? forkDefaultBranch;

        const [forkBranchCommit, upMeta, rootUpMeta] = await Promise.all([
          ghJsonAsync<Record<string, any>>(`repos/${spec.forkRepo}/commits/${forkTargetBranch}`),
          ghJsonAsync<Record<string, any>>(`repos/${upstreamRepo}`),
          spec.rootUpstream ? ghJsonAsync<Record<string, any>>(`repos/${spec.rootUpstream}`) : Promise.resolve(null),
        ]);

        if (forkBranchCommit?.sha) {
          forkSha = String(forkBranchCommit.sha);
        }
        if (upMeta) {
          upstreamDefaultBranch = String(upMeta.default_branch ?? "main");
          upstreamStars = Number(upMeta.stargazers_count ?? 0);
          upstreamPushedAt = String(upMeta.pushed_at ?? "");
          upstreamLicense = String(upMeta.license?.spdx_id ?? "");
        }

        const [upCommit, rootCommit] = await Promise.all([
          ghJsonAsync<Record<string, any>>(`repos/${upstreamRepo}/commits/${upstreamDefaultBranch}`),
          spec.rootUpstream
            ? ghJsonAsync<Record<string, any>>(`repos/${spec.rootUpstream}/commits/${String(rootUpMeta?.default_branch ?? "master")}`)
            : Promise.resolve(null),
        ]);

        if (upCommit?.sha) {
          upstreamSha = String(upCommit.sha);
        }
        if (rootCommit?.sha) {
          rootUpstreamSha = String(rootCommit.sha);
        }

        const localCmpPromise =
          localSha && forkSha && localSha !== forkSha
            ? ghJsonAsync<Record<string, any>>(`repos/${spec.forkRepo}/compare/${localSha}...${forkTargetBranch}`)
            : Promise.resolve(null);

        const forkOwner = spec.forkRepo.split("/")[0];
        const forkCmpPromise =
          forkSha && upstreamSha && forkSha !== upstreamSha
            ? ghJsonAsync<Record<string, any>>(
                `repos/${upstreamRepo}/compare/${upstreamDefaultBranch}...${forkOwner}:${forkTargetBranch}`,
              ).then((res) =>
                res ??
                ghJsonAsync<Record<string, any>>(
                  `repos/${upstreamRepo}/compare/${upstreamDefaultBranch}...${forkOwner}:${upstreamDefaultBranch}`,
                ),
              )
            : Promise.resolve(null);

        const [cmpLocal, cmpFork] = await Promise.all([localCmpPromise, forkCmpPromise]);

        if (localSha && forkSha) {
          if (localSha === forkSha) {
            localBehindFork = 0;
          } else if (cmpLocal && typeof cmpLocal.ahead_by === "number") {
            localBehindFork = cmpLocal.ahead_by;
          }
        }

        if (forkSha && upstreamSha) {
          if (forkSha === upstreamSha) {
            forkAheadUpstream = 0;
            forkBehindUpstream = 0;
          } else if (cmpFork && typeof cmpFork.ahead_by === "number") {
            forkAheadUpstream = cmpFork.ahead_by;
            forkBehindUpstream = typeof cmpFork.behind_by === "number" ? cmpFork.behind_by : 0;
            for (const c of (cmpFork.commits ?? []).slice(0, 10) as Array<Record<string, any>>) {
              customCommits.push({
                sha: String(c.sha ?? "").slice(0, 8),
                date: String(c.commit?.committer?.date ?? ""),
                subject: String(c.commit?.message ?? "").split(/\r?\n/)[0] ?? "",
              });
            }
          }
        }
      }

      const pkgKeyCandidates = [spec.path.split("/").pop() ?? ""];
      if (spec.path === "vendor/wgpu") pkgKeyCandidates.push("wgpu", "naga");
      const patchedInRootCargo = pkgKeyCandidates.some((k) => patchKeys.has(k));
      const excludedFromRootWorkspace = excludedList.has(spec.path);

      const driftStatus = classifyForkDrift({
        localSha,
        forkSha,
        upstreamSha,
        aheadBy: forkAheadUpstream,
        behindBy: forkBehindUpstream,
        localBehindFork,
      });

      return {
        path: spec.path,
        gitmodulesUrl: gm?.url ?? "",
        gitmodulesBranch,
        localSha,
        localState: st?.state ?? (existsSync(subDir) ? "clean" : "uninitialized"),
        localCommitDate,
        localCommitSubject,
        forkRepo: spec.forkRepo,
        forkDefaultBranch,
        forkTargetBranch,
        forkSha,
        upstreamRepo,
        upstreamDefaultBranch,
        upstreamSha,
        upstreamStars,
        upstreamPushedAt,
        upstreamLicense,
        rootUpstreamRepo: spec.rootUpstream,
        rootUpstreamSha,
        localBehindFork,
        forkAheadUpstream,
        forkBehindUpstream,
        driftStatus,
        customCommits,
        patchedInRootCargo,
        excludedFromRootWorkspace,
        role: spec.role,
      };
    }),
  );
}

export async function runAudit(options: { root?: string; offline?: boolean } = {}): Promise<GpuiVsDioxusAuditReport> {
  const root = options.root ?? REPO_ROOT;
  const offline = Boolean(options.offline);

  const rootCargoRaw = readFileSync(join(root, "Cargo.toml"), "utf8");
  const rootCargo = Bun.TOML.parse(rootCargoRaw) as Record<string, any>;
  const rootWs = (rootCargo.workspace ?? {}) as Record<string, any>;
  const wsMembers = Array.isArray(rootWs.members) ? rootWs.members.map(String) : [];
  const wsExcluded = new Set<string>(Array.isArray(rootWs.exclude) ? rootWs.exclude.map(String) : []);
  const patchCratesIo = ((rootCargo.patch ?? {}) as Record<string, any>)["crates-io"] ?? {};
  const patchKeys = new Set(Object.keys(patchCratesIo));
  const rootWsVersion = typeof rootWs.package?.version === "string" ? rootWs.package.version : "0.5.0";

  const gitmodulesPath = join(root, ".gitmodules");
  const gitmodules = existsSync(gitmodulesPath) ? parseGitmodules(readFileSync(gitmodulesPath, "utf8")) : [];

  // Read-only cargo metadata --no-deps verification on root workspace
  const cargoMetaRes = runCommand(["cargo", "metadata", "--no-deps", "--format-version", "1"], root);
  let workspaceMembersCount = wsMembers.length;
  const metaPkgMap = new Map<string, Record<string, any>>();
  if (cargoMetaRes.ok) {
    try {
      const meta = JSON.parse(cargoMetaRes.stdout) as { workspace_members?: string[]; packages?: Array<Record<string, any>> };
      if (Array.isArray(meta.workspace_members)) {
        workspaceMembersCount = meta.workspace_members.length;
      }
      for (const p of meta.packages ?? []) {
        if (typeof p.name === "string") metaPkgMap.set(p.name, p);
      }
    } catch {
      // Fallback cleanly to Cargo.toml counts
    }
  }

  const rootHeadSha = runCommand(["git", "-C", root, "rev-parse", "HEAD"], root).stdout.trim();
  const rootLog = runCommand(["git", "-C", root, "log", "-1", "--format=%cI%n%s"], root).stdout.trim().split(/\r?\n/);

  let forkSha = rootHeadSha;
  let upstreamStars = 0;
  let upstreamSha = "";
  let aheadBy: number | null = null;
  let behindBy: number | null = null;
  let rootDrift = "offline";

  if (!offline) {
    const [aphRepo, zedRepo, cmpRoot] = await Promise.all([
      ghJsonAsync<Record<string, any>>("repos/Aphrody-Dev/aphrody"),
      ghJsonAsync<Record<string, any>>("repos/zed-industries/zed"),
      ghJsonAsync<Record<string, any>>("repos/zed-industries/zed/compare/main...Aphrody-Dev:main"),
    ]);
    if (zedRepo) upstreamStars = Number(zedRepo.stargazers_count ?? 0);
    const [aphHead, zedHead] = await Promise.all([
      ghJsonAsync<Record<string, any>>(`repos/Aphrody-Dev/aphrody/commits/${aphRepo?.default_branch ?? "main"}`),
      ghJsonAsync<Record<string, any>>(`repos/zed-industries/zed/commits/${zedRepo?.default_branch ?? "main"}`),
    ]);
    if (aphHead?.sha) forkSha = String(aphHead.sha);
    if (zedHead?.sha) upstreamSha = String(zedHead.sha);
    if (cmpRoot && typeof cmpRoot.ahead_by === "number") {
      aheadBy = cmpRoot.ahead_by;
      behindBy = typeof cmpRoot.behind_by === "number" ? cmpRoot.behind_by : 0;
      rootDrift = String(cmpRoot.status ?? `diverged (+${aheadBy}/-${behindBy})`);
    }
  }

  const [submodules, cratesIo] = await Promise.all([
    inspectSubmodulesAndForks(root, gitmodules, patchKeys, wsExcluded, offline),
    offline ? Promise.resolve([] as CratesIoSnapshot[]) : fetchCratesIoSnapshots(),
  ]);
  const hostTools = inspectHostTools(root);

  const wsVersionCache = new Map<string, string | undefined>();
  const readWorkspacePkgVersion = (tomlRel: string): string | undefined => {
    if (wsVersionCache.has(tomlRel)) return wsVersionCache.get(tomlRel);
    const p = join(root, tomlRel);
    let ver: string | undefined;
    if (existsSync(p)) {
      const parsed = Bun.TOML.parse(readFileSync(p, "utf8")) as Record<string, any>;
      if (typeof parsed.workspace?.package?.version === "string") {
        ver = parsed.workspace.package.version;
      }
    }
    wsVersionCache.set(tomlRel, ver);
    return ver;
  };
  const resolveWorkspaceVersionFallback = (manifestRel: string): string | undefined => {
    if (manifestRel.startsWith("vendor/")) {
      const subName = manifestRel.split("/")[1];
      const subWsVer = readWorkspacePkgVersion(`vendor/${subName}/Cargo.toml`);
      if (subWsVer) return subWsVer;
    }
    return rootWsVersion;
  };

  const featureMatrix: ManifestSummary[] = [];
  for (const spec of FEATURE_MANIFEST_TARGETS) {
    const full = join(root, spec.manifestRel);
    if (!existsSync(full)) continue;
    const wsFallback = resolveWorkspaceVersionFallback(spec.manifestRel);
    const summary = extractManifestSummary(readFileSync(full, "utf8"), spec, wsFallback);
    const metaPkg = metaPkgMap.get(summary.packageName);
    if (metaPkg && metaPkg.features && typeof metaPkg.features === "object") {
      const metaFeatures: Record<string, string[]> = {};
      for (const [k, v] of Object.entries(metaPkg.features as Record<string, unknown>)) {
        metaFeatures[k] = Array.isArray(v) ? v.map(String) : [];
      }
      summary.features = metaFeatures;
      summary.featureCount = Object.keys(metaFeatures).length;
    }
    featureMatrix.push(summary);
  }

  const bridges = scanIntegrationBridges(root);
  const gpuiPkg = featureMatrix.find((m) => m.manifestRel === "crates/gpui/Cargo.toml");

  const rootRepo = {
    localSha: rootHeadSha,
    localCommitDate: rootLog[0] ?? "",
    localCommitSubject: rootLog[1] ?? "",
    forkRepo: "Aphrody-Dev/aphrody",
    forkSha,
    upstreamRepo: "zed-industries/zed",
    upstreamStars,
    upstreamSha,
    aheadBy,
    behindBy,
    driftStatus: rootDrift,
    workspaceMembersCount,
    workspaceExcludedCount: wsExcluded.size,
    gitmodulesCount: gitmodules.length,
    patchCratesIoCount: patchKeys.size,
    gpuiCrateVersion: gpuiPkg?.version ?? "0.2.2",
  };

  const architectureMatrix = buildArchitectureComparison({
    submodules,
    hostTools,
    featureMatrix,
    bridges,
    rootRepo,
  });

  const report: GpuiVsDioxusAuditReport = {
    schemaVersion: 1,
    generatedAt: new Date().toISOString(),
    offlineMode: offline,
    rootRepo,
    submodules,
    cratesIo,
    hostTools,
    featureMatrix,
    bridges,
    architectureMatrix,
    summary: {
      submoduleCount: submodules.length,
      forkAheadSubmoduleCount: submodules.filter((s) => (s.forkAheadUpstream ?? 0) > 0).length,
      localBehindForkSubmoduleCount: submodules.filter((s) => (s.localBehindFork ?? 0) > 0).length,
      patchedSubmoduleCount: submodules.filter((s) => s.patchedInRootCargo).length,
      cratesIoCheckedCount: cratesIo.filter((c) => c.exists).length,
      featureMatrixManifestCount: featureMatrix.length,
      totalFeaturesCounted: featureMatrix.reduce((acc, m) => acc + m.featureCount, 0),
      activeBridgeCount: bridges.filter((b) => b.kind === "active-code-bridge").length,
      versionSplitCount: bridges.filter((b) => b.kind === "version-split").length,
      installedHostToolCount: hostTools.tools.filter((t) => t.installed).length,
    },
  };

  return report;
}

// ---------------------------------------------------------------------------
// TOML, Mermaid & Markdown Emission
// ---------------------------------------------------------------------------

export function serializeMatrixToml(report: GpuiVsDioxusAuditReport): string {
  const clean = stripNullish(report);
  const lines: string[] = [
    "# Auto-generated by `bun tooling/bun/gpui-vs-dioxus-audit.ts emit`.",
    "# Verified live GPUI vs Dioxus/Blitz/AnyRender fork graph, Cargo feature matrix, crates.io telemetry & bridge inventory.",
    `schema_version = ${clean.schemaVersion}`,
    `generated_at = ${tomlStr(clean.generatedAt)}`,
    `offline_mode = ${clean.offlineMode}`,
    "",
    "[summary]",
    `submodule_count = ${clean.summary.submoduleCount}`,
    `fork_ahead_submodule_count = ${clean.summary.forkAheadSubmoduleCount}`,
    `local_behind_fork_submodule_count = ${clean.summary.localBehindForkSubmoduleCount}`,
    `patched_submodule_count = ${clean.summary.patchedSubmoduleCount}`,
    `crates_io_checked_count = ${clean.summary.cratesIoCheckedCount}`,
    `feature_matrix_manifest_count = ${clean.summary.featureMatrixManifestCount}`,
    `total_features_counted = ${clean.summary.totalFeaturesCounted}`,
    `active_bridge_count = ${clean.summary.activeBridgeCount}`,
    `version_split_count = ${clean.summary.versionSplitCount}`,
    `installed_host_tool_count = ${clean.summary.installedHostToolCount}`,
    "",
    "[root_repo]",
    `local_sha = ${tomlStr(clean.rootRepo.localSha)}`,
    `local_commit_date = ${tomlStr(clean.rootRepo.localCommitDate)}`,
    `local_commit_subject = ${tomlStr(clean.rootRepo.localCommitSubject)}`,
    `fork_repo = ${tomlStr(clean.rootRepo.forkRepo)}`,
    `fork_sha = ${tomlStr(clean.rootRepo.forkSha)}`,
    `upstream_repo = ${tomlStr(clean.rootRepo.upstreamRepo)}`,
    `upstream_stars = ${clean.rootRepo.upstreamStars}`,
    `upstream_sha = ${tomlStr(clean.rootRepo.upstreamSha)}`,
    ...(clean.rootRepo.aheadBy !== undefined ? [`ahead_by = ${clean.rootRepo.aheadBy}`] : []),
    ...(clean.rootRepo.behindBy !== undefined ? [`behind_by = ${clean.rootRepo.behindBy}`] : []),
    `drift_status = ${tomlStr(clean.rootRepo.driftStatus)}`,
    `workspace_members_count = ${clean.rootRepo.workspaceMembersCount}`,
    `workspace_excluded_count = ${clean.rootRepo.workspaceExcludedCount}`,
    `gitmodules_count = ${clean.rootRepo.gitmodulesCount}`,
    `patch_crates_io_count = ${clean.rootRepo.patchCratesIoCount}`,
    `gpui_crate_version = ${tomlStr(clean.rootRepo.gpuiCrateVersion)}`,
    "",
  ];

  for (const s of clean.submodules) {
    lines.push("[[submodules]]");
    lines.push(`path = ${tomlStr(s.path)}`);
    lines.push(`gitmodules_url = ${tomlStr(s.gitmodulesUrl)}`);
    lines.push(`gitmodules_branch = ${tomlStr(s.gitmodulesBranch)}`);
    lines.push(`local_sha = ${tomlStr(s.localSha)}`);
    lines.push(`local_state = ${tomlStr(s.localState)}`);
    lines.push(`local_commit_date = ${tomlStr(s.localCommitDate)}`);
    lines.push(`local_commit_subject = ${tomlStr(s.localCommitSubject)}`);
    lines.push(`fork_repo = ${tomlStr(s.forkRepo)}`);
    lines.push(`fork_default_branch = ${tomlStr(s.forkDefaultBranch)}`);
    lines.push(`fork_target_branch = ${tomlStr(s.forkTargetBranch)}`);
    lines.push(`fork_sha = ${tomlStr(s.forkSha)}`);
    lines.push(`upstream_repo = ${tomlStr(s.upstreamRepo)}`);
    lines.push(`upstream_default_branch = ${tomlStr(s.upstreamDefaultBranch)}`);
    lines.push(`upstream_sha = ${tomlStr(s.upstreamSha)}`);
    lines.push(`upstream_stars = ${s.upstreamStars}`);
    lines.push(`upstream_pushed_at = ${tomlStr(s.upstreamPushedAt)}`);
    lines.push(`upstream_license = ${tomlStr(s.upstreamLicense)}`);
    if (s.rootUpstreamRepo) lines.push(`root_upstream_repo = ${tomlStr(s.rootUpstreamRepo)}`);
    if (s.rootUpstreamSha) lines.push(`root_upstream_sha = ${tomlStr(s.rootUpstreamSha)}`);
    if (s.localBehindFork !== undefined) lines.push(`local_behind_fork = ${s.localBehindFork}`);
    if (s.forkAheadUpstream !== undefined) lines.push(`fork_ahead_upstream = ${s.forkAheadUpstream}`);
    if (s.forkBehindUpstream !== undefined) lines.push(`fork_behind_upstream = ${s.forkBehindUpstream}`);
    lines.push(`drift_status = ${tomlStr(s.driftStatus)}`);
    lines.push(`patched_in_root_cargo = ${s.patchedInRootCargo}`);
    lines.push(`excluded_from_root_workspace = ${s.excludedFromRootWorkspace}`);
    lines.push(`role = ${tomlStr(s.role)}`);
    lines.push(`custom_commit_subjects = ${tomlArr(s.customCommits.map((c) => `${c.sha} ${c.subject}`))}`);
    lines.push("");
  }

  for (const c of clean.cratesIo) {
    lines.push("[[crates_io]]");
    lines.push(`name = ${tomlStr(c.name)}`);
    lines.push(`max_version = ${tomlStr(c.maxVersion)}`);
    lines.push(`max_stable_version = ${tomlStr(c.maxStableVersion)}`);
    lines.push(`downloads = ${c.downloads}`);
    lines.push(`recent_downloads = ${c.recentDownloads}`);
    lines.push(`updated_at = ${tomlStr(c.updatedAt)}`);
    lines.push(`repository = ${tomlStr(c.repository)}`);
    lines.push(`exists = ${c.exists}`);
    lines.push("");
  }

  for (const t of clean.hostTools.tools) {
    lines.push("[[host_tools]]");
    lines.push(`name = ${tomlStr(t.name)}`);
    lines.push(`command = ${tomlStr(t.command)}`);
    lines.push(`installed = ${t.installed}`);
    lines.push(`version = ${tomlStr(t.version)}`);
    lines.push("");
  }

  lines.push("[host_toolchains]");
  lines.push(`rustup_targets = ${tomlArr(clean.hostTools.rustupTargets)}`);
  lines.push(`web_build_frameworks = ${tomlArr(clean.hostTools.webBuildFrameworks)}`);
  lines.push("");

  for (const m of clean.featureMatrix) {
    lines.push("[[feature_matrix]]");
    lines.push(`manifest = ${tomlStr(m.manifestRel)}`);
    lines.push(`ecosystem = ${tomlStr(m.ecosystem)}`);
    lines.push(`package_name = ${tomlStr(m.packageName)}`);
    lines.push(`version = ${tomlStr(m.version)}`);
    if (m.edition) lines.push(`edition = ${tomlStr(m.edition)}`);
    if (m.license) lines.push(`license = ${tomlStr(m.license)}`);
    if (m.rustVersion) lines.push(`rust_version = ${tomlStr(m.rustVersion)}`);
    lines.push(`feature_count = ${m.featureCount}`);
    lines.push(`feature_names = ${tomlArr(Object.keys(m.features).sort())}`);
    lines.push(`default_features = ${tomlArr(m.features.default ?? [])}`);
    lines.push(`dependencies = ${tomlArr(m.dependencies)}`);
    lines.push(`target_dependencies = ${tomlArr(m.targetDepsSummary)}`);
    lines.push(`profiles = ${tomlArr(m.profiles)}`);
    lines.push(`role = ${tomlStr(m.role)}`);
    lines.push("");
  }

  for (const b of clean.bridges) {
    lines.push("[[integration_bridges]]");
    lines.push(`id = ${tomlStr(b.id)}`);
    lines.push(`kind = ${tomlStr(b.kind)}`);
    lines.push(`files = ${tomlArr(b.files)}`);
    lines.push(`symbols = ${tomlArr(b.symbols)}`);
    lines.push(`summary = ${tomlStr(b.summary)}`);
    lines.push("");
  }

  for (const row of clean.architectureMatrix) {
    lines.push("[[architecture_matrix]]");
    lines.push(`dimension = ${tomlStr(row.dimension)}`);
    lines.push(`title = ${tomlStr(row.title)}`);
    lines.push(`gpui_aphrody = ${tomlStr(row.gpuiAphrody)}`);
    lines.push(`dioxus_blitz_anyrender = ${tomlStr(row.dioxusBlitzAnyrender)}`);
    lines.push(`nsh_bridge = ${tomlStr(row.nshBridge)}`);
    lines.push(`unification_path = ${tomlStr(row.unificationPath)}`);
    lines.push("");
  }

  return lines.join("\n");
}

export function buildArchitectureMermaid(report: GpuiVsDioxusAuditReport): string {
  const byPath = new Map(report.submodules.map((s) => [s.path, s]));
  const lbl = (path: string, fallback: string) => {
    const s = byPath.get(path);
    if (!s) return fallback;
    const shortLocal = s.localSha.slice(0, 7) || "none";
    return `${s.path} (${shortLocal} - ${s.driftStatus})`;
  };
  const zedDelta =
    report.rootRepo.aheadBy !== null && report.rootRepo.behindBy !== null
      ? `+${report.rootRepo.aheadBy}/-${report.rootRepo.behindBy}`
      : report.rootRepo.driftStatus;

  return [
    "```mermaid",
    "flowchart LR",
    '  subgraph Upstreams["Live Upstream GitHub Repositories"]',
    `    UP_ZED["zed-industries/zed (${report.rootRepo.upstreamSha.slice(0, 7) || "main"})"]`,
    `    UP_DX["DioxusLabs/dioxus (${byPath.get("vendor/dioxus")?.upstreamSha.slice(0, 7) || "main"})"]`,
    `    UP_BLZ["DioxusLabs/blitz (${byPath.get("vendor/blitz")?.upstreamSha.slice(0, 7) || "main"})"]`,
    `    UP_ANY["DioxusLabs/anyrender (${byPath.get("vendor/anyrender")?.upstreamSha.slice(0, 7) || "main"})"]`,
    `    UP_TAFFY["DioxusLabs/taffy (${byPath.get("vendor/taffy")?.upstreamSha.slice(0, 7) || "main"})"]`,
    `    UP_WGPU["gfx-rs/wgpu (${byPath.get("vendor/wgpu")?.upstreamSha.slice(0, 7) || "trunk"})"]`,
    `    UP_VELLO["linebender/vello (${byPath.get("vendor/vello")?.upstreamSha.slice(0, 7) || "main"})"]`,
    `    UP_COSMIC["pop-os/cosmic-text (${byPath.get("vendor/cosmic-text")?.upstreamSha.slice(0, 7) || "main"})"]`,
    `    UP_A11Y["AccessKit/accesskit (${byPath.get("vendor/accesskit")?.upstreamSha.slice(0, 7) || "main"})"]`,
    `    UP_WINIT["DioxusLabs/winit (${byPath.get("vendor/winit")?.upstreamSha.slice(0, 7) || "master"})"]`,
    "  end",
    "",
    '  subgraph Forks["Aphrody Submodules (../../aphrody-labs/*.git)"]',
    `    V_DX["${lbl("vendor/dioxus", "vendor/dioxus")}"]`,
    `    V_BLZ["${lbl("vendor/blitz", "vendor/blitz")}"]`,
    `    V_ANY["${lbl("vendor/anyrender", "vendor/anyrender")}"]`,
    `    V_TAFFY["${lbl("vendor/taffy", "vendor/taffy")}"]`,
    `    V_WGPU["${lbl("vendor/wgpu", "vendor/wgpu")}"]`,
    `    V_VELLO["${lbl("vendor/vello", "vendor/vello")}"]`,
    `    V_COSMIC["${lbl("vendor/cosmic-text", "vendor/cosmic-text")}"]`,
    `    V_A11Y["${lbl("vendor/accesskit", "vendor/accesskit")}"]`,
    `    V_WINIT["${lbl("vendor/winit", "vendor/winit")}"]`,
    "  end",
    "",
    '  subgraph Workspace["Aphrody Monorepo (Aphrody-Dev/aphrody)"]',
    `    APH_ROOT["Cargo.toml (${report.rootRepo.workspaceMembersCount} members, ${zedDelta} vs zed)"]`,
    '    GPUI_CORE["crates/gpui (hot-reload: subsecond + dx-cli-config)"]',
    '    GPUI_WGPU["crates/gpui_wgpu (SharedWgpuHandles + WebGPU/Linux)"]',
    '    GPUI_UI["crates/gpui_ui + os_ui (FrameworkLayer::Blitz + nsh.rs)"]',
    '    WEB_BUILD["tooling/bun/web-build.ts (wasm_hello cohort)"]',
    "  end",
    "",
    `  UP_ZED -->|"${zedDelta}"| APH_ROOT`,
    '  UP_DX -->|"fork aphrody-labs/dioxus"| V_DX',
    '  UP_BLZ -->|"fork aphrody-labs/blitz"| V_BLZ',
    '  UP_ANY -->|"fork aphrody-labs/anyrender"| V_ANY',
    '  UP_TAFFY -->|"fork aphrody-labs/taffy"| V_TAFFY',
    '  UP_WGPU -->|"fork aphrody-labs/wgpu"| V_WGPU',
    '  UP_VELLO -->|"fork aphrody-labs/vello"| V_VELLO',
    '  UP_COSMIC -->|"fork aphrody-labs/cosmic-text"| V_COSMIC',
    '  UP_A11Y -->|"fork aphrody-labs/accesskit"| V_A11Y',
    '  UP_WINIT -->|"fork aphrody-labs/winit"| V_WINIT',
    "",
    '  V_TAFFY -->|"patch.crates-io taffy 0.15.1"| GPUI_CORE',
    '  V_WGPU -->|"patch.crates-io wgpu 30.0.0"| GPUI_WGPU',
    '  V_COSMIC -->|"patch.crates-io cosmic-text 0.19.0"| GPUI_WGPU',
    '  V_A11Y -->|"patch.crates-io accesskit 0.25.2"| GPUI_CORE',
    '  V_DX -->|"subsecond 0.7.10 + dioxus-devtools-types"| GPUI_CORE',
    '  V_BLZ -->|"blitz-dom / stylo_taffy"| V_TAFFY',
    '  V_BLZ -->|"blitz-paint -> anyrender"| V_ANY',
    '  V_ANY -->|"vello / vello_gpu"| V_VELLO',
    '  V_BLZ -->|"winit 0.31"| V_WINIT',
    '  V_BLZ -->|"examples/wasm_hello"| WEB_BUILD',
    '  GPUI_WGPU -->|"SharedWgpuHandles"| GPUI_UI',
    '  GPUI_CORE --> APH_ROOT',
    '  GPUI_UI --> APH_ROOT',
    "```",
  ].join("\n");
}

function replaceMarkerBlock(doc: string, marker: string, body: string): string {
  const begin = `<!-- BEGIN ${marker} -->`;
  const end = `<!-- END ${marker} -->`;
  const startIdx = doc.indexOf(begin);
  const endIdx = doc.indexOf(end);
  if (startIdx >= 0 && endIdx > startIdx) {
    return `${doc.slice(0, startIdx + begin.length)}\n${body}\n${doc.slice(endIdx)}`;
  }
  return doc;
}

export function renderArchitectureMarkdown(report: GpuiVsDioxusAuditReport, existingDoc?: string): string {
  const numFmt = new Intl.NumberFormat();
  const hostDx = report.hostTools.tools.find((t) => t.name === "dx");
  const metricsBlock = [
    "| Audit Dimension | Live Verified Telemetry | Evidence Anchor |",
    "| :--- | :--- | :--- |",
    `| **Aphrody Root Workspace** | \`${report.rootRepo.workspaceMembersCount}\` members, \`${report.rootRepo.workspaceExcludedCount}\` excluded, \`${report.rootRepo.gitmodulesCount}\` submodules, \`${report.rootRepo.patchCratesIoCount}\` \`[patch.crates-io]\` overrides (\`${report.rootRepo.driftStatus}\` vs \`zed-industries/zed\` ${numFmt.format(report.rootRepo.upstreamStars)}★) | [\`Cargo.toml\`](../../Cargo.toml), [\`.gitmodules\`](../../.gitmodules) |`,
    `| **GPUI Core & Subsecond Pin** | \`gpui ${report.rootRepo.gpuiCrateVersion}\` (\`${report.featureMatrix.find((m) => m.manifestRel === "crates/gpui/Cargo.toml")?.featureCount ?? 0}\` Cargo features), \`hot-reload\` pins \`subsecond =0.7.10\` matching host \`dx\` (\`${hostDx?.version ?? "missing"}\`) | [\`crates/gpui/Cargo.toml\`](../../crates/gpui/Cargo.toml), [\`crates/gpui/src/hot_reload.rs\`](../../crates/gpui/src/hot_reload.rs) |`,
    `| **Tracked UI/GPU Submodules** | \`${report.summary.submoduleCount}\` submodules audited (\`${report.summary.forkAheadSubmoduleCount}\` forks ahead of upstream, \`${report.summary.patchedSubmoduleCount}\` patched into root \`Cargo.toml\`) | [\`assets/os/gpui_vs_dioxus_matrix.toml\`](../../assets/os/gpui_vs_dioxus_matrix.toml) |`,
    `| **Cargo Feature Matrices** | \`${report.summary.featureMatrixManifestCount}\` Cargo manifests parsed (\`${report.summary.totalFeaturesCounted}\` total declared Cargo features across GPUI, NSH, Dioxus, Blitz, AnyRender, Foundation) | \`cargo metadata --no-deps --format-version 1\` |`,
    `| **Live crates.io Snapshots** | \`${report.summary.cratesIoCheckedCount}/${CRATES_IO_TARGETS.length}\` crates queried live (\`gpui\` \`${report.cratesIo.find((c) => c.name === "gpui")?.maxVersion ?? "n/a"}\`, \`dioxus\` \`${report.cratesIo.find((c) => c.name === "dioxus")?.maxVersion ?? "n/a"}\`, \`taffy\` \`${report.cratesIo.find((c) => c.name === "taffy")?.maxVersion ?? "n/a"}\`, \`wgpu\` \`${report.cratesIo.find((c) => c.name === "wgpu")?.maxVersion ?? "n/a"}\`) | \`https://crates.io/api/v1/crates/*\` |`,
    `| **Code Bridges & Version Splits** | \`${report.summary.activeBridgeCount}\` active Rust integration bridges + \`${report.summary.versionSplitCount}\` verified multi-version splits + \`${report.summary.installedHostToolCount}/${report.hostTools.tools.length}\` host CLI tools | \`target/gpui-vs-dioxus/audit.json\` |`,
  ].join("\n");

  const forkGraphBlock = buildArchitectureMermaid(report);

  const submoduleRows = report.submodules
    .map((s) => {
      const aheadBehind =
        s.forkAheadUpstream !== null && s.forkBehindUpstream !== null
          ? `+${s.forkAheadUpstream}/-${s.forkBehindUpstream}`
          : s.driftStatus;
      const rootUpNote = s.rootUpstreamRepo ? ` (root \`${s.rootUpstreamRepo}\` \`${(s.rootUpstreamSha ?? "").slice(0, 7)}\`)` : "";
      const commitSummary =
        s.customCommits.length > 0 ? s.customCommits.map((c) => `\`${c.sha}\` ${c.subject}`).slice(0, 2).join("; ") : s.localCommitSubject;
      return `| \`${s.path}\` | \`${s.localSha.slice(0, 8)}\` (\`${s.localState}\`) | \`${s.forkRepo}@${s.forkTargetBranch}\` (\`${s.forkSha.slice(0, 8)}\`) | \`${s.upstreamRepo}@${s.upstreamDefaultBranch}\` (\`${s.upstreamSha.slice(0, 8)}\`)${rootUpNote} | \`${aheadBehind}\` / \`${s.driftStatus}\` | ${s.patchedInRootCargo ? "Yes (`[patch.crates-io]`)" : "Excluded workspace"} | ${commitSummary} |`;
    })
    .join("\n");
  const submodulesBlock = [
    "| Submodule Path | Local Pin | Aphrody Fork Head | Live Upstream Head | Drift (Fork vs Upstream) | Root Cargo Wiring | Latest / Custom Delta |",
    "| :--- | :--- | :--- | :--- | :--- | :--- | :--- |",
    submoduleRows,
  ].join("\n");

  const cratesRows =
    report.cratesIo.length > 0
      ? report.cratesIo
          .map(
            (c) =>
              `| \`${c.name}\` | \`${c.maxVersion}\` | \`${c.maxStableVersion}\` | ${numFmt.format(c.downloads)} | ${numFmt.format(c.recentDownloads)} | \`${c.updatedAt.slice(0, 10) || "n/a"}\` |`,
          )
          .join("\n")
      : "| *(offline mode)* | — | — | — | — | — |";
  const cratesBlock = [
    "| Crate | Max Version (`crates.io`) | Max Stable | Total Downloads | Recent 90d | Updated |",
    "| :--- | :--- | :--- | ---: | ---: | :--- |",
    cratesRows,
  ].join("\n");

  const featureRows = report.featureMatrix
    .map((m) => {
      const featNames = Object.keys(m.features).sort();
      const preview = featNames.slice(0, 10).join(", ") + (featNames.length > 10 ? `, +${featNames.length - 10} more` : "");
      return `| \`${m.manifestRel}\` | \`${m.ecosystem}\` | \`${m.packageName}\` \`${m.version}\` | **${m.featureCount}** | \`${preview || "(none)"}\` |`;
    })
    .join("\n");
  const featuresBlock = [
    "| Manifest Path | Ecosystem | Crate & Version | Feature Count | Extracted `[features]` |",
    "| :--- | :--- | :--- | ---: | :--- |",
    featureRows,
  ].join("\n");

  const bridgeRows = report.bridges
    .map(
      (b) =>
        `| \`${b.id}\` | \`${b.kind}\` | ${b.files.map((f) => `\`${f}\``).join(", ")} | ${b.symbols.map((s) => `\`${s}\``).join(", ")} | ${b.summary} |`,
    )
    .join("\n");
  const bridgesBlock = [
    "| Bridge / Drift ID | Kind | Verified Files | Verified Symbols / Pins | Code-Verified Findings |",
    "| :--- | :--- | :--- | :--- | :--- |",
    bridgeRows,
  ].join("\n");

  const ecoRows = report.architectureMatrix
    .map(
      (r) =>
        `| **${r.title}** | ${r.gpuiAphrody} | ${r.dioxusBlitzAnyrender} | ${r.nshBridge} | ${r.unificationPath} |`,
    )
    .join("\n");
  const ecosystemBlock = [
    "| Dimension | GPUI / Aphrody | Dioxus / Blitz / AnyRender | NSH Bridge (`crates/nsh-*`) | Aphrody Unification Path |",
    "| :--- | :--- | :--- | :--- | :--- |",
    ecoRows,
  ].join("\n");

  const bySub = new Map(report.submodules.map((s) => [s.path, s]));
  const wgpuSub = bySub.get("vendor/wgpu");
  const taffySub = bySub.get("vendor/taffy");
  const splitSym = report.bridges.find((b) => b.id === "verified-version-splits")?.symbols ?? [];
  const gpuiSubVer = splitSym.find((s) => s.startsWith("gpui->subsecond")) ?? "gpui->subsecond";
  const dioxusWsSym = splitSym.find((s) => s.startsWith("vendor/dioxus")) ?? "vendor/dioxus";
  const dioxusAnySym = splitSym.find((s) => s.startsWith("dioxus->anyrender")) ?? "dioxus->anyrender";
  const blitzAnySym = splitSym.find((s) => s.startsWith("blitz->anyrender")) ?? "blitz->anyrender";
  const taffySym = splitSym.find((s) => s.startsWith("taffy(")) ?? "taffy";

  const wgpuLocalShort = wgpuSub?.localSha.slice(0, 8) || "local";
  const wgpuUpShort = wgpuSub?.upstreamSha.slice(0, 8) || "upstream";
  const wgpuDrift = wgpuSub?.driftStatus ?? "diverged";
  const taffyDrift = taffySub?.driftStatus ?? "ahead";

  const unificationBlock = [
    `1. **Subsecond Version Pin Alignment**: [\`crates/gpui/Cargo.toml\`](../../crates/gpui/Cargo.toml) pins \`subsecond\`, \`dioxus-cli-config\`, and \`dioxus-devtools-types\` (\`${gpuiSubVer}\`, matching installed host CLI \`${hostDx?.version ?? "dx"}\`), whereas [\`vendor/dioxus/Cargo.toml\`](../../vendor/dioxus/Cargo.toml) tracks \`${dioxusWsSym}\`. Keep the \`=0.7.10\` pin while host \`dx\` is \`0.7.10\`, and bump in lockstep when \`0.8.0\` ships.`,
    `2. **Blitz / AnyRender / Dioxus Convergence**: [\`vendor/dioxus/Cargo.toml\`](../../vendor/dioxus/Cargo.toml) pins \`${dioxusAnySym}\` + \`anyrender_vello_hybrid\` + \`vello 0.11\` + \`winit =0.31.0-beta.3\`, whereas [\`vendor/blitz/Cargo.toml\`](../../vendor/blitz/Cargo.toml) and [\`vendor/anyrender/Cargo.toml\`](../../vendor/anyrender/Cargo.toml) upgraded to \`${blitzAnySym}\` + \`anyrender_vello_gpu\` + \`vello 0.12\` + \`winit =0.31.0-beta.4\`. Both workspaces stay isolated under \`[workspace.exclude]\` in root [\`Cargo.toml\`](../../Cargo.toml).`,
    `3. **WGPU Shared Device Composition**: [\`crates/gpui_wgpu/src/wgpu_context.rs\`](../../crates/gpui_wgpu/src/wgpu_context.rs) exposes \`WgpuContext::shared_handles()\` on vendored \`vendor/wgpu\` (\`${wgpuLocalShort}\` on \`aphrody-labs/wgpu@${wgpuSub?.forkTargetBranch ?? "main"}\`, \`${wgpuDrift}\` vs upstream \`${wgpuSub?.upstreamDefaultBranch ?? "trunk"}\` \`${wgpuUpShort}\`), enabling zero-copy AnyRender/Vello surface embedding inside GPUI.`,
    `4. **Taffy Single-Source Patch**: [\`vendor/taffy/Cargo.toml\`](../../vendor/taffy/Cargo.toml) carries \`${taffySym}\` (\`${taffyDrift}\` vs \`DioxusLabs/taffy\`) so root \`[patch.crates-io]\` outranks \`crates.io\`, feeding GPUI ([\`crates/gpui/src/taffy.rs\`](../../crates/gpui/src/taffy.rs)), NSH ([\`crates/nsh-core/Cargo.toml\`](../../crates/nsh-core/Cargo.toml)), and Blitz ([\`vendor/blitz/packages/blitz-dom/Cargo.toml\`](../../vendor/blitz/packages/blitz-dom/Cargo.toml)) from one source.`,
  ].join("\n");

  if (existingDoc) {
    let doc = existingDoc;
    doc = replaceMarkerBlock(doc, "GPUI_DIOXUS_METRICS", metricsBlock);
    doc = replaceMarkerBlock(doc, "GPUI_DIOXUS_FORK_GRAPH", forkGraphBlock);
    doc = replaceMarkerBlock(doc, "GPUI_DIOXUS_SUBMODULES", submodulesBlock);
    doc = replaceMarkerBlock(doc, "GPUI_DIOXUS_CRATES_IO", cratesBlock);
    doc = replaceMarkerBlock(doc, "GPUI_DIOXUS_FEATURES", featuresBlock);
    doc = replaceMarkerBlock(doc, "GPUI_DIOXUS_BRIDGES", bridgesBlock);
    doc = replaceMarkerBlock(doc, "GPUI_DIOXUS_ECOSYSTEM_MATRIX", ecosystemBlock);
    doc = replaceMarkerBlock(doc, "GPUI_DIOXUS_UNIFICATION", unificationBlock);
    return doc;
  }

  return `# GPUI vs Dioxus / Blitz / AnyRender Architecture & Fork Audit

> Live code-verified architecture graph, submodule fork telemetry, Cargo feature matrices, \`crates.io\` snapshots, and integration bridge verification for [\`Aphrody-Dev/aphrody\`](https://github.com/Aphrody-Dev/aphrody) across [\`crates/gpui\`](../../crates/gpui/Cargo.toml), [\`vendor/dioxus\`](../../vendor/dioxus/Cargo.toml), [\`vendor/blitz\`](../../vendor/blitz/Cargo.toml), and [\`vendor/anyrender\`](../../vendor/anyrender/Cargo.toml).

---

## 1. Automated Audit & Verification CLI

All telemetry in [\`assets/os/gpui_vs_dioxus_matrix.toml\`](../../assets/os/gpui_vs_dioxus_matrix.toml), \`target/gpui-vs-dioxus/audit.json\`, and the generated sections below is computed by [\`tooling/bun/gpui-vs-dioxus-audit.ts\`](../../tooling/bun/gpui-vs-dioxus-audit.ts) and verified by [\`tooling/bun/gpui-vs-dioxus-audit.test.ts\`](../../tooling/bun/gpui-vs-dioxus-audit.test.ts):

\`\`\`bash
bun tooling/bun/gpui-vs-dioxus-audit.ts audit   # Live submodule + fork + cargo metadata + crates.io + bridge scan -> target/gpui-vs-dioxus/audit.json
bun tooling/bun/gpui-vs-dioxus-audit.ts emit    # Regenerate assets/os/gpui_vs_dioxus_matrix.toml and synchronized doc sections
bun tooling/bun/gpui-vs-dioxus-audit.ts all     # Full audit + emit refresh
bun test tooling/bun/gpui-vs-dioxus-audit.test.ts
\`\`\`

### Live Verified Telemetry Snapshot

<!-- BEGIN GPUI_DIOXUS_METRICS -->
${metricsBlock}
<!-- END GPUI_DIOXUS_METRICS -->

---

## 2. Live Fork & Workspace Architecture Graph

<!-- BEGIN GPUI_DIOXUS_FORK_GRAPH -->
${forkGraphBlock}
<!-- END GPUI_DIOXUS_FORK_GRAPH -->

---

## 3. Submodule & Fork Drift Audit (\`.gitmodules\` vs \`aphrody-labs/*\` vs Upstream)

<!-- BEGIN GPUI_DIOXUS_SUBMODULES -->
${submodulesBlock}
<!-- END GPUI_DIOXUS_SUBMODULES -->

---

## 4. Live \`crates.io\` Upstream Ecosystem Snapshot

<!-- BEGIN GPUI_DIOXUS_CRATES_IO -->
${cratesBlock}
<!-- END GPUI_DIOXUS_CRATES_IO -->

---

## 5. Exact Extracted Cargo Feature Matrices

Extracted directly from root \`cargo metadata --no-deps --format-version 1\` and live \`Cargo.toml\` manifests ([\`crates/gpui/Cargo.toml\`](../../crates/gpui/Cargo.toml), [\`crates/gpui_wgpu/Cargo.toml\`](../../crates/gpui_wgpu/Cargo.toml), [\`crates/gpui_web/Cargo.toml\`](../../crates/gpui_web/Cargo.toml), [\`crates/gpui_ui/Cargo.toml\`](../../crates/gpui_ui/Cargo.toml), [\`vendor/dioxus/Cargo.toml\`](../../vendor/dioxus/Cargo.toml), [\`vendor/dioxus/packages/dioxus/Cargo.toml\`](../../vendor/dioxus/packages/dioxus/Cargo.toml), [\`vendor/blitz/Cargo.toml\`](../../vendor/blitz/Cargo.toml), and companions):

<!-- BEGIN GPUI_DIOXUS_FEATURES -->
${featuresBlock}
<!-- END GPUI_DIOXUS_FEATURES -->

---

## 6. Code-Verified Integration Bridges & Manifest Version Splits

Verified by scanning \`crates/\` plus [\`tooling/bun/web-build.ts\`](../../tooling/bun/web-build.ts), [\`crates/gpui/src/hot_reload.rs\`](../../crates/gpui/src/hot_reload.rs), [\`crates/gpui/src/view.rs\`](../../crates/gpui/src/view.rs), [\`crates/gpui/build.rs\`](../../crates/gpui/build.rs), [\`crates/gpui_wgpu/src/wgpu_context.rs\`](../../crates/gpui_wgpu/src/wgpu_context.rs), [\`crates/os_ui/src/unified_runtime.rs\`](../../crates/os_ui/src/unified_runtime.rs), and [\`crates/os_desktop/tests/visual_smoke.rs\`](../../crates/os_desktop/tests/visual_smoke.rs):

<!-- BEGIN GPUI_DIOXUS_BRIDGES -->
${bridgesBlock}
<!-- END GPUI_DIOXUS_BRIDGES -->

---

## 7. 8-Dimension Architecture & Tooling Comparison

<!-- BEGIN GPUI_DIOXUS_ECOSYSTEM_MATRIX -->
${ecosystemBlock}
<!-- END GPUI_DIOXUS_ECOSYSTEM_MATRIX -->

---

## 8. Host Tooling & Unification Action Plan

Host tools probed: ${report.hostTools.tools.map((t) => `\`${t.name}\` (\`${t.version}\`)`).join(", ")}. Installed \`rustup\` targets: ${report.hostTools.rustupTargets.map((t) => `\`${t}\``).join(", ")}. Unified Wasm cohort specs in [\`tooling/bun/web-build.ts\`](../../tooling/bun/web-build.ts): ${report.hostTools.webBuildFrameworks.map((w) => `\`${w}\``).join(", ")}.

<!-- BEGIN GPUI_DIOXUS_UNIFICATION -->
${unificationBlock}
<!-- END GPUI_DIOXUS_UNIFICATION -->
`;
}

export function emitArtifacts(report: GpuiVsDioxusAuditReport, root = REPO_ROOT): {
  tomlPath: string;
  docPath: string;
} {
  const tomlPath = join(root, MATRIX_TOML_REL);
  mkdirSync(resolve(tomlPath, ".."), { recursive: true });
  writeFileSync(tomlPath, serializeMatrixToml(report), "utf8");

  const docPath = join(root, ARCH_DOC_REL);
  mkdirSync(resolve(docPath, ".."), { recursive: true });
  const existingDoc = existsSync(docPath) ? readFileSync(docPath, "utf8") : undefined;
  writeFileSync(docPath, renderArchitectureMarkdown(report, existingDoc), "utf8");

  return { tomlPath, docPath };
}

// ---------------------------------------------------------------------------
// CLI Entrypoint
// ---------------------------------------------------------------------------

if (import.meta.main) {
  const args = Bun.argv.slice(2);
  const cmd = args[0] ?? "all";
  const offline = args.includes("--offline");
  const jsonFlag = args.includes("--json");
  const auditPath = join(REPO_ROOT, AUDIT_JSON_REL);

  const writeAudit = (report: GpuiVsDioxusAuditReport) => {
    mkdirSync(resolve(auditPath, ".."), { recursive: true });
    writeFileSync(auditPath, JSON.stringify(report, null, 2), "utf8");
  };

  if (cmd === "audit") {
    const report = await runAudit({ root: REPO_ROOT, offline });
    writeAudit(report);
    console.log(
      JSON.stringify(
        jsonFlag ? report : { auditPath: AUDIT_JSON_REL, summary: report.summary, rootRepo: report.rootRepo },
        null,
        2,
      ),
    );
  } else if (cmd === "emit" || cmd === "all") {
    let report: GpuiVsDioxusAuditReport;
    if (cmd === "emit" && existsSync(auditPath) && statSync(auditPath).isFile()) {
      report = JSON.parse(readFileSync(auditPath, "utf8")) as GpuiVsDioxusAuditReport;
    } else {
      report = await runAudit({ root: REPO_ROOT, offline });
      writeAudit(report);
    }
    const emitted = emitArtifacts(report, REPO_ROOT);
    console.log(
      JSON.stringify(
        {
          auditPath: AUDIT_JSON_REL,
          toml: relative(REPO_ROOT, emitted.tomlPath).replace(/\\/g, "/"),
          doc: relative(REPO_ROOT, emitted.docPath).replace(/\\/g, "/"),
          summary: report.summary,
        },
        null,
        2,
      ),
    );
  } else {
    console.error("Usage: bun tooling/bun/gpui-vs-dioxus-audit.ts [audit|emit|all] [--offline] [--json]");
    process.exit(1);
  }
}
