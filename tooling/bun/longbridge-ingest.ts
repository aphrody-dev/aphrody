#!/usr/bin/env bun
/**
 * Reproducible, real-time Longbridge + Zed + Community GPUI Ingestion, Deduplication,
 * Graphify, Feature-Matrix, Centralization Audit, Registry & Sitemap/Screenshot Scanner.
 *
 * Usage:
 *   bun tooling/bun/longbridge-ingest.ts scan [--json]
 *   bun tooling/bun/longbridge-ingest.ts graph [--json]
 *   bun tooling/bun/longbridge-ingest.ts centralize [--json]
 *   bun tooling/bun/longbridge-ingest.ts sitemaps [--json]
 *   bun tooling/bun/longbridge-ingest.ts screenshots [--json]
 *   bun tooling/bun/longbridge-ingest.ts all [--json]
 */
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, relative, resolve } from "node:path";
import { parseGitmodules, type SubmoduleEntry } from "./submodules.ts";

export const REPO_ROOT = resolve(import.meta.dir, "../..");
export const INVENTORY_TOML_REL = "assets/os/longbridge_gpui_inventory.toml";
export const ARCH_DOC_REL = "aphrody/docs/architecture/longbridge-gpui-unification.md";
export const GRAPH_OUT_REL = "target/longbridge-ingest/graph.json";
export const SCAN_OUT_REL = "target/longbridge-ingest/scan.json";
export const CENTRALIZE_OUT_REL = "target/longbridge-ingest/centralize.json";
export const SITEMAPS_OUT_REL = "target/longbridge-ingest/sitemaps.json";
export const SCREENSHOTS_OUT_REL = "target/longbridge-ingest/screenshots.json";
export const SCREENSHOTS_DIR_REL = "target/longbridge-screenshots";

const USER_AGENT = "AphrodyLongbridgeIngest/1.0 (+https://github.com/aphrody-dev/aphrody)";

export type RepoClassification =
  | "ABSORBED_FLATTENED"
  | "MERGE_CANDIDATE"
  | "REFERENCE_BENCH_SHOWCASE"
  | "INFRA_OUT_OF_SCOPE";

export interface GitHubRepoSummary {
  name: string;
  fullName: string;
  owner: string;
  description: string;
  homepage: string;
  htmlUrl: string;
  language: string;
  stars: number;
  forks: number;
  openIssuesAndPrs: number;
  archived: boolean;
  isFork: boolean;
  defaultBranch: string;
  pushedAt: string;
  topics: string[];
}

export interface CargoManifestSummary {
  packageName?: string;
  version?: string;
  edition?: string;
  license?: string;
  workspaceMembers: string[];
  features: Record<string, string[]>;
  dependencies: string[];
}

export interface PackageJsonSummary {
  name?: string;
  version?: string;
  scripts: string[];
  dependencies: string[];
  devDependencies: string[];
}

export interface CratesIoMeta {
  name: string;
  maxVersion: string;
  downloads: number;
  recentDownloads: number;
  updatedAt: string;
  repository: string;
  homepage: string;
  exists: boolean;
  error?: string;
}

export interface NpmMeta {
  name: string;
  latestVersion: string;
  modified: string;
  repository: string;
  homepage: string;
  exists: boolean;
  error?: string;
}

export interface EnrichedRepoDetails {
  nameWithOwner: string;
  openPRs: number;
  openIssues: number;
  latestCommitSha: string;
  latestCommitDate: string;
  latestCommitMessage: string;
  activeBranches: string[];
  totalBranchCount: number;
  cargoManifest?: CargoManifestSummary;
  subCargoManifest?: CargoManifestSummary;
  subCargoPath?: string;
  packageJson?: PackageJsonSummary;
  cratesIo: CratesIoMeta[];
  npm: NpmMeta[];
}

export interface ClassifiedRepo extends GitHubRepoSummary {
  sourceOrg: "longbridge" | "zed-upstream" | "community-gpui";
  classification: RepoClassification;
  category: string;
  relevanceScore: number;
  localMatches: string[];
  targetCrates: string[];
  rationale: string;
  enriched?: EnrichedRepoDetails;
}

export interface LocalAphrodyIndex {
  gitmodules: SubmoduleEntry[];
  vendorDirs: string[];
  crateDirs: string[];
  workspaceMembers: string[];
  workspaceExcluded: string[];
  workspaceDependencies: Record<string, unknown>;
  patchCratesIo: Record<string, unknown>;
  packageMeta: Record<string, unknown>;
}

export interface NonWorkspaceDepFinding {
  crateName: string;
  manifestPath: string;
  section: string;
  scope: "top" | "target";
  depKey: string;
  actualPkgName: string;
  specSummary: string;
  inRootWorkspaceDeps: boolean;
}

export interface WorkspaceDepEntry {
  key: string;
  packageName: string;
  kind: "path" | "git" | "registry";
  sourceDetail: string;
}

export interface DuplicatePackageEntry {
  name: string;
  versions: string[];
  majorMinorVersions: string[];
  hasMajorMinorSplit: boolean;
  instanceCount: number;
  sources: string[];
  dependentsSample: string[];
}

export interface CargoCentralizationAudit {
  generatedAt: string;
  workspaceMemberCount: number;
  auditedMemberCount: number;
  extensionCountSkipped: number;
  missingWorkspaceLints: string[];
  nonWorkspaceDependencies: NonWorkspaceDepFinding[];
  nonWorkspaceTopLevelCount: number;
  nonWorkspaceTargetLevelCount: number;
  uniqueNonWorkspaceDepNames: string[];
  workspaceDependencyBreakdown: {
    total: number;
    pathCount: number;
    gitCount: number;
    registryCount: number;
    gitEntries: WorkspaceDepEntry[];
    pathEntriesSample: WorkspaceDepEntry[];
  };
  patchBreakdown: {
    total: number;
    pathCount: number;
    gitCount: number;
    entries: Array<{ name: string; kind: "path" | "git"; detail: string }>;
  };
  duplicatePackages: DuplicatePackageEntry[];
  duplicatePackageCount: number;
  duplicateMajorMinorCount: number;
}

export interface CrateFeatureEntry {
  name: string;
  version: string;
  path: string;
  license: string;
  usesWorkspaceLints: boolean;
  featureCount: number;
  features: Record<string, string[]>;
  directWorkspaceDeps: string[];
  nonWorkspaceDepCount: number;
}

export interface GraphifyNode {
  id: string;
  kind: "upstream_repo" | "workspace_crate" | "vendor_submodule" | "website_target";
  label: string;
  classification?: RepoClassification;
  path?: string;
  version?: string;
  stars?: number;
  featureCount?: number;
  url?: string;
}

export interface GraphifyEdge {
  id: string;
  from: string;
  to: string;
  kind: "ABSORBED_INTO" | "MERGE_TARGET" | "PATCHED_VIA" | "DEPENDS_ON" | "BENCHMARKS" | "DOCS_SITE";
  note?: string;
}

export interface GraphifyDocument {
  schemaVersion: number;
  generatedAt: string;
  summary: {
    longbridgeRepoCount: number;
    absorbedCount: number;
    mergeCandidateCount: number;
    referenceBenchShowcaseCount: number;
    infraOutOfScopeCount: number;
    communityGpuiCount: number;
    workspaceCrateCount: number;
    featureMatrixCrateCount: number;
    nonWorkspaceDepCount: number;
    uniqueNonWorkspaceDepCount: number;
    missingWorkspaceLintsCount: number;
    duplicateGraphCrateCount: number;
    duplicateMajorMinorCount: number;
    sitemapTargetsCount: number;
    sitemapTotalUrls: number;
    screenshotCount: number;
    screenshotSuccessCount: number;
  };
  nodes: GraphifyNode[];
  edges: GraphifyEdge[];
  duplicatePackages: DuplicatePackageEntry[];
  mergeCandidates: Array<{
    fullName: string;
    name: string;
    stars: number;
    language: string;
    homepage: string;
    targetCrates: string[];
    rationale: string;
    openPRs?: number;
    openIssues?: number;
    latestCommitSha?: string;
    latestCommitDate?: string;
    activeBranches?: string[];
    cratesIo?: CratesIoMeta[];
    features?: Record<string, string[]>;
  }>;
  featureMatrix: Record<string, CrateFeatureEntry>;
  centralizationAudit: CargoCentralizationAudit;
}

export interface SitemapCrawlResult {
  id: string;
  url: string;
  finalUrl: string;
  origin: string;
  httpStatus: number;
  title: string;
  byteLength: number;
  sitemapUrl: string;
  sitemapStatus: number;
  sitemapUrls: string[];
  sitemapUrlCount: number;
  internalLinks: string[];
  internalLinkCount: number;
  totalDiscoveredUrls: number;
  status: "ok" | "error";
  error?: string;
}

export interface ScreenshotTarget {
  id: string;
  url: string;
  filename: string;
  description: string;
}

export interface ScreenshotResult {
  id: string;
  url: string;
  finalUrl: string;
  outputPath: string;
  httpStatus?: number;
  title: string;
  status: "ok" | "fallback_error";
  byteLength: number;
  width: number;
  height: number;
  validPng: boolean;
  error?: string;
}

export interface ScanReport {
  schemaVersion: number;
  generatedAt: string;
  longbridgeTotalRepos: number;
  countsByClassification: Record<RepoClassification, number>;
  localIndexSummary: {
    gitmodulesCount: number;
    vendorDirsCount: number;
    crateDirsCount: number;
    workspaceMembersCount: number;
    workspaceDepsCount: number;
    patchCount: number;
  };
  longbridgeRepos: ClassifiedRepo[];
  upstreamZed: ClassifiedRepo;
  communityGpuiRepos: ClassifiedRepo[];
}

interface CoreRepoSpec {
  owner: string;
  name: string;
  subManifestPath?: string;
  cratesIoNames: string[];
  npmNames: string[];
}

export const CORE_ENRICHED_SPECS: CoreRepoSpec[] = [
  { owner: "zed-industries", name: "zed", subManifestPath: "crates/gpui/Cargo.toml", cratesIoNames: ["gpui"], npmNames: [] },
  {
    owner: "longbridge",
    name: "gpui-kit",
    subManifestPath: "crates/kit/Cargo.toml",
    cratesIoNames: ["gpui-kit", "gpui-component", "gpui-base", "gpui-shell"],
    npmNames: [],
  },
  {
    owner: "longbridge",
    name: "nsh",
    subManifestPath: "crates/nsh-core/Cargo.toml",
    cratesIoNames: ["nsh-core", "nsh-base", "nsh-engine", "nsh-js"],
    npmNames: ["nsh"],
  },
  { owner: "longbridge", name: "gpui-fast", subManifestPath: "crates/gpui/Cargo.toml", cratesIoNames: [], npmNames: [] },
  { owner: "longbridge", name: "gpui-mobile", cratesIoNames: ["gpui-pre-mobile"], npmNames: [] },
  { owner: "longbridge", name: "wef", subManifestPath: "crates/wef/Cargo.toml", cratesIoNames: ["wef", "gpui-webview"], npmNames: [] },
  { owner: "longbridge", name: "quickjs-jit", cratesIoNames: ["quickjs-jit", "quickjs-jit-stdlib"], npmNames: [] },
  { owner: "longbridge", name: "nsh-perf-bench", subManifestPath: "core/Cargo.toml", cratesIoNames: [], npmNames: [] },
  { owner: "longbridge", name: "gpui-kit-showcases", cratesIoNames: [], npmNames: [] },
  { owner: "longbridge", name: "nsh-showcases", cratesIoNames: [], npmNames: [] },
  { owner: "longbridge", name: "rust-i18n", cratesIoNames: ["rust-i18n", "rust-i18n-support"], npmNames: [] },
  { owner: "longbridge", name: "longbridge-mcp", cratesIoNames: ["longbridge-mcp"], npmNames: [] },
  { owner: "longbridge", name: "longbridge-ai-acp", cratesIoNames: ["longbridge-ai-acp"], npmNames: [] },
  { owner: "longbridge", name: "longbridge-terminal", cratesIoNames: ["longbridge-terminal"], npmNames: [] },
  { owner: "longbridge", name: "purview", subManifestPath: "purview/Cargo.toml", cratesIoNames: ["purview", "purview-gpui"], npmNames: [] },
  { owner: "longbridge", name: "nsh-compat", cratesIoNames: [], npmNames: [] },
  { owner: "longbridge", name: "native-shell-v1", subManifestPath: "crates/core/Cargo.toml", cratesIoNames: [], npmNames: [] },
];

export const DEFAULT_WEBSITE_TARGETS: ScreenshotTarget[] = [
  { id: "nsh-home", url: "https://nsh.dev/", filename: "nsh-home.png", description: "Longbridge Native Shell (NSH) homepage" },
  { id: "nsh-apps", url: "https://nsh.dev/apps", filename: "nsh-apps.png", description: "Longbridge NSH showcase applications" },
  { id: "gpui-kit-home", url: "https://gpui-kit.com/", filename: "gpui-kit-home.png", description: "Longbridge GPUI Kit homepage" },
  { id: "gpui-kit-apps", url: "https://gpui-kit.com/apps/", filename: "gpui-kit-apps.png", description: "Longbridge GPUI Kit showcase apps" },
  { id: "zed-home", url: "https://zed.dev/", filename: "zed-home.png", description: "Upstream Zed / GPUI homepage" },
  { id: "purview-docs", url: "https://longbridge.github.io/purview/", filename: "purview-docs.png", description: "Longbridge Purview GPUI MCP bridge docs" },
];

