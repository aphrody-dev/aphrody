// The libraries open offers to other projects, with their API, written as a manifest (libraries.json) and a page
// (docs/libraries.md); with --pack, the Bun packages are packed as tarballs whose internal dependencies point at the
// release that carries them, so a consumer needs no registry.
//   bun scripts/libraries.ts [--pack <dir>] [--repo <owner/name>]
// Rust crates are consumed as git dependencies of this repository; Bun packages as tarball URLs of the release
// `libraries-v<version>`; the native libraries (C ABI) are attached to the same release by the libraries workflow.
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync, cpSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";

const ROOT = resolve(import.meta.dir, "..");
const { values: a } = parseArgs({ options: { pack: { type: "string" }, repo: { type: "string" } } });

// the unit lists: the libraries offered for consumption; everything else in the workspace stays internal
const CRATES = ["crates/c-abi", "crates/ai-router", "crates/hub"];
const PACKAGES = ["packages/browser-engines", "packages/crawl", "packages/ai-router"];

const sh = (cmd: string[], cwd = ROOT) => {
  const p = Bun.spawnSync(cmd, { cwd, stdout: "pipe", stderr: "pipe" });
  if (p.exitCode !== 0) throw new Error(`${cmd.join(" ")}: ${p.stderr.toString().trim().slice(0, 300)}`);
  return p.stdout.toString().trim();
};

const version = /\[workspace\.package\][^[]*?\nversion\s*=\s*"([^"]+)"/.exec(readFileSync(join(ROOT, "Cargo.toml"), "utf8"))?.[1];
if (!version) throw new Error("no [workspace.package] version in Cargo.toml");
// the repository: --repo, the CI's GITHUB_REPOSITORY, else the origin remote
const repo = a.repo ?? process.env.GITHUB_REPOSITORY ?? sh(["git", "remote", "get-url", "origin"]).replace(/^.*github\.com[/:]/, "").replace(/\.git$/, "");
const server = process.env.GITHUB_SERVER_URL ?? "https://github.com";
const tag = `libraries-v${version}`;
const release = `${server}/${repo}/releases/download/${tag}`;
const rev = sh(["git", "rev-parse", "HEAD"]);
const tarball = (name: string) => `${name.replace(/^@/, "").replace("/", "-")}-${version}.tgz`;

type Lib = Record<string, unknown>;
const libs: Lib[] = [];

