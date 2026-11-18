#!/usr/bin/env bun
/**
 * Autonomous cross-PC, cross-agent serverless Learn & Convergence CLI for Aphrody Labs.
 * Synchronizes Antigravity (~/.gemini), Claude Code (~/.claude), Codex (~/.codex),
 * OpenCode, Pi, and repo .agents/ onto the same Cloudflare Worker canonical bundle
 * (brain, memory, knowledge, skills, lean MCP profile) with zero human approval.
 *
 * Usage:
 *   bun tooling/bun/learn-cloud.ts status [--json]
 *   bun tooling/bun/learn-cloud.ts push [--dry-run] [--json]
 *   bun tooling/bun/learn-cloud.ts pull [--force] [--dry-run] [--json]
 *   bun tooling/bun/learn-cloud.ts publish-local [--dry-run] [--json]
 *   bun tooling/bun/learn-cloud.ts cycle [--json]
 */
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { homedir, hostname, platform } from "node:os";
import { basename, join, resolve } from "node:path";

export const DEFAULT_LEARN_URLS = ["https://learn.aphrody.com", "https://releases.aphrody.com"];

export async function fetchWithLearnFallback(
  path: string,
  init: RequestInit,
  explicitUrl = process.env.APHRODY_LEARN_URL?.trim(),
): Promise<{ response: Response; baseUrl: string }> {
  const candidates = explicitUrl ? [explicitUrl.replace(/\/$/, "")] : DEFAULT_LEARN_URLS;
  let lastErr: unknown;
  for (const base of candidates) {
    try {
      const response = await fetch(`${base}${path}`, init);
      return { response, baseUrl: base };
    } catch (err) {
      lastErr = err;
    }
  }
  throw lastErr instanceof Error ? lastErr : new Error(String(lastErr ?? "unreachable learn worker"));
}

export interface JournalEntry {
  at: string;
  harness: string;
  agent?: string;
  pcId?: string;
  session?: string;
  cwd?: string;
  kind: "error" | "recovery" | "correction";
  tool?: string;
  summary: string;
}

export interface RemoteBundle {
  schemaVersion: number;
  revision: number;
  sha256: string;
  updatedAt: string;
  sources: { harnesses: string[]; pcs: string[] };
  rules: { globalDirectives: Record<string, string> };
  memory: {
    topics: Record<
      string,
      {
        slug: string;
        title: string;
        summary: string;
        bullets: string[];
        updatedAt: string;
        sources: string[];
        confidence: number;
      }
    >;
  };
  brain: { docs: Record<string, { path: string; sha256: string; content: string; updatedAt: string }> };
  skills: { deltas: Record<string, { skill: string; notes: string[]; updatedAt: string }> };
  mcp: { profile: { keepBaseline: string[]; disableRecommended: string[]; updatedAt: string } };
}

export interface ClientPaths {
  home: string;
  repoRoot: string;
  geminiDir: string;
  claudeDir: string;
  codexDir: string;
  opencodeDir: string;
  piDir: string;
  stateDir: string;
  tokenFile: string;
}

const SECRET_RE =
  /(sk-[A-Za-z0-9_-]{16,}|ghp_\w{20,}|gho_\w{20,}|github_pat_\w{20,}|AIza[\w-]{30,}|xox[bp]-[\w-]+|eyJ[\w-]{10,}\.[\w-]{10,}\.[\w-]{10,}|Bearer\s+\S{16,}|-----BEGIN [A-Z ]*PRIVATE KEY-----|(?:api[_-]?key|token|secret|passw(?:or)?d)["']?\s*[=:]\s*["']?[^\s"',;]{6,})/gi;

export function resolveClientPaths(homeDir = homedir(), repoRoot = resolve(import.meta.dir, "../..")): ClientPaths {
  const home = homeDir.replaceAll("\\", "/");
  return {
    home,
    repoRoot: repoRoot.replaceAll("\\", "/"),
    geminiDir: `${home}/.gemini`,
    claudeDir: `${home}/.claude`,
    codexDir: `${home}/.codex`,
    opencodeDir: `${home}/.config/opencode`,
    piDir: `${home}/.pi/agent`,
    stateDir: `${home}/.config/aphrody`,
    tokenFile: `${home}/.config/aphrody/learn.token`,
  };
}

export function resolveLearnUrl(): string {
  return (process.env.APHRODY_LEARN_URL?.trim() || DEFAULT_LEARN_URLS[0]!).replace(/\/$/, "");
}

