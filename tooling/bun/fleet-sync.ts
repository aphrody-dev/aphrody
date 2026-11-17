#!/usr/bin/env bun
/**
 * Reproducible remote fleet synchronization & verification runner (`fr1` builds, `fr2` tests/bench).
 * Derives product identity from `product.toml`, submodule SHAs from `git ls-files -s vendor`, and
 * resolves SSH hosts cleanly via `ssh` config (no hardcoded IPs or personal paths).
 *
 * Usage:
 *   bun tooling/bun/fleet-sync.ts sync --host fr1 [--remote-dir /srv/xbuild/scratch/aphrody]
 *   bun tooling/bun/fleet-sync.ts verify --host fr1 [--pull-shots]
 */
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { parseGitmodules } from "./submodules.ts";

const repoRoot = resolve(import.meta.dir, "../..");

function opt(flag: string): string | undefined {
  const args = Bun.argv.slice(2);
  const idx = args.indexOf(flag);
  if (idx >= 0) return args[idx + 1];
  const eq = args.find((a) => a.startsWith(`${flag}=`));
  return eq ? eq.slice(flag.length + 1) : undefined;
}

function runChecked(cmd: string[], cwd?: string): string {
  const res = Bun.spawnSync(cmd, { cwd, stdout: "pipe", stderr: "pipe" });
  if (res.exitCode !== 0) {
    const stderr = new TextDecoder().decode(res.stderr);
    throw new Error(`Command failed (${res.exitCode}): ${cmd.join(" ")}\n${stderr}`);
  }
  return new TextDecoder().decode(res.stdout);
}

function readProductMeta(): { slug: string; repoSlug: string } {
  const product = Bun.TOML.parse(readFileSync(join(repoRoot, "product.toml"), "utf8")) as {
    product?: { slug?: string };
    urls?: { repository?: string };
  };
  const slug = product.product?.slug ?? "aphrody";
  const repoUrl = product.urls?.repository ?? "https://github.com/aphrody-dev/aphrody";
  const repoSlug = repoUrl.replace(/\.git$/, "");
  return { slug, repoSlug };
}

function resolveRemoteDir(host: string, slug: string): string {
  return opt("--remote-dir") ?? process.env.APHRODY_FLEET_DIR ?? `/srv/xbuild/scratch/${slug}`;
}

