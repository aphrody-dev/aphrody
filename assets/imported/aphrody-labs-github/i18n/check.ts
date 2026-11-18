#!/usr/bin/env bun
// i18n checker for Aphrody repositories. Rules: i18n.toml, rationale: I18N.md.
//
//   bun i18n/check.ts [dir...] [--fix [--hygiene-only]] [--json out.json] [--fail-on error|warn|none] [--quiet]
//   bun i18n/check.ts rules
//
// Files come from `git ls-files` (tracked plus untracked-not-ignored for checks, tracked only for --fix; no submodule contents) or a directory walk outside git.
import { chmodSync, existsSync, lstatSync, readdirSync, readFileSync, realpathSync, renameSync, rmSync, statSync, writeFileSync } from "node:fs";
import { basename, dirname, extname, join, relative, resolve, sep } from "node:path";

export type Severity = "error" | "warn";
export interface Finding {
  rule: string;
  severity: Severity;
  file: string;
  line?: number;
  message: string;
}
export interface PatternDef {
  id: string;
  exts: string[];
  re: string;
  message: string;
}
export interface Config {
  default_lang: string;
  default_dir: string;
  exclude: string[];
  exclude_content: string[];
  binary_ext: string[];
  crlf_allowed: string[];
  bom_allowed: string[];
  max_bytes: number;
  max_content_bytes: number;
  max_findings_per_rule_per_file: number;
  html_ext: string[];
  code_ext: string[];
  ui_ext: string[];
  catalogs: string[];
  i18n_libraries: string[];
}
export interface Spec {
  config: Config;
  rules: Record<string, { severity: Severity; summary: string; fix?: string }>;
  patterns: { locale: PatternDef[]; ui: PatternDef[] };
  openapi: {
    negotiation_headers: string[];
    natural_language_properties: string[];
    language_properties: string[];
    direction_properties: string[];
    localizable_schemas: string[];
  };
}
export interface RepoResult {
  root: string;
  files: number;
  findings: Finding[];
  counts: Record<string, number>;
  truncated: number;
  fixed?: string[];
}

const here = import.meta.dir;
export const loadSpec = async (path = join(here, "i18n.toml")): Promise<Spec> =>
  Bun.TOML.parse(await Bun.file(path).text()) as unknown as Spec;

const globs = (list: string[]) => list.map((p) => new Bun.Glob(p));
const matchAny = (gs: Bun.Glob[], path: string) => gs.some((g) => g.match(path));
const ext = (path: string) => extname(path).slice(1).toLowerCase();

const EDITORCONFIG = `root = true

[*]
charset = utf-8
end_of_line = lf
insert_final_newline = true
trim_trailing_whitespace = true

[*.{bat,cmd}]
end_of_line = crlf

[*.{ps1,psm1,psd1}]
charset = utf-8-bom

[{Makefile,*.mk}]
indent_style = tab
`;
const GITATTRIBUTES_LINE = "* text=auto eol=lf";
const GITATTRIBUTES_EXTRA = "*.bat text eol=crlf\n*.cmd text eol=crlf\n";

// ---------- file listing ----------

const git = (root: string, args: string[]) =>
  Bun.spawnSync(["git", "-C", root, ...args], { stdout: "pipe", stderr: "pipe" });

const isGitRoot = (root: string) => existsSync(join(root, ".git"));

const WALK_SKIP = new Set([".git", "node_modules", "target", "dist", "build", ".next", "third_party", "vendor"]);
function walk(root: string, dir = root, out: string[] = []): string[] {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.isSymbolicLink() || WALK_SKIP.has(entry.name)) continue;
    const full = join(dir, entry.name);
    if (entry.isDirectory()) walk(root, full, out);
    else if (entry.isFile()) out.push(relative(root, full).replaceAll("\\", "/"));
  }
  return out;
}

