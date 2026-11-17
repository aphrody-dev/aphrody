// Derived views, generated deterministically from the store: same store and same `now`, byte-identical files.
import { mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import type { Kernel, Manifest } from "./manifest.ts";
import type { Store, StoredItem } from "./store.ts";
import { buildCycles, closenessKey, compareVersions, distance, formatVersion, parseVersion, type Distance, type KernelVersion } from "./version.ts";
import type { VerifiedFile } from "./verify.ts";

export interface VersionEntry extends Distance {
  kernel: string;
  kernelName: string;
  role: string;
  channel: string;
  version: string;
  normalized: string;
  /** Epoch seconds at which the watch first saw this exact version in this channel. */
  since: number;
  source: string;
}
export interface VersionsView {
  asOf: number;
  mainline: string | null;
  stable: string | null;
  longterm: string[];
  /** Every observed channel, closest to mainline first. */
  entries: VersionEntry[];
  /** One row per downstream kernel: its newest channel. */
  distros: VersionEntry[];
}
export interface Window {
  label: string;
  since: number;
  until: number;
  counts: Record<string, number>;
  updated: { issue: number; pr: number };
  mergedPrs: number;
  commitsLowerBound: boolean;
  versionChanges: { channel: string; version: string; at: number }[];
  metrics: Record<string, { current: number | null; delta: number | null }>;
  notable: { kind: string; title: string; url: string; time: number }[];
}
export interface Digest {
  kernel: string;
  windows: Window[];
}
export interface GenerateOptions {
  outDir: string;
  /** Epoch seconds the windows end at; default is the last poll rounded up to the hour. */
  now?: number;
  verifiedPath?: string;
}
export interface Generated {
  files: string[];
  versions: VersionsView;
  versionsTable: string;
  digests: Digest[];
  contents: Map<string, string>;
}

const HOUR = 3600;
const byId = (a: { id: string }, b: { id: string }): number => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0);
const WINDOWS: [string, number][] = [["24h", 86400], ["7d", 7 * 86400]];
const COUNTED = ["commit", "release", "issue", "pr", "discussion", "mail", "tag", "package", "news", "build"];

export const iso = (epoch: number): string => new Date(epoch * 1000).toISOString().replace(/:\d\d\.\d+Z$/, "Z");
const cell = (text: string, max = 90): string => {
  const flat = text.replace(/\s+/g, " ").replace(/\|/g, "\\|").trim();
  return flat.length > max ? `${flat.slice(0, max - 1)}…` : flat;
};
const link = (text: string, url: string): string => (url ? `[${cell(text)}](${url})` : cell(text));
const json = (value: unknown): string => `${JSON.stringify(value, null, 2)}\n`;
const signed = (n: number | null): string => (n === null ? "n/a" : n > 0 ? `+${n}` : String(n));

export function computeVersions(store: Store, manifest: Manifest, asOf: number): VersionsView {
  const kernels = new Map(manifest.kernels.map((k) => [k.id, k]));
  const rows = store.versionRows().filter((r) => kernels.get(r.kernel)?.lineage === "linux");
  const parsed = new Map<string, KernelVersion>();
  for (const r of rows) {
    const v = parseVersion(r.version);
    if (v) parsed.set(`${r.source}\u0000${r.channel}`, v);
  }
  const pick = (channel: string): KernelVersion | null => parsed.get(`linux-releases\u0000${channel}`) ?? null;
  const rc = pick("mainline");
  const stable = pick("stable");
  const longterm = rows
    .filter((r) => r.source === "linux-releases" && r.channel.startsWith("longterm-"))
    .map((r) => r.version)
    .sort((a, b) => compareVersions(parseVersion(a)!, parseVersion(b)!));
  const empty: VersionsView = { asOf, mainline: rc ? formatVersion(rc) : null, stable: stable ? formatVersion(stable) : null, longterm, entries: [], distros: [] };
  if (!rc || !stable) return empty;

  const tags = store.items({ source: "linux-tags", kind: "tag" }).map((t) => t.id);
  const cycles = buildCycles(tags, [...parsed.values()]);
  const entries: VersionEntry[] = [];
  for (const r of rows) {
    const v = parsed.get(`${r.source}\u0000${r.channel}`);
    const kernel = kernels.get(r.kernel);
    if (!v || !kernel) continue;
    entries.push({
      kernel: r.kernel, kernelName: kernel.name, role: kernel.role, channel: r.channel, version: r.version, normalized: formatVersion(v),
      since: r.since, source: r.source, ...distance(v, rc, stable, cycles),
    });
  }
  const byCloseness = (a: VersionEntry, b: VersionEntry): number => {
    const ka = closenessKey(a), kb = closenessKey(b);
    return ka[0] - kb[0] || ka[1] - kb[1] || ka[2] - kb[2] || (a.kernel < b.kernel ? -1 : a.kernel > b.kernel ? 1 : 0) || (a.channel < b.channel ? -1 : a.channel > b.channel ? 1 : 0);
  };
  entries.sort(byCloseness);
  const newest = new Map<string, VersionEntry>();
  for (const e of entries) {
    if (e.role !== "downstream") continue;
    const best = newest.get(e.kernel);
    if (!best || compareVersions(parseVersion(e.version)!, parseVersion(best.version)!) > 0) newest.set(e.kernel, e);
  }
  return { ...empty, entries, distros: [...newest.values()].sort(byCloseness) };
}

