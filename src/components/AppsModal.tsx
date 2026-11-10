import React, { useRef, useEffect } from 'react';
import { GOOGLE_APPS } from '../data/apps';

interface AppsModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const AppsModal: React.FC<AppsModalProps> = ({ isOpen, onClose }) => {
  const modalRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      // If click was on apps button, the Header handles toggling
      const target = e.target as HTMLElement;
      if (target.closest('.apps-button')) return;
      if (modalRef.current && !modalRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside);
    }
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  return (
    <div className="apps-popover" ref={modalRef} role="dialog" aria-label="Applications Google">
      <div className="apps-grid">
        {GOOGLE_APPS.map((app) => (
          <a
            key={app.id}
            href={app.url}
            className="app-item"
            target="_blank"
            rel="noreferrer"
          >
            <div className="app-icon-wrapper">
              {renderAppIcon(app.iconType)}
            </div>
            <span className="app-name">{app.name}</span>
          </a>
        ))}
      </div>

      <style>{`
        .apps-popover {
          position: absolute;
          top: 65px;
          right: 20px;
          width: 328px;
          max-height: 460px;
          background-color: var(--popup-bg);
          border: 1px solid var(--popup-border);
          border-radius: 28px;
          box-shadow: var(--popup-shadow);
          overflow-y: auto;
          padding: 16px;
          z-index: 1000;
        }

        .apps-grid {
          display: grid;
          grid-template-columns: repeat(3, 1fr);
          gap: 12px 8px;
        }

        .app-item {
          display: flex;
          flex-direction: column;
          align-items: center;
          padding: 10px 4px;
          border-radius: 16px;
          text-decoration: none;
          color: var(--text-color);
          transition: background-color 0.15s;
        }

        .app-item:hover {
          background-color: var(--icon-hover-bg);
          text-decoration: none;
        }

        .app-icon-wrapper {
          width: 44px;
          height: 44px;
          display: flex;
          align-items: center;
          justify-content: center;
          margin-bottom: 6px;
        }

        .app-name {
          font-size: 12px;
          font-family: Roboto, Arial, sans-serif;
          text-align: center;
          line-height: 1.2;
        }

        /* SVG icon styles */
        .app-svg {
          width: 38px;
          height: 38px;
        }
      `}</style>
    </div>
  );
};

