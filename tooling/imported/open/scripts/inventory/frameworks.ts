// Ecosystem inventory from package registries and GitHub. Input: frameworks.json (categories and their sources).
// Sources: crates.io (category / keyword), npm search, NuGet search, GitHub repository search. Registry hits are
// libraries by construction; GitHub hits must look like a library (`libraryPattern` on description or topics).
// Records merge on the GitHub repository when known and are enriched with stars, last push, licence; archived
// repositories are dropped. Output: docs/ideas/inventory/frameworks.{json,md}.
// bun scripts/inventory/frameworks.ts
import { join } from "node:path";

const root = join(import.meta.dir, "../..");
const cfg = await Bun.file(join(import.meta.dir, "frameworks.json")).json();
const UA = "aphrody-open-inventory (github.com/aphrody-labs/open)";
const token = Bun.env.GH_TOKEN ?? Bun.spawnSync(["gh", "auth", "token"]).stdout.toString().trim();
const gh: Record<string, string> = { accept: "application/vnd.github+json", "user-agent": UA, ...(token ? { authorization: `Bearer ${token}` } : {}) };
const libraryRe = new RegExp(cfg.libraryPattern, "i");
// categories that keep every repository their queries return (no library filter, no star floor)
const everyRepository = new Set<string>(cfg.categories.filter((c: { everyRepository?: boolean }) => c.everyRepository).map((c: { id: string }) => c.id));

async function getJson(url: string, headers: Record<string, string> = { "user-agent": UA }, pause = 0): Promise<any> {
  for (let attempt = 0; attempt < 6; attempt++) {
    const r = await fetch(url, { headers, signal: AbortSignal.timeout(30_000) }).catch(() => null);
    if (r?.ok) {
      if (pause) await Bun.sleep(pause);
      return r.json();
    }
    if (r && r.status !== 403 && r.status !== 429 && r.status < 500) return null;
    const reset = Number(r?.headers.get("x-ratelimit-reset") ?? 0) * 1000;
    await Bun.sleep(Math.max(2_000 * (attempt + 1), reset - Date.now() + 1_000));
  }
  return null;
}

