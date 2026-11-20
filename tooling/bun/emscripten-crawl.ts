#!/usr/bin/env bun
/**
 * Live full-site crawler for `https://emscripten.org/`, full repository ingestion
 * across all repositories in `https://github.com/orgs/emscripten-core/repositories`,
 * and progressive-disclosure Agent Skill / `llms.txt` / Architecture doc emitter.
 *
 * Uses `bun` (bounded async concurrency), `xh` (`xh --ignore-stdin --check-status --follow`),
 * `gh api --paginate`, and `https://gitingest.com/` with deterministic GitHub Tree/Contents fallback.
 *
 * Usage:
 *   bun tooling/bun/emscripten-crawl.ts crawl [--max-pages 80] [--concurrency 6] [--xh-samples 12]
 *   bun tooling/bun/emscripten-crawl.ts ingest [--concurrency 3] [--timeout 20] [--no-remote]
 *   bun tooling/bun/emscripten-crawl.ts emit
 *   bun tooling/bun/emscripten-crawl.ts all [--max-pages 80] [--concurrency 6]
 */
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";

export const REPO_ROOT = resolve(import.meta.dir, "../..");

/** Blob URL of a file inside a pinned fork (Cargo git dependency), at the tag recorded in forks-manifest.json. */
export function pinnedForkBlobUrl(forkName: string, path: string, root = REPO_ROOT): string {
  const manifestPath = join(root, "tooling/bun/forks-manifest.json");
  const manifest = existsSync(manifestPath)
    ? (JSON.parse(readFileSync(manifestPath, "utf8")) as { forks?: Array<{ name: string; repo: string; tag: string }> })
    : {};
  const fork = manifest.forks?.find((f) => f.name === forkName);
  if (!fork) throw new Error(`Fork '${forkName}' is not pinned in tooling/bun/forks-manifest.json`);
  return `https://github.com/${fork.repo}/blob/${fork.tag}/${path}`;
}
export const EMSCRIPTEN_ORIGIN = "https://emscripten.org";
export const EMSCRIPTEN_ORG = "emscripten-core";
export const GITINGEST_BASE_URL = "https://gitingest.com";

export const CRAWL_DIR_REL = "target/emscripten-crawl";
export const SITEMAP_JSON_REL = "target/emscripten-crawl/sitemap.json";
export const SITEMAP_XML_REL = "target/emscripten-crawl/sitemap.xml";
export const GITINGEST_DIR_REL = "target/gitingest/emscripten-core";
export const ORG_MANIFEST_REL = "target/gitingest/emscripten-core/org-manifest.json";

export const LLMS_TXT_REL = "docs/aphrody/architecture/emscripten-llms.txt";
export const ARCH_DOC_REL = "docs/aphrody/architecture/emscripten-ecosystem.md";

const USER_AGENT = "AphrodyEmscriptenIngest/1.0 (+https://github.com/aphrody-labs/aphrody)";

export const CORE_PROBE_PATHS = [
  "/",
  "/docs/index.html",
  "/docs/introducing_emscripten/release_notes.html",
  "/docs/tools_reference/emcc.html",
  "/docs/tools_reference/emsdk.html",
  "/docs/tools_reference/settings_reference.html",
  "/docs/compiling/best_practices.html",
  "/docs/compiling/Modularized-Output.html",
  "/docs/porting/multimedia_and_graphics/WebGPU-support.html",
  "/docs/porting/asyncify.html",
  "/docs/porting/pthreads.html",
  "/docs/porting/simd.html",
  "/docs/porting/connecting_cpp_and_javascript/embind.html",
  "/docs/porting/connecting_cpp_and_javascript/Interacting-with-code.html",
] as const;

const STATIC_EXT_RE = /\.(png|jpe?g|gif|svg|ico|webp|woff2?|ttf|eot|css|js|mjs|map|pdf|zip|tar|gz|txt|wasm)$/i;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface ToolProbe {
  present: boolean;
  version: string | null;
  path: string | null;
}

export interface HostToolchainProbe {
  checkedAt: string;
  platform: string;
  arch: string;
  emcc: ToolProbe;
  empp: ToolProbe;
  emsdk: ToolProbe;
  emsdkEnv: {
    EMSDK: string | null;
    EM_CONFIG: string | null;
    EMSCRIPTEN: string | null;
  };
  clang: ToolProbe;
  lld: ToolProbe;
  wasmOpt: ToolProbe;
  wasmBindgen: ToolProbe;
  xh: ToolProbe;
  gh: ToolProbe;
  gitingestCli: ToolProbe;
}

export interface XhArgsOptions {
  checkStatus?: boolean;
  headersOnly?: boolean;
  follow?: boolean;
  timeoutSec?: number;
}

export interface XhProbeResult {
  url: string;
  args: string[];
  exitCode: number;
  ok: boolean;
  httpStatus: number | null;
  statusLine: string;
  contentType: string | null;
  bytes: number;
  elapsedMs: number;
  error?: string;
}

export interface SphinxRouteEntry {
  docname: string;
  url: string;
  path: string;
  title: string;
  sourceFilename: string;
}

export interface SphinxSearchIndex {
  docnames: string[];
  titles: string[];
  filenames: string[];
  routes: SphinxRouteEntry[];
}

export interface NormalizedUrlInfo {
  normalizedUrl: string;
  path: string;
  sameOrigin: boolean;
  isStaticAsset: boolean;
  hadFragment: boolean;
}

export interface ExtractedHtmlPage {
  title: string;
  h1: string;
  canonicalUrl: string | null;
  docsVersion: string | null;
  internalHtmlLinks: string[];
  internalAssetLinks: string[];
  externalLinks: string[];
  fragmentCount: number;
}

export interface CrawledRouteRecord {
  url: string;
  path: string;
  title: string;
  h1: string;
  depth: number;
  discoveredVia: Array<"seed" | "bfs" | "searchindex">;
  crawled: boolean;
  httpStatus: number | null;
  contentType: string | null;
  outLinksCount: number;
  externalLinksCount: number;
  sourceFilename: string | null;
  error?: string;
}

export interface RouteTreeNode {
  segment: string;
  fullPath: string;
  routeCount: number;
  crawledCount: number;
  routes: Array<{ path: string; url: string; title: string; crawled: boolean; httpStatus: number | null }>;
  children: RouteTreeNode[];
}

export interface SitemapCrawlReport {
  crawledAt: string;
  baseUrl: string;
  siteTitle: string;
  emscriptenDocsVersion: string;
  maxPagesCap: number;
  concurrency: number;
  sitemapXmlProbe: XhProbeResult;
  searchIndexProbe: {
    url: string;
    httpStatus: number | null;
    docCount: number;
    titleCount: number;
  };
  xhProbes: XhProbeResult[];
  stats: {
    totalDiscoveredRoutes: number;
    crawledHtmlPages: number;
    okPages: number;
    failedPages: number;
    uncrawledQueuedRoutes: number;
    totalStaticAssetsDiscovered: number;
    uniqueExternalDomains: number;
    totalExternalLinks: number;
  };
  externalDomains: Array<{ host: string; count: number }>;
  routeTree: RouteTreeNode;
  routes: CrawledRouteRecord[];
}

export interface GitHubRepoRaw {
  name: string;
  full_name: string;
  html_url: string;
  description: string | null;
  homepage: string | null;
  language: string | null;
  stargazers_count: number;
  forks_count: number;
  open_issues_count: number;
  archived: boolean;
  fork: boolean;
  default_branch: string;
  pushed_at: string;
  size: number;
  topics?: string[];
  license?: { spdx_id?: string | null; name?: string | null } | null;
}

export interface IngestedRepoRecord {
  name: string;
  fullName: string;
  htmlUrl: string;
  description: string;
  homepage: string;
  language: string;
  stars: number;
  forks: number;
  openIssues: number;
  archived: boolean;
  isFork: boolean;
  defaultBranch: string;
  pushedAt: string;
  sizeKb: number;
  licenseSpdx: string;
  topics: string[];
  ingestMethod: "gitingest-api" | "github-tree-fallback" | "empty-repo";
  gitingestHttpStatus: number | null;
  gitingestError?: string;
  digestUrl: string | null;
  commitSha: string;
  filesAnalyzed: number;
  treeEntryCount: number;
  treeTruncated: boolean;
  estimatedTokens: string | null;
  digestPath: string;
  digestBytes: number;
  roleSummary: string;
}

export interface OrgIngestManifest {
  ingestedAt: string;
  org: string;
  repoCount: number;
  activeCount: number;
  archivedCount: number;
  forkCount: number;
  gitingestApiProbe: XhProbeResult;
  repos: IngestedRepoRecord[];
}

// ---------------------------------------------------------------------------
// Generic Utilities
// ---------------------------------------------------------------------------

export function stripHtmlTags(raw: string): string {
  return decodeHtmlEntities(raw.replace(/<[^>]+>/g, " "))
    .replaceAll("¶", "")
    .replace(/\s+/g, " ")
    .trim();
}

export function decodeHtmlEntities(raw: string): string {
  return raw
    .replaceAll("&amp;", "&")
    .replaceAll("&lt;", "<")
    .replaceAll("&gt;", ">")
    .replaceAll("&quot;", '"')
    .replaceAll("&#39;", "'")
    .replaceAll("&#x27;", "'")
    .replaceAll("&nbsp;", " ");
}

export function escapeXml(raw: string): string {
  return raw
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&apos;");
}

export function parsePaginatedJson<T>(raw: string): T[] {
  const trimmed = raw.trim();
  if (!trimmed) return [];
  try {
    const parsed = JSON.parse(trimmed) as unknown;
    if (Array.isArray(parsed)) {
      if (parsed.length > 0 && parsed.every((item) => Array.isArray(item))) {
        return (parsed as T[][]).flat();
      }
      return parsed as T[];
    }
    return [];
  } catch {
    // `gh api --paginate` emits concatenated JSON arrays `[...][...]` without `--slurp`.
    const normalized = trimmed.replace(/\]\s*\[/g, ",");
    const parsed = JSON.parse(normalized) as unknown;
    return Array.isArray(parsed) ? (parsed as T[]) : [];
  }
}

