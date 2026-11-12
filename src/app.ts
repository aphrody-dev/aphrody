// SPDX-License-Identifier: Apache-2.0
import { AppsPopover } from "./components/apps-popover";
import { Footer } from "./components/footer";
import { Header } from "./components/header";
import { LanguageBar } from "./components/language-bar";
import { LensModal } from "./components/lens-modal";
import { Logo } from "./components/logo";
import { SearchBar } from "./components/search-bar";
import { VoiceModal } from "./components/voice-modal";
import { AdaptiveShell, type ShellMode } from "./design/adaptive-shell";
import { WindowManager } from "./windowing/macos-window";

export class App {
  public rootElement: HTMLDivElement;
  private theme: "light" | "dark" = "light";
  private adaptiveShell: AdaptiveShell;
  private windowManager: WindowManager;

  private header!: Header;
  private appsPopover!: AppsPopover;
  private logo!: Logo;
  private searchBar!: SearchBar;
  private languageBar!: LanguageBar;
  private footer!: Footer;
  private voiceModal!: VoiceModal;
  private lensModal!: LensModal;
  private dockElement!: HTMLDivElement;
  private windowContainer!: HTMLDivElement;

  constructor() {
    this.rootElement = document.createElement("div");
    this.rootElement.className = "google-page-wrapper";
    this.adaptiveShell = new AdaptiveShell();
    this.windowManager = WindowManager.getInstance();

    AdaptiveShell.injectStyles();
    this.initTheme();
    this.buildUI();
    this.handleInitialRoute();
  }

  private initTheme(): void {
    if (typeof window !== "undefined") {
      const saved = localStorage.getItem("aphrody_theme");
      if (saved === "dark" || saved === "light") {
        this.theme = saved;
      } else if (
        window.matchMedia &&
        window.matchMedia("(prefers-color-scheme: dark)").matches
      ) {
        this.theme = "dark";
      }
      document.documentElement.setAttribute("data-theme", this.theme);
    }
  }

  private toggleTheme(): void {
    this.theme = this.theme === "light" ? "dark" : "light";
    document.documentElement.setAttribute("data-theme", this.theme);
    if (typeof localStorage !== "undefined") {
      localStorage.setItem("aphrody_theme", this.theme);
    }
    this.footer.updateTheme(this.theme);
  }

  private toggleShellMode(): void {
    const current = this.adaptiveShell.getMode();
    let next: ShellMode = "google-classic";
    if (current === "google-classic") next = "fluent-desktop";
    else if (current === "fluent-desktop") next = "material-adaptive";
    else next = "google-classic";

    this.adaptiveShell.setMode(next);
    this.header.updateShellMode(next);
  }

  public async openSearchWindow(query: string): Promise<void> {
    const { ClusterSearchApp } = await import("./suite/cluster-search-app");
    const app = new ClusterSearchApp(
      query,
      (suiteId) => void this.openAppWindow(suiteId),
    );
    this.windowManager.openWindow({
      id: "win-cluster-search",
      title: `Aphrody 3-VPS Search — ${query}`,
      width: 920,
      height: 600,
      content: app.element,
    });
  }

