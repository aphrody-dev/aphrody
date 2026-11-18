// The Python environment of the workflow, managed by uv in the user state directory: the package in ./py with
// PyTorch's CUDA build when an NVIDIA driver answers (uv --torch-backend auto) and its CPU build otherwise, and
// TensorFlow with its CUDA extra on Linux with a driver (TensorFlow ships no GPU build for native Windows or macOS).
// The Python version is the newest one both TensorFlow and PyTorch publish wheels for on this platform (PyPI).
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

const PY_DIR = join(import.meta.dir, "py");

export function stateDir(): string {
  const base = process.env.LOCALAPPDATA || process.env.XDG_STATE_HOME || join(homedir(), ".local", "state");
  return join(base, "replica");
}

const venv = () => join(stateDir(), "py");
export const python = () => join(venv(), process.platform === "win32" ? "Scripts/python.exe" : "bin/python");

async function sh(cmd: string[], log?: (s: string) => void): Promise<{ code: number; out: string }> {
  const p = Bun.spawn(cmd, { stdin: "ignore", stdout: "pipe", stderr: "pipe" });
  const [out, err] = await Promise.all([new Response(p.stdout).text(), new Response(p.stderr).text()]);
  const code = await p.exited;
  if (code !== 0 && log) log(`${cmd.slice(0, 4).join(" ")} failed: ${(err || out).trim().split("\n").slice(-3).join(" | ")}`);
  return { code, out };
}

export async function gpu(): Promise<string | null> {
  const r = await sh(["nvidia-smi", "--query-gpu=name", "--format=csv,noheader"]).catch(() => ({ code: 1, out: "" }));
  return r.code === 0 ? r.out.trim().split("\n")[0] || null : null;
}

function platformTag(): RegExp {
  const arm = process.arch === "arm64";
  if (process.platform === "win32") return arm ? /win_arm64/ : /win_amd64/;
  if (process.platform === "darwin") return /macosx_.*_(arm64|universal2)/;
  // glibc wheels; a musl host sets REPLICA_PYTHON
  return arm ? /manylinux.*aarch64/ : /manylinux.*x86_64/;
}

/** Newest CPython minor with wheels of every package's latest release for this platform; REPLICA_PYTHON wins. */
export async function pythonVersion(packages = ["tensorflow", "torch"]): Promise<string> {
  if (process.env.REPLICA_PYTHON) return process.env.REPLICA_PYTHON;
  const tag = platformTag();
  let common: string[] | null = null;
  for (const p of packages) {
    const doc = (await fetch(`https://pypi.org/pypi/${p}/json`, { signal: AbortSignal.timeout(30000) }).then((r) => r.json())) as { urls: { filename: string }[] };
    const tags = new Set(doc.urls.filter((u) => tag.test(u.filename)).flatMap((u) => [...u.filename.matchAll(/-cp3(\d+)-/g)].map((m) => `3.${m[1]}`)));
    common = common ? common.filter((t) => tags.has(t)) : [...tags];
  }
  const best = (common ?? []).sort((a, b) => Number(a.split(".")[1]) - Number(b.split(".")[1])).at(-1);
  if (!best) throw new Error(`no CPython version has wheels of ${packages.join(", ")} for this platform`);
  return best;
}

/** Creates or updates the environment; a marker keyed on pyproject.toml skips the work when nothing changed. */
export async function ensureEnv(log: (s: string) => void): Promise<{ python: string; gpu: string | null }> {
  const card = await gpu();
  const key = createHash("sha256").update(readFileSync(join(PY_DIR, "pyproject.toml"))).update(String(card)).digest("hex");
  const marker = join(venv(), ".replica-ready");
  if (existsSync(marker) && readFileSync(marker, "utf8") === key && existsSync(python())) return { python: python(), gpu: card };
  mkdirSync(stateDir(), { recursive: true });
  if (!existsSync(python())) {
    const version = await pythonVersion();
    log(`python ${version} environment in ${venv()}`);
    if ((await sh(["uv", "venv", venv(), "--python", version], log)).code) throw new Error("uv venv failed");
  }
  const extra = card && process.platform === "linux" ? ["tensorflow[and-cuda]"] : [];
  log(`installing (torch ${card ? `CUDA for ${card}` : "CPU"}, tensorflow ${extra.length ? "CUDA" : "CPU"})`);
  if ((await sh(["uv", "pip", "install", "--python", python(), "--link-mode", "copy", "--torch-backend", "auto", "-e", PY_DIR, ...extra], log)).code) throw new Error("uv pip install failed");
  // crawl4ai's post-install: the Playwright browser it drives
  await sh([join(venv(), process.platform === "win32" ? "Scripts/crawl4ai-setup.exe" : "bin/crawl4ai-setup")], log);
  writeFileSync(marker, key);
  return { python: python(), gpu: card };
}

/** Runs `python -m open_replica <args>`, streaming its progress lines; returns the JSON document it prints last. */
export async function py<T = unknown>(args: string[], log: (s: string) => void): Promise<T> {
  const p = Bun.spawn([python(), "-m", "open_replica", ...args], { cwd: PY_DIR, stdin: "ignore", stdout: "pipe", stderr: "pipe", env: { ...process.env, PYTHONUTF8: "1", PYTHONIOENCODING: "utf-8" } });
  // stderr drained while stdout is read: libraries that log a lot would otherwise fill its pipe and block
  const errText = new Response(p.stderr).text();
  let last = "";
  const decoder = new TextDecoder();
  let buf = "";
  for await (const chunk of p.stdout) {
    buf += decoder.decode(chunk, { stream: true });
    let i: number;
    while ((i = buf.indexOf("\n")) >= 0) {
      const line = buf.slice(0, i).trimEnd();
      buf = buf.slice(i + 1);
      if (!line) continue;
      if (/^[[{]/.test(line)) last = line;
      else log(line);
    }
  }
  if (buf.trim()) last = /^[[{]/.test(buf.trim()) ? buf.trim() : last;
  const err = await errText;
  if ((await p.exited) !== 0) throw new Error(`open-replica ${args[0]}: ${err.trim().split("\n").slice(-5).join(" | ")}`);
  return JSON.parse(last) as T;
}
