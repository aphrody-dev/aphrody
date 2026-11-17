#!/usr/bin/env bun
/**
 * Modern Shell (Nushell + PowerShell 7.7 + aysh), November 2026 Rust/Wasm CLI,
 * and Global PATH Doctor & Optimizer for Aphrody.
 *
 * Subcommands:
 *   bun tooling/bun/shell-path-doctor.ts audit [--json]
 *   bun tooling/bun/shell-path-doctor.ts optimize [--dry-run] [--configure-shells] [--json]
 */
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join, resolve } from "node:path";

export const repoRoot = resolve(import.meta.dir, "../..");

export interface ToolProbeSpec {
  id: string;
  bin: string;
  category: "shell" | "wasm" | "gpu" | "rust-cli" | "build";
  versionArgs?: string[];
  required?: boolean;
}

export interface ToolProbeResult {
  id: string;
  bin: string;
  category: ToolProbeSpec["category"];
  required: boolean;
  found: boolean;
  path: string | null;
  allMatches: string[];
  version: string;
}

export interface PathConflict {
  kind: "dead" | "duplicate" | "shadow" | "missing-core";
  entry: string;
  detail: string;
}

export interface ShellDoctorReport {
  schemaVersion: number;
  platform: string;
  generatedAt: string;
  userPathCount: number;
  machinePathCount: number;
  deadEntries: string[];
  duplicateEntries: string[];
  userDuplicateEntries: string[];
  machineDuplicateEntries: string[];
  conflicts: PathConflict[];
  tools: ToolProbeResult[];
  missingRequiredTools: string[];
  ok: boolean;
}

export interface OptimizeResult {
  dryRun: boolean;
  backupPath: string | null;
  beforeCount: number;
  afterCount: number;
  removedEntries: string[];
  addedEntries: string[];
  reorderedToFront: string[];
  configuredFiles: string[];
  audit: ShellDoctorReport;
}

export const MODERN_CLI_MATRIX: ToolProbeSpec[] = [
  { id: "nu", bin: "nu", category: "shell", versionArgs: ["--version"], required: true },
  { id: "pwsh", bin: "pwsh", category: "shell", versionArgs: ["--version"], required: true },
  { id: "bun", bin: "bun", category: "build", versionArgs: ["--version"], required: true },
  { id: "cargo", bin: "cargo", category: "build", versionArgs: ["--version"], required: true },
  { id: "rustc", bin: "rustc", category: "build", versionArgs: ["--version"], required: true },
  { id: "mise", bin: "mise", category: "build", versionArgs: ["--version"], required: true },
  { id: "wasmtime", bin: "wasmtime", category: "wasm", versionArgs: ["--version"], required: true },
  { id: "wasm-opt", bin: "wasm-opt", category: "wasm", versionArgs: ["--version"], required: true },
  { id: "wasm-bindgen", bin: "wasm-bindgen", category: "wasm", versionArgs: ["--version"], required: true },
  { id: "wasm-tools", bin: "wasm-tools", category: "wasm", versionArgs: ["--version"], required: true },
  { id: "wat2wasm", bin: "wat2wasm", category: "wasm", versionArgs: ["--version"], required: true },
  { id: "wasm2wat", bin: "wasm2wat", category: "wasm", versionArgs: ["--version"], required: true },
  { id: "wit-bindgen", bin: "wit-bindgen", category: "wasm", versionArgs: ["--version"], required: true },
  { id: "wac", bin: "wac", category: "wasm", versionArgs: ["--version"], required: true },
  { id: "dxc", bin: "dxc", category: "gpu", versionArgs: ["--version"], required: true },
  { id: "glslangValidator", bin: "glslangValidator", category: "gpu", versionArgs: ["--version"], required: true },
  { id: "spirv-val", bin: "spirv-val", category: "gpu", versionArgs: ["--version"], required: true },
  { id: "rg", bin: "rg", category: "rust-cli", versionArgs: ["--version"], required: true },
  { id: "fd", bin: "fd", category: "rust-cli", versionArgs: ["--version"], required: true },
  { id: "eza", bin: "eza", category: "rust-cli", versionArgs: ["--version"], required: true },
  { id: "bat", bin: "bat", category: "rust-cli", versionArgs: ["--version"], required: true },
  { id: "zoxide", bin: "zoxide", category: "rust-cli", versionArgs: ["--version"], required: true },
  { id: "starship", bin: "starship", category: "rust-cli", versionArgs: ["--version"], required: true },
  { id: "atuin", bin: "atuin", category: "rust-cli", versionArgs: ["--version"], required: true },
  { id: "ast-grep", bin: "ast-grep", category: "rust-cli", versionArgs: ["--version"], required: true },
  { id: "jj", bin: "jj", category: "rust-cli", versionArgs: ["--version"], required: true },
  { id: "hyperfine", bin: "hyperfine", category: "rust-cli", versionArgs: ["--version"], required: true },
  { id: "procs", bin: "procs", category: "rust-cli", versionArgs: ["--version"], required: false },
  { id: "choose", bin: "choose", category: "rust-cli", versionArgs: ["--version"], required: false },
  { id: "xh", bin: "xh", category: "rust-cli", versionArgs: ["--version"], required: true },
  { id: "monolith", bin: "monolith", category: "rust-cli", versionArgs: ["--version"], required: true },
  { id: "cargo-hakari", bin: "cargo-hakari", category: "rust-cli", versionArgs: ["--version"], required: false },
  { id: "cargo-nextest", bin: "cargo-nextest", category: "rust-cli", versionArgs: ["--version"], required: false },
  { id: "cargo-deny", bin: "cargo-deny", category: "rust-cli", versionArgs: ["--version"], required: false },
  { id: "cargo-machete", bin: "cargo-machete", category: "rust-cli", versionArgs: ["--version"], required: false },
  { id: "cargo-shear", bin: "cargo-shear", category: "rust-cli", versionArgs: ["--version"], required: false },
  { id: "sccache", bin: "sccache", category: "build", versionArgs: ["--version"], required: false },
];

