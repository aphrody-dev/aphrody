#!/usr/bin/env bun
/**
 * Reproducible, parallel submodule manager driven strictly by `.gitmodules` and root Git index.
 * Usage:
 *   bun tooling/bun/submodules.ts status
 *   bun tooling/bun/submodules.ts check
 *   bun tooling/bun/submodules.ts sync [--jobs 8] [--all]
 */
import { existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

export interface SubmoduleEntry {
  name: string;
  path: string;
  url: string;
  branch?: string;
}

const repoRoot = resolve(import.meta.dir, "../..");

export function parseGitmodules(content: string): SubmoduleEntry[] {
  const entries: SubmoduleEntry[] = [];
  let current: Partial<SubmoduleEntry> = {};

  for (const rawLine of content.split(/\r?\n/)) {
    const line = rawLine.trim();
    const header = /^\[submodule "([^"]+)"\]$/.exec(line);
    if (header) {
      if (current.name && current.path && current.url) {
        entries.push(current as SubmoduleEntry);
      }
      current = { name: header[1] };
      continue;
    }
    if (line.startsWith("path = ")) current.path = line.slice(7).trim();
    else if (line.startsWith("url = ")) current.url = line.slice(6).trim();
    else if (line.startsWith("branch = ")) current.branch = line.slice(9).trim();
  }
  if (current.name && current.path && current.url) {
    entries.push(current as SubmoduleEntry);
  }
  return entries;
}

function runGit(args: string[], inherit = false): { exitCode: number; stdout: string; stderr: string } {
  const res = Bun.spawnSync(["git", "-C", repoRoot, ...args], {
    stdout: inherit ? "inherit" : "pipe",
    stderr: inherit ? "inherit" : "pipe",
  });
  return {
    exitCode: res.exitCode,
    stdout: res.stdout ? new TextDecoder().decode(res.stdout) : "",
    stderr: res.stderr ? new TextDecoder().decode(res.stderr) : "",
  };
}

export function listReferencedPaths(): string[] {
  const raw = readFileSync(join(repoRoot, "Cargo.toml"), "utf8");
  const all = parseGitmodules(readFileSync(join(repoRoot, ".gitmodules"), "utf8"));
  return all
    .map((entry) => entry.path)
    .filter((p) => raw.includes(`"${p}"`) || raw.includes(`"${p}/`));
}

if (import.meta.main) {
  const args = Bun.argv.slice(2);
  const cmd = args[0] ?? "status";
  const jobsIdx = args.indexOf("--jobs");
  const jobs = jobsIdx >= 0 ? args[jobsIdx + 1] : "8";
  const syncAll = args.includes("--all");
  const gitmodulesPath = join(repoRoot, ".gitmodules");
  const entries = parseGitmodules(readFileSync(gitmodulesPath, "utf8"));

  if (cmd === "status" || cmd === "check") {
    const subStatus = runGit(["submodule", "status"]);
    if (subStatus.exitCode !== 0) {
      throw new Error(`git submodule status failed: ${subStatus.stderr}`);
    }
    const dirty: string[] = [];
    const uninitialized: string[] = [];
    for (const line of subStatus.stdout.split(/\r?\n/)) {
      if (!line.trim()) continue;
      const prefix = line[0];
      const match = /^[ +-U]?([0-9a-f]+)\s+(\S+)/.exec(line);
      if (!match) continue;
      const subPath = match[2];
      if (prefix === "-") {
        uninitialized.push(subPath);
        continue;
      }
      if (prefix === "+") dirty.push(subPath);
      if (existsSync(join(repoRoot, subPath))) {
        const inner = Bun.spawnSync(["git", "-C", join(repoRoot, subPath), "status", "--porcelain"], {
          stdout: "pipe",
          stderr: "pipe",
        });
        if (inner.exitCode === 0 && new TextDecoder().decode(inner.stdout).trim().length > 0) {
          if (!dirty.includes(subPath)) dirty.push(subPath);
        }
      }
    }
    const referenced = listReferencedPaths();
    console.log(
      JSON.stringify(
        {
          totalDeclared: entries.length,
          cargoReferencedCount: referenced.length,
          cargoReferenced: referenced,
          uninitialized,
          dirty,
          ok: (cmd === "status" || dirty.length === 0) && uninitialized.length === 0,
        },
        null,
        2,
      ),
    );
    if (cmd === "check" && (dirty.length > 0 || uninitialized.length > 0)) {
      process.exit(1);
    }
  } else if (cmd === "sync") {
    runGit(["submodule", "sync", "--recursive"], true);
    const targets = syncAll ? entries.map((e) => e.path) : listReferencedPaths();
    const res = runGit(["submodule", "update", "--init", "--depth", "1", "--jobs", jobs, "--", ...targets], true);
    if (res.exitCode !== 0) process.exit(res.exitCode);
  } else {
    console.error("Usage: bun tooling/bun/submodules.ts [status|check|sync] [--jobs 8] [--all]");
    process.exit(1);
  }
}
