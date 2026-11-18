#!/usr/bin/env bun
/**
 * Production Standards & GPUI Starter Pack Matrix / Benchmark / AI-Agent Readiness Engine.
 *
 * Unifies:
 * - Core Web Vitals (LCP/INP/CLS/FCP/TTFB + 60/120Hz WebGPU frame & Wasm Brotli budgets)
 * - Lighthouse 4-category scoring (Performance, Accessibility, Best Practices, SEO/Agentic)
 * - W3C Standards (HTML5, WCAG 2.2 AA, WAI-ARIA, CharMod-Norm, BCP 47 + String-Meta, DTCG)
 * - Google Style Guides (HTML/CSS, TypeScript, Rust, https://design.google/ M3 Expressive)
 * - Vercel Web Interface Guidelines (focus-visible, reduced motion, tabular-nums, touch targets)
 * - shadcn/ui + Base UI + Tailwind v4 -> Pure-Rust GPUI & NSH Starter Pack + Fork Submodule Parity Matrix
 *
 * Usage:
 *   bun tooling/bun/prod-standards-starterpack.ts matrix [--json]
 *   bun tooling/bun/prod-standards-starterpack.ts bench [--json]
 *   bun tooling/bun/prod-standards-starterpack.ts audit [--json]
 *   bun tooling/bun/prod-standards-starterpack.ts all [--json]
 */
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, join, relative, resolve } from "node:path";
import { parseGitmodules } from "./submodules.ts";

export const REPO_ROOT = resolve(import.meta.dir, "../..");
export const PARITY_MATRIX_TOML_REL = "assets/os/submodule_parity_matrix.toml";
export const PARITY_DOC_REL = "aphrody/docs/architecture/gpui-starterpack-and-parity-matrix.md";
export const AGENT_LLMS_REL = "aphrody/docs/architecture/ai-agent-readiness-llms.txt";
export const BENCH_OUT_REL = "target/wasm-wgpu-proof/starterpack-bench.json";

export interface CoreWebVitalsBudgets {
  lcpGoodMs: number;
  inpGoodMs: number;
  clsGoodMax: number;
  fcpGoodMs: number;
  ttfbGoodMs: number;
  gpuFrame60HzMs: number;
  gpuFrame120HzMs: number;
  gpuiInputLatencyMs: number;
  wasmCriticalBrotliBytes: number;
}

export const CORE_WEB_VITALS_BUDGETS: CoreWebVitalsBudgets = {
  lcpGoodMs: 2500,
  inpGoodMs: 200,
  clsGoodMax: 0.1,
  fcpGoodMs: 1800,
  ttfbGoodMs: 800,
  gpuFrame60HzMs: 16.67,
  gpuFrame120HzMs: 8.33,
  gpuiInputLatencyMs: 8.0,
  wasmCriticalBrotliBytes: 358_400,
};

export interface FrameworkParityRow {
  id: string;
  label: string;
  repoOrSubmodule: string;
  presentOnDisk: boolean;
  manifestPresent: boolean;
  cargoPatched: boolean;
  wasm32Unknown: boolean;
  wasip2Aot: boolean;
  wgpuV30Webgpu: boolean;
  taffyLayout: boolean;
  accesskitAria: boolean;
  googleDesignTailwind: boolean;
  shadcnBaseUiStarter: boolean;
  aiAgentMcp: boolean;
  moduleFileCount: number;
  role: string;
}

export interface BenchmarkMetric {
  id: string;
  iterations: number;
  opsPerSec: number;
  p50Us: number;
  p95Us: number;
}

export interface StandardsAuditFinding {
  scope: "typescript" | "rust";
  rule: string;
  severity: "error" | "warn";
  file: string;
  line: number;
  message: string;
}

export interface MonorepoReadinessReport {
  ok: boolean;
  tsFilesScanned: number;
  tsCompileErrors: number;
  tsExplicitAnyWarnings: number;
  rustFilesScanned: number;
  rustModRsViolations: number;
  rustProdUnwrapViolations: number;
  rustLetUnderscoreViolations: number;
  machinePathLeaks: number;
  findings: StandardsAuditFinding[];
}

interface SubmoduleTemplateSpec {
  id: string;
  label: string;
  path: string;
  wasm32Unknown: boolean;
  wasip2Aot: boolean;
  wgpuV30Webgpu: boolean;
  taffyLayout: boolean;
  accesskitAria: boolean;
  googleDesignTailwind: boolean;
  shadcnBaseUiStarter: boolean;
  aiAgentMcp: boolean;
  role: string;
}

