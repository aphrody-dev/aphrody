// forks.toml loader and validator. Pure: no network, no git.
import { TOML } from "bun";

export const ROLES = [
  "compiler", "backend", "codegen", "std", "package-manager", "installer", "ide", "devtool", "registry",
  "docs", "governance", "infra", "wasm", "consumer",
] as const;
export type Role = (typeof ROLES)[number];

export type Policy = "merge" | "ff-only";

export interface Fork {
  /** Path of the submodule inside the monorepo: `<group>/<name>`. */
  path: string;
  /** Repository name inside the fork organisation. */
  name: string;
  /** Upstream `owner/repo`. */
  upstream: string;
  /** Branch that is tracked, synced and pinned. */
  branch: string;
  /** Default branch of the fork when it differs from `branch`. */
  defaultBranch: string;
  group: string;
  role: Role;
  /** Path of this repository inside rust-lang/rust when rust carries it as a git submodule. */
  rustSubmodule?: string;
  /** Path of this repository inside rust-lang/rust when rust carries a josh subtree copy. */
  rustSubtree?: string;
  /** `merge` also merges upstream into a fork that carries commits; `ff-only` only fast-forwards. */
  policy: Policy;
  note?: string;
}

export interface Manifest {
  org: string;
  /** Organisation of the repository that carries the submodules, when it is not the fork organisation. */
  consumer: string;
  groups: Record<string, string>;
  forks: Fork[];
  notForked: { upstream: string; reason: string }[];
}

type Raw = Record<string, unknown>;

function str(o: Raw, k: string, where: string): string {
  const v = o[k];
  if (typeof v !== "string" || v === "") throw new Error(`${where}: "${k}" must be a non-empty string`);
  return v;
}

export function parseManifest(text: string): Manifest {
  const doc = TOML.parse(text) as Raw;
  const meta = (doc.meta ?? {}) as Raw;
  const org = str(meta, "org", "[meta]");
  const consumer = meta.consumer === undefined ? org : str(meta, "consumer", "[meta]");
  const groups = (doc.groups ?? {}) as Record<string, string>;
  const forks: Fork[] = [];
  for (const [i, e] of ((doc.fork ?? []) as Raw[]).entries()) {
    const where = `[[fork]] #${i + 1} (${String(e.path ?? "?")})`;
    const path = str(e, "path", where);
    const role = str(e, "role", where) as Role;
    if (!ROLES.includes(role)) throw new Error(`${where}: unknown role "${role}"`);
    const branch = str(e, "branch", where);
    const policy = (e.policy ?? "merge") as Policy;
    if (policy !== "merge" && policy !== "ff-only") throw new Error(`${where}: unknown policy "${policy}"`);
    const upstream = str(e, "upstream", where);
    if (!/^[\w.-]+\/[\w.-]+$/.test(upstream)) throw new Error(`${where}: upstream must be owner/repo`);
    const [group, name] = path.split("/");
    if (!group || !name || path.split("/").length !== 2) throw new Error(`${where}: path must be <group>/<name>`);
    forks.push({
      path,
      name: typeof e.name === "string" ? e.name : name,
      upstream,
      branch,
      defaultBranch: typeof e.default_branch === "string" ? e.default_branch : branch,
      group,
      role,
      rustSubmodule: typeof e.rust_submodule === "string" ? e.rust_submodule : undefined,
      rustSubtree: typeof e.rust_subtree === "string" ? e.rust_subtree : undefined,
      policy,
      note: typeof e.note === "string" ? e.note : undefined,
    });
  }
  const notForked = ((doc.not_forked ?? []) as Raw[]).map((e, i) => ({
    upstream: str(e, "upstream", `[[not_forked]] #${i + 1}`),
    reason: str(e, "reason", `[[not_forked]] #${i + 1}`),
  }));
  const m: Manifest = { org, consumer, groups, forks, notForked };
  const problems = validate(m);
  if (problems.length) throw new Error(`forks.toml is invalid:\n- ${problems.join("\n- ")}`);
  return m;
}

/** Structural checks that need no network. */
export function validate(m: Manifest): string[] {
  const out: string[] = [];
  const seen = (what: string, vals: string[]) => {
    for (const v of new Set(vals.filter((x, i) => vals.indexOf(x) !== i))) out.push(`duplicate ${what}: ${v}`);
  };
  seen("path", m.forks.map((f) => f.path));
  seen("fork name", m.forks.map((f) => f.name.toLowerCase()));
  seen("upstream", m.forks.map((f) => f.upstream.toLowerCase()));
  for (const f of m.forks) {
    if (!(f.group in m.groups)) out.push(`${f.path}: group "${f.group}" is not declared under [groups]`);
    if (f.rustSubmodule && f.rustSubtree) out.push(`${f.path}: rust_submodule and rust_subtree are exclusive`);
    if (f.path !== `${f.group}/${f.name}`) out.push(`${f.path}: path must be ${f.group}/${f.name}`);
  }
  return out;
}

export async function loadManifest(file: string): Promise<Manifest> {
  return parseManifest(await Bun.file(file).text());
}

export const isRustSubmodule = (f: Fork): boolean => f.rustSubmodule !== undefined;

/** `.gitmodules` text for a manifest: relative URLs resolve against the monorepo's own remote, one level up for a fork in the same organisation, two levels up and down for another one. */
export function renderGitmodules(m: Manifest): string {
  const base = m.consumer === m.org ? ".." : `../../${m.org}`;
  return m.forks
    .map((f) => `[submodule "${f.path}"]\n\tpath = ${f.path}\n\turl = ${base}/${f.name}.git\n`)
    .join("");
}
