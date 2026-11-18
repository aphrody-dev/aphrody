// Inventory of the Aphrody monorepo (aphrody-labs/aphrody): third party (src.lock.json), first party (Cargo workspace
// members, Bun workspaces, apps), documentation (every tracked .md/.mdx with its title). Read-only.
// Output: docs/ideas/inventory/aphrody.{json,md}.
// The checkout to read is given explicitly; text quoted from it (descriptions) goes through the private-data
// redaction of scripts/check/private-terms.ts, so this machine's folders and hosts never reach the output.
// bun scripts/inventory/aphrody.ts --src <checkout of aphrody-labs/aphrody> [--fleet <fleet file>]
import { existsSync } from "node:fs";
import { privateMatchers } from "../check/private-terms.ts";
import { dirname, join, resolve } from "node:path";

const argv = Bun.argv.slice(2);
const at = argv.indexOf("--src");
const srcArg = at >= 0 ? argv[at + 1] : Bun.env.SRC_ROOT;
if (!srcArg) throw new Error("give the checkout to inventory: --src <path> (or SRC_ROOT)");
const src = resolve(srcArg);
const fleetAt = argv.indexOf("--fleet");
const { redact } = privateMatchers(join(import.meta.dir, "../.."), fleetAt >= 0 ? argv[fleetAt + 1] : undefined);
// paths inside the checkout are shown relative to it
// an absolute path in quoted text (a drive-rooted one in a title) is reduced to its last segment
const portable = (s: string) => s.replace(/(?<![A-Za-z0-9_])[A-Za-z]:[\\/][^\s)\]"']*/g, (p) => p.split(/[\\/]/).filter(Boolean).pop()!);
const quote = (s: string | null) => (s == null ? null : redact(portable(s.replaceAll(src, "").replaceAll(src.replaceAll("\\", "/"), ""))));
if (!existsSync(join(src, "src.lock.json"))) throw new Error(`${src}: not an aphrody src checkout (no src.lock.json)`);
const open = join(import.meta.dir, "../..");
const tracked = Bun.spawnSync(["git", "-C", src, "-c", "core.quotepath=false", "ls-files"]).stdout.toString().split("\n").filter(Boolean);
const trackedSet = new Set(tracked);
// the committed state of the checkout (HEAD), not files another process is editing
const read = async (p: string) => { const r = Bun.spawnSync(["git", "-C", src, "show", `HEAD:${p}`], { stdout: "pipe", stderr: "ignore" }); return r.exitCode === 0 ? r.stdout.toString() : ""; };

// third party
const lock = JSON.parse((await read("src.lock.json")) || "{}");
const thirdParty = Object.entries<any>(lock.repositories ?? {}).map(([path, r]) => ({
  path, url: r.url, upstream: r.upstream_url ?? null, commit: r.commit ?? null, branch: r.branch ?? null, pillar: r.pillar ?? null,
})).sort((a, b) => a.path.localeCompare(b.path));

// first party: Cargo workspace members
const cargo = Bun.TOML.parse(await read("Cargo.toml")) as any;
const memberDirs = new Set<string>();
for (const m of cargo.workspace?.members ?? []) {
  if (!m.includes("*")) { memberDirs.add(m); continue; }
  for (const f of new Bun.Glob(`${m}/Cargo.toml`).scanSync({ cwd: src })) memberDirs.add(dirname(f).replaceAll("\\", "/"));
}
const crates = [];
for (const d of [...memberDirs].sort()) {
  const t = await read(`${d}/Cargo.toml`);
  if (!t) continue;
  const p = (Bun.TOML.parse(t) as any).package ?? {};
  crates.push({ path: d, name: p.name ?? d.split("/").pop(), description: quote(p.description ?? null) });
}

// first party: Bun workspaces and apps
const rootPkg = JSON.parse((await read("package.json")) || "{}");
const wsGlobs: string[] = Array.isArray(rootPkg.workspaces) ? rootPkg.workspaces : (rootPkg.workspaces?.packages ?? []);
const packages = [];
for (const f of tracked.filter((f) => f.endsWith("package.json") && !f.includes("node_modules/") && !f.startsWith("third_party/"))) {
  const dir = dirname(f);
  if (dir === ".") continue;
  const inWs = wsGlobs.some((g) => new Bun.Glob(g).match(dir));
  if (!inWs && !dir.startsWith("apps/")) continue;
  const p = JSON.parse((await read(f)) || "{}");
  packages.push({ path: dir, name: p.name ?? null, description: quote(p.description ?? null), workspace: inWs });
}
const apps = [...new Set(tracked.filter((f) => f.startsWith("apps/")).map((f) => f.split("/")[1]!))].sort();

// documentation
const docs = [];
for (const f of tracked.filter((f) => /\.(md|mdx)$/i.test(f) && !f.startsWith("third_party/") && !f.includes("node_modules/"))) {
  const t = await read(f);
  const title = t.match(/^#\s+(.+)$/m)?.[1]?.trim() ?? null;
  docs.push({ path: f, title: quote(title), bytes: t.length });
}
const docsByArea: Record<string, number> = {};
for (const d of docs) {
  const area = d.path.split("/").slice(0, d.path.startsWith("docs/") ? 2 : 1).join("/");
  docsByArea[area] = (docsByArea[area] ?? 0) + 1;
}

const head = Bun.spawnSync(["git", "-C", src, "rev-parse", "--short", "HEAD"]).stdout.toString().trim();
const data = { source: `aphrody-labs/aphrody @ ${head}`, counts: { thirdParty: thirdParty.length, crates: crates.length, packages: packages.length, apps: apps.length, docs: docs.length, trackedFiles: trackedSet.size }, thirdParty, crates, packages, apps, docsByArea, docs };
const outDir = join(open, "docs/ideas/inventory");
await Bun.write(join(outDir, "aphrody.json"), `${JSON.stringify(data, null, 2)}\n`);

const cell = (s: string | null) => (s ?? "").replace(/\|/g, "\\|").replace(/\s+/g, " ").slice(0, 110);
const md = [
  "# Aphrody monorepo inventory", "",
  `Generated by \`bun scripts/inventory/aphrody.ts\` from ${data.source}. Full data: [aphrody.json](aphrody.json).`, "",
  `| Third party | Rust crates | Bun packages | Apps | Docs | Tracked files |`, "|---|---|---|---|---|---|",
  `| ${thirdParty.length} | ${crates.length} | ${packages.length} | ${apps.length} | ${docs.length} | ${trackedSet.size} |`, "",
  "## Third party (src.lock.json)", "", "| Path | Repository | Upstream | Commit |", "|---|---|---|---|",
  ...thirdParty.map((t) => `| ${t.path} | ${t.url ?? "-"} | ${t.upstream ?? "-"} | ${t.commit?.slice(0, 9) ?? "-"} |`), "",
  "## First party: Rust crates", "", "| Path | Name | Description |", "|---|---|---|",
  ...crates.map((c) => `| ${c.path} | ${c.name} | ${cell(c.description)} |`), "",
  "## First party: Bun packages and apps", "", "| Path | Name | Description |", "|---|---|---|",
  ...packages.map((p) => `| ${p.path} | ${p.name ?? "-"} | ${cell(p.description)} |`), "",
  `Apps: ${apps.join(", ")}`, "",
  "## Documentation by area", "", "| Area | Files |", "|---|---|",
  ...Object.entries(docsByArea).sort((a, b) => b[1] - a[1]).map(([a, n]) => `| ${a} | ${n} |`), "",
];
await Bun.write(join(outDir, "aphrody.md"), md.join("\n"));
console.log(JSON.stringify(data.counts));
