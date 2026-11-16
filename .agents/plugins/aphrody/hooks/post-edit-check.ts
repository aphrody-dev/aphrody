#!/usr/bin/env bun
import { basename } from "node:path";

interface PostToolUsePayload {
  toolCall?: {
    args?: {
      Path?: string;
    };
  };
}

const payload = (await Bun.stdin.json().catch(() => ({}))) as PostToolUsePayload;
const filePath = payload.toolCall?.args?.Path ?? "";
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

console.log(JSON.stringify({}));
