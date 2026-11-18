#!/usr/bin/env bun
// Reads forks.toml and checks every entry against the live GitHub API: the fork exists, its parent and default branch match,
// the tracked branch exists. Prints ahead/behind for each fork. --check exits non-zero on drift.
import { dirname, resolve } from "node:path";
import { fixture, ghCli, recording, type Gh, type Recording } from "./lib/gh.ts";
import { gitlinks } from "./lib/git.ts";
import { inspect, layoutProblems, type ForkStatus } from "./lib/inspect.ts";
import { loadManifest } from "./lib/manifest.ts";

const USAGE = `usage: bun tools/forks.ts [--check] [--json] [--only a,b] [--max-behind N] [--manifest forks.toml]
                          [--fixture file | --record file]
  (default)       table of every fork: tracked branch, ahead/behind upstream, pin, problems
  --check         exit 1 when forks.toml, .gitmodules, the pins or the live state disagree
  --max-behind N  with --check, also fail when a fork is more than N commits behind upstream
  --fixture FILE  replay a recorded API session instead of calling GitHub (tests)
  --record FILE   write the API responses of this run to FILE (makes a fixture)`;

export interface Options {
  check: boolean;
  json: boolean;
  only?: Set<string>;
  maxBehind?: number;
  manifest: string;
  fixture?: string;
  record?: string;
}

export function parseArgs(argv: string[]): Options | "help" {
  const o: Options = { check: false, json: false, manifest: "forks.toml" };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i]!;
    const val = () => argv[++i] ?? (() => { throw new Error(`${a} needs a value`); })();
    if (a === "--help" || a === "-h") return "help";
    else if (a === "--check") o.check = true;
    else if (a === "--json") o.json = true;
    else if (a === "--only") o.only = new Set(val().split(","));
    else if (a === "--max-behind") o.maxBehind = Number(val());
    else if (a === "--manifest") o.manifest = val();
    else if (a === "--fixture") o.fixture = val();
    else if (a === "--record") o.record = val();
    else throw new Error(`unknown argument ${a}\n${USAGE}`);
  }
  return o;
}

const short = (s: string | null) => (s ? s.slice(0, 9) : "-");

export function renderTable(rows: ForkStatus[]): string {
  const head = ["path", "branch", "ahead", "behind", "pin", "head", "state"];
  const body = rows.map((r) => [
    r.fork.path,
    r.fork.branch,
    r.div ? String(r.div.ahead) : "?",
    r.div ? String(r.div.behind) : "?",
    short(r.pinned),
    short(r.head),
    r.problems.length ? `DRIFT: ${r.problems.join("; ")}` : r.pinned && r.head && r.pinned !== r.head ? "pin behind head" : "ok",
  ]);
  const w = head.map((h, i) => Math.max(h.length, ...body.map((b) => b[i]!.length)));
  const line = (c: string[]) => c.map((x, i) => (i === 6 ? x : x.padEnd(w[i]!))).join("  ");
  return [line(head), ...body.map(line)].join("\n");
}

export async function run(argv: string[], gh?: Gh): Promise<number> {
  const opts = parseArgs(argv);
  if (opts === "help") {
    console.log(USAGE);
    return 0;
  }
  const manifestPath = resolve(opts.manifest);
  const root = dirname(manifestPath);
  const m = await loadManifest(manifestPath);
  const store: Recording = {};
  let client = gh;
  if (!client) {
    client = opts.fixture ? fixture(JSON.parse(await Bun.file(opts.fixture).text())) : ghCli();
    if (opts.record) client = recording(client, store);
  }
  const pins = await gitlinks(root);
  const rows = await inspect(client, m, pins, opts.only);
  if (opts.record) await Bun.write(opts.record, JSON.stringify(store, null, 1) + "\n");

  const gm = await Bun.file(resolve(root, ".gitmodules")).text().catch(() => "");
  const layout = opts.only ? [] : layoutProblems(m, gm, pins);
  const behind = opts.maxBehind === undefined ? [] : rows.filter((r) => r.div && r.div.behind > opts.maxBehind!).map((r) => `${r.fork.path} is ${r.div!.behind} commits behind upstream`);
  const drift = [...layout, ...rows.flatMap((r) => r.problems.map((p) => `${r.fork.path}: ${p}`)), ...behind];

  if (opts.json) console.log(JSON.stringify({ org: m.org, forks: rows.map((r) => ({ path: r.fork.path, upstream: r.fork.upstream, branch: r.fork.branch, ahead: r.div?.ahead ?? null, behind: r.div?.behind ?? null, pinned: r.pinned, head: r.head, problems: r.problems })), drift }, null, 1));
  else {
    console.log(renderTable(rows));
    console.log(`\n${rows.length} forks, ${drift.length} drift finding(s)`);
    for (const d of drift) console.log(`  ${d}`);
  }
  return opts.check && drift.length ? 1 : 0;
}

if (import.meta.main) {
  try {
    process.exit(await run(Bun.argv.slice(2)));
  } catch (e) {
    console.error((e as Error).message);
    process.exit(2);
  }
}
