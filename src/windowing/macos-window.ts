/**
 * Apple macOS Traffic-Light Windowing System & Window Manager
 * Pixel-accurate macOS Sequoia / Tahoe controls, dragging, snapping & focus stack
 * Zero external npm dependencies - 100% pure Web Standards
 */

export interface WindowOptions {
  id: string;
  title: string;
  width?: number;
  height?: number;
  minWidth?: number;
  minHeight?: number;
  x?: number;
  y?: number;
  iconSvg?: string;
  content?: HTMLElement | string;
  onClose?: () => void;
  onMinimize?: () => void;
  onMaximize?: () => void;
}

export class MacOsWindow {
  public id: string;
  public title: string;
  public element: HTMLDivElement;
  public contentContainer: HTMLDivElement;

  private isFocused = true;
  private isMinimized = false;
  private isMaximized = false;
  private prevRect: {
    x: number;
    y: number;
    width: number;
    height: number;
  } | null = null;

  private x: number;
  private y: number;
  private width: number;
  private height: number;
  private minWidth: number;
  private minHeight: number;

  private titleBar!: HTMLDivElement;
  private trafficLightGroup!: HTMLDivElement;
  private btnClose!: HTMLButtonElement;
  private btnMin!: HTMLButtonElement;
  private btnMax!: HTMLButtonElement;

  private onCloseCb?: () => void;
  private onMinimizeCb?: () => void;
  private onMaximizeCb?: () => void;

  constructor(options: WindowOptions) {
    this.id = options.id;
    this.title = options.title;
    this.width = options.width || 780;
    this.height = options.height || 540;
    this.minWidth = options.minWidth || 360;
    this.minHeight = options.minHeight || 260;
    this.x =
      options.x ??
      Math.max(
        40,
        window?.innerWidth ? (window.innerWidth - this.width) / 2 : 80,
      );
    this.y =
      options.y ??
      Math.max(
        40,
        window?.innerHeight ? (window.innerHeight - this.height) / 2 : 60,
      );

    this.onCloseCb = options.onClose;
    this.onMinimizeCb = options.onMinimize;
    this.onMaximizeCb = options.onMaximize;

    this.element = document.createElement("div");
    this.element.className = "macos-window active";
    this.element.setAttribute("data-window-id", this.id);
    this.contentContainer = document.createElement("div");
    this.contentContainer.className = "macos-window-content";

    this.render();
    if (options.content) {
      this.setContent(options.content);
    }
    this.attachEvents();
    this.updatePosition();
  }

  private render(): void {
    // Titlebar
    this.titleBar = document.createElement("div");
    this.titleBar.className = "macos-titlebar";

    // Traffic Light cluster
    this.trafficLightGroup = document.createElement("div");
    this.trafficLightGroup.className = "traffic-light-group";

    // Close button: #FF5F56 (1px solid #E0443E) + glyph #4D0000
    this.btnClose = document.createElement("button");
    this.btnClose.className = "traffic-light btn-close";
    this.btnClose.setAttribute("aria-label", "Fermer la fenêtre");
    this.btnClose.innerHTML = `
      <svg class="traffic-glyph" viewBox="0 0 12 12" width="8" height="8">
        <path d="M2.5 2.5l7 7M9.5 2.5l-7 7" stroke="#4D0000" stroke-width="1.3" stroke-linecap="round"/>
      </svg>
    `;

    // Minimize button: #FFBD2E (1px solid #DEA123) + glyph #995700
    this.btnMin = document.createElement("button");
    this.btnMin.className = "traffic-light btn-min";
    this.btnMin.setAttribute("aria-label", "Réduire la fenêtre");
    this.btnMin.innerHTML = `
      <svg class="traffic-glyph" viewBox="0 0 12 12" width="8" height="8">
        <path d="M2 6h8" stroke="#995700" stroke-width="1.4" stroke-linecap="round"/>
      </svg>
    `;

    // Maximize/Zoom button: #27C93F (1px solid #1AAB29) + glyph #006500
    this.btnMax = document.createElement("button");
    this.btnMax.className = "traffic-light btn-max";
    this.btnMax.setAttribute("aria-label", "Plein écran ou agrandir");
    this.btnMax.innerHTML = `
      <svg class="traffic-glyph" viewBox="0 0 12 12" width="8" height="8">
        <path d="M3 9l6-6M9 6.5V3H5.5M3 5.5V9h3.5" stroke="#006500" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    `;

    this.trafficLightGroup.appendChild(this.btnClose);
    this.trafficLightGroup.appendChild(this.btnMin);
    this.trafficLightGroup.appendChild(this.btnMax);

    // Title text
    const titleSpan = document.createElement("div");
    titleSpan.className = "macos-window-title";
    titleSpan.textContent = this.title;

    // Window quick actions / split button
    const actionsGroup = document.createElement("div");
    actionsGroup.className = "macos-titlebar-actions";
    actionsGroup.innerHTML = `
      <button class="window-action-btn btn-tile-left" title="Split Left">◧</button>
      <button class="window-action-btn btn-tile-right" title="Split Right">◨</button>
    `;

    this.titleBar.appendChild(this.trafficLightGroup);
    this.titleBar.appendChild(titleSpan);
    this.titleBar.appendChild(actionsGroup);

    this.element.appendChild(this.titleBar);
    this.element.appendChild(this.contentContainer);
  }

