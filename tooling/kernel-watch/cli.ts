#!/usr/bin/env bun
// watch: poll the kernel sources into a SQLite store, search it, and generate the derived views.
import { join, resolve } from "node:path";
import { loadManifest } from "./manifest.ts";
import { gate } from "./publish.ts";
import { poll } from "./poll.ts";
import { Store } from "./store.ts";
import { defaultOutDir } from "./private.ts";
import { generate } from "./views.ts";
import { fetchLaneRecords, readLaneRecords, renderLanePage } from "./lane.ts";
import { loadLanes } from "./lanes.ts";
import { fetchWslRecords } from "./wsl.ts";
import { buildReport, collect, liveDeps, readProbeReports, recordingDeps, referenceFromStore, renderProbePage } from "./probe.ts";
import { verify } from "./verify.ts";

const HERE = import.meta.dir;
const USAGE = `usage: bun cli.ts <command> [options]
  validate                          check sources.toml
  poll [--only id,id] [--force] [--max-sources N] [--budget SECONDS]
  status                            per-source poll state
  search <text> [--kernel id] [--kind kind] [--limit N]
  versions [--json]                 kernel versions table, closest to mainline first
  build [--now EPOCH]               regenerate docs/aphrody/watch from the store
  gate [--slow-minutes N]           print commit or skip for the regenerated files in --out (versions now, activity every N minutes, default 360)
  verify [--write]                  probe every manifest URL live and record status and shape
  probe <host-id> [--record FIXTURE_DIR]   read-only probe of a lanes.toml host over ssh (never from CI): writes data/probe/<id>.json and probe.md
  wsl-sync --root DIR               copy the newest WSL2 build, boot test and merge experiment records from the build host into data/wsl
  lane-sync --root DIR              copy the latest rc lane records from the build host (read over ssh) into data/lane and regenerate lane.md
options: --lanes PATH  --manifest PATH  --store PATH  --out DIR (defaults: sources.toml next to this file, $WATCH_STORE or .store/watch.sqlite, the git-ignored .private/kernel-watch/watch, or $KERNEL_WATCH_PRIVATE_DIR/watch)`;

const flags = new Map<string, string>();
const positional: string[] = [];
const args = Bun.argv.slice(2);
for (let i = 0; i < args.length; i++) {
  const arg = args[i]!;
  if (arg.startsWith("--")) {
    const next = args[i + 1];
    if (next === undefined || next.startsWith("--")) flags.set(arg.slice(2), "true");
    else (flags.set(arg.slice(2), next), i++);
  } else positional.push(arg);
}
const [command, ...rest] = positional;
const manifestPath = resolve(flags.get("manifest") ?? join(HERE, "sources.toml"));
const storePath = resolve(flags.get("store") ?? Bun.env.WATCH_STORE ?? join(HERE, ".store", "watch.sqlite"));
const outDir = resolve(flags.get("out") ?? defaultOutDir());
const when = (epoch: number | null): string => (epoch ? new Date(epoch * 1000).toISOString().replace(/\.\d+Z$/, "Z") : "-");

async function probeCommand(store: Store): Promise<number> {
  const lanes = await loadLanes(resolve(flags.get("lanes") ?? join(HERE, "lanes.toml")));
  const host = lanes.probe.hosts.find((h) => h.id === rest[0]);
  if (!host) throw new Error(`probe needs a host id from lanes.toml: ${lanes.probe.hosts.map((h) => h.id).join(", ")}`);
  const live = liveDeps();
  const deps = flags.get("record") ? recordingDeps(live, resolve(flags.get("record")!)) : live;
  const collected = await collect(host, lanes.probe, deps);
  for (const e of collected.errors) console.error(`warning: ${e}`);
  const report = buildReport(host, collected, referenceFromStore(store));
  const file = join(outDir, "data", "probe", `${host.id}.json`);
  await Bun.write(file, `${JSON.stringify(report, null, 2)}\n`);
  await Bun.write(join(outDir, "probe.md"), renderProbePage(await readProbeReports(outDir)));
  console.log(JSON.stringify(report, null, 2));
  return 0;
}

