// SPDX-License-Identifier: Apache-2.0
/**
 * Sovereign Aphrody Brand Lockup & Interactive Companion Toggle
 * Zero third-party logo impersonation — authentic Aphrody vector geometry & <aphrody-pet> integration.
 */

export class Logo {
  public element: HTMLDivElement;
  private showCompanion = false;

  constructor() {
    this.element = document.createElement("div");
    this.element.className = "google-logo-wrapper";
    this.element.setAttribute(
      "title",
      "Aphrody Sovereign Suite — Cliquez pour afficher le compagnon Aphrody Pet",
    );
    this.render();
    this.element.addEventListener("click", () => this.toggle());
  }

  private render(): void {
    const petHtml = this.showCompanion
      ? `<aphrody-pet src="/assets/pets/sprites/spritesheet.webp" size="72" label="Aphrody Companion"></aphrody-pet>`
      : "";

    this.element.innerHTML = `
      <div class="aphrody-brand-hero" role="img" aria-label="Aphrody">
        <svg class="aphrody-svg-mark" width="56" height="56" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
          <rect x="1.5" y="1.5" width="21" height="21" rx="6" fill="#0B57D0"/>
          <path d="M12 5.2L18.4 18H15.2L13.9 15.1H10.1L8.8 18H5.6L12 5.2ZM11.1 12.7H12.9L12 10.5L11.1 12.7Z" fill="#FFFFFF"/>
          <circle cx="18" cy="6" r="2" fill="#34A853"/>
        </svg>
        <span class="aphrody-brand-wordmark">Aphrody</span>
        ${petHtml}
      </div>
    `;

    if (
      this.showCompanion &&
      typeof customElements !== "undefined" &&
      !customElements.get("aphrody-pet")
    ) {
      void import("../../../../packages/google-design/src/pets/element.ts").then(
        (m) => m.defineAphrodyPet(),
      );
    }
  }

  public toggle(): void {
    this.showCompanion = !this.showCompanion;
    this.render();
  }
}
