// Hermetic probe tests: recorded host output and recorded repository bodies, no ssh and no network.
import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { parseLanes } from "./lanes.ts";
import { buildReport, collect, parseArchApi, parseAur, parseCachyosIndex, parseRemote, REMOTE_SCRIPT, renderProbePage, type ProbeDeps, type Reference } from "./probe.ts";
import { buildCycles, parseVersion } from "./version.ts";

const dir = join(import.meta.dir, "fixtures", "probe");
const read = (name: string): string => readFileSync(join(dir, name), "utf8");
const lanes = parseLanes(readFileSync(join(import.meta.dir, "lanes.toml"), "utf8"));
const rc = parseVersion("7.4-rc3")!;
const stable = parseVersion("7.3.4")!;
const ref: Reference = { mainline: "7.4-rc3", stable: "7.3.4", rc, stableVersion: stable, cycles: buildCycles(["v6.18", "v6.19", "v7.0", "v7.1", "v7.2", "v7.3", "v7.4-rc3"], [rc, stable]) };

const bodies = new Map<string, string>([
  ["name=linux-lts", read("arch-linux-lts.json")], ["name=linux-zen", read("arch-linux-zen.json")], ["name=linux-hardened", read("arch-linux-hardened.json")],
  ["name=linux-rt-lts", read("arch-linux-rt-lts.json")], ["name=linux-rt", read("arch-linux-rt.json")], ["name=linux&", read("arch-linux.json")],
  ["rpc/v5/info", read("aur.json")], ["x86_64/cachyos/", read("cachyos.html")], ["cachyos-v3/", read("cachyos-v3.html")], ["cachyos-v4/", read("cachyos-v4.html")], ["cachyos-znver4/", read("cachyos-znver4.html")],
]);
const deps: ProbeDeps = {
  run: async () => read("remote.txt"),
  fetchText: async (url) => {
    const key = [...bodies.keys()].find((k) => (k === "name=linux&" ? `${url}&`.includes(k) : url.includes(k)));
    if (!key) throw new Error(`no fixture for ${url}`);
    return bodies.get(key)!;
  },
  now: () => 1794893569,
};
const host = lanes.probe.hosts[0]!;

