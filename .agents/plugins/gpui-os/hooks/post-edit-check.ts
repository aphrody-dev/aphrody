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
  console.error(`gpui-os .rules violation: avoid creating mod.rs (${filePath}); use sibling module files instead.`);
  console.log(JSON.stringify({}));
  process.exit(2);
}

console.log(JSON.stringify({}));
