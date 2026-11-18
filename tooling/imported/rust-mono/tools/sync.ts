#!/usr/bin/env bun
// Upstream sync: merge-upstream into every fork, move the superproject's gitlink to the fork's new head, append a dated entry to the
// ledger. Dry run unless --apply. Idempotent: a second run with nothing new writes nothing.
import { mkdir, readdir } from "node:fs/promises";
import { dirname, relative, resolve } from "node:path";
import { fixture, ghCli, recording, type Gh, type Recording } from "./lib/gh.ts";
import { commit, gitlinks, setGitlink, stage, stagedPaths } from "./lib/git.ts";
import { loadManifest } from "./lib/manifest.ts";
import { ledgerFile, ledgerWorthy, parseLedger, renderLedger, syncAll, type LedgerEntry, type Outcome } from "./lib/sync.ts";

const USAGE = `usage: bun tools/sync.ts [--apply] [--commit] [--only a,b] [--manifest forks.toml] [--ledger ledger]
                         [--fixture file | --record file]
  (default)  dry run: print what would happen, write nothing
  --apply    call merge-upstream on the forks, move the gitlinks in the index, append to the ledger
  --commit   with --apply: commit the bumped gitlinks and the ledger (message: chore(sync): ...)
  exit status: 1 when any fork is in the "error" state, 0 otherwise (conflicts and diverged forks are recorded, not errors)`;

interface Options {
  apply: boolean;
  commit: boolean;
  only?: Set<string>;
  manifest: string;
  ledger: string;
  fixture?: string;
  record?: string;
}

export function parseArgs(argv: string[]): Options | "help" {
  const o: Options = { apply: false, commit: false, manifest: "forks.toml", ledger: "ledger" };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i]!;
    const val = () => argv[++i] ?? (() => { throw new Error(`${a} needs a value`); })();
    if (a === "--help" || a === "-h") return "help";
    else if (a === "--apply") o.apply = true;
    else if (a === "--commit") o.commit = true;
    else if (a === "--only") o.only = new Set(val().split(","));
    else if (a === "--manifest") o.manifest = val();
    else if (a === "--ledger") o.ledger = val();
    else if (a === "--fixture") o.fixture = val();
    else if (a === "--record") o.record = val();
    else throw new Error(`unknown argument ${a}\n${USAGE}`);
  }
  if (o.commit && !o.apply) throw new Error("--commit needs --apply");
  return o;
}

const s9 = (s: string | null) => (s ? s.slice(0, 9) : "-");

export function renderOutcomes(outs: Outcome[]): string {
  const head = ["path", "status", "ahead", "behind", "merged", "pin"];
  const rows = outs.map(({ entry: e }) => [
    e.fork, e.status, String(e.ahead ?? "?"), String(e.behind ?? "?"), String(e.merged),
    e.old === e.new ? s9(e.new) : `${s9(e.old)} -> ${s9(e.new)}`,
  ]);
  const w = head.map((h, i) => Math.max(h.length, ...rows.map((r) => r[i]!.length)));
  const line = (c: string[]) => c.map((x, i) => (i === c.length - 1 ? x : x.padEnd(w[i]!))).join("  ");
  return [line(head), ...rows.map(line)].join("\n");
}

async function readLedger(dir: string): Promise<LedgerEntry[]> {
  const files = (await readdir(dir).catch(() => [] as string[])).filter((f) => f.endsWith(".jsonl")).sort();
  const all: LedgerEntry[] = [];
  for (const f of files) all.push(...parseLedger(await Bun.file(resolve(dir, f)).text()));
  return all;
}

export function commitMessage(entries: LedgerEntry[]): string {
  const moved = entries.filter((e) => e.old !== e.new);
  const subject = `chore(sync): move ${moved.length} fork pin${moved.length === 1 ? "" : "s"} to upstream`;
  const lines = entries.map((e) => `${e.fork}: ${s9(e.old)}..${s9(e.new)} ${e.status}${e.merged ? ` (+${e.merged} upstream)` : ""}`);
  return `${subject}\n\n${lines.join("\n")}\n`;
}

export async function run(argv: string[], gh?: Gh, now: () => Date = () => new Date()): Promise<number> {
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
  if (opts.commit) {
    const staged = await stagedPaths(root);
    if (staged.length) throw new Error(`refusing to commit: the index already has staged paths: ${staged.join(", ")}`);
  }
  const ledgerDir = resolve(root, opts.ledger);
  const previous = await readLedger(ledgerDir);
  const pins = await gitlinks(root);
  const runAt = now(); // one timestamp per run: its ledger entries group together
  const outs = await syncAll(client, m, pins, { apply: opts.apply, now: () => runAt }, opts.only);
  if (opts.record) await Bun.write(opts.record, JSON.stringify(store, null, 1) + "\n");

  console.log(renderOutcomes(outs));
  const worthy = ledgerWorthy(outs, previous);
  const errors = outs.filter((o) => o.entry.status === "error");
  for (const o of errors) console.log(`error: ${o.entry.fork}: ${o.entry.message}`);
  console.log(`\n${outs.length} forks, ${worthy.length} ledger entr${worthy.length === 1 ? "y" : "ies"}, ${errors.length} error(s)${opts.apply ? "" : " (dry run, nothing written)"}`);
  if (!opts.apply) return errors.length ? 1 : 0;

  const bumped = outs.filter((o) => o.bump);
  for (const o of bumped) await setGitlink(root, o.entry.fork, o.bump!);
  if (worthy.length) {
    await mkdir(ledgerDir, { recursive: true });
    const file = ledgerFile(ledgerDir, runAt);
    const old = await Bun.file(file).text().catch(() => "");
    await Bun.write(file, old + renderLedger(worthy));
  }
  if (opts.commit && (bumped.length || worthy.length)) {
    if (worthy.length) await stage(root, [relative(root, ledgerFile(ledgerDir, runAt)).replaceAll("\\", "/")]);
    console.log(`committed ${(await commit(root, commitMessage(worthy.length ? worthy : bumped.map((o) => o.entry)))).slice(0, 9)}`);
  }
  return errors.length ? 1 : 0;
}

if (import.meta.main) {
  try {
    process.exit(await run(Bun.argv.slice(2)));
  } catch (e) {
    console.error((e as Error).message);
    process.exit(2);
  }
}
