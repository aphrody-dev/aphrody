#!/usr/bin/env bun
// Apply check.ts --fix to repositories already cloned by scan-remote.ts, commit the changed paths on main and push.
//
//   bun i18n/fix-remote.ts --host fr1 --dir /srv/xbuild/i18n --repos aphrody-labs/aphrody-m3,aphrody-labs/yolo --out after.json
//
// Commit identity is the PC's git identity; the token travels on ssh stdin only.
export {};

const arg = (name: string, fallback?: string) => {
  const i = process.argv.indexOf(name);
  return i >= 0 ? process.argv[i + 1] : fallback;
};
const sh = (cmd: string[], input?: string) => {
  const p = Bun.spawnSync(cmd, { stdin: input ? Buffer.from(input) : undefined, stdout: "pipe", stderr: "pipe" });
  if (p.exitCode !== 0) throw new Error(`${cmd.join(" ")}: ${p.stderr.toString()}`);
  return p.stdout.toString();
};

const host = arg("--host", "fr1")!;
const dir = arg("--dir", "/srv/xbuild/i18n")!;
const repos = arg("--repos", "")!.split(",").filter(Boolean);
const out = arg("--out", "i18n-remote-after.json")!;
const message = arg("--message", "i18n: UTF-8 and LF repository hygiene (W3C Internationalization), .editorconfig and .gitattributes")!;
const gitCfg = (key: string) => sh(["git", "-C", import.meta.dir, "config", key]).trim();
const token = sh(["gh", "auth", "token"]).trim();

// The checker travels with the call so the clones use the current rules.
const here = import.meta.dir;
for (const file of ["check.ts", "i18n.toml"]) sh(["ssh", host, `cat > ${dir}/_tool/${file}`], await Bun.file(`${here}/${file}`).text());

const script = `AUTH=$(printf "x-access-token:%s" "$TOKEN" | base64 -w0)
cd ${dir}
for repo in ${repos.join(" ")}; do
  name="\${repo//\\//__}"
  cd "$name" || continue
  git fetch -q --depth 1 origin 2>/dev/null; git checkout -q main 2>/dev/null || true
  bun ../_tool/check.ts "$PWD" --fix --quiet --fail-on none --json "../_out/after-$name.json" < /dev/null
  bun -e 'const fs=require("fs");const r=JSON.parse(fs.readFileSync(process.argv[1],"utf8"))[0];fs.writeFileSync("../_out/paths.txt",(r.fixed||[]).join("\\0"));(r.fixed||[]).length&&fs.writeFileSync("../_out/fixed-"+process.argv[2]+".json",JSON.stringify(r.fixed))' "../_out/after-$name.json" "$name"
  if [ -s ../_out/paths.txt ]; then
    git add --pathspec-from-file=../_out/paths.txt --pathspec-file-nul
    git -c user.name=${JSON.stringify(gitCfg("user.name"))} -c user.email=${JSON.stringify(gitCfg("user.email"))} commit -q -m ${JSON.stringify(message)} --pathspec-from-file=../_out/paths.txt --pathspec-file-nul
    git -c http.extraheader="Authorization: Basic $AUTH" push -q origin HEAD:main 2>&1 | tail -2
  fi
  bun ../_tool/check.ts "$PWD" --quiet --fail-on none --json "../_out/after-$name.json" < /dev/null
  cd ..
done
bun -e '
const fs = require("fs"); const res = [];
for (const f of fs.readdirSync("_out")) if (f.startsWith("after-")) { const r = JSON.parse(fs.readFileSync("_out/" + f, "utf8"))[0]; const n = f.slice(6, -5); r.repo = n.replace("__", "/"); r.fixed = fs.existsSync("_out/fixed-" + n + ".json") ? JSON.parse(fs.readFileSync("_out/fixed-" + n + ".json", "utf8")) : []; res.push(r); }
fs.writeFileSync("_out/after.json", JSON.stringify({ repos: res }, null, 2));
'
`;
sh(["ssh", host, "bash -s"], `set -u\nread -r TOKEN\n${token}\n${script}`);
await Bun.write(out, sh(["ssh", host, `cat ${dir}/_out/after.json`]));
console.error(`fixed ${repos.length} repositories -> ${out}`);
