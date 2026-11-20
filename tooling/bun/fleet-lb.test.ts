import { describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import worker, { type WorkerEnv } from "../../.cloudflare/aphrody-worker/src/index.ts";
import {
  buildLinuxProbeScript,
  computeNodeScore,
  discoverFleetNodes,
  dispatchWithWorkStealing,
  loadFleetManifest,
  type NodeProbeTelemetry,
  parseLinuxProbeOutput,
  parseMacosVmStatAvailGb,
  parseSshConfigHosts,
  probeLocalHost,
  publishBeacon,
  repoRoot,
  scheduleWorkloads,
  signBeaconBody,
} from "./fleet-lb.ts";

describe("fleet-lb declarative cluster manifest, SSH discovery & dynamic load balancing", () => {
  test("loads assets/os/fleet_cluster.toml without hardcoded IPs, usernames, or account IDs", () => {
    const rawToml = readFileSync(join(repoRoot, "assets/os/fleet_cluster.toml"), "utf8");
    expect(rawToml).not.toMatch(/\b\d{1,3}(?:\.\d{1,3}){3}\b/);
    expect(rawToml).not.toMatch(/[A-Za-z]:\\Users\\/i);
    expect(rawToml).not.toMatch(/\/home\/[a-z0-9_-]+/i);

    const manifest = loadFleetManifest(repoRoot);
    expect(manifest.schemaVersion).toBe(1);
    expect(manifest.supportedCapabilities).toEqual([
      "xfs_reflink",
      "btrfs_reflink",
      "wasm",
      "native_rust",
      "edge_proxy",
    ]);
    expect(manifest.cloudflare.r2Buckets).toEqual({
      releases: "aphrody-releases",
      assets: "aphrody-assets",
    });
    expect(manifest.cloudflare.queueName).toBe("github-events");
    expect(manifest.cloudflare.endpoints.fleetBeacon).toBe("/api/fleet/beacon");
    expect(manifest.cloudflare.endpoints.fleetRoute).toBe("/api/fleet/route");
    expect(manifest.cloudflare.endpoints.fleetStatus).toBe("/api/fleet/status");
    expect(manifest.nodes.map((n) => n.alias)).toEqual(["fr1", "fr2", "prd"]);
  });

  test("discovers SSH aliases dynamically from ~/.ssh/config and deduplicates multi-alias Host blocks", () => {
    const manifest = loadFleetManifest(repoRoot);
    const sampleSshConfig = `
# Git forge should be ignored
Host git github.com
  HostName github.com

Host f1p vps fr1
  HostName node1.example.internal

Host f2p cdn fr2
  HostName node2.example.internal

Host usp prd
  HostName edge.example.internal

# Dynamically added extra compute node not in fleet_cluster.toml
Host eu3 eu3-alias
  HostName node3.example.internal

Host *
  ServerAliveInterval 60
`;
    const discovered = parseSshConfigHosts(sampleSshConfig, manifest);
    expect(discovered.map((d) => d.alias)).toEqual(["fr1", "fr2", "prd", "eu3"]);
    expect(discovered[0]?.declared?.tier).toBe("builder");
    expect(discovered[3]?.declared).toBeUndefined();
    expect(discovered[3]?.sshTarget).toBe("eu3");

    const dir = mkdtempSync(join(tmpdir(), "aphrody-fleet-ssh-"));
    try {
      const cfgPath = join(dir, "config");
      writeFileSync(cfgPath, sampleSshConfig, "utf8");
      const allNodes = discoverFleetNodes(manifest, { all: true, sshConfigPath: cfgPath });
      expect(allNodes.map((n) => n.alias)).toEqual(["fr1", "fr2", "prd", "eu3"]);

      const explicit = discoverFleetNodes(manifest, { hostsArg: "prd,local", sshConfigPath: cfgPath });
      expect(explicit.map((n) => n.alias)).toEqual(["prd", "local"]);

      const missingFallback = discoverFleetNodes(manifest, {
        all: true,
        sshConfigPath: join(dir, "nonexistent-config"),
      });
      expect(missingFallback.map((n) => n.alias)).toEqual(["local"]);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test("parses Linux /proc/loadavg, /proc/meminfo, nproc, df, and active processes cleanly", () => {
    const manifest = loadFleetManifest(repoRoot);
    const script = buildLinuxProbeScript("/srv/xbuild");
    expect(script).toContain("/proc/loadavg");
    expect(script).toContain("/proc/meminfo");
    expect(script).toContain("nproc");
    expect(script).toContain("df -Pk");

    const discovered = {
      alias: "fr1",
      sshTarget: "fr1",
      aliasesInBlock: ["fr1"],
      declared: manifest.nodes[0],
      discoveredFromSshConfig: true,
    };

    const sampleOut = [
      "HOSTNAME=builder-01",
      "OS=Linux",
      "ARCH=x86_64",
      "KERNEL=7.3.0-generic",
      "CPUS=16",
      "LOADAVG=2.40 1.90 1.50 3/612 4210",
      "MEMTOTAL_KB=67108864",
      "MEMAVAIL_KB=50331648",
      "WORK_DIR=/srv/xbuild",
      "DF=209715200 157286400 25%",
      "FS_TYPE=xfs",
      "REFLINK=1",
      "CARGO_PROCS=5",
      "CARGO_ROOTS=1",
      "RUNNER_WORKERS=1",
      "HAS_CARGO=1",
      "HAS_WASM=1",
      "HAS_PROXY=0",
    ].join("\n");

    const parsed = parseLinuxProbeOutput(discovered, sampleOut, 18, manifest.scoring);
    expect(parsed.reachable).toBe(true);
    expect(parsed.healthy).toBe(true);
    expect(parsed.cpus).toBe(16);
    expect(parsed.load1).toBe(2.4);
    expect(parsed.loadRatio).toBe(0.15);
    expect(parsed.memTotalGb).toBe(64);
    expect(parsed.memAvailGb).toBe(48);
    expect(parsed.workAvailGb).toBe(150);
    expect(parsed.reflinkSupported).toBe(true);
    expect(parsed.activeCompilerProcs).toBe(5);
    expect(parsed.activeRunnerWorkers).toBe(1);
    expect(parsed.activeJobs).toBe(2);
    expect(parsed.capabilities).toContain("xfs_reflink");
    expect(parsed.capabilities).toContain("native_rust");
    expect(parsed.capabilities).toContain("wasm");
    expect(parsed.score).toBeGreaterThan(60);
    expect(parsed.score).toBeLessThanOrEqual(100);

    // Also validate unreachable / degraded node handling when MemAvailable falls back and disk is low
    const degradedOut = [
      "OS=Linux",
      "ARCH=x86_64",
      "KERNEL=6.12.0",
      "CPUS=4",
      "LOADAVG=6.00 4.00 2.00 8/200 99",
      "MEMTOTAL_KB=16777216",
      "MEMAVAIL_KB=4194304",
      "WORK_DIR=/srv/xbuild",
      "DF=10485760 5242880 50%",
      "FS_TYPE=btrfs",
      "REFLINK=1",
      "CARGO_PROCS=0",
      "CARGO_ROOTS=0",
      "RUNNER_WORKERS=0",
      "HAS_CARGO=0",
      "HAS_WASM=0",
      "HAS_PROXY=1",
    ].join("\n");
    const degraded = parseLinuxProbeOutput(
      { alias: "edge2", sshTarget: "edge2", aliasesInBlock: ["edge2"], discoveredFromSshConfig: true },
      degradedOut,
      50,
      manifest.scoring,
    );
    expect(degraded.healthy).toBe(false);
    expect(degraded.saturated).toBe(true);
    expect(degraded.capabilities).toEqual(["btrfs_reflink", "edge_proxy"]);
    expect(degraded.score).toBe(0);
  });

  test("probes local Windows/macOS/Linux host dynamically using os APIs", async () => {
    const manifest = loadFleetManifest(repoRoot);
    const local = await probeLocalHost(manifest);
    expect(local.alias).toBe("local");
    expect(local.reachable).toBe(true);
    expect(local.cpus).toBeGreaterThanOrEqual(1);
    expect(local.memTotalGb).toBeGreaterThan(0);
    expect(local.memAvailGb).toBeGreaterThan(0);
    expect(local.score).toBeGreaterThan(0);
    expect(local.score).toBeLessThanOrEqual(100);
  });

  test("schedules jobs dynamically by load, capabilities, region affinity, and shard spread", () => {
    const manifest = loadFleetManifest(repoRoot);
    const makeNode = (overrides: Partial<NodeProbeTelemetry>): NodeProbeTelemetry => {
      const cpus = overrides.cpus ?? 16;
      const load1 = overrides.load1 ?? 1.0;
      const memTotalGb = overrides.memTotalGb ?? 64;
      const memAvailGb = overrides.memAvailGb ?? 48;
      const workAvailGb = overrides.workAvailGb ?? 120;
      const activeJobs = overrides.activeJobs ?? 0;
      const rttMs = overrides.rttMs ?? 20;
      const reachable = overrides.reachable ?? true;
      const priorityBias = overrides.priorityBias ?? 0;
      const scored = computeNodeScore(
        { reachable, cpus, load1, memTotalGb, memAvailGb, workAvailGb, activeJobs, rttMs, priorityBias },
        manifest.scoring,
      );
      return {
        alias: "node",
        sshTarget: "node",
        region: "eu-west",
        tier: "builder",
        priorityBias,
        reachable,
        healthy: scored.healthy,
        saturated: scored.saturated,
        healthReasons: scored.healthReasons,
        os: "Linux",
        arch: "x86_64",
        kernel: "7.3.0",
        cpus,
        load1,
        load5: load1,
        load15: load1,
        loadRatio: scored.loadRatio,
        memTotalGb,
        memAvailGb,
        workDir: "/srv/xbuild",
        workTotalGb: 200,
        workAvailGb,
        workUsePct: 40,
        fsType: "xfs",
        reflinkSupported: true,
        activeCompilerProcs: 0,
        activeRunnerWorkers: 0,
        activeJobs,
        rttMs,
        capabilities: ["xfs_reflink", "native_rust", "wasm"],
        labels: [],
        score: scored.score,
        reportedAt: new Date().toISOString(),
        ...overrides,
      };
    };

    const nodes = [
      makeNode({ alias: "fr1", sshTarget: "fr1", load1: 12.0, activeJobs: 2, priorityBias: 4 }),
      makeNode({ alias: "fr2", sshTarget: "fr2", load1: 0.8, activeJobs: 0, priorityBias: 2 }),
      makeNode({
        alias: "prd",
        sshTarget: "prd",
        region: "us-edge",
        tier: "edge",
        load1: 0.3,
        fsType: "btrfs",
        capabilities: ["btrfs_reflink", "edge_proxy", "wasm"],
      }),
    ];

    const rustPlan = scheduleWorkloads(nodes, manifest, { jobKind: "native_rust", shards: 2 });
    expect(rustPlan.ok).toBe(true);
    expect(rustPlan.primaryHost).toBe("fr2");
    expect(rustPlan.fallbackChain).toEqual(["fr1"]);
    // Heavily loaded fr1 (12/16 load + 2 active jobs) yields both shards to near-idle fr2
    expect(rustPlan.assignments.map((a) => a.host)).toEqual(["fr2", "fr2"]);
    expect(rustPlan.rejected.map((r) => r.alias)).toEqual(["prd"]);

    const balancedNodes = [
      makeNode({ alias: "fr1", sshTarget: "fr1", load1: 3.2, activeJobs: 0, priorityBias: 4 }),
      makeNode({ alias: "fr2", sshTarget: "fr2", load1: 0.4, activeJobs: 0, priorityBias: 2 }),
      nodes[2]!,
    ];
    const spreadPlan = scheduleWorkloads(balancedNodes, manifest, { jobKind: "native_rust", shards: 2 });
    expect(spreadPlan.ok).toBe(true);
    expect(spreadPlan.primaryHost).toBe("fr2");
    expect(spreadPlan.assignments.map((a) => a.host)).toEqual(["fr2", "fr1"]);

    const edgePlan = scheduleWorkloads(nodes, manifest, { jobKind: "edge_proxy", shards: 1 });
    expect(edgePlan.ok).toBe(true);
    expect(edgePlan.primaryHost).toBe("prd");
    expect(edgePlan.rejected.map((r) => r.alias).sort()).toEqual(["fr1", "fr2"]);
  });

  test("dispatches commands with automatic work-stealing fallback to next-lightest host", async () => {
    const candidates = [
      { alias: "fr2", sshTarget: "fr2", score: 92 },
      { alias: "fr1", sshTarget: "fr1", score: 74 },
      { alias: "prd", sshTarget: "prd", score: 65 },
    ];

    const dry = await dispatchWithWorkStealing(
      candidates,
      "cargo check -p brand",
      async () => ({ exitCode: 1, stdout: "", stderr: "should not run", durationMs: 1 }),
      2,
      true,
    );
    expect(dry.ok).toBe(true);
    expect(dry.dryRun).toBe(true);
    expect(dry.executedHost).toBe("fr2");
    expect(dry.fallbackChain).toEqual(["fr1", "prd"]);

    const stolen = await dispatchWithWorkStealing(
      candidates,
      "cargo check -p brand",
      async (cand) => {
        if (cand.alias === "fr2") {
          return { exitCode: 255, stdout: "", stderr: "Connection timed out", durationMs: 42 };
        }
        return { exitCode: 0, stdout: "Finished dev\n", stderr: "", durationMs: 120 };
      },
      2,
    );
    expect(stolen.ok).toBe(true);
    expect(stolen.executedHost).toBe("fr1");
    expect(stolen.attempts).toHaveLength(2);
    expect(stolen.attempts[0]?.ok).toBe(false);
    expect(stolen.attempts[1]?.stolenFrom).toBe("fr2");
    expect(stolen.fallbackChain).toEqual(["prd"]);

    const exhausted = await dispatchWithWorkStealing(
      candidates,
      "false",
      async () => ({ exitCode: 1, stdout: "", stderr: "boom", durationMs: 5 }),
      1,
    );
    expect(exhausted.ok).toBe(false);
    expect(exhausted.attempts).toHaveLength(2);
    expect(exhausted.executedHost).toBeNull();
  });

  test("publishes signed and unauthenticated beacons compatible with aphrody-worker /api/fleet/*", async () => {
    const manifest = loadFleetManifest(repoRoot);
    const local = await probeLocalHost(manifest);

    const dryUnauth = await publishBeacon([local], manifest, {
      workerUrl: "https://releases.aphrody.com",
      dryRun: true,
    });
    expect(dryUnauth.ok).toBe(true);
    expect(dryUnauth.dryRun).toBe(true);
    expect(dryUnauth.authMode).toBe("unauthenticated");
    expect(dryUnauth.signed).toBe(false);

    const drySigned = await publishBeacon([local], manifest, {
      workerUrl: "https://releases.aphrody.com",
      token: "beacon-test-secret",
      authMode: "hmac",
      dryRun: true,
    });
    expect(drySigned.ok).toBe(true);
    expect(drySigned.authMode).toBe("hmac-sha256");
    expect(drySigned.signed).toBe(true);

    // End-to-end verification against .cloudflare/aphrody-worker fetch handler
    const bucketMap = new Map<string, string>();
    const env: WorkerEnv = {
      FLEET_BEACON_TOKEN: "beacon-test-secret",
      ASSETS_BUCKET: {
        async get(key: string) {
          const val = bucketMap.get(key);
          return val ? { body: val } : null;
        },
        async put(key: string, value: string | Uint8Array | ArrayBuffer | ReadableStream) {
          bucketMap.set(key, typeof value === "string" ? value : new TextDecoder().decode(value as Uint8Array));
        },
      },
    };
    const bodyText = JSON.stringify(drySigned.payload);
    const sig = await signBeaconBody("beacon-test-secret", bodyText);
    const beaconRes = await worker.fetch(
      new Request(drySigned.endpoint, {
        method: "POST",
        headers: { "content-type": "application/json", "x-fleet-signature-256": sig },
        body: bodyText,
      }),
      env,
    );
    expect(beaconRes.status).toBe(200);

    const routeRes = await worker.fetch(new Request("https://releases.aphrody.com/api/fleet/route"), env);
    expect(routeRes.status).toBe(200);
    const routeJson = (await routeRes.json()) as { selected: { alias: string } | null };
    expect(routeJson.selected?.alias).toBe("local");
  });

  test("parses macOS vm_stat reclaimable pages cleanly for available memory", () => {
    const sample = [
      "Mach Virtual Memory Statistics: (page size of 16384 bytes)",
      "Pages free:                               10240.",
      "Pages active:                            180000.",
      "Pages inactive:                          250000.",
      "Pages speculative:                        12000.",
      "Pages purgeable:                          55440.",
    ].join("\n");
    const availGb = parseMacosVmStatAvailGb(sample);
    expect(availGb).not.toBeNull();
    expect(availGb!).toBeCloseTo((327680 * 16384) / 1024 ** 3, 2);
  });
});
