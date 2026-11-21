#!/usr/bin/env bun
/**
 * Measures the size and build time of the web desktop (aphrody_desktop, example web_os) on the build host (fr1):
 * fetches origin/main of aphrody and fw into /srv/xbuild/aphrody-ci/wasm/{aphrody,fw}, then runs
 *   bun tooling/bun/web-build.ts build --package aphrody_desktop
 * twice, `--release --opt` (full) and `--release --opt --slim --webgpu-only` (slim), and prints raw / brotli / gzip
 * bytes of web_os_bg.wasm per cohort plus build seconds. Reference run (2026-11-21): full simd 12,979,146 raw /
 * 4,477,344 br in 602 s; slim simd 9,942,908 / 2,827,411 in 281 s.
 *
 * Run: bun tooling/bun/web-size.ts [--only full|slim]     (about 15 minutes cold; both variants build serially)
 */
import { ciRoot, host, remote, syncRepos, wasmTargetDir } from "./ci-fr1.ts";

const variants: Record<string, string> = {
  full: "--release --opt",
  slim: "--release --opt --slim --webgpu-only",
};
const only = Bun.argv.includes("--only") ? Bun.argv[Bun.argv.indexOf("--only") + 1] : undefined;
const selected = Object.entries(variants).filter(([name]) => !only || name === only);
if (selected.length === 0) throw new Error(`--only must be one of ${Object.keys(variants).join(", ")}`);

const script = `${syncRepos}
cd ${ciRoot}/wasm/aphrody
export CARGO_TARGET_DIR=${wasmTargetDir}
${selected
  .map(
    ([name, flags]) => `
start=$(date +%s)
if bun tooling/bun/web-build.ts build --package aphrody_desktop ${flags} --out-dir ${ciRoot}/web-${name} > ${ciRoot}/web-${name}.log 2>&1; then
  echo "BUILD ${name} ok $(( $(date +%s) - start ))"
  for wasm in ${ciRoot}/web-${name}/*/web_os_bg.wasm; do
    echo "COHORT ${name} $(basename $(dirname $wasm)) $(stat -c%s $wasm) $(stat -c%s $wasm.br 2>/dev/null || echo -) $(stat -c%s $wasm.gz 2>/dev/null || echo -)"
  done
else
  echo "BUILD ${name} failed $(( $(date +%s) - start ))"
  grep -E "^error" -A6 ${ciRoot}/web-${name}.log | head -30
  tail -5 ${ciRoot}/web-${name}.log
fi`,
  )
  .join("\n")}
`;

const { code, stdout } = await remote(script, true);
const rows: { variant: string; cohort: string; raw: string; br: string; gz: string; seconds: string }[] = [];
const seconds: Record<string, string> = {};
let failed = code !== 0;
for (const line of stdout.split("\n")) {
  const parts = line.trim().split(/\s+/);
  if (parts[0] === "BUILD") {
    seconds[parts[1]] = parts[3];
    if (parts[2] !== "ok") failed = true;
  } else if (parts[0] === "COHORT") rows.push({ variant: parts[1], cohort: parts[2], raw: parts[3], br: parts[4], gz: parts[5], seconds: "" });
  else if (line.trim()) console.error(line);
}
const format = (value: string) => (/^\d+$/.test(value) ? Number(value).toLocaleString("en-US") : value);
console.log(`host=${host}`);
console.log("variant  cohort            raw         br          gz          build_s");
for (const row of rows) {
  console.log(`${row.variant.padEnd(8)} ${row.cohort.padEnd(16)} ${format(row.raw).padStart(11)} ${format(row.br).padStart(11)} ${format(row.gz).padStart(11)} ${(seconds[row.variant] ?? "?").padStart(9)}`);
}
if (failed || rows.length === 0) process.exit(1);
