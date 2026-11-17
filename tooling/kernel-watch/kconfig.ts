#!/usr/bin/env bun
// Kconfig dataset: for every symbol a fragment requests, what each fragment wants, what the final merged .config holds, what the tree's own
// defaults (defconfig) hold, and why we set it (the fragment name is the rationale tag: required-by:<fragment>, perf, rust, ...).
// Source of the values: .config files written by `make olddefconfig` in a kernel tree (scripts/kconfig resolves dependencies), never parsed
// from Kconfig files here; symbol existence is therefore whatever the tree's Kconfig accepts.
//   bun kconfig.ts --final OUT/.config --defaults DEFCONFIG/.config --fragments DIR [--arch-base BASE.config] [--out FILE]
import { readdirSync, readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { basename, dirname, join } from "node:path";

export type Value = string | "n" | null;
export interface Entry {
  symbol: string;
  requested: Record<string, string>;
  final: Value;
  defaults: Value;
  base: Value;
  applied: boolean;
  deltaVsDefaults: boolean;
  deltaVsBase: boolean | null;
  rationale: string[];
}

const SYMBOL = /^(?:# )?(CONFIG_[A-Za-z0-9_]+)(?:=(.*)| is not set)$/;

export function parseConfig(text: string): Map<string, string> {
  const out = new Map<string, string>();
  for (const line of text.split(/\r?\n/)) {
    const m = SYMBOL.exec(line.trim());
    if (m) out.set(m[1]!, m[2] ?? "n");
  }
  return out;
}

const RATIONALE: Record<string, string> = { rust: "rust", android: "required-by:android", wayland: "required-by:wayland", optimize: "perf", wsl: "required-by:wsl" };
export const rationaleOf = (fragment: string): string => RATIONALE[fragment] ?? `required-by:${fragment}`;

export function buildDataset(fragments: Map<string, Map<string, string>>, final: Map<string, string>, defaults: Map<string, string>, base: Map<string, string> | null): Entry[] {
  const symbols = new Set<string>();
  for (const f of fragments.values()) for (const s of f.keys()) symbols.add(s);
  return [...symbols].sort().map((symbol): Entry => {
    const requested: Record<string, string> = {};
    for (const [name, f] of fragments) if (f.has(symbol)) requested[name] = f.get(symbol)!;
    const get = (m: Map<string, string> | null): Value => (m === null ? null : (m.get(symbol) ?? "n"));
    const now = get(final);
    const wanted = Object.values(requested);
    return {
      symbol, requested, final: now, defaults: get(defaults), base: get(base),
      applied: wanted.every((w) => w === now),
      deltaVsDefaults: now !== get(defaults),
      deltaVsBase: base === null ? null : now !== get(base),
      rationale: Object.keys(requested).map(rationaleOf),
    };
  });
}

export function summarize(entries: Entry[]): { symbols: number; applied: number; notApplied: string[]; changedVsDefaults: number; changedVsBase: number } {
  return {
    symbols: entries.length,
    applied: entries.filter((e) => e.applied).length,
    notApplied: entries.filter((e) => !e.applied).map((e) => `${e.symbol}: wanted ${Object.values(e.requested).join("/")}, got ${e.final}`),
    changedVsDefaults: entries.filter((e) => e.deltaVsDefaults).length,
    changedVsBase: entries.filter((e) => e.deltaVsBase).length,
  };
}

function flag(args: string[], name: string): string | undefined {
  const i = args.indexOf(`--${name}`);
  return i >= 0 ? args[i + 1] : undefined;
}

if (import.meta.main) {
  const args = Bun.argv.slice(2);
  const finalPath = flag(args, "final"), defaultsPath = flag(args, "defaults"), dir = flag(args, "fragments");
  if (!finalPath || !defaultsPath || !dir) {
    console.error("usage: bun kconfig.ts --final .config --defaults .config --fragments DIR [--arch-base .config] [--out FILE]");
    process.exit(2);
  }
  const fragments = new Map<string, Map<string, string>>();
  for (const f of readdirSync(dir).filter((n) => n.endsWith(".config")).sort()) fragments.set(basename(f, ".config"), parseConfig(readFileSync(join(dir, f), "utf8")));
  const basePath = flag(args, "arch-base");
  const entries = buildDataset(fragments, parseConfig(readFileSync(finalPath, "utf8")), parseConfig(readFileSync(defaultsPath, "utf8")), basePath ? parseConfig(readFileSync(basePath, "utf8")) : null);
  const summary = summarize(entries);
  const out = flag(args, "out");
  if (out) {
    mkdirSync(dirname(out), { recursive: true });
    writeFileSync(out, `${JSON.stringify({ summary, entries }, null, 2)}\n`);
  }
  console.log(JSON.stringify(summary, null, 2));
}
