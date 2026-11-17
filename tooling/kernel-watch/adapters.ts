// Adapters turn one HTTP body into normalised items, version observations and metrics. They never write anywhere.
import type { Source } from "./manifest.ts";
import { compareVersions, formatVersion, parseVersion } from "./version.ts";

export interface Item {
  id: string;
  kind: string;
  title: string;
  url: string;
  author: string;
  /** Epoch seconds of creation or publication. */
  time: number;
  /** Epoch seconds of the last change, when the source tracks one. */
  updated?: number;
  version?: string;
  body: string;
  meta?: Record<string, string | number | boolean | null>;
}
export interface VersionObs {
  channel: string;
  version: string;
}
export interface Metric {
  name: string;
  value: number;
}
export interface Parsed {
  items: Item[];
  versions: VersionObs[];
  metrics: Metric[];
  /** Next page of the same listing, when the source paginates. */
  next?: string;
}
export interface FetchedBody {
  status: number;
  text: string;
  headers: Headers;
}
export interface Ctx {
  source: Source;
  text: string;
  headers: Headers;
  /** Epoch seconds of the poll. */
  now: number;
  /** Unconditional GET for adapters that need a second request (discovery, counts). */
  get(url: string): Promise<FetchedBody>;
}
export interface Adapter {
  accept: string;
  parse(ctx: Ctx): Promise<Parsed> | Parsed;
}

const BODY_LIMIT = 400;
const empty = (): Parsed => ({ items: [], versions: [], metrics: [] });

const ENTITIES: Record<string, string> = { amp: "&", lt: "<", gt: ">", quot: '"', apos: "'", nbsp: " " };
export function decodeEntities(s: string): string {
  return s.replace(/&(#x[0-9a-f]+|#\d+|[a-z]+);/gi, (m, e: string) => {
    if (e[0] === "#") {
      const code = e[1]!.toLowerCase() === "x" ? parseInt(e.slice(2), 16) : parseInt(e.slice(1), 10);
      return Number.isFinite(code) && code > 0 && code < 0x110000 ? String.fromCodePoint(code) : m;
    }
    return ENTITIES[e.toLowerCase()] ?? m;
  });
}
export function excerpt(text: string | null | undefined, max = BODY_LIMIT): string {
  const plain = decodeEntities(String(text ?? "").replace(/<[^>]*>/g, " ")).replace(/\s+/g, " ").trim();
  return plain.length > max ? `${plain.slice(0, max - 1)}\u2026` : plain;
}
const firstLine = (s: string): string => s.split("\n", 1)[0]!.trim();
const rest = (s: string): string => s.slice(s.indexOf("\n") + 1);
const epoch = (iso: string | null | undefined, fallback: number): number => {
  const ms = iso ? Date.parse(iso) : NaN;
  return Number.isFinite(ms) ? Math.floor(ms / 1000) : fallback;
};
const json = (text: string): any => JSON.parse(text.replace(/^\)\]\}'\s*/, ""));
const str = (v: unknown): string => (v === null || v === undefined ? "" : String(v));

function pathGet(value: unknown, path: string): unknown {
  return path.split(".").reduce<any>((v, k) => (v === null || v === undefined ? undefined : v[k]), value);
}
function param(s: Source, key: string, fallback = ""): string {
  const v = s.params[key];
  return v === undefined ? fallback : String(v);
}
function paramList(s: Source, key: string): string[] {
  const v = s.params[key];
  return v === undefined ? [] : [v].flat().map(String);
}
/** "{1}.{2}" style template over regexp captures. */
const fill = (template: string, groups: string[]): string => template.replace(/\{(\d+)\}/g, (_, i: string) => groups[+i - 1] ?? "");

