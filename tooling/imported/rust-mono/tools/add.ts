#!/usr/bin/env bun
// Adds upstream repositories to a manifest as [[fork]] entries, so the other tools (fork.ts, submodules.ts, forks.ts, sync.ts)
// take them from there. The name is the upstream name; when that name is taken by another fork of the manifest or by a repository
// of the fork organization with another parent, it becomes <owner>-<name>. The tracked branch is the upstream default branch.
//   bun tools/add.ts --group wasm --role wasm [--manifest forks.toml] [--dry-run] owner/repo ...
import { appendFileSync } from "node:fs";
import { resolve } from "node:path";
import { loadManifest } from "./lib/manifest.ts";

const gh = (...a: string[]) => Bun.spawnSync(["gh", ...a], { stdout: "pipe", stderr: "pipe" });
const argv = Bun.argv.slice(2);
const flag = (name: string) => argv.find((a, i) => argv[i - 1] === name);
const repos = argv.filter((a, i) => /^[\w.-]+\/[\w.-]+$/.test(a) && !["--group", "--role", "--manifest"].includes(argv[i - 1] ?? ""));
const manifestPath = resolve(flag("--manifest") ?? "forks.toml");
const group = flag("--group");
const role = flag("--role") ?? group;
if (!group || !role || repos.length === 0) {
  console.error("usage: bun tools/add.ts --group <group> [--role <role>] [--manifest forks.toml] [--dry-run] owner/repo ...");
  process.exit(2);
}
const m = await loadManifest(manifestPath);
if (!(group in m.groups)) throw new Error(`group "${group}" is not declared under [groups]`);

const listed = gh("repo", "list", m.org, "--limit", "5000", "--json", "name");
if (listed.exitCode !== 0) throw new Error(`cannot list ${m.org}: ${listed.stderr.toString().trim()}`);
const orgRepos = new Set((JSON.parse(listed.stdout.toString()) as { name: string }[]).map((r) => r.name.toLowerCase()));
const parentOf = (name: string) => gh("api", `repos/${m.org}/${name}`, "--jq", ".parent.full_name // empty").stdout.toString().trim().toLowerCase();
const taken = new Map(m.forks.map((f) => [f.name.toLowerCase(), f.upstream.toLowerCase()]));
const upstreams = new Set(m.forks.map((f) => f.upstream.toLowerCase()));
const entries: string[] = [];
for (const upstream of repos) {
  if (upstreams.has(upstream.toLowerCase())) {
    console.log(`skip ${upstream}: already in the manifest`);
    continue;
  }
  const info = JSON.parse(gh("api", `repos/${upstream}`).stdout.toString()) as { name: string; default_branch: string; archived: boolean; owner: { login: string } };
  if (info.archived) {
    console.log(`skip ${upstream}: archived`);
    continue;
  }
  const clash = (name: string) => {
    const key = name.toLowerCase();
    const owner = taken.get(key);
    if (owner !== undefined) return owner !== upstream.toLowerCase();
    return orgRepos.has(key) && parentOf(name) !== upstream.toLowerCase();
  };
  let name = info.name;
  if (clash(name)) name = `${info.owner.login}-${info.name}`;
  if (clash(name)) name = `${name}-fork`;
  taken.set(name.toLowerCase(), upstream.toLowerCase());
  entries.push(`\n[[fork]]\npath = "${group}/${name}"\nname = "${name}"\nupstream = "${upstream}"\nbranch = "${info.default_branch}"\nrole = "${role}"\n`);
  console.log(`add ${upstream} -> ${group}/${name} (${info.default_branch})`);
}
if (!argv.includes("--dry-run") && entries.length) appendFileSync(manifestPath, entries.join(""));
