# DESIGN.md — Aphrody (`aphrody.com`)

> Canonical machine- and human-readable design specification following the `DESIGN.md` open standard (`google-labs-code/design.md`) and enforced by `@aphrody/google-design` (`gds`) and `@aphrody/wck`.

## 1. Brand & Product Philosophy

- **Identity**: Aphrody (`https://aphrody.com`), generalist web search portal, multi-ecosystem package catalog, native Omarchy x CachyOS WSL2 hub, and open-source creative asset bank.
- **Engineering Lineage**:
  - _Linus Torvalds_: Flat, honest information hierarchy; zero decorative bloat; deterministic system behavior.
  - _Jarred Sumner (Bun v1.4.4)_: Sub-millisecond feedback, zero external npm dependencies, pre-compressed in-memory asset serving.
  - _Larry Page_: Immediate focal point on the primary task (search & discovery), generous negative space, sub-second Largest Contentful Paint.
- **Strict AI Boundary**: `@aphrody/ai-os` is the sole public AI product in the monorepo. `aphrody.com` uses factual, human-edited technical language with zero AI marketing buzzwords and zero emojis.

## 2. Adaptive Tri-System Token Architecture

`aphrody.com` supports three user-selectable visual shells sharing one semantic CSS variable contract:

### 2.1 Color System (Light / Dark)

| Token Role        | Classic Web (Light / Dark) | Material Design 3 HCT (Light / Dark) | Fluent UI 2 Mica (Light / Dark)       |
| :---------------- | :------------------------- | :----------------------------------- | :------------------------------------ |
| Surface Primary   | `#ffffff` / `#202124`      | `#fef7ff` / `#141218`                | `#ffffff` / `#292929`                 |
| Surface Container | `#f8f9fa` / `#303134`      | `#f3edf7` / `#211f26`                | `#f5f5f5` / `#1f1f1f`                 |
| Text Primary      | `#1f1f1f` / `#e8eaed`      | `#1d1b20` / `#e6e0e9`                | `#242424` / `#ffffff`                 |
| Text Secondary    | `#4d5156` / `#bdc1c6`      | `#49454f` / `#cac4d0`                | `#616161` / `#d6d6d6`                 |
| Accent Primary    | `#1a73e8` / `#8ab4f8`      | `#6750a4` / `#d0bcff`                | `#0f6cbd` / `#479ef5`                 |
| Outline / Stroke  | `#dfe1e5` / `#5f6368`      | `#79747e` / `#938f99`                | `#e0e0e0` / `#424242`                 |
| Focus Ring        | `2px solid #1a73e8`        | `2px solid #6750a4`                  | `2px solid #000000` (inner `#ffffff`) |

### 2.2 Typography Scale (Zero-CLS System & Metric-Matched OFL Stack)

- **UI Sans**: `"Inter", "Segoe UI Variable", system-ui, -apple-system, BlinkMacSystemFont, Roboto, sans-serif`
- **Display / Brand**: `"Space Grotesk", "Inter", system-ui, sans-serif`
- **Code / Commands**: `"JetBrains Mono", "Cascadia Mono", ui-monospace, SFMono-Regular, Menlo, monospace`
- **Ramp**:
  - `H1 Page`: `32px / 40px`, weight `400`, letter-spacing `-0.01em`
  - `H2 Section`: `22px / 28px`, weight `500`, letter-spacing `-0.005em`
  - `Lead`: `16px / 24px`, weight `400`, max-measure `68ch`
  - `Body / Card`: `14px / 20px`, weight `400`
  - `Code`: `13px / 18px`, weight `400`

### 2.3 Spacing, Touch Targets & Layout Grid

- **Base Unit**: `4px` spatial rhythm (`4, 8, 12, 16, 24, 32, 48, 64px`).
- **Interactive Touch Minimum**: Every interactive link, chip, or button in navigation and content pages maintains `min-height: 48px` (WCAG 2.2 / Material 3 target parity).
- **Content Container**: `max-width: 840px` for editorial/hub reading; `repeat(auto-fill, minmax(260px, 1fr))` responsive grid for asset, theme, and package cards.
- **Safe Areas**: Respect `env(safe-area-inset-top|right|bottom|left)` on notched mobile and tablet viewports.

## 3. Motion, 3D WebGPU & Asset Standards

- **Reduced Motion First**: Honor `@media (prefers-reduced-motion: reduce)` across all transitions, canvas scenes, and sprite animations.
- **Zero Layout Shift (`CLS = 0`)**: Every image, SVG preview, Open Pet GIF, 3D canvas, and optional ad container reserves explicit intrinsic dimensions (`width`/`height` or `min-height` + `contain: strict`).
- **Open Creative Asset Formats**:
  - Icons: `24x24` viewBox, `1.75px` uniform stroke, rounded caps/joins, pixel-aligned coordinates.
  - Open Pets: Crisp integer-scaled pixel art exported as looping `GIF89a` binaries and `<svg>` sprite sheets.
  - 3D Models: Valid binary glTF 2.0 (`.glb`), JSON `.gltf`, and Wavefront `.obj` with physically based metallic-roughness parameters.
  - Audio: 44.1 kHz 16-bit PCM `.wav` micro-interactions under `60 KB` each, triggered only on explicit user click.

## 4. Anti-AI-Slop Enforcement Checklist (`impeccable` + `humanizer`)

- Strictly zero emoji characters (`\p{Extended_Pictographic}`) in UI, copy, SVG labels, or metadata.
- Strictly zero puffery clichés (`revolutionary`, `unleash`, `delve`, `seamless`, `game-changer`, `AI-powered magic`).
- Strictly zero unrequested decorative purple/neon SaaS mesh gradients or fake testimonials/ratings (`aggregateRating` is never emitted without verifiable user reviews).
- Every claim on `aphrody.com` describes concrete, verifiable behavior inspectable via public endpoints (`/api/q`, `/api/pm`, `/api/bnc`, `/api/distro/themes`, `/api/assets/catalog`, `/ok`).
