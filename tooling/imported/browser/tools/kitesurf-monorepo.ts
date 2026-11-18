#!/usr/bin/env bun
// Turns the kitesurf checkout into one Cargo workspace that owns the engine crates it used to patch at build time: every crate that is patched
// or depends (transitively) on a patched one is copied, patched, into crates/<name>, renamed with a prefix (the published name; the library
// name stays, so no source changes), wired to its siblings by path + version (publish-ready) and registered in [workspace.dependencies].
// The `[patch.crates-io]` table and the build-time materialiser are no longer needed. The reverse-dependency closure is computed from
// `cargo metadata`, never listed by hand: a crate left on crates.io that depends on a renamed one would pull a second copy of it.
//   bun tools/kitesurf-monorepo.ts <kitesurf checkout> [--prefix browser-] [--dry-run]
//   bun tools/kitesurf-monorepo.ts matrix <kitesurf checkout>     write crates/FEATURES.md, the feature matrix of the in-tree crates
// Run it on a build host after `bun scripts/get-patched-vendor-crates.mjs` (the patched trees) and a `cargo fetch` (the registry sources).
import { homedir } from "node:os";
import { join, resolve } from "node:path";

export type Meta = {
  packages: { id: string; name: string; version: string; source: string | null }[];
  resolve: { nodes: { id: string; deps: { pkg: string }[] }[] };
  workspace_members: string[];
};
export type Member = { name: string; version: string; lib?: string };

/** The seeds and every package that depends on one of them, transitively, outside the workspace's own members. */
export function closureOf(meta: Meta, seeds: string[]): Member[] {
  const reverse = new Map<string, Set<string>>();
  for (const node of meta.resolve.nodes) {
    for (const dep of node.deps) {
      let parents = reverse.get(dep.pkg);
      if (!parents) {
        parents = new Set();
        reverse.set(dep.pkg, parents);
      }
      parents.add(node.id);
    }
  }
  const members = new Set(meta.workspace_members);
  const wanted = new Set(seeds);
  const queue = meta.packages.filter((pkg) => wanted.has(pkg.name) && !members.has(pkg.id)).map((pkg) => pkg.id);
  const seen = new Set(queue);
  while (queue.length) {
    for (const parent of reverse.get(queue.pop()!) ?? []) {
      if (!seen.has(parent) && !members.has(parent)) {
        seen.add(parent);
        queue.push(parent);
      }
    }
  }
  return meta.packages
    .filter((pkg) => seen.has(pkg.id))
    .map(({ name, version }) => ({ name, version }))
    .sort((a, b) => a.name.localeCompare(b.name));
}

const lib = (name: string) => name.replace(/-/g, "_");

