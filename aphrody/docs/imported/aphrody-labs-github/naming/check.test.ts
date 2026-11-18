// naming-ignore-file
import { describe, expect, test } from "bun:test";
import { checkFiles, envPrefixOf, expected, inventory, loadScheme, pascal, shortOf, type Decl, type Files } from "./check.ts";

const scheme = await loadScheme();
const ay: Decl = { id: "ay", kind: "sdk", org: "aphrody-dev" };
const run = (files: Record<string, string>, decl: Decl = ay) => checkFiles(new Map(Object.entries(files)) as Files, decl, scheme);
const rules = (f: { rule: string; severity: string }[], sev?: string) => f.filter(x => !sev || x.severity === sev).map(x => x.rule);

describe("derivation", () => {
  test("names come from the repository's own id", () => {
    expect(shortOf("aphrody-m3")).toBe("m3");
    expect(shortOf("aphrody")).toBe("");
    expect(pascal("ai-os")).toBe("AiOs");
    expect(envPrefixOf("ai-os")).toBe("AI_OS");
    const e = expected({ id: "ai-os", kind: "ai", org: "aphrody-dev" }, scheme);
    expect(e.scope).toBe("@aphrody");
    expect(e.crateStems).toEqual(["ai-os"]);
    expect(e.envPrefixes).toEqual(["AI_OS_"]);
    expect(expected({ id: "rosegriffon", kind: "app", org: "rose-griffon" }, scheme).scope).toBe("@rosegriffon");
  });
  test("a family adds its prefix", () => {
    const e = expected({ id: "yolo-ui", kind: "family", org: "aphrody-labs", family: "yolo" }, scheme);
    expect(e.crateStems).toEqual(["yolo-ui", "yolo"]);
  });
});

describe("names", () => {
  test("crate prefix, hyphens, unpublished crates exempt", () => {
    const f = run({
      "a/Cargo.toml": '[package]\nname = "ay-core"\n',
      "b/Cargo.toml": '[package]\nname = "other-core"\n',
      "c/Cargo.toml": '[package]\nname = "ay_core"\n',
      "d/Cargo.toml": '[package]\nname = "whatever"\npublish = false\n',
    });
    expect(rules(f)).toEqual(["N011", "N013"]);
    expect(f.find(x => x.rule === "N011")!.file).toBe("b/Cargo.toml");
  });
  test("declared legacy prefix is a note, not a warning", () => {
    const f = run({ "a/Cargo.toml": '[package]\nname = "aphrody-a2a"\n' }, { id: "ai-os", kind: "ai", org: "aphrody-dev", legacy: { crates: ["aphrody-"] } });
    expect(f.map(x => x.severity)).toEqual(["info"]);
  });
  test("npm scope and stem; private packages exempt", () => {
    const f = run({
      "a/package.json": '{"name":"@aphrody/ay-sdk"}',
      "b/package.json": '{"name":"@other/ay-sdk"}',
      "c/package.json": '{"name":"@aphrody/chromium"}',
      "d/package.json": '{"name":"ay","private":true}',
      "e/package.json": '{"name":"unscoped"}',
    });
    expect(rules(f)).toEqual(["N010", "N012", "N010"]);
  });
  test("forks and mirrors keep upstream names", () => {
    const f = run({ "Cargo.toml": '[package]\nname = "zig-thing"\n' }, { id: "zig", kind: "mirror", org: "aphrody-labs" });
    expect(f).toEqual([]);
  });
  test("go module is the repository URL; pypi and nuget stems", () => {
    const f = run({
      "go.mod": "module example.com/x\n",
      "p/pyproject.toml": '[project]\nname = "foo"\n',
      "n/App.csproj": "<Project><PropertyGroup><PackageId>Contoso.App</PackageId></PropertyGroup></Project>",
    });
    expect(rules(f).sort()).toEqual(["N020", "N030", "N040"]);
    expect(rules(run({ "go.mod": "module github.com/aphrody-dev/ay/tools\n" }))).toEqual([]);
  });
  test("declaration errors", () => {
    expect(rules(run({}, { id: "Bad_Name", kind: "sdk", org: "aphrody-dev" }), "error")).toContain("N002");
    expect(rules(run({}, { id: "x", kind: "nope", org: "aphrody-dev" }), "error")).toContain("N001");
  });
  test("env vars owned by another repository", () => {
    const f = run({ "src/a.ts": "const o = process.env.OPEN_AI_API_KEY; const a = process.env.NIE_CDN; const b = process.env.AY_HOME; const c = process.env.GITHUB_TOKEN; const d = process.env.SOMEVENDOR_KEY;" });
    expect(f.map(x => x.rule)).toEqual(["I120"]);
    expect(f[0].message).toContain("NIE_CDN");
  });
  test("cloudflare resources", () => {
    const f = run({ "w/wrangler.toml": 'name = "ay-api-prod"\n[[r2_buckets]]\nbinding = "B"\nbucket_name = "shared-bucket"\n' });
    expect(rules(f)).toEqual(["N060"]);
  });
});