export function listFiles(root: string, untracked = false): string[] {
  if (!isGitRoot(root)) return walk(root);
  const res = git(root, ["ls-files", "-z", "-s"]);
  const files: string[] = [];
  if (untracked) for (const path of git(root, ["ls-files", "-z", "--others", "--exclude-standard"]).stdout.toString("utf8").split("\0")) if (path) files.push(path);
  for (const rec of res.stdout.toString("utf8").split("\0")) {
    const tab = rec.indexOf("\t");
    if (tab < 0) continue;
    const mode = rec.slice(0, 6);
    if (mode === "160000" || mode === "120000") continue; // submodule, symlink
    const path = rec.slice(tab + 1);
    if (existsSync(join(root, path))) files.push(path);
  }
  return files;
}

// ---------- text helpers ----------

const lineOf = (text: string, index: number) => {
  let n = 1;
  for (let i = text.indexOf("\n"); i >= 0 && i < index; i = text.indexOf("\n", i + 1)) n++;
  return n;
};

const RTL_SCRIPTS = new Set(["Arab", "Hebr", "Thaa", "Syrc", "Adlm", "Nkoo", "Rohg", "Mand", "Samr"]);
export function directionOf(lang: string): "ltr" | "rtl" | "auto" {
  try {
    const script = new Intl.Locale(lang).maximize().script;
    if (!script) return "auto";
    return RTL_SCRIPTS.has(script) ? "rtl" : "ltr";
  } catch {
    return "auto";
  }
}

export function wellFormedLang(value: string): boolean {
  if (value === "") return true; // empty = unknown, allowed by HTML
  if (value.includes("_") || value !== value.trim()) return false;
  try {
    return Intl.getCanonicalLocales(value).length === 1;
  } catch {
    return false;
  }
}

const dynamic = (value: string) => /[{}$<>%@]/.test(value);

// Decode bytes to text; returns the problem when the bytes are not UTF-8 (or UTF-16 with BOM).
type Decoded =
  | { kind: "binary" }
  | { kind: "text"; text: string; bom: boolean }
  | { kind: "utf16"; text: string }
  | { kind: "invalid"; line: number; text: string };

export function decode(bytes: Uint8Array): Decoded {
  if (bytes.length >= 2 && ((bytes[0] === 0xff && bytes[1] === 0xfe) || (bytes[0] === 0xfe && bytes[1] === 0xff))) {
    const label = bytes[0] === 0xff ? "utf-16le" : "utf-16be";
    return { kind: "utf16", text: new TextDecoder(label).decode(bytes.subarray(2)) };
  }
  const head = bytes.subarray(0, 8000);
  if (head.includes(0)) return { kind: "binary" };
  const bom = bytes.length >= 3 && bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf;
  try {
    const text = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(bytes);
    return { kind: "text", text: bom ? text.slice(1) : text, bom };
  } catch {
    const loose = new TextDecoder("utf-8", { ignoreBOM: true }).decode(bytes);
    return { kind: "invalid", line: lineOf(loose, loose.indexOf("\uFFFD")), text: loose };
  }
}

// ---------- per-file rules ----------

interface Ctx {
  spec: Spec;
  exclude: Bun.Glob[];
  excludeContent: Bun.Glob[];
  crlfAllowed: Bun.Glob[];
  bomAllowed: Bun.Glob[];
  catalogs: Bun.Glob[];
  locale: { def: PatternDef; re: RegExp }[];
  ui: { def: PatternDef; re: RegExp }[];
  binary: Set<string>;
  html: Set<string>;
  code: Set<string>;
  uiExt: Set<string>;
}

export function makeCtx(spec: Spec): Ctx {
  const c = spec.config;
  const compile = (defs: PatternDef[]) => defs.map((def) => ({ def, re: new RegExp(def.re, "g") }));
  return {
    spec,
    exclude: globs(c.exclude),
    excludeContent: globs(c.exclude_content),
    crlfAllowed: globs(c.crlf_allowed),
    bomAllowed: globs(c.bom_allowed),
    catalogs: globs(c.catalogs),
    locale: compile(spec.patterns.locale),
    ui: compile(spec.patterns.ui),
    binary: new Set(c.binary_ext),
    html: new Set(c.html_ext),
    code: new Set(c.code_ext),
    uiExt: new Set(c.ui_ext),
  };
}

const BIDI = /[\u202A-\u202E\u2066-\u2069]/;
const OPENAPI_NAME = /(^|\/)(openapi|swagger)[^/]*\.(json|ya?ml)$|\.openapi\.(json|ya?ml)$/i;

