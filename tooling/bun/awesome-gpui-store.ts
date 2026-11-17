#!/usr/bin/env bun
/**
 * Awesome GPUI Store — live catalog sync from `zed-industries/awesome-gpui` + `gh api`,
 * cross-platform store registry (`assets/os/awesome_gpui_store.toml`), 22-target universal
 * store & distro packager, and structural manifest verifier.
 *
 * Supported targets:
 *   winget, msstore, flatpak, playstore, appstore, snap, arch, nix, cargo, npm, chrome,
 *   apk, rpm, deb, xbps, gentoo, freebsd, brew, scoop, choco, appimage, oci
 *
 * Usage:
 *   bun run tooling/bun/awesome-gpui-store.ts sync [--out assets/os/awesome_gpui_store.toml] [--no-gh]
 *   bun run tooling/bun/awesome-gpui-store.ts list [--category <cat>] [--target <target>]
 *   bun run tooling/bun/awesome-gpui-store.ts package --id <app|all> --target <all|...> [--out target/gpui-store-dist]
 *   bun run tooling/bun/awesome-gpui-store.ts verify --dir target/gpui-store-dist
 */
import { existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { basename, join, relative, resolve } from "node:path";

export const repoRoot = resolve(import.meta.dir, "../..");
export const DEFAULT_CATALOG_PATH = join(repoRoot, "assets/os/awesome_gpui_store.toml");
export const DEFAULT_EXTENSIONS_CATALOG_PATH = join(repoRoot, "assets/os/zed_extensions_catalog.toml");
export const DEFAULT_WINGET_BASELINE_PATH = join(repoRoot, "assets/os/winget_upstream_baseline.toml");
export const UPSTREAM_REPO = "zed-industries/awesome-gpui";
export const UPSTREAM_README_URL = `https://raw.githubusercontent.com/${UPSTREAM_REPO}/main/README.md`;
export const UPSTREAM_PROJECTS_URL = `https://raw.githubusercontent.com/${UPSTREAM_REPO}/main/projects.json`;
export const UPSTREAM_EXTENSIONS_REPO = "zed-industries/extensions";
export const UPSTREAM_EXTENSIONS_TOML_URL = `https://raw.githubusercontent.com/${UPSTREAM_EXTENSIONS_REPO}/main/extensions.toml`;
export const UPSTREAM_EXTENSIONS_GITMODULES_URL = `https://raw.githubusercontent.com/${UPSTREAM_EXTENSIONS_REPO}/main/.gitmodules`;
export const UPSTREAM_WINGET_REPO = "zed-industries/winget-pkgs";
export const WINGET_MANIFEST_VERSION = "1.28.0";

export const ALL_TARGETS = [
  "winget",
  "msstore",
  "flatpak",
  "playstore",
  "appstore",
  "snap",
  "arch",
  "nix",
  "cargo",
  "npm",
  "chrome",
  "apk",
  "rpm",
  "deb",
  "xbps",
  "gentoo",
  "freebsd",
  "brew",
  "scoop",
  "choco",
  "appimage",
  "oci",
] as const;

export type StoreTarget = (typeof ALL_TARGETS)[number];
export type StoreCategory = "app" | "library" | "showcase" | "tool" | "resource";
export type EntryStatus = "active" | "quiet" | "dormant" | "archived" | "unavailable";
export type PlatformTarget = "windows" | "macos" | "linux" | "web" | "ios" | "android" | "freebsd";

export interface StoreEntry {
  id: string;
  name: string;
  tagline: string;
  description: string;
  category: StoreCategory;
  subcategory: string;
  repo: string;
  homepage: string;
  icon_url: string;
  publisher: string;
  publisher_id: string;
  app_id: string;
  crate_name: string;
  npm_package: string;
  bin_name: string;
  version: string;
  license: string;
  stars: number;
  status: EntryStatus;
  platforms: PlatformTarget[];
  targets: StoreTarget[];
  tags: string[];
  first_party: boolean;
}

export interface StoreCatalog {
  schema_version: number;
  source_repo: string;
  source_url: string;
  synced_at: string;
  upstream_stars: number;
  total_entries: number;
  entries: StoreEntry[];
}

export interface ParsedReadmeEntry {
  name: string;
  url: string;
  description: string;
  iconUrl: string;
  section: string;
  subsection: string;
  stars: number;
  status: EntryStatus;
}

export interface UpstreamProjectJson {
  name: string;
  category: string;
  subcategory?: string;
  url: string;
  image?: string;
  description: string;
}

interface RepoMetadata {
  owner: string;
  repo: string;
  stars: number;
  license: string;
  defaultBranch: string;
  archived: boolean;
  pushedAt: string;
  homepage: string;
  topics: string[];
  crateName?: string;
  version?: string;
  cargoLicense?: string;
  binName?: string;
}

export interface VerifyReport {
  ok: boolean;
  checkedFiles: number;
  targetsChecked: string[];
  errors: string[];
}

export function slugify(raw: string): string {
  const cleaned = raw
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return cleaned || "gpui-app";
}

export function pascalIdentifier(raw: string): string {
  const parts = raw
    .split(/[^a-zA-Z0-9]+/)
    .map((p) => p.trim())
    .filter(Boolean);
  const joined = parts.map((p) => p.charAt(0).toUpperCase() + p.slice(1)).join("");
  if (!joined) return "GpuiApp";
  return /^[0-9]/.test(joined) ? `App${joined}` : joined;
}

export function sanitizeSemver(raw: string): string {
  const match = raw.match(/(\d+)\.(\d+)\.(\d+)/);
  if (match) return `${match[1]}.${match[2]}.${match[3]}`;
  const two = raw.match(/(\d+)\.(\d+)/);
  if (two) return `${two[1]}.${two[2]}.0`;
  return "0.1.0";
}

export function sanitizeMsixVersion(raw: string): string {
  return `${sanitizeSemver(raw)}.0`;
}

export function sanitizeArchVersion(raw: string): string {
  return sanitizeSemver(raw).replace(/[^0-9A-Za-z._]/g, ".");
}

export function parseStarCount(cell: string): number {
  const clean = cell.replace(/[^0-9.kKmM]/g, "").trim();
  if (!clean) return 0;
  const lower = clean.toLowerCase();
  if (lower.endsWith("k")) {
    const n = Number.parseFloat(lower.slice(0, -1));
    return Number.isFinite(n) ? Math.round(n * 1000) : 0;
  }
  if (lower.endsWith("m")) {
    const n = Number.parseFloat(lower.slice(0, -1));
    return Number.isFinite(n) ? Math.round(n * 1_000_000) : 0;
  }
  const n = Number.parseInt(lower, 10);
  return Number.isFinite(n) ? n : 0;
}

export function parseStatusEmoji(cell: string): EntryStatus {
  if (cell.includes("🟢")) return "active";
  if (cell.includes("🟡")) return "quiet";
  if (cell.includes("⚪")) return "dormant";
  if (cell.includes("📦")) return "archived";
  return "unavailable";
}

export function extractGithubOwnerRepo(url: string): { owner: string; repo: string } | null {
  const m = url.match(/^https?:\/\/github\.com\/([^/]+)\/([^/#?]+)/i);
  if (!m) return null;
  const owner = m[1].trim();
  const repo = m[2].replace(/\.git$/i, "").trim();
  if (!owner || !repo) return null;
  return { owner, repo };
}

function stripMarkdownLinks(text: string): string {
  return text.replace(/\[([^\]]+)\]\([^)]+\)/g, "$1").replace(/\s+/g, " ").trim();
}

function splitTableRow(line: string): string[] {
  const trimmed = line.trim();
  if (!trimmed.startsWith("|")) return [];
  const inner = trimmed.startsWith("|") ? trimmed.slice(1) : trimmed;
  const withoutEnd = inner.endsWith("|") ? inner.slice(0, -1) : inner;
  const cells: string[] = [];
  let cur = "";
  for (let i = 0; i < withoutEnd.length; i++) {
    const ch = withoutEnd[i];
    if (ch === "\\" && withoutEnd[i + 1] === "|") {
      cur += "|";
      i++;
    } else if (ch === "|") {
      cells.push(cur.trim());
      cur = "";
    } else {
      cur += ch;
    }
  }
  cells.push(cur.trim());
  return cells;
}

export function parseAwesomeGpuiMarkdown(markdown: string): ParsedReadmeEntry[] {
  const entries: ParsedReadmeEntry[] = [];
  let section = "Apps";
  let subsection = "";

  for (const rawLine of markdown.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (line.startsWith("## ")) {
      section = line.slice(3).trim();
      subsection = "";
      continue;
    }
    if (line.startsWith("### ")) {
      subsection = line.slice(4).trim();
      continue;
    }
    if (!line.startsWith("|") || line.startsWith("| Image") || /^\|[\s:|-]+$/.test(line)) {
      continue;
    }
    const cols = splitTableRow(line);
    if (cols.length < 5) continue;
    const projectMatch = cols[1].match(/^\[([^\]]+)\]\(([^)]+)\)$/);
    if (!projectMatch) continue;

    const imgMatch = cols[0].match(/src=["']([^"']+)["']/i);
    entries.push({
      name: projectMatch[1].trim(),
      url: projectMatch[2].trim(),
      description: stripMarkdownLinks(cols[2]),
      iconUrl: imgMatch ? imgMatch[1].trim() : "",
      section,
      subsection,
      stars: parseStarCount(cols[3]),
      status: parseStatusEmoji(cols[4]),
    });
  }

  return entries;
}

function classifyCategory(
  section: string,
  subsection: string,
  description: string,
  topics: string[],
): { category: StoreCategory; subcategory: string } {
  const sec = section.toLowerCase();
  const sub = subsection.toLowerCase();
  const desc = description.toLowerCase();
  if (sec === "apps" && sub.includes("demo")) {
    return { category: "showcase", subcategory: subsection || "Demos & Examples" };
  }
  if (sec === "apps") {
    return { category: "app", subcategory: subsection || "General" };
  }
  if (sec === "libraries") {
    const isUiKit =
      topics.some((t) => /ui|component|kit|theme|design-system/i.test(t)) ||
      /\b(ui component|component library|ui kit|design|theme|human interface|mantine|swiftui|base ui|storybook)\b/i.test(
        desc,
      );
    return { category: "library", subcategory: isUiKit ? "UI Kits" : "Libraries" };
  }
  if (sec === "tooling") {
    return { category: "tool", subcategory: subsection || "Tooling" };
  }
  return { category: "resource", subcategory: subsection || "Resources" };
}

function inferPlatforms(
  category: StoreCategory,
  description: string,
  topics: string[],
  repoText = "",
): PlatformTarget[] {
  const haystack = `${description} ${topics.join(" ")} ${repoText}`.toLowerCase();
  const set = new Set<PlatformTarget>();

  if (/\b(windows|win32|wsl2|direct3d|d3d11|winget|msix|scoop|choco)\b/.test(haystack)) {
    set.add("windows");
  }
  if (/\b(macos|mac os|apple|appkit|swiftui|finder|menu-bar|catalyst|homebrew)\b/.test(haystack)) {
    set.add("macos");
  }
  if (/\b(linux|wayland|x11|omarchy|dbus|d-bus|arch|flatpak|gtk|alpine|fedora|debian|void|gentoo)\b/.test(haystack)) {
    set.add("linux");
    set.add("freebsd");
  }
  if (/\b(freebsd|bsd)\b/.test(haystack)) set.add("freebsd");
  if (/\b(wasm|webassembly|webgpu|webgl|browser|web|chrome)\b/.test(haystack)) set.add("web");
  if (/\b(ios|iphone|ipad|uikit|mobile)\b/.test(haystack)) set.add("ios");
  if (/\b(android|apk|ndk|mobile|openharmony)\b/.test(haystack)) set.add("android");

  if (/\b(cross[- ]platform|multiplatform|desktop)\b/.test(haystack) || set.size === 0) {
    set.add("windows");
    set.add("macos");
    set.add("linux");
    set.add("freebsd");
  }
  if (category === "library" || category === "tool" || category === "showcase") {
    set.add("windows");
    set.add("macos");
    set.add("linux");
    set.add("freebsd");
  }
  if (category === "resource") {
    set.add("web");
  }

  return Array.from(set);
}

