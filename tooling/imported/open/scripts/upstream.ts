// Rewrites the pins table of docs/upstream.md from Cargo.toml and, with --check, resolves each pinned git revision
// against its repository (live) and reports how far the default branch has moved.
//
//   bun scripts/upstream.ts [--check]
import { join } from "node:path";

const root = join(import.meta.dir, "..");
const manifest = await Bun.file(join(root, "Cargo.toml")).text();
const pins = new Map<string, { repo: string; rev: string; crates: string[] }>();
for (const m of manifest.matchAll(/^([\w-]+) = \{ git = "https:\/\/github\.com\/([^"]+)", rev = "([0-9a-f]{40})"/gm)) {
  const key = `${m[2]}@${m[3]}`;
  const pin = pins.get(key) ?? { repo: m[2]!, rev: m[3]!, crates: [] };
  if (!pin.crates.includes(m[1]!)) pin.crates.push(m[1]!);
  pins.set(key, pin);
}
const crates = [...manifest.matchAll(/^(opentunnel|adblock) = "([^"]+)"/gm)].map((m) => `| ${m[1]} | crates.io | \`${m[2]}\` | |`);

const rows: string[] = [];
for (const pin of pins.values()) {
  let moved = "";
  if (Bun.argv.includes("--check")) {
    const head = (await (await fetch(`https://api.github.com/repos/${pin.repo}/commits/HEAD`, { headers: { "User-Agent": "open-upstream" } })).json()) as { sha?: string; commit?: { committer?: { date?: string } } };
    moved = head.sha === pin.rev ? "at head" : `head ${head.sha?.slice(0, 10)} (${head.commit?.committer?.date})`;
  }
  rows.push(`| ${pin.crates.join(", ")} | ${pin.repo} | \`${pin.rev.slice(0, 10)}\` | ${moved} |`);
}
const table = ["| Crates | Source | Pin | Head |", "| :-- | :-- | :-- | :-- |", ...rows, ...crates].join("\n");
const file = join(root, "docs", "upstream.md");
const text = await Bun.file(file).text();
await Bun.write(file, text.replace(/<!-- pins:begin -->[\s\S]*<!-- pins:end -->/, `<!-- pins:begin -->\n${table}\n<!-- pins:end -->`));
console.log(table);
