#!/usr/bin/env bun
// Absorbs the vendor patch series of the kitesurf submodule into aphrody-labs forks: for every patched crate, the upstream commit the published
// crate was cut from (`.cargo_vcs_info.json` in the crates.io tarball, never typed) is checked out in the fork, and the crate's series is applied
// there with `git am` at the crate's path. One branch per fork (`kitesurf`), one commit per patch with its real author, so the forks carry what
// kitesurf carries and a rebase on a newer upstream is a plain git operation.
//   bun tools/kitesurf-absorb.ts [--kitesurf <checkout>] [--work <dir>] [--push] [--only <crate,...>]
// Run it on a build host (clones are large). The pinned crates come from the checkout's own scripts/get-patched-vendor-crates.mjs, the
// upstream of each crate from its published Cargo.toml `repository`, the fork from the organization (a fork is created when missing).
import { join, resolve } from "node:path";

const ORG = "aphrody-labs";
const BRANCH = "kitesurf";

export type Pin = { name: string; version: string; sha: string; path: string; upstream: string };
export type Group = { sha: string; upstream: string; crates: Pin[] };

/** Crates of one upstream commit, in the order of the checkout's own list (the order the series are meant to land in). */
export function groupPins(pins: Pin[]): Group[] {
  const groups = new Map<string, Group>();
  for (const pin of pins) {
    const key = `${pin.upstream}@${pin.sha}`;
    const group = groups.get(key) ?? { sha: pin.sha, upstream: pin.upstream, crates: [] };
    group.crates.push(pin);
    groups.set(key, group);
  }
  return [...groups.values()];
}

