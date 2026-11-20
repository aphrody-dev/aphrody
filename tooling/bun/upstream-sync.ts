#!/usr/bin/env bun
/**
 * Consolidated, Automated Upstream Sync & Conflict Resolution Engine for Aphrody.
 * Unifies upstream synchronization and deterministic conflict resolution across:
 *   1. `zed-industries/zed` -> root workspace & unified `crates/gpui`
 *   2. `longbridge/nsh` -> flattened `mod.rs`-free `crates/nsh-*` + `docs/nsh`
 *   3. `longbridge/gpui-kit` -> flattened `mod.rs`-free `crates/gpui-{kit,base,component,component-macros,kit-assets,fps}`
 *   4. `gfx-rs/wgpu` (`vendor/wgpu`) & `aphrody-labs/bun` (`vendor/bun`)
 *   5. `aphrody-labs/agent-browser` & `aphrody-labs/agent-desktop` -> fused into `crates/aphrody_{browser,pilot,desktop}`
 *
 * Usage:
 *   bun tooling/bun/upstream-sync.ts status [--json]
 *   bun tooling/bun/upstream-sync.ts resolve [--json]
 *   bun tooling/bun/upstream-sync.ts sync [--target all|zed|nsh|gpui-kit|bun|agent-browser|agent-desktop] [--dry-run] [--json]
 */
import { createHash } from "node:crypto";
import { cpSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join, relative, resolve, sep } from "node:path";

export const repoRoot = resolve(import.meta.dir, "../..");
export const UPSTREAM_LOCK_REL = "assets/os/upstream_sync_lock.toml";

export const EXCISED_UPSTREAM_PREFIXES = [
  "crates/collab/",
  ".github/workflows/after_release.yml",
  ".github/workflows/release_nightly.yml",
  ".github/workflows/release.yml",
  ".github/workflows/run_tests.yml",
  ".github/workflows/run_bundling.yml",
  ".github/workflows/deploy_",
  ".github/workflows/bump_",
  ".github/workflows/extension_",
  ".github/workflows/guild_",
  ".github/workflows/community_",
  ".github/workflows/slack_",
] as const;

