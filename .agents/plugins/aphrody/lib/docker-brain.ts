import { existsSync, readdirSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";

export interface DockerBrainSearchHit {
  file: string;
  section: string;
  lineStart: number;
  lineEnd: number;
  score: number;
  excerpt: string;
}

export interface DockerSkillMeta {
  id: string;
  description: string;
  path: string;
}

export interface DockerAuditFinding {
  severity: "error" | "warn" | "info";
  rule: string;
  line?: number;
  message: string;
}

export function resolveDockerBrainDir(explicit?: string): string {
  if (explicit && existsSync(explicit)) return resolve(explicit);
  if (process.env.DOCKER_BRAIN_DIR && existsSync(process.env.DOCKER_BRAIN_DIR)) {
    return resolve(process.env.DOCKER_BRAIN_DIR);
  }
  let cur = resolve(import.meta.dir);
  for (let i = 0; i < 6; i++) {
    const candidate = join(cur, "brain", "docker");
    if (existsSync(join(candidate, "INDEX.md"))) return candidate;
    const parent = dirname(cur);
    if (parent === cur) break;
    cur = parent;
  }
  const home = process.env.USERPROFILE ?? process.env.HOME ?? homedir();
  return join(home, ".gemini", "brain", "docker");
}

function tokenize(query: string): string[] {
  return (query.toLowerCase().match(/[a-z0-9_.:/-]+/g) ?? []).filter((t) => t.length >= 2);
}

export function searchDockerBrain(
  query: string,
  options?: { file?: string; limit?: number; brainDir?: string },
): DockerBrainSearchHit[] {
  const dir = resolveDockerBrainDir(options?.brainDir);
  if (!existsSync(dir)) return [];
  const tokens = tokenize(query);
  const limit = Math.min(Math.max(1, options?.limit ?? 8), 30);
  const fileFilter = options?.file?.trim().toLowerCase();

  const hits: DockerBrainSearchHit[] = [];
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
          excerpt: text.slice(0, 700),
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

  return hits.sort((a, b) => b.score - a.score).slice(0, limit);
}

export function listInstalledDockerSkills(skillsDir?: string): DockerSkillMeta[] {
  const home = process.env.USERPROFILE ?? process.env.HOME ?? homedir();
  const root =
    skillsDir && existsSync(skillsDir)
      ? resolve(skillsDir)
      : existsSync(join(import.meta.dir, "..", "skills"))
        ? resolve(join(import.meta.dir, "..", "skills"))
        : join(home, ".gemini", "plugins", "aphrody", "skills");
  if (!existsSync(root)) return [];
  const out: DockerSkillMeta[] = [];
  for (const entry of readdirSync(root, { withFileTypes: true }).sort((a, b) =>
    a.name.localeCompare(b.name),
  )) {
    if (!entry.isDirectory() || !entry.name.startsWith("docker-")) continue;
    const skillMd = join(root, entry.name, "SKILL.md");
    if (!existsSync(skillMd)) continue;
    const raw = readFileSync(skillMd, "utf8");
    const desc =
      raw.match(/^description:\s*["']?(.+?)["']?\s*$/m)?.[1]?.trim() ?? "";
    out.push({ id: entry.name, description: desc, path: skillMd.replaceAll("\\", "/") });
  }
  return out;
}

export function auditDockerfileContent(
  content: string,
  hasDockerignore = true,
): DockerAuditFinding[] {
  const findings: DockerAuditFinding[] = [];
  const lines = content.replace(/\r\n/g, "\n").split("\n");

  if (!lines[0]?.trim().startsWith("# syntax=docker/dockerfile:")) {
    findings.push({
      severity: "warn",
      rule: "buildkit-syntax-directive",
      line: 1,
      message: "Pin `# syntax=docker/dockerfile:1.12` (or `1.7+`) for BuildKit mounts and cache features.",
    });
  }

  if (!hasDockerignore) {
    findings.push({
      severity: "warn",
      rule: "missing-dockerignore",
      message: "Add a .dockerignore excluding .git, node_modules, target, build caches, and .env secrets.",
    });
  }

  let hasUser = false;
  let hasSecretExposure = false;
  let usesMountCache = false;
  for (let i = 0; i < lines.length; i++) {
    const raw = lines[i]!;
    const trimmed = raw.trim();
    const ln = i + 1;
    if (!trimmed || trimmed.startsWith("#")) continue;

    if (/^USER\s+/i.test(trimmed) && !/^USER\s+(root|0(:0)?)\b/i.test(trimmed)) {
      hasUser = true;
    }
    if (/--mount=type=cache/i.test(trimmed)) usesMountCache = true;
    if (/^FROM\s+[^:\s]+(?::latest)?(?:\s+AS\s+|$)/i.test(trimmed) && !trimmed.includes("@sha256:") && !trimmed.includes("$")) {
      findings.push({
        severity: "warn",
        rule: "pin-base-tag",
        line: ln,
        message: `Avoid unpinned ':latest' or untagged base image in: ${trimmed}`,
      });
    }
    if (/^(?:ENV|ARG)\s+[A-Z0-9_]*(?:SECRET|TOKEN|PASSWORD|API_KEY|PRIVATE_KEY)[A-Z0-9_]*\s*=\s*\S+/i.test(trimmed)) {
      hasSecretExposure = true;
      findings.push({
        severity: "error",
        rule: "secret-in-layer",
        line: ln,
        message: "Avoid baking literal secrets in ENV/ARG; use `RUN --mount=type=secret`.",
      });
    }
    if (/^COPY\s+.*\.(env|pem|p12|key)\b/i.test(trimmed)) {
      findings.push({
        severity: "error",
        rule: "copy-credential-file",
        line: ln,
        message: `Never COPY credential files into layers (${trimmed}).`,
      });
    }
  }

  if (!hasUser) {
    findings.push({
      severity: "warn",
      rule: "non-root-user",
      message: "Set an explicit non-root `USER` (e.g. `USER 1000:1000` or `USER bun`) for the runtime stage.",
    });
  }
  if (!usesMountCache && content.length > 150) {
    findings.push({
      severity: "info",
      rule: "buildkit-cache-mount",
      message: "Consider `RUN --mount=type=cache,...` for package manager / compiler caches.",
    });
  }
  if (!/HEALTHCHECK\s+/i.test(content)) {
    findings.push({
      severity: "info",
      rule: "healthcheck-recommended",
      message: "Define `HEALTHCHECK` in the Dockerfile or in `compose.yaml`.",
    });
  }
  if (!hasSecretExposure && findings.length === 0) {
    findings.push({
      severity: "info",
      rule: "compliant",
      message: "Dockerfile follows core BuildKit and hardening best practices.",
    });
  }
  return findings;
}

export function auditComposeContent(content: string): DockerAuditFinding[] {
  const findings: DockerAuditFinding[] = [];
  const norm = content.replace(/\r\n/g, "\n");
  if (!/security_opt:\s*.*no-new-privileges/s.test(norm)) {
    findings.push({
      severity: "warn",
      rule: "no-new-privileges",
      message: 'Enforce `security_opt: ["no-new-privileges:true"]` across runtime services.',
    });
  }
  if (/image:\s*[^\s#:]+:latest\b/i.test(norm)) {
    findings.push({
      severity: "warn",
      rule: "compose-pinned-tags",
      message: "Avoid `:latest` tags on Compose images; pin explicit version tags.",
    });
  }
  if (!/healthcheck:/i.test(norm)) {
    findings.push({
      severity: "warn",
      rule: "compose-healthcheck",
      message: "Add `healthcheck:` definitions and use `condition: service_healthy` on dependents.",
    });
  }
  if (/ports:\s*[\s\S]*-\s*["']?(?:\d+:\d+)/.test(norm) && !/127\.0\.0\.1|\$\{[A-Z0-9_]*IP/.test(norm)) {
    findings.push({
      severity: "info",
      rule: "bind-loopback-or-mesh",
      message: "Verify published ports intentionally bind 0.0.0.0 rather than 127.0.0.1 / private mesh.",
    });
  }
  if (!/resources:\s*[\s\S]*limits:/m.test(norm)) {
    findings.push({
      severity: "info",
      rule: "resource-limits",
      message: "Consider explicit `deploy.resources.limits.memory` caps for multi-service stability.",
    });
  }
  return findings;
}
