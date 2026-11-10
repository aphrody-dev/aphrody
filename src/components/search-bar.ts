import { getSuggestions, SuggestionItem } from '../data/suggestions';

export interface SearchBarCallbacks {
  onOpenVoice: () => void;
  onOpenLens: () => void;
}

export class SearchBar {
  public element: HTMLDivElement;
  private input!: HTMLInputElement;
  private clearBtn!: HTMLButtonElement;
  private divider!: HTMLDivElement;
  private dropdown!: HTMLDivElement;
  private suggestionsList!: HTMLUListElement;
  private searchBox!: HTMLDivElement;

  private currentQuery = '';
  private suggestions: SuggestionItem[] = [];
  private selectedIndex = -1;
  private isDropdownOpen = false;

  constructor(private callbacks: SearchBarCallbacks) {
    this.element = document.createElement('div');
    this.element.className = 'search-container';
    this.render();
    this.attachEvents();
  }

  private render(): void {
    // 1. Search Box
    this.searchBox = document.createElement('div');
    this.searchBox.className = 'search-box';

    // Left magnifying glass
    const leftIcon = document.createElement('div');
    leftIcon.className = 'search-icon-left';
    leftIcon.setAttribute('role', 'button');
    leftIcon.setAttribute('aria-label', 'Rechercher');
    leftIcon.innerHTML = `
      <svg focusable="false" viewBox="0 0 24 24" class="lens-search-svg">
        <path d="M15.5 14h-.79l-.28-.27A6.471 6.471 0 0 0 16 9.5 6.5 6.5 0 1 0 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z"/>
      </svg>
    `;
    leftIcon.addEventListener('click', () => this.executeSearch(this.input.value));

    // Input field
    this.input = document.createElement('input');
    this.input.type = 'text';
    this.input.className = 'search-input';
    this.input.setAttribute('aria-label', 'Recherche');
    this.input.setAttribute('autocomplete', 'off');
    this.input.setAttribute('spellcheck', 'false');

    // Clear button
    this.clearBtn = document.createElement('button');
    this.clearBtn.type = 'button';
    this.clearBtn.className = 'clear-btn';
    this.clearBtn.setAttribute('aria-label', 'Effacer la recherche');
    this.clearBtn.innerHTML = `
      <svg focusable="false" viewBox="0 0 24 24">
        <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/>
      </svg>
    `;

    // Divider
    this.divider = document.createElement('div');
    this.divider.className = 'search-divider';

    // Right Tools (Mic & Lens)
    const rightTools = document.createElement('div');
    rightTools.className = 'search-tools-right';

    const micBtn = document.createElement('button');
    micBtn.type = 'button';
    micBtn.className = 'tool-btn mic-btn';
    micBtn.setAttribute('aria-label', 'Recherche vocale');
    micBtn.setAttribute('title', 'Recherche vocale');
    micBtn.innerHTML = `
      <svg class="mic-svg" viewBox="0 0 24 24">
        <path fill="#4285f4" d="m12 15c1.66 0 3-1.31 3-2.97v-7.02c0-1.66-1.34-3.01-3-3.01s-3 1.34-3 3.01v7.02c0 1.66 1.34 2.97 3 2.97z"/>
        <path fill="#34a853" d="m11 18.08h2v3.92h-2z"/>
        <path fill="#fbbc05" d="m7.05 10.87-1.42-1.42c-.44.58-.75 1.25-.89 1.98l1.97.39c.07-.36.19-.68.34-.95z"/>
        <path fill="#ea4335" d="m12 17c-2.76 0-5-2.24-5-5h-2c0 3.53 2.61 6.43 6 6.92v2.08h2v-2.08c3.39-.49 6-3.39 6-6.92h-2c0 2.76-2.24 5-5 5z"/>
      </svg>
    `;
    micBtn.addEventListener('click', () => this.callbacks.onOpenVoice());

    const lensBtn = document.createElement('button');
    lensBtn.type = 'button';
    lensBtn.className = 'tool-btn lens-btn';
    lensBtn.setAttribute('aria-label', 'Recherche par image');
    lensBtn.setAttribute('title', 'Recherche par image');
    lensBtn.innerHTML = `
      <svg class="lens-svg" viewBox="0 0 192 192">
        <path fill="#4285F4" d="M129 119a33 33 0 1 1-66 0 33 33 0 0 1 66 0"/>
        <path fill="#EA4335" d="M141 33h-19l-9-19H79l-9 19H51C37.7 33 27 43.7 27 57v78c0 13.3 10.7 24 24 24h90c13.3 0 24-10.7 24-24V57c0-13.3-10.7-24-24-24m-45 107c-24.8 0-45-20.2-45-45s20.2-45 45-45 45 20.2 45 45-20.2 45-45 45m53-73c-4.4 0-8-3.6-8-8s3.6-8 8-8 8 3.6 8 8-3.6 8-8 8"/>
      </svg>
    `;
    lensBtn.addEventListener('click', () => this.callbacks.onOpenLens());

    rightTools.appendChild(micBtn);
    rightTools.appendChild(lensBtn);

    this.searchBox.appendChild(leftIcon);
    this.searchBox.appendChild(this.input);
    this.searchBox.appendChild(this.clearBtn);
    this.searchBox.appendChild(this.divider);
    this.searchBox.appendChild(rightTools);

    // 2. Dropdown
    this.dropdown = document.createElement('div');
    this.dropdown.className = 'search-dropdown';

    const dropDiv = document.createElement('div');
    dropDiv.className = 'dropdown-divider';

    this.suggestionsList = document.createElement('ul');
    this.suggestionsList.className = 'suggestions-list';
    this.suggestionsList.setAttribute('role', 'listbox');

    this.dropdown.appendChild(dropDiv);
    this.dropdown.appendChild(this.suggestionsList);

    // 3. Center Search Buttons Row
    const btnRow = document.createElement('div');
    btnRow.className = 'search-buttons-row';

    const searchBtn = document.createElement('button');
    searchBtn.type = 'button';
    searchBtn.className = 'google-btn';
    searchBtn.textContent = 'Recherche Google';
    searchBtn.addEventListener('click', () => this.executeSearch(this.input.value));

    const luckyBtn = document.createElement('button');
    luckyBtn.type = 'button';
    luckyBtn.className = 'google-btn';
    luckyBtn.textContent = "J'ai de la chance";
    luckyBtn.addEventListener('click', () => this.executeLucky(this.input.value));

    btnRow.appendChild(searchBtn);
    btnRow.appendChild(luckyBtn);

    this.element.appendChild(this.searchBox);
    this.element.appendChild(this.dropdown);
    this.element.appendChild(btnRow);
  }

