import { afterEach, beforeEach, expect, test } from "bun:test";
import { mkdtemp, readdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fixture, type Gh, type GhResult, type Recording } from "./lib/gh.ts";
import { loadManifest } from "./lib/manifest.ts";
import { gitlinks, setGitlink } from "./lib/git.ts";
import { isRepeat, parseLedger, type LedgerEntry } from "./lib/sync.ts";
import { commitMessage, run } from "./sync.ts";

// A stateful stand-in for GitHub: merge-upstream really moves heads, so a second run sees the new state.
interface Repo { parent: string; head: string; ahead: number; behind: number; conflict?: boolean; mergeStatus?: number; fork?: boolean }
const sha = (c: string) => c.repeat(40);

function fakeGithub(repos: Record<string, Repo>): { gh: Gh; posts: string[] } {
  const posts: string[] = [];
  const ok = (json: unknown): GhResult => ({ status: 200, json });
  const gh: Gh = {
    async request(method, path) {
      const hit = /^repos\/([^/]+\/[^/?]+)(?:\/([^?]+))?/.exec(path)!;
      const r = repos[hit[1]!];
      if (!r) return { status: 404, json: { message: "Not Found" } };
      const rest = hit[2] ?? "";
      if (rest === "") return ok({ full_name: hit[1], default_branch: "main", fork: r.fork ?? true, archived: false, parent: { full_name: r.parent } });
      if (rest.startsWith("branches/")) return ok({ commit: { sha: r.head } });
      if (rest.startsWith("compare/")) return ok({ ahead_by: r.ahead, behind_by: r.behind });
      if (rest === "merge-upstream" && method === "POST") {
        posts.push(hit[1]!);
        if (r.mergeStatus) return { status: r.mergeStatus, json: { message: "boom" } };
        if (r.conflict) return { status: 409, json: { message: "There are merge conflicts" } };
        const ff = r.ahead === 0;
        r.head = ff ? sha("f") : sha("e");
        if (!ff) r.ahead += 1;
        r.behind = 0;
        return ok({ message: ff ? "Successfully fetched and fast-forwarded from upstream." : "Successfully merged upstream.", merge_type: ff ? "fast-forward" : "merge", base_branch: "u:main" });
      }
      throw new Error(`unexpected ${method} ${path}`);
    },
  };
  return { gh, posts };
}

const forksToml = `[meta]
org = "org"
[groups]
g = "x"
${["ff", "mrg", "conf", "div", "cur", "pin", "bad", "boom"].map((n) => `[[fork]]
path = "g/${n}"
name = "${n}"
upstream = "up/${n}"
branch = "main"
role = "std"
${n === "div" ? 'policy = "ff-only"\n' : ""}`).join("\n")}`;

let dir: string;
let repos: Record<string, Repo>;
const fixed = () => new Date("2026-11-17T12:00:00Z");

beforeEach(async () => {
  dir = await mkdtemp(join(tmpdir(), "rust-mono-sync-"));
  Bun.spawnSync(["git", "init", "-q", "-b", "main"], { cwd: dir });
  // Repository-local identity: CI runners have no global one.
  Bun.spawnSync(["git", "config", "user.name", "test"], { cwd: dir });
  Bun.spawnSync(["git", "config", "user.email", "test@example.com"], { cwd: dir });
  await Bun.write(join(dir, "forks.toml"), forksToml);
  repos = {
    "org/ff": { parent: "up/ff", head: sha("1"), ahead: 0, behind: 5 },
    "org/mrg": { parent: "up/mrg", head: sha("2"), ahead: 2, behind: 3 },
    "org/conf": { parent: "up/conf", head: sha("3"), ahead: 1, behind: 4, conflict: true },
    "org/div": { parent: "up/div", head: sha("4"), ahead: 9, behind: 1 },
    "org/cur": { parent: "up/cur", head: sha("5"), ahead: 0, behind: 0 },
    "org/pin": { parent: "up/pin", head: sha("7"), ahead: 0, behind: 0 },
    "org/bad": { parent: "someone/else", head: sha("8"), ahead: 0, behind: 0 },
    "org/boom": { parent: "up/boom", head: sha("9"), ahead: 0, behind: 2, mergeStatus: 422 },
  };
  for (const [n, h] of [["ff", "1"], ["mrg", "2"], ["conf", "3"], ["div", "4"], ["cur", "5"], ["pin", "6"], ["bad", "8"], ["boom", "9"]] as const) {
    await setGitlink(dir, `g/${n}`, sha(h));
  }
});
afterEach(() => rm(dir, { recursive: true, force: true }));

