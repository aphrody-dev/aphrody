import React, { useState } from 'react';

interface LensModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const LensModal: React.FC<LensModalProps> = ({ isOpen, onClose }) => {
  const [imageUrl, setImageUrl] = useState('');
  const [isDragging, setIsDragging] = useState(false);

  if (!isOpen) return null;

  const handleSearchImage = () => {
    if (imageUrl.trim()) {
      window.location.href = `https://lens.google.com/uploadbyurl?url=${encodeURIComponent(imageUrl.trim())}`;
    }
  };

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      alert(`Image sélectionnée: ${file.name}. Redirection vers Google Lens...`);
      window.location.href = 'https://lens.google.com/';
    }
  };

  return (
    <div className="lens-overlay" role="dialog" aria-modal="true">
      <div className="lens-card">
        <div className="lens-header">
          <span className="lens-title">Rechercher n'importe quelle image avec Google Lens</span>
          <button
            type="button"
            className="lens-close-btn"
            onClick={onClose}
            aria-label="Fermer"
          >
            <svg viewBox="0 0 24 24">
              <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z" />
            </svg>
          </button>
        </div>

        {/* Drag and Drop Zone */}
        <div
          className={`lens-dropzone ${isDragging ? 'dragging' : ''}`}
          onDragOver={(e) => {
            e.preventDefault();
            setIsDragging(true);
          }}
          onDragLeave={() => setIsDragging(false)}
          onDrop={(e) => {
            e.preventDefault();
            setIsDragging(false);
            window.location.href = 'https://lens.google.com/';
          }}
        >
          <div className="lens-camera-icon">
            <svg viewBox="0 0 192 192">
              <path fill="#4285F4" d="M129 119a33 33 0 1 1-66 0 33 33 0 0 1 66 0" />
              <path fill="#EA4335" d="M141 33h-19l-9-19H79l-9 19H51C37.7 33 27 43.7 27 57v78c0 13.3 10.7 24 24 24h90c13.3 0 24-10.7 24-24V57c0-13.3-10.7-24-24-24m-45 107c-24.8 0-45-20.2-45-45s20.2-45 45-45 45 20.2 45 45-20.2 45-45 45m53-73c-4.4 0-8-3.6-8-8s3.6-8 8-8 8 3.6 8 8-3.6 8-8 8" />
            </svg>
          </div>
          <p className="lens-drop-text">
            Glissez une image ici ou{' '}
            <label className="lens-browse-label">
              importez un fichier
              <input
                type="file"
                accept="image/*"
                style={{ display: 'none' }}
                onChange={handleFileUpload}
              />
            </label>
          </p>
        </div>

        {/* OR Divider */}
        <div className="lens-or-divider">
          <span>OU</span>
        </div>

        {/* Paste URL */}
        <div className="lens-url-row">
          <input
            type="text"
            className="lens-url-input"
            placeholder="Coller le lien de l'image"
            value={imageUrl}
            onChange={(e) => setImageUrl(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleSearchImage()}
          />
          <button
            type="button"
            className="lens-search-btn"
            onClick={handleSearchImage}
            disabled={!imageUrl.trim()}
          >
            Rechercher
          </button>
        </div>
      </div>

      <style>{`
        .lens-overlay {
          position: fixed;
          top: 0;
          left: 0;
          right: 0;
          bottom: 0;
          background-color: rgba(0, 0, 0, 0.45);
          display: flex;
          align-items: center;
          justify-content: center;
          z-index: 2000;
          backdrop-filter: blur(2px);
        }

        .lens-card {
          width: 90%;
          max-width: 580px;
          background-color: var(--popup-bg);
          border-radius: 24px;
          box-shadow: var(--popup-shadow);
          padding: 24px 28px;
        }

        .lens-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: 20px;
        }

        .lens-title {
          font-family: 'Google Sans', Roboto, sans-serif;
          font-size: 16px;
          font-weight: 500;
          color: var(--text-color);
        }

        .lens-close-btn {
          width: 32px;
          height: 32px;
          border-radius: 50%;
          border: none;
          background: transparent;
          cursor: pointer;
          color: var(--icon-color);
          display: flex;
          align-items: center;
          justify-content: center;
        }

        .lens-close-btn svg {
          width: 20px;
          height: 20px;
          fill: currentColor;
        }

        .lens-dropzone {
          border: 1px dashed var(--lens-border);
          border-radius: 16px;
          padding: 36px 20px;
          text-align: center;
          background-color: var(--icon-hover-bg);
          transition: border-color 0.2s, background-color 0.2s;
        }

        .lens-dropzone.dragging {
          border-color: var(--blue-primary);
          background-color: rgba(26, 115, 232, 0.05);
        }

        .lens-camera-icon {
          width: 48px;
          height: 48px;
          margin: 0 auto 12px auto;
        }

        .lens-camera-icon svg {
          width: 100%;
          height: 100%;
        }

        .lens-drop-text {
          font-size: 14px;
          color: var(--text-color);
        }

        .lens-browse-label {
          color: var(--blue-primary);
          font-weight: 500;
          cursor: pointer;
        }

        .lens-browse-label:hover {
          text-decoration: underline;
        }

        .lens-or-divider {
          position: relative;
          text-align: center;
          margin: 20px 0;
        }

        .lens-or-divider::before {
          content: '';
          position: absolute;
          top: 50%;
          left: 0;
          right: 0;
          height: 1px;
          background-color: var(--divider);
        }

        .lens-or-divider span {
          position: relative;
          background-color: var(--popup-bg);
          padding: 0 12px;
          font-size: 12px;
          color: var(--text-secondary);
        }

        .lens-url-row {
          display: flex;
          gap: 12px;
        }

        .lens-url-input {
          flex: 1;
          height: 44px;
          border: 1px solid var(--search-border);
          border-radius: 22px;
          padding: 0 16px;
          background-color: var(--search-bg);
          color: var(--text-color);
          outline: none;
          font-size: 14px;
        }

        .lens-url-input:focus {
          border-color: var(--blue-primary);
        }

        .lens-search-btn {
          height: 44px;
          padding: 0 20px;
          border-radius: 22px;
          border: none;
          background-color: var(--blue-primary);
          color: #ffffff;
          font-family: 'Google Sans', Roboto, sans-serif;
          font-size: 14px;
          font-weight: 500;
          cursor: pointer;
          transition: background-color 0.2s, opacity 0.2s;
        }

        .lens-search-btn:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }

        .lens-search-btn:not(:disabled):hover {
          background-color: var(--blue-hover);
        }
      `}</style>
    </div>
  );
};