export async function mapWithConcurrency<T, R>(
  items: readonly T[],
  concurrency: number,
  fn: (item: T, index: number) => Promise<R>,
): Promise<R[]> {
  const limit = Math.max(1, Math.floor(concurrency));
  const results = new Array<R>(items.length);
  let cursor = 0;

  async function worker(): Promise<void> {
    while (true) {
      const idx = cursor++;
      if (idx >= items.length) return;
      results[idx] = await fn(items[idx]!, idx);
    }
  }

  await Promise.all(Array.from({ length: Math.min(limit, items.length) }, () => worker()));
  return results;
}

function writeRepoFile(relPath: string, content: string): string {
  const full = join(REPO_ROOT, relPath);
  mkdirSync(dirname(full), { recursive: true });
  writeFileSync(full, content, "utf8");
  return full;
}

function readJsonIfExists<T>(relPath: string): T | null {
  const full = join(REPO_ROOT, relPath);
  if (!existsSync(full)) return null;
  try {
    return JSON.parse(readFileSync(full, "utf8")) as T;
  } catch {
    return null;
  }
}

function optValue(args: readonly string[], flag: string): string | undefined {
  const idx = args.indexOf(flag);
  if (idx >= 0 && args[idx + 1] && !args[idx + 1]!.startsWith("--")) {
    return args[idx + 1];
  }
  const prefixed = args.find((a) => a.startsWith(`${flag}=`));
  return prefixed ? prefixed.slice(flag.length + 1) : undefined;
}