// ---------------------------------------------------------------------------
// Generic Utilities
// ---------------------------------------------------------------------------

export async function mapWithConcurrency<T, R>(
  items: readonly T[],
  limit: number,
  fn: (item: T, index: number) => Promise<R>,
): Promise<R[]> {
  const safeLimit = Math.max(1, Math.floor(limit));
  const results = new Array<R>(items.length);
  let cursor = 0;

  async function worker(): Promise<void> {
    while (cursor < items.length) {
      const idx = cursor++;
      results[idx] = await fn(items[idx], idx);
    }
  }

  const workers = Array.from({ length: Math.min(safeLimit, items.length) }, () => worker());
  await Promise.all(workers);
  return results;
}

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

export function parsePaginatedJson<T>(raw: string): T[] {
  const trimmed = raw.trim();
  if (!trimmed) return [];
  try {
    const parsed = JSON.parse(trimmed) as unknown;
    if (Array.isArray(parsed)) {
      return parsed.flatMap((entry) => (Array.isArray(entry) ? (entry as T[]) : [entry as T]));
    }
    return [parsed as T];
  } catch {
    // Handle concatenated JSON arrays/objects emitted by `gh api --paginate` without `--slurp`.
    const normalized = `[${trimmed.replace(/\]\s*\[/g, ",").replace(/\}\s*\{/g, "},{")}]`;
    const parsed = JSON.parse(normalized) as unknown[];
    return parsed.flatMap((entry) => (Array.isArray(entry) ? (entry as T[]) : [entry as T]));
  }
}

export function normalizeVersionMajorMinor(version: string): string {
  const core = version.split(/[-+]/)[0] ?? version;
  const parts = core.split(".");
  if (parts.length >= 2) {
    return `${parts[0]}.${parts[1]}`;
  }
  return core;
}

export function extractCargoManifestSummary(tomlText: string): CargoManifestSummary {
  const parsed = Bun.TOML.parse(tomlText) as Record<string, any>;
  const pkg = (parsed.package ?? {}) as Record<string, any>;
  const ws = (parsed.workspace ?? {}) as Record<string, any>;
  const rawFeatures = (parsed.features ?? {}) as Record<string, unknown>;
  const features: Record<string, string[]> = {};
  for (const [key, val] of Object.entries(rawFeatures)) {
    features[key] = Array.isArray(val) ? val.map(String) : [];
  }
  const deps = Object.keys((parsed.dependencies ?? {}) as Record<string, unknown>);
  return {
    packageName: typeof pkg.name === "string" ? pkg.name : undefined,
    version: typeof pkg.version === "string" ? pkg.version : undefined,
    edition: typeof pkg.edition === "string" ? pkg.edition : undefined,
    license: typeof pkg.license === "string" ? pkg.license : undefined,
    workspaceMembers: Array.isArray(ws.members) ? ws.members.map(String) : [],
    features,
    dependencies: deps,
  };
}

export function extractPackageJsonSummary(jsonText: string): PackageJsonSummary {
  const parsed = JSON.parse(jsonText) as Record<string, any>;
  return {
    name: typeof parsed.name === "string" ? parsed.name : undefined,
    version: typeof parsed.version === "string" ? parsed.version : undefined,
    scripts: Object.keys((parsed.scripts ?? {}) as Record<string, unknown>),
    dependencies: Object.keys((parsed.dependencies ?? {}) as Record<string, unknown>),
    devDependencies: Object.keys((parsed.devDependencies ?? {}) as Record<string, unknown>),
  };
}

// ---------------------------------------------------------------------------
// Local Aphrody Workspace Index & Deduplication
// ---------------------------------------------------------------------------

export function loadLocalAphrodyIndex(root = REPO_ROOT): LocalAphrodyIndex {
  const gitmodulesPath = join(root, ".gitmodules");
  const gitmodules = existsSync(gitmodulesPath) ? parseGitmodules(readFileSync(gitmodulesPath, "utf8")) : [];

  const vendorRoot = join(root, "vendor");
  const vendorDirs = existsSync(vendorRoot)
    ? readdirSync(vendorRoot, { withFileTypes: true })
        .filter((d) => d.isDirectory())
        .map((d) => d.name)
        .sort()
    : [];

  const cratesRoot = join(root, "crates");
  const crateDirs = existsSync(cratesRoot)
    ? readdirSync(cratesRoot, { withFileTypes: true })
        .filter((d) => d.isDirectory())
        .map((d) => d.name)
        .sort()
    : [];

  const rootTomlRaw = readFileSync(join(root, "Cargo.toml"), "utf8");
  const rootToml = Bun.TOML.parse(rootTomlRaw) as Record<string, any>;
  const ws = (rootToml.workspace ?? {}) as Record<string, any>;
  const patchSection = (rootToml.patch ?? {}) as Record<string, any>;

  return {
    gitmodules,
    vendorDirs,
    crateDirs,
    workspaceMembers: Array.isArray(ws.members) ? ws.members.map(String) : [],
    workspaceExcluded: Array.isArray(ws.exclude) ? ws.exclude.map(String) : [],
    workspaceDependencies: (ws.dependencies ?? {}) as Record<string, unknown>,
    patchCratesIo: (patchSection["crates-io"] ?? {}) as Record<string, unknown>,
    packageMeta: (ws.package ?? {}) as Record<string, unknown>,
  };
}

const KNOWN_ABSORBED_SPECS: Record<
  string,
  { category: string; targets: string[]; rationale: string }
> = {
  "zed-industries/zed": {
    category: "core-gpui-editor",
    targets: ["crates/gpui", "crates/gpui_wgpu", "crates/gpui_web", "crates/ui", "crates/editor", "crates/zed"],
    rationale: "Upstream GPUI + Zed workspace flattened directly into Aphrody crates/* with unified Cargo workspace.",
  },
  "longbridge/nsh": {
    category: "nsh-runtime",
    targets: ["crates/nsh-core", "crates/nsh-base", "crates/nsh-engine", "crates/nsh-js", "crates/nsh-platform-gpui", "crates/aphrody_nsh", "crates/gpui_ui/src/nsh.rs"],
    rationale: "Longbridge Native Shell (v0.5.0) already flattened into crates/nsh-* and bridged into gpui_ui + gpui_app_shell.",
  },
  "longbridge/native-shell-v1": {
    category: "nsh-legacy",
    targets: ["crates/nsh-core", "crates/nsh-engine"],
    rationale: "Gen-1 Native Shell superseded and absorbed by crates/nsh-* v0.5.0.",
  },
  "longbridge/quickjs-jit": {
    category: "js-runtime",
    targets: ["Cargo.toml#rquickjs", "Cargo.toml#quickjs-jit-stdlib", "crates/nsh-js"],
    rationale: "Centralized in root [workspace.dependencies] as rquickjs (quickjs-jit =0.14.2) + quickjs-jit-stdlib for crates/nsh-js.",
  },
  "longbridge/rust-i18n": {
    category: "i18n",
    targets: ["Cargo.toml#rust-i18n", "Cargo.toml#rust-i18n-support", "crates/nsh-i18n", "crates/nsh-core"],
    rationale: "Centralized in root [workspace.dependencies] (rust-i18n 4.2.4) and wired into crates/nsh-i18n + nsh-core.",
  },
  "longbridge/wry": {
    category: "webview",
    targets: ["vendor/tauri/wry", "Cargo.toml#wry", "crates/nsh-platform-gpui"],
    rationale: "Deduplicated onto single relative submodule vendor/tauri/wry and root Cargo.toml workspace dependency.",
  },
};

const KNOWN_MERGE_CANDIDATES: Record<
  string,
  { category: string; targets: string[]; score: number; rationale: string }
> = {
  "longbridge/gpui-kit": {
    category: "gpui-ui-components",
    targets: ["crates/ui", "crates/gpui_ui", "crates/aphrody_ui"],
    score: 98,
    rationale: "60+ desktop GPUI widgets, virtualized tables, dock, webview, fps monitor, and gpui-component/gpui-kit ecosystem alignment.",
  },
  "longbridge/gpui-fast": {
    category: "gpui-layout-perf",
    targets: ["crates/gpui/src/taffy.rs", "crates/gpui", "tooling/perf"],
    score: 97,
    rationale: "Incremental dirty-tracking TaffyTree layout engine rework + gpui_perf; local crates/gpui/src/taffy.rs currently clears TaffyTree every frame.",
  },
  "longbridge/gpui-mobile": {
    category: "gpui-mobile",
    targets: ["crates/gpui_ios", "crates/nsh-platform-android", "crates/gpui_platform"],
    score: 94,
    rationale: "Android Vulkan + iOS Metal gpui-pre-mobile with 31 mobile hardware capability features missing from local gpui_ios.",
  },
  "longbridge/wef": {
    category: "gpui-cef-webview",
    targets: ["crates/nsh-platform-gpui", "vendor/tauri/cef-rs", "crates/aphrody_desktop"],
    score: 91,
    rationale: "Offscreen CEF3 gpui-webview + cargo-wef complements local wry and vendor/tauri/cef-rs without OS WebView inconsistencies.",
  },
  "longbridge/purview": {
    category: "gpui-mcp-inspection",
    targets: ["crates/aphrody_craft", "crates/inspector_ui", "crates/gpui_ui"],
    score: 90,
    rationale: "Live multi-window GPUI projection & rmcp automation bridge (purview + purview-gpui) for AI-native app control.",
  },
  "longbridge/longbridge-ai-acp": {
    category: "ai-acp",
    targets: ["crates/acp_thread", "crates/acp_tools"],
    score: 84,
    rationale: "Embeddable ACP client/auth bridge complements Aphrody agent-client-protocol integration.",
  },
  "longbridge/longbridge-mcp": {
    category: "ai-mcp",
    targets: ["crates/aphrody_craft", "crates/context_server"],
    score: 82,
    rationale: "Production rmcp streamable-HTTP + stdio MCP server patterns relevant to aphrody_craft MCP hardening.",
  },
  "longbridge/longbridge-terminal": {
    category: "tui-ai-cli",
    targets: ["crates/nsh-platform-tui", "crates/terminal_view"],
    score: 80,
    rationale: "AI-native Ratatui terminal interface and split-debuginfo release profile patterns.",
  },
  "longbridge/autocorrect": {
    category: "typography-linter",
    targets: ["crates/editor", "crates/nsh-base", "docs/"],
    score: 76,
    rationale: "CJK/English mixed typography spacing & linter useful for multilingual docs and rich text editing.",
  },
  "longbridge/open-pencil": {
    category: "design-canvas",
    targets: ["crates/aphrody_desktop", "crates/aphrody_craft"],
    score: 75,
    rationale: "AI-native Figma/design canvas concepts aligned with FigCraft studio in assets/os/craft_catalog.toml.",
  },
  "longbridge/ri": {
    category: "ai-agent-core",
    targets: ["crates/agent", "crates/language_model"],
    score: 72,
    rationale: "Lightweight Rust agent core + LLM provider abstractions.",
  },
  "longbridge/acp-registry": {
    category: "ai-acp-registry",
    targets: ["crates/agent_servers", "crates/acp_thread"],
    score: 70,
    rationale: "Canonical Agent Client Protocol server registry metadata.",
  },
};

const KNOWN_SHOWCASE_BENCH_NAMES = new Set([
  "nsh-perf-bench",
  "gpui-kit-showcases",
  "nsh-showcases",
  "nsh-compat",
  "nsh-longbridge-ai",
  "nsh-website-translates",
  "longbridge-lite",
  "rust-i18n-example",
  "parser-benchmark",
  "cli-candlestick-chart",
  "skills",
  "extras-skills",
  "awesome-copilot",
  "omarchy-longbridge",
  "longbridge-desktop-website",
  "longbridge-presence",
  "mcp-go",
  "open-connector",
  "financial-services-plugins",
  "financial-services",
  "feishu-pages",
  "developers",
  "docs",
  "longbridge.github.io",
  "longbridgeai-docs",
  "mintlify-docs",
  "whale-docs",
  "navi",
  "vscode-i18n-extractor",
]);

const COMMUNITY_GPUI_LIBS = new Set([
  "gpui-ce/gpui-ce",
  "gpui-ce/gpui-component",
  "1bye/gpui-motion",
  "wsafight/gpui-rsx",
  "tiborgats/native-theme",
  "remorses/gpuix",
]);

