#!/usr/bin/env bun
import { existsSync } from "node:fs";
import { join, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "../../../..");
const exe = process.platform === "win32" ? "aphrody-os.exe" : "aphrody-os";
const candidates = [
  join(repoRoot, "target", "release", exe),
  join(repoRoot, "target", "release-fast", exe),
  join(repoRoot, "target", "debug", exe),
  Bun.which("aphrody-os") ?? "",
].filter(Boolean);

const bin = candidates.find((p) => existsSync(p));
const cmd = bin
  ? [bin, "mcp"]
  : ["cargo", "run", "--quiet", "-p", "os_desktop", "--bin", "aphrody-os", "--", "mcp"];

const proc = Bun.spawn(cmd, {
  cwd: repoRoot,
  stdin: "inherit",
  stdout: "inherit",
  stderr: "inherit",
  env: process.env,
});
process.exit(await proc.exited);