interface FileScan {
  findings: Finding[];
  uiLiterals: number;
}

function openapiFindings(rel: string, text: string, spec: Spec): Finding[] {
  const o = spec.openapi;
  const isDoc = OPENAPI_NAME.test(rel) || /^\s*["']?(openapi|swagger)["']?\s*:\s*["']?[23]/m.test(text);
  if (!isDoc) return [];
  const out: Finding[] = [];
  const lower = text.toLowerCase();
  const missing = o.negotiation_headers.filter((h) => !lower.includes(h.toLowerCase()));
  if (missing.length)
    out.push({ rule: "openapi-negotiation", severity: "error", file: rel, message: `missing ${missing.join(", ")}` });

  let doc: unknown;
  try {
    doc = rel.endsWith(".json") ? JSON.parse(text) : Bun.YAML.parse(text);
  } catch {
    return out;
  }
  const props = new Set<string>();
  const schemas = new Set<string>();
  const visit = (node: unknown, key = "") => {
    if (Array.isArray(node)) return node.forEach((n) => visit(n, key));
    if (!node || typeof node !== "object") return;
    for (const [k, v] of Object.entries(node)) {
      if (k === "properties" && v && typeof v === "object") for (const p of Object.keys(v)) props.add(p);
      if (key === "schemas") schemas.add(k);
      visit(v, k);
    }
  };
  visit(doc);
  const natural = o.natural_language_properties.filter((p) => props.has(p));
  const hasLang = o.language_properties.some((p) => props.has(p));
  const hasDir = o.direction_properties.some((p) => props.has(p));
  const hasType = o.localizable_schemas.some((s) => schemas.has(s));
  if (natural.length && !hasType && !(hasLang && hasDir)) {
    out.push({
      rule: "openapi-string-meta",
      severity: "warn",
      file: rel,
      message: `natural-language fields (${natural.slice(0, 4).join(", ")}) without lang/dir metadata or a Localizable schema`,
    });
  }
  return out;
}