export function classifyRepo(
  repo: GitHubRepoSummary,
  localIndex: LocalAphrodyIndex,
  sourceOrg: "longbridge" | "zed-upstream" | "community-gpui" = "longbridge",
): Omit<ClassifiedRepo, "enriched"> {
  const key = repo.fullName.toLowerCase();
  const shortName = repo.name.toLowerCase();
  const blob = `${repo.name} ${repo.description} ${repo.topics.join(" ")}`.toLowerCase();

  // 1. Check explicit absorbed/flattened mappings + live workspace matches.
  const absorbed = KNOWN_ABSORBED_SPECS[key];
  if (absorbed) {
    const verifiedMatches = absorbed.targets.filter((target) => {
      if (target.startsWith("Cargo.toml#")) {
        const dep = target.slice("Cargo.toml#".length);
        return dep in localIndex.workspaceDependencies || dep in localIndex.patchCratesIo;
      }
      if (target.startsWith("vendor/")) {
        const vend = target.split("/")[1];
        return localIndex.vendorDirs.includes(vend) || localIndex.gitmodules.some((g) => g.path === target);
      }
      if (target.startsWith("crates/")) {
        const crateDir = target.split("/")[1];
        return localIndex.crateDirs.includes(crateDir) || localIndex.workspaceMembers.includes(`crates/${crateDir}`);
      }
      return false;
    });
    return {
      ...repo,
      sourceOrg,
      classification: "ABSORBED_FLATTENED",
      category: absorbed.category,
      relevanceScore: 100,
      localMatches: verifiedMatches.length > 0 ? verifiedMatches : absorbed.targets,
      targetCrates: absorbed.targets,
      rationale: absorbed.rationale,
    };
  }

  // 2. Check known high-value merge candidates.
  const merge = KNOWN_MERGE_CANDIDATES[key];
  if (merge) {
    return {
      ...repo,
      sourceOrg,
      classification: "MERGE_CANDIDATE",
      category: merge.category,
      relevanceScore: merge.score,
      localMatches: findLocalAnchorsForRepo(shortName, localIndex),
      targetCrates: merge.targets,
      rationale: merge.rationale,
    };
  }

  // 3. Check explicit benchmark/showcase/docs/skill reference repos.
  if (KNOWN_SHOWCASE_BENCH_NAMES.has(shortName)) {
    const isBench = shortName.includes("bench") || shortName.includes("compat");
    const isShowcase = shortName.includes("showcase") || shortName.includes("lite") || shortName.includes("longbridge-ai");
    return {
      ...repo,
      sourceOrg,
      classification: "REFERENCE_BENCH_SHOWCASE",
      category: isBench ? "benchmark-compat" : isShowcase ? "showcase-app" : "docs-skills-reference",
      relevanceScore: isBench || isShowcase ? 78 : 55,
      localMatches: findLocalAnchorsForRepo(shortName, localIndex),
      targetCrates: shortName.includes("bench") ? ["tooling/perf", "crates/bench_metrics"] : ["aphrody/docs/architecture", "crates/aphrody_desktop"],
      rationale: isBench
        ? "Benchmark/compatibility harness retained as reference validation suite."
        : isShowcase
          ? "Reference showcase / sample application used for UX, layout, and sitemap/screenshot parity."
          : "Documentation, Agent Skills, or ecosystem integration reference.",
    };
  }

  // 4. Community GPUI repos or dynamically discovered GPUI/NSH/Wasm/AI repos.
  const isGpuiUiSignal = /\b(gpui|nsh|native-shell|wef|purview|wasm|webgpu|ratatui|mcp|acp|webview|quickjs|i18n)\b/.test(blob);
  const isRustOrTs = ["rust", "typescript", "javascript"].includes(repo.language.toLowerCase());

  if (sourceOrg === "community-gpui") {
    const localMatch = findLocalAnchorsForRepo(shortName, localIndex);
    const isLib =
      COMMUNITY_GPUI_LIBS.has(key) || /\b(kit|component|motion|theme|rsx|binding|framework|widget|layout)\b/.test(blob);
    return {
      ...repo,
      sourceOrg,
      classification: localMatch.length > 0 ? "ABSORBED_FLATTENED" : isLib ? "MERGE_CANDIDATE" : "REFERENCE_BENCH_SHOWCASE",
      category: isLib ? "community-gpui-lib" : "community-gpui-app",
      relevanceScore: Math.min(95, (isLib ? 74 : 58) + Math.round(Math.log10(Math.max(10, repo.stars)) * 5)),
      localMatches: localMatch,
      targetCrates: isLib ? ["crates/gpui", "crates/ui", "crates/gpui_ui", "crates/aphrody_ui"] : ["aphrody/docs/architecture"],
      rationale: isLib
        ? "Community GPUI library/framework crate evaluated for widget, theme, animation, or JS/TS binding capabilities."
        : "Community GPUI production application benchmarked for UX patterns and ecosystem showcase parity.",
    };
  }

  if (isGpuiUiSignal && isRustOrTs && !repo.archived) {
    if (/(showcase|bench|example|docs|website|translate)/.test(blob)) {
      return {
        ...repo,
        sourceOrg,
        classification: "REFERENCE_BENCH_SHOWCASE",
        category: "discovered-showcase-docs",
        relevanceScore: 60,
        localMatches: findLocalAnchorsForRepo(shortName, localIndex),
        targetCrates: ["aphrody/docs/architecture"],
        rationale: "Discovered GPUI/NSH/UI showcase, benchmark, or documentation repository.",
      };
    }
    return {
      ...repo,
      sourceOrg,
      classification: "MERGE_CANDIDATE",
      category: "discovered-gpui-ai",
      relevanceScore: 72,
      localMatches: findLocalAnchorsForRepo(shortName, localIndex),
      targetCrates: ["crates/gpui_ui", "crates/aphrody_desktop"],
      rationale: "Active GPUI/NSH/Wasm/AI repository matched by live keyword and language classifier.",
    };
  }

  return {
    ...repo,
    sourceOrg,
    classification: "INFRA_OUT_OF_SCOPE",
    category: repo.language ? `infra-${repo.language.toLowerCase()}` : "infra-misc",
    relevanceScore: 10,
    localMatches: [],
    targetCrates: [],
    rationale: "Non-GPUI/NSH backend, SDK, big-data, or archived utility outside Aphrody UI/OS core scope.",
  };
}

function findLocalAnchorsForRepo(shortName: string, localIndex: LocalAphrodyIndex): string[] {
  const matches: string[] = [];
  const normalized = shortName.replaceAll("-", "_");
  for (const crate of localIndex.crateDirs) {
    if (crate === shortName || crate === normalized || (shortName.startsWith("nsh") && crate.startsWith("nsh"))) {
      matches.push(`crates/${crate}`);
    }
  }
  for (const vend of localIndex.vendorDirs) {
    if (vend === shortName || vend === normalized) {
      matches.push(`vendor/${vend}`);
    }
  }
  for (const dep of Object.keys(localIndex.workspaceDependencies)) {
    if (dep === shortName || dep === normalized) {
      matches.push(`Cargo.toml#${dep}`);
    }
  }
  return matches.slice(0, 8);
}

// ---------------------------------------------------------------------------
// GitHub REST + Batched GraphQL + crates.io + npm Fetchers
// ---------------------------------------------------------------------------

function mapRawGhRepo(raw: Record<string, any>): GitHubRepoSummary {
  const fullName = String(raw.full_name ?? `${raw.owner?.login ?? "unknown"}/${raw.name ?? "unknown"}`);
  const [owner, name] = fullName.split("/");
  return {
    name: String(raw.name ?? name ?? ""),
    fullName,
    owner: String(raw.owner?.login ?? owner ?? ""),
    description: String(raw.description ?? ""),
    homepage: String(raw.homepage ?? ""),
    htmlUrl: String(raw.html_url ?? `https://github.com/${fullName}`),
    language: String(raw.language ?? ""),
    stars: Number(raw.stargazers_count ?? 0),
    forks: Number(raw.forks_count ?? 0),
    openIssuesAndPrs: Number(raw.open_issues_count ?? 0),
    archived: Boolean(raw.archived),
    isFork: Boolean(raw.fork),
    defaultBranch: String(raw.default_branch ?? "main"),
    pushedAt: String(raw.pushed_at ?? ""),
    topics: Array.isArray(raw.topics) ? raw.topics.map(String) : [],
  };
}

function runGhApi(args: string[], stdinPayload?: string): string {
  const proc = Bun.spawnSync(["gh", "api", ...args], {
    cwd: REPO_ROOT,
    stdin: stdinPayload !== undefined ? new TextEncoder().encode(stdinPayload) : undefined,
    stdout: "pipe",
    stderr: "pipe",
  });
  if (proc.exitCode !== 0) {
    const err = new TextDecoder().decode(proc.stderr);
    throw new Error(`gh api ${args.join(" ")} failed (exit ${proc.exitCode}): ${err}`);
  }
  return new TextDecoder().decode(proc.stdout);
}

export async function fetchLongbridgeOrgRepos(): Promise<GitHubRepoSummary[]> {
  const rawText = runGhApi(["--paginate", "--slurp", "orgs/longbridge/repos?per_page=100"]);
  const rawRepos = parsePaginatedJson<Record<string, any>>(rawText);
  return rawRepos.map(mapRawGhRepo);
}

export async function fetchSingleGitHubRepo(owner: string, name: string): Promise<GitHubRepoSummary> {
  const rawText = runGhApi([`repos/${owner}/${name}`]);
  return mapRawGhRepo(JSON.parse(rawText) as Record<string, any>);
}

export async function fetchCommunityGpuiRepos(): Promise<GitHubRepoSummary[]> {
  const seen = new Map<string, GitHubRepoSummary>();
  try {
    const raw = runGhApi(["search/repositories?q=gpui+language:Rust+stars:%3E100&sort=stars&per_page=20"]);
    const parsed = JSON.parse(raw) as { items?: Array<Record<string, any>> };
    for (const item of parsed.items ?? []) {
      const mapped = mapRawGhRepo(item);
      if (mapped.owner.toLowerCase() === "longbridge" || mapped.fullName.toLowerCase() === "zed-industries/zed") continue;
      seen.set(mapped.fullName.toLowerCase(), mapped);
    }
  } catch {
    // Search API fallback is non-fatal.
  }

  // Also ensure high-value crates.io-discovered community GPUI repos are queried if missed by star sort.
  const curatedCommunity = [
    ["tiborgats", "native-theme"],
    ["1bye", "gpui-motion"],
    ["wsafight", "gpui-rsx"],
    ["gpui-ce", "gpui-component"],
  ] as const;
  await mapWithConcurrency(curatedCommunity, 3, async ([owner, name]) => {
    const key = `${owner}/${name}`.toLowerCase();
    if (seen.has(key)) return;
    try {
      seen.set(key, await fetchSingleGitHubRepo(owner, name));
    } catch {
      // Ignore transient unavailable community repos.
    }
  });

  return Array.from(seen.values()).sort((a, b) => b.stars - a.stars);
}

export async function fetchCratesIoMetadata(crateNames: readonly string[], concurrency = 4): Promise<Record<string, CratesIoMeta>> {
  const unique = Array.from(new Set(crateNames.filter(Boolean)));
  const out: Record<string, CratesIoMeta> = {};
  await mapWithConcurrency(unique, concurrency, async (name) => {
    try {
      const resp = await fetch(`https://crates.io/api/v1/crates/${encodeURIComponent(name)}`, {
        headers: { "User-Agent": USER_AGENT, Accept: "application/json" },
        signal: AbortSignal.timeout(10_000),
      });
      if (!resp.ok) {
        out[name] = {
          name,
          maxVersion: "",
          downloads: 0,
          recentDownloads: 0,
          updatedAt: "",
          repository: "",
          homepage: "",
          exists: false,
          error: `HTTP ${resp.status}`,
        };
        return;
      }
      const data = (await resp.json()) as { crate?: Record<string, any> };
      const c = data.crate ?? {};
      out[name] = {
        name,
        maxVersion: String(c.max_version ?? c.newest_version ?? ""),
        downloads: Number(c.downloads ?? 0),
        recentDownloads: Number(c.recent_downloads ?? 0),
        updatedAt: String(c.updated_at ?? ""),
        repository: String(c.repository ?? ""),
        homepage: String(c.homepage ?? ""),
        exists: true,
      };
    } catch (err) {
      out[name] = {
        name,
        maxVersion: "",
        downloads: 0,
        recentDownloads: 0,
        updatedAt: "",
        repository: "",
        homepage: "",
        exists: false,
        error: String(err),
      };
    }
  });
  return out;
}

export async function fetchNpmMetadata(pkgNames: readonly string[], concurrency = 4): Promise<Record<string, NpmMeta>> {
  const unique = Array.from(new Set(pkgNames.filter(Boolean)));
  const out: Record<string, NpmMeta> = {};
  await mapWithConcurrency(unique, concurrency, async (name) => {
    try {
      const resp = await fetch(`https://registry.npmjs.org/${encodeURIComponent(name)}`, {
        headers: { "User-Agent": USER_AGENT, Accept: "application/json" },
        signal: AbortSignal.timeout(10_000),
      });
      if (!resp.ok) {
        out[name] = { name, latestVersion: "", modified: "", repository: "", homepage: "", exists: false, error: `HTTP ${resp.status}` };
        return;
      }
      const data = (await resp.json()) as Record<string, any>;
      const distTags = (data["dist-tags"] ?? {}) as Record<string, string>;
      const timeMap = (data.time ?? {}) as Record<string, string>;
      out[name] = {
        name,
        latestVersion: String(distTags.latest ?? ""),
        modified: String(timeMap.modified ?? ""),
        repository: typeof data.repository?.url === "string" ? data.repository.url : "",
        homepage: String(data.homepage ?? ""),
        exists: true,
      };
    } catch (err) {
      out[name] = { name, latestVersion: "", modified: "", repository: "", homepage: "", exists: false, error: String(err) };
    }
  });
  return out;
}