  private attachEvents(): void {
    this.input.addEventListener('input', () => {
      this.currentQuery = this.input.value;
      this.updateClearBtn();
      this.selectedIndex = -1;
      this.fetchSuggestions(this.currentQuery);
      this.openDropdown();
    });

    this.input.addEventListener('focus', () => {
      this.fetchSuggestions(this.input.value);
      this.openDropdown();
    });

    this.clearBtn.addEventListener('click', () => {
      this.input.value = '';
      this.currentQuery = '';
      this.selectedIndex = -1;
      this.updateClearBtn();
      this.fetchSuggestions('');
      this.input.focus();
    });

    this.input.addEventListener('keydown', (e: KeyboardEvent) => {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        if (this.suggestions.length > 0) {
          this.selectedIndex = (this.selectedIndex + 1) % this.suggestions.length;
          this.highlightItem(this.selectedIndex);
        }
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        if (this.suggestions.length > 0) {
          this.selectedIndex = this.selectedIndex <= 0 ? this.suggestions.length - 1 : this.selectedIndex - 1;
          this.highlightItem(this.selectedIndex);
        }
      } else if (e.key === 'Enter') {
        e.preventDefault();
        if (this.selectedIndex >= 0 && this.suggestions[this.selectedIndex]) {
          this.executeSearch(this.suggestions[this.selectedIndex].text);
        } else {
          this.executeSearch(this.input.value);
        }
      } else if (e.key === 'Escape') {
        this.closeDropdown();
        this.input.blur();
      }
    });