// ---- GitHub REST -----------------------------------------------------------------------------------------------
const github = {
  releases: (ctx: Ctx): Parsed => ({
    ...empty(),
    items: (json(ctx.text) as any[])
      .filter((r) => !r.draft)
      .map((r) => ({
        id: str(r.id), kind: "release", title: str(r.name || r.tag_name), url: str(r.html_url), author: str(r.author?.login),
        time: epoch(r.published_at ?? r.created_at, ctx.now), version: str(r.tag_name), body: excerpt(r.body),
        meta: { prerelease: !!r.prerelease },
      })),
  }),
  commits: (ctx: Ctx): Parsed => ({
    ...empty(),
    items: (json(ctx.text) as any[]).map((c) => ({
      id: str(c.sha), kind: "commit", title: firstLine(str(c.commit?.message)), url: str(c.html_url),
      author: str(c.commit?.author?.name || c.author?.login), time: epoch(c.commit?.committer?.date ?? c.commit?.author?.date, ctx.now),
      body: excerpt(rest(str(c.commit?.message))),
    })),
  }),
  issues: (ctx: Ctx): Parsed => ({
    ...empty(),
    items: (json(ctx.text) as any[]).filter((i) => !i.pull_request).map((i) => ({
      id: str(i.number), kind: "issue", title: str(i.title), url: str(i.html_url), author: str(i.user?.login),
      time: epoch(i.created_at, ctx.now), updated: epoch(i.updated_at, ctx.now), body: excerpt(i.body),
      meta: { state: str(i.state), comments: Number(i.comments ?? 0) },
    })),
  }),
  pulls: (ctx: Ctx): Parsed => ({
    ...empty(),
    items: (json(ctx.text) as any[]).map((p) => ({
      id: str(p.number), kind: "pr", title: str(p.title), url: str(p.html_url), author: str(p.user?.login),
      time: epoch(p.created_at, ctx.now), updated: epoch(p.updated_at, ctx.now), body: excerpt(p.body),
      meta: { state: str(p.state), merged: !!p.merged_at, draft: !!p.draft },
    })),
  }),
  repos: (ctx: Ctx): Parsed => ({
    ...empty(),
    items: (json(ctx.text) as any[]).map((r) => ({
      id: str(r.full_name), kind: "repo", title: str(r.full_name), url: str(r.html_url), author: str(r.owner?.login),
      time: epoch(r.created_at, ctx.now), updated: epoch(r.pushed_at ?? r.updated_at, ctx.now), body: excerpt(r.description),
      meta: { stars: Number(r.stargazers_count ?? 0), archived: !!r.archived, fork: !!r.fork, language: str(r.language) },
    })),
  }),
  repo: (ctx: Ctx): Parsed => {
    const r = json(ctx.text);
    return { ...empty(), metrics: [{ name: "stars", value: Number(r.stargazers_count ?? 0) }, { name: "forks", value: Number(r.forks_count ?? 0) }, { name: "size_kb", value: Number(r.size ?? 0) }] };
  },
};

async function openCounts(ctx: Ctx, read: (f: FetchedBody) => number | null, secondUrl: string, names: [string, string]): Promise<Parsed> {
  const first = read({ status: 200, text: ctx.text, headers: ctx.headers });
  const second = await ctx.get(secondUrl);
  const metrics: Metric[] = [];
  if (first !== null) metrics.push({ name: names[0], value: first });
  const other = second.status === 200 ? read(second) : null;
  if (other !== null) metrics.push({ name: names[1], value: other });
  return { ...empty(), metrics };
}