describe("probe", () => {
  test("the remote script only reads: no package manager write verb and no write outside a mktemp directory", () => {
    expect(REMOTE_SCRIPT).not.toMatch(/pacman -(S[^yl ]|[SU]\b|R|D|F|U)/);
    expect(REMOTE_SCRIPT).not.toMatch(/\b(reboot|systemctl|paru|yay|sed -i|tee|>>)\b/);
    expect(REMOTE_SCRIPT).not.toMatch(/\/(etc|boot|usr)\/[^'" ]*\s*>/);
    expect(REMOTE_SCRIPT).toContain("--dbpath");
  });

  test("lanes.toml names an ssh alias for the host and no address", () => {
    expect(host.ssh).toBeTruthy();
    expect(readFileSync(join(import.meta.dir, "lanes.toml"), "utf8")).not.toMatch(/\b\d{1,3}(\.\d{1,3}){3}\b/);
  });

  test("parses the recorded host output", () => {
    const facts = parseRemote(read("remote.txt"));
    expect(facts.release).toBe("7.0.3-arch1-1");
    expect(facts.installed).toEqual([{ name: "linux", version: "7.0.3.arch1-1" }]);
    expect(facts.sync.find((s) => s.name === "linux-zen")).toEqual({ repo: "extra", name: "linux-zen", version: "7.1.0.zen1-1" });
    expect(facts.repos).toEqual(["core", "extra"]);
    expect(facts.syncedAt).toBe(1790003600);
    expect(facts.pendingError).toBeNull();
    expect(facts.pending!.length).toBe(3);
    expect(facts.pending!.find((p) => p.name === "linux")).toEqual({ name: "linux", from: "7.0.3.arch1-1", to: "7.3.4.arch1-2" });
  });

  test("reads the boot setup facts needed to plan a kernel install", () => {
    const facts = parseRemote(read("remote.txt"));
    expect(facts.boot).toMatchObject({ firmware: "bios", virt: "kvm", "grub-cfg": "yes", root: "btrfs", serial: "ttyS0", "boot-free-mb": "800" });
    expect(facts.boot.cmdline).toContain("console=ttyS0");
    expect(facts.boot["boot-files"]).toContain("vmlinuz-linux");
  });

  test("reports a failed temporary sync as unknown, not as zero", () => {
    const facts = parseRemote("@@uname-r\n7.2.1-arch1-1\n@@pending\n@@pending-error\nerror: nope\n@@end\n");
    expect(facts.pending).toBeNull();
    expect(facts.pendingError).toContain("nope");
  });

  test("parses Arch API, AUR RPC and CachyOS listings", () => {
    const arch = parseArchApi(read("arch-linux.json"));
    expect(arch.map((c) => [c.name, c.where, c.version])).toContainEqual(["linux", "core", "7.3.4.arch1-2"]);
    const aur = parseAur(read("aur.json"));
    expect(aur.find((c) => c.name === "linux-mainline")).toMatchObject({ origin: "aur", version: "7.4rc3-1", normalized: "7.4-rc3" });
    const cachy = parseCachyosIndex(read("cachyos-znver4.html"), "cachyos-znver4", ["linux-cachyos", "linux-cachyos-rc"], "https://example.test/");
    expect(cachy.find((c) => c.name === "linux-cachyos-rc")).toMatchObject({ version: "7.4.rc2-1", normalized: "7.4-rc2" });
    expect(cachy.every((c) => !c.version.includes("dbg") && !c.version.includes("headers"))).toBe(true);
  });

  test("ranks candidates by closeness to the rc and scores the running kernel", async () => {
    const collected = await collect(host, lanes.probe, deps);
    expect(collected.errors).toEqual([]);
    const report = buildReport(host, collected, ref);
    expect(report.running).toMatchObject({ release: "7.0.3-arch1-1", cyclesBehindRc: 4, cyclesBehindStable: 3, relation: "older" });
    expect(report.summary.newestRepo).toBe("linux 7.3.4.arch1-2 (core)");
    expect(report.summary.newestAur).toContain("linux-mainline 7.4rc3-1");
    expect(report.summary.newestCachyos).toContain("linux-cachyos-rc 7.4.rc2-1");
    expect(report.summary.runningCyclesBehindNewestRepo).toBe(3);
    expect(report.pendingTotal).toBe(3);
    expect(report.candidates[0]).toMatchObject({ name: "linux-mainline", cyclesBehindRc: 0, rcSteps: 0 });
    const keys = report.candidates.map((c) => [Math.abs(c.cyclesBehindRc), c.rcSteps]);
    expect(keys).toEqual([...keys].sort((a, b) => a[0]! - b[0]! || a[1]! - b[1]!));
    expect(report.candidates.at(-1)!.cyclesBehindRc).toBeGreaterThan(3);
  });

  test("a failing source is reported and does not abort the probe", async () => {
    const broken: ProbeDeps = { ...deps, fetchText: async (url) => (url.includes("aur") ? Promise.reject(new Error("HTTP 503")) : deps.fetchText(url)) };
    const collected = await collect(host, lanes.probe, broken);
    expect(collected.errors).toEqual([expect.stringContaining("aur: ")]);
    expect(collected.candidates.some((c) => c.origin === "repo")).toBe(true);
  });

  test("the page is deterministic and lists the running kernel first among equals", async () => {
    const report = buildReport(host, await collect(host, lanes.probe, deps), ref);
    const page = renderProbePage([report]);
    expect(renderProbePage([JSON.parse(JSON.stringify(report))])).toBe(page);
    expect(page).toContain("| Running kernel | 7.0.3-arch1-1 (7.0.3) |");
    expect(page).toContain("| 1 | aur | [linux-mainline]");
    expect(renderProbePage([])).toContain("No probe has been recorded yet");
  });
});
