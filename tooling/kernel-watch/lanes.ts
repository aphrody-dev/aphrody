// Loader for lanes.toml: probe targets, the build host lanes and the WSL kernel. Every host is an ssh alias read here, never in code.
import { parseDuration } from "./manifest.ts";

export interface ProbeHost {
  id: string;
  ssh: string;
  label: string;
}
export interface ProbeConfig {
  packages: string[];
  aur: string[];
  cachyosPackages: string[];
  cachyos: { id: string; url: string }[];
  archApi: string;
  aurRpc: string;
  /** Seconds. */
  timeout: number;
  hosts: ProbeHost[];
}
export interface LaneKernel {
  id: string;
  name: string;
  /** Kernel id and channel of the versions table this lane kernel reports its test status to. */
  kernel: string;
  channel: string;
  source: "repo" | "aur" | "cachyos" | "tree";
  package: string;
  repos: string[];
}
export interface LaneConfig {
  ssh: string;
  repoUrl: string;
  image: string;
  aurGit: string;
  kernelGit: string;
  testingRepos: string[];
  vmCpus: number;
  vmMemoryMb: number;
  diskGb: number;
  bootTimeout: number;
  buildTimeout: number;
  refreshRootfsDays: number;
  timer: string;
  kselftestTargets: string[];
  kernels: LaneKernel[];
}
export interface Lanes {
  probe: ProbeConfig;
  lane: LaneConfig;
}

type Table = Record<string, unknown>;
const table = (value: unknown, where: string): Table => {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error(`lanes.toml: ${where} must be a table`);
  return value as Table;
};
const text = (t: Table, key: string, where: string): string => {
  const v = t[key];
  if (typeof v !== "string" || !v) throw new Error(`lanes.toml: ${where}.${key} must be a non-empty string`);
  return v;
};
const list = (t: Table, key: string, where: string): string[] => {
  const v = t[key];
  if (!Array.isArray(v) || v.some((x) => typeof x !== "string")) throw new Error(`lanes.toml: ${where}.${key} must be a list of strings`);
  return v as string[];
};
const num = (t: Table, key: string, where: string): number => {
  const v = t[key];
  if (typeof v !== "number" || !Number.isFinite(v)) throw new Error(`lanes.toml: ${where}.${key} must be a number`);
  return v;
};
const rows = (t: Table, key: string, where: string): Table[] => {
  const v = t[key];
  if (!Array.isArray(v) || v.length === 0) throw new Error(`lanes.toml: ${where}.${key} must be a non-empty array of tables`);
  return v.map((x, i) => table(x, `${where}.${key}[${i}]`));
};

export function parseLanes(source: string): Lanes {
  const root = table(Bun.TOML.parse(source), "root");
  const p = table(root.probe, "probe");
  const l = table(root.lane, "lane");
  return {
    lane: {
      ssh: text(l, "ssh", "lane"),
      repoUrl: text(l, "repo_url", "lane"),
      image: text(l, "image", "lane"),
      aurGit: text(l, "aur_git", "lane"),
      kernelGit: text(l, "kernel_git", "lane"),
      testingRepos: list(l, "testing_repos", "lane"),
      vmCpus: num(l, "vm_cpus", "lane"),
      vmMemoryMb: num(l, "vm_memory_mb", "lane"),
      diskGb: num(l, "disk_gb", "lane"),
      bootTimeout: parseDuration(text(l, "boot_timeout", "lane")),
      buildTimeout: parseDuration(text(l, "build_timeout", "lane")),
      refreshRootfsDays: num(l, "refresh_rootfs_days", "lane"),
      timer: text(l, "timer", "lane"),
      kselftestTargets: list(l, "kselftest_targets", "lane"),
      kernels: rows(l, "kernel", "lane").map((r, i): LaneKernel => {
        const source = text(r, "source", `lane.kernel[${i}]`);
        if (source !== "repo" && source !== "aur" && source !== "cachyos" && source !== "tree") throw new Error(`lanes.toml: lane.kernel[${i}].source must be repo, aur, cachyos or tree`);
        return { id: text(r, "id", `lane.kernel[${i}]`), name: text(r, "name", `lane.kernel[${i}]`), kernel: text(r, "kernel", `lane.kernel[${i}]`), channel: text(r, "channel", `lane.kernel[${i}]`), source, package: text(r, "package", `lane.kernel[${i}]`), repos: r.repos === undefined ? [] : list(r, "repos", `lane.kernel[${i}]`) };
      }),
    },
    probe: {
      packages: list(p, "packages", "probe"),
      aur: list(p, "aur", "probe"),
      cachyosPackages: list(p, "cachyos_packages", "probe"),
      cachyos: rows(p, "cachyos", "probe").map((r, i) => ({ id: text(r, "id", `probe.cachyos[${i}]`), url: text(r, "url", `probe.cachyos[${i}]`) })),
      archApi: text(p, "arch_api", "probe"),
      aurRpc: text(p, "aur_rpc", "probe"),
      timeout: parseDuration(text(p, "timeout", "probe")),
      hosts: rows(p, "host", "probe").map((r, i) => ({ id: text(r, "id", `probe.host[${i}]`), ssh: text(r, "ssh", `probe.host[${i}]`), label: text(r, "label", `probe.host[${i}]`) })),
    },
  };
}

export async function loadLanes(path: string): Promise<Lanes> {
  return parseLanes(await Bun.file(path).text());
}
