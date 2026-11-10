export class Header {
  public element: HTMLElement;
  private appsBtn!: HTMLButtonElement;

  constructor(private onToggleApps: () => void) {
    this.element = document.createElement('header');
    this.element.className = 'google-header';
    this.render();
  }

  private render(): void {
    const nav = document.createElement('div');
    nav.className = 'google-header-right';

    const gmailLink = document.createElement('a');
    gmailLink.className = 'header-link';
    gmailLink.href = 'https://mail.google.com/';
    gmailLink.target = '_blank';
    gmailLink.rel = 'noreferrer';
    gmailLink.textContent = 'Gmail';

    const imagesLink = document.createElement('a');
    imagesLink.className = 'header-link';
    imagesLink.href = 'https://www.google.com/imghp?hl=fr';
    imagesLink.target = '_blank';
    imagesLink.rel = 'noreferrer';
    imagesLink.textContent = 'Images';

    this.appsBtn = document.createElement('button');
    this.appsBtn.type = 'button';
    this.appsBtn.className = 'apps-button';
    this.appsBtn.setAttribute('aria-label', 'Applications Google');
    this.appsBtn.setAttribute('title', 'Applications Google');
    this.appsBtn.setAttribute('aria-haspopup', 'dialog');
    this.appsBtn.innerHTML = `
      <svg class="apps-icon" focusable="false" viewBox="0 0 24 24">
        <path d="M6,8c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM12,20c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM6,20c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM6,14c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM12,14c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM16,6c0,1.1 0.9,2 2,2s2,-0.9 2,-2 -0.9,-2 -2,-2 -2,0.9 -2,2zM12,8c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM18,14c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM18,20c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2z"/>
      </svg>
    `;
    this.appsBtn.addEventListener('click', () => {
      this.onToggleApps();
    });

    const signInBtn = document.createElement('a');
    signInBtn.className = 'sign-in-btn';
    signInBtn.href = 'https://accounts.google.com/signin';
    signInBtn.target = '_blank';
    signInBtn.rel = 'noreferrer';
    signInBtn.textContent = 'Connexion';

    nav.appendChild(gmailLink);
    nav.appendChild(imagesLink);
    nav.appendChild(this.appsBtn);
    nav.appendChild(signInBtn);

    this.element.appendChild(nav);
  }

  public setAppsActive(active: boolean): void {
    if (active) {
      this.appsBtn.classList.add('active');
    } else {
      this.appsBtn.classList.remove('active');
    }
  }
}
