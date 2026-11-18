#!/usr/bin/env bun
// Checks the tree against tools/layout.toml: nothing in the legacy src/ or upstream/, every crate of the lock where the layout puts it,
// every first-party crate marked as such, every submodule where the layout says. Prints the problems, exit 1 if there are any.
//   bun tools/layout.ts [repo]
import { join } from "node:path";

const repo = Bun.argv[2] ?? ".";
const at = (path: string) => join(repo, path);
const layout = Bun.TOML.parse(await Bun.file(at("tools/layout.toml")).text()) as {
  bun: { dir: string };
  kitesurf?: { dir: string; commit?: string };
  browse?: { dir: string; commit?: string };
  mullvad?: { dir: string; commit?: string };
  first_party: { dir: string };
  third_party: { dir: string };
};
const lock = (await Bun.file(at("tools/upstreams.lock.json")).json()) as Record<
  string,
  { crates: Record<string, string> }
>;
const tracked = async (...paths: string[]) =>
  (await Bun.$`git -C ${repo} ls-files -z -- ${paths}`.text()).split("\0").filter(Boolean);
const problems: string[] = [];

for (const legacy of await tracked("src", "upstream")) problems.push(`legacy location: ${legacy}`);

for (const [name, entry] of Object.entries(lock))
  for (const dir of Object.values(entry.crates)) {
    if (!dir.startsWith(`${layout.third_party.dir}/${name}/`))
      problems.push(`${name}: crate at ${dir}, expected under ${layout.third_party.dir}/${name}/`);
    else if (!(await Bun.file(at(`${dir}/Cargo.toml`)).exists())) problems.push(`${name}: no manifest at ${dir}`);
  }

for (const manifest of await tracked(`${layout.first_party.dir}/*/Cargo.toml`)) {
  if (!(await Bun.file(at(manifest)).exists())) continue;
  const meta = (
    Bun.TOML.parse(await Bun.file(at(manifest)).text()) as {
      package?: { metadata?: { browser?: { native?: boolean } } };
    }
  ).package?.metadata?.browser;
  if (meta?.native !== true)
    problems.push(`${manifest}: first-party crates set [package.metadata.browser] native = true`);
}

const modules = (
  await Bun.file(at(".gitmodules"))
    .text()
    .catch(() => "")
).matchAll(/path = (.+)/g);
const submodules = [...modules].map((m) => m[1].trim());
if (!submodules.includes(layout.bun.dir)) problems.push(`${layout.bun.dir} is not a submodule`);
if (layout.kitesurf && !submodules.includes(layout.kitesurf.dir))
  problems.push(`${layout.kitesurf.dir} is not a submodule`);
if (layout.browse && !submodules.includes(layout.browse.dir)) problems.push(`${layout.browse.dir} is not a submodule`);
if (layout.mullvad && !submodules.includes(layout.mullvad.dir))
  problems.push(`${layout.mullvad.dir} is not a submodule`);

console.log(problems.length ? problems.join("\n") : "layout ok");
process.exit(problems.length ? 1 : 0);
