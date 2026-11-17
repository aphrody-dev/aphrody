#!/usr/bin/env bun
/**
 * W3C Internationalization (https://www.w3.org/International/) auditor & enforcer
 * for the Aphrody monorepo, developer PC settings, and companion user configs.
 *
 * Implements verification rules grounded in:
 * - W3C CharMod-Norm (https://www.w3.org/TR/charmod-norm/): UTF-8 only, no BOM, NFC paths, Bidi isolate safety
 * - W3C LTLI (https://www.w3.org/TR/ltli/) & BCP 47: well-formed tags, subtag canonicalization, negotiation
 * - W3C String-Meta (https://www.w3.org/TR/string-meta/): `lang` + `dir` (`ltr`|`rtl`|`auto`) metadata
 * - W3C Authoring HTML/SVG/CSS & International Specs: `<meta charset="utf-8">`, `<html lang dir>`, `<svg xml:lang>`
 * - CLDR / ECMA-402 / ISO 8601: runtime `Intl.*`, balanced translation catalogs across `assets/os/locales.toml`
 */
import {
  existsSync,
  readdirSync,
  readFileSync,
  realpathSync,
  renameSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, extname, join, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "../..");

export interface W3cI18nFinding {
  rule: string;
  severity: "error" | "warn";
  file: string;
  line?: number;
  message: string;
}

export interface W3cI18nReport {
  ok: boolean;
  root: string;
  scannedFiles: number;
  errorCount: number;
  warnCount: number;
  catalogLocales: string[];
  nshLocales: string[];
  findings: W3cI18nFinding[];
  pcStatus?: PcI18nStatus;
}

export interface PcI18nStatus {
  gitQuotepathFalse: boolean;
  gitCommitEncodingUtf8: boolean;
  gitLogOutputEncodingUtf8: boolean;
  gitAutocrlfFalse: boolean;
  userEditorconfigExists: boolean;
  userLangUtf8: boolean;
  pythonUtf8Enabled: boolean;
  windowsSystemCodePageAcp: string | null;
  windowsSystemCodePageUtf8: boolean;
}

function readWindowsUserEnv(name: string): string {
  if (process.platform !== "win32") return process.env[name] ?? "";
  const r = Bun.spawnSync(["reg", "query", "HKCU\\Environment", "/v", name], {
    stdout: "pipe",
    stderr: "pipe",
  });
  const out = new TextDecoder().decode(r.stdout);
  const m = out.match(new RegExp(`${name}\\s+REG_(?:EXPAND_)?SZ\\s+(.+)`));
  return m?.[1]?.trim() ?? process.env[name] ?? "";
}

const SKIP_DIRS = new Set([
  ".git",
  "node_modules",
  "target",
  "dist",
  "build",
  ".next",
  ".turbo",
  ".vercel",
  ".cache",
  "vendor",
  "third_party",
  "upstream",
  "testdata",
  "fixtures",
]);

const TEXT_EXTS = new Set([
  ".rs",
  ".ts",
  ".tsx",
  ".js",
  ".mjs",
  ".cjs",
  ".json",
  ".toml",
  ".yaml",
  ".yml",
  ".html",
  ".htm",
  ".xhtml",
  ".css",
  ".svg",
  ".md",
  ".mdx",
  ".sh",
  ".ps1",
  ".cmd",
  ".bat",
  ".py",
  ".txt",
]);

const CRLF_ALLOWED_EXTS = new Set([".bat", ".cmd"]);
const BIDI_UNSAFE_RE = /[\u202A-\u202E]/;

export function isWellFormedBcp47(tag: string): boolean {
  const trimmed = tag.trim();
  if (!trimmed || trimmed.includes("_")) return false;
  return /^[a-z]{2,3}(-[A-Z][a-z]{3})?(-([A-Z]{2}|\d{3}))?(-[a-z0-9]{4,8}|-[a-z]-[a-z0-9-]+)*$/.test(
    trimmed,
  );
}

