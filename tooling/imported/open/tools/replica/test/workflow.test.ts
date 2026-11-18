import { expect, test } from "bun:test";
import { robots, Shapes } from "../crawl";
import { parity } from "../parity";
import { parseCsv, toCsv, type Row } from "../recon";

test("robots: longest match wins, Allow wins ties, agent groups chosen over *", () => {
  const r = robots("User-agent: *\nDisallow: /private\nAllow: /private/open\nCrawl-delay: 3\n\nUser-agent: other\nDisallow: /");
  expect(r.allowed("/private/x")).toBe(false);
  expect(r.allowed("/private/open/x")).toBe(true);
  expect(r.allowed("/public")).toBe(true);
  expect(r.delay).toBe(3);
  expect(robots("User-agent: other\nDisallow: /", "other-bot").allowed("/a")).toBe(false);
  expect(robots("User-agent: *\nDisallow: /*.json$").allowed("/a.json")).toBe(false);
});

test("shapes: item pages of one route collapse, navigation roots and route words stay", () => {
  const s = new Shapes();
  s.addNav(["/models", "/datasets"]);
  for (const p of ["/a/x/tree/main", "/b/y/tree/main", "/e/z/blob/main/f"]) s.learn(p);
  expect(s.of("/models")).toBe("/models");
  expect(s.of("/a/x/tree/main")).toBe(s.of("/b/y/tree/main"));
  expect(s.of("/a/x/tree/main")).toBe("/*/*/tree/main");
  // a namespace under several roots is not a route word
  for (const p of ["/models/acme/x", "/datasets/acme/y"]) s.learn(p);
  expect(s.of("/models/acme/x")).toBe("/models/*/*");
  expect(s.of("/f/w")).toBe("/*/*");
});

test("parity: weights, partial credit, skip and extras left out, layout share", () => {
  const rows: Row[] = [
    { feature: "a", area: "x", priority: "must", original: "yes", clone: "yes", notes: "" },
    { feature: "b", area: "x", priority: "should", original: "yes", clone: "partial", notes: "" },
    { feature: "c", area: "y", priority: "could", original: "yes", clone: "no", notes: "" },
    { feature: "d", area: "y", priority: "must", original: "yes", clone: "skip", notes: "licensed" },
    { feature: "e", area: "y", priority: "must", original: "no", clone: "yes", notes: "" },
  ];
  const p = parity(rows);
  expect(p.counted).toBe(3);
  expect(p.feature).toBe(66.7); // (3 + 1) / 6
  expect(p.missing.map((r) => r.feature)).toEqual(["b", "c"]);
  expect(parity(rows, [50]).overall).toBe(63.3);
});

test("csv: quoted cells survive a round trip", () => {
  const rows: Row[] = [{ feature: 'Form "POST", with comma', area: "a", priority: "must", original: "yes", clone: "no", notes: "line\nbreak" }];
  expect(parseCsv(toCsv(rows))).toEqual(rows as unknown as Record<string, string>[]);
});