export function scanFile(rel: string, bytes: Uint8Array, ctx: Ctx): FileScan {
  const { spec } = ctx;
  const f: Finding[] = [];
  const add = (rule: string, message: string, line?: number) =>
    f.push({ rule, severity: spec.rules[rule].severity, file: rel, line, message });
  const e = ext(rel);
  const d = decode(bytes);
  if (d.kind === "binary") return { findings: f, uiLiterals: 0 };
  if (d.kind === "utf16") {
    add("utf16", "UTF-16 with BOM");
    return { findings: f, uiLiterals: 0 };
  }
  if (d.kind === "invalid") {
    add("utf8-invalid", "bytes are not valid UTF-8", d.line);
    return { findings: f, uiLiterals: 0 };
  }
  const text = d.text;
  if (d.bom && !matchAny(ctx.bomAllowed, rel)) add("bom", "UTF-8 byte order mark");
  const crlf = text.indexOf("\r\n");
  if (crlf >= 0 && !matchAny(ctx.crlfAllowed, rel)) add("crlf", "CRLF line endings", lineOf(text, crlf));

  const head = text.slice(0, 4096);
  if (e === "css") {
    const m = /^\s*@charset\s+["']([^"']+)["']/i.exec(text);
    if (m && !/^utf-?8$/i.test(m[1])) add("html-charset", `@charset ${m[1]}`, 1);
  }
  if (e === "xml" || e === "xhtml" || e === "svg") {
    const m = /^\s*<\?xml[^>]*encoding\s*=\s*["']([^"']+)["']/i.exec(head);
    if (m && !/^utf-?8$/i.test(m[1])) add("html-charset", `XML declaration encoding ${m[1]}`, 1);
  }

  const contentScope = !matchAny(ctx.excludeContent, rel) && bytes.length <= spec.config.max_content_bytes;
  if (contentScope && ctx.html.has(e)) {
    const root = /<html\b([^>]*)>/i.exec(text);
    if (root) {
      const line = lineOf(text, root.index);
      const attrs = root[1];
      const lang = /\blang\s*=\s*(?:"([^"]*)"|'([^']*)'|\{([^}]*)\}|([^\s>]+))/i.exec(attrs);
      if (!lang) add("html-lang", "<html> has no lang", line);
      else {
        const value = lang[1] ?? lang[2] ?? lang[4];
        if (value !== undefined && !dynamic(value) && !wellFormedLang(value))
          add("html-lang-malformed", `lang="${value}" is not well-formed BCP 47`, line);
      }
      if (!/\bdir\s*=/i.test(attrs)) add("html-dir", "<html> has no dir", line);
    }
    if (["html", "htm", "xhtml"].includes(e) && root && /<head\b/i.test(text)) {
      const meta = /<meta\b[^>]*\bcharset\s*=\s*["']?([\w-]+)/i.exec(text) ?? /<meta\b[^>]*charset=([\w-]+)/i.exec(text);
      if (!meta) add("html-charset-missing", "no <meta charset>");
      else if (!/^utf-?8$/i.test(meta[1])) add("html-charset", `<meta charset=${meta[1]}>`, lineOf(text, meta.index));
      else if (meta.index > 1024) add("html-charset-missing", "<meta charset> beyond the first 1024 bytes", lineOf(text, meta.index));
    }
    const mcl = /<meta\b[^>]*http-equiv\s*=\s*["']?content-language/i.exec(text);
    if (mcl) add("html-meta-content-language", "meta content-language declares text language", lineOf(text, mcl.index));
  }

  let uiLiterals = 0;
  if (contentScope && ctx.code.has(e)) {
    const bidi = BIDI.exec(text);
    if (bidi) add("bidi-control", `bidi control U+${bidi[0].codePointAt(0)!.toString(16).toUpperCase()} in source`, lineOf(text, bidi.index));
    const lines = text.split(/\r?\n/);
    const caps = new Map<string, number>();
    const emit = (rule: string, id: string, message: string, line: number) => {
      const n = (caps.get(rule) ?? 0) + 1;
      caps.set(rule, n);
      if (n <= spec.config.max_findings_per_rule_per_file) add(rule, `${id}: ${message}`, line);
    };
    const inCatalog = matchAny(ctx.catalogs, rel);
    lines.forEach((text, i) => {
      if (text.length > 2000 || text.includes("i18n-ignore")) return;
      for (const { def, re } of ctx.locale) {
        if (!def.exts.includes(e)) continue;
        re.lastIndex = 0;
        if (re.test(text)) emit("locale-hardcode", def.id, def.message, i + 1);
      }
      if (inCatalog || !ctx.uiExt.has(e)) return;
      for (const { def, re } of ctx.ui) {
        if (!def.exts.includes(e)) continue;
        re.lastIndex = 0;
        if (re.test(text)) {
          uiLiterals++;
          emit("ui-string-literal", def.id, def.message, i + 1);
        }
      }
    });
  }
  if (contentScope && (e === "json" || e === "yaml" || e === "yml")) f.push(...openapiFindings(rel, text, spec));
  return { findings: f, uiLiterals };
}

// ---------- repo-level rules ----------

function editorconfigFindings(root: string, files: string[], spec: Spec, ctx: Ctx): Finding[] {
  const out: Finding[] = [];
  const add = (rule: string, message: string, file = ".editorconfig") =>
    out.push({ rule, severity: spec.rules[rule].severity, file, message });
  const path = join(root, ".editorconfig");
  if (!existsSync(path)) {
    add("editorconfig-charset", ".editorconfig is missing");
    return out;
  }
  const bomExts = spec.config.bom_allowed.map((g) => g.split(".").pop()!);
  for (const rel of files.filter((p) => basename(p) === ".editorconfig")) {
    const sections = parseIni(readFileSync(join(root, rel), "utf8"));
    for (const [name, kv] of sections) {
      const charset = kv.get("charset");
      if (!charset || charset === "utf-8") continue;
      const bomOk = charset === "utf-8-bom" && bomExts.some((x) => name.includes(x));
      if (!bomOk) add("editorconfig-charset", `[${name}] sets charset = ${charset}`, rel);
    }
  }
  const star = parseIni(readFileSync(path, "utf8")).find(([n]) => n === "*")?.[1];
  if (star?.get("charset") !== "utf-8") add("editorconfig-charset", "no [*] section with charset = utf-8");
  if (star?.get("end_of_line") !== "lf") add("editorconfig-eol", "no [*] end_of_line = lf");
  return out;
}

function parseIni(text: string): [string, Map<string, string>][] {
  const sections: [string, Map<string, string>][] = [["", new Map()]];
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    if (!line || line.startsWith("#") || line.startsWith(";")) continue;
    const s = /^\[(.+)\]$/.exec(line);
    if (s) sections.push([s[1], new Map()]);
    else {
      const eq = line.indexOf("=");
      if (eq > 0) sections.at(-1)![1].set(line.slice(0, eq).trim().toLowerCase(), line.slice(eq + 1).trim().toLowerCase());
    }
  }
  return sections;
}

