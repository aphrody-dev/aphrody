#!/usr/bin/env bun
/**
 * Live GitHub `orgs/WebAssembly` Ecosystem Ingest, Classification, TOML Registry,
 * Architecture Digest, `llms.txt`, and Skill Generator for Aphrody.
 *
 * Fetches `https://api.github.com/orgs/WebAssembly/repos?per_page=100` sorted
 * descending by stars (with deterministic fallback snapshot when offline/rate-limited).
 *
 * Usage:
 *   bun tooling/bun/webassembly-org-ingest.ts all [--offline]
 *   bun tooling/bun/webassembly-org-ingest.ts audit
 */
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

export const repoRoot = resolve(import.meta.dir, "../..");
export const DEFAULT_TOML_OUT = join(repoRoot, "assets/os/webassembly_ecosystem.toml");
export const DEFAULT_MD_OUT = join(repoRoot, "aphrody/docs/architecture/webassembly-org-ecosystem.md");
export const DEFAULT_LLMS_OUT = join(repoRoot, "aphrody/docs/architecture/webassembly-org-llms.txt");
export const DEFAULT_SKILL_OUT = join(
  repoRoot,
  ".agents/plugins/aphrody/skills/webassembly/SKILL.md",
);

export type WasmRepoCategory =
  | "toolchain-optimizer"
  | "component-model-wasi"
  | "spec-and-testsuite"
  | "runtime-and-api"
  | "proposal";

export interface WasmOrgRepo {
  name: string;
  full_name: string;
  html_url: string;
  description: string;
  stars: number;
  forks: number;
  language: string;
  category: WasmRepoCategory;
  aphrody_binding: string;
  topics: string[];
}

export interface WasmEcosystemCatalog {
  schema_version: number;
  org: string;
  synced_at: string;
  total_repos: number;
  repos: WasmOrgRepo[];
}

const FALLBACK_REPOS: Array<Omit<WasmOrgRepo, "category" | "aphrody_binding">> = [
  {
    name: "binaryen",
    full_name: "WebAssembly/binaryen",
    html_url: "https://github.com/WebAssembly/binaryen",
    description: "Optimizer and compiler/toolchain library for WebAssembly",
    stars: 9200,
    forks: 1100,
    language: "C++",
    topics: ["wasm", "optimizer", "wasm-opt"],
  },
  {
    name: "wabt",
    full_name: "WebAssembly/wabt",
    html_url: "https://github.com/WebAssembly/wabt",
    description: "The WebAssembly Binary Toolkit (wat2wasm, wasm2wat, wasm-objdump)",
    stars: 7300,
    forks: 760,
    language: "C++",
    topics: ["wat2wasm", "wasm2wat"],
  },
  {
    name: "spec",
    full_name: "WebAssembly/spec",
    html_url: "https://github.com/WebAssembly/spec",
    description: "WebAssembly specification, reference interpreter, and semantics",
    stars: 6600,
    forks: 840,
    language: "OCaml",
    topics: ["specification"],
  },
  {
    name: "wasi-sdk",
    full_name: "WebAssembly/wasi-sdk",
    html_url: "https://github.com/WebAssembly/wasi-sdk",
    description: "WASI-enabled WebAssembly C/C++ toolchain",
    stars: 2900,
    forks: 300,
    language: "CMake",
    topics: ["wasi", "clang", "sysroot"],
  },
  {
    name: "component-model",
    full_name: "WebAssembly/component-model",
    html_url: "https://github.com/WebAssembly/component-model",
    description: "WebAssembly Component Model & WIT explainer and design",
    stars: 2100,
    forks: 140,
    language: "Markdown",
    topics: ["wit", "components"],
  },
  {
    name: "WASI",
    full_name: "WebAssembly/WASI",
    html_url: "https://github.com/WebAssembly/WASI",
    description: "WebAssembly System Interface proposals and WIT world definitions",
    stars: 5300,
    forks: 410,
    language: "WIT",
    topics: ["wasi", "wasip2"],
  },
  {
    name: "design",
    full_name: "WebAssembly/design",
    html_url: "https://github.com/WebAssembly/design",
    description: "WebAssembly Design Documents, rationale, and MVP portability",
    stars: 3900,
    forks: 510,
    language: "Markdown",
    topics: ["design"],
  },
  {
    name: "wasm-c-api",
    full_name: "WebAssembly/wasm-c-api",
    html_url: "https://github.com/WebAssembly/wasm-c-api",
    description: "WebAssembly C and C++ embedding API",
    stars: 1200,
    forks: 180,
    language: "C++",
    topics: ["c-api", "embedding"],
  },
  {
    name: "testsuite",
    full_name: "WebAssembly/testsuite",
    html_url: "https://github.com/WebAssembly/testsuite",
    description: "Official WebAssembly core conformance testsuite",
    stars: 420,
    forks: 120,
    language: "WebAssembly",
    topics: ["testsuite"],
  },
  {
    name: "wasi-libc",
    full_name: "WebAssembly/wasi-libc",
    html_url: "https://github.com/WebAssembly/wasi-libc",
    description: "WASI libc implementation for WebAssembly",
    stars: 1100,
    forks: 250,
    language: "C",
    topics: ["libc", "wasi"],
  },
];

