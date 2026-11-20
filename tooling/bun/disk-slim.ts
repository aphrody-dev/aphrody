#!/usr/bin/env bun
/**
 * Local Disk Footprint Auditor & Reproducible Slimmer for Aphrody.
 * Prunes multi-GB incremental Rust/Wasm target caches and wrangler dry-run bundles without touching tracked
 * sources. Forks are pinned Cargo git dependencies, so no checkout `.git` packs exist to compact.
 *
 * Usage:
 *   bun tooling/bun/disk-slim.ts audit [--json]
 *   bun tooling/bun/disk-slim.ts apply [--json]
 */
import { existsSync, readdirSync, rmSync, statSync } from "node:fs";
import { join, resolve } from "node:path";

export const repoRoot = resolve(import.meta.dir, "../..");

export interface DiskTargetMetric {
  relPath: string;
  bytes: number;
  mb: number;
  prunable: boolean;
  reason: string;
}

export interface DiskAuditReport {
  ok: boolean;
  repoRoot: string;
  totalMeasuredMb: number;
  prunableTargetMb: number;
  metrics: DiskTargetMetric[];
}

export const PRUNABLE_TARGET_PATHS: ReadonlyArray<{ rel: string; reason: string }> = [
  { rel: "target/debug", reason: "Local incremental debug Rust build artifacts (rebuild remotely on fr1)" },
  { rel: "target/wasm32-unknown-unknown", reason: "Cached wasm32 target artifacts" },
  { rel: "target/wasm32-wasip2", reason: "Cached wasip2 target artifacts" },
  { rel: "target/flycheck0", reason: "Rust-analyzer flycheck cache" },
  { rel: "target/flycheck1", reason: "Rust-analyzer flycheck cache" },
  { rel: "target/wrangler-dry", reason: "Wrangler dry-run bundle cache" },
  { rel: ".cloudflare/aphrody-releases/target", reason: "Legacy isolated worker wrangler-dry cache" },
  { rel: ".cloudflare/docs-proxy/target", reason: "Legacy isolated worker wrangler-dry cache" },
  { rel: ".cloudflare/open-source-website-assets/target", reason: "Legacy isolated worker wrangler-dry cache" },
  { rel: ".cloudflare/aphrody-github/target", reason: "Legacy isolated worker wrangler-dry cache" },
  { rel: ".cloudflare/aphrody-worker/target", reason: "Worker wrangler-dry cache" },
];

export function measurePathBytes(absPath: string): number {
  if (!existsSync(absPath)) return 0;
  let total = 0;
  const stack = [absPath];
  while (stack.length > 0) {
    const cur = stack.pop();
    if (!cur) continue;
    let stat;
    try {
      stat = statSync(cur);
    } catch {
      continue;
    }
    if (!stat.isDirectory()) {
      total += stat.size;
      continue;
    }
    let entries;
    try {
      entries = readdirSync(cur, { withFileTypes: true });
    } catch {
      continue;
    }
    for (const entry of entries) {
      const child = join(cur, entry.name);
      if (entry.isDirectory()) {
        stack.push(child);
      } else if (!entry.isSymbolicLink()) {
        try {
          total += statSync(child).size;
        } catch {
          // Ignore transient files
        }
      }
    }
  }
  return total;
}

export function auditLocalDisk(root = repoRoot): DiskAuditReport {
  const metrics: DiskTargetMetric[] = [];
  let prunableTargetMb = 0;
  for (const item of PRUNABLE_TARGET_PATHS) {
    const abs = join(root, item.rel);
    if (!existsSync(abs)) continue;
    const bytes = measurePathBytes(abs);
    const mb = Number((bytes / (1024 * 1024)).toFixed(2));
    prunableTargetMb += mb;
    metrics.push({
      relPath: item.rel,
      bytes,
      mb,
      prunable: true,
      reason: item.reason,
    });
  }

  return {
    ok: true,
    repoRoot: root,
    totalMeasuredMb: Number(prunableTargetMb.toFixed(2)),
    prunableTargetMb: Number(prunableTargetMb.toFixed(2)),
    metrics,
  };
}

function removeTreeBestEffort(absDir: string, skippedLocked: string[]): void {
  if (!existsSync(absDir)) return;
  try {
    rmSync(absDir, { recursive: true, force: true });
    return;
  } catch {
    // Fall back to per-entry cleanup on Windows when a single binary is held open by a live MCP daemon
  }
  let entries;
  try {
    entries = readdirSync(absDir, { withFileTypes: true });
  } catch {
    skippedLocked.push(absDir);
    return;
  }
  for (const entry of entries) {
    const child = join(absDir, entry.name);
    if (entry.isDirectory()) {
      removeTreeBestEffort(child, skippedLocked);
    } else {
      try {
        rmSync(child, { force: true });
      } catch {
        skippedLocked.push(child);
      }
    }
  }
  try {
    rmSync(absDir, { recursive: true, force: true });
  } catch {
    // Directory still holds a locked file
  }
}

export function applyDiskSlim(root = repoRoot): {
  ok: boolean;
  reclaimedMb: number;
  removedTargets: Array<{ relPath: string; mb: number; remainingMb: number }>;
  skippedLocked: string[];
} {
  const removedTargets: Array<{ relPath: string; mb: number; remainingMb: number }> = [];
  const skippedLocked: string[] = [];
  let reclaimedMb = 0;

  for (const item of PRUNABLE_TARGET_PATHS) {
    const abs = join(root, item.rel);
    if (!existsSync(abs)) continue;
    const beforeMb = Number((measurePathBytes(abs) / (1024 * 1024)).toFixed(2));
    removeTreeBestEffort(abs, skippedLocked);
    const remainingMb = existsSync(abs) ? Number((measurePathBytes(abs) / (1024 * 1024)).toFixed(2)) : 0;
    const freedMb = Number(Math.max(0, beforeMb - remainingMb).toFixed(2));
    removedTargets.push({ relPath: item.rel, mb: freedMb, remainingMb });
    reclaimedMb += freedMb;
  }

  return {
    ok: true,
    reclaimedMb: Number(reclaimedMb.toFixed(2)),
    removedTargets,
    skippedLocked,
  };
}

if (import.meta.main) {
  const args = Bun.argv.slice(2);
  const cmd = args[0] ?? "audit";

  if (cmd === "audit") {
    console.log(JSON.stringify(auditLocalDisk(repoRoot), null, 2));
  } else if (cmd === "apply") {
    console.log(JSON.stringify(applyDiskSlim(repoRoot), null, 2));
  } else {
    console.error("Usage: bun tooling/bun/disk-slim.ts [audit|apply] [--json]");
    process.exit(1);
  }
}
