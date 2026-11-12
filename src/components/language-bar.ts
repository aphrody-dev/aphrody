// SPDX-License-Identifier: Apache-2.0
/**
 * Sovereign Locale & Translation Switcher
 * Zero external *.google.com/setprefs links — navigates first-party locales and Sovereign M3 Translate.
 */
export class LanguageBar {
  public element: HTMLDivElement;

  constructor() {
    this.element = document.createElement("div");
    this.element.className = "google-lang-bar";
    this.render();
  }

  private render(): void {
    this.element.innerHTML = `
      <span>Aphrody disponible en : </span>
      <a href="/" class="lang-link">Français</a>
      <a href="/en" class="lang-link">English</a>
      <a href="/#suite=m3:translate" class="lang-link">Traduction</a>
    `;
  }
}
