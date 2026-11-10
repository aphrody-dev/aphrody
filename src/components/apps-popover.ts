import {
  GOOGLE_APPS,
  APHRODY_SYSTEM_APPS,
  getAppIconSvg,
  GoogleApp,
} from "../data/apps";

export class AppsPopover {
  public element: HTMLDivElement;
  private isOpen = false;
  private onSelectAppCb?: (appId: string) => void;

  constructor(onSelectApp?: (appId: string) => void) {
    this.onSelectAppCb = onSelectApp;
    this.element = document.createElement("div");
    this.element.className = "apps-popover";
    this.element.setAttribute("role", "dialog");
    this.element.setAttribute("aria-label", "Applications Google & Aphrody");
    this.render();
    this.attachEvents();
  }

  private render(): void {
    const grid = document.createElement("div");
    grid.className = "apps-grid";

    // 1. Google Official Apps
    GOOGLE_APPS.forEach((app) => {
      grid.appendChild(this.createAppItem(app, false));
    });

    // 2. Divider
    const divider = document.createElement("div");
    divider.className = "apps-grid-divider";
    divider.innerHTML = "<span>Aphrody Suite & Outils</span>";
    grid.appendChild(divider);

    // 3. Aphrody System Apps
    APHRODY_SYSTEM_APPS.forEach((app) => {
      grid.appendChild(this.createAppItem(app, true));
    });

    this.element.appendChild(grid);
  }

  private createAppItem(app: GoogleApp, isSystem: boolean): HTMLElement {
    const a = document.createElement("a");
    a.className = `app-item ${isSystem ? "system-app-item" : ""}`;
    a.href = app.url;
    if (!isSystem) {
      a.target = "_blank";
      a.rel = "noreferrer";
    }
    a.setAttribute("aria-label", app.name);

    const iconWrap = document.createElement("div");
    iconWrap.className = "app-icon-wrapper";
    iconWrap.innerHTML = getAppIconSvg(app.iconType);

    const name = document.createElement("span");
    name.className = "app-name";
    name.textContent = app.name;

    a.appendChild(iconWrap);
    a.appendChild(name);

    if (isSystem) {
      a.addEventListener("click", (e) => {
        e.preventDefault();
        this.close();
        this.onSelectAppCb?.(app.id);
      });
    }

    return a;
  }

  private attachEvents(): void {
    document.addEventListener("mousedown", (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      if (
        this.isOpen &&
        !this.element.contains(target) &&
        !target.closest(".apps-button")
      ) {
        this.close();
      }
    });

    document.addEventListener("keydown", (e: KeyboardEvent) => {
      if (this.isOpen && e.key === "Escape") {
        this.close();
      }
    });
  }

  public toggle(): void {
    if (this.isOpen) {
      this.close();
    } else {
      this.open();
    }
  }

  public open(): void {
    this.isOpen = true;
    this.element.classList.add("open");
  }

  public close(): void {
    this.isOpen = false;
    this.element.classList.remove("open");
  }

  public getIsOpen(): boolean {
    return this.isOpen;
  }
}
