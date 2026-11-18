#!/usr/bin/env bun
// Lints docs/standards/code-style.md: every rule row has a unique id, a known source tag, a valid status and an existing conflict row.
// Usage: bun tools/lint-code-style.ts [root]   Exit 1 on any error.
import { join, resolve } from "node:path";

export const STATUSES = ["adopted", "adapted", "rejected"] as const;
export const ENFORCERS = ["fmt", "lint", "clippy", "commit", "review", "ci"] as const;
const ROW = /^\| ([A-Z]{2}-\d{2}) \| (.+) \| (.+) \| (adopted|adapted|rejected)(?: \(((?:C-\d{2})(?:, C-\d{2})*)\))? \| ([a-z, ]+) \|$/;

export interface Rule { id: string; rule: string; source: string; status: string; conflicts: string[]; enforced: string[] }

export function parseRules(markdown: string): { rules: Rule[]; malformed: string[] } {
  const rules: Rule[] = [];
  const malformed: string[] = [];
  for (const line of markdown.split("\n")) {
    if (!/^\| [A-Z]{2}-\d{2} \|/.test(line)) continue;
    const m = ROW.exec(line);
    if (!m) { malformed.push(line.slice(0, 60)); continue; }
    rules.push({ id: m[1]!, rule: m[2]!, source: m[3]!, status: m[4]!, conflicts: m[5] ? m[5].split(", ") : [], enforced: m[6]!.split(", ") });
  }
  return { rules, malformed };
}

export function lint(markdown: string, conflictsMd: string, tags: string[]): { errors: string[]; warnings: string[] } {
  const errors: string[] = [];
  const warnings: string[] = [];
  const { rules, malformed } = parseRules(markdown);
  for (const m of malformed) errors.push(`malformed rule row: ${m}`);
  const known = new Set(tags);
  const conflictIds = new Set([...conflictsMd.matchAll(/^\| (C-\d{2}) \|/gm)].map((m) => m[1]!));
  const seen = new Set<string>();
  const usedTags = new Set<string>();
  const usedConflicts = new Set<string>();
  for (const r of rules) {
    if (seen.has(r.id)) errors.push(`${r.id}: duplicate id`);
    seen.add(r.id);
    const tagsInRow = [...r.source.matchAll(/(?:^|; )([A-Z]{3,6}) /g)].map((m) => m[1]!);
    const internal = /(?:^|; )(STANDARD|CONTRIBUTING|owner) /.test(r.source);
    if (!tagsInRow.length && !internal) errors.push(`${r.id}: no source tag`);
    for (const t of tagsInRow) { usedTags.add(t); if (!known.has(t)) errors.push(`${r.id}: unknown source tag ${t}`); }
    for (const part of r.source.split("; ")) if (!/^[A-Z]{3,6} \S/.test(part) && !/^(STANDARD|CONTRIBUTING|owner)/.test(part)) errors.push(`${r.id}: source part without a section: ${part}`);
    for (const c of r.conflicts) { usedConflicts.add(c); if (!conflictIds.has(c)) errors.push(`${r.id}: unknown conflict ${c}`); }
    if (r.status === "rejected" && !r.conflicts.length) errors.push(`${r.id}: rejected without a conflict row`);
    for (const e of r.enforced) if (!(ENFORCERS as readonly string[]).includes(e)) errors.push(`${r.id}: unknown enforcer ${e}`);
    if (/[A-Za-z]:\\|\/home\/|\/Users\//.test(r.rule)) errors.push(`${r.id}: absolute host path in rule`);
  }
  for (const t of known) if (!usedTags.has(t)) warnings.push(`tag ${t} is defined but no rule cites it`);
  for (const c of conflictIds) if (!usedConflicts.has(c)) warnings.push(`${c} is not cited by any rule`);
  if (!rules.length) errors.push("no rules found");
  return { errors, warnings };
}

if (import.meta.main) {
  const root = resolve(process.argv[2] ?? join(import.meta.dir, ".."));
  const dir = join(root, "docs/standards");
  const sources = Bun.TOML.parse(await Bun.file(join(dir, "sources.toml")).text()) as { tags?: Record<string, string[]> };
  const { errors, warnings } = lint(await Bun.file(join(dir, "code-style.md")).text(), await Bun.file(join(dir, "conflicts.md")).text(), Object.keys(sources.tags ?? {}));
  for (const w of warnings) console.warn(`warning: ${w}`);
  for (const e of errors) console.error(`error: ${e}`);
  console.log(`${errors.length} errors, ${warnings.length} warnings`);
  process.exit(errors.length ? 1 : 0);
}
