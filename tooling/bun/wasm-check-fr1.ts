#!/usr/bin/env bun
/**
 * Runs CI's exact wasm check on the build host (fr1) at origin/main of aphrody and fw (/srv/xbuild/aphrody-ci/wasm/{aphrody,fw}):
 *   cargo check -p aphrody_desktop --features web --example web_os --target wasm32-unknown-unknown
 * Measures: pass/fail (exit code of cargo), error lines and wall seconds.
 *
 * Run: bun tooling/bun/wasm-check-fr1.ts
 */
import { ciRoot, host, remote, syncRepos, wasmTargetDir } from "./ci-fr1.ts";

const started = Date.now();
const { code, stdout } = await remote(`${syncRepos}
cd ${ciRoot}/wasm/aphrody
export CARGO_TARGET_DIR=${wasmTargetDir}
cargo check -p aphrody_desktop --features web --example web_os --target wasm32-unknown-unknown 2>&1 | tail -40
exit \${PIPESTATUS[0]}
`);
const errors = stdout.split("\n").filter((line) => /^error/.test(line));
console.log(JSON.stringify({ host, command: "cargo check -p aphrody_desktop --features web --example web_os --target wasm32-unknown-unknown", exit: code, errors: errors.length, seconds: Math.round((Date.now() - started) / 1000) }));
if (code !== 0) process.exit(1);
