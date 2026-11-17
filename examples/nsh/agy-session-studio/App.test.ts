import { describe, expect, test } from "bun:test";
import { buildStudioViewModel, studioActions } from "./App";
import { getSnapshot, hydrateSessions } from "./src/store";

describe("NSH AGY & Storytold Parity Studio app model", () => {
  test("hydrates sessions, filters reactively, and uses semantic token classes only", () => {
    hydrateSessions([
      {
        id: "aace0256-452b-4cd1-83a0-64b5bbce9f27",
        source: "agy",
        title: "Google Design Stitch, Wasmtime AOT & 24-Tool aphrody_cli MCP",
        stepCount: 214,
        toolCallCount: 168,
        artifactCount: 3,
        updatedAt: "2026-11-17T13:00:00Z",
        highlights: ["e544b6b90d", "aphrody_design", "starter_pack"],
      },
      {
        id: "4b7bb484-ef0e-44b2-a766-b0becc2adca3",
        source: "agy",
        title: "Storytold 82 Repos + egui 0.37 + Unreal Bevy 5.8 Unification",
        stepCount: 96,
        toolCallCount: 84,
        artifactCount: 1,
        updatedAt: "2026-11-17T14:30:00Z",
        highlights: ["egui_gpui", "bevy_gpui", "aphrody_craft"],
      },
    ]);

    const vm = buildStudioViewModel();
    expect(vm.visibleSessionCount).toBe(2);
    expect(vm.cardClassName).toContain("bg-card");
    expect(vm.cardClassName).not.toMatch(/#[0-9a-fA-F]{6}/);

    studioActions.setSearchQuery("bevy_gpui");
    expect(buildStudioViewModel().visibleSessionCount).toBe(1);

    studioActions.setSearchQuery("");
    studioActions.setActiveTab("parity");
    expect(getSnapshot().activeTab).toBe("parity");

    const initialTheme = getSnapshot().theme;
    studioActions.toggleTheme();
    expect(getSnapshot().theme).not.toBe(initialTheme);
  });
});
