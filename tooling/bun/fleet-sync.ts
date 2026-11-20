#!/usr/bin/env bun
/**
 * Reproducible Parallel Multi-VPS Fleet Synchronization, XFS/Btrfs CoW Worktree Cloning,
 * Sharded CI Verification, and End-to-End Cloudflare + VPS Deploy Runner (`fr1`, `fr2`, `prd`).
 *
 * Derives product identity from `product.toml` and resolves SSH hosts cleanly via `~/.ssh/config`.
 * Syncs the repo itself (HEAD bundle + tracked patch + untracked files); upstream forks are pinned
 * Cargo git dependencies fetched by cargo on the remote, never checked out.
 *
 * Usage:
 *   bun tooling/bun/fleet-sync.ts sync [--host fr1 | --hosts fr1,fr2,prd | --all]
 *   bun tooling/bun/fleet-sync.ts verify [--host fr1 | --hosts fr1,fr2 | --all] [--pull-shots]
 *   bun tooling/bun/fleet-sync.ts xfs-clone [--host fr1] --job <id>
 *   bun tooling/bun/fleet-sync.ts parallel-build [--hosts fr1,fr2,prd]
 *   bun tooling/bun/fleet-sync.ts deploy [--host fr1] [--edge-host prd] [--channel canary] [--dry-run]
 */
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { resolveWranglerCmd } from "./cloudflare-deploy.ts";

export const repoRoot = resolve(import.meta.dir, "../..");
export const DEFAULT_FLEET_HOSTS = ["fr1", "fr2", "prd"];

function opt(flag: string): string | undefined {
  const args = Bun.argv.slice(2);
  const idx = args.indexOf(flag);
  if (idx >= 0) return args[idx + 1];
  const eq = args.find((a) => a.startsWith(`${flag}=`));
  return eq ? eq.slice(flag.length + 1) : undefined;
}

export function resolveHostList(
  singleHost = opt("--host"),
  multiHosts = opt("--hosts"),
  all = Bun.argv.includes("--all"),
): string[] {
  if (all) return [...DEFAULT_FLEET_HOSTS];
  if (multiHosts) {
    return multiHosts
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);
  }
  return [singleHost ?? process.env.APHRODY_FLEET_HOST ?? "fr1"];
}

function runChecked(cmd: string[], cwd?: string): string {
  const res = Bun.spawnSync(cmd, { cwd, stdout: "pipe", stderr: "pipe" });
  if (res.exitCode !== 0) {
    const stderr = new TextDecoder().decode(res.stderr);
    throw new Error(`Command failed (${res.exitCode}): ${cmd.join(" ")}\n${stderr}`);
  }
  return new TextDecoder().decode(res.stdout);
}

async function runSshAsync(
  host: string,
  remoteCmd: string,
  stdinBytes?: Uint8Array | string,
): Promise<{ host: string; exitCode: number; stdout: string; stderr: string }> {
  const proc = Bun.spawn(["ssh", host, remoteCmd], {
    stdin:
      typeof stdinBytes === "string"
        ? Buffer.from(stdinBytes, "utf8")
        : stdinBytes
          ? Buffer.from(stdinBytes)
          : undefined,
    stdout: "pipe",
    stderr: "pipe",
  });
  const [exitCode, stdout, stderr] = await Promise.all([
    proc.exited,
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
  ]);
  return { host, exitCode, stdout, stderr };
}

export function readProductMeta(root = repoRoot): { slug: string; repoSlug: string } {
  const product = Bun.TOML.parse(readFileSync(join(root, "product.toml"), "utf8")) as {
    product?: { slug?: string };
    urls?: { repository?: string };
  };
  const slug = product.product?.slug ?? "aphrody";
  const repoUrl = product.urls?.repository ?? "https://github.com/aphrody-labs/aphrody";
  const repoSlug = repoUrl.replace(/\.git$/, "");
  return { slug, repoSlug };
}

function resolveRemoteDir(slug: string): string {
  return opt("--remote-dir") ?? process.env.APHRODY_FLEET_DIR ?? `/srv/xbuild/scratch/${slug}`;
}

interface LocalSyncPayload {
  localHead: string;
  trackedPatch: Uint8Array;
  untrackedTar: Uint8Array | null;
  untrackedCount: number;
}