function gitattributesFindings(root: string, spec: Spec): Finding[] {
  const path = join(root, ".gitattributes");
  const ok =
    existsSync(path) &&
    readFileSync(path, "utf8")
      .split(/\r?\n/)
      .some((l) => /^\*\s/.test(l) && /\btext=auto\b/.test(l) && /\beol=lf\b/.test(l));
  return ok ? [] : [{ rule: "gitattributes-eol", severity: spec.rules["gitattributes-eol"].severity, file: ".gitattributes", message: `missing "${GITATTRIBUTES_LINE}"` }];
}

function commitEncodingFindings(root: string, spec: Spec): Finding[] {
  if (!isGitRoot(root)) return [];
  const out: Finding[] = [];
  const add = (message: string) =>
    out.push({ rule: "git-commit-encoding", severity: spec.rules["git-commit-encoding"].severity, file: ".git/config", message });
  const cfg = git(root, ["config", "--get", "i18n.commitEncoding"]).stdout.toString().trim();
  if (cfg && !/^utf-?8$/i.test(cfg)) add(`i18n.commitEncoding = ${cfg}`);
  const enc = git(root, ["log", "-n", "5000", "--format=%e"]).stdout.toString().split("\n").filter((l) => l && !/^utf-?8$/i.test(l));
  if (enc.length) add(`${enc.length} commit(s) carry encoding header ${[...new Set(enc)].join(", ")}`);
  return out;
}

export function manifestHasI18n(root: string, files: string[], spec: Spec): boolean {
  const manifests = /(^|\/)(package\.json|pubspec\.yaml|Cargo\.toml|go\.mod|[^/]+\.csproj|build\.gradle(\.kts)?|requirements\.txt|pyproject\.toml)$/;
  const catalogs = globs(spec.config.catalogs);
  if (files.some((p) => matchAny(catalogs, p) && !/\.(rs|ts|js|md)$/.test(p))) return true;
  for (const rel of files.filter((p) => manifests.test(p)).slice(0, 200)) {
    const text = readFileSync(join(root, rel), "utf8");
    if (spec.config.i18n_libraries.some((l) => text.includes(l))) return true;
  }
  return false;
}

// ---------- repo scan ----------

async function readPool<T>(items: string[], size: number, fn: (item: string) => Promise<T>): Promise<T[]> {
  const out: T[] = [];
  for (let i = 0; i < items.length; i += size) out.push(...(await Promise.all(items.slice(i, i + size).map(fn))));
  return out;
}

export async function mergeRepoConfig(spec: Spec, root: string): Promise<Spec> {
  const path = join(root, "i18n.toml");
  if (!existsSync(path)) return spec;
  const { exclude_extra = [], exclude_content_extra = [], ...local } = (Bun.TOML.parse(await Bun.file(path).text()) as { config?: Partial<Config> & { exclude_extra?: string[]; exclude_content_extra?: string[] } }).config ?? {};
  const merged = { ...spec.config, ...local };
  return { ...spec, config: { ...merged, exclude: [...merged.exclude, ...exclude_extra], exclude_content: [...merged.exclude_content, ...exclude_content_extra] } };
}

