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

if (filePath && basename(filePath) === "mod.rs" && !filePath.includes("\\vendor\\") && !filePath.includes("/vendor/")) {
  console.log(
    JSON.stringify({
      systemMessage: `aphrody style note: prefer sibling module files instead of mod.rs (${filePath}).`,
    }),
  );
  process.exit(0);
}

console.log(JSON.stringify({}));