function prepareLocalPayload(): LocalSyncPayload {
  const localHead = runChecked(["git", "-C", repoRoot, "rev-parse", "HEAD"]).trim();
  const diffRes = Bun.spawnSync([
    "git",
    "-C",
    repoRoot,
    "diff",
    "HEAD",
    "--binary",
    "--",
    ".",
  ]);
  if (diffRes.exitCode !== 0) {
    throw new Error(`git diff HEAD failed: ${new TextDecoder().decode(diffRes.stderr)}`);
  }

  const untrackedRaw = runChecked(["git", "-C", repoRoot, "ls-files", "--others", "--exclude-standard"]);
  const untracked = untrackedRaw
    .split(/\r?\n/)
    .map((s) => s.trim())
    .filter(Boolean);

  let untrackedTar: Uint8Array | null = null;
  if (untracked.length > 0) {
    const tmpDir = join(repoRoot, "target", "fleet-tmp");
    mkdirSync(tmpDir, { recursive: true });
    const listPath = join(tmpDir, "untracked.txt");
    writeFileSync(listPath, `${untracked.join("\n")}\n`, "utf8");
    const tarCreate = Bun.spawnSync(["tar", "-C", repoRoot, "-cf", "-", "-T", listPath]);
    if (tarCreate.exitCode !== 0) {
      throw new Error(`tar create failed: ${new TextDecoder().decode(tarCreate.stderr)}`);
    }
    untrackedTar = tarCreate.stdout;
  }

  return {
    localHead,
    trackedPatch: diffRes.stdout,
    untrackedTar,
    untrackedCount: untracked.length,
  };
}

async function syncOneHostAsync(
  host: string,
  preferredRemoteRepo: string,
  repoBaseUrl: string,
  payload: LocalSyncPayload,
) {
  const prepScript = [
    "set -euo pipefail",
    'TARGET_REPO="' + preferredRemoteRepo + '"',
    'if [ -d /srv/xbuild ] && [ -w /srv/xbuild ]; then',
    '  FREE_GB=$(df -BG --output=avail /srv/xbuild 2>/dev/null | tail -n 1 | tr -dc "0-9" || echo 0)',
    '  if [ "${FREE_GB:-0}" -lt 10 ]; then TARGET_REPO="/var/cache/aphrody/scratch/aphrody"; fi',
    'else',
    '  TARGET_REPO="$HOME/.cache/aphrody/scratch/aphrody"',
    'fi',
    'mkdir -p "$(dirname "$TARGET_REPO")"',
    'if [ ! -e "$TARGET_REPO/.git" ]; then',
    `  git clone --no-checkout "${repoBaseUrl}.git" "$TARGET_REPO" || git init "$TARGET_REPO"`,
    "fi",
    'git -C "$TARGET_REPO" reset --hard HEAD >/dev/null',
    'git -C "$TARGET_REPO" clean -fd >/dev/null',
    'HEAD_SHA=$(git -C "$TARGET_REPO" rev-parse HEAD 2>/dev/null || echo "")',
    'echo "TARGET_REPO=$TARGET_REPO"',
    'echo "HEAD_SHA=$HEAD_SHA"',
  ].join("\n");

  const prep = await runSshAsync(host, "bash -s", prepScript);
  if (prep.exitCode !== 0) {
    throw new Error(`[${host}] prep failed (${prep.exitCode}): ${prep.stderr}`);
  }
  const remoteRepo =
    /^TARGET_REPO=(.+)$/m.exec(prep.stdout)?.[1]?.trim() ?? preferredRemoteRepo;
  const remoteHead = /^HEAD_SHA=(.*)$/m.exec(prep.stdout)?.[1]?.trim() ?? "";

  if (!remoteHead || payload.localHead !== remoteHead) {
    const bundleBytes = remoteHead
      ? Bun.spawnSync(["git", "-C", repoRoot, "bundle", "create", "-", "main", `^${remoteHead}`])
      : { exitCode: 1, stdout: new Uint8Array(), stderr: new Uint8Array() };
    const effectiveBytes =
      bundleBytes.exitCode === 0
        ? bundleBytes.stdout
        : (() => {
            const full = Bun.spawnSync(["git", "-C", repoRoot, "bundle", "create", "-", "main"]);
            if (full.exitCode !== 0) {
              throw new Error(`git bundle failed: ${new TextDecoder().decode(full.stderr)}`);
            }
            return full.stdout;
          })();
    const pushBundle = await runSshAsync(
      host,
      `cat > /tmp/aphrody-delta.bundle && git -C ${remoteRepo} fetch /tmp/aphrody-delta.bundle main && git -C ${remoteRepo} checkout -f -B main FETCH_HEAD && rm -f /tmp/aphrody-delta.bundle`,
      effectiveBytes,
    );
    if (pushBundle.exitCode !== 0) {
      throw new Error(`[${host}] remote bundle apply failed: ${pushBundle.stderr}`);
    }
  }

  if (payload.trackedPatch.length > 0) {
    const apply = await runSshAsync(
      host,
      `git -C ${remoteRepo} apply --binary --whitespace=nowarn -`,
      payload.trackedPatch,
    );
    if (apply.exitCode !== 0) {
      throw new Error(`[${host}] remote git apply failed: ${apply.stderr}`);
    }
  }

  if (payload.untrackedTar) {
    const tarExtract = await runSshAsync(host, `tar -C ${remoteRepo} -xf -`, payload.untrackedTar);
    if (tarExtract.exitCode !== 0) {
      throw new Error(`[${host}] remote tar extract failed: ${tarExtract.stderr}`);
    }
  }

  return {
    ok: true,
    host,
    remoteRepo,
    syncedCommit: payload.localHead.slice(0, 10),
    patchBytes: payload.trackedPatch.length,
    untrackedCount: payload.untrackedCount,
  };
}