// ---- GitLab REST -----------------------------------------------------------------------------------------------
const gitlab = {
  commits: (ctx: Ctx): Parsed => {
    const pattern = param(ctx.source, "version_from");
    const re = pattern ? new RegExp(pattern) : null;
    const commits = json(ctx.text) as any[];
    const versions: VersionObs[] = [];
    if (re && ctx.source.params.channel) {
      for (const c of commits) {
        const m = re.exec(str(c.title));
        if (m?.[1]) {
          versions.push({ channel: param(ctx.source, "channel"), version: m[1] });
          break;
        }
      }
    }
    return {
      ...empty(), versions,
      items: commits.map((c) => ({
        id: str(c.id), kind: "commit", title: str(c.title), url: str(c.web_url), author: str(c.author_name),
        time: epoch(c.committed_date ?? c.created_at, ctx.now), body: excerpt(rest(str(c.message))),
      })),
    };
  },
  mergeRequests: (ctx: Ctx): Parsed => ({
    ...empty(),
    items: (json(ctx.text) as any[]).map((m) => ({
      id: str(m.iid), kind: "pr", title: str(m.title), url: str(m.web_url), author: str(m.author?.username),
      time: epoch(m.created_at, ctx.now), updated: epoch(m.updated_at, ctx.now), body: excerpt(m.description),
      meta: { state: str(m.state), merged: m.state === "merged", draft: !!m.draft },
    })),
  }),
  issues: (ctx: Ctx): Parsed => ({
    ...empty(),
    items: (json(ctx.text) as any[]).map((i) => ({
      id: str(i.iid), kind: "issue", title: str(i.title), url: str(i.web_url), author: str(i.author?.username),
      time: epoch(i.created_at, ctx.now), updated: epoch(i.updated_at, ctx.now), body: excerpt(i.description),
      meta: { state: str(i.state), comments: Number(i.user_notes_count ?? 0) },
    })),
  }),
  tags: (ctx: Ctx): Parsed => {
    const web = param(ctx.source, "web");
    const re = param(ctx.source, "version_from") ? new RegExp(param(ctx.source, "version_from")) : null;
    const items: Item[] = [];
    const versions: VersionObs[] = [];
    for (const t of json(ctx.text) as any[]) {
      const m = re?.exec(str(t.name));
      if (m?.[1] && versions.length === 0 && ctx.source.params.channel) versions.push({ channel: param(ctx.source, "channel"), version: m[1] });
      items.push({
        id: str(t.name), kind: "tag", title: str(t.name), url: `${web}/${encodeURIComponent(str(t.name))}`, author: str(t.commit?.author_name),
        time: epoch(t.commit?.created_at, ctx.now), ...(m?.[1] ? { version: m[1] } : {}), body: excerpt(t.release?.description ?? t.message),
      });
    }
    return { ...empty(), items, versions };
  },
  releases: (ctx: Ctx): Parsed => ({
    ...empty(),
    items: (json(ctx.text) as any[]).map((r) => ({
      id: str(r.tag_name), kind: "release", title: str(r.name || r.tag_name), url: str(r._links?.self ?? ""), author: str(r.author?.username),
      time: epoch(r.released_at ?? r.created_at, ctx.now), version: str(r.tag_name), body: excerpt(r.description),
    })),
  }),
  projects: (ctx: Ctx): Parsed => ({
    ...empty(),
    items: (json(ctx.text) as any[]).map((p) => ({
      id: str(p.path_with_namespace), kind: "repo", title: str(p.path_with_namespace), url: str(p.web_url), author: str(p.namespace?.full_path),
      time: epoch(p.created_at, ctx.now), updated: epoch(p.last_activity_at, ctx.now), body: excerpt(p.description),
      meta: { stars: Number(p.star_count ?? 0), archived: !!p.archived },
    })),
  }),
  project: (ctx: Ctx): Parsed => {
    const p = json(ctx.text);
    return { ...empty(), metrics: [{ name: "stars", value: Number(p.star_count ?? 0) }, { name: "forks", value: Number(p.forks_count ?? 0) }] };
  },
};

// ---- Feeds: Atom (GitHub, cgit, lore, GitLab) and RSS ----------------------------------------------------------
const tagText = (block: string, tag: string): string => {
  const m = new RegExp(`<${tag}(?:\\s[^>]*)?>([\\s\\S]*?)</${tag}>`, "i").exec(block);
  if (!m) return "";
  return decodeEntities(m[1]!.replace(/^\s*<!\[CDATA\[([\s\S]*?)\]\]>\s*$/, "$1")).trim();
};
const feedId = (id: string): string => (id.startsWith("urn:sha1:") ? id.slice(9) : /^tag:github\.com,2008:/.test(id) ? id.replace(/^.*[/:]/, "") : id);