function renderAppIcon(type: string) {
  switch (type) {
    case 'account':
      return (
        <svg className="app-svg" viewBox="0 0 40 40">
          <circle cx="20" cy="20" r="18" fill="#1a73e8" />
          <circle cx="20" cy="15" r="6" fill="#fff" />
          <path d="M10 32c0-5.5 4.5-10 10-10s10 4.5 10 10" fill="#fff" />
        </svg>
      );
    case 'search':
      return (
        <svg className="app-svg" viewBox="0 0 40 40">
          <circle cx="20" cy="20" r="18" fill="#f8f9fa" stroke="#dfe1e5" />
          <path d="M18 13a6 6 0 1 0 4.24 10.24l4.5 4.5 1.41-1.41-4.5-4.5A6 6 0 0 0 18 13z" fill="#4285f4" />
        </svg>
      );
    case 'maps':
      return (
        <svg className="app-svg" viewBox="0 0 40 40">
          <path d="M20 6c-5.52 0-10 4.48-10 10 0 7.5 10 18 10 18s10-10.5 10-18c0-5.52-4.48-10-10-10z" fill="#ea4335" />
          <circle cx="20" cy="16" r="4" fill="#fff" />
        </svg>
      );
    case 'youtube':
      return (
        <svg className="app-svg" viewBox="0 0 40 40">
          <rect x="6" y="10" width="28" height="20" rx="6" fill="#ff0000" />
          <polygon points="17,15 26,20 17,25" fill="#fff" />
        </svg>
      );
    case 'news':
      return (
        <svg className="app-svg" viewBox="0 0 40 40">
          <rect x="8" y="10" width="24" height="20" rx="3" fill="#1a73e8" />
          <rect x="12" y="14" width="8" height="6" fill="#fff" />
          <line x1="22" y1="15" x2="28" y2="15" stroke="#fff" strokeWidth="2" />
          <line x1="22" y1="19" x2="28" y2="19" stroke="#fff" strokeWidth="2" />
          <line x1="12" y1="23" x2="28" y2="23" stroke="#fff" strokeWidth="2" />
          <line x1="12" y1="26" x2="24" y2="26" stroke="#fff" strokeWidth="2" />
        </svg>
      );
    case 'gmail':
      return (
        <svg className="app-svg" viewBox="0 0 40 40">
          <rect x="8" y="11" width="24" height="18" rx="2" fill="#fff" stroke="#dadce0" />
          <path d="M8 11l12 9 12-9" fill="none" stroke="#ea4335" strokeWidth="3" />
          <path d="M8 11v18l8-9" fill="#4285f4" />
          <path d="M32 11v18l-8-9" fill="#34a853" />
        </svg>
      );
    case 'meet':
      return (
        <svg className="app-svg" viewBox="0 0 40 40">
          <rect x="8" y="12" width="16" height="16" rx="3" fill="#00ac47" />
          <polygon points="26,16 32,12 32,28 26,24" fill="#00832d" />
        </svg>
      );
    case 'chat':
      return (
        <svg className="app-svg" viewBox="0 0 40 40">
          <path d="M8 12c0-2.2 1.8-4 4-4h16c2.2 0 4 1.8 4 4v12c0 2.2-1.8 4-4 4H14l-6 5V12z" fill="#00ac47" />
          <circle cx="15" cy="18" r="2" fill="#fff" />
          <circle cx="20" cy="18" r="2" fill="#fff" />
          <circle cx="25" cy="18" r="2" fill="#fff" />
        </svg>
      );
    case 'drive':
      return (
        <svg className="app-svg" viewBox="0 0 40 40">
          <polygon points="15,8 25,8 33,22 23,22" fill="#ffba00" />
          <polygon points="7,22 15,8 20,16 12,30" fill="#0066da" />
          <polygon points="12,30 28,30 33,22 17,22" fill="#00ac47" />
        </svg>
      );
    case 'calendar':
      return (
        <svg className="app-svg" viewBox="0 0 40 40">
          <rect x="8" y="9" width="24" height="22" rx="3" fill="#fff" stroke="#1a73e8" strokeWidth="2" />
          <rect x="8" y="9" width="24" height="7" fill="#1a73e8" />
          <text x="20" y="26" fontSize="11" fontWeight="bold" fill="#1a73e8" textAnchor="middle">31</text>
        </svg>
      );
    case 'contacts':
      return (
        <svg className="app-svg" viewBox="0 0 40 40">
          <rect x="9" y="8" width="22" height="24" rx="2" fill="#1a73e8" />
          <circle cx="20" cy="16" r="4" fill="#fff" />
          <path d="M14 26c0-3.3 2.7-6 6-6s6 2.7 6 6" fill="#fff" />
        </svg>
      );
    case 'translate':
      return (
        <svg className="app-svg" viewBox="0 0 40 40">
          <polygon points="8,10 22,10 18,24 8,24" fill="#4285f4" />
          <polygon points="18,16 32,16 28,30 18,30" fill="#ea4335" />
          <text x="14" y="19" fontSize="10" fill="#fff" fontWeight="bold">G</text>
          <text x="24" y="25" fontSize="10" fill="#fff" fontWeight="bold">文</text>
        </svg>
      );
    case 'photos':
      return (
        <svg className="app-svg" viewBox="0 0 40 40">
          <path d="M20 20v-9a4.5 4.5 0 0 1 9 0v9z" fill="#ea4335" />
          <path d="M20 20h9a4.5 4.5 0 0 1 0 9h-9z" fill="#fbbc05" />
          <path d="M20 20v9a4.5 4.5 0 0 1-9 0v-9z" fill="#34a853" />
          <path d="M20 20h-9a4.5 4.5 0 0 1 0-9h9z" fill="#4285f4" />
        </svg>
      );
    default:
      return (
        <svg className="app-svg" viewBox="0 0 40 40">
          <circle cx="20" cy="20" r="16" fill="#4285f4" />
          <text x="20" y="25" fontSize="14" fill="#fff" fontWeight="bold" textAnchor="middle">
            G
          </text>
        </svg>
      );
  }
}