describe("independence", () => {
  test("submodules: originals are errors, forks are warnings, branches are pinned by SHA", () => {
    const f = run({ ".gitmodules": '[submodule "a"]\n\tpath = a\n\turl = https://github.com/aphrody-labs/nie.git\n[submodule "b"]\n\tpath = b\n\turl = ../bevy\n\tbranch = main\n[submodule "c"]\n\tpath = c\n\turl = https://github.com/upstream/x.git\n' });
    expect(rules(f, "error")).toEqual(["I001"]);
    expect(rules(f)).toContain("I004");
    expect(f.filter(x => x.file === ".gitmodules" && x.message.includes('"c"'))).toEqual([]);
  });
  test("Cargo path and git dependencies", () => {
    const f = run({
      "crates/a/Cargo.toml": '[package]\nname = "ay-a"\n[dependencies]\nx = { path = "../../../nie/x" }\ny = { path = "../b" }\nz = { git = "https://github.com/aphrody-labs/nie", branch = "main" }\nw = { git = "https://github.com/aphrody-labs/nie", rev = "abc" }\nv = { git = "https://github.com/aphrody-labs/nie", rev = "abc", optional = true }\n',
    });
    expect(rules(f).sort()).toEqual(["I010", "I011", "I012"]);
  });
  test("package.json links and github installs", () => {
    const f = run({ "p/package.json": '{"name":"@aphrody/ay-p","dependencies":{"a":"file:../../../nie/a","b":"file:../b","c":"github:aphrody-labs/nie#main","d":"github:aphrody-labs/nie#' + "a".repeat(40) + '","e":"github:vendor/x"},"workspaces":["../../ws"]}' });
    expect(rules(f).sort()).toEqual(["I020", "I021", "I021", "I022"]);
    expect(f.filter(x => x.rule === "I021").map(x => x.severity).sort()).toEqual(["error", "warn"]);
  });
  test("workflows: cross-repo uses, checkouts, dispatches", () => {
    const sha = "b".repeat(40);
    const f = run({
      ".github/workflows/ci.yml": [
        "jobs:", "  a:", "    steps:",
        "      - uses: aphrody-labs/.github/.github/workflows/ci.yml@main",
        "      - uses: aphrody-labs/nie/.github/workflows/ci.yml@main",
        `      - uses: aphrody-labs/nie/.github/actions/x@${sha}`,
        "      - uses: actions/checkout@v4",
        "      - uses: actions/checkout@v4",
        "        with:",
        "          repository: aphrody-labs/nie",
        "      - run: gh api repos/aphrody-labs/nie/dispatches -f event_type=x",
        "      - uses: aphrody-dev/ay/.github/workflows/z.yml@main",
      ].join("\n"),
    });
    expect(rules(f, "error").sort()).toEqual(["I071", "I072", "I073"]);
    expect(f.filter(x => x.rule === "I071").map(x => x.severity).sort()).toEqual(["error", "warn"]);
    expect(rules(f, "info")).toContain("I070");
  });
  test("sibling checkouts", () => {
    const f = run({
      "tools/x.ts": 'const p = join(here, "../../nie/data"); const q = "$WORKSPACE_ROOT/bun/x"; const ok = "../local/x"; const doc = "../CLAUDE.md";',
      "src/deep/y.ts": 'import x from "../bun/x.ts"; import z from "../../../../nie/z";',
      "ay.toml": 'checkout = "../aphrody"\n',
      "docs/a.md": "see ../../nie/readme",
    });
    expect(f.filter(x => x.rule === "I090").map(x => `${x.file}:${x.severity}`).sort()).toEqual(["ay.toml:info", "docs/a.md:info", "src/deep/y.ts:error", "tools/x.ts:error", "tools/x.ts:error"]);
  });
  test("branch-pinned raw fetches, clones, generated copies, hardcoded slugs", () => {
    const f = run({
      "scripts/a.sh": "curl https://raw.githubusercontent.com/aphrody-labs/nie/main/x.toml\ncurl https://raw.githubusercontent.com/aphrody-labs/nie/" + "c".repeat(40) + "/x\ngit clone https://github.com/aphrody-labs/nie.git",
      "gen.toml": "# Generated by tools/extract.ts from aphrody-labs/legacy@cf409c1643e8; edit extract.toml\n",
      "src/a.ts": 'const slug = "aphrody-labs/shenron";',
    });
    expect(rules(f).sort()).toEqual(["I080", "I081", "I100", "I100", "I100", "I110"].sort());
    expect(f.find(x => x.rule === "I080")!.severity).toBe("error");
  });
  test("own repository, images and fixtures are not coupling", () => {
    const f = run({
      "a.ts": 'const own = "https://github.com/aphrody-dev/ay"; const r = "aphrody-dev/ay";',
      "Dockerfile": "FROM ghcr.io/aphrody-dev/ay-tools:1\n",
      "t/fixtures/Cargo.toml": '[package]\nname = "zzz"\n[dependencies]\nx = { path = "../../../../q" }\n',
      "node_modules/x/package.json": '{"name":"junk"}',
      "ignored.ts": "// naming-ignore-file\n",
    });
    expect(f).toEqual([]);
  });
  test("images of another repository", () => {
    const f = run({ "Dockerfile": "FROM ghcr.io/aphrody-labs/nie-base:latest\n" });
    expect(rules(f)).toEqual(["I060"]);
  });
  test("exceptions demote a finding to a note", () => {
    const f = run({ "tools/x.ts": 'x("../../nie/data")' }, { ...ay, exceptions: { I090: "dev-only helper" } });
    expect(f.map(x => x.severity)).toEqual(["info"]);
    expect(f[0].message).toContain("dev-only helper");
  });
});

