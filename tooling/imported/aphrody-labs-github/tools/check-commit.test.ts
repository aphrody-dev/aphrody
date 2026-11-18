import { expect, test, describe } from "bun:test";
import { check, loadConfig, stripComments, type Options } from "./check-commit";
import { lint, parseRules } from "./lint-code-style";
import { join } from "node:path";

const cfg = await loadConfig(join(import.meta.dir, ".."));
const { types } = cfg;
const DEFAULTS = { maxSubject: cfg.maxSubject, maxBody: cfg.maxBody };
const opt = (o: Partial<Options> = {}): Options => ({ ...cfg, style: "conventional", dco: false, title: false, ...o });
const ids = (msg: string, o: Partial<Options> = {}) => check(msg, opt(o)).filter((f) => f.level === "error").map((f) => f.id);
const GOOD_BODY = "Why this exists and what it fixes.\n\nVerified with bun test.";

describe("types come from standard.toml", () => {
  test("the standard's list is loaded", () => {
    expect(types).toEqual(expect.arrayContaining(["feat", "fix", "docs", "chore", "refactor", "perf", "test", "build", "ci", "revert"]));
  });
});

describe("subject", () => {
  test("accepts type(scope): summary", () => {
    expect(check(`fix(install): resolve peer dependencies like pnpm\n\n${GOOD_BODY}`, opt())).toEqual([]);
  });
  test("accepts type without scope and with breaking marker plus footer", () => {
    expect(ids(`feat!: drop the legacy flag\n\n${GOOD_BODY}\n\nBREAKING CHANGE: use --new instead`)).toEqual([]);
  });
  test("rejects an unknown type", () => {
    expect(ids("feature: add a thing\n\nwhy")).toEqual(["CO-01"]);
  });
  test("the standard's default style is area, and conventional is the strict alternative", () => {
    expect(cfg.style).toBe("area");
    expect(ids("install: fix the bin link\n\nwhy", { style: "area" })).toEqual([]);
    expect(ids("install: fix the bin link\n\nwhy", { style: "conventional" })).toEqual(["CO-01"]);
    expect(ids("fix(install): fix the bin link\n\nwhy", { style: "area" })).toEqual([]);
  });
  test("area style: Bun-shaped subjects pass, shapeless ones fail, the other subject rules still apply", () => {
    const area = (m: string) => ids(m, { style: "area" });
    expect(area("AOT: added evaluated imports and tests of the engine's fixes\n\nwhy")).toEqual(["CO-02"]);
    expect(area("tls: stop a node:https busy loop after a rejected send\n\nwhy")).toEqual([]);
    expect(area("css/parser: keep the comment above a rule\n\nwhy")).toEqual([]);
    expect(area("Fix the build of bun_bundler\n\nwhy")).toEqual(["CO-01"]);
    expect(area("http: fixed a hang.\n\nwhy")).toEqual(["CO-02", "CO-02"]);
    expect(area("http: stop a hang.\n\nwhy")).toEqual(["CO-02"]);
    expect(area("ci: update\n\nwhy")).toEqual(["CO-02"]);
    expect(area("http: " + "ab ".repeat(30) + "\n\nwhy")).toEqual(["CO-02"]);
  });
  test("area style: a breaking subject needs the footer, with or without a bang", () => {
    expect(ids("http!: drop the legacy flag entirely\n\nwhy", { style: "area" })).toEqual(["CO-08"]);
    expect(ids("http!: drop the legacy flag entirely\n\nwhy\n\nBREAKING CHANGE: use --new", { style: "area" })).toEqual([]);
  });
  test("rejects a subject over 72 characters and counts exactly 72 as fine", () => {
    const ok = "fix: " + "ab ".repeat(22) + "z";
    expect(ok.length).toBe(72);
    expect(ids(`${ok}\n\nwhy`)).toEqual([]);
    expect(ids(`${ok}a\n\nwhy`)).toEqual(["CO-02"]);
  });
  test("rejects a trailing period, non-imperative verbs and vague summaries", () => {
    expect(ids("fix: stop the leak.\n\nwhy")).toEqual(["CO-02"]);
    expect(ids("fix: fixed the leak\n\nwhy")).toEqual(["CO-02"]);
    expect(ids("feat: adds a flag\n\nwhy")).toEqual(["CO-02"]);
    expect(ids("fix: bug\n\nwhy")).toEqual(["CO-02"]);
    expect(ids("chore: update\n\nwhy")).toEqual(["CO-02"]);
    expect(ids("chore: phase 1\n\nwhy")).toEqual(["CO-02"]);
  });
  test("does not mistake an imperative that starts like a banned word", () => {
    expect(ids("fix: changelog entry for the release\n\nwhy")).toEqual([]);
    expect(ids("feat: addressable ids for tabs\n\nwhy")).toEqual([]);
  });
  test("empty subject, autosquash prefixes, merges and git reverts", () => {
    expect(ids("")).toEqual(["CO-02"]);
    expect(ids("fixup! fix: a thing here")).toContain("CO-14");
    expect(ids("Merge branch 'x' into main")).toEqual([]);
    expect(ids('Revert "fix: a thing here"')).toEqual([]);
  });
});

