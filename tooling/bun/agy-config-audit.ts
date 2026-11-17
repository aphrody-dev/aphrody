#!/usr/bin/env bun
/**
 * Reproducible Antigravity CLI, App, Config & SDK conformance auditor for Aphrody.
 * Dynamically inspects ~/.gemini, ~/.gemini/config, workspace .agents/plugins,
 * installed plugins via `agy plugin validate --strict`, hook wire contracts,
 * MCP stdio handshake, and Python SDK imports without hardcoding user paths.
 */

import { existsSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join, resolve } from "node:path";

export interface AgyCheckResult {
  name: string;
  ok: boolean;
  detail: string;
}

export interface AgyAuditReport {
  cliVersion: string;
  geminiRoot: string;
  checks: AgyCheckResult[];
  passed: number;
  failed: number;
}

const LEGACY_SIDECARS = [
  "android-cli-plugin/compose-preview",
  "android-cli-plugin/device-hub",
  "science/biomolecular_structure_viewer",
];

export function hasCaseDuplicates(paths: string[]): string[] {
  const seen = new Map<string, string>();
  const dups: string[] = [];
  for (const raw of paths) {
    const norm = raw.replaceAll("\\", "/").replace(/\/+$/, "");
    const key = norm.toLowerCase();
    const prev = seen.get(key);
    if (prev && prev !== raw) {
      dups.push(`${prev} vs ${raw}`);
    } else {
      seen.set(key, raw);
    }
  }
  return dups;
}

export function auditSettingsObject(settings: Record<string, unknown>): AgyCheckResult[] {
  const results: AgyCheckResult[] = [];
  const schema = settings.$schema;
  results.push({
    name: "settings:$schema",
    ok: schema === "https://antigravity.google/schemas/v1/settings.json",
    detail: typeof schema === "string" ? schema : "missing $schema",
  });

  const permissions = settings.permissions as Record<string, unknown> | undefined;
  const deny = permissions?.deny;
  const hasDeny = Array.isArray(deny) && deny.length > 0;
  results.push({
    name: "settings:zero-deny",
    ok: !hasDeny,
    detail: hasDeny ? `found ${deny.length} deny rule(s)` : "zero deny rules",
  });

  const trusted = Array.isArray(settings.trustedWorkspaces) ? (settings.trustedWorkspaces as string[]) : [];
  const dups = hasCaseDuplicates(trusted);
  const missing = trusted.filter((w) => !existsSync(w));
  const legacy = trusted.filter((w) => /^[a-z]:[\\/](aphrody|labs)(?:[\\/]|$)/i.test(w));
  results.push({
    name: "settings:trustedWorkspaces",
    ok: trusted.length > 0 && dups.length === 0 && missing.length === 0 && legacy.length === 0,
    detail: `count=${trusted.length}, dups=${dups.length}, missing=${missing.length}, legacy=${legacy.length}`,
  });

  for (const key of ["statusLine", "title"] as const) {
    const blk = settings[key] as Record<string, unknown> | undefined;
    const valid = Boolean(blk && typeof blk === "object" && typeof blk.command === "string" && blk.command.trim());
    results.push({
      name: `settings:${key}`,
      ok: valid,
      detail: valid ? String(blk?.command) : `missing or invalid ${key}`,
    });
  }

  return results;
}

export function auditProjectsObject(projectsDoc: Record<string, unknown>, repoRoot: string): AgyCheckResult[] {
  const projects = (projectsDoc.projects ?? {}) as Record<string, string>;
  const keys = Object.keys(projects);
  const dups = hasCaseDuplicates(keys);
  const missing = keys.filter((k) => !existsSync(k));
  const normRepo = repoRoot.replaceAll("\\", "/").toLowerCase();
  const hasRepo = keys.some((k) => k.replaceAll("\\", "/").toLowerCase() === normRepo);
  return [
    {
      name: "projects:case-and-existence",
      ok: keys.length > 0 && dups.length === 0 && missing.length === 0 && hasRepo,
      detail: `entries=${keys.length}, dups=${dups.length}, missing=${missing.length}, hasAphrody=${hasRepo}`,
    },
  ];
}