test("inventory lists names found in manifests", () => {
  const inv = inventory(new Map([
    ["Cargo.toml", '[package]\nname = "ay-a"\n[[bin]]\nname = "ay"\n'],
    ["package.json", '{"name":"@aphrody/ay-x","bin":{"ay-x":"x.js"}}'],
    ["src/a.ts", "process.env.AY_HOME"],
  ]));
  expect(inv.crates).toEqual(["ay-a"]);
  expect(inv.npm).toEqual(["@aphrody/ay-x"]);
  expect(inv.bins).toEqual(["ay", "ay-x"]);
  expect(inv.envPrefixes).toEqual(["AY"]);
});

describe("declared upstream names and runtime forks", () => {
  test("names.upstream frees registry names", () => {
    const f = run({ "Cargo.toml": '[package]\nname = "gpui"\n', "package.json": '{"name":"zed-x"}' }, { id: "aphrody", kind: "os", org: "aphrody-dev", names: { upstream: true as unknown as string } });
    expect(f).toEqual([]);
  });
  test("a runtime fork as submodule is a warning, not an error", () => {
    const f = run({ ".gitmodules": '[submodule "vendor/bun"]\n\tpath = vendor/bun\n\turl = https://github.com/aphrody-labs/bun.git\n' }, { id: "aphrody", kind: "os", org: "aphrody-dev" });
    expect(rules(f)).toEqual(["I002"]);
  });
});

test("underscore crate names are not flagged when the repository keeps upstream names", () => {
  const files = new Map([["Cargo.toml", '[package]\nname = "acp_thread"\n']]) as Files;
  expect(checkFiles(files, { id: "bun", kind: "runtime", org: "aphrody-labs" }, scheme)).toEqual([]);
  expect(rules(checkFiles(files, ay, scheme))).toContain("N013");
});