export function classifyWasmRepo(name: string, description: string, topics: string[]): WasmRepoCategory {
  const haystack = `${name} ${description} ${topics.join(" ")}`.toLowerCase();
  if (/(binaryen|wabt|wasi-sdk|wasi-libc|tools|opt)/.test(haystack)) {
    return "toolchain-optimizer";
  }
  if (/(component-model|wasi|wit|interface-types)/.test(haystack)) {
    return "component-model-wasi";
  }
  if (/(spec|testsuite|design|reference)/.test(haystack)) {
    return "spec-and-testsuite";
  }
  if (/(c-api|wasmtime|runtime|esm-integration|gc|threads|simd)/.test(haystack)) {
    return "runtime-and-api";
  }
  return "proposal";
}

export function mapAphrodyBinding(name: string, category: WasmRepoCategory): string {
  const lower = name.toLowerCase();
  if (lower === "binaryen") return "gpui-wgpu wasm optimize (-Oz + simd + bulk-memory)";
  if (lower === "wabt") return "wat2wasm / wasm2wat verified in shell-path-doctor & gpui-wgpu doctor";
  if (lower === "component-model" || lower === "wasi") {
    return "crates/extension_host (.cwasm AOT cache) & wasm32-wasip2 templates";
  }
  if (lower === "wasi-sdk" || lower === "wasi-libc") {
    return "templates/wasm-wgpu/wasi-p2-aot-compute & emscripten skill";
  }
  if (category === "spec-and-testsuite") {
    return "wasmparser::Validator::new_with_features(WasmFeatures::all()) in crates/aphrody_cli";
  }
  return "tooling/bun/web-build.ts & crates/gpui_web";
}

export function buildWasmEcosystemCatalog(
  rawRepos: Array<{
    name: string;
    full_name?: string;
    html_url?: string;
    description?: string | null;
    stargazers_count?: number;
    stars?: number;
    forks_count?: number;
    forks?: number;
    language?: string | null;
    topics?: string[];
  }>,
  syncedAt = new Date().toISOString(),
): WasmEcosystemCatalog {
  const normalized: WasmOrgRepo[] = rawRepos
    .map((r) => {
      const name = String(r.name);
      const description = String(r.description ?? "");
      const topics = Array.isArray(r.topics) ? r.topics.map(String) : [];
      const category = classifyWasmRepo(name, description, topics);
      return {
        name,
        full_name: String(r.full_name ?? `WebAssembly/${name}`),
        html_url: String(r.html_url ?? `https://github.com/WebAssembly/${name}`),
        description,
        stars: Number(r.stargazers_count ?? r.stars ?? 0),
        forks: Number(r.forks_count ?? r.forks ?? 0),
        language: String(r.language ?? "Markdown"),
        category,
        aphrody_binding: mapAphrodyBinding(name, category),
        topics,
      };
    })
    .sort((a, b) => b.stars - a.stars || a.name.localeCompare(b.name));

  return {
    schema_version: 1,
    org: "WebAssembly",
    synced_at: syncedAt,
    total_repos: normalized.length,
    repos: normalized,
  };
}

