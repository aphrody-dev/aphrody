import { describe, expect, test } from "bun:test";
import { runConsolidatedChecks, TOOLING_REGISTRY, verifyRegistryFilesOnDisk } from "./cli";

describe("Consolidated Bun xtask CLI router (tooling/bun/cli.ts)", () => {
  test("maps all 7 domains cleanly to existing repo-native scripts with zero missing files", () => {
    const check = verifyRegistryFilesOnDisk();
    expect(check.missing).toEqual([]);
    expect(check.ok).toBe(true);

    const domains = new Set(TOOLING_REGISTRY.map((c) => c.domain));
    expect([...domains].sort()).toEqual([
      "design",
      "fleet",
      "ingest",
      "session",
      "storytold",
      "web",
      "workspace",
    ]);
  });

  test("executes consolidated offline check pipeline with zero failures", () => {
    const report = runConsolidatedChecks();
    expect(report.steps.map((s) => ({ id: s.id, ok: s.ok }))).toEqual([
      { id: "workspace:hygiene", ok: true },
      { id: "design:audit", ok: true },
      { id: "design:pack-verify", ok: true },
      { id: "storytold:audit", ok: true },
      { id: "workspace:agy-audit", ok: true },
      { id: "workspace:i18n", ok: true },
    ]);
    expect(report.ok).toBe(true);
  });
});
