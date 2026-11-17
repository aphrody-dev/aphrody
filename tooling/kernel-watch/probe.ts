// Read-only probe of an Arch host: running kernel, installed kernel packages, pending updates, and the newest kernels in the
// Arch repositories (including core-testing and extra-testing), the AUR and the CachyOS repositories, ranked by closeness to
// the mainline rc. The host side is one fixed read-only script; nothing here installs, syncs the live database or writes under
// /etc, /boot or /usr. Run it from the build host or a workstation over ssh, never from CI.
import type { ProbeConfig, ProbeHost } from "./lanes.ts";
import type { Store } from "./store.ts";
import { buildCycles, closenessKey, compareVersions, distance, formatVersion, parseVersion, type Distance, type KernelVersion } from "./version.ts";

/** What runs on the probed host. Reads only; the temporary database for pending updates lives in a mktemp directory removed at the end. */
export const REMOTE_SCRIPT = String.raw`
LC_ALL=C; export LC_ALL
KERNEL_RE='^linux(-(lts|zen|hardened|rt|rt-lts|mainline|rc|git|xanmod|tkg[a-z0-9-]*|cachyos[a-z0-9-]*|clear|surface|t2|ck))?$'
echo "@@uname-r"; uname -r
echo "@@uname-v"; uname -v
echo "@@proc-version"; cat /proc/version
echo "@@installed"; pacman -Q 2>/dev/null | awk '$1 ~ /'"$KERNEL_RE"'/ && $1 !~ /-(headers|docs)$/ { print $1, $2 }'
echo "@@sync-list"; pacman -Sl 2>/dev/null | awk '$2 ~ /'"$KERNEL_RE"'/ && $2 !~ /-(headers|docs)$/ { print $1, $2, $3 }'
echo "@@sync-age"; for f in /var/lib/pacman/sync/*.db; do echo "$(basename "$f" .db) $(stat -c %Y "$f")"; done
echo "@@repos"; pacman-conf --repo-list 2>/dev/null
echo "@@pending"
tmp=$(mktemp -d /tmp/kernel-probe.XXXXXX) && {
  mkdir -p "$tmp/sync" && chmod 755 "$tmp" && chmod 1777 "$tmp/sync" && ln -s /var/lib/pacman/local "$tmp/local"
  if pacman -Sy --dbpath "$tmp" --logfile /dev/null >/dev/null 2>"$tmp/err"; then pacman -Qu --dbpath "$tmp" 2>/dev/null; else echo "@@pending-error"; head -c 300 "$tmp/err"; fi
  rm -rf "$tmp"
}
echo "@@end"
`;

export interface RemoteFacts {
  release: string;
  build: string;
  procVersion: string;
  installed: { name: string; version: string }[];
  /** Kernel packages in the host's own synced databases (as old as the last sync). */
  sync: { repo: string; name: string; version: string }[];
  /** Epoch seconds of the newest synced database, null when unknown. */
  syncedAt: number | null;
  repos: string[];
  /** Pending updates against a freshly synced temporary database; null when that sync failed. */
  pending: { name: string; from: string; to: string }[] | null;
  pendingError: string | null;
}

export function parseRemote(output: string): RemoteFacts {
  const sections = new Map<string, string[]>();
  let current = "";
  for (const line of output.split("\n")) {
    const mark = /^@@([\w-]+)$/.exec(line.trim());
    if (mark) {
      current = mark[1]!;
      if (!sections.has(current)) sections.set(current, []);
    } else if (current && line.trim()) sections.get(current)!.push(line.trim());
  }
  const lines = (name: string): string[] => sections.get(name) ?? [];
  const pairs = (name: string): string[][] => lines(name).map((l) => l.split(/\s+/));
  const ages = pairs("sync-age").map((p) => Number(p[1])).filter((n) => Number.isFinite(n));
  const pendingError = sections.has("pending-error") ? lines("pending-error").join(" ") || "temporary sync failed" : null;
  const pendingLines = lines("pending").filter((l) => !l.startsWith("@@"));
  return {
    release: lines("uname-r")[0] ?? "",
    build: lines("uname-v")[0] ?? "",
    procVersion: lines("proc-version")[0] ?? "",
    installed: pairs("installed").map(([name, version]) => ({ name: name!, version: version! })),
    sync: pairs("sync-list").map(([repo, name, version]) => ({ repo: repo!, name: name!, version: version! })),
    syncedAt: ages.length ? Math.max(...ages) : null,
    repos: lines("repos"),
    pending: pendingError ? null : pendingLines.map((l) => /^(\S+) (\S+) -> (\S+)$/.exec(l)).filter((m): m is RegExpExecArray => m !== null).map((m) => ({ name: m[1]!, from: m[2]!, to: m[3]! })),
    pendingError,
  };
}

