# aphrody.com

Pixel-identical replica of `https://www.google.com/` aligned with `https://design.google/` and Chromium UI (`C:\src\chromium\src\docs\ui\index.md` / GoogleChrome standards).

Built in **100% pure oven-sh/bun TypeScript + Web Standards (DOM, HTML5, CSS3, SVG) with ZERO external dependencies**.

## Architecture & Principles
- **Zero External Dependencies**: Zero npm dependencies (`dependencies: {}`, `devDependencies: {}`), no `node_modules`, no `bun.lock`.
- **Pure Bun Toolchain**: `Bun.serve()` HTTP server, `Bun.build()` native bundler, `bun:test` test framework, and `Bun.gzipSync()` compression.
- **Web Standards**: Native DOM manipulation, Custom Elements, HTML5 Web Speech API (`SpeechRecognition` / `webkitSpeechRecognition`), CSS variables with Material Design 3 and Chromium UI design tokens.
- **Ultra-light Bundle**: Full gzipped JS/CSS/HTML bundle is under 15 KB (~12.3 KB total).

## Features
- **Official Google Design System**: Pixel-accurate proportions, typography (`Google Sans`, `Roboto`, `Arial`), colors, elevation tokens, focus rings, and hover states.
- **Top Header**: Right-aligned Gmail, Images, 9-dot Google Apps launcher grid (interactive popover with 18 official Google services and crisp vector SVGs), and "Connexion" Material 3 pill button.
- **Center Area**: Authentic 272x92 Google multicolor vector wordmark (with interactive click toggle for Aphrody wordmark in the Google palette), search bar with left magnifying glass, dynamic clear button `X` with vertical divider, Voice Search multicolor mic with animated pulsing listening modal, Google Lens camera with drag-and-drop / upload / paste URL dialog, and autocomplete suggestion dropdown with keyboard navigation (ArrowUp, ArrowDown, Enter, Escape).
- **Search Actions**: Centered "Recherche Google" and "J'ai de la chance" buttons with live redirects to search queries and doodles.
- **Footer**: Top row "France", bottom flex row with links ("À propos", "Publicité", "Entreprise", "Comment fonctionne la recherche Google ?", climate leaf link, "Confidentialité", "Conditions", and "Paramètres" popup menu with Dark theme toggle).
- **Dark Theme**: Authentic Google dark mode with `#202124` background and `#303134` elements, persisted across sessions in `localStorage`.

## Commands

```bash
# Build production bundle with native Bun.build()
bun run build

# Run native Bun HTTP server (serves static dist/ + /api/complete)
bun run start

# Run pure bun:test test suite
bun test
```
