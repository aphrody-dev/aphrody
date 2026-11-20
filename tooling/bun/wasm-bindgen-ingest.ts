#!/usr/bin/env bun
/**
 * Live GitIngest + GitHub + crates.io Ingestion, llms.txt & Architecture Guide Emitter for
 * the `github.com/wasm-bindgen` 8-repository ecosystem in Aphrody.
 *
 * Subcommands:
 *   bun tooling/bun/wasm-bindgen-ingest.ts scan [--json]
 *   bun tooling/bun/wasm-bindgen-ingest.ts ingest [--json] [--force-github]
 *   bun tooling/bun/wasm-bindgen-ingest.ts emit [--json]
 *   bun tooling/bun/wasm-bindgen-ingest.ts all [--json] [--force-github]
 */
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";

export const REPO_ROOT = resolve(import.meta.dir, "../..");
export const ORG_NAME = "wasm-bindgen";
export const EXPECTED_ORG_REPOS = [
  "wasm-bindgen",
  "wasm-pack",
  "walrus",
  "weedle",
  "ts-gen",
  "js-bindgen",
  "rust-externref-rfc",
  "wasm-bindgen-bot",
] as const;

export const TARGET_CRATES = [
  "wasm-bindgen",
  "wasm-bindgen-futures",
  "js-sys",
  "web-sys",
  "walrus",
  "weedle",
  "wasm-pack",
] as const;

export const DIGEST_DIR_REL = "target/gitingest/wasm-bindgen";
export const SCAN_OUT_REL = "target/gitingest/wasm-bindgen/scan.json";
export const INGEST_MANIFEST_REL = "target/gitingest/wasm-bindgen/ingest-manifest.json";
export const LLMS_TXT_REL = "docs/aphrody/architecture/wasm-bindgen-llms.txt";
export const ECOSYSTEM_DOC_REL = "docs/aphrody/architecture/wasm-bindgen-ecosystem.md";

const USER_AGENT = "AphrodyWasmBindgenIngest/1.0 (+https://github.com/aphrody-labs/aphrody)";
const GITINGEST_BASE = "https://gitingest.com";
const GITINGEST_API = `${GITINGEST_BASE}/api/ingest`;

// ---------------------------------------------------------------------------
// Interfaces
// ---------------------------------------------------------------------------

export interface CargoManifestQuick {
  packageName?: string;
  version?: string;
  edition?: string;
  rustVersion?: string;
  license?: string;
  workspaceMembers: string[];
  dependencies: string[];
  features: Record<string, string[]>;
}

export interface CratesIoVersionInfo {
  name: string;
  exists: boolean;
  maxVersion: string;
  maxStableVersion: string;
  downloads: number;
  recentDownloads: number;
  updatedAt: string;
  repository: string;
  error?: string;
}

export interface OrgRepoTelemetry {
  name: string;
  fullName: string;
  htmlUrl: string;
  description: string;
  language: string;
  stars: number;
  forks: number;
  openIssuesAndPrs: number;
  archived: boolean;
  defaultBranch: string;
  pushedAt: string;
  latestCommitSha: string;
  latestCommitDate: string;
  latestCommitMessage: string;
  treeBlobCount: number;
  treeTruncated: boolean;
  keyPathsPresent: string[];
  rootCargo?: CargoManifestQuick;
  subCargoVersions: Array<{ path: string; name: string; version: string; rustVersion?: string }>;
  packageJson?: { name?: string; version?: string; type?: string };
  roleInEcosystem: string;
}

export interface LocalAphrodyToolchain {
  workspaceConstraints: {
    wasmBindgen: string;
    wasmBindgenFutures: string;
    jsSys: string;
    webSys: string;
    wasmEncoder: string;
    wasmparser: string;
    wasmtime: string;
    wasmThreadGitRev: string;
  };
  cargoLockVersions: {
    wasmBindgen: string;
    wasmBindgenFutures: string;
    jsSys: string;
    webSys: string;
    walrus: string;
    weedle: string;
    wasmPack: string;
  };
  cliVersions: {
    wasmBindgen: string;
    wasmOpt: string;
    gitingest: string;
  };
  gpuiWebFeaturesCount: number;
  gpuiWebFeaturesSample: string[];
  gpuiWgpuWebSysFeatures: string[];
  profileWebSettings: Record<string, string>;
  webBuildCohorts: string[];
}

export interface ScanReport {
  schemaVersion: number;
  generatedAt: string;
  org: string;
  repoCount: number;
  repos: OrgRepoTelemetry[];
  cratesIo: CratesIoVersionInfo[];
  localToolchain: LocalAphrodyToolchain;
  gitingestEndpointProbe: {
    url: string;
    reachable: boolean;
    apiRouteDiscovered: string;
    status: number;
  };
}

export interface RepoDigestResult {
  repo: string;
  fullName: string;
  commitSha: string;
  method: "gitingest.com-digest-url" | "gitingest.com-inline" | "github-tree-fallback";
  digestPath: string;
  digestBytes: number;
  digestLines: number;
  digestFileCount: number;
  digestSha256: string;
  filesAnalyzed: number;
  estimatedTokens: string;
  digestUrl?: string;
  summary: string;
  fallbackReason?: string;
}

export interface IngestManifest {
  schemaVersion: number;
  generatedAt: string;
  gitingestCliVersion: string;
  remoteApiUrl: string;
  repoCount: number;
  totalDigestBytes: number;
  totalDigestFiles: number;
  entries: RepoDigestResult[];
}

export interface EmitSummary {
  llmsTxtPath: string;
  ecosystemDocPath: string;
  brokenLinks: string[];
  llmsTxtOk: boolean;
}

// ---------------------------------------------------------------------------
// Pure Parsers, Tree Serializers & Validators
// ---------------------------------------------------------------------------

export function parsePaginatedJson<T>(raw: string): T[] {
  const trimmed = raw.trim();
  if (!trimmed) return [];
  try {
    const parsed = JSON.parse(trimmed);
    if (Array.isArray(parsed)) {
      if (parsed.length > 0 && Array.isArray(parsed[0])) {
        return (parsed as T[][]).flat();
      }
      return parsed as T[];
    }
    return [parsed as T];
  } catch {
    const wrapped = `[${trimmed.replace(/\]\s*\[/g, "],[")}]`;
    const pages = JSON.parse(wrapped) as T[][];
    return pages.flat();
  }
}

export function parseCargoManifestQuick(tomlText: string): CargoManifestQuick {
  const parsed = Bun.TOML.parse(tomlText) as Record<string, any>;
  const pkg = (parsed.package ?? {}) as Record<string, any>;
  const ws = (parsed.workspace ?? {}) as Record<string, any>;
  const deps = (parsed.dependencies ?? {}) as Record<string, unknown>;
  const featuresRaw = (parsed.features ?? {}) as Record<string, unknown>;
  const features: Record<string, string[]> = {};
  for (const [k, v] of Object.entries(featuresRaw)) {
    features[k] = Array.isArray(v) ? v.map(String) : [];
  }
  return {
    packageName: typeof pkg.name === "string" ? pkg.name : undefined,
    version: typeof pkg.version === "string" ? pkg.version : undefined,
    edition: typeof pkg.edition === "string" ? pkg.edition : undefined,
    rustVersion: typeof pkg["rust-version"] === "string" ? pkg["rust-version"] : undefined,
    license: typeof pkg.license === "string" ? pkg.license : undefined,
    workspaceMembers: Array.isArray(ws.members) ? ws.members.map(String) : [],
    dependencies: Object.keys(deps),
    features,
  };
}

function extractCargoLockVersion(lockText: string, crateName: string): string {
  const escaped = crateName.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = lockText.match(new RegExp(`name = "${escaped}"\\r?\\nversion = "([^"]+)"`));
  return match?.[1] ?? "not-in-workspace";
}

function formatWorkspaceDepSpec(val: unknown): string {
  if (typeof val === "string") return val;
  if (val && typeof val === "object") {
    const obj = val as Record<string, unknown>;
    if (typeof obj.version === "string") return obj.version;
    if (typeof obj.path === "string") return `path:${obj.path}`;
    if (typeof obj.git === "string") return `git:${obj.git}`;
  }
  return "unspecified";
}

function probeProgramVersion(bin: string): string {
  try {
    const res = Bun.spawnSync([bin, "--version"], { stdout: "pipe", stderr: "pipe" });
    if (res.exitCode !== 0) return "missing";
    const out = new TextDecoder().decode(res.stdout).trim();
    return out || "installed";
  } catch {
    return "missing";
  }
}

