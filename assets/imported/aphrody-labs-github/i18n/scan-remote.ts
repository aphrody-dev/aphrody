#!/usr/bin/env bun
// Clone the non-archived repositories of GitHub owners on a fleet host and run check.ts there.
//
//   bun i18n/scan-remote.ts --host fr1 --dir /srv/xbuild/i18n --owners aphrody-labs \
//       --max-fork-mb 200 --out result.json
//
// Files travel over ssh stdin and stdout (no scp). The token never reaches a command line or the remote environment listing: it is sent on stdin.
import { join } from "node:path";

interface Repo {
  name: string;
  nameWithOwner: string;
  isArchived: boolean;
  isFork: boolean;
  diskUsage: number;
}
export interface Skipped {
  repo: string;
  reason: string;
}

const arg = (name: string, fallback?: string) => {
  const i = process.argv.indexOf(name);
  return i >= 0 ? process.argv[i + 1] : fallback;
};

const sh = (cmd: string[], input?: string) => {
  const p = Bun.spawnSync(cmd, { stdin: input ? Buffer.from(input) : undefined, stdout: "pipe", stderr: "pipe" });
  if (p.exitCode !== 0) throw new Error(`${cmd.join(" ")}: ${p.stderr.toString()}`);
  return p.stdout.toString();
};

export function selectRepos(repos: Repo[], maxForkMb: number) {
  const take: Repo[] = [];
  const skipped: Skipped[] = [];
  for (const r of repos) {
    if (r.isArchived) skipped.push({ repo: r.nameWithOwner, reason: "archived" });
    else if (r.isFork && r.diskUsage / 1024 > maxForkMb) skipped.push({ repo: r.nameWithOwner, reason: `fork larger than ${maxForkMb} MB (${Math.round(r.diskUsage / 1024)} MB)` });
    else take.push(r);
  }
  return { take, skipped };
}

if (import.meta.main) {
  const host = arg("--host", "fr1")!;
  const dir = arg("--dir", "/srv/xbuild/i18n")!;
  const owners = arg("--owners", "aphrody-labs")!.split(",");
  const maxForkMb = Number(arg("--max-fork-mb", "200"));
  const out = arg("--out", "i18n-remote.json")!;
  const jobs = Number(arg("--jobs", "4"));

  const all: Repo[] = owners.flatMap((o) =>
    JSON.parse(sh(["gh", "repo", "list", o, "--limit", "1000", "--json", "name,nameWithOwner,isArchived,isFork,diskUsage"])),
  );
  const { take, skipped } = selectRepos(all, maxForkMb);
  const token = sh(["gh", "auth", "token"]).trim();

  sh(["ssh", host, `mkdir -p ${dir}/_tool ${dir}/_out`]);
  for (const file of ["check.ts", "i18n.toml"])
    sh(["ssh", host, `cat > ${dir}/_tool/${file}`], await Bun.file(join(import.meta.dir, file)).text());

  const list = take.map((r) => r.nameWithOwner).join("\n");
  const script = `
cd ${dir}
cat > _list <<'EOF_LIST'
${list}
EOF_LIST
one() {
  repo="$1"; name="\${repo//\\//__}"
  if [ ! -d "$name/.git" ]; then
    git -c http.extraheader="Authorization: Basic $AUTH" clone -q --depth 1 --no-recurse-submodules "https://github.com/$repo.git" "$name" 2>/dev/null || { echo "$repo clone-failed" >> _out/_failed; return; }
  fi
  bun _tool/check.ts "$PWD/$name" --quiet --fail-on none --json "_out/$name.json" 2>/dev/null || echo "$repo check-failed" >> _out/_failed
}
AUTH=$(printf "x-access-token:%s" "$TOKEN" | base64 -w0)
export -f one; export AUTH
rm -f _out/_failed
xargs -a _list -P ${jobs} -I{} bash -c 'one {}' < /dev/null
bun -e '
const fs = require("fs");
const res = [];
for (const f of fs.readdirSync("_out")) if (f.endsWith(".json") && f !== "result.json") { const r = JSON.parse(fs.readFileSync("_out/" + f, "utf8"))[0]; r.repo = f.replace(/\\.json$/, "").replace("__", "/"); res.push(r); }
const failed = fs.existsSync("_out/_failed") ? fs.readFileSync("_out/_failed", "utf8").trim().split("\\n") : [];
fs.writeFileSync("_out/result.json", JSON.stringify({ repos: res, failed }, null, 2));
'
`;
  sh(["ssh", host, "bash -s"], `set -u\nread -r TOKEN\n${token}\n${script}`);
  const { repos, failed } = JSON.parse(sh(["ssh", host, `cat ${dir}/_out/result.json`]));
  await Bun.write(out, `${JSON.stringify({ host, dir, owners, skipped, failed, repos }, null, 2)}\n`);
  console.error(`${repos.length} scanned, ${skipped.length} skipped, ${failed.length} failed -> ${out}`);
}
