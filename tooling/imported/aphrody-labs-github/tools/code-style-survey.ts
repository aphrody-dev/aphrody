#!/usr/bin/env bun
// Read-only survey of how repositories measure against docs/standards/code-style.md. Counts only; nothing is changed.
// Usage: bun tools/code-style-survey.ts <repo-dir> [<repo-dir> ...] [--commits=100] [--json]
// Needs ripgrep (rg) and git on PATH. Vendored, generated and build trees are excluded from every count.
import { basename, join, resolve } from "node:path";
import { check, loadConfig, type Finding } from "./check-commit";

const argv = process.argv.slice(2);
const dirs = argv.filter((a) => !a.startsWith("--")).map((d) => resolve(d));
const commits = Number(argv.find((a) => a.startsWith("--commits="))?.split("=")[1] ?? 100);
const EXCLUDE = ["vendor", "vendored", "third_party", "upstream", "node_modules", "target", "build", "dist", ".next", "fixtures", "testdata", "snapshots"].flatMap((d) => ["-g", `!**/${d}/**`]);

async function run(cmd: string[], cwd: string): Promise<string> {
  const p = Bun.spawn(cmd, { cwd, stdout: "pipe", stderr: "pipe" });
  const out = await new Response(p.stdout).text();
  await p.exited;
  return out;
}
async function count(dir: string, pattern: string, globs: string[], extra: string[] = []): Promise<number> {
  const out = await run(["rg", "-c", "--no-filename", "--no-messages", ...extra, ...EXCLUDE, ...globs.flatMap((g) => ["-g", g]), "-e", pattern, "."], dir);
  return out.split("\n").reduce((s, l) => s + (Number(l.trim()) || 0), 0);
}
async function files(dir: string, globs: string[]): Promise<number> {
  const out = await run(["rg", "--files", "--no-messages", ...EXCLUDE, ...globs.flatMap((g) => ["-g", g]), "."], dir);
  return out.split("\n").filter(Boolean).length;
}
const exists = (dir: string, f: string) => Bun.file(join(dir, f)).exists();
const text = async (dir: string, f: string) => ((await exists(dir, f)) ? await Bun.file(join(dir, f)).text() : "");

const cfg = await loadConfig(join(import.meta.dir, ".."));
const RS = ["*.rs"];
const TS = ["*.ts", "*.tsx", "*.mts", "*.cts", "*.js", "*.mjs", "*.cjs", "*.jsx", "!*.d.ts", "!*.min.js"];
const report: Record<string, unknown>[] = [];

for (const dir of dirs) {
  const name = basename(dir);
  const log = (await run(["git", "log", `-n`, String(commits), "--no-merges", "--format=%B%x1e"], dir)).split("\x1e").map((m) => m.trim()).filter(Boolean);
  const byRule = new Map<string, number>();
  let failing = 0;
  for (const m of log) {
    const f: Finding[] = check(m, { ...cfg, dco: false, title: false }).filter((x) => x.level === "error");
    if (f.length) failing++;
    for (const id of new Set(f.map((x) => x.id))) byRule.set(id, (byRule.get(id) ?? 0) + 1);
  }
  const cargo = await text(dir, "Cargo.toml");
  const bunfig = await text(dir, "bunfig.toml");
  const rustFiles = await files(dir, RS);
  const tsFiles = await files(dir, TS);
  report.push({
    repo: name,
    commits: { sampled: log.length, failing, byRule: Object.fromEntries([...byRule].sort()) },
    files: { rust: rustFiles, tsjs: tsFiles },
    config: {
      rustfmt: (await exists(dir, "rustfmt.toml")) || (await exists(dir, ".rustfmt.toml")),
      clippyToml: (await exists(dir, "clippy.toml")) || (await exists(dir, ".clippy.toml")),
      cargoLints: /^\[(workspace\.)?lints/m.test(cargo),
      bunfigFmt: /^\[fmt\]/m.test(bunfig), bunfigLint: /^\[lint\]/m.test(bunfig),
      prettierrc: (await exists(dir, ".prettierrc")) || (await exists(dir, ".prettierrc.json")),
      prTemplate: (await exists(dir, ".github/pull_request_template.md")), reviewMd: await exists(dir, "REVIEW.md"), gitmessage: await exists(dir, ".gitmessage"), agentsMd: await exists(dir, "AGENTS.md"),
    },
    rust: rustFiles ? {
      unsafeBlocks: await count(dir, String.raw`\bunsafe\s*\{`, RS), safetyComments: await count(dir, String.raw`//+\s*SAFETY:`, RS),
      unwrapCalls: await count(dir, String.raw`\.unwrap\(\)`, RS), allowAttrs: await count(dir, String.raw`#!?\[allow\(`, RS), expectAttrs: await count(dir, String.raw`#!?\[expect\(`, RS),
      dbgTodo: await count(dir, String.raw`\b(dbg|todo|unimplemented)!\(`, RS), globImports: await count(dir, String.raw`^\s*(pub )?use [^;]*::\*;`, RS),
      ptrAsCasts: await count(dir, String.raw`\bas \*(const|mut)\b`, RS), asUnderscore: await count(dir, String.raw`\bas _\b`, RS),
    } : null,
    tsjs: tsFiles ? {
      anyType: await count(dir, String.raw`(:|\bas|<)\s*any\b`, TS), tsIgnore: await count(dir, String.raw`@ts-(ignore|nocheck)\b`, TS), tsExpectError: await count(dir, String.raw`@ts-expect-error\b`, TS),
      exportDefault: await count(dir, String.raw`^\s*export default\b`, TS), exportLet: await count(dir, String.raw`^\s*export let\b`, TS), varDecl: await count(dir, String.raw`^\s*var\s+\w`, TS),
      looseEquality: await count(dir, String.raw`[^=!<>]==[^=]`, TS, ["--pcre2"]), looseEqualityNotNull: await count(dir, String.raw`[^=!<>]==(?!=)(?!\s*null\b)`, TS, ["--pcre2"]),
      namespaces: await count(dir, String.raw`^\s*(export )?namespace\s+\w+\s*\{`, TS), onlyLeft: await count(dir, String.raw`\b(test|it|describe)\.only\(`, TS), debuggerStmt: await count(dir, String.raw`^\s*debugger;?\s*$`, TS),
    } : null,
  });
}

if (argv.includes("--json")) console.log(JSON.stringify(report, null, 2));
else for (const r of report) console.log(JSON.stringify(r));