export function normalizePathEntry(raw: string): string {
  return raw.trim().replace(/^"(.*)"$/, "$1").replace(/[\\/]+$/, "");
}

export function pathKey(raw: string, isWindows = process.platform === "win32"): string {
  const norm = normalizePathEntry(raw).replaceAll("/", "\\");
  return isWindows ? norm.toLowerCase() : normalizePathEntry(raw);
}

export function splitPathList(raw: string): string[] {
  const sep = process.platform === "win32" ? ";" : ":";
  return raw
    .split(sep)
    .map((s) => s.trim())
    .filter(Boolean);
}

export function analyzePathEntries(
  userEntries: string[],
  machineEntries: string[],
  checkExists: (p: string) => boolean = existsSync,
  isWindows = process.platform === "win32",
): {
  deadEntries: string[];
  duplicateEntries: string[];
  userDuplicateEntries: string[];
  machineDuplicateEntries: string[];
  conflicts: PathConflict[];
} {
  const seen = new Map<string, string>();
  const deadEntries: string[] = [];
  const duplicateEntries: string[] = [];
  const userDuplicateEntries: string[] = [];
  const machineDuplicateEntries: string[] = [];
  const conflicts: PathConflict[] = [];

  for (const [scope, list] of [
    ["user", userEntries],
    ["machine", machineEntries],
  ] as const) {
    for (const raw of list) {
      const clean = normalizePathEntry(raw);
      if (!clean) continue;
      const key = pathKey(clean, isWindows);
      if (seen.has(key)) {
        duplicateEntries.push(clean);
        if (scope === "user") {
          userDuplicateEntries.push(clean);
        } else {
          machineDuplicateEntries.push(clean);
        }
        conflicts.push({
          kind: "duplicate",
          entry: clean,
          detail: `[${scope}] Duplicates earlier PATH entry '${seen.get(key)}'`,
        });
      } else {
        seen.set(key, clean);
      }
      if (!checkExists(clean)) {
        deadEntries.push(clean);
        conflicts.push({
          kind: "dead",
          entry: clean,
          detail: `[${scope}] Directory does not exist on disk`,
        });
      }
    }
  }

  // Check ordering conflicts where legacy Unix shims or older CMake precede native Rust/modern SDK bins.
  const combined = [...userEntries, ...machineEntries].map((e) => normalizePathEntry(e));
  const idxOf = (pattern: RegExp) => combined.findIndex((p) => pattern.test(p));

  const cargoIdx = idxOf(/[/\\]\.cargo[/\\]bin$/i);
  const niubashIdx = idxOf(/niubash[/\\]winuxcmd/i);
  if (niubashIdx >= 0 && (cargoIdx < 0 || niubashIdx < cargoIdx)) {
    conflicts.push({
      kind: "shadow",
      entry: combined[niubashIdx],
      detail: "Niubash Unix shim precedes ~/.cargo/bin and shadows native Rust CLIs",
    });
  }

  const rosbeCmakeIdx = idxOf(/rosbe[/\\].*cmake/i);
  const modernCmakeIdx = idxOf(/[/\\]cmake-4\./i);
  if (rosbeCmakeIdx >= 0 && modernCmakeIdx >= 0 && rosbeCmakeIdx < modernCmakeIdx) {
    conflicts.push({
      kind: "shadow",
      entry: combined[rosbeCmakeIdx],
      detail: `Older CMake '${combined[rosbeCmakeIdx]}' precedes '${combined[modernCmakeIdx]}'`,
    });
  }

  return { deadEntries, duplicateEntries, userDuplicateEntries, machineDuplicateEntries, conflicts };
}

