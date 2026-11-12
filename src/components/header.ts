// SPDX-License-Identifier: Apache-2.0
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
    const leftNav = document.createElement("div");
    leftNav.className = "header-left-tools";

    this.modeBtn = document.createElement("button");
    this.modeBtn.type = "button";
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

    const quickItems: Array<{
      id: string;
      icon: string;
      label: string;
      title: string;
    }> = [
      {
        id: "craft",
        icon: "palette",
        label: "Craft Cloud",
        title: "Ouvrir Craft Creative Cloud (WASM + WebGPU)",
      },
      {
        id: "docs",
        icon: "code",
        label: "Office 365",
        title: "Ouvrir Fluent 2 Office (WordCraft, GridCraft, DeckCraft)",
      },
      {
        id: "drive",
        icon: "storefront",
        label: "Spacedrive",
        title: "Ouvrir Spacedrive VDFS (Blake3 CAS)",
      },
      {
        id: "studio",
        icon: "rocket_launch",
        label: "Local Studio",
        title: "Ouvrir Locally Uncensored Studio",
      },
      {
        id: "bench",
        icon: "speed",
        label: "Benchmark",
        title: "Ouvrir Stack Benchmark & Picker",
      },
    ];

    leftNav.appendChild(this.modeBtn);
    for (const item of quickItems) {
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = "header-tool-btn";
      btn.title = item.title;
      btn.innerHTML = `${renderMaterialSymbol(item.icon, 16)} <span class="hide-mobile">${item.label}</span>`;
      btn.addEventListener("click", () =>
        this.callbacks.onOpenAppWindow(item.id),
      );
      leftNav.appendChild(btn);
    }

    const nav = document.createElement("div");
    nav.className = "google-header-right";

    const mailLink = document.createElement("a");
    mailLink.className = "header-link";
    mailLink.href = "/?suite=mail";
    mailLink.textContent = "Mail";
    mailLink.addEventListener("click", (e) => {
      e.preventDefault();
      this.callbacks.onOpenAppWindow("mail");
    });

    const photosLink = document.createElement("a");
    photosLink.className = "header-link";
    photosLink.href = "/?suite=photos";
    photosLink.textContent = "PhotoCraft";
    photosLink.addEventListener("click", (e) => {
      e.preventDefault();
      this.callbacks.onOpenAppWindow("photos");
    });

    this.appsBtn = document.createElement("button");
    this.appsBtn.type = "button";
    this.appsBtn.className = "apps-button";
    this.appsBtn.setAttribute("aria-label", "Applications Aphrody Workspace");
    this.appsBtn.setAttribute("title", "Applications Aphrody Workspace");
    this.appsBtn.setAttribute("aria-haspopup", "dialog");
    this.appsBtn.innerHTML = `
      <svg class="apps-icon" focusable="false" viewBox="0 0 24 24" aria-hidden="true">
        <path d="M6,8c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM12,20c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM6,20c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM6,14c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM12,14c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM16,6c0,1.1 0.9,2 2,2s2,-0.9 2,-2 -0.9,-2 -2,-2 -2,0.9 -2,2zM12,8c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM18,14c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM18,20c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2z"/>
      </svg>
    `;
    this.appsBtn.addEventListener("click", () => {
      this.callbacks.onToggleApps();
    });

    const signInBtn = document.createElement("a");
    signInBtn.className = "sign-in-btn";
    signInBtn.href = "/?suite=account";
    signInBtn.textContent = "Compte";
    signInBtn.addEventListener("click", (e) => {
      e.preventDefault();
      this.callbacks.onOpenAppWindow("account");
    });

    nav.appendChild(mailLink);
    nav.appendChild(photosLink);
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
    if (active) this.appsBtn.classList.add("active");
    else this.appsBtn.classList.remove("active");
  }
}
