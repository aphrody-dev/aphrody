#!/usr/bin/env bun
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

export const AGENT_DESKTOP_REPO = "aphrody-labs/agent-desktop";
export const AGENT_DESKTOP_URL = `https://github.com/${AGENT_DESKTOP_REPO}/tree/main`;

export interface PurgeCategory {
  category: string;
  upstreamPatterns: RegExp[];
  purgedReason: string;
  aphrodyReplacement: string;
  count: number;
  bytes: number;
}

export interface AgentDesktopAuditReport {
  generatedAt: string;
  upstreamRepo: string;
  totalUpstreamBlobs: number;
  totalUpstreamBytes: number;
  purgedCategories: Omit<PurgeCategory, "upstreamPatterns">[];
  migratedFiles: {
    path: string;
    exists: boolean;
    loc: number;
    hasModRs: boolean;
    bannedBrandLeaks: string[];
  }[];
  zeroModRsInOsPilot: boolean;
  zeroModRsInBrowser: boolean;
  zeroBannedLeaks: boolean;
  mcpToolParityCount: number;
  requiredMcpToolsPresent: string[];
  missingMcpTools: string[];
  requiredFusedSymbolsPresent: string[];
  missingFusedSymbols: string[];
  ok: boolean;
}

export const REQUIRED_PILOT_TOOLS = [
  "desktop_status",
  "desktop_permissions",
  "desktop_list_apps",
  "desktop_list_windows",
  "desktop_list_displays",
  "desktop_list_surfaces",
  "desktop_snapshot",
  "desktop_find",
  "desktop_get",
  "desktop_is",
  "desktop_click",
  "desktop_double_click",
  "desktop_triple_click",
  "desktop_right_click",
  "desktop_type",
  "desktop_set_value",
  "desktop_clear",
  "desktop_focus",
  "desktop_select",
  "desktop_toggle",
  "desktop_check",
  "desktop_uncheck",
  "desktop_expand",
  "desktop_collapse",
  "desktop_scroll",
  "desktop_scroll_to",
  "desktop_press",
  "desktop_key_down",
  "desktop_key_up",
  "desktop_hover",
  "desktop_drag",
  "desktop_mouse_move",
  "desktop_mouse_click",
  "desktop_mouse_down",
  "desktop_mouse_up",
  "desktop_mouse_wheel",
  "desktop_launch",
  "desktop_close_app",
  "desktop_focus_window",
  "desktop_move_window",
  "desktop_resize_window",
  "desktop_minimize",
  "desktop_maximize",
  "desktop_restore",
  "desktop_open_system_surface",
  "desktop_clipboard_get",
  "desktop_clipboard_set",
  "desktop_clipboard_clear",
  "desktop_list_notifications",
  "desktop_dismiss_notification",
  "desktop_dismiss_all_notifications",
  "desktop_notification_action",
  "desktop_screenshot",
  "desktop_wait",
  "desktop_batch",
  "desktop_session",
  "desktop_trace",
  "desktop_act_verify",
  "desktop_ai_act_verify",
  "browser_status",
  "browser_validate_url",
  "browser_connect_cdp",
  "browser_open",
  "browser_snapshot",
  "browser_snapshot_diff",
  "browser_find",
  "browser_eval",
  "browser_screenshot",
  "browser_screenshot_annotated",
  "browser_route",
  "browser_unroute",
  "browser_har_start",
  "browser_har_stop",
  "browser_console",
  "browser_errors",
  "browser_state_save",
  "browser_state_load",
  "browser_cookies",
  "browser_storage",
  "browser_tab_list",
  "browser_tab_new",
  "browser_tab_switch",
  "browser_tab_close",
  "browser_policy_check",
  "browser_batch",
];

export const REQUIRED_FUSED_SYMBOLS: { file: string; symbols: string[] }[] = [
  {
    file: "crates/aphrody_pilot/src/model.rs",
    symbols: ["RetryDisposition", "SnapshotSurface", "PlatformNotSupported"],
  },
  {
    file: "crates/aphrody_pilot/src/action.rs",
    symbols: ["PostconditionVerdict", "evaluate_postcondition", "execute_verified_action"],
  },
  {
    file: "crates/aphrody_pilot/src/policy.rs",
    symbols: ["act_verify", "BatchExecutionState", "MAX_BATCH_OUTPUT_BYTES"],
  },
  {
    file: "crates/aphrody_pilot/src/browser.rs",
    symbols: [
      "SnapshotDiff",
      "SemanticLocator",
      "AnnotatedElementBox",
      "NetworkRouteTable",
      "HarRingBuffer",
      "BrowserEventRingBuffer",
      "BrowserStorageState",
      "TabSessionManager",
      "BrowserActionPolicy",
      "BrowserBatchReport",
    ],
  },
  {
    file: "crates/aphrody_browser/src/agent_browser_core.rs",
    symbols: [
      "AgentBrowserCore",
      "SnapshotDiff",
      "SemanticLocator",
      "AnnotatedElementBox",
      "HarRingBuffer",
      "BrowserEventRingBuffer",
      "BrowserStorageState",
      "TabSessionManager",
      "BrowserActionPolicy",
    ],
  },
  {
    file: "crates/aphrody_browser/src/chromium_gpui.rs",
    symbols: ["ChromiumBrowserView", "agent_core", "apply_omnibox_intent"],
  },
  {
    file: "crates/aphrody_desktop/src/pilot_hud.rs",
    symbols: ["render_pilot_status_pill", "append_pilot_drawer_section", "pilot_hud_summary"],
  },
];

