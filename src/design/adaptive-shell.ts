/**
 * Aphrody Adaptive Shell
 * Seamless switcher between:
 * 1. google-classic: Pixel-perfect https://www.google.com/ home surface
 * 2. fluent-desktop: Windows 11 Fluent 2 Desktop OS & Command Bar
 * 3. material-adaptive: Material 3 Adaptive Layout (Rail, Drawer, Bottom Bar)
 */

import { getFluentCssVariables } from "./fluent-tokens";
import {
  getMaterialCssVariables,
  getMaterialBreakpoint,
  MaterialBreakpoint,
} from "./material-tokens";

export type ShellMode =
  "google-classic" | "fluent-desktop" | "material-adaptive";

export class AdaptiveShell {
  private currentMode: ShellMode = "google-classic";
  private currentBreakpoint: MaterialBreakpoint = "expanded";
  private listeners: Array<(mode: ShellMode) => void> = [];

  constructor() {
    this.initMode();
    this.setupResizeListener();
  }

  private initMode(): void {
    if (typeof window !== "undefined") {
      const saved = localStorage.getItem(
        "aphrody_shell_mode",
      ) as ShellMode | null;
      if (
        saved &&
        ["google-classic", "fluent-desktop", "material-adaptive"].includes(
          saved,
        )
      ) {
        this.currentMode = saved;
      }
      this.currentBreakpoint = getMaterialBreakpoint(window.innerWidth);
      this.applyMode(this.currentMode);
    }
  }

  private setupResizeListener(): void {
    if (typeof window !== "undefined") {
      window.addEventListener("resize", () => {
        const bp = getMaterialBreakpoint(window.innerWidth);
        if (bp !== this.currentBreakpoint) {
          this.currentBreakpoint = bp;
          document.documentElement.setAttribute("data-md-breakpoint", bp);
        }
      });
    }
  }

  public getMode(): ShellMode {
    return this.currentMode;
  }

  public getBreakpoint(): MaterialBreakpoint {
    return this.currentBreakpoint;
  }

  public setMode(mode: ShellMode): void {
    if (this.currentMode === mode) return;
    this.currentMode = mode;
    if (typeof localStorage !== "undefined") {
      localStorage.setItem("aphrody_shell_mode", mode);
    }
    this.applyMode(mode);
    this.listeners.forEach((fn) => fn(mode));
  }

  private applyMode(mode: ShellMode): void {
    if (typeof document === "undefined") return;
    document.documentElement.setAttribute("data-shell-mode", mode);
    document.documentElement.setAttribute(
      "data-md-breakpoint",
      this.currentBreakpoint,
    );
  }

  public onModeChange(listener: (mode: ShellMode) => void): () => void {
    this.listeners.push(listener);
    return () => {
      this.listeners = this.listeners.filter((l) => l !== listener);
    };
  }

  public static injectStyles(): void {
    if (typeof document === "undefined") return;
    const styleId = "aphrody-design-system-variables";
    let styleEl = document.getElementById(styleId) as HTMLStyleElement | null;
    if (!styleEl) {
      styleEl = document.createElement("style");
      styleEl.id = styleId;
      document.head.appendChild(styleEl);
    }
    styleEl.textContent = `
:root {
  ${getFluentCssVariables("light")}
  ${getMaterialCssVariables("light")}
}
:root[data-theme="dark"] {
  ${getFluentCssVariables("dark")}
  ${getMaterialCssVariables("dark")}
}
    `;
  }
}
