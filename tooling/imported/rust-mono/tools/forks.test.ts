import { expect, test } from "bun:test";
import { resolve } from "node:path";
import { run, parseArgs } from "./forks.ts";
import { fixture, type Recording } from "./lib/gh.ts";
import { judge } from "./lib/inspect.ts";
import { parseManifest } from "./lib/manifest.ts";

const fx = (n: string) => resolve(import.meta.dir, "fixtures", n);
const recorded = async () => JSON.parse(await Bun.file(fx("forks-api.json")).text()) as Recording;
const ONLY = "rust,rust-lang-llvm-project,cargo,libc";

async function capture(fn: () => Promise<number>): Promise<{ code: number; out: string }> {
  const lines: string[] = [];
  const log = console.log;
  console.log = (...a: unknown[]) => void lines.push(a.join(" "));
  try {
    return { code: await fn(), out: lines.join("\n") };
  } finally {
    console.log = log;
  }
}

test("recorded API session: table shows ahead/behind and --check passes on a sound manifest", async () => {
  const gh = fixture(await recorded());
  const { code, out } = await capture(() => run(["--check", "--only", ONLY, "--manifest", fx("forks.fixture.toml")], gh));
  expect(code).toBe(0);
  expect(out).toMatch(/devtools\/cargo\s+master\s+0\s+86\s/);
  expect(out).toMatch(/compiler\/rust\s+main\s+0\s+531\s/);
  expect(out).toMatch(/compiler\/rust-lang-llvm-project\s+rustc\/23\.1-2026-07-22\s+0\s+0\s/);
  expect(out).toContain("4 forks, 0 drift finding(s)");
});

test("--max-behind turns a lagging fork into drift", async () => {
  const gh = fixture(await recorded());
  const { code, out } = await capture(() => run(["--check", "--max-behind", "100", "--only", ONLY, "--manifest", fx("forks.fixture.toml")], gh));
  expect(code).toBe(1);
  expect(out).toContain("compiler/rust is 531 commits behind upstream");
  expect(out).not.toContain("devtools/cargo is");
});

test("a manifest that disagrees with the recorded API is drift", async () => {
  const gh = fixture(await recorded());
  const text = (await Bun.file(fx("forks.fixture.toml")).text()).replace('name = "libc"', 'name = "libc"\ndefault_branch = "dev"');
  const tmp = fx("forks.drift.toml");
  await Bun.write(tmp, text);
  try {
    const { code, out } = await capture(() => run(["--check", "--only", "libc", "--manifest", tmp], gh));
    expect(code).toBe(1);
    expect(out).toContain("default branch is main, manifest says dev");
  } finally {
    await Bun.file(tmp).delete();
  }
});

const m = parseManifest(`[meta]\norg = "o"\n[groups]\ng = "x"\n[[fork]]\npath = "g/a"\nname = "a"\nupstream = "u/a"\nbranch = "dev"\ndefault_branch = "main"\nrole = "std"\n`);
const f = m.forks[0]!;
const repo = { fullName: "o/a", defaultBranch: "main", fork: true, archived: false, parent: "U/A" };
const div = { ahead: 0, behind: 0 };

test("judge: a sound fork has no problems (parent compared case-insensitively)", () => {
  expect(judge("o", f, repo, "x".repeat(40), div)).toEqual([]);
});

test("judge: every kind of drift is named", () => {
  expect(judge("o", f, null, null, null)).toEqual(["o/a does not exist"]);
  expect(judge("o", f, { ...repo, fork: false }, "s", div)[0]).toBe("o/a is not a fork");
  expect(judge("o", f, { ...repo, parent: "x/y" }, "s", div)[0]).toBe("parent is x/y, manifest says u/a");
  expect(judge("o", f, { ...repo, defaultBranch: "master" }, "s", div)[0]).toBe("default branch is master, manifest says main");
  expect(judge("o", f, { ...repo, archived: true }, "s", div)[0]).toBe("o/a is archived");
  expect(judge("o", f, repo, null, null)[0]).toBe("branch dev does not exist on o/a");
  expect(judge("o", f, repo, "s", null)[0]).toBe("cannot compare dev with u/a");
});

test("argument parsing", () => {
  expect(parseArgs(["--check", "--only", "a,b"])).toMatchObject({ check: true });
  expect(() => parseArgs(["--nope"])).toThrow(/unknown argument/);
  expect(parseArgs(["-h"])).toBe("help");
});