    document.addEventListener('mousedown', (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      if (this.isDropdownOpen && !this.element.contains(target)) {
        this.closeDropdown();
      }
    });
  }

  private updateClearBtn(): void {
    if (this.input.value.length > 0) {
      this.clearBtn.classList.add('visible');
      this.divider.classList.add('visible');
    } else {
      this.clearBtn.classList.remove('visible');
      this.divider.classList.remove('visible');
    }
  }

  private async fetchSuggestions(query: string): Promise<void> {
    try {
      if (typeof window !== 'undefined' && window.location.origin) {
        const res = await fetch(`/api/complete?q=${encodeURIComponent(query)}`);
        if (res.ok) {
          const data = await res.json();
          this.suggestions = data.suggestions || getSuggestions(query);
          this.renderSuggestions();
          return;
        }
      }
    } catch {
      // Fallback to local synchronous suggestions
    }
    this.suggestions = getSuggestions(query);
    this.renderSuggestions();
  }

  private renderSuggestions(): void {
    this.suggestionsList.innerHTML = '';
    this.suggestions.forEach((item, index) => {
      const li = document.createElement('li');
      li.className = 'suggestion-item';
      li.setAttribute('role', 'option');
      if (index === this.selectedIndex) {
        li.classList.add('selected');
      }

      const iconWrap = document.createElement('div');
      iconWrap.className = 'suggestion-icon';
      if (item.isHistory) {
        iconWrap.innerHTML = `
          <svg viewBox="0 0 24 24">
            <path d="M13 3a9 9 0 0 0-9 9H1l3.89 3.89.07.14L9 12H6c0-3.87 3.13-7 7-7s7 3.13 7 7-3.13 7-7 7c-1.93 0-3.68-.79-4.94-2.06l-1.42 1.42A8.954 8.954 0 0 0 13 21a9 9 0 0 0 0-18zm-1 5v5l4.28 2.54.72-1.21-3.5-2.08V8H12z"/>
          </svg>
        `;
      } else {
        iconWrap.innerHTML = `
          <svg viewBox="0 0 24 24">
            <path d="M15.5 14h-.79l-.28-.27A6.471 6.471 0 0 0 16 9.5 6.5 6.5 0 1 0 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z"/>
          </svg>
        `;
      }

      const textSpan = document.createElement('span');
      textSpan.className = 'suggestion-text';
      textSpan.textContent = item.text;

      li.appendChild(iconWrap);
      li.appendChild(textSpan);

      li.addEventListener('mousedown', () => {
        this.executeSearch(item.text);
      });

      li.addEventListener('mouseenter', () => {
        this.selectedIndex = index;
        this.highlightItem(index);
      });

      this.suggestionsList.appendChild(li);
    });
  }

  private highlightItem(index: number): void {
    const items = this.suggestionsList.querySelectorAll('.suggestion-item');
    items.forEach((item, idx) => {
      if (idx === index) {
        item.classList.add('selected');
        if (this.suggestions[idx]) {
          this.input.value = this.suggestions[idx].text;
          this.updateClearBtn();
        }
      } else {
        item.classList.remove('selected');
      }
    });
  }

  private openDropdown(): void {
    if (this.suggestions.length > 0) {
      this.isDropdownOpen = true;
      this.searchBox.classList.add('open');
      this.dropdown.classList.add('open');
    }
  }

  private closeDropdown(): void {
    this.isDropdownOpen = false;
    this.searchBox.classList.remove('open');
    this.dropdown.classList.remove('open');
  }

  public executeSearch(query: string): void {
    const trimmed = (query || this.input.value || '').trim();
    if (trimmed) {
      window.location.href = `https://www.google.com/search?q=${encodeURIComponent(trimmed)}`;
    } else {
      this.input.focus();
    }
  }

  public executeLucky(query: string): void {
    const trimmed = (query || this.input.value || '').trim();
    if (trimmed) {
      window.location.href = `https://www.google.com/search?q=${encodeURIComponent(trimmed)}&btnI=I`;
    } else {
      window.location.href = 'https://doodles.google/';
    }
  }

  public setQueryAndSearch(query: string): void {
    this.input.value = query;
    this.updateClearBtn();
    this.executeSearch(query);
  }
}