function inferTargets(
  category: StoreCategory,
  platforms: PlatformTarget[],
  isFirstParty: boolean,
  hasGithubRepo: boolean,
): StoreTarget[] {
  if (isFirstParty) return [...ALL_TARGETS];
  const targets = new Set<StoreTarget>();
  if (hasGithubRepo && category !== "resource") {
    targets.add("cargo");
  }
  if (platforms.includes("windows")) {
    targets.add("winget");
    targets.add("msstore");
    targets.add("scoop");
    targets.add("choco");
  }
  if (platforms.includes("linux")) {
    targets.add("flatpak");
    targets.add("snap");
    targets.add("arch");
    targets.add("nix");
    targets.add("apk");
    targets.add("rpm");
    targets.add("deb");
    targets.add("xbps");
    targets.add("gentoo");
    targets.add("appimage");
    targets.add("oci");
  }
  if (platforms.includes("freebsd")) {
    targets.add("freebsd");
  }
  if (platforms.includes("macos")) {
    targets.add("appstore");
    targets.add("brew");
    targets.add("nix");
  } else if (platforms.includes("ios")) {
    targets.add("appstore");
  }
  if (platforms.includes("android")) {
    targets.add("playstore");
  }
  if (platforms.includes("web")) {
    targets.add("npm");
    targets.add("chrome");
  }
  if (targets.size === 0) {
    targets.add("cargo");
    targets.add("npm");
    targets.add("chrome");
  }
  return ALL_TARGETS.filter((t) => targets.has(t));
}

function makeTagline(desc: string): string {
  const sentence = desc.split(/(?<=[.!?])\s+/)[0] ?? desc;
  if (sentence.length <= 80) return sentence;
  return `${sentence.slice(0, 77).trimEnd()}...`;
}

function parseCargoMetadata(text: string | undefined): {
  crateName?: string;
  version?: string;
  license?: string;
  binName?: string;
} {
  if (!text) return {};
  try {
    const parsed = Bun.TOML.parse(text) as Record<string, unknown>;
    const pkg = (parsed.package ?? {}) as Record<string, unknown>;
    const wp = ((parsed.workspace as Record<string, unknown> | undefined)?.package ?? {}) as Record<
      string,
      unknown
    >;
    const bins = Array.isArray(parsed.bin) ? (parsed.bin as Array<Record<string, unknown>>) : [];
    const crateName = typeof pkg.name === "string" ? pkg.name : undefined;
    const version =
      typeof pkg.version === "string"
        ? pkg.version
        : typeof wp.version === "string"
          ? wp.version
          : undefined;
    const license =
      typeof pkg.license === "string"
        ? pkg.license
        : typeof wp.license === "string"
          ? wp.license
          : undefined;
    const binName = typeof bins[0]?.name === "string" ? (bins[0].name as string) : crateName;
    return { crateName, version, license, binName };
  } catch {
    return {};
  }
}

async function fetchRepoMetadataBatch(
  repos: Array<{ owner: string; repo: string }>,
): Promise<Map<string, RepoMetadata>> {
  const out = new Map<string, RepoMetadata>();
  const unique = new Map<string, { owner: string; repo: string }>();
  for (const r of repos) {
    const key = `${r.owner.toLowerCase()}/${r.repo.toLowerCase()}`;
    if (!unique.has(key)) unique.set(key, r);
  }
  const items = Array.from(unique.values());
  const batchSize = 35;

  for (let i = 0; i < items.length; i += batchSize) {
    const chunk = items.slice(i, i + batchSize);
    const fields = chunk
      .map(
        (r, idx) =>
          `r${idx}: repository(owner: ${JSON.stringify(r.owner)}, name: ${JSON.stringify(r.repo)}) {
            stargazerCount
            isArchived
            pushedAt
            homepageUrl
            licenseInfo { spdxId }
            defaultBranchRef { name }
            repositoryTopics(first: 8) { nodes { topic { name } } }
            cargoToml: object(expression: "HEAD:Cargo.toml") { ... on Blob { text } }
          }`,
      )
      .join("\n");
    const query = `query {\n${fields}\n}`;
    const proc = Bun.spawnSync(["gh", "api", "graphql", "-f", `query=${query}`], {
      stdout: "pipe",
      stderr: "pipe",
    });
    if (proc.exitCode !== 0) continue;

    try {
      const payload = JSON.parse(new TextDecoder().decode(proc.stdout)) as {
        data?: Record<string, Record<string, unknown> | null>;
      };
      const data = payload.data ?? {};
      for (let idx = 0; idx < chunk.length; idx++) {
        const node = data[`r${idx}`];
        if (!node) continue;
        const r = chunk[idx];
        const key = `${r.owner.toLowerCase()}/${r.repo.toLowerCase()}`;
        const licenseInfo = node.licenseInfo as { spdxId?: string } | null;
        const defaultBranch = (node.defaultBranchRef as { name?: string } | null)?.name ?? "main";
        const topicNodes =
          ((node.repositoryTopics as { nodes?: Array<{ topic?: { name?: string } }> } | null)
            ?.nodes ?? []);
        const topics = topicNodes
          .map((n) => n.topic?.name)
          .filter((n): n is string => Boolean(n));
        const cargoText = (node.cargoToml as { text?: string } | null)?.text;
        const cargo = parseCargoMetadata(cargoText);
        out.set(key, {
          owner: r.owner,
          repo: r.repo,
          stars: typeof node.stargazerCount === "number" ? node.stargazerCount : 0,
          license:
            cargo.license ??
            (licenseInfo?.spdxId && licenseInfo.spdxId !== "NOASSERTION"
              ? licenseInfo.spdxId
              : "Unknown"),
          defaultBranch,
          archived: Boolean(node.isArchived),
          pushedAt: typeof node.pushedAt === "string" ? node.pushedAt : "",
          homepage: typeof node.homepageUrl === "string" ? node.homepageUrl : "",
          topics,
          crateName: cargo.crateName,
          version: cargo.version,
          cargoLicense: cargo.license,
          binName: cargo.binName,
        });
      }
    } catch {
      // Fall back cleanly to markdown-parsed data for this batch.
    }
  }
  return out;
}

function buildFirstPartyEntries(): StoreEntry[] {
  const productPath = join(repoRoot, "product.toml");
  const product = existsSync(productPath)
    ? (Bun.TOML.parse(readFileSync(productPath, "utf8")) as Record<string, Record<string, unknown>>)
    : {};
  const prod = product.product ?? {};
  const urls = product.urls ?? {};
  const plugin = product.plugin ?? {};

  const appName = String(prod.name ?? "Aphrody");
  const appSlug = String(prod.slug ?? "aphrody");
  const company = String(prod.company_name ?? "Aphrody Labs");
  const publisherId = pascalIdentifier(company);
  const identifierPrefix = String(prod.identifier_prefix ?? "com.aphrody.Aphrody");
  const version = sanitizeSemver(String(plugin.version ?? "1.30.0"));
  const repoUrl = String(urls.repository ?? "https://github.com/aphrody-dev/aphrody");
  const homepageUrl = String(urls.homepage ?? "https://aphrody.com");
  const tagline = String(prod.tagline ?? "GPU-native code & UI studio");
  const description = String(
    prod.description ?? "Ultra-fast, modular GPU-native code and UI studio.",
  );

  return [
    {
      id: appSlug,
      name: appName,
      tagline,
      description,
      category: "app",
      subcategory: "Developer Tools",
      repo: repoUrl,
      homepage: homepageUrl,
      icon_url: `${homepageUrl}/assets/images/aphrody_logo.svg`,
      publisher: company,
      publisher_id: publisherId,
      app_id: identifierPrefix,
      crate_name: "zed",
      npm_package: `@${appSlug}/${appSlug}`,
      bin_name: appSlug,
      version,
      license: "GPL-3.0-or-later",
      stars: 0,
      status: "active",
      platforms: ["windows", "macos", "linux", "freebsd", "web", "ios", "android"],
      targets: [...ALL_TARGETS],
      tags: ["gpui", "nsh", "webgpu", "rust", "ide", "first-party"],
      first_party: true,
    },
    {
      id: `${appSlug}-os`,
      name: `${appName} OS`,
      tagline: "Cross-platform COSMIC + GPUI + NSH Desktop & Store",
      description:
        "Unified GPUI desktop shell, ArtCraft studio, Craft Anything binary inspector, and Awesome GPUI Store.",
      category: "app",
      subcategory: "System Utilities",
      repo: repoUrl,
      homepage: homepageUrl,
      icon_url: `${homepageUrl}/assets/images/aphrody_logo.svg`,
      publisher: company,
      publisher_id: publisherId,
      app_id: `${identifierPrefix}.OS`,
      crate_name: "os_desktop",
      npm_package: `@${appSlug}/os-desktop`,
      bin_name: `${appSlug}-os`,
      version,
      license: "GPL-3.0-or-later",
      stars: 0,
      status: "active",
      platforms: ["windows", "macos", "linux", "freebsd", "web", "ios", "android"],
      targets: [...ALL_TARGETS],
      tags: ["gpui", "cosmic", "nsh", "wgpu", "os", "first-party"],
      first_party: true,
    },
    {
      id: "gpui-store",
      name: "Awesome GPUI Store",
      tagline: "NSH + GPUI cross-platform app store & 22-target universal packager",
      description:
        "Browse live awesome-gpui apps, UI kits, showcases, and tools and emit manifests across 22 desktop, mobile, browser, distro, and OCI ecosystems.",
      category: "tool",
      subcategory: "Packaging & Store",
      repo: repoUrl,
      homepage: `${homepageUrl}/store`,
      icon_url: `${homepageUrl}/assets/images/aphrody_logo.svg`,
      publisher: company,
      publisher_id: publisherId,
      app_id: `${identifierPrefix}.Store`,
      crate_name: "gpui_store",
      npm_package: `@${appSlug}/gpui-store`,
      bin_name: "gpui-store",
      version,
      license: "GPL-3.0-or-later",
      stars: 0,
      status: "active",
      platforms: ["windows", "macos", "linux", "freebsd", "web", "ios", "android"],
      targets: [...ALL_TARGETS],
      tags: ["gpui", "store", "winget", "msix", "flatpak", "rpm", "deb", "apk", "brew", "nsh"],
      first_party: true,
    },
    {
      id: "gpui-wgpu",
      name: "GPUI WebGPU & Wasm CLI",
      tagline: "Native Wasm LTO/AOT, WGSL/Naga shader compiler, and 15-tool MCP server",
      description:
        "Unified Wasm + WebGPU CLI and MCP toolchain for WGSL/SPIR-V/HLSL/MSL transpilation, wasmparser inspection, binaryen LTO optimization, Wasmtime AOT .cwasm compilation, and project scaffolding.",
      category: "tool",
      subcategory: "Graphics & Wasm",
      repo: repoUrl,
      homepage: `${homepageUrl}/docs/webgpu-wasm-first`,
      icon_url: `${homepageUrl}/assets/images/aphrody_logo.svg`,
      publisher: company,
      publisher_id: publisherId,
      app_id: `${identifierPrefix}.WgpuCli`,
      crate_name: "wgpu_cli",
      npm_package: `@${appSlug}/gpui-wgpu`,
      bin_name: "gpui-wgpu",
      version,
      license: "Apache-2.0",
      stars: 0,
      status: "active",
      platforms: ["windows", "macos", "linux", "freebsd", "web"],
      targets: [...ALL_TARGETS],
      tags: ["wgpu", "webgpu", "wasm", "wgsl", "mcp", "cli", "aot", "lto", "first-party"],
      first_party: true,
    },
  ];
}