async function main(): Promise<number> {
  if (!command || command === "help" || flags.has("help")) {
    console.log(USAGE);
    return command ? 0 : 2;
  }
  const manifest = await loadManifest(manifestPath);
  if (command === "validate") {
    console.log(`ok: ${manifest.kernels.length} kernels, ${manifest.sources.length} sources, ${new Set(manifest.sources.map((s) => s.host)).size} hosts`);
    return 0;
  }
  if (command === "gate") {
    console.log(gate(outDir, Number(flags.get("slow-minutes") ?? 360) * 60));
    return 0;
  }
  if (command === "wsl-sync") {
    const lanes = await loadLanes(resolve(flags.get("lanes") ?? join(HERE, "lanes.toml")));
    const root = flags.get("root");
    if (!root) throw new Error("wsl-sync needs --root DIR, the WSL root on the build host");
    const records = await fetchWslRecords(lanes.lane.ssh, root);
    if (records.build) await Bun.write(join(outDir, "data", "wsl", "build.json"), `${JSON.stringify(records.build, null, 2)}\n`);
    if (records.merge) await Bun.write(join(outDir, "data", "wsl", "merge.json"), `${JSON.stringify(records.merge, null, 2)}\n`);
    console.log(`build: ${records.build?.release ?? "none"}; merge: ${records.merge ? `${records.merge.conflictedFiles} conflicted files` : "none"}`);
    return 0;
  }
  if (command === "lane-sync") {
    const lanes = await loadLanes(resolve(flags.get("lanes") ?? join(HERE, "lanes.toml")));
    const root = flags.get("root");
    if (!root) throw new Error("lane-sync needs --root DIR, the lane root on the build host");
    const records = await fetchLaneRecords(lanes.lane.ssh, root);
    if (records.length === 0) throw new Error("the build host has no lane records yet");
    for (const r of records) await Bun.write(join(outDir, "data", "lane", `${r.id}.json`), `${JSON.stringify(r, null, 2)}\n`);
    await Bun.write(join(outDir, "lane.md"), renderLanePage(await readLaneRecords(outDir)));
    for (const r of records) console.log(`${r.id}: ${r.status} ${r.package} ${r.version} ${r.release}`);
    return 0;
  }
  if (command === "verify") return verify(manifest, join(HERE, "fixtures", "verified.json"), flags.has("write"));

  const store = new Store(storePath);
  try {
    if (command === "probe") return await probeCommand(store);
    switch (command) {
      case "poll": {
        const summary = await poll(store, manifest, {
          ...(flags.get("only") ? { only: flags.get("only")!.split(",") } : {}),
          force: flags.has("force"),
          ...(flags.get("max-sources") ? { maxSources: Number(flags.get("max-sources")) } : {}),
          ...(flags.get("budget") ? { budget: Number(flags.get("budget")) } : {}),
        });
        for (const r of summary.results) console.log(`${r.outcome.padEnd(12)} ${r.source.padEnd(28)} ${String(r.status ?? "-").padEnd(4)} pages=${r.pages} new=${r.inserted} changed=${r.changed}${r.error ? ` error=${r.error}` : ""}`);
        console.log(`polled ${summary.polled} of ${summary.due} due; deferred ${summary.deferred}; failed ${summary.results.filter((r) => r.outcome === "failed").length}`);
        return 0;
      }
      case "status": {
        for (const s of store.allStates()) console.log(`${s.source.padEnd(28)} status=${String(s.lastStatus ?? "-").padEnd(4)} fails=${s.fails} polled=${when(s.lastPolled)} next=${when(s.nextAllowed)} changed=${when(s.lastChanged)}${s.lastError ? ` error=${s.lastError}` : ""}`);
        console.log(JSON.stringify(store.counts()));
        return 0;
      }
      case "search": {
        const query = rest.join(" ");
        if (!query) throw new Error("search needs a query");
        for (const hit of store.search(query, { ...(flags.get("kernel") ? { kernel: flags.get("kernel")! } : {}), ...(flags.get("kind") ? { kind: flags.get("kind")! } : {}), limit: Number(flags.get("limit") ?? 20) })) {
          console.log(`${hit.kernel}/${hit.kind} ${when(hit.time)} ${hit.title}\n    ${hit.url}${hit.snippet ? `\n    ${hit.snippet}` : ""}`);
        }
        return 0;
      }
      case "versions":
      case "build": {
        const now = flags.get("now") ? Number(flags.get("now")) : undefined;
        const out = await generate(store, manifest, { outDir, verifiedPath: join(HERE, "fixtures", "verified.json"), ...(now !== undefined ? { now } : {}) });
        if (command === "versions" && flags.has("json")) console.log(JSON.stringify(out.versions, null, 2));
        else if (command === "versions") console.log(out.versionsTable);
        else console.log(`wrote ${out.files.length} files under ${outDir}`);
        return 0;
      }
      default:
        console.error(`unknown command "${command}"\n${USAGE}`);
        return 2;
    }
  } finally {
    store.close();
  }
}

process.exit(await main().catch((err) => (console.error(err instanceof Error ? err.message : err), 1)));