function distanceText(e: VersionEntry): string {
  if (e.relation === "ahead") return `ahead by ${-e.cyclesBehindRc}`;
  if (e.cyclesBehindRc === 0) return e.rcSteps === 0 ? "at rc" : `${e.rcSteps} rc behind`;
  return `${e.cyclesBehindRc} cycle${e.cyclesBehindRc === 1 ? "" : "s"}`;
}
function stableText(e: VersionEntry): string {
  if (e.cyclesBehindStable === 0) return e.patchesBehindStable === null ? "rc of next" : e.patchesBehindStable === 0 ? "current" : `${e.patchesBehindStable} patch${e.patchesBehindStable === 1 ? "" : "es"}`;
  return e.cyclesBehindStable < 0 ? `${-e.cyclesBehindStable} cycle${e.cyclesBehindStable === -1 ? "" : "s"} ahead` : `${e.cyclesBehindStable} cycles`;
}

export function versionsTable(view: VersionsView, rows: VersionEntry[] = view.distros, firstSeen = true): string {
  const head = `| Rank | Kernel | Channel | Version | Behind mainline rc | Behind stable |${firstSeen ? " First seen |" : ""}\n| --: | :-- | :-- | :-- | :-- | :--${firstSeen ? " | :-- |" : " |"}`;
  const body = rows.map((e, i) => `| ${i + 1} | ${cell(e.kernelName)} | ${cell(e.channel)} | ${cell(e.version, 40)} | ${distanceText(e)} | ${stableText(e)} |${firstSeen ? ` ${iso(e.since)} |` : ""}`);
  return [head, ...body].join("\n");
}

export function computeDigests(store: Store, manifest: Manifest, asOf: number): Digest[] {
  const sourceKernel = new Map(manifest.sources.map((s) => [s.id, s.kernel]));
  const history = store.versionHistory();
  const changes = new Map<string, { source: string; channel: string; version: string; at: number }[]>();
  const seen = new Map<string, string>();
  for (const h of history) {
    const key = `${h.source}\u0000${h.channel}`;
    if (seen.has(key)) {
      const kernel = sourceKernel.get(h.source);
      if (kernel) changes.set(kernel, [...(changes.get(kernel) ?? []), h]);
    }
    seen.set(key, h.version);
  }
  const metricRows = store.metricNames();

  return manifest.kernels.map((kernel): Digest => {
    const items = store.items({ kernel: kernel.id });
    const commitSources = manifest.sources.filter((s) => s.kernel === kernel.id && s.yields.includes("commit"));
    const oldestCommit = (source: string): number | null => {
      const times = items.filter((i) => i.source === source && i.kind === "commit").map((i) => i.time);
      return times.length ? Math.min(...times) : null;
    };
    return {
      kernel: kernel.id,
      windows: WINDOWS.map(([label, seconds]): Window => {
        const since = asOf - seconds;
        const inWindow = items.filter((i) => i.time > 0 && i.time >= since && i.time <= asOf);
        const counts: Record<string, number> = {};
        for (const kind of COUNTED) counts[kind] = inWindow.filter((i) => i.kind === kind).length;
        const touched = (kind: string): number => items.filter((i) => i.kind === kind && i.time < since && (i.updated ?? 0) >= since && (i.updated ?? 0) <= asOf).length;
        const merged = items.filter((i) => i.kind === "pr" && i.meta?.merged === true && (i.updated ?? i.time) >= since && (i.updated ?? i.time) <= asOf);
        const versionChanges = (changes.get(kernel.id) ?? []).filter((c) => c.at >= since && c.at <= asOf).map((c) => ({ channel: c.channel, version: c.version, at: c.at }))
          .sort((a, b) => b.at - a.at || (a.channel < b.channel ? -1 : 1));
        const metrics: Window["metrics"] = {};
        for (const m of metricRows.filter((r) => r.kernel === kernel.id)) {
          const { current, past } = store.metric(m.source, m.name, since);
          metrics[m.name] = { current, delta: current !== null && past !== null ? current - past : null };
        }
        const notable = [
          ...inWindow.filter((i) => i.kind === "release" || i.kind === "tag" && i.source !== "linux-tags").slice(0, 5),
          ...merged.slice(0, 5),
          ...inWindow.filter((i) => i.kind === "discussion").slice(0, 3),
          ...inWindow.filter((i) => i.kind === "issue").slice(0, 3),
          ...inWindow.filter((i) => i.kind === "commit").slice(0, 5),
        ]
          .map((i) => ({ kind: i.kind, title: cell(i.title, 120), url: i.url, time: i.time }))
          .filter((n, index, all) => all.findIndex((o) => o.url === n.url && o.kind === n.kind) === index)
          .sort((a, b) => b.time - a.time || (a.url < b.url ? -1 : 1));
        return {
          label, since, until: asOf, counts, updated: { issue: touched("issue"), pr: touched("pr") }, mergedPrs: merged.length,
          commitsLowerBound: commitSources.some((s) => { const oldest = oldestCommit(s.id); return oldest === null || oldest > since; }),
          versionChanges, metrics, notable,
        };
      }),
    };
  });
}

