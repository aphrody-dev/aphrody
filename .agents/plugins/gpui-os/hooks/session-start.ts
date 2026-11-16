#!/usr/bin/env bun
import { existsSync } from "node:fs";
import { join } from "node:path";

interface SessionStartPayload {
  cwd?: string;
}

const payload = (await Bun.stdin.json().catch(() => ({}))) as SessionStartPayload;
const root = payload.cwd ?? process.cwd();
const hasProductManifest = existsSync(join(root, "product.toml"));
const hasPluginDocs = existsSync(join(root, ".agents", "plugins", "gpui-os", "docs", "INDEX.md"));

if (!hasProductManifest) {
  console.log(JSON.stringify({}));
  process.exit(0);
}

const summary = [
  "[gpui-os plugin active]",
  "Brand manifest: product.toml (verify with `cargo xtask brand check`).",
  "Rules: never create mod.rs files, never use .unwrap() on fallible calls, preserve README.md lines 1-2.",
  hasPluginDocs ? "Docs index: .agents/plugins/gpui-os/docs/INDEX.md." : "",
]
  .filter(Boolean)
  .join(" ");

console.log(
  JSON.stringify({
    injectSteps: [
      {
        systemMessage: {
          systemMessage: summary,
        },
      },
    ],
  }),
);
