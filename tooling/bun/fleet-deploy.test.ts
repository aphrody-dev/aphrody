import { describe, expect, it } from "bun:test";
import { analyzeProbe, type HostProbeReport } from "./ci-host-optimize.ts";
import { auditCloudflareRepoConfig } from "./cloudflare-deploy.ts";
import { buildParallelShardPlans, readProductMeta, resolveHostList } from "./fleet-sync.ts";

describe("cloudflare & multi-vps fleet orchestration", () => {
  it("audits all Cloudflare worker configs cleanly with zero zed.dev domain leaks", () => {
    const audit = auditCloudflareRepoConfig(undefined, false);
    expect(audit.ok).toBe(true);
    expect(audit.errors).toEqual([]);
    expect(audit.installScriptsPresent).toBe(true);
    expect(audit.releaseDomain).toBe("releases.aphrody.com");
    expect(audit.workers.map((w) => w.name)).toEqual([
      "aphrody-releases",
      "docs-proxy",
      "open-source-website-assets",
    ]);
    for (const worker of audit.workers) {
      expect(worker.hasLegacyZedDomain).toBe(false);
    }
  });

  it("resolves host lists and constructs parallel shard plans across fr1/fr2/prd", () => {
    expect(resolveHostList(undefined, "fr1,fr2,prd", false)).toEqual(["fr1", "fr2", "prd"]);
    expect(resolveHostList(undefined, undefined, true)).toEqual(["fr1", "fr2", "prd"]);
    const meta = readProductMeta();
    expect(meta.slug).toBe("aphrody");
    const shards = buildParallelShardPlans(["fr1", "fr2", "prd"], `/srv/xbuild/scratch/${meta.slug}`);
    expect(shards).toHaveLength(3);
    expect(shards[0]?.role).toBe("primary-xfs-native-builder");
    expect(shards[1]?.role).toBe("secondary-xfs-wasm-verifier");
    expect(shards[2]?.role).toBe("edge-btrfs-cloudflare-deploy");
    expect(shards[0]?.script).toContain("cargo build --profile bun-release");
    expect(shards[1]?.script).toContain("wasm32-wasip2");
    expect(shards[2]?.script).toContain("cloudflare-deploy.ts");
  });

  it("analyzes Arch Btrfs hosts cleanly without AppArmor sysctl breakage", () => {
    const archProbe: HostProbeReport = {
      host: "prd",
      osPrettyName: "Arch Linux",
      osId: "arch",
      remoteUser: "root",
      remoteHome: "/root",
      kernelRelease: "7.2.14-arch1-1",
      cpus: 8,
      memTotalGb: 31.3,
      kvmPresent: false,
      vhostNetPresent: true,
      xfsMount: "/srv/xbuild",
      xfsReflinkSupported: true,
      reflinkFsType: "btrfs",
      rootFsUsePct: 40,
      rootFsAvailGb: 232,
      xfsAvailGb: 232,
      tmpfsAvailGb: 16,
      blockDevices: [
        {
          name: "sda",
          size: "400G",
          fstype: "btrfs",
          mountpoints: ["/"],
          rotational: 0,
          scheduler: "mq-deadline",
          readAheadKb: 128,
          nrRequests: 256,
        },
      ],
      sysctl: {
        "kernel.io_uring_disabled": "0",
        "vm.vfs_cache_pressure": "50",
      },
      sccache: {
        installed: true,
        version: "0.19.0",
        dir: "/var/cache/aphrody/sccache",
        maxSize: "40 GiB",
        baseDirs: "/srv/xbuild:/root:/srv",
        requests: 42,
        hits: 30,
        rustcWrapperEnv: "/usr/bin/sccache",
      },
      linkers: { wild: false, mold: true, lld: false },
      runners: [],
      coldRootDirsGb: [],
    };
    const plan = analyzeProbe(archProbe);
    expect(plan.score).toBe(100);
    expect(plan.sysctlDropin).not.toContain("apparmor_restrict_unprivileged_userns");
    expect(plan.profileEnvDropin).toContain("/root");
  });
});