export type Origin = "running" | "installed" | "host-repo" | "pending" | "repo" | "testing" | "aur" | "cachyos";
export interface Candidate {
  origin: Origin;
  /** Repository, channel or database the version comes from. */
  where: string;
  name: string;
  version: string;
  normalized: string;
  /** Link to the package page when the source has one. */
  url?: string;
}
export interface RankedCandidate extends Candidate, Distance {}

export interface Reference {
  mainline: string;
  stable: string;
  rc: KernelVersion;
  stableVersion: KernelVersion;
  cycles: string[];
}

export interface ProbeReport {
  host: string;
  label: string;
  /** Epoch seconds of the probe. */
  at: number;
  mainline: string;
  stable: string;
  running: { release: string; build: string; normalized: string } & Distance;
  syncedAt: number | null;
  repos: string[];
  pendingError: string | null;
  pendingTotal: number | null;
  /** Newest version of anything kernel-like that the host could install, and how far the running kernel is from it. */
  summary: {
    runningCyclesBehindNewestRepo: number | null;
    newestRepo: string | null;
    newestTesting: string | null;
    newestAur: string | null;
    newestCachyos: string | null;
  };
  candidates: RankedCandidate[];
}

const archVersion = (r: { epoch?: number; pkgver: string; pkgrel: string }): string => `${r.epoch ? `${r.epoch}:` : ""}${r.pkgver}-${r.pkgrel}`;
const REPO_PAGE = "https://archlinux.org/packages";
const TESTING = /testing$/i;

interface ArchApiResult {
  pkgname: string;
  repo: string;
  pkgver: string;
  pkgrel: string;
  epoch?: number;
  arch?: string;
}
export function parseArchApi(body: string): Candidate[] {
  const data = JSON.parse(body) as { results?: ArchApiResult[] };
  const out: Candidate[] = [];
  for (const r of data.results ?? []) {
    const version = archVersion(r);
    const parsed = parseVersion(r.pkgver);
    if (!parsed || (r.arch && r.arch !== "x86_64" && r.arch !== "any")) continue;
    const repo = r.repo.toLowerCase();
    out.push({
      origin: TESTING.test(repo) ? "testing" : "repo", where: repo, name: r.pkgname, version, normalized: formatVersion(parsed),
      url: `${REPO_PAGE}/${r.repo}/${r.arch ?? "x86_64"}/${r.pkgname}/`,
    });
  }
  return out;
}

interface AurResult {
  Name: string;
  Version: string;
  OutOfDate: number | null;
}
export function parseAur(body: string): Candidate[] {
  const data = JSON.parse(body) as { results?: AurResult[] };
  const out: Candidate[] = [];
  for (const r of data.results ?? []) {
    const parsed = parseVersion(r.Version);
    if (parsed) out.push({ origin: "aur", where: r.OutOfDate ? "aur (flagged out of date)" : "aur", name: r.Name, version: r.Version, normalized: formatVersion(parsed), url: `https://aur.archlinux.org/packages/${r.Name}` });
  }
  return out;
}

/** Newest package file per requested name in a CachyOS repository directory listing. */
export function parseCachyosIndex(html: string, repo: string, names: string[], url: string): Candidate[] {
  const best = new Map<string, Candidate>();
  for (const [, file] of html.matchAll(/href="([^"?/]+\.pkg\.tar\.zst)"/g)) {
    for (const name of names) {
      if (!file!.startsWith(`${name}-`)) continue;
      const m = /^(\d[^-]*)-(\d+)-(x86_64(?:_v[34])?|any)\.pkg\.tar\.zst$/.exec(file!.slice(name.length + 1));
      const parsed = m ? parseVersion(m[1]!) : null;
      if (!m || !parsed) continue;
      const candidate: Candidate = { origin: "cachyos", where: repo, name, version: `${m[1]}-${m[2]}`, normalized: formatVersion(parsed), url: `${url}${file}` };
      const key = `${repo}\u0000${name}`;
      const held = best.get(key);
      if (!held || compareVersions(parsed, parseVersion(held.version)!) > 0 || (compareVersions(parsed, parseVersion(held.version)!) === 0 && Number(m[2]) > Number(held.version.split("-").pop()))) best.set(key, candidate);
    }
  }
  return [...best.values()];
}