const slugOf = (url?: string | null) => url?.match(/github\.com[/:]([^/]+\/[^/#?]+?)(?:\.git)?(?:[/#?]|$)/i)?.[1]?.toLowerCase() ?? null;

type Rec = {
  key: string; name: string; repo: string | null; description: string | null; language: string | null; stars: number | null;
  pushed: string | null; license: string | null; topics: string[]; archived: boolean;
  packages: { kind: string; name: string; downloads: number | null }[]; categories: Set<string>; sources: Set<string>;
};
const recs = new Map<string, Rec>();
const ghMeta = new Map<string, any>();

function add(cat: string, src: string, r: Partial<Rec> & { key: string; name: string }, pkg?: Rec["packages"][number]) {
  const e = recs.get(r.key) ?? { repo: null, description: null, language: null, stars: null, pushed: null, license: null, topics: [], archived: false, packages: [], categories: new Set(), sources: new Set(), ...r } as Rec;
  if (pkg && !e.packages.some((p) => p.kind === pkg.kind && p.name === pkg.name)) e.packages.push(pkg);
  e.description ??= r.description ?? null;
  e.categories.add(cat);
  e.sources.add(src);
  recs.set(e.key, e);
}

async function crates(cat: string, q: { category?: string; keyword?: string }) {
  const p = new URLSearchParams({ sort: "downloads", per_page: String(cfg.perQuery) });
  if (q.category) p.set("category", q.category);
  if (q.keyword) p.set("keyword", q.keyword);
  const src = `crates.io ${p}`;
  const d = await getJson(`https://crates.io/api/v1/crates?${p}`, { "user-agent": UA }, 1_000);
  for (const c of d?.crates ?? []) {
    const repo = slugOf(c.repository);
    add(cat, src, { key: repo ?? `crate:${c.name}`, name: c.name, repo, description: c.description }, { kind: "crate", name: c.name, downloads: c.downloads });
  }
  return d?.crates?.length ?? 0;
}

async function npm(cat: string, text: string) {
  const d = await getJson(`https://registry.npmjs.org/-/v1/search?text=${encodeURIComponent(text)}&size=${cfg.perQuery}&popularity=1.0`);
  for (const o of d?.objects ?? []) {
    const p = o.package;
    const repo = slugOf(p.links?.repository);
    add(cat, `npm ${text}`, { key: repo ?? `npm:${p.name}`, name: p.name, repo, description: p.description }, { kind: "npm", name: p.name, downloads: o.downloads?.monthly ?? null });
  }
  return d?.objects?.length ?? 0;
}

async function nuget(cat: string, q: string) {
  const d = await getJson(`https://azuresearch-usnc.nuget.org/query?q=${encodeURIComponent(q)}&take=${cfg.perQuery}`);
  for (const p of d?.data ?? []) {
    const repo = slugOf(p.projectUrl);
    add(cat, `nuget ${q}`, { key: repo ?? `nuget:${p.id}`, name: p.id, repo, description: p.description }, { kind: "nuget", name: p.id, downloads: p.totalDownloads });
  }
  return d?.data?.length ?? 0;
}

async function github(cat: string, q: string) {
  const d = await getJson(`https://api.github.com/search/repositories?q=${encodeURIComponent(q)}&sort=stars&order=desc&per_page=${cfg.perQuery}`, gh, 2_100);
  let kept = 0;
  for (const r of d?.items ?? []) {
    const slug = r.full_name.toLowerCase();
    ghMeta.set(slug, r);
    const looksLibrary = everyRepository.has(cat) || libraryRe.test(`${r.description ?? ""} ${(r.topics ?? []).join(" ")}`);
    if (r.archived || !looksLibrary || (!everyRepository.has(cat) && r.stargazers_count < cfg.minStars)) continue;
    add(cat, `github ${q}`, { key: slug, name: r.full_name, repo: slug, description: r.description });
    kept++;
  }
  return kept;
}

// --render: rebuild frameworks.md from an existing frameworks.json without querying the registries again
const renderOnly = Bun.argv.includes("--render");
for (const c of renderOnly ? [] : cfg.categories) {
  for (const s of c.sources) {
    const n = s.crates ? await crates(c.id, s.crates) : s.npm ? await npm(c.id, s.npm) : s.nuget ? await nuget(c.id, s.nuget) : await github(c.id, s.github);
    console.log(`${c.id}: ${JSON.stringify(s)} -> ${n}`);
  }
}

// GitHub metadata for every record that points to a repository not seen in a search result
for (const e of recs.values()) {
  if (!e.repo) continue;
  const m = ghMeta.get(e.repo) ?? (await getJson(`https://api.github.com/repos/${e.repo}`, gh));
  if (!m) continue;
  ghMeta.set(e.repo, m);
  Object.assign(e, { stars: m.stargazers_count, pushed: m.pushed_at?.slice(0, 10) ?? null, license: m.license?.spdx_id ?? null, language: m.language, topics: m.topics ?? [], archived: m.archived });
  e.description ??= m.description;
}

const records = renderOnly
  ? (await Bun.file(join(root, "docs/ideas/inventory/frameworks.json")).json()).records.map((r: any) => ({ ...r, name: r.repo ?? r.name }))
  : [...recs.values()]
  .filter((e) => !e.archived)
  .map((e) => ({ ...e, name: e.repo ?? e.name, categories: [...e.categories], sources: [...e.sources], url: e.repo ? `https://github.com/${e.repo}` : null }))
  .sort((a, b) => (b.stars ?? -1) - (a.stars ?? -1));

const outDir = join(root, "docs/ideas/inventory");
await Bun.write(join(outDir, "frameworks.json"), `${JSON.stringify({ config: "scripts/inventory/frameworks.json", records }, null, 2)}\n`);
const cell = (s: string | null) => (s ?? "").replace(/\|/g, "\\|").replace(/\s+/g, " ").slice(0, 100);
const md = ["# Ecosystem inventory", "", "Generated by `bun scripts/inventory/frameworks.ts` (crates.io, npm, NuGet, GitHub search); sources in `scripts/inventory/frameworks.json`; archived repositories excluded. Full data: [frameworks.json](frameworks.json).", ""];
for (const c of cfg.categories) {
  const rows = records.filter((r) => r.categories.includes(c.id));
  md.push(`## ${c.id} (${c.equivalentOf.join(", ")}): ${rows.length}`, "", "| Project | Stars | Packages | Last push | License | Description |", "|---|---|---|---|---|---|");
  for (const r of rows) {
    const pk = r.packages.map((p) => `${p.kind}:${p.name}`).slice(0, 3).join(", ");
    md.push(`| ${r.url ? `[${r.name}](${r.url})` : r.name} | ${r.stars ?? "-"} | ${cell(pk)} | ${r.pushed ?? "-"} | ${r.license ?? "-"} | ${cell(r.description)} |`);
  }
  md.push("");
}
await Bun.write(join(outDir, "frameworks.md"), md.join("\n"));
console.log(`${records.length} records -> docs/ideas/inventory/frameworks.{json,md}`);