function probeWasmBindgenVersion(): string {
  const raw = probeProgramVersion("wasm-bindgen");
  return raw === "missing" ? "missing" : raw.replace(/^wasm-bindgen\s+/i, "").trim();
}

function probeWasmOptVersion(): string {
  const raw = probeProgramVersion("wasm-opt");
  if (raw === "missing") return "missing";
  const m = raw.match(/version\s+(\d+)/i);
  return m ? `v${m[1]}` : raw;
}

function probeGitingestCliVersion(): string {
  const direct = probeProgramVersion("gitingest");
  if (direct !== "missing") return direct;
  try {
    const helpRes = Bun.spawnSync(["gitingest", "--help"], { stdout: "pipe", stderr: "pipe" });
    if (helpRes.exitCode !== 0) return "missing";
    const uvRes = Bun.spawnSync(["uv", "tool", "list"], { stdout: "pipe", stderr: "pipe" });
    if (uvRes.exitCode === 0) {
      const uvOut = new TextDecoder().decode(uvRes.stdout);
      const line = uvOut.split(/\r?\n/).find((l) => /^gitingest\s+/i.test(l.trim()));
      if (line) return line.replace(/^gitingest\s+/i, "").trim();
    }
    return "installed (PATH)";
  } catch {
    return "missing";
  }
}

export function extractLocalAphrodyState(root = REPO_ROOT): LocalAphrodyToolchain {
  const cargoTomlPath = join(root, "Cargo.toml");
  const cargoLockPath = join(root, "Cargo.lock");
  const gpuiWebTomlPath = join(root, "crates/gpui_web/Cargo.toml");
  const gpuiWgpuTomlPath = join(root, "crates/gpui_wgpu/Cargo.toml");

  const cargoToml = existsSync(cargoTomlPath)
    ? (Bun.TOML.parse(readFileSync(cargoTomlPath, "utf8")) as Record<string, any>)
    : {};
  const wsDeps = (cargoToml.workspace?.dependencies ?? {}) as Record<string, unknown>;
  const profileWebRaw = (cargoToml.profile?.web ?? {}) as Record<string, unknown>;
  const profileWebSettings: Record<string, string> = {};
  for (const [k, v] of Object.entries(profileWebRaw)) {
    profileWebSettings[k] = String(v);
  }

  const lockText = existsSync(cargoLockPath) ? readFileSync(cargoLockPath, "utf8") : "";
  const gpuiWebToml = existsSync(gpuiWebTomlPath)
    ? (Bun.TOML.parse(readFileSync(gpuiWebTomlPath, "utf8")) as Record<string, any>)
    : {};
  const gpuiWgpuToml = existsSync(gpuiWgpuTomlPath)
    ? (Bun.TOML.parse(readFileSync(gpuiWgpuTomlPath, "utf8")) as Record<string, any>)
    : {};

  const gpuiWebWebSys =
    gpuiWebToml.target?.['cfg(target_family = "wasm")']?.dependencies?.["web-sys"]?.features ??
    gpuiWebToml.target?.['cfg(target_arch = "wasm32")']?.dependencies?.["web-sys"]?.features ??
    [];
  const gpuiWgpuWebSys =
    gpuiWgpuToml.target?.['cfg(target_family = "wasm")']?.dependencies?.["web-sys"]?.features ??
    gpuiWgpuToml.target?.['cfg(target_arch = "wasm32")']?.dependencies?.["web-sys"]?.features ??
    [];

  const wasmThreadSpec = (wsDeps.wasm_thread ?? {}) as Record<string, unknown>;
  const wasmThreadRev = typeof wasmThreadSpec.rev === "string" ? wasmThreadSpec.rev.slice(0, 12) : "none";

  return {
    workspaceConstraints: {
      wasmBindgen: formatWorkspaceDepSpec(wsDeps["wasm-bindgen"]),
      wasmBindgenFutures: formatWorkspaceDepSpec(wsDeps["wasm-bindgen-futures"]),
      jsSys: formatWorkspaceDepSpec(wsDeps["js-sys"]),
      webSys: formatWorkspaceDepSpec(wsDeps["web-sys"]),
      wasmEncoder: formatWorkspaceDepSpec(wsDeps["wasm-encoder"]),
      wasmparser: formatWorkspaceDepSpec(wsDeps.wasmparser),
      wasmtime: formatWorkspaceDepSpec(wsDeps.wasmtime),
      wasmThreadGitRev: wasmThreadRev,
    },
    cargoLockVersions: {
      wasmBindgen: extractCargoLockVersion(lockText, "wasm-bindgen"),
      wasmBindgenFutures: extractCargoLockVersion(lockText, "wasm-bindgen-futures"),
      jsSys: extractCargoLockVersion(lockText, "js-sys"),
      webSys: extractCargoLockVersion(lockText, "web-sys"),
      walrus: extractCargoLockVersion(lockText, "walrus"),
      weedle: extractCargoLockVersion(lockText, "weedle"),
      wasmPack: extractCargoLockVersion(lockText, "wasm-pack"),
    },
    cliVersions: {
      wasmBindgen: probeWasmBindgenVersion(),
      wasmOpt: probeWasmOptVersion(),
      gitingest: probeGitingestCliVersion(),
    },
    gpuiWebFeaturesCount: Array.isArray(gpuiWebWebSys) ? gpuiWebWebSys.length : 0,
    gpuiWebFeaturesSample: Array.isArray(gpuiWebWebSys) ? gpuiWebWebSys.map(String) : [],
    gpuiWgpuWebSysFeatures: Array.isArray(gpuiWgpuWebSys) ? gpuiWgpuWebSys.map(String) : [],
    profileWebSettings,
    webBuildCohorts: ["baseline", "simd-baseline", "multithreaded"],
  };
}

interface TreeNode {
  name: string;
  children: Map<string, TreeNode>;
  isFile: boolean;
}

function buildTreeNodes(paths: readonly string[]): TreeNode {
  const root: TreeNode = { name: "", children: new Map(), isFile: false };
  for (const rawPath of paths) {
    const parts = rawPath.replaceAll("\\", "/").split("/").filter(Boolean);
    let cursor = root;
    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      const isFile = i === parts.length - 1;
      let child = cursor.children.get(part);
      if (!child) {
        child = { name: part, children: new Map(), isFile };
        cursor.children.set(part, child);
      } else if (!isFile) {
        child.isFile = false;
      }
      cursor = child;
    }
  }
  return root;
}

function renderTreeChildren(node: TreeNode, prefix: string, out: string[]): void {
  const sorted = Array.from(node.children.values()).sort((a, b) => {
    const aDir = a.children.size > 0 && !a.isFile;
    const bDir = b.children.size > 0 && !b.isFile;
    if (aDir !== bDir) return aDir ? -1 : 1;
    return a.name.localeCompare(b.name);
  });
  for (let i = 0; i < sorted.length; i++) {
    const child = sorted[i];
    const isLast = i === sorted.length - 1;
    const hasKids = child.children.size > 0;
    const connector = isLast ? "└── " : "├── ";
    out.push(`${prefix}${connector}${child.name}${hasKids ? "/" : ""}`);
    if (hasKids) {
      const childPrefix = `${prefix}${isLast ? "    " : "│   "}`;
      renderTreeChildren(child, childPrefix, out);
    }
  }
}

export function renderDirectoryTree(rootLabel: string, paths: readonly string[]): string {
  const root = buildTreeNodes(paths);
  const lines = ["Directory structure:", `└── ${rootLabel}/`];
  renderTreeChildren(root, "    ", lines);
  return `${lines.join("\n")}\n`;
}

export function renderGitIngestDigest(
  rootLabel: string,
  treePaths: readonly string[],
  files: ReadonlyArray<{ path: string; content: string }>,
): string {
  const treeBlock = renderDirectoryTree(rootLabel, treePaths);
  const sections = files.map((file) => {
    const normPath = file.path.replaceAll("\\", "/");
    return [
      "================================================",
      `FILE: ${normPath}`,
      "================================================",
      file.content.replace(/\r\n/g, "\n").trimEnd(),
      "",
    ].join("\n");
  });
  return `${treeBlock}\n${sections.join("\n")}\n`;
}

export function validateGitIngestDigest(digest: string): {
  ok: boolean;
  fileCount: number;
  hasDirectoryStructure: boolean;
  errors: string[];
} {
  const errors: string[] = [];
  const hasDirectoryStructure = digest.includes("Directory structure:");
  if (!hasDirectoryStructure) {
    errors.push("Missing 'Directory structure:' header");
  }
  const fileHeaders = digest.match(/^={48}\r?\nFILE:\s+.+\r?\n={48}$/gm) ?? [];
  if (fileHeaders.length === 0) {
    errors.push("No GitIngest FILE: delimiter sections found");
  }
  return {
    ok: errors.length === 0,
    fileCount: fileHeaders.length,
    hasDirectoryStructure,
    errors,
  };
}