function syncToHost(host: string, remoteRepo: string, repoBaseUrl: string) {
  const localHead = runChecked(["git", "-C", repoRoot, "rev-parse", "HEAD"]).trim();
  const remoteHead = runChecked(["ssh", host, `git -C ${remoteRepo} rev-parse HEAD`]).trim();

  runChecked(["ssh", host, `git -C ${remoteRepo} reset --hard HEAD && git -C ${remoteRepo} clean -fd -e vendor`]);

  if (localHead !== remoteHead) {
    const bundleBytes = Bun.spawnSync(["git", "-C", repoRoot, "bundle", "create", "-", "main", `^${remoteHead}`]);
    if (bundleBytes.exitCode !== 0) {
      throw new Error(`git bundle failed: ${new TextDecoder().decode(bundleBytes.stderr)}`);
    }
    const pushBundle = Bun.spawnSync(
      [
        "ssh",
        host,
        `cat > /tmp/aphrody-delta.bundle && git -C ${remoteRepo} fetch /tmp/aphrody-delta.bundle main && git -C ${remoteRepo} checkout -f -B main FETCH_HEAD && rm -f /tmp/aphrody-delta.bundle`,
      ],
      { stdin: bundleBytes.stdout, stderr: "pipe" },
    );
    if (pushBundle.exitCode !== 0) {
      throw new Error(`remote bundle apply failed: ${new TextDecoder().decode(pushBundle.stderr)}`);
    }
  }

  const trackedPatch = Bun.spawnSync([
    "git",
    "-C",
    repoRoot,
    "diff",
    "HEAD",
    "--binary",
    "--",
    ".",
    ":(exclude)vendor",
  ]);
  if (trackedPatch.exitCode !== 0) {
    throw new Error(`git diff HEAD failed: ${new TextDecoder().decode(trackedPatch.stderr)}`);
  }

  if (trackedPatch.stdout.length > 0) {
    const apply = Bun.spawnSync(["ssh", host, `git -C ${remoteRepo} apply --binary --whitespace=nowarn -`], {
      stdin: trackedPatch.stdout,
      stderr: "pipe",
    });
    if (apply.exitCode !== 0) {
      throw new Error(`remote git apply failed: ${new TextDecoder().decode(apply.stderr)}`);
    }
  }

  const untrackedRaw = runChecked(["git", "-C", repoRoot, "ls-files", "--others", "--exclude-standard"]);
  const untracked = untrackedRaw
    .split(/\r?\n/)
    .map((s) => s.trim())
    .filter((p) => Boolean(p) && !p.startsWith("vendor/"));

  if (untracked.length > 0) {
    const tmpDir = join(repoRoot, "target", "fleet-tmp");
    mkdirSync(tmpDir, { recursive: true });
    const listPath = join(tmpDir, "untracked.txt");
    writeFileSync(listPath, `${untracked.join("\n")}\n`, "utf8");
    const tarCreate = Bun.spawnSync(["tar", "-C", repoRoot, "-cf", "-", "-T", listPath]);
    if (tarCreate.exitCode !== 0) {
      throw new Error(`tar create failed: ${new TextDecoder().decode(tarCreate.stderr)}`);
    }
    const tarExtract = Bun.spawnSync(["ssh", host, `tar -C ${remoteRepo} -xf -`], {
      stdin: tarCreate.stdout,
      stderr: "pipe",
    });
    if (tarExtract.exitCode !== 0) {
      throw new Error(`remote tar extract failed: ${new TextDecoder().decode(tarExtract.stderr)}`);
    }
  }

  const lsSub = runChecked(["git", "-C", repoRoot, "ls-files", "-s", "vendor"]);
  const shaByPath = new Map<string, string>();
  for (const line of lsSub.split(/\r?\n/)) {
    const m = /^160000\s+([0-9a-f]{40})\s+\d+\s+(.+)$/.exec(line.trim());
    if (m) shaByPath.set(m[2], m[1]);
  }
  const specs = parseGitmodules(readFileSync(join(repoRoot, ".gitmodules"), "utf8"))
    .filter((entry) => shaByPath.has(entry.path))
    .map((entry) => ({
      ...entry,
      sha: shaByPath.get(entry.path)!,
      httpsUrl: new URL(entry.url, `${repoBaseUrl}/`).toString(),
    }));

  const remoteSubSync =
    [
      "set -euo pipefail",
      `cd ${remoteRepo}`,
      "git rm -rfq --ignore-unmatch vendor/wgpu || true",
      "if [ -d vendor/wgpu ] && [ ! -e vendor/wgpu/.git ]; then rm -rf vendor/wgpu; fi",
      "sync_one() {",
      '  local path="$1" sha="$2" url="$3" branch="$4"',
      '  mkdir -p "$(dirname "$path")"',
      '  if [ -e "$path/.git" ]; then',
      '    local cur; cur=$(git -C "$path" rev-parse HEAD 2>/dev/null || true)',
      '    if [ "$cur" != "$sha" ]; then',
      '      git -C "$path" fetch --depth 1 "$url" "$sha" >/dev/null 2>&1 || git -C "$path" fetch "$url" >/dev/null 2>&1',
      '      git -C "$path" checkout -qf "$sha"',
      "    fi",
      "  else",
      '    rm -rf "$path"',
      '    if [ -n "$branch" ]; then git clone -q --depth 1 --branch "$branch" "$url" "$path" 2>/dev/null || git clone -q --depth 1 "$url" "$path"; else git clone -q --depth 1 "$url" "$path"; fi',
      '    local cur; cur=$(git -C "$path" rev-parse HEAD 2>/dev/null || true)',
      '    if [ "$cur" != "$sha" ]; then',
      '      git -C "$path" fetch --depth 1 "$url" "$sha" >/dev/null 2>&1 || git -C "$path" fetch "$url" >/dev/null 2>&1',
      '      git -C "$path" checkout -qf "$sha"',
      "    fi",
      "  fi",
      "}",
      "export -f sync_one",
      `printf '%s\\n' ${specs.map((s) => `"${s.path}|${s.sha}|${s.httpsUrl}|${s.branch ?? ""}"`).join(" ")} | xargs -P 8 -I {} bash -c 'IFS="|" read -r p s u b <<< "{}"; sync_one "$p" "$s" "$u" "$b"'`,
      ...specs.map((s) => `git update-index --add --cacheinfo 160000,${s.sha},${s.path}`),
      "git submodule sync --recursive >/dev/null",
    ].join("\n") + "\n";

  const syncRes = Bun.spawnSync(["ssh", host, "bash -s"], {
    stdin: Buffer.from(remoteSubSync, "utf8"),
    stdout: "inherit",
    stderr: "inherit",
  });
  if (syncRes.exitCode !== 0) {
    throw new Error(`remote submodule sync failed (${syncRes.exitCode})`);
  }

  return {
    ok: true,
    host,
    remoteRepo,
    syncedCommit: localHead.slice(0, 10),
    patchBytes: trackedPatch.stdout.length,
    untrackedCount: untracked.length,
    submodules: specs.length,
  };
}

