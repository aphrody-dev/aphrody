// Fetches OpenTUI's native library for this platform into target/opentui-native (OPENTUI_LIB_DIR of .cargo/config.toml).
// The version is the one of the pinned opentui revision (packages/rust/Cargo.toml at that revision, read live), and the
// library comes from the npm platform package of that version: libopentui.so (or .dylib, opentui.dll) and opentui.h.
//
//   bun scripts/opentui-native.ts [--force]
import { existsSync, mkdirSync, readdirSync, renameSync, rmSync } from "node:fs";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const out = join(root, "target", "opentui-native");
const manifest = await Bun.file(join(root, "Cargo.toml")).text();
const rev = /opentui = \{ git = "[^"]+", rev = "([0-9a-f]{40})"/.exec(manifest)?.[1];
if (!rev) throw new Error("no pinned opentui revision in Cargo.toml");

const rust = await (await fetch(`https://raw.githubusercontent.com/anomalyco/opentui/${rev}/packages/rust/Cargo.toml`)).text();
const version = /^version = "([^"]+)"/m.exec(rust)?.[1];
if (!version) throw new Error(`no version in packages/rust/Cargo.toml at ${rev}`);

const platforms: Record<string, string> = {
  "linux-x64": "linux-x64",
  "linux-arm64": "linux-arm64",
  "darwin-x64": "darwin-x64",
  "darwin-arm64": "darwin-arm64",
  "win32-x64": "win32-x64",
  "win32-arm64": "win32-arm64",
};
const key = `${process.platform}-${process.arch}`;
const platform = platforms[key];
if (!platform) throw new Error(`no OpenTUI native package for ${key}`);

const stamp = join(out, ".version");
if (!Bun.argv.includes("--force") && existsSync(stamp) && (await Bun.file(stamp).text()).trim() === `${version}@${key}`) {
  console.log(`opentui-native ${version} already in ${out}`);
  process.exit(0);
}

const url = `https://registry.npmjs.org/@opentui/core-${platform}/-/core-${platform}-${version}.tgz`;
const response = await fetch(url);
if (!response.ok) throw new Error(`${url}: HTTP ${response.status}`);
rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });
const archive = join(out, "package.tgz");
await Bun.write(archive, await response.arrayBuffer());
const tar = Bun.spawnSync(["tar", "xzf", archive, "-C", out]);
if (tar.exitCode !== 0) throw new Error(`tar failed: ${tar.stderr.toString()}`);
rmSync(archive);
// The archive unpacks into package/: lift its files so OPENTUI_LIB_DIR holds them directly.
const inner = join(out, "package");
for (const name of readdirSync(inner)) renameSync(join(inner, name), join(out, name));
rmSync(inner, { recursive: true });
await Bun.write(stamp, `${version}@${key}\n`);
console.log(`opentui-native ${version} (${key}) from ${rev.slice(0, 10)} -> ${out}`);