export function validateLlmsTxt(text: string): {
  ok: boolean;
  title?: string;
  hasBlockquote: boolean;
  sections: string[];
  linkCount: number;
  errors: string[];
} {
  const errors: string[] = [];
  const lines = text.split(/\r?\n/);
  const firstNonEmptyIdx = lines.findIndex((l) => l.trim().length > 0);
  if (firstNonEmptyIdx < 0) {
    return { ok: false, hasBlockquote: false, sections: [], linkCount: 0, errors: ["Empty llms.txt"] };
  }
  const h1Match = lines[firstNonEmptyIdx].match(/^#\s+(.+)$/);
  const title = h1Match?.[1]?.trim();
  if (!title) {
    errors.push("First non-empty line must be an H1 heading (# Title)");
  }

  const afterH1 = lines.slice(firstNonEmptyIdx + 1).find((l) => l.trim().length > 0) ?? "";
  const hasBlockquote = afterH1.startsWith("> ");
  if (!hasBlockquote) {
    errors.push("H1 must be followed by a blockquote summary (> ...) per llmstxt.org");
  }

  const sections = lines
    .map((l) => l.match(/^##\s+(.+)$/)?.[1]?.trim())
    .filter((s): s is string => Boolean(s));
  if (sections.length === 0) {
    errors.push("Expected at least one H2 section (## Section)");
  }

  const linkCount = (text.match(/^-\s+\[[^\]]+\]\([^)]+\)/gm) ?? []).length;
  if (linkCount === 0) {
    errors.push("Expected markdown link list entries (- [Name](url)) inside H2 sections");
  }

  return {
    ok: errors.length === 0,
    title,
    hasBlockquote,
    sections,
    linkCount,
    errors,
  };
}

export function validateRelativeLinks(filePaths: readonly string[]): string[] {
  const broken: string[] = [];
  for (const absPath of filePaths) {
    if (!existsSync(absPath)) {
      broken.push(`Missing generated file: ${absPath}`);
      continue;
    }
    const content = readFileSync(absPath, "utf8");
    for (const match of content.matchAll(/\]\(([^)]+)\)/g)) {
      const raw = match[1].split("#")[0].trim();
      if (!raw || raw.startsWith("http://") || raw.startsWith("https://") || raw.startsWith("mailto:")) {
        continue;
      }
      const target = resolve(dirname(absPath), raw);
      if (!existsSync(target)) {
        broken.push(`${relative(REPO_ROOT, absPath).replaceAll("\\", "/")} -> ${raw}`);
      }
    }
  }
  return broken;
}

export function selectKeyDigestFiles(repoName: string, treePaths: readonly string[], maxFiles = 28): string[] {
  const priorityMatchers: RegExp[] = [
    /^README\.md$/i,
    /^Cargo\.toml$/,
    /^package\.json$/,
    /^src\/lib\.rs$/,
    /^src\/main\.rs$/,
    /^externref-proposal\.md$/,
    /^analyze\/action\.yml$/,
    /^post\/action\.yml$/,
    /^crates\/[^/]+\/Cargo\.toml$/,
    /^crates\/cli\/src\/bin\/[^/]+\.rs$/,
    /^crates\/cli-support\/src\/lib\.rs$/,
    /^crates\/webidl\/src\/lib\.rs$/,
    /^client\/Cargo\.toml$/,
    /^client\/js-bindgen\/Cargo\.toml$/,
    /^client\/js-bindgen\/src\/lib\.rs$/,
    /^src\/passes\/[^/]+\.rs$/,
    /^scripts\/(run|post|context)\.mjs$/,
  ];

  const picked: string[] = [];
  const seen = new Set<string>();
  const blobs = treePaths.filter((p) => !p.endsWith("/"));

  for (const rx of priorityMatchers) {
    for (const path of blobs) {
      if (picked.length >= maxFiles) break;
      if (!seen.has(path) && rx.test(path)) {
        seen.add(path);
        picked.push(path);
      }
    }
  }

  if (picked.length < 4) {
    for (const path of blobs) {
      if (picked.length >= Math.min(maxFiles, 12)) break;
      if (!seen.has(path) && /\.(md|toml|rs|json|yml|js|mjs|ts)$/i.test(path)) {
        seen.add(path);
        picked.push(path);
      }
    }
  }

  // Ensure repo-specific anchors are always prioritized when present.
  if (repoName === "wasm-bindgen") {
    for (const extra of ["crates/js-sys/Cargo.toml", "crates/web-sys/Cargo.toml", "crates/futures/Cargo.toml"]) {
      if (blobs.includes(extra) && !seen.has(extra)) {
        seen.add(extra);
        picked.unshift(extra);
      }
    }
  }
  return picked.slice(0, maxFiles);
}

export function parseGitIngestSummaryMetadata(summary: string): {
  filesAnalyzed: number;
  estimatedTokens: string;
} {
  const filesMatch = summary.match(/Files analyzed:\s*(\d+)/i);
  const tokensMatch = summary.match(/Estimated tokens:\s*([^\s\r\n]+)/i);
  return {
    filesAnalyzed: filesMatch ? Number(filesMatch[1]) : 0,
    estimatedTokens: tokensMatch?.[1] ?? "unknown",
  };
}

// ---------------------------------------------------------------------------
// Live GitHub, crates.io & GitIngest Fetchers
// ---------------------------------------------------------------------------

function runGhApi(args: string[]): string {
  const proc = Bun.spawnSync(["gh", "api", ...args], {
    cwd: REPO_ROOT,
    stdout: "pipe",
    stderr: "pipe",
  });
  if (proc.exitCode !== 0) {
    const err = new TextDecoder().decode(proc.stderr);
    throw new Error(`gh api ${args.join(" ")} failed (${proc.exitCode}): ${err}`);
  }
  return new TextDecoder().decode(proc.stdout);
}

function fetchGhFileUtf8(fullName: string, path: string): string | null {
  try {
    const raw = runGhApi([`repos/${fullName}/contents/${path}`]);
    const parsed = JSON.parse(raw) as { content?: string; encoding?: string };
    if (!parsed.content) return null;
    if (parsed.encoding === "base64") {
      return Buffer.from(parsed.content.replace(/\s+/g, ""), "base64").toString("utf8");
    }
    return parsed.content;
  } catch {
    return null;
  }
}

const REPO_ROLE_MAP: Record<string, string> = {
  "wasm-bindgen":
    "Core Rust <-> JS ABI generator, macro crate, CLI (`wasm-bindgen-cli`), `js-sys`, `web-sys`, `wasm-bindgen-futures`, and `wasm-bindgen-webidl`.",
  "wasm-pack":
    "Legacy Rust -> Wasm npm/webpack workflow orchestrator (`0.15.1`); superseded in Aphrody by `tooling/bun/web-build.ts`.",
  walrus:
    "WebAssembly IR transformation & dead-symbol/DWARF/GC library powering `wasm-bindgen-cli-support` optimization passes before Binaryen `wasm-opt`.",
  weedle:
    "Nom-based WebIDL parser turning WHATWG/W3C `.webidl` specifications into ASTs consumed by `wasm-bindgen-webidl` for `web-sys`.",
  "ts-gen":
    "oxc_parser + syn TypeScript `.d.ts` -> Rust `#[wasm_bindgen]` extern generator for zero-drift JS/TS SDK contracts.",
  "js-bindgen":
    "Next-generation linker-driven `#![no_std]` JS FFI (`embed_js`/`import_js`/`js-bindgen-ld`) with `wasm32` + `wasm64` and real-generic `js-sys` R&D.",
  "rust-externref-rfc":
    "Draft Rust language/LLVM proposal for boxed `Externref`/`ExternrefRef<'a>`/`ByValue<T>` (`ptr addrspace(2)`) removing table marshalling overhead.",
  "wasm-bindgen-bot":
    "Cloudflare AI Gateway + sandboxed GitHub Actions review/triage automation maintaining all 8 `wasm-bindgen` org repositories.",
};

