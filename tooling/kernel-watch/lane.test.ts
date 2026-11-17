// Hermetic lane tests: a recorded guest serial log, no QEMU and no ssh.
import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { dmesgComparison, dmesgFrom, EXPECTED_CHECKS, finalizeChecks, judgeKselftest, laneIndex, laneSummary, overall, parseSerial, renderLanePage, type LaneRecord } from "./lane.ts";
import { parseLanes } from "./lanes.ts";
import { versionsTable, type VersionEntry, type VersionsView } from "./views.ts";

const serial = readFileSync(join(import.meta.dir, "fixtures", "lane", "serial-arch-baseline.log"), "utf8");
const lanes = parseLanes(readFileSync(join(import.meta.dir, "lanes.toml"), "utf8"));

function record(overrides: Partial<LaneRecord> = {}): LaneRecord {
  const parsed = parseSerial(serial);
  const checks = finalizeChecks(parsed, false);
  return {
    id: "arch-baseline", name: "Arch linux (stable baseline)", kernel: "arch", channel: "core/linux", source: "repo", package: "linux", version: "7.3.4.arch1-2",
    release: parsed.release!, at: 1794893569, buildSeconds: null, bootSeconds: 80, origin: "Arch repository", testing: parsed.testing, checks, metrics: parsed.metrics, kselftestFailures: parsed.kselftestFailures, dmesg: dmesgFrom(parsed.metrics),
    dmesgPrevious: null, status: overall(checks), serial: parsed.excerpt, ...overrides,
  };
}

describe("lane", () => {
  test("lanes.toml lists the three kernels and a schedule", () => {
    expect(lanes.lane.kernels.map((k) => k.id)).toEqual(["arch-baseline", "arch-mainline", "cachyos-rc"]);
    expect(lanes.lane.timer).toBeTruthy();
    expect(readFileSync(join(import.meta.dir, "lanes.toml"), "utf8")).not.toMatch(/\b\d{1,3}(\.\d{1,3}){3}\b/);
  });

  test("parses the guest serial log through journal prefixes and terminal escapes", () => {
    const parsed = parseSerial(serial);
    expect(parsed.release).toMatch(/^\d+\.\d+/);
    expect(parsed.finished).toBe(true);
    expect(parsed.checks.map((c) => c.name)).toEqual([...EXPECTED_CHECKS]);
    expect(parsed.checks.find((c) => c.name === "analyze")!.detail).not.toContain("\x1b");
    expect(parsed.metrics["dmesg-bug"]).toBe(0);
    expect(parsed.excerpt.some((l) => /Linux version \d/.test(l))).toBe(true);
  });

  test("a check the guest never reported fails, with the timeout named", () => {
    const parsed = parseSerial("[ 1.0] lane-smoke[1]: LANE-BEGIN 7.4.0-rc3\n[ 2.0] lane-smoke[1]: LANE-RESULT boot pass systemd running\n");
    const checks = finalizeChecks(parsed, true);
    expect(checks.find((c) => c.name === "boot")!.status).toBe("pass");
    expect(checks.find((c) => c.name === "kselftest")).toMatchObject({ status: "fail", detail: expect.stringContaining("timeout") });
    expect(overall(checks)).toBe("fail");
  });

  test("kselftest failures fail the kernel only when they exceed the reference kernel's", () => {
    const parsed = parseSerial(serial);
    expect(parsed.metrics["kselftest-notok"]).toBe(1);
    expect(parsed.kselftestFailures).toEqual(["selftests: membarrier: membarrier_rseq_stress # exit=1"]);
    const checks = finalizeChecks(parsed, false);
    const verdict = (reference: { against: string; notOk: number } | null): { status: string; detail: string } => judgeKselftest(checks, parsed.metrics, parsed.kselftestFailures, reference).find((c) => c.name === "kselftest")!;
    expect(verdict({ against: "baseline", notOk: 1 }).status).toBe("pass");
    expect(verdict(null).status).toBe("pass");
    const worse = verdict({ against: "baseline", notOk: 0 });
    expect(worse.status).toBe("fail");
    expect(worse.detail).toContain("membarrier_rseq_stress");
  });

  test("dmesg comparison reports deltas against the previous kernel", () => {
    const now = record({ dmesg: { err: 3, warn: 6, crit: 0, bug: 0 }, dmesgPrevious: { against: "previous run", counts: { err: 1, warn: 8, crit: 0, bug: 0 } } });
    expect(dmesgComparison(now)).toBe("err 3 (+2), warn 6 (-2) vs previous run");
    expect(dmesgComparison(record({ dmesgPrevious: null }))).toContain("no previous run");
  });

  test("the page is deterministic and carries the serial excerpt", () => {
    const page = renderLanePage([record()]);
    expect(renderLanePage([JSON.parse(JSON.stringify(record()))])).toBe(page);
    expect(page).toContain("Serial log excerpt");
    expect(page).toContain("Linux version");
    expect(renderLanePage([])).toContain("No run has been recorded yet");
  });

  test("the versions table gains a linked Test lane column only when a record exists", () => {
    const entry: VersionEntry = {
      kernel: "arch", kernelName: "Arch Linux", role: "downstream", channel: "core/linux", version: "7.3.4.arch1-2", normalized: "7.3.4", since: 1794893569, source: "arch-core",
      cyclesBehindRc: 1, rcSteps: 0, cyclesBehindStable: 0, patchesBehindStable: 0, relation: "latest-stable-cycle",
    };
    const view: VersionsView = { asOf: 1794893569, mainline: "7.4-rc3", stable: "7.3.4", longterm: [], entries: [entry], distros: [entry] };
    expect(versionsTable(view)).not.toContain("Test lane");
    const table = versionsTable(view, view.distros, true, laneIndex([record()]));
    expect(table).toContain("| Test lane |");
    expect(table).toContain(`[${laneSummary(record())} on 7.3.4.arch1-2](lane.md)`);
  });
});
