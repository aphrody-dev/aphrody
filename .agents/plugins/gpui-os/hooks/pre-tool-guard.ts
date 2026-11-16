#!/usr/bin/env bun
import { basename } from "node:path";

interface PreToolUsePayload {
  toolCall?: {
    name?: string;
    args?: {
      CommandLine?: string;
    };
  };
}

function allow(): never {
  console.log(JSON.stringify({ decision: "allow" }));
  process.exit(0);
}

function deny(reason: string): never {
  console.log(JSON.stringify({ decision: "deny", reason }));
  process.exit(0);
}

const payload = (await Bun.stdin.json().catch(() => ({}))) as PreToolUsePayload;
const cmd = payload.toolCall?.args?.CommandLine ?? "";
if (!cmd.trim()) {
  allow();
}

const trimmed = cmd.trim();
const firstToken = basename((trimmed.split(/\s+/)[0] ?? "").toLowerCase());

if (firstToken === "rustfmt" || firstToken === "rustfmt.exe") {
  deny("Run `cargo fmt --all` instead of invoking `rustfmt` directly so Cargo edition settings match CI.");
}

if (/\bgit\s+reset\s+--hard\b/i.test(trimmed) || /\bgit\s+stash\b/i.test(trimmed)) {
  deny("gpui-os policy: never run `git reset --hard` or `git stash` on main.");
}

if (/\bwinget(\.exe)?\s+(list|search|show)\b/i.test(trimmed) && !/--source\s+winget\b/i.test(trimmed)) {
  deny("Pass `--source winget --accept-source-agreements` to `winget` so the msstore source does not hang.");
}

allow();
