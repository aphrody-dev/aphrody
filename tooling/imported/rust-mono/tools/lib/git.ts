// Git plumbing for the superproject. Never touches a submodule working tree: gitlinks are written straight into the index.

async function git(cwd: string, args: string[], input?: string): Promise<string> {
  const p = Bun.spawn(["git", ...args], {
    cwd,
    stdin: input === undefined ? "ignore" : new Blob([input]),
    stdout: "pipe",
    stderr: "pipe",
  });
  const [out, err, code] = await Promise.all([new Response(p.stdout).text(), new Response(p.stderr).text(), p.exited]);
  if (code !== 0) throw new Error(`git ${args.join(" ")} failed (${code}): ${err.trim()}`);
  return out;
}

/** Gitlinks (mode 160000) in the index: path to pinned commit. */
export async function gitlinks(cwd: string): Promise<Map<string, string>> {
  const out = await git(cwd, ["ls-files", "-s"]);
  const m = new Map<string, string>();
  for (const line of out.split("\n")) {
    const hit = /^160000 ([0-9a-f]{40}) \d\t(.+)$/.exec(line);
    if (hit) m.set(hit[2]!, hit[1]!);
  }
  return m;
}

export async function setGitlink(cwd: string, path: string, sha: string): Promise<void> {
  await git(cwd, ["update-index", "--add", "--cacheinfo", `160000,${sha},${path}`]);
}

/** Paths staged in the index relative to HEAD (all of them when there is no commit yet). */
export async function stagedPaths(cwd: string): Promise<string[]> {
  const out = await git(cwd, ["diff", "--cached", "--name-only", "--no-renames"]);
  return out.split("\n").filter(Boolean);
}

export async function stage(cwd: string, paths: string[]): Promise<void> {
  await git(cwd, ["add", "--", ...paths]);
}

export async function commit(cwd: string, message: string): Promise<string> {
  await git(cwd, ["commit", "-q", "-m", message]);
  return (await git(cwd, ["rev-parse", "HEAD"])).trim();
}

export async function repoRoot(cwd: string): Promise<string> {
  return (await git(cwd, ["rev-parse", "--show-toplevel"])).trim();
}