export function resolveLearnToken(paths = resolveClientPaths()): string | undefined {
  const envTok = process.env.APHRODY_LEARN_TOKEN?.trim() || process.env.APHRODY_FLEET_TOKEN?.trim();
  if (envTok) return envTok;
  if (existsSync(paths.tokenFile)) {
    const raw = readFileSync(paths.tokenFile, "utf8").trim();
    if (raw) return raw;
  }
  return undefined;
}

export function resolvePcId(): string {
  const h = hostname().replace(/[^A-Za-z0-9_.-]/g, "_").slice(0, 32);
  return `${platform()}-${h}`;
}

export function redactText(text: string): string {
  return text.replace(SECRET_RE, "[REDACTED]");
}

interface SyncState {
  lastRevision: number;
  lastSha256: string;
  journalCursors: Record<string, number>;
  lastSyncAt: string;
}

function loadSyncState(paths: ClientPaths): SyncState {
  const p = `${paths.stateDir}/learn-sync-state.json`;
  try {
    return JSON.parse(readFileSync(p, "utf8")) as SyncState;
  } catch {
    return { lastRevision: -1, lastSha256: "", journalCursors: {}, lastSyncAt: "" };
  }
}

function saveSyncState(paths: ClientPaths, state: SyncState): void {
  mkdirSync(paths.stateDir, { recursive: true });
  writeFileSync(`${paths.stateDir}/learn-sync-state.json`, `${JSON.stringify(state, null, 2)}\n`, "utf8");
}

function writeIfChanged(path: string, content: string): boolean {
  if (existsSync(path) && readFileSync(path, "utf8") === content) return false;
  mkdirSync(resolve(path, ".."), { recursive: true });
  writeFileSync(path, content, "utf8");
  return true;
}

export function collectJournalDeltas(
  paths: ClientPaths,
  cursors: Record<string, number>,
): { entries: JournalEntry[]; nextCursors: Record<string, number> } {
  const sources: Array<{ harness: string; file: string }> = [
    { harness: "agy", file: `${paths.geminiDir}/learning/journal.jsonl` },
    { harness: "claude", file: `${paths.claudeDir}/learning/journal.jsonl` },
  ];
  const entries: JournalEntry[] = [];
  const nextCursors = { ...cursors };
  const pcId = resolvePcId();

  for (const src of sources) {
    if (!existsSync(src.file)) continue;
    const lines = readFileSync(src.file, "utf8").split(/\r?\n/).filter(Boolean);
    const prev = Math.min(cursors[src.file] ?? 0, lines.length);
    for (const line of lines.slice(prev)) {
      try {
        const parsed = JSON.parse(line) as Partial<JournalEntry>;
        if (!parsed.summary || (parsed.kind !== "error" && parsed.kind !== "recovery" && parsed.kind !== "correction")) {
          continue;
        }
        entries.push({
          at: parsed.at ?? new Date().toISOString(),
          harness: parsed.harness || src.harness,
          agent: parsed.agent || parsed.harness || src.harness,
          pcId,
          session: parsed.session,
          cwd: parsed.cwd ? redactText(parsed.cwd) : undefined,
          kind: parsed.kind,
          tool: parsed.tool,
          summary: redactText(parsed.summary).slice(0, 360),
        });
      } catch {
        // Ignore malformed journal line.
      }
    }
    nextCursors[src.file] = lines.length;
  }

  return { entries: entries.slice(-100), nextCursors };
}

