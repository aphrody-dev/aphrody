#!/usr/bin/env bun
/**
 * Unified, reproducible Bun CLI router matching Zed upstream's `cargo xtask` structure.
 * Consolidates all `tooling/bun/*.ts` workflows under 7 canonical domains (`workspace`,
 * `design`, `storytold`, `web`, `session`, `ingest`, `fleet`) + `list` and `check`.
 *
 * Usage:
 *   bun tooling/bun/cli.ts list [--json]
 *   bun tooling/bun/cli.ts check [--json]
 *   bun tooling/bun/cli.ts <domain> <action> [...args]
 */
import { existsSync } from "node:fs";
import { join, resolve } from "node:path";

export const repoRoot = resolve(import.meta.dir, "../..");

export interface ToolCommandSpec {
  domain: "workspace" | "design" | "storytold" | "web" | "session" | "ingest" | "fleet";
  action: string;
  scriptRel: string;
  defaultArgs: string[];
  summary: string;
}

export const TOOLING_REGISTRY: readonly ToolCommandSpec[] = [
  // 1. Workspace Hygiene, Submodules, Conformity & Standards
  { domain: "workspace", action: "hygiene", scriptRel: "tooling/bun/repo-hygiene.ts", defaultArgs: [], summary: "Verify README guard, zero ephemeral path leaks, and doc link integrity" },
  { domain: "workspace", action: "submodules", scriptRel: "tooling/bun/submodules.ts", defaultArgs: ["check"], summary: "Inspect or sync git submodules from .gitmodules" },
  { domain: "workspace", action: "vendor-doctor", scriptRel: "tooling/bun/vendor-lan-doctor.ts", defaultArgs: ["audit"], summary: "Audit Cargo patches, deny.toml licenses/sources, and WireGuard LAN" },
  { domain: "workspace", action: "i18n", scriptRel: "tooling/bun/w3c-i18n.ts", defaultArgs: ["check"], summary: "Run W3C Internationalization (BCP 47, UTF-8, Bidi, HTML/SVG) checks" },
  { domain: "workspace", action: "standards", scriptRel: "tooling/bun/prod-standards-starterpack.ts", defaultArgs: ["all"], summary: "Verify Core Web Vitals, Submodule Parity Matrix, and TS/Rust hygiene" },
  { domain: "workspace", action: "agy-audit", scriptRel: "tooling/bun/agy-config-audit.ts", defaultArgs: [], summary: "Validate Antigravity settings, MCP schemas, hooks, and plugins" },
  { domain: "workspace", action: "shell-doctor", scriptRel: "tooling/bun/shell-path-doctor.ts", defaultArgs: ["audit"], summary: "Audit modern Windows/Linux CLI PATH tiering" },
  { domain: "workspace", action: "worktree", scriptRel: "tooling/bun/worktree.ts", defaultArgs: ["list"], summary: "Manage isolated out-of-tree Git worktrees" },
  { domain: "workspace", action: "agent-desktop", scriptRel: "tooling/bun/agent-desktop-audit.ts", defaultArgs: [], summary: "Verify aphrody_pilot zero-mod.rs & 58-tool MCP parity" },
  { domain: "workspace", action: "gpui-vs-dioxus", scriptRel: "tooling/bun/gpui-vs-dioxus-audit.ts", defaultArgs: ["all"], summary: "Audit 9-submodule GPUI/Wgpu/Taffy/AccessKit parity matrix" },
  { domain: "workspace", action: "crate-skill", scriptRel: "tooling/bun/crate-to-skill.ts", defaultArgs: ["--help"], summary: "Generate portable Agent Skill bundle from a workspace crate" },

  // 2. Google Design System & Stitch Studio (`https://design.google/`)
  { domain: "design", action: "sync", scriptRel: "tooling/bun/google-design-sync.ts", defaultArgs: ["sync"], summary: "Synchronize 3-tier Material 3 Expressive tokens, sitemap, and root DESIGN.md" },
  { domain: "design", action: "audit", scriptRel: "tooling/bun/google-design-sync.ts", defaultArgs: ["audit"], summary: "Verify WCAG 2.2 AA contrast ratios and Google Design token integrity" },
  { domain: "design", action: "pack", scriptRel: "tooling/bun/design-docs-pack.ts", defaultArgs: ["build"], summary: "Build & validate complete docs/design SVGs, CSS, Figma DTCG, Stitch & MDX pack" },

  // 3. Storytold 82-Repository Ecosystem (`craft-libs` + `craft-anything` + `egui` + `bevy`)
  { domain: "storytold", action: "sync", scriptRel: "tooling/bun/storytold-unify.ts", defaultArgs: ["sync"], summary: "Generate assets/os/storytold_ecosystem.toml across all 82 repositories" },
  { domain: "storytold", action: "audit", scriptRel: "tooling/bun/storytold-unify.ts", defaultArgs: ["audit"], summary: "Verify 82-repo mapping and root Cargo.toml dependency deduplication" },

  // 4. WebAssembly, WebGPU & Multi-Target Builds
  { domain: "web", action: "matrix", scriptRel: "tooling/bun/web-build.ts", defaultArgs: ["matrix"], summary: "Print Wasm32 & WebGPU framework cohort matrix" },
  { domain: "web", action: "check", scriptRel: "tooling/bun/web-build.ts", defaultArgs: ["check"], summary: "Type-check wasm32-unknown-unknown & wasm32-wasip2 targets" },
  { domain: "web", action: "build", scriptRel: "tooling/bun/web-build.ts", defaultArgs: ["build", "--package", "aphrody_desktop", "--opt"], summary: "Build optimized Wasm32 + WebGPU bundle" },
  { domain: "web", action: "verify", scriptRel: "tooling/bun/web-verify.ts", defaultArgs: ["--package", "aphrody_desktop"], summary: "Run headless Chrome CDP DOM/ARIA/WebGPU visual verification" },
  { domain: "web", action: "cross", scriptRel: "tooling/bun/cross-build.ts", defaultArgs: ["doctor"], summary: "Inspect cargo-xwin, cargo-zigbuild, LLVM, and cross toolchains" },
  { domain: "web", action: "issues", scriptRel: "tooling/bun/zed-wasm-wgpu-issues.ts", defaultArgs: ["audit"], summary: "Verify upstream Zed Wasm/WGPU issue parity fixes" },

  // 5. NSH AGY / Gemini Session & Parity Studio
  { domain: "session", action: "serve", scriptRel: "tooling/bun/agy-session-studio.ts", defaultArgs: ["serve"], summary: "Start live SSE auto-reload AGY/Gemini & Storytold Studio in Chrome" },
  { domain: "session", action: "snapshot", scriptRel: "tooling/bun/agy-session-studio.ts", defaultArgs: ["snapshot"], summary: "Emit structured JSON snapshot of tracked sessions & parity state" },
  { domain: "session", action: "export", scriptRel: "tooling/bun/agy-session-studio.ts", defaultArgs: ["export"], summary: "Export single-file gzip-compressed offline HTML report" },
  { domain: "session", action: "verify", scriptRel: "tooling/bun/agy-session-studio.ts", defaultArgs: ["verify"], summary: "Capture headless Chrome Light/Dark/Storytold/Parity visual proofs" },

  // 6. Upstream Ecosystem Ingestion & Store Packaging
  { domain: "ingest", action: "store", scriptRel: "tooling/bun/awesome-gpui-store.ts", defaultArgs: ["sync"], summary: "Sync awesome-gpui catalog and 22-target store manifests" },
  { domain: "ingest", action: "longbridge", scriptRel: "tooling/bun/longbridge-ingest.ts", defaultArgs: ["scan"], summary: "Ingest longbridge & community GPUI dependency graph" },
  { domain: "ingest", action: "wasm-bindgen", scriptRel: "tooling/bun/wasm-bindgen-ingest.ts", defaultArgs: ["all"], summary: "Ingest wasm-bindgen org digests, llms.txt, and skill" },
  { domain: "ingest", action: "emscripten", scriptRel: "tooling/bun/emscripten-crawl.ts", defaultArgs: ["all"], summary: "Crawl Emscripten docs & 12 repos into llms.txt and skill" },
  { domain: "ingest", action: "webassembly", scriptRel: "tooling/bun/webassembly-org-ingest.ts", defaultArgs: ["all"], summary: "Ingest top 100 WebAssembly org repositories" },

  // 7. Remote Build Fleet (`fr1`) & Rust Toolchain Optimization
  { domain: "fleet", action: "sync", scriptRel: "tooling/bun/fleet-sync.ts", defaultArgs: ["sync", "--host", "fr1"], summary: "Delta-sync workspace to remote Linux builder fr1" },
  { domain: "fleet", action: "verify", scriptRel: "tooling/bun/fleet-sync.ts", defaultArgs: ["verify", "--host", "fr1"], summary: "Run remote Linux verification on fr1" },
  { domain: "fleet", action: "host-audit", scriptRel: "tooling/bun/ci-host-optimize.ts", defaultArgs: ["audit", "--host", "fr1"], summary: "Audit Ubuntu/Kernel/XFS/sccache/build.slice on fr1" },
  { domain: "fleet", action: "toolchain", scriptRel: "tooling/bun/toolchain-audit.ts", defaultArgs: ["doctor"], summary: "Audit Rust profiles, linkers, sccache, and Cargo.lock duplicates" },
];