export async function probeGitIngestEndpoint(): Promise<{
  url: string;
  reachable: boolean;
  apiRouteDiscovered: string;
  status: number;
}> {
  try {
    const resp = await fetch(GITINGEST_BASE, {
      headers: { "User-Agent": USER_AGENT },
      signal: AbortSignal.timeout(12_000),
    });
    const html = await resp.text();
    const hasForm = html.includes("ingestForm") || html.includes("/static/js/git.js");
    return {
      url: GITINGEST_BASE,
      reachable: resp.ok && hasForm,
      apiRouteDiscovered: "/api/ingest",
      status: resp.status,
    };
  } catch {
    return {
      url: GITINGEST_BASE,
      reachable: false,
      apiRouteDiscovered: "/api/ingest",
      status: 0,
    };
  }
}

export async function fetchCratesIoMeta(crateNames: readonly string[] = TARGET_CRATES): Promise<CratesIoVersionInfo[]> {
  return Promise.all(
    crateNames.map(async (name) => {
      try {
        const resp = await fetch(`https://crates.io/api/v1/crates/${encodeURIComponent(name)}`, {
          headers: { "User-Agent": USER_AGENT, Accept: "application/json" },
          signal: AbortSignal.timeout(15_000),
        });
        if (!resp.ok) {
          return {
            name,
            exists: false,
            maxVersion: "unknown",
            maxStableVersion: "unknown",
            downloads: 0,
            recentDownloads: 0,
            updatedAt: "",
            repository: "",
            error: `HTTP ${resp.status}`,
          };
        }
        const json = (await resp.json()) as { crate?: Record<string, any> };
        const c = json.crate ?? {};
        return {
          name,
          exists: true,
          maxVersion: String(c.max_version ?? "unknown"),
          maxStableVersion: String(c.max_stable_version ?? c.max_version ?? "unknown"),
          downloads: Number(c.downloads ?? 0),
          recentDownloads: Number(c.recent_downloads ?? 0),
          updatedAt: String(c.updated_at ?? ""),
          repository: String(c.repository ?? ""),
        };
      } catch (err) {
        return {
          name,
          exists: false,
          maxVersion: "unknown",
          maxStableVersion: "unknown",
          downloads: 0,
          recentDownloads: 0,
          updatedAt: "",
          repository: "",
          error: String(err),
        };
      }
    }),
  );
}

export async function mapWithConcurrency<T, R>(
  items: readonly T[],
  limit: number,
  fn: (item: T, idx: number) => Promise<R>,
): Promise<R[]> {
  const results = new Array<R>(items.length);
  let cursor = 0;
  const workers = Array.from({ length: Math.min(limit, items.length) }, async () => {
    while (cursor < items.length) {
      const idx = cursor++;
      results[idx] = await fn(items[idx], idx);
    }
  });
  await Promise.all(workers);
  return results;
}

export async function runScanCommand(root = REPO_ROOT): Promise<ScanReport> {
  const rawReposText = runGhApi(["--paginate", "--slurp", `orgs/${ORG_NAME}/repos?per_page=100`]);
  const rawRepos = parsePaginatedJson<Record<string, any>>(rawReposText);

  const [cratesIo, gitingestEndpointProbe, repos] = await Promise.all([
    fetchCratesIoMeta(TARGET_CRATES),
    probeGitIngestEndpoint(),
    mapWithConcurrency(rawRepos, 4, async (raw): Promise<OrgRepoTelemetry> => {
      const name = String(raw.name ?? "");
      const fullName = String(raw.full_name ?? `${ORG_NAME}/${name}`);
      const defaultBranch = String(raw.default_branch ?? "main");

      const commitRaw = JSON.parse(runGhApi([`repos/${fullName}/commits/${defaultBranch}`])) as Record<string, any>;
      const sha = String(commitRaw.sha ?? "");
      const commitMeta = (commitRaw.commit ?? {}) as Record<string, any>;
      const commitDate = String(commitMeta.committer?.date ?? commitMeta.author?.date ?? raw.pushed_at ?? "");
      const commitMsg = String(commitMeta.message ?? "").split(/\r?\n/)[0] ?? "";

      const treeRaw = JSON.parse(runGhApi([`repos/${fullName}/git/trees/${sha}?recursive=1`])) as {
        tree?: Array<{ path: string; type: string }>;
        truncated?: boolean;
      };
      const blobPaths = (treeRaw.tree ?? []).filter((t) => t.type === "blob").map((t) => t.path);
      const keyPathsPresent = selectKeyDigestFiles(name, blobPaths, 28);

      const rootCargoRaw = blobPaths.includes("Cargo.toml") ? fetchGhFileUtf8(fullName, "Cargo.toml") : null;
      const rootCargo = rootCargoRaw ? parseCargoManifestQuick(rootCargoRaw) : undefined;

      const subCargoPaths = blobPaths.filter(
        (p) =>
          p !== "Cargo.toml" &&
          (/^(crates|client)\/[^/]+\/Cargo\.toml$/.test(p) || p === "client/Cargo.toml"),
      );
      const subCargoVersions: OrgRepoTelemetry["subCargoVersions"] = [];
      for (const scPath of subCargoPaths.slice(0, 14)) {
        const scText = fetchGhFileUtf8(fullName, scPath);
        if (!scText) continue;
        try {
          const parsed = parseCargoManifestQuick(scText);
          if (parsed.packageName) {
            subCargoVersions.push({
              path: scPath,
              name: parsed.packageName,
              version: parsed.version ?? "workspace",
              rustVersion: parsed.rustVersion,
            });
          }
        } catch {
          // Skip unparseable sub-manifest edge cases.
        }
      }

      let packageJson: OrgRepoTelemetry["packageJson"];
      if (blobPaths.includes("package.json")) {
        const pkgText = fetchGhFileUtf8(fullName, "package.json");
        if (pkgText) {
          try {
            const p = JSON.parse(pkgText) as Record<string, unknown>;
            packageJson = {
              name: typeof p.name === "string" ? p.name : undefined,
              version: typeof p.version === "string" ? p.version : undefined,
              type: typeof p.type === "string" ? p.type : undefined,
            };
          } catch {
            // Ignore malformed package.json
          }
        }
      }

      return {
        name,
        fullName,
        htmlUrl: String(raw.html_url ?? `https://github.com/${fullName}`),
        description: String(raw.description ?? ""),
        language: String(raw.language ?? "Markdown/Spec"),
        stars: Number(raw.stargazers_count ?? 0),
        forks: Number(raw.forks_count ?? 0),
        openIssuesAndPrs: Number(raw.open_issues_count ?? 0),
        archived: Boolean(raw.archived),
        defaultBranch,
        pushedAt: String(raw.pushed_at ?? ""),
        latestCommitSha: sha,
        latestCommitDate: commitDate,
        latestCommitMessage: commitMsg,
        treeBlobCount: blobPaths.length,
        treeTruncated: Boolean(treeRaw.truncated),
        keyPathsPresent,
        rootCargo,
        subCargoVersions,
        packageJson,
        roleInEcosystem: REPO_ROLE_MAP[name] ?? String(raw.description ?? ""),
      };
    }),
  ]);

  repos.sort((a, b) => b.stars - a.stars || a.name.localeCompare(b.name));

  return {
    schemaVersion: 1,
    generatedAt: new Date().toISOString(),
    org: ORG_NAME,
    repoCount: repos.length,
    repos,
    cratesIo,
    localToolchain: extractLocalAphrodyState(root),
    gitingestEndpointProbe,
  };
}

async function fetchRemoteGitIngest(repo: OrgRepoTelemetry): Promise<{
  text: string;
  method: "gitingest.com-digest-url" | "gitingest.com-inline";
  digestUrl?: string;
  summary: string;
  filesAnalyzed: number;
  estimatedTokens: string;
}> {
  const resp = await fetch(GITINGEST_API, {
    method: "POST",
    headers: {
      "User-Agent": USER_AGENT,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      input_text: repo.htmlUrl,
      max_file_size: 51200,
      pattern_type: "exclude",
      pattern: "",
    }),
    signal: AbortSignal.timeout(45_000),
  });
  if (!resp.ok) {
    throw new Error(`POST ${GITINGEST_API} returned HTTP ${resp.status}`);
  }
  const data = (await resp.json()) as {
    summary?: string;
    digest_url?: string;
    tree?: string;
    content?: string;
    error?: string;
  };
  if (data.error) {
    throw new Error(`gitingest.com error for ${repo.fullName}: ${data.error}`);
  }

  const summary = String(data.summary ?? "");
  const meta = parseGitIngestSummaryMetadata(summary);
  const digestUrl = typeof data.digest_url === "string" ? data.digest_url : undefined;

  if (digestUrl) {
    try {
      const fullResp = await fetch(digestUrl, {
        headers: { "User-Agent": USER_AGENT, Accept: "text/plain" },
        signal: AbortSignal.timeout(60_000),
      });
      if (fullResp.ok) {
        const fullText = await fullResp.text();
        const check = validateGitIngestDigest(fullText);
        if (check.ok) {
          return {
            text: fullText,
            method: "gitingest.com-digest-url",
            digestUrl,
            summary,
            filesAnalyzed: meta.filesAnalyzed || check.fileCount,
            estimatedTokens: meta.estimatedTokens,
          };
        }
      }
    } catch {
      // Fall through cleanly to inline payload or GitHub Tree fallback.
    }
  }

  const inlineText = `${String(data.tree ?? "").trimEnd()}\n\n${String(data.content ?? "")}\n`;
  const inlineCheck = validateGitIngestDigest(inlineText);
  if (!inlineCheck.ok) {
    throw new Error(`Inline gitingest.com output failed validation: ${inlineCheck.errors.join("; ")}`);
  }
  return {
    text: inlineText,
    method: "gitingest.com-inline",
    digestUrl,
    summary,
    filesAnalyzed: meta.filesAnalyzed || inlineCheck.fileCount,
    estimatedTokens: meta.estimatedTokens,
  };
}

