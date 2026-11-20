import { describe, expect, test } from "bun:test";
import {
  CANONICAL_REFERENCE_FORKS,
  loadManifest,
  validateReferenceForks,
} from "./multimedia-forks-audit";

describe("multimedia-forks-audit", () => {
  test("canonical reference forks cover all 6 domains without collisions", () => {
    expect(validateReferenceForks(CANONICAL_REFERENCE_FORKS)).toEqual([]);
    const domains = new Set(CANONICAL_REFERENCE_FORKS.map((f) => f.domain));
    expect(domains.has("media")).toBe(true);
    expect(domains.has("google-rust")).toBe(true);
    expect(domains.has("google-go")).toBe(true);
    expect(domains.has("google-ts")).toBe(true);
    expect(domains.has("google-python")).toBe(true);
    expect(domains.has("google-kotlin")).toBe(true);
    expect(CANONICAL_REFERENCE_FORKS.some((f) => f.repo === "aphrody-labs/google-workspace-cli")).toBe(true);
    expect(CANONICAL_REFERENCE_FORKS.some((f) => f.repo === "aphrody-labs/remotion")).toBe(true);
  });

  test("forks-manifest.json keeps referenceForks separate from OCI Cargo forks", () => {
    const manifest = loadManifest();
    expect(Array.isArray(manifest.forks)).toBe(true);
    expect(manifest.forks.length).toBeGreaterThanOrEqual(11);
    if (manifest.referenceForks) {
      expect(validateReferenceForks(manifest.referenceForks)).toEqual([]);
    }
  });
});
