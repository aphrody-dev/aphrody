// SPDX-License-Identifier: Apache-2.0
/**
 * Aphrody Vision Inspector Modal
 * Analyzes local images via HTML5 Canvas RGBA histogram & dominant color extraction
 * and dispatches visual queries directly to the 3-VPS Sovereign Search Cluster.
 */

export class LensModal {
  public element: HTMLDivElement;
  private urlInput!: HTMLInputElement;
  private searchBtn!: HTMLButtonElement;
  private dropzone!: HTMLDivElement;
  private resultBox!: HTMLDivElement;
  private isOpen = false;
  private onSearchQuery?: (q: string) => void;

  constructor(onSearchQuery?: (q: string) => void) {
    this.onSearchQuery = onSearchQuery;
    this.element = document.createElement("div");
    this.element.className = "lens-overlay";
    this.element.setAttribute("role", "dialog");
    this.element.setAttribute("aria-modal", "true");
    this.element.setAttribute("aria-label", "Aphrody Vision Inspector");
    this.render();
    this.attachEvents();
  }

  private render(): void {
    const card = document.createElement("div");
    card.className = "lens-card";

    const header = document.createElement("div");
    header.className = "lens-header";

    const title = document.createElement("span");
    title.className = "lens-title";
    title.textContent = "Inspection visuelle locale (RGBA & Palette M3)";

    const closeBtn = document.createElement("button");
    closeBtn.type = "button";
    closeBtn.className = "lens-close-btn";
    closeBtn.setAttribute("aria-label", "Fermer");
    closeBtn.innerHTML = `
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/>
      </svg>
    `;
    closeBtn.addEventListener("click", () => this.close());

    header.appendChild(title);
    header.appendChild(closeBtn);

    this.dropzone = document.createElement("div");
    this.dropzone.className = "lens-dropzone";

    const camIcon = document.createElement("div");
    camIcon.className = "lens-camera-icon";
    camIcon.innerHTML = `
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path fill="#0B57D0" d="M16.1 14.9a4.1 4.1 0 1 1-8.2 0 4.1 4.1 0 0 1 8.2 0"/>
        <path fill="#34A853" d="M17.6 4.1h-2.4L14.1 1.8H9.9L8.8 4.1H6.4C4.7 4.1 3.4 5.5 3.4 7.1v9.8c0 1.7 1.3 3 3 3h11.2c1.7 0 3-1.3 3-3V7.1c0-1.6-1.3-3-3-3M12 17.5c-3.1 0-5.6-2.5-5.6-5.6S8.9 6.3 12 6.3s5.6 2.5 5.6 5.6-2.5 5.6-5.6 5.6m6.6-9.1c-.5 0-1-.5-1-1s.5-1 1-1 1 .5 1 1-.5 1-1 1"/>
      </svg>
    `;

    const dropText = document.createElement("p");
    dropText.className = "lens-drop-text";
    dropText.innerHTML = `Glissez une image ici ou <label class="lens-browse-label">analysez un fichier local<input type="file" accept="image/*" style="display:none"></label>`;

    const fileInput = dropText.querySelector("input") as HTMLInputElement;
    fileInput.addEventListener("change", () => {
      const f = fileInput.files?.[0];
      if (f) void this.inspectLocalFile(f);
    });

    this.resultBox = document.createElement("div");
    this.resultBox.className = "suite-muted";
    this.resultBox.style.marginTop = "8px";

    this.dropzone.appendChild(camIcon);
    this.dropzone.appendChild(dropText);
    this.dropzone.appendChild(this.resultBox);

    const divider = document.createElement("div");
    divider.className = "lens-or-divider";
    divider.innerHTML = `<span>OU</span>`;

    const urlRow = document.createElement("div");
    urlRow.className = "lens-url-row";

    this.urlInput = document.createElement("input");
    this.urlInput.type = "text";
    this.urlInput.className = "lens-url-input";
    this.urlInput.placeholder =
      "Entrer une URL ou terme visuel (ex: craft wasm)";

    this.searchBtn = document.createElement("button");
    this.searchBtn.type = "button";
    this.searchBtn.className = "lens-search-btn";
    this.searchBtn.textContent = "Analyser";
    this.searchBtn.disabled = true;

    this.urlInput.addEventListener("input", () => {
      this.searchBtn.disabled = !this.urlInput.value.trim();
    });

    this.urlInput.addEventListener("keydown", (e: KeyboardEvent) => {
      if (e.key === "Enter" && this.urlInput.value.trim()) this.submitUrl();
    });

    this.searchBtn.addEventListener("click", () => this.submitUrl());

    urlRow.appendChild(this.urlInput);
    urlRow.appendChild(this.searchBtn);

    card.appendChild(header);
    card.appendChild(this.dropzone);
    card.appendChild(divider);
    card.appendChild(urlRow);

    this.element.appendChild(card);
  }

  private async inspectLocalFile(file: File): Promise<void> {
    const bmp = await createImageBitmap(file);
    const canvas = document.createElement("canvas");
    canvas.width = 32;
    canvas.height = 32;
    const ctx = canvas.getContext("2d");
    let hex = "#0b57d0";
    if (ctx) {
      ctx.drawImage(bmp, 0, 0, 32, 32);
      const data = ctx.getImageData(0, 0, 32, 32).data;
      let r = 0;
      let g = 0;
      let b = 0;
      const count = data.length / 4;
      for (let i = 0; i < data.length; i += 4) {
        r += data[i]!;
        g += data[i + 1]!;
        b += data[i + 2]!;
      }
      const toHex = (n: number) =>
        Math.round(n / count)
          .toString(16)
          .padStart(2, "0");
      hex = `#${toHex(r)}${toHex(g)}${toHex(b)}`;
    }
    this.resultBox.textContent = `${file.name} (${bmp.width}x${bmp.height}px, ${(file.size / 1024).toFixed(1)} KB) · Dominant Hex: ${hex}`;
  }

  private attachEvents(): void {
    this.dropzone.addEventListener("dragover", (e) => {
      e.preventDefault();
      this.dropzone.classList.add("dragging");
    });

    this.dropzone.addEventListener("dragleave", () => {
      this.dropzone.classList.remove("dragging");
    });

    this.dropzone.addEventListener("drop", (e) => {
      e.preventDefault();
      this.dropzone.classList.remove("dragging");
      const file = e.dataTransfer?.files?.[0];
      if (file) void this.inspectLocalFile(file);
    });

    this.element.addEventListener("mousedown", (e: MouseEvent) => {
      if (e.target === this.element) this.close();
    });

    document.addEventListener("keydown", (e: KeyboardEvent) => {
      if (this.isOpen && e.key === "Escape") this.close();
    });
  }

  private submitUrl(): void {
    const val = this.urlInput.value.trim();
    if (!val) return;
    this.close();
    if (this.onSearchQuery) {
      this.onSearchQuery(val);
    } else {
      window.location.href = `/search?q=${encodeURIComponent(val)}`;
    }
  }

  public open(): void {
    this.isOpen = true;
    this.element.classList.add("open");
    this.urlInput.value = "";
    this.searchBtn.disabled = true;
    setTimeout(() => this.urlInput.focus(), 100);
  }

  public close(): void {
    this.isOpen = false;
    this.element.classList.remove("open");
  }
}