export function classifyPriorityTier(entry: string, homeDir: string): number {
  const norm = normalizePathEntry(entry).replaceAll("/", "\\").toLowerCase();
  const homeNorm = normalizePathEntry(homeDir).replaceAll("/", "\\").toLowerCase();

  if (
    norm === `${homeNorm}\\.bun\\bin` ||
    norm === `${homeNorm}\\.cargo\\bin` ||
    norm === `${homeNorm}\\.local\\bin` ||
    norm.endsWith("\\programs\\nu\\bin") ||
    norm.endsWith("\\wasmtime\\bin") ||
    norm.endsWith("\\microsoft\\winget\\links") ||
    norm.includes("\\tools\\cmake-4.") ||
    norm === `${homeNorm}\\.deno\\bin` ||
    norm === `${homeNorm}\\.dotnet\\tools` ||
    norm.endsWith("\\agy\\bin") ||
    norm.endsWith("\\antigravity ide\\bin") ||
    norm.endsWith("\\bin") && norm.includes("hermes")
  ) {
    return 0;
  }

  // Fallback shims & legacy Unix compatibility layers last so native tools resolve cleanly.
  if (
    norm.includes("\\niubash") ||
    norm.endsWith("\\microsoft\\windowsapps") ||
    norm.includes("\\rosbe\\") ||
    norm.includes("copilot-shim")
  ) {
    return 3;
  }

  // Keep redundant non-shared FFmpeg build directory dropped/demoted if shared dir exists.
  if (norm.includes("ffmpeg") && !norm.includes("shared")) {
    return 2;
  }

  return 1;
}