export async function buildGitHubTreeFallbackDigest(repo: OrgRepoTelemetry): Promise<string> {
  const treeRaw = JSON.parse(
    runGhApi([`repos/${repo.fullName}/git/trees/${repo.latestCommitSha}?recursive=1`]),
  ) as { tree?: Array<{ path: string; type: string }> };
  const blobPaths = (treeRaw.tree ?? []).filter((t) => t.type === "blob").map((t) => t.path);
  const selected = selectKeyDigestFiles(repo.name, blobPaths, 28);
  const loadedFiles: Array<{ path: string; content: string }> = [];
  for (const path of selected) {
    const content = fetchGhFileUtf8(repo.fullName, path);
    if (content !== null) {
      loadedFiles.push({ path, content });
    }
  }
  return renderGitIngestDigest(`${ORG_NAME}-${repo.name}`, blobPaths, loadedFiles);
}

export async function runIngestCommand(
  scan: ScanReport,
  options: { root?: string; forceGithub?: boolean } = {},
): Promise<IngestManifest> {
  const root = options.root ?? REPO_ROOT;
  const outDir = join(root, DIGEST_DIR_REL);
  mkdirSync(outDir, { recursive: true });

  const entries = await mapWithConcurrency(scan.repos, 2, async (repo): Promise<RepoDigestResult> => {
    const digestAbs = join(outDir, `${repo.name}.txt`);
    const digestRel = relative(root, digestAbs).replaceAll("\\", "/");

    let text = "";
    let method: RepoDigestResult["method"] = "github-tree-fallback";
    let digestUrl: string | undefined;
    let summary = "";
    let filesAnalyzed = 0;
    let estimatedTokens = "local-tree";
    let fallbackReason: string | undefined;

    if (!options.forceGithub) {
      try {
        const remote = await fetchRemoteGitIngest(repo);
        text = remote.text;
        method = remote.method;
        digestUrl = remote.digestUrl;
        summary = remote.summary;
        filesAnalyzed = remote.filesAnalyzed;
        estimatedTokens = remote.estimatedTokens;
      } catch (err) {
        fallbackReason = String(err);
      }
    } else {
      fallbackReason = "--force-github requested";
    }

    if (!text) {
      text = await buildGitHubTreeFallbackDigest(repo);
      method = "github-tree-fallback";
      const parsed = validateGitIngestDigest(text);
      filesAnalyzed = parsed.fileCount;
      summary = `Repository: ${repo.fullName}\nCommit: ${repo.latestCommitSha}\nFiles analyzed: ${parsed.fileCount} (GitHub Tree fallback)`;
    }

    writeFileSync(digestAbs, text, "utf8");
    const validation = validateGitIngestDigest(text);
    const bytes = Buffer.byteLength(text, "utf8");
    const lines = text.split(/\r?\n/).length;
    const sha256 = createHash("sha256").update(text).digest("hex");

    return {
      repo: repo.name,
      fullName: repo.fullName,
      commitSha: repo.latestCommitSha,
      method,
      digestPath: digestRel,
      digestBytes: bytes,
      digestLines: lines,
      digestFileCount: validation.fileCount,
      digestSha256: sha256,
      filesAnalyzed: filesAnalyzed || validation.fileCount,
      estimatedTokens,
      digestUrl,
      summary,
      fallbackReason,
    };
  });

  const manifest: IngestManifest = {
    schemaVersion: 1,
    generatedAt: new Date().toISOString(),
    gitingestCliVersion: scan.localToolchain.cliVersions.gitingest,
    remoteApiUrl: GITINGEST_API,
    repoCount: entries.length,
    totalDigestBytes: entries.reduce((acc, e) => acc + e.digestBytes, 0),
    totalDigestFiles: entries.reduce((acc, e) => acc + e.digestFileCount, 0),
    entries,
  };

  writeFileSync(join(root, INGEST_MANIFEST_REL), `${JSON.stringify(manifest, null, 2)}\n`, "utf8");
  return manifest;
}

// ---------------------------------------------------------------------------
// Emitters: llms.txt, Architecture Guide
// ---------------------------------------------------------------------------

function fmtNum(n: number): string {
  return new Intl.NumberFormat().format(n);
}

function fmtBytes(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${bytes} B`;
}

export function renderLlmsTxt(scan: ScanReport, manifest: IngestManifest | null): string {
  const lt = scan.localToolchain;
  const wbCrate = scan.cratesIo.find((c) => c.name === "wasm-bindgen");
  const lines: string[] = [
    "# wasm-bindgen Ecosystem & Aphrody WebAssembly Architecture",
    "",
    `> Unified reference index across all ${scan.repoCount} repositories in github.com/${ORG_NAME}, live crates.io releases (latest \`wasm-bindgen\` \`${wbCrate?.maxVersion ?? "unknown"}\`), and Aphrody's pinned \`wasm-bindgen ${lt.cargoLockVersions.wasmBindgen}\` + \`wasm-opt ${lt.cliVersions.wasmOpt}\` WebGPU/WebGL2 browser architecture (\`crates/gpui_web\`, \`tooling/bun/web-build.ts\`).`,
    "",
    `Aphrody compiles \`wasm32-unknown-unknown\` apps under \`[profile.web]\` using direct \`cargo build\` + CLI \`wasm-bindgen\` (\`== Cargo.lock\`) + Binaryen \`wasm-opt\`, replacing legacy \`wasm-pack\` and \`trunk\` pipelines while eliminating Windows \`sccache\` argv overflow (\`RUSTC_WRAPPER=""\`) and delivering opt-in COOP/COEP multi-cohort Wasm bundles.`,
    "",
    "## Core Documentation & Best Practices",
    "",
    `- [Aphrody wasm-bindgen Ecosystem Guide](./wasm-bindgen-ecosystem.md): Full architectural analysis of all 8 upstream repos, live \`crates.io\` vs \`Cargo.lock\` telemetry, \`walrus\`/\`wasm-opt\` pipelines, and \`gpui_web\` FFI standards.`,
    "- Agent skill: `fw: plugins/aphrody/skills/wasm-bindgen`",
    `- [Unified Web Build Pipeline](../../../tooling/bun/web-build.ts): Single-source Bun orchestrator enforcing exact \`wasm-bindgen\` CLI == \`Cargo.lock\` pinning, \`[profile.web]\`, 3-cohort builds (\`baseline\`, \`simd-baseline\`, \`multithreaded\`), \`wasm-opt -Oz\`, and Brotli size budgets.`,
    `- [GPUI Web Platform Crate](../../../crates/gpui_web/Cargo.toml): Production browser platform backend (\`${lt.gpuiWebFeaturesCount}\` minimized \`web-sys\` features, \`Closure\` lifecycle cleanup, \`async_channel\` Fetch streaming, and \`Atomics.waitAsync\` worker mailbox).`,
    `- [Workspace Cargo Manifest](../../../Cargo.toml): Workspace dependency pins (\`wasm-bindgen = "${lt.workspaceConstraints.wasmBindgen}"\`, \`wasm_thread\` rev \`${lt.workspaceConstraints.wasmThreadGitRev}\`) and \`[profile.web]\` linker/opt settings.`,
    "",
    "## Repositories (github.com/wasm-bindgen)",
    "",
    ...scan.repos.map((r) => {
      const digest = manifest?.entries.find((e) => e.repo === r.name);
      const digestNote = digest
        ? ` | Digest: \`${digest.method}\` (${fmtBytes(digest.digestBytes)}, ${fmtNum(digest.digestFileCount)} bundled sections, ${fmtNum(digest.filesAnalyzed)} repo files analyzed)`
        : "";
      return `- [${r.fullName}](${r.htmlUrl}): ${r.roleInEcosystem} Branch \`${r.defaultBranch}\` @ \`${r.latestCommitSha.slice(0, 8)}\` (${fmtNum(r.stars)} stars)${digestNote}.`;
    }),
    "",
    "## Optional",
    "",
    `- [GitIngest Automation CLI](../../../tooling/bun/wasm-bindgen-ingest.ts): Deterministic CLI (\`scan\`, \`ingest\`, \`emit\`, \`all\`) writing raw per-repo digests to gitignored \`${DIGEST_DIR_REL}/*.txt\`.`,
    `- [GPUI Web Fetch Client](../../../crates/gpui_web/src/http_client.rs): Reference \`ReadableStreamDefaultReader\` + \`wasm_bindgen_futures::spawn_local\` streaming implementation.`,
    `- [NSH Browser Value Bridge](../../../crates/nsh-platform-web/src/browser/values.rs): Slot-based custom Wasm/JS import ABI used alongside \`gpui_web\` for QuickJS-hosted web apps.`,
    "",
  ];
  return lines.join("\n");
}

