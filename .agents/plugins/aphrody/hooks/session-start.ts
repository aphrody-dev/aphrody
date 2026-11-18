#!/usr/bin/env bun
import { existsSync, readFileSync } from "node:fs";
import { basename, join, resolve } from "node:path";

interface SessionStartPayload {
  cwd?: string;
}

interface ProductManifestSummary {
  product?: { name?: string };
  plugin?: { name?: string };
  urls?: { repository?: string };
  git?: { default_branch?: string };
}

const payload = (await Bun.stdin.json().catch(() => ({}))) as SessionStartPayload;
const root = payload.cwd ?? process.cwd();
const manifestPath = join(root, "product.toml");
const hasProductManifest = existsSync(manifestPath);

if (!hasProductManifest) {
  console.log(JSON.stringify({}));
  process.exit(0);
}

const manifest = Bun.TOML.parse(readFileSync(manifestPath, "utf8")) as ProductManifestSummary;
const productName = manifest.product?.name ?? "App";
const pluginName = manifest.plugin?.name ?? basename(resolve(import.meta.dir, ".."));
const defaultBranch = manifest.git?.default_branch ?? "main";
const docsIndexRel = `.agents/plugins/${pluginName}/docs/INDEX.md`;
const hasPluginDocs = existsSync(join(root, docsIndexRel));

const summary = [
  `[${pluginName} plugin active for ${productName}]`,
  `Brand manifest: product.toml (branch: ${defaultBranch}, verify with \`cargo xtask brand check\`).`,
  "Rules: never create mod.rs files, never use .unwrap() on fallible calls, preserve README.md lines 1-2, keep docs/scripts repo-native.",
  "Host & MCP: os-craft MCP (aphrody-os mcp) exposes Craft/Compositor/AI Pilot tools; CI/host tuning lives in tooling/bun/ci-host-optimize.ts.",
  hasPluginDocs
    ? `Docs index: ${docsIndexRel}; Architecture: aphrody/docs/architecture/README.md; Hygiene: bun tooling/bun/repo-hygiene.ts.`
    : "",
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