export function buildCatalogEntries(
  parsedRows: ParsedReadmeEntry[],
  ghMeta: Map<string, RepoMetadata>,
  includeFirstParty = true,
): StoreEntry[] {
  const seenIds = new Set<string>();
  const entries: StoreEntry[] = [];

  if (includeFirstParty) {
    for (const fp of buildFirstPartyEntries()) {
      seenIds.add(fp.id);
      entries.push(fp);
    }
  }

  for (const row of parsedRows) {
    const ghRef = extractGithubOwnerRepo(row.url);
    const ghKey = ghRef ? `${ghRef.owner.toLowerCase()}/${ghRef.repo.toLowerCase()}` : "";
    const meta = ghKey ? ghMeta.get(ghKey) : undefined;
    const topics = meta?.topics ?? [];

    let id = slugify(row.name);
    if (seenIds.has(id)) {
      id = ghRef ? slugify(`${ghRef.owner}-${row.name}`) : `${id}-2`;
    }
    while (seenIds.has(id)) {
      id = `${id}-entry`;
    }
    seenIds.add(id);

    const { category, subcategory } = classifyCategory(
      row.section,
      row.subsection,
      row.description,
      topics,
    );
    const platforms = inferPlatforms(category, row.description, topics);
    const targets = inferTargets(category, platforms, false, Boolean(ghRef));
    const owner = ghRef?.owner ?? id;
    const repoName = ghRef?.repo ?? id;
    const publisher = ghRef ? ghRef.owner : pascalIdentifier(row.name);
    const publisherId = pascalIdentifier(publisher);
    const pkgId = pascalIdentifier(row.name);
    const crateName = meta?.crateName ?? slugify(repoName);
    const binName = meta?.binName ?? slugify(crateName);
    const version = sanitizeSemver(meta?.version ?? "0.1.0");
    const license = meta?.license ?? (ghRef ? "MIT OR Apache-2.0" : "Proprietary");
    const homepage = meta?.homepage && meta.homepage.startsWith("http") ? meta.homepage : row.url;
    const domainSeg = slugify(owner).replace(/-/g, "");
    const appSeg = slugify(pkgId).replace(/-/g, "");
    const appId = ghRef ? `io.github.${domainSeg}.${pkgId}` : `org.gpui.${appSeg}.${pkgId}`;
    const tags = Array.from(
      new Set(["gpui", "rust", category, ...topics.slice(0, 5), ...platforms.slice(0, 3)]),
    );

    entries.push({
      id,
      name: row.name,
      tagline: makeTagline(row.description),
      description: row.description,
      category,
      subcategory,
      repo: ghRef ? `https://github.com/${ghRef.owner}/${ghRef.repo}` : row.url,
      homepage,
      icon_url: row.iconUrl,
      publisher,
      publisher_id: publisherId,
      app_id: appId,
      crate_name: crateName,
      npm_package: `@gpui-store/${slugify(crateName)}`,
      bin_name: binName,
      version,
      license: license === "Unknown" ? "MIT OR Apache-2.0" : license,
      stars: meta?.stars && meta.stars > 0 ? meta.stars : row.stars,
      status: meta?.archived ? "archived" : row.status,
      platforms,
      targets,
      tags,
      first_party: false,
    });
  }

  return entries;
}

function tomlStr(val: string): string {
  return JSON.stringify(val);
}

function tomlArr(items: string[]): string {
  return `[${items.map((item) => tomlStr(item)).join(", ")}]`;
}

export function serializeCatalogToml(catalog: StoreCatalog): string {
  const lines: string[] = [
    "# Auto-generated by tooling/bun/awesome-gpui-store.ts sync — edit upstream or re-run sync.",
    `schema_version = ${catalog.schema_version}`,
    `source_repo = ${tomlStr(catalog.source_repo)}`,
    `source_url = ${tomlStr(catalog.source_url)}`,
    `synced_at = ${tomlStr(catalog.synced_at)}`,
    `upstream_stars = ${catalog.upstream_stars}`,
    `total_entries = ${catalog.entries.length}`,
    "",
  ];

  for (const e of catalog.entries) {
    lines.push("[[entries]]");
    lines.push(`id = ${tomlStr(e.id)}`);
    lines.push(`name = ${tomlStr(e.name)}`);
    lines.push(`tagline = ${tomlStr(e.tagline)}`);
    lines.push(`description = ${tomlStr(e.description)}`);
    lines.push(`category = ${tomlStr(e.category)}`);
    lines.push(`subcategory = ${tomlStr(e.subcategory)}`);
    lines.push(`repo = ${tomlStr(e.repo)}`);
    lines.push(`homepage = ${tomlStr(e.homepage)}`);
    lines.push(`icon_url = ${tomlStr(e.icon_url)}`);
    lines.push(`publisher = ${tomlStr(e.publisher)}`);
    lines.push(`publisher_id = ${tomlStr(e.publisher_id)}`);
    lines.push(`app_id = ${tomlStr(e.app_id)}`);
    lines.push(`crate_name = ${tomlStr(e.crate_name)}`);
    lines.push(`npm_package = ${tomlStr(e.npm_package)}`);
    lines.push(`bin_name = ${tomlStr(e.bin_name)}`);
    lines.push(`version = ${tomlStr(e.version)}`);
    lines.push(`license = ${tomlStr(e.license)}`);
    lines.push(`stars = ${e.stars}`);
    lines.push(`status = ${tomlStr(e.status)}`);
    lines.push(`platforms = ${tomlArr(e.platforms)}`);
    lines.push(`targets = ${tomlArr(e.targets)}`);
    lines.push(`tags = ${tomlArr(e.tags)}`);
    lines.push(`first_party = ${e.first_party}`);
    lines.push("");
  }

  return `${lines.join("\n")}`;
}

export async function syncCatalog(outPath = DEFAULT_CATALOG_PATH, useGh = true): Promise<StoreCatalog> {
  const readmeResp = await fetch(UPSTREAM_README_URL);
  if (!readmeResp.ok) {
    throw new Error(`Failed to fetch ${UPSTREAM_README_URL}: ${readmeResp.status}`);
  }
  const readmeText = await readmeResp.text();
  const parsedRows = parseAwesomeGpuiMarkdown(readmeText);

  let upstreamStars = 0;
  let ghMeta = new Map<string, RepoMetadata>();
  if (useGh) {
    const repoInfo = Bun.spawnSync(
      ["gh", "api", `repos/${UPSTREAM_REPO}`, "--jq", ".stargazers_count"],
      { stdout: "pipe", stderr: "pipe" },
    );
    if (repoInfo.exitCode === 0) {
      const n = Number.parseInt(new TextDecoder().decode(repoInfo.stdout).trim(), 10);
      if (Number.isFinite(n)) upstreamStars = n;
      const githubRepos = parsedRows
        .map((r) => extractGithubOwnerRepo(r.url))
        .filter((x): x is { owner: string; repo: string } => x !== null);
      ghMeta = await fetchRepoMetadataBatch(githubRepos);
    }
  }

  const entries = buildCatalogEntries(parsedRows, ghMeta, true);
  const catalog: StoreCatalog = {
    schema_version: 1,
    source_repo: UPSTREAM_REPO,
    source_url: `https://github.com/${UPSTREAM_REPO}`,
    synced_at: new Date().toISOString(),
    upstream_stars: upstreamStars,
    total_entries: entries.length,
    entries,
  };

  mkdirSync(resolve(outPath, ".."), { recursive: true });
  writeFileSync(outPath, serializeCatalogToml(catalog), "utf8");
  return catalog;
}

export function loadStoreCatalog(tomlPath = DEFAULT_CATALOG_PATH): StoreCatalog {
  const raw = readFileSync(tomlPath, "utf8");
  return Bun.TOML.parse(raw) as unknown as StoreCatalog;
}

export interface ZedExtensionEntry {
  id: string;
  submodule: string;
  repo_url: string;
  version: string;
  category: string;
}

export interface ZedExtensionsCatalog {
  schema_version: number;
  source_repo: string;
  source_url: string;
  synced_at: string;
  total_extensions: number;
  total_submodules: number;
  category_counts: Record<string, number>;
  entries: ZedExtensionEntry[];
}

export interface WingetUpstreamBaseline {
  schema_version: number;
  source_repo: string;
  source_url: string;
  synced_at: string;
  package_identifier: string;
  latest_version: string;
  manifest_version: string;
  scope: string;
  install_modes: string[];
  upgrade_behavior: string;
  moniker: string;
}

function classifyZedExtension(id: string): string {
  const lower = id.toLowerCase();
  if (lower.includes("icon")) return "icon-theme";
  if (
    /theme|color|night|catppuccin|dracula|gruvbox|monokai|nord|solarized|one-dark|one-light|ayu/.test(
      lower,
    )
  ) {
    return "theme";
  }
  if (lower.includes("mcp") || lower.includes("context-server")) return "context-server";
  if (lower.includes("snippet")) return "snippet";
  if (lower.includes("debug") || lower.includes("dap")) return "debugger";
  return "language";
}

function parseGitmodulesUrlMap(raw: string): Map<string, string> {
  const map = new Map<string, string>();
  let path = "";
  let url = "";
  for (const rawLine of raw.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (line.startsWith("[submodule ")) {
      if (path && url) map.set(path, url);
      path = "";
      url = "";
      continue;
    }
    if (line.startsWith("path = ")) path = line.slice(7).trim();
    else if (line.startsWith("url = ")) url = line.slice(6).trim();
  }
  if (path && url) map.set(path, url);
  return map;
}

export function buildZedExtensionsCatalog(
  extensionsTomlRaw: string,
  gitmodulesRaw: string,
  syncedAt = new Date().toISOString(),
): ZedExtensionsCatalog {
  const parsed = Bun.TOML.parse(extensionsTomlRaw) as Record<
    string,
    { submodule?: string; version?: string }
  >;
  const submoduleUrls = parseGitmodulesUrlMap(gitmodulesRaw);
  const categoryCounts: Record<string, number> = {
    language: 0,
    theme: 0,
    "icon-theme": 0,
    "context-server": 0,
    snippet: 0,
    debugger: 0,
  };
  const entries: ZedExtensionEntry[] = [];
  for (const id of Object.keys(parsed).sort()) {
    const item = parsed[id];
    if (!item || typeof item !== "object") continue;
    const submodule = String(item.submodule ?? `extensions/${id}`);
    const version = String(item.version ?? "0.0.0");
    const repoUrl = (submoduleUrls.get(submodule) ?? `https://github.com/${UPSTREAM_EXTENSIONS_REPO}`).replace(
      /\.git$/,
      "",
    );
    const category = classifyZedExtension(id);
    categoryCounts[category] = (categoryCounts[category] ?? 0) + 1;
    entries.push({ id, submodule, repo_url: repoUrl, version, category });
  }
  return {
    schema_version: 1,
    source_repo: UPSTREAM_EXTENSIONS_REPO,
    source_url: `https://github.com/${UPSTREAM_EXTENSIONS_REPO}`,
    synced_at: syncedAt,
    total_extensions: entries.length,
    total_submodules: submoduleUrls.size,
    category_counts: categoryCounts,
    entries,
  };
}

