# aphrody.com

Pixel-faithful replica of `https://www.google.com/`.

## Features
- **Official Google Design System**: Pixel-accurate proportions, typography (`Google Sans`, `Roboto`, `Arial`), colors, and hover states.
- **Top Header**: Right-aligned Gmail, Images, 9-dot Google Apps launcher grid (interactive modal with 18 Google services), and "Connexion" Material 3 pill button.
- **Center Area**: Authentic 272x92 Google multicolor vector wordmark (with interactive click toggle for Aphrody wordmark in the Google palette), search bar with left magnifying glass, dynamic clear button `X` with vertical divider, Voice Search multicolor mic with animated listening modal, Google Lens camera with drag-and-drop / upload / paste URL dialog, and autocomplete suggestion dropdown with keyboard navigation.
- **Search Actions**: Centered "Recherche Google" and "J'ai de la chance" buttons with live redirects to search queries and doodles.
- **Footer**: Top row "France", bottom flex row with links ("À propos", "Publicité", "Entreprise", "Comment fonctionne la recherche Google ?", climate leaf badge, "Confidentialité", "Conditions", and "Paramètres" popup menu with Dark theme toggle).
- **Dark Theme**: Authentic Google dark mode with `#202124` background and `#303134` elements, persisted across sessions.

## Development & Build
Built with Bun, Vite, React 19, and TypeScript. Zero legacy runtime dependencies.

```bash
# Install dependencies
bun install

# Run dev server
bun run dev

# Run test suite
bun test

# Production build
bun run build

# Start production server
bun start
```