export interface ProbeDeps {
  /** Runs the fixed script on the ssh alias and returns its stdout. */
  run: (ssh: string, script: string, timeoutSeconds: number) => Promise<string>;
  fetchText: (url: string) => Promise<string>;
  now: () => number;
}

export const liveDeps = (userAgent = "kernel-watch-probe"): ProbeDeps => ({
  run: async (ssh, script, timeoutSeconds) => {
    const proc = Bun.spawn(["ssh", "-o", "BatchMode=yes", "-o", "ConnectTimeout=20", ssh, "sh -s"], { stdin: new TextEncoder().encode(script), stdout: "pipe", stderr: "pipe", timeout: timeoutSeconds * 1000 });
    const [out, err, code] = await Promise.all([new Response(proc.stdout).text(), new Response(proc.stderr).text(), proc.exited]);
    if (code !== 0) throw new Error(`ssh ${ssh} exited ${code}: ${err.trim().slice(0, 300)}`);
    return out;
  },
  fetchText: async (url) => {
    const res = await fetch(url, { headers: { "user-agent": userAgent }, signal: AbortSignal.timeout(60_000) });
    if (!res.ok) throw new Error(`${url}: HTTP ${res.status}`);
    return res.text();
  },
  now: () => Math.floor(Date.now() / 1000),
});

/** Mainline rc, stable and the ordered cycle list from the watch store. */
export function referenceFromStore(store: Store, extra: KernelVersion[] = []): Reference {
  const rows = store.versionRows().filter((r) => r.source === "linux-releases");
  const pick = (channel: string): KernelVersion => {
    const row = rows.find((r) => r.channel === channel);
    const v = row ? parseVersion(row.version) : null;
    if (!v) throw new Error(`the store has no linux-releases ${channel} version; run "bun cli.ts poll --only linux-releases,linux-tags" first`);
    return v;
  };
  const rc = pick("mainline");
  const stable = pick("stable");
  const tags = store.items({ source: "linux-tags", kind: "tag" }).map((t) => t.id);
  return { mainline: formatVersion(rc), stable: formatVersion(stable), rc, stableVersion: stable, cycles: buildCycles(tags, [rc, stable, ...extra]) };
}

export async function collect(host: ProbeHost, config: ProbeConfig, deps: ProbeDeps): Promise<{ facts: RemoteFacts; candidates: Candidate[]; at: number; errors: string[] }> {
  const facts = parseRemote(await deps.run(host.ssh, REMOTE_SCRIPT, config.timeout));
  if (!facts.release) throw new Error("the remote probe returned no kernel release");
  const candidates: Candidate[] = [];
  const errors: string[] = [];
  const attempt = async (what: string, job: () => Promise<Candidate[]>): Promise<void> => {
    try {
      candidates.push(...(await job()));
    } catch (err) {
      errors.push(`${what}: ${err instanceof Error ? err.message : String(err)}`);
    }
  };
  const names = [...new Set([...config.packages, ...facts.installed.map((p) => p.name)])].sort();
  await Promise.all([
    ...names.map((name) => attempt(`arch ${name}`, async () => parseArchApi(await deps.fetchText(`${config.archApi}?name=${encodeURIComponent(name)}`)))),
    attempt("aur", async () => parseAur(await deps.fetchText(`${config.aurRpc}?${config.aur.map((n) => `arg[]=${encodeURIComponent(n)}`).join("&")}`))),
    ...config.cachyos.map((repo) => attempt(`cachyos ${repo.id}`, async () => parseCachyosIndex(await deps.fetchText(repo.url), repo.id, config.cachyosPackages, repo.url))),
  ]);

  const running = parseVersion(facts.release);
  if (running) candidates.push({ origin: "running", where: "uname -r", name: "running kernel", version: facts.release, normalized: formatVersion(running) });
  for (const p of facts.installed) {
    const v = parseVersion(p.version);
    if (v) candidates.push({ origin: "installed", where: "local db", name: p.name, version: p.version, normalized: formatVersion(v) });
  }
  for (const p of facts.sync) {
    const v = parseVersion(p.version);
    if (v) candidates.push({ origin: "host-repo", where: `${p.repo} (host sync db)`, name: p.name, version: p.version, normalized: formatVersion(v) });
  }
  for (const p of facts.pending ?? []) {
    const v = parseVersion(p.to);
    if (v && /^linux(-|$)/.test(p.name) && !/-(headers|docs|api-headers|firmware)$/.test(p.name) && p.name !== "linux-api-headers" && p.name !== "linux-firmware") {
      candidates.push({ origin: "pending", where: `pending update from ${p.from}`, name: p.name, version: p.to, normalized: formatVersion(v) });
    }
  }
  return { facts, candidates, at: deps.now(), errors };
}