export function serializeExtensionsCatalogToml(catalog: ZedExtensionsCatalog): string {
  const lines: string[] = [
    "# Auto-generated by tooling/bun/awesome-gpui-store.ts sync-extensions — do not hand-edit.",
    `schema_version = ${catalog.schema_version}`,
    `source_repo = ${tomlStr(catalog.source_repo)}`,
    `source_url = ${tomlStr(catalog.source_url)}`,
    `synced_at = ${tomlStr(catalog.synced_at)}`,
    `total_extensions = ${catalog.total_extensions}`,
    `total_submodules = ${catalog.total_submodules}`,
    "",
    "[category_counts]",
    ...Object.entries(catalog.category_counts)
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([k, v]) => `${JSON.stringify(k)} = ${v}`),
    "",
  ];
  for (const e of catalog.entries) {
    lines.push("[[entries]]");
    lines.push(`id = ${tomlStr(e.id)}`);
    lines.push(`submodule = ${tomlStr(e.submodule)}`);
    lines.push(`repo_url = ${tomlStr(e.repo_url)}`);
    lines.push(`version = ${tomlStr(e.version)}`);
    lines.push(`category = ${tomlStr(e.category)}`);
    lines.push("");
  }
  return lines.join("\n");
}

export async function syncExtensionsCatalog(
  outPath = DEFAULT_EXTENSIONS_CATALOG_PATH,
): Promise<ZedExtensionsCatalog> {
  const [extResp, modResp] = await Promise.all([
    fetch(UPSTREAM_EXTENSIONS_TOML_URL),
    fetch(UPSTREAM_EXTENSIONS_GITMODULES_URL),
  ]);
  if (!extResp.ok) {
    throw new Error(`Failed to fetch ${UPSTREAM_EXTENSIONS_TOML_URL}: ${extResp.status}`);
  }
  const extRaw = await extResp.text();
  const modRaw = modResp.ok ? await modResp.text() : "";
  const catalog = buildZedExtensionsCatalog(extRaw, modRaw);
  mkdirSync(resolve(outPath, ".."), { recursive: true });
  writeFileSync(outPath, serializeExtensionsCatalogToml(catalog), "utf8");
  return catalog;
}

function serializeWingetBaselineToml(b: WingetUpstreamBaseline): string {
  return [
    "# Auto-generated by tooling/bun/awesome-gpui-store.ts sync-winget — do not hand-edit.",
    `schema_version = ${b.schema_version}`,
    `source_repo = ${tomlStr(b.source_repo)}`,
    `source_url = ${tomlStr(b.source_url)}`,
    `synced_at = ${tomlStr(b.synced_at)}`,
    `package_identifier = ${tomlStr(b.package_identifier)}`,
    `latest_version = ${tomlStr(b.latest_version)}`,
    `manifest_version = ${tomlStr(b.manifest_version)}`,
    `scope = ${tomlStr(b.scope)}`,
    `install_modes = ${tomlArr(b.install_modes)}`,
    `upgrade_behavior = ${tomlStr(b.upgrade_behavior)}`,
    `moniker = ${tomlStr(b.moniker)}`,
    "",
  ].join("\n");
}

export async function syncWingetBaseline(
  outPath = DEFAULT_WINGET_BASELINE_PATH,
): Promise<WingetUpstreamBaseline> {
  let latestVersion = "1.27.0";
  let manifestVersion = WINGET_MANIFEST_VERSION;
  let scope = "user";
  let installModes = ["interactive", "silent", "silentWithProgress"];
  let upgradeBehavior = "install";
  let moniker = "zed";

  const listProc = Bun.spawnSync(
    ["gh", "api", `repos/${UPSTREAM_WINGET_REPO}/contents/manifests/z/ZedIndustries/Zed`, "--jq", ".[].name"],
    { stdout: "pipe", stderr: "pipe" },
  );
  if (listProc.exitCode === 0) {
    const versions = new TextDecoder()
      .decode(listProc.stdout)
      .split(/\r?\n/)
      .map((s) => s.trim())
      .filter((s) => /^\d+\.\d+\.\d+$/.test(s))
      .sort((a, b) => a.localeCompare(b, undefined, { numeric: true }));
    if (versions.length > 0) {
      latestVersion = versions[versions.length - 1];
      const instUrl = `https://raw.githubusercontent.com/${UPSTREAM_WINGET_REPO}/main/manifests/z/ZedIndustries/Zed/${latestVersion}/ZedIndustries.Zed.installer.yaml`;
      const locUrl = `https://raw.githubusercontent.com/${UPSTREAM_WINGET_REPO}/main/manifests/z/ZedIndustries/Zed/${latestVersion}/ZedIndustries.Zed.locale.en-US.yaml`;
      const [instRes, locRes] = await Promise.all([fetch(instUrl), fetch(locUrl)]);
      if (instRes.ok) {
        const inst = Bun.YAML.parse(await instRes.text()) as Record<string, unknown>;
        if (typeof inst.ManifestVersion === "string") manifestVersion = inst.ManifestVersion;
        if (typeof inst.Scope === "string") scope = inst.Scope;
        if (Array.isArray(inst.InstallModes)) installModes = inst.InstallModes.map(String);
        if (typeof inst.UpgradeBehavior === "string") upgradeBehavior = inst.UpgradeBehavior;
      }
      if (locRes.ok) {
        const loc = Bun.YAML.parse(await locRes.text()) as Record<string, unknown>;
        if (typeof loc.Moniker === "string") moniker = loc.Moniker;
      }
    }
  }

  const baseline: WingetUpstreamBaseline = {
    schema_version: 1,
    source_repo: UPSTREAM_WINGET_REPO,
    source_url: `https://github.com/${UPSTREAM_WINGET_REPO}`,
    synced_at: new Date().toISOString(),
    package_identifier: "ZedIndustries.Zed",
    latest_version: latestVersion,
    manifest_version: manifestVersion,
    scope,
    install_modes: installModes,
    upgrade_behavior: upgradeBehavior,
    moniker,
  };
  mkdirSync(resolve(outPath, ".."), { recursive: true });
  writeFileSync(outPath, serializeWingetBaselineToml(baseline), "utf8");
  return baseline;
}

function xmlEscape(val: string): string {
  return val
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&apos;");
}

function yamlStr(val: string): string {
  return JSON.stringify(val);
}

function snapSummary(tagline: string, name: string): string {
  const clean = (tagline || name).replace(/\s+/g, " ").trim();
  return clean.length <= 78 ? clean : `${clean.slice(0, 75).trimEnd()}...`;
}

function androidPackageName(appId: string): string {
  const parts = appId
    .split(".")
    .map((seg) => seg.toLowerCase().replace(/[^a-z0-9_]/g, ""))
    .filter(Boolean)
    .map((seg) => (/^[0-9]/.test(seg) ? `a${seg}` : seg));
  return parts.length >= 2 ? parts.join(".") : `org.gpui.${parts[0] ?? "app"}`;
}

function wingetIdentifier(entry: StoreEntry): string {
  return `${entry.publisher_id}.${pascalIdentifier(entry.name)}`;
}

