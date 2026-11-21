#!/usr/bin/env bun
/**
 * Audits and auto-merges all GitHub-side reference forks in `tooling/bun/forks-manifest.json`
 * (`referenceForks`) across the Multimedia, YouTube/YouTube Music, 3D WebGPU, and Google
 * official/community ecosystems (Rust, Go, TypeScript, Python, Kotlin/JVM) without adding
 * any local checkout (reference forks live on GitHub only).
 *
 * Usage:
 *   bun tooling/bun/multimedia-forks-audit.ts --check [--json]
 *   bun tooling/bun/multimedia-forks-audit.ts --sync [--json]
 */
import { readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

export type ReferenceForkDomain =
  | "media"
  | "google-rust"
  | "google-go"
  | "google-ts"
  | "google-python"
  | "google-kotlin";

export interface ReferenceForkEntry {
  repo: string;
  name: string;
  upstream: string;
  branch: string;
  domain: ReferenceForkDomain;
  migrationTarget: string;
  sha?: string;
  syncedAt?: string;
}

export interface ForksManifest {
  schemaVersion: number;
  releaseDate: string;
  ociPackage: string;
  forks: Array<Record<string, unknown>>;
  referenceForks?: ReferenceForkEntry[];
}

export const CANONICAL_REFERENCE_FORKS: ReferenceForkEntry[] = [
  // Pure-Rust multimedia, programmatic video, ingest, watch & 3D WebGPU
  { repo: "aphrody-labs/filmcraft", name: "filmcraft", upstream: "storytold/filmcraft", branch: "main", domain: "media", migrationTarget: "crates/aphrody_media" },
  { repo: "aphrody-labs/photocraft", name: "photocraft", upstream: "storytold/photocraft", branch: "main", domain: "media", migrationTarget: "crates/aphrody_media::photo_dev" },
  { repo: "aphrody-labs/lightcraft", name: "lightcraft", upstream: "storytold/lightcraft", branch: "main", domain: "media", migrationTarget: "crates/aphrody_media::photo_dev" },
  { repo: "aphrody-labs/effectcraft", name: "effectcraft", upstream: "storytold/effectcraft", branch: "main", domain: "media", migrationTarget: "crates/aphrody_media::motion" },
  { repo: "aphrody-labs/soundcraft", name: "soundcraft", upstream: "storytold/soundcraft", branch: "main", domain: "media", migrationTarget: "crates/aphrody_media::audio_out" },
  { repo: "aphrody-labs/yt-dlp", name: "yt-dlp", upstream: "yt-dlp/yt-dlp", branch: "master", domain: "media", migrationTarget: "crates/aphrody_media::ingest" },
  { repo: "aphrody-labs/remotion", name: "remotion", upstream: "remotion-dev/remotion", branch: "main", domain: "media", migrationTarget: "crates/aphrody_nsh" },
  { repo: "aphrody-labs/watch-skill", name: "watch-skill", upstream: "oxbshw/watch-skill", branch: "main", domain: "media", migrationTarget: "crates/aphrody_media::watch" },
  { repo: "aphrody-labs/three.js", name: "three.js", upstream: "mrdoob/three.js", branch: "dev", domain: "media", migrationTarget: "crates/bevy_gpui" },
  // Google Rust (official & community)
  { repo: "aphrody-labs/google-workspace-cli", name: "google-workspace-cli", upstream: "googleworkspace/cli", branch: "main", domain: "google-rust", migrationTarget: "crates/aphrody_google::workspace" },
  { repo: "aphrody-labs/google-cloud-rust", name: "google-cloud-rust", upstream: "googleapis/google-cloud-rust", branch: "main", domain: "google-rust", migrationTarget: "crates/aphrody_google" },
  { repo: "aphrody-labs/adk-rust", name: "adk-rust", upstream: "zavora-ai/adk-rust", branch: "main", domain: "google-rust", migrationTarget: "crates/aphrody_pilot" },
  { repo: "aphrody-labs/gemini-rust", name: "gemini-rust", upstream: "flachesis/gemini-rust", branch: "main", domain: "google-rust", migrationTarget: "crates/aphrody_google::ai" },
  { repo: "aphrody-labs/a2a-rs", name: "a2a-rs", upstream: "EmilLindfors/a2a-rs", branch: "master", domain: "google-rust", migrationTarget: "crates/aphrody_pilot" },
  { repo: "aphrody-labs/google-apis-rs", name: "google-apis-rs", upstream: "Byron/google-apis-rs", branch: "main", domain: "google-rust", migrationTarget: "crates/aphrody_google::youtube_data" },
  { repo: "aphrody-labs/youtui", name: "youtui", upstream: "nick42d/youtui", branch: "main", domain: "google-rust", migrationTarget: "crates/aphrody_google::ytmusic" },
  { repo: "aphrody-labs/omniget", name: "omniget", upstream: "tonhowtf/omniget", branch: "main", domain: "google-rust", migrationTarget: "crates/aphrody_media::ingest" },
  { repo: "aphrody-labs/libreget", name: "libreget", upstream: "OpenSelena/libreget", branch: "main", domain: "google-rust", migrationTarget: "crates/aphrody_media::ingest" },
  { repo: "aphrody-labs/sonora", name: "sonora", upstream: "sonorahq/sonora", branch: "main", domain: "google-rust", migrationTarget: "crates/aphrody_desktop" },
  // Google Go (official & community)
  { repo: "aphrody-labs/go-genai", name: "go-genai", upstream: "googleapis/go-genai", branch: "main", domain: "google-go", migrationTarget: "crates/aphrody_google::ai" },
  { repo: "aphrody-labs/adk-go", name: "adk-go", upstream: "google/adk-go", branch: "main", domain: "google-go", migrationTarget: "crates/aphrody_pilot" },
  { repo: "aphrody-labs/mcp-toolbox", name: "mcp-toolbox", upstream: "googleapis/mcp-toolbox", branch: "main", domain: "google-go", migrationTarget: "crates/aphrody_pilot" },
  { repo: "aphrody-labs/gogcli", name: "gogcli", upstream: "openclaw/gogcli", branch: "main", domain: "google-go", migrationTarget: "crates/aphrody_google::workspace" },
  { repo: "aphrody-labs/rclone", name: "rclone", upstream: "rclone/rclone", branch: "master", domain: "google-go", migrationTarget: "crates/aphrody_google::workspace" },
  { repo: "aphrody-labs/youtube", name: "youtube", upstream: "kkdai/youtube", branch: "master", domain: "google-go", migrationTarget: "crates/aphrody_google::innertube" },
  // Google TypeScript (official & community)
  { repo: "aphrody-labs/gemini-cli", name: "gemini-cli", upstream: "google-gemini/gemini-cli", branch: "main", domain: "google-ts", migrationTarget: "crates/aphrody_google::auth" },
  { repo: "aphrody-labs/js-genai", name: "js-genai", upstream: "googleapis/js-genai", branch: "main", domain: "google-ts", migrationTarget: "crates/aphrody_google::ai" },
  { repo: "aphrody-labs/adk-js", name: "adk-js", upstream: "google/adk-js", branch: "main", domain: "google-ts", migrationTarget: "crates/aphrody_nsh" },
  { repo: "aphrody-labs/a2ui", name: "a2ui", upstream: "a2ui-project/a2ui", branch: "main", domain: "google-ts", migrationTarget: "crates/aphrody_nsh" },
  { repo: "aphrody-labs/genkit", name: "genkit", upstream: "genkit-ai/genkit", branch: "main", domain: "google-ts", migrationTarget: "crates/aphrody_google::ai" },
  { repo: "aphrody-labs/live-api-web-console", name: "live-api-web-console", upstream: "google-gemini/live-api-web-console", branch: "main", domain: "google-ts", migrationTarget: "crates/aphrody_google::ai" },
  { repo: "aphrody-labs/stitch-sdk", name: "stitch-sdk", upstream: "google-labs-code/stitch-sdk", branch: "main", domain: "google-ts", migrationTarget: "crates/aphrody_design" },
  { repo: "aphrody-labs/YouTube.js", name: "YouTube.js", upstream: "LuanRT/YouTube.js", branch: "main", domain: "google-ts", migrationTarget: "crates/aphrody_google::innertube" },
  { repo: "aphrody-labs/pear-desktop", name: "pear-desktop", upstream: "pear-devs/pear-desktop", branch: "master", domain: "google-ts", migrationTarget: "crates/aphrody_desktop" },
  // Google Python (official & community)
  { repo: "aphrody-labs/python-genai", name: "python-genai", upstream: "googleapis/python-genai", branch: "main", domain: "google-python", migrationTarget: "crates/aphrody_google::ai" },
  { repo: "aphrody-labs/adk-python", name: "adk-python", upstream: "google/adk-python", branch: "main", domain: "google-python", migrationTarget: "crates/aphrody_pilot" },
  { repo: "aphrody-labs/A2A", name: "A2A", upstream: "a2aproject/A2A", branch: "main", domain: "google-python", migrationTarget: "crates/aphrody_pilot" },
  { repo: "aphrody-labs/gemini-skills", name: "gemini-skills", upstream: "google-gemini/gemini-skills", branch: "main", domain: "google-python", migrationTarget: ".agents/plugins/aphrody" },
  { repo: "aphrody-labs/computer-use-preview", name: "computer-use-preview", upstream: "google-gemini/computer-use-preview", branch: "main", domain: "google-python", migrationTarget: "crates/aphrody_pilot" },
  { repo: "aphrody-labs/ytmusicapi", name: "ytmusicapi", upstream: "sigma67/ytmusicapi", branch: "main", domain: "google-python", migrationTarget: "crates/aphrody_google::ytmusic" },
  { repo: "aphrody-labs/google_workspace_mcp", name: "google_workspace_mcp", upstream: "taylorwilsdon/google_workspace_mcp", branch: "main", domain: "google-python", migrationTarget: "crates/aphrody_pilot" },
  // Google Kotlin / JVM / Android Material 3 Expressive
  { repo: "aphrody-labs/java-genai", name: "java-genai", upstream: "googleapis/java-genai", branch: "main", domain: "google-kotlin", migrationTarget: "crates/aphrody_google::ai" },
  { repo: "aphrody-labs/adk-java", name: "adk-java", upstream: "google/adk-java", branch: "main", domain: "google-kotlin", migrationTarget: "crates/aphrody_pilot" },
  { repo: "aphrody-labs/material-components-android", name: "material-components-android", upstream: "material-components/material-components-android", branch: "master", domain: "google-kotlin", migrationTarget: "crates/aphrody_ui" },
];

export const repoRoot = resolve(import.meta.dir, "../..");
export const DEFAULT_MANIFEST_PATH = join(repoRoot, "tooling/bun/forks-manifest.json");

export function validateReferenceForks(entries: ReferenceForkEntry[]): string[] {
  const errors: string[] = [];
  const seenRepos = new Set<string>();
  const seenNames = new Set<string>();
  for (const entry of entries) {
    if (!entry.repo.startsWith("aphrody-labs/")) {
      errors.push(`Invalid fork org for ${entry.repo}`);
    }
    if (entry.repo === "aphrody-labs/cli") {
      errors.push("Forbidden collision with aphrody-labs/cli (GitHub CLI); use aphrody-labs/google-workspace-cli");
    }
    if (seenRepos.has(entry.repo)) errors.push(`Duplicate repo: ${entry.repo}`);
    if (seenNames.has(entry.name)) errors.push(`Duplicate name: ${entry.name}`);
    seenRepos.add(entry.repo);
    seenNames.add(entry.name);
    if (!entry.upstream.includes("/")) errors.push(`Invalid upstream: ${entry.upstream}`);
    if (!entry.migrationTarget.trim()) errors.push(`Missing migrationTarget for ${entry.repo}`);
  }
  return errors;
}

export function loadManifest(path = DEFAULT_MANIFEST_PATH): ForksManifest {
  return JSON.parse(readFileSync(path, "utf8")) as ForksManifest;
}

export function writeReferenceForksToManifest(
  entries: ReferenceForkEntry[],
  path = DEFAULT_MANIFEST_PATH,
): ForksManifest {
  const manifest = loadManifest(path);
  manifest.referenceForks = entries;
  writeFileSync(path, `${JSON.stringify(manifest, null, 2)}\n`);
  return manifest;
}

interface ForkSyncReport {
  repo: string;
  upstream: string;
  branch: string;
  domain: ReferenceForkDomain;
  ok: boolean;
  action: "checked" | "merged" | "error";
  sha?: string;
  detail: string;
}

async function runGh(args: string[]): Promise<{ code: number; stdout: string; stderr: string }> {
  const proc = Bun.spawn(["gh", ...args], { stdout: "pipe", stderr: "pipe" });
  const [stdout, stderr, code] = await Promise.all([
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
    proc.exited,
  ]);
  return { code, stdout: stdout.trim(), stderr: stderr.trim() };
}

export async function syncOrCheckReferenceForks(
  mode: "check" | "sync",
  manifestPath = DEFAULT_MANIFEST_PATH,
): Promise<{ ok: boolean; total: number; failures: number; reports: ForkSyncReport[] }> {
  const validationErrors = validateReferenceForks(CANONICAL_REFERENCE_FORKS);
  if (validationErrors.length > 0) {
    throw new Error(`Invalid canonical reference forks: ${validationErrors.join("; ")}`);
  }

  const nowIso = new Date().toISOString();
  const updated: ReferenceForkEntry[] = [];
  const reports: ForkSyncReport[] = [];

  for (const entry of CANONICAL_REFERENCE_FORKS) {
    if (mode === "sync") {
      const merged = await runGh([
        "api",
        "-X",
        "POST",
        `repos/${entry.repo}/merge-upstream`,
        "-f",
        `branch=${entry.branch}`,
        "--jq",
        ".message // .merge_type",
      ]);
      const shaRes = await runGh([
        "api",
        `repos/${entry.repo}/commits/${entry.branch}`,
        "--jq",
        ".sha",
      ]);
      const sha = shaRes.code === 0 ? shaRes.stdout : undefined;
      updated.push({ ...entry, sha, syncedAt: nowIso });
      reports.push({
        repo: entry.repo,
        upstream: entry.upstream,
        branch: entry.branch,
        domain: entry.domain,
        ok: merged.code === 0 && shaRes.code === 0,
        action: merged.code === 0 ? "merged" : "error",
        sha,
        detail: merged.code === 0 ? merged.stdout : merged.stderr,
      });
    } else {
      const shaRes = await runGh([
        "api",
        `repos/${entry.repo}/commits/${entry.branch}`,
        "--jq",
        ".sha",
      ]);
      const sha = shaRes.code === 0 ? shaRes.stdout : undefined;
      updated.push({ ...entry, sha, syncedAt: nowIso });
      reports.push({
        repo: entry.repo,
        upstream: entry.upstream,
        branch: entry.branch,
        domain: entry.domain,
        ok: shaRes.code === 0,
        action: shaRes.code === 0 ? "checked" : "error",
        sha,
        detail: shaRes.code === 0 ? `HEAD ${sha?.slice(0, 12)}` : shaRes.stderr,
      });
    }
  }

  if (mode === "sync") {
    writeReferenceForksToManifest(updated, manifestPath);
  }

  const failures = reports.filter((r) => !r.ok).length;
  return { ok: failures === 0, total: reports.length, failures, reports };
}

if (import.meta.main) {
  const args = new Set(Bun.argv.slice(2));
  const mode = args.has("--sync") ? "sync" : "check";
  const asJson = args.has("--json");
  const result = await syncOrCheckReferenceForks(mode);
  if (asJson) {
    console.log(JSON.stringify(result, null, 2));
  } else {
    console.log(
      `multimedia-forks-audit (${mode}): ${result.total - result.failures}/${result.total} OK`,
    );
    for (const r of result.reports) {
      if (!r.ok) {
        console.error(`  FAIL ${r.repo} (${r.branch}): ${r.detail}`);
      }
    }
  }
  if (!result.ok) {
    process.exit(1);
  }
}