export async function fetchRepoDetailsGraphql(
  specs: readonly CoreRepoSpec[],
  cratesMeta: Record<string, CratesIoMeta>,
  npmMeta: Record<string, NpmMeta>,
): Promise<Map<string, EnrichedRepoDetails>> {
  const chunks: CoreRepoSpec[][] = [];
  for (let i = 0; i < specs.length; i += 6) {
    chunks.push(specs.slice(i, i + 6));
  }

  const results = new Map<string, EnrichedRepoDetails>();

  await mapWithConcurrency(chunks, 3, async (chunk) => {
    const fields = chunk
      .map((spec, idx) => {
        const subExpr = spec.subManifestPath
          ? `subCargoToml: object(expression: "HEAD:${spec.subManifestPath}") { ... on Blob { text } }`
          : "";
        return `
  r_${idx}: repository(owner: "${spec.owner}", name: "${spec.name}") {
    nameWithOwner
    defaultBranchRef {
      name
      target {
        ... on Commit {
          oid
          committedDate
          messageHeadline
        }
      }
    }
    openPRs: pullRequests(states: OPEN) { totalCount }
    openIssues: issues(states: OPEN) { totalCount }
    branches: refs(refPrefix: "refs/heads/", first: 15, orderBy: { field: TAG_COMMIT_DATE, direction: DESC }) {
      totalCount
      nodes { name }
    }
    cargoToml: object(expression: "HEAD:Cargo.toml") { ... on Blob { text } }
    packageJson: object(expression: "HEAD:package.json") { ... on Blob { text } }
    ${subExpr}
  }`;
      })
      .join("\n");

    const query = `query {\n${fields}\n}`;
    const raw = runGhApi(["graphql", "--input", "-"], JSON.stringify({ query }));
    const parsed = JSON.parse(raw) as { data?: Record<string, any> };
    const data = parsed.data ?? {};

    chunk.forEach((spec, idx) => {
      const node = data[`r_${idx}`];
      if (!node) return;
      const fullName = String(node.nameWithOwner ?? `${spec.owner}/${spec.name}`);
      const commit = node.defaultBranchRef?.target ?? {};
      const branches = Array.isArray(node.branches?.nodes)
        ? node.branches.nodes.map((n: any) => String(n.name)).filter(Boolean)
        : [];
      let cargoManifest: CargoManifestSummary | undefined;
      if (typeof node.cargoToml?.text === "string") {
        try {
          cargoManifest = extractCargoManifestSummary(node.cargoToml.text);
        } catch {
          // Ignore malformed upstream TOML.
        }
      }
      let subCargoManifest: CargoManifestSummary | undefined;
      if (typeof node.subCargoToml?.text === "string") {
        try {
          subCargoManifest = extractCargoManifestSummary(node.subCargoToml.text);
        } catch {
          // Ignore malformed upstream sub-TOML.
        }
      }
      let packageJson: PackageJsonSummary | undefined;
      if (typeof node.packageJson?.text === "string") {
        try {
          packageJson = extractPackageJsonSummary(node.packageJson.text);
        } catch {
          // Ignore malformed upstream package.json.
        }
      }

      results.set(fullName.toLowerCase(), {
        nameWithOwner: fullName,
        openPRs: Number(node.openPRs?.totalCount ?? 0),
        openIssues: Number(node.openIssues?.totalCount ?? 0),
        latestCommitSha: String(commit.oid ?? ""),
        latestCommitDate: String(commit.committedDate ?? ""),
        latestCommitMessage: String(commit.messageHeadline ?? ""),
        activeBranches: branches,
        totalBranchCount: Number(node.branches?.totalCount ?? branches.length),
        cargoManifest,
        subCargoManifest,
        subCargoPath: spec.subManifestPath,
        packageJson,
        cratesIo: spec.cratesIoNames.map((n) => cratesMeta[n]).filter(Boolean),
        npm: spec.npmNames.map((n) => npmMeta[n]).filter(Boolean),
      });
    });
  });

  return results;
}

export async function runScanCommand(root = REPO_ROOT): Promise<ScanReport> {
  const localIndex = loadLocalAphrodyIndex(root);
  const [lbRaw, zedRaw, communityRaw] = await Promise.all([
    fetchLongbridgeOrgRepos(),
    fetchSingleGitHubRepo("zed-industries", "zed"),
    fetchCommunityGpuiRepos(),
  ]);

  const allCrateNames = CORE_ENRICHED_SPECS.flatMap((s) => s.cratesIoNames);
  const allNpmNames = CORE_ENRICHED_SPECS.flatMap((s) => s.npmNames);
  const [cratesMeta, npmMeta] = await Promise.all([
    fetchCratesIoMetadata(allCrateNames),
    fetchNpmMetadata(allNpmNames),
  ]);
  const enrichedMap = await fetchRepoDetailsGraphql(CORE_ENRICHED_SPECS, cratesMeta, npmMeta);

  const longbridgeRepos: ClassifiedRepo[] = lbRaw
    .map((repo) => {
      const classified = classifyRepo(repo, localIndex, "longbridge");
      const enriched = enrichedMap.get(repo.fullName.toLowerCase());
      return enriched ? { ...classified, enriched } : classified;
    })
    .sort((a, b) => b.relevanceScore - a.relevanceScore || b.stars - a.stars || a.name.localeCompare(b.name));

  const upstreamZedBase = classifyRepo(zedRaw, localIndex, "zed-upstream");
  const upstreamZed: ClassifiedRepo = {
    ...upstreamZedBase,
    enriched: enrichedMap.get("zed-industries/zed"),
  };

  const communityGpuiRepos: ClassifiedRepo[] = communityRaw.map((repo) => classifyRepo(repo, localIndex, "community-gpui"));

  const countsByClassification: Record<RepoClassification, number> = {
    ABSORBED_FLATTENED: 0,
    MERGE_CANDIDATE: 0,
    REFERENCE_BENCH_SHOWCASE: 0,
    INFRA_OUT_OF_SCOPE: 0,
  };
  for (const r of longbridgeRepos) {
    countsByClassification[r.classification]++;
  }

  return {
    schemaVersion: 1,
    generatedAt: new Date().toISOString(),
    longbridgeTotalRepos: longbridgeRepos.length,
    countsByClassification,
    localIndexSummary: {
      gitmodulesCount: localIndex.gitmodules.length,
      vendorDirsCount: localIndex.vendorDirs.length,
      crateDirsCount: localIndex.crateDirs.length,
      workspaceMembersCount: localIndex.workspaceMembers.length,
      workspaceDepsCount: Object.keys(localIndex.workspaceDependencies).length,
      patchCount: Object.keys(localIndex.patchCratesIo).length,
    },
    longbridgeRepos,
    upstreamZed,
    communityGpuiRepos,
  };
}

// ---------------------------------------------------------------------------
// Cargo Metadata, Feature Matrix & Centralization Audit
// ---------------------------------------------------------------------------

export interface CargoMetadataRaw {
  packages: Array<{
    id: string;
    name: string;
    version: string;
    source: string | null;
    license?: string | null;
    manifest_path: string;
    features?: Record<string, string[]>;
    dependencies?: Array<{ name: string; kind: string | null; path?: string | null }>;
  }>;
  workspace_members: string[];
  workspace_default_members?: string[];
  target_directory: string;
  workspace_root: string;
  resolve?: {
    nodes: Array<{
      id: string;
      dependencies: string[];
      deps?: Array<{ name: string; pkg: string }>;
      features?: string[];
    }>;
  } | null;
}

export async function runCargoMetadata(root = REPO_ROOT): Promise<CargoMetadataRaw> {
  for (const extraArgs of [["--locked"], ["--offline"], []]) {
    const proc = Bun.spawn(["cargo", "metadata", "--format-version", "1", ...extraArgs], {
      cwd: root,
      stdout: "pipe",
      stderr: "pipe",
    });
    const [stdoutText, stderrText, exitCode] = await Promise.all([
      new Response(proc.stdout).text(),
      new Response(proc.stderr).text(),
      proc.exited,
    ]);
    if (exitCode === 0 && stdoutText.trim().length > 0) {
      return JSON.parse(stdoutText) as CargoMetadataRaw;
    }
    if (extraArgs.length === 0) {
      throw new Error(`cargo metadata failed (exit ${exitCode}): ${stderrText}`);
    }
  }
  throw new Error("cargo metadata failed unexpectedly");
}

function summarizeDepSpec(spec: unknown): string {
  if (typeof spec === "string") return `version = "${spec}"`;
  if (spec && typeof spec === "object") {
    const obj = spec as Record<string, unknown>;
    if (obj.path) return `path = "${obj.path}"`;
    if (obj.git) return `git = "${obj.git}"`;
    if (obj.version) return `version = "${obj.version}"`;
    return JSON.stringify(obj);
  }
  return String(spec);
}

function classifyDepSourceSpec(spec: unknown): { kind: "path" | "git" | "registry"; detail: string; pkgName?: string } {
  if (typeof spec === "string") {
    return { kind: "registry", detail: spec };
  }
  if (spec && typeof spec === "object") {
    const obj = spec as Record<string, unknown>;
    const pkgName = typeof obj.package === "string" ? obj.package : undefined;
    if (typeof obj.path === "string") return { kind: "path", detail: obj.path, pkgName };
    if (typeof obj.git === "string") return { kind: "git", detail: obj.git, pkgName };
    if (typeof obj.version === "string") return { kind: "registry", detail: obj.version, pkgName };
  }
  return { kind: "registry", detail: "unspecified" };
}

export function isExtensionPackage(manifestPath: string, packageName: string): boolean {
  const normalized = manifestPath.replaceAll("\\", "/");
  const parts = normalized.split("/");
  const grandParent = parts.length >= 3 ? parts[parts.length - 3] : "";
  return grandParent === "extensions" || packageName === "zed_extension_api";
}

export function auditCentralization(metadata: CargoMetadataRaw, root = REPO_ROOT): CargoCentralizationAudit {
  const rootToml = Bun.TOML.parse(readFileSync(join(root, "Cargo.toml"), "utf8")) as Record<string, any>;
  const rootWorkspaceDeps = (rootToml.workspace?.dependencies ?? {}) as Record<string, unknown>;
  const rootPatchCratesIo = (rootToml.patch?.["crates-io"] ?? {}) as Record<string, unknown>;

  const memberIds = new Set(metadata.workspace_members);
  const memberPackages = metadata.packages.filter((p) => memberIds.has(p.id));

  const missingWorkspaceLints: string[] = [];
  const nonWorkspaceDependencies: NonWorkspaceDepFinding[] = [];
  let extensionCountSkipped = 0;
  let auditedMemberCount = 0;

  for (const pkg of memberPackages) {
    if (!existsSync(pkg.manifest_path)) continue;
    const parsed = Bun.TOML.parse(readFileSync(pkg.manifest_path, "utf8")) as Record<string, any>;
    if (parsed.lints?.workspace !== true) {
      missingWorkspaceLints.push(pkg.name);
    }

    if (isExtensionPackage(pkg.manifest_path, pkg.name)) {
      extensionCountSkipped++;
      continue;
    }
    auditedMemberCount++;

    const relManifest = relative(root, pkg.manifest_path).replaceAll("\\", "/");
    const sections: Array<{ key: string; scope: "top" | "target"; map: Record<string, unknown> | undefined }> = [
      { key: "dependencies", scope: "top", map: parsed.dependencies },
      { key: "dev-dependencies", scope: "top", map: parsed["dev-dependencies"] },
      { key: "build-dependencies", scope: "top", map: parsed["build-dependencies"] },
    ];

    if (parsed.target && typeof parsed.target === "object") {
      for (const [cfg, cfgTable] of Object.entries(parsed.target as Record<string, any>)) {
        if (!cfgTable || typeof cfgTable !== "object") continue;
        sections.push({ key: `target.${cfg}.dependencies`, scope: "target", map: cfgTable.dependencies });
        sections.push({ key: `target.${cfg}.dev-dependencies`, scope: "target", map: cfgTable["dev-dependencies"] });
        sections.push({ key: `target.${cfg}.build-dependencies`, scope: "target", map: cfgTable["build-dependencies"] });
      }
    }

    for (const sec of sections) {
      if (!sec.map || typeof sec.map !== "object") continue;
      for (const [depKey, spec] of Object.entries(sec.map)) {
        const isWorkspace = typeof spec === "object" && spec !== null && (spec as Record<string, unknown>).workspace === true;
        if (isWorkspace) continue;
        const actualPkgName =
          typeof spec === "object" && spec !== null && typeof (spec as Record<string, unknown>).package === "string"
            ? String((spec as Record<string, unknown>).package)
            : depKey;
        nonWorkspaceDependencies.push({
          crateName: pkg.name,
          manifestPath: relManifest,
          section: sec.key,
          scope: sec.scope,
          depKey,
          actualPkgName,
          specSummary: summarizeDepSpec(spec),
          inRootWorkspaceDeps: depKey in rootWorkspaceDeps || actualPkgName in rootWorkspaceDeps,
        });
      }
    }
  }

  const wsEntries: WorkspaceDepEntry[] = Object.entries(rootWorkspaceDeps).map(([key, spec]) => {
    const info = classifyDepSourceSpec(spec);
    return {
      key,
      packageName: info.pkgName ?? key,
      kind: info.kind,
      sourceDetail: info.detail,
    };
  });

  const patchEntries = Object.entries(rootPatchCratesIo).map(([name, spec]) => {
    const info = classifyDepSourceSpec(spec);
    return {
      name,
      kind: (info.kind === "path" ? "path" : "git") as "path" | "git",
      detail: info.detail,
    };
  });

  // Duplicate third-party crate analysis across resolved graph.
  const pkgById = new Map(metadata.packages.map((p) => [p.id, p]));
  const resolvedIds = new Set(metadata.resolve?.nodes?.map((n) => n.id) ?? metadata.packages.map((p) => p.id));
  const dependentsByPkgId = new Map<string, Set<string>>();

  for (const node of metadata.resolve?.nodes ?? []) {
    const parent = pkgById.get(node.id);
    if (!parent) continue;
    for (const depId of node.dependencies ?? []) {
      let set = dependentsByPkgId.get(depId);
      if (!set) {
        set = new Set();
        dependentsByPkgId.set(depId, set);
      }
      set.add(`${parent.name}@${parent.version}`);
    }
  }

  const byName = new Map<string, Array<(typeof metadata.packages)[number]>>();
  for (const pkg of metadata.packages) {
    if (!resolvedIds.has(pkg.id)) continue;
    if (pkg.source === null) continue; // Third-party resolved crates only.
    const arr = byName.get(pkg.name) ?? [];
    arr.push(pkg);
    byName.set(pkg.name, arr);
  }

  const duplicatePackages: DuplicatePackageEntry[] = [];
  for (const [name, list] of byName.entries()) {
    if (list.length <= 1) continue;
    const versions = Array.from(new Set(list.map((p) => p.version))).sort();
    const majorMinorVersions = Array.from(new Set(versions.map(normalizeVersionMajorMinor))).sort();
    const sources = Array.from(
      new Set(
        list.map((p) => {
          if (!p.source) return "path";
          if (p.source.startsWith("registry+")) return "crates.io";
          if (p.source.startsWith("git+")) return p.source.split("#")[0];
          return p.source;
        }),
      ),
    );
    const depsSample = new Set<string>();
    for (const p of list) {
      for (const d of dependentsByPkgId.get(p.id) ?? []) {
        depsSample.add(d);
        if (depsSample.size >= 8) break;
      }
      if (depsSample.size >= 8) break;
    }

    duplicatePackages.push({
      name,
      versions,
      majorMinorVersions,
      hasMajorMinorSplit: majorMinorVersions.length > 1,
      instanceCount: list.length,
      sources,
      dependentsSample: Array.from(depsSample),
    });
  }

  duplicatePackages.sort(
    (a, b) =>
      b.instanceCount - a.instanceCount ||
      Number(b.hasMajorMinorSplit) - Number(a.hasMajorMinorSplit) ||
      a.name.localeCompare(b.name),
  );

  const nonWorkspaceTopLevelCount = nonWorkspaceDependencies.filter((d) => d.scope === "top").length;
  const nonWorkspaceTargetLevelCount = nonWorkspaceDependencies.filter((d) => d.scope === "target").length;
  const uniqueNonWorkspaceDepNames = Array.from(new Set(nonWorkspaceDependencies.map((d) => d.depKey))).sort();

  return {
    generatedAt: new Date().toISOString(),
    workspaceMemberCount: memberPackages.length,
    auditedMemberCount,
    extensionCountSkipped,
    missingWorkspaceLints: missingWorkspaceLints.sort(),
    nonWorkspaceDependencies,
    nonWorkspaceTopLevelCount,
    nonWorkspaceTargetLevelCount,
    uniqueNonWorkspaceDepNames,
    workspaceDependencyBreakdown: {
      total: wsEntries.length,
      pathCount: wsEntries.filter((e) => e.kind === "path").length,
      gitCount: wsEntries.filter((e) => e.kind === "git").length,
      registryCount: wsEntries.filter((e) => e.kind === "registry").length,
      gitEntries: wsEntries.filter((e) => e.kind === "git"),
      pathEntriesSample: wsEntries.filter((e) => e.kind === "path").slice(0, 25),
    },
    patchBreakdown: {
      total: patchEntries.length,
      pathCount: patchEntries.filter((e) => e.kind === "path").length,
      gitCount: patchEntries.filter((e) => e.kind === "git").length,
      entries: patchEntries,
    },
    duplicatePackages,
    duplicatePackageCount: duplicatePackages.length,
    duplicateMajorMinorCount: duplicatePackages.filter((d) => d.hasMajorMinorSplit).length,
  };
}