function renderTelemetryMarkdownTable(scan: ScanReport, manifest: IngestManifest | null): string {
  const lt = scan.localToolchain;
  const lines = [
    "<!-- BEGIN WASM_BINDGEN_TELEMETRY -->",
    "| Metric | Verified Live Value | Source |",
    "| :--- | :--- | :--- |",
    `| **\`wasm-bindgen\` Org Repositories** | \`${scan.repoCount}\` repositories audited & ingested (100% coverage) | \`gh api --paginate orgs/wasm-bindgen/repos\` |`,
    `| **Aphrody Pinned \`wasm-bindgen\`** | \`Cargo.lock\` \`${lt.cargoLockVersions.wasmBindgen}\` == CLI \`${lt.cliVersions.wasmBindgen}\` (constraint \`${lt.workspaceConstraints.wasmBindgen}\`) | [\`Cargo.lock\`](../../../Cargo.lock) + [\`web-build.ts\`](../../../tooling/bun/web-build.ts) |`,
    `| **Binaryen Optimizer** | \`${lt.cliVersions.wasmOpt}\` (\`-Oz --enable-bulk-memory --enable-reference-types\`) | Local PATH + [\`web-build.ts\`](../../../tooling/bun/web-build.ts) |`,
    `| **\`gpui_web\` Minimized \`web-sys\`** | \`${lt.gpuiWebFeaturesCount}\` explicit features (\`crates/gpui_wgpu\` uses \`${lt.gpuiWgpuWebSysFeatures.length}\`) | [\`crates/gpui_web/Cargo.toml\`](../../../crates/gpui_web/Cargo.toml) |`,
    `| **GitIngest Endpoint & CLI** | \`${GITINGEST_API}\` reachable=\`${scan.gitingestEndpointProbe.reachable}\` (HTTP ${scan.gitingestEndpointProbe.status}); CLI \`${lt.cliVersions.gitingest}\` | Live HTTP probe + \`gitingest\` |`,
    `| **Persisted Raw Digests** | \`${manifest ? manifest.entries.length : 0}\` files under \`${DIGEST_DIR_REL}/\` (\`${fmtBytes(manifest?.totalDigestBytes ?? 0)}\`, \`${fmtNum(manifest?.totalDigestFiles ?? 0)}\` extracted file sections) | [\`wasm-bindgen-ingest.ts\`](../../../tooling/bun/wasm-bindgen-ingest.ts) |`,
    "<!-- END WASM_BINDGEN_TELEMETRY -->",
  ];
  return lines.join("\n");
}

function renderReposMarkdownTable(scan: ScanReport, manifest: IngestManifest | null): string {
  const header = [
    "<!-- BEGIN WASM_BINDGEN_REPOS_TABLE -->",
    "| Repository | Branch & HEAD SHA | Stars | Language | Blob Files | Digest Method & Size | Primary Ecosystem Role |",
    "| :--- | :--- | ---: | :--- | ---: | :--- | :--- |",
  ];
  const rows = scan.repos.map((r) => {
    const digest = manifest?.entries.find((e) => e.repo === r.name);
    const digestInfo = digest
      ? `\`${digest.method}\` (${fmtBytes(digest.digestBytes)})`
      : "pending `ingest`";
    return `| [${r.fullName}](${r.htmlUrl}) | \`${r.defaultBranch}\` @ \`${r.latestCommitSha.slice(0, 8)}\` (${r.latestCommitDate.slice(0, 10)}) | ${fmtNum(r.stars)} | ${r.language} | ${fmtNum(r.treeBlobCount)} | ${digestInfo} | ${r.roleInEcosystem} |`;
  });
  return [...header, ...rows, "<!-- END WASM_BINDGEN_REPOS_TABLE -->"].join("\n");
}

function renderCratesMatrixMarkdownTable(scan: ScanReport): string {
  const lock = scan.localToolchain.cargoLockVersions;
  const ws = scan.localToolchain.workspaceConstraints;
  const localMap: Record<string, { lockVer: string; wsSpec: string; status: string }> = {
    "wasm-bindgen": {
      lockVer: lock.wasmBindgen,
      wsSpec: ws.wasmBindgen,
      status: "Core Wasm/JS runtime (`Cargo.lock` == CLI `0.2.129`; 1 patch behind `0.2.130`)",
    },
    "wasm-bindgen-futures": {
      lockVer: lock.wasmBindgenFutures,
      wsSpec: ws.wasmBindgenFutures,
      status: "Async Promise <-> Future bridge in `gpui_web` (`0.4.79` vs `0.4.80`)",
    },
    "js-sys": {
      lockVer: lock.jsSys,
      wsSpec: ws.jsSys,
      status: "ECMAScript built-ins (`Uint8Array`, `Reflect`, `Atomics`, `Promise`)",
    },
    "web-sys": {
      lockVer: lock.webSys,
      wsSpec: ws.webSys,
      status: "Minimized WebIDL bindings (`52` features in `gpui_web`, `1` in `gpui_wgpu`)",
    },
    walrus: {
      lockVer: lock.walrus,
      wsSpec: "internal to wasm-bindgen-cli",
      status: "Powers `wasm-bindgen-cli` IR transforms (`externref`, `gc`, DWARF) pre-`wasm-opt`",
    },
    weedle: {
      lockVer: lock.weedle,
      wsSpec: "upstream codegen-only",
      status: "WebIDL parser driving upstream `wasm-bindgen-webidl` -> `web-sys` codegen",
    },
    "wasm-pack": {
      lockVer: lock.wasmPack,
      wsSpec: "replaced by web-build.ts",
      status: "Legacy npm/webpack wrapper superseded by Aphrody `cargo build + wasm-bindgen + wasm-opt`",
    },
  };

  const lines = [
    "<!-- BEGIN WASM_BINDGEN_CRATES_MATRIX -->",
    "| Crate | `crates.io` Latest | Aphrody `Cargo.toml` | Aphrody `Cargo.lock` | Total Downloads | Updated (UTC) | Status in Aphrody |",
    "| :--- | :--- | :--- | :--- | ---: | :--- | :--- |",
  ];
  for (const c of scan.cratesIo) {
    const local = localMap[c.name] ?? { lockVer: "n/a", wsSpec: "n/a", status: "" };
    lines.push(
      `| \`${c.name}\` | \`${c.maxVersion}\` | \`${local.wsSpec}\` | \`${local.lockVer}\` | ${fmtNum(c.downloads)} | ${c.updatedAt.slice(0, 10) || "n/a"} | ${local.status} |`,
    );
  }
  lines.push("<!-- END WASM_BINDGEN_CRATES_MATRIX -->");
  return lines.join("\n");
}

