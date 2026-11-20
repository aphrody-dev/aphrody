#!/usr/bin/env bun
/**
 * Vendor, Third-Party Supply-Chain, and WireGuard LAN Doctor for Aphrody.
 *
 * Audits:
 *   1. All 10 `.gitmodules` entries and their local manifest/git status
 *   2. Root `Cargo.toml` `[patch.crates-io]` path & git consistency
 *   3. `deny.toml` policy readiness (`[licenses]`, `[sources]`, `unknown-git`)
 *   4. Companion workspace repos in parent directory (`../<repo>/.gitmodules`) without mutating them
 *   5. `crates/aphrody_vpn` LAN-sharing toggle (`Toggle::AllowLan`)
 *   6. WireGuard LAN SSH fleet (`fr1`, `fr2`, `prd`) via `ssh -G` (+ optional live probe)
 *
 * Usage:
 *   bun tooling/bun/vendor-lan-doctor.ts audit [--offline] [--json]
 */
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { listNestedReferencedPaths, listReferencedPaths, parseGitmodules } from "./submodules.ts";

export const repoRoot = resolve(import.meta.dir, "../..");

export interface PatchEntry {
  crateName: string;
  kind: "path" | "git";
  target: string;
  exists: boolean;
}

export interface CompanionSubmoduleStatus {
  repo: string;
  declared: number;
  initialized: number;
  optionalUninitialized: string[];
  missingRequired: string[];
}

export interface LanHostStatus {
  alias: string;
  configured: boolean;
  hostname?: string;
  user?: string;
  port?: string;
  inWireguardLan: boolean;
  reachable?: boolean;
  latencyMs?: number;
}

export interface VendorLanReport {
  ok: boolean;
  vendor: {
    declaredSubmodules: number;
    cargoReferenced: number;
    nestedCargoSubmodules: Array<{
      parentPath: string;
      nestedPath: string;
      manifestPresent: boolean;
    }>;
    patches: PatchEntry[];
    missingPatchPaths: string[];
  };
  supplyChain: {
    denyTomlPresent: boolean;
    hasLicensesSection: boolean;
    hasSourcesSection: boolean;
    unknownGitPolicy: string | null;
  };
  companionRepos: CompanionSubmoduleStatus[];
  vpnAllowLanWired: boolean;
  lanHosts: LanHostStatus[];
  errors: string[];
}

const OPTIONAL_MIRROR_PREFIXES = ["books/", "windows/git-sdk-64"];

export function parsePatchesSection(cargoToml: string): Array<{ crateName: string; kind: "path" | "git"; target: string }> {
  const results: Array<{ crateName: string; kind: "path" | "git"; target: string }> = [];
  let inPatch = false;
  for (const rawLine of cargoToml.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith("#")) continue;
    if (line.startsWith("[")) {
      inPatch = line === "[patch.crates-io]";
      continue;
    }
    if (!inPatch) continue;
    const eqIdx = line.indexOf("=");
    if (eqIdx <= 0) continue;
    const crateName = line.slice(0, eqIdx).trim();
    const rhs = line.slice(eqIdx + 1).trim();
    const pathMatch = /path\s*=\s*"([^"]+)"/.exec(rhs);
    if (pathMatch) {
      results.push({ crateName, kind: "path", target: pathMatch[1] });
      continue;
    }
    const gitMatch = /git\s*=\s*"([^"]+)"/.exec(rhs);
    if (gitMatch) {
      results.push({ crateName, kind: "git", target: gitMatch[1] });
    }
  }
  return results;
}

export function isWireguardOrPrivateLanIp(host: string | undefined): boolean {
  if (!host) return false;
  if (host.startsWith("10.200.77.")) return true;
  if (/^10\.\d+\.\d+\.\d+$/.test(host)) return true;
  if (/^192\.168\.\d+\.\d+$/.test(host)) return true;
  if (/^172\.(1[6-9]|2\d|3[01])\.\d+\.\d+$/.test(host)) return true;
  return false;
}