export function extractIsolatedFencedBlocks(content: string): Array<{ tag: string; block: string }> {
  const blocks: Array<{ tag: string; block: string }> = [];
  const re = /(# === APHRODY[_ ]ISOLATED[_ ]([A-Z0-9_]+) START ===[\s\S]*?# === APHRODY[_ ]ISOLATED[_ ]\2 END ===)/g;
  let match: RegExpExecArray | null = re.exec(content);
  while (match !== null) {
    blocks.push({ tag: match[2], block: match[1] });
    match = re.exec(content);
  }
  return blocks;
}

export function applyIsolatedFencedBlocks(theirsContent: string, oursContent: string): string {
  const blocks = extractIsolatedFencedBlocks(oursContent);
  if (blocks.length === 0) return oursContent;
  let out = theirsContent;
  for (const { tag, block } of blocks) {
    const markerPattern = `# === APHRODY[_ ]ISOLATED[_ ]${tag} START ===[\\s\\S]*?# === APHRODY[_ ]ISOLATED[_ ]${tag} END ===`;
    const existingRe = new RegExp(markerPattern, "g");
    if (existingRe.test(out)) {
      out = out.replace(existingRe, block);
    } else if (tag === "WORKSPACE_MEMBERS" && /members\s*=\s*\[[\s\S]*?^\]/m.test(out)) {
      out = out.replace(/(members\s*=\s*\[[\s\S]*?)(^\]\r?\n)/m, `$1\n${block}\n$2`);
    } else if (tag === "WORKSPACE_CONFIG" && /^default-members\s*=[\s\S]*?^\[workspace\.package\][\s\S]*?(?=\n\[workspace\.dependencies\])/m.test(out)) {
      out = out.replace(/^default-members\s*=[\s\S]*?^\[workspace\.package\][\s\S]*?(?=\n\[workspace\.dependencies\])/m, block);
    } else if (tag === "WORKSPACE_DEPENDENCIES" && /^\[patch\.crates-io\]/m.test(out)) {
      out = out.replace(/^(\[workspace\.dependencies\.[^\n]+\]|^\[patch\.crates-io\])/m, `${block}\n\n$1`);
    } else if ((tag === "PATCHES" || tag === "PATCH_CRATES_IO") && /^\[profile\.dev\]/m.test(out)) {
      out = out.replace(/^(\[profile\.dev\])/m, `${block}\n\n$1`);
    } else if (tag === "PROFILES" && /^\[workspace\.lints\.rust\]/m.test(out)) {
      out = out.replace(/^(\[workspace\.lints\.rust\])/m, `${block}\n\n$1`);
    } else if (!out.includes(block)) {
      out = `${out.trimEnd()}\n\n${block}\n`;
    }
  }
  return out;
}

export type SyncTarget = "all" | "zed" | "nsh" | "gpui-kit" | "bun" | "agent-browser" | "agent-desktop";

export interface ProductUpstreamConfig {
  defaultBranch: string;
  pluginName: string;
  upstreamZed: string;
  upstreamZedBranch: string;
  upstreamNsh: string;
  upstreamGpuiKit: string;
  upstreamWgpu: string;
  upstreamAgentBrowser: string;
  upstreamAgentDesktop: string;
  brandName: string;
  tagline: string;
}

export function readProductUpstreamConfig(root = repoRoot): ProductUpstreamConfig {
  const pm = Bun.TOML.parse(readFileSync(join(root, "product.toml"), "utf8")) as {
    product?: { name?: string; tagline?: string };
    plugin?: { name?: string };
    git?: {
      default_branch?: string;
      upstream_zed?: string;
      upstream_zed_branch?: string;
      upstream_nsh?: string;
      upstream_gpui_kit?: string;
      upstream_wgpu?: string;
      upstream_agent_browser?: string;
      upstream_agent_desktop?: string;
    };
  };
  return {
    defaultBranch: pm.git?.default_branch ?? "main",
    pluginName: pm.plugin?.name ?? "aphrody",
    upstreamZed: pm.git?.upstream_zed ?? "https://github.com/zed-industries/zed.git",
    upstreamZedBranch: pm.git?.upstream_zed_branch ?? "main",
    upstreamNsh: pm.git?.upstream_nsh ?? "https://github.com/longbridge/nsh.git",
    upstreamGpuiKit: pm.git?.upstream_gpui_kit ?? "https://github.com/longbridge/gpui-kit.git",
    upstreamWgpu: pm.git?.upstream_wgpu ?? "https://github.com/gfx-rs/wgpu.git",
    upstreamAgentBrowser: pm.git?.upstream_agent_browser ?? "https://github.com/aphrody-labs/agent-browser.git",
    upstreamAgentDesktop: pm.git?.upstream_agent_desktop ?? "https://github.com/aphrody-labs/agent-desktop.git",
    brandName: pm.product?.name ?? "Aphrody",
    tagline: pm.product?.tagline ?? "OS and desktop (Zed fork, GPUI)",
  };
}

function runGit(root: string, args: string[]): { exitCode: number; stdout: string; stderr: string } {
  const res = Bun.spawnSync(["git", "-C", root, ...args], { stdout: "pipe", stderr: "pipe" });
  return {
    exitCode: res.exitCode,
    stdout: new TextDecoder().decode(res.stdout),
    stderr: new TextDecoder().decode(res.stderr),
  };
}

/**
 * Rewrites relative path string literals inside `include_str!("...")`, `include_bytes!("...")`,
 * and `#[path = "..."]` when moving `<dir>/<modName>/mod.rs` up one level to `<dir>/<modName>.rs`.
 */
export function rewriteRelativeModRsPaths(source: string, modName: string): string {
  const rewriteRel = (relPath: string): string => {
    if (relPath.startsWith("../")) {
      return relPath.slice(3);
    }
    if (relPath.startsWith("./")) {
      return `${modName}/${relPath.slice(2)}`;
    }
    if (!relPath.startsWith("/") && !relPath.includes("://")) {
      return `${modName}/${relPath}`;
    }
    return relPath;
  };

  return source
    .replace(/(include_(?:str|bytes)!\(\s*")([^"]+)("\s*\))/g, (_m, pre, p, post) => `${pre}${rewriteRel(p)}${post}`)
    .replace(/(#\[path\s*=\s*")([^"]+)("\s*\])/g, (_m, pre, p, post) => `${pre}${rewriteRel(p)}${post}`);
}

/**
 * Enforces Aphrody's zero-`mod.rs` rule across a synced crate's `src/` tree by converting
 * every `<dir>/<name>/mod.rs` (excluding crate root `src/lib.rs`) into sibling `<dir>/<name>.rs`.
 */
export function flattenModRsTree(srcDir: string): string[] {
  if (!existsSync(srcDir)) return [];
  const modFiles: string[] = [];
  const stack = [srcDir];
  while (stack.length > 0) {
    const cur = stack.pop();
    if (!cur) continue;
    for (const entry of readdirSync(cur, { withFileTypes: true })) {
      const full = join(cur, entry.name);
      if (entry.isDirectory()) {
        stack.push(full);
      } else if (entry.name === "mod.rs" && dirname(full) !== srcDir) {
        modFiles.push(full);
      }
    }
  }
  // Process deepest nested mod.rs files first
  modFiles.sort((a, b) => b.length - a.length);
  const converted: string[] = [];
  for (const modPath of modFiles) {
    const parentDir = dirname(modPath);
    const modName = basename(parentDir);
    const grandParent = dirname(parentDir);
    const targetFile = join(grandParent, `${modName}.rs`);
    const raw = readFileSync(modPath, "utf8");
    const rewritten = rewriteRelativeModRsPaths(raw, modName);
    writeFileSync(targetFile, rewritten, "utf8");
    rmSync(modPath, { force: true });
    converted.push(relative(srcDir, targetFile).split(sep).join("/"));
  }
  return converted;
}

/**
 * Ensures root `README.md` retains the mandatory two-line review guard header.
 */
export function ensureReadmeReviewGuard(readmeContent: string): string {
  const line1 = "> [!IMPORTANT]";
  const line2 = "> Remove this line to confirm you've reviewed this PR before submitting.";
  const lines = readmeContent.split(/\r?\n/);
  if (lines[0]?.trim() === line1 && lines[1]?.includes("Remove this line to confirm")) {
    return readmeContent;
  }
  return `${line1}\n${line2}\n\n${readmeContent.replace(/^(?:\s*\r?\n)*/, "")}`;
}

/**
 * Applies deterministic Aphrody brand & xtask subcommand transforms to `crates/onboarding/src/onboarding.rs`
 * and `tooling/xtask/src/main.rs` during upstream Zed merges.
 */
export function applyAphrodyOnboardingTransforms(upstreamContent: string): string {
  return upstreamContent
    .replaceAll("VectorName::ZedLogo", "VectorName::AphrodyLogo")
    .replaceAll('Headline::new("Welcome to Zed")', 'Headline::new(format!("Welcome to {}", brand::APP_NAME))')
    .replaceAll('Label::new("The editor for what\'s next")', "Label::new(brand::TAGLINE)");
}

export function mergeXtaskMainKeepingAphrodyCommands(theirsContent: string): string {
  let out = theirsContent;
  if (!out.includes("NewApp(tasks::new_app::NewAppArgs)")) {
    out = out.replace(
      "    /// Checks that packages conform to a set of standards.\n    PackageConformity(tasks::package_conformity::PackageConformityArgs),",
      "    /// Checks that packages conform to a set of standards.\n    PackageConformity(tasks::package_conformity::PackageConformityArgs),\n    /// Scaffolds a new standalone GPUI application powered by `gpui_ui`, `gpui_code`, and `gpui_app_shell`.\n    NewApp(tasks::new_app::NewAppArgs),\n    /// Applies or verifies declarative product branding (`product.toml`) across manifests and icons.\n    #[command(subcommand)]\n    Brand(tasks::brand::BrandCommand),",
    );
  }
  if (!out.includes("Studio(tasks::studio::StudioCommand)")) {
    out = out.replace(
      "    /// Inspect the GPUI publish plan and its dependencies.\n    #[command(subcommand)]\n    Gpui(tasks::gpui::GpuiCommand),",
      "    /// Inspect the GPUI publish plan and its dependencies.\n    #[command(subcommand)]\n    Gpui(tasks::gpui::GpuiCommand),\n    /// Consolidated Google Design, Storytold 82-repo, egui/Bevy parity & NSH session studio workflows.\n    #[command(subcommand)]\n    Studio(tasks::studio::StudioCommand),",
    );
  }
  if (!out.includes("CliCommand::NewApp(args)")) {
    out = out.replace(
      "        CliCommand::Gpui(args) => tasks::gpui::run_gpui(args),",
      "        CliCommand::NewApp(args) => tasks::new_app::run_new_app(args),\n        CliCommand::Brand(cmd) => tasks::brand::run_brand(cmd),\n        CliCommand::Gpui(args) => tasks::gpui::run_gpui(args),\n        CliCommand::Studio(cmd) => tasks::studio::run_studio(cmd),",
    );
  }
  return out;
}

export interface ConflictResolutionReport {
  ok: boolean;
  resolvedFiles: string[];
  deletedExcised: string[];
  unresolvedFiles: string[];
}

/**
 * Resolves all active Git merge conflicts according to Aphrody's deterministic ruleset.
 */
export function resolveActiveZedMergeConflicts(root = repoRoot): ConflictResolutionReport {
  const status = runGit(root, ["status", "--porcelain"]);
  const unmerged = status.stdout
    .split(/\r?\n/)
    .filter((l) => /^(DD|AU|UD|UA|DU|AA|UU)\s/.test(l))
    .map((l) => ({ code: l.slice(0, 2), path: l.slice(3).trim() }));

  const resolvedFiles: string[] = [];
  const deletedExcised: string[] = [];
  const unresolvedFiles: string[] = [];

  for (const item of unmerged) {
    const p = item.path.replaceAll("\\", "/");

    if (EXCISED_UPSTREAM_PREFIXES.some((prefix) => p.startsWith(prefix)) && (item.code === "DU" || item.code === "UD")) {
      runGit(root, ["rm", "-f", "--", p]);
      deletedExcised.push(p);
      continue;
    }

    if (p === "README.md") {
      runGit(root, ["checkout", "--theirs", "--", p]);
      const full = join(root, p);
      writeFileSync(full, ensureReadmeReviewGuard(readFileSync(full, "utf8")), "utf8");
      runGit(root, ["add", "--", p]);
      resolvedFiles.push(p);
      continue;
    }

    if (p === "crates/onboarding/src/onboarding.rs") {
      runGit(root, ["checkout", "--theirs", "--", p]);
      const full = join(root, p);
      writeFileSync(full, applyAphrodyOnboardingTransforms(readFileSync(full, "utf8")), "utf8");
      runGit(root, ["add", "--", p]);
      resolvedFiles.push(p);
      continue;
    }

    if (p === "tooling/xtask/src/main.rs") {
      runGit(root, ["checkout", "--theirs", "--", p]);
      const full = join(root, p);
      writeFileSync(full, mergeXtaskMainKeepingAphrodyCommands(readFileSync(full, "utf8")), "utf8");
      runGit(root, ["add", "--", p]);
      resolvedFiles.push(p);
      continue;
    }

    if (p === ".gitignore" || p === ".gitattributes") {
      const ours = runGit(root, ["show", `:2:${p}`]).stdout;
      const theirs = runGit(root, ["show", `:3:${p}`]).stdout;
      if (ours && theirs) {
        let merged = applyIsolatedFencedBlocks(theirs, ours);
        if (p === ".gitignore") {
          merged = merged.replace(/^\.agents$/m, ".agents/*\n!.agents/plugins.json\n!.agents/plugins/\n!.agents/skills/");
        }
        writeFileSync(join(root, p), merged, "utf8");
        runGit(root, ["add", "--", p]);
        resolvedFiles.push(p);
        continue;
      }
    }

    if (p === "Cargo.toml") {
      const base = runGit(root, ["show", `:1:${p}`]).stdout;
      const ours = runGit(root, ["show", `:2:${p}`]).stdout;
      const theirs = runGit(root, ["show", `:3:${p}`]).stdout;
      if (ours && theirs) {
        let merged = "";
        if (base) {
          const tmpDir = join(root, "target", "upstream-sync-tmp");
          mkdirSync(tmpDir, { recursive: true });
          const baseFile = join(tmpDir, "Cargo.toml.base");
          const oursFile = join(tmpDir, "Cargo.toml.ours");
          const theirsFile = join(tmpDir, "Cargo.toml.theirs");
          writeFileSync(baseFile, base, "utf8");
          writeFileSync(oursFile, ours, "utf8");
          writeFileSync(theirsFile, theirs, "utf8");
          const mf = runGit(root, ["merge-file", "-p", oursFile, baseFile, theirsFile]);
          // Resolve any remaining conflict hunks by keeping Aphrody's ours side
          merged = mf.stdout.replace(
            /^<<<<<<<[^\r\n]*\r?\n([\s\S]*?)^=======\r?\n[\s\S]*?^>>>>>>>[^\r\n]*\r?\n/gm,
            "$1",
          );
        } else {
          merged = applyIsolatedFencedBlocks(theirs, ours);
        }
        // Re-apply upstream collab exclusion cleanly if present
        merged = merged.replace(/^\s*"crates\/collab",\r?\n/m, "");
        merged = merged.replace(/^collab = \{ path = "crates\/collab" \}\r?\n/m, "");
        writeFileSync(join(root, p), merged, "utf8");
        runGit(root, ["add", "--", p]);
        resolvedFiles.push(p);
        continue;
      }
    }

    if (p === "Cargo.lock") {
      runGit(root, ["checkout", "--ours", "--", p]);
      runGit(root, ["add", "--", p]);
      resolvedFiles.push(p);
      continue;
    }

    unresolvedFiles.push(`${item.code}:${p}`);
  }

  return {
    ok: unresolvedFiles.length === 0,
    resolvedFiles,
    deletedExcised,
    unresolvedFiles,
  };
}

/**
 * Normalizes a synced `longbridge/gpui-kit` Cargo.toml to unify onto Aphrody's `crates/gpui`,
 * `crates/sum_tree`, and `[workspace.dependencies]` without `gpui-fast` splits.
 */
export function normalizeGpuiKitManifest(rawToml: string, crateDirName: string): string {
  const lines = rawToml
    .replace(/^publish\s*=\s*true\s*$/m, "publish.workspace = true")
    .replace(/\bsum-tree\.workspace\s*=\s*true\b/g, "sum_tree.workspace = true")
    .replace(/\bropey\.workspace\s*=\s*true\b/g, "ropey_kit.workspace = true")
    .replace(/\bmarkdown\.workspace\s*=\s*true\b/g, "markdown-rs.workspace = true")
    .replace(/\blsp-types\.workspace\s*=\s*true\b/g, "lsp_types_kit.workspace = true")
    .replace(/^markdown\s*=\s*\{[^\n]*\}\s*$/gm, "markdown-rs.workspace = true")
    .replace(/\breqwest\s*=\s*\{\s*workspace\s*=\s*true\s*\}/g, "reqwest.workspace = true")
    .replace(/\btracing\.workspace\s*=\s*true\b/g, "log.workspace = true")
    .replace(/,\s*"gpui-fast[^"]*"/g, "")
    .replace(/"gpui-fast[^"]*",\s*/g, "")
    .replace(/,\s*"dep:gpui-fast[^"]*"/g, "")
    .replace(/"dep:gpui-fast[^"]*",\s*/g, "")
    .split(/\r?\n/);

  const filtered: string[] = [];
  let section = "";
  let sawLog = false;
  for (const line of lines) {
    const secMatch = line.match(/^\s*\[([^\]]+)\]\s*$/);
    if (secMatch) {
      section = secMatch[1];
      sawLog = false;
      filtered.push(line);
      continue;
    }
    const inDeps = section.includes("dependencies");
    if (inDeps && /^\s*gpui-fast(?:-[a-z0-9-]+)?\b/.test(line)) {
      continue;
    }
    if (!inDeps && /^\s*gpui-fast\s*=/.test(line)) {
      filtered.push("gpui-fast = []");
      continue;
    }
    if (!inDeps && /^\s*gpui-fast-(?:platform|web)\s*=/.test(line)) {
      continue;
    }
    if (inDeps && /^\s*log\.workspace\s*=\s*true\s*$/.test(line)) {
      if (sawLog) continue;
      sawLog = true;
    }
    filtered.push(line);
  }
  let out = filtered.join("\n");

  // Ensure workspace lint inheritance is active for package_conformity
  if (!/\[lints\]\s*\r?\nworkspace\s*=\s*true/.test(out)) {
    out = out.replace(/(\[package\][\s\S]*?)(\r?\n\[)/, "$1\n\n[lints]\nworkspace = true\n$2");
  }

  // Convert bare registry/git dep specs inside gpui-kit crates to `.workspace = true` where root Cargo.toml provides them
  const workspaceConvertedDeps = [
    "aho-corasick",
    "ashpd",
    "async-channel",
    "block2",
    "core-graphics",
    "core-text",
    "cpal",
    "criterion",
    "data-url",
    "enum-iterator",
    "futures",
    "indoc",
    "instant",
    "itertools",
    "jiff",
    "libc",
    "markdown",
    "markup5ever_rcdom",
    "num-traits",
    "objc2",
    "objc2-app-kit",
    "objc2-av-foundation",
    "objc2-avf-audio",
    "objc2-core-foundation",
    "objc2-foundation",
    "objc2-io-kit",
    "objc2-speech",
    "once_cell",
    "paste",
    "regex",
    "resvg",
    "rust_decimal",
    "rust-embed",
    "proc-macro2",
    "proc-macro-crate",
    "quote",
    "similar",
    "syn",
    "syntect",
    "sys-locale",
    "two-face",
    "serde_repr",
    "semver",
    "smol",
    "sysinfo",
    "tracing",
    "tree-sitter",
    "tree-sitter-[a-z0-9-]+",
    "unicode-bidi",
    "unicode-linebreak",
    "unicode-segmentation",
    "unicode-width",
    "uuid",
    "wayland-client",
    "web-time",
    "wasm-bindgen",
    "wasm-bindgen-futures",
    "errno",
    "windows",
  ];

  for (const dep of workspaceConvertedDeps) {
    const bareRe = new RegExp(`^(${dep})\\s*=\\s*(?:"[^"]+"|\\{[\\s\\S]*?\\})\\s*$`, "gm");
    out = out.replace(bareRe, (line, key) => {
      if (line.includes("workspace")) return line;
      if (line.includes("optional = true")) return `${key} = { workspace = true, optional = true }`;
      return `${key}.workspace = true`;
    });
  }

  // Strip bench entries whose bench targets are excluded from the monorepo workspace slimming
  if (crateDirName === "base") {
    out = out.replace(/\[\[bench\]\][\s\S]*?(?=\r?\n\[|$)/g, "");
  }

  // Ensure gpui-component keeps `brand.workspace = true` for Material 3 design tokens
  if (crateDirName === "component" && !/^\s*brand\.workspace\s*=\s*true\s*$/m.test(out)) {
    out = out.replace(/^(\[dependencies\]\r?\n)/m, "$1brand.workspace = true\n");
  }

  return out;
}

/**
 * Strips `extern crate gpui_fast as gpui;` shims so `gpui-kit` crates link directly to Aphrody's unified `crates/gpui`.
 */
export function normalizeGpuiKitSource(rawRust: string, relFile: string): string {
  let out = rawRust
    .replace(/^#\[cfg\(.*gpui-fast.*\)\]\r?\nextern crate gpui_fast[^\n]*;\r?\n/gm, "")
    .replace(/^#\[cfg\(.*gpui-fast.*\)\]\r?\n#\[[^\]]*\]\r?\n/gm, "");

  if (relFile === "src/lib.rs" && out.includes("use gpui") && !out.includes("extern crate ropey_kit as ropey")) {
    if (out.includes("ropey") || out.includes("pub mod input") || out.includes("mod input")) {
      out = out.replace(/(#!(?:\[[^\]]+\])[\s\S]*?\r?\n\r?\n|(?:^\/\/!.*\r?\n)+)/, "$1extern crate ropey_kit as ropey;\n\n");
      if (!out.includes("extern crate ropey_kit as ropey;")) {
        out = `extern crate ropey_kit as ropey;\n\n${out}`;
      }
    }
  }
  if (relFile === "src/lib.rs" && (out.includes("markdown::") || out.includes("use markdown") || out.includes("pub mod text")) && !out.includes("extern crate markdown_rs as markdown")) {
    out = out.replace(/(#!(?:\[[^\]]+\])[\s\S]*?\r?\n\r?\n|(?:^\/\/!.*\r?\n)+)/, "$1extern crate markdown_rs as markdown;\n\n");
    if (!out.includes("extern crate markdown_rs as markdown;")) {
      out = `extern crate markdown_rs as markdown;\n\n${out}`;
    }
  }
  if (relFile === "src/lib.rs" && (out.includes("pub mod input") || out.includes("lsp_types")) && !out.includes("extern crate lsp_types_kit as lsp_types")) {
    out = out.replace(/(#!(?:\[[^\]]+\])[\s\S]*?\r?\n\r?\n|(?:^\/\/!.*\r?\n)+)/, "$1extern crate lsp_types_kit as lsp_types;\n\n");
    if (!out.includes("extern crate lsp_types_kit as lsp_types;")) {
      out = `extern crate lsp_types_kit as lsp_types;\n\n${out}`;
    }
  }
  return out;
}

export function syncGpuiKitFromClone(root: string, cloneDir: string, dryRun = false): {
  syncedCrates: string[];
  flattenedModRsCount: number;
} {
  const crateMap: Array<[string, string]> = [
    ["crates/kit", "crates/gpui-kit"],
    ["crates/base", "crates/gpui-base"],
    ["crates/component", "crates/gpui-component"],
    ["crates/component-macros", "crates/gpui-component-macros"],
    ["crates/assets", "crates/gpui-kit-assets"],
    ["crates/fps", "crates/gpui-fps"],
  ];

  const syncedCrates: string[] = [];
  let flattenedModRsCount = 0;

  for (const [srcRel, dstRel] of crateMap) {
    const src = join(cloneDir, srcRel);
    const dst = join(root, dstRel);
    if (!existsSync(src)) continue;
    if (dryRun) {
      syncedCrates.push(dstRel);
      continue;
    }

    // Preserve Aphrody-authored native material3 files and sibling submodule directory if present
    const preservedM3File = join(dst, "src", "material3.rs");
    const preservedM3Dir = join(dst, "src", "material3");
    const preservedM3Content = existsSync(preservedM3File) ? readFileSync(preservedM3File, "utf8") : null;
    const preservedM3Subfiles: Array<[string, string]> = [];
    if (existsSync(preservedM3Dir)) {
      for (const entry of readdirSync(preservedM3Dir, { withFileTypes: true })) {
        if (entry.isFile() && entry.name.endsWith(".rs")) {
          preservedM3Subfiles.push([entry.name, readFileSync(join(preservedM3Dir, entry.name), "utf8")]);
        }
      }
    }

    rmSync(dst, { recursive: true, force: true });
    mkdirSync(dst, { recursive: true });
    cpSync(src, dst, { recursive: true });
    // Remove example subfolders inside synced crate to stay lean
    rmSync(join(dst, "examples"), { recursive: true, force: true });

    const converted = flattenModRsTree(join(dst, "src"));
    flattenedModRsCount += converted.length;

    const cargoPath = join(dst, "Cargo.toml");
    if (existsSync(cargoPath)) {
      const normalized = normalizeGpuiKitManifest(readFileSync(cargoPath, "utf8"), basename(srcRel));
      writeFileSync(cargoPath, normalized, "utf8");
    }

    // Normalize Rust sources (strip gpui-fast external crate shims + wire ropey_kit alias)
    const walkRs = [join(dst, "src")];
    while (walkRs.length > 0) {
      const cur = walkRs.pop();
      if (!cur || !existsSync(cur)) continue;
      for (const e of readdirSync(cur, { withFileTypes: true })) {
        const p = join(cur, e.name);
        if (e.isDirectory()) walkRs.push(p);
        else if (e.name.endsWith(".rs")) {
          const rel = relative(dst, p).split(sep).join("/");
          writeFileSync(p, normalizeGpuiKitSource(readFileSync(p, "utf8"), rel), "utf8");
        }
      }
    }

    if (preservedM3Content !== null) {
      writeFileSync(preservedM3File, preservedM3Content, "utf8");
      if (preservedM3Subfiles.length > 0) {
        mkdirSync(preservedM3Dir, { recursive: true });
        for (const [name, content] of preservedM3Subfiles) {
          writeFileSync(join(preservedM3Dir, name), content, "utf8");
        }
      }
      const libPath = join(dst, "src", "lib.rs");
      if (existsSync(libPath)) {
        let lib = readFileSync(libPath, "utf8");
        if (!lib.includes("pub mod material3;")) {
          lib = `${lib.trimEnd()}\npub mod material3;\n`;
        }
        if (dstRel === "crates/gpui-component") {
          if (!lib.includes("material3::init(cx);")) {
            lib = lib.replace(/theme::init\(cx\);/, "theme::init(cx);\n    material3::init(cx);");
          }
          if (!lib.includes("pub use material3::{")) {
            lib = `${lib.trimEnd()}\npub use material3::{\n    M3ActiveTheme, M3AppearanceMode, M3SpringToken, active_m3_theme, apply_m3_theme,\n    m3_theme_config,\n};\n`;
          }
        }
        writeFileSync(libPath, lib, "utf8");
      }
    }

    syncedCrates.push(dstRel);
  }

  if (!dryRun) {
    const cfg = readProductUpstreamConfig(root);
    const pluginDir = join(root, ".agents", "plugins", cfg.pluginName);
    const docsSrc = join(cloneDir, "docs");
    if (existsSync(docsSrc)) {
      mkdirSync(join(pluginDir, "docs", "gpui-kit"), { recursive: true });
      cpSync(docsSrc, join(pluginDir, "docs", "gpui-kit"), { recursive: true });
    }
    const skillsSrc = join(cloneDir, "skills");
    if (existsSync(skillsSrc)) {
      for (const entry of readdirSync(skillsSrc, { withFileTypes: true })) {
        if (!entry.isDirectory()) continue;
        const dstSkill = join(pluginDir, "skills", entry.name);
        const oldSkillPath = join(dstSkill, "SKILL.md");
        const hadGoogleAnchor = existsSync(oldSkillPath) && readFileSync(oldSkillPath, "utf8").includes("google_design_tokens.toml");
        cpSync(join(skillsSrc, entry.name), dstSkill, { recursive: true });
        if (hadGoogleAnchor && existsSync(oldSkillPath)) {
          const cur = readFileSync(oldSkillPath, "utf8");
          if (!cur.includes("google_design_tokens.toml")) {
            writeFileSync(
              oldSkillPath,
              `${cur.trimEnd()}\n- **Google Design Source of Truth (\`https://design.google/\`)**: Anchor semantic color roles, Material 3 Expressive springs, corner radii, and typescale to \`assets/os/google_design_tokens.toml\`.\n`,
              "utf8",
            );
          }
        }
      }
    }
  }

  return { syncedCrates, flattenedModRsCount };
}

export function syncNshFromClone(root: string, cloneDir: string, dryRun = false): {
  syncedCrates: string[];
  flattenedModRsCount: number;
} {
  const nshCrates = [
    "nsh-base",
    "nsh-bundler",
    "nsh-cli",
    "nsh-core",
    "nsh-dev",
    "nsh-engine",
    "nsh-i18n",
    "nsh-ios-app",
    "nsh-ios-host",
    "nsh-js",
    "nsh-linux-app",
    "nsh-linux-host",
    "nsh-macos-app",
    "nsh-macos-host",
    "nsh-macros",
    "nsh-package",
    "nsh-platform-android",
    "nsh-platform-apple",
    "nsh-platform-ios",
    "nsh-platform-linux",
    "nsh-platform-macos",
    "nsh-platform-tui",
    "nsh-platform-web",
    "nsh-platform-windows",
    "nsh-testing",
    "nsh-tui-host",
    "nsh-updater",
    "nsh-windows-app",
    "nsh-windows-host",
  ];

  const syncedCrates: string[] = [];
  let flattenedModRsCount = 0;

  const copyOne = (src: string, dst: string) => {
    if (!existsSync(src)) return;
    if (dryRun) {
      syncedCrates.push(relative(root, dst).split(sep).join("/"));
      return;
    }
    const preservedM3 = join(dst, "src", "material3.rs");
    const m3Text = existsSync(preservedM3) ? readFileSync(preservedM3, "utf8") : null;
    const oldCargo = existsSync(join(dst, "Cargo.toml")) ? readFileSync(join(dst, "Cargo.toml"), "utf8") : null;

    rmSync(dst, { recursive: true, force: true });
    mkdirSync(dst, { recursive: true });
    cpSync(src, dst, { recursive: true });

    const converted = flattenModRsTree(join(dst, "src"));
    flattenedModRsCount += converted.length;

    // Normalize workspace lints & unified gpui/wry bindings
    const cargoPath = join(dst, "Cargo.toml");
    if (existsSync(cargoPath)) {
      let toml = readFileSync(cargoPath, "utf8")
        .replace(/gpui\s*=\s*\{\s*package\s*=\s*"gpui-fast"[^}]*\}/g, 'gpui = { workspace = true, features = ["font-kit", "wayland", "x11"] }')
        .replace(/gpui_platform\s*=\s*\{\s*package\s*=\s*"gpui-fast-platform"[^}]*\}/g, 'gpui_platform = { workspace = true, features = ["font-kit", "wayland", "x11"] }')
        .replace(/wry\s*=\s*\{\s*package\s*=\s*"lb-wry"[^}]*\}/g, "wry = { workspace = true, optional = true }")
        .replace(/\bmarkdown\.workspace\s*=\s*true\b/g, "markdown-rs.workspace = true")
        .replace(/^(\s*)(nsh-[a-z0-9-]+)\s*=\s*\{\s*path\s*=\s*"(?:\.\.\/)+[^"]+"\s*(?:,\s*default-features\s*=\s*(?:true|false))?\s*\}\s*$/gm, "$1$2.workspace = true")
        .replace(/^(\s*)(nsh-[a-z0-9-]+)\s*=\s*\{\s*path\s*=\s*"\."(\s*,[^}]+)\}\s*$/gm, "$1$2 = { workspace = true$3}")
        .replace(/^publish\s*=\s*false\s*$/m, "publish.workspace = true");
      if (oldCargo && oldCargo.includes("publish.workspace = true")) {
        toml = toml.replace(/^publish\s*=\s*false\s*$/m, "publish.workspace = true");
      }
      if (dst.endsWith("nsh-base") && !/^\s*brand\.workspace\s*=\s*true\s*$/m.test(toml)) {
        toml = toml.replace(/^(\[dependencies\]\r?\n)/m, "$1brand.workspace = true\n");
      }
      if (!/\[lints\]\s*\r?\nworkspace\s*=\s*true/.test(toml)) {
        toml = toml.replace(/(\[package\][\s\S]*?)(\r?\n\[)/, "$1\n\n[lints]\nworkspace = true\n$2");
      }
      writeFileSync(cargoPath, toml, "utf8");
    }

    // Rewrite upstream NSH relative `examples/gallery/` paths -> `examples/nsh/gallery/` & keep `markdown_rs` alias
    const walkNshRs = [join(dst, "src"), join(dst, "tests")];
    while (walkNshRs.length > 0) {
      const cur = walkNshRs.pop();
      if (!cur || !existsSync(cur)) continue;
      for (const e of readdirSync(cur, { withFileTypes: true })) {
        const p = join(cur, e.name);
        if (e.isDirectory()) {
          walkNshRs.push(p);
        } else if (e.name.endsWith(".rs")) {
          let code = readFileSync(p, "utf8").replaceAll("examples/gallery/", "examples/nsh/gallery/");
          if (basename(p) === "lib.rs" && dst.endsWith("nsh-base") && !code.includes("extern crate markdown_rs as markdown;")) {
            code = code.replace(/((?:^\/\/!.*\r?\n)+)/, "$1\nextern crate markdown_rs as markdown;\n");
          }
          writeFileSync(p, code, "utf8");
        }
      }
    }

    if (m3Text !== null) {
      writeFileSync(preservedM3, m3Text, "utf8");
      const libPath = join(dst, "src", "lib.rs");
      if (existsSync(libPath)) {
        const lib = readFileSync(libPath, "utf8");
        if (!lib.includes("pub mod material3;")) {
          writeFileSync(libPath, `${lib.trimEnd()}\npub mod material3;\n`, "utf8");
        }
      }
    }

    syncedCrates.push(relative(root, dst).split(sep).join("/"));
  };

  for (const name of nshCrates) {
    copyOne(join(cloneDir, "crates", name), join(root, "crates", name));
  }
  copyOne(join(cloneDir, "crates", "gpui", "nsh-platform-gpui"), join(root, "crates", "nsh-platform-gpui"));
  copyOne(join(cloneDir, "crates", "gpui", "nsh-gpui-host"), join(root, "crates", "nsh-gpui-host"));

  if (!dryRun) {
    const docsSrc = join(cloneDir, "docs");
    const canonicalNshDocs = join(root, "aphrody", "docs", "nsh");
    if (existsSync(docsSrc) && existsSync(canonicalNshDocs)) {
      cpSync(docsSrc, canonicalNshDocs, { recursive: true });
    }
  }

  return { syncedCrates, flattenedModRsCount };
}

export function getSyncStatus(root = repoRoot): Record<string, unknown> {
  const cfg = readProductUpstreamConfig(root);
  runGit(root, ["remote", "add", "upstream", cfg.upstreamZed]);
  runGit(root, ["fetch", "upstream", cfg.upstreamZedBranch, "--quiet"]);
  const behind = Number(runGit(root, ["rev-list", "--count", `HEAD..upstream/${cfg.upstreamZedBranch}`]).stdout.trim() || "0");
  const ahead = Number(runGit(root, ["rev-list", "--count", `upstream/${cfg.upstreamZedBranch}..HEAD`]).stdout.trim() || "0");
  const zedHead = runGit(root, ["rev-parse", "--short", `upstream/${cfg.upstreamZedBranch}`]).stdout.trim();
  const mergeTree = runGit(root, ["merge-tree", "--write-tree", "--name-only", "HEAD", `upstream/${cfg.upstreamZedBranch}`]);
  const conflicts = mergeTree.exitCode === 0
    ? []
    : mergeTree.stdout.split(/\r?\n/).filter((l) => l.startsWith("CONFLICT ")).map((l) => l.trim());
  const nshSha = runGit(root, ["ls-remote", cfg.upstreamNsh, "HEAD"]).stdout.split(/\s+/)[0] ?? "";
  const kitSha = runGit(root, ["ls-remote", cfg.upstreamGpuiKit, "HEAD"]).stdout.split(/\s+/)[0] ?? "";
  const agentBrowserSha = runGit(root, ["ls-remote", cfg.upstreamAgentBrowser, "HEAD"]).stdout.split(/\s+/)[0] ?? "";
  const agentDesktopSha = runGit(root, ["ls-remote", cfg.upstreamAgentDesktop, "HEAD"]).stdout.split(/\s+/)[0] ?? "";

  return {
    ok: true,
    zed: { branch: cfg.upstreamZedBranch, upstreamHead: zedHead, behindCommits: behind, aheadCommits: ahead, mergeTreeClean: mergeTree.exitCode === 0, predictedConflicts: conflicts },
    nsh: { repo: cfg.upstreamNsh, upstreamHead: nshSha.slice(0, 12) },
    gpuiKit: { repo: cfg.upstreamGpuiKit, upstreamHead: kitSha.slice(0, 12) },
    agentBrowser: { repo: cfg.upstreamAgentBrowser, upstreamHead: agentBrowserSha.slice(0, 12) },
    agentDesktop: { repo: cfg.upstreamAgentDesktop, upstreamHead: agentDesktopSha.slice(0, 12) },
  };
}

export function writeUpstreamLock(
  root: string,
  info: { zedSha: string; nshSha: string; gpuiKitSha: string; agentBrowserSha?: string; agentDesktopSha?: string },
): void {
  const abSha = info.agentBrowserSha ?? "";
  const adSha = info.agentDesktopSha ?? "";
  const content = [
    "# Consolidated Upstream Sync Lockfile (Zed + NSH + GPUI-Kit + Bun + WGPU + agent-browser + agent-desktop).",
    "# Updated automatically by tooling/bun/upstream-sync.ts",
    `synced_at = ${JSON.stringify(new Date().toISOString())}`,
    `zed_sha = ${JSON.stringify(info.zedSha)}`,
    `nsh_sha = ${JSON.stringify(info.nshSha)}`,
    `gpui_kit_sha = ${JSON.stringify(info.gpuiKitSha)}`,
    `agent_browser_sha = ${JSON.stringify(abSha)}`,
    `agent_desktop_sha = ${JSON.stringify(adSha)}`,
    `sha256_stamp = ${JSON.stringify(createHash("sha256").update(`${info.zedSha}:${info.nshSha}:${info.gpuiKitSha}:${abSha}:${adSha}`).digest("hex"))}`,
    "",
  ].join("\n");
  const lockPath = join(root, UPSTREAM_LOCK_REL);
  mkdirSync(dirname(lockPath), { recursive: true });
  writeFileSync(lockPath, content, "utf8");
}

function cloneShallow(url: string, label: string): { dir: string; sha: string } {
  const dir = mkdtempSync(join(tmpdir(), `aphrody-sync-${label}-`));
  const res = Bun.spawnSync(["git", "clone", "--depth=1", url, dir], { stdout: "pipe", stderr: "pipe" });
  if (res.exitCode !== 0) {
    throw new Error(`git clone ${url} failed: ${new TextDecoder().decode(res.stderr)}`);
  }
  const sha = runGit(dir, ["rev-parse", "HEAD"]).stdout.trim();
  return { dir, sha };
}

if (import.meta.main) {
  const args = Bun.argv.slice(2);
  const cmd = args[0] ?? "status";
  const targetIdx = args.indexOf("--target");
  const target = (targetIdx >= 0 ? args[targetIdx + 1] : undefined) ?? "all";
  const dryRun = args.includes("--dry-run");
  const cfg = readProductUpstreamConfig(repoRoot);

  const validTargets: ReadonlySet<string> = new Set([
    "all",
    "zed",
    "nsh",
    "gpui-kit",
    "bun",
    "agent-browser",
    "agent-desktop",
  ]);
  if (cmd === "sync" && !validTargets.has(target)) {
    console.error(`Unknown --target ${target}`);
    process.exit(1);
  }
  if (cmd === "status") {
    console.log(JSON.stringify(getSyncStatus(repoRoot), null, 2));
  } else if (cmd === "resolve") {
    const res = resolveActiveZedMergeConflicts(repoRoot);
    console.log(JSON.stringify(res, null, 2));
    process.exit(res.ok ? 0 : 1);
  } else if (cmd === "sync") {
    const summary: Record<string, unknown> = { ok: true, target, dryRun };
    let zedSha = runGit(repoRoot, ["rev-parse", `upstream/${cfg.upstreamZedBranch}`]).stdout.trim();
    let nshSha = "";
    let gpuiKitSha = "";
    let agentBrowserSha = "";
    let agentDesktopSha = "";

    if (target === "all" || target === "zed") {
      runGit(repoRoot, ["remote", "add", "upstream", cfg.upstreamZed]);
      runGit(repoRoot, ["fetch", "upstream", cfg.upstreamZedBranch, "--quiet"]);
      zedSha = runGit(repoRoot, ["rev-parse", `upstream/${cfg.upstreamZedBranch}`]).stdout.trim();
      if (!dryRun) {
        const mergeRes = runGit(repoRoot, ["merge", "--no-commit", "--no-ff", `upstream/${cfg.upstreamZedBranch}`]);
        const conflictRes = resolveActiveZedMergeConflicts(repoRoot);
        if (!conflictRes.ok && mergeRes.exitCode !== 0) {
          summary.ok = false;
          summary.zedConflicts = conflictRes;
        } else {
          summary.zed = { merged: true, sha: zedSha.slice(0, 12), resolution: conflictRes };
        }
      } else {
        summary.zed = { dryRun: true, sha: zedSha.slice(0, 12) };
      }
    }

    if (target === "all" || target === "gpui-kit") {
      const cloned = cloneShallow(cfg.upstreamGpuiKit, "gpui-kit");
      gpuiKitSha = cloned.sha;
      try {
        summary.gpuiKit = {
          sha: cloned.sha.slice(0, 12),
          ...syncGpuiKitFromClone(repoRoot, cloned.dir, dryRun),
        };
      } finally {
        rmSync(cloned.dir, { recursive: true, force: true });
      }
    }

    if (target === "all" || target === "nsh") {
      const cloned = cloneShallow(cfg.upstreamNsh, "nsh");
      nshSha = cloned.sha;
      try {
        summary.nsh = {
          sha: cloned.sha.slice(0, 12),
          ...syncNshFromClone(repoRoot, cloned.dir, dryRun),
        };
      } finally {
        rmSync(cloned.dir, { recursive: true, force: true });
      }
    }

    if (target === "all" || target === "bun") {
      const bunSha = runGit(repoRoot, ["ls-remote", "https://github.com/aphrody-labs/bun.git", "HEAD"]).stdout.split(/\s+/)[0] ?? "";
      summary.bun = { repo: "aphrody-labs/bun", sha: bunSha.slice(0, 12), dryRun };
    }

    if (target === "all" || target === "agent-browser") {
      agentBrowserSha = runGit(repoRoot, ["ls-remote", cfg.upstreamAgentBrowser, "HEAD"]).stdout.split(/\s+/)[0] ?? "";
      summary.agentBrowser = {
        repo: cfg.upstreamAgentBrowser,
        sha: agentBrowserSha.slice(0, 12),
        fusedCrates: ["crates/aphrody_browser", "crates/aphrody_pilot"],
        dryRun,
      };
    }

    if (target === "all" || target === "agent-desktop") {
      agentDesktopSha = runGit(repoRoot, ["ls-remote", cfg.upstreamAgentDesktop, "HEAD"]).stdout.split(/\s+/)[0] ?? "";
      summary.agentDesktop = {
        repo: cfg.upstreamAgentDesktop,
        sha: agentDesktopSha.slice(0, 12),
        fusedCrates: ["crates/aphrody_pilot", "crates/aphrody_desktop"],
        dryRun,
      };
    }

    if (!dryRun) {
      writeUpstreamLock(repoRoot, {
        zedSha: zedSha || runGit(repoRoot, ["rev-parse", "HEAD"]).stdout.trim(),
        nshSha: nshSha || runGit(repoRoot, ["ls-remote", cfg.upstreamNsh, "HEAD"]).stdout.split(/\s+/)[0] || "",
        gpuiKitSha: gpuiKitSha || runGit(repoRoot, ["ls-remote", cfg.upstreamGpuiKit, "HEAD"]).stdout.split(/\s+/)[0] || "",
        agentBrowserSha:
          agentBrowserSha || runGit(repoRoot, ["ls-remote", cfg.upstreamAgentBrowser, "HEAD"]).stdout.split(/\s+/)[0] || "",
        agentDesktopSha:
          agentDesktopSha || runGit(repoRoot, ["ls-remote", cfg.upstreamAgentDesktop, "HEAD"]).stdout.split(/\s+/)[0] || "",
      });
    }

    console.log(JSON.stringify(summary, null, 2));
    process.exit(summary.ok ? 0 : 1);
  } else {
    console.error("Usage: bun tooling/bun/upstream-sync.ts [status|resolve|sync] [--target all|zed|nsh|gpui-kit|bun|agent-browser|agent-desktop] [--dry-run]");
    process.exit(1);
  }
}
