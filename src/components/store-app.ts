/**
 * Universal Store Application UI
 * Material 3 Storefront + Flathub Badges + NixOS Declarative Generations
 * Zero external npm dependencies - 100% pure Web Standards
 */

import { StoreClient } from "../store/client";
import type { StorePackage, StoreGeneration, Ecosystem } from "../store/types";
import { renderMaterialSymbol } from "../design/material-tokens";

export class StoreApp {
  public element: HTMLDivElement;
  private client: StoreClient;
  private currentTab: "browse" | "generations" | "awesome" = "browse";
  private selectedEcosystem: Ecosystem | "all" = "all";
  private searchQuery = "";
  private packages: StorePackage[] = [];
  private generations: StoreGeneration[] = [];
  private activeGeneration: StoreGeneration | null = null;

  constructor() {
    this.client = new StoreClient();
    this.element = document.createElement("div");
    this.element.className = "store-app-wrapper";
    this.render();
    this.loadData();
  }

  private render(): void {
    this.element.innerHTML = `
      <div class="store-app-toolbar">
        <div class="store-app-tabs">
          <button class="store-tab-btn active" data-tab="browse">
            ${renderMaterialSymbol("storefront", 16)} <span>Parcourir</span>
          </button>
          <button class="store-tab-btn" data-tab="generations">
            ${renderMaterialSymbol("layers", 16)} <span>Générations NixOS</span>
          </button>
          <button class="store-tab-btn" data-tab="awesome">
            ${renderMaterialSymbol("auto_awesome", 16)} <span>Awesome Converter</span>
          </button>
        </div>
        <div class="store-search-box">
          ${renderMaterialSymbol("search", 16)}
          <input type="text" class="store-search-input" placeholder="Rechercher des paquets (Bun, Winget, AUR, Nix, Flathub)..." />
        </div>
      </div>
      <div class="store-ecosystem-filters">
        <button class="eco-filter-chip active" data-eco="all">Tous (Multi-Écosystème)</button>
        <button class="eco-filter-chip" data-eco="bun">Bun (Full Rust)</button>
        <button class="eco-filter-chip" data-eco="winget">Winget (Windows NT)</button>
        <button class="eco-filter-chip" data-eco="wasm-hub">WASM + WebGPU</button>
        <button class="eco-filter-chip" data-eco="nix">Nixpkgs / Flakes</button>
        <button class="eco-filter-chip" data-eco="flatpak">Flathub Sandbox</button>
        <button class="eco-filter-chip" data-eco="aur">Arch Linux AUR</button>
        <button class="eco-filter-chip" data-eco="playstore">Play Store APK</button>
      </div>
      <div class="store-content-area">
        <div class="store-loading">Chargement du catalogue universel...</div>
      </div>
    `;

    this.attachEvents();
  }

  private attachEvents(): void {
    // Tabs
    const tabBtns = this.element.querySelectorAll(".store-tab-btn");
    tabBtns.forEach((btn) => {
      btn.addEventListener("click", () => {
        tabBtns.forEach((b) => b.classList.remove("active"));
        btn.classList.add("active");
        this.currentTab = (btn as HTMLElement).dataset.tab as any;
        this.renderCurrentView();
      });
    });

    // Ecosystem filter chips
    const ecoChips = this.element.querySelectorAll(".eco-filter-chip");
    ecoChips.forEach((chip) => {
      chip.addEventListener("click", () => {
        ecoChips.forEach((c) => c.classList.remove("active"));
        chip.classList.add("active");
        this.selectedEcosystem = (chip as HTMLElement).dataset.eco as any;
        this.loadData();
      });
    });

    // Search input
    const searchInput = this.element.querySelector(
      ".store-search-input",
    ) as HTMLInputElement;
    let debounceTimer: any = null;
    searchInput?.addEventListener("input", () => {
      clearTimeout(debounceTimer);
      debounceTimer = setTimeout(() => {
        this.searchQuery = searchInput.value;
        this.loadData();
      }, 250);
    });
  }