export function scanBufferI18n(rel: string, buf: Buffer): { findings: W3cI18nFinding[]; modified?: Buffer } {
  const findings: W3cI18nFinding[] = [];
  const ext = extname(rel).toLowerCase();

  // Check UTF-16 BOM
  if (
    buf.length >= 2 &&
    ((buf[0] === 0xff && buf[1] === 0xfe) || (buf[0] === 0xfe && buf[1] === 0xff))
  ) {
    findings.push({
      rule: "charmod-utf16",
      severity: "error",
      file: rel,
      message: "UTF-16 BOM detected; W3C CharMod requires UTF-8 encoding",
    });
    return { findings };
  }

  // Check UTF-8 BOM
  const hasBom = buf.length >= 3 && buf[0] === 0xef && buf[1] === 0xbb && buf[2] === 0xbf;
  if (hasBom) {
    findings.push({
      rule: "charmod-bom",
      severity: "error",
      file: rel,
      message: "Unexpected UTF-8 BOM (EF BB BF); strip BOM for clean POSIX/web UTF-8",
    });
  }

  // Validate strict UTF-8 & check Unicode NFC filename
  const body = hasBom ? buf.subarray(3) : buf;
  let text: string;
  try {
    text = new TextDecoder("utf-8", { fatal: true }).decode(body);
  } catch {
    findings.push({
      rule: "charmod-utf8-invalid",
      severity: "error",
      file: rel,
      message: "Invalid UTF-8 byte sequence",
    });
    return { findings };
  }

  if (rel.normalize("NFC") !== rel) {
    findings.push({
      rule: "charmod-nfc-path",
      severity: "warn",
      file: rel,
      message: "File path is not Unicode NFC normalized",
    });
  }

  if (!CRLF_ALLOWED_EXTS.has(ext) && text.includes("\r\n")) {
    findings.push({
      rule: "eol-crlf",
      severity: "error",
      file: rel,
      message: "CRLF line endings found; repository enforces LF via .editorconfig/.gitattributes",
    });
  }

  // Bidi override (Trojan Source) check on code files unless explicit test file
  const isTest = /(?:^|\/)(?:tests?|__tests__|unicode_confusables\.rs|w3c-i18n(?:\.test)?\.ts|i18n\.rs)/.test(rel);
  if (!isTest && BIDI_UNSAFE_RE.test(text)) {
    findings.push({
      rule: "charmod-bidi-override",
      severity: "error",
      file: rel,
      message: "Unsafe Bidi embedding/override control (U+202A..U+202E) detected; use LRI/RLI/FSI..PDI isolates",
    });
  }

  // HTML checks (root lang, dir, <meta charset="utf-8">)
  if ([".html", ".htm", ".xhtml"].includes(ext) && !rel.includes("test")) {
    const htmlTag = text.match(/<html\b[^>]*>/i)?.[0] ?? "";
    if (htmlTag) {
      const langMatch = htmlTag.match(/\blang\s*=\s*["']([^"']+)["']/i);
      if (!langMatch) {
        findings.push({
          rule: "html-lang",
          severity: "error",
          file: rel,
          message: "<html> missing lang attribute",
        });
      } else if (!isWellFormedBcp47(langMatch[1])) {
        findings.push({
          rule: "ltli-bcp47-malformed",
          severity: "error",
          file: rel,
          message: `Malformed BCP 47 language tag '${langMatch[1]}' on <html>`,
        });
      }
      if (!/\bdir\s*=/i.test(htmlTag)) {
        findings.push({
          rule: "string-meta-html-dir",
          severity: "error",
          file: rel,
          message: "<html> missing dir attribute (ltr | rtl | auto)",
        });
      }
    }
    if (!/<meta\s+charset\s*=\s*["']?utf-8/i.test(text.slice(0, 1024))) {
      findings.push({
        rule: "html-meta-charset",
        severity: "error",
        file: rel,
        message: "HTML missing <meta charset=\"utf-8\"> within first 1024 bytes of <head>",
      });
    }
  }

  // Check brand/text SVG files (skipbing test fixtures already in SKIP_DIRS)
  if (ext === ".svg" && /<text\b/i.test(text)) {
    const svgTag = text.match(/<svg\b[^>]*>/i)?.[0] ?? "";
    if (svgTag && !/\b(?:xml:)?lang\s*=/.test(svgTag)) {
      findings.push({
        rule: "svg-lang",
        severity: "warn",
        file: rel,
        message: "SVG with <text> lacks xml:lang/lang attribute per W3C Authoring SVG i18n",
      });
    }
  }

  // Check hardcoded literal locale passed to ECMA-402 Intl / toLocale*String outside tests/catalogs
  if ([".ts", ".tsx", ".js", ".mjs"].includes(ext) && !isTest && !rel.startsWith("docs/")) {
    const lines = text.split("\n");
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      if (line.includes("i18n-ignore")) continue;
      if (
        /(?:toLocale(?:Date|Time)?String|new\s+Intl\.[A-Za-z]+)\(\s*["'][a-z]{2}(?:-[A-Za-z0-9]+)*["']/.test(
          line,
        )
      ) {
        findings.push({
          rule: "cldr-literal-locale",
          severity: "warn",
          file: rel,
          line: i + 1,
          message: "Literal locale tag passed to Intl/toLocale*; derive locale from request, user settings, or document.documentElement.lang",
        });
      }
    }
  }

  return { findings };
}

export function checkCatalogParity(root = repoRoot): {
  findings: W3cI18nFinding[];
  catalogLocales: string[];
  nshLocales: string[];
} {
  const findings: W3cI18nFinding[] = [];
  const localesTomlPath = join(root, "assets/os/locales.toml");
  const catalogLocales: string[] = [];

  if (!existsSync(localesTomlPath)) {
    findings.push({
      rule: "catalog-missing",
      severity: "error",
      file: "assets/os/locales.toml",
      message: "Missing OS i18n locale catalog",
    });
  } else {
    const parsed = Bun.TOML.parse(readFileSync(localesTomlPath, "utf8")) as {
      default_locale?: string;
      locales?: Record<string, { dir?: string; messages?: Record<string, string> }>;
    };
    const locales = parsed.locales ?? {};
    catalogLocales.push(...Object.keys(locales).sort());
    const baseKeys = Object.keys(locales.en?.messages ?? {});
    if (baseKeys.length === 0) {
      findings.push({
        rule: "catalog-en-empty",
        severity: "error",
        file: "assets/os/locales.toml",
        message: "English base catalog [locales.en.messages] is empty",
      });
    }
    for (const [tag, entry] of Object.entries(locales)) {
      if (!isWellFormedBcp47(tag)) {
        findings.push({
          rule: "ltli-bcp47-malformed",
          severity: "error",
          file: "assets/os/locales.toml",
          message: `Malformed BCP 47 key '${tag}'`,
        });
      }
      if (!entry.dir || !["ltr", "rtl"].includes(entry.dir)) {
        findings.push({
          rule: "string-meta-catalog-dir",
          severity: "error",
          file: "assets/os/locales.toml",
          message: `Locale '${tag}' missing valid dir ('ltr'|'rtl')`,
        });
      }
      for (const k of baseKeys) {
        if (!entry.messages?.[k]) {
          findings.push({
            rule: "catalog-key-missing",
            severity: "error",
            file: "assets/os/locales.toml",
            message: `Locale '${tag}' missing message key '${k}'`,
          });
        }
      }
    }
    for (const requiredRtl of ["ar", "he"]) {
      if (!locales[requiredRtl]) {
        findings.push({
          rule: "catalog-rtl-coverage",
          severity: "error",
          file: "assets/os/locales.toml",
          message: `RTL coverage missing required locale '${requiredRtl}'`,
        });
      }
    }
  }

  const nshDir = join(root, "crates/nsh-i18n/data");
  const nshLocales: string[] = [];
  if (existsSync(nshDir)) {
    for (const entry of readdirSync(nshDir)) {
      if (!entry.endsWith(".json")) continue;
      nshLocales.push(entry.replace(/\.json$/, ""));
    }
    nshLocales.sort();
    for (const required of ["en", "fr", "ar", "fa", "he"]) {
      if (!nshLocales.includes(required)) {
        findings.push({
          rule: "nsh-i18n-locale-missing",
          severity: "error",
          file: "crates/nsh-i18n/data",
          message: `crates/nsh-i18n/data missing CLDR dataset '${required}.json'`,
        });
      }
    }
  }

  return { findings, catalogLocales, nshLocales };
}

export function inspectPcI18n(): PcI18nStatus {
  const gitGet = (key: string) => {
    const r = Bun.spawnSync(["git", "config", "--global", "--get", key], {
      stdout: "pipe",
      stderr: "pipe",
    });
    return new TextDecoder().decode(r.stdout).trim();
  };
  const home = process.env.USERPROFILE || process.env.HOME || "";
  const editorconfig = home ? join(home, ".editorconfig") : "";
  const lang = (
    process.env.LANG ||
    process.env.LC_ALL ||
    readWindowsUserEnv("LANG") ||
    readWindowsUserEnv("LC_CTYPE")
  ).toLowerCase();
  const pythonUtf8 =
    process.env.PYTHONUTF8 === "1" || readWindowsUserEnv("PYTHONUTF8") === "1";
  let windowsSystemCodePageAcp: string | null = null;
  if (process.platform === "win32") {
    const reg = Bun.spawnSync(
      ["reg", "query", "HKLM\\SYSTEM\\CurrentControlSet\\Control\\Nls\\CodePage", "/v", "ACP"],
      { stdout: "pipe", stderr: "pipe" },
    );
    const out = new TextDecoder().decode(reg.stdout);
    windowsSystemCodePageAcp = out.match(/ACP\s+REG_SZ\s+(\S+)/)?.[1] ?? null;
  }
  return {
    gitQuotepathFalse: gitGet("core.quotepath") === "false",
    gitCommitEncodingUtf8: gitGet("i18n.commitEncoding").toLowerCase() === "utf-8",
    gitLogOutputEncodingUtf8: gitGet("i18n.logOutputEncoding").toLowerCase() === "utf-8",
    gitAutocrlfFalse: gitGet("core.autocrlf") === "false",
    userEditorconfigExists: Boolean(editorconfig && existsSync(editorconfig)),
    userLangUtf8: lang.includes("utf-8") || lang.includes("utf8"),
    pythonUtf8Enabled: pythonUtf8,
    windowsSystemCodePageAcp,
    windowsSystemCodePageUtf8: windowsSystemCodePageAcp === "65001",
  };
}

export function auditRepoW3cI18n(root = repoRoot, options: { fix?: boolean; includePc?: boolean } = {}): W3cI18nReport {
  const findings: W3cI18nFinding[] = [];
  let scannedFiles = 0;

  // Root config checks (.editorconfig + .gitattributes)
  const ecPath = join(root, ".editorconfig");
  if (!existsSync(ecPath)) {
    findings.push({
      rule: "editorconfig-missing",
      severity: "error",
      file: ".editorconfig",
      message: "Root .editorconfig missing (required for charset = utf-8 & end_of_line = lf)",
    });
  } else {
    const ec = readFileSync(ecPath, "utf8");
    if (!/charset\s*=\s*utf-8/i.test(ec)) {
      findings.push({
        rule: "editorconfig-charset",
        severity: "error",
        file: ".editorconfig",
        message: ".editorconfig missing charset = utf-8",
      });
    }
    if (!/end_of_line\s*=\s*lf/i.test(ec)) {
      findings.push({
        rule: "editorconfig-eol",
        severity: "error",
        file: ".editorconfig",
        message: ".editorconfig missing end_of_line = lf",
      });
    }
  }

  const gaPath = join(root, ".gitattributes");
  if (!existsSync(gaPath) || !readFileSync(gaPath, "utf8").includes("text=auto eol=lf")) {
    findings.push({
      rule: "gitattributes-eol",
      severity: "error",
      file: ".gitattributes",
      message: ".gitattributes missing '* text=auto eol=lf'",
    });
  }

  const resolvedRoot = resolve(root);
  const realRoot = realpathSync(resolvedRoot);
  const containedCache = new Map<string, boolean>([[resolvedRoot, true]]);

  function isContainedRealPath(target: string): boolean {
    const cached = containedCache.get(target);
    if (cached !== undefined) return cached;
    try {
      const rp = realpathSync(target);
      const ok = rp === realRoot || rp.startsWith(realRoot + "/") || rp.startsWith(realRoot + "\\");
      containedCache.set(target, ok);
      return ok;
    } catch {
      containedCache.set(target, false);
      return false;
    }
  }

  function atomicWrite(target: string, data: Buffer) {
    if (!isContainedRealPath(target)) return;
    const tmp = join(dirname(target), `.${basename(target)}.i18n-tmp-${process.pid}`);
    writeFileSync(tmp, data);
    renameSync(tmp, target);
  }

  function walk(dir: string) {
    if (!isContainedRealPath(dir)) return;
    let entries;
    try {
      entries = readdirSync(dir, { withFileTypes: true });
    } catch {
      return;
    }
    for (const entry of entries) {
      if (SKIP_DIRS.has(entry.name)) continue;
      const full = join(dir, entry.name);
      if (entry.isSymbolicLink()) {
        const relLink = full.slice(resolvedRoot.length + 1).replace(/\\/g, "/");
        findings.push({
          rule: "charmod-symlink-skipped",
          severity: "warn",
          file: relLink,
          message: "Skipped symbolic link / junction to prevent out-of-tree traversal or write",
        });
        continue;
      }
      if (entry.isDirectory()) {
        walk(full);
        continue;
      }
      const ext = extname(entry.name).toLowerCase();
      if (!TEXT_EXTS.has(ext) && entry.name !== ".editorconfig" && entry.name !== ".gitattributes") {
        continue;
      }
      let size = 0;
      try {
        size = statSync(full).size;
      } catch {
        continue;
      }
      if (size > 2 * 1024 * 1024) continue;
      scannedFiles++;
      const rel = full.slice(resolvedRoot.length + 1).replace(/\\/g, "/");
      const raw = readFileSync(full);

      if (options.fix) {
        let fixedBuf = raw;
        let changed = false;
        if (fixedBuf.length >= 3 && fixedBuf[0] === 0xef && fixedBuf[1] === 0xbb && fixedBuf[2] === 0xbf) {
          fixedBuf = fixedBuf.subarray(3);
          changed = true;
        }
        if (!CRLF_ALLOWED_EXTS.has(ext)) {
          const text = fixedBuf.toString("utf8");
          if (text.includes("\r\n")) {
            fixedBuf = Buffer.from(text.replace(/\r\n/g, "\n"), "utf8");
            changed = true;
          }
        }
        if (changed) atomicWrite(full, fixedBuf);
      }

      const checked = scanBufferI18n(rel, options.fix ? readFileSync(full) : raw);
      findings.push(...checked.findings);
    }
  }

  walk(resolvedRoot);

  const catalog = checkCatalogParity(root);
  findings.push(...catalog.findings);

  const errorCount = findings.filter((f) => f.severity === "error").length;
  const warnCount = findings.filter((f) => f.severity === "warn").length;
  const pcStatus = options.includePc ? inspectPcI18n() : undefined;

  return {
    ok: errorCount === 0,
    root,
    scannedFiles,
    errorCount,
    warnCount,
    catalogLocales: catalog.catalogLocales,
    nshLocales: catalog.nshLocales,
    findings,
    pcStatus,
  };
}

if (import.meta.main) {
  const args = Bun.argv.slice(2);
  const cmd = args[0] ?? "check";
  const rootArg = args.find((a) => a.startsWith("--root="))?.slice(7) ?? repoRoot;
  const report = auditRepoW3cI18n(resolve(rootArg), {
    fix: cmd === "fix" || args.includes("--fix"),
    includePc: cmd === "pc" || cmd === "audit" || args.includes("--pc"),
  });

  if (cmd === "audit" || cmd === "pc" || args.includes("--json")) {
    console.log(JSON.stringify(report, null, 2));
  } else {
    console.log(
      `[w3c-i18n] root=${report.root} scanned=${report.scannedFiles} errors=${report.errorCount} warnings=${report.warnCount} os_locales=${report.catalogLocales.length} nsh_locales=${report.nshLocales.length}`,
    );
    for (const f of report.findings.slice(0, 50)) {
      console.log(`  [${f.severity}] ${f.rule}: ${f.file}${f.line ? `:${f.line}` : ""} — ${f.message}`);
    }
  }
  if (!report.ok && cmd !== "audit") {
    process.exit(1);
  }
}