  private attachEvents(): void {
    // Traffic light events
    this.btnClose.addEventListener("click", (e) => {
      e.stopPropagation();
      this.close();
    });

    this.btnMin.addEventListener("click", (e) => {
      e.stopPropagation();
      this.minimize();
    });

    this.btnMax.addEventListener("click", (e) => {
      e.stopPropagation();
      this.toggleMaximize();
    });

    // Tile split actions
    const tileLeftBtn = this.titleBar.querySelector(".btn-tile-left");
    tileLeftBtn?.addEventListener("click", (e) => {
      e.stopPropagation();
      this.tileLeft();
    });

    const tileRightBtn = this.titleBar.querySelector(".btn-tile-right");
    tileRightBtn?.addEventListener("click", (e) => {
      e.stopPropagation();
      this.tileRight();
    });

    // Double click titlebar to zoom
    this.titleBar.addEventListener("dblclick", (e) => {
      if (
        (e.target as HTMLElement).closest(".traffic-light-group") ||
        (e.target as HTMLElement).closest(".window-action-btn")
      ) {
        return;
      }
      this.toggleMaximize();
    });

    // Dragging
    let isDragging = false;
    let dragStartX = 0;
    let dragStartY = 0;
    let initX = 0;
    let initY = 0;

    this.titleBar.addEventListener("mousedown", (e: MouseEvent) => {
      if (
        (e.target as HTMLElement).closest(".traffic-light-group") ||
        (e.target as HTMLElement).closest(".window-action-btn")
      ) {
        return;
      }
      if (this.isMaximized) return;

      isDragging = true;
      dragStartX = e.clientX;
      dragStartY = e.clientY;
      initX = this.x;
      initY = this.y;
      document.body.style.userSelect = "none";
      this.focus();
    });

    window.addEventListener("mousemove", (e: MouseEvent) => {
      if (!isDragging) return;
      const dx = e.clientX - dragStartX;
      const dy = e.clientY - dragStartY;
      this.x = Math.max(0, Math.min(window.innerWidth - 100, initX + dx));
      this.y = Math.max(30, Math.min(window.innerHeight - 80, initY + dy));
      this.updatePosition();
    });

    window.addEventListener("mouseup", () => {
      if (isDragging) {
        isDragging = false;
        document.body.style.userSelect = "";
      }
    });

    // Window click focus
    this.element.addEventListener("mousedown", () => {
      this.focus();
    });
  }

  public setContent(content: HTMLElement | string): void {
    if (typeof content === "string") {
      this.contentContainer.innerHTML = content;
    } else {
      this.contentContainer.innerHTML = "";
      this.contentContainer.appendChild(content);
    }
  }

  public updatePosition(): void {
    this.element.style.transform = `translate3d(${this.x}px, ${this.y}px, 0)`;
    this.element.style.width = `${this.width}px`;
    this.element.style.height = `${this.height}px`;
  }

  public focus(): void {
    this.isFocused = true;
    this.element.classList.add("active");
    this.element.classList.remove("unfocused");
    WindowManager.getInstance().bringToFront(this);
  }

  public blur(): void {
    this.isFocused = false;
    this.element.classList.remove("active");
    this.element.classList.add("unfocused");
  }

  public close(): void {
    this.element.classList.add("window-closing");
    setTimeout(() => {
      this.element.remove();
      WindowManager.getInstance().unregister(this);
      this.onCloseCb?.();
    }, 180);
  }

  public minimize(): void {
    this.isMinimized = true;
    this.element.classList.add("window-minimized");
    WindowManager.getInstance().updateDock();
    this.onMinimizeCb?.();
  }

