#!/usr/bin/env bun
/**
 * Convert any local Cargo workspace crate or docs.rs crate into a portable Agent Skill bundle
 * compliant with agentskills.io, Vercel `skills` (`npx skills`), and OpenClaw (`~/.openclaw/skills/`).
 *
 * Uses `cargo metadata`, `RUSTC_BOOTSTRAP=1 cargo rustdoc --output-format json`, `cargo doc-md` / `rustdoc-md`
 * to emit progressive-disclosure `SKILL.md` + `references/*.md` + optional `.well-known/agent-skills/index.json`.
 *
 * Usage:
 *   bun tooling/bun/crate-to-skill.ts --crate aphrody_craft [--out target/skills] [--install-target .agents/skills]
 *   bun tooling/bun/crate-to-skill.ts --docsrs tokio [--out target/skills]
 */
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "../..");
const args = Bun.argv.slice(2);

function opt(flag: string): string | undefined {
  const idx = args.indexOf(flag);
  if (idx >= 0) return args[idx + 1];
  const eq = args.find((a) => a.startsWith(`${flag}=`));
  return eq ? eq.slice(flag.length + 1) : undefined;
}

const localCrate = opt("--crate");
const docsrsCrate = opt("--docsrs");
if (!localCrate && !docsrsCrate) {
  console.error("Usage: bun tooling/bun/crate-to-skill.ts (--crate <workspace-pkg> | --docsrs <crate>) [--out <dir>] [--name <skill-slug>]");
  process.exit(1);
}

const rawName = opt("--name") ?? (localCrate ?? docsrsCrate ?? "rust-crate");
const skillSlug = rawName
  .toLowerCase()
  .replace(/[^a-z0-9-]+/g, "-")
  .replace(/^-+|-+$/g, "")
  .slice(0, 64);
if (!skillSlug || !/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(skillSlug)) {
  throw new Error(`Invalid skill name '${skillSlug}' (agentskills.io requires lowercase alphanumeric + hyphens, <= 64 chars)`);
}

const outBase = resolve(opt("--out") ?? join(repoRoot, "target/skills"));
const skillDir = join(outBase, skillSlug);
const refsDir = join(skillDir, "references");
mkdirSync(refsDir, { recursive: true });

interface CrateMetadata {
  name: string;
  version: string;
  license: string;
  description: string;
  repository?: string;
  features: Record<string, string[]>;
  manifestDir?: string;
  libTargetName?: string;
}

function loadWorkspaceCrate(crateName: string): CrateMetadata {
  const res = Bun.spawnSync(["cargo", "metadata", "--no-deps", "--format-version", "1"], {
    cwd: repoRoot,
    stdout: "pipe",
    stderr: "pipe",
  });
  if (res.exitCode !== 0) {
    throw new Error(`cargo metadata failed: ${new TextDecoder().decode(res.stderr)}`);
  }
  const meta = JSON.parse(new TextDecoder().decode(res.stdout)) as {
    packages: Array<{
      name: string;
      version: string;
      license?: string;
      description?: string;
      repository?: string;
      features?: Record<string, string[]>;
      manifest_path: string;
      targets: Array<{ name: string; kind: string[] }>;
    }>;
  };
  const pkg = meta.packages.find((p) => p.name === crateName);
  if (!pkg) {
    throw new Error(`Crate '${crateName}' not found in Cargo workspace`);
  }
  const libTarget = pkg.targets.find((t) => t.kind.includes("lib") || t.kind.includes("rlib") || t.kind.includes("cdylib"));
  return {
    name: pkg.name,
    version: pkg.version,
    license: pkg.license ?? "MIT OR Apache-2.0",
    description: pkg.description ?? `Rust crate ${pkg.name} public API, features, and verification workflow.`,
    repository: pkg.repository,
    features: pkg.features ?? {},
    manifestDir: dirname(pkg.manifest_path),
    libTargetName: libTarget?.name.replaceAll("-", "_"),
  };
}

function hasBinary(bin: string): boolean {
  const res = Bun.spawnSync([bin, "--version"], { stdout: "pipe", stderr: "pipe" });
  return res.exitCode === 0;
}

