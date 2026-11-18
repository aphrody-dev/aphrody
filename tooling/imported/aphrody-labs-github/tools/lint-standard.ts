#!/usr/bin/env bun
// Lints the standard itself: standard.toml shape, ruleset JSON, workflow pinning and permissions, issue forms, starter workflows.
// Usage: bun tools/lint-standard.ts [root]   Exit 1 on any error; warnings are printed only.
import { Glob } from "bun";
import { join, resolve, basename } from "node:path";

const root = resolve(process.argv[2] ?? join(import.meta.dir, ".."));
const errors: string[] = [];
const warnings: string[] = [];
const err = (m: string) => errors.push(m);
const warn = (m: string) => warnings.push(m);

type Toml = Record<string, any>;
const std = Bun.TOML.parse(await Bun.file(join(root, "standard.toml")).text()) as Toml;

// --- standard.toml ----------------------------------------------------------------------------
for (const key of ["meta", "scope", "labels", "files", "repo_settings", "security", "actions", "dependabot", "rulesets", "release", "publishing", "apps", "secrets", "cloudflare", "profile"]) {
  if (!(key in std)) err(`standard.toml: missing [${key}]`);
}
if (!/^\d+\.\d+\.\d+$/.test(std.meta?.version ?? "")) err("standard.toml: meta.version must be semver");

const names = new Set<string>();
for (const l of std.labels ?? []) {
  if (!l.name || !/^[0-9a-f]{6}$/.test(l.color ?? "")) err(`label ${JSON.stringify(l.name)}: needs name and 6-hex color`);
  if ((l.description ?? "").length > 100) err(`label ${l.name}: description over 100 chars`);
  if (names.has(l.name)) err(`label ${l.name}: duplicate`);
  names.add(l.name);
}
for (const [, to] of Object.entries(std.labels_rename ?? {})) if (!names.has(to as string)) err(`labels_rename target ${to} is not a defined label`);

for (const [name, rs] of Object.entries<any>(std.rulesets ?? {})) {
  if (typeof rs !== "object" || !("body" in rs)) continue;
  try {
    const body = JSON.parse(rs.body);
    if (body.name !== name) err(`ruleset ${name}: body.name is ${body.name}`);
    if (!["branch", "tag", "push"].includes(body.target)) err(`ruleset ${name}: bad target`);
    if (!Array.isArray(body.rules) || !body.rules.length) err(`ruleset ${name}: no rules`);
  } catch (e) {
    err(`ruleset ${name}: body is not JSON (${(e as Error).message})`);
  }
}
for (const [tier, list] of Object.entries<string[]>(std.rulesets?.tiers ?? {})) {
  for (const n of list) if (!std.rulesets?.[n]?.body) err(`rulesets.tiers.${tier}: unknown ruleset ${n}`);
}

const pins = new Map<string, { sha: string; tag: string }>();
for (const [action, v] of Object.entries<any>(std.actions?.pinned_versions ?? {})) {
  if (!/^[0-9a-f]{40}$/.test(v.sha)) err(`pinned_versions.${action}: sha is not 40 hex`);
  pins.set(action, v);
}

const profile = std.profile ?? {};
for (const r of profile.roles ?? []) if (!r.id || !r.title) err("profile.roles: id and title required");
for (const f of [...(profile.roles ?? []), ...(profile.fork_families ?? [])]) {
  for (const p of f.patterns ?? []) {
    try {
      new RegExp(p, "i");
    } catch {
      err(`profile pattern is not a valid regex: ${p}`);
    }
  }
}

