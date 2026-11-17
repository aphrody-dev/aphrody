// The rc lane's records: parsing of the guest's serial output, the dmesg comparison with the previous kernel, the generated page and
// the test status shown in the versions table. Pure functions; the host side that builds and boots lives in lane-host.ts.
import type { LaneKernel } from "./lanes.ts";

export type CheckStatus = "pass" | "fail" | "skip";
export interface Check {
  name: string;
  status: CheckStatus;
  detail: string;
}
export interface DmesgCounts {
  err: number;
  warn: number;
  crit: number;
  bug: number;
}
export interface LaneRecord {
  id: string;
  name: string;
  /** Versions-table keys this record reports to. */
  kernel: string;
  channel: string;
  source: LaneKernel["source"];
  package: string;
  /** Package version that was booted, as pacman prints it. */
  version: string;
  /** uname -r inside the guest. */
  release: string;
  /** Epoch seconds when the run finished. */
  at: number;
  buildSeconds: number | null;
  bootSeconds: number;
  /** Where the kernel came from: AUR commit, CachyOS repository and file, or the Arch repository. */
  origin: string;
  /** Versions of the kernel packages seen in the testing repositories from inside the guest. */
  testing: { repo: string; package: string; version: string }[];
  checks: Check[];
  /** Numbers the guest reported (dmesg counts, kselftest ok and not ok, pacman -Syu seconds). */
  metrics: Record<string, number>;
  /** Kselftest tests that failed in this run. */
  kselftestFailures: string[];
  dmesg: DmesgCounts | null;
  /** Counts of the previous run of this kernel, or of the baseline kernel when there is none. */
  dmesgPrevious: { against: string; counts: DmesgCounts } | null;
  status: "pass" | "fail";
  serial: string[];
}

export interface ParsedSerial {
  release: string | null;
  checks: Check[];
  metrics: Record<string, number>;
  testing: { repo: string; package: string; version: string }[];
  kselftestFailures: string[];
  /** Lines that prove the boot: the kernel banner, the guest's own uname, the login prompt. */
  excerpt: string[];
  finished: boolean;
}

