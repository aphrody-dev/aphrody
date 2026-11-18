// Gate: no private data of the machine running it in tracked files. The patterns are read from this machine at run
// time and never written down: user name, home directory, the folders this checkout lives in, host name, git
// identity, the Host aliases and HostName values of ~/.ssh/config, and the nodes, addresses and aliases of a fleet
// file given with --fleet (with the /24 networks of its addresses).
// Strong terms (folders, e-mail, addresses) are findings anywhere, in any case. Weak terms (bare lower-case words:
// ssh aliases, node names, a host name) are findings only where they name a host, in their own case: ssh/scp/rsync
// targets, user@host, host-valued keys and flags, quoted literals, "on/to/from/via/at <host>", "(<host>)".
// A weak term that is a command on PATH or a part of the repository's own name is public, not private.
// bun scripts/check/private.ts [--fleet <file>] [--paths a,b] [--json]
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { parseArgs } from "node:util";
import { privateMatchers } from "./private-terms.ts";

const { values: a } = parseArgs({ options: { fleet: { type: "string" }, paths: { type: "string" }, json: { type: "boolean", default: false } } });
const root = join(import.meta.dir, "../..");
const sh = (cmd: string[]) => Bun.spawnSync(cmd, { cwd: root, stdout: "pipe", stderr: "ignore" }).stdout.toString().trim();
const { terms, strongRe, weakRe, kindOf } = privateMatchers(root, a.fleet);

// files: every tracked file
const tracked = (a.paths ? sh(["git", "ls-files", "--", ...a.paths.split(",")]) : sh(["git", "ls-files"])).split("\n");
const files = tracked.filter(Boolean);

const hits: { file: string; line: number; kind: string }[] = [];
for (const f of files) {
  let text: string;
  try {
    const buf = readFileSync(join(root, f));
    if (buf.length > 4 * 2 ** 20 || buf.includes(0)) continue;
    text = buf.toString("utf8");
  } catch {
    continue;
  }
  // test() moves lastIndex and matchAll() copies it: reset before each use
  strongRe.lastIndex = weakRe.lastIndex = 0;
  const found = strongRe.test(text) || weakRe.test(text);
  if (!found) continue;
  text.split("\n").forEach((l, i) => {
    strongRe.lastIndex = weakRe.lastIndex = 0;
    for (const m of [...l.matchAll(strongRe), ...l.matchAll(weakRe)]) hits.push({ file: f, line: i + 1, kind: kindOf(m.slice(1).find(Boolean) ?? "") });
  });
}
if (a.json) console.log(JSON.stringify({ terms: terms.size, files: files.length, hits }, null, 2));
else {
  for (const h of hits) console.log(`${h.file}:${h.line}: ${h.kind}`);
  console.log(`${hits.length} hits in ${new Set(hits.map((h) => h.file)).size} files (${files.length} files scanned, ${terms.size} private terms of this machine)`);
}
process.exitCode = hits.length ? 1 : 0;