export function optimizeUserPathEntries(
  userEntries: string[],
  machineEntries: string[],
  homeDir = homedir(),
  localAppData = process.env.LOCALAPPDATA ?? join(homeDir, "AppData", "Local"),
  programFiles = process.env.ProgramFiles ?? "C:\\Program Files",
  checkExists: (p: string) => boolean = existsSync,
): {
  optimized: string[];
  removed: string[];
  added: string[];
  reorderedToFront: string[];
} {
  const machineKeys = new Set(machineEntries.map((e) => pathKey(e)));
  const candidatesToEnsure = [
    join(homeDir, ".bun", "bin"),
    join(homeDir, ".cargo", "bin"),
    join(homeDir, ".local", "bin"),
    join(localAppData, "Programs", "nu", "bin"),
    join(programFiles, "Wasmtime", "bin"),
    join(localAppData, "Microsoft", "WinGet", "Links"),
  ].filter((p) => checkExists(p) && !machineKeys.has(pathKey(p)));

  const rawSeen = new Set<string>();
  const kept: string[] = [];
  const removed: string[] = [];
  const added: string[] = [];

  for (const raw of userEntries) {
    const clean = normalizePathEntry(raw);
    if (!clean) continue;
    const key = pathKey(clean);
    if (!checkExists(clean)) {
      removed.push(`${clean} (missing)`);
      continue;
    }
    if (machineKeys.has(key)) {
      removed.push(`${clean} (already in Machine PATH)`);
      continue;
    }
    if (rawSeen.has(key)) {
      removed.push(`${clean} (duplicate)`);
      continue;
    }
    // Drop redundant non-shared FFmpeg directory when shared build exists.
    if (/ffmpeg-[^\\/]+-full_build\\bin$/i.test(clean)) {
      const hasShared = userEntries.some((other) => /ffmpeg-[^\\/]+-full_build-shared\\bin$/i.test(other) && checkExists(normalizePathEntry(other)));
      if (hasShared) {
        removed.push(`${clean} (redundant FFmpeg build; using shared bin)`);
        continue;
      }
    }
    rawSeen.add(key);
    kept.push(clean);
  }

  for (const ensureDir of candidatesToEnsure) {
    const key = pathKey(ensureDir);
    if (!rawSeen.has(key)) {
      rawSeen.add(key);
      kept.push(ensureDir);
      added.push(ensureDir);
    }
  }

  const tierZeroPriority = [
    pathKey(join(homeDir, ".bun", "bin")),
    pathKey(join(homeDir, ".cargo", "bin")),
    pathKey(join(homeDir, ".local", "bin")),
    pathKey(join(localAppData, "Programs", "nu", "bin")),
    pathKey(join(programFiles, "Wasmtime", "bin")),
    pathKey(join(localAppData, "Microsoft", "WinGet", "Links")),
  ];

  const indexed = kept.map((entry, idx) => ({
    entry,
    idx,
    tier: classifyPriorityTier(entry, homeDir),
    subRank: tierZeroPriority.indexOf(pathKey(entry)),
  }));

  indexed.sort((a, b) => {
    if (a.tier !== b.tier) return a.tier - b.tier;
    if (a.tier === 0) {
      const ra = a.subRank >= 0 ? a.subRank : 99;
      const rb = b.subRank >= 0 ? b.subRank : 99;
      if (ra !== rb) return ra - rb;
    }
    return a.idx - b.idx;
  });

  const optimized = indexed.map((x) => x.entry);
  const reorderedToFront = indexed.filter((x) => x.tier === 0).map((x) => x.entry);

  return { optimized, removed, added, reorderedToFront };
}

function readWindowsRegistryPath(scope: "User" | "Machine"): string[] {
  const res = Bun.spawnSync(
    ["pwsh", "-NoLogo", "-NoProfile", "-Command", `[Environment]::GetEnvironmentVariable('Path', '${scope}')`],
    { stdout: "pipe", stderr: "pipe" },
  );
  if (res.exitCode !== 0) return [];
  return splitPathList(new TextDecoder().decode(res.stdout));
}

function writeWindowsUserPath(entries: string[]): void {
  const joined = entries.join(";");
  const b64 = Buffer.from(joined, "utf8").toString("base64");
  const script = `
$newPath = [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String('${b64}'))
[Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
Add-Type -Namespace Win32 -Name EnvNotify -MemberDefinition '
[System.Runtime.InteropServices.DllImport("user32.dll", SetLastError=true, CharSet=System.Runtime.InteropServices.CharSet.Auto)]
public static extern System.IntPtr SendMessageTimeout(System.IntPtr hWnd, uint Msg, System.UIntPtr wParam, string lParam, uint fuFlags, uint uTimeout, out System.UIntPtr lpdwResult);
' -ErrorAction SilentlyContinue
$res = [UIntPtr]::Zero
[Win32.EnvNotify]::SendMessageTimeout([IntPtr]0xffff, 0x001A, [UIntPtr]::Zero, "Environment", 2, 2000, [ref]$res) | Out-Null
`;
  const res = Bun.spawnSync(["pwsh", "-NoLogo", "-NoProfile", "-Command", script], {
    stdout: "pipe",
    stderr: "pipe",
  });
  if (res.exitCode !== 0) {
    throw new Error(`Failed to update Windows User PATH: ${new TextDecoder().decode(res.stderr)}`);
  }
}