export function generateTargetManifests(
  entry: StoreEntry,
  target: StoreTarget,
): Record<string, string> {
  const ver = sanitizeSemver(entry.version);
  const msixVer = sanitizeMsixVersion(ver);
  const archVer = sanitizeArchVersion(ver);
  const wgId = wingetIdentifier(entry);
  const pkgSlug = slugify(entry.id || entry.crate_name);
  const bin = entry.bin_name || pkgSlug;
  const androidPkg = androidPackageName(entry.app_id);
  const androidLib = slugify(entry.crate_name || entry.id).replace(/-/g, "_");
  const releaseBase = entry.repo.startsWith("https://github.com/")
    ? `${entry.repo}/releases/download/v${ver}`
    : `${entry.homepage.replace(/\/$/, "")}/releases/v${ver}`;
  const archiveUrl = `${entry.repo}/archive/refs/tags/v${ver}.tar.gz`;
  const zeroSha = "0".repeat(64);
  const ghRef = extractGithubOwnerRepo(entry.repo);

  switch (target) {
    case "winget":
      return {
        [`${wgId}.yaml`]: [
          `PackageIdentifier: ${wgId}`,
          `PackageVersion: ${yamlStr(ver)}`,
          "DefaultLocale: en-US",
          "ManifestType: version",
          `ManifestVersion: ${WINGET_MANIFEST_VERSION}`,
          "",
        ].join("\n"),
        [`${wgId}.installer.yaml`]: [
          `PackageIdentifier: ${wgId}`,
          `PackageVersion: ${yamlStr(ver)}`,
          "Platform:",
          "  - Windows.Desktop",
          "MinimumOSVersion: 10.0.19041.0",
          "InstallerType: zip",
          "Scope: user",
          "InstallModes:",
          "  - interactive",
          "  - silent",
          "  - silentWithProgress",
          "UpgradeBehavior: install",
          "Installers:",
          "  - Architecture: x64",
          `    InstallerUrl: ${yamlStr(`${releaseBase}/${bin}-windows-x64.zip`)}`,
          `    InstallerSha256: ${zeroSha}`,
          "    NestedInstallerType: portable",
          "    NestedInstallerFiles:",
          `      - RelativeFilePath: ${yamlStr(`${bin}.exe`)}`,
          `        PortableCommandAlias: ${yamlStr(bin)}`,
          "  - Architecture: arm64",
          `    InstallerUrl: ${yamlStr(`${releaseBase}/${bin}-windows-arm64.zip`)}`,
          `    InstallerSha256: ${zeroSha}`,
          "    NestedInstallerType: portable",
          "    NestedInstallerFiles:",
          `      - RelativeFilePath: ${yamlStr(`${bin}.exe`)}`,
          `        PortableCommandAlias: ${yamlStr(bin)}`,
          "ManifestType: installer",
          `ManifestVersion: ${WINGET_MANIFEST_VERSION}`,
          "",
        ].join("\n"),
        [`${wgId}.locale.en-US.yaml`]: [
          `PackageIdentifier: ${wgId}`,
          `PackageVersion: ${yamlStr(ver)}`,
          "PackageLocale: en-US",
          `Publisher: ${yamlStr(entry.publisher)}`,
          `PublisherUrl: ${yamlStr(entry.homepage)}`,
          `PackageName: ${yamlStr(entry.name)}`,
          `PackageUrl: ${yamlStr(entry.repo)}`,
          `License: ${yamlStr(entry.license)}`,
          `ShortDescription: ${yamlStr(entry.tagline)}`,
          `Description: ${yamlStr(entry.description)}`,
          `Moniker: ${yamlStr(bin)}`,
          "Tags:",
          ...entry.tags.slice(0, 8).map((t) => `  - ${yamlStr(t)}`),
          "Documentations:",
          "  - DocumentLabel: Documentation",
          `    DocumentUrl: ${yamlStr(entry.homepage)}`,
          "ManifestType: defaultLocale",
          `ManifestVersion: ${WINGET_MANIFEST_VERSION}`,
          "",
        ].join("\n"),
      };

    case "msstore":
      return {
        "AppxManifest.xml": `<?xml version="1.0" encoding="utf-8"?>
<Package
  xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
  xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
  xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"
  IgnorableNamespaces="uap rescap">
  <Identity
    Name="${xmlEscape(entry.app_id)}"
    Publisher="CN=${xmlEscape(entry.publisher)}"
    Version="${xmlEscape(msixVer)}"
    ProcessorArchitecture="x64" />
  <Properties>
    <DisplayName>${xmlEscape(entry.name)}</DisplayName>
    <PublisherDisplayName>${xmlEscape(entry.publisher)}</PublisherDisplayName>
    <Logo>Assets\\StoreLogo.png</Logo>
    <Description>${xmlEscape(entry.description)}</Description>
  </Properties>
  <Dependencies>
    <TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.19041.0" MaxVersionTested="10.0.26100.0" />
  </Dependencies>
  <Resources>
    <Resource Language="en-us" />
  </Resources>
  <Applications>
    <Application
      Id="${xmlEscape(pascalIdentifier(bin))}"
      Executable="${xmlEscape(`${bin}.exe`)}"
      EntryPoint="Windows.FullTrustApplication">
      <uap:VisualElements
        DisplayName="${xmlEscape(entry.name)}"
        Description="${xmlEscape(entry.tagline)}"
        BackgroundColor="transparent"
        Square150x150Logo="Assets\\Square150x150Logo.png"
        Square44x44Logo="Assets\\Square44x44Logo.png">
        <uap:DefaultTile Wide310x150Logo="Assets\\Wide310x150Logo.png" />
      </uap:VisualElements>
    </Application>
  </Applications>
  <Capabilities>
    <rescap:Capability Name="runFullTrust" />
  </Capabilities>
</Package>
`,
      };

    case "flatpak":
      return {
        [`${entry.app_id}.yml`]: [
          `id: ${yamlStr(entry.app_id)}`,
          "runtime: org.freedesktop.Platform",
          'runtime-version: "24.08"',
          "sdk: org.freedesktop.Sdk",
          "sdk-extensions:",
          "  - org.freedesktop.Sdk.Extension.rust-stable",
          "  - org.freedesktop.Sdk.Extension.llvm19",
          `command: ${yamlStr(bin)}`,
          "finish-args:",
          "  - --share=ipc",
          "  - --socket=fallback-x11",
          "  - --socket=wayland",
          "  - --device=dri",
          "  - --filesystem=home",
          "  - --share=network",
          "modules:",
          `  - name: ${yamlStr(bin)}`,
          "    buildsystem: simple",
          "    build-options:",
          "      append-path: /usr/lib/sdk/rust-stable/bin:/usr/lib/sdk/llvm19/bin",
          "      env:",
          `        CARGO_HOME: /run/build/${bin}/cargo`,
          "    build-commands:",
          `      - cargo build --release --locked --package ${entry.crate_name}`,
          `      - install -Dm755 target/release/${bin} /app/bin/${bin}`,
          `      - install -Dm644 ${entry.app_id}.metainfo.xml /app/share/metainfo/${entry.app_id}.metainfo.xml`,
          "    sources:",
          "      - type: git",
          `        url: ${yamlStr(`${entry.repo}.git`)}`,
          `        tag: ${yamlStr(`v${ver}`)}`,
          "",
        ].join("\n"),
        [`${entry.app_id}.metainfo.xml`]: `<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>${xmlEscape(entry.app_id)}</id>
  <metadata_license>CC0-1.0</metadata_license>
  <project_license>${xmlEscape(entry.license)}</project_license>
  <name>${xmlEscape(entry.name)}</name>
  <summary>${xmlEscape(entry.tagline)}</summary>
  <description>
    <p>${xmlEscape(entry.description)}</p>
  </description>
  <launchable type="desktop-id">${xmlEscape(`${entry.app_id}.desktop`)}</launchable>
  <url type="homepage">${xmlEscape(entry.homepage)}</url>
  <url type="vcs-browser">${xmlEscape(entry.repo)}</url>
  <developer id="${xmlEscape(entry.app_id)}">
    <name>${xmlEscape(entry.publisher)}</name>
  </developer>
  <releases>
    <release version="${xmlEscape(ver)}" date="2026-11-16" />
  </releases>
  <content_rating type="oars-1.1" />
</component>
`,
      };

    case "playstore":
      return {
        "AndroidManifest.xml": `<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="${xmlEscape(androidPkg)}"
    android:versionCode="1"
    android:versionName="${xmlEscape(ver)}">
    <uses-sdk android:minSdkVersion="26" android:targetSdkVersion="35" />
    <uses-feature android:glEsVersion="0x00030002" android:required="true" />
    <uses-feature android:name="android.hardware.vulkan.version" android:version="0x00401000" android:required="false" />
    <uses-permission android:name="android.permission.INTERNET" />
    <application
        android:label="${xmlEscape(entry.name)}"
        android:hasCode="true"
        android:hardwareAccelerated="true">
        <activity
            android:name="android.app.NativeActivity"
            android:exported="true"
            android:configChanges="orientation|keyboardHidden|screenSize|screenLayout|smallestScreenSize|uiMode">
            <meta-data android:name="android.app.lib_name" android:value="${xmlEscape(androidLib)}" />
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
`,
        "build.gradle.kts": `plugins {
    id("com.android.application")
}

android {
    namespace = "${androidPkg}"
    compileSdk = 35
    ndkVersion = "27.2.12479018"

    defaultConfig {
        applicationId = "${androidPkg}"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "${ver}"
        ndk {
            abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64")
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
        }
    }
}

val buildCargoNdk by tasks.registering(Exec::class) {
    commandLine(
        "cargo", "ndk",
        "-t", "arm64-v8a",
        "-t", "armeabi-v7a",
        "-t", "x86_64",
        "-o", "src/main/jniLibs",
        "build", "--release", "--package", "${entry.crate_name}"
    )
}

tasks.named("preBuild") {
    dependsOn(buildCargoNdk)
}
`,
      };

    case "appstore":
      return {
        "Info.plist": `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleDisplayName</key>
  <string>${xmlEscape(entry.name)}</string>
  <key>CFBundleExecutable</key>
  <string>${xmlEscape(bin)}</string>
  <key>CFBundleIdentifier</key>
  <string>${xmlEscape(entry.app_id)}</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>${xmlEscape(entry.name)}</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>${xmlEscape(ver)}</string>
  <key>CFBundleVersion</key>
  <string>1</string>
  <key>LSApplicationCategoryType</key>
  <string>public.app-category.developer-tools</string>
  <key>LSMinimumSystemVersion</key>
  <string>14.0</string>
  <key>MinimumOSVersion</key>
  <string>17.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>UIRequiredDeviceCapabilities</key>
  <array>
    <string>arm64</string>
    <string>metal</string>
  </array>
</dict>
</plist>
`,
        [`${pascalIdentifier(bin)}.entitlements`]: `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>com.apple.security.app-sandbox</key>
  <true/>
  <key>com.apple.security.network.client</key>
  <true/>
  <key>com.apple.security.files.user-selected.read-write</key>
  <true/>
  <key>com.apple.security.cs.allow-jit</key>
  <false/>
</dict>
</plist>
`,
        "ExportOptions.plist": `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>method</key>
  <string>app-store-connect</string>
  <key>signingStyle</key>
  <string>automatic</string>
  <key>destination</key>
  <string>export</string>
  <key>stripSwiftSymbols</key>
  <true/>
  <key>uploadSymbols</key>
  <true/>
</dict>
</plist>
`,
      };

    case "snap":
      return {
        "snap/snapcraft.yaml": [
          `name: ${pkgSlug}`,
          "base: core24",
          `version: ${yamlStr(ver)}`,
          `summary: ${yamlStr(snapSummary(entry.tagline, entry.name))}`,
          "description: |",
          `  ${entry.description.replace(/\r?\n/g, " ")}`,
          "grade: stable",
          "confinement: strict",
          `license: ${yamlStr(entry.license)}`,
          "platforms:",
          "  amd64:",
          "  arm64:",
          "apps:",
          `  ${pkgSlug}:`,
          `    command: bin/${bin}`,
          "    plugs:",
          "      - desktop",
          "      - desktop-legacy",
          "      - wayland",
          "      - x11",
          "      - opengl",
          "      - network",
          "      - home",
          "parts:",
          `  ${pkgSlug}:`,
          "    plugin: rust",
          `    source: ${yamlStr(`${entry.repo}.git`)}`,
          "    rust-channel: stable",
          "",
        ].join("\n"),
      };

    case "arch":
      return {
        PKGBUILD: `# Maintainer: ${entry.publisher}
pkgname=${pkgSlug}
pkgver=${archVer}
pkgrel=1
pkgdesc=${JSON.stringify(entry.tagline)}
arch=('x86_64' 'aarch64')
url=${JSON.stringify(entry.homepage)}
license=(${JSON.stringify(entry.license)})
depends=('gcc-libs' 'vulkan-icd-loader' 'wayland' 'libxkbcommon')
makedepends=('cargo' 'git' 'clang' 'lld')
source=("$pkgname-$pkgver.tar.gz::${archiveUrl}")
sha256sums=('SKIP')

prepare() {
  cd "$srcdir/$pkgname-$pkgver"
  export RUSTUP_TOOLCHAIN=stable
  cargo fetch --locked --target "$(rustc -vV | sed -n 's/host: //p')"
}

build() {
  cd "$srcdir/$pkgname-$pkgver"
  export RUSTUP_TOOLCHAIN=stable
  export CARGO_TARGET_DIR=target
  cargo build --frozen --release --package ${entry.crate_name}
}

package() {
  cd "$srcdir/$pkgname-$pkgver"
  install -Dm755 "target/release/${bin}" "$pkgdir/usr/bin/${bin}"
}
`,
        ".SRCINFO": `pkgbase = ${pkgSlug}
\tpkgdesc = ${entry.tagline}
\tpkgver = ${archVer}
\tpkgrel = 1
\turl = ${entry.homepage}
\tarch = x86_64
\tarch = aarch64
\tlicense = ${entry.license}
\tmakedepends = cargo
\tmakedepends = git
\tmakedepends = clang
\tmakedepends = lld
\tdepends = gcc-libs
\tdepends = vulkan-icd-loader
\tdepends = wayland
\tdepends = libxkbcommon
\tsource = ${pkgSlug}-${archVer}.tar.gz::${archiveUrl}
\tsha256sums = SKIP

pkgname = ${pkgSlug}
`,
      };

    case "nix":
      return {
        "flake.nix": `{
  description = ${JSON.stringify(`${entry.name} — ${entry.tagline}`)};

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = ${JSON.stringify(pkgSlug)};
          version = ${JSON.stringify(ver)};
          src = ${
            ghRef
              ? `pkgs.fetchFromGitHub {
            owner = ${JSON.stringify(ghRef.owner)};
            repo = ${JSON.stringify(ghRef.repo)};
            rev = ${JSON.stringify(`v${ver}`)};
            hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
          }`
              : "./."
          };
          cargoLock.lockFile = ./Cargo.lock;
          cargoBuildFlags = [ "--package" ${JSON.stringify(entry.crate_name)} ];
          nativeBuildInputs = with pkgs; [ pkg-config cmake ];
          buildInputs = with pkgs; [
            vulkan-loader
            wayland
            libxkbcommon
            libGL
            fontconfig
          ];
          meta = with pkgs.lib; {
            description = ${JSON.stringify(entry.tagline)};
            homepage = ${JSON.stringify(entry.homepage)};
            mainProgram = ${JSON.stringify(bin)};
            platforms = platforms.linux ++ platforms.darwin;
          };
        };

        apps.default = flake-utils.lib.mkApp {
          drv = self.packages.\${system}.default;
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ self.packages.\${system}.default ];
          packages = with pkgs; [ rustc cargo rustfmt clippy ];
        };
      });
}
`,
      };

    case "cargo":
      return {
        "Cargo.toml": `[package]
name = ${tomlStr(entry.crate_name || pkgSlug)}
version = ${tomlStr(ver)}
edition = "2024"
license = ${tomlStr(entry.license)}
description = ${tomlStr(entry.tagline)}
homepage = ${tomlStr(entry.homepage)}
repository = ${tomlStr(entry.repo)}
keywords = ["gpui", "nsh", "webgpu", "ui"]
categories = ["gui"]

[lib]
crate-type = ["cdylib", "rlib"]

[[bin]]
name = ${tomlStr(bin)}
path = "src/main.rs"

[dependencies]
gpui = "0.2"
nsh-core = "0.5"
serde = { version = "1.0", features = ["derive"] }

[package.metadata.deb]
maintainer = ${tomlStr(entry.publisher)}
section = "utils"
depends = "$auto"
assets = [
  ["target/release/${bin}", "usr/bin/${bin}", "755"],
]

[package.metadata.generate-rpm]
assets = [
  { source = "target/release/${bin}", dest = "/usr/bin/${bin}", mode = "755" },
]

[profile.web]
inherits = "release"
opt-level = "z"
lto = "thin"
codegen-units = 1
panic = "abort"
`,
      };

    case "npm":
      return {
        "package.json": `${JSON.stringify(
          {
            name: entry.npm_package,
            version: ver,
            description: entry.tagline,
            license: entry.license,
            homepage: entry.homepage,
            repository: {
              type: "git",
              url: `git+${entry.repo}.git`,
            },
            type: "module",
            bin: {
              [bin]: `./bin/${bin}.js`,
            },
            exports: {
              ".": "./dist/index.js",
              "./wasm": "./pkg/index.js",
              "./nsh": "./src/App.tsx",
            },
            files: ["dist/", "pkg/", "bin/", "nsh.json"],
            scripts: {
              "build:wasm": `wasm-pack build --target web --out-dir pkg -- --package ${entry.crate_name}`,
              "dev:nsh": "nsh dev src/App.tsx",
            },
            keywords: entry.tags,
            engines: {
              node: ">=20",
              bun: ">=1.2",
            },
            nsh: {
              appId: entry.app_id,
              entry: "src/App.tsx",
              targets: ["web", "desktop", "mobile"],
            },
          },
          null,
          2,
        )}\n`,
      };

    case "chrome":
      return {
        "manifest.json": `${JSON.stringify(
          {
            manifest_version: 3,
            name: entry.name.slice(0, 45),
            version: ver,
            description: entry.tagline.slice(0, 132),
            homepage_url: entry.homepage,
            action: {
              default_title: entry.name,
              default_popup: "popup.html",
            },
            background: {
              service_worker: "service_worker.js",
              type: "module",
            },
            permissions: ["storage", "activeTab"],
            content_security_policy: {
              extension_pages: "script-src 'self' 'wasm-unsafe-eval'; object-src 'self'",
            },
            web_accessible_resources: [
              {
                resources: ["pkg/*.wasm", "pkg/*.js"],
                matches: ["<all_urls>"],
              },
            ],
          },
          null,
          2,
        )}\n`,
        "popup.html": `<!doctype html>
<html lang="en" dir="auto">
<head>
  <meta charset="utf-8" />
  <title>${xmlEscape(entry.name)}</title>
  <style>body{margin:0;width:420px;height:540px;background:#0b1020;color:#e2e8f0;font-family:system-ui,sans-serif;}canvas{width:100%;height:100%;display:block;}</style>
</head>
<body>
  <canvas id="gpui-canvas" aria-label="${xmlEscape(entry.name)} WebGPU Surface"></canvas>
  <script type="module" src="./host.js"></script>
</body>
</html>
`,
        "host.js": `export async function mountWebGpuExtension(canvasId = "gpui-canvas") {
  const canvas = document.getElementById(canvasId);
  const hasWebGpu = typeof navigator !== "undefined" && "gpu" in navigator;
  if (!canvas || !hasWebGpu) return { mounted: false, webgpu: hasWebGpu };
  const adapter = await navigator.gpu.requestAdapter();
  return { mounted: Boolean(adapter), appId: ${JSON.stringify(entry.app_id)}, wasmTarget: "wasm32-unknown-unknown" };
}
mountWebGpuExtension();
`,
        "service_worker.js": `chrome.runtime.onInstalled.addListener(() => {
  console.info("Mounted GPUI WebGPU extension", ${JSON.stringify(entry.app_id)});
});
`,
      };

    case "apk":
      return {
        APKBUILD: `# Maintainer: ${entry.publisher}
pkgname=${pkgSlug}
pkgver=${archVer}
pkgrel=0
pkgdesc=${JSON.stringify(entry.tagline)}
url=${JSON.stringify(entry.homepage)}
arch="x86_64 aarch64"
license=${JSON.stringify(entry.license)}
depends="vulkan-loader wayland-libs-client libxkbcommon fontconfig"
makedepends="cargo clang lld wayland-dev libxkbcommon-dev vulkan-loader-dev fontconfig-dev"
source="$pkgname-$pkgver.tar.gz::${archiveUrl}"
builddir="$srcdir/$pkgname-$pkgver"

prepare() {
\tdefault_prepare
\tcargo fetch --locked
}

build() {
\tcargo build --release --frozen --package ${entry.crate_name}
}

check() {
\tcargo test --release --frozen --package ${entry.crate_name} --lib
}

package() {
\tinstall -Dm755 "target/release/${bin}" "$pkgdir"/usr/bin/${bin}
}
`,
      };

    case "rpm":
      return {
        [`${pkgSlug}.spec`]: `Name:           ${pkgSlug}
Version:        ${archVer}
Release:        1%{?dist}
Summary:        ${entry.tagline}
License:        ${entry.license}
URL:            ${entry.homepage}
Source0:        ${archiveUrl}

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  clang
BuildRequires:  lld
BuildRequires:  wayland-devel
BuildRequires:  libxkbcommon-devel
BuildRequires:  vulkan-loader-devel
Requires:       vulkan-loader
Requires:       wayland
Requires:       libxkbcommon

%description
${entry.description}

%prep
%autosetup -n %{name}-%{version}

%build
cargo build --release --locked --package ${entry.crate_name}

%install
install -Dm0755 target/release/${bin} %{buildroot}%{_bindir}/${bin}

%files
%{_bindir}/${bin}

%changelog
* Mon Nov 16 2026 ${entry.publisher} - ${archVer}-1
- Initial GPUI Store RPM package for ${entry.name}
`,
        "rpm-metadata.toml": `[package]
name = ${tomlStr(entry.crate_name || pkgSlug)}
version = ${tomlStr(ver)}
license = ${tomlStr(entry.license)}
description = ${tomlStr(entry.tagline)}

[package.metadata.generate-rpm]
assets = [
  { source = "target/release/${bin}", dest = "/usr/bin/${bin}", mode = "755" },
]
`,
      };

    case "deb":
      return {
        "DEBIAN/control": `Package: ${pkgSlug}
Version: ${ver}-1
Section: utils
Priority: optional
Architecture: amd64
Maintainer: ${entry.publisher}
Homepage: ${entry.homepage}
Depends: libc6 (>= 2.34), libwayland-client0, libxkbcommon0, libvulkan1
Description: ${entry.tagline}
 ${entry.description.replace(/\r?\n/g, " ")}
`,
        "deb-metadata.toml": `[package]
name = ${tomlStr(entry.crate_name || pkgSlug)}
version = ${tomlStr(ver)}
license = ${tomlStr(entry.license)}
description = ${tomlStr(entry.tagline)}

[package.metadata.deb]
maintainer = ${tomlStr(entry.publisher)}
section = "utils"
depends = "$auto"
assets = [
  ["target/release/${bin}", "usr/bin/${bin}", "755"],
]
`,
      };

    case "xbps":
      return {
        [`srcpkgs/${pkgSlug}/template`]: `# Template file for '${pkgSlug}'
pkgname=${pkgSlug}
version=${archVer}
revision=1
build_style=cargo
configure_args="--package ${entry.crate_name}"
hostmakedepends="pkg-config clang lld"
makedepends="wayland-devel libxkbcommon-devel vulkan-loader-devel fontconfig-devel"
short_desc=${JSON.stringify(entry.tagline)}
maintainer=${JSON.stringify(entry.publisher)}
license=${JSON.stringify(entry.license)}
homepage=${JSON.stringify(entry.homepage)}
distfiles="${archiveUrl}"
checksum="${zeroSha}"

post_install() {
\tvbin "target/\${RUST_TARGET}/release/${bin}" "${bin}"
}
`,
      };

    case "gentoo":
      return {
        [`${pkgSlug}-${archVer}.ebuild`]: `# Copyright 2026 ${entry.publisher}
# Distributed under the terms of the ${entry.license} license

EAPI=8

inherit cargo

DESCRIPTION=${JSON.stringify(entry.tagline)}
HOMEPAGE=${JSON.stringify(entry.homepage)}
SRC_URI="${archiveUrl} -> \${P}.tar.gz"

LICENSE=${JSON.stringify(entry.license)}
SLOT="0"
KEYWORDS="~amd64 ~arm64"

RDEPEND="
\tdev-libs/wayland
\tx11-libs/libxkbcommon
\tmedia-libs/vulkan-loader
\tmedia-libs/fontconfig
"
DEPEND="\${RDEPEND}"
BDEPEND="
\tvirtual/rust
\tsys-devel/clang
"

src_compile() {
\tcargo_src_compile --package ${entry.crate_name}
}

src_install() {
\tdobin "target/release/${bin}"
}
`,
      };

    case "freebsd":
      return {
        Makefile: `PORTNAME=\t${pkgSlug}
DISTVERSIONPREFIX=\tv
DISTVERSION=\t${ver}
CATEGORIES=\tx11 devel
MAINTAINER=\tports@aphrody.com
COMMENT=\t${entry.tagline}
WWW=\t\t${entry.homepage}

LICENSE=\t${entry.license}

LIB_DEPENDS=\tlibvulkan.so:graphics/vulkan-loader \\
\t\tlibwayland-client.so:graphics/wayland \\
\t\tlibxkbcommon.so:x11/libxkbcommon \\
\t\tlibfontconfig.so:x11-fonts/fontconfig

USES=\t\tcargo pkgconfig
USE_GITHUB=\tyes
GH_ACCOUNT=\t${ghRef?.owner ?? entry.publisher_id.toLowerCase()}
GH_PROJECT=\t${ghRef?.repo ?? pkgSlug}
CARGO_BUILD_ARGS=\t--package ${entry.crate_name}

.include <bsd.port.mk>
`,
        "pkg-plist": `bin/${bin}
share/applications/${entry.app_id}.desktop
`,
      };

    case "brew":
      return {
        [`Formula/${pkgSlug}.rb`]: `class ${pascalIdentifier(pkgSlug)} < Formula
  desc ${JSON.stringify(entry.tagline)}
  homepage ${JSON.stringify(entry.homepage)}
  url ${JSON.stringify(archiveUrl)}
  version ${JSON.stringify(ver)}
  license ${JSON.stringify(entry.license)}

  depends_on "rust" => :build
  depends_on "pkg-config" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: "."), "--package", ${JSON.stringify(entry.crate_name)}
  end

  test do
    assert_match ${JSON.stringify(ver)}, shell_output("#{bin}/${bin} --version")
  end
end
`,
      };

    case "scoop":
      return {
        [`bucket/${pkgSlug}.json`]: `${JSON.stringify(
          {
            version: ver,
            description: entry.tagline,
            homepage: entry.homepage,
            license: entry.license,
            architecture: {
              "64bit": {
                url: `${releaseBase}/${bin}-windows-x64.zip`,
                hash: zeroSha,
              },
              arm64: {
                url: `${releaseBase}/${bin}-windows-arm64.zip`,
                hash: zeroSha,
              },
            },
            bin: `${bin}.exe`,
            checkver: ghRef ? { github: entry.repo } : undefined,
            autoupdate: {
              architecture: {
                "64bit": {
                  url: `${entry.repo}/releases/download/v$version/${bin}-windows-x64.zip`,
                },
                arm64: {
                  url: `${entry.repo}/releases/download/v$version/${bin}-windows-arm64.zip`,
                },
              },
            },
          },
          null,
          2,
        )}\n`,
      };

    case "choco":
      return {
        [`${pkgSlug}.nuspec`]: `<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://schemas.microsoft.com/packaging/2015/06/nuspec.xsd">
  <metadata>
    <id>${xmlEscape(pkgSlug)}</id>
    <version>${xmlEscape(ver)}</version>
    <title>${xmlEscape(entry.name)}</title>
    <authors>${xmlEscape(entry.publisher)}</authors>
    <projectUrl>${xmlEscape(entry.homepage)}</projectUrl>
    <licenseUrl>${xmlEscape(`${entry.repo}/blob/main/LICENSE`)}</licenseUrl>
    <tags>${xmlEscape(entry.tags.join(" "))}</tags>
    <summary>${xmlEscape(entry.tagline)}</summary>
    <description>${xmlEscape(entry.description)}</description>
  </metadata>
  <files>
    <file src="tools\\**" target="tools" />
  </files>
</package>
`,
        "tools/chocolateyinstall.ps1": `$ErrorActionPreference = 'Stop'
$packageName = '${pkgSlug}'
$toolsDir = "$(Split-Path -Parent $MyInvocation.MyCommand.Definition)"
$url64 = '${releaseBase}/${bin}-windows-x64.zip'

Install-ChocolateyZipPackage \`
  -PackageName $packageName \`
  -Url64bit $url64 \`
  -UnzipLocation $toolsDir \`
  -Checksum64 '${zeroSha}' \`
  -ChecksumType64 'sha256'
`,
      };

    case "appimage":
      return {
        "AppImageBuilder.yml": [
          "version: 1",
          "AppDir:",
          "  path: ./AppDir",
          "  app_info:",
          `    id: ${yamlStr(entry.app_id)}`,
          `    name: ${yamlStr(entry.name)}`,
          `    icon: ${yamlStr(pkgSlug)}`,
          `    version: ${yamlStr(ver)}`,
          `    exec: ${yamlStr(`usr/bin/${bin}`)}`,
          '    exec_args: "$@"',
          "  apt:",
          "    arch: amd64",
          "    allow_unauthenticated: true",
          "    sources:",
          '      - sourceline: "deb http://archive.ubuntu.com/ubuntu/ noble main universe"',
          "    include:",
          "      - libwayland-client0",
          "      - libxkbcommon0",
          "      - libvulkan1",
          "      - libfontconfig1",
          "AppImage:",
          "  arch: x86_64",
          `  file_name: ${yamlStr(`${pascalIdentifier(entry.name)}-${ver}-x86_64.AppImage`)}`,
          "",
        ].join("\n"),
      };

    case "oci":
      return {
        Dockerfile: `FROM rust:1.86-slim AS builder
WORKDIR /src
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config clang lld libwayland-dev libxkbcommon-dev libvulkan-dev libfontconfig1-dev && rm -rf /var/lib/apt/lists/*
COPY . .
RUN cargo build --release --locked --package ${entry.crate_name}

FROM gcr.io/distroless/cc-debian12 AS runtime
LABEL org.opencontainers.image.title=${JSON.stringify(entry.name)}
LABEL org.opencontainers.image.description=${JSON.stringify(entry.tagline)}
LABEL org.opencontainers.image.version=${JSON.stringify(ver)}
LABEL org.opencontainers.image.licenses=${JSON.stringify(entry.license)}
LABEL org.opencontainers.image.source=${JSON.stringify(entry.repo)}
LABEL org.opencontainers.image.url=${JSON.stringify(entry.homepage)}
COPY --from=builder /src/target/release/${bin} /usr/local/bin/${bin}
ENTRYPOINT ["/usr/local/bin/${bin}"]
`,
      };
  }
}

