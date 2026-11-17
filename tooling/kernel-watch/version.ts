// Kernel version parsing, ordering and distance. Ordering goes through Bun.semver: "7.4-rc3" < "7.4" < "7.4.1".
export interface KernelVersion {
  raw: string;
  major: number;
  minor: number;
  patch: number;
  /** Release candidate number, null for a final release. */
  rc: number | null;
}

const CORE = /^[vV]?(\d+)\.(\d+)(?:\.(\d+))?/;
const RC = /(?:^|[^a-z])rc[.-]?(\d+)/i;

/** Parses the kernel version at the start of any distro version string ("7.4.0-0.rc3.26.fc46", "7.4rc3-1", "6.12.0-284.el10"). */
export function parseVersion(raw: string): KernelVersion | null {
  const text = raw.trim();
  const core = CORE.exec(text);
  if (!core) return null;
  const rest = text.slice(core[0].length);
  const rc = RC.exec(rest);
  return { raw: text, major: +core[1]!, minor: +core[2]!, patch: core[3] ? +core[3] : 0, rc: rc ? +rc[1]! : null };
}

/** The semver form used for ordering: rc numbers become numeric identifiers so rc10 sorts after rc3. */
export function semverOf(v: KernelVersion): string {
  return `${v.major}.${v.minor}.${v.patch}${v.rc === null ? "" : `-rc.${v.rc}`}`;
}

export function compareVersions(a: KernelVersion, b: KernelVersion): number {
  return Bun.semver.order(semverOf(a), semverOf(b));
}

/** kernel.org naming: "7.4" for X.Y.0, "7.4-rc3" for candidates, "7.3.4" for stable updates. */
export function formatVersion(v: KernelVersion): string {
  const base = v.patch === 0 && v.rc === null ? `${v.major}.${v.minor}` : `${v.major}.${v.minor}${v.rc === null ? `.${v.patch}` : ""}`;
  return v.rc === null ? base : `${base}-rc${v.rc}`;
}

export const cycleOf = (v: KernelVersion): string => `${v.major}.${v.minor}`;

/** Ordered mainline cycles ("6.19", "7.0", ...) built from the tags the watch has seen plus the versions being compared. */
export function buildCycles(tagNames: string[], extra: KernelVersion[] = []): string[] {
  const seen = new Map<string, [number, number]>();
  for (const name of tagNames) {
    const v = /^v(\d+)\.(\d+)(?:-rc\d+)?$/.exec(name);
    if (v) seen.set(`${v[1]}.${v[2]}`, [+v[1]!, +v[2]!]);
  }
  for (const v of extra) seen.set(cycleOf(v), [v.major, v.minor]);
  return [...seen.entries()].sort((a, b) => a[1][0] - b[1][0] || a[1][1] - b[1][1]).map(([key]) => key);
}

export interface Distance {
  /** Mainline cycles between this kernel and the current release candidate (0: same cycle, positive: older). */
  cyclesBehindRc: number;
  /** Release candidates behind inside the rc's own cycle; 0 for any other cycle. */
  rcSteps: number;
  cyclesBehindStable: number;
  /** Stable point releases behind inside the latest stable cycle; null for other cycles. */
  patchesBehindStable: number | null;
  relation: "mainline-rc" | "latest-stable-cycle" | "older" | "ahead";
}

export function distance(v: KernelVersion, rc: KernelVersion, stable: KernelVersion, cycles: string[]): Distance {
  const idx = (x: KernelVersion): number => cycles.indexOf(cycleOf(x));
  const cyclesBehindRc = idx(rc) - idx(v);
  const cyclesBehindStable = idx(stable) - idx(v);
  const sameRcCycle = cyclesBehindRc === 0;
  const sameStableCycle = cyclesBehindStable === 0;
  return {
    cyclesBehindRc,
    rcSteps: sameRcCycle && v.rc !== null && rc.rc !== null ? Math.max(0, rc.rc - v.rc) : 0,
    cyclesBehindStable,
    patchesBehindStable: sameStableCycle && v.rc === null ? stable.patch - v.patch : null,
    relation: cyclesBehindRc < 0 ? "ahead" : sameRcCycle ? "mainline-rc" : sameStableCycle ? "latest-stable-cycle" : "older",
  };
}

/** Closeness to mainline: nearest cycle first, then fewest rc steps, then fewest stable patches behind. */
export function closenessKey(d: Distance): [number, number, number] {
  return [Math.abs(d.cyclesBehindRc), d.rcSteps, d.patchesBehindStable ?? 0];
}