export function isCoreFeatureMatrixCrate(name: string): boolean {
  return (
    name.startsWith("gpui") ||
    name.startsWith("nsh") ||
    name.startsWith("aphrody") ||
    name.startsWith("os_") ||
    name === "iced_gpui" ||
    name === "egui_gpui" ||
    name === "bevy_gpui" ||
    name === "tauri_gpui" ||
    name === "cosmic_gpui" ||
    name === "ui" ||
    name === "ui_input" ||
    name === "brand" ||
    name === "editor" ||
    name.startsWith("acp_") ||
    name === "context_server" ||
    name.startsWith("terminal")
  );
}

export function extractFeatureMatrix(
  metadata: CargoMetadataRaw,
  audit: CargoCentralizationAudit,
  root = REPO_ROOT,
): Record<string, CrateFeatureEntry> {
  const memberIds = new Set(metadata.workspace_members);
  const memberNameSet = new Set(metadata.packages.filter((p) => memberIds.has(p.id)).map((p) => p.name));
  const missingLintsSet = new Set(audit.missingWorkspaceLints);

  const nonWsCounts = new Map<string, number>();
  for (const f of audit.nonWorkspaceDependencies) {
    nonWsCounts.set(f.crateName, (nonWsCounts.get(f.crateName) ?? 0) + 1);
  }

  const matrix: Record<string, CrateFeatureEntry> = {};
  const corePackages = metadata.packages
    .filter((p) => memberIds.has(p.id) && isCoreFeatureMatrixCrate(p.name))
    .sort((a, b) => a.name.localeCompare(b.name));

  for (const pkg of corePackages) {
    const features = pkg.features ?? {};
    const directWorkspaceDeps = Array.from(
      new Set((pkg.dependencies ?? []).map((d) => d.name).filter((n) => memberNameSet.has(n))),
    ).sort();
    matrix[pkg.name] = {
      name: pkg.name,
      version: pkg.version,
      path: dirname(relative(root, pkg.manifest_path)).replaceAll("\\", "/"),
      license: pkg.license ?? "UNSPECIFIED",
      usesWorkspaceLints: !missingLintsSet.has(pkg.name),
      featureCount: Object.keys(features).length,
      features,
      directWorkspaceDeps,
      nonWorkspaceDepCount: nonWsCounts.get(pkg.name) ?? 0,
    };
  }
  return matrix;
}