// --- workflows --------------------------------------------------------------------------------
const first = (std.actions?.conventions?.first_party_ref as string) ?? "main";
const workflowGlobs = [".github/workflows/*.yml", "workflow-templates/*.yml", "templates/**/*.yml"];
for (const pattern of workflowGlobs) {
  for await (const rel of new Glob(pattern).scan({ cwd: root, dot: true })) {
    const text = await Bun.file(join(root, rel)).text();
    let doc: any;
    try {
      doc = Bun.YAML.parse(text);
    } catch (e) {
      err(`${rel}: invalid YAML (${(e as Error).message})`);
      continue;
    }
    if (!doc || typeof doc !== "object" || !doc.jobs) continue; // not a workflow
    if (!doc.permissions) err(`${rel}: top-level permissions are required`);
    for (const [id, job] of Object.entries<any>(doc.jobs)) {
      if (!job["timeout-minutes"] && !job.uses) warn(`${rel}: job ${id} has no timeout-minutes`);
    }
    for (const m of text.matchAll(/^\s*-?\s*uses:\s*([^\s#]+)(?:\s*#\s*(\S+))?/gm)) {
      const ref = m[1];
      if (ref.startsWith("./") || ref.startsWith("docker://")) continue;
      const [path, version] = ref.split("@");
      const owner = path.split("/")[0];
      if (owner === "aphrody-labs" || owner === "aphrody-dev") {
        if (version !== first && !/^[0-9a-f]{40}$/.test(version ?? "")) warn(`${rel}: ${ref} should reference @${first} or a sha`);
        continue;
      }
      if (!/^[0-9a-f]{40}$/.test(version ?? "")) {
        err(`${rel}: ${ref} is not pinned to a full commit sha`);
        continue;
      }
      if (!m[2]) warn(`${rel}: ${ref} has no version comment`);
      const key = path.split("/").slice(0, 2).join("/");
      const pin = pins.get(key);
      if (pin && pin.sha !== version) warn(`${rel}: ${key} sha differs from standard.toml pin ${pin.tag} (Dependabot may have bumped it; refresh standard.toml)`);
    }
    if (/\$\{\{\s*github\.event\.(issue|pull_request|comment|review|head_commit|pages|commits)[^}]*\}\}/.test(text.replace(/^\s*#.*$/gm, "")) && /run:/.test(text)) {
      warn(`${rel}: github.event data may be interpolated into a script; pass it through env`);
    }
  }
}

// --- starter workflows ------------------------------------------------------------------------
for await (const rel of new Glob("workflow-templates/*.yml").scan({ cwd: root })) {
  const props = join(root, rel.replace(/\.yml$/, ".properties.json"));
  if (!(await Bun.file(props).exists())) {
    err(`${rel}: missing ${basename(props)}`);
    continue;
  }
  const p = await Bun.file(props).json();
  if (!p.name || !p.description) err(`${basename(props)}: name and description required`);
}

// --- issue forms ------------------------------------------------------------------------------
const labelNames = names;
for await (const rel of new Glob(".github/ISSUE_TEMPLATE/*.yml").scan({ cwd: root, dot: true })) {
  const doc: any = Bun.YAML.parse(await Bun.file(join(root, rel)).text());
  if (basename(rel) === "config.yml") {
    if (doc.blank_issues_enabled !== false) warn(`${rel}: blank issues are enabled`);
    continue;
  }
  if (!doc.name || !doc.description || !Array.isArray(doc.body) || !doc.body.length) err(`${rel}: needs name, description and body`);
  const ids = new Set<string>();
  for (const el of doc.body ?? []) {
    if (el.id) {
      if (ids.has(el.id)) err(`${rel}: duplicate id ${el.id}`);
      ids.add(el.id);
    }
  }
  for (const l of doc.labels ?? []) if (!labelNames.has(l)) err(`${rel}: label ${l} is not in standard.toml`);
}

// --- code standard (section 19) --------------------------------------------------------------
const cs = std.code_style ?? {};
for (const k of ["doc", "sources", "conflicts"]) {
  if (!cs[k]) err(`standard.toml: code_style.${k} is missing`);
  else if (!(await Bun.file(join(root, cs[k])).exists())) err(`code_style.${k}: ${cs[k]} does not exist`);
}
if (!(std.naming?.commit_styles ?? []).includes(cs.commit_style)) err("standard.toml: code_style.commit_style must be one of naming.commit_styles");
if (cs.commit_style !== std.naming?.commit) err("standard.toml: code_style.commit_style and naming.commit differ");
for (const k of ["commit_subject_max", "commit_body_max", "commit_body_wrap"]) if (!Number.isInteger(cs[k]) || cs[k] < 20) err(`standard.toml: code_style.${k} must be an integer`);
for (const t of std.files?.templates ?? []) {
  if (!t.source || !t.dest || !["copy", "merge"].includes(t.mode)) err(`files.templates: needs source, dest and mode copy|merge (${JSON.stringify(t)})`);
  else if (!(await Bun.file(join(root, t.source)).exists())) err(`files.templates: ${t.source} does not exist`);
}
for (const f of std.files?.original_recommended ?? []) {
  const t = (std.files?.templates ?? []).find((x: Toml) => x.dest === f && x.mode === "copy");
  if ((f === ".gitmessage" || f === "REVIEW.md") && !t) warn(`files.original_recommended: ${f} has no [[files.templates]] copy entry`);
}

// --- required files ---------------------------------------------------------------------------
for (const f of std.files?.org_fallback ?? []) if (!(await Bun.file(join(root, f)).exists())) err(`org fallback file missing: ${f}`);
for (const f of ["STANDARD.md", "profile/README.md", "profile/generate.ts", "docs/standards/commit-format.md"]) if (!(await Bun.file(join(root, f)).exists())) err(`missing ${f}`);

for (const w of warnings) console.warn(`warning: ${w}`);
for (const e of errors) console.error(`error: ${e}`);
console.log(`${errors.length} errors, ${warnings.length} warnings`);
process.exit(errors.length ? 1 : 0);
