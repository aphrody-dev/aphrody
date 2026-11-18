#!/usr/bin/env bun
// third_party/<name>/README.chromium for every imported upstream, generated from what is recorded elsewhere: the pin in
// tools/upstreams.lock.json, the URL in tools/upstreams.toml, the licences and descriptions in the crates' own manifests.
// Chromium's convention (//docs/adding_to_third_party.md): one metadata file per third-party directory, read by tooling.
//   bun tools/third_party.ts [repo]       (idempotent; run after every import)
import { join } from "node:path";

const repo = Bun.argv[2] ?? ".";
const at = (path: string) => join(repo, path);
const layout = Bun.TOML.parse(await Bun.file(at("tools/layout.toml")).text()) as { third_party: { dir: string } };
const spec = Bun.TOML.parse(await Bun.file(at("tools/upstreams.toml")).text()) as {
  upstream: {
    name: string;
    url: string;
    branch: string;
    mode?: string;
    drop?: string[];
    local?: string[];
    shipped?: boolean;
  }[];
};
const lock = (await Bun.file(at("tools/upstreams.lock.json")).json()) as Record<
  string,
  { sha: string; branch: string; url: string; crates: Record<string, string> }
>;
const root = Bun.TOML.parse(await Bun.file(at("Cargo.toml")).text()) as { workspace: { members: string[] } };
const members = new Set(root.workspace.members);

/** Most permissive first: Chromium's rule is one licence per option ("choose the most permissive one, do not list all options"). */
const permissive = [
  "Unlicense",
  "0BSD",
  "CC0-1.0",
  "MIT",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "ISC",
  "Zlib",
  "Apache-2.0",
  "MPL-2.0",
];
const rank = (l: string) => (permissive.includes(l) ? permissive.indexOf(l) : permissive.length);
const android = new Set([
  "Unlicense",
  "0BSD",
  "CC0-1.0",
  "MIT",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "ISC",
  "Zlib",
  "Apache-2.0",
]);

/** An SPDX expression as Chromium writes it: an OR becomes the most permissive of its options, an AND keeps every part. */
export function chooseLicenses(expression: string): string[] {
  return expression
    .replaceAll("/", " OR ")
    .replace(/[()]/g, "")
    .split(/\s+AND\s+/)
    .map(
      (part) =>
        part
          .split(/\s+OR\s+/)
          .map((l) => l.trim())
          .filter(Boolean)
          .sort((a, b) => rank(a) - rank(b))[0],
    )
    .filter(Boolean);
}

const fixupsText = await Bun.file(at("tools/fixups.sh")).text();
const dir = layout.third_party.dir;
let written = 0;
for (const u of spec.upstream) {
  const entry = lock[u.name];
  const base = `${dir}/${u.name}`;
  if (
    !entry ||
    !(await Bun.file(at(base))
      .stat()
      .then(
        (s) => s.isDirectory(),
        () => false,
      ))
  )
    continue;

  const crates = await Promise.all(
    Object.values(entry.crates).map(async (path) => {
      const file = Bun.file(at(`${path}/Cargo.toml`));
      if (!(await file.exists())) return null;
      const pkg = (Bun.TOML.parse(await file.text()) as any).package ?? {};
      return {
        path,
        name: pkg.name as string,
        version: pkg.version as string | undefined,
        license: pkg.license as string | undefined,
        description: pkg.description as string | undefined,
        shipped: members.has(path),
      };
    }),
  );
  const found = crates.filter((c): c is NonNullable<typeof c> => c !== null);
  const licenses = [...new Set(found.flatMap((c) => (c.license ? chooseLicenses(c.license) : [])))].sort(
    (a, b) => rank(a) - rank(b) || a.localeCompare(b),
  );
  const versions = found.flatMap((c) => (c.version ? [c.version] : []));
  const mostCommon = (xs: string[]) => [...Map.groupBy(xs, (x) => x)].sort((a, b) => b[1].length - a[1].length)[0]?.[0];
  // The licence files that exist, relative to this directory: its own, then those of its crates (one level down).
  const names = "{LICEN[CS]E*,COPYING*,UNLICENSE*}";
  const scanned = await Promise.all(
    [names, `*/${names}`].map((pattern) => Array.fromAsync(new Bun.Glob(pattern).scan({ cwd: at(base), dot: false }))),
  );
  const files = scanned
    .flat()
    .map((p) => p.replaceAll("\\", "/"))
    .sort((x, y) => x.split("/").length - y.split("/").length || x.localeCompare(y))
    .slice(0, 8);
  const known = new Set([u.name, ...found.map((c) => c.name)]);
  const patchDir = at("tools/patches");
  const patches = (await Array.fromAsync(new Bun.Glob("*/*").scan({ cwd: patchDir })))
    .map((p) => p.replaceAll("\\", "/"))
    .filter((p) => known.has(p.split("/")[0]) || known.has(p.split("/")[0].replaceAll("_", "-")));
  const fixes = [...known].some((name) => fixupsText.includes(name));
  const shipped = (u.shipped ?? true) && found.some((c) => c.shipped);
  const lines = [
    `Name: ${u.name}`,
    `Short Name: ${u.name}`,
    `URL: ${entry.url}`,
    `Version: ${mostCommon(versions) ?? "N/A"}`,
    `Revision: ${entry.sha}`,
    `Update Mechanism: Manual`,
    `License: ${licenses.join(", ") || "Unknown"}`,
    `License File: ${files.length ? files.join(", ") : "N/A"}`,
    `License Android Compatible: ${licenses.length && licenses.every((l) => android.has(l)) ? "yes" : "no"}`,
    `Security Critical: ${shipped ? "yes" : "no"}`,
    `Shipped: ${shipped ? "yes" : "no"}`,
    `CPEPrefix: unknown`,
    ``,
    `Description:`,
    found.find((c) => c.description)?.description ?? `Upstream ${u.name}.`,
    ``,
    `Local Modifications:`,
    `Updated with tools/import.py ${u.name} (history replayed linearly); how far behind it is: \`bun testing/acceptance.ts track\`.`,
    `The crates (${found.length}) live in ${base}/<crate>, one directory per package; their manifests are rewritten by tools/flatten.py`,
    `(path dependencies, inlined workspace inheritance, one version per crate). Lockfiles of the upstream are not kept.`,
    `Patches: ${patches.length ? patches.join(", ") : "none"}. Integration fixes: ${fixes ? "yes, in tools/fixups.sh" : "none"}.`,
    `Everything else of the upstream is in ${base}/ unchanged.${u.drop?.length ? ` Not imported: ${u.drop.join(", ")}.` : ""}`,
    ``,
    ...(u.local ?? []).map((line) => `- ${line}`),
    `Crates:`,
    ...found.map((c) => `  ${c.name} ${c.version ?? ""} ${c.path}${c.shipped ? "" : " (not in the workspace)"}`),
    ``,
  ];
  await Bun.write(at(`${base}/README.chromium`), lines.join("\n"));
  written++;
}
console.log(`README.chromium: ${written} upstreams`);
