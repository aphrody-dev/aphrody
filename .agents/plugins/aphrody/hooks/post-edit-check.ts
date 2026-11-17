#!/usr/bin/env bun
import { basename, dirname, resolve } from "node:path";

interface PostToolUsePayload {
  toolCall?: {
    name?: string;
    args?: {
      Path?: string;
      TargetFile?: string;
      path?: string;
    };
  };
  tool_input?: {
    file_path?: string;
    path?: string;
  };
}

const payload = (await Bun.stdin.json().catch(() => ({}))) as PostToolUsePayload;
const filePath =
  payload.toolCall?.args?.Path ??
  payload.toolCall?.args?.TargetFile ??
  payload.toolCall?.args?.path ??
  payload.tool_input?.file_path ??
  payload.tool_input?.path ??
  "";
const normalized = filePath.replaceAll("\\", "/");

if (
  normalized.includes(".gemini/antigravity-cli/brain") ||
  normalized.includes("/AppData/Local/Temp/")
) {
  console.log(
    JSON.stringify({
      systemMessage: `aphrody workflow guard: keep plans under docs/architecture/ and scripts under tooling/bun/ inside the repo, never ${filePath}.`,
    }),
  );
  process.exit(0);
}

if (filePath && basename(filePath) === "mod.rs" && !normalized.includes("/vendor/")) {
  console.log(
    JSON.stringify({
      systemMessage: `aphrody style note: prefer sibling module files instead of mod.rs (${filePath}).`,
    }),
  );
  process.exit(0);
}

if (normalized.endsWith(".ts") && !normalized.includes("/node_modules/")) {
  const cwd = resolve(dirname(filePath));
  const check = Bun.spawnSync(["bun", "check", filePath], {
    cwd,
    stdout: "pipe",
    stderr: "pipe",
    stdin: "ignore",
  });
  if (check.exitCode !== 0) {
    const snippet = new TextDecoder().decode(check.stderr || check.stdout).trim().split(/\r?\n/).slice(0, 6).join("\n");
    if (snippet) {
      process.stderr.write(`[aphrody bun check] ${snippet}\n`);
    }
  }
}

console.log(JSON.stringify({}));