export interface CheckStepResult {
  id: string;
  ok: boolean;
  exitCode: number;
  durationMs: number;
}

export function verifyRegistryFilesOnDisk(root = repoRoot): { ok: boolean; missing: string[] } {
  const missing: string[] = [];
  for (const spec of TOOLING_REGISTRY) {
    if (!existsSync(join(root, spec.scriptRel))) {
      missing.push(`${spec.domain}:${spec.action} -> ${spec.scriptRel}`);
    }
  }
  return { ok: missing.length === 0, missing };
}

export function runConsolidatedChecks(root = repoRoot): { ok: boolean; steps: CheckStepResult[] } {
  const checks: Array<{ id: string; args: string[] }> = [
    { id: "workspace:hygiene", args: ["tooling/bun/repo-hygiene.ts"] },
    { id: "design:audit", args: ["tooling/bun/google-design-sync.ts", "audit"] },
    { id: "design:pack-verify", args: ["tooling/bun/design-docs-pack.ts", "build"] },
    { id: "storytold:audit", args: ["tooling/bun/storytold-unify.ts", "audit"] },
    { id: "workspace:agy-audit", args: ["tooling/bun/agy-config-audit.ts"] },
    { id: "workspace:i18n", args: ["tooling/bun/w3c-i18n.ts", "check"] },
  ];

  const steps: CheckStepResult[] = [];
  for (const c of checks) {
    const t0 = performance.now();
    const proc = Bun.spawnSync([process.execPath, ...c.args], {
      cwd: root,
      stdout: "pipe",
      stderr: "pipe",
    });
    steps.push({
      id: c.id,
      ok: proc.exitCode === 0,
      exitCode: proc.exitCode,
      durationMs: Math.round(performance.now() - t0),
    });
  }
  return {
    ok: steps.every((s) => s.ok),
    steps,
  };
}

