#!/usr/bin/env bun
// Prints, and with --apply writes, the .wslconfig lines that make WSL2 boot a custom kernel. The default is a dry run: nothing is
// changed, downloaded or restarted. Run it on the Windows side with Bun.
//   bun install-wsl-kernel.ts --from <dir|url>            show what would change, using manifest.json and the files next to it
//   bun install-wsl-kernel.ts --from <dir|url> --apply    download into the install directory, verify sha256, back up and edit .wslconfig
//   bun install-wsl-kernel.ts --bzimage PATH [--modules PATH] [--cmdline "..."]   point at files you already have
// Afterwards run `wsl --shutdown` and start WSL again; `uname -r` inside it then reports the new release.
import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, join, resolve } from "node:path";

export interface KernelEntries {
  kernel: string;
  kernelModules?: string;
  kernelCommandLine?: string;
}
export interface Manifest {
  release: string;
  files: { name: string; sha256: string; size: number }[];
}

/** .wslconfig wants Windows paths with doubled backslashes. */
export const wslPath = (path: string): string => path.replace(/\//g, "\\").replace(/\\+/g, "\\").replace(/\\/g, "\\\\");

const KEYS = ["kernel", "kernelModules", "kernelCommandLine"] as const;

/** Returns .wslconfig text with the keys set inside [wsl2], every other line kept as it was. */
export function mergeWslConfig(existing: string, entries: KernelEntries): string {
  const eol = existing.includes("\r\n") ? "\r\n" : "\n";
  const lines = existing === "" ? [] : existing.split(/\r?\n/);
  if (lines.at(-1) === "") lines.pop();
  const wanted = new Map<string, string>();
  for (const key of KEYS) if (entries[key] !== undefined) wanted.set(key, `${key}=${entries[key]}`);
  const sectionOf = (line: string): string | null => /^\s*\[([^\]]+)\]\s*$/.exec(line)?.[1]?.toLowerCase() ?? null;

  let start = lines.findIndex((l) => sectionOf(l) === "wsl2");
  if (start === -1) {
    if (lines.length && lines.at(-1)!.trim() !== "") lines.push("");
    lines.push("[wsl2]");
    start = lines.length - 1;
  }
  let end = lines.findIndex((l, i) => i > start && sectionOf(l) !== null);
  if (end === -1) end = lines.length;
  const done = new Set<string>();
  for (let i = start + 1; i < end; i++) {
    const key = /^\s*([A-Za-z]+)\s*=/.exec(lines[i]!)?.[1];
    const match = key ? KEYS.find((k) => k.toLowerCase() === key.toLowerCase()) : undefined;
    if (match && wanted.has(match)) {
      lines[i] = wanted.get(match)!;
      done.add(match);
    }
  }
  const added = [...wanted].filter(([k]) => !done.has(k)).map(([, v]) => v);
  let insertAt = end;
  while (insertAt > start + 1 && lines[insertAt - 1]!.trim() === "") insertAt--;
  lines.splice(insertAt, 0, ...added);
  return `${lines.join(eol)}${eol}`;
}

export function sha256Hex(bytes: Uint8Array): string {
  return new Bun.CryptoHasher("sha256").update(bytes).digest("hex");
}

async function readSource(from: string, name: string): Promise<Uint8Array> {
  if (/^https?:\/\//.test(from)) {
    const res = await fetch(`${from.replace(/\/$/, "")}/${name}`);
    if (!res.ok) throw new Error(`${from}/${name}: HTTP ${res.status}`);
    return new Uint8Array(await res.arrayBuffer());
  }
  return new Uint8Array(readFileSync(join(from, name)));
}

function flag(args: string[], name: string): string | undefined {
  const i = args.indexOf(`--${name}`);
  return i >= 0 && args[i + 1] && !args[i + 1]!.startsWith("--") ? args[i + 1] : undefined;
}

async function main(args: string[]): Promise<number> {
  const apply = args.includes("--apply");
  const home = Bun.env.USERPROFILE ?? Bun.env.HOME;
  if (!home) throw new Error("neither USERPROFILE nor HOME is set; pass --wslconfig PATH");
  const wslconfig = resolve(flag(args, "wslconfig") ?? join(home, ".wslconfig"));
  const cmdline = flag(args, "cmdline");
  const from = flag(args, "from");
  let kernel = flag(args, "bzimage");
  let modules = flag(args, "modules");

  if (from) {
    const manifest = JSON.parse(new TextDecoder().decode(await readSource(from, "manifest.json"))) as Manifest;
    const dir = resolve(flag(args, "install-dir") ?? join(home, "wsl-kernel", manifest.release));
    console.log(`release ${manifest.release}; install directory ${dir}`);
    for (const file of manifest.files) {
      const bytes = apply ? await readSource(from, file.name) : null;
      if (bytes && sha256Hex(bytes) !== file.sha256) throw new Error(`${file.name}: sha256 mismatch, refusing to continue`);
      if (bytes) {
        mkdirSync(dir, { recursive: true });
        writeFileSync(join(dir, file.name), bytes);
      }
      console.log(`${apply ? "wrote" : "would download"} ${file.name} (${file.size} bytes, sha256 ${file.sha256})`);
    }
    kernel = join(dir, "bzImage");
    if (manifest.files.some((f) => f.name === "modules.vhdx")) modules = join(dir, "modules.vhdx");
  }
  if (!kernel) {
    console.error("usage: bun install-wsl-kernel.ts (--from <dir|url> | --bzimage PATH [--modules PATH]) [--cmdline \"...\"] [--wslconfig PATH] [--apply]");
    return 2;
  }

  const entries: KernelEntries = { kernel: wslPath(resolve(kernel)), ...(modules ? { kernelModules: wslPath(resolve(modules)) } : {}), ...(cmdline ? { kernelCommandLine: cmdline } : {}) };
  const before = existsSync(wslconfig) ? readFileSync(wslconfig, "utf8") : "";
  const after = mergeWslConfig(before, entries);
  console.log(`\n${wslconfig} ${apply ? "becomes" : "would become"}:\n`);
  console.log(after);
  if (!apply) {
    console.log("Dry run: nothing was changed. Add --apply to write it (a backup is kept next to the file).");
    return 0;
  }
  if (existsSync(wslconfig)) copyFileSync(wslconfig, `${wslconfig}.bak-${new Date().toISOString().replace(/[:.]/g, "-")}`);
  writeFileSync(wslconfig, after);
  console.log(`Written. Run "wsl --shutdown", start WSL again, then check "uname -r" (expected: ${basename(resolve(kernel)) === "bzImage" ? "the release in manifest.json" : "your kernel"}).`);
  return 0;
}

if (import.meta.main) process.exit(await main(Bun.argv.slice(2)).catch((err) => (console.error(err instanceof Error ? err.message : err), 1)));
