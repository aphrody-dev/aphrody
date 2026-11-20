#!/usr/bin/env bun
/**
 * Reproducible, Zero-Hardcoding, Cross-Platform Multi-VPS + Cloudflare Edge Load Balancer CLI.
 *
 * Discovers SSH nodes dynamically from `~/.ssh/config` + `assets/os/fleet_cluster.toml`,
 * probes Linux hosts asynchronously via `ssh -o BatchMode=yes` (`tr -d '\r' | bash -s`)
 * alongside local Windows/macOS/Linux host load (`os.loadavg`, `os.cpus`, `os.freemem`),
 * scores capacity dynamically, schedules sharded workloads, dispatches commands with
 * automatic work-stealing fallback, and publishes signed/unauthenticated telemetry beacons
 * to Cloudflare Worker `/api/fleet/beacon`.
 *
 * Usage:
 *   bun tooling/bun/fleet-lb.ts probe [--all|--hosts h1,h2|--local] [--json]
 *   bun tooling/bun/fleet-lb.ts schedule --job <kind> [--shards N] [--hosts ...|--all|--local] [--json]
 *   bun tooling/bun/fleet-lb.ts dispatch --cmd <command> [--hosts ...|--all|--local] [--dry-run] [--json]
 *   bun tooling/bun/fleet-lb.ts beacon [--worker-url <url>] [--dry-run] [--json]
 */
import { existsSync, readFileSync, statfsSync } from "node:fs";
import os from "node:os";
import { join, resolve } from "node:path";

export const repoRoot = resolve(import.meta.dir, "../..");
export const DEFAULT_CLUSTER_MANIFEST_REL = "assets/os/fleet_cluster.toml";

export interface DiscoveryConfig {
  sshConfigPath: string;
  sshBatchMode: boolean;
  sshConnectTimeoutSec: number;
  sshProbeTimeoutMs: number;
  ignoreHostPatterns: string[];
  envHostsOverride: string;
  envSshConfigOverride: string;
  includeLocalFallback: boolean;
}

export interface ScoringWeights {
  loadPenaltyWeight: number;
  loadRatioCap: number;
  memoryPenaltyWeight: number;
  diskPenaltyWeight: number;
  minWorkAvailGb: number;
  activeJobPenalty: number;
  latencyPenaltyWeight: number;
  rttScaleMs: number;
  maxLoadPerCpu: number;
  minFreeMemGb: number;
  minFreeDiskGb: number;
  roleAffinityBonus: number;
  regionAffinityBonus: number;
  staleTelemetryTtlSec: number;
  workStealingMaxRetries: number;
}

export interface JobProfile {
  kind: string;
  requiredCapabilities: string[];
  preferredCapabilities: string[];
  defaultShards: number;
  minFreeMemGb: number;
  minFreeDiskGb: number;
}

export interface DeclaredNodeConfig {
  alias: string;
  sshAliases: string[];
  region?: string;
  tier?: string;
  priorityBias: number;
  workDir: string;
  capabilities: string[];
  labels: string[];
}

export interface FleetClusterManifest {
  schemaVersion: number;
  discovery: DiscoveryConfig;
  supportedCapabilities: string[];
  scoring: ScoringWeights;
  jobs: Record<string, JobProfile>;
  cloudflare: {
    workerName: string;
    workerDir: string;
    wranglerConfig: string;
    queueBinding: string;
    queueName: string;
    telemetryR2Key: string;
    beaconTokenEnv: string;
    workerUrlEnv: string;
    r2Buckets: { releases: string; assets: string };
    endpoints: {
      releasesBase: string;
      docsBase: string;
      assetsBase: string;
      health: string;
      fleetStatus: string;
      fleetRoute: string;
      fleetBeacon: string;
      githubWebhook: string;
    };
  };
  nodes: DeclaredNodeConfig[];
}

export interface DiscoveredSshNode {
  alias: string;
  sshTarget: string;
  aliasesInBlock: string[];
  declared?: DeclaredNodeConfig;
  discoveredFromSshConfig: boolean;
}

export interface NodeProbeTelemetry {
  alias: string;
  sshTarget: string;
  region?: string;
  tier?: string;
  priorityBias: number;
  reachable: boolean;
  healthy: boolean;
  saturated: boolean;
  healthReasons: string[];
  os: string;
  arch: string;
  kernel: string;
  cpus: number;
  load1: number;
  load5: number;
  load15: number;
  loadRatio: number;
  memTotalGb: number;
  memAvailGb: number;
  workDir: string;
  workTotalGb: number;
  workAvailGb: number;
  workUsePct: number;
  fsType: string;
  reflinkSupported: boolean;
  activeCompilerProcs: number;
  activeRunnerWorkers: number;
  activeJobs: number;
  rttMs: number;
  capabilities: string[];
  labels: string[];
  score: number;
  reportedAt: string;
  error?: string;
}

export interface ShardAssignment {
  shard: number;
  host: string;
  effectiveScore: number;
  region?: string;
  capabilities: string[];
}

export interface SchedulePlan {
  ok: boolean;
  job: string;
  shardsRequested: number;
  requiredCapabilities: string[];
  preferredCapabilities: string[];
  primaryHost: string | null;
  fallbackChain: string[];
  assignments: ShardAssignment[];
  rankedCandidates: Array<{
    alias: string;
    effectiveScore: number;
    loadRatio: number;
    memAvailGb: number;
    workAvailGb: number;
    activeJobs: number;
    capabilities: string[];
  }>;
  rejected: Array<{ alias: string; reasons: string[] }>;
}

export interface DispatchAttempt {
  host: string;
  score: number;
  exitCode: number;
  durationMs: number;
  ok: boolean;
  stolenFrom?: string;
  stdout: string;
  stderr: string;
}

export interface DispatchResult {
  ok: boolean;
  command: string;
  dryRun: boolean;
  executedHost: string | null;
  fallbackChain: string[];
  attempts: DispatchAttempt[];
  exitCode: number;
  stdout: string;
  stderr: string;
}

const DEFAULT_SCORING: ScoringWeights = {
  loadPenaltyWeight: 42,
  loadRatioCap: 1.5,
  memoryPenaltyWeight: 25,
  diskPenaltyWeight: 18,
  minWorkAvailGb: 25,
  activeJobPenalty: 8,
  latencyPenaltyWeight: 7,
  rttScaleMs: 3000,
  maxLoadPerCpu: 1.35,
  minFreeMemGb: 2,
  minFreeDiskGb: 10,
  roleAffinityBonus: 10,
  regionAffinityBonus: 4,
  staleTelemetryTtlSec: 300,
  workStealingMaxRetries: 2,
};

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function round2(value: number): number {
  return Number(value.toFixed(2));
}