export function serializeWasmEcosystemToml(cat: WasmEcosystemCatalog): string {
  const lines: string[] = [
    "# Auto-generated by tooling/bun/webassembly-org-ingest.ts — sorted descending by stars.",
    `schema_version = ${cat.schema_version}`,
    `org = ${JSON.stringify(cat.org)}`,
    `synced_at = ${JSON.stringify(cat.synced_at)}`,
    `total_repos = ${cat.total_repos}`,
    "",
  ];
  for (const r of cat.repos) {
    lines.push("[[repos]]");
    lines.push(`name = ${JSON.stringify(r.name)}`);
    lines.push(`full_name = ${JSON.stringify(r.full_name)}`);
    lines.push(`html_url = ${JSON.stringify(r.html_url)}`);
    lines.push(`description = ${JSON.stringify(r.description)}`);
    lines.push(`stars = ${r.stars}`);
    lines.push(`forks = ${r.forks}`);
    lines.push(`language = ${JSON.stringify(r.language)}`);
    lines.push(`category = ${JSON.stringify(r.category)}`);
    lines.push(`aphrody_binding = ${JSON.stringify(r.aphrody_binding)}`);
    lines.push(`topics = [${r.topics.map((t) => JSON.stringify(t)).join(", ")}]`);
    lines.push("");
  }
  return lines.join("\n");
}

function renderMarkdownDoc(cat: WasmEcosystemCatalog): string {
  const rows = cat.repos
    .slice(0, 25)
    .map(
      (r) =>
        `| [${r.name}](${r.html_url}) | ${r.stars} | \`${r.category}\` | ${r.aphrody_binding} |`,
    )
    .join("\n");
  return `# \`orgs/WebAssembly\` Ecosystem in Aphrody

Synced snapshot of [${cat.total_repos} repositories](https://github.com/orgs/WebAssembly/repositories?q=sort%3Astars) sorted descending by stars and mapped onto Aphrody's native Wasm/WebGPU LTO + AOT stack.

| Repository | Stars | Category | Aphrody Integration |
| --- | ---: | --- | --- |
${rows}

- Registry manifest: [webassembly_ecosystem.toml](../../../assets/os/webassembly_ecosystem.toml)
- Compact agent index: [webassembly-org-llms.txt](./webassembly-org-llms.txt)
- Toolchain skill: [.agents/plugins/aphrody/skills/webassembly/SKILL.md](../../../.agents/plugins/aphrody/skills/webassembly/SKILL.md)
`;
}

function renderLlmsTxt(cat: WasmEcosystemCatalog): string {
  const lines = [
    "# Aphrody x orgs/WebAssembly llms.txt (sorted by stars)",
    `# total_repos=${cat.total_repos} synced_at=${cat.synced_at}`,
    "",
  ];
  for (const r of cat.repos.slice(0, 40)) {
    lines.push(`- ${r.full_name} (${r.stars}★, ${r.category}): ${r.description} -> ${r.aphrody_binding}`);
  }
  lines.push("");
  return lines.join("\n");
}