const SUBMODULE_SPECS: SubmoduleTemplateSpec[] = [
  {
    id: "wgpu",
    label: "WGPU v30 + Naga (aphrody-labs/wgpu)",
    path: "vendor/wgpu",
    wasm32Unknown: true,
    wasip2Aot: true,
    wgpuV30Webgpu: true,
    taffyLayout: false,
    accesskitAria: false,
    googleDesignTailwind: false,
    shadcnBaseUiStarter: false,
    aiAgentMcp: true,
    role: "Single-source DX12/Metal/Vulkan/WebGPU/WebGL2 + WGSL/Naga shader stack",
  },
  {
    id: "taffy",
    label: "Taffy Layout (aphrody-labs/taffy)",
    path: "vendor/taffy",
    wasm32Unknown: true,
    wasip2Aot: true,
    wgpuV30Webgpu: false,
    taffyLayout: true,
    accesskitAria: false,
    googleDesignTailwind: false,
    shadcnBaseUiStarter: false,
    aiAgentMcp: false,
    role: "CSS Flexbox, Grid, and Block layout engine patched into GPUI & NSH",
  },
  {
    id: "accesskit",
    label: "AccessKit (aphrody-labs/accesskit)",
    path: "vendor/accesskit",
    wasm32Unknown: true,
    wasip2Aot: false,
    wgpuV30Webgpu: false,
    taffyLayout: false,
    accesskitAria: true,
    googleDesignTailwind: false,
    shadcnBaseUiStarter: false,
    aiAgentMcp: true,
    role: "OS UIA/AT-SPI2/AX/ARIA semantic tree bridge used by aphrody_pilot, egui & GPUI",
  },
  {
    id: "cosmic-text",
    label: "Cosmic Text (aphrody-labs/cosmic-text)",
    path: "vendor/cosmic-text",
    wasm32Unknown: true,
    wasip2Aot: true,
    wgpuV30Webgpu: false,
    taffyLayout: false,
    accesskitAria: false,
    googleDesignTailwind: false,
    shadcnBaseUiStarter: false,
    aiAgentMcp: false,
    role: "Shared HarfBuzz/Swash multi-line text shaping and Google Sans Flex metrics",
  },
  {
    id: "egui",
    label: "Egui (aphrody-labs/egui)",
    path: "vendor/egui",
    wasm32Unknown: true,
    wasip2Aot: false,
    wgpuV30Webgpu: true,
    taffyLayout: false,
    accesskitAria: true,
    googleDesignTailwind: false,
    shadcnBaseUiStarter: false,
    aiAgentMcp: false,
    role: "Immediate-mode inspector embedded cleanly via crates/egui_gpui on shared wgpu + accesskit",
  },
  {
    id: "wry",
    label: "WRY WebView (aphrody-labs/wry)",
    path: "vendor/tauri/wry",
    wasm32Unknown: false,
    wasip2Aot: false,
    wgpuV30Webgpu: false,
    taffyLayout: false,
    accesskitAria: true,
    googleDesignTailwind: false,
    shadcnBaseUiStarter: false,
    aiAgentMcp: false,
    role: "Cross-platform OS WebView engine consumed by nsh-platform-gpui",
  },
  {
    id: "craft-libs",
    label: "Craft Libs (aphrody-labs/craft-libs)",
    path: "vendor/craft-libs",
    wasm32Unknown: true,
    wasip2Aot: true,
    wgpuV30Webgpu: false,
    taffyLayout: false,
    accesskitAria: false,
    googleDesignTailwind: false,
    shadcnBaseUiStarter: false,
    aiAgentMcp: true,
    role: "Pure-Rust file sniffing and format decoders embedded in aphrody_craft",
  },
  {
    id: "craft-anything",
    label: "Craft Anything (aphrody-labs/craft-anything)",
    path: "vendor/craft-anything",
    wasm32Unknown: true,
    wasip2Aot: false,
    wgpuV30Webgpu: true,
    taffyLayout: false,
    accesskitAria: true,
    googleDesignTailwind: false,
    shadcnBaseUiStarter: false,
    aiAgentMcp: true,
    role: "Creative format analysis, JS/disasm inspector & MCP integration",
  },
  {
    id: "mullvad",
    label: "Mullvad VPN (aphrody-labs/mullvadvpn-app)",
    path: "vendor/mullvad",
    wasm32Unknown: false,
    wasip2Aot: false,
    wgpuV30Webgpu: false,
    taffyLayout: false,
    accesskitAria: false,
    googleDesignTailwind: false,
    shadcnBaseUiStarter: false,
    aiAgentMcp: true,
    role: "WireGuard split-tunnel daemon & Talpid crates used by aphrody_vpn",
  },
  {
    id: "browser",
    label: "Aphrody Browser (aphrody-labs/browser)",
    path: "vendor/browser",
    wasm32Unknown: true,
    wasip2Aot: false,
    wgpuV30Webgpu: true,
    taffyLayout: false,
    accesskitAria: true,
    googleDesignTailwind: false,
    shadcnBaseUiStarter: false,
    aiAgentMcp: true,
    role: "Browser automation CLI + CDP daemon powering aphrody_browser & web verification",
  },
];

