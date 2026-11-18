#!/usr/bin/env bun
// Checks commit messages and pull request titles against docs/standards/code-style.md section 5 (CO-xx) and section 6 (PR-01).
// Commit types and limits come from standard.toml (naming.commit_types, [code_style]) at run time; nothing is hardcoded here.
//
// Usage:
//   bun tools/check-commit.ts --message-file <path>      commit-msg hook: one message, git comment lines ignored
//   bun tools/check-commit.ts --title "<pull request title>"
//   bun tools/check-commit.ts --range <git range>        every commit of the range, e.g. origin/main..HEAD
// Options: --style=area|conventional (default: code_style.commit_style, "area": "area: summary", type(scope) form also accepted; "conventional": only type(scope): summary), --types=a,b (override the type list), --dco (Signed-off-by required),
//          --strict (warnings fail too), --json.
// Exit: 0 clean, 1 at least one error (or warning with --strict), 2 usage.
import { join } from "node:path";

export type Style = "conventional" | "area";
export interface Options { types: string[]; style: Style; dco: boolean; maxSubject: number; maxBody: number; title: boolean }
export interface Finding { level: "error" | "warning"; id: string; message: string }

const SUBJECT = (types: string[]) => new RegExp(`^(${types.join("|")})(\\(([a-z0-9][a-z0-9._/-]*)\\))?(!)?: (\\S.*)$`);
const AREA = /^([A-Za-z0-9][A-Za-z0-9._/-]*(?:\([^)]*\))?)(!)?: (\S.*)$/;
const NON_IMPERATIVE = /^(added|adds|adding|fixed|fixes|fixing|updated|updates|updating|removed|removes|removing|changed|changes|changing|deleted|deleting|implemented|implements|implementing|refactored|refactoring|improved|improves|improving)\b/i;
const VAGUE = /^(fix(es)?( (bug|bugs|it|stuff|things|build|issue|issues))?|bugs?|issues?|things?|it|update[sd]?|wip|misc|changes?|stuff|tweaks?|cleanup|clean up|more fixes|phase \d+|add patch|temp|test|asdf|\.+)$/i;
const ATTRIBUTION = [
  /^co-authored-by:.*(claude|anthropic|copilot|chatgpt|openai|codex|gemini|cursor|devin|aider|\[bot\]|autofix|robobun|noreply@(anthropic|openai|github)\.)/im,
  /^(assisted|generated|created|authored)-(by|with):/im,
  /generated (with|by) .{0,40}(claude|copilot|chatgpt|codex|gemini|cursor|ai\b)/i,
  /\u{1F916}/u,
];
const HOST_PATH = /(?:\b[A-Za-z]:\\(?:Users|src|home)\\|\/home\/[a-z_][a-z0-9_-]*\/|\/Users\/[A-Za-z][^/\s]*\/)/;

export function stripComments(message: string): string {
  // Git removes lines starting with "#" from a commit-msg file; so does the check. A scissors line ends the message.
  const out: string[] = [];
  for (const line of message.replace(/\r\n/g, "\n").split("\n")) {
    if (line.startsWith("# ------------------------ >8 ------------------------")) break;
    if (line.startsWith("#")) continue;
    out.push(line);
  }
  return out.join("\n").replace(/\s+$/, "");
}

function isException(subject: string): boolean {
  return /^Merge (branch|pull request|remote-tracking branch|tag) /.test(subject) || /^Revert ".+"$/.test(subject);
}