export async function checkRepo(rootArg: string, baseSpec?: Spec): Promise<RepoResult> {
  const root = resolve(rootArg);
  const spec = await mergeRepoConfig(baseSpec ?? (await loadSpec()), root);
  const ctx = makeCtx(spec);
  const all = listFiles(root, true);
  const files = all.filter((p) => !matchAny(ctx.exclude, p) && !ctx.binary.has(ext(p)));
  const findings: Finding[] = [];
  let uiLiterals = 0;
  const scans = await readPool(files, 64, async (rel) => {
    const file = Bun.file(join(root, rel));
    if (file.size > spec.config.max_bytes) return null;
    return scanFile(rel, new Uint8Array(await file.arrayBuffer()), ctx);
  });
  for (const s of scans) {
    if (!s) continue;
    findings.push(...s.findings);
    uiLiterals += s.uiLiterals;
  }
  findings.push(...editorconfigFindings(root, all.filter((p) => !matchAny(ctx.exclude, p)), spec, ctx), ...gitattributesFindings(root, spec), ...commitEncodingFindings(root, spec));
  if (uiLiterals > 0 && !manifestHasI18n(root, all, spec))
    findings.push({ rule: "no-i18n-catalog", severity: spec.rules["no-i18n-catalog"].severity, file: ".", message: `${uiLiterals} UI literal(s), no i18n library or catalog found` });
  const counts: Record<string, number> = {};
  for (const x of findings) counts[x.rule] = (counts[x.rule] ?? 0) + 1;
  return { root, files: files.length, findings, counts, truncated: 0 };
}

// ---------- fixes ----------

// windows-1252 text uses 0xA0-0xFF for letters; C1 bytes only as typographic punctuation. Multi-byte legacy
// encodings (Shift-JIS, GBK, EUC) hit other C1 bytes or leave lone high bytes, and are not converted blindly.
const CP1252_PUNCT = new Set([0x80, 0x85, 0x91, 0x92, 0x93, 0x94, 0x96, 0x97]);
export function isPlausibleCp1252(bytes: Uint8Array): boolean {
  for (const b of bytes) if (b >= 0x80 && b <= 0x9f && !CP1252_PUNCT.has(b)) return false;
  return true;
}

export function fixBytes(rel: string, bytes: Uint8Array, ctx: Ctx): Uint8Array | null {
  const d = decode(bytes);
  if (d.kind === "binary") return null;
  let text: string;
  if (d.kind === "utf16") text = d.text;
  else if (d.kind === "invalid") {
    if (!isPlausibleCp1252(bytes)) return null; // Shift-JIS, GBK, binary-ish: needs a human choice of encoding
    text = new TextDecoder("windows-1252").decode(bytes);
  } else text = d.text;
  const keepBom = d.kind === "text" && d.bom && matchAny(ctx.bomAllowed, rel);
  if (!matchAny(ctx.crlfAllowed, rel)) text = text.replaceAll("\r\n", "\n");
  const e = ext(rel);
  if (ctx.html.has(e)) text = fixHtml(text, e, ctx.spec.config);
  const body = new TextEncoder().encode(text);
  const out = keepBom ? Uint8Array.from([0xef, 0xbb, 0xbf, ...body]) : body;
  return Buffer.compare(out, bytes) === 0 ? null : out;
}

export function fixHtml(text: string, e: string, cfg: Config): string {
  const root = /<html\b([^>]*)>/i.exec(text);
  if (!root) return text;
  let attrs = root[1];
  const lang = /\blang\s*=\s*(?:"([^"]*)"|'([^']*)'|\{([^}]*)\}|([^\s>]+))/i.exec(attrs);
  const selfClose = attrs.endsWith("/") ? "/" : "";
  const base = selfClose ? attrs.slice(0, -1) : attrs;
  let add = "";
  const langValue = lang ? (lang[1] ?? lang[2] ?? lang[4]) : cfg.default_lang;
  if (!lang) add += ` lang="${cfg.default_lang}"`;
  if (!/\bdir\s*=/i.test(attrs)) {
    if (langValue !== undefined && !dynamic(langValue)) add += ` dir="${langValue === "" ? "auto" : directionOf(langValue)}"`;
  }
  attrs = base + add + selfClose;
  let out = text.slice(0, root.index) + `<html${attrs}>` + text.slice(root.index + root[0].length);
  if (["html", "htm", "xhtml"].includes(e) && !/<meta\b[^>]*charset/i.test(out)) {
    const head = /<head\b[^>]*>/i.exec(out);
    if (head) {
      const at = head.index + head[0].length;
      const nl = out.includes("\r\n") ? "\r\n" : "\n";
      out = out.slice(0, at) + `${nl}    <meta charset="utf-8">` + out.slice(at);
    }
  }
  return out;
}

