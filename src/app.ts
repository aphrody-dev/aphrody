import { Header } from "./components/header";
import { AppsPopover } from "./components/apps-popover";
import { Logo } from "./components/logo";
import { SearchBar } from "./components/search-bar";
import { LanguageBar } from "./components/language-bar";
import { Footer } from "./components/footer";
import { VoiceModal } from "./components/voice-modal";
import { LensModal } from "./components/lens-modal";
import { AdaptiveShell, ShellMode } from "./design/adaptive-shell";
import { WindowManager } from "./windowing/macos-window";
import { StorytoldSuite } from "./suite/storytold-suite";
import { StoreApp } from "./components/store-app";
import { BenchmarkApp } from "./components/benchmark-app";

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

  public openAppWindow(appId: string): void {
    if (appId === "storytold") {
      const suite = new StorytoldSuite();
      this.windowManager.openWindow({
        id: "win-storytold",
        title: "Storytold Creative Suite (WASM + WebGPU)",
        width: 960,
        height: 620,
        content: suite.element,
        onClose: () => suite.destroy(),
      });
    } else if (appId === "store") {
      const store = new StoreApp();
      this.windowManager.openWindow({
        id: "win-store",
        title:
          "Universal Store & Awesome Converter (Play Store + Flathub + NixOS)",
        width: 980,
        height: 640,
        content: store.element,
      });
    } else if (appId === "bench") {
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

  private buildUI(): void {
    // 1. Window Container & Dock for macOS Window Manager
    this.windowContainer = document.createElement("div");
    this.windowContainer.className = "macos-window-stage";
    this.windowManager.setContainer(this.windowContainer);

    this.dockElement = document.createElement("div");
    this.dockElement.className = "macos-dock";
    this.windowManager.setDock(this.dockElement);

    // 2. Modals
    this.appsPopover = new AppsPopover((appId) => {
      this.openAppWindow(appId);
    });

    this.voiceModal = new VoiceModal((transcript) => {
      this.searchBar.setQueryAndSearch(transcript);
    });
    this.lensModal = new LensModal();

    // 3. Header
    this.header = new Header({
      onToggleApps: () => {
        this.appsPopover.toggle();
        this.header.setAppsActive(this.appsPopover.getIsOpen());
      },
      onOpenAppWindow: (appId) => this.openAppWindow(appId),
      onToggleShellMode: () => this.toggleShellMode(),
    });
    this.header.updateShellMode(this.adaptiveShell.getMode());

    // 4. Main section (Pixel-perfect Google.com replica)
    const main = document.createElement("main");
    main.className = "google-main";

    this.logo = new Logo();
    this.searchBar = new SearchBar({
      onOpenVoice: () => this.voiceModal.open(),
      onOpenLens: () => this.lensModal.open(),
    });
    this.languageBar = new LanguageBar();

    main.appendChild(this.logo.element);
    main.appendChild(this.searchBar.element);
    main.appendChild(this.languageBar.element);

    // 5. Footer
    this.footer = new Footer(this.theme, () => this.toggleTheme());

    // Assemble root
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
