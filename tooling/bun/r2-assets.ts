#!/usr/bin/env bun
/**
 * Content-Addressed Cloudflare R2 Asset Offloader & On-Demand Hydrator.
 * Migrates heavy non-boot binary assets (PNG reference captures, GIFs, Material 3 Zip,
 * gallery renders, audio WAVs) from local Git checkout to `aphrody-assets` R2 bucket
 * served zero-egress via `https://assets.aphrody.app/v1/<category>/<sha256>.<ext>`
 * (and unified worker `/r2-assets/*`).
 *
 * Usage:
 *   bun tooling/bun/r2-assets.ts manifest [--json]
 *   bun tooling/bun/r2-assets.ts push [--dry-run] [--json]
 *   bun tooling/bun/r2-assets.ts hydrate [--filter <substr>] [--json]
 *   bun tooling/bun/r2-assets.ts prune-local [--dry-run] [--json]
 */
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { dirname, extname, join, relative, resolve, sep } from "node:path";
import { resolveWranglerCmd } from "./cloudflare-deploy.ts";

export const repoRoot = resolve(import.meta.dir, "../..");
export const R2_BUCKET_NAME = "aphrody-assets";
export const R2_PUBLIC_BASE_URL = "https://assets.aphrody.app";
export const R2_MANIFEST_REL = "assets/os/r2_assets_manifest.toml";
export const R2_CACHE_REL = "target/r2-cache";

export interface R2AssetEntry {
  logicalPath: string;
  category: string;
  r2Key: string;
  publicUrl: string;
  sha256: string;
  bytes: number;
  contentType: string;
  keepLocalBoot: boolean;
}

export interface R2AssetManifest {
  schemaVersion: number;
  bucket: string;
  publicBaseUrl: string;
  generatedAt: string;
  totalAssets: number;
  offloadedBytes: number;
  keptBootBytes: number;
  assets: R2AssetEntry[];
}

const MIME_BY_EXT: Record<string, string> = {
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".webp": "image/webp",
  ".gif": "image/gif",
  ".svg": "image/svg+xml; charset=utf-8",
  ".ico": "image/x-icon",
  ".icns": "image/icns",
  ".ttf": "font/ttf",
  ".otf": "font/otf",
  ".woff": "font/woff",
  ".woff2": "font/woff2",
  ".wav": "audio/wav",
  ".mp3": "audio/mpeg",
  ".ogg": "audio/ogg",
  ".mp4": "video/mp4",
  ".zip": "application/zip",
};

const ASSET_ROOTS = [
  "assets/sounds",
  "assets/fonts",
  "assets/images",
  "docs/aphrody/design",
  "crates/gpui/examples/image",
  "examples/gpu/webgpu",
  "examples/nsh/gallery",
  "examples/nsh/gallery-rs",
];

const MEDIA_EXT_RE = /\.(png|jpe?g|webp|gif|wav|mp3|ogg|mp4|zip|ttf|otf|woff2?)$/i;

/**
 * Assets required at compile time (`include_bytes!`, `util::fs_embed!` icon/logo checks,
 * `cargo xtask brand check`, or `Assets::load_test_fonts`) MUST remain on disk (`keepLocalBoot = true`).
 * Runtime showcase PNGs, GIFs, reference ZIPs, and optional audio assets are offloaded to R2.
 */
export function isKeepLocalBoot(relPath: string): boolean {
  const p = relPath.replaceAll("\\", "/");
  // Compile-time include_bytes! fonts across gpui, gpui_wgpu, mermaid_render, and nsh-core tests:
  if (p.startsWith("assets/fonts/lilex/")) return true;
  if (p.startsWith("assets/fonts/ibm-plex-sans/")) return true;
  if (p === "examples/nsh/gallery/fonts/Pacifico-Regular.ttf") return true;
  if (p === "assets/fonts/google-sans-code/GoogleSansCode-Variable.ttf") return true;
  if (p === "assets/fonts/jetbrains-mono/JetBrainsMono-Regular.ttf") return true;
  // Brand & app icon files required by cargo xtask brand check & gpui examples:
  if (p.startsWith("assets/images/")) return true;
  if (p === "crates/gpui/examples/image/app-icon.png") return true;
  if (p === "examples/nsh/gallery/nsh.json" || p === "examples/nsh/gallery-rs/nsh.json") return true;
  // Exact compile-time `asset!()` usage inside gallery-rs/native:
  if (p.startsWith("examples/nsh/gallery-rs/native/assets/")) return true;
  return false;
}

