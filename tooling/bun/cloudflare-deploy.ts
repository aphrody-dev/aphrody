#!/usr/bin/env bun
/**
 * Reproducible Cloudflare Workers, R2 release bucket, DNS route, MCP bridge, and edge verification CLI.
 * Uses installed `wrangler` CLI, `CLOUDFLARE_API_TOKEN` / `CLOUDFLARE_ACCOUNT_ID` env vars, and Streamable HTTP MCP.
 *
 * Usage:
 *   bun tooling/bun/cloudflare-deploy.ts audit [--json]
 *   bun tooling/bun/cloudflare-deploy.ts ensure-infra [--json]
 *   bun tooling/bun/cloudflare-deploy.ts deploy [--dry-run] [--all] [--json]
 *   bun tooling/bun/cloudflare-deploy.ts publish-scripts [--dry-run]
 *   bun tooling/bun/cloudflare-deploy.ts smoke [--base-url https://releases.aphrody.com] [--json]
 *   bun tooling/bun/cloudflare-deploy.ts mcp-stdio
 */
import { existsSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { extname, join, relative, resolve } from "node:path";
import { brotliCompressSync, constants, gzipSync } from "node:zlib";

export const repoRoot = resolve(import.meta.dir, "../..");

export function resolveWranglerCmd(): string[] {
  const bunBin = join(homedir(), ".bun", "bin");
  for (const candidate of [
    Bun.which("wrangler"),
    join(bunBin, process.platform === "win32" ? "wrangler.exe" : "wrangler"),
    Bun.which("cf-wrangler"),
    join(bunBin, process.platform === "win32" ? "cf-wrangler.exe" : "cf-wrangler"),
  ]) {
    if (candidate && existsSync(candidate)) return [candidate];
  }
  return ["bun", "x", "wrangler"];
}

export interface WorkerManifestAudit {
  name: string;
  dir: string;
  configPath: string;
  bucketNames: string[];
  routes: string[];
  hasLegacyZedDomain: boolean;
  dryRunOk?: boolean;
}

export interface CloudflareAuditReport {
  ok: boolean;
  accountId: string | null;
  releaseDomain: string;
  installScriptsPresent: boolean;
  workers: WorkerManifestAudit[];
  errors: string[];
}

export function readProductUrls(root = repoRoot): {
  releasesUrl: string;
  docsUrl: string;
  downloadUrl: string;
  slug: string;
} {
  const pm = Bun.TOML.parse(readFileSync(join(root, "product.toml"), "utf8")) as {
    product?: { slug?: string };
    urls?: { releases?: string; docs?: string; download?: string };
  };
  return {
    slug: pm.product?.slug ?? "aphrody",
    releasesUrl: pm.urls?.releases ?? "https://releases.aphrody.com",
    docsUrl: pm.urls?.docs ?? "https://docs.aphrody.com",
    downloadUrl: pm.urls?.download ?? "https://aphrody.com/download",
  };
}

export function auditCloudflareRepoConfig(root = repoRoot, runDryRun = false): CloudflareAuditReport {
  const urls = readProductUrls(root);
  const errors: string[] = [];
  const workerDirs = [".cloudflare/aphrody-worker"];
  const workers: WorkerManifestAudit[] = [];

  for (const rel of workerDirs) {
    const absDir = join(root, rel);
    const configPath = join(absDir, "wrangler.toml");
    if (!existsSync(configPath)) {
      errors.push(`Missing wrangler.toml: ${rel}`);
      continue;
    }
    const raw = readFileSync(configPath, "utf8");
    const parsed = Bun.TOML.parse(raw) as {
      name?: string;
      routes?: Array<{ pattern?: string; zone_name?: string }>;
      r2_buckets?: Array<{ bucket_name?: string }>;
    };
    const routes = (parsed.routes ?? []).map((r) => r.pattern ?? "").filter(Boolean);
    const bucketNames = (parsed.r2_buckets ?? []).map((b) => b.bucket_name ?? "").filter(Boolean);
    const hasLegacyZedDomain = /\bzed\.dev\b|zed-open-source-website-assets/.test(raw);
    if (hasLegacyZedDomain) {
      errors.push(`${rel}/wrangler.toml still references legacy zed.dev / zed-open-source-website-assets`);
    }
    for (const expectedBucket of ["aphrody-releases", "aphrody-assets"]) {
      if (!bucketNames.includes(expectedBucket)) {
        errors.push(`${rel}/wrangler.toml missing consolidated R2 bucket binding: ${expectedBucket}`);
      }
    }

    let dryRunOk: boolean | undefined;
    if (runDryRun) {
      const dry = Bun.spawnSync(
        [
          ...resolveWranglerCmd(),
          "deploy",
          "--env=",
          "--dry-run",
          "--outdir",
          join(root, "target", "wrangler-dry", parsed.name ?? "worker"),
        ],
        { cwd: absDir, stdout: "pipe", stderr: "pipe" },
      );
      dryRunOk = dry.exitCode === 0;
      if (!dryRunOk) {
        errors.push(`wrangler dry-run failed in ${rel}: ${new TextDecoder().decode(dry.stderr).slice(0, 300)}`);
      }
    }

    workers.push({
      name: parsed.name ?? rel,
      dir: rel,
      configPath: `${rel}/wrangler.toml`,
      bucketNames,
      routes,
      hasLegacyZedDomain,
      dryRunOk,
    });
  }

  for (const legacyDir of [
    ".cloudflare/aphrody-releases",
    ".cloudflare/docs-proxy",
    ".cloudflare/open-source-website-assets",
    ".cloudflare/aphrody-github",
  ]) {
    if (existsSync(join(root, legacyDir))) {
      errors.push(`Legacy isolated worker directory still present: ${legacyDir}`);
    }
  }

  const installSh = join(root, "script", "aphrody-install.sh");
  const installPs1 = join(root, "script", "aphrody-install.ps1");
  const installScriptsPresent = existsSync(installSh) && existsSync(installPs1);
  if (!installScriptsPresent) {
    errors.push("Missing script/aphrody-install.sh or script/aphrody-install.ps1");
  }

  const topologyPath = join(root, "assets", "os", "cloudflare_topology.toml");
  if (!existsSync(topologyPath)) {
    errors.push("Missing assets/os/cloudflare_topology.toml");
  } else {
    const rawTopo = readFileSync(topologyPath, "utf8");
    if (/[0-9a-f]{32}|[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i.test(rawTopo)) {
      errors.push("assets/os/cloudflare_topology.toml must not contain literal Cloudflare IDs or UUIDs");
    }
    const parsedTopo = Bun.TOML.parse(rawTopo) as {
      buckets?: Record<string, { prefixes?: Array<{ prefix: string; owner: string }> }>;
      workers?: Array<{ name: string; routes?: string[] }>;
    };
    const seenRoutes = new Set<string>();
    for (const w of parsedTopo.workers ?? []) {
      for (const rt of w.routes ?? []) {
        if (seenRoutes.has(rt)) errors.push(`Duplicate route in cloudflare_topology.toml: ${rt}`);
        seenRoutes.add(rt);
      }
    }
    for (const [bKey, bVal] of Object.entries(parsedTopo.buckets ?? {})) {
      const seenPrefixes = new Set<string>();
      for (const p of bVal.prefixes ?? []) {
        if (seenPrefixes.has(p.prefix)) errors.push(`Duplicate R2 prefix in ${bKey}: ${p.prefix}`);
        seenPrefixes.add(p.prefix);
      }
    }
  }

  return {
    ok: errors.length === 0,
    accountId: process.env.CLOUDFLARE_ACCOUNT_ID ?? null,
    releaseDomain: new URL(urls.releasesUrl).hostname,
    installScriptsPresent,
    workers,
    errors,
  };
}

async function resolveAccountId(token: string): Promise<string> {
  if (process.env.CLOUDFLARE_ACCOUNT_ID) return process.env.CLOUDFLARE_ACCOUNT_ID;
  const res = await fetch("https://api.cloudflare.com/client/v4/accounts?per_page=5", {
    headers: { Authorization: `Bearer ${token}` },
  });
  const data = (await res.json()) as { success: boolean; result?: Array<{ id: string }> };
  const first = data.result?.[0]?.id;
  if (!data.success || !first) {
    throw new Error("Unable to resolve Cloudflare account ID via API token");
  }
  return first;
}

async function ensureInfra(): Promise<Record<string, unknown>> {
  const token = process.env.CLOUDFLARE_API_TOKEN;
  if (!token) throw new Error("CLOUDFLARE_API_TOKEN is required for Cloudflare ensure-infra");
  const accountId = await resolveAccountId(token);

  const wrangler = resolveWranglerCmd();
  const bucketsRes = Bun.spawnSync([...wrangler, "r2", "bucket", "list"], {
    cwd: repoRoot,
    stdout: "pipe",
    stderr: "pipe",
  });
  const bucketOut = new TextDecoder().decode(bucketsRes.stdout);
  let bucketCreated = false;
  if (!/name:\s+aphrody-releases\b/.test(bucketOut)) {
    const createRes = Bun.spawnSync([...wrangler, "r2", "bucket", "create", "aphrody-releases"], {
      cwd: repoRoot,
      stdout: "pipe",
      stderr: "pipe",
    });
    if (createRes.exitCode !== 0) {
      throw new Error(`R2 create failed: ${new TextDecoder().decode(createRes.stderr)}`);
    }
    bucketCreated = true;
  }

  const zonesResp = await fetch(
    `https://api.cloudflare.com/client/v4/zones?name=aphrody.com&account.id=${accountId}`,
    { headers: { Authorization: `Bearer ${token}` } },
  );
  const zonesJson = (await zonesResp.json()) as { success: boolean; result?: Array<{ id: string }> };
  const zoneId = zonesJson.result?.[0]?.id;
  if (!zoneId) throw new Error("Cloudflare zone aphrody.com not found");

  let dnsCreated = false;
  for (const host of ["releases.aphrody.com", "learn.aphrody.com", "web.aphrody.com"]) {
    const dnsResp = await fetch(
      `https://api.cloudflare.com/client/v4/zones/${zoneId}/dns_records?name=${host}`,
      { headers: { Authorization: `Bearer ${token}` } },
    );
    const dnsJson = (await dnsResp.json()) as { result?: Array<{ id: string; type: string }> };
    if ((dnsJson.result ?? []).length === 0) {
      const addRes = await fetch(`https://api.cloudflare.com/client/v4/zones/${zoneId}/dns_records`, {
        method: "POST",
        headers: {
          Authorization: `Bearer ${token}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          type: "AAAA",
          name: host,
          content: "100::",
          proxied: true,
          comment: `Worker-only edge endpoint for ${host}`,
        }),
      });
      const addJson = (await addRes.json()) as { success: boolean; errors?: unknown[] };
      if (!addJson.success) {
        throw new Error(`DNS creation failed (${host}): ${JSON.stringify(addJson.errors)}`);
      }
      if (host === "releases.aphrody.com") dnsCreated = true;
    }
  }

  return {
    ok: true,
    accountId,
    zoneId,
    r2Bucket: "aphrody-releases",
    bucketCreated,
    dnsRecord: "releases.aphrody.com",
    dnsCreated,
  };
}

function publishInstallScripts(dryRun: boolean): { uploaded: string[]; dryRun: boolean } {
  const uploads: Array<{ key: string; file: string; contentType: string }> = [
    {
      key: "aphrody-releases/scripts/install.sh",
      file: join(repoRoot, "script", "aphrody-install.sh"),
      contentType: "text/x-shellscript; charset=utf-8",
    },
    {
      key: "aphrody-releases/scripts/install.ps1",
      file: join(repoRoot, "script", "aphrody-install.ps1"),
      contentType: "text/plain; charset=utf-8",
    },
  ];
  const wrangler = resolveWranglerCmd();
  const uploaded: string[] = [];
  for (const item of uploads) {
    if (!dryRun) {
      const res = Bun.spawnSync(
        [...wrangler, "r2", "object", "put", item.key, "--file", item.file, "--content-type", item.contentType, "--remote"],
        { cwd: repoRoot, stdout: "pipe", stderr: "pipe" },
      );
      if (res.exitCode !== 0) {
        throw new Error(`Failed R2 upload ${item.key}: ${new TextDecoder().decode(res.stderr)}`);
      }
    }
    uploaded.push(item.key);
  }
  return { uploaded, dryRun };
}

const WEB_MIME_TYPES: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".wasm": "application/wasm",
  ".json": "application/json; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".svg": "image/svg+xml",
  ".png": "image/png",
};

const WEB_COMPRESSIBLE_EXTS = new Set([".wasm", ".js", ".mjs", ".html", ".json", ".css", ".svg"]);

function walkFiles(dir: string): string[] {
  const out: string[] = [];
  for (const entry of readdirSync(dir)) {
    const abs = join(dir, entry);
    const st = statSync(abs);
    if (st.isDirectory()) {
      out.push(...walkFiles(abs));
    } else if (st.isFile()) {
      out.push(abs);
    }
  }
  return out;
}

export function publishWebDist(
  distDir = join(repoRoot, "target", "web-dist"),
  appName = "aphrody_desktop",
  dryRun = false,
): { app: string; distDir: string; uploaded: Array<{ key: string; bytes: number; encoding?: string }>; dryRun: boolean } {
  const absDist = resolve(distDir);
  if (!existsSync(absDist)) {
    throw new Error(`Web dist directory not found: ${absDist}`);
  }
  const wrangler = resolveWranglerCmd();
  const baseFiles = walkFiles(absDist).filter((f) => !f.endsWith(".br") && !f.endsWith(".gz"));
  const uploaded: Array<{ key: string; bytes: number; encoding?: string }> = [];

  for (const file of baseFiles) {
    const rel = relative(absDist, file).replace(/\\/g, "/");
    const ext = extname(file).toLowerCase();
    const contentType = WEB_MIME_TYPES[ext] ?? "application/octet-stream";
    const rawBytes = readFileSync(file);
    const items: Array<{ key: string; filePath: string; bytes: number; encoding?: string }> = [
      { key: `aphrody-assets/web/${appName}/${rel}`, filePath: file, bytes: rawBytes.length },
    ];
    if (WEB_COMPRESSIBLE_EXTS.has(ext)) {
      const brPath = `${file}.br`;
      const gzPath = `${file}.gz`;
      const brBytes = existsSync(brPath)
        ? readFileSync(brPath)
        : brotliCompressSync(rawBytes, { params: { [constants.BROTLI_PARAM_QUALITY]: 11 } });
      if (!existsSync(brPath)) writeFileSync(brPath, brBytes);
      const gzBytes = existsSync(gzPath) ? readFileSync(gzPath) : gzipSync(rawBytes, { level: 9 });
      if (!existsSync(gzPath)) writeFileSync(gzPath, gzBytes);
      items.push(
        { key: `aphrody-assets/web/${appName}/${rel}.br`, filePath: brPath, bytes: brBytes.length, encoding: "br" },
        { key: `aphrody-assets/web/${appName}/${rel}.gz`, filePath: gzPath, bytes: gzBytes.length, encoding: "gzip" },
      );
    }

    for (const item of items) {
      if (!dryRun) {
        const putArgs = [
          ...wrangler,
          "r2",
          "object",
          "put",
          item.key,
          "--file",
          item.filePath,
          "--content-type",
          contentType,
          "--remote",
        ];
        if (item.encoding) {
          putArgs.push("--content-encoding", item.encoding);
        }
        const res = Bun.spawnSync(putArgs, { cwd: repoRoot, stdout: "pipe", stderr: "pipe" });
        if (res.exitCode !== 0) {
          throw new Error(`Failed R2 web upload ${item.key}: ${new TextDecoder().decode(res.stderr)}`);
        }
      }
      uploaded.push({ key: item.key, bytes: item.bytes, ...(item.encoding ? { encoding: item.encoding } : {}) });
    }
  }

  return { app: appName, distDir: absDist, uploaded, dryRun };
}

function deployWorkers(
  dryRun: boolean,
  allWorkers: boolean,
  onlyEnv?: string,
): Array<{ worker: string; env: string; exitCode: number }> {
  const wrangler = resolveWranglerCmd();
  const dirs = [".cloudflare/aphrody-worker"];
  const envs = onlyEnv !== undefined ? [onlyEnv] : allWorkers ? ["", "web"] : [""];
  const results: Array<{ worker: string; env: string; exitCode: number }> = [];
  for (const rel of dirs) {
    for (const envName of envs) {
      const args = dryRun
        ? [
            ...wrangler,
            "deploy",
            ...(envName ? ["--env", envName] : ["--env="]),
            "--dry-run",
            "--outdir",
            join(repoRoot, "target", "wrangler-dry", `${rel.replace(/[/.]/g, "_")}_${envName || "default"}`),
          ]
        : [...wrangler, "deploy", ...(envName ? ["--env", envName] : ["--env="])];
      const res = Bun.spawnSync(args, { cwd: join(repoRoot, rel), stdout: "inherit", stderr: "inherit" });
      results.push({ worker: rel, env: envName || "default", exitCode: res.exitCode });
      if (res.exitCode !== 0) {
        throw new Error(`Worker deploy failed in ${rel} (env=${envName || "default"}, exit=${res.exitCode})`);
      }
    }
  }
  return results;
}

async function smokeTest(baseUrl: string): Promise<{ ok: boolean; baseUrl: string; checks: Array<Record<string, unknown>> }> {
  const endpoints = ["/healthz", "/api/channels", "/download", "/install.sh", "/install.ps1"];
  const checks: Array<Record<string, unknown>> = [];
  let allOk = true;
  for (const ep of endpoints) {
    const url = `${baseUrl.replace(/\/$/, "")}${ep}`;
    const res = await fetch(url);
    const body = await res.text();
    const ok = res.status >= 200 && res.status < 300 && body.length > 0;
    if (!ok) allOk = false;
    checks.push({
      url,
      status: res.status,
      contentType: res.headers.get("content-type"),
      bytes: body.length,
      ok,
    });
  }
  return { ok: allOk, baseUrl, checks };
}

function parseMcpSseOrJson(body: string): Record<string, unknown> | null {
  const trimmed = body.trimStart();
  if (!trimmed) return null;
  if (trimmed.startsWith("{") || trimmed.startsWith("[")) {
    return JSON.parse(trimmed) as Record<string, unknown>;
  }
  for (const event of body.split(/\r?\n\r?\n/)) {
    const data = event
      .split(/\r?\n/)
      .filter((line) => line.startsWith("data:"))
      .map((line) => line.slice(5).replace(/^ /, ""))
      .join("\n");
    if (data) return JSON.parse(data) as Record<string, unknown>;
  }
  return null;
}

async function runMcpStdioBridge(endpoint = "https://mcp.cloudflare.com/mcp"): Promise<void> {
  const token = process.env.CLOUDFLARE_API_TOKEN?.trim();
  let sessionId: string | undefined;
  let buffer = "";
  const decoder = new TextDecoder();

  for await (const chunk of Bun.stdin.stream()) {
    buffer += decoder.decode(chunk, { stream: true });
    let newlineIndex: number;
    while ((newlineIndex = buffer.indexOf("\n")) >= 0) {
      const line = buffer.slice(0, newlineIndex).trim();
      buffer = buffer.slice(newlineIndex + 1);
      if (!line) continue;

      let msg: Record<string, unknown>;
      try {
        msg = JSON.parse(line) as Record<string, unknown>;
      } catch (err) {
        process.stdout.write(
          `${JSON.stringify({ jsonrpc: "2.0", id: null, error: { code: -32700, message: String(err) } })}\n`,
        );
        continue;
      }

      const headers: Record<string, string> = {
        "content-type": "application/json",
        accept: "application/json, text/event-stream",
        "mcp-protocol-version": "2025-11-25",
        ...(token ? { authorization: `Bearer ${token}` } : {}),
        ...(sessionId ? { "mcp-session-id": sessionId } : {}),
      };

      try {
        const res = await fetch(endpoint, { method: "POST", headers, body: line });
        const nextSession = res.headers.get("mcp-session-id");
        if (nextSession) sessionId = nextSession;
        const text = await res.text();
        if (!res.ok) {
          if (msg.id !== undefined) {
            process.stdout.write(
              `${JSON.stringify({
                jsonrpc: "2.0",
                id: msg.id,
                error: { code: -32000, message: `HTTP ${res.status}: ${text.slice(0, 300)}` },
              })}\n`,
            );
          }
          continue;
        }
        const parsed = parseMcpSseOrJson(text);
        if (msg.id !== undefined && parsed) {
          process.stdout.write(`${JSON.stringify(parsed)}\n`);
        }
      } catch (err) {
        if (msg.id !== undefined) {
          process.stdout.write(
            `${JSON.stringify({
              jsonrpc: "2.0",
              id: msg.id,
              error: { code: -32603, message: String(err) },
            })}\n`,
          );
        }
      }
    }
  }
}

if (import.meta.main) {
  const args = Bun.argv.slice(2);
  const cmd = args[0] ?? "audit";
  const dryRun = args.includes("--dry-run");
  const allWorkers = args.includes("--all");
  const baseIdx = args.indexOf("--base-url");
  const baseUrl =
    (baseIdx >= 0 ? args[baseIdx + 1] : undefined) ??
    readProductUrls().releasesUrl;

  if (cmd === "audit") {
    const report = auditCloudflareRepoConfig(repoRoot, dryRun);
    console.log(JSON.stringify(report, null, 2));
    process.exit(report.ok ? 0 : 1);
  } else if (cmd === "ensure-infra") {
    const res = await ensureInfra();
    console.log(JSON.stringify(res, null, 2));
  } else if (cmd === "publish-scripts") {
    const res = publishInstallScripts(dryRun);
    console.log(JSON.stringify(res, null, 2));
  } else if (cmd === "publish-web") {
    const distIdx = args.indexOf("--dist");
    const appIdx = args.indexOf("--app");
    const distDir = (distIdx >= 0 ? args[distIdx + 1] : undefined) ?? join(repoRoot, "target", "web-dist");
    const appName = (appIdx >= 0 ? args[appIdx + 1] : undefined) ?? "aphrody_desktop";
    const res = publishWebDist(distDir, appName, dryRun);
    console.log(JSON.stringify(res, null, 2));
  } else if (cmd === "deploy") {
    const envIdx = args.indexOf("--env");
    const onlyEnv = envIdx >= 0 ? args[envIdx + 1] : undefined;
    const infra = dryRun ? { skipped: true } : await ensureInfra();
    const scripts = onlyEnv === "web" ? { skipped: true } : publishInstallScripts(dryRun);
    const workers = deployWorkers(dryRun, allWorkers, onlyEnv);
    console.log(JSON.stringify({ ok: true, infra, scripts, workers }, null, 2));
  } else if (cmd === "smoke") {
    const res = await smokeTest(baseUrl);
    console.log(JSON.stringify(res, null, 2));
    process.exit(res.ok ? 0 : 1);
  } else if (cmd === "mcp-stdio") {
    await runMcpStdioBridge(args[1] ?? "https://mcp.cloudflare.com/mcp");
  } else {
    console.error(
      "Usage: bun tooling/bun/cloudflare-deploy.ts [audit|ensure-infra|publish-scripts|publish-web|deploy|smoke|mcp-stdio] [--dry-run] [--all] [--dist <dir>] [--app <name>] [--base-url <url>]",
    );
    process.exit(1);
  }
}