function sortedRepos(items: StoredItem[], limit: number): StoredItem[] {
  return items.filter((i) => i.kind === "repo").sort((a, b) => (b.updated ?? b.time) - (a.updated ?? a.time) || (a.id < b.id ? -1 : 1)).slice(0, limit);
}

function kernelPage(kernel: Kernel, manifest: Manifest, digest: Digest, view: VersionsView, store: Store, verified: VerifiedFile | null): string {
  const items = store.items({ kernel: kernel.id });
  const lines: string[] = [`# ${kernel.name}`, "", kernel.description, "", `Generated from the watch store as of ${iso(digest.windows[0]!.until)}. Do not edit; run \`bun tooling/kernel-watch/cli.ts build\`.`, ""];

  const mine = view.entries.filter((e) => e.kernel === kernel.id);
  if (mine.length) lines.push("## Kernel versions", "", versionsTable(view, mine), "");

  lines.push("## Activity", "", "| Window | Commits | Releases | Issues opened | PRs opened | PRs merged | Discussions | Mail | Tags |", "| :-- | --: | --: | --: | --: | --: | --: | --: | --: |");
  for (const w of digest.windows) {
    const commits = `${w.commitsLowerBound ? "≥" : ""}${w.counts.commit}`;
    lines.push(`| ${w.label} | ${commits} | ${w.counts.release} | ${w.counts.issue} | ${w.counts.pr} | ${w.mergedPrs} | ${w.counts.discussion} | ${w.counts.mail} | ${w.counts.tag} |`);
  }
  lines.push("", "A leading ≥ marks a lower bound: the polled feed page does not reach back to the start of the window.", "");
  const metricNames = Object.keys(digest.windows[0]!.metrics).sort();
  if (metricNames.length) {
    lines.push("| Metric | Now | 24h delta | 7d delta |", "| :-- | --: | --: | --: |");
    for (const name of metricNames) lines.push(`| ${name} | ${digest.windows[0]!.metrics[name]!.current ?? "n/a"} | ${signed(digest.windows[0]!.metrics[name]!.delta)} | ${signed(digest.windows[1]!.metrics[name]?.delta ?? null)} |`);
    lines.push("");
  }
  const changed = digest.windows[1]!.versionChanges;
  if (changed.length) lines.push("## Version changes (7d)", "", ...changed.map((c) => `- ${iso(c.at)} ${cell(c.channel)}: ${c.version}`), "");
  const notable = digest.windows[1]!.notable;
  if (notable.length) lines.push("## Notable (7d)", "", ...notable.map((n) => `- ${iso(n.time)} ${n.kind}: ${link(n.title, n.url)}`), "");

  const releases = items.filter((i) => i.kind === "release").slice(0, 5);
  if (releases.length) lines.push("## Latest releases", "", ...releases.map((r) => `- ${iso(r.time)} ${link(r.title, r.url)}`), "");
  const repos = sortedRepos(items, 15);
  if (repos.length) lines.push("## Organisation repositories", "", "| Repository | Stars | Pushed | Description |", "| :-- | --: | :-- | :-- |", ...repos.map((r) => `| ${link(r.id, r.url)} | ${r.meta?.stars ?? ""} | ${iso(r.updated ?? r.time).slice(0, 10)} | ${cell(r.body, 80)} |`), "");

  lines.push("## Sites and organisations", "", "| Name | Kind | URL |", "| :-- | :-- | :-- |");
  for (const s of kernel.sites) lines.push(`| ${s.id} | ${s.kind} | ${s.url}${s.note ? ` (${cell(s.note, 80)})` : ""} |`);
  for (const o of kernel.orgs) lines.push(`| ${o.id} | ${o.provider} org | ${o.url} |`);

  lines.push("", "## Sources", "", "| Source | Kind | Yields | Interval | Verified |", "| :-- | :-- | :-- | --: | :-- |");
  for (const s of manifest.sources.filter((x) => x.kernel === kernel.id).sort(byId)) {
    const v = verified?.sources.find((x) => x.id === s.id);
    lines.push(`| [${s.id}](${s.url}) | ${s.kind} | ${s.yields.join(", ")} | ${Math.round(s.minInterval / 60)}m | ${v ? `${v.status ?? "error"}` : "n/a"} |`);
  }
  return `${lines.join("\n")}\n`;
}