export function categorizeAsset(relPath: string): string {
  const p = relPath.replaceAll("\\", "/");
  if (p.startsWith("docs/aphrody/design/") || p.startsWith("docs/design/")) return "design-reference";
  if (p.startsWith("assets/sounds/")) return "audio-sfx";
  if (p.startsWith("assets/fonts/")) return "fonts";
  if (p.startsWith("assets/images/")) return "brand-images";
  if (p.startsWith("crates/gpui/examples/")) return "gpui-examples";
  if (p.startsWith("examples/gpu/")) return "gpu-renders";
  if (p.startsWith("examples/nsh/")) return "nsh-examples";
  return "misc";
}

function collectCandidateFiles(root: string): string[] {
  const results: string[] = [];
  for (const sub of ASSET_ROOTS) {
    const abs = join(root, sub);
    if (!existsSync(abs)) continue;
    const stack = [abs];
    while (stack.length > 0) {
      const cur = stack.pop();
      if (!cur) continue;
      for (const entry of readdirSync(cur, { withFileTypes: true })) {
        if (entry.name === "target" || entry.name === "node_modules") continue;
        const full = join(cur, entry.name);
        if (entry.isDirectory()) {
          stack.push(full);
        } else if (MEDIA_EXT_RE.test(entry.name)) {
          results.push(relative(root, full).split(sep).join("/"));
        }
      }
    }
  }
  results.sort();
  return results;
}

export function loadExistingManifest(root = repoRoot): R2AssetManifest | null {
  const path = join(root, R2_MANIFEST_REL);
  if (!existsSync(path)) return null;
  const parsed = Bun.TOML.parse(readFileSync(path, "utf8")) as {
    schema_version?: number;
    bucket?: string;
    public_base_url?: string;
    generated_at?: string;
    offloaded_bytes?: number;
    kept_boot_bytes?: number;
    assets?: Array<{
      logical_path: string;
      category: string;
      r2_key: string;
      public_url: string;
      sha256: string;
      bytes: number;
      content_type: string;
      keep_local_boot: boolean;
    }>;
  };
  const assets: R2AssetEntry[] = (parsed.assets ?? []).map((a) => ({
    logicalPath: a.logical_path,
    category: a.category,
    r2Key: a.r2_key,
    publicUrl: a.public_url,
    sha256: a.sha256,
    bytes: a.bytes,
    contentType: a.content_type,
    keepLocalBoot: a.keep_local_boot,
  }));
  return {
    schemaVersion: parsed.schema_version ?? 1,
    bucket: parsed.bucket ?? R2_BUCKET_NAME,
    publicBaseUrl: parsed.public_base_url ?? R2_PUBLIC_BASE_URL,
    generatedAt: parsed.generated_at ?? "",
    totalAssets: assets.length,
    offloadedBytes: parsed.offloaded_bytes ?? 0,
    keptBootBytes: parsed.kept_boot_bytes ?? 0,
    assets,
  };
}

export function buildR2Manifest(root = repoRoot): R2AssetManifest {
  const existing = loadExistingManifest(root);
  const existingByPath = new Map<string, R2AssetEntry>();
  for (const item of existing?.assets ?? []) {
    existingByPath.set(item.logicalPath, item);
  }

  const diskPaths = collectCandidateFiles(root);
  for (const rel of diskPaths) {
    const abs = join(root, rel);
    const buf = readFileSync(abs);
    const sha256 = createHash("sha256").update(buf).digest("hex");
    const ext = extname(rel).toLowerCase();
    const category = categorizeAsset(rel);
    const r2Key = `v1/${category}/${sha256}${ext}`;
    const keepLocalBoot = isKeepLocalBoot(rel);
    existingByPath.set(rel, {
      logicalPath: rel,
      category,
      r2Key,
      publicUrl: `${R2_PUBLIC_BASE_URL}/${r2Key}`,
      sha256,
      bytes: statSync(abs).size,
      contentType: MIME_BY_EXT[ext] ?? "application/octet-stream",
      keepLocalBoot,
    });
  }

  // Re-evaluate keepLocalBoot policy on previously offloaded entries
  const assets = [...existingByPath.values()]
    .map((a) => ({
      ...a,
      keepLocalBoot: isKeepLocalBoot(a.logicalPath),
    }))
    .sort((a, b) => a.logicalPath.localeCompare(b.logicalPath));

  const offloadedBytes = assets.filter((a) => !a.keepLocalBoot).reduce((acc, a) => acc + a.bytes, 0);
  const keptBootBytes = assets.filter((a) => a.keepLocalBoot).reduce((acc, a) => acc + a.bytes, 0);

  return {
    schemaVersion: 1,
    bucket: R2_BUCKET_NAME,
    publicBaseUrl: R2_PUBLIC_BASE_URL,
    generatedAt: new Date().toISOString(),
    totalAssets: assets.length,
    offloadedBytes,
    keptBootBytes,
    assets,
  };
}