function walkFiles(dir: string, exts: Set<string>, limit = 2000): string[] {
  if (!existsSync(dir)) return [];
  const results: string[] = [];
  const stack = [dir];
  while (stack.length > 0 && results.length < limit) {
    const current = stack.pop();
    if (!current) continue;
    let entries;
    try {
      entries = readdirSync(current, { withFileTypes: true });
    } catch {
      continue;
    }
    for (const entry of entries) {
      if (results.length >= limit) break;
      if (entry.name === ".git" || entry.name === "target" || entry.name === "node_modules") {
        continue;
      }
      const full = join(current, entry.name);
      if (entry.isDirectory()) {
        stack.push(full);
      } else {
        const dot = entry.name.lastIndexOf(".");
        const ext = dot >= 0 ? entry.name.slice(dot) : "";
        if (exts.has(ext)) results.push(full);
      }
    }
  }
  return results;
}

function parsePatchPaths(cargoTomlRaw: string): Set<string> {
  const patched = new Set<string>();
  const idx = cargoTomlRaw.indexOf("[patch.crates-io]");
  if (idx < 0) return patched;
  const tail = cargoTomlRaw.slice(idx);
  const nextSection = tail.slice(17).search(/\n\[/);
  const block = nextSection >= 0 ? tail.slice(0, nextSection + 17) : tail;
  const re = /path\s*=\s*"([^"]+)"/g;
  let match: RegExpExecArray | null = re.exec(block);
  while (match !== null) {
    if (match[1]) patched.add(match[1]);
    match = re.exec(block);
  }
  return patched;
}

export function buildParityMatrix(root = REPO_ROOT): FrameworkParityRow[] {
  const cargoTomlRaw = readFileSync(join(root, "Cargo.toml"), "utf8");
  const patchedPaths = parsePatchPaths(cargoTomlRaw);
  const gitmodulesRaw = readFileSync(join(root, ".gitmodules"), "utf8");
  const declared = new Map(parseGitmodules(gitmodulesRaw).map((entry) => [entry.path, entry]));

  const coreModuleCount =
    walkFiles(join(root, "crates/aphrody_ui/src"), new Set([".rs"])).length +
    walkFiles(join(root, "crates/ui/src"), new Set([".rs"])).length +
    walkFiles(join(root, "crates/nsh-base/src"), new Set([".rs"])).length +
    walkFiles(join(root, "crates/aphrody_design/src"), new Set([".rs"])).length;

  const rows: FrameworkParityRow[] = [
    {
      id: "aphrody-core",
      label: "Aphrody GPUI Core + Starter Pack",
      repoOrSubmodule: "crates/{gpui,gpui_ui,nsh-base,aphrody_ui,gpui_wgpu,aphrody_design}",
      presentOnDisk: true,
      manifestPresent: true,
      cargoPatched: true,
      wasm32Unknown: true,
      wasip2Aot: true,
      wgpuV30Webgpu: true,
      taffyLayout: true,
      accesskitAria: true,
      googleDesignTailwind: true,
      shadcnBaseUiStarter: true,
      aiAgentMcp: true,
      moduleFileCount: coreModuleCount,
      role: "Unified Core Web Vitals, Lighthouse, W3C, Google Design, Vercel, and shadcn/Base UI/Tailwind GPUI starter stack",
    },
  ];

  for (const spec of SUBMODULE_SPECS) {
    const dir = join(root, spec.path);
    const present = existsSync(dir);
    const manifestPresent =
      existsSync(join(dir, "Cargo.toml")) || existsSync(join(dir, "package.json"));
    const cargoPatched = Array.from(patchedPaths).some(
      (p) => p === spec.path || p.startsWith(`${spec.path}/`),
    );
    const moduleFileCount =
      spec.id === "bun"
        ? walkFiles(join(dir, "packages"), new Set([".ts", ".tsx", ".rs"]), 500).length
        : walkFiles(dir, new Set([".rs"]), 500).length;

    const declaredEntry = declared.get(spec.path);
    rows.push({
      id: spec.id,
      label: spec.label,
      repoOrSubmodule: declaredEntry?.path ?? spec.path,
      presentOnDisk: present,
      manifestPresent,
      cargoPatched,
      wasm32Unknown: spec.wasm32Unknown,
      wasip2Aot: spec.wasip2Aot,
      wgpuV30Webgpu: spec.wgpuV30Webgpu,
      taffyLayout: spec.taffyLayout,
      accesskitAria: spec.accesskitAria,
      googleDesignTailwind: spec.googleDesignTailwind,
      shadcnBaseUiStarter: spec.shadcnBaseUiStarter,
      aiAgentMcp: spec.aiAgentMcp,
      moduleFileCount,
      role: spec.role,
    });
  }

  return rows;
}