const BANNED_LEGACY_LEAKS = ["vercel-labs", "agent-browser.dev", "jev-desktop"];

const DEFAULT_PURGE_CATEGORIES: Omit<PurgeCategory, "count" | "bytes">[] = [
  {
    category: "media-assets",
    upstreamPatterns: [/^assets\//],
    purgedReason: "Heavy demo MP4/GIF/PNG files unnecessary in Aphrody core monorepo",
    aphrodyReplacement: "Headless GPUI visual_smoke PNG checks + live desktop_screenshot",
  },
  {
    category: "nextjs-docs-site",
    upstreamPatterns: [/^apps\/docs\//],
    purgedReason: "Standalone Vercel/Next.js Geist web template",
    aphrodyReplacement: "docs/aphrody/architecture/agent-desktop-ai-os.md",
  },
  {
    category: "exploratory-probes",
    upstreamPatterns: [/^probes\//],
    purgedReason: "345 ad-hoc PowerShell/Python phase-discovery scripts",
    aphrodyReplacement: "crates/aphrody_pilot/src/backend_*.rs & platforms-matrix.md",
  },
  {
    category: "ffi-and-cross-lang-rigs",
    upstreamPatterns: [
      /^crates\/ffi\//,
      /^tests\/ffi-python\//,
      /^tests\/fixture-app/,
      /^tests\/native\//,
      /^tests\/real-apps\//,
      /^tests\/linux-dist\//,
      /^tests\/npm\//,
    ],
    purgedReason: "External C-ABI cdylib & C#/Swift/ObjC/Python harness bloat",
    aphrodyReplacement: "Pure Rust crates/aphrody_pilot + GPUI AccessKit unit tests",
  },
  {
    category: "npm-and-vm-wrappers",
    upstreamPatterns: [/^npm\//, /^providers\/macos\//, /^skills\/jev-desktop\//],
    purgedReason: "Standalone JS launcher, macOS VM scripts, and personal skill",
    aphrodyReplacement: "crates/aphrody_store (22 packagers) + fleet:verify + ai-os-pilot skill",
  },
  {
    category: "cursor-overlay-hack",
    upstreamPatterns: [/cursor_overlay/],
    purgedReason: "80+ native GDI/Cocoa/X11 cursor window overlay files",
    aphrodyReplacement: "crates/aphrody_desktop/src/pilot_hud.rs native GPUI HUD",
  },
];

export function categorizeUpstreamBlobs(
  blobs: { path: string; size: number }[],
): Omit<PurgeCategory, "upstreamPatterns">[] {
  const buckets: PurgeCategory[] = DEFAULT_PURGE_CATEGORIES.map((c) => ({
    ...c,
    count: 0,
    bytes: 0,
  }));
  for (const blob of blobs) {
    for (const bucket of buckets) {
      if (bucket.upstreamPatterns.some((rx) => rx.test(blob.path))) {
        bucket.count += 1;
        bucket.bytes += blob.size;
        break;
      }
    }
  }
  return buckets.map(({ upstreamPatterns: _rx, ...rest }) => rest);
}

export function scanMigratedTree(root: string): AgentDesktopAuditReport["migratedFiles"] {
  const targets = [
    "crates/aphrody_pilot/Cargo.toml",
    "crates/aphrody_pilot/src/aphrody_pilot.rs",
    "crates/aphrody_pilot/src/model.rs",
    "crates/aphrody_pilot/src/refs.rs",
    "crates/aphrody_pilot/src/locator.rs",
    "crates/aphrody_pilot/src/action.rs",
    "crates/aphrody_pilot/src/policy.rs",
    "crates/aphrody_pilot/src/adapter.rs",
    "crates/aphrody_pilot/src/browser.rs",
    "crates/aphrody_pilot/src/gpui_bridge.rs",
    "crates/aphrody_pilot/src/backend_windows.rs",
    "crates/aphrody_pilot/src/backend_linux.rs",
    "crates/aphrody_pilot/src/backend_macos.rs",
    "crates/aphrody_browser/src/aphrody_browser.rs",
    "crates/aphrody_browser/src/agent_browser_core.rs",
    "crates/aphrody_browser/src/chromium_gpui.rs",
    "crates/aphrody_desktop/src/pilot_hud.rs",
    "crates/aphrody_desktop/src/main.rs",
    ".agents/plugins/aphrody/skills/ai-os-pilot/SKILL.md",
    "docs/aphrody/architecture/agent-desktop-ai-os.md",
  ];
  return targets.map((rel) => {
    const abs = join(root, rel);
    if (!existsSync(abs)) {
      return {
        path: rel,
        exists: false,
        loc: 0,
        hasModRs: rel.endsWith("/mod.rs"),
        bannedBrandLeaks: [],
      };
    }
    const content = readFileSync(abs, "utf8");
    const leaks = BANNED_LEGACY_LEAKS.filter((token) =>
      content.toLowerCase().includes(token),
    );
    return {
      path: rel,
      exists: true,
      loc: content.split(/\r?\n/).length,
      hasModRs: rel.endsWith("/mod.rs"),
      bannedBrandLeaks: leaks,
    };
  });
}

export function findModRsRecursive(dir: string): string[] {
  if (!existsSync(dir)) return [];
  const found: string[] = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      found.push(...findModRsRecursive(full));
    } else if (entry.name === "mod.rs") {
      found.push(full);
    }
  }
  return found;
}

export function checkToolParity(osPilotRootSrc: string): {
  present: string[];
  missing: string[];
} {
  if (!existsSync(osPilotRootSrc)) {
    return { present: [], missing: [...REQUIRED_PILOT_TOOLS] };
  }
  const src = readFileSync(osPilotRootSrc, "utf8");
  const present: string[] = [];
  const missing: string[] = [];
  for (const tool of REQUIRED_PILOT_TOOLS) {
    if (src.includes(`"${tool}"`)) present.push(tool);
    else missing.push(tool);
  }
  return { present, missing };
}

export function fetchUpstreamTreeSync(): { path: string; size: number }[] {
  try {
    const out = Bun.spawnSync({
      cmd: [
        "gh",
        "api",
        `repos/${AGENT_DESKTOP_REPO}/git/trees/main?recursive=1`,
        "--jq",
        '.tree[] | select(.type=="blob") | "\\(.size // 0)\\t\\(.path)"',
      ],
      stdout: "pipe",
      stderr: "pipe",
    });
    if (out.exitCode === 0) {
      return out.stdout
        .toString("utf8")
        .split(/\r?\n/)
        .filter((l) => /^\d+\t/.test(l))
        .map((l) => {
          const idx = l.indexOf("\t");
          return { size: Number(l.slice(0, idx)), path: l.slice(idx + 1) };
        });
    }
  } catch {
    // Offline fallback handled below.
  }
  return [];
}

export function checkFusedSymbols(root: string): {
  present: string[];
  missing: string[];
} {
  const present: string[] = [];
  const missing: string[] = [];
  for (const group of REQUIRED_FUSED_SYMBOLS) {
    const abs = join(root, group.file);
    const content = existsSync(abs) ? readFileSync(abs, "utf8") : "";
    for (const sym of group.symbols) {
      const qualified = `${group.file}::${sym}`;
      if (content.includes(sym)) present.push(qualified);
      else missing.push(qualified);
    }
  }
  return { present, missing };
}

export function runAudit(root: string, offline = false): AgentDesktopAuditReport {
  const blobs = offline ? [] : fetchUpstreamTreeSync();
  const totalBytes = blobs.reduce((acc, b) => acc + b.size, 0);
  const purgedCategories = categorizeUpstreamBlobs(blobs);
  const migratedFiles = scanMigratedTree(root);
  const modRsFiles = findModRsRecursive(join(root, "crates/aphrody_pilot/src"));
  const browserModRsFiles = findModRsRecursive(join(root, "crates/aphrody_browser/src"));
  const { present, missing } = checkToolParity(join(root, "crates/aphrody_pilot/src/aphrody_pilot.rs"));
  const fused = checkFusedSymbols(root);
  const zeroModRsInOsPilot = modRsFiles.length === 0;
  const zeroModRsInBrowser = browserModRsFiles.length === 0;
  const zeroBannedLeaks = migratedFiles.every((f) => f.bannedBrandLeaks.length === 0);
  const allMigratedExist = migratedFiles.every((f) => f.exists);
  const ok =
    zeroModRsInOsPilot &&
    zeroModRsInBrowser &&
    zeroBannedLeaks &&
    allMigratedExist &&
    missing.length === 0 &&
    fused.missing.length === 0;

  return {
    generatedAt: new Date().toISOString(),
    upstreamRepo: AGENT_DESKTOP_REPO,
    totalUpstreamBlobs: blobs.length,
    totalUpstreamBytes: totalBytes,
    purgedCategories,
    migratedFiles,
    zeroModRsInOsPilot,
    zeroModRsInBrowser,
    zeroBannedLeaks,
    mcpToolParityCount: present.length,
    requiredMcpToolsPresent: present,
    missingMcpTools: missing,
    requiredFusedSymbolsPresent: fused.present,
    missingFusedSymbols: fused.missing,
    ok,
  };
}

if (import.meta.main) {
  const root = resolve(import.meta.dir, "../..");
  const offline = Bun.argv.includes("--offline");
  const report = runAudit(root, offline);
  const outDir = join(root, "target/agent-desktop-audit");
  mkdirSync(outDir, { recursive: true });
  writeFileSync(join(outDir, "audit.json"), JSON.stringify(report, null, 2));
  console.log(JSON.stringify(report, null, 2));
  if (!report.ok) {
    process.exit(1);
  }
}