export function serializeR2ManifestToml(manifest: R2AssetManifest): string {
  const lines = [
    "# Content-Addressed Cloudflare R2 Asset Manifest (bucket: aphrody-assets).",
    "# Generated by tooling/bun/r2-assets.ts manifest — do not hand-edit.",
    `schema_version = ${manifest.schemaVersion}`,
    `bucket = ${JSON.stringify(manifest.bucket)}`,
    `public_base_url = ${JSON.stringify(manifest.publicBaseUrl)}`,
    `generated_at = ${JSON.stringify(manifest.generatedAt)}`,
    `total_assets = ${manifest.totalAssets}`,
    `offloaded_bytes = ${manifest.offloadedBytes}`,
    `kept_boot_bytes = ${manifest.keptBootBytes}`,
    "",
  ];
  for (const asset of manifest.assets) {
    lines.push("[[assets]]");
    lines.push(`logical_path = ${JSON.stringify(asset.logicalPath)}`);
    lines.push(`category = ${JSON.stringify(asset.category)}`);
    lines.push(`r2_key = ${JSON.stringify(asset.r2Key)}`);
    lines.push(`public_url = ${JSON.stringify(asset.publicUrl)}`);
    lines.push(`sha256 = ${JSON.stringify(asset.sha256)}`);
    lines.push(`bytes = ${asset.bytes}`);
    lines.push(`content_type = ${JSON.stringify(asset.contentType)}`);
    lines.push(`keep_local_boot = ${asset.keepLocalBoot}`);
    lines.push("");
  }
  return lines.join("\n");
}

export function writeR2Manifest(root = repoRoot): R2AssetManifest {
  const manifest = buildR2Manifest(root);
  const outPath = join(root, R2_MANIFEST_REL);
  mkdirSync(dirname(outPath), { recursive: true });
  writeFileSync(outPath, serializeR2ManifestToml(manifest), "utf8");
  return manifest;
}

export function pushAssetsToR2(
  root = repoRoot,
  dryRun = false,
  onlyOffloadable = false,
): { ok: boolean; uploaded: string[]; skippedMissing: string[]; dryRun: boolean; errors: string[] } {
  const manifest = writeR2Manifest(root);
  const wrangler = resolveWranglerCmd();
  const uploaded: string[] = [];
  const skippedMissing: string[] = [];
  const errors: string[] = [];
  const uploadedShas = new Set<string>();

  for (const entry of manifest.assets) {
    if (onlyOffloadable && entry.keepLocalBoot) continue;
    const absFile = join(root, entry.logicalPath);
    if (!existsSync(absFile)) {
      skippedMissing.push(entry.logicalPath);
      continue;
    }
    // Deduplicate identical SHA-256 content across duplicate local paths
    if (uploadedShas.has(entry.r2Key)) {
      uploaded.push(entry.r2Key);
      continue;
    }
    uploadedShas.add(entry.r2Key);
    if (!dryRun) {
      const target = `${manifest.bucket}/${entry.r2Key}`;
      const res = Bun.spawnSync(
        [
          ...wrangler,
          "r2",
          "object",
          "put",
          target,
          "--file",
          absFile,
          "--content-type",
          entry.contentType,
          "--cache-control",
          "public, max-age=31536000, immutable",
          "--remote",
        ],
        { cwd: root, stdout: "pipe", stderr: "pipe" },
      );
      if (res.exitCode !== 0) {
        errors.push(`${target}: ${new TextDecoder().decode(res.stderr).trim().slice(0, 240)}`);
        continue;
      }
    }
    uploaded.push(entry.r2Key);
  }

  return { ok: errors.length === 0, uploaded, skippedMissing, dryRun, errors };
}