function runMicroBench(id: string, iterations: number, fn: () => unknown): BenchmarkMetric {
  for (let i = 0; i < 10; i++) {
    fn();
  }
  const samplesNs: number[] = [];
  let totalNs = 0;
  for (let i = 0; i < iterations; i++) {
    const t0 = Bun.nanoseconds();
    fn();
    const dt = Bun.nanoseconds() - t0;
    samplesNs.push(dt);
    totalNs += dt;
  }
  samplesNs.sort((a, b) => a - b);
  const p50 = samplesNs[Math.floor(samplesNs.length * 0.5)] ?? 0;
  const p95 = samplesNs[Math.floor(samplesNs.length * 0.95)] ?? 0;
  const avgSec = totalNs / 1e9;
  return {
    id,
    iterations,
    opsPerSec: avgSec > 0 ? Math.round(iterations / avgSec) : 0,
    p50Us: Number((p50 / 1000).toFixed(2)),
    p95Us: Number((p95 / 1000).toFixed(2)),
  };
}

export function runParityBenchmarks(root = REPO_ROOT): BenchmarkMetric[] {
  const designMdRaw = readFileSync(join(root, "DESIGN.md"), "utf8");
  const sampleHtml = [
    "<!doctype html>",
    '<html lang="en" dir="ltr">',
    "<head>",
    '<meta charset="utf-8">',
    '<meta name="viewport" content="width=device-width, initial-scale=1.0">',
    "<title>Aphrody Starter Pack</title>",
    "<style>:root { --md-sys-color-primary: #8FD0FF; --background: #0E141B; } :focus-visible { outline: 2px solid var(--md-sys-color-primary); }</style>",
    "</head>",
    '<body><main data-slot="shell"><canvas width="1280" height="720"></canvas><button type="button" data-stitch-id="btn-1">Run</button></main></body>',
    "</html>",
  ].join("\n");
  const gitmodulesRaw = readFileSync(join(root, ".gitmodules"), "utf8");
  const cargoTomlRaw = readFileSync(join(root, "Cargo.toml"), "utf8");

  return [
    runMicroBench("design_md_frontmatter_parse", 200, () => {
      const m = /^---\r?\n([\s\S]*?)\r?\n---/.exec(designMdRaw);
      return m ? m[1]?.split("\n").length ?? 0 : 0;
    }),
    runMicroBench("shadcn_tailwind_v4_css_emit", 200, () => {
      const lines = designMdRaw.match(/^\s{2}[a-z0-9-]+:\s*"#[0-9A-Fa-f]{6}"/gm) ?? [];
      return `@theme inline {\n${lines.map((l) => `  --color-${l.trim()}`).join(";\n")}\n}`;
    }),
    runMicroBench("cwv_lighthouse_w3c_html_audit", 200, () => {
      const lower = sampleHtml.toLowerCase();
      return (
        lower.includes("<!doctype html>") &&
        lower.includes('charset="utf-8"') &&
        lower.includes('name="viewport"') &&
        lower.includes("lang=") &&
        lower.includes("dir=") &&
        lower.includes(":focus-visible") &&
        lower.includes("--md-sys-")
      );
    }),
    runMicroBench("submodule_parity_graph_resolve", 50, () => {
      const subs = parseGitmodules(gitmodulesRaw);
      const patches = parsePatchPaths(cargoTomlRaw);
      return subs.length + patches.size;
    }),
  ];
}

