import { describe, expect, it } from "bun:test";
import { resolve } from "node:path";
import {
  categorizeUpstreamBlobs,
  checkToolParity,
  findModRsRecursive,
  REQUIRED_PILOT_TOOLS,
  runAudit,
  scanMigratedTree,
} from "./agent-desktop-audit.ts";

const repoRoot = resolve(import.meta.dir, "../..");

describe("agent-desktop purge & AI OS pilot migration", () => {
  it("categorizes bloat directories cleanly into purge buckets", () => {
    const sample = [
      { path: "assets/Videos/demo.mp4", size: 10_000_000 },
      { path: "apps/docs/app/page.tsx", size: 4_000 },
      { path: "probes/windows/01-tree-dump.ps1", size: 40_000 },
      { path: "crates/ffi/src/lib.rs", size: 2_000 },
      { path: "tests/fixture-app-windows/AgentDeskFixture.cs", size: 5_000 },
      { path: "npm/package.json", size: 800 },
      { path: "crates/core/src/cursor_overlay/mod.rs", size: 1_200 },
    ];
    const buckets = categorizeUpstreamBlobs(sample);
    for (const bucket of buckets) {
      expect(bucket.count).toBeGreaterThanOrEqual(1);
      expect(bucket.bytes).toBeGreaterThan(0);
    }
  });

  it("enforces Zed style (zero mod.rs in crates/aphrody_pilot) and zero Vercel brand leaks", () => {
    const modRs = findModRsRecursive(`${repoRoot}/crates/aphrody_pilot/src`);
    expect(modRs).toEqual([]);
    const migrated = scanMigratedTree(repoRoot);
    for (const file of migrated) {
      expect(file.exists).toBe(true);
      expect(file.bannedBrandLeaks).toEqual([]);
    }
  });

  it("maintains 100% parity across all 55+ desktop/AI OS pilot tools", () => {
    const { present, missing } = checkToolParity(
      `${repoRoot}/crates/aphrody_pilot/src/aphrody_pilot.rs`,
    );
    expect(missing).toEqual([]);
    expect(present.length).toBe(REQUIRED_PILOT_TOOLS.length);
  });

  it("passes full offline audit cleanly", () => {
    const report = runAudit(repoRoot, true);
    expect(report.ok).toBe(true);
    expect(report.zeroModRsInOsPilot).toBe(true);
    expect(report.zeroBannedLeaks).toBe(true);
    expect(report.missingMcpTools).toEqual([]);
  });
});
