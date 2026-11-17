import { describe, expect, test } from "bun:test";
import { existsSync } from "node:fs";
import {
  buildWasmEcosystemCatalog,
  classifyWasmRepo,
  DEFAULT_TOML_OUT,
  serializeWasmEcosystemToml,
} from "./webassembly-org-ingest.ts";

describe("webassembly-org-ingest", () => {
  test("sorts orgs/WebAssembly repos descending by stars and classifies accurately", () => {
    expect(classifyWasmRepo("binaryen", "Optimizer library", [])).toBe("toolchain-optimizer");
    expect(classifyWasmRepo("component-model", "WIT components", [])).toBe("component-model-wasi");

    const cat = buildWasmEcosystemCatalog(
      [
        { name: "wabt", stargazers_count: 7300, description: "Binary toolkit" },
        { name: "binaryen", stargazers_count: 9200, description: "Optimizer" },
      ],
      "2026-11-17T00:00:00Z",
    );
    expect(cat.repos[0].name).toBe("binaryen");
    expect(cat.repos[1].name).toBe("wabt");

    const parsed = Bun.TOML.parse(serializeWasmEcosystemToml(cat)) as {
      total_repos: number;
      repos: Array<{ name: string }>;
    };
    expect(parsed.total_repos).toBe(2);
    expect(parsed.repos[0].name).toBe("binaryen");
    if (existsSync(DEFAULT_TOML_OUT)) {
      expect(existsSync(DEFAULT_TOML_OUT)).toBe(true);
    }
  });
});