function runVerify(host: string, remoteRepo: string, pullShots: boolean) {
  const script =
    [
      "set -euo pipefail",
      'export PATH="$HOME/.cargo/bin:$HOME/.bun/bin:/usr/local/bin:$PATH"',
      'if command -v sccache >/dev/null 2>&1; then export RUSTC_WRAPPER="$(command -v sccache)"; fi',
      'export SCCACHE_DIR="${SCCACHE_DIR:-/var/cache/aphrody/sccache}"',
      'export SCCACHE_BASEDIRS="${SCCACHE_BASEDIRS:-/srv/xbuild:/home/ubuntu:/srv}"',
      'export TMPDIR="/tmp"',
      `cd ${remoteRepo}`,
      "cargo test --lib -p util -p brand -p gpui_ui -p gpui_store -p os_session -p os_pilot -p os_craft -p os_ui -p os_desktop",
      "cargo test -p os_desktop --test visual_smoke -- --nocapture",
      "cargo clippy --no-deps -p os_session -p os_pilot -p os_craft -p os_ui -p os_desktop -p gpui_store -p gpui_wgpu -p gpui_web -p gpui_ui -p util -p brand -p xtask -- -D warnings",
      "cargo xtask brand check",
      "cargo xtask package-conformity",
      "cargo run -p os_desktop --bin aphrody-os -- doctor --json",
    ].join("\n") + "\n";

  const res = Bun.spawnSync(["ssh", host, "bash -s"], {
    stdin: Buffer.from(script, "utf8"),
    stdout: "inherit",
    stderr: "inherit",
  });
  if (res.exitCode !== 0) process.exit(res.exitCode);

  if (pullShots) {
    const localShots = join(repoRoot, "target", "os_screenshots");
    mkdirSync(localShots, { recursive: true });
    const tar = Bun.spawnSync(["ssh", host, `tar -C ${remoteRepo}/target/os_screenshots -cf - .`]);
    if (tar.exitCode !== 0) throw new Error(new TextDecoder().decode(tar.stderr));
    const ext = Bun.spawnSync(["tar", "-C", localShots, "-xf", "-"], { stdin: tar.stdout });
    if (ext.exitCode !== 0) throw new Error(new TextDecoder().decode(ext.stderr));
  }
}

if (import.meta.main) {
  const args = Bun.argv.slice(2);
  const cmd = args[0] ?? "sync";
  const host = opt("--host") ?? process.env.APHRODY_FLEET_HOST ?? "fr1";
  const pullShots = args.includes("--pull-shots");
  const { slug, repoSlug } = readProductMeta();
  const remoteRepo = resolveRemoteDir(host, slug);

  if (cmd === "sync") {
    console.log(JSON.stringify(syncToHost(host, remoteRepo, repoSlug), null, 2));
  } else if (cmd === "verify") {
    console.log(JSON.stringify(syncToHost(host, remoteRepo, repoSlug), null, 2));
    runVerify(host, remoteRepo, pullShots);
  } else {
    console.error(
      "Usage: bun tooling/bun/fleet-sync.ts [sync|verify] --host <ssh-host> [--remote-dir <path>] [--pull-shots]",
    );
    process.exit(1);
  }
}
