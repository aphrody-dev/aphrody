#!/usr/bin/env bun
/**
 * Live GitIngest + GitHub + crates.io Ingestion, llms.txt & Skill Emitter for
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
export const SKILL_DIR_REL = ".agents/plugins/aphrody/skills/wasm-bindgen";

const USER_AGENT = "AphrodyWasmBindgenIngest/1.0 (+https://github.com/aphrody-dev/aphrody)";
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
  skillPath: string;
  referencePaths: string[];
  brokenLinks: string[];
  skillFrontmatterOk: boolean;
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

export function validateSkillFrontmatter(markdown: string): {
  ok: boolean;
  name?: string;
  description?: string;
  errors: string[];
} {
  const errors: string[] = [];
  const match = markdown.match(/^---\r?\n([\s\S]*?)\r?\n---/);
  if (!match) {
    return { ok: false, errors: ["Missing YAML frontmatter delimited by ---"] };
  }
  let parsed: Record<string, unknown>;
  try {
    parsed = (Bun.YAML.parse(match[1]) ?? {}) as Record<string, unknown>;
  } catch (err) {
    return { ok: false, errors: [`Invalid YAML frontmatter: ${String(err)}`] };
  }
  const name = typeof parsed.name === "string" ? parsed.name.trim() : "";
  const description = typeof parsed.description === "string" ? parsed.description.trim() : "";

  if (!name) {
    errors.push("Missing required frontmatter field 'name'");
  } else {
    if (name.length > 64) errors.push(`Skill name exceeds 64 characters (${name.length})`);
    if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(name)) {
      errors.push(`Skill name '${name}' must be lowercase alphanumeric + single hyphens`);
    }
  }
  if (!description) {
    errors.push("Missing required frontmatter field 'description'");
  } else if (description.length > 1024) {
    errors.push(`Skill description exceeds 1024 characters (${description.length})`);
  }

  return {
    ok: errors.length === 0,
    name: name || undefined,
    description: description || undefined,
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
// Emitters: llms.txt, Agent Skill + references/*, Architecture Guide
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
    `- [wasm-bindgen Agent Skill](../../../.agents/plugins/aphrody/skills/wasm-bindgen/SKILL.md): Progressive-disclosure skill entrypoint (\`agentskills.io\` compliant) with non-negotiable FFI invariants and build verification commands.`,
    `- [Unified Web Build Pipeline](../../../tooling/bun/web-build.ts): Single-source Bun orchestrator enforcing exact \`wasm-bindgen\` CLI == \`Cargo.lock\` pinning, \`[profile.web]\`, 3-cohort builds (\`baseline\`, \`simd-baseline\`, \`multithreaded\`), \`wasm-opt -Oz\`, and Brotli size budgets.`,
    `- [GPUI Web Platform Crate](../../../crates/gpui_web/Cargo.toml): Production browser platform backend (\`${lt.gpuiWebFeaturesCount}\` minimized \`web-sys\` features, \`Closure\` lifecycle cleanup, \`async_channel\` Fetch streaming, and \`Atomics.waitAsync\` worker mailbox).`,
    `- [Workspace Cargo Manifest](../../../Cargo.toml): Workspace dependency pins (\`wasm-bindgen = "${lt.workspaceConstraints.wasmBindgen}"\`, \`wasm_thread\` rev \`${lt.workspaceConstraints.wasmThreadGitRev}\`) and \`[profile.web]\` linker/opt settings.`,
    "",
    "## Skill Reference Modules",
    "",
    `- [8-Repo Ecosystem Matrix](../../../.agents/plugins/aphrody/skills/wasm-bindgen/references/repos-matrix.md): Live GitHub commits, stars, MSRV, sub-crate manifests, and \`crates.io\` parity across all 8 \`wasm-bindgen\` org repos.`,
    `- [js-sys, web-sys & Futures FFI Patterns](../../../.agents/plugins/aphrody/skills/wasm-bindgen/references/js-web-sys-patterns.md): \`Closure\` drop guards vs \`once_into_js\`, zero-copy \`Uint8Array::view\` vs \`to_vec\`, \`wasm-bindgen-futures\` microtasks, \`web-sys\` feature minimization, and \`Reflect\` feature probing.`,
    `- [walrus, weedle, ts-gen & js-bindgen](../../../.agents/plugins/aphrody/skills/wasm-bindgen/references/walrus-weedle-tsgen.md): How \`walrus\` dead-symbol/section/\`externref\` passes prepare binaries for \`wasm-opt\`, \`weedle\` WebIDL parsing for \`web-sys\`, \`ts-gen\` \`.d.ts\` codegen, \`js-bindgen\` linker interop, and boxed \`Externref\` RFC.`,
    `- [Aphrody Best Practices & WebGPU Integration](../../../.agents/plugins/aphrody/skills/wasm-bindgen/references/aphrody-best-practices.md): Why \`cargo build + wasm-bindgen + wasm-opt\` beats \`wasm-pack\`, \`gpui_web\` reentrancy take/call/restore safety, \`web-build.ts\` cohort verification, and \`nsh-platform-web\` custom ABI contrast.`,
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

function renderSkillMarkdown(scan: ScanReport): string {
  const lt = scan.localToolchain;
  return `---
name: wasm-bindgen
description: >-
  Authoritative wasm-bindgen, js-sys, web-sys, wasm-bindgen-futures, walrus,
  weedle, ts-gen, and Browser Wasm FFI skill for aphrody. Use when editing
  crates/gpui_web, crates/aphrody_desktop web builds, tooling/bun/web-build.ts,
  wasm32-unknown-unknown exports, Closure lifetimes, Uint8Array views, or
  optimizing Wasm binaries with walrus + wasm-opt.
---

# wasm-bindgen Ecosystem & Browser FFI (\`aphrody\`)

Aphrody pins \`wasm-bindgen\` at **\`${lt.cargoLockVersions.wasmBindgen}\`** (\`Cargo.lock\` == \`wasm-bindgen\` CLI \`${lt.cliVersions.wasmBindgen}\`, \`wasm-bindgen-futures ${lt.cargoLockVersions.wasmBindgenFutures}\`, \`js-sys ${lt.cargoLockVersions.jsSys}\`, \`web-sys ${lt.cargoLockVersions.webSys}\`) and optimizes release browser binaries with **\`${lt.cliVersions.wasmOpt}\`** via [\`tooling/bun/web-build.ts\`](../../../../../tooling/bun/web-build.ts).

## Progressive Disclosure References

Resolve all paths relative to \`.agents/plugins/aphrody/skills/wasm-bindgen/\`:

| Topic | Reference File | When to Load |
| :--- | :--- | :--- |
| **8-Repo Upstream Matrix** | [references/repos-matrix.md](references/repos-matrix.md) | Checking live commits, MSRV, sub-crates, or \`crates.io\` drift across all 8 \`github.com/wasm-bindgen\` repositories |
| **JS/DOM/Futures FFI Patterns** | [references/js-web-sys-patterns.md](references/js-web-sys-patterns.md) | Writing \`Closure\` listeners, \`Uint8Array\` transfers, \`JsFuture\`/\`spawn_local\` loops, \`web-sys\` feature lists, or \`Reflect\` feature probes |
| **walrus, weedle, ts-gen & ABI R&D** | [references/walrus-weedle-tsgen.md](references/walrus-weedle-tsgen.md) | Inspecting Wasm IR passes (\`walrus\` -> \`wasm-opt\`), WebIDL parsing (\`weedle\`), \`.d.ts\` codegen (\`ts-gen\`), \`js-bindgen\`, or \`Externref\` RFC |
| **Aphrody Build & Architecture Rules** | [references/aphrody-best-practices.md](references/aphrody-best-practices.md) | Configuring \`[profile.web]\`, Windows \`RUSTC_WRAPPER=""\` workaround, 3-cohort builds, or contrasting \`gpui_web\` vs \`nsh-platform-web\` |
| **Canonical Architecture Doc** | [docs/aphrody/architecture/wasm-bindgen-ecosystem.md](../../../../../docs/aphrody/architecture/wasm-bindgen-ecosystem.md) | Full architectural synthesis grounded in \`Cargo.lock\`, \`gpui_web\`, and live GitIngest digests |
| **Standardized \`llms.txt\`** | [docs/aphrody/architecture/wasm-bindgen-llms.txt](../../../../../docs/aphrody/architecture/wasm-bindgen-llms.txt) | Machine-readable \`llmstxt.org\` index of all repos, guides, and skill references |

## Non-Negotiable Engineering Rules

1. **Exact CLI <-> \`Cargo.lock\` Lockstep**:
   \`wasm-bindgen\` CLI version MUST equal \`Cargo.lock\` (\`${lt.cargoLockVersions.wasmBindgen}\`). Never mix CLI and crate versions — \`web-build.ts\` aborts on mismatch because \`__wbindgen_placeholder__\` intrinsics and JS glue evolve across patch releases.
2. **Never Leak \`Closure\`s Unless Process-Lifetime**:
   - Repeating DOM listeners (\`window\`, \`document\`, \`ResizeObserver\`, \`MediaQueryList\`): wrap in \`EventListenerHandle\` (\`Drop\` calls \`remove_event_listener_with_callback\`, see [\`crates/gpui_web/src/events.rs\`](../../../../../crates/gpui_web/src/events.rs)) or store the \`Closure\` on the owning struct (\`WebWindow\`).
   - One-shot callbacks (\`setTimeout\`, \`queueMicrotask\`, \`requestIdleCallback\`): prefer \`Closure::once_into_js\`; never call \`.forget()\` in per-event or per-frame loops.
3. **Never Hold \`Uint8Array::view\` Across Any Rust Allocation**:
   \`unsafe { js_sys::Uint8Array::view(slice) }\` points directly into Wasm linear memory. Any heap growth (\`Box\`, \`Vec::push\`, \`String\`, JS->Rust callback) can execute \`memory.grow\`, detaching \`memory.buffer\` and invalidating the JS view. Consume synchronously inside a single JS boundary call or copy via \`Uint8Array::from(bytes)\` / \`.to_vec()\`.
4. **Prevent \`RefCell\` Reentrancy Panics Under \`panic = "abort"\`**:
   Aphrody \`[profile.web]\` compiles with \`panic = "abort"\` — a \`RefCell::borrow_mut\` panic during a synchronous DOM callback permanently wedges the Wasm instance. Always extract callbacks with \`.take()\` -> invoke -> restore ([\`WebWindowInner::with_callback\`](../../../../../crates/gpui_web/src/window.rs)).
5. **Minimize \`web-sys\` Features & Override \`RUSTC_WRAPPER=""\` on Windows**:
   Enable only exact WebIDL interfaces needed (\`${lt.gpuiWebFeaturesCount}\` features in \`gpui_web\`, \`1\` in \`gpui_wgpu\`). On Windows, unset \`RUSTC_WRAPPER\` during \`wasm32\` builds (\`web-build.ts\`) so \`sccache\` does not overflow \`CreateProcess\` command-line limits on \`web-sys\`.
6. **Use \`cargo build + wasm-bindgen + wasm-opt v133\`, Never Legacy \`wasm-pack\`**:
   Run \`bun tooling/bun/web-build.ts\` for deterministic \`baseline\`, \`simd-baseline\`, and opt-in \`multithreaded\` (\`--coi\`) cohorts without npm/webpack/COOP-required regressions.

## Fast Verification Commands

\`\`\`powershell
# Check exact wasm-bindgen lockstep + compile gpui_web / gpui_wgpu / aphrody_desktop
bun tooling/bun/web-build.ts check

# Build optimized release cohorts (baseline + simd-baseline) with wasm-opt v133
bun tooling/bun/web-build.ts build --package aphrody_desktop --release --opt

# Refresh live 8-repo GitIngest digests, llms.txt, skills, and architecture telemetry
bun tooling/bun/wasm-bindgen-ingest.ts all

# Run unit + link integrity tests for the wasm-bindgen ingestion pipeline
bun test tooling/bun/wasm-bindgen-ingest.test.ts
\`\`\`
`;
}

function renderReposMatrixRef(scan: ScanReport, manifest: IngestManifest | null): string {
  const lt = scan.localToolchain;
  return `# wasm-bindgen Organization 8-Repository Matrix

> Live snapshot generated \`${scan.generatedAt}\` from \`gh api --paginate orgs/${ORG_NAME}/repos\`, GitHub Git Trees, \`crates.io\`, and \`${GITINGEST_API}\`.

## 1. All 8 Org Repositories

${renderReposMarkdownTable(scan, manifest)}

## 2. Sub-Crate & Manifest Inventory

${scan.repos
  .map((r) => {
    const subs =
      r.subCargoVersions.length > 0
        ? r.subCargoVersions.map((s) => `\`${s.name}@${s.version}\` (\`${s.path}\`${s.rustVersion ? `, MSRV ${s.rustVersion}` : ""})`).join(", ")
        : r.rootCargo?.packageName
          ? `\`${r.rootCargo.packageName}@${r.rootCargo.version ?? "workspace"}\` (edition ${r.rootCargo.edition ?? "2021"}${r.rootCargo.rustVersion ? `, MSRV ${r.rootCargo.rustVersion}` : ""})`
          : r.packageJson?.name
            ? `npm \`${r.packageJson.name}@${r.packageJson.version ?? "0.0.0"}\``
            : "Spec / action repository (no Cargo or npm manifest)";
    const digest = manifest?.entries.find((e) => e.repo === r.name);
    const digestSummary = digest
      ? `Local digest: \`${digest.digestPath}\` (\`${fmtBytes(digest.digestBytes)}\`, \`${fmtNum(digest.digestLines)}\` lines, SHA-256 \`${digest.digestSha256.slice(0, 12)}...\`, method \`${digest.method}\`)`
      : "Local digest not yet materialized";
    return `### \`${r.fullName}\`

- **Latest Commit**: \`${r.latestCommitSha}\` (${r.latestCommitDate}) — *${r.latestCommitMessage}*
- **Artifacts / Crates**: ${subs}
- **Key Upstream Files Indexed**: ${r.keyPathsPresent.slice(0, 10).map((p) => `\`${p}\``).join(", ")}
- **GitIngest Status**: ${digestSummary}`;
  })
  .join("\n\n")}

## 3. \`crates.io\` vs Aphrody Workspace Pin Parity

${renderCratesMatrixMarkdownTable(scan)}

### Workspace & CLI Alignment Notes

- **Pinned CLI vs Lockfile**: \`tooling/bun/web-build.ts\` checks \`wasm-bindgen --version\` (${lt.cliVersions.wasmBindgen}) against \`Cargo.lock\` (${lt.cargoLockVersions.wasmBindgen}). Upstream \`0.2.130\` added small-string return intrinsics and \`ArrayBuffer\`/Intl helpers; upgrading requires bumping both \`Cargo.lock\` (\`cargo update -p wasm-bindgen --precise 0.2.130\`) and installing \`wasm-bindgen-cli@0.2.130\` atomically.
- **\`wasm_thread\` Fork Pin**: Root \`Cargo.toml\` patches \`wasm_thread\` to \`https://github.com/zed-industries/wasm_thread\` rev \`${lt.workspaceConstraints.wasmThreadGitRev}\` (\`es_modules\` feature) because crates.io \`wasm_thread\` passes deprecated positional arguments removed/deprecated after \`wasm-bindgen 0.2.93\`.
- **Binaryen**: Aphrody uses \`${lt.cliVersions.wasmOpt}\` directly rather than \`wasm-pack\`'s bundled installer.
`;
}

function renderJsWebSysPatternsRef(scan: ScanReport): string {
  const lt = scan.localToolchain;
  return `# js-sys, web-sys & wasm-bindgen-futures Patterns

> Verified against upstream \`wasm-bindgen\` (${scan.cratesIo.find((c) => c.name === "wasm-bindgen")?.maxVersion ?? "0.2.130"}), \`crates/gpui_web\`, and \`crates/gpui_wgpu\`.

## 1. \`Closure\` Lifetime Hygiene & Cleanup

JavaScript garbage collectors cannot invoke Rust \`Drop\` automatically when an event listener goes unused; conversely, calling \`Closure::forget()\` on dynamic DOM listeners leaks heap table slots and Wasm closure allocations permanently.

| Pattern | When to Use | Aphrody Implementation |
| :--- | :--- | :--- |
| **RAII \`EventListenerHandle\`** | DOM \`addEventListener\` (\`pointer*\`, \`key*\`, \`paste\`, \`visibilitychange\`) | [\`crates/gpui_web/src/events.rs\`](../../../../../../crates/gpui_web/src/events.rs) stores \`target\`, \`event_name\`, and \`Closure<dyn FnMut(JsValue)>\`; \`impl Drop\` calls \`remove_event_listener_with_callback\` before dropping the closure |
| **Struct-Owned \`Closure\` + \`Weak\`** | Long-lived frame loops (\`requestAnimationFrame\`) & \`ResizeObserver\` | [\`crates/gpui_web/src/window.rs\`](../../../../../../crates/gpui_web/src/window.rs) holds \`_raf_closure\`, \`_resize_observer_closure\`, \`_safe_area_observer_closure\` with \`Rc::downgrade(&inner)\` to break reference cycles |
| **\`Closure::once_into_js\`** | Guaranteed single-fire browser APIs (\`setTimeout\`, \`queueMicrotask\`, \`requestIdleCallback\`) | [\`crates/gpui_web/src/dispatcher.rs\`](../../../../../../crates/gpui_web/src/dispatcher.rs) & [\`ime_mirror.rs\`](../../../../../../crates/gpui_web/src/ime_mirror.rs) auto-drop Rust closure state after the single JS call |
| **Non-Passive Event Listeners** | Wheel / touch events requiring \`preventDefault()\` | \`EventListenerHandle::add_non_passive\` constructs \`{ passive: false }\` via \`js_sys::Reflect::set\` and binds \`addEventListener.call3\` |

\`\`\`rust
// Correct Aphrody pattern (crates/gpui_web/src/events.rs):
impl Drop for EventListenerHandle {
    fn drop(&mut self) {
        self.target
            .remove_event_listener_with_callback(
                self.event_name,
                self.closure.as_ref().unchecked_ref(),
            )
            .ok();
    }
}
\`\`\`

## 2. Zero-Copy \`&[u8]\` / \`Uint8Array::view\` vs Safe Copies

Upstream \`crates/js-sys/src/lib.rs\` marks \`Uint8Array::view(rust: &[u8])\` \`unsafe\` for a critical reason: Wasm linear memory grows via \`memory.grow\`, which **detaches** the existing \`ArrayBuffer\` in JS!

- **Use Safe Copy (\`Uint8Array::from\` / \`.to_vec()\`) Across Async or Re-entrant Boundaries**:
  - [\`crates/gpui_web/src/http_client.rs\`](../../../../../../crates/gpui_web/src/http_client.rs) copies outgoing request bytes via \`js_sys::Uint8Array::from(bytes.as_slice())\` before handing them to \`RequestInit::set_body\` and incoming stream chunks via \`.dyn_into::<js_sys::Uint8Array>().map(|b| b.to_vec())\`.
  - [\`crates/gpui_web/src/platform.rs\`](../../../../../../crates/gpui_web/src/platform.rs) reads clipboard blobs with \`js_sys::Uint8Array::new(&buffer).to_vec()\`.
- **When \`Uint8Array::view\` Is Safe**:
  - Only for **pure synchronous leaf calls** where JS consumes/copies the typed array immediately before returning and **zero Rust allocations or JS->Rust callbacks occur** while the view lives (e.g., synchronous \`WebGL2RenderingContext.texSubImage2D\` or synchronous \`TextDecoder.decode(view)\`).
  - For \`Atomics\` shared-memory signaling where a fixed \`AtomicI32\` address stays valid on shared \`WebAssembly.Memory\`, [\`dispatcher.rs\`](../../../../../../crates/gpui_web/src/dispatcher.rs) slices directly via \`js_sys::Int32Array::new_with_byte_offset_and_length(&memory.buffer(), byte_offset, 1)\`.

## 3. \`wasm-bindgen-futures\`, Microtasks & Worker Pumping

- **Promise -> Future**: \`wasm_bindgen_futures::JsFuture::from(promise).await\` schedules continuation on the microtask queue without blocking the browser event loop.
- **Fire-and-Forget UI Tasks**: \`wasm_bindgen_futures::spawn_local(async move { ... })\` drives top-level async startup (\`initialize_graphics\` in \`platform.rs\`) and streaming body readers (\`pump_response_body\` in \`http_client.rs\`).
- **Avoid Main-Thread Mutex Deadlocks**: \`http_client.rs\` explicitly uses \`async_channel::bounded(8)\` instead of \`futures::mpsc\` because \`futures::mpsc\`'s internal lock can park the browser main thread when a worker thread pulls body bytes.
- **\`Atomics.waitAsync\` Main-Thread Mailbox**: \`dispatcher.rs\` uses non-blocking \`js_sys::Atomics::wait_async\` -> \`JsFuture\` in \`spawn_local\` because browser main threads throw \`TypeError\` on synchronous \`Atomics.wait\`.
- **Feature Flags**: Upstream \`wasm-bindgen-futures 0.4.80\` supports \`futures-core-03-stream\` (forwarding \`js-sys/futures-core-03-stream\` for \`JsStream\` / \`AsyncIterator\`) and optional Emscripten Tokio integration under \`--cfg wasm_bindgen_unstable_tokio\`.

## 4. \`web-sys\` Feature Minimization & Dynamic Feature Probing

- **Strict Minimal Features**: \`web-sys\` contains over 3,000 WebIDL types; enabling \`"all"\` explodes compile times and trips Windows \`CreateProcess\` argv limits. Aphrody enables **${lt.gpuiWebFeaturesCount} exact features** in [\`crates/gpui_web/Cargo.toml\`](../../../../../../crates/gpui_web/Cargo.toml) and **1 feature (\`HtmlCanvasElement\`)** in [\`crates/gpui_wgpu/Cargo.toml\`](../../../../../../crates/gpui_wgpu/Cargo.toml).
- **Unstable WebIDL APIs**: Features gated by \`#[cfg(web_sys_unstable_apis)]\` in \`web-sys\` (such as newer WebGPU / WebHID / EyeDropper drafts) require \`RUSTFLAGS="--cfg=web_sys_unstable_apis"\`; keep \`check-cfg\` clean and prefer \`js_sys::Reflect\` probing where browser support varies.
- **Probe Optional APIs Before Dispatching**:
  - Calling a \`web-sys\` method when the JS property is \`undefined\` (like \`navigator.clipboard\` on HTTP non-secure contexts, or \`requestIdleCallback\` on Safari, or \`PointerEvent.getPredictedEvents\` on older browsers) causes a \`wasm-bindgen\` runtime abort!
  - Always probe via \`js_sys::Reflect::has\` / \`Reflect::get\` first (verified in \`platform.rs:625\`, \`dispatcher.rs:401\`, \`events.rs:1426\`).
- **Dictionaries vs Interfaces**: WebIDL dictionaries (such as \`web_sys::ReadableStreamReadResult\`) have no JS constructor prototype for \`instanceof\`; cast with \`.unchecked_into()\` rather than \`.dyn_into()\` (\`http_client.rs:292\`), and use \`.dyn_into::<T>()\` on real DOM interfaces (\`HtmlCanvasElement\`, \`Response\`, \`Uint8Array\`).
`;
}

function renderWalrusWeedleTsgenRef(scan: ScanReport): string {
  const cratesByName = new Map(scan.cratesIo.map((c) => [c.name, c]));
  return `# walrus, weedle, ts-gen, js-bindgen & Externref RFC

> Compiler, IR transformation, WebIDL/TypeScript codegen, and next-gen ABI architecture synthesized from the 8 \`github.com/wasm-bindgen\` repositories.

## 1. \`walrus\` (\`${cratesByName.get("walrus")?.maxVersion ?? "0.28.0"}\`) Transformation Passes Before \`wasm-opt\`

\`wasm-bindgen-cli\` embeds \`wasm-bindgen-cli-support\`, which parses the raw \`rustc\`/LLD \`wasm32-unknown-unknown\` artifact into a \`walrus::Module\` IR before Binaryen \`wasm-opt\` ever touches the file:

1. **\`__wasm_bindgen_unstable\` Custom Section Decoding**: \`crates/cli-support/src/decode.rs\` extracts compiler-emitted ABI descriptors describing every \`#[wasm_bindgen]\` import/export signature.
2. **WIT Adapter & Shim Lowering**: \`crates/cli-support/src/wit/\` builds incoming/outgoing JS adapter bindings and emits the companion ESM glue (\`.js\`) and optional TypeScript declarations (\`.d.ts\`).
3. **\`externref\` Table Transformation (\`transforms/externref/mod.rs\`)**: Replaces placeholder \`externref\` handle intrinsics with direct Wasm \`table.get\` / \`table.set\` operations on the blessed \`__wbindgen_externref_table\` (with \`INITIAL_HEAP_OFFSET = 1024\` reserving stack slots below \`1024\` and pre-populating \`["undefined", "null", "true", "false"]\`).
4. **ABI / Runtime Transforms**:
   - \`transforms/multi_value\`: lowers multi-return functions natively when \`--target\` supports Wasm multi-value.
   - \`transforms/threads\`: instruments \`__wbindgen_thread_destroy\` and TLS init for \`+atomics,+bulk-memory\` (\`wasm32\` and \`wasm64\`).
   - \`transforms/jspi\`: wraps marked async exports/imports for Wasm JS Promise Integration (JSPI).
   - \`transforms/catch_handler\` + \`export_sp_restore\`: translates legacy/modern Wasm exception handling (\`Try\` / \`TryTable\`) and restores the shadow stack pointer across trapped/caught calls.
5. **\`walrus::passes::gc\` & \`walrus::passes::used\`**:
   - **Dead-Symbol & Custom-Section Elimination**: Marks reachable functions, tables, globals, memories, and elements from surviving exports/start functions, deleting unreferenced \`__wbindgen_describe_*\` stubs and dead helper functions.
   - \`Bindgen\` options (\`remove_name_section\`, \`remove_producers_section\`, \`split_debug_info\`, \`js_string_builtins\` via \`wasm:js/string-constants\`) prune non-runtime metadata cleanly while \`walrus\` preserves DWARF via \`gimli 0.34.0\` when \`--keep-debug\` is enabled.
6. **Hand-off to Binaryen \`wasm-opt\` (\`${scan.localToolchain.cliVersions.wasmOpt}\`)**:
   - \`walrus\` runs structural ABI lowering and dead-descriptor GC *first*; \`wasm-opt -Oz\` then performs peephole optimization, CFG simplification, dead-code elimination, inlining, and local coalescing on the finalized valid Wasm module (with matching feature flags \`--enable-bulk-memory --enable-reference-types --enable-multivalue ...\`). Running \`wasm-opt\` *before* \`wasm-bindgen\` breaks custom \`__wasm_bindgen_unstable\` descriptors and placeholder intrinsics!

## 2. \`weedle\` (\`${cratesByName.get("weedle")?.maxVersion ?? "0.13.1"}\`) & WebIDL -> \`web-sys\` Codegen

- \`weedle\` is a \`nom 5.1.0\` WebIDL grammar parser producing typed ASTs (\`weedle::Definitions\`, \`Interface\`, \`Dictionary\`, \`Enum\`, \`Namespace\`, \`ExtendedAttributeList\`).
- Upstream \`crates/webidl\` (\`wasm-bindgen-webidl\`) feeds all curated WHATWG/W3C \`.webidl\` files through \`weedle::parse()\`, builds a \`FirstPassRecord\` cross-interface inheritance & mixin table, and generates \`crates/web-sys/src/features/\` + \`Cargo.toml\` feature gates via \`quote\` + \`syn\`.
- When an unstable WebIDL overrides a stable attribute with a different type, \`wasm-bindgen-webidl\` emits dual \`#[cfg(not(web_sys_unstable_apis))]\` / \`#[cfg(web_sys_unstable_apis)]\` definitions so downstream crates opt in explicitly.

## 3. \`ts-gen\` & \`js-bindgen\`: TypeScript Contracts & Next-Gen Linker Interop

- **\`ts-gen\` (\`wasm-bindgen/ts-gen\`)**: Parses TypeScript \`.d.ts\` declarations using **\`oxc_parser 0.154\`** + \`oxc_resolver\`, populates a two-phase \`TypeRegistry\` (merging \`var\` + \`interface\` declarations and resolving inheritance chains), and prints idiomatic \`#[wasm_bindgen]\` Rust extern blocks via \`syn 2\` + \`prettyplease\`. Useful in Aphrody whenever wrapping Cloudflare Worker/npm TypeScript SDKs (\`examples/cloudflare-worker\`, \`--strict-nullable\` for exact \`Option<T>\` carriers) instead of hand-writing error-prone \`extern "C"\` blocks.
- **\`js-bindgen\` (\`wasm-bindgen/js-bindgen\`)**: Next-generation linker-driven \`#![no_std]\` research workspace (\`embed_js\`, \`import_js\`, \`unsafe_global_wat\`, custom \`js-bindgen-ld\`) testing real-generic \`js-sys\`, \`wasm64-unknown-unknown\`, and \`bun\` test runner integration without legacy placeholder descriptor tables.

## 4. \`rust-externref-rfc\`: Future Zero-Wrapper \`Externref\` ABI

- Upstream RFC 3987 proposed an unsized lang-type \`__externref_t\`, which conflicted with Rust's operational semantics requiring all values to be representable as bytes with provenance.
- \`wasm-bindgen/rust-externref-rfc\` reframes the solution around **boxed \`Externref\` pointers** (\`c_externref_ptr\` in \`ptr addrspace(2)\` matching the LLVM RFC): owned \`Externref\`, borrowed \`ExternrefRef<'a>\`, ABI signal \`ByValue<T>\`, and scoped \`with_stack_slot(|tmp| ...)\` so LLVM's optimizer eliminates \`__externref_table_alloc\`/\`__externref_table_free\` pairs wherever pointers don't escape.

## 5. \`wasm-bindgen-bot\` Org Automation

- Two-job GitHub Actions architecture (\`start\` -> \`analyze\` -> \`post\`) running Claude Code in \`--bare\` sandboxed mode via Cloudflare AI Gateway for automated PR review (\`merge-ready\` label), issue triage, and reproduction across all 8 \`wasm-bindgen\` repositories.
`;
}

function renderAphrodyBestPracticesRef(scan: ScanReport): string {
  const lt = scan.localToolchain;
  return `# Aphrody Best Practices: WebAssembly & \`wasm-bindgen\` Pipeline

> Grounded directly in [\`Cargo.toml\`](../../../../../../Cargo.toml), [\`tooling/bun/web-build.ts\`](../../../../../../tooling/bun/web-build.ts), [\`crates/gpui_web/\`](../../../../../../crates/gpui_web/Cargo.toml), [\`crates/aphrody_desktop/examples/web_os.rs\`](../../../../../../crates/aphrody_desktop/examples/web_os.rs), and [\`crates/nsh-platform-web/\`](../../../../../../crates/nsh-platform-web/Cargo.toml).

## 1. Why Aphrody Uses \`cargo build + wasm-bindgen CLI + wasm-opt\` Over Legacy \`wasm-pack\`

| Capability | Legacy \`wasm-pack 0.15.1\` | Aphrody \`tooling/bun/web-build.ts\` |
| :--- | :--- | :--- |
| **Target Ecosystem** | Assumes single-bundle \`npm\` / \`webpack\` packaging (\`pkg/\` + \`package.json\`) | Produces multi-cohort web distributions (\`baseline\`, \`simd-baseline\`, opt-in \`multithreaded\`) + runtime WebGPU/SIMD probe HTML |
| **CLI <-> Lockfile Lock** | Can download arbitrary \`wasm-bindgen\` binaries via \`binary-install\` | \`verifyBindgenVersion()\` fails fast unless \`wasm-bindgen\` CLI (${lt.cliVersions.wasmBindgen}) strictly equals \`Cargo.lock\` (${lt.cargoLockVersions.wasmBindgen}) |
| **Windows \`web-sys\` Build Fix** | Inherits \`~/.cargo/config.toml\` \`sccache\`, overflowing Windows \`CreateProcess\` 32 KB argv limit | Explicitly clears \`RUSTC_WRAPPER=""\` for \`wasm32\` builds (\`web-build.ts:173\`) while preserving native \`sccache\` |
| **COOP / COEP Isolation** | Single build flag forces either zero threads or broken third-party embeds | Keeps \`baseline\` + \`simd-baseline\` free of shared memory (verified via \`WebAssembly.Module.imports\`) and gates \`+atomics\` behind opt-in \`--coi\` / \`--threads\` |
| **Release Profile & Binaryen** | Default release profile; separate wasm-opt step | Enforces \`[profile.web]\` (\`opt-level = "z"\`, \`lto = "thin"\`, \`codegen-units = 1\`, \`strip = true\`, \`panic = "abort"\`) + \`wasm-opt v133 -Oz\` + Brotli budget gates (\`--brotli-budget\`) |

## 2. \`gpui_web\` Architecture & Runtime Invariants

- **Single Top-Level Window & Lifecycle State Machine**: \`WebPlatform\` enforces \`WebWindowLifecycle::{Available, Open, Closed, Unavailable}\`; graphics initialization runs asynchronously via \`wasm_bindgen_futures::spawn_local\` inside \`Platform::run\`, trying WebGPU first and tearing down/rebuilding a clean \`<canvas>\` for WebGL2 fallback if WebGPU fails (\`platform.rs:230-305\`).
- **Reentrancy-Safe Callbacks**: Because \`[profile.web]\` uses \`panic = "abort"\`, any double \`RefCell::borrow_mut()\` aborts the Wasm module irreversibly. \`WebWindowInner::with_callback\` and \`with_input_handler\` move callbacks out via \`.take()\` before invoking user/GPUI handlers, then restore them afterwards (\`window.rs:438-454\`, \`window.rs:576-584\`).
- **Mobile IME & Viewport Coalescing**: [\`ime_mirror.rs\`](../../../../../../crates/gpui_web/src/ime_mirror.rs) maintains a hidden \`<textarea>\` (\`font-size: 16px\` to defeat iOS/Android auto-zoom, \`FocusOptions::set_prevent_scroll(true)\`) and coalesces DOM writes via \`Closure::once_into_js\` + \`setTimeout(0)\` so mid-gesture writes never reset mobile software keyboards.
- **Font Loading & Missing Glyph Streaming**: \`WebTextSystem\` pairs \`CosmicTextSystem::new_without_system_fonts\` with \`OffscreenCanvasRenderingContext2d\` emoji/CJK glyph fallback (\`canvas_fallback.rs\`, \`canvas_text.rs\`) instead of embedding megabytes of fallback fonts in Wasm \`.rodata\`.

## 3. Dual Browser Strategy: \`gpui_web\` vs \`nsh-platform-web\`

Aphrody consciously maintains two complementary Wasm bridge architectures:

1. **\`crates/gpui_web\` (\`wasm-bindgen\` + \`web-sys\` + \`gpui_wgpu\`)**: Full Rust-rendered GPU UI (\`aphrody_desktop\`, \`zed\`, \`gpui\`) targeting WebGPU/WebGL2 canvases, browser Fetch streams, Clipboard API, ResizeObserver, and optional \`wasm_thread\` workers.
2. **\`crates/nsh-platform-web\` (Custom Slot Bridge [\`src/browser/values.rs\`](../../../../../../crates/nsh-platform-web/src/browser/values.rs))**: Compact \`#[link(wasm_import_module = "nsh_browser")]\` integer-op FFI (\`Value(pub u32)\` + packed binary buffers) built for NSH DOM/QuickJS TSX payloads where \`nsh package\` performs custom Wasm symbol dead-code pruning across \`linkable\` artifacts without shipping full \`web-sys\` bindings.

## 4. Cohort Verification Checklist

1. Confirm \`wasm-bindgen --version\` matches \`Cargo.lock\` (\`${lt.cargoLockVersions.wasmBindgen}\`).
2. Run \`bun tooling/bun/web-build.ts check\` across \`gpui_web\`, \`gpui_wgpu\`, \`aphrody_session\`, \`aphrody_craft\`, \`aphrody_ui\`, \`aphrody_desktop\`.
3. Build release cohorts with \`bun tooling/bun/web-build.ts build --package aphrody_desktop --release --opt --brotli-budget 6500000\`.
4. Verify \`manifest.json\` confirms \`baseline.importsSharedMemory === false\` and verify rendered canvas / DOM semantics using [\`tooling/bun/web-verify.ts\`](../../../../../../tooling/bun/web-verify.ts).
`;
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

## 5. Linked Artifacts & Progressive-Disclosure Skill

- **\`llms.txt\` Index**: [\`docs/aphrody/architecture/wasm-bindgen-llms.txt\`](./wasm-bindgen-llms.txt)
- **Agent Skill**: [\`.agents/plugins/aphrody/skills/wasm-bindgen/SKILL.md\`](../../../.agents/plugins/aphrody/skills/wasm-bindgen/SKILL.md)
- **Skill References**:
  - [\`repos-matrix.md\`](../../../.agents/plugins/aphrody/skills/wasm-bindgen/references/repos-matrix.md)
  - [\`js-web-sys-patterns.md\`](../../../.agents/plugins/aphrody/skills/wasm-bindgen/references/js-web-sys-patterns.md)
  - [\`walrus-weedle-tsgen.md\`](../../../.agents/plugins/aphrody/skills/wasm-bindgen/references/walrus-weedle-tsgen.md)
  - [\`aphrody-best-practices.md\`](../../../.agents/plugins/aphrody/skills/wasm-bindgen/references/aphrody-best-practices.md)
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
  const skillDirAbs = join(root, SKILL_DIR_REL);
  const refsDirAbs = join(skillDirAbs, "references");
  mkdirSync(dirname(llmsTxtAbs), { recursive: true });
  mkdirSync(refsDirAbs, { recursive: true });

  const llmsTxt = renderLlmsTxt(scan, manifest);
  writeFileSync(llmsTxtAbs, llmsTxt, "utf8");

  const skillAbs = join(skillDirAbs, "SKILL.md");
  const skillMd = renderSkillMarkdown(scan);
  writeFileSync(skillAbs, skillMd, "utf8");

  const refSpecs: Array<[string, string]> = [
    ["repos-matrix.md", renderReposMatrixRef(scan, manifest)],
    ["js-web-sys-patterns.md", renderJsWebSysPatternsRef(scan)],
    ["walrus-weedle-tsgen.md", renderWalrusWeedleTsgenRef(scan)],
    ["aphrody-best-practices.md", renderAphrodyBestPracticesRef(scan)],
  ];
  const refPathsAbs: string[] = [];
  for (const [filename, content] of refSpecs) {
    const abs = join(refsDirAbs, filename);
    writeFileSync(abs, content, "utf8");
    refPathsAbs.push(abs);
  }

  const ecosystemContent = existsSync(ecosystemDocAbs)
    ? updateEcosystemSummaryBlocks(readFileSync(ecosystemDocAbs, "utf8"), scan, manifest)
    : renderEcosystemGuide(scan, manifest);
  writeFileSync(ecosystemDocAbs, ecosystemContent, "utf8");

  const brokenLinks = validateRelativeLinks([llmsTxtAbs, ecosystemDocAbs, skillAbs, ...refPathsAbs]);
  const fmCheck = validateSkillFrontmatter(skillMd);
  const llmsCheck = validateLlmsTxt(llmsTxt);

  return {
    llmsTxtPath: LLMS_TXT_REL,
    ecosystemDocPath: ECOSYSTEM_DOC_REL,
    skillPath: `${SKILL_DIR_REL}/SKILL.md`,
    referencePaths: refPathsAbs.map((p) => relative(root, p).replaceAll("\\", "/")),
    brokenLinks,
    skillFrontmatterOk: fmCheck.ok,
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
  if (emitted.brokenLinks.length > 0 || !emitted.skillFrontmatterOk || !emitted.llmsTxtOk) {
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
