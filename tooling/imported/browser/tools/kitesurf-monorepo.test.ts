import { expect, test } from "bun:test";
import { closureOf, prefixManifest, rewireLine, rewriteRoot, type Meta } from "./kitesurf-monorepo.ts";

const meta: Meta = {
  packages: ["app", "top", "mid", "seed", "other"].map((name) => ({
    id: name,
    name,
    version: "1.0.0",
    source: name === "app" ? null : "registry",
  })),
  resolve: {
    nodes: [
      { id: "app", deps: [{ pkg: "top" }, { pkg: "other" }] },
      { id: "top", deps: [{ pkg: "mid" }] },
      { id: "mid", deps: [{ pkg: "seed" }] },
      { id: "seed", deps: [] },
      { id: "other", deps: [] },
    ],
  },
  workspace_members: ["app"],
};

test("the closure is the seeds and whatever depends on them, never the workspace's own members or unrelated crates", () => {
  expect(closureOf(meta, ["seed"]).map((member) => member.name)).toEqual(["mid", "seed", "top"]);
});

const manifest = `[package]
name = "blitz-dom"
version = "0.3.0-beta.4"

[dependencies.stylo]
version = "0.22"
default-features = false

[target."cfg(unix)".dependencies.taffy]
version = "0.15"

[dependencies.serde]
version = "1"

[workspace]
`;

test("a manifest gets the prefixed name, the library name, sibling path dependencies and no stamped workspace", () => {
  const text = prefixManifest(manifest, ["blitz-dom", "stylo", "taffy"], "browser-");
  expect(text).toContain('name = "browser-blitz-dom"');
  expect(text).toContain('[dependencies.stylo]\npackage = "browser-stylo"\npath = "../stylo"');
  expect(text).toContain('.dependencies.taffy]\npackage = "browser-taffy"\npath = "../taffy"');
  expect(text).not.toContain('package = "browser-serde"');
  expect(text).not.toContain("[workspace]");
  expect(text.trimEnd().endsWith('[lib]\nname = "blitz_dom"')).toBe(true);
});

test("an existing [lib] keeps its name and one without a name gets it", () => {
  expect(prefixManifest('[package]\nname = "stylo"\n\n[lib]\nname = "style"\n', ["stylo"], "browser-")).toContain(
    '[lib]\nname = "style"',
  );
  expect(prefixManifest('[package]\nname = "a-b"\n\n[lib]\npath = "x.rs"\n', ["a-b"], "browser-")).toContain(
    '[lib]\nname = "a_b"\npath = "x.rs"',
  );
});

test("hand-written dependency lines and the root manifest are rewired, and the patch table is dropped", () => {
  const members = [{ name: "taffy", version: "0.15.0" }];
  expect(
    rewireLine('taffy = { version = "0.15", default-features = false }', members, "browser-", "../../crates"),
  ).toBe(
    'taffy = { package = "browser-taffy", path = "../../crates/taffy", version = "0.15", default-features = false }',
  );
  expect(rewireLine('serde = { version = "1" }', members, "browser-", "x")).toBe('serde = { version = "1" }');
  const root = rewriteRoot(
    '[workspace]\nmembers = ["packages/core"]\n\n[workspace.dependencies]\ntaffy = { version = "0.15" }\n\n[patch.crates-io]\ntaffy = { path = "x" }\n\n[profile.release]\nlto = true\n',
    members,
    "browser-",
  );
  expect(root).toContain('members = ["packages/core", "crates/*"]');
  expect(root).toContain('package = "browser-taffy", path = "crates/taffy"');
  expect(root).not.toContain("patch.crates-io");
  expect(root).toContain("[profile.release]");
});

test("a dependency under another alias keeps the alias and points its package at the renamed sibling", () => {
  const text = prefixManifest(
    '[package]\nname = "blitz-dom"\n\n[dependencies.style]\nversion = "0.22.0"\npackage = "stylo"\n\n[dependencies.style_config]\nversion = "0.22.0"\npackage = "stylo_static_prefs"\n',
    ["blitz-dom", "stylo"],
    "browser-",
  );
  expect(text).toContain('[dependencies.style]\npackage = "browser-stylo"\npath = "../stylo"\nversion = "0.22.0"');
  expect(text).toContain('package = "stylo_static_prefs"');
  expect(text).not.toContain('package = "stylo"');
});

