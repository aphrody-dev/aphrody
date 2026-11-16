#!/usr/bin/env bun
/**
 * Out-of-tree isolated Git worktree orchestrator keeping the root checkout cleanly on `main`.
 * Pairs each worktree under `<repo-parent>/worktrees/<slug>-<id>` with an isolated Cargo target dir
 * `<repo-parent>/target-worktrees/<slug>-<id>` so parallel agents never contend on Cargo locks or index files.
 *
 * Usage:
 *   bun tooling/bun/worktree.ts list
 *   bun tooling/bun/worktree.ts create --id feat-cache [--base main]
 *   bun tooling/bun/worktree.ts prune [--id feat-cache]
 */
import { existsSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "../..");

function opt(flag: string): string | undefined {
  const args = Bun.argv.slice(2);
  const idx = args.indexOf(flag);
  if (idx >= 0) return args[idx + 1];
  const eq = args.find((a) => a.startsWith(`${flag}=`));
  return eq ? eq.slice(flag.length + 1) : undefined;
}

function readSlug(): string {
  const pm = Bun.TOML.parse(readFileSync(join(repoRoot, "product.toml"), "utf8")) as {
    product?: { slug?: string };
  };
  return pm.product?.slug ?? "aphrody";
}

export function worktreePaths(id: string, root = repoRoot, slug = readSlug()) {
  const cleanId = id.toLowerCase().replace(/[^a-z0-9-]+/g, "-").replace(/^-+|-+$/g, "");
  if (!cleanId) throw new Error(`Invalid worktree id '${id}'`);
  const parent = dirname(root);
  const worktreeDir = resolve(parent, "worktrees", `${slug}-${cleanId}`);
  const targetDir = resolve(parent, "target-worktrees", `${slug}-${cleanId}`);
  const branch = `wt/${cleanId}`;
  return { cleanId, branch, worktreeDir, targetDir };
}

function runGit(args: string[]): { exitCode: number; stdout: string; stderr: string } {
  const res = Bun.spawnSync(["git", "-C", repoRoot, ...args], { stdout: "pipe", stderr: "pipe" });
  return {
    exitCode: res.exitCode,
    stdout: new TextDecoder().decode(res.stdout),
    stderr: new TextDecoder().decode(res.stderr),
  };
}

if (import.meta.main) {
  const cmd = Bun.argv[2] ?? "list";
  if (cmd === "list") {
    const res = runGit(["worktree", "list", "--porcelain"]);
    if (res.exitCode !== 0) throw new Error(res.stderr);
    console.log(res.stdout.trim());
  } else if (cmd === "create") {
    const id = opt("--id");
    if (!id) throw new Error("Missing --id <name>");
    const base = opt("--base") ?? "main";
    const paths = worktreePaths(id);
    mkdirSync(dirname(paths.worktreeDir), { recursive: true });
    mkdirSync(paths.targetDir, { recursive: true });
    if (!existsSync(paths.worktreeDir)) {
      const add = runGit(["worktree", "add", "-B", paths.branch, paths.worktreeDir, base]);
      if (add.exitCode !== 0) throw new Error(`git worktree add failed: ${add.stderr}`);
    }
    console.log(JSON.stringify({ ok: true, ...paths, cargoEnv: { CARGO_TARGET_DIR: paths.targetDir } }, null, 2));
  } else if (cmd === "prune") {
    const id = opt("--id");
    if (id) {
      const paths = worktreePaths(id);
      if (existsSync(paths.worktreeDir)) {
        const rm = runGit(["worktree", "remove", "--force", paths.worktreeDir]);
        if (rm.exitCode !== 0) throw new Error(`git worktree remove failed: ${rm.stderr}`);
      }
      runGit(["branch", "-D", paths.branch]);
      rmSync(paths.targetDir, { recursive: true, force: true });
    }
    const pr = runGit(["worktree", "prune"]);
    if (pr.exitCode !== 0) throw new Error(pr.stderr);
    console.log(JSON.stringify({ ok: true, pruned: id ?? "all-stale" }, null, 2));
  } else {
    console.error("Usage: bun tooling/bun/worktree.ts [list|create --id <name>|prune [--id <name>]]");
    process.exit(1);
  }
}
