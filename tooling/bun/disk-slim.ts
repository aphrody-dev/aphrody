#!/usr/bin/env bun
/**
 * Local Disk Footprint Auditor & Reproducible Slimmer for Aphrody.
 * Prunes multi-GB incremental Rust/Wasm target caches and compacts bloated submodule `.git` packs
 * (such as `vendor/browser/.git`) without altering pinned submodule commits or tracked sources.
 *
 * Usage:
 *   bun tooling/bun/disk-slim.ts audit [--json]
 *   bun tooling/bun/disk-slim.ts apply [--keep-git-pack] [--json]
 */
import { existsSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
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
  compactableSubmoduleGitMb: number;
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

export function findBloatedSubmoduleGitDirs(root = repoRoot, thresholdMb = 40): Array<{ subPath: string; gitDir: string; bytes: number }> {
  const vendorDir = join(root, "vendor");
  if (!existsSync(vendorDir)) return [];
  const findings: Array<{ subPath: string; gitDir: string; bytes: number }> = [];
  for (const entry of readdirSync(vendorDir, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    const subPath = `vendor/${entry.name}`;
    const gitDir = join(vendorDir, entry.name, ".git");
    if (existsSync(gitDir) && statSync(gitDir).isDirectory()) {
      const bytes = measurePathBytes(gitDir);
      if (bytes >= thresholdMb * 1024 * 1024) {
        findings.push({ subPath, gitDir, bytes });
      }
    }
  }
  findings.sort((a, b) => b.bytes - a.bytes);
  return findings;
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

  let compactableSubmoduleGitMb = 0;
  for (const sub of findBloatedSubmoduleGitDirs(root, 40)) {
    const mb = Number((sub.bytes / (1024 * 1024)).toFixed(2));
    compactableSubmoduleGitMb += mb;
    metrics.push({
      relPath: `${sub.subPath}/.git`,
      bytes: sub.bytes,
      mb,
      prunable: true,
      reason: "Multi-pack submodule git history (compactable to shallow depth=1 at pinned SHA)",
    });
  }

  const totalMeasuredMb = Number((prunableTargetMb + compactableSubmoduleGitMb).toFixed(2));
  const browserThirdPartyDir = join(root, "vendor", "browser", "third_party");
  if (existsSync(browserThirdPartyDir)) {
    const bytes = measurePathBytes(browserThirdPartyDir);
    metrics.push({
      relPath: "vendor/browser/third_party",
      bytes,
      mb: Number((bytes / (1024 * 1024)).toFixed(2)),
      prunable: false,
      reason: "Slimmed to zero-byte submodule mountpoints + lockfile pins (mode = pin)",
    });
  }
  return {
    ok: true,
    repoRoot: root,
    totalMeasuredMb,
    prunableTargetMb: Number(prunableTargetMb.toFixed(2)),
    compactableSubmoduleGitMb: Number(compactableSubmoduleGitMb.toFixed(2)),
    metrics,
  };
}

function shallowCompactSubmodule(root: string, subPath: string): { beforeMb: number; afterMb: number; compacted: boolean; note: string } {
  const absSub = join(root, subPath);
  const gitDir = join(absSub, ".git");
  const beforeBytes = measurePathBytes(gitDir);
  const beforeMb = Number((beforeBytes / (1024 * 1024)).toFixed(2));

  const statusRes = Bun.spawnSync(["git", "-C", absSub, "status", "--porcelain"], { stdout: "pipe", stderr: "pipe" });
  if (statusRes.exitCode !== 0 || new TextDecoder().decode(statusRes.stdout).trim().length > 0) {
    return { beforeMb, afterMb: beforeMb, compacted: false, note: "Skipped dirty or invalid submodule working tree" };
  }

  const headRes = Bun.spawnSync(["git", "-C", absSub, "rev-parse", "HEAD"], { stdout: "pipe", stderr: "pipe" });
  const originRes = Bun.spawnSync(["git", "-C", absSub, "remote", "get-url", "origin"], { stdout: "pipe", stderr: "pipe" });
  const sha = new TextDecoder().decode(headRes.stdout).trim();
  const origin = new TextDecoder().decode(originRes.stdout).trim();
  if (!sha || !origin) {
    return { beforeMb, afterMb: beforeMb, compacted: false, note: "Missing HEAD SHA or origin URL" };
  }

  // Try shallow re-fetch at depth=1 + aggressive prune; keeps worktree untouched & exact SHA checked out.
  const fetchShallow = Bun.spawnSync(["git", "-C", absSub, "fetch", "--depth=1", "origin", sha], {
    stdout: "pipe",
    stderr: "pipe",
  });
  if (fetchShallow.exitCode !== 0) {
    return {
      beforeMb,
      afterMb: beforeMb,
      compacted: false,
      note: `Shallow fetch failed: ${new TextDecoder().decode(fetchShallow.stderr).trim().slice(0, 180)}`,
    };
  }

  Bun.spawnSync(["git", "-C", absSub, "checkout", "--detach", sha], { stdout: "pipe", stderr: "pipe" });
  Bun.spawnSync(["git", "-C", absSub, "tag", "-d", ...listGitTags(absSub)], { stdout: "pipe", stderr: "pipe" });
  for (const ref of listRefs(absSub)) {
    if (ref !== "HEAD") {
      Bun.spawnSync(["git", "-C", absSub, "update-ref", "--no-deref", "-d", ref], { stdout: "pipe", stderr: "pipe" });
    }
  }
  const shallowFile = join(gitDir, "shallow");
  Bun.spawnSync(["git", "-C", absSub, "config", "--unset-all", "branch.main.remote"], { stdout: "pipe", stderr: "pipe" });
  writeFileSync(shallowFile, `${sha}\n`, "utf8");
  Bun.spawnSync(["git", "-C", absSub, "-c", "gc.pruneExpire=now", "reflog", "expire", "--expire=now", "--all"], {
    stdout: "pipe",
    stderr: "pipe",
  });
  // Use `-a -d` (not `-A`) so unreachable historical blobs are discarded immediately instead of unpacked as loose files.
  Bun.spawnSync(["git", "-C", absSub, "repack", "-a", "-d", "--depth=1", "--window=10"], {
    stdout: "pipe",
    stderr: "pipe",
  });
  Bun.spawnSync(["git", "-C", absSub, "prune", "--expire=now"], {
    stdout: "pipe",
    stderr: "pipe",
  });

  const afterMb = Number((measurePathBytes(gitDir) / (1024 * 1024)).toFixed(2));
  return {
    beforeMb,
    afterMb,
    compacted: afterMb < beforeMb,
    note: afterMb < beforeMb ? `Compacted ${beforeMb} MB -> ${afterMb} MB at ${sha.slice(0, 12)}` : "No size reduction",
  };
}

function listGitTags(absRepo: string): string[] {
  const res = Bun.spawnSync(["git", "-C", absRepo, "tag", "-l"], { stdout: "pipe", stderr: "pipe" });
  if (res.exitCode !== 0) return [];
  return new TextDecoder().decode(res.stdout).split(/\r?\n/).map((s) => s.trim()).filter(Boolean);
}

function listRefs(absRepo: string): string[] {
  const res = Bun.spawnSync(["git", "-C", absRepo, "for-each-ref", "--format=%(refname)"], { stdout: "pipe", stderr: "pipe" });
  if (res.exitCode !== 0) return [];
  return new TextDecoder().decode(res.stdout).split(/\r?\n/).map((s) => s.trim()).filter(Boolean);
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

export function applyDiskSlim(root = repoRoot, compactGit = true): {
  ok: boolean;
  reclaimedMb: number;
  removedTargets: Array<{ relPath: string; mb: number; remainingMb: number }>;
  skippedLocked: string[];
  compactedSubmodules: Array<{ subPath: string; beforeMb: number; afterMb: number; compacted: boolean; note: string }>;
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

  const compactedSubmodules: Array<{ subPath: string; beforeMb: number; afterMb: number; compacted: boolean; note: string }> = [];
  if (compactGit) {
    for (const sub of findBloatedSubmoduleGitDirs(root, 40)) {
      const res = shallowCompactSubmodule(root, sub.subPath);
      compactedSubmodules.push({ subPath: sub.subPath, ...res });
      if (res.compacted) {
        reclaimedMb += Math.max(0, res.beforeMb - res.afterMb);
      }
    }
  }

  return {
    ok: true,
    reclaimedMb: Number(reclaimedMb.toFixed(2)),
    removedTargets,
    skippedLocked,
    compactedSubmodules,
  };
}

if (import.meta.main) {
  const args = Bun.argv.slice(2);
  const cmd = args[0] ?? "audit";
  const compactGit = !args.includes("--keep-git-pack");

  if (cmd === "audit") {
    console.log(JSON.stringify(auditLocalDisk(repoRoot), null, 2));
  } else if (cmd === "apply") {
    console.log(JSON.stringify(applyDiskSlim(repoRoot, compactGit), null, 2));
  } else {
    console.error("Usage: bun tooling/bun/disk-slim.ts [audit|apply] [--keep-git-pack] [--json]");
    process.exit(1);
  }
}