if (import.meta.main) {
  const args = process.argv.slice(2);
  const first = args[0] ?? "list";
  const wantJson = args.includes("--json");

  if (first === "list" || first === "--help" || first === "-h") {
    const disk = verifyRegistryFilesOnDisk();
    if (wantJson) {
      console.log(JSON.stringify({ ok: disk.ok, count: TOOLING_REGISTRY.length, commands: TOOLING_REGISTRY }, null, 2));
    } else {
      console.log(`Aphrody Consolidated Tooling CLI (${TOOLING_REGISTRY.length} commands across 7 domains)\n`);
      for (const item of TOOLING_REGISTRY) {
        console.log(`  bun tooling/bun/cli.ts ${item.domain.padEnd(10)} ${item.action.padEnd(14)} — ${item.summary}`);
      }
    }
    process.exit(disk.ok ? 0 : 1);
  }

  if (first === "check") {
    const disk = verifyRegistryFilesOnDisk();
    const res = runConsolidatedChecks();
    const ok = disk.ok && res.ok;
    if (wantJson) {
      console.log(JSON.stringify({ ok, registryOk: disk.ok, steps: res.steps }, null, 2));
    } else {
      for (const s of res.steps) {
        console.log(`[${s.ok ? "PASS" : "FAIL"}] ${s.id.padEnd(24)} (${s.durationMs} ms)`);
      }
    }
    process.exit(ok ? 0 : 1);
  }

  const domain = first;
  const action = args[1];
  const spec = TOOLING_REGISTRY.find((s) => s.domain === domain && s.action === action);
  if (!spec) {
    console.error(`Unknown command '${domain} ${action ?? ""}'. Run 'bun tooling/bun/cli.ts list'.`);
    process.exit(2);
  }

  const extraArgs = args.slice(2);
  const finalArgs = extraArgs.length > 0 ? extraArgs : spec.defaultArgs;
  const child = Bun.spawnSync([process.execPath, spec.scriptRel, ...finalArgs], {
    cwd: repoRoot,
    stdio: ["inherit", "inherit", "inherit"],
  });
  process.exit(child.exitCode);
}
