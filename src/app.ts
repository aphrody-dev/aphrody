import { Header } from './components/header';
import { AppsPopover } from './components/apps-popover';
import { Logo } from './components/logo';
import { SearchBar } from './components/search-bar';
import { LanguageBar } from './components/language-bar';
import { Footer } from './components/footer';
import { VoiceModal } from './components/voice-modal';
import { LensModal } from './components/lens-modal';

export class App {
  public rootElement: HTMLDivElement;
  private theme: 'light' | 'dark' = 'light';

  private header!: Header;
  private appsPopover!: AppsPopover;
  private logo!: Logo;
  private searchBar!: SearchBar;
  private languageBar!: LanguageBar;
  private footer!: Footer;
  private voiceModal!: VoiceModal;
  private lensModal!: LensModal;

  constructor() {
    this.rootElement = document.createElement('div');
    this.rootElement.className = 'google-page-wrapper';
    this.initTheme();
    this.buildUI();
  }

  private initTheme(): void {
    if (typeof window !== 'undefined') {
      const saved = localStorage.getItem('aphrody_theme');
      if (saved === 'dark' || saved === 'light') {
        this.theme = saved;
      } else if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
        this.theme = 'dark';
      }
      document.documentElement.setAttribute('data-theme', this.theme);
    }
  }

  private toggleTheme(): void {
    this.theme = this.theme === 'light' ? 'dark' : 'light';
    document.documentElement.setAttribute('data-theme', this.theme);
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('aphrody_theme', this.theme);
    }
    this.footer.updateTheme(this.theme);
  }

  private buildUI(): void {
    // 1. Modals
    this.appsPopover = new AppsPopover();
    this.voiceModal = new VoiceModal((transcript) => {
      this.searchBar.setQueryAndSearch(transcript);
    });
    this.lensModal = new LensModal();

    // 2. Header
    this.header = new Header(() => {
      this.appsPopover.toggle();
      this.header.setAppsActive(this.appsPopover.getIsOpen());
    });

    // 3. Main section
    const main = document.createElement('main');
    main.className = 'google-main';

    this.logo = new Logo();
    this.searchBar = new SearchBar({
      onOpenVoice: () => this.voiceModal.open(),
      onOpenLens: () => this.lensModal.open()
    });
    this.languageBar = new LanguageBar();

    main.appendChild(this.logo.element);
    main.appendChild(this.searchBar.element);
    main.appendChild(this.languageBar.element);

    // 4. Footer
    this.footer = new Footer(this.theme, () => this.toggleTheme());

    // Assemble root
    this.rootElement.appendChild(this.header.element);
    this.rootElement.appendChild(this.appsPopover.element);
    this.rootElement.appendChild(main);
    this.rootElement.appendChild(this.footer.element);
    this.rootElement.appendChild(this.voiceModal.element);
    this.rootElement.appendChild(this.lensModal.element);
  }

  public mount(container: HTMLElement): void {
    container.innerHTML = '';
    container.appendChild(this.rootElement);
  }
}