function stateOfKernels(manifest: Manifest, view: VersionsView, digests: Digest[], store: Store, asOf: number): string {
  const lines = [
    "# State of the kernels", "",
    `As of ${iso(asOf)}. Generated by tooling/kernel-watch; condensed from docs/watch. Query the store for detail: \`bun tooling/kernel-watch/cli.ts search "<text>"\`.`, "",
    "## Mainline reference", "",
    view.mainline ? `- Mainline: ${view.mainline}` : "- Mainline: not observed yet",
    view.stable ? `- Stable: ${view.stable}` : "- Stable: not observed yet",
    ...(view.longterm.length ? [`- Longterm: ${view.longterm.join(", ")}`] : []), "",
  ];
  if (view.distros.length) {
    lines.push("## Distributions closest to the mainline rc", "", versionsTable(view, view.distros, false), "", "Behind mainline rc counts mainline cycles (0 cycles and 0 rc: tracking the rc itself). Channel is the newest kernel the distribution ships.", "");
  }
  lines.push("## Kernels", "", "| Kernel | Latest release | Commits 24h | PRs merged 7d | Open PRs | Open issues |", "| :-- | :-- | --: | --: | --: | --: |");
  for (const kernel of manifest.kernels) {
    const digest = digests.find((d) => d.kernel === kernel.id)!;
    const release = store.items({ kernel: kernel.id, kind: "release", limit: 1 })[0] ?? store.items({ kernel: kernel.id, kind: "tag", limit: 1 }).find((t) => t.source !== "linux-tags");
    const day = digest.windows[0]!;
    const week = digest.windows[1]!;
    const commits = `${day.commitsLowerBound ? "≥" : ""}${day.counts.commit}`;
    lines.push(`| ${kernel.name} | ${release ? cell(release.version ?? release.title, 40) : "-"} | ${commits} | ${week.mergedPrs} | ${day.metrics.open_prs?.current ?? "-"} | ${day.metrics.open_issues?.current ?? "-"} |`);
  }
  lines.push("", "Pages: docs/watch/versions.md, docs/watch/kernels/<id>.md; machine-readable: docs/watch/data/*.json.", "");
  return lines.join("\n");
}