const byCloseness = (a: RankedCandidate, b: RankedCandidate): number => {
  const ka = closenessKey(a), kb = closenessKey(b);
  const wa = a.origin === "running" ? 0 : 1, wb = b.origin === "running" ? 0 : 1;
  return ka[0] - kb[0] || ka[1] - kb[1] || ka[2] - kb[2] || wa - wb || (a.name < b.name ? -1 : a.name > b.name ? 1 : 0) || (a.where < b.where ? -1 : a.where > b.where ? 1 : 0);
};

export function buildReport(host: ProbeHost, collected: { facts: RemoteFacts; candidates: Candidate[]; at: number }, ref: Reference): ProbeReport {
  const parsedOf = (c: Candidate): KernelVersion => parseVersion(c.normalized)!;
  const extra = collected.candidates.map(parsedOf);
  const cycles = buildCycles([], [...ref.cycles.map((c) => parseVersion(c)!), ref.rc, ref.stableVersion, ...extra]);
  const ranked: RankedCandidate[] = collected.candidates.map((c) => ({ ...c, ...distance(parsedOf(c), ref.rc, ref.stableVersion, cycles) })).sort(byCloseness);
  const running = ranked.find((c) => c.origin === "running");
  const newest = (origins: Origin[]): RankedCandidate | null =>
    ranked.filter((c) => origins.includes(c.origin) && c.name !== "linux-lts" && !c.name.endsWith("-lts") && c.name !== "linux-hardened" && c.name !== "linux-rt").sort((a, b) => compareVersions(parsedOf(b), parsedOf(a)))[0] ?? null;
  const label = (c: RankedCandidate | null): string | null => (c ? `${c.name} ${c.version} (${c.where})` : null);
  const newestRepo = newest(["repo"]);
  return {
    host: host.id,
    label: host.label,
    at: collected.at,
    mainline: ref.mainline,
    stable: ref.stable,
    running: { release: collected.facts.release, build: collected.facts.build, normalized: running?.normalized ?? collected.facts.release, ...(running ? distanceOf(running) : distance(parseVersion(collected.facts.release)!, ref.rc, ref.stableVersion, cycles)) },
    syncedAt: collected.facts.syncedAt,
    repos: collected.facts.repos,
    pendingError: collected.facts.pendingError,
    pendingTotal: collected.facts.pending === null ? null : collected.facts.pending.length,
    summary: {
      runningCyclesBehindNewestRepo: running && newestRepo ? Math.max(0, cyclesBetween(cycles, parsedOf(running), parsedOf(newestRepo))) : null,
      newestRepo: label(newestRepo),
      newestTesting: label(newest(["testing"])),
      newestAur: label(newest(["aur"])),
      newestCachyos: label(newest(["cachyos"])),
    },
    candidates: ranked,
  };
}

const distanceOf = (d: Distance): Distance => ({ cyclesBehindRc: d.cyclesBehindRc, rcSteps: d.rcSteps, cyclesBehindStable: d.cyclesBehindStable, patchesBehindStable: d.patchesBehindStable, relation: d.relation });
const cyclesBetween = (cycles: string[], from: KernelVersion, to: KernelVersion): number => cycles.indexOf(`${to.major}.${to.minor}`) - cycles.indexOf(`${from.major}.${from.minor}`);

const cell = (text: string, max = 70): string => {
  const flat = text.replace(/\s+/g, " ").replace(/\|/g, "\\|").trim();
  return flat.length > max ? `${flat.slice(0, max - 1)}…` : flat;
};
export const isoOf = (epoch: number): string => new Date(epoch * 1000).toISOString().replace(/:\d\d\.\d+Z$/, "Z");

function lag(d: Distance): string {
  if (d.relation === "ahead") return `ahead by ${-d.cyclesBehindRc}`;
  if (d.cyclesBehindRc === 0) return d.rcSteps === 0 ? "at rc" : `${d.rcSteps} rc behind`;
  return `${d.cyclesBehindRc} cycle${d.cyclesBehindRc === 1 ? "" : "s"}`;
}

