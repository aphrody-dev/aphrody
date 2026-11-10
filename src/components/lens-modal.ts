export class LensModal {
  public element: HTMLDivElement;
  private urlInput!: HTMLInputElement;
  private searchBtn!: HTMLButtonElement;
  private dropzone!: HTMLDivElement;
  private isOpen = false;

  constructor() {
    this.element = document.createElement('div');
    this.element.className = 'lens-overlay';
    this.element.setAttribute('role', 'dialog');
    this.element.setAttribute('aria-modal', 'true');
    this.element.setAttribute('aria-label', 'Google Lens');
    this.render();
    this.attachEvents();
  }

  private render(): void {
    const card = document.createElement('div');
    card.className = 'lens-card';

    // Header
    const header = document.createElement('div');
    header.className = 'lens-header';

    const title = document.createElement('span');
    title.className = 'lens-title';
    title.textContent = "Rechercher n'importe quelle image avec Google Lens";

    const closeBtn = document.createElement('button');
    closeBtn.type = 'button';
    closeBtn.className = 'lens-close-btn';
    closeBtn.setAttribute('aria-label', 'Fermer');
    closeBtn.innerHTML = `
      <svg viewBox="0 0 24 24">
        <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/>
      </svg>
    `;
    closeBtn.addEventListener('click', () => this.close());

    header.appendChild(title);
    header.appendChild(closeBtn);

    // Dropzone
    this.dropzone = document.createElement('div');
    this.dropzone.className = 'lens-dropzone';

    const camIcon = document.createElement('div');
    camIcon.className = 'lens-camera-icon';
    camIcon.innerHTML = `
      <svg viewBox="0 0 192 192">
        <path fill="#4285F4" d="M129 119a33 33 0 1 1-66 0 33 33 0 0 1 66 0"/>
        <path fill="#EA4335" d="M141 33h-19l-9-19H79l-9 19H51C37.7 33 27 43.7 27 57v78c0 13.3 10.7 24 24 24h90c13.3 0 24-10.7 24-24V57c0-13.3-10.7-24-24-24m-45 107c-24.8 0-45-20.2-45-45s20.2-45 45-45 45 20.2 45 45-20.2 45-45 45m53-73c-4.4 0-8-3.6-8-8s3.6-8 8-8 8 3.6 8 8-3.6 8-8 8"/>
      </svg>
    `;

    const dropText = document.createElement('p');
    dropText.className = 'lens-drop-text';
    dropText.innerHTML = `Glissez une image ici ou <label class="lens-browse-label">importez un fichier<input type="file" accept="image/*" style="display:none"></label>`;

    const fileInput = dropText.querySelector('input') as HTMLInputElement;
    fileInput.addEventListener('change', () => {
      if (fileInput.files && fileInput.files.length > 0) {
        window.location.href = 'https://lens.google.com/';
      }
    });

    this.dropzone.appendChild(camIcon);
    this.dropzone.appendChild(dropText);

    // OR divider
    const divider = document.createElement('div');
    divider.className = 'lens-or-divider';
    divider.innerHTML = `<span>OU</span>`;

    // URL row
    const urlRow = document.createElement('div');
    urlRow.className = 'lens-url-row';

    this.urlInput = document.createElement('input');
    this.urlInput.type = 'text';
    this.urlInput.className = 'lens-url-input';
    this.urlInput.placeholder = "Coller le lien de l'image";

    this.searchBtn = document.createElement('button');
    this.searchBtn.type = 'button';
    this.searchBtn.className = 'lens-search-btn';
    this.searchBtn.textContent = 'Rechercher';
    this.searchBtn.disabled = true;

    this.urlInput.addEventListener('input', () => {
      this.searchBtn.disabled = !this.urlInput.value.trim();
    });

    this.urlInput.addEventListener('keydown', (e: KeyboardEvent) => {
      if (e.key === 'Enter' && this.urlInput.value.trim()) {
        this.submitUrl();
      }
    });

    this.searchBtn.addEventListener('click', () => {
      this.submitUrl();
    });

    urlRow.appendChild(this.urlInput);
    urlRow.appendChild(this.searchBtn);

    card.appendChild(header);
    card.appendChild(this.dropzone);
    card.appendChild(divider);
    card.appendChild(urlRow);

    this.element.appendChild(card);
  }

  private attachEvents(): void {
    this.dropzone.addEventListener('dragover', (e) => {
      e.preventDefault();
      this.dropzone.classList.add('dragging');
    });

    this.dropzone.addEventListener('dragleave', () => {
      this.dropzone.classList.remove('dragging');
    });

    this.dropzone.addEventListener('drop', (e) => {
      e.preventDefault();
      this.dropzone.classList.remove('dragging');
      window.location.href = 'https://lens.google.com/';
    });

    this.element.addEventListener('mousedown', (e: MouseEvent) => {
      if (e.target === this.element) {
        this.close();
      }
    });

    document.addEventListener('keydown', (e: KeyboardEvent) => {
      if (this.isOpen && e.key === 'Escape') {
        this.close();
      }
    });
  }

  private submitUrl(): void {
    const val = this.urlInput.value.trim();
    if (val) {
      window.location.href = `https://lens.google.com/uploadbyurl?url=${encodeURIComponent(val)}`;
    }
  }

  public open(): void {
    this.isOpen = true;
    this.element.classList.add('open');
    this.urlInput.value = '';
    this.searchBtn.disabled = true;
    setTimeout(() => this.urlInput.focus(), 100);
  }

  public close(): void {
    this.isOpen = false;
    this.element.classList.remove('open');
  }
}