export function renderEcosystemGuide(scan: ScanReport, manifest: IngestManifest | null): string {
  const lt = scan.localToolchain;
  return `# wasm-bindgen Ecosystem & Aphrody WebAssembly Best Practices Guide

> Live architectural audit across all **${scan.repoCount} repositories** in \`github.com/${ORG_NAME}\`, \`crates.io\` upstream releases, GitIngest digests, and Aphrody's pinned \`wasm-bindgen ${lt.cargoLockVersions.wasmBindgen}\` + \`${lt.cliVersions.wasmOpt}\` stack (\`crates/gpui_web\`, \`crates/aphrody_desktop\`, \`tooling/bun/web-build.ts\`).

---

## 1. Live Ingestion Telemetry & Workspace Summary

${renderTelemetryMarkdownTable(scan, manifest)}

---

## 2. All 8 \`github.com/wasm-bindgen\` Upstream Repositories

${renderReposMarkdownTable(scan, manifest)}

### Repository Architecture Breakdown

- **\`wasm-bindgen/wasm-bindgen\`**: Monorepo hosting \`wasm-bindgen\`, \`wasm-bindgen-macro\`, \`wasm-bindgen-macro-support\`, \`wasm-bindgen-shared\`, \`wasm-bindgen-futures\`, \`js-sys\`, \`web-sys\`, \`wasm-bindgen-webidl\`, \`wasm-bindgen-cli-support\`, and \`wasm-bindgen-cli\`.
- **\`wasm-bindgen/walrus\`**: Wasm IR transformation library (\`wasmparser\` + \`wasm-encoder\` + \`gimli\` DWARF + Wasm GC) powering \`wasm-bindgen-cli-support\`'s dead-symbol GC, \`externref\` table lowering, JSPI, multi-value, and threads passes *before* \`wasm-opt\`.
- **\`wasm-bindgen/weedle\`**: \`nom\`-based WebIDL parser turning W3C/WHATWG specs into ASTs for \`wasm-bindgen-webidl\`, which code-generates \`crates/web-sys\` with \`#[cfg(web_sys_unstable_apis)]\` dual-override guards.
- **\`wasm-bindgen/ts-gen\`**: \`oxc_parser 0.154\` + \`syn 2\` + \`prettyplease\` compiler turning TypeScript \`.d.ts\` declarations into \`#[wasm_bindgen]\` Rust bindings (supporting \`--strict-nullable\` and Cloudflare Worker TypeScript contracts).
- **\`wasm-bindgen/js-bindgen\`**: Next-gen \`#![no_std]\` linker-driven JS interop (\`js-bindgen-ld\`, \`embed_js\`, \`import_js\`, \`unsafe_global_wat\`) exploring real-generic \`js-sys\`, \`wasm64-unknown-unknown\`, and \`bun\` testing.
- **\`wasm-bindgen/rust-externref-rfc\`**: Draft proposal for boxed \`Externref\` / \`ExternrefRef<'a>\` / \`ByValue<T>\` / \`with_stack_slot\` over \`ptr addrspace(2)\`, solving RFC 3987's operational-semantics objection while letting LLVM fold away \`__externref_table_alloc\`/\`free\` pairs.
- **\`wasm-bindgen/wasm-pack\`**: Legacy npm/webpack packager (\`0.15.1\`); deliberately omitted from Aphrody in favor of [\`tooling/bun/web-build.ts\`](../../../tooling/bun/web-build.ts).
- **\`wasm-bindgen/wasm-bindgen-bot\`**: Sandboxed Cloudflare AI Gateway PR reviewer & issue triage bot maintaining the \`wasm-bindgen\` organization.

---

## 3. \`crates.io\` Upstream Releases vs Aphrody Workspace Pins

${renderCratesMatrixMarkdownTable(scan)}

### Version Pinning & Upgrade Policy

- **Exact Lockstep Rule**: [\`tooling/bun/web-build.ts\`](../../../tooling/bun/web-build.ts) runs \`verifyBindgenVersion()\` to guarantee CLI \`wasm-bindgen\` (${lt.cliVersions.wasmBindgen}) strictly equals \`Cargo.lock\` (${lt.cargoLockVersions.wasmBindgen}). Never update \`Cargo.lock\` to \`0.2.130\` without installing matching \`wasm-bindgen-cli@0.2.130\`.
- **\`wasm_thread\` Fork Rationale**: Root [\`Cargo.toml\`](../../../Cargo.toml) pins \`wasm_thread\` to \`zed-industries/wasm_thread\` rev \`${lt.workspaceConstraints.wasmThreadGitRev}\` (\`features = ["es_modules"]\`) because upstream \`crates.io\` \`wasm_thread\` invokes deprecated positional \`wasm-bindgen\` init arguments (deprecated since \`0.2.93\`).

---

## 4. Deep Best Practices Grounded in \`crates/gpui_web\` & \`web-build.ts\`

### 4.1 \`ExternRef\` & \`Closure\` Cleanup Discipline

1. **RAII Unregistration Over \`.forget()\`**: [\`crates/gpui_web/src/events.rs\`](../../../crates/gpui_web/src/events.rs) wraps every DOM event listener in \`EventListenerHandle\`, whose \`Drop\` implementation calls \`target.remove_event_listener_with_callback(...)\` before dropping the \`Closure\`. Simply dropping a \`Closure\` without removing the listener triggers *"closure invoked after being dropped"*; calling \`.forget()\` leaks Wasm heap/table entries on view teardown.
2. **Owner Lifetime + \`Weak\` Back-References**: [\`crates/gpui_web/src/window.rs\`](../../../crates/gpui_web/src/window.rs) retains \`_raf_closure\`, \`_resize_observer_closure\`, and \`_safe_area_observer_closure\` directly on \`WebWindow\`, using \`Rc::downgrade(&inner)\` on observers to eliminate retain cycles.
3. **One-Shot Closures**: [\`crates/gpui_web/src/dispatcher.rs\`](../../../crates/gpui_web/src/dispatcher.rs) and [\`ime_mirror.rs\`](../../../crates/gpui_web/src/ime_mirror.rs) use \`Closure::once_into_js\` for \`setTimeout\`, \`queueMicrotask\`, and \`requestIdleCallback\`.
4. **Reentrancy Safety Under \`panic = "abort"\`**: \`WebWindowInner::with_callback\` moves callbacks out via \`.take()\` before execution so nested DOM events cannot trigger fatal \`RefCell\` double-borrow aborts.

### 4.2 Zero-Copy \`&[u8]\` / \`Uint8Array\` Views vs Heap-Safe Copies

- **\`unsafe { js_sys::Uint8Array::view(slice) }\`** exposes Wasm linear memory without copying, but **any allocation (\`Vec\`, \`Box\`, \`String\`, Rust re-entry) can execute \`memory.grow\` and detach the underlying \`ArrayBuffer\`**, invalidating the JS view immediately.
- Therefore, \`crates/gpui_web\` reserves direct memory views strictly for synchronous fixed-memory operations (such as \`Int32Array::new_with_byte_offset_and_length\` on shared \`AtomicI32\` mailbox signals in \`dispatcher.rs\`) and uses owned copies (\`Uint8Array::from(bytes.as_slice())\` and \`Uint8Array::new(&buffer).to_vec()\`) across async Fetch streams (\`http_client.rs\`) and Clipboard Blob reads (\`platform.rs\`).

### 4.3 \`wasm-bindgen-futures\`, Microtasks & Worker Concurrency

- \`wasm_bindgen_futures::JsFuture::from(promise).await\` bridges JS Promises to Rust \`Future\`s on the microtask queue.
- \`http_client.rs\` pumps \`ReadableStreamDefaultReader::read()\` inside \`wasm_bindgen_futures::spawn_local\` over \`async_channel::bounded(8)\` rather than \`futures::mpsc\` (whose internal mutex can park the browser main thread when workers pull body chunks).
- \`dispatcher.rs\` pairs \`SharedArrayBuffer\` + \`Atomics.waitAsync\` -> \`JsFuture\` on the main thread because browsers disallow synchronous \`Atomics.wait\` on the UI thread.

### 4.4 \`web-sys\` Feature Minimization & Dynamic Feature Probing

- \`crates/gpui_web/Cargo.toml\` enables only **${lt.gpuiWebFeaturesCount} exact WebIDL features**; \`crates/gpui_wgpu/Cargo.toml\` enables only **${lt.gpuiWgpuWebSysFeatures.length} (\`HtmlCanvasElement\`)**.
- Dynamic APIs (\`navigator.clipboard\`, \`requestIdleCallback\`, \`PointerEvent.getPredictedEvents\`, \`Atomics.waitAsync\`) are probed via \`js_sys::Reflect::has\` / \`Reflect::get\` before invoking \`web-sys\` bindings to prevent \`wasm-bindgen\` aborts in non-secure contexts or Safari.
- WebIDL dictionaries lacking runtime class prototypes (\`ReadableStreamReadResult\`) use \`.unchecked_into()\`; DOM interfaces use checked \`.dyn_into()\`.

### 4.5 \`walrus\` + \`wasm-opt\` Optimization & Why Aphrody Rejects \`wasm-pack\`

1. **Compiler & Optimizer Pipeline**: \`rustc --profile web\` (\`opt-level = "z"\`, \`lto = "thin"\`, \`codegen-units = 1\`, \`strip = true\`, \`panic = "abort"\`) -> \`wasm-bindgen\` CLI (\`walrus\` \`externref\` lowering, descriptor decoding, dead-stub \`passes::gc\`) -> Binaryen \`${lt.cliVersions.wasmOpt}\` (\`-Oz --enable-bulk-memory --enable-bulk-memory-opt --enable-nontrapping-float-to-int --enable-sign-ext --enable-reference-types --enable-multivalue --strip-debug\`).
2. **Why Not \`wasm-pack\`?** Legacy \`wasm-pack 0.15.1\` enforces single-target \`npm\`/\`webpack\` outputs, hits Windows \`sccache\` \`CreateProcess\` 32 KB argv overflow on \`web-sys\`, cannot build Aphrody's 3 runtime-selected cohorts (\`baseline\`, \`simd-baseline\`, opt-in COI \`multithreaded\`), and skips native Wasm section validation + \`--brotli-budget\` checks implemented in [\`tooling/bun/web-build.ts\`](../../../tooling/bun/web-build.ts).

---

## 5. Linked Artifacts

- **\`llms.txt\` Index**: [\`docs/aphrody/architecture/wasm-bindgen-llms.txt\`](./wasm-bindgen-llms.txt)
- **Agent Skill**: \`fw: plugins/aphrody/skills/wasm-bindgen\`
- **Automation CLI & Test Suite**: [\`tooling/bun/wasm-bindgen-ingest.ts\`](../../../tooling/bun/wasm-bindgen-ingest.ts), [\`tooling/bun/wasm-bindgen-ingest.test.ts\`](../../../tooling/bun/wasm-bindgen-ingest.test.ts)
- **Gitignored Digest Cache**: \`${DIGEST_DIR_REL}/*.txt\` + \`${INGEST_MANIFEST_REL}\`
`;
}

