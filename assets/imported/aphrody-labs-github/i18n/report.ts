#!/usr/bin/env bun
// Assemble the fleet-wide i18n report from check.ts JSON outputs.
//
//   bun i18n/report.ts --before-dir D --after-dir D [--remote before.json] [--remote-after after.json] \
//       [--report-only ai-os,aphrody,...] --out i18n-report.json
//
// Local JSON files are named before-<repo>.json, fix-<repo>.json (--fix run) and after-<repo>.json (check.ts --json). Remote files come from scan-remote.ts.
import { existsSync, readdirSync } from "node:fs";
import { join } from "node:path";
import type { Finding, RepoResult } from "./check";

type Counts = Record<string, number>;
export interface RepoEntry {
  name: string;
  where: "pc" | "fr1";
  owner: "editable" | "report-only";
  kind: "first-party" | "fork" | "mirror";
  files: number;
  before: Counts;
  after: Counts;
  findings: Finding[];
  truncated: number;
  fixed: string[];
}

const arg = (name: string, fallback?: string) => {
  const i = process.argv.indexOf(name);
  return i >= 0 ? process.argv[i + 1] : fallback;
};

export const total = (c: Counts) => Object.values(c).reduce((a, b) => a + b, 0);
export function sumCounts(list: Counts[]): Counts {
  const out: Counts = {};
  for (const c of list) for (const [k, v] of Object.entries(c)) out[k] = (out[k] ?? 0) + v;
  return out;
}

const readJson = async <T>(path: string): Promise<T> => JSON.parse(await Bun.file(path).text());

async function localDir(dir: string, prefix: string): Promise<Map<string, RepoResult & { before?: Counts }>> {
  const out = new Map<string, RepoResult & { before?: Counts }>();
  for (const f of readdirSync(dir).filter((f) => f.startsWith(prefix) && f.endsWith(".json"))) {
    const name = f.slice(prefix.length, -".json".length);
    const [res] = await readJson<(RepoResult & { before?: Counts })[]>(join(dir, f));
    out.set(name, res);
  }
  return out;
}

async function remote(path?: string) {
  const out = new Map<string, RepoResult & { before?: Counts }>();
  if (!path || !(await Bun.file(path).exists())) return { repos: out, skipped: [] as unknown[], failed: [] as string[] };
  const j = await readJson<{ repos: (RepoResult & { repo: string; before?: Counts })[]; skipped: unknown[]; failed: string[] }>(path);
  for (const r of j.repos) out.set(r.repo, r);
  return { repos: out, skipped: j.skipped, failed: j.failed };
}

export async function main() {
  const beforeDir = arg("--before-dir")!;
  const afterDir = arg("--after-dir")!;
  const reportOnly = new Set((arg("--report-only", "") ?? "").split(",").filter(Boolean));
  const out = arg("--out")!;
  const srcRoot = arg("--src-root", "C:/src")!;
  const meta = new Map<string, { isFork: boolean; description: string }>();
  for (const owner of ["aphrody-labs", "rose-griffon"]) {
    const list = JSON.parse(Bun.spawnSync(["gh", "repo", "list", owner, "--limit", "1000", "--json", "name,isFork,description"]).stdout.toString());
    for (const r of list) meta.set(`${owner}/${r.name}`, { isFork: r.isFork, description: r.description ?? "" });
  }
  const kindOf = (full: string): RepoEntry["kind"] => {
    const m = meta.get(full);
    return m?.isFork ? "fork" : /mirror/i.test(m?.description ?? "") ? "mirror" : "first-party";
  };

  const entries: RepoEntry[] = [];
  const before = await localDir(beforeDir, "before-");
  const after = await localDir(afterDir, "after-");
  const fixes = await localDir(beforeDir, "fix-");
  const origin = (name: string) => {
    const url = Bun.spawnSync(["git", "-C", `${srcRoot}/${name}`, "remote", "get-url", "origin"]).stdout.toString().trim();
    return /github\.com[/:]([^/]+\/[^/]+?)(?:\.git)?$/.exec(url)?.[1] ?? name;
  };
  for (const [name, b] of before) {
    if (!existsSync(`${srcRoot}/${name}/.git`)) continue; // files of other agents sharing the directory
    const a = after.get(name) ?? b;
    const full = origin(name);
    const owner = reportOnly.has(name) ? "report-only" : "editable";
    entries.push({ name: full, where: "pc", owner, kind: kindOf(full), files: a.files, before: b.counts, after: a.counts, findings: a.findings, truncated: a.truncated, fixed: fixes.get(name)?.fixed ?? [] });
  }
  const rb = await remote(arg("--remote"));
  const ra = await remote(arg("--remote-after"));
  const localNames = new Set(entries.map((e) => e.name.toLowerCase()));
  for (const [full, b] of rb.repos) {
    if (localNames.has(full.toLowerCase())) continue; // the PC checkout is the working copy
    const a = ra.repos.get(full) ?? b;
    const kind = kindOf(full);
    entries.push({ name: full, where: "fr1", owner: kind === "first-party" && !reportOnly.has(full.split("/")[1]) ? "editable" : "report-only", kind, files: a.files, before: b.counts, after: a.counts, findings: a.findings, truncated: a.truncated, fixed: ra.repos.get(full)?.fixed ?? [] });
  }
  entries.sort((x, y) => total(y.before) - total(x.before));
  const sel = (f: (e: RepoEntry) => boolean) => entries.filter(f);
  const sums = (list: RepoEntry[]) => ({ repos: list.length, before: sumCounts(list.map((e) => e.before)), after: sumCounts(list.map((e) => e.after)) });
  const report = {
    generated: new Date().toISOString(),
    standard: "https://github.com/aphrody-labs/.github/tree/main/i18n",
    sources: [
      "https://www.w3.org/TR/international-specs/ (Group Note, 7 Aug 2026, read 2026-11-17)",
      "https://www.w3.org/TR/charmod/ (Recommendation, 15 Feb 2005, read 2026-11-17)",
      "https://www.w3.org/TR/string-meta/ (First Public Working Draft, 16 Jul 2026, read 2026-11-17)",
      "https://www.w3.org/TR/timezone/ (Group Draft Note, 26 Jul 2025, read 2026-11-17)",
    ],
    summary: {
      all: sums(entries),
      editable: sums(sel((e) => e.owner === "editable")),
      reportOnly: sums(sel((e) => e.owner === "report-only")),
    },
    notScanned: { skippedByScanRemote: rb.skipped, failed: rb.failed },
    repos: entries,
  };
  await Bun.write(out, `${JSON.stringify(report, null, 2)}\n`);
  console.error(`${entries.length} repos -> ${out}`);
}

if (import.meta.main) await main();