/** The generated page for one or more probe reports; deterministic for given reports. */
export function renderProbePage(reports: ProbeReport[]): string {
  const lines = ["# Host kernel probe", "", "Generated from the recorded probes in `data/probe/`; do not edit. Each probe is a read-only run of `bun tooling/kernel-watch/cli.ts probe <id>` against an Arch host (see [PLAN.md](PLAN.md)). Packages are ranked by closeness to the mainline rc, the same way as [versions.md](versions.md).", ""];
  if (reports.length === 0) return `${lines.join("\n")}\nNo probe has been recorded yet.\n`;
  for (const r of reports) {
    lines.push(`## ${r.host}: ${cell(r.label)}`, "", `Probed ${isoOf(r.at)}. Reference: mainline ${r.mainline}, stable ${r.stable}.`, "");
    lines.push("| | |", "| :-- | :-- |");
    lines.push(`| Running kernel | ${cell(r.running.release)} (${r.running.normalized}) |`);
    lines.push(`| Build | ${cell(r.running.build, 90)} |`);
    lines.push(`| Behind mainline rc | ${lag(r.running)} |`);
    lines.push(`| Behind stable | ${r.running.cyclesBehindStable === 0 ? "same cycle" : r.running.cyclesBehindStable > 0 ? `${r.running.cyclesBehindStable} cycle${r.running.cyclesBehindStable === 1 ? "" : "s"}` : "ahead"} |`);
    lines.push(`| Newest in Arch repositories | ${r.summary.newestRepo ? cell(r.summary.newestRepo) : "n/a"} |`);
    lines.push(`| Newest in testing repositories | ${r.summary.newestTesting ? cell(r.summary.newestTesting) : "none newer observed"} |`);
    lines.push(`| Newest in the AUR | ${r.summary.newestAur ? cell(r.summary.newestAur) : "n/a"} |`);
    lines.push(`| Newest in CachyOS repositories | ${r.summary.newestCachyos ? cell(r.summary.newestCachyos) : "n/a"} |`);
    lines.push(`| Running kernel behind newest Arch repository kernel | ${r.summary.runningCyclesBehindNewestRepo === null ? "n/a" : `${r.summary.runningCyclesBehindNewestRepo} cycle${r.summary.runningCyclesBehindNewestRepo === 1 ? "" : "s"}`} |`);
    lines.push(`| Pending updates (all packages) | ${r.pendingTotal === null ? `unknown (${cell(r.pendingError ?? "no result", 90)})` : r.pendingTotal} |`);
    lines.push(`| Host database sync | ${r.syncedAt === null ? "n/a" : isoOf(r.syncedAt)} |`);
    lines.push(`| Repositories | ${r.repos.length ? r.repos.join(", ") : "n/a"} |`, "");
    lines.push("| Rank | Origin | Package | Version | Where | Behind mainline rc | Behind stable |", "| --: | :-- | :-- | :-- | :-- | :-- | :-- |");
    r.candidates.forEach((c, i) => {
      const name = c.url ? `[${cell(c.name)}](${c.url})` : cell(c.name);
      const stable = c.cyclesBehindStable === 0 ? (c.patchesBehindStable === null ? "rc of next" : c.patchesBehindStable === 0 ? "current" : `${c.patchesBehindStable} patch${c.patchesBehindStable === 1 ? "" : "es"}`) : c.cyclesBehindStable < 0 ? `${-c.cyclesBehindStable} ahead` : `${c.cyclesBehindStable} cycle${c.cyclesBehindStable === 1 ? "" : "s"}`;
      lines.push(`| ${i + 1} | ${c.origin} | ${name} | ${cell(c.version, 40)} | ${cell(c.where, 50)} | ${lag(c)} | ${stable} |`);
    });
    lines.push("");
  }
  return `${lines.join("\n")}\n`;
}

/** Wraps deps so every host output and fetched body is saved under dir, to build test fixtures from a live run. */
export function recordingDeps(inner: ProbeDeps, dir: string): ProbeDeps {
  const safe = (url: string): string => url.replace(/^https?:\/\//, "").replace(/[^\w.-]+/g, "_").slice(0, 120);
  return {
    ...inner,
    run: async (ssh, script, timeout) => {
      const out = await inner.run(ssh, script, timeout);
      await Bun.write(`${dir}/remote.txt`, out);
      return out;
    },
    fetchText: async (url) => {
      const body = await inner.fetchText(url);
      await Bun.write(`${dir}/${safe(url)}`, body);
      return body;
    },
  };
}

/** Reads every recorded probe report under <watchDir>/data/probe, ordered by host id. */
export async function readProbeReports(watchDir: string): Promise<ProbeReport[]> {
  const files = [...new Bun.Glob("*.json").scanSync({ cwd: `${watchDir}/data/probe` })].sort();
  return Promise.all(files.map(async (f) => JSON.parse(await Bun.file(`${watchDir}/data/probe/${f}`).text()) as ProbeReport));
}
