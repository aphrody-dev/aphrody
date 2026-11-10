import React from 'react';

export const LanguageBar: React.FC = () => {
  return (
    <div className="google-lang-bar">
      <span>Google disponible en : </span>
      <a href="https://www.google.com/setprefs?sig=0_aX...&hl=fr" className="lang-link">
        Français
      </a>
      <a href="https://www.google.com/setprefs?sig=0_aX...&hl=en" className="lang-link">
        English
      </a>
      <a href="https://www.google.com/setprefs?sig=0_aX...&hl=es" className="lang-link">
        Español
      </a>

      <style>{`
        .google-lang-bar {
          display: flex;
          justify-content: center;
          align-items: center;
          gap: 8px;
          margin-top: 24px;
          font-size: 13px;
          color: var(--text-secondary);
        }

        .lang-link {
          color: #1a0dab;
          text-decoration: none;
        }

        [data-theme="dark"] .lang-link {
          color: #8ab4f8;
        }

        .lang-link:hover {
          text-decoration: underline;
        }
      `}</style>
    </div>
  );
};
