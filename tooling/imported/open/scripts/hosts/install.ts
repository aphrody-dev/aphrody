// Idempotent global install of the benchmark tools listed in tools.json, on the host it runs on (Windows, Linux,
// macOS). GitHub releases: latest release (or latest with `tagPrefix`), asset picked by platform regex, archive
// unpacked into <bin>. Python tools through `uv tool install`, npm tools through `bun add -g`. Each tool is then
// verified by running its `verify` command. `--dry-run` only prints the chosen release asset per tool.
// bun scripts/hosts/install.ts [--dry-run] [--only a,b]
import { chmodSync, cpSync, existsSync, mkdirSync, mkdtempSync, readdirSync, renameSync, rmSync, statSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
import { basename, join } from "node:path";

const cfg = await Bun.file(join(import.meta.dir, "tools.json")).json();
const argv = Bun.argv.slice(2);
const dry = argv.includes("--dry-run");
const only = argv.includes("--only") ? argv[argv.indexOf("--only") + 1]!.split(",") : null;
const os = process.platform === "win32" ? "windows" : process.platform === "darwin" ? "macos" : "linux";
const arch = process.arch === "arm64" ? "arm64" : "x64";
const platform = `${os}-${arch}`;
const bin = cfg.bin.replace(/^~/, homedir());
const exe = (n: string) => (os === "windows" ? `${n}.exe` : n);
const token = Bun.env.GH_TOKEN ?? Bun.spawnSync(["gh", "auth", "token"]).stdout.toString().trim();
const gh = { accept: "application/vnd.github+json", "user-agent": "aphrody-open-installer", ...(token ? { authorization: `Bearer ${token}` } : {}) };
mkdirSync(bin, { recursive: true });

const run = (cmd: string[]) => {
  const r = Bun.spawnSync(cmd, { stdout: "pipe", stderr: "pipe" });
  return { ok: r.exitCode === 0, out: `${r.stdout}${r.stderr}`.trim() };
};

async function release(t: any) {
  if (!t.tagPrefix) return (await fetch(`https://api.github.com/repos/${t.github}/releases/latest`, { headers: gh })).json();
  const list: any[] = await (await fetch(`https://api.github.com/repos/${t.github}/releases?per_page=30`, { headers: gh })).json();
  return list.find((r) => r.tag_name.startsWith(t.tagPrefix) && !r.prerelease && !r.draft);
}

/** Every file under dir (recursive). */
const files = (d: string): string[] => readdirSync(d).flatMap((n) => (statSync(join(d, n)).isDirectory() ? files(join(d, n)) : [join(d, n)]));

async function fromRelease(t: any) {
  const re = t.asset?.[platform];
  if (!re) return `skipped: ${t.unsupported?.[platform] ?? `no asset rule for ${platform}`}`;
  const rel = await release(t);
  const asset = rel?.assets?.find((a: any) => new RegExp(re, "i").test(a.name));
  if (!asset) return `FAIL: no asset matching /${re}/ in ${t.github} ${rel?.tag_name}`;
  if (dry) return `${rel.tag_name} ${asset.name}`;
  const work = mkdtempSync(join(tmpdir(), `tool-${t.name}-`));
  const file = join(work, asset.name);
  await Bun.write(file, await (await fetch(asset.browser_download_url, { headers: { "user-agent": "aphrody-open-installer" } })).arrayBuffer());
  const out = join(work, "x");
  mkdirSync(out);
  if (/\.(zip|tar\.gz|tgz|tar\.xz)$/i.test(asset.name)) run(["tar", "-xf", file, "-C", out]);
  else if (/\.gz$/i.test(asset.name)) await Bun.write(join(out, exe(t.name)), Bun.gunzipSync(await Bun.file(file).bytes()));
  else renameSync(file, join(out, exe(t.name)));
  const binName = exe(t.bin ?? t.name);
  const found = files(out).find((f) => basename(f).toLowerCase() === binName.toLowerCase()) ?? files(out).find((f) => basename(f).toLowerCase().startsWith((t.bin ?? t.name).toLowerCase()));
  if (!found) return `FAIL: ${binName} not found in ${asset.name}`;
  if (t.keepDir) {
    // the binary needs its sibling files (helix runtime/): install the whole directory and link the binary
    const dir = join(bin, `${t.name}.d`);
    rmSync(dir, { recursive: true, force: true });
    cpSync(join(found, ".."), dir, { recursive: true });
    if (os === "windows") await Bun.write(join(bin, `${t.name}.cmd`), `@"%~dp0${t.name}.d\\${binName}" %*\r\n`);
    else { rmSync(join(bin, t.name), { force: true }); Bun.spawnSync(["ln", "-s", join(dir, binName), join(bin, t.name)]); }
  } else {
    const dest = join(bin, binName);
    rmSync(dest, { force: true });
    cpSync(found, dest);
    if (os !== "windows") chmodSync(dest, 0o755);
  }
  rmSync(work, { recursive: true, force: true });
  return `${rel.tag_name} ${asset.name}`;
}

const results: Record<string, string> = {};
for (const t of cfg.tools) {
  if (only && !only.includes(t.name)) continue;
  let r: string;
  if (t.github) r = await fromRelease(t);
  else if (t.uv) r = dry ? `uv tool install ${t.uv}` : run(["uv", "tool", "install", "--upgrade", t.uv]).ok ? `uv ${t.uv}` : `FAIL: uv ${t.uv}`;
  else if (t.bun) r = dry ? `bun add -g ${t.bun}` : run(["bun", "add", "-g", `${t.bun}@latest`]).ok ? `bun ${t.bun}` : `FAIL: bun ${t.bun}`;
  else r = "FAIL: no install method";
  if (!dry && !r.startsWith("FAIL") && !r.startsWith("skipped")) {
    const v = run(t.verify);
    r += v.ok ? ` | ${v.out.split("\n")[0]}` : ` | FAIL verify: ${v.out.slice(0, 120)}`;
  }
  results[t.name] = r;
  console.log(`${t.name}: ${r}`);
}
const host = (await Bun.$`hostname`.text()).trim();
await Bun.write(join(import.meta.dir, "../../docs/ideas/inventory", `tools-${host}.json`), `${JSON.stringify({ host, platform, dry, results }, null, 2)}\n`);