export function fixEditorconfig(text: string | null): string {
  if (text === null) return EDITORCONFIG;
  const lines = text.replace(/\r\n/g, "\n").split("\n");
  const idx = lines.findIndex((l) => l.trim() === "[*]");
  if (idx < 0) return `${lines.join("\n").trimEnd()}\n\n[*]\ncharset = utf-8\nend_of_line = lf\n`;
  let end = idx + 1;
  while (end < lines.length && !lines[end].trim().startsWith("[")) end++;
  const section = lines.slice(idx + 1, end);
  const set = (key: string, value: string) => {
    const i = section.findIndex((l) => l.trim().toLowerCase().startsWith(`${key} `) || l.trim().toLowerCase().startsWith(`${key}=`));
    if (i >= 0) section[i] = `${key} = ${value}`;
    else section.unshift(`${key} = ${value}`);
  };
  set("end_of_line", "lf");
  set("charset", "utf-8");
  return [...lines.slice(0, idx + 1), ...section, ...lines.slice(end)].join("\n");
}

export function fixGitattributes(text: string | null): string {
  if (text === null) return `${GITATTRIBUTES_LINE}\n${GITATTRIBUTES_EXTRA}`;
  const lines = text.replace(/\r\n/g, "\n").split("\n");
  const keep = lines.filter((l) => !(/^\*\s/.test(l) && /\btext=auto\b/.test(l) && !/\beol=lf\b/.test(l)));
  const hasBat = keep.some((l) => l.startsWith("*.bat") || l.startsWith("*.cmd"));
  return `${GITATTRIBUTES_LINE}\n${hasBat ? "" : GITATTRIBUTES_EXTRA}${keep.join("\n")}`.replace(/\n*$/, "\n");
}

// Writes rel under root only when neither the file nor its directory leaves the root (symlink, junction):
// temp file in the same directory, then rename. Returns false and writes nothing otherwise.
export function safeWrite(root: string, rel: string, data: Uint8Array | string): boolean {
  const path = join(root, rel);
  let mode: number | undefined;
  try {
    if (lstatSync(path).isSymbolicLink()) return false;
    mode = statSync(path).mode;
  } catch (e) {
    if ((e as NodeJS.ErrnoException).code !== "ENOENT") return false;
  }
  try {
    const realRoot = realpathSync(root);
    const realDir = realpathSync(dirname(path));
    if (realDir !== realRoot && !realDir.startsWith(realRoot + sep)) return false;
    const tmp = join(realDir, `.${basename(path)}.i18n-${process.pid}.tmp`);
    try {
      writeFileSync(tmp, data);
      if (mode !== undefined) chmodSync(tmp, mode & 0o7777);
      renameSync(tmp, join(realDir, basename(path)));
    } catch (e) {
      rmSync(tmp, { force: true });
      throw e;
    }
    return true;
  } catch {
    return false;
  }
}

// Paths with uncommitted changes belong to someone else's work in progress; fixes leave them alone.
function dirtyPaths(root: string): Set<string> {
  if (!isGitRoot(root)) return new Set();
  const out = git(root, ["status", "--porcelain", "-z", "--untracked-files=no"]).stdout.toString("utf8").split("\0");
  return new Set(out.filter((r) => r.length > 3).map((r) => r.slice(3)));
}

