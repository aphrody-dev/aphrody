// Compares forks.toml with the live GitHub state and with the pins in the index.
import { branchHead, divergence, getRepo, type Divergence, type Gh, type RepoInfo } from "./gh.ts";
import { renderGitmodules, type Fork, type Manifest } from "./manifest.ts";

export interface ForkStatus {
  fork: Fork;
  repo: RepoInfo | null;
  head: string | null;
  div: Divergence | null;
  pinned: string | null;
  /** Disagreements between the manifest and reality. Empty means the entry is sound. */
  problems: string[];
}

const same = (a?: string, b?: string) => !!a && !!b && a.toLowerCase() === b.toLowerCase();

export function judge(org: string, f: Fork, repo: RepoInfo | null, head: string | null, div: Divergence | null): string[] {
  const slug = `${org}/${f.name}`;
  if (!repo) return [`${slug} does not exist`];
  const p: string[] = [];
  if (!repo.fork) p.push(`${slug} is not a fork`);
  else if (!same(repo.parent, f.upstream)) p.push(`parent is ${repo.parent ?? "none"}, manifest says ${f.upstream}`);
  if (repo.archived) p.push(`${slug} is archived`);
  if (repo.defaultBranch !== f.defaultBranch) p.push(`default branch is ${repo.defaultBranch}, manifest says ${f.defaultBranch}`);
  if (!head) p.push(`branch ${f.branch} does not exist on ${slug}`);
  else if (!div) p.push(`cannot compare ${f.branch} with ${f.upstream}`);
  return p;
}

async function pool<T, R>(items: T[], size: number, fn: (x: T) => Promise<R>): Promise<R[]> {
  const out = new Array<R>(items.length);
  let next = 0;
  await Promise.all(
    Array.from({ length: Math.min(size, items.length) }, async () => {
      while (next < items.length) {
        const i = next++;
        out[i] = await fn(items[i]!);
      }
    }),
  );
  return out;
}

export async function inspectOne(gh: Gh, org: string, f: Fork, pins: Map<string, string>): Promise<ForkStatus> {
  const slug = `${org}/${f.name}`;
  const repo = await getRepo(gh, slug);
  const head = repo ? await branchHead(gh, slug, f.branch) : null;
  const div = head ? await divergence(gh, slug, f.upstream, f.branch) : null;
  return { fork: f, repo, head, div, pinned: pins.get(f.path) ?? null, problems: judge(org, f, repo, head, div) };
}

export async function inspect(gh: Gh, m: Manifest, pins: Map<string, string>, only?: Set<string>, concurrency = 8): Promise<ForkStatus[]> {
  const list = m.forks.filter((f) => !only || only.has(f.name) || only.has(f.path));
  return pool(list, concurrency, (f) => inspectOne(gh, m.org, f, pins));
}

/** Offline drift: .gitmodules text and gitlinks against the manifest. */
export function layoutProblems(m: Manifest, gitmodules: string, pins: Map<string, string>): string[] {
  const p: string[] = [];
  if (gitmodules !== renderGitmodules(m)) p.push(".gitmodules does not match forks.toml (run: bun tools/submodules.ts --write)");
  const want = new Set(m.forks.map((f) => f.path));
  const unpinned = m.forks.filter((f) => !pins.has(f.path)).map((f) => f.path);
  if (unpinned.length) p.push(`${unpinned.length} gitlink(s) missing: ${unpinned.join(", ")} (run: bun tools/submodules.ts --write)`);
  for (const path of pins.keys()) if (!want.has(path)) p.push(`gitlink ${path} is not in forks.toml`);
  return p;
}