export function parseSshGOutput(alias: string, output: string): LanHostStatus {
  const map = new Map<string, string>();
  for (const line of output.split(/\r?\n/)) {
    const sp = line.trim().indexOf(" ");
    if (sp > 0) {
      const k = line.slice(0, sp).toLowerCase();
      const v = line.slice(sp + 1).trim();
      if (!map.has(k)) map.set(k, v);
    }
  }
  const hostname = map.get("hostname");
  const user = map.get("user");
  const port = map.get("port");
  const configured = Boolean(hostname && hostname !== alias);
  return {
    alias,
    configured,
    hostname,
    user,
    port,
    inWireguardLan: isWireguardOrPrivateLanIp(hostname),
  };
}

export function inspectDenyPolicy(denyPath: string): VendorLanReport["supplyChain"] {
  if (!existsSync(denyPath)) {
    return {
      denyTomlPresent: false,
      hasLicensesSection: false,
      hasSourcesSection: false,
      unknownGitPolicy: null,
    };
  }
  const text = readFileSync(denyPath, "utf8");
  const parsed = Bun.TOML.parse(text) as {
    licenses?: Record<string, unknown>;
    sources?: { "unknown-git"?: string };
  };
  return {
    denyTomlPresent: true,
    hasLicensesSection: Boolean(parsed.licenses),
    hasSourcesSection: Boolean(parsed.sources),
    unknownGitPolicy: parsed.sources?.["unknown-git"] ?? null,
  };
}

export function inspectCompanionRepos(parentDir: string, selfRepoName: string): CompanionSubmoduleStatus[] {
  if (!existsSync(parentDir)) return [];
  const results: CompanionSubmoduleStatus[] = [];
  for (const entry of readdirSync(parentDir, { withFileTypes: true })) {
    if (!entry.isDirectory() || entry.name === selfRepoName) continue;
    const repoPath = join(parentDir, entry.name);
    const gitmodulesPath = join(repoPath, ".gitmodules");
    if (!existsSync(gitmodulesPath)) continue;
    const submodules = parseGitmodules(readFileSync(gitmodulesPath, "utf8"));
    let initialized = 0;
    const optionalUninitialized: string[] = [];
    const missingRequired: string[] = [];
    for (const sub of submodules) {
      const subDir = join(repoPath, sub.path);
      const hasGit = existsSync(join(subDir, ".git"));
      if (hasGit) {
        initialized++;
      } else if (OPTIONAL_MIRROR_PREFIXES.some((prefix) => sub.path.startsWith(prefix))) {
        optionalUninitialized.push(sub.path);
      } else {
        missingRequired.push(sub.path);
      }
    }
    results.push({
      repo: entry.name,
      declared: submodules.length,
      initialized,
      optionalUninitialized,
      missingRequired,
    });
  }
  return results.sort((a, b) => a.repo.localeCompare(b.repo));
}

export function checkVpnAllowLan(root = repoRoot): boolean {
  const backendPath = join(root, "crates/aphrody_vpn/src/backend.rs");
  const daemonPath = join(root, "crates/aphrody_vpn/src/daemon_backend.rs");
  if (!existsSync(backendPath) || !existsSync(daemonPath)) return false;
  const backend = readFileSync(backendPath, "utf8");
  const daemon = readFileSync(daemonPath, "utf8");
  return backend.includes("AllowLan") && daemon.includes("AllowLan");
}

function probeLanHosts(offline: boolean): LanHostStatus[] {
  const aliases = ["fr1", "fr2", "prd"];
  const statuses: LanHostStatus[] = [];
  for (const alias of aliases) {
    const g = Bun.spawnSync(["ssh", "-G", alias], { stdout: "pipe", stderr: "pipe" });
    if (g.exitCode !== 0) {
      statuses.push({ alias, configured: false, inWireguardLan: false });
      continue;
    }
    const status = parseSshGOutput(alias, new TextDecoder().decode(g.stdout));
    if (!offline && status.configured) {
      const started = performance.now();
      const probe = Bun.spawnSync(
        ["ssh", "-o", "BatchMode=yes", "-o", "ConnectTimeout=4", alias, "echo", "ok"],
        { stdout: "pipe", stderr: "pipe" },
      );
      status.latencyMs = Math.round(performance.now() - started);
      status.reachable =
        probe.exitCode === 0 && new TextDecoder().decode(probe.stdout).trim() === "ok";
    }
    statuses.push(status);
  }
  return statuses;
}

