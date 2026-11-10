import React, { useEffect, useState } from 'react';

interface VoiceModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSpeechResult: (text: string) => void;
}

export const VoiceModal: React.FC<VoiceModalProps> = ({ isOpen, onClose, onSpeechResult }) => {
  const [statusText, setStatusText] = useState('Écoute en cours...');

  useEffect(() => {
    if (!isOpen) return;

    setStatusText('Écoute en cours...');

    // Attempt HTML5 SpeechRecognition if available
    const SpeechRecognition =
      (window as unknown as { SpeechRecognition?: any }).SpeechRecognition ||
      (window as unknown as { webkitSpeechRecognition?: any }).webkitSpeechRecognition;

    if (SpeechRecognition) {
      try {
        const recognition = new SpeechRecognition();
        recognition.lang = 'fr-FR';
        recognition.interimResults = false;
        recognition.maxAlternatives = 1;

        recognition.onresult = (event: any) => {
          const transcript = event.results[0][0].transcript;
          if (transcript) {
            onSpeechResult(transcript);
            onClose();
          }
        };

        recognition.onerror = () => {
          setStatusText('Dites quelque chose...');
        };

        recognition.start();

        return () => {
          recognition.abort();
        };
      } catch (err) {
        console.warn('SpeechRecognition error:', err);
      }
    }
  }, [isOpen, onClose, onSpeechResult]);

  if (!isOpen) return null;

  return (
    <div className="voice-overlay" role="dialog" aria-modal="true">
      <div className="voice-card">
        <button
          type="button"
          className="voice-close-btn"
          onClick={onClose}
          aria-label="Fermer"
        >
          <svg viewBox="0 0 24 24">
            <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z" />
          </svg>
        </button>

        <h2 className="voice-status">{statusText}</h2>

        <div className="voice-mic-circle" onClick={onClose} title="Arrêter l'écoute">
          <div className="pulse-ring ring-1" />
          <div className="pulse-ring ring-2" />
          <svg className="voice-mic-svg" viewBox="0 0 24 24">
            <path fill="#ffffff" d="m12 15c1.66 0 3-1.31 3-2.97v-7.02c0-1.66-1.34-3.01-3-3.01s-3 1.34-3 3.01v7.02c0 1.66 1.34 2.97 3 2.97z" />
            <path fill="#ffffff" d="m11 18.08h2v3.92h-2z" />
            <path fill="#ffffff" d="m7.05 10.87-1.42-1.42c-.44.58-.75 1.25-.89 1.98l1.97.39c.07-.36.19-.68.34-.95z" />
            <path fill="#ffffff" d="m12 17c-2.76 0-5-2.24-5-5h-2c0 3.53 2.61 6.43 6 6.92v2.08h2v-2.08c3.39-.49 6-3.39 6-6.92h-2c0 2.76-2.24 5-5 5z" />
          </svg>
        </div>
      </div>

      <style>{`
        .voice-overlay {
          position: fixed;
          top: 0;
          left: 0;
          right: 0;
          bottom: 0;
          background-color: rgba(0, 0, 0, 0.4);
          display: flex;
          align-items: center;
          justify-content: center;
          z-index: 2000;
          backdrop-filter: blur(2px);
        }

        .voice-card {
          position: relative;
          width: 90%;
          max-width: 480px;
          background-color: var(--popup-bg);
          border-radius: 24px;
          box-shadow: var(--popup-shadow);
          padding: 40px 24px;
          display: flex;
          flex-direction: column;
          align-items: center;
        }

        .voice-close-btn {
          position: absolute;
          top: 16px;
          right: 16px;
          width: 36px;
          height: 36px;
          border-radius: 50%;
          border: none;
          background: transparent;
          cursor: pointer;
          color: var(--icon-color);
          display: flex;
          align-items: center;
          justify-content: center;
        }

        .voice-close-btn svg {
          width: 22px;
          height: 22px;
          fill: currentColor;
        }

        .voice-status {
          font-family: 'Google Sans', Roboto, sans-serif;
          font-size: 24px;
          font-weight: 400;
          color: var(--text-color);
          margin-bottom: 40px;
        }

        .voice-mic-circle {
          position: relative;
          width: 80px;
          height: 80px;
          border-radius: 50%;
          background-color: #ea4335;
          display: flex;
          align-items: center;
          justify-content: center;
          cursor: pointer;
        }

        .voice-mic-svg {
          width: 36px;
          height: 36px;
          z-index: 2;
        }

        .pulse-ring {
          position: absolute;
          border-radius: 50%;
          border: 3px solid #ea4335;
          opacity: 0.8;
          animation: pulse 1.8s cubic-bezier(0.215, 0.61, 0.355, 1) infinite;
        }

        .ring-1 {
          width: 95px;
          height: 95px;
        }

        .ring-2 {
          width: 115px;
          height: 115px;
          animation-delay: 0.4s;
        }

        @keyframes pulse {
          0% {
            transform: scale(0.8);
            opacity: 0.8;
          }
          50% {
            transform: scale(1.3);
            opacity: 0.3;
          }
          100% {
            transform: scale(1.6);
            opacity: 0;
          }
        }
      `}</style>
    </div>
  );
};
