import React, { useState, useRef, useEffect } from 'react';
import { getSuggestions, SuggestionItem } from '../data/suggestions';

interface SearchBarProps {
  onOpenVoice: () => void;
  onOpenLens: () => void;
}

export const SearchBar: React.FC<SearchBarProps> = ({ onOpenVoice, onOpenLens }) => {
  const [query, setQuery] = useState('');
  const [isOpen, setIsOpen] = useState(false);
  const [suggestions, setSuggestions] = useState<SuggestionItem[]>([]);
  const [selectedIndex, setSelectedIndex] = useState<number>(-1);
  const inputRef = useRef<HTMLInputElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    setSuggestions(getSuggestions(query));
  }, [query]);

  // Handle click outside to close dropdown
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setIsOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const handleSearch = (searchQuery: string) => {
    const target = searchQuery.trim() || query.trim();
    if (target) {
      window.location.href = `https://www.google.com/search?q=${encodeURIComponent(target)}`;
    } else {
      inputRef.current?.focus();
    }
  };

  const handleFeelingLucky = () => {
    const target = query.trim();
    if (target) {
      window.location.href = `https://www.google.com/search?q=${encodeURIComponent(target)}&btnI=I`;
    } else {
      window.location.href = 'https://doodles.google/';
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex((prev) => (prev < suggestions.length - 1 ? prev + 1 : prev));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex((prev) => (prev > 0 ? prev - 1 : -1));
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (selectedIndex >= 0 && suggestions[selectedIndex]) {
        handleSearch(suggestions[selectedIndex].text);
      } else {
        handleSearch(query);
      }
    } else if (e.key === 'Escape') {
      setIsOpen(false);
      inputRef.current?.blur();
    }
  };

  const handleClear = () => {
    setQuery('');
    setSelectedIndex(-1);
    inputRef.current?.focus();
  };

  return (
    <div className="search-container" ref={containerRef}>
      <div className={`search-box ${isOpen ? 'open' : ''}`}>
        {/* Left Magnifying Glass */}
        <div className="search-icon-left" onClick={() => handleSearch(query)}>
          <svg focusable="false" viewBox="0 0 24 24" className="lens-search-svg">
            <path d="M15.5 14h-.79l-.28-.27A6.471 6.471 0 0 0 16 9.5 6.5 6.5 0 1 0 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z" />
          </svg>
        </div>

        {/* Search Input */}
        <input
          ref={inputRef}
          type="text"
          className="search-input"
          value={selectedIndex >= 0 ? suggestions[selectedIndex]?.text : query}
          onChange={(e) => {
            setQuery(e.target.value);
            setSelectedIndex(-1);
            setIsOpen(true);
          }}
          onFocus={() => setIsOpen(true)}
          onKeyDown={handleKeyDown}
          autoComplete="off"
          aria-label="Recherche"
          spellCheck={false}
        />

        {/* Clear Button & Divider */}
        {query && (
          <>
            <button
              type="button"
              className="clear-btn"
              onClick={handleClear}
              aria-label="Effacer la recherche"
            >
              <svg focusable="false" viewBox="0 0 24 24">
                <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z" />
              </svg>
            </button>
            <div className="search-divider" />
          </>
        )}

        {/* Right Google Mic & Lens */}
        <div className="search-tools-right">
          {/* Voice Search (Mic) */}
          <button
            type="button"
            className="tool-btn mic-btn"
            onClick={onOpenVoice}
            aria-label="Recherche vocale"
            title="Recherche vocale"
          >
            <svg className="mic-svg" viewBox="0 0 24 24">
              <path fill="#4285f4" d="m12 15c1.66 0 3-1.31 3-2.97v-7.02c0-1.66-1.34-3.01-3-3.01s-3 1.34-3 3.01v7.02c0 1.66 1.34 2.97 3 2.97z" />
              <path fill="#34a853" d="m11 18.08h2v3.92h-2z" />
              <path fill="#fbbc05" d="m7.05 10.87-1.42-1.42c-.44.58-.75 1.25-.89 1.98l1.97.39c.07-.36.19-.68.34-.95z" />
              <path fill="#ea4335" d="m12 17c-2.76 0-5-2.24-5-5h-2c0 3.53 2.61 6.43 6 6.92v2.08h2v-2.08c3.39-.49 6-3.39 6-6.92h-2c0 2.76-2.24 5-5 5z" />
            </svg>
          </button>

          {/* Google Lens (Camera) */}
          <button
            type="button"
            className="tool-btn lens-btn"
            onClick={onOpenLens}
            aria-label="Recherche par image"
            title="Recherche par image"
          >
            <svg className="lens-svg" viewBox="0 0 192 192">
              <path fill="#4285F4" d="M129 119a33 33 0 1 1-66 0 33 33 0 0 1 66 0" />
              <path fill="#EA4335" d="M141 33h-19l-9-19H79l-9 19H51C37.7 33 27 43.7 27 57v78c0 13.3 10.7 24 24 24h90c13.3 0 24-10.7 24-24V57c0-13.3-10.7-24-24-24m-45 107c-24.8 0-45-20.2-45-45s20.2-45 45-45 45 20.2 45 45-20.2 45-45 45m53-73c-4.4 0-8-3.6-8-8s3.6-8 8-8 8 3.6 8 8-3.6 8-8 8" />
            </svg>
          </button>
        </div>
      </div>

      {/* Autocomplete Dropdown */}
      {isOpen && suggestions.length > 0 && (
        <div className="search-dropdown">
          <div className="dropdown-divider" />
          <ul className="suggestions-list" role="listbox">
            {suggestions.map((item, idx) => (
              <li
                key={idx}
                className={`suggestion-item ${selectedIndex === idx ? 'selected' : ''}`}
                onMouseDown={() => handleSearch(item.text)}
                onMouseEnter={() => setSelectedIndex(idx)}
              >
                <div className="suggestion-icon">
                  {item.isHistory ? (
                    <svg viewBox="0 0 24 24" className="history-svg">
                      <path d="M13 3a9 9 0 0 0-9 9H1l3.89 3.89.07.14L9 12H6c0-3.87 3.13-7 7-7s7 3.13 7 7-3.13 7-7 7c-1.93 0-3.68-.79-4.94-2.06l-1.42 1.42A8.954 8.954 0 0 0 13 21a9 9 0 0 0 0-18zm-1 5v5l4.28 2.54.72-1.21-3.5-2.08V8H12z" />
                    </svg>
                  ) : (
                    <svg viewBox="0 0 24 24" className="sugg-lens-svg">
                      <path d="M15.5 14h-.79l-.28-.27A6.471 6.471 0 0 0 16 9.5 6.5 6.5 0 1 0 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z" />
                    </svg>
                  )}
                </div>
                <span className="suggestion-text">{item.text}</span>
              </li>
            ))}
          </ul>
        </div>
      )}

      {/* Google Search Buttons */}
      <div className="search-buttons-row">
        <button
          type="button"
          className="google-btn"
          onClick={() => handleSearch(query)}
        >
          Recherche Google
        </button>
        <button
          type="button"
          className="google-btn"
          onClick={handleFeelingLucky}
        >
          J'ai de la chance
        </button>
      </div>

      <style>{`
        .search-container {
          width: 100%;
          max-width: 584px;
          margin: 0 auto;
          position: relative;
        }

        .search-box {
          display: flex;
          align-items: center;
          height: 46px;
          background-color: var(--search-bg);
          border: 1px solid var(--search-border);
          border-radius: 24px;
          padding: 0 14px;
          box-shadow: none;
          transition: background-color 0.2s, border-color 0.2s, box-shadow 0.2s;
        }

        .search-box:hover {
          box-shadow: var(--search-shadow);
          border-color: var(--search-border-hover);
        }

        .search-box.open {
          border-radius: 24px 24px 0 0;
          box-shadow: var(--search-shadow-focus);
          border-bottom-color: transparent;
        }

        .search-icon-left {
          display: flex;
          align-items: center;
          justify-content: center;
          width: 20px;
          height: 20px;
          margin-right: 12px;
          color: #9aa0a6;
          cursor: pointer;
        }

        .lens-search-svg {
          width: 20px;
          height: 20px;
          fill: currentColor;
        }

        .search-input {
          flex: 1;
          height: 100%;
          background: transparent;
          border: none;
          outline: none;
          color: var(--text-color);
          font-size: 16px;
        }

        .clear-btn {
          background: transparent;
          border: none;
          color: #70757a;
          cursor: pointer;
          display: flex;
          align-items: center;
          justify-content: center;
          padding: 0 4px;
        }

        .clear-btn svg {
          width: 20px;
          height: 20px;
          fill: currentColor;
        }

        .search-divider {
          width: 1px;
          height: 65%;
          background-color: var(--divider);
          margin: 0 8px;
        }

        .search-tools-right {
          display: flex;
          align-items: center;
          gap: 6px;
        }

        .tool-btn {
          width: 32px;
          height: 32px;
          display: flex;
          align-items: center;
          justify-content: center;
          background: transparent;
          border: none;
          cursor: pointer;
          border-radius: 50%;
          transition: background 0.15s;
        }

        .tool-btn:hover {
          background: var(--icon-hover-bg);
        }

        .mic-svg {
          width: 24px;
          height: 24px;
        }

        .lens-svg {
          width: 22px;
          height: 22px;
        }

        .search-dropdown {
          position: absolute;
          top: 46px;
          left: 0;
          right: 0;
          background-color: var(--search-bg);
          border: 1px solid var(--search-border);
          border-top: none;
          border-radius: 0 0 24px 24px;
          box-shadow: var(--search-shadow-focus);
          z-index: 50;
          padding-bottom: 12px;
        }

        .dropdown-divider {
          height: 1px;
          background-color: var(--divider);
          margin: 0 14px 4px 14px;
        }

        .suggestions-list {
          list-style: none;
        }

        .suggestion-item {
          display: flex;
          align-items: center;
          height: 36px;
          padding: 0 16px;
          cursor: pointer;
        }

        .suggestion-item:hover,
        .suggestion-item.selected {
          background-color: var(--dropdown-hover);
        }

        .suggestion-icon {
          width: 20px;
          height: 20px;
          margin-right: 14px;
          color: #9aa0a6;
          display: flex;
          align-items: center;
          justify-content: center;
        }

        .suggestion-icon svg {
          width: 18px;
          height: 18px;
          fill: currentColor;
        }

        .suggestion-text {
          font-size: 14px;
          color: var(--text-color);
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }

        .search-buttons-row {
          display: flex;
          justify-content: center;
          gap: 12px;
          margin-top: 28px;
        }

        .google-btn {
          background-color: var(--btn-bg);
          border: 1px solid var(--btn-border);
          color: var(--btn-text);
          font-family: Arial, Roboto, sans-serif;
          font-size: 14px;
          height: 36px;
          padding: 0 16px;
          border-radius: 4px;
          cursor: pointer;
          user-select: none;
          transition: border-color 0.15s, box-shadow 0.15s, color 0.15s;
        }

        .google-btn:hover {
          background-color: var(--btn-hover-bg);
          border-color: var(--btn-hover-border);
          color: var(--btn-hover-text);
          box-shadow: 0 1px 1px rgba(0, 0, 0, 0.1);
        }
      `}</style>
    </div>
  );
};