function stripCfgTestTail(rustSource: string): { code: string; testStartLine: number } {
  const idx = rustSource.search(/\n\s*#\[cfg\(test\)\]/);
  if (idx < 0) return { code: rustSource, testStartLine: Number.MAX_SAFE_INTEGER };
  const prefix = rustSource.slice(0, idx);
  return { code: prefix, testStartLine: prefix.split(/\r?\n/).length };
}

export function auditMonorepoReadiness(root = REPO_ROOT): MonorepoReadinessReport {
  const findings: StandardsAuditFinding[] = [];
  const transpiler = new Bun.Transpiler({ loader: "ts" });

  const tsFiles = [
    ...walkFiles(join(root, "tooling/bun"), new Set([".ts", ".tsx"])),
    ...walkFiles(join(root, ".agents/plugins/aphrody"), new Set([".ts", ".tsx"])),
  ];

  let tsCompileErrors = 0;
  let tsExplicitAnyWarnings = 0;
  let machinePathLeaks = 0;

  for (const file of tsFiles) {
    const rel = relative(root, file).replaceAll("\\", "/");
    const raw = readFileSync(file, "utf8");
    try {
      transpiler.transformSync(raw);
    } catch (err) {
      tsCompileErrors++;
      findings.push({
        scope: "typescript",
        rule: "google-ts/valid-syntax",
        severity: "error",
        file: rel,
        line: 1,
        message: err instanceof Error ? err.message : String(err),
      });
    }

    const lines = raw.split(/\r?\n/);
    for (let idx = 0; idx < lines.length; idx++) {
      const line = lines[idx] ?? "";
      const lineNo = idx + 1;
      const isSelfAuditHelper = rel.endsWith("prod-standards-starterpack.ts") || rel.endsWith("repo-hygiene.ts");
      if (
        !isSelfAuditHelper &&
        (/\bfrom\s+["'](?:node:)?child_process["']/.test(line) || /\baxios\b/.test(line))
      ) {
        findings.push({
          scope: "typescript",
          rule: "bun-native/no-legacy-node-subprocess",
          severity: "error",
          file: rel,
          line: lineNo,
          message: "Use Bun.spawn / Bun.spawnSync / fetch instead of child_process or axios",
        });
      }
      if (
        !isSelfAuditHelper &&
        (/C:\\Users\\[A-Za-z0-9._-]+/i.test(line) || /file:\/\/\/[A-Za-z]:\/Users\//i.test(line))
      ) {
        machinePathLeaks++;
        findings.push({
          scope: "typescript",
          rule: "aphrody/no-machine-paths",
          severity: "error",
          file: rel,
          line: lineNo,
          message: "Machine-specific user directory path leaked in tracked TypeScript",
        });
      }
      if (!isSelfAuditHelper && /(?::\s*any\b|\bas\s+any\b)/.test(line)) {
        tsExplicitAnyWarnings++;
        findings.push({
          scope: "typescript",
          rule: "google-ts/no-explicit-any",
          severity: "warn",
          file: rel,
          line: lineNo,
          message: "Prefer explicit interfaces or `unknown` over `any`",
        });
      }
    }
  }

  const rustCrateDirs = [
    "crates/brand/src",
    "crates/aphrody_design/src",
    "crates/aphrody_ui/src",
    "crates/aphrody_store/src",
    "crates/gpui_wgpu/src",
    "crates/aphrody_nsh/src",
    "crates/aphrody_cli/src",
    "crates/aphrody_craft/src",
  ];
  const rustFiles = rustCrateDirs.flatMap((d) => walkFiles(join(root, d), new Set([".rs"])));
  let rustModRsViolations = 0;
  let rustProdUnwrapViolations = 0;
  let rustLetUnderscoreViolations = 0;

  for (const file of rustFiles) {
    const rel = relative(root, file).replaceAll("\\", "/");
    if (basename(file) === "mod.rs") {
      rustModRsViolations++;
      findings.push({
        scope: "rust",
        rule: "aphrody-rust/no-mod-rs",
        severity: "error",
        file: rel,
        line: 1,
        message: "Use `<module>.rs` instead of `mod.rs`",
      });
    }
    const raw = readFileSync(file, "utf8");
    const { code } = stripCfgTestTail(raw);
    const prodLines = code.split(/\r?\n/);
    for (let idx = 0; idx < prodLines.length; idx++) {
      const line = prodLines[idx] ?? "";
      const lineNo = idx + 1;
      const trimmed = line.trim();
      if (trimmed.startsWith("//")) continue;
      if (/[A-Za-z0-9_)\]]\.unwrap\(\)/.test(line)) {
        rustProdUnwrapViolations++;
        findings.push({
          scope: "rust",
          rule: "aphrody-rust/no-prod-unwrap",
          severity: "error",
          file: rel,
          line: lineNo,
          message: "Avoid `.unwrap()` in production code; propagate Result via `?`",
        });
      }
      if (/^\s*let\s+_\s*=/.test(line) && !trimmed.includes("// intentional")) {
        rustLetUnderscoreViolations++;
        findings.push({
          scope: "rust",
          rule: "aphrody-rust/no-let-underscore",
          severity: "warn",
          file: rel,
          line: lineNo,
          message: "Handle or log fallible results explicitly instead of `let _ = ...`",
        });
      }
      if (/C:\\Users\\[A-Za-z0-9._-]+/i.test(line)) {
        machinePathLeaks++;
        findings.push({
          scope: "rust",
          rule: "aphrody/no-machine-paths",
          severity: "error",
          file: rel,
          line: lineNo,
          message: "Machine-specific user directory path leaked in Rust source",
        });
      }
    }
  }

  const errorCount = findings.filter((f) => f.severity === "error").length;
  return {
    ok: errorCount === 0,
    tsFilesScanned: tsFiles.length,
    tsCompileErrors,
    tsExplicitAnyWarnings,
    rustFilesScanned: rustFiles.length,
    rustModRsViolations,
    rustProdUnwrapViolations,
    rustLetUnderscoreViolations,
    machinePathLeaks,
    findings,
  };
}

function escapeTomlString(value: string): string {
  return `"${value.replaceAll("\\", "\\\\").replaceAll('"', '\\"')}"`;
}

export function renderParityMatrixToml(
  rows: FrameworkParityRow[],
  audit: MonorepoReadinessReport,
): string {
  const lines: string[] = [
    "# Declarative GPUI Starter Pack & Submodule Parity Feature Matrix",
    "# Generated by `bun tooling/bun/prod-standards-starterpack.ts all` — embedded by `crates/aphrody_ui`.",
    "schema_version = 1",
    'design_source_of_truth = "https://design.google/"',
    'starter_pack_crate = "crates/aphrody_ui/src/starter_pack.rs"',
    'web_standards_crate = "crates/aphrody_design/src/web_standards.rs"',
    "",
    "[core_web_vitals_budgets]",
    `lcp_good_ms = ${CORE_WEB_VITALS_BUDGETS.lcpGoodMs}`,
    `inp_good_ms = ${CORE_WEB_VITALS_BUDGETS.inpGoodMs}`,
    `cls_good_max = ${CORE_WEB_VITALS_BUDGETS.clsGoodMax}`,
    `fcp_good_ms = ${CORE_WEB_VITALS_BUDGETS.fcpGoodMs}`,
    `ttfb_good_ms = ${CORE_WEB_VITALS_BUDGETS.ttfbGoodMs}`,
    `gpu_frame_60hz_ms = ${CORE_WEB_VITALS_BUDGETS.gpuFrame60HzMs}`,
    `gpu_frame_120hz_ms = ${CORE_WEB_VITALS_BUDGETS.gpuFrame120HzMs}`,
    `gpui_input_latency_ms = ${CORE_WEB_VITALS_BUDGETS.gpuiInputLatencyMs}`,
    `wasm_critical_brotli_bytes = ${CORE_WEB_VITALS_BUDGETS.wasmCriticalBrotliBytes}`,
    "",
    "[monorepo_audit_summary]",
    `ts_files_scanned = ${audit.tsFilesScanned}`,
    `rust_files_scanned = ${audit.rustFilesScanned}`,
    `ts_compile_errors = ${audit.tsCompileErrors}`,
    `rust_mod_rs_violations = ${audit.rustModRsViolations}`,
    `rust_prod_unwrap_violations = ${audit.rustProdUnwrapViolations}`,
    `machine_path_leaks = ${audit.machinePathLeaks}`,
    "",
  ];

  for (const row of rows) {
    lines.push("[[frameworks]]");
    lines.push(`id = ${escapeTomlString(row.id)}`);
    lines.push(`label = ${escapeTomlString(row.label)}`);
    lines.push(`repo_or_submodule = ${escapeTomlString(row.repoOrSubmodule)}`);
    lines.push(`present_on_disk = ${row.presentOnDisk}`);
    lines.push(`manifest_present = ${row.manifestPresent}`);
    lines.push(`cargo_patched = ${row.cargoPatched}`);
    lines.push(`wasm32_unknown = ${row.wasm32Unknown}`);
    lines.push(`wasip2_aot = ${row.wasip2Aot}`);
    lines.push(`wgpu_v30_webgpu = ${row.wgpuV30Webgpu}`);
    lines.push(`taffy_layout = ${row.taffyLayout}`);
    lines.push(`accesskit_aria = ${row.accesskitAria}`);
    lines.push(`google_design_tailwind = ${row.googleDesignTailwind}`);
    lines.push(`shadcn_base_ui_starter = ${row.shadcnBaseUiStarter}`);
    lines.push(`ai_agent_mcp = ${row.aiAgentMcp}`);
    lines.push(`module_file_count = ${row.moduleFileCount}`);
    lines.push(`role = ${escapeTomlString(row.role)}`);
    lines.push("");
  }

  return `${lines.join("\n")}\n`;
}

export function renderArchitectureMarkdown(
  rows: FrameworkParityRow[],
  benches: BenchmarkMetric[],
  audit: MonorepoReadinessReport,
): string {
  const lines: string[] = [
    "# GPUI Starter Pack, Web Standards & Submodule Parity Matrix",
    "",
    "> Authoritative cross-stack specification unifying **Core Web Vitals**, **Lighthouse**, **W3C** (HTML5, WCAG 2.2 AA, WAI-ARIA, CharMod, BCP 47, DTCG), **Google Style Guides**, **Vercel Web Interface Guidelines**, and **shadcn/ui + Base UI + Tailwind v4** directly into Aphrody GPUI, NSH, and WebGPU.",
    "",
    "## Core Web Vitals & GPUI Frame Budgets",
    "",
    "| Metric | Web Budget (Good) | GPUI / WebGPU / Wasm Contract |",
    "| --- | --- | --- |",
    `| LCP / First Surface Paint | \`<= ${CORE_WEB_VITALS_BUDGETS.lcpGoodMs} ms\` | Non-blocking module bootstrap, zero sync \`document.write()\` or CSS \`@import\` |`,
    `| INP / Dispatch Latency | \`<= ${CORE_WEB_VITALS_BUDGETS.inpGoodMs} ms\` | GPUI action dispatch \`<= ${CORE_WEB_VITALS_BUDGETS.gpuiInputLatencyMs} ms\` |`,
    `| CLS / Layout Stability | \`<= ${CORE_WEB_VITALS_BUDGETS.clsGoodMax}\` | Explicit canvas/media sizing & Taffy constraints |`,
    `| GPU Frame Cadence | \`<= ${CORE_WEB_VITALS_BUDGETS.gpuFrame60HzMs} ms\` (60Hz) / \`<= ${CORE_WEB_VITALS_BUDGETS.gpuFrame120HzMs} ms\` (120Hz) | Shared WGSL Naga v30 pipelines + M3 Expressive damped springs |`,
    `| Critical Wasm Payload | \`<= ${CORE_WEB_VITALS_BUDGETS.wasmCriticalBrotliBytes} B\` Brotli | \`[profile.web-lto]\` + Binaryen \`wasm-opt -Oz\` cohorts |`,
    "",
    "## Fork Submodule Parity Matrix",
    "",
    "| Stack / Submodule | Patched | Wasm32 | WASI P2 | WGPU v30 | Taffy | AccessKit | M3 + Tailwind | shadcn/Base UI | MCP |",
    "| --- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |",
  ];

  const mark = (v: boolean) => (v ? "YES" : "-");
  for (const r of rows) {
    lines.push(
      `| ${r.label} | ${mark(r.cargoPatched)} | ${mark(r.wasm32Unknown)} | ${mark(r.wasip2Aot)} | ${mark(r.wgpuV30Webgpu)} | ${mark(r.taffyLayout)} | ${mark(r.accesskitAria)} | ${mark(r.googleDesignTailwind)} | ${mark(r.shadcnBaseUiStarter)} | ${mark(r.aiAgentMcp)} |`,
    );
  }

  lines.push(
    "",
    "## Deterministic Micro-Benchmarks (Bun Native)",
    "",
    "| Benchmark | Iterations | Throughput (ops/sec) | p50 (us) | p95 (us) |",
    "| --- | ---: | ---: | ---: | ---: |",
  );
  for (const b of benches) {
    lines.push(`| \`${b.id}\` | ${b.iterations} | ${b.opsPerSec} | ${b.p50Us} | ${b.p95Us} |`);
  }

  lines.push(
    "",
    "## Monorepo TS -> Rust AI-Agent Readiness Audit",
    "",
    `- TypeScript files checked via \`Bun.Transpiler\`: **${audit.tsFilesScanned}** (syntax errors: ${audit.tsCompileErrors}, explicit \`any\` warnings: ${audit.tsExplicitAnyWarnings})`,
    `- Pure-Rust core crate files checked: **${audit.rustFilesScanned}** (\`mod.rs\` violations: ${audit.rustModRsViolations}, production \`.unwrap()\`: ${audit.rustProdUnwrapViolations})`,
    `- Machine path leaks across TS + Rust: **${audit.machinePathLeaks}**`,
    "",
    "## Key Source Files",
    "",
    "- [Root `DESIGN.md`](../../../DESIGN.md)",
    "- [`assets/os/submodule_parity_matrix.toml`](../../../assets/os/submodule_parity_matrix.toml)",
    "- [`crates/aphrody_design`](../../../crates/aphrody_design/src/aphrody_design.rs)",
    "- [`crates/aphrody_ui`](../../../crates/aphrody_ui/src/aphrody_ui.rs)",
    "- [`crates/ui`](../../../crates/ui/src/ui.rs)",
    "- [`crates/nsh-base`](../../../crates/nsh-base/src/lib.rs)",
    "- [`crates/aphrody_cli`](../../../crates/aphrody_cli/src/main.rs)",
    "- [`tooling/bun/prod-standards-starterpack.ts`](../../../tooling/bun/prod-standards-starterpack.ts)",
  );

  return `${lines.join("\n")}\n`;
}

export function renderAgentLlmsTxt(rows: FrameworkParityRow[]): string {
  return [
    "# Aphrody AI-Agent Production & GPUI Starter Pack Reference",
    "",
    "> Canonical entrypoints for coding agents working in aphrody (TypeScript/Bun + Pure Rust GPUI + WebGPU + Wasm).",
    "",
    "## Standards & Design Contracts",
    "- Design source of truth: https://design.google/ (Material 3 Expressive + Google Sans Flex)",
    "- Workspace token spec: DESIGN.md + assets/os/google_design_tokens.toml",
    "- Web standards & Lighthouse auditor: crates/aphrody_design/src/web_standards.rs",
    "- Pure-Rust GPUI starter pack (shadcn / Base UI / Tailwind v4): crates/aphrody_ui/src/starter_pack.rs",
    "- NSH workspace studio actions: OpenGoogleDesignMirror, OpenStitchStudio, OpenGpuiStarterPack",
    "",
    "## Verification Commands",
    "- `bun run standards:audit` — submodule parity matrix + micro-bench + TS/Rust Google Style audit",
    "- `bun run design:audit` — https://design.google/ token & contrast verification",
    "- `bun run i18n:check` — W3C CharMod / LTLI / BCP 47 / String-Meta verification",
    "- `bun run web:check` — wasm32-unknown-unknown + wasm32-wasip2 full workspace verification",
    "- `gpui-wgpu design lighthouse templates/wasm-wgpu --json` — Core Web Vitals & Lighthouse check",
    "- `gpui-wgpu mcp` — 24-tool JSON-RPC MCP server for Wasm/WGPU/Google Design/Lighthouse/Starter Pack",
    "",
    `## Audited Stacks (${rows.length} entries)`,
    ...rows.map((r) => `- ${r.id}: ${r.label} -> ${r.repoOrSubmodule} (${r.moduleFileCount} source modules)`),
    "",
  ].join("\n");
}

if (import.meta.main) {
  const args = Bun.argv.slice(2);
  const cmd = args[0] ?? "all";
  const asJson = args.includes("--json");

  if (cmd === "matrix") {
    const matrix = buildParityMatrix(REPO_ROOT);
    console.log(asJson ? JSON.stringify({ frameworks: matrix }, null, 2) : `Parity rows: ${matrix.length}`);
  } else if (cmd === "bench") {
    const benches = runParityBenchmarks(REPO_ROOT);
    mkdirSync(resolve(REPO_ROOT, "target/wasm-wgpu-proof"), { recursive: true });
    writeFileSync(resolve(REPO_ROOT, BENCH_OUT_REL), JSON.stringify({ benchmarks: benches }, null, 2), "utf8");
    console.log(JSON.stringify({ benchmarks: benches }, null, 2));
  } else if (cmd === "audit") {
    const report = auditMonorepoReadiness(REPO_ROOT);
    if (asJson) {
      console.log(JSON.stringify(report, null, 2));
    } else {
      console.log(
        `AI-Agent Readiness Audit: ok=${report.ok} ts=${report.tsFilesScanned} (errors=${report.tsCompileErrors}) rust=${report.rustFilesScanned} (mod.rs=${report.rustModRsViolations}, prod_unwrap=${report.rustProdUnwrapViolations}) leaks=${report.machinePathLeaks}`,
      );
      for (const f of report.findings) {
        console.log(`  [${f.severity.toUpperCase()}] ${f.rule} ${f.file}:${f.line} — ${f.message}`);
      }
    }
    if (!report.ok) process.exit(1);
  } else if (cmd === "all") {
    const matrix = buildParityMatrix(REPO_ROOT);
    const benches = runParityBenchmarks(REPO_ROOT);
    const audit = auditMonorepoReadiness(REPO_ROOT);

    writeFileSync(resolve(REPO_ROOT, PARITY_MATRIX_TOML_REL), renderParityMatrixToml(matrix, audit), "utf8");
    writeFileSync(resolve(REPO_ROOT, PARITY_DOC_REL), renderArchitectureMarkdown(matrix, benches, audit), "utf8");
    writeFileSync(resolve(REPO_ROOT, AGENT_LLMS_REL), renderAgentLlmsTxt(matrix), "utf8");
    mkdirSync(resolve(REPO_ROOT, "target/wasm-wgpu-proof"), { recursive: true });
    writeFileSync(
      resolve(REPO_ROOT, BENCH_OUT_REL),
      JSON.stringify({ generatedAt: new Date().toISOString(), benchmarks: benches, audit }, null, 2),
      "utf8",
    );

    if (asJson) {
      console.log(JSON.stringify({ frameworks: matrix, benchmarks: benches, audit }, null, 2));
    } else {
      console.log(
        `Starter Pack & Parity Matrix synced: rows=${matrix.length} ts=${audit.tsFilesScanned} rust=${audit.rustFilesScanned} ok=${audit.ok}`,
      );
    }
    if (!audit.ok) process.exit(1);
  } else {
    throw new Error(`Unknown command '${cmd}'. Use matrix | bench | audit | all.`);
  }
}
