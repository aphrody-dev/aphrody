#!/usr/bin/env bun
/**
 * Repository reproducibility, link integrity, and anti-ephemeral hygiene verifier.
 * Runs cleanly in CI and locally to ensure zero references to `.gemini/.../brain`, `%TEMP%`,
 * or broken relative links leak into tracked code, docs, or plugin configs.
 *
 * Usage:
 *   bun tooling/bun/repo-hygiene.ts
 */
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";

const repoRoot = resolve(import.meta.dir, "../..");

export interface HygieneReport {
  ok: boolean;
  readmeGuardOk: boolean;
  checkedFiles: number;
  ephemeralLeaks: string[];
  brokenArchLinks: string[];
  invalidManifests: string[];
}

const EPHEMERAL_PATTERNS = [
  /\.gemini\/antigravity-cli\/brain/i,
  /file:\/\/\/[A-Za-z]:\/Users\/[^/]+\/\.gemini/i,
  /AppData\/Local\/Temp/i,
];

const GUARD_ALLOWLIST = new Set([
  ".agents/plugins/aphrody/hooks/post-edit-check.ts",
  ".agents/plugins/aphrody/hooks/session-start.ts",
  ".agents/plugins/aphrody/rules/AGENTS.md",
  "tooling/bun/repo-hygiene.ts",
  "tooling/bun/tooling.test.ts",
]);

/**
 * Submodule checkouts are optional (CI jobs that do not build vendored crates skip them), so a link into a gitlink
 * whose working tree is not initialised cannot be verified on disk; the pinned gitlink is the contract.
 */
export function isInsideUninitialisedSubmodule(absolutePath: string, root = repoRoot): boolean {
  const relativePath = relative(root, absolutePath).split(sep).join("/");
  if (relativePath.startsWith("vendor/") && existsSync(join(root, "tooling/bun/forks-manifest.json"))) {
    return true;
  }
  const staged = Bun.spawnSync(["git", "-C", root, "ls-files", "--stage"], { stdout: "pipe", stderr: "pipe" });
  return new TextDecoder()
    .decode(staged.stdout)
    .split(/\r?\n/)
    .filter((line) => line.startsWith("160000 "))
    .map((line) => line.split("\t")[1])
    .some(
      (path) =>
        path !== undefined &&
        (relativePath === path || relativePath.startsWith(`${path}/`)) &&
        !existsSync(join(root, path, ".git")),
    );
}

export function checkRepoHygiene(root = repoRoot): HygieneReport {
  const readmePath = join(root, "README.md");
  const readmeLines = existsSync(readmePath) ? readFileSync(readmePath, "utf8").split(/\r?\n/) : [];
  const readmeGuardOk =
    readmeLines[0]?.trim() === "> [!IMPORTANT]" &&
    (readmeLines[1]?.includes("Remove this line to confirm you've reviewed this PR") ?? false);

  const managedScopes = [
    "aphrody/docs/architecture",
    ".agents/plugins/aphrody",
    "tooling/bun",
    ".github",
    ".cloudflare",
    "crates/aphrody_desktop",
    "crates/aphrody_ui",
  ];
  const tracked = Bun.spawnSync(["git", "-C", root, "ls-files", ...managedScopes], {
    stdout: "pipe",
    stderr: "pipe",
  });
  const files = new TextDecoder()
    .decode(tracked.stdout)
    .split(/\r?\n/)
    .map((s) => s.trim())
    .filter(Boolean);

  // Include untracked candidate files under managed folders so new additions are validated pre-commit.
  const untracked = Bun.spawnSync(
    ["git", "-C", root, "ls-files", "--others", "--exclude-standard", "aphrody/docs/architecture", ".agents/plugins/aphrody", "tooling/bun", ".github", ".cloudflare"],
    { stdout: "pipe", stderr: "pipe" },
  );
  for (const extra of new TextDecoder().decode(untracked.stdout).split(/\r?\n/).map((s) => s.trim()).filter(Boolean)) {
    if (!files.includes(extra)) files.push(extra);
  }

  const ephemeralLeaks: string[] = [];
  const brokenArchLinks: string[] = [];
  const invalidManifests: string[] = [];

  for (const rel of files) {
    if (GUARD_ALLOWLIST.has(rel)) continue;
    const full = join(root, rel);
    if (!existsSync(full)) continue;
    const content = readFileSync(full, "utf8");
    for (const pattern of EPHEMERAL_PATTERNS) {
      if (pattern.test(content)) {
        ephemeralLeaks.push(`${rel} matched ${pattern.source}`);
      }
    }

    if (rel.startsWith("aphrody/docs/architecture/") && rel.endsWith(".md")) {
      for (const match of content.matchAll(/\]\(([^)]+)\)/g)) {
        const rawTarget = match[1].split("#")[0].trim();
        if (!rawTarget || rawTarget.startsWith("http://") || rawTarget.startsWith("https://") || rawTarget.startsWith("mailto:")) {
          continue;
        }
        const resolved = resolve(dirname(full), rawTarget);
        if (!existsSync(resolved) && !isInsideUninitialisedSubmodule(resolved, root)) {
          brokenArchLinks.push(`${rel} -> ${rawTarget}`);
        }
      }
    }
  }

  for (const jsonRel of [".agents/plugins.json", ".agents/plugins/aphrody/plugin.json", ".agents/plugins/aphrody/hooks.json", "package.json", ".github/renovate.json"]) {
    const p = join(root, jsonRel);
    if (!existsSync(p)) continue;
    try {
      JSON.parse(readFileSync(p, "utf8"));
    } catch (err) {
      invalidManifests.push(`${jsonRel}: ${String(err)}`);
    }
  }

  for (const rel of files) {
    if (rel.startsWith(".github/workflows/") && (rel.endsWith(".yml") || rel.endsWith(".yaml"))) {
      const full = join(root, rel);
      if (existsSync(full)) {
        try {
          Bun.YAML.parse(readFileSync(full, "utf8"));
        } catch (err) {
          invalidManifests.push(`${rel}: ${String(err)}`);
        }
      }
    }
  }

  return {
    ok: readmeGuardOk && ephemeralLeaks.length === 0 && brokenArchLinks.length === 0 && invalidManifests.length === 0,
    readmeGuardOk,
    checkedFiles: files.length,
    ephemeralLeaks,
    brokenArchLinks,
    invalidManifests,
  };
}

if (import.meta.main) {
  const report = checkRepoHygiene();
  console.log(JSON.stringify(report, null, 2));
  if (!report.ok) process.exit(1);
}
