#!/usr/bin/env bun
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

const START_MARKER = "<!-- === APHRODY_AUTO_SKILLS_SYNC START === -->";
const END_MARKER = "<!-- === APHRODY_AUTO_SKILLS_SYNC END === -->";

interface CrateSpec {
  name: string;
}

interface RepoSpec {
  repo: string;
}

interface SkillTarget {
  path: string;
  crates: CrateSpec[];
  repos: RepoSpec[];
}

const TARGETS: SkillTarget[] = [
  {
    path: ".agents/plugins/aphrody/skills/gpui/SKILL.md",
    crates: [{ name: "gpui" }, { name: "taffy" }, { name: "accesskit" }],
    repos: [{ repo: "zed-industries/zed" }, { repo: "aphrody-dev/aphrody" }],
  },
  {
    path: ".agents/plugins/aphrody/skills/nsh/SKILL.md",
    crates: [{ name: "gpui" }],
    repos: [{ repo: "longbridge/nsh" }, { repo: "aphrody-dev/aphrody" }],
  },
  {
    path: ".agents/skills/gpui-kit/SKILL.md",
    crates: [{ name: "gpui-component" }],
    repos: [{ repo: "longbridge/gpui-component" }],
  },
  {
    path: ".agents/skills/gpui-fast/SKILL.md",
    crates: [{ name: "gpui" }],
    repos: [{ repo: "zed-industries/zed" }],
  },
  {
    path: ".agents/plugins/aphrody/skills/wgpu/SKILL.md",
    crates: [{ name: "wgpu" }, { name: "naga" }],
    repos: [{ repo: "gfx-rs/wgpu" }],
  },
];

async function fetchCrateInfo(name: string): Promise<{ name: string; version: string; updated: string } | null> {
  try {
    const res = await fetch(`https://crates.io/api/v1/crates/${encodeURIComponent(name)}`, {
      headers: { "User-Agent": "aphrody-skills-sync/1.0 (https://github.com/aphrody-dev/aphrody)" },
    });
    if (!res.ok) return null;
    const data = (await res.json()) as {
      crate?: { max_version?: string; updated_at?: string };
    };
    return {
      name,
      version: data.crate?.max_version ?? "unknown",
      updated: (data.crate?.updated_at ?? "").slice(0, 10) || "unknown",
    };
  } catch {
    return null;
  }
}

async function fetchRepoInfo(repo: string): Promise<{ repo: string; ref: string; date: string } | null> {
  const headers: Record<string, string> = {
    "User-Agent": "aphrody-skills-sync/1.0",
    Accept: "application/vnd.github+json",
  };
  const token = process.env.GITHUB_TOKEN || process.env.GH_TOKEN;
  if (token) headers.Authorization = `Bearer ${token}`;

  try {
    const relRes = await fetch(`https://api.github.com/repos/${repo}/releases/latest`, { headers });
    if (relRes.ok) {
      const rel = (await relRes.json()) as { tag_name?: string; published_at?: string };
      return {
        repo,
        ref: rel.tag_name ?? "latest",
        date: (rel.published_at ?? "").slice(0, 10) || "unknown",
      };
    }
    const commitRes = await fetch(`https://api.github.com/repos/${repo}/commits?per_page=1`, { headers });
    if (commitRes.ok) {
      const commits = (await commitRes.json()) as Array<{
        sha?: string;
        commit?: { committer?: { date?: string } };
      }>;
      const top = commits[0];
      return {
        repo,
        ref: top?.sha ? top.sha.slice(0, 8) : "main",
        date: (top?.commit?.committer?.date ?? "").slice(0, 10) || "unknown",
      };
    }
    return null;
  } catch {
    return null;
  }
}

function upsertAutoBlock(content: string, block: string): string {
  const fullBlock = `${START_MARKER}\n${block.trimEnd()}\n${END_MARKER}`;
  const re = new RegExp(`${START_MARKER}[\\s\\S]*?${END_MARKER}`, "m");
  if (re.test(content)) {
    return content.replace(re, fullBlock);
  }
  return `${content.trimEnd()}\n\n${fullBlock}\n`;
}

async function main() {
  const dryRun = process.argv.includes("--dry-run");
  const jsonOut = process.argv.includes("--json");
  const root = resolve(import.meta.dir, "..", "..");
  const today = new Date().toISOString().slice(0, 10);

  const uniqueCrates = [...new Set(TARGETS.flatMap((t) => t.crates.map((c) => c.name)))];
  const uniqueRepos = [...new Set(TARGETS.flatMap((t) => t.repos.map((r) => r.repo)))];

  const [crateResults, repoResults] = await Promise.all([
    Promise.all(uniqueCrates.map((c) => fetchCrateInfo(c))),
    Promise.all(uniqueRepos.map((r) => fetchRepoInfo(r))),
  ]);

  const crateMap = new Map(crateResults.filter(Boolean).map((c) => [c!.name, c!]));
  const repoMap = new Map(repoResults.filter(Boolean).map((r) => [r!.repo, r!]));

  const updated: string[] = [];

  for (const target of TARGETS) {
    const absPath = join(root, target.path);
    if (!existsSync(absPath)) continue;

    const lines: string[] = [
      `## Upstream Snapshot (auto-generated \`${today}\` via \`bun tooling/bun/skills-sync.ts\`)`,
      "",
    ];

    const crateLines = target.crates
      .map((c) => crateMap.get(c.name))
      .filter(Boolean)
      .map((c) => `- **crates.io** \`${c!.name}\`: \`${c!.version}\` (updated ${c!.updated})`);
    const repoLines = target.repos
      .map((r) => repoMap.get(r.repo))
      .filter(Boolean)
      .map((r) => `- **GitHub** \`${r!.repo}\`: \`${r!.ref}\` (${r!.date})`);

    if (crateLines.length > 0) lines.push(...crateLines);
    if (repoLines.length > 0) lines.push(...repoLines);

    const original = readFileSync(absPath, "utf8");
    const next = upsertAutoBlock(original, lines.join("\n"));
    if (next !== original) {
      if (!dryRun) {
        writeFileSync(absPath, next, "utf8");
      }
      updated.push(target.path);
    }
  }

  if (jsonOut) {
    console.log(JSON.stringify({ ok: true, dryRun, updated, crates: Object.fromEntries(crateMap), repos: Object.fromEntries(repoMap) }, null, 2));
  } else {
    console.log(`skills-sync: ${dryRun ? "dry-run" : "updated"} ${updated.length} skill(s): ${updated.join(", ") || "none"}`);
  }
}

await main();
