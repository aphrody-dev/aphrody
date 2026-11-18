import { expect, test } from "bun:test";
import { resolve } from "node:path";
import { gitlinks } from "./lib/git.ts";
import { layoutProblems } from "./lib/inspect.ts";
import { loadManifest, parseManifest, renderGitmodules, ROLES } from "./lib/manifest.ts";

const root = resolve(import.meta.dir, "..");

test("forks.toml is structurally valid and every role is known", async () => {
  const m = await loadManifest(resolve(root, "forks.toml"));
  expect(m.forks.length).toBeGreaterThan(40);
  for (const f of m.forks) expect(ROLES).toContain(f.role);
  expect(m.forks.find((f) => f.upstream === "rust-lang/rust")?.path).toBe("compiler/rust");
  expect(m.notForked.map((n) => n.upstream)).toContain("rust-lang/crates.io-index");
});

test(".gitmodules and the index pins match forks.toml, with relative URLs only", async () => {
  const m = await loadManifest(resolve(root, "forks.toml"));
  const gm = await Bun.file(resolve(root, ".gitmodules")).text();
  expect(gm).toBe(renderGitmodules(m));
  expect([...gm.matchAll(/url = (.+)/g)].every((x) => /^\.\.\/[\w.-]+\.git$/.test(x[1]!))).toBe(true);
  expect(layoutProblems(m, gm, await gitlinks(root))).toEqual([]);
});

const base = `[meta]\norg = "o"\n[groups]\ng = "x"\n`;
const fork = (extra = "", path = "g/a") => `[[fork]]\npath = "${path}"\nname = "${path.split("/")[1]}"\nupstream = "u/a"\nbranch = "main"\nrole = "std"\n${extra}`;

test("a manifest with a duplicate path, an undeclared group or a bad role is rejected", () => {
  expect(() => parseManifest(base + fork() + fork())).toThrow(/duplicate path/);
  expect(() => parseManifest(base + fork("", "h/a"))).toThrow(/group "h"/);
  expect(() => parseManifest(base + fork().replace('"std"', '"wizard"'))).toThrow(/unknown role/);
  expect(() => parseManifest(base + fork("rust_submodule = \"a\"\nrust_subtree = \"b\"\n"))).toThrow(/exclusive/);
  expect(() => parseManifest(base + fork("policy = \"yolo\"\n"))).toThrow(/unknown policy/);
});

test("layoutProblems reports a stale .gitmodules, missing gitlinks and strays", () => {
  const m = parseManifest(base + fork());
  expect(layoutProblems(m, renderGitmodules(m), new Map([["g/a", "a".repeat(40)]]))).toEqual([]);
  const bad = layoutProblems(m, "", new Map([["z/z", "b".repeat(40)]]));
  expect(bad.join("\n")).toMatch(/\.gitmodules does not match/);
  expect(bad.join("\n")).toMatch(/1 gitlink\(s\) missing: g\/a/);
  expect(bad.join("\n")).toMatch(/gitlink z\/z is not in forks.toml/);
});

test("a consumer outside the fork organisation gets two-level relative URLs, one inside gets one level", () => {
  const same = parseManifest(base + fork());
  expect(renderGitmodules(same)).toContain("url = ../a.git\n");
  const other = parseManifest(`[meta]\norg = "forks"\nconsumer = "app"\n[groups]\ng = "x"\n` + fork());
  expect(other.consumer).toBe("app");
  expect(renderGitmodules(other)).toContain("url = ../../forks/a.git\n");
});