const ANSI = /\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)|\x1b\[[0-9;?]*[ -\/]*[@-~]|\x1b[78=>]/g;

export function parseSerial(text: string): ParsedSerial {
  const checks: Check[] = [];
  const metrics: Record<string, number> = {};
  const testing: ParsedSerial["testing"] = [];
  const excerpt: string[] = [];
  const kselftestFailures: string[] = [];
  let release: string | null = null;
  let finished = false;
  for (const raw of text.split(/\r?\n/)) {
    const clean = raw.replace(ANSI, "");
    const at = clean.indexOf("LANE-");
    const line = (at >= 0 ? clean.slice(at) : clean).trim();
    if (!line) continue;
    const begin = /^LANE-BEGIN (\S+)/.exec(line);
    const result = /^LANE-RESULT (\S+) (pass|fail|skip) ?(.*)$/.exec(line);
    const metric = /^LANE-METRIC (\S+) (-?\d+(?:\.\d+)?)$/.exec(line);
    const repoVersion = /^LANE-TESTING (\S+) (\S+) (\S+)$/.exec(line);
    if (begin) {
      release = begin[1]!;
      excerpt.push(line);
    } else if (result) checks.push({ name: result[1]!, status: result[2] as CheckStatus, detail: result[3]! });
    else if (metric) metrics[metric[1]!] = Number(metric[2]);
    else if (repoVersion) testing.push({ package: repoVersion[1]!, repo: repoVersion[2]!, version: repoVersion[3]! });
    else if (line.startsWith("LANE-KSELFTEST ")) kselftestFailures.push(line.slice("LANE-KSELFTEST ".length));
    else if (line === "LANE-END") finished = true;
    else if (/^Linux version \d/.test(line.replace(/^\[\s*[\d.]+\]\s*/, "")) || /login:\s*$/.test(line) || /^Arch Linux \S+ \(ttyS0\)/.test(line)) excerpt.push(line);
  }
  return { release, checks, metrics, testing, kselftestFailures, excerpt: excerpt.slice(0, 6), finished };
}

export const dmesgFrom = (metrics: Record<string, number>): DmesgCounts | null =>
  "dmesg-err" in metrics ? { err: metrics["dmesg-err"]!, warn: metrics["dmesg-warn"] ?? 0, crit: metrics["dmesg-crit"] ?? 0, bug: metrics["dmesg-bug"] ?? 0 } : null;

/** Every expected check must have run; a check the guest never reported counts as a failure, not a skip. */
export const EXPECTED_CHECKS = ["boot", "login", "dmesg", "analyze", "network", "pacman-syu", "pacman-qkk", "kselftest"] as const;

export function finalizeChecks(parsed: ParsedSerial, timedOut: boolean): Check[] {
  const seen = new Map(parsed.checks.map((c) => [c.name, c]));
  return EXPECTED_CHECKS.map((name) => seen.get(name) ?? { name, status: "fail" as const, detail: timedOut ? "guest did not report before the boot timeout" : "guest did not report this check" });
}

/**
 * Kselftest failures are judged against a reference kernel (the previous run, else the baseline): a kernel fails only when it has more
 * failing tests than the reference, because some tests fail on any kernel in a minimal guest.
 */
export function judgeKselftest(checks: Check[], metrics: Record<string, number>, failures: string[], reference: { against: string; notOk: number } | null): Check[] {
  const notOk = metrics["kselftest-notok"];
  return checks.map((c) => {
    if (c.name !== "kselftest" || c.status !== "pass" || notOk === undefined) return c;
    const named = failures.slice(0, 3).map((f) => f.replace(/\s+#.*$/, "")).join("; ");
    if (!reference) return { ...c, detail: `${c.detail}; no reference run yet${named ? `; failing: ${named}` : ""}` };
    if (notOk > reference.notOk) return { ...c, status: "fail", detail: `${c.detail}; ${notOk} not ok against ${reference.notOk} in ${reference.against}${named ? `; failing: ${named}` : ""}` };
    return { ...c, detail: `${c.detail}; ${reference.notOk} not ok in ${reference.against}` };
  });
}

export const overall = (checks: Check[]): "pass" | "fail" => (checks.some((c) => c.status === "fail") ? "fail" : "pass");

const delta = (now: number, before: number): string => (now === before ? "same" : now > before ? `+${now - before}` : `${now - before}`);
export function dmesgComparison(record: LaneRecord): string {
  if (!record.dmesg) return "n/a";
  const now = record.dmesg;
  if (!record.dmesgPrevious) return `err ${now.err}, warn ${now.warn} (no previous run)`;
  const before = record.dmesgPrevious.counts;
  return `err ${now.err} (${delta(now.err, before.err)}), warn ${now.warn} (${delta(now.warn, before.warn)}) vs ${record.dmesgPrevious.against}`;
}

export const iso = (epoch: number): string => new Date(epoch * 1000).toISOString().replace(/:\d\d\.\d+Z$/, "Z");
const cell = (text: string, max = 90): string => {
  const flat = text.replace(/\s+/g, " ").replace(/\|/g, "\\|").trim();
  return flat.length > max ? `${flat.slice(0, max - 1)}…` : flat;
};
const duration = (seconds: number | null): string => (seconds === null ? "n/a" : seconds >= 3600 ? `${Math.floor(seconds / 3600)}h${String(Math.floor((seconds % 3600) / 60)).padStart(2, "0")}m` : seconds >= 60 ? `${Math.floor(seconds / 60)}m${String(Math.round(seconds % 60)).padStart(2, "0")}s` : `${Math.round(seconds)}s`);

export function laneSummary(r: LaneRecord): string {
  const passed = r.checks.filter((c) => c.status === "pass").length;
  const skipped = r.checks.filter((c) => c.status === "skip").length;
  return `${r.status} ${passed}/${r.checks.length - skipped}`;
}

/** Test status keyed by "<kernel>\0<channel>" for the versions table. */
export function laneIndex(records: LaneRecord[]): Map<string, { summary: string; version: string; at: number }> {
  const index = new Map<string, { summary: string; version: string; at: number }>();
  for (const r of records) {
    const key = `${r.kernel}\u0000${r.channel}`;
    const held = index.get(key);
    if (!held || r.at > held.at) index.set(key, { summary: laneSummary(r), version: r.version, at: r.at });
  }
  return index;
}

export function renderLanePage(records: LaneRecord[]): string {
  const head = [
    "# rc lane", "",
    "Generated from the records in `data/lane/`; do not edit. The lane runs on the build host: it builds or fetches each kernel, boots it in QEMU/KVM against an Arch root filesystem and runs the smoke suite (see [PLAN.md](PLAN.md) for what runs where and how to rerun it). Test status also appears in [versions.md](versions.md).", "",
  ];
  if (records.length === 0) return `${head.join("\n")}\nNo run has been recorded yet.\n`;
  const sorted = [...records].sort((a, b) => (a.id < b.id ? -1 : 1));
  const lines = [...head, "## Latest run per kernel", "", "| Kernel | Package | Guest uname | Status | Checks | dmesg vs previous | Build | Boot and suite | Run |", "| :-- | :-- | :-- | :-- | :-- | :-- | --: | --: | :-- |"];
  for (const r of sorted) lines.push(`| ${cell(r.name)} | ${cell(`${r.package} ${r.version}`, 50)} | ${cell(r.release, 40)} | ${r.status} | ${laneSummary(r)} | ${cell(dmesgComparison(r), 70)} | ${duration(r.buildSeconds)} | ${duration(r.bootSeconds)} | ${iso(r.at)} |`);
  lines.push("");
  for (const r of sorted) {
    lines.push(`## ${r.name}`, "", `- Package: ${r.package} ${r.version} (${cell(r.origin, 160)})`, `- Guest uname -r: ${r.release}`, `- Run finished: ${iso(r.at)}`, "");
    lines.push("| Check | Result | Detail |", "| :-- | :-- | :-- |", ...r.checks.map((c) => `| ${c.name} | ${c.status} | ${cell(c.detail, 120)} |`), "");
    if (r.testing.length) lines.push("Kernel packages in the testing repositories seen from the guest:", "", ...r.testing.map((t) => `- ${t.package} ${t.version} (${t.repo})`), "");
    if (r.serial.length) lines.push("Serial log excerpt:", "", "```", ...r.serial, "```", "");
  }
  return `${lines.join("\n")}\n`;
}

export async function readLaneRecords(watchDir: string): Promise<LaneRecord[]> {
  const dir = `${watchDir}/data/lane`;
  const files = [...new Bun.Glob("*.json").scanSync({ cwd: dir })].sort();
  return Promise.all(files.map(async (f) => JSON.parse(await Bun.file(`${dir}/${f}`).text()) as LaneRecord));
}

/** Reads the latest record of every kernel from the build host's records directory over ssh and returns them (read-only). */
export async function fetchLaneRecords(ssh: string, root: string): Promise<LaneRecord[]> {
  const proc = Bun.spawn(["ssh", "-o", "BatchMode=yes", "-o", "ConnectTimeout=20", ssh, "sh", "-s", "--", root], {
    stdin: new TextEncoder().encode('for f in "$1"/records/*/latest.json; do [ -f "$f" ] && { cat "$f"; echo; }; done\n'),
    stdout: "pipe", stderr: "pipe",
  });
  const [out, err, code] = await Promise.all([new Response(proc.stdout).text(), new Response(proc.stderr).text(), proc.exited]);
  if (code !== 0) throw new Error(`ssh ${ssh}: ${err.trim().slice(0, 300)}`);
  return out.split("\n").filter(Boolean).map((line) => JSON.parse(line) as LaneRecord);
}