export function pruneLocalOffloadedAssets(
  root = repoRoot,
  dryRun = false,
): { ok: boolean; prunedPaths: string[]; reclaimedBytes: number; dryRun: boolean } {
  const manifest = writeR2Manifest(root);
  const prunedPaths: string[] = [];
  let reclaimedBytes = 0;

  for (const entry of manifest.assets) {
    if (entry.keepLocalBoot) continue;
    const abs = join(root, entry.logicalPath);
    if (!existsSync(abs)) continue;
    reclaimedBytes += entry.bytes;
    prunedPaths.push(entry.logicalPath);
    if (!dryRun) {
      rmSync(abs, { force: true });
    }
  }

  return { ok: true, prunedPaths, reclaimedBytes, dryRun };
}

export async function hydrateFromR2(
  root = repoRoot,
  filter?: string,
  intoWorkingTree = false,
): Promise<{ ok: boolean; hydrated: string[]; skipped: string[]; errors: string[] }> {
  const manifest = loadExistingManifest(root) ?? writeR2Manifest(root);
  const hydrated: string[] = [];
  const skipped: string[] = [];
  const errors: string[] = [];

  for (const entry of manifest.assets) {
    if (entry.keepLocalBoot) continue;
    if (filter && !entry.logicalPath.includes(filter)) continue;
    const dest = intoWorkingTree
      ? join(root, entry.logicalPath)
      : join(root, R2_CACHE_REL, entry.logicalPath);
    if (existsSync(dest)) {
      const existingSha = createHash("sha256").update(readFileSync(dest)).digest("hex");
      if (existingSha === entry.sha256) {
        skipped.push(entry.logicalPath);
        continue;
      }
    }
    try {
      const resp = await fetch(entry.publicUrl, { signal: AbortSignal.timeout(20_000) });
      if (!resp.ok) {
        errors.push(`${entry.logicalPath}: HTTP ${resp.status} from ${entry.publicUrl}`);
        continue;
      }
      const bytes = new Uint8Array(await resp.arrayBuffer());
      const actualSha = createHash("sha256").update(bytes).digest("hex");
      if (actualSha !== entry.sha256) {
        errors.push(`${entry.logicalPath}: SHA-256 mismatch (expected ${entry.sha256}, got ${actualSha})`);
        continue;
      }
      mkdirSync(dirname(dest), { recursive: true });
      writeFileSync(dest, bytes);
      hydrated.push(entry.logicalPath);
    } catch (err) {
      errors.push(`${entry.logicalPath}: ${String(err)}`);
    }
  }
  return { ok: errors.length === 0, hydrated, skipped, errors };
}

if (import.meta.main) {
  const args = Bun.argv.slice(2);
  const cmd = args[0] ?? "manifest";
  const dryRun = args.includes("--dry-run");
  const filterIdx = args.indexOf("--filter");
  const filter = filterIdx >= 0 ? args[filterIdx + 1] : undefined;

  if (cmd === "manifest") {
    const manifest = writeR2Manifest(repoRoot);
    console.log(JSON.stringify({
      ok: true,
      manifestPath: R2_MANIFEST_REL,
      totalAssets: manifest.totalAssets,
      offloadableCount: manifest.assets.filter((a) => !a.keepLocalBoot).length,
      offloadedMb: Number((manifest.offloadedBytes / (1024 * 1024)).toFixed(2)),
      keptBootMb: Number((manifest.keptBootBytes / (1024 * 1024)).toFixed(2)),
    }, null, 2));
  } else if (cmd === "push") {
    const res = pushAssetsToR2(repoRoot, dryRun, args.includes("--offload-only"));
    console.log(JSON.stringify(res, null, 2));
    process.exit(res.ok ? 0 : 1);
  } else if (cmd === "hydrate") {
    const res = await hydrateFromR2(repoRoot, filter, args.includes("--working-tree"));
    console.log(JSON.stringify(res, null, 2));
    process.exit(res.ok ? 0 : 1);
  } else if (cmd === "prune-local") {
    const res = pruneLocalOffloadedAssets(repoRoot, dryRun);
    console.log(JSON.stringify(res, null, 2));
  } else {
    console.error("Usage: bun tooling/bun/r2-assets.ts [manifest|push|hydrate|prune-local] [--dry-run] [--filter <substr>]");
    process.exit(1);
  }
}