export function parseFeed(xml: string, kind: string, host: string, now: number): Item[] {
  const items: Item[] = [];
  const blocks = [...xml.matchAll(/<(entry|item)(?:\s[^>]*)?>([\s\S]*?)<\/\1>/gi)];
  for (const [, , block] of blocks) {
    const b = block!;
    const rawId = tagText(b, "id") || tagText(b, "guid");
    const href = /<link[^>]*\brel=['"]alternate['"][^>]*\bhref=['"]([^'"]+)['"]/i.exec(b)?.[1] ?? /<link[^>]*\bhref=['"]([^'"]+)['"]/i.exec(b)?.[1] ?? tagText(b, "link");
    let url = decodeEntities(href);
    if (url.startsWith("http://")) {
      try {
        if (new URL(url).host === host) url = `https://${url.slice(7)}`;
      } catch {}
    }
    const author = tagText(tagText(b, "author"), "name") || tagText(b, "author") || tagText(b, "dc:creator");
    const published = tagText(b, "published") || tagText(b, "pubDate") || tagText(b, "updated");
    const updated = tagText(b, "updated");
    const body = tagText(b, "content") || tagText(b, "summary") || tagText(b, "description");
    items.push({
      id: feedId(rawId || url), kind, title: excerpt(tagText(b, "title"), 200), url, author: excerpt(author, 80),
      time: epoch(published, now), ...(updated ? { updated: epoch(updated, now) } : {}), body: excerpt(body),
    });
  }
  return items;
}
const feed = (ctx: Ctx): Parsed => ({ ...empty(), items: parseFeed(ctx.text, ctx.source.yields[0] ?? "item", ctx.source.host, ctx.now) });

// ---- kernel.org ------------------------------------------------------------------------------------------------
function kernelOrgReleases(ctx: Ctx): Parsed {
  const data = json(ctx.text);
  const items: Item[] = [];
  const versions: VersionObs[] = [];
  let stableSeen = false;
  for (const r of data.releases as any[]) {
    const v = str(r.version);
    const parsed = parseVersion(v);
    const moniker = str(r.moniker);
    if (moniker === "linux-next") continue;
    items.push({
      id: v, kind: "release", title: `Linux ${v} (${moniker})`, url: str(r.gitweb || r.source), author: "", time: Number(r.released?.timestamp ?? ctx.now),
      version: v, body: str(r.released?.isodate), meta: { moniker, eol: !!r.iseol },
    });
    if (!parsed) continue;
    if (moniker === "mainline") versions.push({ channel: "mainline", version: v });
    else if (moniker === "stable" && !r.iseol && !stableSeen) (stableSeen = true), versions.push({ channel: "stable", version: v });
    else if (moniker === "longterm" && !r.iseol) versions.push({ channel: `longterm-${parsed.major}.${parsed.minor}`, version: v });
  }
  return { ...empty(), items, versions };
}

function cgitTags(ctx: Ctx): Parsed {
  const items: Item[] = [];
  const row = /<tr><td><a href='[^']*\/tag\/\?h=([^']+)'>[^<]*<\/a><\/td>[\s\S]*?<td>([^<]*)<\/td><td colspan='2'><span class='age-[a-z]+' data-ut='(\d+)'/g;
  const wanted = new RegExp(param(ctx.source, "tag_pattern", "^v\\d+\\.\\d+(?:-rc\\d+)?$"));
  const base = ctx.source.url.replace(/\/refs\/tags.*$/, "");
  for (const m of ctx.text.matchAll(row)) {
    const name = decodeEntities(m[1]!);
    if (!wanted.test(name)) continue;
    items.push({ id: name, kind: "tag", title: name, url: `${base}/tag/?h=${name}`, author: decodeEntities(m[2]!), time: +m[3]!, version: name.slice(1), body: "" });
  }
  return { ...empty(), items };
}

// ---- Gitiles (googlesource) ------------------------------------------------------------------------------------
function gitilesLog(ctx: Ctx): Parsed {
  const data = json(ctx.text);
  const web = param(ctx.source, "web");
  const items = (data.log as any[]).map((c): Item => ({
    id: str(c.commit), kind: "commit", title: firstLine(str(c.message)), url: `${web}/+/${c.commit}`, author: str(c.author?.name),
    time: epoch(c.committer?.time ? `${c.committer.time} UTC` : null, ctx.now), body: excerpt(rest(str(c.message))),
  }));
  const next = data.next ? `${ctx.source.url}${ctx.source.url.includes("?") ? "&" : "?"}s=${data.next}` : undefined;
  return { ...empty(), items, ...(next ? { next } : {}) };
}

/** Linux Makefile header to a version. */
export function makefileVersion(text: string): string | null {
  const field = (name: string): string | undefined => new RegExp(`^${name}[ \\t]*=[ \\t]*(\\S*)`, "m").exec(text)?.[1];
  const major = field("VERSION"), minor = field("PATCHLEVEL"), patch = field("SUBLEVEL");
  if (!major || !minor || !patch) return null;
  return `${major}.${minor}.${patch}${field("EXTRAVERSION") ?? ""}`;
}
const decodeBase64 = (s: string): string => Buffer.from(s, "base64").toString("utf8");

function versionItem(s: Source, channel: string, version: string, now: number, url = s.url): Item {
  return { id: `${channel}@${version}`, kind: "version", title: `${s.kernel} ${channel}: ${version}`, url, author: "", time: now, version, body: "" };
}

async function gitilesNewestBranch(ctx: Ctx): Promise<Parsed> {
  const heads = json(ctx.text) as Record<string, unknown>;
  const re = new RegExp(param(ctx.source, "branch_pattern"));
  let best: { name: string; v: NonNullable<ReturnType<typeof parseVersion>> } | null = null;
  for (const name of Object.keys(heads)) {
    const m = re.exec(name);
    const v = m?.[1] ? parseVersion(m[1]) : null;
    if (v && (!best || compareVersions(v, best.v) > 0)) best = { name, v };
  }
  if (!best) throw new Error(`no branch of ${ctx.source.url} matches ${re}`);
  const file = await ctx.get(param(ctx.source, "file_url").replace("{branch}", best.name));
  if (file.status !== 200) throw new Error(`${best.name}: Makefile answered ${file.status}`);
  const version = makefileVersion(decodeBase64(file.text));
  if (!version) throw new Error(`${best.name}: no VERSION in Makefile`);
  const channel = `${param(ctx.source, "channel")}`;
  const item = versionItem(ctx.source, `${channel} (${best.name})`, version, ctx.now);
  return { ...empty(), items: [item], versions: [{ channel: `${channel} (${best.name})`, version }] };
}

// ---- text and JSON version probes -----------------------------------------------------------------------------
function textVersion(ctx: Ctx): Parsed {
  const body = param(ctx.source, "decode") === "base64" ? decodeBase64(ctx.text) : ctx.text;
  const groups = paramList(ctx.source, "patterns").map((p) => new RegExp(p, "m").exec(body)?.[1]?.trim() ?? "");
  const version = fill(param(ctx.source, "template", "{1}"), groups).trim();
  if (!parseVersion(version)) throw new Error(`no kernel version found (got "${version}")`);
  const channel = param(ctx.source, "channel");
  return { ...empty(), items: [versionItem(ctx.source, channel, version, ctx.now)], versions: [{ channel, version }] };
}
function jsonVersion(ctx: Ctx): Parsed {
  const data = json(ctx.text);
  const version = str(pathGet(data, param(ctx.source, "path")));
  if (!parseVersion(version)) throw new Error(`no kernel version at ${param(ctx.source, "path")} (got "${version}")`);
  const channel = param(ctx.source, "channel");
  const when = param(ctx.source, "time_path") ? pathGet(data, param(ctx.source, "time_path")) : null;
  const item = versionItem(ctx.source, channel, version, ctx.now);
  if (typeof when === "string" || typeof when === "number") item.time = typeof when === "number" ? when : epoch(when, ctx.now);
  return { ...empty(), items: [item], versions: [{ channel, version }] };
}

function nixKernels(ctx: Ctx): Parsed {
  const data = json(ctx.text) as Record<string, { version: string; lts?: boolean }>;
  const versions: VersionObs[] = [];
  const items: Item[] = [];
  for (const [key, entry] of Object.entries(data)) {
    if (!parseVersion(entry.version)) continue;
    const channel = key === "testing" ? "testing" : `${key}${entry.lts ? " lts" : ""}`;
    versions.push({ channel, version: entry.version });
    items.push(versionItem(ctx.source, channel, entry.version, ctx.now));
  }
  return { ...empty(), items, versions };
}

function launchpadSources(ctx: Ctx): Parsed {
  const entries = json(ctx.text).entries as any[];
  const items: Item[] = [];
  const perSeries = new Map<string, { version: string; pocket: string }>();
  for (const e of entries) {
    const series = str(e.distro_series_link).split("/").pop()!;
    const version = str(e.source_package_version);
    const published = epoch(e.date_published, ctx.now);
    items.push({
      id: str(e.self_link).split("/").pop()!, kind: "package", title: `${str(e.source_package_name)} ${version} (${series}/${str(e.pocket)})`,
      url: str(e.self_link), author: "", time: published, version, body: "", meta: { series, pocket: str(e.pocket), status: str(e.status) },
    });
    if (e.status === "Published" && !perSeries.has(series)) perSeries.set(series, { version, pocket: str(e.pocket) });
  }
  const versions = [...perSeries].filter(([, v]) => parseVersion(v.version)).map(([series, v]) => ({ channel: `${series}/${v.pocket.toLowerCase()}`, version: v.version }));
  return { ...empty(), items, versions };
}

function bodhiUpdates(ctx: Ctx): Parsed {
  const updates = json(ctx.text).updates as any[];
  const items: Item[] = [];
  const perRelease = new Map<string, string>();
  const prefix = param(ctx.source, "package", "kernel");
  const excluded = paramList(ctx.source, "exclude_releases");
  for (const u of updates) {
    const nvr = str(u.builds?.[0]?.nvr ?? u.title);
    const version = nvr.startsWith(`${prefix}-`) ? nvr.slice(prefix.length + 1) : nvr;
    const release = str(u.release?.name);
    if (excluded.includes(release)) continue;
    items.push({
      id: str(u.alias), kind: "package", title: `${nvr} (${release}, ${str(u.status)})`, url: str(u.url), author: str(u.user?.name),
      time: epoch(`${str(u.date_stable ?? u.date_submitted)} UTC`, ctx.now), version, body: excerpt(u.notes), meta: { release, status: str(u.status) },
    });
    if (u.status === "stable" && !perRelease.has(release) && parseVersion(version)) perRelease.set(release, version);
  }
  return { ...empty(), items, versions: [...perRelease].map(([channel, version]) => ({ channel, version })) };
}

function htmlIndexVersions(ctx: Ctx): Parsed {
  const row = /<a href="(v\d[^"/]*)\/">[^<]*<\/a><\/td><td align="right">(\d{4}-\d\d-\d\d \d\d:\d\d)/g;
  const found: { name: string; time: number; v: NonNullable<ReturnType<typeof parseVersion>> }[] = [];
  for (const m of ctx.text.matchAll(row)) {
    const v = parseVersion(m[1]!);
    if (v) found.push({ name: m[1]!, time: epoch(`${m[2]!.replace(" ", "T")}:00Z`, ctx.now), v });
  }
  found.sort((a, b) => compareVersions(b.v, a.v));
  const top = found.slice(0, Number(ctx.source.params.limit ?? 8));
  const items = top.map((f): Item => ({ id: f.name, kind: "build", title: `mainline build ${f.name}`, url: `${ctx.source.url}${f.name}/`, author: "", time: f.time, version: formatVersion(f.v), body: "" }));
  const best = top[0];
  return { ...empty(), items, versions: best ? [{ channel: param(ctx.source, "channel"), version: formatVersion(best.v) }] : [] };
}

function sitemap(ctx: Ctx): Parsed {
  const limit = Number(ctx.source.params.limit ?? 100);
  const items: Item[] = [];
  for (const m of ctx.text.matchAll(/<url>([\s\S]*?)<\/url>/g)) {
    const loc = tagText(m[1]!, "loc");
    if (!loc) continue;
    const lastmod = tagText(m[1]!, "lastmod");
    items.push({ id: loc, kind: "page", title: new URL(loc).pathname, url: loc, author: "", time: epoch(lastmod, 0), body: "" });
  }
  items.sort((a, b) => b.time - a.time || (a.id < b.id ? -1 : 1));
  return { ...empty(), items: items.slice(0, limit) };
}

const ghTotal = (f: FetchedBody): number | null => {
  try {
    const n = JSON.parse(f.text).total_count;
    return typeof n === "number" ? n : null;
  } catch {
    return null;
  }
};
const glTotal = (f: FetchedBody): number | null => {
  const n = Number(f.headers.get("x-total"));
  return f.headers.has("x-total") && Number.isFinite(n) ? n : null;
};

const JSON_ACCEPT = "application/json";
const FEED_ACCEPT = "application/atom+xml, application/rss+xml, application/xml;q=0.9, */*;q=0.5";
export const ADAPTERS: Record<string, Adapter> = {
  "github-releases": { accept: JSON_ACCEPT, parse: github.releases },
  "github-commits": { accept: JSON_ACCEPT, parse: github.commits },
  "github-issues": { accept: JSON_ACCEPT, parse: github.issues },
  "github-pulls": { accept: JSON_ACCEPT, parse: github.pulls },
  "github-org-repos": { accept: JSON_ACCEPT, parse: github.repos },
  "github-repo": { accept: JSON_ACCEPT, parse: github.repo },
  "github-open-counts": { accept: JSON_ACCEPT, parse: (c) => openCounts(c, ghTotal, param(c.source, "issues_url"), ["open_prs", "open_issues"]) },
  "gitlab-commits": { accept: JSON_ACCEPT, parse: gitlab.commits },
  "gitlab-merge-requests": { accept: JSON_ACCEPT, parse: gitlab.mergeRequests },
  "gitlab-issues": { accept: JSON_ACCEPT, parse: gitlab.issues },
  "gitlab-tags": { accept: JSON_ACCEPT, parse: gitlab.tags },
  "gitlab-releases": { accept: JSON_ACCEPT, parse: gitlab.releases },
  "gitlab-group-projects": { accept: JSON_ACCEPT, parse: gitlab.projects },
  "gitlab-project": { accept: JSON_ACCEPT, parse: gitlab.project },
  "gitlab-open-counts": { accept: JSON_ACCEPT, parse: (c) => openCounts(c, glTotal, param(c.source, "issues_url"), ["open_prs", "open_issues"]) },
  atom: { accept: FEED_ACCEPT, parse: feed },
  rss: { accept: FEED_ACCEPT, parse: feed },
  "cgit-atom": { accept: FEED_ACCEPT, parse: feed },
  "cgit-tags": { accept: "text/html", parse: cgitTags },
  "kernel-org-releases": { accept: JSON_ACCEPT, parse: kernelOrgReleases },
  "gitiles-log": { accept: JSON_ACCEPT, parse: gitilesLog },
  "gitiles-newest-branch": { accept: JSON_ACCEPT, parse: gitilesNewestBranch },
  "text-version": { accept: "text/plain, */*", parse: textVersion },
  "json-version": { accept: JSON_ACCEPT, parse: jsonVersion },
  "nix-kernels": { accept: "*/*", parse: nixKernels },
  "launchpad-sources": { accept: JSON_ACCEPT, parse: launchpadSources },
  "bodhi-updates": { accept: JSON_ACCEPT, parse: bodhiUpdates },
  "html-index-versions": { accept: "text/html", parse: htmlIndexVersions },
  sitemap: { accept: "application/xml, text/xml, */*;q=0.5", parse: sitemap },
};