export function buildGraphifyDocument(
  scan: ScanReport | null,
  metadata: CargoMetadataRaw,
  audit: CargoCentralizationAudit,
  featureMatrix: Record<string, CrateFeatureEntry>,
  localIndex: LocalAphrodyIndex,
  sitemaps: SitemapCrawlResult[],
  screenshots: ScreenshotResult[],
  root = REPO_ROOT,
): GraphifyDocument {
  const nodesMap = new Map<string, GraphifyNode>();
  const edgesMap = new Map<string, GraphifyEdge>();

  const addNode = (node: GraphifyNode) => {
    if (!nodesMap.has(node.id)) nodesMap.set(node.id, node);
  };
  const addEdge = (edge: Omit<GraphifyEdge, "id">) => {
    const id = `${edge.from}|${edge.kind}|${edge.to}`;
    if (!edgesMap.has(id)) edgesMap.set(id, { id, ...edge });
  };

  // 1. Submodule nodes.
  for (const sub of localIndex.gitmodules) {
    addNode({
      id: `sub:${sub.path}`,
      kind: "vendor_submodule",
      label: sub.path,
      path: sub.path,
      url: sub.url,
    });
  }

  // 2. Workspace crate nodes + feature matrix / core dependency edges.
  const memberIds = new Set(metadata.workspace_members);
  const memberPkgMap = new Map(
    metadata.packages.filter((p) => memberIds.has(p.id)).map((p) => [p.name, p]),
  );

  for (const pkg of memberPkgMap.values()) {
    const fm = featureMatrix[pkg.name];
    addNode({
      id: `crate:${pkg.name}`,
      kind: "workspace_crate",
      label: pkg.name,
      path: fm?.path ?? dirname(relative(root, pkg.manifest_path)).replaceAll("\\", "/"),
      version: pkg.version,
      featureCount: Object.keys(pkg.features ?? {}).length,
    });
  }

  for (const fm of Object.values(featureMatrix)) {
    for (const depName of fm.directWorkspaceDeps) {
      if (memberPkgMap.has(depName)) {
        addEdge({ from: `crate:${fm.name}`, to: `crate:${depName}`, kind: "DEPENDS_ON" });
      }
    }
  }

  for (const patch of audit.patchBreakdown.entries) {
    if (patch.kind === "path") {
      const sub = localIndex.gitmodules.find((g) => patch.detail.startsWith(g.path));
      if (sub) {
        addEdge({
          from: `crate:gpui`,
          to: `sub:${sub.path}`,
          kind: "PATCHED_VIA",
          note: `${patch.name} -> ${patch.detail}`,
        });
      }
    }
  }

  // 3. Upstream / Longbridge / Community repo nodes + deduplicated merge/absorbed edges.
  const trackedRepos: ClassifiedRepo[] = [];
  if (scan) {
    trackedRepos.push(scan.upstreamZed);
    trackedRepos.push(...scan.longbridgeRepos.filter((r) => r.classification !== "INFRA_OUT_OF_SCOPE"));
    trackedRepos.push(...scan.communityGpuiRepos);
  }

  const mergeCandidates: GraphifyDocument["mergeCandidates"] = [];

  for (const repo of trackedRepos) {
    const repoNodeId = `gh:${repo.fullName.toLowerCase()}`;
    addNode({
      id: repoNodeId,
      kind: "upstream_repo",
      label: repo.fullName,
      classification: repo.classification,
      stars: repo.stars,
      url: repo.htmlUrl,
    });

    const edgeKind: GraphifyEdge["kind"] =
      repo.classification === "ABSORBED_FLATTENED"
        ? "ABSORBED_INTO"
        : repo.classification === "MERGE_CANDIDATE"
          ? "MERGE_TARGET"
          : "BENCHMARKS";

    for (const target of repo.targetCrates) {
      const cleanCrate = target.replace(/^crates\//, "").split("/")[0];
      if (memberPkgMap.has(cleanCrate)) {
        addEdge({ from: repoNodeId, to: `crate:${cleanCrate}`, kind: edgeKind, note: repo.category });
      } else if (target.startsWith("vendor/")) {
        const sub = localIndex.gitmodules.find((g) => target.startsWith(g.path));
        if (sub) addEdge({ from: repoNodeId, to: `sub:${sub.path}`, kind: edgeKind, note: repo.category });
      }
    }

    if (repo.classification === "MERGE_CANDIDATE") {
      mergeCandidates.push({
        fullName: repo.fullName,
        name: repo.name,
        stars: repo.stars,
        language: repo.language,
        homepage: repo.homepage,
        targetCrates: repo.targetCrates,
        rationale: repo.rationale,
        openPRs: repo.enriched?.openPRs,
        openIssues: repo.enriched?.openIssues,
        latestCommitSha: repo.enriched?.latestCommitSha,
        latestCommitDate: repo.enriched?.latestCommitDate,
        activeBranches: repo.enriched?.activeBranches,
        cratesIo: repo.enriched?.cratesIo,
        features: repo.enriched?.subCargoManifest?.features ?? repo.enriched?.cargoManifest?.features,
      });
    }
  }

  // 4. Website nodes.
  for (const site of sitemaps) {
    const siteId = `web:${site.id}`;
    addNode({
      id: siteId,
      kind: "website_target",
      label: site.url,
      url: site.finalUrl || site.url,
    });
    if (site.id.startsWith("nsh")) {
      addEdge({ from: "gh:longbridge/nsh", to: siteId, kind: "DOCS_SITE" });
    } else if (site.id.startsWith("gpui-kit")) {
      addEdge({ from: "gh:longbridge/gpui-kit", to: siteId, kind: "DOCS_SITE" });
    } else if (site.id.startsWith("zed")) {
      addEdge({ from: "gh:zed-industries/zed", to: siteId, kind: "DOCS_SITE" });
    } else if (site.id.startsWith("purview")) {
      addEdge({ from: "gh:longbridge/purview", to: siteId, kind: "DOCS_SITE" });
    }
  }

  const totalSitemapUrls = computeDeduplicatedSitemapCount(sitemaps);

  return {
    schemaVersion: 1,
    generatedAt: new Date().toISOString(),
    summary: {
      longbridgeRepoCount: scan?.longbridgeTotalRepos ?? 0,
      absorbedCount: scan?.countsByClassification.ABSORBED_FLATTENED ?? 0,
      mergeCandidateCount: scan?.countsByClassification.MERGE_CANDIDATE ?? 0,
      referenceBenchShowcaseCount: scan?.countsByClassification.REFERENCE_BENCH_SHOWCASE ?? 0,
      infraOutOfScopeCount: scan?.countsByClassification.INFRA_OUT_OF_SCOPE ?? 0,
      communityGpuiCount: scan?.communityGpuiRepos.length ?? 0,
      workspaceCrateCount: audit.workspaceMemberCount,
      featureMatrixCrateCount: Object.keys(featureMatrix).length,
      nonWorkspaceDepCount: audit.nonWorkspaceDependencies.length,
      uniqueNonWorkspaceDepCount: audit.uniqueNonWorkspaceDepNames.length,
      missingWorkspaceLintsCount: audit.missingWorkspaceLints.length,
      duplicateGraphCrateCount: audit.duplicatePackageCount,
      duplicateMajorMinorCount: audit.duplicateMajorMinorCount,
      sitemapTargetsCount: sitemaps.length,
      sitemapTotalUrls: totalSitemapUrls,
      screenshotCount: screenshots.length,
      screenshotSuccessCount: screenshots.filter((s) => s.validPng && s.status === "ok").length,
    },
    nodes: Array.from(nodesMap.values()),
    edges: Array.from(edgesMap.values()),
    duplicatePackages: audit.duplicatePackages,
    mergeCandidates,
    featureMatrix,
    centralizationAudit: audit,
  };
}

// ---------------------------------------------------------------------------
// Sitemap XML + Internal Route Link Crawler
// ---------------------------------------------------------------------------

export function parseSitemapXml(xml: string): { urls: string[]; nestedSitemaps: string[] } {
  const urls: string[] = [];
  const nestedSitemaps: string[] = [];
  const isIndex = /<sitemapindex\b/i.test(xml);

  const blockRegex = isIndex ? /<sitemap\b[^>]*>([\s\S]*?)<\/sitemap>/gi : /<url\b[^>]*>([\s\S]*?)<\/url>/gi;
  for (const match of xml.matchAll(blockRegex)) {
    const locMatch = /<loc>\s*([^<]+?)\s*<\/loc>/i.exec(match[1]);
    if (!locMatch) continue;
    const loc = locMatch[1].trim().replaceAll("&amp;", "&");
    if (isIndex) nestedSitemaps.push(loc);
    else urls.push(loc);
  }

  if (urls.length === 0 && nestedSitemaps.length === 0) {
    for (const locMatch of xml.matchAll(/<loc>\s*([^<]+?)\s*<\/loc>/gi)) {
      const loc = locMatch[1].trim().replaceAll("&amp;", "&");
      if (isIndex) nestedSitemaps.push(loc);
      else urls.push(loc);
    }
  }

  return {
    urls: Array.from(new Set(urls)),
    nestedSitemaps: Array.from(new Set(nestedSitemaps)),
  };
}

export function extractInternalLinks(html: string, baseUrl: string): string[] {
  const origin = new URL(baseUrl).origin;
  const links = new Set<string>();
  for (const match of html.matchAll(/<a\b[^>]*\bhref\s*=\s*["']([^"']+)["']/gi)) {
    const href = match[1].trim();
    if (!href || href.startsWith("#") || href.startsWith("mailto:") || href.startsWith("tel:") || href.startsWith("javascript:")) continue;
    try {
      const resolved = new URL(href, baseUrl);
      if (resolved.origin !== origin) continue;
      resolved.hash = "";
      links.add(resolved.href);
    } catch {
      // Ignore malformed relative hrefs.
    }
  }
  return Array.from(links).sort();
}

export function extractHtmlTitle(html: string): string {
  const m = /<title[^>]*>([\s\S]*?)<\/title>/i.exec(html);
  return m ? m[1].replace(/\s+/g, " ").trim() : "";
}

export function computeDeduplicatedSitemapCount(sitemaps: readonly SitemapCrawlResult[]): number {
  // Deduplicate origin-wide sitemaps (e.g. `nsh.dev/` and `nsh.dev/apps` share `nsh.dev/sitemap.xml`)
  // while adding any per-page unique internal links or exact page URLs.
  const seenUrls = new Set<string>();
  const originSitemapMax = new Map<string, number>();
  for (const s of sitemaps) {
    seenUrls.add(s.finalUrl || s.url);
    for (const u of s.internalLinks) seenUrls.add(u);
    if (s.sitemapUrlCount > 0) {
      const prev = originSitemapMax.get(s.sitemapUrl) ?? 0;
      if (s.sitemapUrlCount > prev) originSitemapMax.set(s.sitemapUrl, s.sitemapUrlCount);
      for (const u of s.sitemapUrls) seenUrls.add(u);
    }
  }
  // If any origin sitemap had > 200 URLs (only first 200 stored in `sitemapUrls`), add the remaining tail once per sitemapUrl.
  let extraTail = 0;
  for (const [smUrl, count] of originSitemapMax.entries()) {
    const sample = sitemaps.find((s) => s.sitemapUrl === smUrl);
    const storedCount = sample?.sitemapUrls.length ?? 0;
    if (count > storedCount) extraTail += count - storedCount;
  }
  return seenUrls.size + extraTail;
}

export async function crawlSites(
  targets: readonly ScreenshotTarget[] = DEFAULT_WEBSITE_TARGETS,
  concurrency = 4,
): Promise<SitemapCrawlResult[]> {
  return mapWithConcurrency(targets, concurrency, async (target) => {
    const parsedUrl = new URL(target.url);
    const origin = parsedUrl.origin;
    const candidateSitemaps = [
      `${origin}/sitemap.xml`,
      ...(parsedUrl.pathname !== "/" ? [`${origin}${parsedUrl.pathname.replace(/\/$/, "")}/sitemap.xml`] : []),
    ];

    let finalUrl = target.url;
    let httpStatus = 0;
    let title = "";
    let byteLength = 0;
    let internalLinks: string[] = [];
    let status: "ok" | "error" = "ok";
    let error: string | undefined;

    try {
      const pageResp = await fetch(target.url, {
        headers: { "User-Agent": USER_AGENT, Accept: "text/html,application/xhtml+xml" },
        redirect: "follow",
        signal: AbortSignal.timeout(12_000),
      });
      httpStatus = pageResp.status;
      finalUrl = pageResp.url || target.url;
      const html = await pageResp.text();
      byteLength = Buffer.byteLength(html, "utf8");
      title = extractHtmlTitle(html);
      internalLinks = extractInternalLinks(html, finalUrl);
      if (!pageResp.ok) {
        status = "error";
        error = `HTTP ${pageResp.status}`;
      }
    } catch (err) {
      status = "error";
      error = String(err);
    }

    let sitemapUrl = candidateSitemaps[0];
    let sitemapStatus = 0;
    const sitemapUrlSet = new Set<string>();

    for (const smCandidate of candidateSitemaps) {
      try {
        const smResp = await fetch(smCandidate, {
          headers: { "User-Agent": USER_AGENT, Accept: "application/xml,text/xml" },
          redirect: "follow",
          signal: AbortSignal.timeout(10_000),
        });
        sitemapUrl = smCandidate;
        sitemapStatus = smResp.status;
        if (!smResp.ok) continue;
        const xml = await smResp.text();
        if (!xml.includes("<urlset") && !xml.includes("<sitemapindex") && !xml.includes("<loc>")) continue;
        const parsedSm = parseSitemapXml(xml);
        for (const u of parsedSm.urls) sitemapUrlSet.add(u);
        for (const childSm of parsedSm.nestedSitemaps.slice(0, 5)) {
          try {
            const childResp = await fetch(childSm, {
              headers: { "User-Agent": USER_AGENT },
              signal: AbortSignal.timeout(8_000),
            });
            if (childResp.ok) {
              const childXml = await childResp.text();
              for (const u of parseSitemapXml(childXml).urls) sitemapUrlSet.add(u);
            }
          } catch {
            // Skip unreachable child sitemap.
          }
        }
        if (sitemapUrlSet.size > 0) break;
      } catch {
        // Try next candidate sitemap URL.
      }
    }

    const sitemapUrls = Array.from(sitemapUrlSet).sort();
    const combined = new Set([...sitemapUrls, ...internalLinks]);

    return {
      id: target.id,
      url: target.url,
      finalUrl,
      origin,
      httpStatus,
      title,
      byteLength,
      sitemapUrl,
      sitemapStatus,
      sitemapUrls: sitemapUrls.slice(0, 200),
      sitemapUrlCount: sitemapUrls.length,
      internalLinks: internalLinks.slice(0, 200),
      internalLinkCount: internalLinks.length,
      totalDiscoveredUrls: combined.size,
      status,
      error,
    };
  });
}

// ---------------------------------------------------------------------------
// Headless Browser Screenshot Capture (reusing web-verify.ts CDP pattern)
// ---------------------------------------------------------------------------

export function findBrowserExecutable(): string {
  const fromEnv = process.env.APHRODY_CHROME_BIN ?? process.env.CHROME_BIN;
  if (fromEnv && existsSync(fromEnv)) return fromEnv;

  const candidates =
    process.platform === "win32"
      ? [
          "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
          "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
        ]
      : [
          "/usr/bin/google-chrome",
          "/usr/bin/chromium",
          "/usr/bin/chromium-browser",
          "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        ];

  for (const candidate of candidates) {
    if (existsSync(candidate)) return candidate;
  }
  throw new Error("No Chromium/Chrome/Edge executable found; set APHRODY_CHROME_BIN or CHROME_BIN.");
}

export function parsePngMetadata(buf: Uint8Array): { valid: boolean; width: number; height: number } {
  const PNG_SIG = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
  if (buf.byteLength < 24) return { valid: false, width: 0, height: 0 };
  for (let i = 0; i < PNG_SIG.length; i++) {
    if (buf[i] !== PNG_SIG[i]) return { valid: false, width: 0, height: 0 };
  }
  const view = new DataView(buf.buffer, buf.byteOffset, buf.byteLength);
  const width = view.getUint32(16, false);
  const height = view.getUint32(20, false);
  return {
    valid: width > 0 && height > 0 && buf.byteLength > 1024,
    width,
    height,
  };
}

async function readDevToolsWsUrl(stderrStream: ReadableStream<Uint8Array>): Promise<string> {
  const deadline = Date.now() + 12_000;
  const reader = stderrStream.getReader();
  let buf = "";
  while (Date.now() < deadline) {
    const { value, done } = await reader.read();
    if (done) break;
    buf += new TextDecoder().decode(value);
    const match = buf.match(/DevTools listening on (ws:\/\/[^\s]+)/);
    if (match) return match[1];
  }
  throw new Error(`Chrome DevTools WS not found. stderr=${buf}`);
}

export async function captureSiteScreenshots(
  targets: readonly ScreenshotTarget[] = DEFAULT_WEBSITE_TARGETS,
  outDir = join(REPO_ROOT, SCREENSHOTS_DIR_REL),
  perSiteTimeoutMs = 20_000,
): Promise<ScreenshotResult[]> {
  mkdirSync(outDir, { recursive: true });
  let chromePath = "";
  try {
    chromePath = findBrowserExecutable();
  } catch (err) {
    return targets.map((t) => ({
      id: t.id,
      url: t.url,
      finalUrl: t.url,
      outputPath: relative(REPO_ROOT, join(outDir, t.filename)).replaceAll("\\", "/"),
      title: "",
      status: "fallback_error",
      byteLength: 0,
      width: 0,
      height: 0,
      validPng: false,
      error: String(err),
    }));
  }

  const profileDir = mkdtempSync(join(tmpdir(), "aphrody-lb-shot-"));
  const proc = Bun.spawn(
    [
      chromePath,
      "--headless=new",
      "--remote-debugging-port=0",
      `--user-data-dir=${profileDir}`,
      "--window-size=1280,800",
      "--disable-gpu-sandbox",
      "--no-first-run",
      "--no-default-browser-check",
      "about:blank",
    ],
    { stdout: "pipe", stderr: "pipe" },
  );

  const results: ScreenshotResult[] = [];
  try {
    const browserWs = await readDevToolsWsUrl(proc.stderr);
    const baseHttp = browserWs.replace(/^ws:/, "http:").replace(/\/devtools\/browser\/.*$/, "");
    let pageWs = "";
    for (let i = 0; i < 30; i++) {
      const pageTargets = (await fetch(`${baseHttp}/json/list`).then((r) => r.json())) as Array<{
        type: string;
        webSocketDebuggerUrl: string;
      }>;
      const page = pageTargets.find((t) => t.type === "page");
      if (page?.webSocketDebuggerUrl) {
        pageWs = page.webSocketDebuggerUrl;
        break;
      }
      await Bun.sleep(150);
    }
    if (!pageWs) throw new Error("No page target found in headless browser");

    const ws = new WebSocket(pageWs);
    await new Promise<void>((res, rej) => {
      ws.onopen = () => res();
      ws.onerror = (err) => rej(err);
    });

    let nextId = 1;
    const pending = new Map<number, { resolve: (v: any) => void; reject: (e: any) => void }>();
    let loadResolvers: Array<() => void> = [];

    ws.onmessage = (ev) => {
      const msg = JSON.parse(String(ev.data)) as { id?: number; method?: string; result?: unknown; error?: unknown };
      if (msg.id && pending.has(msg.id)) {
        const p = pending.get(msg.id)!;
        pending.delete(msg.id);
        if (msg.error) p.reject(msg.error);
        else p.resolve(msg.result);
      } else if (msg.method === "Page.loadEventFired") {
        const waiters = loadResolvers;
        loadResolvers = [];
        for (const fn of waiters) fn();
      }
    };

    const cdp = (method: string, params: Record<string, unknown> = {}): Promise<any> => {
      const id = nextId++;
      return new Promise((resolvePromise, rejectPromise) => {
        pending.set(id, { resolve: resolvePromise, reject: rejectPromise });
        ws.send(JSON.stringify({ id, method, params }));
      });
    };

    await cdp("Page.enable");
    await cdp("Runtime.enable");
    await cdp("Network.enable");
    await cdp("Network.setUserAgentOverride", { userAgent: USER_AGENT });
    await cdp("Emulation.setDeviceMetricsOverride", { width: 1280, height: 800, deviceScaleFactor: 1, mobile: false });

    for (const target of targets) {
      const shotPath = join(outDir, target.filename);
      const relShotPath = relative(REPO_ROOT, shotPath).replaceAll("\\", "/");
      try {
        const loadPromise = new Promise<void>((resolveLoad) => {
          loadResolvers.push(resolveLoad);
        });
        const navRes = await cdp("Page.navigate", { url: target.url });
        if (navRes?.errorText) {
          throw new Error(`Navigation error: ${navRes.errorText}`);
        }
        await Promise.race([loadPromise, Bun.sleep(perSiteTimeoutMs)]);

        // Poll readyState briefly for SPA hydration before capturing.
        const pollDeadline = Date.now() + 6_000;
        let stateInfo: { readyState: string; title: string; href: string; bodyLen: number } = {
          readyState: "",
          title: "",
          href: target.url,
          bodyLen: 0,
        };
        while (Date.now() < pollDeadline) {
          const evalRes = await cdp("Runtime.evaluate", {
            expression: `(() => ({
              readyState: document.readyState,
              title: document.title || "",
              href: location.href,
              bodyLen: (document.body?.innerText || "").trim().length
            }))()`,
            returnByValue: true,
          });
          stateInfo = evalRes?.result?.value ?? stateInfo;
          if (stateInfo.readyState === "complete" && stateInfo.bodyLen > 20) {
            await Bun.sleep(800);
            break;
          }
          await Bun.sleep(250);
        }

        const shot = await cdp("Page.captureScreenshot", { format: "png" });
        const pngBuf = Buffer.from(shot.data, "base64");
        writeFileSync(shotPath, pngBuf);
        const pngMeta = parsePngMetadata(pngBuf);

        results.push({
          id: target.id,
          url: target.url,
          finalUrl: stateInfo.href || target.url,
          outputPath: relShotPath,
          title: stateInfo.title,
          status: pngMeta.valid ? "ok" : "fallback_error",
          byteLength: pngBuf.byteLength,
          width: pngMeta.width,
          height: pngMeta.height,
          validPng: pngMeta.valid,
          error: pngMeta.valid ? undefined : "Captured PNG failed size/IHDR validation",
        });
      } catch (err) {
        results.push({
          id: target.id,
          url: target.url,
          finalUrl: target.url,
          outputPath: relShotPath,
          title: "",
          status: "fallback_error",
          byteLength: 0,
          width: 0,
          height: 0,
          validPng: false,
          error: String(err),
        });
      }
    }

    ws.close();
  } finally {
    proc.kill();
    await proc.exited.catch(() => {});
    rmSync(profileDir, { recursive: true, force: true });
  }

  return results;
}

// ---------------------------------------------------------------------------
// Inventory TOML & Architecture Markdown Writers
// ---------------------------------------------------------------------------

export function buildInventoryTomlPayload(
  scan: ScanReport | null,
  graph: GraphifyDocument | null,
  sitemaps: SitemapCrawlResult[],
  screenshots: ScreenshotResult[],
): Record<string, unknown> {
  const longbridgeCatalog = (scan?.longbridgeRepos ?? []).map((r) => ({
    name: r.name,
    full_name: r.fullName,
    classification: r.classification,
    category: r.category,
    relevance_score: r.relevanceScore,
    language: r.language || "unknown",
    stars: r.stars,
    archived: r.archived,
    fork: r.isFork,
    default_branch: r.defaultBranch,
    pushed_at: r.pushedAt,
    homepage: r.homepage || undefined,
    local_matches: r.localMatches,
    target_crates: r.targetCrates,
    open_prs: r.enriched?.openPRs,
    open_issues: r.enriched?.openIssues,
    latest_sha: r.enriched?.latestCommitSha ? r.enriched.latestCommitSha.slice(0, 12) : undefined,
    latest_commit_date: r.enriched?.latestCommitDate || undefined,
    active_branches: r.enriched?.activeBranches?.length ? r.enriched.activeBranches : undefined,
    crates_io: r.enriched?.cratesIo?.filter((c) => c.exists).map((c) => `${c.name}@${c.maxVersion}`),
    npm_packages: r.enriched?.npm?.filter((n) => n.exists).map((n) => `${n.name}@${n.latestVersion}`),
  }));

  const communityCatalog = (scan?.communityGpuiRepos ?? []).map((r) => ({
    name: r.name,
    full_name: r.fullName,
    classification: r.classification,
    stars: r.stars,
    language: r.language || "Rust",
    homepage: r.homepage || undefined,
    target_crates: r.targetCrates,
    rationale: r.rationale,
  }));

  const workspaceFeatures = Object.values(graph?.featureMatrix ?? {}).map((f) => ({
    crate_name: f.name,
    version: f.version,
    path: f.path,
    license: f.license,
    workspace_lints: f.usesWorkspaceLints,
    feature_count: f.featureCount,
    features: Object.keys(f.features),
    non_workspace_deps: f.nonWorkspaceDepCount,
  }));

  const duplicateCratesTop = (graph?.duplicatePackages ?? []).slice(0, 40).map((d) => ({
    name: d.name,
    versions: d.versions,
    major_minor_versions: d.majorMinorVersions,
    has_major_minor_split: d.hasMajorMinorSplit,
    instance_count: d.instanceCount,
    sources: d.sources,
  }));

  const nonWorkspaceAudit = (graph?.centralizationAudit.nonWorkspaceDependencies ?? []).map((f) => ({
    crate_name: f.crateName,
    manifest_path: f.manifestPath,
    section: f.section,
    scope: f.scope,
    dep_key: f.depKey,
    actual_pkg: f.actualPkgName,
    spec: f.specSummary,
    in_workspace_deps: f.inRootWorkspaceDeps,
  }));

  const websiteEntries = sitemaps.map((s) => ({
    id: s.id,
    url: s.url,
    final_url: s.finalUrl,
    http_status: s.httpStatus,
    title: s.title,
    sitemap_url: s.sitemapUrl,
    sitemap_status: s.sitemapStatus,
    sitemap_url_count: s.sitemapUrlCount,
    internal_link_count: s.internalLinkCount,
    total_discovered_urls: s.totalDiscoveredUrls,
    status: s.status,
  }));

  const screenshotEntries = screenshots.map((s) => ({
    id: s.id,
    url: s.url,
    output_path: s.outputPath,
    status: s.status,
    valid_png: s.validPng,
    byte_length: s.byteLength,
    width: s.width,
    height: s.height,
    title: s.title,
  }));

  return stripNullish({
    schema_version: 1,
    generated_at: new Date().toISOString(),
    summary: {
      longbridge_total_repos: scan?.longbridgeTotalRepos ?? graph?.summary.longbridgeRepoCount ?? 0,
      absorbed_flattened_count: scan?.countsByClassification.ABSORBED_FLATTENED ?? graph?.summary.absorbedCount ?? 0,
      merge_candidate_count: scan?.countsByClassification.MERGE_CANDIDATE ?? graph?.summary.mergeCandidateCount ?? 0,
      reference_bench_showcase_count:
        scan?.countsByClassification.REFERENCE_BENCH_SHOWCASE ?? graph?.summary.referenceBenchShowcaseCount ?? 0,
      infra_out_of_scope_count: scan?.countsByClassification.INFRA_OUT_OF_SCOPE ?? graph?.summary.infraOutOfScopeCount ?? 0,
      community_gpui_count: scan?.communityGpuiRepos.length ?? graph?.summary.communityGpuiCount ?? 0,
      workspace_crate_count: graph?.summary.workspaceCrateCount ?? 0,
      feature_matrix_crate_count: graph?.summary.featureMatrixCrateCount ?? 0,
      non_workspace_dep_count: graph?.summary.nonWorkspaceDepCount ?? 0,
      unique_non_workspace_dep_count: graph?.summary.uniqueNonWorkspaceDepCount ?? 0,
      missing_workspace_lints_count: graph?.summary.missingWorkspaceLintsCount ?? 0,
      duplicate_graph_crate_count: graph?.summary.duplicateGraphCrateCount ?? 0,
      duplicate_major_minor_count: graph?.summary.duplicateMajorMinorCount ?? 0,
      sitemap_targets_count: sitemaps.length,
      sitemap_total_urls: computeDeduplicatedSitemapCount(sitemaps),
      screenshot_count: screenshots.length,
      screenshot_success_count: screenshots.filter((s) => s.validPng && s.status === "ok").length,
    },
    upstream_zed: scan
      ? {
          full_name: scan.upstreamZed.fullName,
          stars: scan.upstreamZed.stars,
          default_branch: scan.upstreamZed.defaultBranch,
          latest_sha: scan.upstreamZed.enriched?.latestCommitSha.slice(0, 12),
          latest_commit_date: scan.upstreamZed.enriched?.latestCommitDate,
          open_prs: scan.upstreamZed.enriched?.openPRs,
          open_issues: scan.upstreamZed.enriched?.openIssues,
          crates_io: scan.upstreamZed.enriched?.cratesIo?.filter((c) => c.exists).map((c) => `${c.name}@${c.maxVersion}`),
          gpui_features: Object.keys(scan.upstreamZed.enriched?.subCargoManifest?.features ?? {}),
        }
      : undefined,
    longbridge_repos: longbridgeCatalog,
    community_gpui: communityCatalog,
    workspace_feature_matrix: workspaceFeatures,
    non_workspace_deps: nonWorkspaceAudit,
    duplicate_crates_top: duplicateCratesTop,
    websites: websiteEntries,
    screenshots: screenshotEntries,
  });
}

export function writeInventoryToml(
  scan: ScanReport | null,
  graph: GraphifyDocument | null,
  sitemaps: SitemapCrawlResult[],
  screenshots: ScreenshotResult[],
  outPath = join(REPO_ROOT, INVENTORY_TOML_REL),
): void {
  mkdirSync(dirname(outPath), { recursive: true });
  const payload = buildInventoryTomlPayload(scan, graph, sitemaps, screenshots);
  const header = [
    "# Auto-generated by `bun tooling/bun/longbridge-ingest.ts`.",
    "# Canonical inventory of Longbridge + Zed upstream + Community GPUI deduplication,",
    "# workspace feature matrix, Cargo dependency centralization, and live showcase verification.",
    "",
  ].join("\n");
  writeFileSync(outPath, `${header}${Bun.TOML.stringify(payload)}`, "utf8");
}

export function buildMermaidTopology(graph: GraphifyDocument): string {
  const lines: string[] = [
    "```mermaid",
    "flowchart LR",
    "  subgraph Upstream[\"Upstream & Community Ecosystems\"]",
    "    ZED[\"zed-industries/zed<br/>GPUI + Editor Core\"]",
    "    NSH[\"longbridge/nsh<br/>Native Shell v0.5.0\"]",
    "    QJS[\"longbridge/quickjs-jit<br/>QuickJS JIT 0.14.2\"]",
    "    I18N[\"longbridge/rust-i18n<br/>i18n Codegen 4.2.4\"]",
    "    KIT[\"longbridge/gpui-kit<br/>60+ UI Components\"]",
    "    FAST[\"longbridge/gpui-fast<br/>Incremental Taffy Layout\"]",
    "    MOB[\"longbridge/gpui-mobile<br/>Android Vulkan + iOS Hardware\"]",
    "    WEF[\"longbridge/wef<br/>CEF3 Offscreen WebView\"]",
    "    PUR[\"longbridge/purview<br/>GPUI MCP Inspection\"]",
    "  end",
    "",
    "  subgraph Submodules[\"Centralized Git Submodules (vendor/*)\"]",
    "    TAFFY[\"vendor/taffy\"]",
    "    WGPU[\"vendor/wgpu\"]",
    "    WRY[\"vendor/tauri/wry\"]",
    "    CEF[\"vendor/tauri/cef-rs\"]",
    "  end",
    "",
    "  subgraph Core[\"Aphrody Flattened Workspace (Cargo.toml)\"]",
    "    GPUI[\"crates/gpui + gpui_wgpu + gpui_web + gpui_ios\"]",
    "    NSHC[\"crates/nsh-core / nsh-base / nsh-js / nsh-platform-gpui\"]",
    "    GPUIUI[\"crates/ui + crates/gpui_ui + crates/gpui_app_shell\"]",
    "    OSC[\"crates/aphrody_craft + aphrody_ui + aphrody_desktop\"]",
    "    ACP[\"crates/acp_thread + context_server\"]",
    "  end",
    "",
    "  ZED -->|ABSORBED_FLATTENED| GPUI",
    "  ZED -->|ABSORBED_FLATTENED| GPUIUI",
    "  NSH -->|ABSORBED_FLATTENED| NSHC",
    "  NSHC -->|bridged via nsh.rs| GPUIUI",
    "  QJS -->|workspace dep| NSHC",
    "  I18N -->|workspace dep| NSHC",
    "  GPUI -->|patch.crates-io| TAFFY",
    "  GPUI -->|patch.crates-io| WGPU",
    "  NSHC -->|workspace dep| WRY",
    "  KIT -.->|MERGE_CANDIDATE| GPUIUI",
    "  FAST -.->|MERGE_CANDIDATE| GPUI",
    "  MOB -.->|MERGE_CANDIDATE| GPUI",
    "  WEF -.->|MERGE_CANDIDATE| NSHC",
    "  WEF -.->|aligns with| CEF",
    "  PUR -.->|MERGE_CANDIDATE| OSC",
    "  PUR -.->|MCP / rmcp| ACP",
    "  GPUIUI --> OSC",
    "  GPUI --> GPUIUI",
    "```",
  ];
  // Mention live node/edge totals in comment for quick auditability.
  lines.push(
    "",
    `> Graphify snapshot: **${graph.nodes.length}** deduplicated nodes, **${graph.edges.length}** typed edges, **${graph.mergeCandidates.length}** merge candidates, **${graph.summary.duplicateGraphCrateCount}** multi-version crates (**${graph.summary.duplicateMajorMinorCount}** major.minor splits).`,
  );
  return lines.join("\n");
}

export function updateArchitectureDoc(graph: GraphifyDocument, docPath = join(REPO_ROOT, ARCH_DOC_REL)): void {
  if (!existsSync(docPath)) return;
  let content = readFileSync(docPath, "utf8");

  const metricsBlock = [
    "<!-- BEGIN LONGBRIDGE_INGEST_METRICS -->",
    "| Telemetry Dimension | Live Verified Count / Value | Source / Audit Path |",
    "| :--- | :--- | :--- |",
    `| **Longbridge Org Repos Scanned** | \`${graph.summary.longbridgeRepoCount}\` repos (\`${graph.summary.absorbedCount}\` absorbed, \`${graph.summary.mergeCandidateCount}\` merge candidates, \`${graph.summary.referenceBenchShowcaseCount}\` ref/bench/showcase, \`${graph.summary.infraOutOfScopeCount}\` infra) | \`gh api --paginate orgs/longbridge/repos\` |`,
    `| **Community GPUI Repos Tracked** | \`${graph.summary.communityGpuiCount}\` repositories | GitHub Search + \`crates.io\` |`,
    `| **Aphrody Workspace Crates** | \`${graph.summary.workspaceCrateCount}\` members (\`${graph.summary.featureMatrixCrateCount}\` core GPUI/NSH/OS crates in feature matrix) | \`cargo metadata --format-version 1\` |`,
    `| **Cargo Dependency Centralization** | \`${graph.summary.nonWorkspaceDepCount}\` non-workspace dep entries across \`${graph.summary.uniqueNonWorkspaceDepCount}\` unique deps; \`${graph.summary.missingWorkspaceLintsCount}\` crates missing \`[lints] workspace = true\` | Root [\`Cargo.toml\`](../../../Cargo.toml) + member manifests |`,
    `| **Resolved Graph Duplicates** | \`${graph.summary.duplicateGraphCrateCount}\` crates with >1 resolved version (\`${graph.summary.duplicateMajorMinorCount}\` major.minor splits) | \`target/longbridge-ingest/graph.json\` |`,
    `| **Live Sitemaps & Screenshots** | \`${graph.summary.sitemapTargetsCount}\` showcase/docs sites (\`${graph.summary.sitemapTotalUrls}\` discovered URLs), \`${graph.summary.screenshotSuccessCount}/${graph.summary.screenshotCount}\` validated PNG captures | \`target/longbridge-screenshots/*.png\` |`,
    "<!-- END LONGBRIDGE_INGEST_METRICS -->",
  ].join("\n");

  const mermaidBlock = [
    "<!-- BEGIN LONGBRIDGE_INGEST_MERMAID -->",
    buildMermaidTopology(graph),
    "<!-- END LONGBRIDGE_INGEST_MERMAID -->",
  ].join("\n");

  content = content.replace(
    /<!-- BEGIN LONGBRIDGE_INGEST_METRICS -->[\s\S]*?<!-- END LONGBRIDGE_INGEST_METRICS -->/,
    metricsBlock,
  );
  content = content.replace(
    /<!-- BEGIN LONGBRIDGE_INGEST_MERMAID -->[\s\S]*?<!-- END LONGBRIDGE_INGEST_MERMAID -->/,
    mermaidBlock,
  );
  writeFileSync(docPath, content, "utf8");
}

// ---------------------------------------------------------------------------
// Artifact Cache Persistence under gitignored `target/longbridge-ingest/`
// ---------------------------------------------------------------------------

function writeTargetJson(relPath: string, data: unknown): string {
  const full = join(REPO_ROOT, relPath);
  mkdirSync(dirname(full), { recursive: true });
  writeFileSync(full, `${JSON.stringify(data, null, 2)}\n`, "utf8");
  return full;
}

function readTargetJsonIfExists<T>(relPath: string): T | null {
  const full = join(REPO_ROOT, relPath);
  if (!existsSync(full)) return null;
  try {
    return JSON.parse(readFileSync(full, "utf8")) as T;
  } catch {
    return null;
  }
}

function discoverWebsiteTargets(scan: ScanReport | null): ScreenshotTarget[] {
  const byNormalized = new Map<string, ScreenshotTarget>();
  const norm = (u: string) => u.replace(/^http:\/\//i, "https://").replace(/\/+$/, "");

  for (const target of DEFAULT_WEBSITE_TARGETS) {
    byNormalized.set(norm(target.url), target);
  }
  if (!scan) return Array.from(byNormalized.values());

  const candidateRepos = [
    ...scan.longbridgeRepos.filter(
      (r) =>
        r.classification !== "INFRA_OUT_OF_SCOPE" &&
        !r.archived &&
        r.homepage &&
        /^https?:\/\//i.test(r.homepage) &&
        !r.homepage.includes("github.com") &&
        !r.homepage.includes("marketplace") &&
        !r.homepage.includes("omarchyplugins.com"),
    ),
  ];

  for (const repo of candidateRepos) {
    if (byNormalized.size >= 12) break;
    const key = norm(repo.homepage);
    if (byNormalized.has(key)) continue;
    const slug = repo.name.toLowerCase().replace(/[^a-z0-9-]+/g, "-");
    byNormalized.set(key, {
      id: `repo-${slug}`,
      url: repo.homepage,
      filename: `${slug}.png`,
      description: `${repo.fullName} homepage (${repo.classification})`,
    });
  }
  return Array.from(byNormalized.values());
}

function syncGraphSiteTelemetry(
  graphDoc: GraphifyDocument | null,
  sitemaps: SitemapCrawlResult[],
  screenshots: ScreenshotResult[],
): GraphifyDocument | null {
  if (!graphDoc) return null;
  const totalSitemapUrls = computeDeduplicatedSitemapCount(sitemaps);
  graphDoc.summary.sitemapTargetsCount = sitemaps.length;
  graphDoc.summary.sitemapTotalUrls = totalSitemapUrls;
  graphDoc.summary.screenshotCount = screenshots.length;
  graphDoc.summary.screenshotSuccessCount = screenshots.filter((s) => s.validPng && s.status === "ok").length;
  writeTargetJson(GRAPH_OUT_REL, graphDoc);
  updateArchitectureDoc(graphDoc);
  return graphDoc;
}

// ---------------------------------------------------------------------------
// CLI Entrypoint
// ---------------------------------------------------------------------------

async function main(): Promise<void> {
  const args = Bun.argv.slice(2);
  const cmd = args.find((a) => !a.startsWith("-")) ?? "all";
  const validCmds = new Set(["scan", "graph", "centralize", "sitemaps", "screenshots", "all"]);
  if (!validCmds.has(cmd)) {
    console.error(`Unknown subcommand '${cmd}'. Expected one of: ${Array.from(validCmds).join(", ")}`);
    process.exit(1);
  }

  let scanReport = readTargetJsonIfExists<ScanReport>(SCAN_OUT_REL);
  let sitemaps = readTargetJsonIfExists<SitemapCrawlResult[]>(SITEMAPS_OUT_REL) ?? [];
  let screenshots = readTargetJsonIfExists<ScreenshotResult[]>(SCREENSHOTS_OUT_REL) ?? [];
  let graphDoc = readTargetJsonIfExists<GraphifyDocument>(GRAPH_OUT_REL);

  if (cmd === "scan" || cmd === "all") {
    scanReport = await runScanCommand(REPO_ROOT);
    writeTargetJson(SCAN_OUT_REL, scanReport);
    writeInventoryToml(scanReport, graphDoc, sitemaps, screenshots);
    if (cmd === "scan") {
      console.log(
        JSON.stringify(
          {
            command: "scan",
            longbridgeTotalRepos: scanReport.longbridgeTotalRepos,
            countsByClassification: scanReport.countsByClassification,
            communityGpuiCount: scanReport.communityGpuiRepos.length,
            upstreamZedSha: scanReport.upstreamZed.enriched?.latestCommitSha.slice(0, 12) ?? "",
            inventoryPath: INVENTORY_TOML_REL,
            scanCachePath: SCAN_OUT_REL,
          },
          null,
          2,
        ),
      );
      return;
    }
  }

  if (cmd === "centralize") {
    const meta = await runCargoMetadata(REPO_ROOT);
    const audit = auditCentralization(meta, REPO_ROOT);
    writeTargetJson(CENTRALIZE_OUT_REL, audit);
    console.log(
      JSON.stringify(
        {
          command: "centralize",
          workspaceMemberCount: audit.workspaceMemberCount,
          auditedMemberCount: audit.auditedMemberCount,
          extensionCountSkipped: audit.extensionCountSkipped,
          missingWorkspaceLintsCount: audit.missingWorkspaceLints.length,
          missingWorkspaceLints: audit.missingWorkspaceLints,
          nonWorkspaceDepsTotal: audit.nonWorkspaceDependencies.length,
          nonWorkspaceTopLevelCount: audit.nonWorkspaceTopLevelCount,
          nonWorkspaceTargetLevelCount: audit.nonWorkspaceTargetLevelCount,
          uniqueNonWorkspaceDepCount: audit.uniqueNonWorkspaceDepNames.length,
          workspaceDependencyBreakdown: {
            total: audit.workspaceDependencyBreakdown.total,
            pathCount: audit.workspaceDependencyBreakdown.pathCount,
            gitCount: audit.workspaceDependencyBreakdown.gitCount,
            registryCount: audit.workspaceDependencyBreakdown.registryCount,
          },
          patchBreakdown: {
            total: audit.patchBreakdown.total,
            pathCount: audit.patchBreakdown.pathCount,
            gitCount: audit.patchBreakdown.gitCount,
          },
          duplicatePackageCount: audit.duplicatePackageCount,
          duplicateMajorMinorCount: audit.duplicateMajorMinorCount,
          topDuplicates: audit.duplicatePackages.slice(0, 15),
          centralizeCachePath: CENTRALIZE_OUT_REL,
        },
        null,
        2,
      ),
    );
    return;
  }

  if (cmd === "sitemaps" || cmd === "all") {
    const targets = discoverWebsiteTargets(scanReport);
    sitemaps = await crawlSites(targets, 4);
    writeTargetJson(SITEMAPS_OUT_REL, sitemaps);
    if (cmd === "sitemaps") {
      graphDoc = syncGraphSiteTelemetry(graphDoc, sitemaps, screenshots);
      writeInventoryToml(scanReport, graphDoc, sitemaps, screenshots);
      console.log(
        JSON.stringify(
          {
            command: "sitemaps",
            siteCount: sitemaps.length,
            totalDiscoveredUrls: computeDeduplicatedSitemapCount(sitemaps),
            sites: sitemaps.map((s) => ({
              id: s.id,
              url: s.url,
              httpStatus: s.httpStatus,
              sitemapStatus: s.sitemapStatus,
              sitemapUrlCount: s.sitemapUrlCount,
              internalLinkCount: s.internalLinkCount,
              totalDiscoveredUrls: s.totalDiscoveredUrls,
              status: s.status,
              error: s.error,
            })),
          },
          null,
          2,
        ),
      );
      return;
    }
  }

  if (cmd === "screenshots" || cmd === "all") {
    screenshots = await captureSiteScreenshots(DEFAULT_WEBSITE_TARGETS, join(REPO_ROOT, SCREENSHOTS_DIR_REL));
    writeTargetJson(SCREENSHOTS_OUT_REL, screenshots);
    if (cmd === "screenshots") {
      graphDoc = syncGraphSiteTelemetry(graphDoc, sitemaps, screenshots);
      writeInventoryToml(scanReport, graphDoc, sitemaps, screenshots);
      console.log(
        JSON.stringify(
          {
            command: "screenshots",
            total: screenshots.length,
            successCount: screenshots.filter((s) => s.validPng && s.status === "ok").length,
            screenshots,
          },
          null,
          2,
        ),
      );
      return;
    }
  }

  if (cmd === "graph" || cmd === "all") {
    const meta = await runCargoMetadata(REPO_ROOT);
    const localIndex = loadLocalAphrodyIndex(REPO_ROOT);
    const audit = auditCentralization(meta, REPO_ROOT);
    writeTargetJson(CENTRALIZE_OUT_REL, audit);
    const featureMatrix = extractFeatureMatrix(meta, audit, REPO_ROOT);
    graphDoc = buildGraphifyDocument(scanReport, meta, audit, featureMatrix, localIndex, sitemaps, screenshots, REPO_ROOT);
    writeTargetJson(GRAPH_OUT_REL, graphDoc);
    writeInventoryToml(scanReport, graphDoc, sitemaps, screenshots);
    updateArchitectureDoc(graphDoc);
    console.log(
      JSON.stringify(
        {
          command: cmd,
          summary: graphDoc.summary,
          nodeCount: graphDoc.nodes.length,
          edgeCount: graphDoc.edges.length,
          graphPath: GRAPH_OUT_REL,
          inventoryPath: INVENTORY_TOML_REL,
          archDocPath: ARCH_DOC_REL,
        },
        null,
        2,
      ),
    );
  }
}

if (import.meta.main) {
  await main();
}