  public async loadData(): Promise<void> {
    try {
      this.packages = await this.client.search(
        this.searchQuery,
        this.selectedEcosystem,
      );
      const genData = await this.client.listGenerations();
      this.generations = genData.generations;
      this.activeGeneration = genData.active;
      this.renderCurrentView();
    } catch (e) {
      const area = this.element.querySelector(".store-content-area");
      if (area) {
        area.innerHTML = `<div class="store-error">Erreur lors de la récupération des paquets.</div>`;
      }
    }
  }

  private renderCurrentView(): void {
    const area = this.element.querySelector(".store-content-area");
    if (!area) return;

    if (this.currentTab === "browse") {
      this.renderBrowseView(area);
    } else if (this.currentTab === "generations") {
      this.renderGenerationsView(area);
    } else {
      this.renderAwesomeView(area);
    }
  }

  private renderBrowseView(container: Element): void {
    if (this.packages.length === 0) {
      container.innerHTML = `<div class="store-empty">Aucun paquet trouvé pour cette recherche.</div>`;
      return;
    }

    const grid = document.createElement("div");
    grid.className = "store-package-grid";

    this.packages.forEach((pkg) => {
      const card = document.createElement("div");
      card.className = "store-package-card md-surface-card";

      const ecoBadgeClass = `badge-eco-${pkg.ecosystem}`;
      const optimalStackName = pkg.pickedStack
        ? pkg.pickedStack.recommendedStack.name
        : "Full Rust Fat LTO";

      card.innerHTML = `
        <div class="card-header">
          <div class="pkg-title-wrap">
            <span class="pkg-name">${pkg.name}</span>
            <span class="pkg-version">v${pkg.version}</span>
          </div>
          <span class="badge-eco ${ecoBadgeClass}">${pkg.ecosystem.toUpperCase()}</span>
        </div>
        <p class="pkg-desc">${pkg.description}</p>
        <div class="pkg-meta-row">
          <span class="meta-item">${renderMaterialSymbol("tune", 14)} ${pkg.category}</span>
          <span class="meta-item">★ ${pkg.stars || 100}</span>
          ${pkg.sandboxed ? '<span class="badge-sandboxed">Sandboxed</span>' : ""}
        </div>
        <div class="pkg-stack-recommendation">
          <span class="stack-label">Stack Optimale:</span>
          <span class="stack-name">${optimalStackName}</span>
        </div>
        <div class="card-footer">
          <code class="install-cmd" title="Click to copy">${pkg.installCmd}</code>
          <button class="btn-install-pkg" data-id="${pkg.id}">Installer</button>
        </div>
      `;

      // Copy install command on click
      card.querySelector(".install-cmd")?.addEventListener("click", () => {
        navigator.clipboard?.writeText(pkg.installCmd);
        const codeEl = card.querySelector(".install-cmd") as HTMLElement;
        codeEl.textContent = "Copié dans le presse-papier !";
        setTimeout(() => {
          codeEl.textContent = pkg.installCmd;
        }, 1500);
      });

      // Install button
      card
        .querySelector(".btn-install-pkg")
        ?.addEventListener("click", async () => {
          try {
            const gen = await this.client.installPackage(pkg.id);
            alert(
              `Paquet ${pkg.name} installé ! Nouvelle génération #${gen.id} créée.`,
            );
            this.loadData();
          } catch (err: any) {
            alert(`Erreur: ${err.message}`);
          }
        });

      grid.appendChild(card);
    });

    container.innerHTML = "";
    container.appendChild(grid);
  }