export function runAudit(root = repoRoot, offline = false): VendorLanReport {
  const errors: string[] = [];
  const gitmodulesPath = join(root, ".gitmodules");
  const gitmodulesRaw = existsSync(gitmodulesPath) ? readFileSync(gitmodulesPath, "utf8") : "";
  const declared = parseGitmodules(gitmodulesRaw);
  const cargoReferenced = listReferencedPaths();
  const nested = listNestedReferencedPaths(root);
  for (const n of nested) {
    if (!n.manifestPresent) {
      errors.push(`nested Cargo-referenced submodule missing manifest: ${n.parentPath}/${n.nestedPath}`);
    }
  }

  const cargoToml = readFileSync(join(root, "Cargo.toml"), "utf8");
  const parsedPatches = parsePatchesSection(cargoToml);
  const patches: PatchEntry[] = parsedPatches.map((p) => ({
    ...p,
    exists: p.kind === "git" ? true : existsSync(join(root, p.target, "Cargo.toml")),
  }));
  const missingPatchPaths = patches.filter((p) => p.kind === "path" && !p.exists).map((p) => p.target);
  for (const mp of missingPatchPaths) {
    errors.push(`[patch.crates-io] target missing Cargo.toml: ${mp}`);
  }

  const supplyChain = inspectDenyPolicy(join(root, "deny.toml"));
  if (!supplyChain.denyTomlPresent) {
    errors.push("deny.toml is missing");
  } else if (!supplyChain.hasLicensesSection || !supplyChain.hasSourcesSection) {
    errors.push("deny.toml missing [licenses] or [sources] section");
  }

  const vpnAllowLanWired = checkVpnAllowLan(root);
  if (!vpnAllowLanWired) {
    errors.push("crates/aphrody_vpn missing Toggle::AllowLan wiring");
  }

  const companionRepos = inspectCompanionRepos(dirname(root), basename(root));
  const lanHosts = probeLanHosts(offline);
  if (!offline) {
    for (const host of lanHosts) {
      if (host.configured && host.reachable === false) {
        errors.push(`LAN host ${host.alias} (${host.hostname ?? "unknown"}) unreachable over SSH`);
      }
    }
  }

  return {
    ok: errors.length === 0,
    vendor: {
      declaredSubmodules: declared.length,
      cargoReferenced: cargoReferenced.length,
      nestedCargoSubmodules: nested.map((n) => ({
        parentPath: n.parentPath,
        nestedPath: n.nestedPath,
        manifestPresent: n.manifestPresent,
      })),
      patches,
      missingPatchPaths,
    },
    supplyChain,
    companionRepos,
    vpnAllowLanWired,
    lanHosts,
    errors,
  };
}

if (import.meta.main) {
  const args = Bun.argv.slice(2);
  const offline = args.includes("--offline");
  const jsonMode = args.includes("--json");
  const report = runAudit(repoRoot, offline);

  if (jsonMode) {
    console.log(JSON.stringify(report, null, 2));
  } else {
    console.log(
      `Vendor/LAN Audit: submodules=${report.vendor.declaredSubmodules} cargoRefs=${report.vendor.cargoReferenced} patches=${report.vendor.patches.length} deny=${report.supplyChain.denyTomlPresent ? "ok" : "missing"} vpnAllowLan=${report.vpnAllowLanWired}`,
    );
    for (const n of report.vendor.nestedCargoSubmodules) {
      console.log(`  [nested] ${n.parentPath}/${n.nestedPath}: manifest=${n.manifestPresent}`);
    }
    for (const c of report.companionRepos) {
      const optNote =
        c.optionalUninitialized.length > 0
          ? ` (optional uninit: ${c.optionalUninitialized.length})`
          : "";
      const missNote =
        c.missingRequired.length > 0
          ? ` [uninit: ${c.missingRequired.join(", ")}]`
          : "";
      console.log(`  [companion] ${c.repo}: ${c.initialized}/${c.declared}${optNote}${missNote}`);
    }
    for (const h of report.lanHosts) {
      const reach =
        h.reachable === undefined ? "offline-skip" : h.reachable ? `ok (${h.latencyMs}ms)` : "FAIL";
      console.log(
        `  [lan] ${h.alias}: configured=${h.configured} wg=${h.inWireguardLan} host=${h.hostname ?? "-"} probe=${reach}`,
      );
    }
    if (report.errors.length > 0) {
      for (const e of report.errors) console.error(`ERROR: ${e}`);
    }
  }
  if (!report.ok) process.exit(1);
}
