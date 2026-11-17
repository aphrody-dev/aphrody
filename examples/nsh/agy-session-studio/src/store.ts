// Private reactive signals for the NSH AGY & Storytold Parity Studio.
export type StudioTab = "overview" | "timeline" | "storytold" | "parity";
export type ThemeMode = "dark" | "light";

export interface SessionItem {
  id: string;
  source: "agy" | "gemini";
  title: string;
  stepCount: number;
  toolCallCount: number;
  artifactCount: number;
  updatedAt: string;
  highlights: string[];
}

export interface StudioSnapshot {
  activeTab: StudioTab;
  theme: ThemeMode;
  query: string;
  selectedSessionId: string;
  liveConnected: boolean;
  storytoldRepoCount: number;
  sessions: SessionItem[];
}

let state: StudioSnapshot = {
  activeTab: "overview",
  theme: "dark",
  query: "",
  selectedSessionId: "aace0256-452b-4cd1-83a0-64b5bbce9f27",
  liveConnected: true,
  storytoldRepoCount: 82,
  sessions: [],
};

const listeners = new Set<() => void>();

export function getSnapshot(): StudioSnapshot {
  return state;
}

export function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function update(patch: Partial<StudioSnapshot>): void {
  state = { ...state, ...patch };
  for (const listener of listeners) listener();
}

export function setActiveTab(activeTab: StudioTab): void {
  update({ activeTab });
}

export function toggleTheme(): void {
  update({ theme: state.theme === "dark" ? "light" : "dark" });
}

export function setSearchQuery(query: string): void {
  update({ query });
}

export function selectSession(selectedSessionId: string): void {
  update({ selectedSessionId });
}

export function hydrateSessions(sessions: SessionItem[]): void {
  const selectedSessionId =
    sessions.find((s) => s.id === state.selectedSessionId)?.id ??
    sessions[0]?.id ??
    state.selectedSessionId;
  update({ sessions, selectedSessionId });
}

export function filteredSessions(): SessionItem[] {
  const q = state.query.trim().toLowerCase();
  if (!q) return state.sessions;
  return state.sessions.filter(
    (s) =>
      s.id.toLowerCase().includes(q) ||
      s.title.toLowerCase().includes(q) ||
      s.highlights.some((h) => h.toLowerCase().includes(q)),
  );
}
