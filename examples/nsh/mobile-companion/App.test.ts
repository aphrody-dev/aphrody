import { describe, expect, test } from "bun:test";
import { buildMobileCompanionViewModel, mobileActions } from "./App";

describe("Aphrody Mobile Companion NSH app", () => {
  test("cycles 4 themes, updates bottom nav, approves diffs, and uses zero raw hex in UI classes", () => {
    const initial = buildMobileCompanionViewModel();
    expect(initial.bottomNav.length).toBe(4);
    expect(initial.hostBridgeContract.minTouchTargetPx).toBe(44);
    expect(initial.cardClassName).toContain("bg-card");
    expect(initial.cardClassName).not.toMatch(/#[0-9a-fA-F]{6}/);
    expect(initial.primaryButtonClassName).toContain("min-h-11");
    expect(initial.primaryButtonClassName).not.toMatch(/#[0-9a-fA-F]{6}/);

    expect(mobileActions.cycleTheme()).toBe("dark");
    expect(mobileActions.cycleTheme()).toBe("light");
    expect(mobileActions.cycleTheme()).toBe("oled");
    expect(mobileActions.cycleTheme()).toBe("neural");

    mobileActions.setActiveTab("diffs");
    const diffVm = buildMobileCompanionViewModel();
    expect(diffVm.activeTab).toBe("diffs");
    expect(diffVm.unapprovedCount).toBe(2);

    mobileActions.approveDiffHunk("hunk-tokens");
    expect(buildMobileCompanionViewModel().unapprovedCount).toBe(1);

    const toggled = mobileActions.toggleVpn();
    expect(toggled).toBe(false);
    expect(mobileActions.toggleVpn()).toBe(true);
  });
});
