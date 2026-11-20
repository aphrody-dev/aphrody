import { describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { applyDiskSlim, auditLocalDisk, measurePathBytes } from "./disk-slim.ts";

describe("disk-slim local footprint reducer", () => {
  test("audits and prunes target/debug + legacy worker targets cleanly", () => {
    const dir = mkdtempSync(join(tmpdir(), "disk-slim-test-"));
    try {
      mkdirSync(join(dir, "target", "debug"), { recursive: true });
      mkdirSync(join(dir, "target", "wasm32-unknown-unknown"), { recursive: true });
      mkdirSync(join(dir, "crates", "gpui"), { recursive: true });
      writeFileSync(join(dir, "target", "debug", "big.pdb"), new Uint8Array(1024 * 64));
      writeFileSync(join(dir, "target", "wasm32-unknown-unknown", "app.wasm"), new Uint8Array(1024 * 32));
      writeFileSync(join(dir, "crates", "gpui", "lib.rs"), "// source kept");

      const before = auditLocalDisk(dir);
      expect(before.metrics.length).toBe(2);
      expect(measurePathBytes(join(dir, "target", "debug"))).toBe(1024 * 64);

      const applied = applyDiskSlim(dir);
      expect(applied.ok).toBe(true);
      expect(applied.removedTargets.map((t) => t.relPath)).toEqual([
        "target/debug",
        "target/wasm32-unknown-unknown",
      ]);
      const after = auditLocalDisk(dir);
      expect(after.prunableTargetMb).toBe(0);
      expect(measurePathBytes(join(dir, "crates", "gpui"))).toBeGreaterThan(0);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});
