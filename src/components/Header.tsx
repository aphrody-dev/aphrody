import React from 'react';

interface HeaderProps {
  onToggleApps: () => void;
  isAppsOpen: boolean;
}

export const Header: React.FC<HeaderProps> = ({ onToggleApps, isAppsOpen }) => {
  return (
    <header className="google-header">
      <div className="google-header-right">
        <a
          href="https://mail.google.com/"
          className="header-link"
          target="_blank"
          rel="noreferrer"
        >
          Gmail
        </a>
        <a
          href="https://www.google.com/imghp?hl=fr"
          className="header-link"
          target="_blank"
          rel="noreferrer"
        >
          Images
        </a>

        {/* 9-dot launcher */}
        <button
          type="button"
          className={`apps-button ${isAppsOpen ? 'active' : ''}`}
          onClick={onToggleApps}
          aria-label="Applications Google"
          title="Applications Google"
        >
          <svg className="apps-icon" focusable="false" viewBox="0 0 24 24">
            <path d="M6,8c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM12,20c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM6,20c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM6,14c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM12,14c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM16,6c0,1.1 0.9,2 2,2s2,-0.9 2,-2 -0.9,-2 -2,-2 -2,0.9 -2,2zM12,8c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM18,14c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2zM18,20c1.1,0 2,-0.9 2,-2s-0.9,-2 -2,-2 -2,0.9 -2,2 0.9,2 2,2z" />
          </svg>
        </button>

        {/* Connexion Button */}
        <a
          href="https://accounts.google.com/signin"
          className="sign-in-btn"
          target="_blank"
          rel="noreferrer"
        >
          Connexion
        </a>
      </div>

      <style>{`
        .google-header {
          display: flex;
          justify-content: flex-end;
          align-items: center;
          height: 60px;
          padding: 6px 16px;
          position: relative;
          z-index: 100;
        }

        .google-header-right {
          display: flex;
          align-items: center;
          gap: 12px;
        }

        .header-link {
          font-family: Arial, Roboto, sans-serif;
          font-size: 13px;
          color: var(--header-link);
          text-decoration: none;
          padding: 8px;
          line-height: 24px;
        }

        .header-link:hover {
          text-decoration: underline;
        }

        .apps-button {
          width: 40px;
          height: 40px;
          border-radius: 50%;
          border: none;
          background: transparent;
          display: flex;
          align-items: center;
          justify-content: center;
          cursor: pointer;
          color: var(--icon-color);
          transition: background-color 0.15s ease;
        }

        .apps-button:hover,
        .apps-button.active {
          background-color: var(--icon-hover-bg);
        }

        .apps-icon {
          width: 24px;
          height: 24px;
          fill: currentColor;
        }

        .sign-in-btn {
          font-family: 'Google Sans', Roboto, Arial, sans-serif;
          font-size: 14px;
          font-weight: 500;
          letter-spacing: 0.25px;
          color: #ffffff !important;
          background-color: var(--blue-primary);
          padding: 0 24px;
          height: 40px;
          line-height: 40px;
          border-radius: 20px;
          text-decoration: none !important;
          display: inline-block;
          border: 1px solid transparent;
          cursor: pointer;
          transition: background-color 0.2s, box-shadow 0.2s;
        }

        .sign-in-btn:hover {
          background-color: var(--blue-hover);
          box-shadow: 0 1px 2px rgba(60,64,67,0.3), 0 1px 3px 1px rgba(60,64,67,0.15);
        }
      `}</style>
    </header>
  );
};
