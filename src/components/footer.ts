// SPDX-License-Identifier: Apache-2.0
/**
 * Aphrody Sovereign Footer
 * Zero external third-party tracking/support redirects; links exclusively to internal sovereign hubs & tools.
 */

export class Footer {
  public element: HTMLElement;
  private settingsMenu!: HTMLDivElement;
  private themeStatusSpan!: HTMLSpanElement;
  private isSettingsOpen = false;
  private onOpenSuite?: (appId: string) => void;

  constructor(
    private currentTheme: "light" | "dark",
    private onToggleTheme: () => void,
    onOpenSuite?: (appId: string) => void,
  ) {
    this.onOpenSuite = onOpenSuite;
    this.element = document.createElement("footer");
    this.element.className = "google-footer";
    this.render();
    this.attachEvents();
  }

  private render(): void {
    const topRow = document.createElement("div");
    topRow.className = "footer-top";
    const locSpan = document.createElement("span");
    locSpan.className = "footer-location";
    locSpan.textContent = "France · EU Sovereign Cluster (fr1 + fr2 + usa)";
    topRow.appendChild(locSpan);

    const bottomRow = document.createElement("div");
    bottomRow.className = "footer-bottom";

    const leftCol = document.createElement("div");
    leftCol.className = "footer-left";
    leftCol.innerHTML = `
      <a href="/search/web" class="footer-link">Architecture Recherche</a>
      <a href="/search/packages" class="footer-link">Catalogue de Paquets</a>
      <a href="/search/benchmarks" class="footer-link">Benchmarks LTO &amp; WASM</a>
      <a href="/llms.txt" class="footer-link">Protocole LLMs.txt</a>
    `;

    const centerCol = document.createElement("div");
    centerCol.className = "footer-center";
    centerCol.innerHTML = `
      <a href="/search/desktop-ui" class="footer-link climate-link">
        <svg class="leaf-icon" viewBox="0 0 24 24" aria-hidden="true">
          <path fill="#34a853" d="M17 8C8 10 5.9 16.17 3.82 21.34l1.89.66.95-2.3c.48.17.98.3 1.34.3C19 20 22 3 22 3c-1 2-8 2.25-13 3.25S2 11.5 2 13.5s1.75 3.75 1.75 3.75C7 8 17 8 17 8z"/>
        </svg>
        <span>Pure Bun v1.4.4 Full-Rust · Zéro dépendance externe</span>
      </a>
    `;

    const rightCol = document.createElement("div");
    rightCol.className = "footer-right";

    const securityLink = document.createElement("a");
    securityLink.href = "/.well-known/security.txt";
    securityLink.className = "footer-link";
    securityLink.textContent = "Sécurité";

    const humansLink = document.createElement("a");
    humansLink.href = "/humans.txt";
    humansLink.className = "footer-link";
    humansLink.textContent = "Ingénierie";

    const settingsWrap = document.createElement("div");
    settingsWrap.className = "settings-wrapper";

    const settingsBtn = document.createElement("button");
    settingsBtn.type = "button";
    settingsBtn.className = "footer-link settings-btn";
    settingsBtn.textContent = "Paramètres";
    settingsBtn.addEventListener("click", () => this.toggleSettings());

    this.settingsMenu = document.createElement("div");
    this.settingsMenu.className = "settings-menu";
    this.settingsMenu.setAttribute("role", "menu");
    this.settingsMenu.innerHTML = `
      <a href="/?suite=search" data-suite="search" class="settings-item" role="menuitem">Cluster Recherche 3-VPS</a>
      <a href="/?suite=craft" data-suite="craft" class="settings-item" role="menuitem">Craft Creative Cloud (WASM)</a>
      <a href="/?suite=docs" data-suite="docs" class="settings-item" role="menuitem">Fluent 2 Office (.docx &amp; Grid)</a>
      <a href="/?suite=drive" data-suite="drive" class="settings-item" role="menuitem">Spacedrive VDFS (Blake3)</a>
      <a href="/.well-known/webmcp.json" class="settings-item" role="menuitem">Manifeste WebMCP</a>
      <div class="settings-menu-divider"></div>
      <button type="button" class="settings-item theme-toggle-item" role="menuitem">
        <span>Thème sombre : </span>
        <span class="theme-status">${this.currentTheme === "dark" ? "Activé" : "Désactivé"}</span>
      </button>
    `;

    for (const item of this.settingsMenu.querySelectorAll<HTMLAnchorElement>(
      "a[data-suite]",
    )) {
      item.addEventListener("click", (ev) => {
        if (this.onOpenSuite && item.dataset.suite) {
          ev.preventDefault();
          this.closeSettings();
          this.onOpenSuite(item.dataset.suite);
        }
      });
    }

    const themeToggleBtn = this.settingsMenu.querySelector(
      ".theme-toggle-item",
    ) as HTMLButtonElement;
    this.themeStatusSpan = this.settingsMenu.querySelector(
      ".theme-status",
    ) as HTMLSpanElement;

    themeToggleBtn.addEventListener("click", () => {
      this.onToggleTheme();
      this.closeSettings();
    });

    settingsWrap.appendChild(settingsBtn);
    settingsWrap.appendChild(this.settingsMenu);

    rightCol.appendChild(securityLink);
    rightCol.appendChild(humansLink);
    rightCol.appendChild(settingsWrap);

    bottomRow.appendChild(leftCol);
    bottomRow.appendChild(centerCol);
    bottomRow.appendChild(rightCol);

    this.element.appendChild(topRow);
    this.element.appendChild(bottomRow);
  }

  private attachEvents(): void {
    document.addEventListener("mousedown", (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      if (this.isSettingsOpen && !this.element.contains(target)) {
        this.closeSettings();
      }
    });

    document.addEventListener("keydown", (e: KeyboardEvent) => {
      if (this.isSettingsOpen && e.key === "Escape") {
        this.closeSettings();
      }
    });
  }

  public updateTheme(newTheme: "light" | "dark"): void {
    this.currentTheme = newTheme;
    if (this.themeStatusSpan) {
      this.themeStatusSpan.textContent =
        newTheme === "dark" ? "Activé" : "Désactivé";
    }
  }

  private toggleSettings(): void {
    if (this.isSettingsOpen) this.closeSettings();
    else this.openSettings();
  }

  private openSettings(): void {
    this.isSettingsOpen = true;
    this.settingsMenu.classList.add("open");
  }

  private closeSettings(): void {
    this.isSettingsOpen = false;
    this.settingsMenu.classList.remove("open");
  }
}
