export type MobileTab = "agent" | "diffs" | "fleet" | "craft";
export type MobileThemeMode = "neural" | "dark" | "light" | "oled";

export interface DiffHunkItem {
  id: string;
  file: string;
  summary: string;
  approved: boolean;
}

export interface FleetJobItem {
  host: string;
  task: string;
  status: "green" | "running" | "queued";
  durationSec: number;
}

export interface MobileCompanionState {
  activeTab: MobileTab;
  theme: MobileThemeMode;
  vpnConnected: boolean;
  otaSignedVerified: boolean;
  promptDraft: string;
  pendingHunks: DiffHunkItem[];
  fleetJobs: FleetJobItem[];
}

const THEME_CYCLE: readonly MobileThemeMode[] = ["neural", "dark", "light", "oled"];

let state: MobileCompanionState = {
  activeTab: "agent",
  theme: "neural",
  vpnConnected: true,
  otaSignedVerified: true,
  promptDraft: "",
  pendingHunks: [
    {
      id: "hunk-tokens",
      file: "crates/aphrody_ui/src/google_design.rs",
      summary: "+ neural_obsidian & oled_pitch M3 color roles (WCAG >= 4.5)",
      approved: false,
    },
    {
      id: "hunk-nav",
      file: "crates/aphrody_ui/src/google_design_components.rs",
      summary: "+ MobileBottomNavBar (64px bar, >= 44px touch targets)",
      approved: false,
    },
  ],
  fleetJobs: [
    { host: "fr1", task: "cargo check -p aphrody_nsh -p theme_settings", status: "green", durationSec: 4.2 },
    { host: "prd", task: "cloudflare-deploy --env releases", status: "green", durationSec: 1.8 },
  ],
};

export function getMobileSnapshot(): MobileCompanionState {
  return state;
}

export function setActiveTab(activeTab: MobileTab): void {
  state = { ...state, activeTab };
}

export function cycleTheme(): MobileThemeMode {
  const idx = THEME_CYCLE.indexOf(state.theme);
  const next = THEME_CYCLE[(idx + 1) % THEME_CYCLE.length];
  state = { ...state, theme: next };
  return next;
}

export function toggleVpn(): boolean {
  state = { ...state, vpnConnected: !state.vpnConnected };
  return state.vpnConnected;
}

export function setPromptDraft(promptDraft: string): void {
  state = { ...state, promptDraft };
}

export function approveDiffHunk(hunkId: string): void {
  state = {
    ...state,
    pendingHunks: state.pendingHunks.map((h) => (h.id === hunkId ? { ...h, approved: true } : h)),
  };
}
