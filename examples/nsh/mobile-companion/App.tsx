import {
  approveDiffHunk,
  cycleTheme,
  getMobileSnapshot,
  setActiveTab,
  setPromptDraft,
  toggleVpn,
  type MobileTab,
  type MobileThemeMode,
} from "./src/store";

export interface MobileTabModel {
  id: MobileTab;
  label: string;
  selected: boolean;
  badge?: string;
  className: string;
}

export interface MobileCompanionViewModel {
  title: string;
  activeTab: MobileTab;
  theme: MobileThemeMode;
  vpnLabel: string;
  otaLabel: string;
  unapprovedCount: number;
  cardClassName: string;
  omnibarClassName: string;
  primaryButtonClassName: string;
  bottomNav: MobileTabModel[];
  hostBridgeContract: {
    iosFramework: string;
    androidAar: string;
    minTouchTargetPx: number;
  };
}

export function resolveMobileTabClasses(selected: boolean): string {
  const base = "min-h-11 min-w-11 px-3 rounded-full text-xs font-semibold flex items-center justify-center";
  return selected
    ? `${base} bg-primary text-primary-foreground`
    : `${base} bg-muted text-muted-foreground`;
}

export function buildMobileCompanionViewModel(): MobileCompanionViewModel {
  const snap = getMobileSnapshot();
  const unapprovedCount = snap.pendingHunks.filter((h) => !h.approved).length;
  const tabs: Array<{ id: MobileTab; label: string; badge?: string }> = [
    { id: "agent", label: "Agent" },
    { id: "diffs", label: "Diffs", badge: unapprovedCount > 0 ? String(unapprovedCount) : undefined },
    { id: "fleet", label: "Fleet" },
    { id: "craft", label: "Craft" },
  ];

  return {
    title: "Aphrody Companion",
    activeTab: snap.activeTab,
    theme: snap.theme,
    vpnLabel: snap.vpnConnected ? "WireGuard FR-PAR Protected" : "Direct LAN Only",
    otaLabel: snap.otaSignedVerified ? "NshTrust Ed25519 Verified" : "OTA Unverified",
    unapprovedCount,
    cardClassName: "rounded-2xl bg-card text-card-foreground border border-border p-4 gap-2",
    omnibarClassName: "min-h-11 rounded-full bg-popover text-popover-foreground border border-primary px-4 py-2",
    primaryButtonClassName: "min-h-11 rounded-full bg-primary text-primary-foreground px-5 py-2.5 font-semibold",
    bottomNav: tabs.map((t) => ({
      id: t.id,
      label: t.label,
      badge: t.badge,
      selected: t.id === snap.activeTab,
      className: resolveMobileTabClasses(t.id === snap.activeTab),
    })),
    hostBridgeContract: {
      iosFramework: "NativeShell.xcframework (gpui_ios Metal)",
      androidAar: "nsh-platform-android.aar (Vulkan JNI)",
      minTouchTargetPx: 44,
    },
  };
}

export const mobileActions = {
  setActiveTab,
  cycleTheme,
  toggleVpn,
  setPromptDraft,
  approveDiffHunk,
};