export function probeToolBinary(spec: ToolProbeSpec, combinedPathEnv?: string): ToolProbeResult {
  const env = combinedPathEnv ? { ...process.env, PATH: combinedPathEnv } : process.env;
  const whereBin = process.platform === "win32" ? "where.exe" : "which";
  const locate = Bun.spawnSync([whereBin, spec.bin], { stdout: "pipe", stderr: "pipe", env });
  const matches =
    locate.exitCode === 0
      ? new TextDecoder()
          .decode(locate.stdout)
          .split(/\r?\n/)
          .map((s) => s.trim())
          .filter(Boolean)
      : [];

  let version = "";
  if (matches.length > 0 && spec.versionArgs) {
    const vres = Bun.spawnSync([matches[0], ...spec.versionArgs], {
      stdout: "pipe",
      stderr: "pipe",
      env,
    });
    const rawOut = `${new TextDecoder().decode(vres.stdout)}\n${new TextDecoder().decode(vres.stderr)}`.trim();
    version = rawOut.split(/\r?\n/)[0]?.trim() ?? "";
  }

  return {
    id: spec.id,
    bin: spec.bin,
    category: spec.category,
    required: spec.required ?? false,
    found: matches.length > 0,
    path: matches[0] ?? null,
    allMatches: matches,
    version,
  };
}

export function runAudit(): ShellDoctorReport {
  const userEntries =
    process.platform === "win32" ? readWindowsRegistryPath("User") : splitPathList(process.env.PATH ?? "");
  const machineEntries = process.platform === "win32" ? readWindowsRegistryPath("Machine") : [];
  const { deadEntries, duplicateEntries, userDuplicateEntries, machineDuplicateEntries, conflicts } =
    analyzePathEntries(userEntries, machineEntries);

  const combinedPath = [...userEntries, ...machineEntries].join(process.platform === "win32" ? ";" : ":");
  const tools = MODERN_CLI_MATRIX.map((spec) => probeToolBinary(spec, combinedPath));
  const missingRequiredTools = tools.filter((t) => t.required && !t.found).map((t) => t.id);
  for (const missing of missingRequiredTools) {
    conflicts.push({
      kind: "missing-core",
      entry: missing,
      detail: `Required modern CLI '${missing}' not resolved on PATH`,
    });
  }

  return {
    schemaVersion: 1,
    platform: process.platform,
    generatedAt: new Date().toISOString(),
    userPathCount: userEntries.length,
    machinePathCount: machineEntries.length,
    deadEntries,
    duplicateEntries,
    userDuplicateEntries,
    machineDuplicateEntries,
    conflicts,
    tools,
    missingRequiredTools,
    ok: deadEntries.length === 0 && userDuplicateEntries.length === 0 && missingRequiredTools.length === 0,
  };
}

export function configureModernShells(): string[] {
  const configured: string[] = [];
  if (process.platform !== "win32") return configured;

  const home = homedir();
  const appData = process.env.APPDATA ?? join(home, "AppData", "Roaming");
  const nuDir = join(appData, "nushell");
  mkdirSync(nuDir, { recursive: true });

  const envNu = [
    "# Managed by aphrody tooling/bun/shell-path-doctor.ts (fast <15ms non-interactive startup)",
    "$env.LANG = 'en_US.UTF-8'",
    "$env.PYTHONUTF8 = '1'",
    "$env.RUSTC_BOOTSTRAP = '1'",
    "$env.CARGO_INCREMENTAL = '1'",
    "",
  ].join("\n");
  writeFileSync(join(nuDir, "env.nu"), envNu, "utf8");
  configured.push(join(nuDir, "env.nu"));

  const configNu = [
    "# Managed by aphrody tooling/bun/shell-path-doctor.ts",
    "$env.config.show_banner = false",
    "$env.config.buffer_editor = 'aphrody'",
    "$env.config.table.mode = 'rounded'",
    "$env.config.use_ansi_coloring = true",
    "if $nu.is-interactive {",
    "    if (which zoxide | is-not-empty) {",
    "        zoxide init nushell | save -f ($nu.default-config-dir | path join '.zoxide.nu')",
    "    }",
    "    if (which starship | is-not-empty) {",
    "        starship init nu | save -f ($nu.default-config-dir | path join '.starship.nu')",
    "    }",
    "}",
    "",
  ].join("\n");
  writeFileSync(join(nuDir, "config.nu"), configNu, "utf8");
  configured.push(join(nuDir, "config.nu"));

  const pwshDir = join(home, "Documents", "PowerShell");
  mkdirSync(pwshDir, { recursive: true });
  const pwshProfile = [
    "# Managed by aphrody tooling/bun/shell-path-doctor.ts (interactive-only hooks)",
    "$env:LANG = 'en_US.UTF-8'",
    "$env:PYTHONUTF8 = '1'",
    "[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)",
    "$isInteractiveShell = [Environment]::UserInteractive -and -not [Console]::IsInputRedirected",
    "if ($isInteractiveShell) {",
    "    if (Get-Command starship -ErrorAction SilentlyContinue) { Invoke-Expression (& starship init powershell) }",
    "    if (Get-Command zoxide -ErrorAction SilentlyContinue) { Invoke-Expression (& zoxide init powershell | Out-String) }",
    "}",
    "",
  ].join("\n");
  writeFileSync(join(pwshDir, "Microsoft.PowerShell_profile.ps1"), pwshProfile, "utf8");
  configured.push(join(pwshDir, "Microsoft.PowerShell_profile.ps1"));

  return configured;
}