  private renderGenerationsView(container: Element): void {
    const list = document.createElement("div");
    list.className = "store-generations-list";

    const activeGen = this.activeGeneration ||
      this.generations.find((g) => g.active) || {
        id: 1,
        description: "Initial Clean State",
        packages: [],
      };

    list.innerHTML = `
      <div class="generations-header">
        <h3>Générations Déclaratives NixOS & Rollbacks</h3>
        <p>Chaque installation ou suppression crée un état système immuable traçable avec rollback instantané.</p>
        <div class="active-gen-banner">
          Génération Active: <b>#${activeGen.id}</b> | ${activeGen.description} (${activeGen.packages.length} paquets actifs)
        </div>
      </div>
      <div class="gen-cards-wrapper"></div>
    `;

    const cardsWrapper = list.querySelector(".gen-cards-wrapper")!;

    this.generations.forEach((gen) => {
      const card = document.createElement("div");
      card.className = `gen-card ${gen.active ? "active-generation" : ""}`;

      card.innerHTML = `
        <div class="gen-card-left">
          <div class="gen-id-badge">#${gen.id}</div>
          <div class="gen-info">
            <div class="gen-desc">
              ${gen.description}
              ${gen.rollbackId ? `<span class="rollback-tag">Rollback de #${gen.rollbackId}</span>` : ""}
              ${gen.active ? '<span class="active-badge">Actuelle</span>' : ""}
            </div>
            <div class="gen-time">${new Date(gen.timestamp).toLocaleString()}</div>
            <div class="gen-pkgs">Paquets: [${gen.packages.join(", ") || "aucun"}]</div>
          </div>
        </div>
        <div class="gen-card-right">
          ${!gen.active ? `<button class="btn-rollback" data-id="${gen.id}">Revenir à cette version</button>` : '<span class="status-active-label">En cours d\'exécution</span>'}
        </div>
      `;

      card
        .querySelector(".btn-rollback")
        ?.addEventListener("click", async () => {
          const newGen = await this.client.rollback(gen.id);
          alert(
            `Rollback exécuté vers l'état #${gen.id}. Nouvelle génération #${newGen.id} créée.`,
          );
          this.loadData();
        });

      cardsWrapper.appendChild(card);
    });

    container.innerHTML = "";
    container.appendChild(list);
  }

  private renderAwesomeView(container: Element): void {
    const wrap = document.createElement("div");
    wrap.className = "awesome-converter-panel";

    wrap.innerHTML = `
      <div class="awesome-header">
        <h3>Awesome-List vers Universal Store Converter</h3>
        <p>Collez une URL GitHub Awesome List (ex: <code>https://github.com/sindresorhus/awesome</code>) ou du Markdown brut pour indexer automatiquement tous les outils avec commande d'installation et analyse de stack de compilation optimale.</p>
      </div>
      <div class="awesome-form">
        <input type="text" class="awesome-url-input" placeholder="https://github.com/owner/awesome-repo ou laissez vide pour la démo..." />
        <button class="btn-convert-awesome">Convertir & Indexer</button>
      </div>
      <div class="awesome-preview-area"></div>
    `;

    const btn = wrap.querySelector(".btn-convert-awesome");
    const input = wrap.querySelector(".awesome-url-input") as HTMLInputElement;
    const previewArea = wrap.querySelector(".awesome-preview-area")!;

    btn?.addEventListener("click", async () => {
      previewArea.innerHTML =
        '<div class="store-loading">Conversion et indexation en cours...</div>';
      const url = input.value.trim() || "https://github.com/gfx-rs/wgpu";
      try {
        const imported = await this.client.convertAwesome(url);
        previewArea.innerHTML = `
          <div class="conversion-success">
            <b>Succès:</b> ${imported.length} paquet(s) convertis et indexés dans le store universel !
          </div>
          <div class="converted-list">
            ${imported
              .slice(0, 8)
              .map(
                (p) => `
              <div class="converted-item">
                <span class="converted-name">${p.name} [${p.ecosystem}]</span>
                <span class="converted-stack">${p.pickedStack?.recommendedStack.name || "Optimal Stack"}</span>
                <code>${p.installCmd}</code>
              </div>
            `,
              )
              .join("")}
          </div>
        `;
        this.loadData();
      } catch (err: any) {
        previewArea.innerHTML = `<div class="store-error">Erreur de conversion: ${err.message}</div>`;
      }
    });

    container.innerHTML = "";
    container.appendChild(wrap);
  }
}