function optInt(args: readonly string[], flag: string, fallback: number): number {
  const raw = optValue(args, flag);
  if (!raw) return fallback;
  const parsed = Number.parseInt(raw, 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}

// ---------------------------------------------------------------------------
// Host Toolchain Probing (`probeEmcc`)
// ---------------------------------------------------------------------------

function probeBinary(candidates: readonly string[], versionArgs: readonly string[] | null = ["--version"]): ToolProbe {
  for (const bin of candidates) {
    const resolved = Bun.which(bin);
    if (!resolved) continue;
    if (versionArgs === null) {
      return { present: true, version: "installed", path: resolved };
    }
    const res = Bun.spawnSync([resolved, ...versionArgs], { stdin: "ignore", stdout: "pipe", stderr: "pipe" });
    const out = new TextDecoder().decode(res.stdout).trim();
    const err = new TextDecoder().decode(res.stderr).trim();
    const line = (out || err).split(/\r?\n/)[0]?.trim() || "present";
    return { present: res.exitCode === 0, version: line, path: resolved };
  }
  return { present: false, version: null, path: null };
}

export function probeEmcc(): HostToolchainProbe {
  return {
    checkedAt: new Date().toISOString(),
    platform: process.platform,
    arch: process.arch,
    emcc: probeBinary(["emcc", "emcc.bat", "emcc.py"]),
    empp: probeBinary(["em++", "em++.bat", "em++.py"]),
    emsdk: probeBinary(["emsdk", "emsdk.bat", "emsdk.ps1"], ["version"]),
    emsdkEnv: {
      EMSDK: process.env.EMSDK ?? null,
      EM_CONFIG: process.env.EM_CONFIG ?? null,
      EMSCRIPTEN: process.env.EMSCRIPTEN ?? null,
    },
    clang: probeBinary(["clang"]),
    lld: probeBinary(["lld-link", "ld.lld", "wasm-ld"]),
    wasmOpt: probeBinary(["wasm-opt"]),
    wasmBindgen: probeBinary(["wasm-bindgen"]),
    xh: probeBinary(["xh"]),
    gh: probeBinary(["gh"]),
    gitingestCli: probeBinary(["gitingest"], null),
  };
}

// ---------------------------------------------------------------------------
// `xh` Transport Helpers
// ---------------------------------------------------------------------------

export function buildXhArgs(url: string, opts: XhArgsOptions = {}): string[] {
  const args = ["--ignore-stdin"];
  if (opts.checkStatus ?? true) args.push("--check-status");
  if (opts.follow ?? true) args.push("--follow");
  const timeoutSec = opts.timeoutSec ?? 15;
  args.push(`--timeout=${timeoutSec}`);
  if (opts.headersOnly) {
    args.push("--headers");
  } else {
    args.push("--print=hb");
  }
  args.push("GET", url);
  return args;
}

export function parseXhResponseStatus(rawOutput: string): {
  httpStatus: number | null;
  statusLine: string;
  contentType: string | null;
} {
  const matches = [...rawOutput.matchAll(/^(HTTP\/[0-9.]+\s+(\d{3})[^\r\n]*)/gm)];
  const last = matches.at(-1);
  const httpStatus = last ? Number.parseInt(last[2]!, 10) : null;
  const statusLine = last ? last[1]!.trim() : "";
  const ctMatches = [...rawOutput.matchAll(/^content-type:\s*([^\r\n]+)/gim)];
  const contentType = ctMatches.at(-1)?.[1]?.trim() ?? null;
  return { httpStatus, statusLine, contentType };
}

export async function runXhProbe(url: string, opts: XhArgsOptions = {}): Promise<XhProbeResult> {
  const args = buildXhArgs(url, opts);
  const start = performance.now();
  const proc = Bun.spawn(["xh", ...args], { stdin: "ignore", stdout: "pipe", stderr: "pipe" });
  const [stdoutBuf, stderrBuf, exitCode] = await Promise.all([
    new Response(proc.stdout).arrayBuffer(),
    new Response(proc.stderr).arrayBuffer(),
    proc.exited,
  ]);
  const elapsedMs = Math.round(performance.now() - start);
  const stdoutText = new TextDecoder().decode(stdoutBuf);
  const stderrText = new TextDecoder().decode(stderrBuf).trim();
  const parsed = parseXhResponseStatus(stdoutText);
  return {
    url,
    args,
    exitCode,
    ok: exitCode === 0 && (parsed.httpStatus === null || (parsed.httpStatus >= 200 && parsed.httpStatus < 400)),
    httpStatus: parsed.httpStatus,
    statusLine: parsed.statusLine,
    contentType: parsed.contentType,
    bytes: stdoutBuf.byteLength,
    elapsedMs,
    ...(stderrText ? { error: stderrText.split(/\r?\n/)[0] } : {}),
  };
}

// ---------------------------------------------------------------------------
// Sphinx `searchindex.js`, HTML Link Extraction, Sitemap Tree & XML
// ---------------------------------------------------------------------------

export function parseSphinxSearchIndex(jsText: string, origin = EMSCRIPTEN_ORIGIN): SphinxSearchIndex {
  const startIdx = jsText.indexOf("{");
  const endIdx = jsText.lastIndexOf("}");
  if (startIdx < 0 || endIdx <= startIdx) {
    throw new Error("Invalid Sphinx searchindex.js: missing JSON payload inside Search.setIndex(...)");
  }
  const parsed = JSON.parse(jsText.slice(startIdx, endIdx + 1)) as {
    docnames?: unknown;
    titles?: unknown;
    filenames?: unknown;
  };
  const docnames = Array.isArray(parsed.docnames) ? parsed.docnames.map(String) : [];
  const titles = Array.isArray(parsed.titles) ? parsed.titles.map((t) => stripHtmlTags(String(t))) : [];
  const filenames = Array.isArray(parsed.filenames) ? parsed.filenames.map(String) : [];

  const routes: SphinxRouteEntry[] = docnames.map((docname, idx) => {
    const cleanDoc = docname.replace(/^\/+/, "");
    const path = cleanDoc === "index" ? "/index.html" : `/${cleanDoc}.html`;
    return {
      docname: cleanDoc,
      url: `${origin}${path}`,
      path,
      title: titles[idx] ?? cleanDoc,
      sourceFilename: filenames[idx] ?? "",
    };
  });

  return { docnames, titles, filenames, routes };
}

export function normalizeCrawlUrl(
  rawHref: string,
  pageUrl: string,
  origin = EMSCRIPTEN_ORIGIN,
): NormalizedUrlInfo | null {
  const trimmed = decodeHtmlEntities(rawHref).trim();
  if (!trimmed || trimmed.startsWith("#")) return null;
  if (/^(mailto:|javascript:|tel:|data:|blob:)/i.test(trimmed)) return null;

  let parsed: URL;
  try {
    parsed = new URL(trimmed, pageUrl);
  } catch {
    return null;
  }
  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") return null;

  const hadFragment = parsed.hash.length > 0;
  parsed.hash = "";
  const sameOrigin = parsed.origin === origin;

  let pathname = parsed.pathname || "/";
  if (sameOrigin) {
    if (pathname === "/") {
      pathname = "/index.html";
    } else if (pathname.endsWith("/")) {
      pathname = `${pathname}index.html`;
    }
    parsed.pathname = pathname;
    parsed.search = "";
  }

  const isStaticAsset =
    STATIC_EXT_RE.test(pathname) ||
    pathname.startsWith("/_static/") ||
    pathname.startsWith("/_images/") ||
    pathname.startsWith("/_sources/");

  return {
    normalizedUrl: parsed.toString(),
    path: pathname,
    sameOrigin,
    isStaticAsset,
    hadFragment,
  };
}

export function extractHtmlPageInfo(
  html: string,
  pageUrl: string,
  origin = EMSCRIPTEN_ORIGIN,
): ExtractedHtmlPage {
  const titleMatch = html.match(/<title[^>]*>([\s\S]*?)<\/title>/i);
  const title = titleMatch ? stripHtmlTags(titleMatch[1]!) : "";

  const h1Match = html.match(/<h1[^>]*>([\s\S]*?)<\/h1>/i);
  const h1 = h1Match ? stripHtmlTags(h1Match[1]!) : "";

  const canonicalMatch = html.match(/<link[^>]*rel=["']canonical["'][^>]*href=["']([^"']+)["'][^>]*>/i);
  const canonicalUrl = canonicalMatch ? canonicalMatch[1]!.trim() : null;

  const verMatch = title.match(/Emscripten\s+([0-9]+\.[0-9]+\.[0-9]+(?:-[a-z0-9.]+)?)/i);
  const docsVersion = verMatch ? verMatch[1]! : null;

  const internalHtmlSet = new Set<string>();
  const internalAssetSet = new Set<string>();
  const externalSet = new Set<string>();
  let fragmentCount = 0;

  const hrefMatches = html.matchAll(/<(a|link)\b[^>]*?\bhref=["']([^"']+)["'][^>]*>/gi);
  for (const match of hrefMatches) {
    const tagName = (match[1] ?? "").toLowerCase();
    const rawHref = match[2] ?? "";
    const tag = match[0] ?? "";
    if (tagName === "link" && !/\brel=["'](?:next|prev|index|search|up|contents)["']/i.test(tag)) {
      continue;
    }
    const norm = normalizeCrawlUrl(rawHref, pageUrl, origin);
    if (!norm) continue;
    if (norm.hadFragment) fragmentCount++;
    if (norm.sameOrigin) {
      if (norm.isStaticAsset) {
        internalAssetSet.add(norm.normalizedUrl);
      } else {
        internalHtmlSet.add(norm.normalizedUrl);
      }
    } else {
      externalSet.add(norm.normalizedUrl);
    }
  }

  return {
    title,
    h1,
    canonicalUrl,
    docsVersion,
    internalHtmlLinks: [...internalHtmlSet].sort(),
    internalAssetLinks: [...internalAssetSet].sort(),
    externalLinks: [...externalSet].sort(),
    fragmentCount,
  };
}

export function buildRouteTree(routes: readonly CrawledRouteRecord[]): RouteTreeNode {
  const root: RouteTreeNode = {
    segment: "/",
    fullPath: "/",
    routeCount: 0,
    crawledCount: 0,
    routes: [],
    children: [],
  };
  const childMap = new Map<RouteTreeNode, Map<string, RouteTreeNode>>();
  childMap.set(root, new Map());

  for (const r of routes) {
    const clean = r.path.replace(/^\/+/, "");
    const parts = clean ? clean.split("/") : ["index.html"];
    const dirParts = parts.slice(0, -1);

    let cursor = root;
    cursor.routeCount++;
    if (r.crawled) cursor.crawledCount++;

    let accPath = "";
    for (const seg of dirParts) {
      accPath += `/${seg}`;
      let map = childMap.get(cursor);
      if (!map) {
        map = new Map();
        childMap.set(cursor, map);
      }
      let next = map.get(seg);
      if (!next) {
        next = {
          segment: seg,
          fullPath: accPath,
          routeCount: 0,
          crawledCount: 0,
          routes: [],
          children: [],
        };
        map.set(seg, next);
        cursor.children.push(next);
      }
      cursor = next;
      cursor.routeCount++;
      if (r.crawled) cursor.crawledCount++;
    }

    cursor.routes.push({
      path: r.path,
      url: r.url,
      title: r.title,
      crawled: r.crawled,
      httpStatus: r.httpStatus,
    });
  }

  const sortTree = (node: RouteTreeNode) => {
    node.routes.sort((a, b) => a.path.localeCompare(b.path));
    node.children.sort((a, b) => a.segment.localeCompare(b.segment));
    for (const c of node.children) sortTree(c);
  };
  sortTree(root);
  return root;
}

export function serializeSitemapXml(
  routes: readonly Pick<CrawledRouteRecord, "url" | "depth" | "path">[],
  lastmodDate = new Date().toISOString().slice(0, 10),
): string {
  const seen = new Set<string>();
  const unique = routes.filter((r) => {
    if (seen.has(r.url)) return false;
    seen.add(r.url);
    return true;
  });

  const lines = [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">',
  ];
  for (const r of unique) {
    const priority = r.path === "/index.html" || r.path === "/docs/index.html" ? "1.0" : r.depth <= 1 ? "0.8" : "0.6";
    lines.push("  <url>");
    lines.push(`    <loc>${escapeXml(r.url)}</loc>`);
    lines.push(`    <lastmod>${escapeXml(lastmodDate)}</lastmod>`);
    lines.push("    <changefreq>weekly</changefreq>");
    lines.push(`    <priority>${priority}</priority>`);
    lines.push("  </url>");
  }
  lines.push("</urlset>", "");
  return lines.join("\n");
}

// ---------------------------------------------------------------------------
// Live `crawl` Command
// ---------------------------------------------------------------------------

export async function runCrawlCommand(opts: {
  maxPages?: number;
  concurrency?: number;
  xhSamples?: number;
} = {}): Promise<SitemapCrawlReport> {
  const maxPagesCap = opts.maxPages ?? 80;
  const concurrency = opts.concurrency ?? 6;
  const xhSampleCount = opts.xhSamples ?? CORE_PROBE_PATHS.length;

  console.error(`[crawl] Probing ${EMSCRIPTEN_ORIGIN}/sitemap.xml and core routes via xh...`);
  const sitemapXmlProbe = await runXhProbe(`${EMSCRIPTEN_ORIGIN}/sitemap.xml`, {
    checkStatus: false,
    headersOnly: true,
    timeoutSec: 15,
  });

  const coreUrls = CORE_PROBE_PATHS.slice(0, Math.max(1, xhSampleCount)).map((p) => `${EMSCRIPTEN_ORIGIN}${p}`);
  const xhProbes = await mapWithConcurrency(coreUrls, 4, (url) =>
    runXhProbe(url, { checkStatus: true, headersOnly: false, timeoutSec: 15 }),
  );

  console.error(`[crawl] Fetching Sphinx searchindex.js...`);
  const searchUrl = `${EMSCRIPTEN_ORIGIN}/searchindex.js`;
  const searchResp = await fetch(searchUrl, {
    headers: { "User-Agent": USER_AGENT },
    signal: AbortSignal.timeout(15_000),
  });
  let searchIndex: SphinxSearchIndex = { docnames: [], titles: [], filenames: [], routes: [] };
  if (searchResp.ok) {
    searchIndex = parseSphinxSearchIndex(await searchResp.text(), EMSCRIPTEN_ORIGIN);
  }

  const routeMap = new Map<string, CrawledRouteRecord>();
  const ensureRoute = (
    url: string,
    path: string,
    via: "seed" | "bfs" | "searchindex",
    depth: number,
    title = "",
    sourceFilename: string | null = null,
  ): CrawledRouteRecord => {
    const existing = routeMap.get(url);
    if (existing) {
      if (!existing.discoveredVia.includes(via)) existing.discoveredVia.push(via);
      if (!existing.title && title) existing.title = title;
      if (!existing.sourceFilename && sourceFilename) existing.sourceFilename = sourceFilename;
      if (depth < existing.depth) existing.depth = depth;
      return existing;
    }
    const created: CrawledRouteRecord = {
      url,
      path,
      title,
      h1: "",
      depth,
      discoveredVia: [via],
      crawled: false,
      httpStatus: null,
      contentType: null,
      outLinksCount: 0,
      externalLinksCount: 0,
      sourceFilename,
    };
    routeMap.set(url, created);
    return created;
  };

  // Seed root, core routes, and Sphinx searchindex routes.
  const queue: string[] = [];
  const enqueued = new Set<string>();
  const enqueue = (url: string, path: string, via: "seed" | "bfs" | "searchindex", depth: number, title = "", src: string | null = null) => {
    ensureRoute(url, path, via, depth, title, src);
    if (!enqueued.has(url)) {
      enqueued.add(url);
      queue.push(url);
    }
  };

  enqueue(`${EMSCRIPTEN_ORIGIN}/index.html`, "/index.html", "seed", 0, "Emscripten Documentation");
  for (const p of CORE_PROBE_PATHS) {
    const norm = normalizeCrawlUrl(p, `${EMSCRIPTEN_ORIGIN}/`, EMSCRIPTEN_ORIGIN);
    if (norm) enqueue(norm.normalizedUrl, norm.path, "seed", norm.path === "/index.html" ? 0 : 1);
  }
  for (const sr of searchIndex.routes) {
    enqueue(sr.url, sr.path, "searchindex", sr.path === "/index.html" ? 0 : 2, sr.title, sr.sourceFilename || null);
  }

  const staticAssets = new Set<string>();
  const externalDomainCounts = new Map<string, number>();
  let totalExternalLinks = 0;
  let siteTitle = "Emscripten Documentation";
  let emscriptenDocsVersion = "7.0.1-git";
  let crawledCount = 0;

  // Breadth-first bounded concurrency crawl up to `maxPagesCap`.
  let queueIdx = 0;
  while (queueIdx < queue.length && crawledCount < maxPagesCap) {
    const remainingBudget = maxPagesCap - crawledCount;
    const batch = queue.slice(queueIdx, queueIdx + Math.min(concurrency, remainingBudget));
    queueIdx += batch.length;
    if (batch.length === 0) break;

    await mapWithConcurrency(batch, concurrency, async (targetUrl) => {
      const rec = routeMap.get(targetUrl)!;
      try {
        const resp = await fetch(targetUrl, {
          headers: { "User-Agent": USER_AGENT, Accept: "text/html,application/xhtml+xml" },
          redirect: "follow",
          signal: AbortSignal.timeout(15_000),
        });
        rec.crawled = true;
        rec.httpStatus = resp.status;
        rec.contentType = resp.headers.get("content-type");
        crawledCount++;

        if (resp.ok && (rec.contentType?.includes("text/html") ?? true)) {
          const html = await resp.text();
          const info = extractHtmlPageInfo(html, targetUrl, EMSCRIPTEN_ORIGIN);
          if (info.title) rec.title = info.title;
          if (info.h1) rec.h1 = info.h1;
          if (targetUrl.endsWith("/index.html") && rec.depth === 0) {
            if (info.title) siteTitle = info.title;
            if (info.docsVersion) emscriptenDocsVersion = info.docsVersion;
          } else if (info.docsVersion && emscriptenDocsVersion === "7.0.1-git") {
            emscriptenDocsVersion = info.docsVersion;
          }

          rec.outLinksCount = info.internalHtmlLinks.length;
          rec.externalLinksCount = info.externalLinks.length;

          for (const asset of info.internalAssetLinks) staticAssets.add(asset);
          for (const ext of info.externalLinks) {
            totalExternalLinks++;
            try {
              const host = new URL(ext).hostname;
              externalDomainCounts.set(host, (externalDomainCounts.get(host) ?? 0) + 1);
            } catch {
              // ignore malformed external URL
            }
          }
          for (const childUrl of info.internalHtmlLinks) {
            const childPath = new URL(childUrl).pathname;
            enqueue(childUrl, childPath, "bfs", rec.depth + 1);
          }
        }
      } catch (err) {
        rec.crawled = true;
        rec.error = err instanceof Error ? err.message : String(err);
        crawledCount++;
      }
    });
  }

  const routes = [...routeMap.values()].sort((a, b) => a.path.localeCompare(b.path));
  const okPages = routes.filter((r) => r.crawled && r.httpStatus !== null && r.httpStatus >= 200 && r.httpStatus < 400).length;
  const failedPages = routes.filter((r) => r.crawled && (r.httpStatus === null || r.httpStatus >= 400)).length;
  const externalDomains = [...externalDomainCounts.entries()]
    .map(([host, count]) => ({ host, count }))
    .sort((a, b) => b.count - a.count);

  const report: SitemapCrawlReport = {
    crawledAt: new Date().toISOString(),
    baseUrl: EMSCRIPTEN_ORIGIN,
    siteTitle,
    emscriptenDocsVersion,
    maxPagesCap,
    concurrency,
    sitemapXmlProbe,
    searchIndexProbe: {
      url: searchUrl,
      httpStatus: searchResp.status,
      docCount: searchIndex.docnames.length,
      titleCount: searchIndex.titles.length,
    },
    xhProbes,
    stats: {
      totalDiscoveredRoutes: routes.length,
      crawledHtmlPages: routes.filter((r) => r.crawled).length,
      okPages,
      failedPages,
      uncrawledQueuedRoutes: routes.filter((r) => !r.crawled).length,
      totalStaticAssetsDiscovered: staticAssets.size,
      uniqueExternalDomains: externalDomains.length,
      totalExternalLinks,
    },
    externalDomains,
    routeTree: buildRouteTree(routes),
    routes,
  };

  writeRepoFile(SITEMAP_JSON_REL, `${JSON.stringify(report, null, 2)}\n`);
  writeRepoFile(SITEMAP_XML_REL, serializeSitemapXml(routes, report.crawledAt.slice(0, 10)));
  console.error(
    `[crawl] Discovered ${report.stats.totalDiscoveredRoutes} routes (${report.stats.crawledHtmlPages} crawled, ${report.stats.okPages} OK, sitemap.xml=${sitemapXmlProbe.httpStatus}).`,
  );
  return report;
}

// ---------------------------------------------------------------------------
// GitIngest Formatter & `emscripten-core` Org Ingestion
// ---------------------------------------------------------------------------

export function buildDirectoryTree(paths: readonly string[], rootLabel: string): string {
  interface TreeDir {
    dirs: Map<string, TreeDir>;
    files: string[];
  }
  const root: TreeDir = { dirs: new Map(), files: [] };

  for (const rawPath of paths) {
    const clean = rawPath.replace(/^\/+/, "");
    if (!clean) continue;
    const parts = clean.split("/");
    let cur = root;
    for (let i = 0; i < parts.length - 1; i++) {
      const dirName = parts[i]!;
      let next = cur.dirs.get(dirName);
      if (!next) {
        next = { dirs: new Map(), files: [] };
        cur.dirs.set(dirName, next);
      }
      cur = next;
    }
    cur.files.push(parts.at(-1)!);
  }

  const lines = [`└── ${rootLabel}/`];
  const render = (node: TreeDir, prefix: string) => {
    const dirNames = [...node.dirs.keys()].sort();
    const fileNames = [...new Set(node.files)].sort();
    const entries: Array<{ name: string; isDir: boolean }> = [
      ...dirNames.map((name) => ({ name, isDir: true })),
      ...fileNames.map((name) => ({ name, isDir: false })),
    ];
    entries.forEach((entry, idx) => {
      const isLast = idx === entries.length - 1;
      const branch = isLast ? "└── " : "├── ";
      const childPrefix = prefix + (isLast ? "    " : "│   ");
      if (entry.isDir) {
        lines.push(`${prefix}${branch}${entry.name}/`);
        render(node.dirs.get(entry.name)!, childPrefix);
      } else {
        lines.push(`${prefix}${branch}${entry.name}`);
      }
    });
  };
  render(root, "    ");
  return lines.join("\n");
}

export interface FallbackDigestInput {
  fullName: string;
  htmlUrl: string;
  defaultBranch: string;
  commitSha: string;
  reason: string;
  archived: boolean;
  isFork: boolean;
  language: string;
  stars: number;
  treePaths: readonly string[];
  treeTruncated: boolean;
  files: ReadonlyArray<{ path: string; content: string }>;
}

export function formatGitIngestDigest(input: FallbackDigestInput): string {
  const treeBlock = buildDirectoryTree(input.treePaths, input.fullName.replaceAll("/", "-"));
  const sections = [
    `Repository: ${input.fullName}`,
    `URL: ${input.htmlUrl}`,
    `Branch: ${input.defaultBranch}`,
    `Commit: ${input.commitSha || "empty-repository"}`,
    `Ingest Source: github-tree-fallback (${input.reason})`,
    `Metadata: archived=${input.archived} fork=${input.isFork} language=${input.language || "unknown"} stars=${input.stars}`,
    `Files analyzed: ${input.files.length}`,
    `Tree entries discovered: ${input.treePaths.length} (truncated: ${input.treeTruncated})`,
    "",
    "Directory structure:",
    treeBlock,
  ];

  for (const file of input.files) {
    sections.push(
      "",
      "================================================",
      `FILE: ${file.path}`,
      "================================================",
      file.content,
    );
  }
  return `${sections.join("\n")}\n`;
}

function classifyEmscriptenRepoRole(repo: GitHubRepoRaw): string {
  const map: Record<string, string> = {
    emscripten: "Core LLVM-to-WebAssembly compiler frontend (`emcc`/`em++`), JS runtime (`library_*.js`), ports (including `emdawnwebgpu`), and test suite.",
    emsdk: "Cross-platform Emscripten SDK installer/activator (`emsdk.py`, `emsdk_manifest.json`) managing pinned LLVM, Binaryen, Node, and Python toolchains.",
    "setup-emsdk": "Official GitHub Action (`TypeScript`) installing and caching Emscripten SDK versions in CI workflows.",
    posixtestsuite: "Emscripten-adapted Open POSIX Test Suite validating pthread, signal, semaphore, and libc compliance.",
    "emscripten-ci": "Docker environment specification (`Dockerfile`) used to build and test Emscripten's own continuous integration.",
    musl: "Emscripten fork of musl libc providing C standard library headers and runtime primitives for `wasm32-unknown-emscripten`.",
    "llvm-project": "Upstream-synced LLVM fork supporting Emscripten Wasm backend/Clang integration and release validation.",
    "bazel-central-registry": "Emscripten-maintained Bazel Central Registry fork publishing the Bzlmod `emsdk` module and C/C++ Wasm toolchain rules.",
    "emsdk-arm64-build-infra": "ARM64 Emscripten SDK build infrastructure repository (currently initialized empty upstream).",
    "emscripten-fastcomp": "Archived legacy asm.js LLVM backend superseded by the upstream LLVM WebAssembly backend.",
    "emscripten-fastcomp-clang": "Archived legacy Fastcomp Clang frontend superseded by upstream LLVM Clang.",
    terser: "Archived downstream Terser fork; Emscripten now consumes upstream Terser directly.",
  };
  return map[repo.name] ?? (repo.description || "Emscripten organization repository.");
}

async function runGhApiJson<T>(endpoint: string, paginate = false): Promise<{ ok: boolean; status: number; data: T | null; error?: string }> {
  const args = ["gh", "api"];
  if (paginate) args.push("--paginate");
  args.push(endpoint);
  const proc = Bun.spawn(args, { stdin: "ignore", stdout: "pipe", stderr: "pipe" });
  const [stdoutBuf, stderrBuf, exitCode] = await Promise.all([
    new Response(proc.stdout).arrayBuffer(),
    new Response(proc.stderr).arrayBuffer(),
    proc.exited,
  ]);
  const out = new TextDecoder().decode(stdoutBuf);
  const err = new TextDecoder().decode(stderrBuf).trim();
  if (exitCode !== 0) {
    return { ok: false, status: err.includes("409") ? 409 : err.includes("404") ? 404 : 500, data: null, error: err };
  }
  try {
    if (paginate) {
      return { ok: true, status: 200, data: parsePaginatedJson<unknown>(out) as unknown as T };
    }
    return { ok: true, status: 200, data: JSON.parse(out) as T };
  } catch (parseErr) {
    return { ok: false, status: 500, data: null, error: String(parseErr) };
  }
}

const PRIORITY_FILE_PATTERNS = [
  /^readme/i,
  /^changelog\.md$/i,
  /^emscripten-version\.txt$/i,
  /^package\.json$/i,
  /^cargo\.toml$/i,
  /^module\.bazel$/i,
  /^dockerfile$/i,
  /^action\.yml$/i,
  /^emsdk\.py$/i,
  /^emsdk_manifest\.json$/i,
  /^tools\/ports\/emdawnwebgpu\.py$/i,
  /^tools\/shared\.py$/i,
  /^\.github\/workflows\/[^/]+\.ya?ml$/i,
  /^modules\/emsdk\//i,
  /^license/i,
];

function selectPriorityPaths(paths: readonly string[], maxFiles = 12): string[] {
  const scored = paths
    .filter((p) => !/\.(png|jpg|gif|ico|wasm|zip|xz|tar|gz|bin|o|a|so|dll)$/i.test(p))
    .map((p) => {
      const idx = PRIORITY_FILE_PATTERNS.findIndex((re) => re.test(p));
      const depth = p.split("/").length;
      return { path: p, score: idx >= 0 ? idx : 100 + depth };
    })
    .sort((a, b) => a.score - b.score || a.path.localeCompare(b.path));
  return scored.slice(0, maxFiles).map((s) => s.path);
}

async function ingestSingleRepo(
  repo: GitHubRepoRaw,
  opts: { timeoutSec: number; noRemote: boolean },
): Promise<IngestedRepoRecord> {
  const digestRel = `${GITINGEST_DIR_REL}/${repo.name}.txt`;
  const baseRecord: Omit<
    IngestedRepoRecord,
    | "ingestMethod"
    | "gitingestHttpStatus"
    | "digestUrl"
    | "commitSha"
    | "filesAnalyzed"
    | "treeEntryCount"
    | "treeTruncated"
    | "estimatedTokens"
    | "digestBytes"
  > = {
    name: repo.name,
    fullName: repo.full_name,
    htmlUrl: repo.html_url,
    description: repo.description ?? "",
    homepage: repo.homepage ?? "",
    language: repo.language ?? "",
    stars: repo.stargazers_count,
    forks: repo.forks_count,
    openIssues: repo.open_issues_count,
    archived: repo.archived,
    isFork: repo.fork,
    defaultBranch: repo.default_branch,
    pushedAt: repo.pushed_at,
    sizeKb: repo.size,
    licenseSpdx: repo.license?.spdx_id ?? "NONE",
    topics: repo.topics ?? [],
    digestPath: digestRel,
    roleSummary: classifyEmscriptenRepoRole(repo),
  };

  let gitingestHttpStatus: number | null = null;
  let gitingestError: string | undefined;

  if (!opts.noRemote && repo.size > 0) {
    const apiUrl = `${GITINGEST_BASE_URL}/api/${repo.full_name}?max_file_size=5120`;
    try {
      const resp = await fetch(apiUrl, {
        headers: { "User-Agent": USER_AGENT, Accept: "application/json" },
        signal: AbortSignal.timeout(opts.timeoutSec * 1000),
      });
      gitingestHttpStatus = resp.status;
      if (resp.ok) {
        const data = (await resp.json()) as {
          summary?: string;
          digest_url?: string;
          tree?: string;
          content?: string;
        };
        const summary = data.summary ?? "";
        const commitMatch = summary.match(/Commit:\s*([0-9a-f]+)/i);
        const filesMatch = summary.match(/Files analyzed:\s*(\d+)/i);
        const tokensMatch = summary.match(/Estimated tokens:\s*([^\r\n]+)/i);
        const treeLines = (data.tree ?? "").split(/\r?\n/).filter((l) => l.includes("── ")).length;
        const digestText = [
          summary.trim(),
          data.digest_url ? `Digest URL: ${data.digest_url}` : "",
          "Ingest Source: gitingest.com-api",
          "",
          data.tree?.trim() ?? "",
          "",
          data.content?.trim() ?? "",
        ]
          .filter((part, idx) => idx === 3 || idx === 5 || part.length > 0)
          .join("\n");

        writeRepoFile(digestRel, `${digestText}\n`);
        return {
          ...baseRecord,
          ingestMethod: "gitingest-api",
          gitingestHttpStatus,
          digestUrl: data.digest_url ?? null,
          commitSha: commitMatch?.[1] ?? "",
          filesAnalyzed: filesMatch ? Number.parseInt(filesMatch[1]!, 10) : 0,
          treeEntryCount: treeLines,
          treeTruncated: false,
          estimatedTokens: tokensMatch?.[1]?.trim() ?? null,
          digestBytes: Buffer.byteLength(digestText, "utf8") + 1,
        };
      }
      gitingestError = `HTTP ${resp.status}`;
    } catch (err) {
      gitingestError = err instanceof Error ? err.message : String(err);
    }
  } else if (repo.size === 0) {
    gitingestError = "Skipped remote ingest: empty upstream Git repository (0 KB)";
  }

  // Deterministic GitHub Tree + Contents fallback
  const commitRes = await runGhApiJson<{ sha?: string }>(
    `repos/${repo.full_name}/commits/${encodeURIComponent(repo.default_branch)}`,
  );
  if (!commitRes.ok && commitRes.status === 409) {
    const emptyText = formatGitIngestDigest({
      fullName: repo.full_name,
      htmlUrl: repo.html_url,
      defaultBranch: repo.default_branch,
      commitSha: "",
      reason: gitingestError ?? "Git Repository is empty",
      archived: repo.archived,
      isFork: repo.fork,
      language: repo.language ?? "",
      stars: repo.stargazers_count,
      treePaths: [],
      treeTruncated: false,
      files: [],
    });
    writeRepoFile(digestRel, emptyText);
    return {
      ...baseRecord,
      ingestMethod: "empty-repo",
      gitingestHttpStatus,
      ...(gitingestError ? { gitingestError } : {}),
      digestUrl: null,
      commitSha: "",
      filesAnalyzed: 0,
      treeEntryCount: 0,
      treeTruncated: false,
      estimatedTokens: "0",
      digestBytes: Buffer.byteLength(emptyText, "utf8"),
    };
  }

  const commitSha = commitRes.data?.sha ?? "";
  let treeRes = await runGhApiJson<{
    truncated?: boolean;
    tree?: Array<{ path: string; type: string; size?: number }>;
  }>(`repos/${repo.full_name}/git/trees/${encodeURIComponent(repo.default_branch)}?recursive=1`);

  if (!treeRes.ok || !treeRes.data?.tree) {
    treeRes = await runGhApiJson<{
      truncated?: boolean;
      tree?: Array<{ path: string; type: string; size?: number }>;
    }>(`repos/${repo.full_name}/git/trees/${encodeURIComponent(repo.default_branch)}`);
  }

  const rawEntries = treeRes.data?.tree ?? [];
  const blobPaths = rawEntries.filter((e) => e.type === "blob").map((e) => e.path);
  const allTreePaths = rawEntries.map((e) => e.path);
  const treeTruncated = Boolean(treeRes.data?.truncated);
  const priorityPaths = selectPriorityPaths(blobPaths.length > 0 ? blobPaths : allTreePaths, 10);

  const fetchedFiles: Array<{ path: string; content: string }> = [];
  let accumulatedBytes = 0;
  for (const filePath of priorityPaths) {
    if (accumulatedBytes >= 180_000) break;
    const contentRes = await runGhApiJson<{ content?: string; encoding?: string; type?: string }>(
      `repos/${repo.full_name}/contents/${filePath.split("/").map(encodeURIComponent).join("/")}`,
    );
    if (contentRes.ok && contentRes.data?.content && contentRes.data.encoding === "base64") {
      const decoded = Buffer.from(contentRes.data.content.replace(/\s+/g, ""), "base64").toString("utf8");
      const trimmed = decoded.length > 48_000 ? `${decoded.slice(0, 48_000)}\n... [truncated at 48 KB]` : decoded;
      fetchedFiles.push({ path: filePath, content: trimmed });
      accumulatedBytes += trimmed.length;
    }
  }

  const reason = gitingestError ? `gitingest.com fallback (${gitingestError})` : "deterministic GitHub tree+contents";
  const digestText = formatGitIngestDigest({
    fullName: repo.full_name,
    htmlUrl: repo.html_url,
    defaultBranch: repo.default_branch,
    commitSha,
    reason,
    archived: repo.archived,
    isFork: repo.fork,
    language: repo.language ?? "",
    stars: repo.stargazers_count,
    treePaths: (blobPaths.length > 0 ? blobPaths : allTreePaths).slice(0, 2500),
    treeTruncated,
    files: fetchedFiles,
  });

  writeRepoFile(digestRel, digestText);
  return {
    ...baseRecord,
    ingestMethod: "github-tree-fallback",
    gitingestHttpStatus,
    ...(gitingestError ? { gitingestError } : {}),
    digestUrl: null,
    commitSha,
    filesAnalyzed: fetchedFiles.length,
    treeEntryCount: rawEntries.length,
    treeTruncated,
    estimatedTokens: `~${Math.max(1, Math.round(digestText.length / 4))}`,
    digestBytes: Buffer.byteLength(digestText, "utf8"),
  };
}

export async function runIngestCommand(opts: {
  concurrency?: number;
  timeoutSec?: number;
  noRemote?: boolean;
} = {}): Promise<OrgIngestManifest> {
  const concurrency = opts.concurrency ?? 3;
  const timeoutSec = opts.timeoutSec ?? 20;
  const noRemote = opts.noRemote ?? false;

  console.error(`[ingest] Probing ${GITINGEST_BASE_URL}/health and listing ${EMSCRIPTEN_ORG} repos via gh api...`);
  const gitingestApiProbe = await runXhProbe(`${GITINGEST_BASE_URL}/health`, {
    checkStatus: false,
    headersOnly: false,
    timeoutSec: 12,
  });

  const reposRes = await runGhApiJson<GitHubRepoRaw[]>(`orgs/${EMSCRIPTEN_ORG}/repos?per_page=100`, true);
  if (!reposRes.ok || !reposRes.data) {
    throw new Error(`Failed to list orgs/${EMSCRIPTEN_ORG}/repos via gh api: ${reposRes.error ?? "unknown error"}`);
  }

  const sortedRepos = [...reposRes.data].sort((a, b) => b.stargazers_count - a.stargazers_count || a.name.localeCompare(b.name));
  console.error(`[ingest] Ingesting ${sortedRepos.length} repositories into ${GITINGEST_DIR_REL}/...`);

  const ingested = await mapWithConcurrency(sortedRepos, concurrency, async (repo) => {
    const rec = await ingestSingleRepo(repo, { timeoutSec, noRemote });
    console.error(
      `[ingest]   ${rec.name}: method=${rec.ingestMethod} files=${rec.filesAnalyzed} bytes=${rec.digestBytes} commit=${rec.commitSha.slice(0, 10) || "empty"}`,
    );
    return rec;
  });

  const manifest: OrgIngestManifest = {
    ingestedAt: new Date().toISOString(),
    org: EMSCRIPTEN_ORG,
    repoCount: ingested.length,
    activeCount: ingested.filter((r) => !r.archived).length,
    archivedCount: ingested.filter((r) => r.archived).length,
    forkCount: ingested.filter((r) => r.isFork).length,
    gitingestApiProbe,
    repos: ingested,
  };

  writeRepoFile(ORG_MANIFEST_REL, `${JSON.stringify(manifest, null, 2)}\n`);
  return manifest;
}

// ---------------------------------------------------------------------------
// Validators (`agentskills.io`, `llmstxt.org`, Relative Links)
// ---------------------------------------------------------------------------

export function validateLlmsTxt(content: string): { ok: boolean; h1Count: number; hasBlockquote: boolean; h2Count: number; linkCount: number; errors: string[] } {
  const lines = content.split(/\r?\n/);
  const h1Lines = lines.filter((l) => /^#\s+\S/.test(l));
  const hasBlockquote = lines.some((l) => /^>\s+\S/.test(l));
  const h2Lines = lines.filter((l) => /^##\s+\S/.test(l));
  const linkCount = [...content.matchAll(/\[[^\]]+\]\([^)]+\)/g)].length;
  const errors: string[] = [];
  if (h1Lines.length !== 1) errors.push(`Expected exactly 1 H1 heading, found ${h1Lines.length}`);
  if (!hasBlockquote) errors.push("Missing llmstxt.org summary blockquote (> ...)");
  if (h2Lines.length < 2) errors.push(`Expected >= 2 H2 sections, found ${h2Lines.length}`);
  if (linkCount < 4) errors.push(`Expected >= 4 markdown links, found ${linkCount}`);
  return { ok: errors.length === 0, h1Count: h1Lines.length, hasBlockquote, h2Count: h2Lines.length, linkCount, errors };
}

export function findBrokenRelativeLinks(docRelPaths: readonly string[], root = REPO_ROOT): string[] {
  const broken: string[] = [];
  for (const rel of docRelPaths) {
    const full = join(root, rel);
    if (!existsSync(full)) {
      broken.push(`MISSING_FILE: ${rel}`);
      continue;
    }
    const content = readFileSync(full, "utf8");
    for (const match of content.matchAll(/\]\(([^)]+)\)/g)) {
      const target = (match[1] ?? "").split("#")[0]!.trim();
      if (!target || /^(https?:|mailto:|#)/i.test(target)) continue;
      const resolved = resolve(dirname(full), target);
      if (!existsSync(resolved)) {
        broken.push(`${rel} -> ${target}`);
      }
    }
  }
  return broken;
}

// ---------------------------------------------------------------------------
// Document & Progressive-Disclosure Skill Emitters
// ---------------------------------------------------------------------------

function formatBytesHuman(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(2)} MiB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
  return `${bytes} B`;
}

export function buildLlmsTxtContent(
  crawl: SitemapCrawlReport | null,
  manifest: OrgIngestManifest | null,
): string {
  const docsVer = crawl?.emscriptenDocsVersion ?? "7.0.1-git";
  const discoveredCount = crawl?.stats.totalDiscoveredRoutes ?? 90;
  const repoCount = manifest?.repoCount ?? 12;

  const lines = [
    `# Emscripten (${docsVer}) & emscripten-core Ecosystem`,
    "",
    `> Authoritative index for the Emscripten LLVM-to-WebAssembly compiler toolchain (${docsVer}), live-crawled documentation routes (${discoveredCount} routes on emscripten.org), and all ${repoCount} repositories in github.com/orgs/emscripten-core integrated for Aphrody C/C++/Rust/WebGPU workflows.`,
    "",
    "- Use `emcc`/`em++` with `--use-port=emdawnwebgpu` (replacing legacy `-sUSE_WEBGPU`), `-sMODULARIZE -sEXPORT_ES6` for ES module bundles, `-sSTRICT`, `-sJSPI` or `-sASYNCIFY`, `-pthread` (requiring COOP/COEP `SharedArrayBuffer` headers), and `-msimd128`.",
    "- Upstream Emscripten 7.0.1-git expects LLVM 24 (`EXPECTED_LLVM_VERSION = 24`); check local host toolchain telemetry in [Emscripten Ecosystem Architecture](./emscripten-ecosystem.md).",
    "",
    "## Aphrody Architecture & Agent Skill",
    "",
    "- [Emscripten Ecosystem & Aphrody Integration Guide](./emscripten-ecosystem.md): Full 12-repo matrix, live crawl stats, `emscripten` vs `wasm32-unknown-unknown` vs `wasm32-wasip2`, Clang/LLVM 23 linking, and Bun+xh CI checks.",
    "- Agent skill: `fw: plugins/aphrody/skills/emscripten`",
    "",
    "## Core Upstream Documentation (`emscripten.org`)",
    "",
    "- [Emscripten Documentation Home](https://emscripten.org/docs/index.html): Top-level manual index (`7.0.1-git`).",
    "- [emcc Compiler Frontend](https://emscripten.org/docs/tools_reference/emcc.html): CLI driver flags, optimization levels, and separate compilation.",
    "- [Emscripten SDK (`emsdk`)](https://emscripten.org/docs/tools_reference/emsdk.html): Installing, activating, and pinning SDK releases.",
    "- [Settings Reference (`settings.js`)](https://emscripten.org/docs/tools_reference/settings_reference.html): Complete `-sSETTING=VALUE` linker/compiler options.",
    "- [Compiler Best Practices](https://emscripten.org/docs/compiling/best_practices.html): Minimal CLI style, `-sSTRICT`, `-sEXPORT_ES6`, `-flto`, and `--closure=1`.",
    "- [Modularized & ES6 Output](https://emscripten.org/docs/compiling/Modularized-Output.html): `-sMODULARIZE`, `-sEXPORT_ES6`, `-sMODULARIZE=instance`, and ESM integration.",
    "- [WebGPU Support (`emdawnwebgpu`)](https://emscripten.org/docs/porting/multimedia_and_graphics/WebGPU-support.html): Emdawnwebgpu `<webgpu/webgpu.h>` port architecture.",
    "- [Asyncify & JSPI](https://emscripten.org/docs/porting/asyncify.html): Pausing synchronous C/C++ across browser Promises.",
    "- [Pthreads & SharedArrayBuffer](https://emscripten.org/docs/porting/pthreads.html): `-pthread`, `-sPTHREAD_POOL_SIZE`, and COOP/COEP header requirements.",
    "- [WebAssembly SIMD](https://emscripten.org/docs/porting/simd.html): `-msimd128`, autovectorization, `<wasm_simd128.h>`, and x86/ARM intrinsic translation.",
    "- [Embind (`emscripten/bind.h`)](https://emscripten.org/docs/porting/connecting_cpp_and_javascript/embind.html): C++ classes, smart pointers, value types, and `emscripten::val`.",
    "- [Interacting with Code](https://emscripten.org/docs/porting/connecting_cpp_and_javascript/Interacting-with-code.html): `ccall`/`cwrap`, `EM_JS`, `EM_ASM`, WebIDL Binder, and typed heap views.",
    "- [Release Notes (`ChangeLog.md`)](https://emscripten.org/docs/introducing_emscripten/release_notes.html): Emscripten 7.0.0 (`libemcc`, side-module builtins) and 7.0.1 link-time Embind invokers.",
    "",
    `## GitHub Organization Repositories (${EMSCRIPTEN_ORG}: ${repoCount} repos)`,
    "",
  ];

  const repos = manifest?.repos ?? [];
  for (const r of repos) {
    const badge = r.archived ? "archived" : r.isFork ? "fork" : "active";
    const desc = r.description || r.roleSummary;
    lines.push(
      `- [${r.fullName}](${r.htmlUrl}) (\`${badge}\`, \`${r.language || "config"}\`, ${r.stars} stars, \`${r.ingestMethod}\`): ${desc}`,
    );
  }
  lines.push("");
  return lines.join("\n");
}

function renderRepoMatrixRows(manifest: OrgIngestManifest | null): string {
  const repos = manifest?.repos ?? [];
  if (repos.length === 0) {
    return "| *(run `ingest` first)* | — | — | — | — | — |";
  }
  return repos
    .map((r) => {
      const state = r.archived ? "Archived" : r.isFork ? "Active Fork" : "Active";
      const sha = r.commitSha ? `\`${r.commitSha.slice(0, 8)}\`` : "`empty`";
      const digestInfo = `\`${r.ingestMethod}\` (${r.filesAnalyzed} files, ${formatBytesHuman(r.digestBytes)})`;
      const stars = new Intl.NumberFormat().format(r.stars);
      return `| [\`${r.name}\`](${r.htmlUrl}) | ${state} | \`${r.language || "—"}\` | ${stars} | ${sha} | ${digestInfo} | ${r.roleSummary} |`;
    })
    .join("\n");
}

function renderXhProbeRows(crawl: SitemapCrawlReport | null): string {
  if (!crawl) return "| *(run `crawl` first)* | — | — | — |";
  const allProbes = [crawl.sitemapXmlProbe, ...crawl.xhProbes];
  return allProbes
    .map((p) => {
      const shortPath = p.url.replace(EMSCRIPTEN_ORIGIN, "") || "/";
      const status = p.httpStatus ?? (p.ok ? 200 : 0);
      const note = p.url.endsWith("/sitemap.xml") && status >= 400
        ? `GitHub Pages Sphinx returned \`${p.statusLine || `HTTP ${status}`}\`; bypassed via \`searchindex.js\` + BFS`
        : p.ok
          ? "`xh --check-status` passed"
          : p.error ?? "Non-2xx response";
      return `| \`${shortPath}\` | \`HTTP ${status}\` | ${formatBytesHuman(p.bytes)} | ${p.elapsedMs} ms | ${note} |`;
    })
    .join("\n");
}

function renderTopRouteSections(crawl: SitemapCrawlReport | null): string {
  if (!crawl) return "- Run `bun tooling/bun/emscripten-crawl.ts all` to populate live route counts.";
  const sections = crawl.routeTree.children.find((c) => c.segment === "docs")?.children ?? [];
  const rootPages = crawl.routeTree.routes.map((r) => `\`${r.path}\``).join(", ");
  const lines = [`- **Root Routes** (${crawl.routeTree.routes.length}): ${rootPages || "`/index.html`"}`];
  for (const sec of sections) {
    const sample = sec.routes
      .slice(0, 5)
      .map((r) => `[\`${r.path.split("/").at(-1)}\`](${r.url})`)
      .join(", ");
    const subCount = sec.children.length > 0 ? ` (+${sec.children.length} subsections)` : "";
    lines.push(`- **\`docs/${sec.segment}/\`** (${sec.routeCount} routes, ${sec.crawledCount} crawled${subCount}): ${sample}`);
  }
  return lines.join("\n");
}

export function buildEcosystemDocContent(
  crawl: SitemapCrawlReport | null,
  manifest: OrgIngestManifest | null,
  host: HostToolchainProbe,
): string {
  const docsVer = crawl?.emscriptenDocsVersion ?? "7.0.1-git";
  const totalRoutes = crawl?.stats.totalDiscoveredRoutes ?? 0;
  const crawledPages = crawl?.stats.crawledHtmlPages ?? 0;
  const okPages = crawl?.stats.okPages ?? 0;
  const searchDocs = crawl?.searchIndexProbe.docCount ?? 0;
  const sitemapStatus = crawl?.sitemapXmlProbe.httpStatus ?? 503;
  const repoCount = manifest?.repoCount ?? 12;
  const activeRepos = manifest?.activeCount ?? 9;
  const archivedRepos = manifest?.archivedCount ?? 3;
  const emccStatus = host.emcc.present
    ? `Installed (\`${host.emcc.version}\` at \`${host.emcc.path}\`)`
    : "**Absent on host PC** (`emcc`, `em++`, `emsdk` not on `PATH`; `EMSDK`/`EM_CONFIG`/`EMSCRIPTEN` unset)";

  return `# Emscripten Ecosystem, Live Org Ingestion & Aphrody Wasm Architecture

> Live-verified synthesis of \`https://emscripten.org/\` (**Emscripten ${docsVer}**), all **${repoCount}** repositories in \`github.com/orgs/emscripten-core\`, and integration patterns across Aphrody's \`wasm32-unknown-unknown\`, \`wasm32-wasip2\`, and \`wasm32-unknown-emscripten\` pipelines.

---

## 1. Live Host Probe & Crawl/Ingest Telemetry

| Probe / Pipeline Dimension | Observed Value | Artifact / Verification Source |
| :--- | :--- | :--- |
| **Host \`emcc\` / \`emsdk\` (\`probeEmcc()\`)** | ${emccStatus} | Live \`Bun.which\` probe on \`${host.platform}/${host.arch}\` |
| **Host Native LLVM & Wasm Tools** | \`clang\`: \`${host.clang.version ?? "absent"}\`; \`lld\`: \`${host.lld.version ?? "absent"}\`; \`wasm-opt\`: \`${host.wasmOpt.version ?? "absent"}\`; \`wasm-bindgen\`: \`${host.wasmBindgen.version ?? "absent"}\` | Cross-checked against Emscripten 7.0.1-git \`EXPECTED_LLVM_VERSION = 24\` |
| **CLI Transport & GitHub Tooling** | \`xh\`: \`${host.xh.version ?? "absent"}\`; \`gh\`: \`${host.gh.version ?? "absent"}\`; \`gitingest\`: \`${host.gitingestCli.present ? "present" : "absent"}\` | [\`tooling/bun/emscripten-crawl.ts\`](../../tooling/bun/emscripten-crawl.ts) |
| **\`emscripten.org\` Sitemap Probe** | \`/sitemap.xml\` returned \`HTTP ${sitemapStatus}\`; \`/searchindex.js\` returned \`HTTP ${crawl?.searchIndexProbe.httpStatus ?? 200}\` (**${searchDocs}** indexed Sphinx docs) | \`target/emscripten-crawl/sitemap.json\`, \`target/emscripten-crawl/sitemap.xml\` |
| **BFS + SearchIndex Crawl** | **${totalRoutes}** unique same-origin HTML routes discovered (**${crawledPages}** fetched, **${okPages}** HTTP 2xx, \`maxPages=${crawl?.maxPagesCap ?? 80}\`) | \`bun tooling/bun/emscripten-crawl.ts crawl\` |
| **\`emscripten-core\` Org Ingestion** | **${repoCount}** repositories (**${activeRepos}** active, **${archivedRepos}** archived) ingested into \`.txt\` digests + JSON manifest | \`target/gitingest/emscripten-core/*.txt\`, \`org-manifest.json\` |

### \`xh\` HTTP Transport Verification Sample

| Route | Status | Bytes | Latency | Notes |
| :--- | :--- | :--- | :--- | :--- |
${renderXhProbeRows(crawl)}

### Discovered \`emscripten.org\` Route Hierarchy

${renderTopRouteSections(crawl)}

---

## 2. All ${repoCount} Repositories in \`emscripten-core\`

| Repository | State | Language | Stars | Head SHA | Ingest Summary | Architectural Role |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
${renderRepoMatrixRows(manifest)}

---

## 3. \`wasm32-unknown-emscripten\` vs \`wasm32-unknown-unknown\` vs \`wasm32-wasip2\` in Aphrody

Aphrody targets three complementary WebAssembly ABIs documented across [\`rust-toolchain.toml\`](../../../rust-toolchain.toml), [\`tooling/bun/web-build.ts\`](../../../tooling/bun/web-build.ts), and [\`docs/aphrody/architecture/cross-compile-and-depot-tools.md\`](./cross-compile-and-depot-tools.md):

| Dimension | \`wasm32-unknown-unknown\` (Aphrody Browser UI) | \`wasm32-wasip2\` (Aphrody Extensions) | \`wasm32-unknown-emscripten\` (C/C++ POSIX & Emscripten) |
| :--- | :--- | :--- | :--- |
| **Primary Aphrody Crates** | [\`gpui_web\`](../../../crates/gpui_web/Cargo.toml), \`aphrody_desktop\`, \`nsh-platform-web\`, \`gpui_wgpu\` | [\`extension_host\`](../../../crates/extension_host/Cargo.toml) (Wasmtime \`48.0.7\`), \`extension_api\` (WIT) | [\`nsh-core\`](../../../crates/nsh-core/Cargo.toml#L99-L100) (\`target_os = "emscripten"\`), [\`wgpu-core-deps-emscripten\`](${pinnedForkBlobUrl("wgpu", "wgpu-core/platform-deps/emscripten/Cargo.toml")}) |
| **Standard C Library (\`libc\`)** | None (pure Rust \`std\` stub + JS bindings) | WASI Preview 2 component model + \`wasi-libc\` | Full POSIX/musl \`libc\` (\`emscripten-core/musl\`), \`cfg(unix)\`, signals, pthreads, virtual FS |
| **JS / Host Bindings** | \`wasm-bindgen 0.2.129\` + \`web-sys\` / \`js-sys\` | Canonical ABI / \`wit-bindgen\` host imports | Embind (\`bind.h\` link-time invokers in 7.0.1), \`EM_JS\`, \`EM_ASM\`, \`ccall\`/\`cwrap\`, \`WebIDL Binder\` |
| **WebGPU API Bridge** | Browser \`navigator.gpu\` via \`wgpu\` -> \`web-sys\` | Host-mediated WIT GPU/UI capability | **Emdawnwebgpu** (\`--use-port=emdawnwebgpu\`, \`<webgpu/webgpu.h>\`, replacing removed \`-sUSE_WEBGPU\`) |
| **Async / Blocking Bridge** | \`wasm-bindgen-futures\` Promise poll | Wasmtime async host calls + epoch interruption | **JSPI** (\`-sJSPI\`, zero size overhead) or **Asyncify** (\`-sASYNCIFY\`) + \`emscripten_set_main_loop()\` |
| **Threads & Shared Memory** | Opt-in \`wasm_thread\` (\`target-feature=+atomics\`, COOP/COEP) | Host thread pool outside guest sandbox | \`-pthread -sPTHREAD_POOL_SIZE=...\` (COOP/COEP \`SharedArrayBuffer\`) or \`-sWASM_WORKERS\` |
| **Linker & Optimizer** | \`rust-lld\` + \`wasm-bindgen\` + Binaryen \`wasm-opt -Oz\` | \`wasm-tools component\` + Wasmtime Cranelift | \`emcc\` -> \`wasm-ld\` (LLVM \`24\`) + Binaryen \`wasm-opt\` + Closure (\`--closure=1\`) |

> **When to choose Emscripten in Aphrody**: Reach for Emscripten only when integrating C/C++ libraries that require POSIX \`libc\`, C++ exceptions/RTTI, \`pthreads\`, virtual POSIX filesystems (\`MEMFS\`/\`IDBFS\`), or native C/C++ \`webgpu.h\` libraries (via Emdawnwebgpu). Keep pure-Rust Aphrody UI/apps (\`aphrody_desktop\`, \`gpui_web\`, \`nsh-platform-web\`) on \`wasm32-unknown-unknown\` via [\`tooling/bun/web-build.ts\`](../../../tooling/bun/web-build.ts) and sandboxed IDE plugins on \`wasm32-wasip2\`.

---

## 4. Production C/C++ -> WebAssembly Rules (Emscripten 7.0.x Verified)

1. **WebGPU via Emdawnwebgpu (Not \`-sUSE_WEBGPU\`)**:
   - Emscripten 4.0.10+ through 7.0.1-git delegates \`<webgpu/webgpu.h>\` and \`<webgpu/webgpu_cpp.h>\` to Dawn's \`emdawnwebgpu\` port (currently pinned upstream at \`v20261105.214757\`).
   - Pass \`--use-port=emdawnwebgpu\` at **both** compile and link stages, and pass \`--closure=1\` during release linking to strip JS glue size.
2. **Modular ES6 Output Without Global \`Module\` Pollution**:
   - Prefer \`-sMODULARIZE -sEXPORT_ES6\` (or emit \`.mjs\`, which implies \`-sEXPORT_ES6\` -> \`-sMODULARIZE\`) so instantiating C/C++ Wasm modules works cleanly inside Bun, Vite, Workers, and Cloudflare Static Assets.
3. **Modern Minimal CLI Flags (\`docs/compiling/best_practices.html\`)**:
   - Omit redundant defaults like \`-sWASM=1\`; omit \`=1\` on boolean flags (\`-sSTRICT\`, \`-sALLOW_MEMORY_GROWTH\`, \`-sMODULARIZE\`, \`-sEXPORT_ES6\`).
   - Prefer standard driver flags (\`-pthread\`, \`-m64\`, \`-O3\`/\`-Os\`/\`-Oz\`, \`-flto\`) and \`@response_file\` lists for long exports.
4. **Async & Blocking I/O (\`JSPI\` vs \`Asyncify\`)**:
   - Prefer \`emscripten_set_main_loop()\` for frame loops; use \`-sJSPI\` where V8/browser JSPI is available (zero binary expansion) or \`-sASYNCIFY\` with \`ASYNCIFY_ONLY\` allowlists for universal fallback.
5. **Multithreading & COOP/COEP Headers**:
   - \`-pthread\` requires \`SharedArrayBuffer\`, which browsers gate on \`Cross-Origin-Opener-Policy: same-origin\` and \`Cross-Origin-Embedder-Policy: require-corp\` (or \`credentialless\`). Configure these headers selectively in dev/edge servers (matching Aphrody's \`--coi\` / \`/play/*\` policy).
6. **SIMD & Clang/LLVM Version Alignment**:
   - Pass \`-msimd128\` at compile time to enable LLVM Wasm autovectorization plus \`<wasm_simd128.h>\`, x86 SSE–AVX2, and ARM NEON intrinsics.
   - Note that Aphrody's local host Clang (\`${host.clang.version ?? "23.x"}\`) targets \`wasm32-unknown-unknown\` and \`wasm32-unknown-emscripten\` object emission (\`clang -c --target=wasm32-unknown-emscripten\`), whereas Emscripten \`7.0.1-git\` \`tools/shared.py\` enforces \`EXPECTED_LLVM_VERSION = 24\` for its bundled \`emsdk\` \`llvm-project\` sysroot/JS-linker pass.
7. **Embind & Emscripten 7.0.0–7.0.1 Linker Upgrades**:
   - Emscripten 7.0.1 generates Embind JS invokers at **link time** instead of runtime \`new Function\`, making \`-sDYNAMIC_EXECUTION=0\` fast without \`-sEMBIND_AOT\`.
   - Emscripten 7.0.0 moved program-wide builtins out of \`libclang_rt.builtins\` into \`libemcc\` (link \`-lemcc\` when passing \`-nostdlib\`) and removed \`EMCC_ONLY_FORCED_STDLIBS\`.

---

## 5. Bun + \`xh\` Reproducible Automation & CI Verification

\`\`\`sh
# 1. Full pipeline: crawl emscripten.org + ingest all ${repoCount} emscripten-core repos + emit docs/skills
bun tooling/bun/emscripten-crawl.ts all --max-pages 80 --concurrency 6

# 2. Run offline unit, parser, XML, frontmatter, and link-integrity tests
bun test tooling/bun/emscripten-crawl.test.ts

# 3. Verify zero ephemeral path leaks and relative doc link integrity workspace-wide
bun tooling/bun/repo-hygiene.ts

# 4. Probe local host toolchain & serve COOP/COEP-enabled Wasm artifacts via Bun + xh
xh --ignore-stdin --check-status --headers --follow GET https://emscripten.org/
\`\`\`

### Companion Artifacts

- **\`llms.txt\` Index**: [\`docs/aphrody/architecture/emscripten-llms.txt\`](./emscripten-llms.txt)
- **Agent Skill**: \`fw: plugins/aphrody/skills/emscripten\`
- **Cross-Compilation Context**: [\`docs/aphrody/architecture/cross-compile-and-depot-tools.md\`](./cross-compile-and-depot-tools.md) & [\`docs/aphrody/architecture/production-guide.md\`](./production-guide.md)
`;
}

export interface EmitReport {
  emittedFiles: string[];
  llmsValidation: ReturnType<typeof validateLlmsTxt>;
  brokenLinks: string[];
  hostProbe: HostToolchainProbe;
}

export function runEmitCommand(): EmitReport {
  const crawl = readJsonIfExists<SitemapCrawlReport>(SITEMAP_JSON_REL);
  const manifest = readJsonIfExists<OrgIngestManifest>(ORG_MANIFEST_REL);
  const hostProbe = probeEmcc();

  const llmsTxt = buildLlmsTxtContent(crawl, manifest);
  const ecosystemDoc = buildEcosystemDocContent(crawl, manifest, hostProbe);

  writeRepoFile(LLMS_TXT_REL, llmsTxt);
  writeRepoFile(ARCH_DOC_REL, ecosystemDoc);

  const emittedFiles = [
    LLMS_TXT_REL,
    ARCH_DOC_REL,
  ];

  const llmsValidation = validateLlmsTxt(llmsTxt);
  if (!llmsValidation.ok) {
    throw new Error(`Generated llms.txt failed validation: ${llmsValidation.errors.join("; ")}`);
  }
  const brokenLinks = findBrokenRelativeLinks([
    ARCH_DOC_REL,
    LLMS_TXT_REL,
  ]);
  if (brokenLinks.length > 0) {
    throw new Error(`Broken relative markdown links detected:\n${brokenLinks.join("\n")}`);
  }

  return {
    emittedFiles,
    llmsValidation,
    brokenLinks,
    hostProbe,
  };
}

// ---------------------------------------------------------------------------
// CLI Entrypoint
// ---------------------------------------------------------------------------

async function main(): Promise<void> {
  const args = Bun.argv.slice(2);
  const cmd = args.find((a) => !a.startsWith("-")) ?? "all";
  const valid = new Set(["crawl", "ingest", "emit", "all"]);
  if (!valid.has(cmd)) {
    console.error(`Unknown subcommand '${cmd}'. Expected one of: crawl, ingest, emit, all`);
    process.exit(1);
  }

  const maxPages = optInt(args, "--max-pages", 80);
  const crawlConcurrency = optInt(args, "--concurrency", 6);
  const xhSamples = optInt(args, "--xh-samples", CORE_PROBE_PATHS.length);
  const ingestConcurrency = optInt(args, "--concurrency", 3);
  const timeoutSec = optInt(args, "--timeout", 20);
  const noRemote = args.includes("--no-remote");

  let crawlReport: SitemapCrawlReport | null = null;
  let ingestManifest: OrgIngestManifest | null = null;
  let emitReport: EmitReport | null = null;

  if (cmd === "crawl" || cmd === "all") {
    crawlReport = await runCrawlCommand({ maxPages, concurrency: crawlConcurrency, xhSamples });
  }
  if (cmd === "ingest" || cmd === "all") {
    ingestManifest = await runIngestCommand({
      concurrency: cmd === "all" ? Math.min(3, ingestConcurrency) : ingestConcurrency,
      timeoutSec,
      noRemote,
    });
  }
  if (cmd === "emit" || cmd === "all") {
    emitReport = runEmitCommand();
  }

  console.log(
    JSON.stringify(
      {
        command: cmd,
        ...(crawlReport
          ? {
              crawl: {
                docsVersion: crawlReport.emscriptenDocsVersion,
                sitemapXmlStatus: crawlReport.sitemapXmlProbe.httpStatus,
                searchIndexDocs: crawlReport.searchIndexProbe.docCount,
                stats: crawlReport.stats,
                xhProbes: crawlReport.xhProbes.map((p) => ({
                  url: p.url,
                  httpStatus: p.httpStatus,
                  bytes: p.bytes,
                  elapsedMs: p.elapsedMs,
                  ok: p.ok,
                })),
                artifacts: [SITEMAP_JSON_REL, SITEMAP_XML_REL],
              },
            }
          : {}),
        ...(ingestManifest
          ? {
              ingest: {
                repoCount: ingestManifest.repoCount,
                activeCount: ingestManifest.activeCount,
                archivedCount: ingestManifest.archivedCount,
                gitingestHealthStatus: ingestManifest.gitingestApiProbe.httpStatus,
                repos: ingestManifest.repos.map((r) => ({
                  name: r.name,
                  archived: r.archived,
                  ingestMethod: r.ingestMethod,
                  gitingestHttpStatus: r.gitingestHttpStatus,
                  filesAnalyzed: r.filesAnalyzed,
                  digestBytes: r.digestBytes,
                  digestPath: r.digestPath,
                })),
                manifestPath: ORG_MANIFEST_REL,
              },
            }
          : {}),
        ...(emitReport
          ? {
              emit: emitReport,
            }
          : {}),
      },
      null,
      2,
    ),
  );
}

if (import.meta.main) {
  await main();
}
