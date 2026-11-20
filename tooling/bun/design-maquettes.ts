/**
 * Interactive 12-Screen Cross-Platform Maquette Suite & Headless CDP Screenshot Capture
 * for Aphrody Labs (Project, Website, Desktop, Mobile) anchored to https://design.google/
 * and Google Stitch (`data-stitch-id`).
 */
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { extname, join, resolve } from "node:path";
import { tmpdir } from "node:os";

export type MaquetteSurface = "labs" | "website" | "desktop" | "mobile";
export type MaquetteDeviceType = "DESKTOP" | "MOBILE" | "TABLET";

export interface MaquetteDefinition {
  id: string;
  file: string;
  stitchId: string;
  surface: MaquetteSurface;
  deviceType: MaquetteDeviceType;
  title: string;
  subtitle: string;
  defaultTheme: "neural" | "dark" | "light" | "oled";
  gpuiSurface: string;
  nshComponent: string;
  keyPatterns: string[];
  slots: string[];
  renderBody: () => string;
}

function escapeHtml(text: string): string {
  return text
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function topChrome(activeId: string, surface: MaquetteSurface, title: string, badge: string): string {
  const navLinks = [
    { id: "index", href: "./index.html", label: "System Index" },
    { id: "labs", href: "./01-labs-ecosystem-architecture.html", label: "Aphrody Labs" },
    { id: "website", href: "./03-web-landing-hero.html", label: "Website" },
    { id: "desktop", href: "./06-desktop-aphrody-ide.html", label: "Desktop" },
    { id: "mobile", href: "./10-mobile-companion-home.html", label: "Mobile" },
  ];
  return `<header class="mq-topbar" role="banner">
  <div class="mq-topbar-left">
    <a class="mq-brand-lockup" href="./index.html" aria-label="Aphrody Labs Design System Home">
      <img src="../svg/aphrody-monogram-neural.svg" width="28" height="28" alt="Aphrody Monogram" />
      <span class="mq-brand-name">Aphrody</span>
      <span class="mq-badge">${escapeHtml(badge)}</span>
    </a>
    <nav class="mq-top-nav" aria-label="Primary Surface Switcher">
      ${navLinks
        .map((n) => {
          const isCurrent = n.id === activeId || n.id === surface;
          return `<a class="mq-nav-link" href="${n.href}" ${isCurrent ? 'aria-current="page"' : ""}>${escapeHtml(n.label)}</a>`;
        })
        .join("")}
    </nav>
  </div>
  <div class="mq-topbar-right">
    <div class="mq-theme-switcher" role="group" aria-label="Theme Appearance Mode">
      <button type="button" class="mq-theme-btn" data-set-theme="neural">Neural</button>
      <button type="button" class="mq-theme-btn" data-set-theme="dark">Dark</button>
      <button type="button" class="mq-theme-btn" data-set-theme="light">Light</button>
      <button type="button" class="mq-theme-btn" data-set-theme="oled">OLED</button>
    </div>
    <span class="mq-meta-pill">${escapeHtml(title)}</span>
  </div>
</header>`;
}

function wrapHtmlDocument(options: {
  id: string;
  stitchId: string;
  surface: MaquetteSurface;
  title: string;
  badge: string;
  defaultTheme: "neural" | "dark" | "light" | "oled";
  bodyHtml: string;
  extraScript?: string;
}): string {
  return `<!doctype html>
<html lang="en" dir="ltr" data-theme="${options.defaultTheme}">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>${escapeHtml(options.title)} — Aphrody Design System</title>
  <link rel="stylesheet" href="../stylesheet.css" />
</head>
<body class="mq-body" data-surface="${options.surface}">
  <a class="mq-skip-link" href="#mq-main">Skip to main maquette canvas</a>
  ${topChrome(options.id, options.surface, options.title, options.badge)}
  <main id="mq-main" class="mq-main" data-stitch-id="${options.stitchId}" data-design-source="https://design.google/">
    ${options.bodyHtml}
  </main>
  <footer class="mq-footer" role="contentinfo">
    <span>Aphrody Labs Design System v3.0 &middot; Source: <a href="https://design.google/">design.google</a> &amp; <a href="https://stitch.withgoogle.com/">Google Stitch</a></span>
    <span class="gd-tabular">WCAG 2.2 AA &ge; 4.5:1 &middot; Google Sans Flex + Google Sans Code + JetBrains Mono</span>
  </footer>
  <script>
    (() => {
      const root = document.documentElement;
      const syncButtons = () => {
        const active = root.getAttribute("data-theme") || "neural";
        document.querySelectorAll("[data-set-theme]").forEach((btn) => {
          btn.setAttribute("aria-pressed", btn.getAttribute("data-set-theme") === active ? "true" : "false");
        });
      };
      document.querySelectorAll("[data-set-theme]").forEach((btn) => {
        btn.addEventListener("click", () => {
          const next = btn.getAttribute("data-set-theme");
          if (next) {
            root.setAttribute("data-theme", next);
            syncButtons();
          }
        });
      });
      syncButtons();
      ${options.extraScript ?? ""}
    })();
  </script>
</body>
</html>
`;
}

export const MAQUETTE_DEFINITIONS: MaquetteDefinition[] = [
  {
    id: "01-labs-ecosystem-architecture",
    file: "01-labs-ecosystem-architecture.html",
    stitchId: "stitch-labs-architecture",
    surface: "labs",
    deviceType: "DESKTOP",
    title: "Aphrody Pure-Rust Desktop OS & Google Cloud Ecosystem Architecture",
    subtitle: "Wayland, Windows, Wasm & Chromium Desktop + Google Workspace, Cloud AI, YouTube & Multimedia in Pure Rust",
    defaultTheme: "neural",
    gpuiSurface: "aphrody_nsh::StudioMode::GoogleDesignMirror",
    nshComponent: "<LabsArchitectureScreen />",
    keyPatterns: ["10 modular Rust domains", "Material 3 Expressive (Android 17/18)", "Pure-Rust Google SDK parity (Kotlin/Go/Python/TS)"],
    slots: ["stitch-labs-arch-hero", "stitch-labs-domains-grid", "stitch-labs-token-flow"],
    renderBody: () => `
      <section class="mq-hero mq-neural-aura" data-stitch-id="stitch-labs-arch-hero">
        <div class="mq-pill-eyebrow">Aphrody Pure-Rust Desktop OS &middot; Wayland &bull; Windows &bull; Wasm &bull; Chromium WebGPU</div>
        <h1 class="mq-display-title">One Pure-Rust Desktop OS &amp; Google Ecosystem</h1>
        <p class="mq-lead">Unified GPUI 120 FPS compositor, WGPU v30 + Naga shaders, Wasm Core 2.0 / WASI P2 AOT, Material 3 Expressive (Android 17/18 + Compose M3 1.5.0), full Google Workspace, official Google Cloud AI, YouTube/YouTube Music, and zero-FFmpeg multimedia studio.</p>
        <div class="mq-kpi-row">
          <article class="mq-kpi-card"><span class="mq-kpi-label">Active Domains</span><strong class="mq-kpi-value gd-tabular">10</strong><span class="mq-kpi-meta">aphrody-dev + aphrody-labs</span></article>
          <article class="mq-kpi-card"><span class="mq-kpi-label">M3 Expressive Modes</span><strong class="mq-kpi-value gd-tabular">4</strong><span class="mq-kpi-meta">Neural, Dark, Light, OLED</span></article>
          <article class="mq-kpi-card"><span class="mq-kpi-label">Auto-Merged Forks</span><strong class="mq-kpi-value gd-tabular">45</strong><span class="mq-kpi-meta">Rust, Go, TS, Python, Kotlin</span></article>
          <article class="mq-kpi-card"><span class="mq-kpi-label">Store Targets</span><strong class="mq-kpi-value gd-tabular">22</strong><span class="mq-kpi-meta">Wayland, Windows, Wasm, Mobile</span></article>
        </div>
      </section>
      <section class="mq-grid-2" data-stitch-id="stitch-labs-domains-grid">
        <div class="mq-card">
          <h2>Pure-Rust Desktop OS &amp; Control Plane (aphrody-dev)</h2>
          <ul class="mq-list">
            <li><strong>aphrody</strong> &mdash; Pure-Rust Desktop OS (Wayland/Windows/Wasm/Chromium), Studio IDE, GPUI, NSH, 22-target Store</li>
            <li><strong>ai-os</strong> &mdash; MCP, ACP, A2A 1.0, ADK-Rust, Gemini Computer Use &amp; AccessKit semantic OS pilotage</li>
            <li><strong>ay</strong> &mdash; SSH fleet orchestration &amp; WireGuard mesh (fr1 / fr2 / prd)</li>
            <li><strong>anu</strong> &mdash; Pure-Rust POSIX coreutils, shell &amp; Btrfs/LUKS2 Wayland userspace</li>
          </ul>
        </div>
        <div class="mq-card">
          <h2>Google Workspace, Cloud AI, Media &amp; Browser (aphrody-labs)</h2>
          <ul class="mq-list">
            <li><strong>aphrody_google</strong> &mdash; OAuth 2.0/gcloud, Drive/Gmail/Calendar/Docs/Sheets, Gemini/Vertex/Imagen/Veo/Lyria/STT/TTS, YouTube &amp; YouTube Music</li>
            <li><strong>aphrody_media &amp; bevy_gpui</strong> &mdash; Zero-FFmpeg video/audio/RAW/HDR studio, TSX motion, Watch FTS5 &amp; 3D WebGPU Gaussian Splatting</li>
            <li><strong>browser &amp; open</strong> &mdash; Chromium Views GPUI shell, CDP, OpenTUI &amp; WebGPU OS host</li>
            <li><strong>bun &amp; auto</strong> &mdash; NSH TSX + A2UI runtime and Cloudflare edge distribution</li>
          </ul>
        </div>
      </section>
      <section class="mq-card" data-stitch-id="stitch-labs-token-flow">
        <h2>Single Source of Truth Pipeline</h2>
        <div class="mq-flow-strip">
          <span class="mq-flow-node">product.toml + google_design_tokens.toml (Android 17/18 M3E)</span>
          <span class="mq-flow-arrow" aria-hidden="true">&rarr;</span>
          <span class="mq-flow-node">DESIGN.md + W3C DTCG Figma Variables</span>
          <span class="mq-flow-arrow" aria-hidden="true">&rarr;</span>
          <span class="mq-flow-node">Google Stitch + A2UI Streaming UI</span>
          <span class="mq-flow-arrow" aria-hidden="true">&rarr;</span>
          <span class="mq-flow-node">Wayland / Windows / Wasm / Chromium / Android / iOS</span>
        </div>
      </section>`,
  },
  {
    id: "02-labs-roadmap-2026-2027",
    file: "02-labs-roadmap-2026-2027.html",
    stitchId: "stitch-labs-roadmap",
    surface: "labs",
    deviceType: "DESKTOP",
    title: "Aphrody Labs Roadmap (2026-Q4 to 2027-Q4)",
    subtitle: "Four-Horizon Execution Plan: Pure-Rust Desktop OS, Google Cloud/Workspace, YouTube, Media & Android 17/18 M3E",
    defaultTheme: "neural",
    gpuiSurface: "aphrody_ui::RoadmapHorizonCard",
    nshComponent: "<RoadmapTimelineScreen />",
    keyPatterns: ["H0-H3 horizon cards", "6 release channel color chips", "Pure-Rust Google SDK parity"],
    slots: ["stitch-roadmap-horizons", "stitch-roadmap-channels"],
    renderBody: () => `
      <section class="mq-hero" data-stitch-id="stitch-roadmap-channels">
        <div class="mq-pill-eyebrow">Multi-Horizon Strategy &middot; H0 (Completed) &rarr; H3 (Pure-Rust Desktop OS 1.0)</div>
        <h1 class="mq-display-title">2026&ndash;2027 Platform &amp; Product Roadmap</h1>
        <div class="mq-chip-row">
          <span class="mq-chip mq-chip-done">LTS &bull; Sky</span>
          <span class="mq-chip mq-chip-active">Stable &bull; Ice</span>
          <span class="mq-chip">Preview &bull; Violet</span>
          <span class="mq-chip">Canary &bull; Magenta</span>
          <span class="mq-chip mq-chip-bolt">Nightly &bull; Amber</span>
          <span class="mq-chip">Dev &bull; Teal</span>
        </div>
      </section>
      <section class="mq-grid-4" data-stitch-id="stitch-roadmap-horizons">
        <article class="mq-card">
          <span class="mq-badge mq-badge-ok">H0 &middot; 2026-Q4 &middot; COMPLETED</span>
          <h2>Unified Engine &amp; Fork Packages</h2>
          <p>Upstream Zed merge, WGPU v30 + Wasm Core 2.0 AOT, 11 Cargo forks published to GHCR &amp; Releases (0 blocking submodules), material-colors 0.5 Liquid Glass.</p>
        </article>
        <article class="mq-card mq-card-focus">
          <span class="mq-badge mq-badge-primary">H1 &middot; 2027-Q1 &middot; ACTIVE</span>
          <h2>Google Workspace, Cloud AI &amp; Media Studio</h2>
          <p>Material 3 Expressive (Android 17/18), crates/aphrody_google (OAuth/gcloud, Workspace, Vertex/Gemini, YouTube &amp; YouTube Music headless+GPUI), crates/aphrody_media &amp; 3D WebGPU.</p>
        </article>
        <article class="mq-card">
          <span class="mq-badge">H2 &middot; 2027-Q2 &middot; PLANNED</span>
          <h2>Android 17/18 &amp; iOS Mobile GA</h2>
          <p>Android 17/18 Vulkan M3 Expressive (.aar) &amp; iOS Metal (.xcframework), gapless YouTube Music, Google Workspace quick actions, NshTrust OTA bundles.</p>
        </article>
        <article class="mq-card">
          <span class="mq-badge">H3 &middot; 2027-Q3..Q4 &middot; PLANNED</span>
          <h2>Pure-Rust Desktop OS 1.0</h2>
          <p>Bootable Btrfs/LUKS2 Wayland Linux ISO, first-class Windows DX12 &amp; Chromium WebGPU desktop parity, Gemini Live + A2A 1.0 + A2UI pilotage, multiplayer 3D &amp; NLE.</p>
        </article>
      </section>`,
  },
  {
    id: "03-web-landing-hero",
    file: "03-web-landing-hero.html",
    stitchId: "stitch-web-landing",
    surface: "website",
    deviceType: "DESKTOP",
    title: "aphrody.com — Website Landing & Live WebGPU Playground",
    subtitle: "Neural Expressive Marketing Hero, 120 FPS Playground & Benchmark Matrix",
    defaultTheme: "neural",
    gpuiSurface: "aphrody_desktop::examples::web_os",
    nshComponent: "<WebLandingHero />",
    keyPatterns: ["Centered serif/sans hero", "Interactive WebGPU shader preview card", "Submodule parity strip"],
    slots: ["stitch-web-hero-cta", "stitch-web-gpu-preview", "stitch-web-parity"],
    renderBody: () => `
      <section class="mq-hero mq-neural-aura mq-centered" data-stitch-id="stitch-web-hero-cta">
        <span class="mq-pill-eyebrow"><img src="../svg/gemini-neural-spark.svg" width="16" height="16" alt="" /> GPUI 120 FPS &bull; WGPU v30 &bull; 100% Zed Compatible</span>
        <h1 class="mq-display-hero">Build GPU-Native Software Across Every Surface</h1>
        <p class="mq-lead">One declarative Rust + NSH TSX codebase compiled to Desktop, iOS Metal, Android Vulkan, and zero-overhead Wasm + WebGPU &mdash; served from Cloudflare Workers at $0 Vercel cost.</p>
        <div class="mq-cta-group">
          <a class="gd-pill-button mq-btn-primary" href="./05-web-store-and-downloads.html">Download Aphrody Studio</a>
          <a class="gd-pill-button mq-btn-secondary" href="./04-web-docs-llms-portal.html">Read Docs &amp; llms.txt</a>
        </div>
      </section>
      <section class="mq-grid-2" data-stitch-id="stitch-web-gpu-preview">
        <div class="mq-card">
          <div class="mq-card-header"><h2>Live Wasm Core 2.0 + WebGPU Canvas</h2><span class="mq-badge">64px Rounded Preview</span></div>
          <div class="mq-shader-stage">
            <img src="../svg/aphrody-webgpu-prism.svg" width="56" height="56" alt="WebGPU Compute Prism" />
            <div>
              <strong>WGSL Naga v30 Pipeline</strong>
              <p>Frame Budget &le; 8.3ms (120Hz) &middot; Wasm Critical &le; 96 KB Brotli &middot; Offline IndexedDB Shader Cache</p>
            </div>
          </div>
        </div>
        <div class="mq-card" data-stitch-id="stitch-web-parity">
          <h2>Production Web Vitals &amp; Agent Readiness</h2>
          <div class="mq-kpi-row">
            <article class="mq-kpi-card"><span class="mq-kpi-label">LCP Budget</span><strong class="mq-kpi-value gd-tabular">&le; 1.8s</strong></article>
            <article class="mq-kpi-card"><span class="mq-kpi-label">INP Latency</span><strong class="mq-kpi-value gd-tabular">&le; 60ms</strong></article>
            <article class="mq-kpi-card"><span class="mq-kpi-label">CLS Score</span><strong class="mq-kpi-value gd-tabular">0.00</strong></article>
          </div>
        </div>
      </section>`,
  },
  {
    id: "04-web-docs-llms-portal",
    file: "04-web-docs-llms-portal.html",
    stitchId: "stitch-web-docs",
    surface: "website",
    deviceType: "DESKTOP",
    title: "docs.aphrody.com — Fumadocs v16, Twoslash & llms.txt Portal",
    subtitle: "Human & AI-Agent Dual-Negotiated Documentation (HTML + Accept: text/markdown)",
    defaultTheme: "dark",
    gpuiSurface: "aphrody_nsh::StudioMode::GpuiStarterPack",
    nshComponent: "<DocsPortalScreen />",
    keyPatterns: ["3-column docs reader", "Google Sans Code Twoslash block", "llms.txt & Agent Skills sidebar"],
    slots: ["stitch-docs-sidebar", "stitch-docs-reader", "stitch-docs-agent-panel"],
    renderBody: () => `
      <div class="mq-shell-3col">
        <aside class="mq-sidebar" aria-label="Documentation Navigation" data-stitch-id="stitch-docs-sidebar">
          <h2>Docs Modules</h2>
          <a class="mq-side-item mq-side-active" href="#getting-started">1. Google Design Tokens</a>
          <a class="mq-side-item" href="#gpui">2. GPUI Elements &amp; Springs</a>
          <a class="mq-side-item" href="#nsh">3. NSH TSX &amp; Mobile Embed</a>
          <a class="mq-side-item" href="#wgpu">4. WGPU v30 &amp; Wasm LTO</a>
          <a class="mq-side-item" href="#skills">5. /.well-known/agent-skills</a>
        </aside>
        <section class="mq-card" data-stitch-id="stitch-docs-reader">
          <div class="mq-card-header"><h1>Google Design &amp; Spring Physics API</h1><span class="mq-badge">Accept: text/markdown</span></div>
          <p>Documentation pages serve hydrated Fumadocs MDX + Twoslash to browsers and lossless Markdown mirrors to AI agents via Cloudflare Edge content negotiation.</p>
          <pre class="mq-code-block"><code>use aphrody_ui::google_design::{GoogleDesignTokens, GoogleSansFlexAxes};
let tokens = GoogleDesignTokens::bundled()?;
let spring = tokens.spring("spatial_fast"); // k=1400, zeta=0.90</code></pre>
        </section>
        <aside class="mq-card" aria-label="Agent Feeds" data-stitch-id="stitch-docs-agent-panel">
          <h2>AI Discovery Endpoints</h2>
          <ul class="mq-list">
            <li><code>/llms.txt</code> (v2 index)</li>
            <li><code>/docs/aphrody/wgpu/llms.txt</code></li>
            <li><code>/openapi.json</code> (Scalar 3.1)</li>
            <li><code>/.well-known/agent-skills/index.json</code></li>
          </ul>
        </aside>
      </div>`,
  },
  {
    id: "05-web-store-and-downloads",
    file: "05-web-store-and-downloads.html",
    stitchId: "stitch-web-store",
    surface: "website",
    deviceType: "DESKTOP",
    title: "releases.aphrody.com & Store — 22-Target Distribution Portal",
    subtitle: "Cloudflare R2 Zero-Egress Downloads, Sigstore Provenance & Awesome GPUI Store",
    defaultTheme: "neural",
    gpuiSurface: "aphrody_store::AwesomeGpuiCatalog",
    nshComponent: "<StoreDownloadPortalScreen />",
    keyPatterns: ["OS auto-detect CTA", "6 channel pill bar", "Desktop + Mobile + 22-target grid"],
    slots: ["stitch-store-hero", "stitch-store-platforms", "stitch-store-targets"],
    renderBody: () => `
      <section class="mq-hero" data-stitch-id="stitch-store-hero">
        <div class="mq-pill-eyebrow">Zero-Egress Cloudflare R2 &bull; Sigstore Cosign &amp; SHA-256 Attested</div>
        <h1 class="mq-display-title">Download Aphrody Across Desktop &amp; Mobile</h1>
        <div class="mq-chip-row">
          <span class="mq-chip mq-chip-active">Stable</span>
          <span class="mq-chip">Preview</span>
          <span class="mq-chip">Canary</span>
          <span class="mq-chip">Nightly</span>
          <span class="mq-chip">LTS</span>
          <span class="mq-chip">Dev</span>
        </div>
      </section>
      <section class="mq-grid-3" data-stitch-id="stitch-store-platforms">
        <article class="mq-card"><h2>Windows &amp; Linux</h2><p>Aphrody-Setup-x64.exe &middot; winget, msstore, scoop, choco, deb, rpm, flatpak, nix, arch AUR, AppImage</p></article>
        <article class="mq-card"><h2>macOS Apple Silicon</h2><p>Aphrody-arm64.dmg &middot; Metal 120Hz GPUI + Homebrew cask &amp; notarized .app bundle</p></article>
        <article class="mq-card"><h2>iOS &amp; Android Mobile</h2><p>App Store / TestFlight (.xcframework) &middot; Play Store / Signed APK (Vulkan NSH .aar)</p></article>
      </section>
      <section class="mq-card" data-stitch-id="stitch-store-targets">
        <h2>Awesome GPUI &amp; Extension Catalog (1,590 Zed Extensions)</h2>
        <pre class="mq-code-block"><code>$Env:APHRODY_CHANNEL = "stable"
irm https://releases.aphrody.com/install.ps1 | iex
gh attestation verify Aphrody-Setup-x64.exe --owner aphrody-dev</code></pre>
      </section>`,
  },
  {
    id: "06-desktop-aphrody-ide",
    file: "06-desktop-aphrody-ide.html",
    stitchId: "stitch-desktop-ide",
    surface: "desktop",
    deviceType: "DESKTOP",
    title: "Desktop: Aphrody Studio IDE (`crates/zed` + Stitch Dock)",
    subtitle: "52px Action Rail, Project Tree, Google Sans Code Editor & Live Stitch Inspector Dock",
    defaultTheme: "dark",
    gpuiSurface: "crates/zed + aphrody_nsh::DesignStitchStudioItem",
    nshComponent: "<AphrodyIdeWorkspace />",
    keyPatterns: ["Zed-compatible multi-pane workspace", "Google Sans Code + ligatures", "Stitch DESIGN.md inspector dock"],
    slots: ["stitch-ide-rail", "stitch-ide-editor", "stitch-ide-stitch-dock"],
    renderBody: () => `
      <div class="mq-ide-frame">
        <aside class="mq-icon-rail" aria-label="IDE Activity Rail" data-stitch-id="stitch-ide-rail">
          <img src="../svg/aphrody-monogram-neural.svg" width="26" height="26" alt="Aphrody" />
          <img src="../svg/aphrody-ide-studio.svg" width="22" height="22" alt="Explorer" />
          <img src="../svg/figcraft-stitch-frame.svg" width="22" height="22" alt="Stitch Studio" />
          <img src="../svg/aphrody-ai-agent-orbit.svg" width="22" height="22" alt="Agent" />
        </aside>
        <section class="mq-card" data-stitch-id="stitch-ide-editor">
          <div class="mq-tab-strip"><span class="mq-tab mq-tab-active">google_design.rs</span><span class="mq-tab">stitch_client.rs</span><span class="mq-tab">App.tsx</span></div>
          <pre class="mq-code-block"><code>pub fn capture_screen(doc: &amp;DesignMdDocument, id: &amp;str) -&gt; anyhow::Result&lt;()&gt; {
    let screen = StitchClient::default().synthesize_screen(id, "DESKTOP", doc, false)?;
    capture_stitch_screen_png(doc, &amp;screen, 1440, 900, Path::new("target/design-maquettes/out.png"))
}</code></pre>
        </section>
        <aside class="mq-card" aria-label="Stitch Inspector" data-stitch-id="stitch-ide-stitch-dock">
          <h2>Stitch &amp; Figma Inspector</h2>
          <p>Source: <code>DESIGN.md</code></p>
          <p>WCAG Contrast: <strong class="gd-tabular">7.92:1</strong></p>
          <p>Spring: <code>spatial_fast (1400, 0.9)</code></p>
          <a class="gd-pill-button mq-btn-primary" href="./07-desktop-aphrody-os-cosmic.html">Open in aphrody-os</a>
        </aside>
      </div>`,
  },
  {
    id: "07-desktop-aphrody-os-cosmic",
    file: "07-desktop-aphrody-os-cosmic.html",
    stitchId: "stitch-desktop-cosmic-os",
    surface: "desktop",
    deviceType: "DESKTOP",
    title: "Desktop: Aphrody Pure-Rust OS (Wayland, Windows, Wasm & Chromium)",
    subtitle: "Material 3 Expressive (Android 17/18) Liquid Glass Shell, Google Workspace, YouTube/Music & Media Studio",
    defaultTheme: "neural",
    gpuiSurface: "aphrody_desktop::OsDesktopShell",
    nshComponent: "<AphrodyOsDesktopShell />",
    keyPatterns: ["44px Material 3 Expressive TopPanel", "Google Workspace + YouTube/Music + 3D WebGPU stage", "64px Liquid Glass floating DockBar"],
    slots: ["stitch-os-toppanel", "stitch-os-stage", "stitch-os-dock"],
    renderBody: () => `
      <section class="mq-os-panel" data-stitch-id="stitch-os-toppanel">
        <span>Workspaces: [1 IDE] [2 Workspace] [3 Media &amp; YouTube] [4 Browser]</span>
        <strong class="gd-tabular">10:42 &middot; Wayland / DX12 / WebGPU 120Hz</strong>
        <span>gcloud OAuth ON &bull; WireGuard: FR-PAR-01</span>
      </section>
      <section class="mq-grid-2" data-stitch-id="stitch-os-stage">
        <article class="mq-card">
          <div class="mq-card-header"><h2>Google Workspace &amp; Cloud AI (Pure Rust)</h2><span class="mq-badge">gcloud / OAuth 2.0</span></div>
          <p>Drive, Gmail, Calendar, Docs, Sheets via Discovery Service + Gemini/Vertex AI, Imagen, Veo, Lyria, STT v2 &amp; TTS over <code>aphrody_google</code>.</p>
          <img src="../svg/parity-split-compare.svg" width="32" height="32" alt="Pure-Rust Google Workspace" />
        </article>
        <article class="mq-card">
          <div class="mq-card-header"><h2>YouTube, YouTube Music &amp; Media Studio</h2><span class="mq-badge">WGPU 30 + Zero FFmpeg</span></div>
          <p>Gapless Opus/AAC playback, dynamic HCT artwork palette, WGSL YUV-&gt;RGB scopes, TSX motion, and <code>bevy_gpui</code> 3D Gaussian Splatting.</p>
          <img src="../svg/bevy-unreal-solari.svg" width="32" height="32" alt="Media Studio &amp; 3D Viewport" />
        </article>
      </section>
      <nav class="mq-os-dock" aria-label="Aphrody Desktop Dock" data-stitch-id="stitch-os-dock">
        <img src="../svg/aphrody-monogram-neural.svg" width="36" height="36" alt="Launcher" />
        <img src="../svg/aphrody-ide-studio.svg" width="32" height="32" alt="Aphrody IDE" />
        <img src="../svg/figcraft-stitch-frame.svg" width="32" height="32" alt="Google Workspace" />
        <img src="../svg/craft-anything-hex.svg" width="32" height="32" alt="Media Studio" />
        <img src="../svg/aphrody-vpn-shield.svg" width="32" height="32" alt="Mullvad VPN" />
      </nav>`,
  },
  {
    id: "08-desktop-agy-neural-session",
    file: "08-desktop-agy-neural-session.html",
    stitchId: "stitch-desktop-neural-agy",
    surface: "desktop",
    deviceType: "DESKTOP",
    title: "Desktop: Neural Expressive Home, Headless Agents & Grounding Stack",
    subtitle: "260px Sidebar, Gemini/Vertex Omnibar, ADK/A2A/MCP Headless Controls & 28px Grounding Stack",
    defaultTheme: "neural",
    gpuiSurface: "aphrody_ui::google_design_components",
    nshComponent: "<NeuralExpressiveSessionScreen />",
    keyPatterns: ["gemini-home 260px rail", "ADK-Rust + A2A 1.0 + A2UI streaming", "win11-chrome157 28px grounding stack"],
    slots: ["stitch-neural-rail", "stitch-neural-omnibar", "stitch-neural-grounding"],
    renderBody: () => `
      <div class="mq-shell-3col">
        <aside class="mq-sidebar" aria-label="Sessions Sidebar" data-stitch-id="stitch-neural-rail">
          <a class="gd-pill-button mq-btn-primary" href="#new-session">+ New Session</a>
          <div class="mq-side-item mq-side-active">Google Workspace &amp; Cloud AI</div>
          <div class="mq-side-item">YouTube &amp; Music Headless Agent</div>
          <div class="mq-side-item">Media Watch FTS5 &amp; Render</div>
          <div class="mq-side-item">Android 17/18 M3 Expressive</div>
        </aside>
        <section class="mq-card mq-neural-aura mq-centered" data-stitch-id="stitch-neural-omnibar">
          <div class="mq-thinking-aura"><img src="../svg/gemini-neural-spark.svg" width="16" height="16" alt="" /> Vertex AI + Gemini Live &middot; gcloud Auth</div>
          <h1 class="mq-display-title">Where should we start?</h1>
          <div class="mq-omnibar-pill">
            <span class="mq-plus-btn" aria-label="Attachment Menu">+</span>
            <span>Query Drive/Gmail, play YouTube Music, analyze video, or run gpui-wgpu...</span>
            <span class="mq-badge">Ctrl+K</span>
          </div>
          <div class="mq-popover-card" aria-label="Cascading Attachment Popover">
            <strong>More Tools &amp; Uploads (20px Radius)</strong>
            <span>Google Workspace &bull; YouTube Watch &bull; Imagen / Veo &bull; Stitch Canvas</span>
          </div>
        </section>
        <aside class="mq-grounding-stack" aria-label="Grounded Sources" data-stitch-id="stitch-neural-grounding">
          <h2>Grounded Context (28px Stack)</h2>
          <article class="mq-ground-item"><span class="mq-badge">design.google</span><h3>Material 3 Expressive (Android 17/18)</h3><p>Semantic 3-tier tokens shared across Wayland, Windows, Wasm, Chromium &amp; Mobile.</p></article>
          <article class="mq-ground-item"><span class="mq-badge">cloud.google</span><h3>Official Google Cloud in Pure Rust</h3><p>Kotlin, Go, Python &amp; TS SDK parity via google-cloud-rust, ADK, A2A 1.0 &amp; MCP.</p></article>
          <article class="mq-ground-item"><span class="mq-badge">stitch.withgoogle.com</span><h3>Stitch + A2UI Slot Binding</h3><p>Zero-drift DOM/GPUI slot binding for deterministic regeneration.</p></article>
        </aside>
      </div>`,
  },
  {
    id: "09-desktop-browser-vpn-shield",
    file: "09-desktop-browser-vpn-shield.html",
    stitchId: "stitch-desktop-browser-vpn",
    surface: "desktop",
    deviceType: "DESKTOP",
    title: "Desktop: Aphrody Browser + Per-Tab Mullvad WireGuard VPN",
    subtitle: "Chromium //ui/views in GPUI, WebGPU Wasm Host & Per-Tab Tunnel Routing",
    defaultTheme: "dark",
    gpuiSurface: "aphrody_browser::ChromiumBrowserView + aphrody_vpn",
    nshComponent: "<BrowserVpnShell />",
    keyPatterns: ["Per-tab WireGuard exit pill", "Kitesurf/Servo/Blink escalation indicator", "agent-browser CDP SSRF guard"],
    slots: ["stitch-browser-tabs", "stitch-browser-vpn-bar", "stitch-browser-viewport"],
    renderBody: () => `
      <section class="mq-card" data-stitch-id="stitch-browser-tabs">
        <div class="mq-tab-strip">
          <span class="mq-tab mq-tab-active">design.google [FR-PAR WireGuard]</span>
          <span class="mq-tab">music.youtube.com [GPUI Native]</span>
          <span class="mq-tab">fr1-xfs.internal [WG LAN Direct]</span>
        </div>
        <div class="mq-omnibar-pill" data-stitch-id="stitch-browser-vpn-bar">
          <img src="../svg/aphrody-vpn-shield.svg" width="20" height="20" alt="VPN Active" />
          <span>https://design.google/?home &mdash; WebGPU Escalation: Native GPUI Canvas</span>
          <span class="mq-badge">SSRF Guard ON</span>
        </div>
      </section>
      <section class="mq-grid-2" data-stitch-id="stitch-browser-viewport">
        <article class="mq-card"><h2>Mullvad Tunnel Split Routing</h2><p>Per-tab isolation routes workstation LAN builds to <code>fr1/fr2</code> directly while pinning external research tabs through encrypted WireGuard hops.</p></article>
        <article class="mq-card"><h2>MCP Browser Relay &amp; Computer Use</h2><p>Deterministic AccessKit + ARIA tree inspection, DOM WebGPU checks, and headless PNG verification built directly into <code>aphrody_pilot</code>.</p></article>
      </section>`,
  },
  {
    id: "10-mobile-companion-home",
    file: "10-mobile-companion-home.html",
    stitchId: "stitch-mobile-home",
    surface: "mobile",
    deviceType: "MOBILE",
    title: "Mobile (Android 17/18 412px & iOS 390px): Companion Home & YouTube Music",
    subtitle: "Material 3 Expressive Safe-Area Telemetry, Compact Cards, Predictive Back & 64px Bottom Nav",
    defaultTheme: "neural",
    gpuiSurface: "nsh-platform-android + gpui_ios (examples/nsh/mobile-companion)",
    nshComponent: "<MobileCompanionApp />",
    keyPatterns: ["Android 17/18 M3 Expressive + iOS frames", "44x44pt minimum touch targets", "64px pill BottomNavBar"],
    slots: ["stitch-mobile-status", "stitch-mobile-cards", "stitch-mobile-bottom-nav"],
    renderBody: () => `
      <div class="mq-device-showcase">
        <section class="mq-phone-frame" aria-label="Android 17/18 Companion Maquette" data-stitch-id="stitch-mobile-status">
          <div class="mq-phone-notch">9:41 &middot; Android 17/18 Vulkan 120Hz</div>
          <div class="mq-phone-content" data-stitch-id="stitch-mobile-cards">
            <div class="mq-card">
              <div class="mq-card-header"><strong>Aphrody Companion</strong><span class="mq-badge">gcloud + WG ON</span></div>
              <p>fr1 XFS Build: <code>cargo check -p aphrody_google</code> green</p>
            </div>
            <div class="mq-card">
              <strong>YouTube Music &middot; Gapless Opus</strong>
              <p>Dynamic Material 3 Expressive HCT palette active with synced lyrics.</p>
            </div>
            <div class="mq-omnibar-pill"><span>+</span><span>Ask Gemini Live, search Drive, or dispatch fr1 job...</span></div>
          </div>
          <nav class="mq-bottom-nav" aria-label="Mobile Navigation" data-stitch-id="stitch-mobile-bottom-nav">
            <a class="mq-bottom-tab mq-bottom-active" href="./10-mobile-companion-home.html">Agent</a>
            <a class="mq-bottom-tab" href="./11-mobile-code-diff-review.html">Diffs</a>
            <a class="mq-bottom-tab" href="./12-mobile-pocket-craft-vpn.html">Media</a>
            <a class="mq-bottom-tab" href="./12-mobile-pocket-craft-vpn.html">Cloud</a>
          </nav>
        </section>
      </div>`,
  },
  {
    id: "11-mobile-code-diff-review",
    file: "11-mobile-code-diff-review.html",
    stitchId: "stitch-mobile-diff",
    surface: "mobile",
    deviceType: "MOBILE",
    title: "Mobile: Code Diff Review & Snap-Point Bottom Sheet",
    subtitle: "Tree-sitter Syntax Hunks, One-Thumb Approve CTA & Bottom Terminal Sheet",
    defaultTheme: "dark",
    gpuiSurface: "nsh_base::DiffView + Sheet",
    nshComponent: "<MobileDiffReviewScreen />",
    keyPatterns: ["Compact monospace code wrapping", "44px approve/request changes buttons", "NshTrust OTA status"],
    slots: ["stitch-mobile-diff-header", "stitch-mobile-diff-hunk", "stitch-mobile-diff-actions"],
    renderBody: () => `
      <div class="mq-device-showcase">
        <section class="mq-phone-frame" aria-label="Mobile Diff Review Screen" data-stitch-id="stitch-mobile-diff-header">
          <div class="mq-phone-notch">Diff &middot; google_design.rs</div>
          <div class="mq-phone-content" data-stitch-id="stitch-mobile-diff-hunk">
            <pre class="mq-code-block"><code>+ pub neural_obsidian: BTreeMap&lt;String, String&gt;,
+ pub oled_pitch: BTreeMap&lt;String, String&gt;,
  // WCAG 2.2 AA &gt;= 4.5:1 verified</code></pre>
            <div class="mq-cta-group" data-stitch-id="stitch-mobile-diff-actions">
              <button type="button" class="gd-pill-button mq-btn-primary">Approve &amp; Merge</button>
              <button type="button" class="gd-pill-button mq-btn-secondary">Run Tests</button>
            </div>
          </div>
          <nav class="mq-bottom-nav" aria-label="Mobile Navigation">
            <a class="mq-bottom-tab" href="./10-mobile-companion-home.html">Agent</a>
            <a class="mq-bottom-tab mq-bottom-active" href="./11-mobile-code-diff-review.html">Diffs</a>
            <a class="mq-bottom-tab" href="./12-mobile-pocket-craft-vpn.html">Media</a>
            <a class="mq-bottom-tab" href="./12-mobile-pocket-craft-vpn.html">Cloud</a>
          </nav>
        </section>
      </div>`,
  },
  {
    id: "12-mobile-pocket-craft-vpn",
    file: "12-mobile-pocket-craft-vpn.html",
    stitchId: "stitch-mobile-craft-oled",
    surface: "mobile",
    deviceType: "TABLET",
    title: "Mobile / Tablet OLED Pitch: Pocket Studio, VPN & Foldable 834px View",
    subtitle: "True-Black Android 17/18 OLED Mode, Ed25519 NshTrust OTA & WireGuard Quick-Tile",
    defaultTheme: "oled",
    gpuiSurface: "nsh-platform-android + nsh-platform-ios OLED",
    nshComponent: "<MobilePocketStudioScreen />",
    keyPatterns: ["#000000 OLED battery saver", "Medium 600-839px foldable 2-pane", "Ed25519 NshTrust OTA badge"],
    slots: ["stitch-mobile-oled-vpn", "stitch-mobile-oled-ota"],
    renderBody: () => `
      <section class="mq-grid-2" data-stitch-id="stitch-mobile-oled-vpn">
        <article class="mq-card">
          <div class="mq-card-header"><h1>Aphrody VPN &amp; Cloud Quick-Tile (OLED)</h1><span class="mq-badge">0% Gray Bleed</span></div>
          <p>Uses true <code>#000000</code> surface planes with <code>#8FD0FF</code> active controls and 44x44pt Material 3 Expressive hit targets.</p>
          <img src="../svg/aphrody-mobile-adaptive-fg.svg" width="52" height="52" alt="Adaptive App Icon" />
        </article>
        <article class="mq-card" data-stitch-id="stitch-mobile-oled-ota">
          <h2>Ed25519 NshTrust Signed OTA Bundles</h2>
          <p>Every mobile and Wasm bundle verifies Ed25519 signatures off the UI thread before hot-swapping screens in <code>NshAppView</code> / <code>NshView</code>.</p>
        </article>
      </section>`,
  },
];

export function renderMaquetteIndexHtml(defs: MaquetteDefinition[]): string {
  const bodyHtml = `
    <section class="mq-hero mq-neural-aura" data-stitch-id="stitch-index-hero">
      <div class="mq-pill-eyebrow">Aphrody Labs Design System v3.0 &middot; Figma DTCG + Google Stitch + GPUI</div>
      <h1 class="mq-display-hero">Unified Design System &amp; Interactive Maquettes</h1>
      <p class="mq-lead">Inspect live Light, Dark, Neural Obsidian, and OLED Pitch tokens, Google Sans Flex 6-axis optical typography, 20 accessible SVGs, and all 12 Project, Website, Desktop, and Mobile maquettes.</p>
      <div class="mq-chip-row" role="group" aria-label="Surface Filter">
        <button type="button" class="mq-chip mq-chip-active" data-filter-surface="all">All (12)</button>
        <button type="button" class="mq-chip" data-filter-surface="labs">Aphrody Labs (2)</button>
        <button type="button" class="mq-chip" data-filter-surface="website">Website (3)</button>
        <button type="button" class="mq-chip" data-filter-surface="desktop">Desktop (4)</button>
        <button type="button" class="mq-chip" data-filter-surface="mobile">Mobile (3)</button>
      </div>
    </section>
    <section class="mq-card" data-stitch-id="stitch-index-typography">
      <div class="mq-card-header"><h2>Google Sans Flex Interactive 6-Axis Laboratory</h2><span class="mq-badge" id="axes-readout">"opsz" 48, "wght" 500, "ROND" 40</span></div>
      <div class="mq-axis-controls">
        <label>wght <input type="range" id="axis-wght" min="200" max="800" value="500" /></label>
        <label>opsz <input type="range" id="axis-opsz" min="10" max="72" value="48" /></label>
        <label>wdth <input type="range" id="axis-wdth" min="75" max="125" value="100" /></label>
        <label>ROND <input type="range" id="axis-rond" min="0" max="100" value="40" /></label>
      </div>
      <p id="axes-sample" class="mq-display-title">Aphrody Studio &mdash; Code Is a Design Material (0123456789)</p>
    </section>
    <section class="mq-grid-3" id="maquette-cards-grid" data-stitch-id="stitch-index-gallery">
      ${defs
        .map(
          (d) => `<article class="mq-card mq-gallery-card" data-card-surface="${d.surface}">
        <div class="mq-card-header"><span class="mq-badge">${escapeHtml(d.surface.toUpperCase())} &middot; ${d.deviceType}</span><span class="mq-badge">${escapeHtml(d.defaultTheme)}</span></div>
        <h2><a href="./${d.file}">${escapeHtml(d.title)}</a></h2>
        <p>${escapeHtml(d.subtitle)}</p>
        <p class="mq-meta-line"><code>${escapeHtml(d.gpuiSurface)}</code></p>
        <a class="gd-pill-button mq-btn-secondary" href="./${d.file}">Open Maquette &rarr;</a>
      </article>`,
        )
        .join("\n      ")}
    </section>`;

  const extraScript = `
    document.querySelectorAll("[data-filter-surface]").forEach((btn) => {
      btn.addEventListener("click", () => {
        const target = btn.getAttribute("data-filter-surface") || "all";
        document.querySelectorAll("[data-filter-surface]").forEach((b) => b.classList.toggle("mq-chip-active", b === btn));
        document.querySelectorAll("[data-card-surface]").forEach((card) => {
          const s = card.getAttribute("data-card-surface");
          card.style.display = target === "all" || s === target ? "" : "none";
        });
      });
    });
    const sample = document.getElementById("axes-sample");
    const readout = document.getElementById("axes-readout");
    const wght = document.getElementById("axis-wght");
    const opsz = document.getElementById("axis-opsz");
    const wdth = document.getElementById("axis-wdth");
    const rond = document.getElementById("axis-rond");
    const updateAxes = () => {
      if (!sample || !readout || !wght || !opsz || !wdth || !rond) return;
      const val = '"opsz" ' + opsz.value + ', "wght" ' + wght.value + ', "wdth" ' + wdth.value + ', "ROND" ' + rond.value;
      sample.style.fontVariationSettings = val;
      readout.textContent = val;
    };
    [wght, opsz, wdth, rond].forEach((el) => el && el.addEventListener("input", updateAxes));
  `;

  return wrapHtmlDocument({
    id: "index",
    stitchId: "stitch-maquettes-index",
    surface: "labs",
    title: "Maquettes Explorer",
    badge: "v3.0.0",
    defaultTheme: "neural",
    bodyHtml,
    extraScript,
  });
}

export function buildAllMaquettes(designDocsDir: string, repoRoot: string): {
  maquettesDir: string;
  entries: Array<{
    id: string;
    file: string;
    path: string;
    stitchId: string;
    surface: MaquetteSurface;
    deviceType: MaquetteDeviceType;
    title: string;
    defaultTheme: string;
    bytes: number;
  }>;
} {
  const maquettesDir = join(designDocsDir, "maquettes");
  mkdirSync(maquettesDir, { recursive: true });

  const entries = MAQUETTE_DEFINITIONS.map((def) => {
    const html = wrapHtmlDocument({
      id: def.id,
      stitchId: def.stitchId,
      surface: def.surface,
      title: def.title,
      badge: `${def.surface.toUpperCase()} / ${def.deviceType}`,
      defaultTheme: def.defaultTheme,
      bodyHtml: def.renderBody(),
    });
    const outPath = join(maquettesDir, def.file);
    writeFileSync(outPath, html, "utf8");
    return {
      id: def.id,
      file: def.file,
      path: `./maquettes/${def.file}`,
      stitchId: def.stitchId,
      surface: def.surface,
      deviceType: def.deviceType,
      title: def.title,
      defaultTheme: def.defaultTheme,
      bytes: Buffer.byteLength(html, "utf8"),
    };
  });

  const indexHtml = renderMaquetteIndexHtml(MAQUETTE_DEFINITIONS);
  writeFileSync(join(maquettesDir, "index.html"), indexHtml, "utf8");
  void repoRoot;
  return { maquettesDir, entries };
}

function findBrowserExecutable(): string {
  const fromEnv = process.env.APHRODY_CHROME_BIN ?? process.env.CHROME_BIN;
  if (fromEnv && existsSync(fromEnv)) return fromEnv;
  const candidates =
    process.platform === "win32"
      ? [
          "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
          "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
        ]
      : [
          "/usr/bin/google-chrome",
          "/usr/bin/chromium",
          "/usr/bin/chromium-browser",
          "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        ];
  for (const c of candidates) {
    if (existsSync(c)) return c;
  }
  throw new Error("No Chromium/Chrome/Edge executable found; set APHRODY_CHROME_BIN or CHROME_BIN.");
}

export async function captureMaquetteScreenshots(repoRoot: string): Promise<{
  ok: boolean;
  outDir: string;
  shots: Array<{ name: string; path: string; bytes: number; primaryColor: string }>;
}> {
  const outDir = join(repoRoot, "target", "design-maquettes");
  mkdirSync(outDir, { recursive: true });

  const mimeMap: Record<string, string> = {
    ".html": "text/html; charset=utf-8",
    ".css": "text/css; charset=utf-8",
    ".svg": "image/svg+xml",
    ".png": "image/png",
    ".json": "application/json; charset=utf-8",
    ".ttf": "font/ttf",
  };

  const server = Bun.serve({
    hostname: "127.0.0.1",
    port: 0,
    async fetch(req) {
      const pathname = new URL(req.url).pathname;
      const rel = pathname === "/" ? "/docs/aphrody/design/maquettes/index.html" : pathname;
      const filePath = resolve(repoRoot, `.${rel}`);
      if (!filePath.startsWith(repoRoot) || !existsSync(filePath)) {
        return new Response("Not Found", { status: 404 });
      }
      return new Response(Bun.file(filePath), {
        headers: { "Content-Type": mimeMap[extname(filePath)] ?? "application/octet-stream" },
      });
    },
  });

  const targets: Array<{
    name: string;
    route: string;
    width: number;
    height: number;
    themeOverride?: string;
  }> = [
    { name: "maquettes-index-neural-1440x900.png", route: "/docs/aphrody/design/maquettes/index.html", width: 1440, height: 900, themeOverride: "neural" },
    { name: "maquettes-index-light-1440x900.png", route: "/docs/aphrody/design/maquettes/index.html", width: 1440, height: 900, themeOverride: "light" },
    { name: "web-landing-hero-1440x900.png", route: "/docs/aphrody/design/maquettes/03-web-landing-hero.html", width: 1440, height: 900 },
    { name: "desktop-aphrody-ide-1440x900.png", route: "/docs/aphrody/design/maquettes/06-desktop-aphrody-ide.html", width: 1440, height: 900 },
    { name: "desktop-aphrody-os-1440x900.png", route: "/docs/aphrody/design/maquettes/07-desktop-aphrody-os-cosmic.html", width: 1440, height: 900 },
    { name: "mobile-companion-home-390x844.png", route: "/docs/aphrody/design/maquettes/10-mobile-companion-home.html", width: 390, height: 844 },
    { name: "mobile-code-diff-390x844.png", route: "/docs/aphrody/design/maquettes/11-mobile-code-diff-review.html", width: 390, height: 844 },
  ];

  const profileDir = mkdtempSync(join(tmpdir(), "aphrody-maquettes-"));
  const chromePath = findBrowserExecutable();
  const proc = Bun.spawn(
    [
      chromePath,
      "--headless=new",
      "--remote-debugging-port=0",
      `--user-data-dir=${profileDir}`,
      "--window-size=1440,900",
      "--no-first-run",
      "--no-default-browser-check",
      `http://127.0.0.1:${server.port}/docs/aphrody/design/maquettes/index.html`,
    ],
    { stdout: "pipe", stderr: "pipe" },
  );

  const shots: Array<{ name: string; path: string; bytes: number; primaryColor: string }> = [];
  const exceptions: string[] = [];

  try {
    const reader = proc.stderr.getReader();
    let stderrBuf = "";
    let browserWs = "";
    const deadline = Date.now() + 10_000;
    while (Date.now() < deadline) {
      const { value, done } = await reader.read();
      if (done) break;
      stderrBuf += new TextDecoder().decode(value);
      const match = /DevTools listening on (ws:\/\/[^\s]+)/.exec(stderrBuf);
      if (match) {
        browserWs = match[1];
        break;
      }
    }
    if (!browserWs) throw new Error("Chrome DevTools WebSocket did not start");

    const baseHttp = browserWs.replace(/^ws:/, "http:").replace(/\/devtools\/browser\/.*$/, "");
    let pageWs = "";
    for (let i = 0; i < 30; i++) {
      const list = (await fetch(`${baseHttp}/json/list`).then((r: Response) => r.json())) as Array<{ type: string; webSocketDebuggerUrl: string }>;
      const page = list.find((t) => t.type === "page");
      if (page?.webSocketDebuggerUrl) {
        pageWs = page.webSocketDebuggerUrl;
        break;
      }
      await Bun.sleep(120);
    }
    if (!pageWs) throw new Error("No page target exposed by headless browser");

    const ws = new WebSocket(pageWs);
    await new Promise<void>((res, rej) => {
      ws.onopen = () => res();
      ws.onerror = (err: unknown) => rej(err);
    });

    let nextId = 1;
    const pending = new Map<number, { resolve: (v: any) => void; reject: (e: any) => void }>();
    ws.onmessage = (ev: { data: unknown }) => {
      const msg = JSON.parse(String(ev.data)) as { id?: number; method?: string; result?: unknown; error?: unknown; params?: any };
      if (msg.id && pending.has(msg.id)) {
        const p = pending.get(msg.id)!;
        pending.delete(msg.id);
        if (msg.error) p.reject(msg.error);
        else p.resolve(msg.result);
      } else if (msg.method === "Runtime.exceptionThrown") {
        exceptions.push(JSON.stringify(msg.params?.exceptionDetails ?? msg.params));
      }
    };

    const cdp = (method: string, params: Record<string, unknown> = {}) => {
      const id = nextId++;
      return new Promise<any>((res, rej) => {
        pending.set(id, { resolve: res, reject: rej });
        ws.send(JSON.stringify({ id, method, params }));
      });
    };

    await cdp("Page.enable");
    await cdp("Runtime.enable");
    await cdp("Emulation.setDeviceMetricsOverride", { width: 1440, height: 900, deviceScaleFactor: 1, mobile: false });

    for (const target of targets) {
      await cdp("Emulation.setDeviceMetricsOverride", {
        width: target.width,
        height: target.height,
        deviceScaleFactor: 1,
        mobile: target.width <= 480,
      });
      await cdp("Page.navigate", { url: `http://127.0.0.1:${server.port}${target.route}` });
      await Bun.sleep(450);
      if (target.themeOverride) {
        await cdp("Runtime.evaluate", {
          expression: `document.documentElement.setAttribute("data-theme", "${target.themeOverride}");`,
        });
        await Bun.sleep(150);
      }
      const computed = await cdp("Runtime.evaluate", {
        expression: `(() => ({
          primary: getComputedStyle(document.documentElement).getPropertyValue("--md-sys-color-primary").trim(),
          brokenImages: Array.from(document.querySelectorAll("img")).filter(i => !i.complete || i.naturalWidth === 0).map(i => i.getAttribute("src"))
        }))()`,
        returnByValue: true,
      });
      const primaryColor = String(computed?.result?.value?.primary ?? "");
      const brokenImages: string[] = Array.isArray(computed?.result?.value?.brokenImages)
        ? computed.result.value.brokenImages
        : [];
      if (brokenImages.length > 0) {
        exceptions.push(`${target.name} broken images: ${brokenImages.join(", ")}`);
      }
      const shot = await cdp("Page.captureScreenshot", { format: "png" });
      const buf = Buffer.from(shot.data, "base64");
      const outPath = join(outDir, target.name);
      writeFileSync(outPath, buf);
      shots.push({ name: target.name, path: outPath, bytes: buf.length, primaryColor });
    }

    ws.close();
  } finally {
    proc.kill();
    await proc.exited.catch(() => {});
    server.stop();
    rmSync(profileDir, { recursive: true, force: true });
  }

  const ok =
    exceptions.length === 0 &&
    shots.length === targets.length &&
    shots.every((s) => s.bytes > 1000 && s.primaryColor.length > 0 && readFileSync(s.path).subarray(0, 4).equals(Buffer.from([0x89, 0x50, 0x4e, 0x47])));
  return { ok, outDir, shots };
}
