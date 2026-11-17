import { describe, expect, test } from "bun:test";
import {
  buildPortableSingleFileHtml,
  buildStudioState,
  parseTranscriptJsonl,
  renderStudioHtml,
} from "./agy-session-studio";

describe("AGY & Gemini NSH Session Studio + Portable Converter", () => {
  test("parses JSONL transcripts into structured tool and file telemetry", () => {
    const sample = [
      JSON.stringify({
        step_index: 0,
        type: "USER_INPUT",
        status: "DONE",
        created_at: "2026-11-17T12:00:00Z",
        content: "<USER_REQUEST>Unifier les 82 dépôts storytold dans Zed</USER_REQUEST>",
      }),
      JSON.stringify({
        step_index: 1,
        type: "PLANNER_RESPONSE",
        status: "DONE",
        created_at: "2026-11-17T12:01:00Z",
        content: "Création de egui_gpui et bevy_gpui.",
        tool_calls: [
          { name: "write_file", args: { Path: "C:/src/aphrody/crates/egui_gpui/src/egui_gpui.rs" } },
          { name: "run_command", args: { CommandLine: "cargo test -p egui_gpui" } },
        ],
      }),
    ].join("\n");

    const report = parseTranscriptJsonl("aace0256-452b-4cd1-83a0-64b5bbce9f27", sample, ["implementation_plan.md"]);
    expect(report.stepCount).toBe(2);
    expect(report.userPromptCount).toBe(1);
    expect(report.toolCallCount).toBe(2);
    expect(report.toolBreakdown.write_file).toBe(1);
    expect(report.touchedPaths).toContain("crates/egui_gpui/src/egui_gpui.rs");
  });

  test("renders semantic Google Design HTML and gzip-compressed portable export", () => {
    const state = buildStudioState();
    expect(state.storytold.totalRepos).toBe(82);
    expect(state.parity.frameworks.length).toBe(4);

    const liveHtml = renderStudioHtml(state);
    expect(liveHtml).toContain('<!DOCTYPE html>');
    expect(liveHtml).toContain('lang="fr"');
    expect(liveHtml).toContain('EventSource("/api/events")');
    expect(liveHtml).toContain("prefers-reduced-motion");

    const portableHtml = buildPortableSingleFileHtml(state);
    expect(portableHtml).toContain('DecompressionStream("gzip")');
    expect(portableHtml).toContain("ARCHIVE PORTABLE GZIP");
  });
});