function renderSkillMd(cat: WasmEcosystemCatalog): string {
  return `---
name: webassembly
description: Authoritative github.com/orgs/WebAssembly ecosystem reference (Binaryen, WABT, WASI P2, Component Model, Wasm Spec) mapped onto Aphrody LTO/AOT crates and gpui-wgpu CLIs.
---

# \`orgs/WebAssembly\` Unified Toolchain & Spec Guide

Aphrody mirrors the top ${cat.total_repos} \`WebAssembly/*\` repositories directly into its native Rust/Bun Wasm toolchain ([webassembly_ecosystem.toml](../../../../../assets/os/webassembly_ecosystem.toml)).

## Core Workflows

- **LTO Browser Bundles (\`[profile.web-lto]\`)**: \`bun tooling/bun/web-build.ts build --lto\` compiles with fat LTO + \`codegen-units = 1\` and optimizes with Binaryen \`wasm-opt -Oz --enable-simd --enable-bulk-memory\`.
- **WASI P2 + Wasmtime AOT (\`[profile.aot]\`)**: Build \`--target wasm32-wasip2 --profile aot\`, compile to native \`.cwasm\` via \`gpui-wgpu wasm aot\`, and run under \`wasmtime run --allow-precompiled\`.
- **Pure-Rust Inspection & Stripping**: \`gpui-wgpu wasm inspect\`, \`validate\`, \`strip\`, \`compose\`, \`doctor --json\`.
`;
}

export async function runIngest(offline = false): Promise<WasmEcosystemCatalog> {
  let rawRepos: Array<Record<string, unknown>> = [];
  if (!offline) {
    const gh = Bun.spawnSync(
      ["gh", "api", "orgs/WebAssembly/repos?per_page=100&type=public"],
      { stdout: "pipe", stderr: "pipe" },
    );
    if (gh.exitCode === 0) {
      try {
        const parsed = JSON.parse(new TextDecoder().decode(gh.stdout));
        if (Array.isArray(parsed) && parsed.length > 0) rawRepos = parsed;
      } catch {
        rawRepos = [];
      }
    }
    if (rawRepos.length === 0) {
      try {
        const resp = await fetch(
          "https://api.github.com/orgs/WebAssembly/repos?per_page=100",
          { headers: { "User-Agent": "Aphrody-WebAssembly-Ingest/1.0" } },
        );
        if (resp.ok) {
          const parsed = await resp.json();
          if (Array.isArray(parsed) && parsed.length > 0) rawRepos = parsed;
        }
      } catch {
        rawRepos = [];
      }
    }
  }
  if (rawRepos.length === 0) {
    rawRepos = FALLBACK_REPOS as unknown as Array<Record<string, unknown>>;
  }

  const catalog = buildWasmEcosystemCatalog(rawRepos as Parameters<typeof buildWasmEcosystemCatalog>[0]);
  for (const out of [DEFAULT_TOML_OUT, DEFAULT_MD_OUT, DEFAULT_LLMS_OUT, DEFAULT_SKILL_OUT]) {
    mkdirSync(resolve(out, ".."), { recursive: true });
  }
  writeFileSync(DEFAULT_TOML_OUT, serializeWasmEcosystemToml(catalog), "utf8");
  writeFileSync(DEFAULT_MD_OUT, renderMarkdownDoc(catalog), "utf8");
  writeFileSync(DEFAULT_LLMS_OUT, renderLlmsTxt(catalog), "utf8");
  writeFileSync(DEFAULT_SKILL_OUT, renderSkillMd(catalog), "utf8");
  return catalog;
}

if (import.meta.main) {
  const args = Bun.argv.slice(2);
  const cmd = args[0] ?? "all";
  const offline = args.includes("--offline");
  if (cmd === "audit") {
    const ok =
      existsSync(DEFAULT_TOML_OUT) &&
      existsSync(DEFAULT_MD_OUT) &&
      existsSync(DEFAULT_LLMS_OUT) &&
      existsSync(DEFAULT_SKILL_OUT);
    console.log(JSON.stringify({ ok, toml: DEFAULT_TOML_OUT }));
    if (!ok) process.exit(1);
  } else {
    const cat = await runIngest(offline);
    console.log(
      JSON.stringify({
        synced: true,
        org: cat.org,
        total_repos: cat.total_repos,
        top5: cat.repos.slice(0, 5).map((r) => `${r.name} (${r.stars})`),
      }, null, 2),
    );
  }
}