function splitShellArgs(raw: string, home: string): string[] {
  const matches = raw.match(/(?:[^\s"]+|"[^"]*")+/g) ?? [];
  return matches.map((tok) => expandHome(tok.replace(/^"|"$/g, ""), home));
}

function runShellCommand(cmd: string, stdinText: string, home: string): { ok: boolean; out: string; err: string } {
  const args = splitShellArgs(cmd, home);
  if (args.length === 0) return { ok: false, out: "", err: "empty command" };
  const r = Bun.spawnSync(args, {
    stdin: Buffer.from(stdinText, "utf8"),
    stdout: "pipe",
    stderr: "pipe",
  });
  return {
    ok: r.exitCode === 0,
    out: r.stdout.toString().trim(),
    err: r.stderr.toString().trim(),
  };
}

function expandHome(raw: string, home: string): string {
  if (raw.startsWith("~/") || raw.startsWith("~\\")) {
    return join(home, raw.slice(2));
  }
  return raw;
}

export function auditLiveEnvironment(repoRoot: string, home = homedir()): AgyAuditReport {
  const geminiRoot = join(home, ".gemini");
  const checks: AgyCheckResult[] = [];

  const verRes = Bun.spawnSync(["agy", "--version"], { stdout: "pipe", stderr: "pipe", stdin: "ignore" });
  const cliVersion = verRes.exitCode === 0 ? verRes.stdout.toString().trim() : "missing";
  checks.push({
    name: "cli:agy-version",
    ok: verRes.exitCode === 0 && cliVersion.length > 0,
    detail: cliVersion,
  });

  // 1. Validate ~/.gemini/antigravity-cli/settings.json + run statusLine & title
  const settingsPath = join(geminiRoot, "antigravity-cli", "settings.json");
  if (existsSync(settingsPath)) {
    const settings = JSON.parse(readFileSync(settingsPath, "utf8")) as Record<string, unknown>;
    checks.push(...auditSettingsObject(settings));

    const samplePayload = JSON.stringify({
      cwd: repoRoot,
      agent_state: "working",
      cycle_mode: "auto",
      conversation_title: "Audit",
      context_window: { used_percentage: 25 },
      cost: { total_usd: 0.05 },
    });
    for (const key of ["statusLine", "title"] as const) {
      const blk = settings[key] as { command?: string; enabled?: boolean } | undefined;
      if (blk?.command && blk.enabled !== false) {
        const res = runShellCommand(blk.command, samplePayload, home);
        checks.push({
          name: `exec:${key}`,
          ok: res.ok && res.out.length > 0 && !res.out.includes("error"),
          detail: res.ok ? res.out.slice(0, 120) : res.err.slice(0, 120),
        });
      }
    }
  } else {
    checks.push({ name: "settings:exists", ok: false, detail: `missing ${settingsPath}` });
  }

  // 2. Validate projects.json + config/config.json sidecars + config/mcp_config.json
  const projectsPath = join(geminiRoot, "projects.json");
  if (existsSync(projectsPath)) {
    const doc = JSON.parse(readFileSync(projectsPath, "utf8")) as Record<string, unknown>;
    checks.push(...auditProjectsObject(doc, repoRoot));
  } else {
    checks.push({ name: "projects:exists", ok: false, detail: `missing ${projectsPath}` });
  }

  const configPath = join(geminiRoot, "config", "config.json");
  if (existsSync(configPath)) {
    const cfg = JSON.parse(readFileSync(configPath, "utf8")) as { sidecars?: Record<string, unknown> };
    const stale = LEGACY_SIDECARS.filter((k) => Boolean(cfg.sidecars && k in cfg.sidecars));
    checks.push({
      name: "config:zero-stale-sidecars",
      ok: stale.length === 0,
      detail: stale.length === 0 ? "clean" : `stale: ${stale.join(", ")}`,
    });
  }

  const mcpPath = join(geminiRoot, "config", "mcp_config.json");
  if (existsSync(mcpPath)) {
    const mcp = JSON.parse(readFileSync(mcpPath, "utf8")) as {
      mcpServers?: Record<string, { command?: string; args?: string[]; url?: string; serverUrl?: string }>;
    };
    const servers = mcp.mcpServers ?? {};
    let allExist = true;
    const details: string[] = [];
    for (const [name, entry] of Object.entries(servers)) {
      if (entry.args) {
        for (const a of entry.args) {
          if (/\.(ts|js|mjs|cjs|py)$/i.test(a)) {
            const target = expandHome(a, home);
            const ok = existsSync(target);
            if (!ok) allExist = false;
            details.push(`${name}:${ok ? "ok" : "missing(" + target + ")"}`);
          }
        }
      } else {
        details.push(`${name}:${entry.url ?? entry.serverUrl ?? "remote"}`);
      }
    }
    checks.push({
      name: "config:mcp-targets",
      ok: Object.keys(servers).length > 0 && allExist,
      detail: details.join(", ") || "empty",
    });

    const relay = servers.relay;
    if (relay?.command && relay.args) {
      const initReq = JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method: "initialize",
        params: {
          protocolVersion: "2024-11-05",
          capabilities: {},
          clientInfo: { name: "agy-audit", version: "1.0" },
        },
      });
      const r = Bun.spawnSync([relay.command, ...relay.args], {
        stdin: Buffer.from(`${initReq}\n`, "utf8"),
        stdout: "pipe",
        stderr: "pipe",
      });
      const out = r.stdout.toString();
      checks.push({
        name: "mcp:relay-initialize",
        ok: r.exitCode === 0 && out.includes('"serverInfo"'),
        detail: out.split(/\r?\n/)[0]?.slice(0, 120) || r.stderr.toString().trim().slice(0, 120),
      });
    }
  }

  // 3. Validate plugins with `agy plugin validate --strict`
  const pluginDirs = [
    resolve(repoRoot, ".agents", "plugins", "aphrody"),
    join(geminiRoot, "config", "plugins", "aphrody"),
    join(geminiRoot, "config", "plugins", "bun"),
  ].filter((p) => existsSync(p));

  for (const dir of pluginDirs) {
    const r = Bun.spawnSync(["agy", "plugin", "validate", dir, "--strict"], {
      stdout: "pipe",
      stderr: "pipe",
      stdin: "ignore",
    });
    checks.push({
      name: `plugin:${dir.replaceAll("\\", "/").split("/").slice(-2).join("/")}`,
      ok: r.exitCode === 0,
      detail: (r.stdout.toString() + r.stderr.toString()).trim().split(/\r?\n/).pop() ?? "",
    });
  }

  // 4. Validate repo post-edit hook with real Antigravity `toolCall` shape
  const postEditHook = resolve(repoRoot, ".agents", "plugins", "aphrody", "hooks", "post-edit-check.ts");
  if (existsSync(postEditHook)) {
    const r = Bun.spawnSync(["bun", "--no-install", "--no-env-file", postEditHook, "agy"], {
      stdin: Buffer.from(
        JSON.stringify({
          conversationId: "00000000-0000-0000-0000-000000000001",
          hookEventName: "PostToolUse",
          toolCall: { name: "write_file", args: { Path: resolve(repoRoot, "README.md") } },
        }),
        "utf8",
      ),
      stdout: "pipe",
      stderr: "pipe",
    });
    checks.push({
      name: "hook:repo-post-edit-agy",
      ok: r.exitCode === 0 && r.stdout.toString().trim() === "{}",
      detail: r.stdout.toString().trim(),
    });
  }

  // 5. Python SDK smoke verification
  const pyWin = join(geminiRoot, "sdk", ".venv", "Scripts", "python.exe");
  const pyUnix = join(geminiRoot, "sdk", ".venv", "bin", "python");
  const pyBin = existsSync(pyWin) ? pyWin : existsSync(pyUnix) ? pyUnix : null;
  if (pyBin) {
    const r = Bun.spawnSync(
      [
        pyBin,
        "-c",
        "from google.antigravity import Agent, LocalAgentConfig, BuiltinTools; assert hasattr(BuiltinTools, 'VIEW_FILE'); print('sdk-ok')",
      ],
      { stdout: "pipe", stderr: "pipe", stdin: "ignore" },
    );
    checks.push({
      name: "sdk:google-antigravity",
      ok: r.exitCode === 0 && r.stdout.toString().includes("sdk-ok"),
      detail: (r.stdout.toString() + r.stderr.toString()).trim(),
    });
  }

  const passed = checks.filter((c) => c.ok).length;
  const failed = checks.length - passed;
  return { cliVersion, geminiRoot, checks, passed, failed };
}

if (import.meta.main) {
  const repoRoot = resolve(import.meta.dir, "..", "..");
  const report = auditLiveEnvironment(repoRoot);
  if (Bun.argv.includes("--json")) {
    console.log(JSON.stringify(report, null, 2));
  } else {
    console.log(`Antigravity CLI ${report.cliVersion} | ${report.passed}/${report.checks.length} checks passed`);
    for (const c of report.checks) {
      console.log(`${c.ok ? "PASS" : "FAIL"} [${c.name}] ${c.detail}`);
    }
  }
  process.exit(report.failed === 0 ? 0 : 1);
}
