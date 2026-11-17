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
export interface Lanes {
  probe: ProbeConfig;
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
const rows = (t: Table, key: string, where: string): Table[] => {
  const v = t[key];
  if (!Array.isArray(v) || v.length === 0) throw new Error(`lanes.toml: ${where}.${key} must be a non-empty array of tables`);
  return v.map((x, i) => table(x, `${where}.${key}[${i}]`));
};

export function parseLanes(source: string): Lanes {
  const root = table(Bun.TOML.parse(source), "root");
  const p = table(root.probe, "probe");
  return {
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