export async function generate(store: Store, manifest: Manifest, options: GenerateOptions): Promise<Generated> {
  const polled = Math.max(0, ...store.allStates().map((s) => s.lastPolled ?? 0));
  const asOf = options.now ?? Math.ceil(polled / HOUR) * HOUR;
  const verifiedFile = options.verifiedPath ? Bun.file(options.verifiedPath) : null;
  const verified: VerifiedFile | null = verifiedFile && (await verifiedFile.exists()) ? JSON.parse(await verifiedFile.text()) : null;
  const view = computeVersions(store, manifest, asOf);
  const digests = computeDigests(store, manifest, asOf);
  const contents = new Map<string, string>();

  const strip = (e: VersionEntry): Record<string, unknown> => ({ ...e, since: iso(e.since) });
  contents.set("data/versions.json", json({ asOf: iso(asOf), mainline: view.mainline, stable: view.stable, longterm: view.longterm, distros: view.distros.map(strip), entries: view.entries.map(strip) }));
  contents.set("data/digests.json", json({ asOf: iso(asOf), digests: digests.map((d) => ({ kernel: d.kernel, windows: d.windows.map((w) => ({ ...w, since: iso(w.since), until: iso(w.until), versionChanges: w.versionChanges.map((c) => ({ ...c, at: iso(c.at) })), notable: w.notable.map((n) => ({ ...n, time: iso(n.time) })) })) })) }));
  contents.set("data/kernels.json", json({
    asOf: iso(asOf),
    kernels: manifest.kernels.map((k) => ({
      id: k.id, name: k.name, role: k.role, lineage: k.lineage, description: k.description, sites: k.sites, orgs: k.orgs,
      sources: manifest.sources.filter((s) => s.kernel === k.id).sort(byId).map((s) => ({ id: s.id, kind: s.kind, url: s.url, yields: s.yields })),
      latestReleases: store.items({ kernel: k.id, kind: "release", limit: 3 }).map((r) => ({ title: r.title, version: r.version ?? null, url: r.url, time: iso(r.time) })),
      repositories: sortedRepos(store.items({ kernel: k.id, kind: "repo" }), 30).map((r) => ({ name: r.id, url: r.url, stars: r.meta?.stars ?? 0, pushed: iso(r.updated ?? r.time), description: r.body })),
    })),
  }));
  contents.set("data/sources.json", json({ verifiedAt: verified?.verifiedAt ?? null, sources: [...manifest.sources].sort(byId).map((s) => ({ id: s.id, kernel: s.kernel, kind: s.kind, adapter: s.adapter, url: s.url, yields: s.yields, minIntervalSeconds: s.minInterval, verifiedStatus: verified?.sources.find((v) => v.id === s.id)?.status ?? null })) }));

  const table = versionsTable(view);
  contents.set("versions.md", [
    "# Kernel versions", "",
    `Generated as of ${iso(asOf)} from the watch store. Reference: mainline ${view.mainline ?? "n/a"}, stable ${view.stable ?? "n/a"}${view.longterm.length ? `, longterm ${view.longterm.join(", ")}` : ""}.`, "",
    "## Newest kernel per distribution, closest to mainline first", "", table, "",
    "## Every observed channel", "", versionsTable(view, view.entries), "",
    "Distance counts mainline cycles between the kernel and the current release candidate, using the tag list of the mainline tree for the cycle order; rc steps are release candidates behind inside the rc's own cycle. Versions are ordered with Bun.semver (`7.4-rc3` < `7.4` < `7.4.1`).", "",
  ].join("\n"));
  for (const kernel of manifest.kernels) contents.set(`kernels/${kernel.id}.md`, kernelPage(kernel, manifest, digests.find((d) => d.kernel === kernel.id)!, view, store, verified));
  contents.set("state-of-kernels.md", stateOfKernels(manifest, view, digests, store, asOf));
  contents.set("sources.md", [
    "# Sources", "", `${manifest.sources.length} sources over ${new Set(manifest.sources.map((s) => s.host)).size} hosts, defined in tooling/kernel-watch/sources.toml. Status and shape come from fixtures/verified.json${verified ? ` (verified ${verified.verifiedAt})` : ""}.`, "",
    "| Source | Kernel | Kind | Yields | Status | Shape |", "| :-- | :-- | :-- | :-- | --: | :-- |",
    ...[...manifest.sources].sort(byId).map((s) => { const v = verified?.sources.find((x) => x.id === s.id); return `| [${s.id}](${s.url}) | ${s.kernel} | ${s.kind} | ${s.yields.join(", ")} | ${v?.status ?? "n/a"} | ${cell(v?.shape ?? "", 110)} |`; }),
    "",
  ].join("\n"));
  contents.set("README.md", [
    "# Kernel watch", "",
    "Real-time-ish tracking of Linux, Asterinas, Redox, LiberNT and the distribution kernels closest to the mainline rc. The pages in this directory are generated by `tooling/kernel-watch`; the design and what is not covered yet are in [PLAN.md](PLAN.md).", "",
    "- [Kernel versions](versions.md): newest kernel per distribution, distance to the mainline rc and stable",
    "- [State of the kernels](state-of-kernels.md): condensed context for agents",
    "- [Sources](sources.md): every endpoint, with the status and shape seen when it was verified",
    "- Per kernel: " + manifest.kernels.map((k) => `[${k.name}](kernels/${k.id}.md)`).join(", "),
    "- Machine-readable (for the site): `data/versions.json`, `data/digests.json`, `data/kernels.json`, `data/sources.json`", "",
  ].join("\n"));

  const files: string[] = [];
  for (const [rel, text] of [...contents].sort((a, b) => (a[0] < b[0] ? -1 : 1))) {
    const path = join(options.outDir, rel);
    mkdirSync(dirname(path), { recursive: true });
    const existing = Bun.file(path);
    if (!(await existing.exists()) || (await existing.text()) !== text) await Bun.write(path, text);
    files.push(path);
  }
  return { files, versions: view, versionsTable: table, digests, contents };
}
