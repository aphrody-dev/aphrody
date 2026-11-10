import { renderMaterialSymbol } from "../design/material-tokens";
import type { ShellMode } from "../design/adaptive-shell";

export interface HeaderCallbacks {
  onToggleApps: () => void;
  onOpenAppWindow: (appId: string) => void;
  onToggleShellMode: () => void;
}

export class Header {
  public element: HTMLElement;
  private appsBtn!: HTMLButtonElement;
  private modeBtn!: HTMLButtonElement;
  private callbacks: HeaderCallbacks;
  private currentMode: ShellMode = "google-classic";

  constructor(callbacks: HeaderCallbacks | (() => void)) {
    if (typeof callbacks === "function") {
      this.callbacks = {
        onToggleApps: callbacks,
        onOpenAppWindow: () => {},
        onToggleShellMode: () => {},
      };
    } else {
      this.callbacks = callbacks;
    }

    this.element = document.createElement("header");
    this.element.className = "google-header";
    this.render();
  }

  private render(): void {
    // Left side: Mode & Quick Shortcuts (for Desktop / Fluent / M3)
    const leftNav = document.createElement("div");
    leftNav.className = "header-left-tools";

    this.modeBtn = document.createElement("button");
    this.modeBtn.className = "header-tool-btn mode-switcher-btn";
    this.modeBtn.title =
      "Basculer le mode de design (Classic / Fluent 2 / Material 3)";
    this.modeBtn.innerHTML = `
      ${renderMaterialSymbol("devices", 16)}
      <span class="mode-label">Mode: Classic</span>
    `;
    this.modeBtn.addEventListener("click", () => {
      this.callbacks.onToggleShellMode();
    });

    const storeQuickBtn = document.createElement("button");
    storeQuickBtn.className = "header-tool-btn";
    storeQuickBtn.title =
      "Ouvrir Universal Store (Play Store + Flathub + NixOS)";
    storeQuickBtn.innerHTML = `${renderMaterialSymbol("storefront", 16)} <span class="hide-mobile">Store</span>`;
    storeQuickBtn.addEventListener("click", () =>
      this.callbacks.onOpenAppWindow("store"),
    );

    const storyQuickBtn = document.createElement("button");
    storyQuickBtn.className = "header-tool-btn";
    storyQuickBtn.title = "Ouvrir Storytold WASM/WebGPU Suite";
    storyQuickBtn.innerHTML = `${renderMaterialSymbol("palette", 16)} <span class="hide-mobile">Storytold</span>`;
    storyQuickBtn.addEventListener("click", () =>
      this.callbacks.onOpenAppWindow("storytold"),
    );

    const benchQuickBtn = document.createElement("button");
    benchQuickBtn.className = "header-tool-btn";
    benchQuickBtn.title = "Ouvrir Stack Benchmark & Auto-Picker";
    benchQuickBtn.innerHTML = `${renderMaterialSymbol("speed", 16)} <span class="hide-mobile">Benchmark</span>`;
    benchQuickBtn.addEventListener("click", () =>
      this.callbacks.onOpenAppWindow("bench"),
    );

    leftNav.appendChild(this.modeBtn);
    leftNav.appendChild(storeQuickBtn);
    leftNav.appendChild(storyQuickBtn);
    leftNav.appendChild(benchQuickBtn);

    // Right side: Authentic Google Navigation
    const nav = document.createElement("div");
    nav.className = "google-header-right";

    const gmailLink = document.createElement("a");
    gmailLink.className = "header-link";
    gmailLink.href = "https://mail.google.com/";
    gmailLink.target = "_blank";
    gmailLink.rel = "noreferrer";
    gmailLink.textContent = "Gmail";

    const imagesLink = document.createElement("a");
    imagesLink.className = "header-link";
    imagesLink.href = "https://www.google.com/imghp?hl=fr";
    imagesLink.target = "_blank";
    imagesLink.rel = "noreferrer";
    imagesLink.textContent = "Images";

    this.appsBtn = document.createElement("button");
    this.appsBtn.type = "button";
    this.appsBtn.className = "apps-button";
    this.appsBtn.setAttribute("aria-label", "Applications Google & Aphrody");
    this.appsBtn.setAttribute("title", "Applications Google & Aphrody");
    this.appsBtn.setAttribute("aria-haspopup", "dialog");
    this.appsBtn.innerHTML = `
      <svg class="apps-icon" focusable="false" viewBox="0 0 24 24">
        <path d="M6,8c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM12,20c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM6,20c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM6,14c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM12,14c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM16,6c0,1.1 0.9,2 2,2s2,-0.9 2,-2 -0.9,-2 -2,-2 -2,0.9 -2,2zM12,8c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM18,14c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM18,20c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2z"/>
      </svg>
    `;
    this.appsBtn.addEventListener("click", () => {
      this.callbacks.onToggleApps();
    });

    const signInBtn = document.createElement("a");
    signInBtn.className = "sign-in-btn";
    signInBtn.href = "https://accounts.google.com/signin";
    signInBtn.target = "_blank";
    signInBtn.rel = "noreferrer";
    signInBtn.textContent = "Connexion";

    nav.appendChild(gmailLink);
    nav.appendChild(imagesLink);
    nav.appendChild(this.appsBtn);
    nav.appendChild(signInBtn);

    this.element.appendChild(leftNav);
    this.element.appendChild(nav);
  }

  public updateShellMode(mode: ShellMode): void {
    this.currentMode = mode;
    const label = this.modeBtn.querySelector(".mode-label");
    if (label) {
      if (mode === "google-classic") label.textContent = "Mode: Classic";
      else if (mode === "fluent-desktop") label.textContent = "Mode: Fluent 2";
      else label.textContent = "Mode: Material 3";
    }
  }

  public setAppsActive(active: boolean): void {
    if (active) {
      this.appsBtn.classList.add("active");
    } else {
      this.appsBtn.classList.remove("active");
    }
  }
}