describe("body", () => {
  test("second line must be blank", () => {
    expect(ids("fix: stop the leak in the pool\nwhy it leaks")).toEqual(["CO-03"]);
  });
  test("feat, fix, perf and refactor without a body warn; docs do not", () => {
    expect(check("fix: stop the leak in the pool", opt()).map((f) => f.level)).toEqual(["warning"]);
    expect(check("docs: describe the leak in the pool", opt())).toEqual([]);
  });
  test("long prose lines warn, tables, code and indented lines do not", () => {
    const long = "x".repeat(DEFAULTS.maxBody + 1);
    expect(check(`fix: stop the leak in the pool\n\n${long}`, opt()).map((f) => f.id)).toEqual(["CO-12"]);
    expect(check(`fix: stop the leak in the pool\n\n| ${long} |`, opt())).toEqual([]);
    expect(check(`fix: stop the leak in the pool\n\n  ${long}`, opt())).toEqual([]);
    expect(check("fix: stop the leak in the pool\n\n```\n" + long + "\n```", opt())).toEqual([]);
  });
  test("git comment lines and the scissors section are ignored", () => {
    const msg = `fix: stop the leak in the pool\n\n${GOOD_BODY}\n# Please enter the commit message\n# ------------------------ >8 ------------------------\ndiff --git a b`;
    expect(stripComments(msg)).toBe(`fix: stop the leak in the pool\n\n${GOOD_BODY}`);
    expect(check(msg, opt())).toEqual([]);
  });
});

describe("trailers", () => {
  test("a breaking marker needs the footer, and the footer needs the marker", () => {
    expect(ids(`feat!: drop the legacy flag\n\n${GOOD_BODY}`)).toEqual(["CO-08"]);
    expect(check(`feat: drop the legacy flag\n\n${GOOD_BODY}\n\nBREAKING CHANGE: gone`, opt()).map((f) => f.level)).toEqual(["warning"]);
  });
  test("AI and bot attribution is an error", () => {
    for (const trailer of ["Co-authored-by: Claude <noreply@anthropic.com>", "Co-authored-by: autofix-ci[bot] <x@users.noreply.github.com>", "Assisted-by: some model", "Generated with Claude Code"]) {
      expect(ids(`fix: stop the leak in the pool\n\n${GOOD_BODY}\n\n${trailer}`)).toEqual(["CO-10"]);
    }
    expect(ids(`fix: stop the leak in the pool\n\n${GOOD_BODY}\n\n\u{1F916} generated`)).toEqual(["CO-10"]);
  });
  test("a human co-author is fine", () => {
    expect(check(`fix: stop the leak in the pool\n\n${GOOD_BODY}\n\nCo-authored-by: Ada Lovelace <ada@example.org>`, opt())).toEqual([]);
  });
  test("DCO is required only when asked", () => {
    const msg = `fix: stop the leak in the pool\n\n${GOOD_BODY}`;
    expect(ids(msg)).toEqual([]);
    expect(ids(msg, { dco: true })).toEqual(["CO-11"]);
    expect(ids(`${msg}\n\nSigned-off-by: Ada Lovelace <ada@example.org>`, { dco: true })).toEqual([]);
  });
  test("absolute developer paths warn; Fixes needs 12 hex and the subject", () => {
    expect(check(`fix: stop the leak in the pool\n\n${GOOD_BODY} See C:\\Users\\someone\\x.`, opt()).map((f) => f.id)).toEqual(["CO-13"]);
    expect(check(`fix: stop the leak in the pool\n\n${GOOD_BODY}\n\nFixes: 54a4f0239f2e ("KVM: make a thing")`, opt())).toEqual([]);
    expect(check(`fix: stop the leak in the pool\n\n${GOOD_BODY}\n\nFixes: 54a4f02`, opt()).map((f) => f.id)).toEqual(["CO-06"]);
  });
});