function asNum(value: unknown, fallback: number): number {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

function asStr(value: unknown, fallback: string): string {
  return typeof value === "string" && value.trim() ? value.trim() : fallback;
}

function asStrArray(value: unknown, fallback: string[] = []): string[] {
  if (!Array.isArray(value)) return [...fallback];
  return value.filter((item): item is string => typeof item === "string" && item.trim().length > 0).map((s) => s.trim());
}

export function expandTildePath(inputPath: string): string {
  if (inputPath === "~") return os.homedir();
  if (inputPath.startsWith("~/") || inputPath.startsWith("~\\")) {
    return join(os.homedir(), inputPath.slice(2));
  }
  return inputPath;
}

export function loadFleetManifest(root = repoRoot): FleetClusterManifest {
  const manifestPath = join(root, DEFAULT_CLUSTER_MANIFEST_REL);
  const raw = existsSync(manifestPath) ? (Bun.TOML.parse(readFileSync(manifestPath, "utf8")) as Record<string, any>) : {};

  const disc = (raw.discovery ?? {}) as Record<string, unknown>;
  const sc = (raw.scoring ?? {}) as Record<string, unknown>;
  const cf = (raw.cloudflare ?? {}) as Record<string, any>;
  const cfBuckets = (cf.r2_buckets ?? {}) as Record<string, unknown>;
  const cfEndpoints = (cf.endpoints ?? {}) as Record<string, unknown>;
  const rawJobs = (raw.jobs ?? {}) as Record<string, Record<string, unknown>>;
  const rawNodes = Array.isArray(raw.nodes) ? (raw.nodes as Array<Record<string, unknown>>) : [];

  const jobs: Record<string, JobProfile> = {};
  for (const [kind, spec] of Object.entries(rawJobs)) {
    jobs[kind] = {
      kind,
      requiredCapabilities: asStrArray(spec.required_capabilities),
      preferredCapabilities: asStrArray(spec.preferred_capabilities),
      defaultShards: Math.max(1, Math.floor(asNum(spec.default_shards, 1))),
      minFreeMemGb: asNum(spec.min_free_mem_gb, asNum(sc.min_free_mem_gb, DEFAULT_SCORING.minFreeMemGb)),
      minFreeDiskGb: asNum(spec.min_free_disk_gb, asNum(sc.min_free_disk_gb, DEFAULT_SCORING.minFreeDiskGb)),
    };
  }

  const nodes: DeclaredNodeConfig[] = rawNodes
    .filter((n) => typeof n.alias === "string" && n.alias.trim().length > 0)
    .map((n) => {
      const alias = String(n.alias).trim();
      const sshAliases = asStrArray(n.ssh_aliases, [alias]);
      if (!sshAliases.includes(alias)) sshAliases.unshift(alias);
      return {
        alias,
        sshAliases,
        region: typeof n.region === "string" ? n.region.trim() : undefined,
        tier: typeof n.tier === "string" ? n.tier.trim() : undefined,
        priorityBias: asNum(n.priority_bias, 0),
        workDir: asStr(n.work_dir, "/srv/xbuild"),
        capabilities: asStrArray(n.capabilities),
        labels: asStrArray(n.labels),
      };
    });

  return {
    schemaVersion: asNum(raw.schema_version, 1),
    discovery: {
      sshConfigPath: asStr(disc.ssh_config_path, "~/.ssh/config"),
      sshBatchMode: typeof disc.ssh_batch_mode === "boolean" ? disc.ssh_batch_mode : true,
      sshConnectTimeoutSec: Math.max(1, Math.floor(asNum(disc.ssh_connect_timeout_sec, 4))),
      sshProbeTimeoutMs: Math.max(1000, Math.floor(asNum(disc.ssh_probe_timeout_ms, 8000))),
      ignoreHostPatterns: asStrArray(disc.ignore_host_patterns, ["*", "git", "github.com", "*.github.com", "localhost"]),
      envHostsOverride: asStr(disc.env_hosts_override, "APHRODY_FLEET_HOSTS"),
      envSshConfigOverride: asStr(disc.env_ssh_config_override, "APHRODY_SSH_CONFIG"),
      includeLocalFallback: typeof disc.include_local_fallback === "boolean" ? disc.include_local_fallback : true,
    },
    supportedCapabilities: asStrArray(raw.capabilities?.supported, [
      "xfs_reflink",
      "btrfs_reflink",
      "wasm",
      "native_rust",
      "edge_proxy",
    ]),
    scoring: {
      loadPenaltyWeight: asNum(sc.load_penalty_weight, DEFAULT_SCORING.loadPenaltyWeight),
      loadRatioCap: asNum(sc.load_ratio_cap, DEFAULT_SCORING.loadRatioCap),
      memoryPenaltyWeight: asNum(sc.memory_penalty_weight, DEFAULT_SCORING.memoryPenaltyWeight),
      diskPenaltyWeight: asNum(sc.disk_penalty_weight, DEFAULT_SCORING.diskPenaltyWeight),
      minWorkAvailGb: asNum(sc.min_work_avail_gb, DEFAULT_SCORING.minWorkAvailGb),
      activeJobPenalty: asNum(sc.active_job_penalty, DEFAULT_SCORING.activeJobPenalty),
      latencyPenaltyWeight: asNum(sc.latency_penalty_weight, DEFAULT_SCORING.latencyPenaltyWeight),
      rttScaleMs: asNum(sc.rtt_scale_ms, DEFAULT_SCORING.rttScaleMs),
      maxLoadPerCpu: asNum(sc.max_load_per_cpu, DEFAULT_SCORING.maxLoadPerCpu),
      minFreeMemGb: asNum(sc.min_free_mem_gb, DEFAULT_SCORING.minFreeMemGb),
      minFreeDiskGb: asNum(sc.min_free_disk_gb, DEFAULT_SCORING.minFreeDiskGb),
      roleAffinityBonus: asNum(sc.role_affinity_bonus, DEFAULT_SCORING.roleAffinityBonus),
      regionAffinityBonus: asNum(sc.region_affinity_bonus, DEFAULT_SCORING.regionAffinityBonus),
      staleTelemetryTtlSec: asNum(sc.stale_telemetry_ttl_sec, DEFAULT_SCORING.staleTelemetryTtlSec),
      workStealingMaxRetries: Math.max(0, Math.floor(asNum(sc.work_stealing_max_retries, DEFAULT_SCORING.workStealingMaxRetries))),
    },
    jobs,
    cloudflare: {
      workerName: asStr(cf.worker_name, "aphrody-worker"),
      workerDir: asStr(cf.worker_dir, ".cloudflare/aphrody-worker"),
      wranglerConfig: asStr(cf.wrangler_config, ".cloudflare/aphrody-worker/wrangler.toml"),
      queueBinding: asStr(cf.queue_binding, "EVENTS"),
      queueName: asStr(cf.queue_name, "github-events"),
      telemetryR2Key: asStr(cf.telemetry_r2_key, "fleet/telemetry.json"),
      beaconTokenEnv: asStr(cf.beacon_token_env, "FLEET_BEACON_TOKEN"),
      workerUrlEnv: asStr(cf.worker_url_env, "APHRODY_WORKER_URL"),
      r2Buckets: {
        releases: asStr(cfBuckets.releases, "aphrody-releases"),
        assets: asStr(cfBuckets.assets, "aphrody-assets"),
      },
      endpoints: {
        releasesBase: asStr(cfEndpoints.releases_base, "https://releases.aphrody.com"),
        docsBase: asStr(cfEndpoints.docs_base, "https://docs.aphrody.com"),
        assetsBase: asStr(cfEndpoints.assets_base, "https://assets.aphrody.app"),
        health: asStr(cfEndpoints.health, "/healthz"),
        fleetStatus: asStr(cfEndpoints.fleet_status, "/api/fleet/status"),
        fleetRoute: asStr(cfEndpoints.fleet_route, "/api/fleet/route"),
        fleetBeacon: asStr(cfEndpoints.fleet_beacon, "/api/fleet/beacon"),
        githubWebhook: asStr(cfEndpoints.github_webhook, "/github"),
      },
    },
    nodes,
  };
}

function globMatches(pattern: string, value: string): boolean {
  const escaped = pattern.replace(/[.+^${}()|[\]\\]/g, "\\$&").replace(/\*/g, ".*").replace(/\?/g, ".");
  return new RegExp(`^${escaped}$`, "i").test(value);
}

function isIgnoredHostPattern(token: string, ignorePatterns: string[]): boolean {
  if (!token || token.startsWith("!") || token.includes("*") || token.includes("?")) return true;
  return ignorePatterns
    .filter((pattern) => pattern !== "*")
    .some((pattern) => globMatches(pattern, token));
}

export function parseSshConfigHosts(sshConfigText: string, manifest: FleetClusterManifest): DiscoveredSshNode[] {
  const ignore = manifest.discovery.ignoreHostPatterns;
  const discovered: DiscoveredSshNode[] = [];
  const seenCanonicals = new Set<string>();

  for (const rawLine of sshConfigText.split(/\r?\n/)) {
    const uncommented = rawLine.replace(/#.*$/, "").trim();
    if (!uncommented) continue;
    const match = /^Host(?:\s+|=)(.+)$/i.exec(uncommented);
    if (!match) continue;

    const tokens = match[1]
      .split(/\s+/)
      .map((t) => t.trim())
      .filter((t) => !isIgnoredHostPattern(t, ignore));
    if (tokens.length === 0) continue;

    let declaredMatch: DeclaredNodeConfig | undefined;
    for (const node of manifest.nodes) {
      if (tokens.some((tok) => node.sshAliases.includes(tok))) {
        declaredMatch = node;
        break;
      }
    }

    const preferredToken =
      declaredMatch?.sshAliases.find((a) => tokens.includes(a)) ?? declaredMatch?.alias ?? tokens[0];
    const canonicalAlias = declaredMatch?.alias ?? preferredToken;
    if (seenCanonicals.has(canonicalAlias)) continue;
    seenCanonicals.add(canonicalAlias);

    discovered.push({
      alias: canonicalAlias,
      sshTarget: preferredToken,
      aliasesInBlock: tokens,
      declared: declaredMatch,
      discoveredFromSshConfig: true,
    });
  }

  // Order declared manifest nodes in manifest order first, followed by newly discovered SSH hosts.
  const manifestOrder = new Map(manifest.nodes.map((n, idx) => [n.alias, idx]));
  return discovered.sort((a, b) => {
    const ia = manifestOrder.get(a.alias) ?? Number.MAX_SAFE_INTEGER;
    const ib = manifestOrder.get(b.alias) ?? Number.MAX_SAFE_INTEGER;
    if (ia !== ib) return ia - ib;
    return a.alias.localeCompare(b.alias);
  });
}

export function resolveSshConfigFile(manifest: FleetClusterManifest, explicitPath?: string): string {
  const envOverride = process.env[manifest.discovery.envSshConfigOverride]?.trim();
  const chosen = explicitPath?.trim() || envOverride || manifest.discovery.sshConfigPath;
  return expandTildePath(chosen);
}

export function discoverFleetNodes(
  manifest: FleetClusterManifest,
  options: {
    localOnly?: boolean;
    includeLocal?: boolean;
    hostsArg?: string;
    hostArg?: string;
    all?: boolean;
    sshConfigPath?: string;
  } = {},
): DiscoveredSshNode[] {
  const sshCfgFile = resolveSshConfigFile(manifest, options.sshConfigPath);
  const sshNodes = existsSync(sshCfgFile)
    ? parseSshConfigHosts(readFileSync(sshCfgFile, "utf8"), manifest)
    : [];
  const byAlias = new Map<string, DiscoveredSshNode>();
  for (const item of sshNodes) {
    byAlias.set(item.alias, item);
    for (const alt of item.aliasesInBlock) {
      if (!byAlias.has(alt)) byAlias.set(alt, item);
    }
  }
  const declaredByAlias = new Map<string, DeclaredNodeConfig>();
  for (const decl of manifest.nodes) {
    declaredByAlias.set(decl.alias, decl);
    for (const alt of decl.sshAliases) {
      if (!declaredByAlias.has(alt)) declaredByAlias.set(alt, decl);
    }
  }

  const makeTarget = (alias: string): DiscoveredSshNode => {
    if (alias === "local") {
      return {
        alias: "local",
        sshTarget: "local",
        aliasesInBlock: ["local"],
        discoveredFromSshConfig: false,
      };
    }
    const found = byAlias.get(alias);
    if (found) return found;
    const decl = declaredByAlias.get(alias);
    return {
      alias: decl?.alias ?? alias,
      sshTarget: alias,
      aliasesInBlock: [alias],
      declared: decl,
      discoveredFromSshConfig: false,
    };
  };

  if (options.localOnly) {
    const list = [makeTarget("local")];
    if (options.hostsArg) {
      for (const h of options.hostsArg.split(",").map((s) => s.trim()).filter(Boolean)) {
        if (h !== "local") list.push(makeTarget(h));
      }
    }
    return list;
  }

  const explicitListRaw =
    options.hostsArg ??
    (options.hostArg ? options.hostArg : undefined) ??
    (!options.all ? process.env[manifest.discovery.envHostsOverride] : undefined);

  if (explicitListRaw && explicitListRaw.trim().length > 0) {
    const seen = new Set<string>();
    const list: DiscoveredSshNode[] = [];
    for (const raw of explicitListRaw.split(",").map((s) => s.trim()).filter(Boolean)) {
      const node = makeTarget(raw);
      if (!seen.has(node.alias)) {
        seen.add(node.alias);
        list.push(node);
      }
    }
    if (options.includeLocal && !seen.has("local")) list.push(makeTarget("local"));
    return list;
  }

  // --all or default discovery mode:
  const results = [...sshNodes];
  if (results.length === 0 && manifest.discovery.includeLocalFallback) {
    results.push(makeTarget("local"));
  } else if (options.includeLocal && !results.some((r) => r.alias === "local")) {
    results.push(makeTarget("local"));
  }
  return results;
}

export function buildLinuxProbeScript(preferredWorkDir = "/srv/xbuild"): string {
  const safeWorkDir = /^[A-Za-z0-9_./~-]+$/.test(preferredWorkDir) ? preferredWorkDir : "/srv/xbuild";
  return [
    "set -u",
    "export LC_ALL=C",
    'export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$HOME/.bun/bin:/usr/local/bin:$PATH"',
    "HOSTNAME_STR=$(hostname 2>/dev/null || uname -n 2>/dev/null || echo unknown)",
    "OS_STR=$(uname -s 2>/dev/null || echo Linux)",
    "ARCH_STR=$(uname -m 2>/dev/null || echo unknown)",
    "KERNEL_STR=$(uname -r 2>/dev/null || echo unknown)",
    "CPUS=$(nproc 2>/dev/null || getconf _NPROCESSORS_ONLN 2>/dev/null || echo 1)",
    "LOADAVG=$(cat /proc/loadavg 2>/dev/null || echo '0.00 0.00 0.00 0/0 0')",
    "MEMTOTAL_KB=$(awk '/^MemTotal:/ {print $2; exit}' /proc/meminfo 2>/dev/null || echo 0)",
    "MEMAVAIL_KB=$(awk '/^MemAvailable:/ {print $2; exit}' /proc/meminfo 2>/dev/null || true)",
    'if [ -z "$MEMAVAIL_KB" ]; then',
    "  MEMFREE_KB=$(awk '/^MemFree:/ {print $2; exit}' /proc/meminfo 2>/dev/null || echo 0)",
    "  BUFFERS_KB=$(awk '/^Buffers:/ {print $2; exit}' /proc/meminfo 2>/dev/null || echo 0)",
    "  CACHED_KB=$(awk '/^Cached:/ {print $2; exit}' /proc/meminfo 2>/dev/null || echo 0)",
    "  MEMAVAIL_KB=$((MEMFREE_KB + BUFFERS_KB + CACHED_KB))",
    "fi",
    `WORK_DIR='${safeWorkDir}'`,
    'if [ ! -d "$WORK_DIR" ]; then WORK_DIR="$HOME"; fi',
    'if [ -d "$WORK_DIR" ]; then',
    '  WORK_AVAIL_KB=$(df -Pk "$WORK_DIR" 2>/dev/null | awk \'NR==2 {print $4}\' || echo 0)',
    '  if [ "${WORK_AVAIL_KB:-0}" -lt 10485760 ]; then',
    '    for CAND in /var/cache/aphrody "$HOME" /; do',
    '      if [ -d "$CAND" ]; then',
    '        CAND_KB=$(df -Pk "$CAND" 2>/dev/null | awk \'NR==2 {print $4}\' || echo 0)',
    '        if [ "${CAND_KB:-0}" -gt "${WORK_AVAIL_KB:-0}" ]; then WORK_DIR="$CAND"; WORK_AVAIL_KB="$CAND_KB"; fi',
    "      fi",
    "    done",
    "  fi",
    "else",
    "  WORK_DIR='/'",
    "fi",
    'DF_LINE=$(df -Pk "$WORK_DIR" 2>/dev/null | awk \'NR==2 {print $2, $4, $5}\' || echo "0 0 0%")',
    'FS_TYPE=$(stat -f -c \'%T\' "$WORK_DIR" 2>/dev/null || echo unknown)',
    "XFS_REFLINK=0",
    'if [ "$FS_TYPE" = "xfs" ] && command -v xfs_info >/dev/null 2>&1; then',
    '  if xfs_info "$WORK_DIR" 2>/dev/null | grep -q \'reflink=1\'; then XFS_REFLINK=1; fi',
    'elif [ "$FS_TYPE" = "btrfs" ]; then',
    "  XFS_REFLINK=1",
    "fi",
    "CARGO_PROCS=$(ps -eo comm= 2>/dev/null | awk '/^(cargo|rustc|cc1plus|lld|mold|wild|zig)$/{c++} END{print c+0}')",
    "CARGO_ROOTS=$(ps -eo comm= 2>/dev/null | awk '/^cargo$/{c++} END{print c+0}')",
    "RUNNER_WORKERS=$(ps -eo args= 2>/dev/null | awk '/Runner\\.Worker|actions\\.runner/ && !/awk/{c++} END{print c+0}')",
    "HAS_CARGO=$(command -v cargo >/dev/null 2>&1 && echo 1 || echo 0)",
    "HAS_WASM=0",
    "if command -v rustup >/dev/null 2>&1; then",
    "  if rustup target list --installed 2>/dev/null | grep -q 'wasm32'; then HAS_WASM=1; fi",
    "fi",
    "HAS_PROXY=0",
    "if command -v nginx >/dev/null 2>&1 || command -v caddy >/dev/null 2>&1 || command -v cloudflared >/dev/null 2>&1; then",
    "  HAS_PROXY=1",
    "fi",
    'echo "HOSTNAME=$HOSTNAME_STR"',
    'echo "OS=$OS_STR"',
    'echo "ARCH=$ARCH_STR"',
    'echo "KERNEL=$KERNEL_STR"',
    'echo "CPUS=$CPUS"',
    'echo "LOADAVG=$LOADAVG"',
    'echo "MEMTOTAL_KB=$MEMTOTAL_KB"',
    'echo "MEMAVAIL_KB=$MEMAVAIL_KB"',
    'echo "WORK_DIR=$WORK_DIR"',
    'echo "DF=$DF_LINE"',
    'echo "FS_TYPE=$FS_TYPE"',
    'echo "REFLINK=$XFS_REFLINK"',
    'echo "CARGO_PROCS=$CARGO_PROCS"',
    'echo "CARGO_ROOTS=$CARGO_ROOTS"',
    'echo "RUNNER_WORKERS=$RUNNER_WORKERS"',
    'echo "HAS_CARGO=$HAS_CARGO"',
    'echo "HAS_WASM=$HAS_WASM"',
    'echo "HAS_PROXY=$HAS_PROXY"',
  ].join("\n");
}

export function computeNodeScore(
  input: {
    reachable: boolean;
    cpus: number;
    load1: number;
    memTotalGb: number;
    memAvailGb: number;
    workAvailGb: number;
    activeJobs: number;
    rttMs: number;
    priorityBias?: number;
  },
  scoring: ScoringWeights,
): {
  score: number;
  healthy: boolean;
  saturated: boolean;
  loadRatio: number;
  healthReasons: string[];
} {
  const cpus = Math.max(1, input.cpus);
  const loadRatio = Number((input.load1 / cpus).toFixed(3));
  const healthReasons: string[] = [];

  if (!input.reachable) {
    return {
      score: 0,
      healthy: false,
      saturated: false,
      loadRatio,
      healthReasons: ["unreachable"],
    };
  }

  if (input.memAvailGb < scoring.minFreeMemGb) {
    healthReasons.push(`low_memory(${input.memAvailGb.toFixed(1)}G<${scoring.minFreeMemGb}G)`);
  }
  if (input.workAvailGb < scoring.minFreeDiskGb) {
    healthReasons.push(`low_disk(${input.workAvailGb.toFixed(1)}G<${scoring.minFreeDiskGb}G)`);
  }
  const saturated = loadRatio > scoring.maxLoadPerCpu;
  if (saturated) {
    healthReasons.push(`high_load(${loadRatio.toFixed(2)}>${scoring.maxLoadPerCpu})`);
  }

  const healthy = input.memAvailGb >= scoring.minFreeMemGb && input.workAvailGb >= scoring.minFreeDiskGb;

  const loadPenalty = clamp(loadRatio, 0, scoring.loadRatioCap) * scoring.loadPenaltyWeight;
  const memFreeRatio = input.memTotalGb > 0 ? clamp(input.memAvailGb / input.memTotalGb, 0, 1) : 1;
  const memPenalty = (1 - memFreeRatio) * scoring.memoryPenaltyWeight;
  const diskRatio = scoring.minWorkAvailGb > 0 ? clamp(input.workAvailGb / scoring.minWorkAvailGb, 0, 1) : 1;
  const diskPenalty = (1 - diskRatio) * scoring.diskPenaltyWeight;
  const jobPenalty = Math.max(0, input.activeJobs) * scoring.activeJobPenalty;
  const latencyPenalty =
    scoring.rttScaleMs > 0 ? clamp(input.rttMs / scoring.rttScaleMs, 0, 1) * scoring.latencyPenaltyWeight : 0;
  const saturationPenalty = saturated ? 18 : 0;
  const bias = input.priorityBias ?? 0;

  const rawScore =
    100 - loadPenalty - memPenalty - diskPenalty - jobPenalty - latencyPenalty - saturationPenalty + bias;
  const score = healthy ? round2(clamp(rawScore, 1, 100)) : 0;

  return {
    score,
    healthy,
    saturated,
    loadRatio,
    healthReasons,
  };
}

export function parseLinuxProbeOutput(
  node: DiscoveredSshNode,
  stdout: string,
  rttMs: number,
  scoring: ScoringWeights,
): NodeProbeTelemetry {
  const kv = new Map<string, string>();
  for (const line of stdout.split(/\r?\n/)) {
    const eq = line.indexOf("=");
    if (eq > 0) {
      kv.set(line.slice(0, eq).trim(), line.slice(eq + 1).trim());
    }
  }

  const cpus = Math.max(1, Number.parseInt(kv.get("CPUS") ?? "1", 10) || 1);
  const loadParts = (kv.get("LOADAVG") ?? "0 0 0").split(/\s+/);
  const load1 = Number.parseFloat(loadParts[0] ?? "0") || 0;
  const load5 = Number.parseFloat(loadParts[1] ?? "0") || 0;
  const load15 = Number.parseFloat(loadParts[2] ?? "0") || 0;

  const memTotalKb = Number.parseInt(kv.get("MEMTOTAL_KB") ?? "0", 10) || 0;
  const memAvailKb = Number.parseInt(kv.get("MEMAVAIL_KB") ?? "0", 10) || 0;
  const memTotalGb = round2(memTotalKb / (1024 * 1024));
  const memAvailGb = round2(memAvailKb / (1024 * 1024));

  const dfParts = (kv.get("DF") ?? "0 0 0%").split(/\s+/);
  const workTotalKb = Number.parseInt(dfParts[0] ?? "0", 10) || 0;
  const workAvailKb = Number.parseInt(dfParts[1] ?? "0", 10) || 0;
  const workUsePct = Number.parseInt((dfParts[2] ?? "0%").replace("%", ""), 10) || 0;
  const workTotalGb = round2(workTotalKb / (1024 * 1024));
  const workAvailGb = round2(workAvailKb / (1024 * 1024));

  const fsType = (kv.get("FS_TYPE") ?? "unknown").toLowerCase();
  const reflinkSupported = kv.get("REFLINK") === "1";

  const activeCompilerProcs = Number.parseInt(kv.get("CARGO_PROCS") ?? "0", 10) || 0;
  const cargoRoots = Number.parseInt(kv.get("CARGO_ROOTS") ?? "0", 10) || 0;
  const activeRunnerWorkers = Number.parseInt(kv.get("RUNNER_WORKERS") ?? "0", 10) || 0;
  const activeJobs = Math.max(cargoRoots + activeRunnerWorkers, activeCompilerProcs > 0 ? 1 : 0);

  const capabilities = new Set<string>(node.declared?.capabilities ?? []);
  if (fsType === "xfs" && reflinkSupported) capabilities.add("xfs_reflink");
  if (fsType === "btrfs") capabilities.add("btrfs_reflink");
  if (kv.get("HAS_CARGO") === "1") capabilities.add("native_rust");
  if (kv.get("HAS_WASM") === "1") capabilities.add("wasm");
  if (kv.get("HAS_PROXY") === "1") capabilities.add("edge_proxy");

  const priorityBias = node.declared?.priorityBias ?? 0;
  const scored = computeNodeScore(
    {
      reachable: true,
      cpus,
      load1,
      memTotalGb,
      memAvailGb,
      workAvailGb,
      activeJobs,
      rttMs,
      priorityBias,
    },
    scoring,
  );

  return {
    alias: node.alias,
    sshTarget: node.sshTarget,
    ...(node.declared?.region ? { region: node.declared.region } : {}),
    ...(node.declared?.tier ? { tier: node.declared.tier } : {}),
    priorityBias,
    reachable: true,
    healthy: scored.healthy,
    saturated: scored.saturated,
    healthReasons: scored.healthReasons,
    os: kv.get("OS") ?? "Linux",
    arch: kv.get("ARCH") ?? "unknown",
    kernel: kv.get("KERNEL") ?? "unknown",
    cpus,
    load1: round2(load1),
    load5: round2(load5),
    load15: round2(load15),
    loadRatio: scored.loadRatio,
    memTotalGb,
    memAvailGb,
    workDir: kv.get("WORK_DIR") ?? node.declared?.workDir ?? "/srv/xbuild",
    workTotalGb,
    workAvailGb,
    workUsePct,
    fsType,
    reflinkSupported,
    activeCompilerProcs,
    activeRunnerWorkers,
    activeJobs,
    rttMs: Math.round(rttMs),
    capabilities: [...capabilities].sort(),
    labels: node.declared?.labels ?? [],
    score: scored.score,
    reportedAt: new Date().toISOString(),
  };
}

function cpuTotalsSnapshot(): { idle: number; total: number } {
  let idle = 0;
  let total = 0;
  for (const cpu of os.cpus()) {
    const t = cpu.times;
    idle += t.idle;
    total += t.user + t.nice + t.sys + t.idle + t.irq;
  }
  return { idle, total };
}

async function sampleLocalLoad(cpus: number): Promise<[number, number, number]> {
  const avg = os.loadavg();
  if (process.platform !== "win32" || avg[0] > 0) {
    return [round2(avg[0]), round2(avg[1]), round2(avg[2])];
  }
  const t0 = cpuTotalsSnapshot();
  await Bun.sleep(40);
  const t1 = cpuTotalsSnapshot();
  const deltaTotal = Math.max(1, t1.total - t0.total);
  const deltaIdle = Math.max(0, t1.idle - t0.idle);
  const busyRatio = clamp(1 - deltaIdle / deltaTotal, 0, 1);
  const est = round2(busyRatio * cpus);
  return [est, est, est];
}

function readLocalLinuxMemAvailGb(): number | null {
  if (process.platform !== "linux" || !existsSync("/proc/meminfo")) return null;
  try {
    const raw = readFileSync("/proc/meminfo", "utf8");
    const m = /^MemAvailable:\s+(\d+)\s+kB/m.exec(raw);
    if (m) return round2(Number.parseInt(m[1], 10) / (1024 * 1024));
  } catch {
    return null;
  }
  return null;
}

export function parseMacosVmStatAvailGb(raw: string): number | null {
  const pageMatch = /page size of (\d+) bytes/i.exec(raw);
  const pageSize = pageMatch ? Number.parseInt(pageMatch[1], 10) : 16384;
  const grab = (label: string): number => {
    const re = new RegExp(`^${label}:\\s+(\\d+)\\.?`, "mi");
    const m = re.exec(raw);
    return m ? Number.parseInt(m[1], 10) : 0;
  };
  const pages =
    grab("Pages free") +
    grab("Pages inactive") +
    grab("Pages speculative") +
    grab("Pages purgeable");
  if (pages <= 0) return null;
  return round2((pages * pageSize) / 1024 ** 3);
}

function readLocalMacosMemAvailGb(): number | null {
  if (process.platform !== "darwin") return null;
  try {
    const res = Bun.spawnSync(["vm_stat"], { stdout: "pipe", stderr: "pipe", timeout: 2000 });
    if (res.exitCode !== 0) return null;
    return parseMacosVmStatAvailGb(new TextDecoder().decode(res.stdout));
  } catch {
    return null;
  }
}

export async function probeLocalHost(manifest: FleetClusterManifest): Promise<NodeProbeTelemetry> {
  const t0 = performance.now();
  const cpuList = os.cpus();
  const cpus = Math.max(1, cpuList.length);
  const [load1, load5, load15] = await sampleLocalLoad(cpus);

  const memTotalGb = round2(os.totalmem() / 1024 ** 3);
  const memAvailGb =
    readLocalLinuxMemAvailGb() ??
    readLocalMacosMemAvailGb() ??
    round2(os.freemem() / 1024 ** 3);

  let workTotalGb = 0;
  let workAvailGb = 0;
  let workUsePct = 0;
  try {
    const fsStats = statfsSync(repoRoot);
    const bsize = Number(fsStats.bsize || 4096);
    workTotalGb = round2((Number(fsStats.blocks) * bsize) / 1024 ** 3);
    workAvailGb = round2((Number(fsStats.bavail) * bsize) / 1024 ** 3);
    workUsePct = workTotalGb > 0 ? Math.round(((workTotalGb - workAvailGb) / workTotalGb) * 100) : 0;
  } catch {
    workTotalGb = 0;
    workAvailGb = 0;
  }

  const capabilities = new Set<string>();
  if (Bun.which("cargo") || Bun.which("rustc")) {
    capabilities.add("native_rust");
  }
  if (Bun.which("rustup")) {
    const targets = Bun.spawnSync(["rustup", "target", "list", "--installed"], {
      stdout: "pipe",
      stderr: "pipe",
      timeout: 3000,
    });
    if (targets.exitCode === 0 && new TextDecoder().decode(targets.stdout).includes("wasm32")) {
      capabilities.add("wasm");
    }
  }

  const rttMs = Math.max(0, Math.round(performance.now() - t0));
  const scored = computeNodeScore(
    {
      reachable: true,
      cpus,
      load1,
      memTotalGb,
      memAvailGb,
      workAvailGb,
      activeJobs: 0,
      rttMs,
      priorityBias: 0,
    },
    manifest.scoring,
  );

  return {
    alias: "local",
    sshTarget: "local",
    region: "local",
    tier: "workstation",
    priorityBias: 0,
    reachable: true,
    healthy: scored.healthy,
    saturated: scored.saturated,
    healthReasons: scored.healthReasons,
    os: os.platform(),
    arch: os.arch(),
    kernel: os.release(),
    cpus,
    load1,
    load5,
    load15,
    loadRatio: scored.loadRatio,
    memTotalGb,
    memAvailGb,
    workDir: repoRoot,
    workTotalGb,
    workAvailGb,
    workUsePct,
    fsType: process.platform === "win32" ? "ntfs" : process.platform === "darwin" ? "apfs" : "local",
    reflinkSupported: process.platform === "darwin",
    activeCompilerProcs: 0,
    activeRunnerWorkers: 0,
    activeJobs: 0,
    rttMs,
    capabilities: [...capabilities].sort(),
    labels: ["local", os.platform(), os.arch()],
    score: scored.score,
    reportedAt: new Date().toISOString(),
  };
}

function buildSshBaseArgs(manifest: FleetClusterManifest, sshConfigPath?: string): string[] {
  const args = ["-T"];
  if (sshConfigPath && existsSync(sshConfigPath)) {
    args.push("-F", sshConfigPath);
  }
  if (manifest.discovery.sshBatchMode) {
    args.push("-o", "BatchMode=yes");
  }
  args.push("-o", `ConnectTimeout=${manifest.discovery.sshConnectTimeoutSec}`);
  return args;
}

function unreachableNodeReport(node: DiscoveredSshNode, rttMs: number, error: string): NodeProbeTelemetry {
  return {
    alias: node.alias,
    sshTarget: node.sshTarget,
    ...(node.declared?.region ? { region: node.declared.region } : {}),
    ...(node.declared?.tier ? { tier: node.declared.tier } : {}),
    priorityBias: node.declared?.priorityBias ?? 0,
    reachable: false,
    healthy: false,
    saturated: false,
    healthReasons: ["unreachable"],
    os: "unknown",
    arch: "unknown",
    kernel: "unknown",
    cpus: 0,
    load1: 0,
    load5: 0,
    load15: 0,
    loadRatio: 0,
    memTotalGb: 0,
    memAvailGb: 0,
    workDir: node.declared?.workDir ?? "/srv/xbuild",
    workTotalGb: 0,
    workAvailGb: 0,
    workUsePct: 0,
    fsType: "unknown",
    reflinkSupported: false,
    activeCompilerProcs: 0,
    activeRunnerWorkers: 0,
    activeJobs: 0,
    rttMs: Math.round(rttMs),
    capabilities: node.declared?.capabilities ?? [],
    labels: node.declared?.labels ?? [],
    score: 0,
    reportedAt: new Date().toISOString(),
    error,
  };
}

export async function probeRemoteHostAsync(
  node: DiscoveredSshNode,
  manifest: FleetClusterManifest,
  sshConfigPath?: string,
): Promise<NodeProbeTelemetry> {
  const script = buildLinuxProbeScript(node.declared?.workDir ?? "/srv/xbuild");
  const sshArgs = [...buildSshBaseArgs(manifest, sshConfigPath), node.sshTarget, "tr -d '\\r' | bash -s"];
  const started = performance.now();

  const proc = Bun.spawn(["ssh", ...sshArgs], {
    stdin: Buffer.from(`${script}\n`, "utf8"),
    stdout: "pipe",
    stderr: "pipe",
  });

  let timedOut = false;
  const timer = setTimeout(() => {
    timedOut = true;
    try {
      proc.kill();
    } catch {
      // Ignore already exited process.
    }
  }, manifest.discovery.sshProbeTimeoutMs);

  try {
    const [exitCode, stdout, stderr] = await Promise.all([
      proc.exited,
      new Response(proc.stdout).text(),
      new Response(proc.stderr).text(),
    ]);
    const rttMs = performance.now() - started;
    if (timedOut) {
      return unreachableNodeReport(node, rttMs, `SSH probe timeout (${manifest.discovery.sshProbeTimeoutMs}ms)`);
    }
    if (exitCode !== 0 || !stdout.includes("CPUS=")) {
      const msg = stderr.trim() || `SSH exit ${exitCode}`;
      return unreachableNodeReport(node, rttMs, msg.slice(0, 300));
    }
    return parseLinuxProbeOutput(node, stdout, rttMs, manifest.scoring);
  } catch (err) {
    return unreachableNodeReport(node, performance.now() - started, String(err));
  } finally {
    clearTimeout(timer);
  }
}

export async function probeFleetAsync(
  nodes: DiscoveredSshNode[],
  manifest: FleetClusterManifest,
  sshConfigPath?: string,
): Promise<NodeProbeTelemetry[]> {
  const results = await Promise.all(
    nodes.map((node) =>
      node.alias === "local" ? probeLocalHost(manifest) : probeRemoteHostAsync(node, manifest, sshConfigPath),
    ),
  );
  return results.sort((a, b) => b.score - a.score || a.alias.localeCompare(b.alias));
}

export function scheduleWorkloads(
  nodes: NodeProbeTelemetry[],
  manifest: FleetClusterManifest,
  options: {
    jobKind?: string;
    shards?: number;
    preferredRegion?: string;
    extraCapabilities?: string[];
  } = {},
): SchedulePlan {
  const jobKind = (options.jobKind ?? "native_rust").trim().toLowerCase();
  const jobSpec: JobProfile = manifest.jobs[jobKind] ?? {
    kind: jobKind,
    requiredCapabilities: [],
    preferredCapabilities: [],
    defaultShards: 1,
    minFreeMemGb: manifest.scoring.minFreeMemGb,
    minFreeDiskGb: manifest.scoring.minFreeDiskGb,
  };

  const requiredCapabilities = [...new Set([...jobSpec.requiredCapabilities, ...(options.extraCapabilities ?? [])])];
  const preferredCapabilities = jobSpec.preferredCapabilities;
  const shardsRequested = Math.max(1, Math.floor(options.shards ?? jobSpec.defaultShards));

  const eligible: Array<{ node: NodeProbeTelemetry; effectiveScore: number }> = [];
  const rejected: Array<{ alias: string; reasons: string[] }> = [];

  for (const node of nodes) {
    const reasons: string[] = [];
    if (!node.reachable) reasons.push(node.error ? `unreachable:${node.error}` : "unreachable");
    if (node.reachable && !node.healthy) reasons.push(...node.healthReasons);
    if (node.reachable && node.memAvailGb < jobSpec.minFreeMemGb) {
      reasons.push(`job_mem(${node.memAvailGb.toFixed(1)}G<${jobSpec.minFreeMemGb}G)`);
    }
    if (node.reachable && node.workAvailGb < jobSpec.minFreeDiskGb) {
      reasons.push(`job_disk(${node.workAvailGb.toFixed(1)}G<${jobSpec.minFreeDiskGb}G)`);
    }
    if (requiredCapabilities.length > 0) {
      const capSet = new Set(node.capabilities);
      const missing = requiredCapabilities.filter((cap) => !capSet.has(cap));
      if (missing.length > 0) {
        reasons.push(`missing_capabilities(${missing.join(",")})`);
      }
    }

    if (reasons.length > 0) {
      rejected.push({ alias: node.alias, reasons });
      continue;
    }

    const capSet = new Set(node.capabilities);
    const preferredCount = preferredCapabilities.filter((p) => capSet.has(p)).length;
    const roleBonus =
      preferredCapabilities.length > 0 ? (preferredCount / preferredCapabilities.length) * manifest.scoring.roleAffinityBonus : 0;
    const regionBonus =
      options.preferredRegion && node.region === options.preferredRegion ? manifest.scoring.regionAffinityBonus : 0;

    const effectiveScore = round2(clamp(node.score + roleBonus + regionBonus, 0, 100));
    eligible.push({ node, effectiveScore });
  }

  // Prefer unsaturated nodes over saturated nodes, then highest effectiveScore, lowest loadRatio, lowest rttMs.
  eligible.sort((a, b) => {
    if (a.node.saturated !== b.node.saturated) return a.node.saturated ? 1 : -1;
    if (b.effectiveScore !== a.effectiveScore) return b.effectiveScore - a.effectiveScore;
    if (a.node.loadRatio !== b.node.loadRatio) return a.node.loadRatio - b.node.loadRatio;
    if (a.node.rttMs !== b.node.rttMs) return a.node.rttMs - b.node.rttMs;
    return a.node.alias.localeCompare(b.node.alias);
  });

  const assignments: ShardAssignment[] = [];
  if (eligible.length > 0) {
    const virtualAssigned = new Map<string, number>();
    for (let shard = 0; shard < shardsRequested; shard++) {
      let best = eligible[0];
      let bestMarginal = -Infinity;
      for (const item of eligible) {
        const assigned = virtualAssigned.get(item.node.alias) ?? 0;
        const cpuFactor = 1 + 2 / Math.max(2, item.node.cpus);
        const shardWeight = 24 * cpuFactor;
        const marginal = item.effectiveScore - assigned * shardWeight;
        if (marginal > bestMarginal) {
          bestMarginal = marginal;
          best = item;
        }
      }
      virtualAssigned.set(best.node.alias, (virtualAssigned.get(best.node.alias) ?? 0) + 1);
      assignments.push({
        shard,
        host: best.node.alias,
        effectiveScore: round2(clamp(bestMarginal, 0, 100)),
        ...(best.node.region ? { region: best.node.region } : {}),
        capabilities: best.node.capabilities,
      });
    }
  }

  return {
    ok: eligible.length > 0,
    job: jobKind,
    shardsRequested,
    requiredCapabilities,
    preferredCapabilities,
    primaryHost: eligible[0]?.node.alias ?? null,
    fallbackChain: eligible.slice(1).map((e) => e.node.alias),
    assignments,
    rankedCandidates: eligible.map((e) => ({
      alias: e.node.alias,
      effectiveScore: e.effectiveScore,
      loadRatio: e.node.loadRatio,
      memAvailGb: e.node.memAvailGb,
      workAvailGb: e.node.workAvailGb,
      activeJobs: e.node.activeJobs,
      capabilities: e.node.capabilities,
    })),
    rejected,
  };
}

export type HostCommandExecutor = (
  candidate: { alias: string; sshTarget: string; score: number },
  command: string,
) => Promise<{ exitCode: number; stdout: string; stderr: string; durationMs: number }>;

export async function dispatchWithWorkStealing(
  candidates: Array<{ alias: string; sshTarget: string; score: number }>,
  command: string,
  executor: HostCommandExecutor,
  maxRetries = 2,
  dryRun = false,
): Promise<DispatchResult> {
  if (candidates.length === 0) {
    return {
      ok: false,
      command,
      dryRun,
      executedHost: null,
      fallbackChain: [],
      attempts: [],
      exitCode: 1,
      stdout: "",
      stderr: "No eligible healthy hosts available for dispatch",
    };
  }

  if (dryRun) {
    return {
      ok: true,
      command,
      dryRun: true,
      executedHost: candidates[0].alias,
      fallbackChain: candidates.slice(1).map((c) => c.alias),
      attempts: [],
      exitCode: 0,
      stdout: "",
      stderr: "",
    };
  }

  const maxAttempts = Math.min(candidates.length, Math.max(1, maxRetries + 1));
  const attempts: DispatchAttempt[] = [];
  let previousFailedHost: string | undefined;

  for (let idx = 0; idx < maxAttempts; idx++) {
    const candidate = candidates[idx];
    const execRes = await executor(candidate, command);
    const attempt: DispatchAttempt = {
      host: candidate.alias,
      score: candidate.score,
      exitCode: execRes.exitCode,
      durationMs: Math.round(execRes.durationMs),
      ok: execRes.exitCode === 0,
      ...(previousFailedHost ? { stolenFrom: previousFailedHost } : {}),
      stdout: execRes.stdout,
      stderr: execRes.stderr,
    };
    attempts.push(attempt);
    if (execRes.exitCode === 0) {
      return {
        ok: true,
        command,
        dryRun: false,
        executedHost: candidate.alias,
        fallbackChain: candidates.slice(idx + 1).map((c) => c.alias),
        attempts,
        exitCode: 0,
        stdout: execRes.stdout,
        stderr: execRes.stderr,
      };
    }
    previousFailedHost = candidate.alias;
  }

  const last = attempts[attempts.length - 1];
  return {
    ok: false,
    command,
    dryRun: false,
    executedHost: null,
    fallbackChain: [],
    attempts,
    exitCode: last?.exitCode ?? 1,
    stdout: last?.stdout ?? "",
    stderr: last?.stderr ?? "All dispatch attempts failed",
  };
}

async function defaultCommandExecutor(
  candidate: { alias: string; sshTarget: string; score: number },
  command: string,
  manifest: FleetClusterManifest,
  sshConfigPath?: string,
): Promise<{ exitCode: number; stdout: string; stderr: string; durationMs: number }> {
  const started = performance.now();
  const spawnCmd =
    candidate.alias === "local"
      ? process.platform === "win32"
        ? ["cmd", "/d", "/c", command]
        : ["bash", "-lc", command]
      : ["ssh", ...buildSshBaseArgs(manifest, sshConfigPath), candidate.sshTarget, "tr -d '\\r' | bash -s"];

  const proc = Bun.spawn(spawnCmd, {
    stdin: candidate.alias === "local" ? "ignore" : Buffer.from(`set -euo pipefail\n${command}\n`, "utf8"),
    stdout: "pipe",
    stderr: "pipe",
  });

  const [exitCode, stdout, stderr] = await Promise.all([
    proc.exited,
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
  ]);
  return {
    exitCode,
    stdout,
    stderr,
    durationMs: performance.now() - started,
  };
}

export async function signBeaconBody(secret: string, bodyText: string): Promise<string> {
  const enc = new TextEncoder();
  const key = await crypto.subtle.importKey("raw", enc.encode(secret), { name: "HMAC", hash: "SHA-256" }, false, [
    "sign",
  ]);
  const sig = new Uint8Array(await crypto.subtle.sign("HMAC", key, enc.encode(bodyText)));
  return `sha256=${[...sig].map((b) => b.toString(16).padStart(2, "0")).join("")}`;
}

export async function publishBeacon(
  nodes: NodeProbeTelemetry[],
  manifest: FleetClusterManifest,
  options: {
    workerUrl?: string;
    token?: string;
    authMode?: "auto" | "hmac" | "bearer" | "none";
    merge?: boolean;
    dryRun?: boolean;
  } = {},
): Promise<{
  ok: boolean;
  dryRun: boolean;
  endpoint: string;
  authMode: string;
  signed: boolean;
  status?: number;
  workerResponse?: unknown;
  payload: Record<string, unknown>;
}> {
  const baseUrl = (
    options.workerUrl ??
    process.env[manifest.cloudflare.workerUrlEnv] ??
    manifest.cloudflare.endpoints.releasesBase
  ).replace(/\/$/, "");
  const endpoint = `${baseUrl}${manifest.cloudflare.endpoints.fleetBeacon}`;
  const token = (options.token ?? process.env[manifest.cloudflare.beaconTokenEnv] ?? "").trim();
  const requestedAuth = options.authMode ?? "auto";

  const payload = {
    schemaVersion: manifest.schemaVersion,
    generatedAt: new Date().toISOString(),
    sourceHost: os.hostname(),
    sourcePlatform: os.platform(),
    ttlSec: manifest.scoring.staleTelemetryTtlSec,
    merge: options.merge === true,
    nodes: nodes.map((n) => ({
      alias: n.alias,
      region: n.region,
      tier: n.tier,
      reachable: n.reachable,
      healthy: n.healthy,
      os: n.os,
      arch: n.arch,
      cpus: n.cpus,
      load1: n.load1,
      load5: n.load5,
      load15: n.load15,
      loadRatio: n.loadRatio,
      memTotalGb: n.memTotalGb,
      memAvailGb: n.memAvailGb,
      workAvailGb: n.workAvailGb,
      activeJobs: n.activeJobs,
      rttMs: n.rttMs,
      capabilities: n.capabilities,
      labels: n.labels,
      score: n.score,
      reportedAt: n.reportedAt,
    })),
  };

  const bodyText = JSON.stringify(payload);
  const headers: Record<string, string> = {
    "Content-Type": "application/json; charset=utf-8",
  };

  let effectiveAuthMode = "unauthenticated";
  let signed = false;
  if (token && requestedAuth !== "none") {
    if (requestedAuth === "bearer") {
      headers.Authorization = `Bearer ${token}`;
      effectiveAuthMode = "bearer";
    } else if (requestedAuth === "hmac") {
      headers["X-Fleet-Signature-256"] = await signBeaconBody(token, bodyText);
      effectiveAuthMode = "hmac-sha256";
      signed = true;
    } else {
      headers["X-Fleet-Signature-256"] = await signBeaconBody(token, bodyText);
      headers.Authorization = `Bearer ${token}`;
      effectiveAuthMode = "hmac-sha256+bearer";
      signed = true;
    }
  }

  if (options.dryRun) {
    return {
      ok: true,
      dryRun: true,
      endpoint,
      authMode: effectiveAuthMode,
      signed,
      payload,
    };
  }

  const res = await fetch(endpoint, {
    method: "POST",
    headers,
    body: bodyText,
    signal: AbortSignal.timeout(15_000),
  });
  const text = await res.text();
  let parsed: unknown = text;
  try {
    parsed = JSON.parse(text);
  } catch {
    // Preserve raw text when server returns non-JSON error.
  }

  return {
    ok: res.ok,
    dryRun: false,
    endpoint,
    authMode: effectiveAuthMode,
    signed,
    status: res.status,
    workerResponse: parsed,
    payload,
  };
}

function getOpt(args: string[], flag: string): string | undefined {
  const idx = args.indexOf(flag);
  if (idx >= 0 && idx + 1 < args.length) return args[idx + 1];
  const eq = args.find((a) => a.startsWith(`${flag}=`));
  return eq ? eq.slice(flag.length + 1) : undefined;
}

function formatProbeTable(nodes: NodeProbeTelemetry[]): string {
  const lines = ["HOST     STATUS     SCORE   LOAD1/CPU       MEM_FREE      DISK_FREE     JOBS  RTT    CAPABILITIES"];
  for (const n of nodes) {
    const status = !n.reachable ? "OFFLINE" : n.saturated ? "BUSY" : n.healthy ? "HEALTHY" : "UNHEALTHY";
    const load = `${n.load1.toFixed(2)}/${n.cpus} (${n.loadRatio.toFixed(2)})`;
    const mem = `${n.memAvailGb.toFixed(1)}/${n.memTotalGb.toFixed(1)}G`;
    const disk = `${n.workAvailGb.toFixed(1)}G`;
    const caps = n.capabilities.join(",");
    lines.push(
      `${n.alias.padEnd(8)} ${status.padEnd(10)} ${String(n.score).padStart(5)}   ${load.padEnd(14)}  ${mem.padEnd(12)}  ${disk.padEnd(12)}  ${String(n.activeJobs).padStart(4)}  ${String(n.rttMs).padStart(4)}ms ${caps}`,
    );
  }
  return lines.join("\n");
}

if (import.meta.main) {
  const args = Bun.argv.slice(2);
  const sub = args[0] ?? "probe";
  const manifest = loadFleetManifest(repoRoot);
  const jsonMode = args.includes("--json");
  const dryRun = args.includes("--dry-run");
  const localOnly = args.includes("--local");
  const includeLocal = args.includes("--include-local");
  const allFlag = args.includes("--all");
  const strictFlag = args.includes("--strict");
  const hostsArg = getOpt(args, "--hosts");
  const hostArg = getOpt(args, "--host");
  const sshConfigOverride = getOpt(args, "--ssh-config");
  const resolvedSshConfig = resolveSshConfigFile(manifest, sshConfigOverride);

  const targetNodes = discoverFleetNodes(manifest, {
    localOnly,
    includeLocal,
    hostsArg,
    hostArg,
    all: allFlag,
    sshConfigPath: resolvedSshConfig,
  });

  if (sub === "probe") {
    const nodes = await probeFleetAsync(targetNodes, manifest, resolvedSshConfig);
    const healthyCount = nodes.filter((n) => n.reachable && n.healthy).length;
    const ok = strictFlag ? nodes.every((n) => n.reachable && n.healthy) : healthyCount > 0;
    if (jsonMode) {
      console.log(
        JSON.stringify(
          {
            ok,
            generatedAt: new Date().toISOString(),
            schemaVersion: manifest.schemaVersion,
            totalNodes: nodes.length,
            healthyNodes: healthyCount,
            nodes,
          },
          null,
          2,
        ),
      );
    } else {
      console.log(formatProbeTable(nodes));
    }
    process.exit(ok ? 0 : 1);
  } else if (sub === "schedule") {
    const jobKind = getOpt(args, "--job") ?? "native_rust";
    const shardsRaw = getOpt(args, "--shards");
    const shards = shardsRaw ? Math.max(1, Number.parseInt(shardsRaw, 10) || 1) : undefined;
    const region = getOpt(args, "--region");
    const extraCapsRaw = getOpt(args, "--capabilities") ?? getOpt(args, "--capability");
    const extraCapabilities = extraCapsRaw
      ? extraCapsRaw
          .split(",")
          .map((s) => s.trim())
          .filter(Boolean)
      : undefined;

    const fromFile = getOpt(args, "--from-file");
    const nodes = fromFile
      ? (JSON.parse(readFileSync(fromFile, "utf8")).nodes as NodeProbeTelemetry[])
      : await probeFleetAsync(targetNodes, manifest, resolvedSshConfig);

    const plan = scheduleWorkloads(nodes, manifest, {
      jobKind,
      shards,
      preferredRegion: region,
      extraCapabilities,
    });
    if (jsonMode) {
      console.log(JSON.stringify(plan, null, 2));
    } else {
      console.log(
        `Job=${plan.job} Primary=${plan.primaryHost ?? "none"} Fallbacks=[${plan.fallbackChain.join(",")}] Shards=${plan.assignments.map((a) => `${a.shard}:${a.host}`).join(",")}`,
      );
    }
    process.exit(plan.ok ? 0 : 1);
  } else if (sub === "dispatch") {
    const dashDashIdx = args.indexOf("--");
    const cmdFlag = getOpt(args, "--cmd");
    const command = cmdFlag ?? (dashDashIdx >= 0 ? args.slice(dashDashIdx + 1).join(" ") : "");
    if (!command.trim()) {
      console.error("Missing --cmd <command> for dispatch");
      process.exit(1);
    }

    const jobKind = getOpt(args, "--job");
    const maxRetriesRaw = getOpt(args, "--max-retries");
    const maxRetries = maxRetriesRaw
      ? Math.max(0, Number.parseInt(maxRetriesRaw, 10) || 0)
      : manifest.scoring.workStealingMaxRetries;

    const probed = await probeFleetAsync(targetNodes, manifest, resolvedSshConfig);
    let orderedAliases: Array<{ alias: string; sshTarget: string; score: number }>;
    if (jobKind) {
      const plan = scheduleWorkloads(probed, manifest, { jobKind, shards: 1 });
      const byAlias = new Map(probed.map((p) => [p.alias, p]));
      orderedAliases = plan.rankedCandidates.map((c) => ({
        alias: c.alias,
        sshTarget: byAlias.get(c.alias)?.sshTarget ?? c.alias,
        score: c.effectiveScore,
      }));
    } else {
      orderedAliases = probed
        .filter((n) => n.reachable && n.healthy)
        .map((n) => ({ alias: n.alias, sshTarget: n.sshTarget, score: n.score }));
    }

    const result = await dispatchWithWorkStealing(
      orderedAliases,
      command,
      (cand, cmd) => defaultCommandExecutor(cand, cmd, manifest, resolvedSshConfig),
      maxRetries,
      dryRun,
    );

    if (jsonMode) {
      console.log(JSON.stringify(result, null, 2));
    } else {
      console.log(
        `dispatch ok=${result.ok} host=${result.executedHost ?? "none"} attempts=${result.attempts.length} exit=${result.exitCode}`,
      );
      if (result.stdout) process.stdout.write(result.stdout);
      if (result.stderr) process.stderr.write(result.stderr);
    }
    process.exit(result.ok ? 0 : result.exitCode || 1);
  } else if (sub === "beacon") {
    const workerUrl = getOpt(args, "--worker-url");
    const token = getOpt(args, "--token");
    const authOpt = (getOpt(args, "--auth") ?? "auto") as "auto" | "hmac" | "bearer" | "none";
    const merge = args.includes("--merge");
    const nodes = await probeFleetAsync(targetNodes, manifest, resolvedSshConfig);
    const result = await publishBeacon(nodes, manifest, {
      workerUrl,
      token,
      authMode: authOpt,
      merge,
      dryRun,
    });

    if (jsonMode || dryRun) {
      console.log(JSON.stringify(result, null, 2));
    } else {
      console.log(
        `beacon ok=${result.ok} endpoint=${result.endpoint} status=${result.status ?? "n/a"} auth=${result.authMode} nodes=${nodes.length}`,
      );
    }
    process.exit(result.ok ? 0 : 1);
  } else {
    console.error(
      "Usage: bun tooling/bun/fleet-lb.ts [probe|schedule|dispatch|beacon] [--all|--hosts h1,h2|--local] [--job <kind>] [--shards N] [--cmd <...>] [--worker-url <url>] [--dry-run] [--json]",
    );
    process.exit(1);
  }
}
