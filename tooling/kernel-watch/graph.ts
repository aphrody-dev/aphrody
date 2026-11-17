#!/usr/bin/env bun
// Structure reports of the kernel repositories (phase B). Heavy: run on the build host, never on a developer machine.
//   bun graph.ts clone  --root DIR         blob-less shallow clones of every [[clone]] in sources.toml
//   bun graph.ts report --root DIR --out DIR   one report (Markdown and JSON) per kernel
// The reports are structure and dependency level: tree, languages, subsystem inventory, Kconfig/Cargo/CMake dependency graphs,
// enforced style. A symbol-level code graph of these trees (millions of nodes) is not attempted.
import { existsSync, mkdirSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { loadManifest, type Clone } from "./manifest.ts";

const HERE = import.meta.dir;

const run = (cwd: string, cmd: string[], env: Record<string, string> = {}): { ok: boolean; out: string; err: string } => {
  const p = Bun.spawnSync(cmd, { cwd, stdout: "pipe", stderr: "pipe", env: { ...Bun.env, ...env } });
  return { ok: p.exitCode === 0, out: p.stdout.toString(), err: p.stderr.toString() };
};

export function cloneAll(root: string, clones: Clone[]): number {
  mkdirSync(root, { recursive: true });
  let failed = 0;
  for (const c of clones) {
    const dir = join(root, c.id);
    const steps = existsSync(join(dir, ".git"))
      ? [["git", "fetch", "--depth", "1", "--filter=blob:none", "origin", c.branch], ["git", "checkout", "--quiet", "--detach", "FETCH_HEAD"]]
      : [["git", "clone", "--depth", "1", "--filter=blob:none", "--single-branch", "--branch", c.branch, c.url, dir]];
    for (const step of steps) {
      const r = run(existsSync(dir) ? dir : root, step);
      if (!r.ok) {
        console.error(`${c.id}: ${step.slice(0, 2).join(" ")} failed: ${r.err.trim().split("\n").pop()}`);
        failed++;
        break;
      }
    }
    console.log(`${c.id}: ${existsSync(dir) ? run(dir, ["git", "rev-parse", "--short=12", "HEAD"]).out.trim() : "missing"}`);
  }
  return failed;
}

// ---- generic tree scan ----------------------------------------------------------------------------------------
const LANGUAGES: Record<string, string> = {
  c: "C", h: "C", cc: "C++", cpp: "C++", cxx: "C++", hpp: "C++", hh: "C++", rs: "Rust", S: "Assembly", s: "Assembly", asm: "Assembly", inc: "Assembly",
  py: "Python", sh: "Shell", pl: "Perl", go: "Go", js: "JavaScript", ts: "TypeScript", java: "Java", cs: "C#", dts: "Device tree", dtsi: "Device tree",
  rst: "Documentation", md: "Documentation", txt: "Text", toml: "TOML", json: "JSON", yaml: "YAML", yml: "YAML", cmake: "CMake", mk: "Make", idl: "IDL", rc: "Resource",
};
const CODE = new Set(["C", "C++", "Rust", "Assembly", "Python", "Shell", "Perl", "Go", "JavaScript", "TypeScript", "Java", "C#", "Device tree", "CMake", "Make", "IDL"]);
const languageOf = (path: string): string => {
  const name = basename(path);
  if (name === "Makefile" || name === "GNUmakefile") return "Make";
  if (name === "CMakeLists.txt") return "CMake";
  if (/^Kconfig/.test(name)) return "Kconfig";
  return LANGUAGES[name.includes(".") ? name.slice(name.lastIndexOf(".") + 1) : ""] ?? "Other";
};

export interface TreeRow {
  name: string;
  files: number;
  bytes: number;
  lines: number;
  topLanguage: string;
}
export interface Tree {
  files: number;
  bytes: number;
  lines: number;
  top: TreeRow[];
  languages: { language: string; files: number; bytes: number; lines: number }[];
}

async function mapLimit<T, R>(items: T[], limit: number, fn: (item: T) => Promise<R>): Promise<R[]> {
  const out: R[] = new Array(items.length);
  let next = 0;
  await Promise.all(Array.from({ length: limit }, async () => { for (let i = next++; i < items.length; i = next++) out[i] = await fn(items[i]!); }));
  return out;
}
function countLines(bytes: Uint8Array): number {
  let n = 0;
  for (let i = 0; i < bytes.length; i++) if (bytes[i] === 10) n++;
  return n;
}

export async function scanTree(dir: string, files: string[]): Promise<Tree> {
  const measured = await mapLimit(files, 64, async (path) => {
    const file = Bun.file(join(dir, path));
    const language = languageOf(path);
    const size = file.size;
    const lines = CODE.has(language) || language === "Kconfig" ? countLines(new Uint8Array(await file.arrayBuffer())) : 0;
    return { path, language, size, lines };
  });
  const top = new Map<string, TreeRow & { byLanguage: Map<string, number> }>();
  const languages = new Map<string, { files: number; bytes: number; lines: number }>();
  for (const m of measured) {
    const name = m.path.includes("/") ? `${m.path.slice(0, m.path.indexOf("/"))}/` : "(root files)";
    const row = top.get(name) ?? { name, files: 0, bytes: 0, lines: 0, topLanguage: "", byLanguage: new Map() };
    row.files++;
    row.bytes += m.size;
    row.lines += m.lines;
    row.byLanguage.set(m.language, (row.byLanguage.get(m.language) ?? 0) + m.size);
    top.set(name, row);
    const lang = languages.get(m.language) ?? { files: 0, bytes: 0, lines: 0 };
    lang.files++;
    lang.bytes += m.size;
    lang.lines += m.lines;
    languages.set(m.language, lang);
  }
  const rows = [...top.values()].map(({ byLanguage, ...row }) => ({ ...row, topLanguage: [...byLanguage].filter(([l]) => l !== "Other").sort((a, b) => b[1] - a[1] || (a[0] < b[0] ? -1 : 1))[0]?.[0] ?? "Other" }));
  return {
    files: measured.length,
    bytes: measured.reduce((n, m) => n + m.size, 0),
    lines: measured.reduce((n, m) => n + m.lines, 0),
    top: rows.sort((a, b) => b.bytes - a.bytes || (a.name < b.name ? -1 : 1)),
    languages: [...languages].map(([language, v]) => ({ language, ...v })).sort((a, b) => b.bytes - a.bytes || (a.language < b.language ? -1 : 1)),
  };
}

// ---- style --------------------------------------------------------------------------------------------------
const STYLE_FILES = [".clang-format", ".clang-tidy", ".editorconfig", "rustfmt.toml", ".rustfmt.toml", "clippy.toml", ".clippy.toml", "deny.toml", "rust-toolchain.toml", "rust-toolchain", ".cargo/config.toml", ".pre-commit-config.yaml", ".pylintrc", ".cmake-format", ".gitattributes", "scripts/checkpatch.pl", "CONTRIBUTING.md", "CODEOWNERS", ".github/CODEOWNERS", "Documentation/process/coding-style.rst", "Documentation/rust/coding-guidelines.rst", "CODE_OF_CONDUCT.md"];
const CLANG_KEYS = ["BasedOnStyle", "ColumnLimit", "IndentWidth", "TabWidth", "UseTab", "BreakBeforeBraces", "AllowShortIfStatementsOnASingleLine", "AlignAfterOpenBracket", "SortIncludes"];

export async function styleOf(dir: string, files: Set<string>): Promise<Record<string, unknown>> {
  const present = STYLE_FILES.filter((f) => files.has(f));
  const out: Record<string, unknown> = { present };
  const read = async (path: string): Promise<string> => (files.has(path) ? Bun.file(join(dir, path)).text() : "");
  const clang = await read(".clang-format");
  if (clang) out.clangFormat = Object.fromEntries(CLANG_KEYS.flatMap((k) => { const m = new RegExp(`^${k}:\\s*(.+)$`, "m").exec(clang); return m ? [[k, m[1]!.trim()]] : []; }));
  for (const name of ["rustfmt.toml", ".rustfmt.toml", "clippy.toml", ".clippy.toml", "rust-toolchain.toml", "deny.toml"]) {
    const text = await read(name);
    if (text) try { out[name] = Bun.TOML.parse(text); } catch { out[name] = "unparseable"; }
  }
  const editor = await read(".editorconfig");
  if (editor) {
    const sections: Record<string, Record<string, string>> = {};
    let current = "*";
    for (const line of editor.split("\n")) {
      const header = /^\[(.+)\]$/.exec(line.trim());
      if (header) current = header[1]!;
      else if (/^[a-z_]+\s*=/.test(line.trim())) (sections[current] ??= {})[line.split("=")[0]!.trim()] = line.split("=").slice(1).join("=").trim();
    }
    out.editorconfig = sections;
  }
  const checkpatch = await read("scripts/checkpatch.pl");
  if (checkpatch) out.checkpatch = { maxLineLength: /\$max_line_length\s*=\s*(\d+)/.exec(checkpatch)?.[1] ?? null, strictOption: /--strict/.test(checkpatch) };
  const cargo = await read("Cargo.toml");
  if (cargo) try {
    const parsed = Bun.TOML.parse(cargo) as any;
    if (parsed.workspace?.lints) out.workspaceLints = parsed.workspace.lints;
    if (parsed.lints) out.lints = parsed.lints;
  } catch {}
  const workflows = [...files].filter((f) => /^\.github\/workflows\/[^/]+\.ya?ml$/.test(f));
  if (workflows.length) out.workflows = workflows.map((w) => basename(w)).sort();
  return out;
}

// ---- Linux: drivers, MAINTAINERS, Kconfig and Makefile graph ---------------------------------------------------
export interface Subsystem {
  name: string;
  status: string;
  lists: number;
  maintainers: number;
  patterns: number;
  trees: string[];
}
export function parseMaintainers(text: string): Subsystem[] {
  const start = text.indexOf("\n-----------------------------------\n");
  const out: Subsystem[] = [];
  for (const block of (start >= 0 ? text.slice(start) : text).split(/\n\s*\n/)) {
    const lines = block.split("\n").map((l) => l.replace(/\s+$/, "")).filter((l) => l && !/^-+$/.test(l));
    const title = lines[0];
    if (!title || /^[A-Z]:\t/.test(title)) continue;
    const tagged = lines.slice(1).filter((l) => /^[A-Z]:\t/.test(l));
    if (!tagged.length) continue;
    const all = (tag: string): string[] => tagged.filter((l) => l[0] === tag).map((l) => l.slice(3).trim());
    out.push({ name: title.trim(), status: all("S")[0] ?? "", lists: all("L").length, maintainers: all("M").length, patterns: all("F").length, trees: all("T") });
  }
  return out;
}

const groupOfDir = (dir: string): string => {
  const parts = dir.split("/").filter(Boolean);
  if (parts.length === 0) return "(root)";
  return ["drivers", "arch", "sound", "net", "fs", "security", "crypto", "tools"].includes(parts[0]!) && parts.length > 1 ? `${parts[0]}/${parts[1]}` : parts[0]!;
};

export interface Edge {
  from: string;
  to: string;
  kind: string;
  weight: number;
}
export async function kconfigGraph(dir: string, files: string[]): Promise<{ symbols: number; kconfigFiles: number; edges: Edge[]; makeEdges: Edge[]; configSymbolsByGroup: [string, number][] }> {
  const kconfigs = files.filter((f) => /(^|\/)Kconfig[^/]*$/.test(f));
  const owner = new Map<string, string>();
  const parsed: { group: string; lines: string[] }[] = [];
  for (const file of kconfigs) {
    const group = groupOfDir(dirname(file) === "." ? "" : dirname(file));
    const lines = (await Bun.file(join(dir, file)).text()).split("\n");
    parsed.push({ group, lines });
    for (const line of lines) {
      const m = /^\s*(?:menu)?config\s+([A-Za-z0-9_]+)/.exec(line);
      if (m && !owner.has(m[1]!)) owner.set(m[1]!, group);
    }
  }
  const edges = new Map<string, Edge>();
  const bump = (map: Map<string, Edge>, from: string, to: string, kind: string): void => {
    const key = `${from}\u0000${to}\u0000${kind}`;
    const e = map.get(key) ?? { from, to, kind, weight: 0 };
    e.weight++;
    map.set(key, e);
  };
  for (const { group, lines } of parsed) {
    for (const line of lines) {
      const sel = /^\s*(select|imply)\s+([A-Za-z0-9_]+)/.exec(line);
      if (sel) {
        const target = owner.get(sel[2]!);
        if (target && target !== group) bump(edges, group, target, sel[1]!);
        continue;
      }
      const dep = /^\s*depends on\s+(.+)$/.exec(line);
      if (dep) for (const sym of new Set(dep[1]!.match(/[A-Z][A-Z0-9_]{2,}/g) ?? [])) {
        const target = owner.get(sym);
        if (target && target !== group) bump(edges, group, target, "depends");
      }
    }
  }
  const makeEdges = new Map<string, Edge>();
  for (const file of files.filter((f) => /(^|\/)(Makefile|Kbuild)$/.test(f))) {
    const base = dirname(file) === "." ? "" : dirname(file);
    const from = groupOfDir(base);
    for (const line of (await Bun.file(join(dir, file)).text()).split("\n")) {
      if (!/^\s*(obj|subdir|lib|core|drivers)-[^\s=]*\s*[:+]?=/.test(line)) continue;
      for (const token of line.split("=").slice(1).join("=").split(/\s+/)) {
        if (token.endsWith("/") && !token.startsWith("$") && !token.startsWith("..")) {
          const to = groupOfDir(`${base}/${token.replace(/\/$/, "")}`);
          if (to !== from) bump(makeEdges, from, to, "obj-subdir");
        }
      }
    }
  }
  const byGroup = new Map<string, number>();
  for (const group of owner.values()) byGroup.set(group, (byGroup.get(group) ?? 0) + 1);
  const heavy = (map: Map<string, Edge>): Edge[] => [...map.values()].sort((a, b) => b.weight - a.weight || (a.from + a.to + a.kind < b.from + b.to + b.kind ? -1 : 1));
  return { symbols: owner.size, kconfigFiles: kconfigs.length, edges: heavy(edges), makeEdges: heavy(makeEdges), configSymbolsByGroup: [...byGroup].sort((a, b) => b[1] - a[1] || (a[0] < b[0] ? -1 : 1)) };
}

const EDGE_CAP = 400;
function capEdges<T extends { edges: Edge[]; makeEdges: Edge[] }>(graph: T): T & { totalEdges: number; totalMakeEdges: number } {
  return { ...graph, totalEdges: graph.edges.length, totalMakeEdges: graph.makeEdges.length, edges: graph.edges.slice(0, EDGE_CAP), makeEdges: graph.makeEdges.slice(0, EDGE_CAP) };
}

export async function driverInventory(dir: string, files: string[]): Promise<{ name: string; files: number; bytes: number; lines: number; configSymbols: number; subdirs: number }[]> {
  const byDriver = new Map<string, { files: string[]; subdirs: Set<string> }>();
  for (const f of files) {
    const m = /^drivers\/([^/]+)\/(.*)$/.exec(f);
    if (!m) continue;
    const entry = byDriver.get(m[1]!) ?? { files: [], subdirs: new Set() };
    entry.files.push(f);
    const sub = m[2]!.split("/");
    if (sub.length > 1) entry.subdirs.add(sub[0]!);
    byDriver.set(m[1]!, entry);
  }
  const rows = await mapLimit([...byDriver], 16, async ([name, entry]) => {
    const tree = await scanTree(dir, entry.files);
    let configSymbols = 0;
    for (const f of entry.files.filter((x) => /(^|\/)Kconfig[^/]*$/.test(x))) configSymbols += ((await Bun.file(join(dir, f)).text()).match(/^\s*(?:menu)?config\s+\w+/gm) ?? []).length;
    return { name, files: tree.files, bytes: tree.bytes, lines: tree.lines, configSymbols, subdirs: entry.subdirs.size };
  });
  return rows.sort((a, b) => b.lines - a.lines || (a.name < b.name ? -1 : 1));
}

// ---- Rust: cargo metadata ------------------------------------------------------------------------------------
export interface CargoReport {
  workspaceMembers: { name: string; version: string; edition: string; path: string; group: string; workspaceDeps: string[]; externalDeps: number; features: number }[];
  internalEdges: [string, string][];
  topExternal: { name: string; usedBy: number }[];
  resolvedPackages: number;
  error?: string;
}
export function cargoReport(dir: string): CargoReport {
  // The toolchain file of the repository pins a nightly that need not be installed; metadata only needs a recent cargo.
  const r = run(dir, ["cargo", "metadata", "--format-version", "1", "--locked"], { RUSTUP_TOOLCHAIN: Bun.env.WATCH_CARGO_TOOLCHAIN ?? "" });
  const retry = r.ok ? r : run(dir, ["cargo", "metadata", "--format-version", "1"], { RUSTUP_TOOLCHAIN: Bun.env.WATCH_CARGO_TOOLCHAIN ?? "" });
  if (!retry.ok) return { workspaceMembers: [], internalEdges: [], topExternal: [], resolvedPackages: 0, error: retry.err.trim().split("\n").slice(-2).join(" ") };
  const meta = JSON.parse(retry.out);
  const members = new Set<string>(meta.workspace_members);
  const byId = new Map<string, any>(meta.packages.map((p: any) => [p.id, p]));
  const root = String(meta.workspace_root).replace(/\\/g, "/");
  const names = new Map([...members].map((id) => [id, byId.get(id).name as string]));
  const internal: [string, string][] = [];
  const external = new Map<string, Set<string>>();
  const rows = [...members].map((id) => {
    const pkg = byId.get(id);
    const deps = pkg.dependencies.filter((d: any) => d.kind !== "dev");
    const workspaceDeps = deps.map((d: any) => d.name).filter((n: string) => [...names.values()].includes(n));
    for (const n of workspaceDeps) internal.push([pkg.name, n]);
    for (const d of deps) if (!workspaceDeps.includes(d.name)) (external.get(d.name) ?? external.set(d.name, new Set()).get(d.name)!).add(pkg.name);
    const path = String(pkg.manifest_path).replace(/\\/g, "/").replace(`${root}/`, "").replace(/\/?Cargo\.toml$/, "") || ".";
    return { name: pkg.name, version: pkg.version, edition: pkg.edition, path, group: path.split("/").slice(0, path.startsWith("kernel/") ? 2 : 1).join("/"), workspaceDeps: [...new Set<string>(workspaceDeps)].sort(), externalDeps: deps.length - workspaceDeps.length, features: Object.keys(pkg.features ?? {}).length };
  });
  rows.sort((a, b) => (a.path < b.path ? -1 : 1));
  return {
    workspaceMembers: rows,
    internalEdges: internal.sort((a, b) => (a.join() < b.join() ? -1 : 1)),
    topExternal: [...external].map(([name, set]) => ({ name, usedBy: set.size })).sort((a, b) => b.usedBy - a.usedBy || (a.name < b.name ? -1 : 1)).slice(0, 30),
    resolvedPackages: meta.packages.length,
  };
}

// ---- CMake (NT components) ---------------------------------------------------------------------------------------
export async function cmakeReport(dir: string, files: string[]): Promise<{ modules: number; moduleTypes: [string, number][]; modulesByTop: [string, number][]; mostLinked: { module: string; linkedBy: number }[] }> {
  const types = new Map<string, number>();
  const byTop = new Map<string, number>();
  const linkedBy = new Map<string, Set<string>>();
  const owners = new Set<string>();
  for (const file of files.filter((f) => basename(f) === "CMakeLists.txt")) {
    const text = await Bun.file(join(dir, file)).text();
    const top = file.includes("/") ? file.slice(0, file.indexOf("/")) : "(root)";
    for (const m of text.matchAll(/^\s*set_module_type\(\s*([A-Za-z0-9_.+-]+)\s+([A-Za-z0-9_]+)/gm)) {
      types.set(m[2]!, (types.get(m[2]!) ?? 0) + 1);
      byTop.set(top, (byTop.get(top) ?? 0) + 1);
      owners.add(m[1]!);
    }
    for (const m of text.matchAll(/^\s*(?:add_importlibs|target_link_libraries|add_dependencies)\(\s*([A-Za-z0-9_.+-]+)\s+([^)]*)\)/gm)) {
      for (const dep of m[2]!.split(/\s+/).filter((t) => /^[A-Za-z0-9_.+-]+$/.test(t) && !["PRIVATE", "PUBLIC", "INTERFACE", "delay"].includes(t))) (linkedBy.get(dep) ?? linkedBy.set(dep, new Set()).get(dep)!).add(m[1]!);
    }
  }
  const sorted = <K>(m: Map<K, number>): [K, number][] => [...m].sort((a, b) => b[1] - a[1] || (String(a[0]) < String(b[0]) ? -1 : 1));
  return {
    modules: owners.size,
    moduleTypes: sorted(types),
    modulesByTop: sorted(byTop),
    mostLinked: [...linkedBy].filter(([name]) => owners.has(name)).map(([module, set]) => ({ module, linkedBy: set.size })).sort((a, b) => b.linkedBy - a.linkedBy || (a.module < b.module ? -1 : 1)).slice(0, 30),
  };
}

// ---- Asterinas: syscall coverage (System Call Matching Language files of the book) ---------------------------------
export interface SyscallCategory {
  category: string;
  scmlFiles: number;
  fullyCovered: number;
  flagLevel: number;
}
export async function syscallCoverage(dir: string, files: string[]): Promise<{ categories: SyscallCategory[]; syscalls: number; fullyCovered: number }> {
  const root = "book/src/kernel/linux-compatibility/syscall-flag-coverage/";
  const names = (text: string): string[] => [...text.matchAll(/^([a-z_][a-z0-9_]*)\(/gm)].map((m) => m[1]!);
  const byCategory = new Map<string, { files: number; full: Set<string>; flags: Set<string> }>();
  for (const file of files.filter((f) => f.startsWith(root) && f.endsWith(".scml"))) {
    const category = file.slice(root.length).split("/")[0]!;
    const entry = byCategory.get(category) ?? { files: 0, full: new Set(), flags: new Set() };
    entry.files++;
    for (const name of names(await Bun.file(join(dir, file)).text())) (basename(file) === "fully_covered.scml" ? entry.full : entry.flags).add(name);
    byCategory.set(category, entry);
  }
  const categories = [...byCategory].map(([category, e]) => ({ category, scmlFiles: e.files, fullyCovered: e.full.size, flagLevel: e.flags.size })).sort((a, b) => (a.category < b.category ? -1 : 1));
  return { categories, syscalls: categories.reduce((n, c) => n + c.fullyCovered + c.flagLevel, 0), fullyCovered: categories.reduce((n, c) => n + c.fullyCovered, 0) };
}

// ---- report -------------------------------------------------------------------------------------------------------
export interface RepoReport {
  id: string;
  url: string;
  branch: string;
  commit: string;
  commitDate: string;
  tree: Tree;
  style: Record<string, unknown>;
  linux?: { maintainers: Subsystem[]; drivers: Awaited<ReturnType<typeof driverInventory>>; kconfig: ReturnType<typeof capEdges<Awaited<ReturnType<typeof kconfigGraph>>>> };
  cargo?: CargoReport;
  cmake?: Awaited<ReturnType<typeof cmakeReport>>;
  syscalls?: Awaited<ReturnType<typeof syscallCoverage>>;
}

export async function analyze(root: string, clone: Clone): Promise<RepoReport> {
  const dir = join(root, clone.id);
  const files = run(dir, ["git", "ls-files", "-z"]).out.split("\0").filter(Boolean);
  const set = new Set(files);
  const commit = run(dir, ["git", "rev-parse", "HEAD"]).out.trim();
  const commitDate = run(dir, ["git", "log", "-1", "--format=%cI"]).out.trim();
  const report: RepoReport = { id: clone.id, url: clone.url, branch: clone.branch, commit, commitDate, tree: await scanTree(dir, files), style: await styleOf(dir, set) };
  if (set.has("MAINTAINERS") && set.has("Kconfig")) {
    report.linux = {
      maintainers: parseMaintainers(await Bun.file(join(dir, "MAINTAINERS")).text()),
      drivers: await driverInventory(dir, files),
      kconfig: capEdges(await kconfigGraph(dir, files)),
    };
  }
  if (set.has("Cargo.toml")) report.cargo = cargoReport(dir);
  if (files.some((f) => basename(f) === "CMakeLists.txt") && !set.has("Cargo.toml")) report.cmake = await cmakeReport(dir, files);
  if (clone.kernel === "asterinas") report.syscalls = await syscallCoverage(dir, files);
  return report;
}

const mib = (n: number): string => `${(n / 1048576).toFixed(1)} MiB`;
const num = (n: number): string => n.toLocaleString("en-US");
const table = (head: string[], rows: (string | number)[][]): string[] => [`| ${head.join(" | ")} |`, `| ${head.map(() => ":--").join(" | ")} |`, ...rows.map((r) => `| ${r.join(" | ")} |`)];

function commitLink(r: RepoReport): string {
  const base = r.url.replace(/\.git$/, "");
  const short = r.commit.slice(0, 12);
  if (r.url.includes("github.com")) return `[${short}](${base}/commit/${r.commit})`;
  if (r.url.includes("gitlab")) return `[${short}](${base}/-/commit/${r.commit})`;
  return `${short} (${r.url})`;
}

export function renderRepo(r: RepoReport): string {
  const l: string[] = [`## ${r.id}`, "", `Built from ${commitLink(r)} on \`${r.branch}\`, committed ${r.commitDate}. ${num(r.tree.files)} tracked files, ${mib(r.tree.bytes)}, ${num(r.tree.lines)} lines of code.`, ""];
  l.push("### Top-level tree", "", ...table(["Entry", "Files", "Size", "Code lines", "Main language"], r.tree.top.slice(0, 40).map((t) => [`\`${t.name}\``, num(t.files), mib(t.bytes), num(t.lines), t.topLanguage])), "");
  l.push("### Languages", "", ...table(["Language", "Files", "Size", "Code lines"], r.tree.languages.slice(0, 14).map((t) => [t.language, num(t.files), mib(t.bytes), num(t.lines)])), "");
  if (r.linux) {
    const { maintainers, drivers, kconfig } = r.linux;
    const status = new Map<string, number>();
    for (const s of maintainers) status.set(s.status.split(/\s+/)[0] || "unspecified", (status.get(s.status.split(/\s+/)[0] || "unspecified") ?? 0) + 1);
    l.push("### MAINTAINERS subsystems", "", `${num(maintainers.length)} subsystems; status ${[...status].sort((a, b) => b[1] - a[1]).map(([k, v]) => `${k} ${v}`).join(", ")}. Maintainer names are not copied here.`, "");
    l.push(...table(["Subsystem", "Status", "Maintainers", "Lists", "File patterns"], [...maintainers].sort((a, b) => b.patterns - a.patterns || (a.name < b.name ? -1 : 1)).slice(0, 25).map((s) => [s.name.replace(/\|/g, "\\|"), s.status.split(/\s+/)[0] ?? "", s.maintainers, s.lists, s.patterns])), "");
    l.push("### drivers/", "", `${drivers.length} directories under drivers/, largest by code lines (full list in the JSON).`, "", ...table(["Directory", "Files", "Code lines", "Kconfig symbols", "Subdirs"], drivers.slice(0, 40).map((d) => [`\`drivers/${d.name}\``, num(d.files), num(d.lines), d.configSymbols, d.subdirs])), "");
    l.push("### Kconfig dependency graph", "", `${num(kconfig.symbols)} config symbols in ${num(kconfig.kconfigFiles)} Kconfig files, grouped by directory (two levels under arch, drivers, fs, net, sound, security, crypto, tools). Edges are select, imply and depends-on references that cross groups (${num(kconfig.totalEdges)} distinct edges; the JSON keeps the heaviest ${EDGE_CAP}).`, "");
    l.push(...table(["From", "To", "Kind", "Edges"], kconfig.edges.slice(0, 40).map((e) => [`\`${e.from}\``, `\`${e.to}\``, e.kind, num(e.weight)])), "");
    l.push("### Makefile composition", "", `${num(kconfig.totalMakeEdges)} group-to-group build descents (obj-y and obj-\$(CONFIG_X) of a subdirectory).`, "", ...table(["From", "To", "Descents"], kconfig.makeEdges.slice(0, 25).map((e) => [`\`${e.from}\``, `\`${e.to}\``, e.weight])), "");
  }
  if (r.cargo) {
    const c = r.cargo;
    l.push("### Cargo workspace", "");
    if (c.error) l.push(`cargo metadata failed: ${c.error}`, "");
    else {
      l.push(`${c.workspaceMembers.length} workspace crates, ${num(c.resolvedPackages)} packages in the resolved graph, ${c.internalEdges.length} workspace-internal dependency edges.`, "");
      l.push(...table(["Crate", "Path", "Version", "Edition", "Workspace deps", "External deps"], c.workspaceMembers.map((m) => [`\`${m.name}\``, `\`${m.path}\``, m.version, m.edition, m.workspaceDeps.join(", ") || "-", m.externalDeps])), "");
      l.push("Most used external crates: " + c.topExternal.slice(0, 15).map((e) => `${e.name} (${e.usedBy})`).join(", ") + ".", "");
    }
  }
  if (r.cmake) {
    l.push("### CMake modules (NT components)", "", `${num(r.cmake.modules)} modules declared with set_module_type.`, "", ...table(["Module type", "Modules"], r.cmake.moduleTypes.map(([k, v]) => [k, v])), "", ...table(["Top-level directory", "Modules"], r.cmake.modulesByTop.slice(0, 20).map(([k, v]) => [`\`${k}/\``, v])), "");
    l.push("Most linked modules: " + r.cmake.mostLinked.slice(0, 15).map((m) => `${m.module} (${m.linkedBy})`).join(", ") + ".", "");
  }
  if (r.syscalls) {
    const s = r.syscalls;
    l.push("### Linux syscall coverage", "", `The book documents supported syscalls in System Call Matching Language files: ${s.syscalls} syscalls described (${s.fullyCovered} fully covered without flag-level caveats, ${s.syscalls - s.fullyCovered} with flag-level patterns).`, "");
    l.push(...table(["Category", "SCML files", "Fully covered", "Flag-level"], s.categories.map((c) => [c.category, c.scmlFiles, c.fullyCovered, c.flagLevel])), "");
  }
  const s = r.style as Record<string, any>;
  l.push("### Enforced style", "", `Configuration files present: ${s.present.length ? s.present.map((f: string) => `\`${f}\``).join(", ") : "none"}.`, "");
  if (s.clangFormat) l.push(`- .clang-format: ${Object.entries(s.clangFormat).map(([k, v]) => `${k} ${v}`).join(", ")}`);
  for (const name of ["rustfmt.toml", ".rustfmt.toml", "clippy.toml", ".clippy.toml", "rust-toolchain.toml"]) if (s[name]) l.push(`- ${name}: \`${JSON.stringify(s[name])}\``);
  if (s.checkpatch) l.push(`- scripts/checkpatch.pl: max line length ${s.checkpatch.maxLineLength}, --strict ${s.checkpatch.strictOption ? "available" : "absent"}`);
  if (s.workspaceLints) l.push(`- workspace lints: \`${JSON.stringify(s.workspaceLints)}\``);
  if (s.lints) l.push(`- lints: \`${JSON.stringify(s.lints)}\``);
  if (s.editorconfig) l.push(`- .editorconfig: \`${JSON.stringify(s.editorconfig).slice(0, 400)}\``);
  if (s.workflows) l.push(`- CI workflows: ${s.workflows.join(", ")}`);
  l.push("");
  return l.join("\n");
}

