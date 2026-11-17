import {
  filteredSessions,
  getSnapshot,
  selectSession,
  setActiveTab,
  setSearchQuery,
  toggleTheme,
} from "./src/store";
import {
  renderSurfaceCardTokens,
  resolveButtonTokenClasses,
  STUDIO_SEGMENTS,
} from "./src/ui/controls";

export interface StudioViewModel {
  headerTitle: string;
  themeLabel: string;
  activeTab: string;
  segmentClasses: Array<{ id: string; label: string; className: string }>;
  cardClassName: string;
  visibleSessionCount: number;
  selectedSessionId: string;
}

export function buildStudioViewModel(): StudioViewModel {
  const snap = getSnapshot();
  const visible = filteredSessions();
  return {
    headerTitle: "Aphrody — Suivi AGY/Gemini & Studio Unifié Storytold",
    themeLabel: snap.theme === "dark" ? "Mode Clair" : "Mode Sombre",
    activeTab: snap.activeTab,
    segmentClasses: STUDIO_SEGMENTS.map((seg) => ({
      id: seg.id,
      label: seg.badge ? `${seg.label} (${seg.badge})` : seg.label,
      className: resolveButtonTokenClasses("secondary", "sm", snap.activeTab === seg.id),
    })),
    cardClassName: renderSurfaceCardTokens(),
    visibleSessionCount: visible.length,
    selectedSessionId: snap.selectedSessionId,
  };
}

export const studioActions = {
  setActiveTab,
  toggleTheme,
  setSearchQuery,
  selectSession,
};