test("dev-dependencies, benches, tests and examples are dropped: they are not in the published sources", () => {
  const text = prefixManifest(
    '[package]\nname = "a"\n\n[dependencies.x]\nversion = "1"\n\n[dev-dependencies.criterion]\nversion = "0.5"\n\n[target."cfg(unix)".dev-dependencies.y]\nversion = "1"\n\n[[bench]]\nname = "b"\nharness = false\n\n[[example]]\nname = "e"\n\n[features]\ndefault = []\n',
    [],
    "browser-",
  );
  expect(text).toContain("[dependencies.x]");
  expect(text).toContain("[features]");
  for (const gone of ["criterion", "dev-dependencies", "[[bench]]", "[[example]]", 'name = "b"'])
    expect(text).not.toContain(gone);
});

test("a feature that forwards to a dropped dev-dependency loses that entry", () => {
  const text = prefixManifest(
    '[package]\nname = "p"\n\n[features]\nlibm = [\n    "fontique/libm",\n    "peniko/libm",\n    "dep:core_maths",\n]\n\n[dependencies.fontique]\nversion = "1"\n\n[dev-dependencies.peniko]\nversion = "1"\n',
    [],
    "browser-",
  );
  expect(text).not.toContain("peniko");
  expect(text).toContain('"fontique/libm"');
  expect(text).toContain('"dep:core_maths"');
});

test("a crate whose library name differs from its package name is keyed by the library name, and string-form dependencies are rewired too", async () => {
  const { renameFeatureRefs } = await import("./kitesurf-monorepo.ts");
  const members = [
    { name: "stylo", version: "0.22.0", lib: "style" },
    { name: "stylo_traits", version: "0.22.0", lib: "style_traits" },
    { name: "taffy", version: "0.15.0", lib: "taffy" },
  ];
  expect(rewireLine('stylo = { version = "0.22", features = ["servo"] }', members, "browser-", "../../crates")).toBe(
    'style = { package = "browser-stylo", path = "../../crates/stylo", version = "0.22", features = ["servo"] }',
  );
  expect(rewireLine('stylo_traits = "0.22"', members, "browser-", "../../crates")).toBe(
    'style_traits = { package = "browser-stylo_traits", path = "../../crates/stylo_traits", version = "0.22" }',
  );
  expect(rewireLine('taffy = "0.15"', members, "browser-", "../../crates")).toBe(
    'taffy = { package = "browser-taffy", path = "../../crates/taffy", version = "0.15" }',
  );
  expect(
    renameFeatureRefs('a = ["stylo/servo", "dep:stylo", "stylo?/x", "taffy/std", "stylo_traits/x"]', members),
  ).toBe('a = ["style/servo", "dep:style", "style?/x", "taffy/std", "style_traits/x"]');
});

test("the feature matrix lists each in-tree crate's features and marks the ones the build enables", async () => {
  const { matrixOf } = await import("./kitesurf-monorepo.ts");
  const meta: import("./kitesurf-monorepo.ts").MatrixMeta = {
    packages: [
      {
        id: "a",
        name: "browser-taffy",
        version: "0.15.0",
        repository: "https://github.com/DioxusLabs/taffy",
        features: { default: ["std"], std: [], grid: ["dep:x"] },
        manifest_path: "/w/crates/taffy/Cargo.toml",
      },
      {
        id: "b",
        name: "browser-other",
        version: "1.0.0",
        repository: null,
        features: {},
        manifest_path: "/w/packages/other/Cargo.toml",
      },
    ],
    resolve: { nodes: [{ id: "a", features: ["std", "default"] }] },
  };
  const text = matrixOf(meta, "browser-", { taffy: 7 });
  expect(text).toContain("## browser-taffy 0.15.0");
  expect(text).toContain("patches carried: 7");
  expect(text).toContain("| `std` | ● |");
  expect(text).toContain("| `grid` | ○ | `dep:x` |");
  expect(text).not.toContain("browser-other");
});