  public async openAppWindow(appId: string): Promise<void> {
    if (appId === "search") {
      await this.openSearchWindow("aphrody");
      return;
    }

    if (appId === "craft" || appId === "photos") {
      const { CraftCloudApp } = await import("./suite/craft-cloud-app");
      const craft = new CraftCloudApp(
        appId === "photos" ? "photocraft" : "photocraft",
      );
      this.windowManager.openWindow({
        id: "win-craft",
        title:
          "Craft Creative Cloud (PhotoCraft, VectorCraft, FigCraft, ShaderCraft, AudioCraft, CodeCraft)",
        width: 960,
        height: 600,
        content: craft.element,
        onClose: () => craft.destroy(),
      });
      return;
    }

    if (
      appId === "docs" ||
      appId === "sheets" ||
      appId === "slides" ||
      appId === "office"
    ) {
      const { FluentOfficeApp } = await import("./suite/fluent-office-app");
      const tab =
        appId === "sheets" ? "grid" : appId === "slides" ? "deck" : "word";
      const office = new FluentOfficeApp(tab);
      this.windowManager.openWindow({
        id: "win-office",
        title:
          "Fluent UI 2 Office 365 — WordCraft (.docx), GridCraft (Formulas) & DeckCraft",
        width: 940,
        height: 600,
        content: office.element,
      });
      return;
    }

    if (appId === "drive") {
      const { SpacedriveApp } = await import("./suite/spacedrive-app");
      const drive = new SpacedriveApp();
      this.windowManager.openWindow({
        id: "win-drive",
        title: "Spacedrive VDFS — Blake3 Content-Addressed Storage (fr1 + fr2)",
        width: 920,
        height: 580,
        content: drive.element,
      });
      return;
    }

    if (
      appId === "mail" ||
      appId === "gmail" ||
      appId === "notes" ||
      appId === "keep" ||
      appId === "calendar" ||
      appId === "contacts" ||
      appId === "translate" ||
      appId === "studio" ||
      appId === "account" ||
      appId === "workspace"
    ) {
      const { M3WorkspaceApp } = await import("./suite/m3-workspace-app");
      const tabMap: Record<
        string,
        | "mail"
        | "notes"
        | "calendar"
        | "contacts"
        | "translate"
        | "studio"
        | "account"
      > = {
        mail: "mail",
        gmail: "mail",
        notes: "notes",
        keep: "notes",
        calendar: "calendar",
        contacts: "contacts",
        translate: "translate",
        studio: "studio",
        account: "account",
        workspace: "mail",
      };
      const ws = new M3WorkspaceApp(tabMap[appId] || "mail");
      this.windowManager.openWindow({
        id: "win-m3-workspace",
        title: "Aphrody Material 3 Workspace & Locally Uncensored Studio",
        width: 940,
        height: 600,
        content: ws.element,
      });
      return;
    }

    if (appId === "storytold") {
      const { StorytoldSuite } = await import("./suite/storytold-suite");
      const suite = new StorytoldSuite();
      this.windowManager.openWindow({
        id: "win-storytold",
        title: "Storytold 3D & WebGPU Shaders",
        width: 960,
        height: 620,
        content: suite.element,
        onClose: () => suite.destroy(),
      });
    } else if (appId === "store") {
      const { StoreApp } = await import("./components/store-app");
      const store = new StoreApp();
      this.windowManager.openWindow({
        id: "win-store",
        title: "Universal Store & Awesome Converter (Flathub + NixOS + Bun)",
        width: 980,
        height: 640,
        content: store.element,
      });
    } else if (appId === "bench") {
      const { BenchmarkApp } = await import("./components/benchmark-app");
      const bench = new BenchmarkApp();
      this.windowManager.openWindow({
        id: "win-bench",
        title: "LTO, AOT & Zero-Copy Stack Benchmark & Picker",
        width: 860,
        height: 580,
        content: bench.element,
      });
    } else if (appId === "shell") {
      this.toggleShellMode();
    }
  }

  private handleInitialRoute(): void {
    if (typeof window === "undefined") return;
    const params = new URLSearchParams(window.location.search);
    const q = params.get("q");
    const suite = params.get("suite");
    if (q) {
      void this.openSearchWindow(q);
    } else if (suite) {
      void this.openAppWindow(suite);
    }
  }

  private buildUI(): void {
    this.windowContainer = document.createElement("div");
    this.windowContainer.className = "macos-window-stage";
    this.windowManager.setContainer(this.windowContainer);

    this.dockElement = document.createElement("div");
    this.dockElement.className = "macos-dock";
    this.windowManager.setDock(this.dockElement);

    this.appsPopover = new AppsPopover((appId) => {
      void this.openAppWindow(appId);
    });

    this.voiceModal = new VoiceModal((transcript) => {
      this.searchBar.setQueryAndSearch(transcript);
    });
    this.lensModal = new LensModal((q) => {
      void this.openSearchWindow(q);
    });

    this.header = new Header({
      onToggleApps: () => {
        this.appsPopover.toggle();
        this.header.setAppsActive(this.appsPopover.getIsOpen());
      },
      onOpenAppWindow: (appId) => void this.openAppWindow(appId),
      onToggleShellMode: () => this.toggleShellMode(),
    });
    this.header.updateShellMode(this.adaptiveShell.getMode());

    const main = document.createElement("main");
    main.className = "google-main";

    this.logo = new Logo();
    this.searchBar = new SearchBar({
      onOpenVoice: () => this.voiceModal.open(),
      onOpenLens: () => this.lensModal.open(),
      onExecuteSearch: (q) => void this.openSearchWindow(q),
      onOpenWorkspace: () => void this.openAppWindow("craft"),
    });
    this.languageBar = new LanguageBar();

    main.appendChild(this.logo.element);
    main.appendChild(this.searchBar.element);
    main.appendChild(this.languageBar.element);

    this.footer = new Footer(
      this.theme,
      () => this.toggleTheme(),
      (appId) => void this.openAppWindow(appId),
    );

    this.rootElement.appendChild(this.header.element);
    this.rootElement.appendChild(this.appsPopover.element);
    this.rootElement.appendChild(main);
    this.rootElement.appendChild(this.footer.element);
    this.rootElement.appendChild(this.voiceModal.element);
    this.rootElement.appendChild(this.lensModal.element);
    this.rootElement.appendChild(this.windowContainer);
    this.rootElement.appendChild(this.dockElement);
  }

  public mount(container: HTMLElement): void {
    container.innerHTML = "";
    container.appendChild(this.rootElement);
  }
}