export function check(raw: string, opt: Options): Finding[] {
  const f: Finding[] = [];
  const add = (level: Finding["level"], id: string, message: string) => f.push({ level, id, message });
  const message = stripComments(raw);
  const lines = message.split("\n");
  let subject = lines[0] ?? "";
  if (!subject.trim()) { add("error", "CO-02", "subject is empty"); return f; }
  if (/^(fixup|squash|amend)! /.test(subject)) add("error", "CO-14", "autosquash commit left behind; squash it before pushing");
  if (opt.title) subject = subject.replace(/ \(#\d+\)$/, "");
  if (isException(subject)) return f;

  const conventional = SUBJECT(opt.types).exec(subject);
  const area = opt.style === "area" && !conventional ? AREA.exec(subject) : null;
  let summary = "";
  let breaking = false;
  if (conventional) { summary = conventional[5]!; breaking = !!conventional[4]; }
  else if (area) { summary = area[3]!; breaking = !!area[2]; }
  else add("error", "CO-01", opt.style === "area" ? 'subject must be "area: summary" (or "type(scope): summary")' : `subject must be "type(scope): summary" with type one of ${opt.types.join(", ")}`);

  if (subject.length > opt.maxSubject) add("error", "CO-02", `subject is ${subject.length} characters, limit ${opt.maxSubject}`);
  if (summary) {
    if (summary.endsWith(".")) add("error", "CO-02", "subject ends with a period");
    if (NON_IMPERATIVE.test(summary)) add("error", "CO-02", `summary is not imperative ("${summary.split(" ")[0]}"): write "fix", "add", "remove"`);
    if (VAGUE.test(summary.trim())) add("error", "CO-02", `summary "${summary}" says nothing; name what changed`);
    else if (summary.trim().split(/\s+/).length < 2) add("warning", "CO-02", "one-word summary; say what changed");
  }
  if (opt.title) return f;

  if (lines.length > 1 && lines[1]!.trim() !== "") add("error", "CO-03", "second line must be blank");
  const body = lines.slice(2);
  // The trailer block is the last paragraph when every line of it is "Token: value" (or an indented continuation).
  const trailerStart = (() => {
    let end = body.length;
    while (end > 0 && body[end - 1]!.trim() === "") end--;
    let i = end;
    while (i > 0 && body[i - 1]!.trim() !== "") i--;
    const para = body.slice(i, end);
    return para.length && i > 0 && para.every((l) => /^([A-Za-z][A-Za-z-]*|BREAKING CHANGE): \S/.test(l) || /^\s+\S/.test(l)) ? i : body.length;
  })();
  const prose = body.slice(0, trailerStart);
  const trailers = body.slice(trailerStart).filter((l) => l.trim());
  const hasBody = prose.some((l) => l.trim());
  if (!hasBody && conventional && ["feat", "fix", "perf", "refactor"].includes(conventional[1]!)) add("warning", "CO-03", `a ${conventional[1]} commit needs a body: say why, and how it was verified`);

  let fence = false;
  prose.forEach((l, i) => {
    if (/^\s*```/.test(l)) fence = !fence;
    const exempt = fence || /^\s/.test(l) || /^\|/.test(l) || /https?:\/\/\S{30,}/.test(l) || /^\[[^\]]+\]: /.test(l);
    if (!exempt && l.length > opt.maxBody) add("warning", "CO-12", `body line ${i + 3} is ${l.length} characters (wrap at 72, hard limit ${opt.maxBody}; tables, code and long URLs are exempt)`);
  });

  if (breaking && !/^BREAKING[ -]CHANGE: \S/m.test(message)) add("error", "CO-08", 'a "!" subject needs a "BREAKING CHANGE:" footer');
  if (/^BREAKING[ -]CHANGE: \S/m.test(message) && !breaking && conventional) add("warning", "CO-08", 'a BREAKING CHANGE footer needs "!" after the type or scope');
  for (const re of ATTRIBUTION) if (re.test(message)) { add("error", "CO-10", "AI or bot attribution is not allowed in commit messages"); break; }
  const signed = trailers.some((l) => /^Signed-off-by: .+ <.+>$/.test(l));
  if (opt.dco && !signed) add("error", "CO-11", "this repository asks for a DCO: add Signed-off-by (git commit -s)");
  if (HOST_PATH.test(message)) add("warning", "CO-13", "absolute path of a developer machine in the message");
  for (const l of trailers) if (/^Fixes: [0-9a-f]{7,40}\b/.test(l) && !/^Fixes: [0-9a-f]{12,40} \(".+"\)$/.test(l)) add("warning", "CO-06", 'write "Fixes: <12 hex or more> (\\"subject of that commit\\")"');
  return f;
}

async function sh(args: string[]): Promise<string> {
  const p = Bun.spawn(args, { stdout: "pipe", stderr: "pipe" });
  const [out, err] = await Promise.all([new Response(p.stdout).text(), new Response(p.stderr).text()]);
  if ((await p.exited) !== 0) throw new Error(`${args.join(" ")}: ${err.trim()}`);
  return out;
}

export async function loadConfig(root: string): Promise<Pick<Options, "types" | "maxSubject" | "maxBody" | "style">> {
  const std = Bun.TOML.parse(await Bun.file(join(root, "standard.toml")).text()) as { naming: { commit_types: string[] }; code_style: { commit_style: Style; commit_subject_max: number; commit_body_max: number } };
  return { types: std.naming.commit_types, style: std.code_style.commit_style, maxSubject: std.code_style.commit_subject_max, maxBody: std.code_style.commit_body_max };
}

if (import.meta.main) {
  const argv = process.argv.slice(2);
  const val = (k: string) => { const i = argv.indexOf(`--${k}`); if (i >= 0) return argv[i + 1]; return argv.find((a) => a.startsWith(`--${k}=`))?.split("=").slice(1).join("="); };
  const has = (k: string) => argv.includes(`--${k}`);
  const typesFile = val("types");
  const root = join(import.meta.dir, "..");
  const cfg = await loadConfig(root);
  const style = (val("style") ?? cfg.style) as Style;
  if (style !== "area" && style !== "conventional") { console.error(`unknown --style ${style}`); process.exit(2); }
  const opt: Options = { ...cfg, types: typesFile ? typesFile.split(",") : cfg.types, style, dco: has("dco"), title: false };
  const jobs: { label: string; text: string; title: boolean }[] = [];
  if (val("message-file")) jobs.push({ label: val("message-file")!, text: await Bun.file(val("message-file")!).text(), title: false });
  else if (val("title") !== undefined) jobs.push({ label: "title", text: val("title")!, title: true });
  else if (val("range")) {
    const log = await sh(["git", "log", "--format=%H%x1f%B%x1e", val("range")!]);
    for (const rec of log.split("\x1e")) {
      const [sha, ...rest] = rec.replace(/^\n/, "").split("\x1f");
      if (sha) jobs.push({ label: sha.slice(0, 12), text: rest.join("\x1f"), title: false });
    }
  } else { console.error("usage: check-commit.ts --message-file <path> | --title <text> | --range <git range>"); process.exit(2); }
  let errors = 0, warnings = 0;
  const report: unknown[] = [];
  for (const j of jobs) {
    const findings = check(j.text, { ...opt, title: j.title });
    for (const x of findings) { x.level === "error" ? errors++ : warnings++; if (!has("json")) console[x.level === "error" ? "error" : "warn"](`${j.label}: ${x.level} ${x.id}: ${x.message}`); }
    report.push({ label: j.label, findings });
  }
  if (has("json")) console.log(JSON.stringify(report));
  else console.log(`${jobs.length} checked, ${errors} errors, ${warnings} warnings`);
  process.exit(errors || (has("strict") && warnings) ? 1 : 0);
}