async function createCowReflinkJobTree(host: string, remoteRepo: string, jobId: string): Promise<string> {
  const jobDir = `/srv/xbuild/jobs/${jobId}`;
  const script = `set -euo pipefail
mkdir -p /srv/xbuild/jobs
rm -rf "${jobDir}"
cp -a --reflink=auto "${remoteRepo}" "${jobDir}"
echo "${jobDir}"
`;
  const res = await runSshAsync(host, "bash -s", script);
  if (res.exitCode !== 0) {
    throw new Error(`[${host}] XFS/Btrfs CoW clone failed: ${res.stderr}`);
  }
  return jobDir;
}

function buildRemoteEnvHeader(): string {
  return [
    "set -euo pipefail",
    'export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$HOME/.bun/bin:/usr/local/bin:$PATH"',
    'if command -v sccache >/dev/null 2>&1; then export RUSTC_WRAPPER="$(command -v sccache)"; fi',
    'export SCCACHE_DIR="${SCCACHE_DIR:-/var/cache/aphrody/sccache}"',
    'export SCCACHE_BASEDIRS="${SCCACHE_BASEDIRS:-/srv/xbuild:/home/ubuntu:/root:/srv}"',
    'export TMPDIR="/tmp"',
  ].join("\n");
}

async function runVerifyHost(host: string, remoteRepo: string, pullShots: boolean) {
  const script = [
    buildRemoteEnvHeader(),
    `cd ${remoteRepo}`,
    "cargo test --lib -p brand -p aphrody -p aphrody_session -p aphrody_design -p aphrody_pilot -p aphrody_craft -p aphrody_ui -p aphrody_desktop -p aphrody_cli -p aphrody_store -p aphrody_installer -p cosmic_gpui -p iced_gpui -p egui_gpui -p bevy_gpui -p tauri_gpui",
    "xvfb-run -a cargo test -p aphrody_desktop --test visual_smoke -- --nocapture || cargo test -p aphrody_desktop --test visual_smoke -- --nocapture",
    "cargo clippy --no-deps -p aphrody -p aphrody_session -p aphrody_design -p aphrody_pilot -p aphrody_craft -p aphrody_ui -p aphrody_desktop -p aphrody_store -p aphrody_installer -p aphrody_cli -p iced_gpui -p tauri_gpui -p cosmic_gpui -- -D warnings",
    "cargo xtask brand check",
    "cargo xtask studio check",
    "cargo xtask package-conformity",
  ].join("\n");

  const res = await runSshAsync(host, "bash -s", script);
  if (res.exitCode !== 0) {
    throw new Error(`[${host}] verify failed:\n${res.stdout.slice(-1500)}\n${res.stderr.slice(-1500)}`);
  }

  if (pullShots) {
    const localShots = join(repoRoot, "target", "os_screenshots");
    mkdirSync(localShots, { recursive: true });
    const tar = Bun.spawnSync(["ssh", host, `tar -C ${remoteRepo}/target/os_screenshots -cf - .`]);
    if (tar.exitCode !== 0) throw new Error(new TextDecoder().decode(tar.stderr));
    const ext = Bun.spawnSync(["tar", "-C", localShots, "-xf", "-"], { stdin: tar.stdout });
    if (ext.exitCode !== 0) throw new Error(new TextDecoder().decode(ext.stderr));
  }
  return { host, ok: true };
}

