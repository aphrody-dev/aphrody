import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import {
  applyBundleToLocalHarnesses,
  collectJournalDeltas,
  renderCloudMemoryMarkdown,
  resolveClientPaths,
  type RemoteBundle,
} from "./learn-cloud.ts";

let tmpHome: string;

beforeAll(() => {
  tmpHome = mkdtempSync(join(tmpdir(), "aphrody-learn-cloud-test-"));
});

afterAll(() => {
  rmSync(tmpHome, { recursive: true, force: true });
});

describe("learn-cloud cross-agent autonomous sync", () => {
  test("collects deltas across agy & claude journals with secret redaction", () => {
    const paths = resolveClientPaths(tmpHome, tmpHome);
    mkdirSync(`${paths.geminiDir}/learning`, { recursive: true });
    mkdirSync(`${paths.claudeDir}/learning`, { recursive: true });

    writeFileSync(
      `${paths.geminiDir}/learning/journal.jsonl`,
      `${JSON.stringify({
        at: "2026-11-18T12:00:00Z",
        harness: "agy",
        kind: "correction",
        summary: "Always drain session_inbox with Bearer supersecrettoken1234567890",
      })}\n`,
    );
    writeFileSync(
      `${paths.claudeDir}/learning/journal.jsonl`,
      `${JSON.stringify({
        at: "2026-11-18T12:05:00Z",
        harness: "claude",
        kind: "recovery",
        tool: "wrangler",
        summary: "Recovered wrangler deploy --env learn cleanly",
      })}\n`,
    );

    const first = collectJournalDeltas(paths, {});
    expect(first.entries.length).toBe(2);
    expect(first.entries[0]?.summary).toContain("[REDACTED]");
    expect(first.entries[0]?.summary).not.toContain("supersecrettoken1234567890");

    const second = collectJournalDeltas(paths, first.nextCursors);
    expect(second.entries.length).toBe(0);
  });

  test("materializes unified bundle to gemini, claude, and codex without cross-path leaks", () => {
    const home = join(tmpHome, "converge");
    const paths = resolveClientPaths(home, home);
    mkdirSync(`${paths.geminiDir}/memory`, { recursive: true });
    writeFileSync(`${paths.geminiDir}/memory/MEMORY.md`, "# Index\n");
    mkdirSync(`${paths.claudeDir}/projects/shared/memory`, { recursive: true });
    mkdirSync(paths.codexDir, { recursive: true });

    const bundle: RemoteBundle = {
      schemaVersion: 1,
      revision: 7,
      sha256: "abc123",
      updatedAt: "2026-11-18T15:30:00Z",
      sources: { harnesses: ["agy", "claude", "codex"], pcs: ["pc-1", "fr1"] },
      rules: { globalDirectives: { fleet: "- Drain relay session_inbox on wakeups." } },
      memory: {
        topics: {
          "cross-session": {
            slug: "cross-session",
            title: "Cross-Session & Relay Coordination",
            summary: "Shared lock protocol across active sessions.",
            bullets: ["[correction] Drain session_inbox before shared edits."],
            updatedAt: "2026-11-18T15:30:00Z",
            sources: ["agy:pc-1"],
            confidence: 0.95,
          },
        },
      },
      brain: {
        docs: {
          "gpui/INDEX.md": {
            path: "gpui/INDEX.md",
            sha256: "deadbeef",
            content: "# GPUI Brain Index\n",
            updatedAt: "2026-11-18T15:30:00Z",
          },
        },
      },
      skills: {
        deltas: {
          fleet: { skill: "fleet", notes: ["Check fleet_status before fr1 builds."], updatedAt: "2026-11-18T15:30:00Z" },
        },
      },
      mcp: {
        profile: {
          keepBaseline: ["relay", "bun-brain", "os-craft"],
          disableRecommended: ["vercel", "rea"],
          updatedAt: "2026-11-18T15:30:00Z",
        },
      },
    };

    const md = renderCloudMemoryMarkdown(bundle);
    expect(md).toContain("(rev 7)");
    expect(md).toContain("Cross-Session & Relay Coordination");

    const res = applyBundleToLocalHarnesses(bundle, paths, { runLocalSync: false });
    expect(res.harnessesTouched).toEqual(["agy", "claude", "codex"]);
    expect(existsSync(`${paths.geminiDir}/memory/learned-cloud.md`)).toBe(true);
    expect(existsSync(`${paths.claudeDir}/projects/shared/memory/learned-cloud.md`)).toBe(true);
    expect(existsSync(`${paths.codexDir}/LEARNED_CLOUD.md`)).toBe(true);
    expect(readFileSync(`${paths.geminiDir}/memory/MEMORY.md`, "utf8")).toContain("(learned-cloud.md)");
    expect(readFileSync(`${paths.geminiDir}/brain/cloud/gpui/INDEX.md`, "utf8")).toBe("# GPUI Brain Index\n");
  });
});
