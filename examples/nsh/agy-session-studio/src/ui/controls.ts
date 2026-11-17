// App-owned styled UI layer built strictly on semantic theme tokens (zero raw hex/rgba literals).
import type { StudioTab } from "../store";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "outline";
export type ControlSize = "sm" | "md" | "lg";

export interface StyledButtonSpec {
  label: string;
  variant?: ButtonVariant;
  size?: ControlSize;
  selected?: boolean;
  onPress?: () => void;
}

export function resolveButtonTokenClasses(
  variant: ButtonVariant = "secondary",
  size: ControlSize = "md",
  selected = false,
): string {
  const sizeClass =
    size === "sm" ? "px-3 py-1 text-xs" : size === "lg" ? "px-5 py-2.5 text-base" : "px-4 py-2 text-sm";
  if (selected || variant === "primary") {
    return `${sizeClass} rounded-full bg-primary text-primary-foreground transition-transform active:scale-95`;
  }
  if (variant === "outline") {
    return `${sizeClass} rounded-full bg-card text-card-foreground border border-border hover:bg-popover`;
  }
  if (variant === "ghost") {
    return `${sizeClass} rounded-full text-muted-foreground hover:bg-muted hover:text-foreground`;
  }
  return `${sizeClass} rounded-full bg-secondary text-secondary-foreground hover:bg-popover`;
}

export interface SegmentedItem {
  id: StudioTab;
  label: string;
  badge?: string;
}

export const STUDIO_SEGMENTS: readonly SegmentedItem[] = [
  { id: "overview", label: "Rapport Exécutif AGY", badge: "LIVE" },
  { id: "timeline", label: "Chronologie & Outils" },
  { id: "storytold", label: "Storytold 82 Repos", badge: "82" },
  { id: "parity", label: "Parité GPUI · egui · Bevy 5.8" },
];

export interface KpiCardSpec {
  label: string;
  value: string;
  detail: string;
}

export function renderSurfaceCardTokens(): string {
  return "rounded-2xl bg-card text-card-foreground border border-border p-4 gap-2";
}
