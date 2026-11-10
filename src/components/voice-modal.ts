export class VoiceModal {
  public element: HTMLDivElement;
  private statusEl!: HTMLHeadingElement;
  private recognition: any = null;
  private isOpen = false;

  constructor(private onSpeechResult: (text: string) => void) {
    this.element = document.createElement("div");
    this.element.className = "voice-overlay";
    this.element.setAttribute("role", "dialog");
    this.element.setAttribute("aria-modal", "true");
    this.element.setAttribute("aria-label", "Recherche vocale");
    this.render();
    this.attachEvents();
  }

  private render(): void {
    const card = document.createElement("div");
    card.className = "voice-card";

    const closeBtn = document.createElement("button");
    closeBtn.type = "button";
    closeBtn.className = "voice-close-btn";
    closeBtn.setAttribute("aria-label", "Fermer");
    closeBtn.innerHTML = `
      <svg viewBox="0 0 24 24">
        <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/>
      </svg>
    `;
    closeBtn.addEventListener("click", () => this.close());

    this.statusEl = document.createElement("h2");
    this.statusEl.className = "voice-status";
    this.statusEl.textContent = "Écoute en cours...";

    const micCircle = document.createElement("div");
    micCircle.className = "voice-mic-circle";
    micCircle.setAttribute("title", "Arrêter l'écoute");
    micCircle.innerHTML = `
      <div class="pulse-ring ring-1"></div>
      <div class="pulse-ring ring-2"></div>
      <svg class="voice-mic-svg" viewBox="0 0 24 24">
        <path fill="#ffffff" d="m12 15c1.66 0 3-1.31 3-2.97v-7.02c0-1.66-1.34-3.01-3-3.01s-3 1.34-3 3.01v7.02c0 1.66 1.34 2.97 3 2.97z"/>
        <path fill="#ffffff" d="m11 18.08h2v3.92h-2z"/>
        <path fill="#ffffff" d="m7.05 10.87-1.42-1.42c-.44.58-.75 1.25-.89 1.98l1.97.39c.07-.36.19-.68.34-.95z"/>
        <path fill="#ffffff" d="m12 17c-2.76 0-5-2.24-5-5h-2c0 3.53 2.61 6.43 6 6.92v2.08h2v-2.08c3.39-.49 6-3.39 6-6.92h-2c0 2.76-2.24 5-5 5z"/>
      </svg>
    `;
    micCircle.addEventListener("click", () => this.close());

    card.appendChild(closeBtn);
    card.appendChild(this.statusEl);
    card.appendChild(micCircle);

    this.element.appendChild(card);
  }

  private attachEvents(): void {
    this.element.addEventListener("mousedown", (e: MouseEvent) => {
      if (e.target === this.element) {
        this.close();
      }
    });

    document.addEventListener("keydown", (e: KeyboardEvent) => {
      if (this.isOpen && e.key === "Escape") {
        this.close();
      }
    });
  }

  public open(): void {
    this.isOpen = true;
    this.element.classList.add("open");
    this.statusEl.textContent = "Écoute en cours...";
    this.startListening();
  }

  public close(): void {
    this.isOpen = false;
    this.element.classList.remove("open");
    this.stopListening();
  }

  private startListening(): void {
    const SpeechRecognition =
      (window as any).SpeechRecognition ||
      (window as any).webkitSpeechRecognition;

    if (SpeechRecognition) {
      try {
        this.recognition = new SpeechRecognition();
        this.recognition.lang = "fr-FR";
        this.recognition.interimResults = false;
        this.recognition.maxAlternatives = 1;

        this.recognition.onresult = (event: any) => {
          const transcript = event.results?.[0]?.[0]?.transcript;
          if (transcript) {
            this.close();
            this.onSpeechResult(transcript);
          }
        };

        this.recognition.onerror = () => {
          this.statusEl.textContent = "Dites quelque chose...";
        };

        this.recognition.start();
      } catch (err) {
        console.warn("SpeechRecognition error:", err);
      }
    } else {
      this.statusEl.textContent = "Reconnaissance vocale non supportée";
    }
  }

  private stopListening(): void {
    if (this.recognition) {
      try {
        this.recognition.abort();
      } catch {}
      this.recognition = null;
    }
  }
}
