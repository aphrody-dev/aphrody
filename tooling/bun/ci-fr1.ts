/**
 * Shared helpers for the scripts that run on the build host (default `fr1`, override with $CI_HOST):
 * run a bash script over ssh with its output streamed, and pin `/srv/xbuild/aphrody-ci/wasm/{aphrody,fw}` to origin/main.
 * Not a CLI; imported by av-wasm.ts, web-size.ts and wasm-check-fr1.ts.
 */
export const host = process.env.CI_HOST ?? "fr1";
/** Work area on the build host: `/` is small, `/srv/xbuild` is the 800 GB XFS volume for builds (override with $CI_ROOT). */
export const ciRoot = process.env.CI_ROOT ?? "/srv/xbuild/aphrody-ci";
export const wasmTargetDir = `${ciRoot}/target-wasm`;

/** Runs `script` with `bash -s` on the build host; stdout is returned and, unless `quiet`, echoed to stderr as it arrives. */
export async function remote(script: string, quiet = false): Promise<{ code: number; stdout: string }> {
  const proc = Bun.spawn(["ssh", "-o", "BatchMode=yes", host, "bash", "-s"], { stdin: "pipe", stdout: "pipe", stderr: "inherit" });
  proc.stdin.write(`export PATH=$HOME/.local/bin:$HOME/.cargo/bin:$PATH\n${script}\n`);
  proc.stdin.end();
  const decoder = new TextDecoder();
  let stdout = "";
  for await (const chunk of proc.stdout) {
    const text = decoder.decode(chunk, { stream: true });
    stdout += text;
    if (!quiet) process.stderr.write(text);
  }
  return { code: await proc.exited, stdout };
}

/** Fetches origin/main of aphrody and fw (shallow) into `$CI_ROOT/wasm/{aphrody,fw}`, discarding local edits; prints the two short hashes. */
export const syncRepos = `
set -eu
mkdir -p ${ciRoot}/wasm && cd ${ciRoot}/wasm
for repo in aphrody fw; do
  if [ -d $repo/.git ]; then git -C $repo fetch -q --depth 1 origin main && git -C $repo checkout -q --force FETCH_HEAD
  else git clone -q --depth 1 https://github.com/aphrody-labs/$repo.git $repo; fi
  echo "$repo $(git -C $repo rev-parse --short=10 HEAD)"
done
`;

/** Reads a binary file from the build host. */
export async function pull(path: string): Promise<Uint8Array> {
  const proc = Bun.spawn(["ssh", "-o", "BatchMode=yes", host, "cat", path], { stdout: "pipe", stderr: "inherit" });
  const bytes = new Uint8Array(await new Response(proc.stdout).arrayBuffer());
  if ((await proc.exited) !== 0) throw new Error(`ssh ${host} cat ${path} failed`);
  return bytes;
}