const args = (...a: string[]) => ["--manifest", join(dir, "forks.toml"), ...a];
const quiet = async (fn: () => Promise<number>) => {
  const log = console.log;
  console.log = () => {};
  try {
    return await fn();
  } finally {
    console.log = log;
  }
};
const ledger = async () => {
  const files = (await readdir(join(dir, "ledger")).catch(() => [])).sort();
  const parts = await Promise.all(files.map(async (f) => parseLedger(await Bun.file(join(dir, "ledger", f)).text())));
  return parts.flat();
};

test("dry run calls no merge, writes no ledger and moves no gitlink", async () => {
  const { gh, posts } = fakeGithub(repos);
  const before = await gitlinks(dir);
  expect(await quiet(() => run(args(), gh, fixed))).toBe(1); // "bad" is an error
  expect(posts).toEqual([]);
  expect(await gitlinks(dir)).toEqual(before);
  expect(await ledger()).toEqual([]);
});

test("apply: fast-forward, merge, conflict, diverged, pin-only, current and error each get the right status and pin", async () => {
  const { gh, posts } = fakeGithub(repos);
  expect(await quiet(() => run(args("--apply"), gh, fixed))).toBe(1);
  expect(posts.sort()).toEqual(["org/boom", "org/conf", "org/ff", "org/mrg"]);
  const by = Object.fromEntries((await ledger()).map((e) => [e.fork, e]));
  expect(by["g/ff"]).toMatchObject({ status: "fast-forward", merged: 5, old: sha("1"), new: sha("f"), ts: "2026-11-17T12:00:00.000Z" });
  expect(by["g/mrg"]).toMatchObject({ status: "merged", merged: 3, old: sha("2"), new: sha("e") });
  expect(by["g/conf"]).toMatchObject({ status: "conflict", merged: 0, old: sha("3"), new: sha("3"), message: "There are merge conflicts" });
  expect(by["g/div"]).toMatchObject({ status: "diverged", merged: 0, ahead: 9, behind: 1 });
  expect(by["g/pin"]).toMatchObject({ status: "pin", old: sha("6"), new: sha("7") });
  expect(by["g/bad"]).toMatchObject({ status: "error" });
  expect(by["g/boom"]).toMatchObject({ status: "error" });
  expect(by["g/boom"]!.message).toContain("HTTP 422");
  expect(by["g/cur"]).toBeUndefined();
  const pins = await gitlinks(dir);
  expect(pins.get("g/ff")).toBe(sha("f"));
  expect(pins.get("g/mrg")).toBe(sha("e"));
  expect(pins.get("g/conf")).toBe(sha("3"));
  expect(pins.get("g/div")).toBe(sha("4"));
  expect(pins.get("g/pin")).toBe(sha("7"));
  expect(pins.get("g/bad")).toBe(sha("8"));
  expect(repos["org/conf"]!.head).toBe(sha("3"));
  expect(repos["org/div"]!.head).toBe(sha("4"));
});

test("apply is idempotent: a second run records nothing new and changes no pin", async () => {
  const { gh, posts } = fakeGithub(repos);
  await quiet(() => run(args("--apply"), gh, fixed));
  const first = await Bun.file(join(dir, "ledger", "2026-11.jsonl")).text();
  const pins = await gitlinks(dir);
  posts.length = 0;
  await quiet(() => run(args("--apply"), gh, () => new Date("2026-11-18T12:00:00Z")));
  expect(await Bun.file(join(dir, "ledger", "2026-11.jsonl")).text()).toBe(first);
  expect(await gitlinks(dir)).toEqual(pins);
  expect(posts.sort()).toEqual(["org/boom", "org/conf"]); // still retried, still recorded once
});

