import { describe, expect, test } from "bun:test";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  auditWorkspace,
  repoRoot,
  serializeLedgerToml,
  WASM_WGPU_ISSUES,
} from "./zed-wasm-wgpu-issues.ts";

describe("zed-wasm-wgpu-issues ledger", () => {
  test("serializes valid TOML covering upstream WGPU/Wasm issues", () => {
    const parsed = Bun.TOML.parse(serializeLedgerToml()) as {
      purged_legacy_python_scripts: number;
      tracked_wasm_wgpu_issues: number;
      issues: Array<{ id: string }>;
    };
    expect(parsed.purged_legacy_python_scripts).toBe(13);
    expect(parsed.tracked_wasm_wgpu_issues).toBe(WASM_WGPU_ISSUES.length);
    const ids = new Set(parsed.issues.map((i) => i.id));
    expect(ids.has("zed#66595")).toBe(true);
    expect(ids.has("zed#62998")).toBe(true);
    expect(ids.has("zed#63327")).toBe(true);
    expect(ids.has("zed#65960")).toBe(true);
    expect(ids.has("zed#67052")).toBe(true);
  });

  test("confirms all 13 non-Wasm Python scripts are purged and referenced crates exist", () => {
    const dir = mkdtempSync(join(tmpdir(), "wasm-wgpu-ledger-"));
    try {
      const report = auditWorkspace(repoRoot, join(dir, "ledger.toml"), true);
      expect(report.missingCratePaths).toEqual([]);
      expect(report.remainingLegacyScripts).toEqual([]);
      expect(report.ok).toBe(true);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});