async function main(): Promise<number> {
  const args = Bun.argv.slice(2);
  const flag = (name: string): string | undefined => { const i = args.indexOf(`--${name}`); return i >= 0 ? args[i + 1] : undefined; };
  const manifest = await loadManifest(resolve(flag("manifest") ?? join(HERE, "sources.toml")));
  const root = resolve(flag("root") ?? join(HERE, ".clones"));
  if (args[0] === "clone") return cloneAll(root, manifest.clones) === 0 ? 0 : 1;
  if (args[0] === "report") {
    const out = resolve(flag("out") ?? join(HERE, "..", "..", "docs", "watch", "graph"));
    mkdirSync(out, { recursive: true });
    for (const kernel of manifest.kernels.filter((k) => manifest.clones.some((c) => c.kernel === k.id))) {
      const repos: RepoReport[] = [];
      for (const clone of manifest.clones.filter((c) => c.kernel === kernel.id)) repos.push(await analyze(root, clone));
      const md = [`# ${kernel.name}: structure report`, "", "Generated on the build host by `tooling/kernel-watch/graph.ts` from blob-less shallow clones; each section names the commit it was built from. Structure and dependency level: tree, languages, subsystem inventory, dependency graph, enforced style. There is no symbol-level code graph.", "", ...repos.map(renderRepo)].join("\n");
      await Bun.write(join(out, `${kernel.id}.md`), `${md}\n`);
      await Bun.write(join(out, `${kernel.id}.json`), `${JSON.stringify({ kernel: kernel.id, repos }, null, 1)}\n`);
      console.log(`${kernel.id}: ${repos.map((r) => `${r.id}@${r.commit.slice(0, 12)}`).join(" ")}`);
    }
    return 0;
  }
  console.error("usage: bun graph.ts clone|report [--root DIR] [--out DIR] [--manifest FILE]");
  return 2;
}

if (import.meta.main) process.exit(await main());
