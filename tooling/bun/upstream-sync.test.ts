import { describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  applyAphrodyOnboardingTransforms,
  ensureReadmeReviewGuard,
  flattenModRsTree,
  mergeXtaskMainKeepingAphrodyCommands,
  normalizeGpuiKitManifest,
  normalizeGpuiKitSource,
  readProductUpstreamConfig,
  rewriteRelativeModRsPaths,
  writeUpstreamLock,
} from "./upstream-sync.ts";

describe("upstream-sync flattening & conflict resolution", () => {
  test("rewrites relative include_str/include_bytes/path literals when flattening mod.rs", () => {
    const src = `
const COLORS: &str = include_str!("default-colors.json");
const DATA: &[u8] = include_bytes!("./assets/icon.bin");
const PARENT: &str = include_str!("../../shared/font.ttf");
#[path = "sub_item.rs"]
mod sub_item;
`;
    const rewritten = rewriteRelativeModRsPaths(src, "theme");
    expect(rewritten).toContain('include_str!("theme/default-colors.json")');
    expect(rewritten).toContain('include_bytes!("theme/assets/icon.bin")');
    expect(rewritten).toContain('include_str!("../shared/font.ttf")');
    expect(rewritten).toContain('#[path = "theme/sub_item.rs"]');
  });

  test("flattenModRsTree converts nested mod.rs files into sibling .rs modules", () => {
    const dir = mkdtempSync(join(tmpdir(), "flatten-mod-test-"));
    try {
      mkdirSync(join(dir, "src", "theme", "nested"), { recursive: true });
      writeFileSync(join(dir, "src", "lib.rs"), "pub mod theme;\n");
      writeFileSync(join(dir, "src", "theme", "mod.rs"), 'pub mod nested;\nconst C: &str = include_str!("default.json");\n');
      writeFileSync(join(dir, "src", "theme", "nested", "mod.rs"), 'const N: &str = include_str!("../../root.txt");\n');

      const converted = flattenModRsTree(join(dir, "src"));
      expect(converted.sort()).toEqual(["theme.rs", "theme/nested.rs"]);
      expect(existsSync(join(dir, "src", "theme", "mod.rs"))).toBe(false);
      expect(existsSync(join(dir, "src", "theme", "nested", "mod.rs"))).toBe(false);
      expect(readFileSync(join(dir, "src", "theme.rs"), "utf8")).toContain('include_str!("theme/default.json")');
      expect(readFileSync(join(dir, "src", "theme", "nested.rs"), "utf8")).toContain('include_str!("../root.txt")');
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test("preserves README guard, onboarding branding, and xtask subcommands on conflict resolution", () => {
    const guarded = ensureReadmeReviewGuard("# Aphrody");
    expect(guarded.startsWith("> [!IMPORTANT]\n> Remove this line")).toBe(true);
    expect(ensureReadmeReviewGuard(guarded)).toBe(guarded);

    const onboard = applyAphrodyOnboardingTransforms(
      'Vector::square(VectorName::ZedLogo, rems(2.5)); Headline::new("Welcome to Zed"); Label::new("The editor for what\'s next");',
    );
    expect(onboard).toContain("VectorName::AphrodyLogo");
    expect(onboard).toContain("brand::APP_NAME");
    expect(onboard).toContain("brand::TAGLINE");

    const xtaskSrc = `
enum CliCommand {
    /// Checks that packages conform to a set of standards.
    PackageConformity(tasks::package_conformity::PackageConformityArgs),
    /// Inspect the GPUI publish plan and its dependencies.
    #[command(subcommand)]
    Gpui(tasks::gpui::GpuiCommand),
}
fn main() {
    match cmd {
        CliCommand::Gpui(args) => tasks::gpui::run_gpui(args),
    }
}`;
    const mergedXtask = mergeXtaskMainKeepingAphrodyCommands(xtaskSrc);
    expect(mergedXtask).toContain("NewApp(tasks::new_app::NewAppArgs)");
    expect(mergedXtask).toContain("Brand(tasks::brand::BrandCommand)");
    expect(mergedXtask).toContain("Studio(tasks::studio::StudioCommand)");
    expect(mergedXtask).toContain("CliCommand::Brand(cmd)");
  });

  test("normalizes gpui-kit Cargo.toml and strips gpui-fast extern shims onto unified gpui", () => {
    const toml = `[package]
name = "gpui-component"
version = "0.10.0"
publish = true
[features]
gpui-fast = ["dep:gpui-fast", "gpui-base/gpui-fast"]
[dependencies]
sum-tree.workspace = true
gpui-fast = { workspace = true, optional = true }
jiff = "1.0"
`;
    const norm = normalizeGpuiKitManifest(toml, "component");
    expect(norm).toContain("publish.workspace = true");
    expect(norm).toContain("sum_tree.workspace = true");
    expect(norm).toContain("[lints]\nworkspace = true");
    expect(norm).toContain("jiff.workspace = true");
    expect(norm).not.toContain("dep:gpui-fast");

    const rs = `//! GPUI Base\n#[cfg(feature = "gpui-fast")]\nextern crate gpui_fast as gpui;\nuse gpui::App;\npub mod input;\n`;
    const normRs = normalizeGpuiKitSource(rs, "src/lib.rs");
    expect(normRs).not.toContain("extern crate gpui_fast");
    expect(normRs).toContain("extern crate ropey_kit as ropey;");
  });

  test("reads product.toml upstream URLs including agent-browser and agent-desktop and writes lockfile", () => {
    const cfg = readProductUpstreamConfig();
    expect(cfg.upstreamAgentBrowser).toContain("aphrody-labs/agent-browser");
    expect(cfg.upstreamAgentDesktop).toContain("aphrody-labs/agent-desktop");

    const dir = mkdtempSync(join(tmpdir(), "upstream-lock-test-"));
    try {
      writeUpstreamLock(dir, {
        zedSha: "111111",
        nshSha: "222222",
        gpuiKitSha: "333333",
        agentBrowserSha: "444444",
        agentDesktopSha: "555555",
      });
      const lockText = readFileSync(join(dir, "assets", "os", "upstream_sync_lock.toml"), "utf8");
      expect(lockText).toContain('agent_browser_sha = "444444"');
      expect(lockText).toContain('agent_desktop_sha = "555555"');
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});
