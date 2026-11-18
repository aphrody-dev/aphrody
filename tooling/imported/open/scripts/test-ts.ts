// Runs `bun test` inside each selected workspace package (never at the root).
// Usage: bun scripts/test-ts.ts [path-or-name ...]   (default: every member that has test files)
import { join } from "node:path";

const ROOT = join(import.meta.dir, "..");
// Members are the workspace globs of the root package.json (packages/*, apps/*, extension).
const globs: string[] = (await Bun.file(join(ROOT, "package.json")).json()).workspaces;
const members: string[] = [];
for (const g of globs) {
  for await (const f of new Bun.Glob(`${g}/package.json`).scan({ cwd: ROOT, onlyFiles: true })) members.push(f.replace(/\\/g, "/").replace(/\/package\.json$/, ""));
}
members.sort();
const want = process.argv.slice(2);
const hasTests = async (dir: string) => {
  for await (const _ of new Bun.Glob("**/*.{test,spec}.{ts,tsx,js,jsx}").scan({ cwd: dir })) return true;
  return false;
};
const picked: string[] = [];
for (const m of members) {
  const j = await Bun.file(join(ROOT, m, "package.json")).json();
  if (want.length ? want.some((w) => m === w || j.name === w) : await hasTests(join(ROOT, m))) picked.push(m);
}
let fail = 0;
for (const m of picked) {
  const p = Bun.spawn(["bun", "test"], { cwd: join(ROOT, m), stdout: "inherit", stderr: "inherit" });
  const code = await p.exited;
  console.log(`${code === 0 ? "pass" : "FAIL"} ${m}`);
  if (code !== 0) fail++;
}
console.log(`test-ts: ${picked.length} packages, ${fail} failed`);
process.exitCode = fail ? 1 : 0;
