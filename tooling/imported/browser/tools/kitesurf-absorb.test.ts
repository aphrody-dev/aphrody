import { expect, test } from "bun:test";
import { forkName, groupPins, upstreamOf, type Pin } from "./kitesurf-absorb.ts";

const pin = (name: string, sha: string, upstream: string): Pin => ({
  name,
  version: "1.0.0",
  sha,
  path: name,
  upstream,
});

test("a repository URL gives owner/name, with or without .git and a trailing path", () => {
  expect(upstreamOf("https://github.com/DioxusLabs/blitz")).toBe("DioxusLabs/blitz");
  expect(upstreamOf("https://github.com/servo/stylo.git")).toBe("servo/stylo");
  expect(upstreamOf("git@github.com:boa-dev/boa")).toBe("boa-dev/boa");
  expect(() => upstreamOf("https://example.com/x")).toThrow("not a GitHub repository");
});

test("the fork name is the lower-cased repository name", () => {
  expect(forkName("DioxusLabs/Blitz")).toBe("blitz");
});

test("crates cut from one upstream commit share a fork branch, in list order; another commit is another group", () => {
  const groups = groupPins([pin("a", "1", "o/x"), pin("b", "2", "o/y"), pin("c", "1", "o/x"), pin("d", "3", "o/x")]);
  expect(groups.map((group) => group.crates.map((crate) => crate.name))).toEqual([["a", "c"], ["b"], ["d"]]);
});

test("a manifest edit lands once at its anchor and a moved anchor is an error", async () => {
  const { editManifest } = await import("./kitesurf-absorb.ts");
  expect(editManifest("a = 1\nb = 2\n", [{ file: "Cargo.toml", from: "a = 1\n", to: "a = 1\nc = 3\n" }])).toBe(
    "a = 1\nc = 3\nb = 2\n",
  );
  expect(() => editManifest("x\n", [{ file: "Cargo.toml", from: "a = 1\n", to: "" }])).toThrow("anchor not found");
});
