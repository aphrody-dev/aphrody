import { describe, expect, test } from "bun:test";
import {
  auditMonorepoReadiness,
  buildParityMatrix,
  CORE_WEB_VITALS_BUDGETS,
  renderArchitectureMarkdown,
  renderParityMatrixToml,
  runParityBenchmarks,
} from "./prod-standards-starterpack.ts";

describe("prod-standards-starterpack", () => {
  test("builds fork submodule parity matrix including aphrody-core and 10 kept vendor stacks", () => {
    const matrix = buildParityMatrix();
    expect(matrix.length).toBe(11);
    const core = matrix.find((r) => r.id === "aphrody-core");
    expect(core).toBeDefined();
    expect(core?.shadcnBaseUiStarter).toBe(true);
    expect(core?.googleDesignTailwind).toBe(true);
    expect(core?.wgpuV30Webgpu).toBe(true);
    expect(CORE_WEB_VITALS_BUDGETS.lcpGoodMs).toBe(2500);
    expect(CORE_WEB_VITALS_BUDGETS.inpGoodMs).toBe(200);
    expect(CORE_WEB_VITALS_BUDGETS.clsGoodMax).toBe(0.1);
  });

  test("executes parity benchmarks and renders valid TOML + Markdown outputs", () => {
    const matrix = buildParityMatrix();
    const benches = runParityBenchmarks();
    const audit = auditMonorepoReadiness();
    expect(benches.length).toBe(4);
    expect(benches.every((b) => b.opsPerSec > 0)).toBe(true);
    const toml = renderParityMatrixToml(matrix, audit);
    expect(toml).toContain('design_source_of_truth = "https://design.google/"');
    expect(toml).toContain("[[frameworks]]");
    const md = renderArchitectureMarkdown(matrix, benches, audit);
    expect(md).toContain("Core Web Vitals & GPUI Frame Budgets");
    expect(md).toContain("Fork Submodule Parity Matrix");
    expect(audit.tsCompileErrors).toBe(0);
    expect(audit.rustModRsViolations).toBe(0);
    expect(audit.machinePathLeaks).toBe(0);
  });
});
