// The one manifest of the watch: every source URL, host, org and satellite site lives in sources.toml.
import { ADAPTERS } from "./adapters.ts";

export type Role = "upstream" | "downstream" | "research";
export type Lineage = "linux" | "own";

export interface Site {
  id: string;
  kind: "docs" | "book" | "blog" | "site" | "rfc" | "list" | "api" | "forge";
  url: string;
  note?: string;
}
export interface Org {
  id: string;
  provider: "github" | "gitlab" | "googlesource" | "launchpad" | "other";
  url: string;
  /** Id of the source that lists the organisation's repositories, when one is polled. */
  source?: string;
}
export interface Kernel {
  id: string;
  name: string;
  role: Role;
  lineage: Lineage;
  description: string;
  sites: Site[];
  orgs: Org[];
}
export type Params = Record<string, string | number | boolean | string[]>;
export interface Source {
  id: string;
  kernel: string;
  adapter: string;
  /** Category from the owner's list: forge API, cgit atom, releases JSON, package index, feed, sitemap, text. */
  kind: string;
  url: string;
  yields: string[];
  /** Seconds between two polls of this source. */
  minInterval: number;
  enabled: boolean;
  params: Params;
  note?: string;
  host: string;
}
export interface Auth {
  scheme: string;
  env: string[];
}
export interface WatchConfig {
  userAgent: string;
  minInterval: number;
  timeout: number;
  maxPages: number;
  concurrencyPerHost: number;
  retentionDays: number;
  maxBackoff: number;
  auth: Record<string, Auth>;
}
export interface Manifest {
  watch: WatchConfig;
  kernels: Kernel[];
  sources: Source[];
}

const UNITS: Record<string, number> = { s: 1, m: 60, h: 3600, d: 86400 };

export function parseDuration(text: string | number): number {
  if (typeof text === "number") return text;
  const m = /^(\d+)([smhd])$/.exec(text.trim());
  if (!m) throw new Error(`invalid duration "${text}" (use 30s, 10m, 6h, 1d)`);
  return +m[1]! * UNITS[m[2]!]!;
}

const KINDS = new Set(["forge-api", "cgit-atom", "releases-json", "package-index", "feed", "sitemap", "text", "html-index", "gitiles"]);
const ROLES = new Set<string>(["upstream", "downstream", "research"]);
const LINEAGES = new Set<string>(["linux", "own"]);
const SITE_KINDS = new Set<string>(["docs", "book", "blog", "site", "rfc", "list", "api", "forge"]);
const PROVIDERS = new Set<string>(["github", "gitlab", "googlesource", "launchpad", "other"]);
const ID = /^[a-z0-9][a-z0-9-]*$/;

