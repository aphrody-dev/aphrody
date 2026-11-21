#!/usr/bin/env bun
/**
 * Builds aphrody_av (pure-Rust probe/decode/convert) to wasm on the build host (fr1) from origin/main, runs
 * wasm-bindgen --target nodejs, copies the output to target/av-wasm/ and writes target/av-wasm/manifest.json
 * with the commit, sha256 and size of every file (hashes checked against the host's own sha256sum).
 * site/build.ts loads the module from there (or from $AV_WASM) instead of ffprobe.
 *
 * Measures: build seconds and artifact bytes.
 * Run: bun tooling/bun/av-wasm.ts
 */
import { createHash } from "node:crypto";
import { mkdirSync, rmSync } from "node:fs";
import { join, resolve } from "node:path";
import { ciRoot, host, pull, remote, syncRepos, wasmTargetDir } from "./ci-fr1.ts";

const outDir = resolve(import.meta.dir, "../../target/av-wasm");
const remoteOut = `${ciRoot}/av-wasm`;
const started = Date.now();

const build = await remote(`${syncRepos}
cd ${ciRoot}/wasm/aphrody
export CARGO_TARGET_DIR=${wasmTargetDir}
cargo build -p aphrody_av --lib --target wasm32-unknown-unknown --no-default-features --features wasm --release
rm -rf ${remoteOut}
wasm-bindgen --version
wasm-bindgen ${wasmTargetDir}/wasm32-unknown-unknown/release/aphrody_av.wasm --target nodejs --out-dir ${remoteOut}
cd ${remoteOut} && sha256sum * | sed 's/^/SHA256 /'
`);
if (build.code !== 0) throw new Error(`av-wasm build failed on ${host} (exit ${build.code})`);

const lines = build.stdout.split("\n");
const commits = Object.fromEntries(lines.filter((line) => /^(aphrody|fw) [0-9a-f]{10}$/.test(line)).map((line) => line.split(" ")));
const remoteHashes = Object.fromEntries(lines.filter((line) => line.startsWith("SHA256 ")).map((line) => line.split(/\s+/).slice(1).reverse()));

rmSync(outDir, { recursive: true, force: true });
mkdirSync(outDir, { recursive: true });
const files: Record<string, { bytes: number; sha256: string }> = {};
for (const name of Object.keys(remoteHashes).sort()) {
  const bytes = await pull(`${remoteOut}/${name}`);
  const sha256 = createHash("sha256").update(bytes).digest("hex");
  if (sha256 !== remoteHashes[name]) throw new Error(`${name}: sha256 differs after copy (${sha256} vs ${remoteHashes[name]})`);
  await Bun.write(join(outDir, name), bytes);
  files[name] = { bytes: bytes.length, sha256 };
}
const manifest = { host, commits, seconds: Math.round((Date.now() - started) / 1000), files };
await Bun.write(join(outDir, "manifest.json"), JSON.stringify(manifest, null, 2));
console.log(JSON.stringify(manifest, null, 2));
