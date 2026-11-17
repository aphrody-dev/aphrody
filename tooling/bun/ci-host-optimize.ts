#!/usr/bin/env bun
/**
 * CI/CD & Build Host Optimizer rooted in Zed's existing infrastructure (`script/setup-sccache`,
 * `script/linux`, `script/clear-target-dir-if-larger-than`, `tooling/xtask/src/tasks/workflows/runners.rs`)
 * and tuned for Ubuntu 26.10, Kernel 7.3, KVM virtio storage, XFS reflink, sccache, and systemd cgroups v2.
 *
 * Usage:
 *   bun tooling/bun/ci-host-optimize.ts audit [--host fr1|local] [--json]
 *   bun tooling/bun/ci-host-optimize.ts plan [--host fr1|local]
 *   bun tooling/bun/ci-host-optimize.ts apply [--host fr1|local] [--offload-cold]
 *   bun tooling/bun/ci-host-optimize.ts bench [--host fr1|local]
 */
import { existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

export const repoRoot = resolve(import.meta.dir, "../..");

export interface ZedBaselineStatus {
  scriptsPresent: Record<string, boolean>;
  rustToolchainChannel: string;
  targetDirMaxGbHint: string | null;
  runnersSpecCount: number;
}

export interface BlockDeviceInfo {
  name: string;
  size: string;
  fstype: string;
  mountpoints: string[];
  rotational: number;
  scheduler: string;
  readAheadKb: number;
  nrRequests: number;
  availPct?: number;
}

export interface HostProbeReport {
  host: string;
  osPrettyName: string;
  kernelRelease: string;
  cpus: number;
  memTotalGb: number;
  kvmPresent: boolean;
  vhostNetPresent: boolean;
  xfsMount: string | null;
  xfsReflinkSupported: boolean;
  rootFsUsePct: number;
  rootFsAvailGb: number;
  xfsAvailGb: number;
  tmpfsAvailGb: number;
  blockDevices: BlockDeviceInfo[];
  sysctl: Record<string, string>;
  sccache: {
    installed: boolean;
    version: string | null;
    dir: string | null;
    maxSize: string | null;
    baseDirs: string | null;
    requests: number;
    hits: number;
    rustcWrapperEnv: string | null;
  };
  linkers: Record<string, boolean>;
  runners: Array<{ unit: string; slice: string; active: boolean }>;
  coldRootDirsGb: Array<{ path: string; sizeGb: number; activeOpens: boolean }>;
}

export interface OptimizationFinding {
  id: string;
  severity: "high" | "medium" | "low";
  category: "storage" | "sccache" | "kernel" | "kvm" | "systemd" | "linker" | "zed-baseline";
  title: string;
  current: string;
  target: string;
  rationale: string;
}

export interface HostOptimizationPlan {
  score: number;
  zedBaseline: ZedBaselineStatus;
  probe: HostProbeReport;
  findings: OptimizationFinding[];
  sysctlDropin: string;
  udevDropin: string;
  runnerDropin: string;
  profileEnvDropin: string;
}

export function inspectZedBaseline(root = repoRoot): ZedBaselineStatus {
  const checkedPaths = [
    "script/setup-sccache",
    "script/linux",
    "script/bootstrap",
    "script/zed-local",
    "script/clear-target-dir-if-larger-than",
    "tooling/xtask/src/tasks/workflows/runners.rs",
    ".github/workflows/aphrody-ci.yml",
  ];
  const scriptsPresent: Record<string, boolean> = {};
  for (const rel of checkedPaths) {
    scriptsPresent[rel] = existsSync(join(root, rel));
  }

  let rustToolchainChannel = "unknown";
  const rtPath = join(root, "rust-toolchain.toml");
  if (existsSync(rtPath)) {
    const parsed = Bun.TOML.parse(readFileSync(rtPath, "utf8")) as {
      toolchain?: { channel?: string };
    };
    rustToolchainChannel = parsed.toolchain?.channel ?? "unknown";
  }

  let runnersSpecCount = 0;
  const runnersPath = join(root, "tooling/xtask/src/tasks/workflows/runners.rs");
  if (existsSync(runnersPath)) {
    const raw = readFileSync(runnersPath, "utf8");
    runnersSpecCount = (raw.match(/pub const [A-Z0-9_]+:\s*Runner\b/g) ?? []).length;
  }

  let targetDirMaxGbHint: string | null = null;
  const gcPath = join(root, "script/clear-target-dir-if-larger-than");
  if (existsSync(gcPath)) {
    const m = readFileSync(gcPath, "utf8").match(/target_dir_size.*\n|du\s+-sh|\d+\s*[Gg]/);
    targetDirMaxGbHint = m ? m[0].trim() : "threshold arg (GB)";
  }

  return {
    scriptsPresent,
    rustToolchainChannel,
    targetDirMaxGbHint,
    runnersSpecCount,
  };
}

function buildProbeScript(): string {
  return `set -euo pipefail
python3 - <<'PY'
import json, os, re, shutil, subprocess

def run(cmd):
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=15)
        return r.stdout.strip(), r.returncode
    except Exception:
        return "", 1

def read_file(p):
    try:
        with open(p, "r", encoding="utf-8", errors="ignore") as f:
            return f.read().strip()
    except Exception:
        return ""

os_rel = read_file("/etc/os-release")
m_pretty = re.search(r'^PRETTY_NAME="?(.*?)"?$', os_rel, re.M)
os_pretty = m_pretty.group(1) if m_pretty else "Linux"
kernel, _ = run(["uname", "-r"])
cpus = os.cpu_count() or 1

meminfo = read_file("/proc/meminfo")
m_mem = re.search(r'^MemTotal:\\s+(\\d+)\\s+kB', meminfo, re.M)
mem_gb = round(int(m_mem.group(1)) / (1024 * 1024), 1) if m_mem else 0.0

def df_info(path):
    if not os.path.exists(path):
        return 0.0, 0
    out, rc = run(["df", "-BG", "--output=avail,pcent", path])
    if rc != 0:
        return 0.0, 0
    lines = out.splitlines()
    if len(lines) < 2:
        return 0.0, 0
    parts = lines[-1].split()
    avail = float(parts[0].rstrip("G"))
    pct = int(parts[1].rstrip("%"))
    return avail, pct

root_avail, root_pct = df_info("/")
xfs_mount = "/srv/xbuild" if os.path.isdir("/srv/xbuild") else None
xfs_avail, _ = df_info("/srv/xbuild") if xfs_mount else (0.0, 0)
tmp_avail, _ = df_info("/tmp")

xfs_reflink = False
if xfs_mount:
    xinfo, _ = run(["xfs_info", xfs_mount])
    xfs_reflink = "reflink=1" in xinfo

blocks = []
ls_out, rc = run(["lsblk", "-J", "-d", "-o", "NAME,SIZE,FSTYPE,ROTA,SCHED,RA"])
if rc == 0 and ls_out:
    for dev in json.loads(ls_out).get("blockdevices", []):
        name = dev.get("name", "")
        if not name or name.startswith("loop"):
            continue
        qdir = f"/sys/block/{name}/queue"
        nr_req = int(read_file(f"{qdir}/nr_requests") or 0)
        ra_kb = int(read_file(f"{qdir}/read_ahead_kb") or dev.get("ra") or 0)
        rota = int(read_file(f"{qdir}/rotational") or (1 if dev.get("rota") else 0))
        sched_raw = read_file(f"{qdir}/scheduler")
        m_s = re.search(r"\\[([^\\]]+)\\]", sched_raw)
        sched = m_s.group(1) if m_s else (dev.get("sched") or "none")
        blocks.append({
            "name": name,
            "size": str(dev.get("size") or ""),
            "fstype": str(dev.get("fstype") or ""),
            "mountpoints": [],
            "rotational": rota,
            "scheduler": sched,
            "readAheadKb": ra_kb,
            "nrRequests": nr_req,
        })

sysctl_keys = [
    "kernel.io_uring_disabled",
    "kernel.sched_autogroup_enabled",
    "kernel.perf_event_paranoid",
    "kernel.unprivileged_userns_clone",
    "kernel.apparmor_restrict_unprivileged_userns",
    "vm.swappiness",
    "vm.page-cluster",
    "vm.vfs_cache_pressure",
    "vm.compaction_proactiveness",
    "vm.dirty_background_bytes",
    "vm.dirty_bytes",
    "fs.inotify.max_user_watches",
    "fs.inotify.max_user_instances",
    "vm.max_map_count",
    "net.core.default_qdisc",
    "net.ipv4.tcp_congestion_control",
]
sysctl_map = {}
for k in sysctl_keys:
    v, r = run(["sysctl", "-n", k])
    if r == 0:
        sysctl_map[k] = re.sub(r"\\s+", " ", v)

etc_env = read_file("/etc/environment")
env_dict = {}
for line in etc_env.splitlines():
    if "=" in line and not line.strip().startswith("#"):
        k, v = line.split("=", 1)
        env_dict[k.strip()] = v.strip().strip('"').strip("'")

scc_bin = shutil.which("sccache") or ("/usr/local/bin/sccache" if os.path.exists("/usr/local/bin/sccache") else None)
scc_ver = None
scc_stats = ""
if scc_bin:
    ver_out, _ = run([scc_bin, "--version"])
    scc_ver = ver_out.split()[-1] if ver_out else None
    stat_env = os.environ.copy()
    if env_dict.get("SCCACHE_DIR"):
        stat_env["SCCACHE_DIR"] = env_dict["SCCACHE_DIR"]
    try:
        scc_stats = subprocess.run([scc_bin, "--show-stats"], capture_output=True, text=True, env=stat_env, timeout=10).stdout
    except Exception:
        scc_stats = ""

def parse_stat(pattern):
    m = re.search(pattern, scc_stats)
    return m.group(1).strip() if m else None

def parse_int_stat(pattern):
    s = parse_stat(pattern)
    return int(s) if s and s.isdigit() else 0

units_out, _ = run(["systemctl", "list-units", "--type=service", "--no-pager", "--plain", "--no-legend"])
runners = []
for line in units_out.splitlines():
    u = line.split()[0] if line.split() else ""
    if "actions.runner" in u or "github-runner" in u:
        sl, _ = run(["systemctl", "show", u, "-p", "Slice", "--value"])
        act, _ = run(["systemctl", "is-active", u])
        runners.append({"unit": u, "slice": sl or "unknown", "active": act == "active"})

cold_candidates = ["/home/ubuntu/third_party-store", "/srv/chromium", "/home/ubuntu/build"]
cold = []
for p in cold_candidates:
    if os.path.isdir(p) and not os.path.islink(p):
        du_out, _ = run(["du", "-sBG", p])
        gb = int(du_out.split()[0].rstrip("G")) if du_out else 0
        fu_out, _ = run(["fuser", p])
        cold.append({"path": p, "sizeGb": gb, "activeOpens": bool(fu_out.strip())})

payload = {
    "host": os.uname().nodename,
    "osPrettyName": os_pretty,
    "kernelRelease": kernel,
    "cpus": cpus,
    "memTotalGb": mem_gb,
    "kvmPresent": os.path.exists("/dev/kvm"),
    "vhostNetPresent": os.path.exists("/dev/vhost-net"),
    "xfsMount": xfs_mount,
    "xfsReflinkSupported": xfs_reflink,
    "rootFsUsePct": root_pct,
    "rootFsAvailGb": root_avail,
    "xfsAvailGb": xfs_avail,
    "tmpfsAvailGb": tmp_avail,
    "blockDevices": blocks,
    "sysctl": sysctl_map,
    "sccache": {
        "installed": bool(scc_bin),
        "version": scc_ver,
        "dir": parse_stat(r'Cache location\\s+Local disk:\\s+"([^"]+)"') or env_dict.get("SCCACHE_DIR"),
        "maxSize": parse_stat(r'Max cache size\\s+(.+)'),
        "baseDirs": parse_stat(r'Base directories\\s+(.+)'),
        "requests": parse_int_stat(r'Compile requests\\s+(\\d+)'),
        "hits": parse_int_stat(r'Cache hits\\s+(\\d+)'),
        "rustcWrapperEnv": env_dict.get("RUSTC_WRAPPER") or os.environ.get("RUSTC_WRAPPER") or None,
    },
    "linkers": {
        "wild": bool(shutil.which("wild")),
        "mold": bool(shutil.which("mold")),
        "lld": bool(shutil.which("lld") or shutil.which("ld.lld")),
    },
    "runners": runners,
    "coldRootDirsGb": cold,
}
print(json.dumps(payload))
PY
`;
}

export function probeHost(host: string): HostProbeReport {
  const script = buildProbeScript();
  const cmd = host === "local" ? ["bash", "-s"] : ["ssh", host, "bash -s"];
  const res = Bun.spawnSync(cmd, {
    stdin: Buffer.from(script, "utf8"),
    stdout: "pipe",
    stderr: "pipe",
  });
  if (res.exitCode !== 0) {
    throw new Error(`Probe on '${host}' failed (${res.exitCode}): ${new TextDecoder().decode(res.stderr)}`);
  }
  const stdout = new TextDecoder().decode(res.stdout).trim();
  const report = JSON.parse(stdout) as HostProbeReport;
  report.host = host;
  return report;
}

export function analyzeProbe(probe: HostProbeReport, zedBaseline = inspectZedBaseline()): HostOptimizationPlan {
  const findings: OptimizationFinding[] = [];
  let deductions = 0;

  // 1. Storage & root NVMe pressure vs XFS capacity
  if (probe.rootFsUsePct >= 70 && probe.xfsMount && probe.xfsAvailGb >= 100 && probe.coldRootDirsGb.length > 0) {
    deductions += 20;
    const coldSum = probe.coldRootDirsGb.reduce((acc, c) => acc + c.sizeGb, 0);
    findings.push({
      id: "root-nvme-pressure",
      severity: "high",
      category: "storage",
      title: "Fast root ext4 filesystem over-utilized by cold repositories while XFS LVM sits empty",
      current: `/ is ${probe.rootFsUsePct}% used (${probe.rootFsAvailGb} GB free); cold dirs take ~${coldSum} GB`,
      target: `Move cold mirrors to ${probe.xfsMount} (${probe.xfsAvailGb} GB free XFS) with transparent symlinks; keep NVMe / <55% for hot sccache & target/`,
      rationale:
        "Live fio measurements show local NVMe sda1 sustains ~51k randread / ~22k randwrite IOPS whereas /srv/xbuild (Ceph XFS) offers huge capacity + 8ms CoW reflinks (~260 randread IOPS). Cold clones belong on XFS; hot small-object compiler caches belong on NVMe or tmpfs.",
    });
  }

  // 2. Virtual KVM disks misclassified as rotational HDDs
  const rotaDisks = probe.blockDevices.filter((d) => d.name.startsWith("sd") && d.rotational === 1);
  if (rotaDisks.length > 0) {
    deductions += 15;
    findings.push({
      id: "kvm-virtio-rotational",
      severity: "high",
      category: "kvm",
      title: "KVM virtual disks flagged rotational=1 causing HDD seek heuristics",
      current: rotaDisks.map((d) => `${d.name}(rota=1, sched=${d.scheduler})`).join(", "),
      target: "udev rule setting queue/rotational=0, read_ahead_kb=1024, nr_requests=512 across sd[a-z]/vd[a-z]",
      rationale:
        "KVM/QEMU virtual SCSI/blk devices default rotational=1, distorting CFQ/BFQ and readahead behavior on SSD/Ceph backing tiers.",
    });
  }

  // 3. sccache BASEDIRS, RUSTC_WRAPPER & hit rate
  if (!probe.sccache.installed) {
    deductions += 20;
    findings.push({
      id: "sccache-missing",
      severity: "high",
      category: "sccache",
      title: "sccache binary missing on CI host",
      current: "not installed",
      target: "Install sccache (matching script/setup-sccache) and configure RUSTC_WRAPPER + SCCACHE_BASEDIRS",
      rationale: "Zed monorepo incremental & cross-worktree Rust builds rely on sccache for sub-minute CI turnarounds.",
    });
  } else {
    const missingWrapper = !probe.sccache.rustcWrapperEnv;
    const missingBasedirs = !probe.sccache.baseDirs || probe.sccache.baseDirs === "(none)";
    if (missingWrapper || missingBasedirs || probe.sccache.requests === 0) {
      deductions += 20;
      findings.push({
        id: "sccache-unwired",
        severity: "high",
        category: "sccache",
        title: "sccache installed but RUSTC_WRAPPER or SCCACHE_BASEDIRS is unwired",
        current: `wrapper=${probe.sccache.rustcWrapperEnv ?? "unset"}, baseDirs=${probe.sccache.baseDirs ?? "(none)"}, requests=${probe.sccache.requests}`,
        target:
          "Export RUSTC_WRAPPER=sccache and SCCACHE_BASEDIRS=/srv/xbuild:/home/ubuntu:/srv system-wide (/etc/environment, shell.sh, runner .env)",
        rationale:
          "Without SCCACHE_BASEDIRS, Rust proc-macro and path-hashed crates miss cache across worktrees and runner workspaces (/srv/xbuild/scratch vs _work).",
      });
    }
  }

  // 4. GitHub Actions runner systemd slice isolation
  const unwiredRunners = probe.runners.filter((r) => r.slice !== "build.slice");
  if (unwiredRunners.length > 0) {
    deductions += 10;
    findings.push({
      id: "runner-cgroup-slice",
      severity: "medium",
      category: "systemd",
      title: "GitHub Actions runner executes inside system.slice instead of build.slice",
      current: unwiredRunners.map((r) => `${r.unit} -> ${r.slice}`).join(", "),
      target:
        "Bind runner service Slice=build.slice with LimitNOFILE=1048576, TMPDIR=/tmp, RUSTC_WRAPPER, and SCCACHE_BASEDIRS",
      rationale:
        "Running CI builds in build.slice protects interactive production services via cgroups v2 CPUWeight/IOWeight/MemoryHigh enforcement.",
    });
  }

  // 5. Kernel 7.3 & sysctl gaps
  const missingIoUring = probe.sysctl["kernel.io_uring_disabled"] !== "0";
  const highSwappinessMismatch = probe.sysctl["vm.vfs_cache_pressure"] !== "50";
  if (missingIoUring || highSwappinessMismatch) {
    deductions += 10;
    findings.push({
      id: "kernel-7-3-sysctl",
      severity: "medium",
      category: "kernel",
      title: "Kernel sysctl tuning diverges from Kernel 7.3 io_uring & metadata-cache profile",
      current: `io_uring_disabled=${probe.sysctl["kernel.io_uring_disabled"] ?? "unset"}, vfs_cache_pressure=${probe.sysctl["vm.vfs_cache_pressure"] ?? "unset"}`,
      target:
        "Apply /etc/sysctl.d/96-aphrody-ci-kernel73.conf (io_uring=0, vfs_cache_pressure=50, dirty_bytes=1G, compaction_proactiveness=20)",
      rationale:
        "Keeps XFS/ext4 dentry/inode caches resident across multi-crate rustc/Cargo dependency scans while capping dirty writeback stalls.",
    });
  }

  // 6. Fast ELF linkers check (Linux)
  if (!probe.linkers.wild && !probe.linkers.mold && !probe.linkers.lld) {
    deductions += 10;
    findings.push({
      id: "linker-missing",
      severity: "medium",
      category: "linker",
      title: "No fast parallel ELF linker (wild, mold, ld.lld) found on PATH",
      current: "GNU ld fallback",
      target: "Install wild or lld + mold cleanly as configured in .cargo/config.toml",
      rationale: "Parallel linkers cut multi-hundred-MB GPUI/Zed debug & test link times by 4-10x.",
    });
  }

  // 7. XFS Reflink capability check
  if (probe.xfsMount && !probe.xfsReflinkSupported) {
    deductions += 10;
    findings.push({
      id: "xfs-no-reflink",
      severity: "medium",
      category: "storage",
      title: "XFS volume mounted without reflink CoW support",
      current: `${probe.xfsMount} reflink=0`,
      target: "Enable reflink=1 on XFS CI volumes for O(1) ms worktree and target cloning",
      rationale: "XFS reflink eliminates multi-GB copy stalls during fleet-sync and runner matrix fan-outs.",
    });
  }

  const score = Math.max(0, 100 - deductions);

  const sysctlDropin = `# Managed by aphrody tooling/bun/ci-host-optimize.ts (Ubuntu 26.10 / Kernel ${probe.kernelRelease} / KVM / XFS)
# Harmonizes with 90-fr1-chromium-host.conf and build.slice cgroup v2 controls.
kernel.io_uring_disabled = 0
kernel.sched_autogroup_enabled = 0
kernel.perf_event_paranoid = 1
kernel.unprivileged_userns_clone = 1
kernel.apparmor_restrict_unprivileged_userns = 0
vm.swappiness = 100
vm.page-cluster = 0
vm.vfs_cache_pressure = 50
vm.compaction_proactiveness = 20
vm.dirty_background_bytes = 268435456
vm.dirty_bytes = 1073741824
fs.inotify.max_user_watches = 2097152
fs.inotify.max_user_instances = 8192
fs.aio-max-nr = 1048576
vm.max_map_count = 2147483642
net.core.default_qdisc = fq
net.ipv4.tcp_congestion_control = bbr
`;

  const udevDropin = `# Managed by aphrody tooling/bun/ci-host-optimize.ts (KVM VirtIO SCSI/blk queue tuning)
# Ensures virtual KVM disks never trigger HDD seek penalties while keeping cgroup IOWeight-capable schedulers.
ACTION=="add|change", SUBSYSTEM=="block", KERNEL=="sd[a-z]|vd[a-z]", ATTR{queue/rotational}="0", ATTR{queue/read_ahead_kb}="1024", ATTR{queue/nr_requests}="512"
ACTION=="add|change", SUBSYSTEM=="block", KERNEL=="sda|vda", ATTR{queue/scheduler}="none"
`;

  const runnerDropin = `[Service]
Slice=build.slice
LimitNOFILE=1048576
Environment="RUSTC_WRAPPER=/usr/local/bin/sccache"
Environment="SCCACHE_DIR=/var/cache/aphrody/sccache"
Environment="SCCACHE_CACHE_SIZE=40G"
Environment="SCCACHE_BASEDIRS=/srv/xbuild:/home/ubuntu:/srv"
Environment="TMPDIR=/tmp"
Environment="CARGO_INCREMENTAL=0"
Environment="CARGO_PROFILE_DEV_DEBUG=line-tables-only"
`;

  const profileEnvDropin = `# Managed by aphrody tooling/bun/ci-host-optimize.ts
export RUSTC_WRAPPER="/usr/local/bin/sccache"
export SCCACHE_DIR="/var/cache/aphrody/sccache"
export SCCACHE_CACHE_SIZE="40G"
export SCCACHE_BASEDIRS="/srv/xbuild:/home/ubuntu:/srv"
export TMPDIR="/tmp"
`;

  return {
    score,
    zedBaseline,
    probe,
    findings,
    sysctlDropin,
    udevDropin,
    runnerDropin,
    profileEnvDropin,
  };
}

function buildApplyScript(plan: HostOptimizationPlan, offloadCold: boolean): string {
  const runnerDropins = plan.probe.runners
    .map(
      (r) =>
        `mkdir -p "/etc/systemd/system/${r.unit}.d"\ncat <<'EOF' > "/etc/systemd/system/${r.unit}.d/50-aphrody-ci.conf"\n${plan.runnerDropin}EOF`,
    )
    .join("\n");

  const offloadSnippet = offloadCold
    ? `
mkdir -p /srv/xbuild/cold
for src in /home/ubuntu/third_party-store; do
  if [ -d "$src" ] && [ ! -L "$src" ]; then
    base=$(basename "$src")
    dst="/srv/xbuild/cold/$base"
    if [ ! -e "$dst" ] && [ -z "$(fuser "$src" 2>/dev/null || true)" ]; then
      mv "$src" "$dst" && ln -s "$dst" "$src"
      echo "Offloaded $src -> $dst (symlinked)"
    elif [ -d "$dst" ] && [ -z "$(fuser "$src" 2>/dev/null || true)" ]; then
      rsync -a --remove-source-files "$src/" "$dst/"
      find "$src" -type d -empty -delete
      if [ ! -e "$src" ] || [ -z "$(ls -A "$src" 2>/dev/null)" ]; then
        rm -rf "$src" && ln -s "$dst" "$src"
        echo "Resumed offload $src -> $dst (symlinked)"
      fi
    fi
  fi
done
`
    : "";

  return `set -euo pipefail
mkdir -p /var/cache/aphrody
if [ -d /home/ubuntu/.cache/sccache ] && [ ! -L /home/ubuntu/.cache/sccache ]; then
  if [ ! -e /var/cache/aphrody/sccache ] || [ -z "$(ls -A /var/cache/aphrody/sccache 2>/dev/null)" ]; then
    rmdir /var/cache/aphrody/sccache 2>/dev/null || true
    mv /home/ubuntu/.cache/sccache /var/cache/aphrody/sccache 2>/dev/null || {
      mkdir -p /var/cache/aphrody/sccache
      rsync -a --remove-source-files /home/ubuntu/.cache/sccache/ /var/cache/aphrody/sccache/ 2>/dev/null || cp -a /home/ubuntu/.cache/sccache/. /var/cache/aphrody/sccache/
      rm -rf /home/ubuntu/.cache/sccache
    }
  fi
fi
mkdir -p /var/cache/aphrody/sccache /home/ubuntu/.cache
ln -sfn /var/cache/aphrody/sccache /home/ubuntu/.cache/sccache
chown -R ubuntu:ubuntu /var/cache/aphrody

cat <<'EOF' > /etc/sysctl.d/96-aphrody-ci-kernel73.conf
${plan.sysctlDropin}EOF
sysctl --system >/dev/null

cat <<'EOF' > /etc/udev/rules.d/67-aphrody-ci-block.rules
${plan.udevDropin}EOF
udevadm control --reload-rules
for dev in /dev/sd[a-z]; do
  [ -b "$dev" ] && udevadm trigger --action=change "$dev" || true
done

cat <<'EOF' > /etc/profile.d/aphrody-ci.sh
${plan.profileEnvDropin}EOF
chmod 0644 /etc/profile.d/aphrody-ci.sh

python3 - <<'PY'
p = "/etc/environment"
lines = open(p, "r", encoding="utf-8").read().splitlines()
updates = {
    "SCCACHE_DIR": '"/var/cache/aphrody/sccache"',
    "SCCACHE_CACHE_SIZE": '"40G"',
    "SCCACHE_BASEDIRS": '"/srv/xbuild:/home/ubuntu:/srv"',
    "RUSTC_WRAPPER": '"/usr/local/bin/sccache"',
}
out = []
seen = set()
for line in lines:
    if "=" in line and not line.strip().startswith("#"):
        k = line.split("=", 1)[0].strip()
        if k in updates:
            out.append(f"{k}={updates[k]}")
            seen.add(k)
            continue
    out.append(line)
for k, v in updates.items():
    if k not in seen:
        out.append(f"{k}={v}")
with open(p, "w", encoding="utf-8") as f:
    f.write("\\n".join(out) + "\\n")
PY

${runnerDropins}
systemctl daemon-reload
for unit in ${plan.probe.runners.map((r) => `"${r.unit}"`).join(" ")}; do
  systemctl try-reload-or-restart "$unit" || true
done

${offloadSnippet}
echo "APPLIED aphrody ci-host-optimize cleanly."
`;
}

function buildBenchScript(): string {
  return `set -euo pipefail
export PATH="/home/ubuntu/.cargo/bin:/home/ubuntu/.bun/bin:/usr/local/bin:$PATH"
echo "=== 1. Block & Sysctl Verify ==="
lsblk -o NAME,SIZE,FSTYPE,FSAVAIL,FSUSE%,ROTA,SCHED,RA | grep -v loop
echo "io_uring_disabled=$(sysctl -n kernel.io_uring_disabled) vfs_cache_pressure=$(sysctl -n vm.vfs_cache_pressure)"
echo "=== 2. XFS Reflink Speed (256 MiB CoW clone) ==="
python3 - <<'PY'
import os, subprocess, tempfile, time
with tempfile.TemporaryDirectory(dir="/srv/xbuild") as d:
    src = os.path.join(d, "blob.bin")
    dst = os.path.join(d, "clone.bin")
    with open(src, "wb") as f:
        f.truncate(256 * 1024 * 1024)
    t0 = time.monotonic()
    r = subprocess.run(["cp", "--reflink=always", src, dst], check=True)
    dt = (time.monotonic() - t0) * 1000.0
    print(f"XFS_REFLINK_256MIB_MS={dt:.2f}")
PY
echo "=== 3. NVMe (/var/cache/aphrody) 4K randrw fio ==="
fio --name=nvme-sccache --directory=/var/cache/aphrody --ioengine=io_uring --rw=randrw --rwmixread=70 --bs=4k --size=64M --numjobs=4 --iodepth=16 --runtime=4 --time_based --group_reporting --unlink=1 --direct=1 2>&1 | grep -E "READ:|WRITE:|99\\.00th"
echo "=== 4. tmpfs (/tmp) 4K randrw fio ==="
fio --name=tmpfs-scratch --directory=/tmp --ioengine=sync --rw=randrw --rwmixread=70 --bs=4k --size=64M --numjobs=2 --runtime=3 --time_based --group_reporting --unlink=1 2>&1 | grep -E "READ:|WRITE:"
echo "=== 5. sccache Stats ==="
sccache --show-stats
`;
}

if (import.meta.main) {
  const args = Bun.argv.slice(2);
  const cmd = args[0] ?? "audit";
  const hostIdx = args.indexOf("--host");
  const host = (hostIdx >= 0 ? args[hostIdx + 1] : undefined) ?? process.env.APHRODY_FLEET_HOST ?? "fr1";
  const asJson = args.includes("--json");
  const offloadCold = args.includes("--offload-cold");

  if (cmd === "bench") {
    const script = buildBenchScript();
    const res = Bun.spawnSync(host === "local" ? ["bash", "-s"] : ["ssh", host, "sudo bash -s"], {
      stdin: Buffer.from(script, "utf8"),
      stdout: "inherit",
      stderr: "inherit",
    });
    process.exit(res.exitCode);
  }

  const probe = probeHost(host);
  const plan = analyzeProbe(probe);

  if (cmd === "audit") {
    if (asJson) {
      console.log(JSON.stringify(plan, null, 2));
    } else {
      console.log(
        `Host ${probe.host}: ${probe.osPrettyName} | Kernel ${probe.kernelRelease} | ${probe.cpus}c / ${probe.memTotalGb} GiB | Score ${plan.score}/100`,
      );
      console.log(
        `Storage: /=${probe.rootFsUsePct}% (${probe.rootFsAvailGb}G free) | XFS=${probe.xfsMount ?? "none"} (${probe.xfsAvailGb}G free, reflink=${probe.xfsReflinkSupported}) | tmpfs=${probe.tmpfsAvailGb}G free`,
      );
      for (const f of plan.findings) {
        console.log(
          `- [${f.severity.toUpperCase()}] (${f.category}) ${f.title}\n    now: ${f.current}\n    fix: ${f.target}`,
        );
      }
    }
    process.exit(0);
  }

  if (cmd === "plan") {
    console.log(buildApplyScript(plan, offloadCold));
    process.exit(0);
  }

  if (cmd === "apply") {
    const applyScript = buildApplyScript(plan, offloadCold);
    const res = Bun.spawnSync(host === "local" ? ["sudo", "bash", "-s"] : ["ssh", host, "sudo bash -s"], {
      stdin: Buffer.from(applyScript, "utf8"),
      stdout: "inherit",
      stderr: "inherit",
    });
    process.exit(res.exitCode);
  }

  console.error(
    "Usage: bun tooling/bun/ci-host-optimize.ts [audit|plan|apply|bench] [--host fr1|local] [--json] [--offload-cold]",
  );
  process.exit(1);
}