describe("pull request titles", () => {
  test("the GitHub suffix is not counted and the body is not required", () => {
    const base = "fix: " + "ab ".repeat(22) + "z";
    expect(check(`${base} (#1234)`, opt({ title: true }))).toEqual([]);
    expect(ids(`${base}a (#1234)`, { title: true })).toEqual(["CO-02"]);
  });
  test("the same shape rules apply", () => {
    expect(ids("Update stuff", { title: true })).toEqual(["CO-01"]);
    expect(ids("chore: update", { title: true })).toEqual(["CO-02"]);
  });
});

describe("code-style.md linter", () => {
  const conflicts = "| C-01 | a | b | c | d |\n| C-02 | a | b | c | d |";
  const row = (extra = "TS-01", source = "BUN Tests", status = "adopted", enforced = "review") => `| ${extra} | A rule. | ${source} | ${status} | ${enforced} |`;
  test("parses rows, conflicts and multi-source cells", () => {
    const { rules } = parseRules(row("TS-01", "BUN Tests; GTS Imports", "adapted (C-01, C-02)", "fmt, lint"));
    expect(rules[0]).toMatchObject({ id: "TS-01", status: "adapted", conflicts: ["C-01", "C-02"], enforced: ["fmt", "lint"] });
  });
  test("a clean document has no errors", () => {
    expect(lint(row(), conflicts, ["BUN"]).errors).toEqual([]);
  });
  test("flags unknown tags, duplicate ids, missing conflicts and unknown enforcers", () => {
    expect(lint(row("TS-01", "XXX Section"), conflicts, ["BUN"]).errors).toContain("TS-01: unknown source tag XXX");
    expect(lint(row() + "\n" + row(), conflicts, ["BUN"]).errors).toContain("TS-01: duplicate id");
    expect(lint(row("TS-01", "BUN Tests", "rejected"), conflicts, ["BUN"]).errors).toContain("TS-01: rejected without a conflict row");
    expect(lint(row("TS-01", "BUN Tests", "adopted (C-09)"), conflicts, ["BUN"]).errors).toContain("TS-01: unknown conflict C-09");
    expect(lint(row("TS-01", "BUN Tests", "adopted", "magic"), conflicts, ["BUN"]).errors).toContain("TS-01: unknown enforcer magic");
  });
  test("flags a row that does not parse", () => {
    expect(lint("| TS-01 | only two cells |", conflicts, ["BUN"]).errors.some((e) => e.startsWith("malformed rule row"))).toBe(true);
  });
  test("the real document passes", async () => {
    const dir = join(import.meta.dir, "../docs/standards");
    const sources = Bun.TOML.parse(await Bun.file(join(dir, "sources.toml")).text()) as { tags: Record<string, string[]> };
    const res = lint(await Bun.file(join(dir, "code-style.md")).text(), await Bun.file(join(dir, "conflicts.md")).text(), Object.keys(sources.tags));
    expect(res.errors).toEqual([]);
  });
});