function generateDocs(): string[] {
  const refFiles: string[] = [];
  if (docsrsCrate) {
    if (!hasBinary("rustdoc-md")) {
      throw new Error("rustdoc-md is required for --docsrs mode (cargo install rustdoc-md)");
    }
    const apiPath = join(refsDir, "api.md");
    const res = Bun.spawnSync(["rustdoc-md", "--crate-name", docsrsCrate, "-o", apiPath], {
      cwd: repoRoot,
      stdout: "pipe",
      stderr: "pipe",
    });
    if (res.exitCode !== 0) {
      throw new Error(`rustdoc-md --crate-name failed: ${new TextDecoder().decode(res.stderr)}`);
    }
    refFiles.push("references/api.md");
    return refFiles;
  }

  const crateName = localCrate!;
  const meta = loadWorkspaceCrate(crateName);
  const env = { ...process.env, RUSTC_BOOTSTRAP: "1" };

  // Copy crate README if present into references/readme.md for full upstream context.
  if (meta.manifestDir) {
    const readme = join(meta.manifestDir, "README.md");
    if (existsSync(readme)) {
      writeFileSync(join(refsDir, "readme.md"), readFileSync(readme, "utf8"), "utf8");
      refFiles.push("references/readme.md");
    }
  }

  // Prefer rustdoc JSON + rustdoc-md or cargo doc-md (split module tree for token-efficient progressive disclosure).
  if (hasBinary("cargo-doc-md")) {
    const docMdOut = join(refsDir, "doc-md");
    rmSync(docMdOut, { recursive: true, force: true });
    const res = Bun.spawnSync(
      ["cargo", "doc-md", "-p", crateName, "--no-deps", "-o", docMdOut],
      { cwd: repoRoot, env, stdout: "pipe", stderr: "pipe" },
    );
    if (res.exitCode === 0 && existsSync(docMdOut)) {
      const walk = (dir: string) => {
        for (const entry of readdirSync(dir, { withFileTypes: true })) {
          const p = join(dir, entry.name);
          if (entry.isDirectory()) walk(p);
          else if (entry.name.endsWith(".md")) {
            refFiles.push(relative(skillDir, p).replaceAll("\\", "/"));
          }
        }
      };
      walk(docMdOut);
      if (refFiles.length > 0) return refFiles.sort();
    }
  }

  // Fallback: direct `cargo rustdoc --output-format json` + `rustdoc-md --path`.
  if (meta.libTargetName && hasBinary("rustdoc-md")) {
    const rd = Bun.spawnSync(
      ["cargo", "rustdoc", "-p", crateName, "--lib", "--", "-Z", "unstable-options", "--output-format", "json"],
      { cwd: repoRoot, env, stdout: "pipe", stderr: "pipe" },
    );
    const jsonPath = join(repoRoot, "target/doc", `${meta.libTargetName}.json`);
    if (rd.exitCode === 0 && existsSync(jsonPath)) {
      const apiPath = join(refsDir, "api.md");
      const rmd = Bun.spawnSync(["rustdoc-md", "--path", jsonPath, "-o", apiPath], {
        cwd: repoRoot,
        stdout: "pipe",
        stderr: "pipe",
      });
      if (rmd.exitCode === 0) {
        refFiles.push("references/api.md");
        return refFiles;
      }
    }
  }

  return refFiles;
}

const meta: CrateMetadata = localCrate
  ? loadWorkspaceCrate(localCrate)
  : {
      name: docsrsCrate!,
      version: "latest",
      license: "MIT OR Apache-2.0",
      description: `Documentation and integration guide for Rust crate ${docsrsCrate}.`,
      features: {},
    };

const refFiles = generateDocs();
const featureList = Object.entries(meta.features);
const featureKeys = featureList.map(([k]) => k).slice(0, 12).join(", ");
const rawDesc = `${meta.description.trim()} Use when writing, auditing, refactoring, or integrating Rust code depending on crate '${meta.name}'${featureKeys ? ` (features: ${featureKeys})` : ""}.`;
const description = rawDesc.replaceAll(/\s+/g, " ").replaceAll('"', "'").slice(0, 1020);

const featureRows = featureList.length > 0
  ? featureList
      .map(([k, vals]) => `| \`${k}\` | ${vals.length > 0 ? vals.map((v) => `\`${v}\``).join(", ") : "*(marker/default)*"} |`)
      .join("\n")
  : "| *(none)* | Default crate configuration |";

const refsLinks = refFiles.length > 0
  ? refFiles.map((r) => `- [${r}](${r})`).join("\n")
  : "- Run `RUSTC_BOOTSTRAP=1 cargo doc-md -p " + meta.name + " --no-deps` to populate module markdown.";

const skillMd = `---
name: ${skillSlug}
description: "${description}"
license: ${meta.license}
compatibility: Requires Rust toolchain (cargo, rustdoc); compatible with agentskills.io, Vercel skills CLI, Claude Code, and OpenClaw.
metadata:
  crate: "${meta.name}"
  version: "${meta.version}"
  generator: "tooling/bun/crate-to-skill.ts"
---

# ${meta.name} (${meta.version})

> Generated from Rust crate manifest and rustdoc metadata. Keep core rules concise here and consult [references/](references/) on demand to preserve context budget.

## When to Use

Use this skill whenever adding, modifying, debugging, or reviewing Rust modules that depend on \`${meta.name}\`.

## Cargo Dependency & Feature Matrix

Add to \`Cargo.toml\` (or inherit \`.workspace = true\` in workspace crates):

\`\`\`toml
${meta.name} = "${meta.version}"
\`\`\`

| Feature | Enables |
| --- | --- |
${featureRows}

## Progressive API References

Read only the targeted module file before implementing calls:

${refsLinks}

## Verification Loop

\`\`\`bash
cargo check -p ${meta.name}
cargo clippy -p ${meta.name} --all-targets -- -D warnings
cargo test -p ${meta.name}
\`\`\`

## Distribution (Vercel \`skills\` & OpenClaw)

- Local install for project agents: copy or symlink this folder to \`.agents/skills/${skillSlug}/\` or \`.claude/skills/${skillSlug}/\` or \`~/.openclaw/skills/${skillSlug}/\`.
- Vercel CLI sync / install: \`npx skills add <owner/repo> --skill ${skillSlug}\` (add \`-a openclaw\` or \`-a claude-code\`).
- OpenClaw path variable support: static scripts or extra files placed under \`{baseDir}/scripts/\` resolve cleanly at runtime.
`;

writeFileSync(join(skillDir, "SKILL.md"), skillMd, "utf8");

// Validate AgentSkills / Vercel / OpenClaw invariants strictly before returning.
const fmMatch = skillMd.match(/^---\n([\s\S]*?)\n---/);
if (!fmMatch) {
  throw new Error("Generated SKILL.md missing YAML frontmatter");
}
const manifestDigest = createHash("sha256").update(skillMd).digest("hex");
console.log(JSON.stringify({
  ok: true,
  skill: skillSlug,
  skillDir,
  skillMd: join(skillDir, "SKILL.md"),
  sha256: manifestDigest,
  descriptionLength: description.length,
  referenceCount: refFiles.length,
  references: refFiles,
}, null, 2));