export function runOptimize(dryRun: boolean, configureShells: boolean): OptimizeResult {
  const userBefore =
    process.platform === "win32" ? readWindowsRegistryPath("User") : splitPathList(process.env.PATH ?? "");
  const machineEntries = process.platform === "win32" ? readWindowsRegistryPath("Machine") : [];
  const { optimized, removed, added, reorderedToFront } = optimizeUserPathEntries(userBefore, machineEntries);

  let backupPath: string | null = null;
  if (!dryRun && process.platform === "win32") {
    const targetDir = join(repoRoot, "target");
    mkdirSync(targetDir, { recursive: true });
    backupPath = join(targetDir, "shell-path-backup.json");
    writeFileSync(
      backupPath,
      JSON.stringify({ savedAt: new Date().toISOString(), userBefore, machineEntries }, null, 2),
      "utf8",
    );
    writeWindowsUserPath(optimized);
  }

  const configuredFiles = !dryRun && configureShells ? configureModernShells() : [];
  const audit = runAudit();

  return {
    dryRun,
    backupPath,
    beforeCount: userBefore.length,
    afterCount: optimized.length,
    removedEntries: removed,
    addedEntries: added,
    reorderedToFront,
    configuredFiles,
    audit,
  };
}

if (import.meta.main) {
  const args = Bun.argv.slice(2);
  const sub = args[0] ?? "audit";
  const json = args.includes("--json");
  const dryRun = args.includes("--dry-run");
  const doShells = args.includes("--configure-shells") || sub === "optimize";

  if (sub === "audit") {
    const report = runAudit();
    if (json) {
      console.log(JSON.stringify(report, null, 2));
    } else {
      console.log(
        `Shell/PATH Audit: user=${report.userPathCount}, machine=${report.machinePathCount}, dead=${report.deadEntries.length}, dupes=${report.duplicateEntries.length}, missing_core=${report.missingRequiredTools.length}`,
      );
      for (const c of report.conflicts) {
        console.log(`  [${c.kind}] ${c.entry} — ${c.detail}`);
      }
      for (const t of report.tools) {
        console.log(
          `  ${t.found ? "OK  " : "MISS"} ${t.id.padEnd(15)} [${t.category.padEnd(8)}] ${t.path ?? "-"} ${t.version ? `(${t.version})` : ""}`,
        );
      }
    }
    if (!report.ok) process.exitCode = 1;
  } else if (sub === "optimize") {
    const result = runOptimize(dryRun, doShells);
    if (json) {
      console.log(JSON.stringify(result, null, 2));
    } else {
      console.log(
        `PATH Optimized (${result.dryRun ? "dry-run" : "applied"}): ${result.beforeCount} -> ${result.afterCount} entries`,
      );
      if (result.backupPath) console.log(`  Backup: ${result.backupPath}`);
      if (result.removedEntries.length) {
        console.log(`  Removed (${result.removedEntries.length}):`);
        for (const r of result.removedEntries) console.log(`    - ${r}`);
      }
      if (result.addedEntries.length) {
        console.log(`  Added (${result.addedEntries.length}):`);
        for (const a of result.addedEntries) console.log(`    + ${a}`);
      }
      if (result.configuredFiles.length) {
        console.log(`  Configured shells: ${result.configuredFiles.join(", ")}`);
      }
      console.log(
        `  Post-Audit: dead=${result.audit.deadEntries.length}, dupes=${result.audit.duplicateEntries.length}, missing_core=${result.audit.missingRequiredTools.join(",") || "none"}`,
      );
    }
  } else {
    console.error("Usage: bun tooling/bun/shell-path-doctor.ts <audit|optimize> [--dry-run] [--json]");
    process.exit(1);
  }
}