test("--commit commits the gitlinks and the ledger in one commit", async () => {
  Bun.spawnSync(["git", "commit", "-q", "--allow-empty", "-m", "init"], { cwd: dir });
  const { gh } = fakeGithub(repos);
  await quiet(() => run(args("--apply", "--commit"), gh, fixed));
  const log = Bun.spawnSync(["git", "log", "-1", "--format=%s%n%b"], { cwd: dir }).stdout.toString();
  expect(log).toContain("chore(sync): move");
  expect(log).toMatch(/g\/ff: 111111111\.\.fffffffff fast-forward \(\+5 upstream\)/);
  const tracked = Bun.spawnSync(["git", "ls-files", "ledger"], { cwd: dir }).stdout.toString();
  expect(tracked).toContain("ledger/2026-11.jsonl");
  expect(Bun.spawnSync(["git", "diff", "--cached", "--name-only"], { cwd: dir }).stdout.toString()).toBe("");
  const ff = Bun.spawnSync(["git", "ls-tree", "HEAD", "g/ff"], { cwd: dir }).stdout.toString();
  expect(ff).toContain(sha("f"));
});

test("--commit refuses to run when the index already has staged paths, before touching anything", async () => {
  Bun.spawnSync(["git", "add", "forks.toml"], { cwd: dir });
  const { gh, posts } = fakeGithub(repos);
  await expect(run(args("--apply", "--commit"), gh, fixed)).rejects.toThrow(/already has staged paths: forks.toml/);
  expect(posts).toEqual([]);
  expect(await ledger()).toEqual([]);
});

test("isRepeat only matches the newest entry of the same fork", () => {
  const e = (status: LedgerEntry["status"], behind: number): LedgerEntry => ({ ts: "t", fork: "g/a", repo: "o/a", upstream: "u/a", branch: "main", old: "a", new: "a", merged: 0, ahead: 1, behind, status });
  expect(isRepeat(e("conflict", 4), [e("conflict", 4)])).toBe(true);
  expect(isRepeat(e("conflict", 5), [e("conflict", 4)])).toBe(false);
  expect(isRepeat(e("conflict", 4), [e("conflict", 4), e("merged", 0)])).toBe(false);
  expect(isRepeat(e("conflict", 4), [])).toBe(false);
});

test("commit message lists every fork and stays within 72 characters on the subject", () => {
  const msg = commitMessage([{ ts: "t", fork: "g/a", repo: "o/a", upstream: "u/a", branch: "main", old: sha("a"), new: sha("b"), merged: 7, ahead: 0, behind: 7, status: "fast-forward" }]);
  expect(msg.split("\n")[0]!.length).toBeLessThanOrEqual(72);
  expect(msg).toContain("g/a: aaaaaaaaa..bbbbbbbbb fast-forward (+7 upstream)");
});

test("replaying the recorded real run (2026-11-17) reproduces its committed ledger entries", async () => {
  const root = join(import.meta.dir, "..");
  const recorded = JSON.parse(await Bun.file(join(root, "tools/fixtures/sync-run.json")).text()) as Recording;
  const all = parseLedger(await Bun.file(join(root, "ledger/2026-11.jsonl")).text());
  const first = all.filter((e) => e.ts === all[0]!.ts);
  expect(first.length).toBeGreaterThan(5);

  // Only the forks the recording knows: forks added to the manifest later have no recorded responses.
  const full = await Bun.file(join(root, "forks.toml")).text();
  const manifest = await loadManifest(join(root, "forks.toml"));
  const recordedName = (f: { name: string; branch: string }) => `GET repos/${manifest.org}/${f.name}/branches/${f.branch}` in recorded;
  const unknown = manifest.forks.filter((f) => !recordedName(f));
  const [header, ...blocks] = full.split(/\n(?=\[\[fork\]\])/);
  const kept = blocks.filter((block) => !unknown.some((f) => block.includes(`\nname = "${f.name}"\n`)));
  await Bun.write(join(dir, "forks.toml"), [header, ...kept].join("\n"));
  for (const f of manifest.forks.filter(recordedName)) {
    const before = recorded[`GET repos/${manifest.org}/${f.name}/branches/${f.branch}`]![0]!.json as { commit: { sha: string } };
    await setGitlink(dir, f.path, before.commit.sha);
  }
  expect(await quiet(() => run(args("--apply"), fixture(recorded), () => new Date(all[0]!.ts)))).toBe(0);
  expect(await ledger()).toEqual(first);
  expect(first.map((e) => e.status).sort()).toEqual(["diverged", "fast-forward", "fast-forward", "fast-forward", "fast-forward", "fast-forward", "fast-forward", "merged"]);
});
