export class LanguageBar {
  public element: HTMLDivElement;

  constructor() {
    this.element = document.createElement("div");
    this.element.className = "google-lang-bar";
    this.render();
  }

  private render(): void {
    this.element.innerHTML = `
      <span>Google disponible en : </span>
      <a href="https://www.google.com/setprefs?sig=0_aX...&hl=fr" class="lang-link">Français</a>
      <a href="https://www.google.com/setprefs?sig=0_aX...&hl=en" class="lang-link">English</a>
      <a href="https://www.google.com/setprefs?sig=0_aX...&hl=es" class="lang-link">Español</a>
    `;
  }
}