export interface ParallelShardPlan {
  host: string;
  role: string;
  remoteRepo: string;
  script: string;
}

export function buildParallelShardPlans(
  hosts: string[],
  defaultRemoteRepo: string,
  repoByHost?: Map<string, string>,
): ParallelShardPlan[] {
  const hdr = buildRemoteEnvHeader();
  const plans: ParallelShardPlan[] = [];
  for (const host of hosts) {
    const remoteRepo = repoByHost?.get(host) ?? defaultRemoteRepo;
    if (host === "fr1") {
      plans.push({
        host,
        role: "primary-xfs-native-builder",
        remoteRepo,
        script: [
          hdr,
          `cd ${remoteRepo}`,
          "cargo build --profile bun-release -p aphrody_cli --bin aphrody-cli --bin gpui-wgpu",
          "cargo check -p zed --bin aphrody -p aphrody_desktop -p gpui_ui",
          "cargo check -p gpui_code --no-default-features",
          "cargo xtask brand check && cargo xtask studio check && cargo xtask package-conformity",
        ].join("\n"),
      });
    } else if (host === "fr2") {
      plans.push({
        host,
        role: "secondary-xfs-wasm-verifier",
        remoteRepo,
        script: [
          hdr,
          `cd ${remoteRepo}`,
          "rustup target add wasm32-unknown-unknown wasm32-wasip2 >/dev/null 2>&1 || true",
          "bun tooling/bun/repo-hygiene.ts",
          "bun test --timeout 20000 tooling/bun",
          "cargo check --target wasm32-unknown-unknown -p aphrody_desktop --features web --example web_os",
          "RUSTC_WRAPPER= cargo check --target wasm32-wasip2 -p aphrody --no-default-features --features minimal",
          "RUSTC_WRAPPER= cargo check --target wasm32-wasip2 -p aphrody_session -p aphrody_craft -p release_channel -p brand --no-default-features",
        ].join("\n"),
      });
    } else {
      plans.push({
        host,
        role: "edge-btrfs-cloudflare-deploy",
        remoteRepo,
        script: [
          hdr,
          `cd ${remoteRepo}`,
          "bun tooling/bun/cloudflare-deploy.ts audit --dry-run",
          "systemctl is-active nginx >/dev/null && echo NGINX_ACTIVE=true",
        ].join("\n"),
      });
    }
  }
  return plans;
}

async function runParallelBuild(hosts: string[], remoteRepo: string) {
  const { repoSlug } = readProductMeta();
  const payload = prepareLocalPayload();
  const syncResults = await Promise.all(
    hosts.map((h) => syncOneHostAsync(h, remoteRepo, repoSlug, payload)),
  );
  const repoByHost = new Map(syncResults.map((r) => [r.host, r.remoteRepo]));
  const shards = buildParallelShardPlans(hosts, remoteRepo, repoByHost);
  const start = performance.now();
  const results = await Promise.all(
    shards.map(async (s) => {
      const t0 = performance.now();
      const res = await runSshAsync(s.host, "bash -s", s.script);
      return {
        host: s.host,
        role: s.role,
        exitCode: res.exitCode,
        durationSec: Number(((performance.now() - t0) / 1000).toFixed(2)),
        tailStdout: res.stdout.trim().split(/\r?\n/).slice(-8),
        tailStderr: res.stderr.trim().split(/\r?\n/).slice(-8),
      };
    }),
  );
  const failed = results.filter((r) => r.exitCode !== 0);
  if (failed.length > 0) {
    throw new Error(`Parallel shard build failed: ${JSON.stringify(failed, null, 2)}`);
  }
  return {
    ok: true,
    totalDurationSec: Number(((performance.now() - start) / 1000).toFixed(2)),
    syncedHosts: syncResults.length,
    shards: results,
  };
}