/** Throws one error listing every problem found. */
export function parseManifest(text: string): Manifest {
  const raw = Bun.TOML.parse(text) as Record<string, any>;
  const problems: string[] = [];
  const bad = (msg: string): void => void problems.push(msg);

  const w = raw.watch ?? {};
  const watch: WatchConfig = {
    userAgent: String(w.user_agent ?? ""),
    minInterval: parseDuration(w.min_interval ?? "10m"),
    timeout: parseDuration(w.timeout ?? "30s"),
    maxPages: Number(w.max_pages ?? 3),
    concurrencyPerHost: Number(w.concurrency_per_host ?? 2),
    retentionDays: Number(w.retention_days ?? 45),
    maxBackoff: parseDuration(w.max_backoff ?? "6h"),
    auth: Object.fromEntries(Object.entries<any>(w.auth ?? {}).map(([host, a]) => [host, { scheme: String(a.scheme ?? "Bearer"), env: [a.env ?? []].flat().map(String) }])),
  };
  if (!watch.userAgent) bad("watch.user_agent is required");

  const kernels: Kernel[] = (raw.kernel ?? []).map((k: any): Kernel => ({
    id: String(k.id ?? ""),
    name: String(k.name ?? ""),
    role: k.role,
    lineage: k.lineage,
    description: String(k.description ?? ""),
    sites: (k.site ?? []).map((s: any): Site => ({ id: String(s.id ?? ""), kind: s.kind, url: String(s.url ?? ""), ...(s.note ? { note: String(s.note) } : {}) })),
    orgs: (k.org ?? []).map((o: any): Org => ({ id: String(o.id ?? ""), provider: o.provider, url: String(o.url ?? ""), ...(o.source ? { source: String(o.source) } : {}) })),
  }));
  const kernelIds = new Set<string>();
  for (const k of kernels) {
    if (!ID.test(k.id)) bad(`kernel id "${k.id}" must match ${ID}`);
    if (kernelIds.has(k.id)) bad(`duplicate kernel id "${k.id}"`);
    kernelIds.add(k.id);
    if (!k.name) bad(`kernel ${k.id}: name is required`);
    if (!ROLES.has(k.role)) bad(`kernel ${k.id}: role must be one of ${[...ROLES]}`);
    if (!LINEAGES.has(k.lineage)) bad(`kernel ${k.id}: lineage must be one of ${[...LINEAGES]}`);
    for (const s of k.sites) {
      if (!ID.test(s.id)) bad(`kernel ${k.id}: site id "${s.id}" is invalid`);
      if (!SITE_KINDS.has(s.kind)) bad(`kernel ${k.id}: site ${s.id} kind "${s.kind}" is invalid`);
      checkUrl(`kernel ${k.id}: site ${s.id}`, s.url, bad);
    }
    for (const o of k.orgs) {
      if (!PROVIDERS.has(o.provider)) bad(`kernel ${k.id}: org ${o.id} provider "${o.provider}" is invalid`);
      checkUrl(`kernel ${k.id}: org ${o.id}`, o.url, bad);
    }
  }

  const sources: Source[] = (raw.source ?? []).map((s: any): Source => {
    const { id, kernel, adapter, kind, url, yields, min_interval, enabled, note, ...params } = s;
    let host = "";
    try {
      host = new URL(String(url)).host;
    } catch {}
    return {
      id: String(id ?? ""),
      kernel: String(kernel ?? ""),
      adapter: String(adapter ?? ""),
      kind: String(kind ?? ""),
      url: String(url ?? ""),
      yields: [yields ?? []].flat().map(String),
      minInterval: parseDuration(min_interval ?? watch.minInterval),
      enabled: enabled !== false,
      params,
      ...(note ? { note: String(note) } : {}),
      host,
    };
  });
  const sourceIds = new Set<string>();
  for (const s of sources) {
    if (!ID.test(s.id)) bad(`source id "${s.id}" must match ${ID}`);
    if (sourceIds.has(s.id)) bad(`duplicate source id "${s.id}"`);
    sourceIds.add(s.id);
    if (!kernelIds.has(s.kernel)) bad(`source ${s.id}: unknown kernel "${s.kernel}"`);
    if (!ADAPTERS[s.adapter]) bad(`source ${s.id}: unknown adapter "${s.adapter}"`);
    if (!KINDS.has(s.kind)) bad(`source ${s.id}: kind "${s.kind}" must be one of ${[...KINDS]}`);
    checkUrl(`source ${s.id}`, s.url, bad);
    if (s.yields.length === 0) bad(`source ${s.id}: yields must list what the source produces`);
    if (s.minInterval < 60) bad(`source ${s.id}: min_interval below 1m`);
  }
  for (const k of kernels) for (const o of k.orgs) if (o.source && !sourceIds.has(o.source)) bad(`kernel ${k.id}: org ${o.id} names unknown source "${o.source}"`);
  if (problems.length) throw new Error(`manifest invalid:\n- ${problems.join("\n- ")}`);
  return { watch, kernels, sources };
}

function checkUrl(where: string, url: string, bad: (m: string) => void): void {
  let u: URL;
  try {
    u = new URL(url);
  } catch {
    return bad(`${where}: invalid URL "${url}"`);
  }
  const local = u.hostname === "localhost" || u.hostname === "127.0.0.1";
  if (u.protocol !== "https:" && !(local && u.protocol === "http:")) bad(`${where}: URL must be https (got ${u.protocol})`);
  if (u.username || u.password) bad(`${where}: URL must not carry credentials`);
  if (/[?&](access_token|token|key|secret|password)=/i.test(u.search)) bad(`${where}: URL must not carry a secret`);
}

export async function loadManifest(path: string): Promise<Manifest> {
  return parseManifest(await Bun.file(path).text());
}
