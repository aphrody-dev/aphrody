import { describe, expect, test } from "bun:test";
import { buildDataset, parseConfig, rationaleOf, summarize } from "./kconfig.ts";

describe("kconfig dataset", () => {
  const fragments = new Map([
    ["android", parseConfig("CONFIG_RUST=y\nCONFIG_ANDROID_BINDER_IPC_RUST=y\n# CONFIG_X86_NATIVE_CPU is not set\n")],
    ["optimize", parseConfig("CONFIG_HZ_1000=y\nCONFIG_DEFAULT_BBR=y\n")],
  ]);
  const final = parseConfig("CONFIG_RUST=y\nCONFIG_ANDROID_BINDER_IPC_RUST=y\nCONFIG_HZ_1000=y\n# CONFIG_X86_NATIVE_CPU is not set\n");
  const defaults = parseConfig("# CONFIG_RUST is not set\nCONFIG_HZ_250=y\n# CONFIG_HZ_1000 is not set\n");
  const entries = buildDataset(fragments, final, defaults, null);

  test("parses set, string and not-set lines", () => {
    expect(parseConfig('CONFIG_A=y\nCONFIG_B="x y"\n# CONFIG_C is not set\nnoise\n')).toEqual(new Map([["CONFIG_A", "y"], ["CONFIG_B", '"x y"'], ["CONFIG_C", "n"]]));
  });
  test("compares the merged result with the tree defaults and flags what did not take effect", () => {
    const rust = entries.find((e) => e.symbol === "CONFIG_RUST")!;
    expect(rust).toMatchObject({ final: "y", defaults: "n", applied: true, deltaVsDefaults: true, rationale: ["required-by:android"] });
    const bbr = entries.find((e) => e.symbol === "CONFIG_DEFAULT_BBR")!;
    expect(bbr).toMatchObject({ final: "n", applied: false, rationale: ["perf"] });
    expect(entries.find((e) => e.symbol === "CONFIG_X86_NATIVE_CPU")).toMatchObject({ applied: true });
    expect(summarize(entries).notApplied).toEqual(["CONFIG_DEFAULT_BBR: wanted y, got n"]);
  });
  test("rationale tags", () => {
    expect(rationaleOf("rust")).toBe("rust");
    expect(rationaleOf("wsl")).toBe("required-by:wsl");
  });
});