async function runEndToEndDeploy(
  buildHost: string,
  edgeHost: string,
  preferredRemoteRepo: string,
  channel: string,
  dryRun: boolean,
) {
  const { slug, repoSlug } = readProductMeta();
  const payload = prepareLocalPayload();
  const [buildSync] = await Promise.all([
    syncOneHostAsync(buildHost, preferredRemoteRepo, repoSlug, payload),
    syncOneHostAsync(edgeHost, preferredRemoteRepo, repoSlug, payload),
  ]);
  const remoteRepo = buildSync.remoteRepo;

  const buildScript = [
    buildRemoteEnvHeader(),
    `cd ${remoteRepo}`,
    "cargo build --profile bun-release -p aphrody_cli --bin aphrody-cli --bin gpui-wgpu",
    "cargo build --profile bun-release -p aphrody_desktop --bin aphrody-os",
    "rm -rf /tmp/aphrody-dist && mkdir -p /tmp/aphrody-dist",
    "cp target/bun-release/aphrody-cli target/bun-release/gpui-wgpu target/bun-release/aphrody-os /tmp/aphrody-dist/",
    "tar -czf /tmp/aphrody-dist/aphrody-x86_64.tar.gz -C target/bun-release aphrody-cli gpui-wgpu aphrody-os",
    "ls -lh /tmp/aphrody-dist",
  ].join("\n");

  const buildRes = await runSshAsync(buildHost, "bash -s", buildScript);
  if (buildRes.exitCode !== 0) {
    throw new Error(`[${buildHost}] release build failed:\n${buildRes.stderr}`);
  }

  const distDir = join(repoRoot, "target", "fleet-dist");
  mkdirSync(distDir, { recursive: true });
  const pullTar = Bun.spawnSync(["ssh", buildHost, "tar -C /tmp/aphrody-dist -cf - ."]);
  if (pullTar.exitCode !== 0) {
    throw new Error(`Failed pulling dist artifacts from ${buildHost}: ${new TextDecoder().decode(pullTar.stderr)}`);
  }
  const ext = Bun.spawnSync(["tar", "-C", distDir, "-xf", "-"], { stdin: pullTar.stdout });
  if (ext.exitCode !== 0) {
    throw new Error(`Failed extracting dist artifacts: ${new TextDecoder().decode(ext.stderr)}`);
  }

  const tarBytes = readFileSync(join(distDir, "aphrody-x86_64.tar.gz"));
  const sha256 = createHash("sha256").update(tarBytes).digest("hex");
  const version = `${new Date().toISOString().slice(0, 10).replace(/-/g, ".")}-${payload.localHead.slice(0, 7)}`;
  const manifest = {
    product: slug,
    version,
    channel,
    os: "linux",
    arch: "x86_64",
    asset: "aphrody-x86_64.tar.gz",
    url: `https://releases.aphrody.com/assets/${channel}/latest/aphrody-x86_64.tar.gz`,
    sha256,
    binaries: ["aphrody-cli", "gpui-wgpu", "aphrody-os"],
  };
  writeFileSync(join(distDir, "aphrody-x86_64.tar.gz.sha256"), `${sha256}  aphrody-x86_64.tar.gz\n`, "utf8");
  writeFileSync(join(distDir, "linux-x86_64.json"), JSON.stringify(manifest, null, 2), "utf8");

  // Deploy native edge binaries to prd (/opt/aphrody/bin) in parallel with Cloudflare R2 + Worker publish
  const pushEdgePromise = (async () => {
    const pushTar = Bun.spawnSync(["tar", "-C", distDir, "-cf", "-", "aphrody-cli", "gpui-wgpu", "aphrody-os", "linux-x86_64.json"]);
    const edgeCmd = [
      "mkdir -p /opt/aphrody/bin",
      "tar -C /opt/aphrody/bin -xf -",
      "chmod +x /opt/aphrody/bin/aphrody-cli /opt/aphrody/bin/gpui-wgpu /opt/aphrody/bin/aphrody-os",
      "/opt/aphrody/bin/aphrody-cli dev matrix --json | head -n 5",
    ].join(" && ");
    const res = await runSshAsync(edgeHost, edgeCmd, pushTar.stdout);
    if (res.exitCode !== 0) {
      throw new Error(`[${edgeHost}] edge deploy failed: ${res.stderr}`);
    }
    return { edgeHost, path: "/opt/aphrody/bin", ok: true };
  })();

  const cfPromise = (async () => {
    const cfArgs = ["bun", "tooling/bun/cloudflare-deploy.ts", "deploy"];
    if (dryRun) cfArgs.push("--dry-run");
    const deployRes = Bun.spawnSync(cfArgs, { cwd: repoRoot, stdout: "pipe", stderr: "pipe" });
    if (deployRes.exitCode !== 0) {
      throw new Error(`Cloudflare worker deploy failed: ${new TextDecoder().decode(deployRes.stderr)}`);
    }
    if (!dryRun) {
      const r2Uploads: Array<[string, string, string]> = [
        [`aphrody-releases/${channel}/${version}/aphrody-x86_64.tar.gz`, join(distDir, "aphrody-x86_64.tar.gz"), "application/gzip"],
        [`aphrody-releases/${channel}/latest/aphrody-x86_64.tar.gz`, join(distDir, "aphrody-x86_64.tar.gz"), "application/gzip"],
        [`aphrody-releases/${channel}/latest/aphrody-x86_64.tar.gz.sha256`, join(distDir, "aphrody-x86_64.tar.gz.sha256"), "text/plain"],
        [`aphrody-releases/manifests/${channel}/${version}/linux-x86_64.json`, join(distDir, "linux-x86_64.json"), "application/json"],
        [`aphrody-releases/channels/${channel}/linux-x86_64.json`, join(distDir, "linux-x86_64.json"), "application/json"],
        [`aphrody-releases/channels/${channel}/latest.json`, join(distDir, "linux-x86_64.json"), "application/json"],
      ];
      const wrangler = resolveWranglerCmd();
      for (const [key, file, ct] of r2Uploads) {
        runChecked([...wrangler, "r2", "object", "put", key, "--file", file, "--content-type", ct, "--remote"], repoRoot);
      }
    }
    return { cloudflareWorker: "aphrody-worker", channel, version, dryRun, ok: true };
  })();

  const [edgeOut, cfOut] = await Promise.all([pushEdgePromise, cfPromise]);
  return { ok: true, manifest, edgeOut, cfOut };
}

