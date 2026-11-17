import { describe, expect, test } from "bun:test";
import { analyzeProbe, inspectZedBaseline, type HostProbeReport } from "./ci-host-optimize.ts";

describe("ci-host-optimize", () => {
  test("detects Zed baseline scripts and rust-toolchain manifest cleanly", () => {
    const baseline = inspectZedBaseline();
    expect(baseline.scriptsPresent["script/setup-sccache"]).toBe(true);
    expect(baseline.scriptsPresent["script/linux"]).toBe(true);
    expect(baseline.scriptsPresent["script/clear-target-dir-if-larger-than"]).toBe(true);
    expect(baseline.scriptsPresent["tooling/xtask/src/tasks/workflows/runners.rs"]).toBe(true);
    expect(baseline.runnersSpecCount).toBeGreaterThanOrEqual(8);
    expect(baseline.rustToolchainChannel).not.toBe("unknown");
  });

  test("identifies rotational KVM disks, unwired sccache, root NVMe pressure, and non-slice runner", () => {
    const synthetic: HostProbeReport = {
      host: "fr1",
      osPrettyName: "Ubuntu 26.10",
      kernelRelease: "7.3.0-10-generic",
      cpus: 12,
      memTotalGb: 45,
      kvmPresent: true,
      vhostNetPresent: true,
      xfsMount: "/srv/xbuild",
      xfsReflinkSupported: true,
      rootFsUsePct: 77,
      rootFsAvailGb: 68,
      xfsAvailGb: 568,
      tmpfsAvailGb: 17,
      blockDevices: [
        {
          name: "sda",
          size: "300G",
          fstype: "ext4",
          mountpoints: ["/"],
          rotational: 0,
          scheduler: "none",
          readAheadKb: 1024,
          nrRequests: 256,
        },
        {
          name: "sdb",
          size: "500G",
          fstype: "LVM2_member",
          mountpoints: [],
          rotational: 1,
          scheduler: "bfq",
          readAheadKb: 1024,
          nrRequests: 256,
        },
      ],
      sysctl: {
        "kernel.io_uring_disabled": "0",
        "vm.vfs_cache_pressure": "100",
      },
      sccache: {
        installed: true,
        version: "0.19.0",
        dir: "/home/ubuntu/.cache/sccache",
        maxSize: "120 GiB",
        baseDirs: "(none)",
        requests: 0,
        hits: 0,
        rustcWrapperEnv: null,
      },
      linkers: { wild: false, mold: true, lld: true },
      runners: [{ unit: "actions.runner.example.service", slice: "system.slice", active: true }],
      coldRootDirsGb: [{ path: "/home/ubuntu/third_party-store", sizeGb: 34, activeOpens: false }],
    };

    const plan = analyzeProbe(synthetic);
    const ids = plan.findings.map((f) => f.id);
    expect(ids).toContain("root-nvme-pressure");
    expect(ids).toContain("kvm-virtio-rotational");
    expect(ids).toContain("sccache-unwired");
    expect(ids).toContain("runner-cgroup-slice");
    expect(ids).toContain("kernel-7-3-sysctl");
    expect(plan.score).toBeLessThan(40);
    expect(plan.udevDropin).toContain('ATTR{queue/rotational}="0"');
    expect(plan.sysctlDropin).toContain("kernel.io_uring_disabled = 0");
    expect(plan.runnerDropin).toContain("Slice=build.slice");
  });
});
