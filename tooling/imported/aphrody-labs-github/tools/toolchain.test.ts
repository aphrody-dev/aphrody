import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { channelOf, devProfiles, hostScript, installDrift, parsePolicy, policyOf, profileDrift, renderToolchain, setChannel, setInstall, setSection } from "./toolchain.ts";

const raw = parsePolicy(readFileSync(join(import.meta.dir, "..", "templates", "toolchain.toml"), "utf8"));
const template = readFileSync(join(import.meta.dir, "..", "templates", "cargo-profiles.toml"), "utf8");

describe("toolchain policy", () => {
  test("a repository takes the defaults and its own exceptions", () => {
    expect(policyOf(raw, "ay").channel).toBe(raw.rust.channel);
    expect(policyOf(raw, "aphrody").targets).toContain("wasm32-wasip2");
    expect(policyOf(raw, "bun").channel).toBe(raw.rust.nightly!.pinned);
    expect(policyOf(raw, "ai-os").install.linker).toBe("hoisted");
    expect(policyOf(raw, "ay").install.linker).toBe("isolated");
    expect(policyOf(raw, "ay").install.globalStore).toBe(true);
  });

  test("every cargo tool is pinned to a plain version", () => {
    for (const [name, version] of Object.entries(raw.tools)) expect(`${name}@${version}`).toMatch(/^[\w-]+@\d+\.\d+\.\d+$/);
  });
});

describe("setInstall", () => {
  const keys = { linker: "isolated", globalStore: true, exact: true };
  test("adds an [install] section to a file that has none, and creates the file content from nothing", () => {
    expect(setInstall("[test]\nroot = \".\"\n", keys)).toBe('[test]\nroot = "."\n\n[install]\nlinker = "isolated"\nglobalStore = true\nexact = true\n');
    expect(setInstall("", { exact: true })).toBe("[install]\nexact = true\n");
  });

  test("replaces existing keys in place, adds missing ones, and keeps comments, other sections and sub-tables", () => {
    const before = '# top\n[install]\nlinker = "hoisted" # old\nsaveTextLockfile = true\n\n[install.cache]\ndir = "x"\n\n[run]\nshell = "bun"\n';
    const after = setInstall(before, keys);
    expect(after).toContain('linker = "isolated" # old');
    expect(after).toContain("saveTextLockfile = true");
    expect(after).toContain("globalStore = true");
    expect(after).toContain('[install.cache]\ndir = "x"');
    expect(after).toContain('[run]\nshell = "bun"');
    expect(after.startsWith("# top\n")).toBe(true);
    expect(installDrift(after, keys)).toEqual([]);
    expect(setInstall(after, keys)).toBe(after);
  });

  test("keeps CRLF line endings", () => {
    const out = setInstall("[install]\r\nlinker = \"hoisted\"\r\n", keys);
    expect(out).toContain("\r\n");
    expect(out.replace(/\r\n/g, "")).not.toContain("\n");
    expect(out).toContain("linker = \"isolated\"");
  });

  test("drift names the key, the value found and the value wanted", () => {
    expect(installDrift('[install]\nlinker = "hoisted"\n', { linker: "isolated" })).toEqual(['bunfig [install] linker is "hoisted", the policy says "isolated"']);
    expect(installDrift("", { exact: true })).toEqual(["bunfig [install] exact is unset, the policy says true"]);
  });
});

describe("rust-toolchain.toml", () => {
  test("renders the minimal profile with the editor components, and targets only when a repository has some", () => {
    const text = renderToolchain(policyOf(raw, "ay"));
    expect(channelOf(text)).toBe(raw.rust.channel);
    expect(text).not.toContain("targets");
    expect(renderToolchain(policyOf(raw, "aphrody"))).toContain('"wasm32-wasip2"');
  });

  test("a channel is replaced without touching comments, components or targets", () => {
    const before = '# CI images install this channel.\n[toolchain]\nchannel = "1.98.1"\ncomponents = ["rustfmt"]\n';
    expect(setChannel(before, "1.100.0")).toBe('# CI images install this channel.\n[toolchain]\nchannel = "1.100.0"\ncomponents = ["rustfmt"]\n');
  });
});

describe("profileDrift", () => {
  test("reports the keys that differ from Zed's profiles and the missing build-override", () => {
    const drift = profileDrift('[workspace]\nmembers = []\n[profile.dev]\ndebug = true\ncodegen-units = 16\n', template);
    expect(drift).toContain('[profile.dev] debug is true, Zed\'s is "limited"');
    expect(drift.some((line) => line.includes("build-override"))).toBe(true);
    expect(drift.some((line) => line.startsWith("[profile.dev] codegen-units"))).toBe(false);
  });

  test("a workspace that matches the template has no drift", () => {
    expect(profileDrift(`[workspace]\n${template}`, template)).toEqual([]);
  });
});

describe("dev profiles", () => {
  test("the dev tables of the template are applied to a manifest, in place when they exist, and the result has no dev drift", () => {
    const tables = devProfiles(template);
    let manifest = '[workspace]\nmembers = []\n\n[profile.dev]\ndebug = true # slow\nopt-level = 1\n\n[profile.release]\nlto = "fat"\n';
    for (const [section, keys] of Object.entries(tables)) manifest = setSection(manifest, section, keys);
    expect(manifest).toContain('debug = "limited" # slow');
    expect(manifest).toContain("opt-level = 1");
    expect(manifest).toContain('lto = "fat"');
    expect(profileDrift(manifest, template).filter((line) => line.startsWith("[profile.dev"))).toEqual([]);
  });
});

describe("hostScript", () => {
  const script = hostScript(raw, ["1.100.0", "nightly-2026-11-14"]);
  test("installs every pinned toolchain with the editor components and llvm-tools, and every cargo tool at its pinned version", () => {
    expect(script).toContain("rustup toolchain install 1.100.0 --profile minimal -c rustfmt -c clippy -c rust-analyzer -c rust-src -c llvm-tools < /dev/null");
    expect(script).toContain("rustup toolchain install nightly-2026-11-14");
    for (const [name, version] of Object.entries(raw.tools)) expect(script).toContain(`${name}@${version}`);
    expect(script).toContain("cargo binstall --no-confirm --locked");
  });

  test("never reads stdin (rustup and binstall would eat the script) and leaves an existing sccache config alone", () => {
    for (const line of script.split("\n").filter((entry) => /^(rustup|cargo)/.test(entry))) expect(line.endsWith("< /dev/null")).toBe(true);
    expect(script).toContain('if [ ! -f "$config" ]');
  });
});