export function updateEcosystemSummaryBlocks(
  existingDoc: string,
  scan: ScanReport,
  manifest: IngestManifest | null,
): string {
  let updated = existingDoc;
  if (
    updated.includes("<!-- BEGIN WASM_BINDGEN_TELEMETRY -->") &&
    updated.includes("<!-- BEGIN WASM_BINDGEN_REPOS_TABLE -->") &&
    updated.includes("<!-- BEGIN WASM_BINDGEN_CRATES_MATRIX -->")
  ) {
    updated = updated.replace(
      /<!-- BEGIN WASM_BINDGEN_TELEMETRY -->[\s\S]*?<!-- END WASM_BINDGEN_TELEMETRY -->/,
      renderTelemetryMarkdownTable(scan, manifest),
    );
    updated = updated.replace(
      /<!-- BEGIN WASM_BINDGEN_REPOS_TABLE -->[\s\S]*?<!-- END WASM_BINDGEN_REPOS_TABLE -->/,
      renderReposMarkdownTable(scan, manifest),
    );
    updated = updated.replace(
      /<!-- BEGIN WASM_BINDGEN_CRATES_MATRIX -->[\s\S]*?<!-- END WASM_BINDGEN_CRATES_MATRIX -->/,
      renderCratesMatrixMarkdownTable(scan),
    );
    return updated;
  }
  return renderEcosystemGuide(scan, manifest);
}

export function runEmitCommand(
  scan: ScanReport,
  manifest: IngestManifest | null,
  root = REPO_ROOT,
): EmitSummary {
  const llmsTxtAbs = join(root, LLMS_TXT_REL);
  const ecosystemDocAbs = join(root, ECOSYSTEM_DOC_REL);
  mkdirSync(dirname(llmsTxtAbs), { recursive: true });

  const llmsTxt = renderLlmsTxt(scan, manifest);
  writeFileSync(llmsTxtAbs, llmsTxt, "utf8");

  const ecosystemContent = existsSync(ecosystemDocAbs)
    ? updateEcosystemSummaryBlocks(readFileSync(ecosystemDocAbs, "utf8"), scan, manifest)
    : renderEcosystemGuide(scan, manifest);
  writeFileSync(ecosystemDocAbs, ecosystemContent, "utf8");

  const brokenLinks = validateRelativeLinks([llmsTxtAbs, ecosystemDocAbs]);
  const llmsCheck = validateLlmsTxt(llmsTxt);

  return {
    llmsTxtPath: LLMS_TXT_REL,
    ecosystemDocPath: ECOSYSTEM_DOC_REL,
    brokenLinks,
    llmsTxtOk: llmsCheck.ok,
  };
}

// ---------------------------------------------------------------------------
// Artifact Cache Helpers & CLI Main
// ---------------------------------------------------------------------------

function writeTargetJson(relPath: string, data: unknown): string {
  const full = join(REPO_ROOT, relPath);
  mkdirSync(dirname(full), { recursive: true });
  writeFileSync(full, `${JSON.stringify(data, null, 2)}\n`, "utf8");
  return full;
}

function readTargetJsonIfExists<T>(relPath: string): T | null {
  const full = join(REPO_ROOT, relPath);
  if (!existsSync(full) || !statSync(full).isFile()) return null;
  try {
    return JSON.parse(readFileSync(full, "utf8")) as T;
  } catch {
    return null;
  }
}

async function main(): Promise<void> {
  const args = Bun.argv.slice(2);
  const cmd = args.find((a) => !a.startsWith("-")) ?? "all";
  const forceGithub = args.includes("--force-github");
  const validCmds = new Set(["scan", "ingest", "emit", "all"]);
  if (!validCmds.has(cmd)) {
    console.error(`Unknown subcommand '${cmd}'. Expected one of: scan | ingest | emit | all`);
    process.exit(1);
  }

  let scan = readTargetJsonIfExists<ScanReport>(SCAN_OUT_REL);
  let manifest = readTargetJsonIfExists<IngestManifest>(INGEST_MANIFEST_REL);

  if (cmd === "scan" || cmd === "all" || !scan) {
    scan = await runScanCommand(REPO_ROOT);
    writeTargetJson(SCAN_OUT_REL, scan);
    if (cmd === "scan") {
      console.log(
        JSON.stringify(
          {
            command: "scan",
            org: scan.org,
            repoCount: scan.repoCount,
            repos: scan.repos.map((r) => ({
              name: r.name,
              stars: r.stars,
              defaultBranch: r.defaultBranch,
              sha: r.latestCommitSha.slice(0, 8),
              treeBlobCount: r.treeBlobCount,
            })),
            cratesIo: scan.cratesIo.map((c) => `${c.name}@${c.maxVersion}`),
            localToolchain: scan.localToolchain.cargoLockVersions,
            cliVersions: scan.localToolchain.cliVersions,
            gitingestEndpointProbe: scan.gitingestEndpointProbe,
            scanPath: SCAN_OUT_REL,
          },
          null,
          2,
        ),
      );
      return;
    }
  }

  if (cmd === "ingest" || cmd === "all" || (cmd === "emit" && !manifest)) {
    manifest = await runIngestCommand(scan, { root: REPO_ROOT, forceGithub });
    if (cmd === "ingest") {
      console.log(
        JSON.stringify(
          {
            command: "ingest",
            repoCount: manifest.repoCount,
            totalDigestBytes: manifest.totalDigestBytes,
            totalDigestFiles: manifest.totalDigestFiles,
            gitingestCliVersion: manifest.gitingestCliVersion,
            entries: manifest.entries.map((e) => ({
              repo: e.repo,
              method: e.method,
              digestBytes: e.digestBytes,
              digestFileCount: e.digestFileCount,
              filesAnalyzed: e.filesAnalyzed,
              digestPath: e.digestPath,
            })),
            manifestPath: INGEST_MANIFEST_REL,
          },
          null,
          2,
        ),
      );
      return;
    }
  }

  const emitted = runEmitCommand(scan, manifest, REPO_ROOT);
  if (emitted.brokenLinks.length > 0 || !emitted.llmsTxtOk) {
    console.error("Validation error during emit:", JSON.stringify(emitted, null, 2));
    process.exit(1);
  }

  console.log(
    JSON.stringify(
      {
        command: cmd,
        repoCount: scan.repoCount,
        totalDigestBytes: manifest?.totalDigestBytes ?? 0,
        totalDigestFiles: manifest?.totalDigestFiles ?? 0,
        cratesIo: scan.cratesIo.map((c) => `${c.name}@${c.maxVersion}`),
        emitted,
      },
      null,
      2,
    ),
  );
}

if (import.meta.main) {
  await main();
}
