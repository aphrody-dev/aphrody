import React, { useState, useRef, useEffect } from 'react';

interface FooterProps {
  theme: 'light' | 'dark';
  onToggleTheme: () => void;
}

export const Footer: React.FC<FooterProps> = ({ theme, onToggleTheme }) => {
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const settingsRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (settingsRef.current && !settingsRef.current.contains(e.target as Node)) {
        setIsSettingsOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  return (
    <footer className="google-footer">
      {/* Top row: France */}
      <div className="footer-top">
        <span className="footer-location">France</span>
      </div>

      {/* Bottom row */}
      <div className="footer-bottom">
        <div className="footer-left">
          <a
            href="https://about.google/?utm_source=google-FR&utm_medium=referral&utm_campaign=hp-footer&fg=1"
            className="footer-link"
            target="_blank"
            rel="noreferrer"
          >
            À propos
          </a>
          <a
            href="https://www.google.com/intl/fr_fr/ads/?subid=ww-ww-et-g-awa-a-g_hpafoot1_1!o2&utm_source=google.com&utm_medium=referral&utm_campaign=google_hpafooter&fg=1"
            className="footer-link"
            target="_blank"
            rel="noreferrer"
          >
            Publicité
          </a>
          <a
            href="https://www.google.com/services/?subid=ww-ww-et-g-awa-a-g_hpbfoot1_1!o2&utm_source=google.com&utm_medium=referral&utm_campaign=google_hpbfooter&fg=1"
            className="footer-link"
            target="_blank"
            rel="noreferrer"
          >
            Entreprise
          </a>
          <a
            href="https://google.com/search/howsearchworks/?fg=1"
            className="footer-link"
            target="_blank"
            rel="noreferrer"
          >
            Comment fonctionne la recherche Google ?
          </a>
        </div>

        {/* Climate link */}
        <div className="footer-center">
          <a
            href="https://sustainability.google/intl/fr_fr/?utm_source=googlehpfooter&utm_medium=housepromos&utm_campaign=bottom-footer"
            className="footer-link climate-link"
            target="_blank"
            rel="noreferrer"
          >
            <svg className="leaf-icon" viewBox="0 0 24 24">
              <path
                fill="#34a853"
                d="M17 8C8 10 5.9 16.17 3.82 21.34l1.89.66.95-2.3c.48.17.98.3 1.34.3C19 20 22 3 22 3c-1 2-8 2.25-13 3.25S2 11.5 2 13.5s1.75 3.75 1.75 3.75C7 8 17 8 17 8z"
              />
            </svg>
            <span>Notre troisième décennie d'action pour le climat</span>
          </a>
        </div>

        <div className="footer-right" ref={settingsRef}>
          <a
            href="https://policies.google.com/privacy?hl=fr&fg=1"
            className="footer-link"
            target="_blank"
            rel="noreferrer"
          >
            Confidentialité
          </a>
          <a
            href="https://policies.google.com/terms?hl=fr&fg=1"
            className="footer-link"
            target="_blank"
            rel="noreferrer"
          >
            Conditions
          </a>

          {/* Settings button & popover */}
          <div className="settings-wrapper">
            <button
              type="button"
              className={`footer-link settings-btn ${isSettingsOpen ? 'active' : ''}`}
              onClick={() => setIsSettingsOpen(!isSettingsOpen)}
              aria-label="Paramètres"
            >
              Paramètres
            </button>

            {isSettingsOpen && (
              <div className="settings-menu" role="menu">
                <a
                  href="https://www.google.com/preferences?hl=fr&fg=1"
                  className="settings-item"
                  role="menuitem"
                  target="_blank"
                  rel="noreferrer"
                >
                  Paramètres de recherche
                </a>
                <a
                  href="https://www.google.com/advanced_search?hl=fr&fg=1"
                  className="settings-item"
                  role="menuitem"
                  target="_blank"
                  rel="noreferrer"
                >
                  Recherche avancée
                </a>
                <a
                  href="https://www.google.com/history/privacyadvisor/search/unauth?utm_source=googlemenu&fg=1"
                  className="settings-item"
                  role="menuitem"
                  target="_blank"
                  rel="noreferrer"
                >
                  Vos données dans la recherche Google
                </a>
                <a
                  href="https://www.google.com/history/optout?hl=fr&fg=1"
                  className="settings-item"
                  role="menuitem"
                  target="_blank"
                  rel="noreferrer"
                >
                  Historique des recherches
                </a>
                <a
                  href="https://support.google.com/websearch/?p=ws_results_help&hl=fr&fg=1"
                  className="settings-item"
                  role="menuitem"
                  target="_blank"
                  rel="noreferrer"
                >
                  Aide sur la recherche
                </a>
                <div className="settings-menu-divider" />
                <button
                  type="button"
                  className="settings-item theme-toggle-item"
                  role="menuitem"
                  onClick={() => {
                    onToggleTheme();
                    setIsSettingsOpen(false);
                  }}
                >
                  <span>Thème sombre : </span>
                  <span className="theme-status">{theme === 'dark' ? 'Activé' : 'Désactivé'}</span>
                </button>
              </div>
            )}
          </div>
        </div>
      </div>

      <style>{`
        .google-footer {
          margin-top: auto;
          background-color: var(--footer-bg);
          color: var(--footer-text);
          font-size: 14px;
        }

        .footer-top {
          padding: 15px 30px;
          border-bottom: 1px solid var(--footer-border);
        }

        .footer-location {
          font-size: 15px;
          color: var(--footer-text);
        }

        .footer-bottom {
          display: flex;
          flex-wrap: wrap;
          justify-content: space-between;
          align-items: center;
          padding: 0 20px;
          min-height: 48px;
        }

        .footer-left,
        .footer-right {
          display: flex;
          flex-wrap: wrap;
          align-items: center;
        }

        .footer-center {
          display: flex;
          align-items: center;
          justify-content: center;
        }

        .footer-link {
          color: var(--footer-text);
          padding: 12px 15px;
          white-space: nowrap;
          text-decoration: none;
          font-size: 14px;
          display: inline-flex;
          align-items: center;
          background: transparent;
          border: none;
          cursor: pointer;
        }

        .footer-link:hover {
          text-decoration: underline;
        }

        .climate-link {
          gap: 8px;
        }

        .leaf-icon {
          width: 16px;
          height: 16px;
        }

        .settings-wrapper {
          position: relative;
        }

        .settings-btn {
          cursor: pointer;
        }

        .settings-menu {
          position: absolute;
          bottom: calc(100% + 8px);
          right: 0;
          background-color: var(--popup-bg);
          border: 1px solid var(--popup-border);
          border-radius: 8px;
          box-shadow: var(--popup-shadow);
          min-width: 260px;
          padding: 8px 0;
          z-index: 100;
        }

        .settings-item {
          display: flex;
          align-items: center;
          justify-content: space-between;
          width: 100%;
          padding: 8px 20px;
          font-size: 13px;
          color: var(--text-color);
          text-decoration: none;
          background: transparent;
          border: none;
          cursor: pointer;
          text-align: left;
          box-sizing: border-box;
        }

        .settings-item:hover {
          background-color: var(--dropdown-hover);
          text-decoration: none;
        }

        .settings-menu-divider {
          height: 1px;
          background-color: var(--divider);
          margin: 6px 0;
        }

        .theme-toggle-item {
          font-weight: 500;
        }

        .theme-status {
          color: var(--blue-primary);
          font-weight: 600;
        }

        @media (max-width: 850px) {
          .footer-bottom {
            flex-direction: column;
            gap: 8px;
            padding: 12px;
          }
          .footer-left,
          .footer-center,
          .footer-right {
            justify-content: center;
          }
        }
      `}</style>
    </footer>
  );
};