export async function fixRepo(rootArg: string, baseSpec?: Spec, hygieneOnly = false): Promise<string[]> {
  const root = resolve(rootArg);
  const spec = await mergeRepoConfig(baseSpec ?? (await loadSpec()), root);
  const ctx = makeCtx(spec);
  const changed: string[] = [];
  const all = listFiles(root);
  const dirty = dirtyPaths(root);
  const files = hygieneOnly ? [] : all.filter((p) => !matchAny(ctx.exclude, p) && !ctx.binary.has(ext(p)) && !dirty.has(p));
  await readPool(files, 64, async (rel) => {
    const file = Bun.file(join(root, rel));
    if (file.size > spec.config.max_bytes || file.size === 0) return;
    const bytes = new Uint8Array(await file.arrayBuffer());
    const next = fixBytes(rel, bytes, ctx);
    if (next) {
      if (safeWrite(root, rel, next)) changed.push(rel);
      else console.error(`skipped (symlink or outside the repository): ${rel}`);
    }
  });
  const rewrite = async (rel: string, make: (t: string | null) => string) => {
    if (dirty.has(rel)) return;
    const p = join(root, rel);
    const prev = existsSync(p) && !lstatSync(p).isSymbolicLink() ? await Bun.file(p).text() : null;
    const next = make(prev);
    if (next === prev) return;
    if (safeWrite(root, rel, next)) changed.push(rel);
    else console.error(`skipped (symlink or outside the repository): ${rel}`);
  };
  const ec = editorconfigFindings(root, all.filter((p) => !matchAny(ctx.exclude, p)), spec, ctx).filter((x) => x.file === ".editorconfig");
  if (ec.length) await rewrite(".editorconfig", fixEditorconfig);
  if (gitattributesFindings(root, spec).length) await rewrite(".gitattributes", fixGitattributes);
  return changed.sort();
}

// ---------- CLI ----------

const RANK: Record<string, number> = { none: 3, warn: 1, error: 2 };

function summarize(res: RepoResult, cap: number) {
  const sample = res.findings.slice(0, cap);
  return { ...res, findings: sample, truncated: res.findings.length - sample.length };
}

export async function main(argv: string[]): Promise<number> {
  const spec = await loadSpec();
  if (argv[0] === "rules") {
    for (const [id, r] of Object.entries(spec.rules)) console.log(`${r.severity.padEnd(5)} ${id.padEnd(28)} ${r.summary}`);
    return 0;
  }
  const flag = (name: string) => {
    const i = argv.indexOf(name);
    return i >= 0 ? argv[i + 1] : undefined;
  };
  const failOn = flag("--fail-on") ?? "error";
  const jsonOut = flag("--json");
  const fix = argv.includes("--fix");
  const quiet = argv.includes("--quiet");
  const hygieneOnly = argv.includes("--hygiene-only");
  const valued = new Set(["--json", "--fail-on"]);
  const dirs = argv.filter((a, i) => !a.startsWith("--") && !valued.has(argv[i - 1]));
  const roots = dirs.length ? dirs : ["."];
  const results: (RepoResult & { before?: Record<string, number> })[] = [];
  for (const root of roots) {
    const before = fix ? await checkRepo(root, spec) : undefined;
    const fixed = fix ? await fixRepo(root, spec, hygieneOnly) : undefined;
    const res: RepoResult & { before?: Record<string, number> } = await checkRepo(root, spec);
    if (fixed) res.fixed = fixed;
    if (before) res.before = before.counts;
    results.push(res);
    if (!quiet) {
      for (const x of res.findings.slice(0, 200)) console.log(`${x.severity}\t${x.rule}\t${x.file}${x.line ? `:${x.line}` : ""}\t${x.message}`);
      console.error(`${root}: ${res.files} files, ${res.findings.length} findings ${JSON.stringify(res.counts)}${fixed ? `, fixed ${fixed.length} files` : ""}`);
    }
  }
  if (jsonOut) await Bun.write(jsonOut, `${JSON.stringify(results.map((r) => summarize(r, spec.config.max_findings_per_rule_per_file * 10)), null, 2)}\n`);
  const worst = results.flatMap((r) => r.findings).reduce((m, x) => Math.max(m, RANK[x.severity]), 0);
  return failOn === "none" ? 0 : worst >= RANK[failOn] ? 1 : 0;
}

if (import.meta.main) process.exit(await main(process.argv.slice(2)));
