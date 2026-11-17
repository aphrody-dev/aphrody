#!/usr/bin/env bun
// watch: poll the kernel sources into a SQLite store, search it, and generate the derived views.
import { join, resolve } from "node:path";
import { loadManifest } from "./manifest.ts";
import { gate } from "./publish.ts";
import { poll } from "./poll.ts";
import { Store } from "./store.ts";
import { generate } from "./views.ts";
import { verify } from "./verify.ts";

const HERE = import.meta.dir;
const USAGE = `usage: bun cli.ts <command> [options]
  validate                          check sources.toml
  poll [--only id,id] [--force] [--max-sources N] [--budget SECONDS]
  status                            per-source poll state
  search <text> [--kernel id] [--kind kind] [--limit N]
  versions [--json]                 kernel versions table, closest to mainline first
  build [--now EPOCH]               regenerate docs/watch from the store
  gate [--slow-minutes N]           print commit or skip for the regenerated files in --out (versions now, activity every N minutes, default 360)
  verify [--write]                  probe every manifest URL live and record status and shape
options: --manifest PATH  --store PATH  --out DIR (defaults: sources.toml next to this file, $WATCH_STORE or .store/watch.sqlite, ../../docs/watch)`;

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
const outDir = resolve(flags.get("out") ?? join(HERE, "..", "..", "docs", "watch"));
const when = (epoch: number | null): string => (epoch ? new Date(epoch * 1000).toISOString().replace(/\.\d+Z$/, "Z") : "-");

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
  if (command === "verify") return verify(manifest, join(HERE, "fixtures", "verified.json"), flags.has("write"));

  const store = new Store(storePath);
  try {
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