export function packageEntry(
  entry: StoreEntry,
  targets: readonly StoreTarget[],
  outDir: string,
): { entryId: string; writtenFiles: string[] } {
  const writtenFiles: string[] = [];
  for (const target of targets) {
    const files = generateTargetManifests(entry, target);
    for (const [relPath, content] of Object.entries(files)) {
      const fullPath = join(outDir, entry.id, target, relPath);
      mkdirSync(resolve(fullPath, ".."), { recursive: true });
      writeFileSync(fullPath, content, "utf8");
      writtenFiles.push(fullPath);
    }
  }
  return { entryId: entry.id, writtenFiles };
}

export function validateXmlWellFormed(xml: string, label: string): string[] {
  const errors: string[] = [];
  const stripped = xml
    .replace(/^\uFEFF/, "")
    .replace(/<\?xml[\s\S]*?\?>/g, "")
    .replace(/<!DOCTYPE[\s\S]*?>/gi, "")
    .replace(/<!--[\s\S]*?-->/g, "");
  const stack: string[] = [];
  const tagRe = /<\/?([a-zA-Z_][\w:.-]*)((?:\s+[\w:.-]+\s*=\s*(?:"[^"]*"|'[^']*'))*)\s*(\/?)>/g;
  let lastIndex = 0;

  for (const match of stripped.matchAll(tagRe)) {
    const idx = match.index ?? 0;
    const textBetween = stripped.slice(lastIndex, idx);
    if (textBetween.includes("<") || textBetween.includes(">")) {
      errors.push(`${label}: stray angle bracket in text near "${textBetween.trim().slice(0, 40)}"`);
    }
    if (/&(?!amp;|lt;|gt;|quot;|apos;|#\d+;|#x[0-9a-fA-F]+;)/.test(textBetween)) {
      errors.push(`${label}: unescaped '&' in XML text`);
    }
    lastIndex = idx + match[0].length;
    const full = match[0];
    const tag = match[1];
    const selfClosing = match[3] === "/";
    if (full.startsWith("</")) {
      const top = stack.pop();
      if (top !== tag) {
        errors.push(`${label}: mismatched closing tag </${tag}> (expected </${top ?? "EOF"}>)`);
      }
    } else if (!selfClosing) {
      stack.push(tag);
    }
  }
  const trailing = stripped.slice(lastIndex).trim();
  if (trailing.length > 0) {
    errors.push(`${label}: trailing non-tag content "${trailing.slice(0, 40)}"`);
  }
  if (stack.length > 0) {
    errors.push(`${label}: unclosed XML tags: ${stack.join(", ")}`);
  }
  if (!xml.includes("<")) {
    errors.push(`${label}: empty XML document`);
  }
  return errors;
}

function collectFiles(dir: string): string[] {
  const out: string[] = [];
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    const st = statSync(full);
    if (st.isDirectory()) {
      out.push(...collectFiles(full));
    } else {
      out.push(full);
    }
  }
  return out;
}

function checkBalancedBraces(source: string, label: string): string[] {
  const errors: string[] = [];
  let braces = 0;
  let brackets = 0;
  let inStr = false;
  let escape = false;
  for (let i = 0; i < source.length; i++) {
    const ch = source[i];
    if (escape) {
      escape = false;
      continue;
    }
    if (inStr) {
      if (ch === "\\") escape = true;
      else if (ch === '"') inStr = false;
      continue;
    }
    if (ch === '"') inStr = true;
    else if (ch === "{") braces++;
    else if (ch === "}") braces--;
    else if (ch === "[") brackets++;
    else if (ch === "]") brackets--;
    if (braces < 0 || brackets < 0) {
      errors.push(`${label}: unexpected closing delimiter at offset ${i}`);
      break;
    }
  }
  if (inStr) errors.push(`${label}: unterminated string literal`);
  if (braces !== 0) errors.push(`${label}: unbalanced '{}' (${braces})`);
  if (brackets !== 0) errors.push(`${label}: unbalanced '[]' (${brackets})`);
  return errors;
}

export function verifyPackagedDir(dir: string): VerifyReport {
  if (!existsSync(dir)) {
    return {
      ok: false,
      checkedFiles: 0,
      targetsChecked: [],
      errors: [`Directory does not exist: ${dir}`],
    };
  }

  const files = collectFiles(dir);
  const errors: string[] = [];
  const targets = new Set<string>();

  for (const file of files) {
    const rel = relative(dir, file).replace(/\\/g, "/");
    const segs = rel.split("/");
    if (segs.length >= 2) targets.add(segs[1]);
    const content = readFileSync(file, "utf8");
    const name = basename(file);

    if (name.endsWith(".json")) {
      try {
        const obj = JSON.parse(content) as Record<string, unknown>;
        if (name === "manifest.json") {
          if (obj.manifest_version !== 3) errors.push(`${rel}: manifest_version must be 3`);
          if (!/^\d+(\.\d+){1,3}$/.test(String(obj.version ?? ""))) {
            errors.push(`${rel}: Chrome MV3 version must be dotted integers`);
          }
          const csp = String(
            (obj.content_security_policy as Record<string, unknown> | undefined)?.extension_pages ??
              "",
          );
          if (!csp.includes("'wasm-unsafe-eval'")) {
            errors.push(`${rel}: Chrome MV3 CSP must include 'wasm-unsafe-eval' for WebGPU Wasm`);
          }
        } else if (name === "package.json") {
          if (!obj.name || !obj.version || !obj.exports || !obj.nsh) {
            errors.push(`${rel}: npm package.json missing name/version/exports/nsh`);
          }
        } else if (rel.includes("/scoop/")) {
          if (!obj.version || !obj.architecture || !obj.bin) {
            errors.push(`${rel}: Scoop manifest missing version/architecture/bin`);
          }
        }
      } catch (err) {
        errors.push(`${rel}: invalid JSON (${String(err)})`);
      }
    } else if (name.endsWith(".yaml") || name.endsWith(".yml")) {
      try {
        const parsed = Bun.YAML.parse(content) as Record<string, unknown>;
        if (!parsed || typeof parsed !== "object") {
          errors.push(`${rel}: YAML root must be a mapping`);
        } else if (rel.includes("/winget/")) {
          if (!parsed.PackageIdentifier || !parsed.PackageVersion || !parsed.ManifestVersion) {
            errors.push(`${rel}: WinGet YAML missing PackageIdentifier/PackageVersion/ManifestVersion`);
          }
        } else if (rel.includes("/flatpak/")) {
          if (!parsed.id || !parsed.runtime || !parsed.sdk || !parsed.command || !parsed.modules) {
            errors.push(`${rel}: Flatpak YAML missing id/runtime/sdk/command/modules`);
          }
        } else if (name === "snapcraft.yaml") {
          if (!parsed.name || !parsed.base || !parsed.apps || !parsed.parts) {
            errors.push(`${rel}: snapcraft.yaml missing name/base/apps/parts`);
          }
          if (String(parsed.summary ?? "").length > 79) {
            errors.push(`${rel}: snapcraft summary exceeds 79 chars`);
          }
        } else if (name === "AppImageBuilder.yml") {
          if (parsed.version !== 1 || !parsed.AppDir || !parsed.AppImage) {
            errors.push(`${rel}: AppImageBuilder.yml missing version/AppDir/AppImage`);
          }
        }
      } catch (err) {
        errors.push(`${rel}: invalid YAML (${String(err)})`);
      }
    } else if (name.endsWith(".toml")) {
      try {
        const parsed = Bun.TOML.parse(content) as Record<string, unknown>;
        const pkg = parsed.package as Record<string, unknown> | undefined;
        if (!pkg?.name || !pkg?.version) {
          errors.push(`${rel}: TOML missing [package] name/version`);
        }
        if (name === "Cargo.toml" && !pkg?.edition) {
          errors.push(`${rel}: Cargo.toml missing [package].edition`);
        }
      } catch (err) {
        errors.push(`${rel}: invalid TOML (${String(err)})`);
      }
    } else if (
      name.endsWith(".xml") ||
      name.endsWith(".plist") ||
      name.endsWith(".entitlements") ||
      name.endsWith(".nuspec")
    ) {
      errors.push(...validateXmlWellFormed(content, rel));
      if (name === "AppxManifest.xml" && !content.includes("<Identity")) {
        errors.push(`${rel}: AppxManifest.xml missing <Identity`);
      }
      if (name === "AndroidManifest.xml" && !content.includes("<application")) {
        errors.push(`${rel}: AndroidManifest.xml missing <application`);
      }
      if (name.endsWith(".metainfo.xml") && !content.includes("<metadata_license>")) {
        errors.push(`${rel}: metainfo.xml missing <metadata_license>`);
      }
      if (name.endsWith(".nuspec") && (!content.includes("<metadata>") || !content.includes("<id>"))) {
        errors.push(`${rel}: .nuspec missing <metadata>/<id>`);
      }
    } else if (name === "PKGBUILD") {
      for (const req of ["pkgname=", "pkgver=", "pkgrel=", "arch=(", "build()", "package()"]) {
        if (!content.includes(req)) errors.push(`${rel}: PKGBUILD missing ${req}`);
      }
    } else if (name === ".SRCINFO") {
      for (const req of ["pkgbase = ", "pkgver = ", "pkgname = "]) {
        if (!content.includes(req)) errors.push(`${rel}: .SRCINFO missing ${req}`);
      }
    } else if (name === "APKBUILD") {
      for (const req of ["pkgname=", "pkgver=", "pkgrel=", "arch=", "build()", "package()"]) {
        if (!content.includes(req)) errors.push(`${rel}: APKBUILD missing ${req}`);
      }
    } else if (name.endsWith(".spec")) {
      for (const req of ["Name:", "Version:", "Release:", "Summary:", "%build", "%install", "%files"]) {
        if (!content.includes(req)) errors.push(`${rel}: RPM spec missing ${req}`);
      }
    } else if (name === "control" && rel.includes("/DEBIAN/")) {
      for (const req of ["Package:", "Version:", "Architecture:", "Maintainer:", "Description:"]) {
        if (!content.includes(req)) errors.push(`${rel}: DEBIAN/control missing ${req}`);
      }
    } else if (name === "template" && rel.includes("/srcpkgs/")) {
      for (const req of ["pkgname=", "version=", "revision=", "build_style=cargo", "short_desc="]) {
        if (!content.includes(req)) errors.push(`${rel}: xbps template missing ${req}`);
      }
    } else if (name.endsWith(".ebuild")) {
      for (const req of ["EAPI=8", "DESCRIPTION=", "LICENSE=", "KEYWORDS=", "src_compile()", "src_install()"]) {
        if (!content.includes(req)) errors.push(`${rel}: Gentoo ebuild missing ${req}`);
      }
    } else if (name === "Makefile" && rel.includes("/freebsd/")) {
      for (const req of ["PORTNAME=", "DISTVERSION=", "USES=", "bsd.port.mk"]) {
        if (!content.includes(req)) errors.push(`${rel}: FreeBSD Makefile missing ${req}`);
      }
    } else if (name === "pkg-plist") {
      if (!content.includes("bin/")) errors.push(`${rel}: pkg-plist missing bin/ entry`);
    } else if (name.endsWith(".rb") && rel.includes("/Formula/")) {
      for (const req of ["< Formula", "def install", "std_cargo_args", "test do"]) {
        if (!content.includes(req)) errors.push(`${rel}: Homebrew formula missing ${req}`);
      }
    } else if (name === "Dockerfile") {
      for (const req of ["FROM ", "LABEL org.opencontainers.image.title=", "ENTRYPOINT "]) {
        if (!content.includes(req)) errors.push(`${rel}: Dockerfile missing ${req}`);
      }
    } else if (name === "flake.nix") {
      errors.push(...checkBalancedBraces(content, rel));
      for (const req of ["inputs", "outputs", "rustPlatform.buildRustPackage"]) {
        if (!content.includes(req)) errors.push(`${rel}: flake.nix missing ${req}`);
      }
    } else if (name === "build.gradle.kts") {
      errors.push(...checkBalancedBraces(content, rel));
      if (!content.includes("com.android.application") || !content.includes("cargo")) {
        errors.push(`${rel}: build.gradle.kts missing Android plugin or cargo ndk task`);
      }
    } else if (name.endsWith(".html")) {
      if (!content.toLowerCase().includes("<!doctype html>")) {
        errors.push(`${rel}: missing doctype`);
      }
    } else if (name.endsWith(".js") || name.endsWith(".ps1")) {
      if (content.trim().length === 0) errors.push(`${rel}: empty script file`);
    }
  }

  return {
    ok: errors.length === 0 && files.length > 0,
    checkedFiles: files.length,
    targetsChecked: Array.from(targets).sort(),
    errors,
  };
}

function getOpt(args: string[], flag: string): string | undefined {
  const idx = args.indexOf(flag);
  if (idx >= 0) return args[idx + 1];
  const eq = args.find((a) => a.startsWith(`${flag}=`));
  return eq ? eq.slice(flag.length + 1) : undefined;
}

if (import.meta.main) {
  const args = Bun.argv.slice(2);
  const cmd = args[0] ?? "list";

  if (cmd === "sync") {
    const out = resolve(repoRoot, getOpt(args, "--out") ?? "assets/os/awesome_gpui_store.toml");
    const useGh = !args.includes("--no-gh");
    const catalog = await syncCatalog(out, useGh);
    const byCategory: Record<string, number> = {};
    for (const e of catalog.entries) {
      byCategory[e.category] = (byCategory[e.category] ?? 0) + 1;
    }
    console.log(
      JSON.stringify(
        {
          synced: true,
          path: out,
          source_repo: catalog.source_repo,
          upstream_stars: catalog.upstream_stars,
          total_entries: catalog.total_entries,
          by_category: byCategory,
        },
        null,
        2,
      ),
    );
  } else if (cmd === "list") {
    const catalog = loadStoreCatalog();
    const catFilter = getOpt(args, "--category");
    const targetFilter = getOpt(args, "--target") as StoreTarget | undefined;
    const filtered = catalog.entries.filter((e) => {
      if (catFilter && e.category !== catFilter && e.subcategory.toLowerCase() !== catFilter.toLowerCase()) {
        return false;
      }
      if (targetFilter && !e.targets.includes(targetFilter)) return false;
      return true;
    });
    const byCategory: Record<string, number> = {};
    const byTarget: Record<string, number> = {};
    for (const e of filtered) {
      byCategory[e.category] = (byCategory[e.category] ?? 0) + 1;
      for (const t of e.targets) byTarget[t] = (byTarget[t] ?? 0) + 1;
    }
    console.log(
      JSON.stringify(
        {
          source_repo: catalog.source_repo,
          synced_at: catalog.synced_at,
          count: filtered.length,
          by_category: byCategory,
          by_target: byTarget,
          entries: filtered.map((e) => ({
            id: e.id,
            name: e.name,
            category: e.category,
            subcategory: e.subcategory,
            stars: e.stars,
            license: e.license,
            targets: e.targets,
          })),
        },
        null,
        2,
      ),
    );
  } else if (cmd === "package") {
    const catalog = loadStoreCatalog();
    const idArg = getOpt(args, "--id") ?? "aphrody";
    const targetArg = getOpt(args, "--target") ?? "all";
    const outDir = resolve(repoRoot, getOpt(args, "--out") ?? "target/gpui-store-dist");

    const targets: StoreTarget[] =
      targetArg === "all"
        ? [...ALL_TARGETS]
        : (targetArg.split(",").map((s) => s.trim()) as StoreTarget[]);

    for (const t of targets) {
      if (!ALL_TARGETS.includes(t)) {
        console.error(`Unknown target "${t}". Expected one of: all, ${ALL_TARGETS.join(", ")}`);
        process.exit(1);
      }
    }

    const selected =
      idArg === "all"
        ? catalog.entries
        : catalog.entries.filter((e) => e.id === idArg || e.name.toLowerCase() === idArg.toLowerCase());
    if (selected.length === 0) {
      console.error(`No catalog entry matching --id "${idArg}"`);
      process.exit(1);
    }

    const packaged = selected.map((entry) => packageEntry(entry, targets, outDir));
    console.log(
      JSON.stringify(
        {
          outDir,
          targets,
          entries: packaged.map((p) => ({
            id: p.entryId,
            files: p.writtenFiles.length,
          })),
        },
        null,
        2,
      ),
    );
  } else if (cmd === "sync-extensions") {
    const out = resolve(repoRoot, getOpt(args, "--out") ?? "assets/os/zed_extensions_catalog.toml");
    const extCat = await syncExtensionsCatalog(out);
    console.log(
      JSON.stringify(
        {
          synced: true,
          path: out,
          source_repo: extCat.source_repo,
          total_extensions: extCat.total_extensions,
          total_submodules: extCat.total_submodules,
          category_counts: extCat.category_counts,
        },
        null,
        2,
      ),
    );
  } else if (cmd === "sync-winget") {
    const out = resolve(repoRoot, getOpt(args, "--out") ?? "assets/os/winget_upstream_baseline.toml");
    const baseline = await syncWingetBaseline(out);
    console.log(JSON.stringify({ synced: true, path: out, baseline }, null, 2));
  } else if (cmd === "sync-all") {
    const useGh = !args.includes("--no-gh");
    const [storeCat, extCat, wingetBaseline] = await Promise.all([
      syncCatalog(DEFAULT_CATALOG_PATH, useGh),
      syncExtensionsCatalog(DEFAULT_EXTENSIONS_CATALOG_PATH),
      syncWingetBaseline(DEFAULT_WINGET_BASELINE_PATH),
    ]);
    console.log(
      JSON.stringify(
        {
          synced: true,
          store_entries: storeCat.total_entries,
          extensions_total: extCat.total_extensions,
          winget_latest: wingetBaseline.latest_version,
          winget_manifest_version: wingetBaseline.manifest_version,
        },
        null,
        2,
      ),
    );
  } else if (cmd === "verify") {
    const dir = resolve(repoRoot, getOpt(args, "--dir") ?? "target/gpui-store-dist");
    const report = verifyPackagedDir(dir);
    console.log(JSON.stringify(report, null, 2));
    if (!report.ok) process.exit(1);
  } else {
    console.error(
      "Usage: bun run tooling/bun/awesome-gpui-store.ts <sync|sync-extensions|sync-winget|sync-all|list|package|verify> [options]",
    );
    process.exit(1);
  }
}
