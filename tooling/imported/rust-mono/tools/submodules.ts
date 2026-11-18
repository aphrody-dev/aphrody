#!/usr/bin/env bun
// Materialises forks.toml in the superproject: writes .gitmodules (relative URLs) and registers a gitlink for every fork that has
// none, at the head of its tracked branch. No submodule is cloned or checked out: gitlinks go straight into the index.
import { dirname, resolve } from "node:path";
import { branchHead, ghCli, type Gh } from "./lib/gh.ts";
import { gitlinks, setGitlink } from "./lib/git.ts";
import { loadManifest, renderGitmodules } from "./lib/manifest.ts";

export async function run(argv: string[], gh: Gh = ghCli()): Promise<number> {
  const manifestPath = resolve(argv.find((a, i) => argv[i - 1] === "--manifest") ?? "forks.toml");
  const root = dirname(manifestPath);
  const m = await loadManifest(manifestPath);
  const write = argv.includes("--write");
  const want = renderGitmodules(m);
  const have = await Bun.file(resolve(root, ".gitmodules")).text().catch(() => "");
  const pins = await gitlinks(root);
  const missing = m.forks.filter((f) => !pins.has(f.path));
  console.log(`.gitmodules ${want === have ? "up to date" : "differs"}; ${missing.length} gitlink(s) missing`);
  if (!write) return want === have && !missing.length ? 0 : 1;
  if (want !== have) await Bun.write(resolve(root, ".gitmodules"), want);
  for (const f of missing) {
    const sha = await branchHead(gh, `${m.org}/${f.name}`, f.branch);
    if (!sha) throw new Error(`${m.org}/${f.name} has no branch ${f.branch}`);
    await setGitlink(root, f.path, sha);
    console.log(`pinned ${f.path} at ${sha.slice(0, 9)}`);
  }
  return 0;
}

if (import.meta.main) process.exit(await run(Bun.argv.slice(2)).catch((e) => (console.error((e as Error).message), 2)));