/** `owner/name` of a repository URL as written in a published Cargo.toml. */
export function upstreamOf(url: string): string {
  const match = url.match(/github\.com[/:]([^/]+)\/([^/.\s"]+)/);
  if (!match) throw new Error(`not a GitHub repository: ${url}`);
  return `${match[1]}/${match[2]}`;
}

/** The fork name is the upstream's repository name, lower-cased (the convention of the organization). */
export const forkName = (upstream: string): string => upstream.split("/")[1]!.toLowerCase();

const run = (cwd: string, ...args: string[]) => {
  const result = Bun.spawnSync(args, {
    cwd,
    stdout: "pipe",
    stderr: "pipe",
    windowsHide: true,
    env: { ...Bun.env, GIT_TERMINAL_PROMPT: "0" },
  });
  return { ok: result.exitCode === 0, out: result.stdout.toString().trim(), err: result.stderr.toString().trim() };
};
const must = (cwd: string, ...args: string[]) => {
  const result = run(cwd, ...args);
  if (!result.ok) throw new Error(`${args.slice(0, 3).join(" ")}: ${result.err.slice(0, 400)}`);
  return result.out;
};
const dirExists = async (path: string) =>
  Bun.file(path)
    .stat()
    .then(
      (s) => s.isDirectory(),
      () => false,
    );

async function pinsOf(kitesurf: string, crates: string, only: string[]): Promise<Pin[]> {
  const { CRATES } = (await import(join(kitesurf, "scripts", "get-patched-vendor-crates.mjs"))) as {
    CRATES: { name: string; version: string }[];
  };
  await Bun.$`mkdir -p ${crates}`.quiet();
  const pins: Pin[] = [];
  for (const { name, version } of CRATES) {
    if (only.length && !only.includes(name)) continue;
    const dir = join(crates, `${name}-${version}`);
    if (!(await dirExists(dir))) {
      must(crates, "sh", "-c", `curl -fsSL https://static.crates.io/crates/${name}/${name}-${version}.crate | tar xz`);
    }
    const vcs = (await Bun.file(join(dir, ".cargo_vcs_info.json")).json()) as {
      git: { sha1: string };
      path_in_vcs: string;
    };
    const repository = (await Bun.file(join(dir, "Cargo.toml")).text()).match(/^repository\s*=\s*"([^"]+)"/m)?.[1];
    if (!repository) throw new Error(`${name}-${version} publishes no repository`);
    pins.push({ name, version, sha: vcs.git.sha1, path: vcs.path_in_vcs, upstream: upstreamOf(repository) });
  }
  return pins;
}

/** The fork of an upstream in the organization, created when missing. Returns the fork's owner/name and its real parent. */
function ensureFork(upstream: string): string {
  const name = forkName(upstream);
  const have = run(".", "gh", "api", `repos/${ORG}/${name}`, "--jq", ".parent.full_name");
  if (have.ok) return `${ORG}/${name}`;
  must(".", "gh", "repo", "fork", upstream, "--org", ORG, "--fork-name", name, "--clone=false");
  return `${ORG}/${name}`;
}

/** A published Cargo.toml is the normalized form of the upstream manifest (inlined workspace keys), so a patch hunk on it cannot apply to the source
 * manifest. Those hunks are left out of `git am` and replayed here as text edits of the upstream form, keyed by the patch they come from. */
export type Edit = { file: string; from: string; to: string };
export const MANIFEST_EDITS: Record<string, Edit[]> = {
  "blitz-dom-0.3.0-beta.4/0032": [
    {
      file: "Cargo.toml",
      from: "bitflags = { workspace = true }\n",
      to: 'bitflags = { workspace = true }\nencoding_rs = "0.8"\n',
    },
  ],
  "parley-0.12.0/0006": [
    {
      file: "Cargo.toml",
      from: '"fontique/libm", "parley_engine/libm", "peniko/libm", "dep:core_maths"',
      to: '"fontique/libm", "parley_engine/libm", "peniko/libm", "skrifa/libm", "dep:core_maths"',
    },
    { file: "Cargo.toml", from: '"peniko/std", "parlance/std"]', to: '"peniko/std", "skrifa/std", "parlance/std"]' },
    {
      file: "Cargo.toml",
      from: "hashbrown = { workspace = true }\n\n[dev-dependencies]",
      to: "hashbrown = { workspace = true }\nskrifa = { workspace = true }\n\n[dev-dependencies]",
    },
  ],
  "ratatui-image-8.1.1/0001": [
    {
      file: "Cargo.toml",
      from: 'default = ["image-defaults", "crossterm"]\n',
      to: 'default = ["image-defaults", "crossterm", "query"]\nquery = ["dep:rustix", "dep:windows"]\n',
    },
    {
      file: "Cargo.toml",
      from: 'rustix = { version = "^0.38.4", features',
      to: 'rustix = { version = "^0.38.4", optional = true, features',
    },
    {
      file: "Cargo.toml",
      from: 'windows = { version = "0.58.0", default-features',
      to: 'windows = { version = "0.58.0", optional = true, default-features',
    },
  ],
  "vello_cpu-0.3.0/0001": [
    {
      file: "Cargo.toml",
      from: "u8_pipeline = []\n",
      to: "u8_pipeline = []\n# Polynomial fit of the blurred-rect p-norm, replacing per-pixel powf with a degree-7 coefficient table.\npoly-pnorm = []\n",
    },
  ],
};

/** Applies the edits of one patch to the upstream manifest; a missing anchor is an error (the upstream moved), never a silent skip. */
export function editManifest(text: string, edits: Edit[]): string {
  let result = text;
  for (const edit of edits) {
    if (!result.includes(edit.from)) throw new Error(`manifest anchor not found: ${edit.from.slice(0, 60)}`);
    result = result.replace(edit.from, () => edit.to);
  }
  return result;
}

export type Outcome = {
  fork: string;
  sha: string;
  crates: string[];
  applied: number;
  failed?: { crate: string; patch: string; why: string };
  head?: string;
};

async function absorb(group: Group, work: string, push: boolean): Promise<Outcome> {
  const fork = ensureFork(group.upstream);
  const dir = join(work, forkName(group.upstream));
  const outcome: Outcome = { fork, sha: group.sha, crates: group.crates.map((crate) => crate.name), applied: 0 };
  if (!(await dirExists(join(dir, ".git")))) {
    must(".", "git", "clone", "--quiet", "--filter=blob:none", "--no-checkout", `https://github.com/${fork}.git`, dir);
    must(dir, "git", "remote", "add", "upstream", `https://github.com/${group.upstream}.git`);
  }
  must(dir, "git", "config", "user.name", "aphrody-labs");
  must(dir, "git", "config", "user.email", "bot@aphrody.invalid");
  if (!run(dir, "git", "cat-file", "-e", `${group.sha}^{commit}`).ok) {
    if (!run(dir, "git", "fetch", "--quiet", "origin", group.sha).ok)
      must(dir, "git", "fetch", "--quiet", "upstream", group.sha);
  }
  run(dir, "git", "am", "--abort");
  must(dir, "git", "checkout", "--quiet", "--force", "-B", BRANCH, group.sha);
  for (const crate of group.crates) {
    const series = join(KITESURF, "scripts", "patches", `${crate.name}-${crate.version}`);
    const files = await Array.fromAsync(new Bun.Glob("*.patch").scan({ cwd: series }));
    const patches = files.sort().map((file) => join(series, file));
    const directory = crate.path ? [`--directory=${crate.path}`] : [];
    for (const patch of patches) {
      const key = `${crate.name}-${crate.version}/${patch.split(/[\\/]/).pop()!.slice(0, 4)}`;
      const edits = MANIFEST_EDITS[key];
      const applied = run(
        dir,
        "git",
        "am",
        "--3way",
        "--whitespace=nowarn",
        ...directory,
        ...(edits ? [`--exclude=${join(crate.path, "Cargo.toml")}`] : []),
        patch,
      );
      if (applied.ok && edits) {
        const manifest = join(dir, crate.path, "Cargo.toml");
        await Bun.write(manifest, editManifest(await Bun.file(manifest).text(), edits));
        must(dir, "git", "add", join(crate.path, "Cargo.toml"));
        must(dir, "git", "commit", "--quiet", "--amend", "--no-edit");
      }
      if (!applied.ok) {
        outcome.failed = {
          crate: crate.name,
          patch: patch.split(/[\\/]/).pop()!,
          why: (applied.err || applied.out).split("\n").slice(0, 3).join(" | ").slice(0, 300),
        };
        run(dir, "git", "am", "--abort");
        return outcome;
      }
      outcome.applied++;
    }
  }
  outcome.head = must(dir, "git", "rev-parse", "--short", "HEAD");
  if (push) must(dir, "git", "push", "--quiet", "--force-with-lease", "origin", BRANCH);
  return outcome;
}

let KITESURF = "";
if (import.meta.main) {
  const argv = Bun.argv.slice(2);
  const flag = (name: string) => argv[argv.indexOf(`--${name}`) + 1];
  KITESURF = resolve(
    argv.includes("--kitesurf")
      ? flag("kitesurf")!
      : Bun.fileURLToPath(new URL("../third_party/kitesurf", import.meta.url)),
  );
  const work = resolve(
    argv.includes("--work") ? flag("work")! : Bun.fileURLToPath(new URL("../.cache/kitesurf-absorb", import.meta.url)),
  );
  const only = argv.includes("--only") ? flag("only")!.split(",") : [];
  await Bun.$`mkdir -p ${work}`.quiet();
  const groups = groupPins(await pinsOf(KITESURF, join(work, "crates"), only));
  let failures = 0;
  for (const group of groups) {
    const outcome = await absorb(group, work, argv.includes("--push"));
    if (outcome.failed) failures++;
    console.log(
      `${outcome.fork} @${group.sha.slice(0, 9)} ${outcome.crates.join(",")}: ${outcome.failed ? `FAILED ${outcome.failed.crate} ${outcome.failed.patch}: ${outcome.failed.why}` : `${outcome.applied} patches -> ${BRANCH} ${outcome.head}${argv.includes("--push") ? " (pushed)" : ""}`}`,
    );
  }
  process.exit(failures ? 1 : 0);
}
