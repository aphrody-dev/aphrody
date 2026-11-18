import { existsSync, readdirSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";

export interface VercelBrainSearchHit {
  file: string;
  section: string;
  lineStart: number;
  lineEnd: number;
  score: number;
  excerpt: string;
}

export interface VercelSkillMeta {
  id: string;
  description: string;
  path: string;
}

export interface VercelAuditFinding {
  severity: "error" | "warn" | "info";
  rule: string;
  file: string;
  message: string;
}

const VALID_VERCEL_JSON_KEYS = new Set([
  "$schema",
  "buildCommand",
  "builds",
  "bunVersion",
  "cleanUrls",
  "crons",
  "daemons",
  "devCommand",
  "env",
  "fluid",
  "framework",
  "functions",
  "git",
  "github",
  "headers",
  "ignoreCommand",
  "images",
  "installCommand",
  "outputDirectory",
  "public",
  "redirects",
  "regions",
  "rewrites",
  "routes",
  "trailingSlash",
  "version",
]);

export function resolveVercelBrainDir(explicit?: string): string {
  if (explicit && existsSync(explicit)) return resolve(explicit);
  if (process.env.VERCEL_BRAIN_DIR && existsSync(process.env.VERCEL_BRAIN_DIR)) {
    return resolve(process.env.VERCEL_BRAIN_DIR);
  }
  let cur = resolve(import.meta.dir);
  for (let i = 0; i < 6; i++) {
    const candidate = join(cur, "brain", "vercel");
    if (existsSync(join(candidate, "INDEX.md"))) return candidate;
    const parent = dirname(cur);
    if (parent === cur) break;
    cur = parent;
  }
  const home = process.env.USERPROFILE ?? process.env.HOME ?? homedir();
  return join(home, ".gemini", "brain", "vercel");
}

function tokenize(query: string): string[] {
  return (query.toLowerCase().match(/[a-z0-9_.:/-]+/g) ?? []).filter((t) => t.length >= 2);
}

export function searchVercelBrain(
  query: string,
  options?: { file?: string; limit?: number; brainDir?: string },
): VercelBrainSearchHit[] {
  const dir = resolveVercelBrainDir(options?.brainDir);
  if (!existsSync(dir)) return [];
  const tokens = tokenize(query);
  const limit = Math.min(Math.max(1, options?.limit ?? 8), 30);
  const fileFilter = options?.file?.trim().toLowerCase();

  const hits: VercelBrainSearchHit[] = [];
  for (const name of readdirSync(dir)) {
    if (!name.endsWith(".md")) continue;
    if (fileFilter && !name.toLowerCase().includes(fileFilter)) continue;
    const raw = readFileSync(join(dir, name), "utf8").replace(/\r\n/g, "\n");
    const lines = raw.split("\n");
    let section = basename(name, ".md");
    let start = 1;
    let buf: string[] = [];

    const scoreAndPush = (endLine: number) => {
      const text = buf.join("\n").trim();
      buf = [];
      if (!text) return;
      const hay = `${name}\n${section}\n${text}`.toLowerCase();
      let score = 0;
      for (const tok of tokens) {
        if (section.toLowerCase().includes(tok)) score += 3;
        if (hay.includes(tok)) score += 1;
      }
      if (score > 0 || tokens.length === 0) {
        hits.push({
          file: name,
          section,
          lineStart: start,
          lineEnd: endLine,
          score,
          excerpt: text.slice(0, 750),
        });
      }
    };

    for (let i = 0; i < lines.length; i++) {
      const hm = lines[i]!.match(/^#{1,3}\s+(.+)$/);
      if (hm) {
        scoreAndPush(i);
        section = hm[1]!.replace(/`/g, "").trim();
        start = i + 1;
      } else {
        buf.push(lines[i]!);
      }
    }
    scoreAndPush(lines.length);
  }

  return hits.sort((a, b) => b.score - a.score || a.file.localeCompare(b.file)).slice(0, limit);
}

export function lookupVercelGraph(topic: string, brainDir?: string): string[] {
  const dir = resolveVercelBrainDir(brainDir);
  const kgPath = join(dir, "KNOWLEDGE_GRAPH.md");
  if (!existsSync(kgPath)) return [];
  const raw = readFileSync(kgPath, "utf8").replace(/\r\n/g, "\n");
  const tokens = tokenize(topic);
  const lines = raw.split("\n");
  const matches: string[] = [];
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]!;
    const low = line.toLowerCase();
    if (tokens.length > 0 && tokens.every((t) => low.includes(t))) {
      const start = Math.max(0, i - 1);
      const end = Math.min(lines.length, i + 4);
      matches.push(lines.slice(start, end).join("\n").trim());
      if (matches.length >= 10) break;
    }
  }
  return matches;
}

export function listInstalledVercelSkills(skillsDir?: string): VercelSkillMeta[] {
  const base = skillsDir ? resolve(skillsDir) : resolve(import.meta.dir, "..", "skills");
  if (!existsSync(base)) return [];
  const out: VercelSkillMeta[] = [];
  for (const e of readdirSync(base, { withFileTypes: true }).sort((a, b) =>
    a.name.localeCompare(b.name),
  )) {
    if (!e.isDirectory()) continue;
    const skillMd = join(base, e.name, "SKILL.md");
    if (!existsSync(skillMd)) continue;
    const raw = readFileSync(skillMd, "utf8");
    const dm = raw.match(/^description:\s*["']?(.+?)["']?\s*$/m);
    out.push({ id: e.name, description: dm?.[1]?.trim() ?? "", path: skillMd });
  }
  return out;
}

export function auditVercelProjectConfig(projectDir: string): VercelAuditFinding[] {
  const findings: VercelAuditFinding[] = [];
  const dir = resolve(projectDir);

  for (const cfgName of ["vercel.json", "vercel.ts"]) {
    const p = join(dir, cfgName);
    if (!existsSync(p)) continue;
    const raw = readFileSync(p, "utf8");
    if (cfgName === "vercel.json") {
      try {
        const parsed = JSON.parse(raw) as Record<string, unknown>;
        for (const k of Object.keys(parsed)) {
          if (!VALID_VERCEL_JSON_KEYS.has(k)) {
            findings.push({
              severity: "error",
              rule: "vercel-json-top-level-key",
              file: cfgName,
              message: `Unknown top-level key '${k}' in vercel.json (rootDirectory belongs only in Project Settings).`,
            });
          }
        }
        if (!parsed.bunVersion && existsSync(join(dir, "bun.lock"))) {
          findings.push({
            severity: "info",
            rule: "bun-version-pin",
            file: cfgName,
            message: "Consider setting bunVersion: \"1.4.x\" to prevent bun 1.3 fallback during remote builds.",
          });
        }
      } catch {
        findings.push({
          severity: "error",
          rule: "vercel-json-parse",
          file: cfgName,
          message: "Failed to parse vercel.json as valid JSON.",
        });
      }
    } else if (/rootDirectory\s*:/m.test(raw)) {
      findings.push({
        severity: "error",
        rule: "vercel-ts-root-directory",
        file: cfgName,
        message: "Do not specify rootDirectory in vercel.ts; set Root Directory in Vercel Project Settings.",
      });
    }
    if (/runtime\s*:\s*["']edge["']/i.test(raw)) {
      findings.push({
        severity: "warn",
        rule: "prefer-fluid-node-over-edge",
        file: cfgName,
        message: "Edge runtime is legacy for most handlers; prefer Fluid Compute with Node.js 24.x or Bun.",
      });
    }
  }

  for (const nextCfg of ["next.config.ts", "next.config.mjs", "next.config.js"]) {
    const p = join(dir, nextCfg);
    if (!existsSync(p)) continue;
    const raw = readFileSync(p, "utf8");
    if (!raw.includes("outputFileTracingExcludes")) {
      findings.push({
        severity: "info",
        rule: "tracing-excludes",
        file: nextCfg,
        message: "Large monorepo Next apps benefit from outputFileTracingExcludes to keep lambda size under 250 MB.",
      });
    }
  }

  return findings;
}