  public restore(): void {
    this.isMinimized = false;
    this.element.classList.remove("window-minimized");
    this.focus();
    WindowManager.getInstance().updateDock();
  }

  public toggleMaximize(): void {
    if (this.isMaximized) {
      if (this.prevRect) {
        this.x = this.prevRect.x;
        this.y = this.prevRect.y;
        this.width = this.prevRect.width;
        this.height = this.prevRect.height;
      }
      this.isMaximized = false;
      this.element.classList.remove("window-maximized");
      this.updatePosition();
    } else {
      this.prevRect = {
        x: this.x,
        y: this.y,
        width: this.width,
        height: this.height,
      };
      this.x = 8;
      this.y = 36;
      this.width = Math.max(
        300,
        (typeof window !== "undefined" ? window.innerWidth : 1200) - 16,
      );
      this.height = Math.max(
        300,
        (typeof window !== "undefined" ? window.innerHeight : 800) - 86,
      );
      this.isMaximized = true;
      this.element.classList.add("window-maximized");
      this.updatePosition();
    }
    this.onMaximizeCb?.();
  }

  public tileLeft(): void {
    this.isMaximized = false;
    this.element.classList.remove("window-maximized");
    this.x = 8;
    this.y = 36;
    this.width = Math.floor(
      ((typeof window !== "undefined" ? window.innerWidth : 1200) - 24) / 2,
    );
    this.height = Math.max(
      300,
      (typeof window !== "undefined" ? window.innerHeight : 800) - 86,
    );
    this.updatePosition();
    this.focus();
  }

  public tileRight(): void {
    this.isMaximized = false;
    this.element.classList.remove("window-maximized");
    const halfWidth = Math.floor(
      ((typeof window !== "undefined" ? window.innerWidth : 1200) - 24) / 2,
    );
    this.x = halfWidth + 16;
    this.y = 36;
    this.width = halfWidth;
    this.height = Math.max(
      300,
      (typeof window !== "undefined" ? window.innerHeight : 800) - 86,
    );
    this.updatePosition();
    this.focus();
  }

  public getIsMinimized(): boolean {
    return this.isMinimized;
  }

  public getIsMaximized(): boolean {
    return this.isMaximized;
  }

  public getIsFocused(): boolean {
    return this.isFocused;
  }
}

export class WindowManager {
  private static instance: WindowManager;
  private windows: MacOsWindow[] = [];
  private zIndexBase = 1000;
  private dockElement: HTMLDivElement | null = null;
  private containerElement: HTMLElement | null = null;

  public static getInstance(): WindowManager {
    if (!WindowManager.instance) {
      WindowManager.instance = new WindowManager();
    }
    return WindowManager.instance;
  }

  public setContainer(container: HTMLElement): void {
    this.containerElement = container;
  }

  public setDock(dock: HTMLDivElement): void {
    this.dockElement = dock;
  }

  public openWindow(options: WindowOptions): MacOsWindow {
    const existing = this.windows.find((w) => w.id === options.id);
    if (existing) {
      if (existing.getIsMinimized()) {
        existing.restore();
      } else {
        existing.focus();
      }
      return existing;
    }

    const win = new MacOsWindow(options);
    this.windows.push(win);

    const target = this.containerElement || document.body;
    target.appendChild(win.element);

    this.bringToFront(win);
    this.updateDock();
    return win;
  }

  public bringToFront(win: MacOsWindow): void {
    this.windows.forEach((w) => {
      if (w !== win) {
        w.blur();
      }
    });
    this.zIndexBase += 2;
    win.element.style.zIndex = String(this.zIndexBase);
  }

  public unregister(win: MacOsWindow): void {
    this.windows = this.windows.filter((w) => w !== win);
    this.updateDock();
  }

  public updateDock(): void {
    if (!this.dockElement) return;
    this.dockElement.innerHTML = "";

    this.windows.forEach((win) => {
      const item = document.createElement("button");
      item.className = `dock-window-item ${win.getIsMinimized() ? "minimized" : "active"}`;
      item.title = win.title;
      item.innerHTML = `
        <span class="dock-window-dot"></span>
        <span class="dock-window-name">${win.title}</span>
      `;
      item.addEventListener("click", () => {
        if (win.getIsMinimized()) {
          win.restore();
        } else if (win.getIsFocused()) {
          win.minimize();
        } else {
          win.focus();
        }
      });
      this.dockElement?.appendChild(item);
    });
  }

  public getWindows(): MacOsWindow[] {
    return [...this.windows];
  }
}