/** The manifest of one crate in the monorepo: prefixed package name, the library name kept, sibling dependencies by path, no stamped [workspace]. */
export function prefixManifest(text: string, names: string[], prefix: string): string {
  const siblings = new Set(names);
  const lines = text.split("\n");
  const out: string[] = [];
  let section = "";
  const devOnly = new Set<string>();
  let sawLib = false;
  let packageName = "";
  for (let index = 0; index < lines.length; index++) {
    const line = lines[index]!;
    const header = line.match(/^\[(.+)\]\s*$/);
    if (header) {
      section = header[1]!;
      // Tests, benches and examples are not in the published sources: their targets and dev-dependencies would only pull a second dependency tree.
      const dev = section.match(/^(?:target\..+\.)?dev-dependencies\.([A-Za-z0-9_-]+)$/);
      if (dev) devOnly.add(dev[1]!);
      if (
        /^(?:target\..+\.)?dev-dependencies(?:\.|$)/.test(section) ||
        /^\[(?:bench|test|example)\]$/.test(line.replace(/^\[\[/, "[").replace(/\]\]$/, "]"))
      ) {
        while (index + 1 < lines.length && !/^\[/.test(lines[index + 1]!)) index++;
        continue;
      }
      if (section === "workspace") {
        while (index + 1 < lines.length && !/^\[/.test(lines[index + 1]!)) index++;
        continue;
      }
      if (section === "lib") sawLib = true;
      out.push(line);
      const dependency = section.match(/^(?:target\..+\.)?(?:dev-|build-)?dependencies\.([A-Za-z0-9_-]+)$/);
      if (dependency) {
        let end = index + 1;
        while (end < lines.length && !/^\[/.test(lines[end]!)) end++;
        const renamed = lines.slice(index + 1, end).findIndex((next) => /^package\s*=/.test(next));
        const target = renamed >= 0 ? lines[index + 1 + renamed]!.match(/"([^"]+)"/)![1]! : dependency[1]!;
        if (siblings.has(target)) {
          out.push(`package = "${prefix}${target}"`, `path = "../${target}"`);
          if (renamed >= 0) lines.splice(index + 1 + renamed, 1);
        }
      }
      continue;
    }
    if (section === "package" && !packageName) {
      const named = line.match(/^name\s*=\s*"([^"]+)"/);
      if (named) {
        packageName = named[1]!;
        out.push(`name = "${prefix}${packageName}"`);
        continue;
      }
    }
    if (section === "lib" && /^name\s*=/.test(line)) sawLib = true;
    out.push(line);
  }
  let result = out.join("\n").replace(/\n+$/, "\n");
  // A feature that forwards to a dropped dev-dependency (`peniko/libm`) has nothing to forward to once it is gone.
  for (const name of devOnly) {
    if (!new RegExp(`^\\[(?:target\\..+\\.)?dependencies\\.${name}\\]`, "m").test(result))
      result = result.replace(new RegExp(`"${name}\\??/[^"]*"\\s*,?[ \\t]*`, "g"), "");
  }
  if (!packageName) throw new Error("the manifest has no [package] name");
  if (!sawLib) result += `\n[lib]\nname = "${lib(packageName)}"\n`;
  else if (!/^\[lib\][^[]*^name\s*=/m.test(result))
    result = result.replace(/^\[lib\]\s*$/m, `[lib]\nname = "${lib(packageName)}"`);
  return result;
}

/** A dependency line of a hand-written manifest on a renamed crate (`name = { version = "…", … }` or `name = "…"`), rewired to its in-tree copy.
 * The dependency key is the extern crate name once the package is renamed, so a crate whose library name differs from its package name
 * (stylo: `style`) gets its library name as the key. */
export function rewireLine(line: string, members: Member[], prefix: string, relative: string): string {
  const match = line.match(/^([A-Za-z0-9_-]+)(\s*=\s*)(?:\{\s*version\s*=\s*("[^"]*")|("[^"]*")\s*$)/);
  const member = match && members.find((entry) => entry.name === match[1]);
  if (!match || !member) return line;
  const key = member.lib && member.lib !== lib(member.name) ? member.lib : member.name;
  const version = match[3] ?? match[4]!;
  const head = `${key}${match[2]}{ package = "${prefix}${member.name}", path = "${relative}/${member.name}", version = ${version}`;
  return match[4] ? `${head} }` : line.replace(/^[^{]*\{\s*version\s*=\s*"[^"]*"/, head);
}

/** Feature entries of a manifest that name a dependency by its old key (`stylo/servo`, `dep:stylo`) follow the key's rename. */
export function renameFeatureRefs(text: string, members: Member[]): string {
  let result = text;
  for (const member of members) {
    if (!member.lib || member.lib === lib(member.name)) continue;
    result = result.replace(
      new RegExp(`"(dep:)?${member.name}(\\??/[^"]*)?"`, "g"),
      (_all, dep: string | undefined, rest: string | undefined) => `"${dep ?? ""}${member.lib}${rest ?? ""}"`,
    );
  }
  return result;
}

/** The root manifest without its [patch.crates-io] table, the workspace members extended, the renamed crates' dependency lines rewired. */
export function rewriteRoot(text: string, members: Member[], prefix: string): string {
  const lines = text.split("\n");
  const out: string[] = [];
  let skipping = false;
  for (const line of lines) {
    if (/^\[/.test(line)) skipping = line.trim() === "[patch.crates-io]";
    if (skipping) continue;
    out.push(rewireLine(line, members, prefix, "crates"));
  }
  return out
    .join("\n")
    .replace(/members = \[([^\]]*)\]/, (_all, inner: string) =>
      inner.includes("crates/*") ? _all : `members = [${inner.trim().replace(/,$/, "")}, "crates/*"]`,
    )
    .replace(/\n{3,}/g, "\n\n");
}

export type MatrixMeta = {
  packages: {
    id: string;
    name: string;
    version: string;
    repository: string | null;
    features: Record<string, string[]>;
    manifest_path: string;
  }[];
  resolve: { nodes: { id: string; features: string[] }[] };
};

/** The feature matrix of the in-tree crates: every feature of each crate, and `●` where the workspace's own build turns it on. */
export function matrixOf(meta: MatrixMeta, prefix: string, patches: Record<string, number> = {}): string {
  const active = new Map(meta.resolve.nodes.map((node) => [node.id, new Set(node.features)]));
  const crates = meta.packages
    .filter((pkg) => pkg.name.startsWith(prefix) && /[\\/]crates[\\/]/.test(pkg.manifest_path))
    .sort((a, b) => a.name.localeCompare(b.name));
  const lines = [
    "# Crates and their feature matrix",
    "",
    "Generated by `tools/kitesurf-monorepo.ts matrix`. `●` = enabled by the workspace's own build, `○` = available.",
    "",
  ];
  for (const pkg of crates) {
    const features = Object.keys(pkg.features)
      .filter((name) => name !== "default")
      .sort();
    const on = active.get(pkg.id) ?? new Set<string>();
    const original = pkg.name.slice(prefix.length);
    lines.push(
      `## ${pkg.name} ${pkg.version}`,
      "",
      `Upstream: ${pkg.repository ?? "n/a"} · patches carried: ${patches[original] ?? 0} · default: ${(pkg.features.default ?? []).join(", ") || "none"}`,
      "",
    );
    if (features.length)
      lines.push(
        "| feature | state | enables |",
        "|---|---|---|",
        ...features.map(
          (name) =>
            `| \`${name}\` | ${on.has(name) ? "●" : "○"} | ${pkg.features[name]!.map((entry) => `\`${entry}\``).join(" ") || "-"} |`,
        ),
        "",
      );
    else lines.push("No features.", "");
  }
  return lines.join("\n");
}

const SKIP = new Set([
  ".git",
  ".patched",
  ".patch-sha8",
  ".cargo-ok",
  "Cargo.toml.orig",
  ".cargo_vcs_info.json",
  "Cargo.lock",
  "target",
]);
const dirExists = async (path: string) =>
  Bun.file(path)
    .stat()
    .then(
      (s) => s.isDirectory(),
      () => false,
    );

async function sourceOf(checkout: string, member: Member): Promise<string> {
  const patched = join(checkout, "packages", "core", "target", "vendor-patched", `${member.name}-${member.version}`);
  if (await dirExists(patched)) return patched;
  const registry = join(homedir(), ".cargo", "registry", "src");
  for await (const index of new Bun.Glob("*").scan({ cwd: registry, onlyFiles: false })) {
    const candidate = join(registry, index, `${member.name}-${member.version}`);
    if (await dirExists(candidate)) return candidate;
  }
  throw new Error(`no source for ${member.name}-${member.version}: run the materialiser and cargo fetch first`);
}

async function copyFiltered(source: string, target: string): Promise<void> {
  await Bun.$`rm -rf ${target} && mkdir -p ${target}`.quiet();
  for await (const rel of new Bun.Glob("**/*").scan({ cwd: source, dot: true })) {
    if (rel.split(/[\\/]/).some((part) => SKIP.has(part))) continue;
    await Bun.write(join(target, rel), Bun.file(join(source, rel)));
  }
}

if (import.meta.main) {
  const argv = Bun.argv.slice(2);
  const checkout = resolve(
    argv.find((arg) => !arg.startsWith("--") && arg !== "matrix" && argv[argv.indexOf(arg) - 1] !== "--prefix") ?? ".",
  );
  const prefix = argv.includes("--prefix") ? argv[argv.indexOf("--prefix") + 1]! : "browser-";
  const dry = argv.includes("--dry-run");
  if (argv.includes("matrix")) {
    const meta = JSON.parse(
      Bun.spawnSync(["cargo", "metadata", "--format-version", "1"], {
        cwd: checkout,
        stdout: "pipe",
        stderr: "pipe",
        windowsHide: true,
      }).stdout.toString(),
    ) as MatrixMeta;
    const patches: Record<string, number> = {};
    const patchRoot = join(checkout, "scripts", "patches");
    if (await dirExists(patchRoot)) {
      for await (const dir of new Bun.Glob("*").scan({ cwd: patchRoot, onlyFiles: false })) {
        if (!/-\d/.test(dir)) continue;
        const count = (await Array.fromAsync(new Bun.Glob("*.patch").scan({ cwd: join(patchRoot, dir) }))).length;
        patches[dir.replace(/-\d[^-]*(?:-[a-z0-9.]+)?$/, "")] = count;
      }
    }
    await Bun.write(join(checkout, "crates", "FEATURES.md"), matrixOf(meta, prefix, patches));
    console.log("wrote crates/FEATURES.md");
    process.exit(0);
  }
  const crates = await Bun.file(join(checkout, "scripts", "get-patched-vendor-crates.mjs")).text();
  const seeds = [...crates.matchAll(/\{ name: "([^"]+)", version: "[^"]+" \}/g)].map((match) => match[1]!);
  const run = Bun.spawnSync(["cargo", "metadata", "--format-version", "1", "--locked"], {
    cwd: checkout,
    stdout: "pipe",
    stderr: "pipe",
    stdio: ["ignore", "pipe", "pipe"],
    windowsHide: true,
  });
  if (run.exitCode !== 0) throw new Error(run.stderr.toString().slice(-400));
  const members = closureOf(JSON.parse(run.stdout.toString()) as Meta, seeds);
  console.log(`${members.length} crates: ${members.map((member) => member.name).join(" ")}`);
  if (dry) process.exit(0);
  const names = members.map((member) => member.name);
  for (const member of members) {
    const target = join(checkout, "crates", member.name);
    await copyFiltered(await sourceOf(checkout, member), target);
    const manifest = join(target, "Cargo.toml");
    await Bun.write(manifest, prefixManifest(await Bun.file(manifest).text(), names, prefix));
  }
  for (const member of members) {
    const parsed = Bun.TOML.parse(await Bun.file(join(checkout, "crates", member.name, "Cargo.toml")).text()) as {
      lib?: { name?: string };
    };
    member.lib = parsed.lib?.name ?? lib(member.name);
  }
  const root = join(checkout, "Cargo.toml");
  await Bun.write(root, rewriteRoot(await Bun.file(root).text(), members, prefix));
  for await (const dir of new Bun.Glob("*").scan({ cwd: join(checkout, "packages"), onlyFiles: false })) {
    const manifest = join(checkout, "packages", dir, "Cargo.toml");
    if (!(await Bun.file(manifest).exists())) continue;
    const before = await Bun.file(manifest).text();
    const after = renameFeatureRefs(
      before
        .split("\n")
        .map((line) => rewireLine(line, members, prefix, "../../crates"))
        .join("\n"),
      members,
    );
    if (after !== before) await Bun.write(manifest, after);
  }
  console.log(`wrote crates/ (${members.length}), rewired the workspace root and the packages' manifests`);
}
