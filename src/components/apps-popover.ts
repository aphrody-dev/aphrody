import { GOOGLE_APPS, getAppIconSvg } from '../data/apps';

export class AppsPopover {
  public element: HTMLDivElement;
  private isOpen = false;

  constructor() {
    this.element = document.createElement('div');
    this.element.className = 'apps-popover';
    this.element.setAttribute('role', 'dialog');
    this.element.setAttribute('aria-label', 'Applications Google');
    this.render();
    this.attachEvents();
  }

  private render(): void {
    const grid = document.createElement('div');
    grid.className = 'apps-grid';

    GOOGLE_APPS.forEach((app) => {
      const a = document.createElement('a');
      a.className = 'app-item';
      a.href = app.url;
      a.target = '_blank';
      a.rel = 'noreferrer';
      a.setAttribute('aria-label', app.name);

      const iconWrap = document.createElement('div');
      iconWrap.className = 'app-icon-wrapper';
      iconWrap.innerHTML = getAppIconSvg(app.iconType);

      const name = document.createElement('span');
      name.className = 'app-name';
      name.textContent = app.name;

      a.appendChild(iconWrap);
      a.appendChild(name);
      grid.appendChild(a);
    });

    this.element.appendChild(grid);
  }

  private attachEvents(): void {
    document.addEventListener('mousedown', (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      if (this.isOpen && !this.element.contains(target) && !target.closest('.apps-button')) {
        this.close();
      }
    });

    document.addEventListener('keydown', (e: KeyboardEvent) => {
      if (this.isOpen && e.key === 'Escape') {
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
    this.element.classList.add('open');
  }

  public close(): void {
    this.isOpen = false;
    this.element.classList.remove('open');
  }

  public getIsOpen(): boolean {
    return this.isOpen;
  }
}