for (const dir of CRATES) {
  const toml = readFileSync(join(ROOT, dir, "Cargo.toml"), "utf8");
  const name = /^\[package\][^[]*?\nname\s*=\s*"([^"]+)"/m.exec(toml)![1]!;
  const description = /\ndescription\s*=\s*"([^"]+)"/.exec(toml)?.[1] ?? "";
  const kinds = /crate-type\s*=\s*\[([^\]]*)\]/.exec(toml)?.[1]?.match(/"(\w+)"/g)?.map((x) => x.replaceAll('"', "")) ?? (existsSync(join(ROOT, dir, "src", "main.rs")) && !existsSync(join(ROOT, dir, "src", "lib.rs")) ? ["bin"] : ["rlib"]);
  // the C ABI: every function a header of the crate declares
  const headers = existsSync(join(ROOT, dir, "include")) ? readdirSync(join(ROOT, dir, "include")).filter((f) => f.endsWith(".h")) : [];
  const symbols = headers.flatMap((h) => [...readFileSync(join(ROOT, dir, "include", h), "utf8").matchAll(/\b(aphrody_\w+)\s*\(/g)].map((m) => m[1]!));
  // the Rust API: public items of the library root
  const lib = join(ROOT, dir, "src", "lib.rs");
  const items = existsSync(lib) ? [...readFileSync(lib, "utf8").matchAll(/^pub (fn|struct|enum|mod|type|const|static|trait|use) ([\w:{}, ]+?)(?=[\s(<:;{]|$)/gm)].map((m) => (m[1] === "use" ? m[2]!.split("::").pop()!.replace(/[{}]/g, "") : m[2]!).trim()).filter(Boolean) : [];
  libs.push({
    kind: "crate", name, version, path: dir, description, crateTypes: kinds, rustApi: [...new Set(items)], cAbi: { headers: headers.map((h) => `${dir}/include/${h}`), functions: [...new Set(symbols)] },
    consume: { cargo: kinds.includes("bin") && kinds.length === 1 ? `cargo install --git ${server}/${repo} --rev ${rev} ${name}` : `${name} = { git = "${server}/${repo}", rev = "${rev}" }` },
  });
}

const transpiler = new Bun.Transpiler({ loader: "ts" });
for (const dir of PACKAGES) {
  const pkg = JSON.parse(readFileSync(join(ROOT, dir, "package.json"), "utf8"));
  const exportsMap: Record<string, string> = typeof pkg.exports === "string" ? { ".": pkg.exports } : (pkg.exports ?? { ".": pkg.main });
  const api: Record<string, string[]> = {};
  for (const [sub, file] of Object.entries(exportsMap)) {
    const f = join(ROOT, dir, String(file));
    if (existsSync(f)) api[sub] = transpiler.scan(readFileSync(f, "utf8")).exports.sort();
  }
  const internal = Object.keys(pkg.dependencies ?? {}).filter((d) => String(pkg.dependencies[d]).startsWith("workspace:"));
  libs.push({ kind: "package", name: pkg.name, version, path: dir, description: pkg.description ?? "", exports: api, internalDependencies: internal, consume: { bun: `bun add ${release}/${tarball(pkg.name)}` } });
}

const manifest = { repository: `${server}/${repo}`, version, rev, release: tag, libraries: libs };
writeFileSync(join(ROOT, "libraries.json"), JSON.stringify(manifest, null, 2) + "\n");

const md = [
  "# Libraries",
  "",
  `Generated by \`bun scripts/libraries.ts\` from the manifests; the same data is in \`libraries.json\`. Version ${version}.`,
  "",
  "Rust crates are git dependencies of this repository at a commit; Bun packages are tarballs of the release `libraries-v<version>`, whose internal dependencies point at the same release; the native libraries of the crates with a C ABI are attached to that release for each operating system.",
  "",
  ...libs.flatMap((l) => [
    `## ${l.name} (${l.kind})`,
    "",
    `\`${l.path}\`${l.description ? `: ${l.description}` : ""}`,
    "",
    "```",
    String((l.consume as Record<string, string>).cargo ?? (l.consume as Record<string, string>).bun).replace(rev, "<commit>"),
    "```",
    "",
    ...(l.kind === "crate"
      ? [`- Rust API: ${(l.rustApi as string[]).map((x) => `\`${x}\``).join(", ") || "binary"}`, ...((l.cAbi as { functions: string[] }).functions.length ? [`- C ABI (${(l.cAbi as { headers: string[] }).headers.join(", ")}): ${(l.cAbi as { functions: string[] }).functions.map((x) => `\`${x}\``).join(", ")}`] : [])]
      : Object.entries(l.exports as Record<string, string[]>).map(([sub, names]) => `- \`${l.name}${sub === "." ? "" : sub.slice(1)}\`: ${names.map((x) => `\`${x}\``).join(", ")}`)),
    "",
  ]),
].join("\n");
writeFileSync(join(ROOT, "docs", "libraries.md"), md);
console.log(`libraries.json and docs/libraries.md: ${libs.length} libraries, version ${version}`);

if (a.pack) {
  const out = resolve(a.pack);
  mkdirSync(out, { recursive: true });
  for (const dir of PACKAGES) {
    // a copy with its version set and its workspace dependencies pointing at the release tarballs
    const tmp = mkdtempSync(join(tmpdir(), "pack-"));
    cpSync(join(ROOT, dir), tmp, { recursive: true, filter: (s) => !/[\\/](node_modules|test)([\\/]|$)/.test(s) });
    const pkg = JSON.parse(readFileSync(join(tmp, "package.json"), "utf8"));
    pkg.version = version;
    for (const d of Object.keys(pkg.dependencies ?? {})) if (String(pkg.dependencies[d]).startsWith("workspace:")) pkg.dependencies[d] = `${release}/${tarball(d)}`;
    delete pkg.devDependencies;
    writeFileSync(join(tmp, "package.json"), JSON.stringify(pkg, null, 2) + "\n");
    sh(["bun", "pm", "pack", "--filename", join(out, tarball(pkg.name))], tmp);
    rmSync(tmp, { recursive: true, force: true });
    console.log(`packed ${tarball(pkg.name)}`);
  }
}
