import { describe, expect, test } from "bun:test";
import {
  DEDUPLICATION_RULES,
  STORYTOLD_82_REPOS,
  auditStorytoldUnification,
  serializeStorytoldManifestToml,
} from "./storytold-unify.ts";

describe("storytold 82-repo monorepo unification & Cargo deduplication", () => {
  test("accounts for all 82 unique storytold GitHub repositories across 6 categories", () => {
    expect(STORYTOLD_82_REPOS.length).toBe(82);
    const names = new Set(STORYTOLD_82_REPOS.map((r) => r.name));
    expect(names.size).toBe(82);
    expect(names.has("figcraft")).toBe(true);
    expect(names.has("craft-anything")).toBe(true);
    expect(names.has("craft-libs")).toBe(true);
    expect(names.has("storyteller-bevy")).toBe(true);
    expect(names.has("egui")).toBe(true);
    expect(DEDUPLICATION_RULES.length).toBeGreaterThanOrEqual(5);
  });

  test("serializes valid TOML and passes root Cargo.toml deduplication audit", () => {
    const tomlText = serializeStorytoldManifestToml();
    const parsed = Bun.TOML.parse(tomlText) as { total_repositories: number; repositories: unknown[] };
    expect(parsed.total_repositories).toBe(82);
    expect(parsed.repositories.length).toBe(82);

    const report = auditStorytoldUnification();
    expect(report.ok).toBe(true);
    expect(report.errors).toEqual([]);
  });
});
