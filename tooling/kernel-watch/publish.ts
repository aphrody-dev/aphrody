// Decides whether regenerated files are worth a commit: version changes at once, activity-only churn at most every few hours.
import { resolve } from "node:path";

export interface GateInput {
  versionsChanged: boolean;
  anyChanged: boolean;
  /** Epoch seconds of the last commit touching the output directory, null when there is none. */
  lastCommit: number | null;
  now: number;
  slowSeconds: number;
}

export function decideCommit(input: GateInput): "commit" | "skip" {
  if (!input.anyChanged) return "skip";
  if (input.versionsChanged || input.lastCommit === null) return "commit";
  return input.now - input.lastCommit >= input.slowSeconds ? "commit" : "skip";
}

const git = (cwd: string, ...args: string[]): string => {
  const run = Bun.spawnSync(["git", "-C", cwd, ...args], { stdout: "pipe", stderr: "pipe" });
  if (run.exitCode !== 0) throw new Error(`git ${args.join(" ")}: ${run.stderr.toString().trim()}`);
  return run.stdout.toString();
};

/** Reads the working tree of the output directory and applies decideCommit. */
export function gate(outDir: string, slowSeconds: number, now = Math.floor(Date.now() / 1000)): "commit" | "skip" {
  const dir = resolve(outDir);
  const changed = git(dir, "status", "--porcelain", "--", ".").split("\n").filter(Boolean).map((line) => line.slice(3));
  const last = git(dir, "log", "-1", "--format=%ct", "--", ".").trim();
  return decideCommit({
    versionsChanged: changed.some((file) => file.endsWith("data/versions.json")),
    anyChanged: changed.length > 0,
    lastCommit: last ? Number(last) : null,
    now,
    slowSeconds,
  });
}