export function collectCanonicalSnapshot(paths = resolveClientPaths()): {
  harness: string;
  pcId: string;
  rules: Record<string, string>;
  memoryTopics: Record<
    string,
    { slug: string; title: string; summary: string; bullets: string[]; sources: string[]; confidence: number }
  >;
  brainDocs: Record<string, { path: string; sha256: string; content: string }>;
  skillDeltas: Record<string, { skill: string; notes: string[] }>;
  mcpProfile: { keepBaseline: string[]; disableRecommended: string[] };
} {
  const rules: Record<string, string> = {};
  const globalRuleFile = `${paths.geminiDir}/rules/global.md`;
  if (existsSync(globalRuleFile)) {
    rules.global = redactText(readFileSync(globalRuleFile, "utf8")).slice(0, 3500);
  }

  const memoryTopics: Record<
    string,
    { slug: string; title: string; summary: string; bullets: string[]; sources: string[]; confidence: number }
  > = {};
  const memDir = `${paths.geminiDir}/memory`;
  if (existsSync(memDir)) {
    for (const name of readdirSync(memDir).sort()) {
      if (!name.endsWith(".md") || name === "MEMORY.md" || name === "learned-cloud.md") continue;
      const slug = basename(name, ".md");
      const raw = redactText(readFileSync(join(memDir, name), "utf8"));
      const lines = raw.split(/\r?\n/).map((l) => l.trim()).filter(Boolean);
      const title = lines.find((l) => l.startsWith("#"))?.replace(/^#+\s*/, "") || slug;
      const bullets = lines
        .filter((l) => l.startsWith("- "))
        .map((l) => l.slice(2).trim())
        .slice(0, 16);
      memoryTopics[slug] = {
        slug,
        title,
        summary: (bullets[0] ?? lines[1] ?? title).slice(0, 320),
        bullets,
        sources: [resolvePcId()],
        confidence: 0.95,
      };
    }
  }

  const brainDocs: Record<string, { path: string; sha256: string; content: string }> = {};
  for (const group of ["bun", "gpui"]) {
    const idx = `${paths.geminiDir}/brain/${group}/INDEX.md`;
    if (existsSync(idx)) {
      const content = redactText(readFileSync(idx, "utf8")).slice(0, 16000);
      brainDocs[`${group}/INDEX.md`] = {
        path: `${group}/INDEX.md`,
        sha256: createHash("sha256").update(content).digest("hex"),
        content,
      };
    }
  }

  return {
    harness: "aphrody-sync",
    pcId: resolvePcId(),
    rules,
    memoryTopics,
    brainDocs,
    skillDeltas: {
      fleet: {
        skill: "fleet",
        notes: [
          "Coordinate via relay session_register/session_list/session_inbox before mutating shared /srv/xbuild dirs or aphrody-labs forks.",
          "Pipe multiline bash through @' ... '@ | ssh <host> \"tr -d '\\r' | bash\" on Windows PowerShell.",
        ],
      },
      "bun-gpui": {
        skill: "bun-gpui",
        notes: ["Prefix dot-directory test paths with ./ under bun test (e.g. ./.cloudflare/...)."],
      },
    },
    mcpProfile: {
      keepBaseline: ["relay", "bun-brain", "os-craft"],
      disableRecommended: [
        "vercel",
        "rea",
        "agent-browser",
        "obscura",
        "context7",
        "cloudflare",
        "cloudflare-docs",
        "better-auth",
        "shadcn",
        "next-devtools",
        "shenron",
        "xh",
      ],
    },
  };
}

export function renderCloudMemoryMarkdown(bundle: RemoteBundle): string {
  const topics = Object.values(bundle.memory.topics).sort((a, b) => a.slug.localeCompare(b.slug));
  const lines: string[] = [
    `# Aphrody Unified Cloud Memory (rev ${bundle.revision})`,
    "",
    `Synchronized autonomously from serverless knowledge hub (${bundle.updatedAt || "live"}).`,
    "",
  ];
  for (const topic of topics) {
    lines.push(`## ${topic.title}`);
    if (topic.summary) lines.push(topic.summary);
    for (const b of topic.bullets.slice(0, 12)) {
      lines.push(`- ${b}`);
    }
    lines.push("");
  }
  const skillEntries = Object.values(bundle.skills.deltas);
  if (skillEntries.length > 0) {
    lines.push("## Skill & Tooling Deltas");
    for (const s of skillEntries) {
      lines.push(`### ${s.skill}`);
      for (const n of s.notes.slice(0, 8)) lines.push(`- ${n}`);
      lines.push("");
    }
  }
  if (bundle.mcp.profile.keepBaseline.length > 0 || bundle.mcp.profile.disableRecommended.length > 0) {
    lines.push("## Canonical Lean MCP Profile");
    if (bundle.mcp.profile.keepBaseline.length) {
      lines.push(`- Keep baseline: \`${bundle.mcp.profile.keepBaseline.join("`, `")}\``);
    }
    if (bundle.mcp.profile.disableRecommended.length) {
      lines.push(`- Keep disabled globally: \`${bundle.mcp.profile.disableRecommended.join("`, `")}\``);
    }
    lines.push("");
  }
  return `${lines.join("\n").trim()}\n`;
}

export function applyBundleToLocalHarnesses(
  bundle: RemoteBundle,
  paths = resolveClientPaths(),
  options: { runLocalSync?: boolean; dryRun?: boolean } = {},
): { updatedFiles: string[]; harnessesTouched: string[] } {
  const updatedFiles: string[] = [];
  const harnessesTouched: string[] = [];
  const md = renderCloudMemoryMarkdown(bundle);
  const dryRun = !!options.dryRun;

  // 1. Antigravity (~/.gemini) — never embeds legacy harness paths.
  if (existsSync(paths.geminiDir)) {
    harnessesTouched.push("agy");
    const cloudMemPath = `${paths.geminiDir}/memory/learned-cloud.md`;
    if (!dryRun && writeIfChanged(cloudMemPath, md)) updatedFiles.push(cloudMemPath);

    const memIndex = `${paths.geminiDir}/memory/MEMORY.md`;
    const linkLine = "- [learned-cloud](learned-cloud.md) — Autonomous serverless shared memory, skills, and lean MCP convergence across Aphrody PCs and agents.";
    if (existsSync(memIndex)) {
      const cur = readFileSync(memIndex, "utf8");
      if (!cur.includes("(learned-cloud.md)")) {
        const next = `${cur.trimEnd()}\n${linkLine}\n`;
        if (!dryRun && writeIfChanged(memIndex, next)) updatedFiles.push(memIndex);
      }
    }

    for (const doc of Object.values(bundle.brain.docs)) {
      const target = `${paths.geminiDir}/brain/cloud/${doc.path.replace(/^\/+/, "")}`;
      if (!dryRun && writeIfChanged(target, doc.content)) updatedFiles.push(target);
    }

    if (!dryRun && options.runLocalSync) {
      const syncScript = `${paths.geminiDir}/scripts/sync.ts`;
      if (existsSync(syncScript)) {
        Bun.spawnSync(["bun", "--no-install", syncScript], { stdout: "ignore", stderr: "ignore", stdin: "ignore" });
      }
    }
  }

  // 2. Claude Code (~/.claude)
  if (existsSync(paths.claudeDir)) {
    harnessesTouched.push("claude");
    const claudeCloudMem = `${paths.claudeDir}/projects/shared/memory/learned-cloud.md`;
    if (!dryRun && writeIfChanged(claudeCloudMem, md)) updatedFiles.push(claudeCloudMem);

    const claudeMemIndex = `${paths.claudeDir}/projects/shared/memory/MEMORY.md`;
    const claudeIndexCur = existsSync(claudeMemIndex) ? readFileSync(claudeMemIndex, "utf8") : "# Shared Memory\n";
    if (!claudeIndexCur.includes("(learned-cloud.md)")) {
      const next = `${claudeIndexCur.trimEnd()}\n- [learned-cloud](learned-cloud.md) — Autonomous serverless shared memory and lean MCP convergence.\n`;
      if (!dryRun && writeIfChanged(claudeMemIndex, next)) updatedFiles.push(claudeMemIndex);
    }

    for (const doc of Object.values(bundle.brain.docs)) {
      const target = `${paths.claudeDir}/learning/brain-cloud/${doc.path.replace(/^\/+/, "")}`;
      if (!dryRun && writeIfChanged(target, doc.content)) updatedFiles.push(target);
    }

    if (!dryRun && options.runLocalSync) {
      const claudeSync = `${paths.claudeDir}/scripts/sync.ts`;
      if (existsSync(claudeSync)) {
        Bun.spawnSync(["bun", "--no-install", claudeSync, "--no-git"], {
          stdout: "ignore",
          stderr: "ignore",
          stdin: "ignore",
        });
      }
    }
  }

  // 3. Codex / OpenCode / Pi AGENTS.md alignment (non-clobbering: update only if already exists)
  for (const [name, targetFile] of [
    ["codex", `${paths.codexDir}/LEARNED_CLOUD.md`],
    ["opencode", `${paths.opencodeDir}/LEARNED_CLOUD.md`],
    ["pi", `${paths.piDir}/LEARNED_CLOUD.md`],
  ] as const) {
    const parent = resolve(targetFile, "..");
    if (existsSync(parent)) {
      harnessesTouched.push(name);
      if (!dryRun && writeIfChanged(targetFile, md)) updatedFiles.push(targetFile);
    }
  }

  return { updatedFiles, harnessesTouched };
}

function authHeaders(token: string | undefined): Record<string, string> {
  return {
    "content-type": "application/json",
    ...(token ? { authorization: `Bearer ${token}` } : {}),
  };
}

export async function syncCycle(
  mode: "status" | "push" | "pull" | "publish-local" | "cycle",
  paths = resolveClientPaths(),
  options: { force?: boolean; dryRun?: boolean } = {},
): Promise<Record<string, unknown>> {
  const baseUrl = resolveLearnUrl();
  const token = resolveLearnToken(paths);
  const state = loadSyncState(paths);
  const result: Record<string, unknown> = { ok: true, mode, baseUrl, dryRun: !!options.dryRun };

  if (mode === "status") {
    const { response: res, baseUrl: liveUrl } = await fetchWithLearnFallback("/api/learn/status", { headers: authHeaders(token) });
    result.baseUrl = liveUrl;
    result.remote = res.ok ? await res.json() : { ok: false, status: res.status };
    result.localState = state;
    return result;
  }

  if (mode === "publish-local") {
    const snapshot = collectCanonicalSnapshot(paths);
    if (!options.dryRun) {
      const { response: res, baseUrl: liveUrl } = await fetchWithLearnFallback("/api/learn/publish", {
        method: "POST",
        headers: authHeaders(token),
        body: JSON.stringify(snapshot),
      });
      result.baseUrl = liveUrl;
      if (!res.ok) throw new Error(`learn publish failed (${res.status}): ${await res.text()}`);
      result.published = await res.json();
    } else {
      result.snapshotSummary = {
        rules: Object.keys(snapshot.rules).length,
        memoryTopics: Object.keys(snapshot.memoryTopics).length,
        brainDocs: Object.keys(snapshot.brainDocs).length,
      };
    }
    if (mode === "publish-local") return result;
  }

  if (mode === "push" || mode === "cycle") {
    const { entries, nextCursors } = collectJournalDeltas(paths, state.journalCursors);
    result.entriesToPush = entries.length;
    if (entries.length > 0 && !options.dryRun) {
      const snapshot = collectCanonicalSnapshot(paths);
      const { response: res, baseUrl: liveUrl } = await fetchWithLearnFallback("/api/learn/events?distill=1", {
        method: "POST",
        headers: authHeaders(token),
        body: JSON.stringify({
          harness: "multi-agent",
          pcId: resolvePcId(),
          entries,
          mcpKeep: snapshot.mcpProfile.keepBaseline,
          mcpDisable: snapshot.mcpProfile.disableRecommended,
        }),
      });
      result.baseUrl = liveUrl;
      if (!res.ok) throw new Error(`learn events push failed (${res.status}): ${await res.text()}`);
      result.pushResponse = await res.json();
      state.journalCursors = nextCursors;
      saveSyncState(paths, state);
    }
  }

  if (mode === "pull" || mode === "cycle") {
    const manifestPath =
      options.force || state.lastRevision < 0
        ? "/api/learn/manifest.json"
        : `/api/learn/manifest.json?since=${state.lastRevision}`;
    const { response: manRes, baseUrl: liveUrl } = await fetchWithLearnFallback(manifestPath, { headers: authHeaders(token) });
    result.baseUrl = liveUrl;
    if (manRes.status === 304) {
      result.pull = { status: "up_to_date", revision: state.lastRevision };
      return result;
    }
    if (!manRes.ok) throw new Error(`learn manifest fetch failed (${manRes.status}): ${await manRes.text()}`);

    const bundleRes = await fetch(`${liveUrl}/api/learn/bundle.json`, { headers: authHeaders(token) });
    if (!bundleRes.ok) throw new Error(`learn bundle fetch failed (${bundleRes.status}): ${await bundleRes.text()}`);
    const bundle = (await bundleRes.json()) as RemoteBundle;
    const applied = applyBundleToLocalHarnesses(bundle, paths, { runLocalSync: true, dryRun: options.dryRun });
    if (!options.dryRun) {
      state.lastRevision = bundle.revision;
      state.lastSha256 = bundle.sha256;
      state.lastSyncAt = new Date().toISOString();
      saveSyncState(paths, state);
    }
    result.pull = { status: "updated", revision: bundle.revision, ...applied };
  }

  return result;
}

if (import.meta.main) {
  const args = Bun.argv.slice(2);
  const mode = (args[0] ?? "cycle") as "status" | "push" | "pull" | "publish-local" | "cycle";
  if (!["status", "push", "pull", "publish-local", "cycle"].includes(mode)) {
    console.error("Usage: bun tooling/bun/learn-cloud.ts [status|push|pull|publish-local|cycle] [--force] [--dry-run]");
    process.exit(1);
  }
  const out = await syncCycle(mode, resolveClientPaths(), {
    force: args.includes("--force"),
    dryRun: args.includes("--dry-run"),
  });
  console.log(JSON.stringify(out, null, 2));
}
