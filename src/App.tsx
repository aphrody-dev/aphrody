import React, { useState, useEffect } from 'react';
import { Header } from './components/Header';
import { Logo } from './components/Logo';
import { SearchBar } from './components/SearchBar';
import { LanguageBar } from './components/LanguageBar';
import { Footer } from './components/Footer';
import { AppsModal } from './components/AppsModal';
import { VoiceModal } from './components/VoiceModal';
import { LensModal } from './components/LensModal';

export const App: React.FC = () => {
  const [theme, setTheme] = useState<'light' | 'dark'>(() => {
    const saved = localStorage.getItem('aphrody_theme');
    if (saved === 'dark' || saved === 'light') return saved;
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  });

  const [isAppsOpen, setIsAppsOpen] = useState(false);
  const [isVoiceOpen, setIsVoiceOpen] = useState(false);
  const [isLensOpen, setIsLensOpen] = useState(false);

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme);
    localStorage.setItem('aphrody_theme', theme);
  }, [theme]);

  const toggleTheme = () => {
    setTheme((prev) => (prev === 'light' ? 'dark' : 'light'));
  };

  const handleSpeechResult = (text: string) => {
    if (text) {
      window.location.href = `https://www.google.com/search?q=${encodeURIComponent(text)}`;
    }
  };

  return (
    <div className="google-page-wrapper">
      {/* Header Bar */}
      <Header
        onToggleApps={() => setIsAppsOpen(!isAppsOpen)}
        isAppsOpen={isAppsOpen}
      />

      {/* Google Apps Launcher Popover */}
      <AppsModal
        isOpen={isAppsOpen}
        onClose={() => setIsAppsOpen(false)}
      />

      {/* Main Centered Content */}
      <main className="google-main">
        <Logo />
        <SearchBar
          onOpenVoice={() => setIsVoiceOpen(true)}
          onOpenLens={() => setIsLensOpen(true)}
        />
        <LanguageBar />
      </main>

      {/* Bottom Sticky Footer */}
      <Footer
        theme={theme}
        onToggleTheme={toggleTheme}
      />

      {/* Voice Search Modal */}
      <VoiceModal
        isOpen={isVoiceOpen}
        onClose={() => setIsVoiceOpen(false)}
        onSpeechResult={handleSpeechResult}
      />

      {/* Google Lens Modal */}
      <LensModal
        isOpen={isLensOpen}
        onClose={() => setIsLensOpen(false)}
      />

      <style>{`
        .google-page-wrapper {
          display: flex;
          flex-direction: column;
          min-height: 100vh;
          position: relative;
        }

        .google-main {
          flex: 1;
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: center;
          padding: 0 20px 80px 20px;
        }
      `}</style>
    </div>
  );
};