if (import.meta.main) {
  const args = Bun.argv.slice(2);
  const cmd = args[0] ?? "sync";
  const hosts = resolveHostList();
  const pullShots = args.includes("--pull-shots");
  const dryRun = args.includes("--dry-run");
  const channel = opt("--channel") ?? "canary";
  const { slug, repoSlug } = readProductMeta();
  const remoteRepo = resolveRemoteDir(slug);

  if (cmd === "sync") {
    const payload = prepareLocalPayload();
    const results = await Promise.all(
      hosts.map((h) => syncOneHostAsync(h, remoteRepo, repoSlug, payload)),
    );
    console.log(JSON.stringify(results.length === 1 ? results[0] : results, null, 2));
  } else if (cmd === "verify") {
    const payload = prepareLocalPayload();
    const syncResults = await Promise.all(
      hosts.map((h) => syncOneHostAsync(h, remoteRepo, repoSlug, payload)),
    );
    const verifyResults = await Promise.all(
      hosts.map((h) => runVerifyHost(h, remoteRepo, pullShots && h === hosts[0])),
    );
    console.log(JSON.stringify({ ok: true, syncResults, verifyResults }, null, 2));
  } else if (cmd === "xfs-clone") {
    const jobId = opt("--job") ?? `ci-${Date.now()}`;
    const jobDir = await createCowReflinkJobTree(hosts[0], remoteRepo, jobId);
    console.log(JSON.stringify({ ok: true, host: hosts[0], jobDir }, null, 2));
  } else if (cmd === "parallel-build") {
    const buildHosts = opt("--hosts") || args.includes("--all") ? hosts : DEFAULT_FLEET_HOSTS;
    const out = await runParallelBuild(buildHosts, remoteRepo);
    console.log(JSON.stringify(out, null, 2));
  } else if (cmd === "deploy") {
    const buildHost = opt("--host") ?? "fr1";
    const edgeHost = opt("--edge-host") ?? "prd";
    const out = await runEndToEndDeploy(buildHost, edgeHost, remoteRepo, channel, dryRun);
    console.log(JSON.stringify(out, null, 2));
  } else {
    console.error(
      "Usage: bun tooling/bun/fleet-sync.ts [sync|verify|xfs-clone|parallel-build|deploy] [--host <h> | --hosts h1,h2 | --all] [--pull-shots] [--dry-run]",
    );
    process.exit(1);
  }
}
