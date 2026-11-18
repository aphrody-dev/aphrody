// GitHub REST access through the `gh` CLI (it owns authentication: GH_TOKEN in CI, the keyring on a workstation),
// plus a recorder and a fixture replayer so tests never touch the network.

export interface GhResult {
  status: number;
  json: unknown;
}

export interface Gh {
  request(method: "GET" | "POST", path: string, body?: unknown): Promise<GhResult>;
}

/** Keys that survive `slim`. Everything else in a response is noise for this tool and is dropped before use or recording. */
const KEEP = new Set([
  "id", "name", "full_name", "default_branch", "fork", "archived", "parent", "source", "owner", "login", "visibility",
  "status", "ahead_by", "behind_by", "total_commits", "merge_base_commit", "sha", "commit", "merge_type", "base_branch",
  "message", "html_url", "pushed_at",
]);

export function slim(v: unknown): unknown {
  if (Array.isArray(v)) return v.map(slim);
  if (v && typeof v === "object") {
    const out: Record<string, unknown> = {};
    for (const [k, x] of Object.entries(v)) if (KEEP.has(k)) out[k] = slim(x);
    return out;
  }
  return v;
}

const key = (method: string, path: string) => `${method} ${path}`;
const transient = (status: number, text: string) =>
  status === 0 || status >= 500 || (status === 403 && /rate limit|abuse/i.test(text));

export function ghCli(): Gh {
  return {
    async request(method, path, body) {
      let last: GhResult = { status: 0, json: null };
      for (let attempt = 0; attempt < 4; attempt++) {
        const p = Bun.spawn(["gh", "api", "-X", method, path, ...(body === undefined ? [] : ["--input", "-"])], {
          stdin: body === undefined ? "ignore" : new Blob([JSON.stringify(body)]),
          stdout: "pipe",
          stderr: "pipe",
        });
        const [out, err, code] = await Promise.all([new Response(p.stdout).text(), new Response(p.stderr).text(), p.exited]);
        let json: unknown = null;
        try {
          json = out.trim() ? slim(JSON.parse(out)) : null;
        } catch {
          json = null;
        }
        const status = code === 0 ? 200 : Number(/HTTP (\d{3})/.exec(err)?.[1] ?? 0);
        last = { status, json };
        if (!transient(status, err + out)) return last;
        await Bun.sleep(1500 * 2 ** attempt);
      }
      return last;
    },
  };
}

/** Responses per `METHOD path`, in the order they were received: a merge changes what the same GET returns afterwards. */
export type Recording = Record<string, GhResult[]>;

/** Wraps a client and stores every response so a live run can become a fixture. */
export function recording(inner: Gh, store: Recording): Gh {
  return {
    async request(method, path, body) {
      const r = await inner.request(method, path, body);
      (store[key(method, path)] ??= []).push(r);
      return r;
    },
  };
}

/** Replays a recording in order; the last response of a key repeats. A request that is not in it is a test bug and throws. */
export function fixture(data: Recording): Gh {
  const seen = new Map<string, number>();
  return {
    async request(method, path) {
      const k = key(method, path);
      const list = data[k];
      if (!list?.length) throw new Error(`fixture has no response for ${k}`);
      const i = seen.get(k) ?? 0;
      seen.set(k, i + 1);
      return list[Math.min(i, list.length - 1)]!;
    },
  };
}

// ---- typed calls -------------------------------------------------------------------------------------------------

export interface RepoInfo {
  fullName: string;
  defaultBranch: string;
  fork: boolean;
  archived: boolean;
  parent?: string;
  source?: string;
}

interface RepoJson {
  full_name: string;
  default_branch: string;
  fork: boolean;
  archived: boolean;
  parent?: { full_name: string };
  source?: { full_name: string };
}

export async function getRepo(gh: Gh, slug: string): Promise<RepoInfo | null> {
  const r = await gh.request("GET", `repos/${slug}`);
  if (r.status === 404) return null;
  if (r.status !== 200) throw new Error(`GET repos/${slug}: HTTP ${r.status}`);
  const j = r.json as RepoJson;
  return {
    fullName: j.full_name,
    defaultBranch: j.default_branch,
    fork: j.fork,
    archived: j.archived,
    parent: j.parent?.full_name,
    source: j.source?.full_name,
  };
}

/** Head commit of a branch, or null when the branch does not exist. */
export async function branchHead(gh: Gh, slug: string, branch: string): Promise<string | null> {
  const r = await gh.request("GET", `repos/${slug}/branches/${encodeURIComponent(branch).replace(/%2F/g, "/")}`);
  if (r.status === 404) return null;
  if (r.status !== 200) throw new Error(`GET branches/${branch} of ${slug}: HTTP ${r.status}`);
  return (r.json as { commit: { sha: string } }).commit.sha;
}

export interface Divergence {
  /** Commits on the fork branch that upstream does not have. */
  ahead: number;
  /** Commits on the upstream branch that the fork does not have. */
  behind: number;
}

/** Compares the fork branch with the same branch of its upstream, through the fork (same network). */
export async function divergence(gh: Gh, fork: string, upstream: string, branch: string): Promise<Divergence | null> {
  const base = `${upstream.split("/")[0]}:${branch}`;
  const head = `${fork.split("/")[0]}:${branch}`;
  const r = await gh.request("GET", `repos/${fork}/compare/${base}...${head}?per_page=1`);
  if (r.status !== 200) return null;
  const j = r.json as { ahead_by: number; behind_by: number };
  return { ahead: j.ahead_by, behind: j.behind_by };
}

export type MergeOutcome =
  | { kind: "ok"; mergeType: "fast-forward" | "merge" | "none"; message: string }
  | { kind: "conflict"; message: string }
  | { kind: "error"; status: number; message: string };

/** POST merge-upstream: fast-forwards, or merges upstream into a fork that carries commits. Leaves the branch untouched on conflict. */
export async function mergeUpstream(gh: Gh, fork: string, branch: string): Promise<MergeOutcome> {
  const r = await gh.request("POST", `repos/${fork}/merge-upstream`, { branch });
  const j = (r.json ?? {}) as { message?: string; merge_type?: string };
  const message = j.message ?? "";
  if (r.status === 200) return { kind: "ok", mergeType: (j.merge_type ?? "none") as "fast-forward" | "merge" | "none", message };
  if (r.status === 409) return { kind: "conflict", message };
  return { kind: "error", status: r.status, message };
}
