export class Footer {
  public element: HTMLElement;
  private settingsMenu!: HTMLDivElement;
  private themeStatusSpan!: HTMLSpanElement;
  private isSettingsOpen = false;

  constructor(
    private currentTheme: "light" | "dark",
    private onToggleTheme: () => void,
  ) {
    this.element = document.createElement("footer");
    this.element.className = "google-footer";
    this.render();
    this.attachEvents();
  }

  private render(): void {
    // 1. Top row: France
    const topRow = document.createElement("div");
    topRow.className = "footer-top";
    const locSpan = document.createElement("span");
    locSpan.className = "footer-location";
    locSpan.textContent = "France";
    topRow.appendChild(locSpan);

    // 2. Bottom row
    const bottomRow = document.createElement("div");
    bottomRow.className = "footer-bottom";

    // Left links
    const leftCol = document.createElement("div");
    leftCol.className = "footer-left";
    leftCol.innerHTML = `
      <a href="https://about.aphrody.com" class="footer-link" target="_blank" rel="noreferrer">À propos</a>
      <a href="https://www.google.com/intl/fr_fr/ads/?subid=ww-ww-et-g-awa-a-g_hpafoot1_1!o2&utm_source=google.com&utm_medium=referral&utm_campaign=google_hpafooter&fg=1" class="footer-link" target="_blank" rel="noreferrer">Publicité</a>
      <a href="https://www.google.com/services/?subid=ww-ww-et-g-awa-a-g_hpbfoot1_1!o2&utm_source=google.com&utm_medium=referral&utm_campaign=google_hpbfooter&fg=1" class="footer-link" target="_blank" rel="noreferrer">Entreprise</a>
      <a href="https://google.com/search/howsearchworks/?fg=1" class="footer-link" target="_blank" rel="noreferrer">Comment fonctionne la recherche Google ?</a>
    `;

    // Center climate link
    const centerCol = document.createElement("div");
    centerCol.className = "footer-center";
    centerCol.innerHTML = `
      <a href="https://sustainability.google/intl/fr_fr/?utm_source=googlehpfooter&utm_medium=housepromos&utm_campaign=bottom-footer" class="footer-link climate-link" target="_blank" rel="noreferrer">
        <svg class="leaf-icon" viewBox="0 0 24 24">
          <path fill="#34a853" d="M17 8C8 10 5.9 16.17 3.82 21.34l1.89.66.95-2.3c.48.17.98.3 1.34.3C19 20 22 3 22 3c-1 2-8 2.25-13 3.25S2 11.5 2 13.5s1.75 3.75 1.75 3.75C7 8 17 8 17 8z"/>
        </svg>
        <span>Notre troisième décennie d'action pour le climat</span>
      </a>
    `;

    // Right links & settings
    const rightCol = document.createElement("div");
    rightCol.className = "footer-right";

    const privacyLink = document.createElement("a");
    privacyLink.href = "https://policies.google.com/privacy?hl=fr&fg=1";
    privacyLink.className = "footer-link";
    privacyLink.target = "_blank";
    privacyLink.rel = "noreferrer";
    privacyLink.textContent = "Confidentialité";

    const termsLink = document.createElement("a");
    termsLink.href = "https://policies.google.com/terms?hl=fr&fg=1";
    termsLink.className = "footer-link";
    termsLink.target = "_blank";
    termsLink.rel = "noreferrer";
    termsLink.textContent = "Conditions";

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
      <a href="https://www.google.com/preferences?hl=fr&fg=1" class="settings-item" role="menuitem" target="_blank" rel="noreferrer">Paramètres de recherche</a>
      <a href="https://www.google.com/advanced_search?hl=fr&fg=1" class="settings-item" role="menuitem" target="_blank" rel="noreferrer">Recherche avancée</a>
      <a href="https://www.google.com/history/privacyadvisor/search/unauth?utm_source=googlemenu&fg=1" class="settings-item" role="menuitem" target="_blank" rel="noreferrer">Vos données dans la recherche Google</a>
      <a href="https://www.google.com/history/optout?hl=fr&fg=1" class="settings-item" role="menuitem" target="_blank" rel="noreferrer">Historique des recherches</a>
      <a href="https://support.google.com/websearch/?p=ws_results_help&hl=fr&fg=1" class="settings-item" role="menuitem" target="_blank" rel="noreferrer">Aide sur la recherche</a>
      <div class="settings-menu-divider"></div>
      <button type="button" class="settings-item theme-toggle-item" role="menuitem">
        <span>Thème sombre : </span>
        <span class="theme-status">${this.currentTheme === "dark" ? "Activé" : "Désactivé"}</span>
      </button>
    `;

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

    rightCol.appendChild(privacyLink);
    rightCol.appendChild(termsLink);
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
    if (this.isSettingsOpen) {
      this.closeSettings();
    } else {
      this.openSettings();
    }
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
