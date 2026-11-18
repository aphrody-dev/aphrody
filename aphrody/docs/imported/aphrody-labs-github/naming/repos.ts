#!/usr/bin/env bun
// naming-ignore-file
// Audits GitHub repository names of the family orgs against naming.toml, live from the GitHub API (gh).
// Usage: bun naming/repos.ts [--json] [--org aphrody-labs] [--scheme naming.toml]
// Originals are checked against their kind's rule; forks and mirrors only need to keep an upstream-style name
// (lowercase is not required: upstream decides); archived repositories are listed, never judged.
import { loadScheme, type Scheme } from "./check.ts";

export interface Repo {
  name: string;
  nameWithOwner: string;
  isFork: boolean;
  isArchived: boolean;
  isPrivate?: boolean;
  description?: string | null;
  parent?: { name: string; owner: { login: string } } | null;
}
export interface Row {
  slug: string;
  cls: "original" | "fork" | "mirror" | "archived";
  kind?: string;
  verdict: "ok" | "violation" | "unregistered";
  why?: string;
}

const MIRROR = /^(mirror|read-only mirror|gnu mirror)\b/i;

export function classify(r: Repo, scheme?: Scheme): Row["cls"] {
  if (!r.isArchived && scheme && Object.keys(scheme.repos).some(k => k.toLowerCase() === r.nameWithOwner.toLowerCase())) return "original"; // registered: the product, even when GitHub still flags it as a fork
  if (r.isArchived) return "archived";
  if (r.isFork) return "fork";
  if (MIRROR.test(r.description ?? "") || /\bmirror of\b/i.test(r.description ?? "")) return "mirror";
  return "original";
}

export function auditRepo(r: Repo, scheme: Scheme): Row {
  const cls = classify(r, scheme);
  const slug = r.nameWithOwner;
  const [org, name] = slug.split("/");
  if (cls === "archived") return { slug, cls, verdict: "ok" };
  if (cls === "fork" || cls === "mirror") {
    const p = r.parent;
    if (!p) return { slug, cls, kind: cls, verdict: "ok" };
    const n = name.toLowerCase();
    const up = p.name.toLowerCase();
    const owner = p.owner.login.toLowerCase();
    const aliases = Object.entries(scheme.forks?.aliases ?? {}).filter(([, v]) => v.toLowerCase() === owner).map(([k]) => k.toLowerCase());
    if (n === up || n === `${owner}-${up}` || aliases.some(a => n === `${a}-${up}`)) return { slug, cls, kind: cls, verdict: "ok" };
    if (/-\d+$/.test(n) && n.replace(/-\d+$/, "") === up) return { slug, cls, kind: cls, verdict: "violation", why: `numeric suffix: use ${owner}-${up}` };
    if (n.includes(up) || n.includes(owner)) return { slug, cls, kind: cls, verdict: "ok" }; // a descriptive qualifier around the upstream name
    return { slug, cls, kind: cls, verdict: "violation", why: `fork of ${owner}/${p.name}: use ${p.name}, or ${owner}-${p.name} on a collision` };
  }
  const kind = scheme.repos[slug] ?? Object.entries(scheme.repos).find(([k]) => k.toLowerCase() === slug.toLowerCase())?.[1];
  if (!kind) return { slug, cls, verdict: "unregistered", why: "original repository without a kind in naming.toml [repos]" };
  const k = scheme.kinds[kind];
  if (!k) return { slug, cls, kind, verdict: "violation", why: `unknown kind ${kind}` };
  if (!k.orgs.includes(org)) return { slug, cls, kind, verdict: "violation", why: `kind ${kind} belongs in ${k.orgs.join(" or ")}` };
  if (k.pattern !== "upstream" && k.pattern !== "any" && !/^[a-z0-9.-]+$/.test(name) && name !== ".github") return { slug, cls, kind, verdict: "violation", why: "repository names are lowercase kebab-case" };
  if (k.pattern !== "upstream" && k.pattern !== "any" && !new RegExp(k.pattern).test(name)) return { slug, cls, kind, verdict: "violation", why: `does not match ${k.pattern}` };
  if (kind === "family") {
    const fam = Object.entries(scheme.families ?? {}).find(([, f]) => f.root === name || f.members.includes(name));
    if (!fam) return { slug, cls, kind, verdict: "violation", why: "family member that is not listed under [families]" };
  }
  return { slug, cls, kind, verdict: "ok" };
}

export async function listRepos(org: string): Promise<Repo[]> {
  const p = Bun.spawn(["gh", "repo", "list", org, "--limit", "2000", "--json", "name,nameWithOwner,isFork,isArchived,isPrivate,description,parent"], { stdout: "pipe", stderr: "pipe" });
  const out = await new Response(p.stdout).text();
  if ((await p.exited) !== 0) return [];
  return JSON.parse(out);
}

if (import.meta.main) {
  const args = process.argv.slice(2);
  const val = (n: string) => (args.includes(n) ? args[args.indexOf(n) + 1] : undefined);
  const scheme = await loadScheme(val("--scheme"));
  const orgs = val("--org") ? [val("--org")!] : [...new Set(scheme.meta.orgs)].filter(o => o !== "rosegriffon");
  const rows: Row[] = [];
  for (const org of orgs) for (const r of await listRepos(org)) rows.push(auditRepo(r, scheme));
  if (args.includes("--json")) console.log(JSON.stringify(rows, null, 2));
  else {
    for (const r of rows.filter(r => r.verdict !== "ok")) console.log(`${r.verdict.padEnd(12)} ${r.slug} (${r.cls}${r.kind ? ", " + r.kind : ""}) ${r.why ?? ""}`);
    const n = (c: string) => rows.filter(r => r.cls === c).length;
    console.log(`\n${rows.length} repositories: ${n("original")} originals, ${n("fork")} forks, ${n("mirror")} mirrors, ${n("archived")} archived; ${rows.filter(r => r.verdict === "violation").length} violations, ${rows.filter(r => r.verdict === "unregistered").length} unregistered`);
  }
}
