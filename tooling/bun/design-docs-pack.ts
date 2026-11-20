#!/usr/bin/env bun
/**
 * Reproducible Google Design (`https://design.google/?home` + Neural Expressive Nov 2026)
 * Asset Optimizer, Stylesheet Compiler, W3C DTCG Figma & Stitch Kit Builder, MDX Guideline
 * Generator, and Typed Contract Validator for \`docs/aphrody/design/\`.
 *
 * Usage:
 *   bun tooling/bun/design-docs-pack.ts build
 *   bun tooling/bun/design-docs-pack.ts verify
 */
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { join, relative, resolve } from "node:path";
import { buildCanonicalDesignTokens, contrastRatioHex } from "./google-design-sync.ts";
import { buildAllMaquettes, captureMaquetteScreenshots, MAQUETTE_DEFINITIONS } from "./design-maquettes.ts";

export const repoRoot = resolve(import.meta.dir, "../..");
export const DESIGN_DOCS_DIR = join(repoRoot, "docs/aphrody/design");

function sha256Hex(content: Uint8Array | string): string {
  return createHash("sha256").update(content).digest("hex");
}

function sampleDampedSpringLinearCss(stiffness: number, dampingRatio: number, durationSec = 0.35): string {
  const steps = 16;
  const omega0 = Math.sqrt(stiffness);
  const zeta = Math.max(0.05, dampingRatio);
  const vals: string[] = [];
  for (let i = 0; i <= steps; i++) {
    const t = (i / steps) * durationSec;
    let y = 0;
    if (t <= 0) {
      y = 0;
    } else if (Math.abs(zeta - 1.0) < 1e-4) {
      const decay = Math.exp(-omega0 * t);
      y = 1 - decay * (1 + omega0 * t);
    } else if (zeta < 1.0) {
      const omegaD = omega0 * Math.sqrt(1 - zeta * zeta);
      const decay = Math.exp(-zeta * omega0 * t);
      const ratio = zeta / Math.sqrt(1 - zeta * zeta);
      y = 1 - decay * (Math.cos(omegaD * t) + ratio * Math.sin(omegaD * t));
    } else {
      y = 1;
    }
    vals.push(y.toFixed(4));
  }
  return `linear(${vals.join(", ")})`;
}

const CURATED_SVGS: Array<{ id: string; title: string; viewBox: string; body: string }> = [
  {
    id: "google-g-4color",
    title: "Google 4-Color Neural Mark",
    viewBox: "0 0 24 24",
    body: `<path fill="#4285F4" d="M23.49 12.27c0-.79-.07-1.54-.19-2.27H12v4.51h6.47c-.29 1.48-1.14 2.73-2.4 3.58v3h3.86c2.26-2.09 3.56-5.17 3.56-8.82Z"/><path fill="#34A853" d="M12 24c3.24 0 5.95-1.08 7.93-2.91l-3.86-3c-1.08.72-2.45 1.16-4.07 1.16-3.13 0-5.78-2.11-6.73-4.96H1.29v3.09C3.26 21.3 7.31 24 12 24Z"/><path fill="#FBBC05" d="M5.27 14.29c-.25-.72-.38-1.49-.38-2.29s.14-1.57.38-2.29V6.62H1.29a11.99 11.99 0 0 0 0 10.76l3.98-3.09Z"/><path fill="#EA4335" d="M12 4.75c1.77 0 3.35.61 4.6 1.8l3.42-3.42C17.95 1.19 15.24 0 12 0 7.31 0 3.26 2.7 1.29 6.62l3.98 3.09c.95-2.85 3.6-4.96 6.73-4.96Z"/>`,
  },
  {
    id: "gemini-neural-spark",
    title: "Gemini Neural Expressive Four-Point Spark",
    viewBox: "0 0 24 24",
    body: `<defs><linearGradient id="gemGrad" x1="0%" y1="0%" x2="100%" y2="100%"><stop offset="0%" stop-color="#4285F4"/><stop offset="50%" stop-color="#9B72CB"/><stop offset="100%" stop-color="#D96570"/></linearGradient></defs><path fill="url(#gemGrad)" d="M12 1.5c.6 5.2 4.7 9.3 9.9 9.9-5.2.6-9.3 4.7-9.9 9.9-.6-5.2-4.7-9.3-9.9-9.9 5.2-.6 9.3-4.7 9.9-9.9Z"/>`,
  },
  {
    id: "code-as-material",
    title: "Code is a Design Material Emblem",
    viewBox: "0 0 24 24",
    body: `<rect x="2" y="3" width="20" height="18" rx="4" fill="none" stroke="currentColor" stroke-width="1.75"/><path d="M8.5 9.5 5.5 12l3 2.5M15.5 9.5l3 2.5-3 2.5M13.2 8l-2.4 8" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"/>`,
  },
  {
    id: "figcraft-stitch-frame",
    title: "Google FigCraft & Stitch Auto-Layout Frame",
    viewBox: "0 0 24 24",
    body: `<rect x="3" y="3" width="8" height="8" rx="2.5" fill="none" stroke="currentColor" stroke-width="1.75"/><rect x="13" y="3" width="8" height="8" rx="4" fill="none" stroke="currentColor" stroke-width="1.75"/><rect x="3" y="13" width="18" height="8" rx="4" fill="none" stroke="currentColor" stroke-width="1.75"/>`,
  },
  {
    id: "craft-anything-hex",
    title: "Google Craft Anything Clean-Room Disassembly & Wasm",
    viewBox: "0 0 24 24",
    body: `<polygon points="12,2 20.66,7 20.66,17 12,22 3.34,17 3.34,7" fill="none" stroke="currentColor" stroke-width="1.75"/><circle cx="12" cy="12" r="3.2" fill="none" stroke="currentColor" stroke-width="1.75"/>`,
  },
  {
    id: "bevy-unreal-solari",
    title: "Unreal Engine 5.8 Bevy Solari GI & Nanite Meshlets",
    viewBox: "0 0 24 24",
    body: `<path d="M12 2.5 20.5 7.2v9.6L12 21.5 3.5 16.8V7.2L12 2.5Z" fill="none" stroke="currentColor" stroke-width="1.75"/><path d="M12 2.5v19M3.5 7.2 12 12l8.5-4.8" fill="none" stroke="currentColor" stroke-width="1.5"/>`,
  },
  {
    id: "parity-split-compare",
    title: "GPUI Google Design vs egui 0.37 Side-by-Side Split",
    viewBox: "0 0 24 24",
    body: `<rect x="2.5" y="4" width="19" height="16" rx="3" fill="none" stroke="currentColor" stroke-width="1.75"/><line x1="12" y1="4" x2="12" y2="20" stroke="currentColor" stroke-width="1.75"/>`,
  },
  {
    id: "omnibar-plus",
    title: "Neural Omnibar Attachment Trigger",
    viewBox: "0 0 24 24",
    body: `<path d="M12 5v14M5 12h14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>`,
  },
  {
    id: "aphrody-monogram-neural",
    title: "Aphrody Neural Expressive Monogram",
    viewBox: "0 0 96 96",
    body: `<defs><linearGradient id="aphTile" x1="0" y1="0" x2="96" y2="96" gradientUnits="userSpaceOnUse"><stop offset="0%" stop-color="#004B75"/><stop offset="100%" stop-color="#135B82"/></linearGradient></defs><rect width="96" height="96" rx="22" fill="url(#aphTile)"/><path d="M48 18 76 74H63.5L57.8 62H38.2L32.5 74H20L48 18Zm0 19-6.6 15.5h13.2L48 37Z" fill="#FFFFFF"/><path d="M56.5 18 42.5 47H54L39.5 76" fill="none" stroke="#FFB300" stroke-width="5.5" stroke-linecap="round" stroke-linejoin="round"/>`,
  },
  {
    id: "aphrody-monogram-mono",
    title: "Aphrody Monochrome Monogram",
    viewBox: "0 0 96 96",
    body: `<rect x="4" y="4" width="88" height="88" rx="20" fill="none" stroke="currentColor" stroke-width="5"/><path d="M48 20 73 72H61.5L56.5 61H39.5L34.5 72H23L48 20Zm0 18-5.5 14h11L48 38Z" fill="currentColor"/>`,
  },
  {
    id: "aphrody-mobile-adaptive-fg",
    title: "Aphrody Mobile Adaptive Icon Foreground (66dp Safe Zone)",
    viewBox: "0 0 108 108",
    body: `<path d="M54 24 79 77H67.5L62.5 66H45.5L40.5 77H29L54 24Zm0 17-5.8 14.5h11.6L54 41Z" fill="#FFFFFF"/><path d="M61 25 48 51h10L45 77" fill="none" stroke="#FFB300" stroke-width="4.8" stroke-linecap="round" stroke-linejoin="round"/>`,
  },
  {
    id: "aphrody-mobile-adaptive-bg",
    title: "Aphrody Mobile Adaptive Icon Background",
    viewBox: "0 0 108 108",
    body: `<defs><radialGradient id="mobBg" cx="50%" cy="35%" r="65%"><stop offset="0%" stop-color="#135B82"/><stop offset="100%" stop-color="#0B0E14"/></radialGradient></defs><rect width="108" height="108" fill="url(#mobBg)"/>`,
  },
  {
    id: "aphrody-os-compositor",
    title: "Aphrody COSMIC Epoch Compositor Tiling Glyph",
    viewBox: "0 0 24 24",
    body: `<rect x="2.5" y="3" width="19" height="18" rx="3.5" fill="none" stroke="currentColor" stroke-width="1.75"/><path d="M12 3v18M12 12h9.5" stroke="currentColor" stroke-width="1.75"/>`,
  },
  {
    id: "aphrody-ide-studio",
    title: "Aphrody Studio IDE Multi-Pane Glyph",
    viewBox: "0 0 24 24",
    body: `<rect x="3" y="3" width="18" height="18" rx="3" fill="none" stroke="currentColor" stroke-width="1.75"/><path d="M8.5 3v18M12 10l2.5 2-2.5 2M15.5 14h2.5" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"/>`,
  },
  {
    id: "aphrody-webgpu-prism",
    title: "WebGPU & WGSL Compute Prism",
    viewBox: "0 0 24 24",
    body: `<polygon points="12,2.5 20.5,7.5 20.5,16.5 12,21.5 3.5,16.5 3.5,7.5" fill="none" stroke="currentColor" stroke-width="1.75"/><polygon points="12,5 18.5,16.2 5.5,16.2" fill="none" stroke="currentColor" stroke-width="1.5"/>`,
  },
  {
    id: "aphrody-wasm-cube",
    title: "WebAssembly Core 2.0 & WASI P2 Cube",
    viewBox: "0 0 24 24",
    body: `<path d="M4 7v10a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2h-2.5a3.5 3.5 0 0 1-7 0H6a2 2 0 0 0-2 2Z" fill="none" stroke="currentColor" stroke-width="1.75"/><path d="M7.5 12.5 9 16l1.5-3.5L12 16l1.5-3.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>`,
  },
  {
    id: "aphrody-vpn-shield",
    title: "Aphrody Sovereign WireGuard VPN Shield",
    viewBox: "0 0 24 24",
    body: `<path d="M12 2.5 19.5 5.8v5.5c0 4.8-3.2 8.7-7.5 10.2-4.3-1.5-7.5-5.4-7.5-10.2V5.8L12 2.5Z" fill="none" stroke="currentColor" stroke-width="1.75"/><path d="m9 12 2.2 2.2L15.5 10" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"/>`,
  },
  {
    id: "aphrody-ai-agent-orbit",
    title: "Multi-Agent MCP / ACP / A2A Control Orbit",
    viewBox: "0 0 24 24",
    body: `<circle cx="12" cy="12" r="3" fill="none" stroke="currentColor" stroke-width="1.75"/><ellipse cx="12" cy="12" rx="9" ry="4" transform="rotate(30 12 12)" fill="none" stroke="currentColor" stroke-width="1.5"/><ellipse cx="12" cy="12" rx="9" ry="4" transform="rotate(-30 12 12)" fill="none" stroke="currentColor" stroke-width="1.5"/>`,
  },
  {
    id: "aphrody-figma-tokens",
    title: "W3C DTCG & Figma Variable Token Grid",
    viewBox: "0 0 24 24",
    body: `<circle cx="8" cy="7" r="3" fill="none" stroke="currentColor" stroke-width="1.75"/><rect x="13" y="4" width="6" height="6" rx="1.5" fill="none" stroke="currentColor" stroke-width="1.75"/><rect x="5" y="13" width="6" height="6" rx="1.5" fill="none" stroke="currentColor" stroke-width="1.75"/><circle cx="16" cy="16" r="3" fill="none" stroke="currentColor" stroke-width="1.75"/>`,
  },
  {
    id: "aphrody-store-22targets",
    title: "Aphrody 22-Target Store Distribution",
    viewBox: "0 0 24 24",
    body: `<path d="M4 9h16l-1.2 10.5a2 2 0 0 1-2 1.8H7.2a2 2 0 0 1-2-1.8L4 9Z" fill="none" stroke="currentColor" stroke-width="1.75"/><path d="M8.5 9V6.5a3.5 3.5 0 0 1 7 0V9" fill="none" stroke="currentColor" stroke-width="1.75"/>`,
  },
];

function normalizeSvgMarkup(raw: string, fallbackTitle: string): { clean: string; viewBox: string } {
  let s = raw.replace(/\r?\n/g, " ").replace(/\s+/g, " ").replace(/>\s+</g, "><").trim();
  s = s.replace(/&(?!amp;|lt;|gt;|quot;|apos;|#\d+;|#x[0-9a-fA-F]+;)/g, "&amp;");
  const vbMatch = s.match(/viewBox=["']([^"']+)["']/i);
  const viewBox = vbMatch?.[1] ?? "0 0 24 24";
  if (!/role=["']/i.test(s)) {
    s = s.replace(/^<svg\b/i, '<svg role="img"');
  }
  if (s.includes("currentColor") && !/style=["'][^"']*color:/i.test(s)) {
    s = s.replace(/^<svg\b/i, '<svg style="color:#8FD0FF"');
  }
  if (!/<title>/i.test(s)) {
    const safeTitle = fallbackTitle.replace(/&(?!amp;)/g, "&amp;");
    s = s.replace(/(<svg[^>]*>)/i, `$1<title>${safeTitle}</title>`);
  }
  if (!/xmlns=/i.test(s)) {
    s = s.replace(/^<svg\b/i, '<svg xmlns="http://www.w3.org/2000/svg"');
  }
  return { clean: s, viewBox };
}

function buildSchemaTsSource(): string {
  return `/**
 * Authoritative TypeScript Schema & Runtime Validator for \`docs/aphrody/design/manifest.json\`.
 * Anchored to \`https://design.google/?home\`, Material 3 Expressive, Google Sans Flex,
 * W3C DTCG Figma Variables, Google Stitch (\`data-stitch-id\`), GPUI, and \`egui 0.37\` / Bevy UE5.8.
 */

export interface ReferenceCaptureSpec {
  id: string;
  path: string;
  bytes: number;
  sha256: string;
  surfaceFamily: "gemini-neural-home" | "gemini-cascading-menu" | "google-ai-mode" | "chrome-omnibar" | "chrome-157-grounding";
  keyPatterns: string[];
}

export interface OptimizedSvgSpec {
  id: string;
  title: string;
  path: string;
  viewBox: string;
  bytes: number;
  sha256: string;
  a11yRoleImg: boolean;
}

export interface GuidelineMdxSpec {
  id: string;
  title: string;
  path: string;
  sourceUrl: string;
  bytes: number;
  sha256: string;
}

export interface GpuiAndEguiComponentContract {
  id: string;
  category: "shell" | "navigation" | "composer" | "overlay" | "data_display" | "viewport_3d";
  gpuiSymbol: string;
  eguiEquivalentSymbol: string;
  nshElement: string;
  stitchSlotId: string;
  tokensConsumed: string[];
  states: ("default" | "hover" | "pressed" | "focused" | "selected" | "disabled")[];
}

export interface DesignSystemManifest {
  schemaVersion: 2;
  name: string;
  sourceOfTruth: "https://design.google/";
  homeUrl: "https://design.google/?home";
  codeAsMaterialUrl: "https://design.google/library/code-is-a-design-material";
  stitchUrl: "https://stitch.withgoogle.com/";
  stylesheetPath: string;
  designMdPath: string;
  wcagContrastDarkPrimary: number;
  wcagContrastLightPrimary: number;
  referenceCaptures: ReferenceCaptureSpec[];
  svgs: OptimizedSvgSpec[];
  guidelines: GuidelineMdxSpec[];
  kits: {
    figma: { path: string; sha256: string; collectionCount: number; componentCount: number };
    figmaCodeConnect: { path: string; sha256: string; mappingCount: number };
    stitch: { path: string; sha256: string; screenCount: number; preserveDataStitchId: boolean };
  };
  maquettes: Array<{
    id: string;
    file: string;
    path: string;
    stitchId: string;
    surface: string;
    deviceType: string;
    title: string;
    defaultTheme: string;
    bytes: number;
  }>;
  components: GpuiAndEguiComponentContract[];
}

export function validateDesignManifest(doc: unknown): { ok: boolean; errors: string[] } {
  const errors: string[] = [];
  if (!doc || typeof doc !== "object") {
    return { ok: false, errors: ["Manifest must be a JSON object"] };
  }
  const m = doc as Partial<DesignSystemManifest>;
  if (m.schemaVersion !== 2) errors.push("schemaVersion must equal 2");
  if (m.sourceOfTruth !== "https://design.google/") errors.push("Invalid sourceOfTruth");
  if ((m.wcagContrastDarkPrimary ?? 0) < 4.5) errors.push("Dark primary WCAG contrast < 4.5");
  if ((m.wcagContrastLightPrimary ?? 0) < 4.5) errors.push("Light primary WCAG contrast < 4.5");
  if (!Array.isArray(m.svgs) || m.svgs.length < 20) errors.push("Expected >= 20 optimized SVGs");
  if (!Array.isArray(m.guidelines) || m.guidelines.length < 6) errors.push("Expected 6 MDX guidelines");
  if (!Array.isArray(m.components) || m.components.length < 8) errors.push("Expected >= 8 dual GPUI/egui component contracts");
  if (!Array.isArray(m.maquettes) || m.maquettes.length < 12) errors.push("Expected 12 cross-platform HTML5 maquettes");
  if (!m.kits?.figma?.path || !m.kits?.stitch?.path || !m.kits?.figmaCodeConnect?.path) errors.push("Missing Figma, Code Connect, or Stitch kit reference");
  return { ok: errors.length === 0, errors };
}
`;
}

function buildStylesheetCss(): string {
  const tokens = buildCanonicalDesignTokens();
  const d = tokens.sys.color.dark;
  const l = tokens.sys.color.light;
  const n = tokens.sys.color.neural_obsidian ?? d;
  const o = tokens.sys.color.oled_pitch ?? d;
  const spFast = sampleDampedSpringLinearCss(
    tokens.sys.motion.springs.spatial_fast.stiffness,
    tokens.sys.motion.springs.spatial_fast.damping_ratio,
    0.24,
  );
  const spDefault = sampleDampedSpringLinearCss(
    tokens.sys.motion.springs.spatial_default.stiffness,
    tokens.sys.motion.springs.spatial_default.damping_ratio,
    0.34,
  );
  const efDefault = sampleDampedSpringLinearCss(
    tokens.sys.motion.springs.effects_default.stiffness,
    tokens.sys.motion.springs.effects_default.damping_ratio,
    0.20,
  );

  const emitPaletteVars = (p: typeof d) =>
    Object.entries(p)
      .map(([k, v]) => `  --md-sys-color-${k.replaceAll("_", "-")}: ${v};`)
      .join("\n");

  return `/* Authoritative Material 3 Expressive + Neural Expressive Stylesheet (https://design.google/?home) */
/* Generated by tooling/bun/design-docs-pack.ts — 100% Baseline & WCAG 2.2 AA compliant */

@font-face {
  font-family: "Google Sans Code";
  src: local("Google Sans Code"), url("../../assets/fonts/google-sans-code/GoogleSansCode-Variable.ttf") format("truetype");
  font-weight: 300 800;
  font-display: swap;
}

@font-face {
  font-family: "JetBrains Mono";
  src: local("JetBrains Mono"), url("../../assets/fonts/jetbrains-mono/JetBrainsMono-Regular.ttf") format("truetype");
  font-weight: 400;
  font-display: swap;
}

@font-face {
  font-family: "IBM Plex Sans";
  src: local("IBM Plex Sans"), url("../../assets/fonts/ibm-plex-sans/IBMPlexSans-Regular.ttf") format("truetype");
  font-weight: 400;
  font-display: swap;
}

:root, [data-theme="dark"] {
  --md-ref-typeface-brand: "Google Sans Flex", "Google Sans", "IBM Plex Sans", system-ui, sans-serif;
  --md-ref-typeface-plain: "Google Sans Flex", "IBM Plex Sans", "Google Sans Text", system-ui, sans-serif;
  --md-ref-typeface-mono: "Google Sans Code", "JetBrains Mono", "Lilex", monospace;
  --md-sys-brand-bolt: #FFB300;
  --md-sys-neural-indigo: #111B3A;

  /* Google Sans Flex 6 Variable Axes */
  --md-ref-font-axes-display: "opsz" 48, "wght" 500, "wdth" 100, "GRAD" 0, "slnt" 0, "ROND" 40;
  --md-ref-font-axes-ui: "opsz" 14, "wght" 500, "wdth" 100, "GRAD" 0, "slnt" 0, "ROND" 20;

${emitPaletteVars(d)}

  /* Neural Expressive Obsidian Backdrop & Aura (from docs/aphrody/design/*.png) */
  --md-sys-neural-obsidian: #0B0E14;
  --md-sys-neural-rail: #13161F;
  --md-sys-neural-popover: #1C202B;
  --md-sys-neural-glow: radial-gradient(circle at 50% 36%, rgba(59, 130, 246, 0.22) 0%, rgba(17, 27, 58, 0.35) 38%, rgba(11, 14, 20, 0) 74%);

  /* Shape Scale */
  --md-sys-shape-corner-xs: ${tokens.sys.shape.extra_small_px}px;
  --md-sys-shape-corner-sm: ${tokens.sys.shape.small_px}px;
  --md-sys-shape-corner-md: ${tokens.sys.shape.medium_px}px;
  --md-sys-shape-corner-lg: ${tokens.sys.shape.large_px}px;
  --md-sys-shape-corner-lg-inc: ${tokens.sys.shape.large_increased_px}px;
  --md-sys-shape-corner-xl: ${tokens.sys.shape.extra_large_px}px;
  --md-sys-shape-corner-full: ${tokens.sys.shape.full_px}px;

  /* Analytical Damped Harmonic Oscillator Springs (aphrody_ui::google_design::SpringSpec) */
  --md-sys-motion-spring-spatial-fast: 240ms ${spFast};
  --md-sys-motion-spring-spatial-default: 340ms ${spDefault};
  --md-sys-motion-spring-effects-default: 200ms ${efDefault};

  /* State Layer Opacities & Accessibility Touch Minimums */
  --md-sys-state-hover-opacity: ${tokens.sys.state.hover_opacity};
  --md-sys-state-pressed-opacity: ${tokens.sys.state.pressed_opacity};
  --md-sys-focus-ring-width: ${tokens.sys.state.focus_ring_px}px;
  --md-sys-touch-target-min: ${tokens.sys.density.touch_target_min_px}px;
}

[data-theme="neural"] {
${emitPaletteVars(n)}
  --md-sys-neural-obsidian: #0B0E14;
  --md-sys-neural-rail: #13161F;
  --md-sys-neural-popover: #1C202B;
  --md-sys-neural-glow: radial-gradient(circle at 50% 34%, rgba(59, 130, 246, 0.24) 0%, rgba(17, 27, 58, 0.42) 40%, rgba(11, 14, 20, 0) 74%);
}

[data-theme="oled"] {
${emitPaletteVars(o)}
  --md-sys-neural-obsidian: #000000;
  --md-sys-neural-rail: #0A0E14;
  --md-sys-neural-popover: #121720;
  --md-sys-neural-glow: radial-gradient(circle at 50% 34%, rgba(143, 208, 255, 0.14) 0%, rgba(0, 0, 0, 0) 68%);
}

[data-theme="light"] {
${emitPaletteVars(l)}
  --md-sys-neural-obsidian: #F7F9FF;
  --md-sys-neural-rail: #EEF3FA;
  --md-sys-neural-popover: #FFFFFF;
  --md-sys-neural-glow: radial-gradient(circle at 50% 36%, rgba(0, 99, 153, 0.10) 0%, rgba(91, 88, 145, 0.05) 40%, rgba(247, 249, 255, 0) 74%);
}

@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }
}

* { box-sizing: border-box; }
.gd-tabular { font-variant-numeric: tabular-nums; }
.gd-pill-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  min-height: var(--md-sys-touch-target-min);
  padding: 0 20px;
  border-radius: var(--md-sys-shape-corner-full);
  font-family: var(--md-ref-typeface-plain);
  font-weight: 600;
  font-size: 14px;
  text-decoration: none;
  cursor: pointer;
  border: 1px solid transparent;
  transition: border-radius var(--md-sys-motion-spring-spatial-fast), transform var(--md-sys-motion-spring-spatial-fast), background-color var(--md-sys-motion-spring-effects-default);
}
.gd-pill-button:active {
  border-radius: var(--md-sys-shape-corner-md);
  transform: scale(0.98);
}

/* Maquette Showcase & Cross-Platform Layout Primitives */
.mq-body {
  margin: 0;
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  font-family: var(--md-ref-typeface-plain);
  background: var(--md-sys-color-background);
  color: var(--md-sys-color-on-surface);
  line-height: 1.5;
}
.mq-skip-link { position: absolute; left: -9999px; }
.mq-skip-link:focus { left: 16px; top: 16px; z-index: 99; padding: 8px 14px; background: var(--md-sys-color-primary); color: var(--md-sys-color-on-primary); border-radius: 8px; }
.mq-topbar {
  position: sticky; top: 0; z-index: 20;
  display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 12px;
  padding: 10px 20px;
  background: var(--md-sys-neural-rail);
  border-bottom: 1px solid var(--md-sys-color-outline-variant);
}
.mq-topbar-left, .mq-topbar-right, .mq-top-nav, .mq-theme-switcher { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }
.mq-brand-lockup { display: inline-flex; align-items: center; gap: 10px; color: var(--md-sys-color-on-surface); text-decoration: none; font-weight: 700; }
.mq-nav-link {
  min-height: 36px; display: inline-flex; align-items: center; padding: 6px 12px; border-radius: 9999px;
  color: var(--md-sys-color-on-surface-variant); text-decoration: none; font-size: 13px; font-weight: 500;
}
.mq-nav-link[aria-current="page"] { background: var(--md-sys-color-primary-container); color: var(--md-sys-color-on-primary-container); }
.mq-theme-btn {
  min-height: 36px; padding: 6px 12px; border-radius: 9999px; border: 1px solid var(--md-sys-color-outline-variant);
  background: var(--md-sys-color-surface-container); color: var(--md-sys-color-on-surface); font-size: 12px; font-weight: 600; cursor: pointer;
}
.mq-theme-btn[aria-pressed="true"] { background: var(--md-sys-color-primary); color: var(--md-sys-color-on-primary); border-color: var(--md-sys-color-primary); }
.mq-main { flex: 1; max-width: 1320px; width: 100%; margin: 0 auto; padding: 24px 20px; display: flex; flex-direction: column; gap: 20px; }
.mq-footer {
  display: flex; flex-wrap: wrap; justify-content: space-between; gap: 12px; padding: 14px 20px;
  border-top: 1px solid var(--md-sys-color-outline-variant); background: var(--md-sys-color-surface-container-low);
  font-size: 12px; color: var(--md-sys-color-on-surface-variant);
}
.mq-footer a { color: var(--md-sys-color-primary); }
.mq-hero { padding: 28px; border-radius: var(--md-sys-shape-corner-xl); background: var(--md-sys-color-surface-container-low); border: 1px solid var(--md-sys-color-outline-variant); }
.mq-neural-aura { background-image: var(--md-sys-neural-glow); }
.mq-centered { text-align: center; display: flex; flex-direction: column; align-items: center; }
.mq-display-hero { font-family: var(--md-ref-typeface-brand); font-variation-settings: var(--md-ref-font-axes-display); font-size: clamp(28px, 4vw, 48px); line-height: 1.15; margin: 12px 0; }
.mq-display-title { font-family: var(--md-ref-typeface-brand); font-variation-settings: var(--md-ref-font-axes-display); font-size: clamp(22px, 2.8vw, 32px); line-height: 1.25; margin: 10px 0; }
.mq-lead { color: var(--md-sys-color-on-surface-variant); max-width: 720px; margin: 0 0 16px; }
.mq-pill-eyebrow { display: inline-flex; align-items: center; gap: 8px; padding: 6px 14px; border-radius: 9999px; background: var(--md-sys-color-secondary-container); color: var(--md-sys-color-on-secondary-container); font-size: 12px; font-weight: 600; }
.mq-badge, .mq-meta-pill { display: inline-flex; align-items: center; padding: 4px 10px; border-radius: 9999px; background: var(--md-sys-color-surface-container-high); color: var(--md-sys-color-on-surface-variant); font-size: 11px; font-weight: 600; }
.mq-badge-ok { background: var(--md-sys-color-secondary-container); color: var(--md-sys-color-on-secondary-container); }
.mq-badge-primary { background: var(--md-sys-color-primary); color: var(--md-sys-color-on-primary); }
.mq-cta-group, .mq-chip-row, .mq-flow-strip { display: flex; flex-wrap: wrap; gap: 10px; align-items: center; }
.mq-btn-primary { background: var(--md-sys-color-primary); color: var(--md-sys-color-on-primary); }
.mq-btn-secondary { background: var(--md-sys-color-surface-container-high); color: var(--md-sys-color-on-surface); border-color: var(--md-sys-color-outline-variant); }
.mq-chip { min-height: 36px; display: inline-flex; align-items: center; padding: 6px 14px; border-radius: 9999px; border: 1px solid var(--md-sys-color-outline-variant); background: var(--md-sys-color-surface-container); color: var(--md-sys-color-on-surface); font-size: 12px; font-weight: 600; cursor: pointer; }
.mq-chip-active { background: var(--md-sys-color-primary); color: var(--md-sys-color-on-primary); border-color: var(--md-sys-color-primary); }
.mq-grid-2 { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 16px; }
.mq-grid-3 { display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: 16px; }
.mq-grid-4 { display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 16px; }
.mq-card { padding: 20px; border-radius: var(--md-sys-shape-corner-lg); background: var(--md-sys-color-surface-container); border: 1px solid var(--md-sys-color-outline-variant); display: flex; flex-direction: column; gap: 10px; }
.mq-card-focus { border-color: var(--md-sys-color-primary); }
.mq-card-header { display: flex; align-items: center; justify-content: space-between; gap: 10px; flex-wrap: wrap; }
.mq-card h2, .mq-card h3 { margin: 0; font-size: 17px; }
.mq-card h2 a { color: var(--md-sys-color-on-surface); text-decoration: none; }
.mq-kpi-row { display: grid; grid-template-columns: repeat(auto-fit, minmax(140px, 1fr)); gap: 12px; width: 100%; margin-top: 12px; }
.mq-kpi-card { padding: 14px; border-radius: var(--md-sys-shape-corner-md); background: var(--md-sys-color-surface-container-high); display: flex; flex-direction: column; gap: 4px; }
.mq-kpi-label, .mq-kpi-meta, .mq-meta-line { font-size: 12px; color: var(--md-sys-color-on-surface-variant); overflow-wrap: anywhere; }
.mq-meta-line code { font-family: var(--md-ref-typeface-mono); overflow-wrap: anywhere; word-break: break-word; }
[data-theme="light"] .mq-card img[src*="../svg/"], [data-theme="light"] .mq-os-dock img[src*="../svg/"], [data-theme="light"] .mq-icon-rail img[src*="../svg/"] { filter: brightness(0.45) contrast(1.4); }
[data-theme="light"] img[src*="aphrody-monogram-neural.svg"] { filter: none !important; }
.mq-kpi-value { font-size: 24px; color: var(--md-sys-color-primary); }
.mq-code-block { margin: 0; padding: 14px; border-radius: var(--md-sys-shape-corner-md); background: var(--md-sys-color-surface-container-lowest); border: 1px solid var(--md-sys-color-outline-variant); font-family: var(--md-ref-typeface-mono); font-size: 12.5px; overflow-x: auto; color: var(--md-sys-color-on-surface); }
.mq-shell-3col { display: grid; grid-template-columns: 240px minmax(0, 1fr) 280px; gap: 16px; }
.mq-ide-frame { display: grid; grid-template-columns: 52px minmax(0, 1fr) 300px; gap: 14px; }
.mq-icon-rail { display: flex; flex-direction: column; align-items: center; gap: 16px; padding: 14px 6px; border-radius: var(--md-sys-shape-corner-lg); background: var(--md-sys-color-surface-container-low); border: 1px solid var(--md-sys-color-outline-variant); }
.mq-sidebar { display: flex; flex-direction: column; gap: 8px; padding: 14px; border-radius: var(--md-sys-shape-corner-lg); background: var(--md-sys-color-surface-container-low); border: 1px solid var(--md-sys-color-outline-variant); }
.mq-side-item { padding: 10px 12px; border-radius: var(--md-sys-shape-corner-md); font-size: 13px; color: var(--md-sys-color-on-surface-variant); text-decoration: none; }
.mq-side-active { background: var(--md-sys-color-primary-container); color: var(--md-sys-color-on-primary-container); font-weight: 600; }
.mq-omnibar-pill { width: 100%; display: flex; align-items: center; justify-content: space-between; gap: 10px; padding: 10px 14px; border-radius: 9999px; background: var(--md-sys-neural-popover); border: 1px solid var(--md-sys-color-primary); font-size: 13px; }
.mq-popover-card { padding: 14px; border-radius: var(--md-sys-shape-corner-lg-inc); background: var(--md-sys-neural-popover); border: 1px solid var(--md-sys-color-outline-variant); display: flex; flex-direction: column; gap: 6px; font-size: 13px; }
.mq-grounding-stack { padding: 16px; border-radius: var(--md-sys-shape-corner-xl); background: var(--md-sys-color-surface-container-low); border: 1px solid var(--md-sys-color-outline-variant); display: flex; flex-direction: column; gap: 10px; }
.mq-ground-item { padding: 12px; border-radius: var(--md-sys-shape-corner-lg); background: var(--md-sys-color-surface-container); }
.mq-thinking-aura { display: inline-flex; align-items: center; gap: 8px; padding: 6px 14px; border-radius: 9999px; border: 1px solid var(--md-sys-color-tertiary); background: var(--md-sys-color-surface-container-high); font-size: 12px; }
.mq-os-panel { display: flex; justify-content: space-between; align-items: center; padding: 10px 16px; border-radius: 9999px; background: var(--md-sys-color-surface-container-high); font-size: 13px; }
.mq-os-dock { align-self: center; display: flex; align-items: center; gap: 14px; padding: 10px 18px; border-radius: var(--md-sys-shape-corner-xl); background: var(--md-sys-color-surface-container-high); border: 1px solid var(--md-sys-color-outline-variant); }
.mq-tab-strip { display: flex; flex-wrap: wrap; gap: 8px; }
.mq-tab { padding: 6px 12px; border-radius: 8px; background: var(--md-sys-color-surface-container-low); font-size: 12px; color: var(--md-sys-color-on-surface-variant); }
.mq-tab-active { background: var(--md-sys-color-primary-container); color: var(--md-sys-color-on-primary-container); font-weight: 600; }
.mq-shader-stage { display: flex; align-items: center; gap: 16px; padding: 18px; border-radius: var(--md-sys-shape-corner-md); background: var(--md-sys-color-surface-container-lowest); border: 1px solid var(--md-sys-color-outline-variant); }
.mq-flow-node { padding: 8px 12px; border-radius: 10px; background: var(--md-sys-color-surface-container-high); font-family: var(--md-ref-typeface-mono); font-size: 12px; }
.mq-axis-controls { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 12px; font-size: 12px; }
.mq-device-showcase { display: flex; justify-content: center; }
.mq-phone-frame { width: min(390px, 100%); min-height: 620px; border-radius: 28px; border: 2px solid var(--md-sys-color-outline); background: var(--md-sys-color-surface); display: flex; flex-direction: column; justify-content: space-between; overflow: hidden; }
.mq-phone-notch { text-align: center; padding: 10px; font-size: 12px; font-weight: 600; background: var(--md-sys-color-surface-container-low); }
.mq-phone-content { padding: 14px; display: flex; flex-direction: column; gap: 12px; flex: 1; }
.mq-bottom-nav { min-height: 64px; display: grid; grid-template-columns: repeat(4, 1fr); gap: 6px; padding: 8px 12px; background: var(--md-sys-color-surface-container-low); border-top: 1px solid var(--md-sys-color-outline-variant); }
.mq-bottom-tab { min-height: 44px; display: flex; align-items: center; justify-content: center; border-radius: 9999px; font-size: 12px; font-weight: 600; color: var(--md-sys-color-on-surface-variant); text-decoration: none; }
.mq-bottom-active { background: var(--md-sys-color-primary-container); color: var(--md-sys-color-on-primary-container); }
@media (max-width: 840px) {
  .mq-shell-3col, .mq-ide-frame { grid-template-columns: 1fr; }
}
`;
}

function buildDesignMd(): string {
  const tokens = buildCanonicalDesignTokens();
  const d = tokens.sys.color.dark;
  const l = tokens.sys.color.light;
  return `---
name: Aphrody Google Neural Expressive Design System
source_of_truth: https://design.google/
system: Material 3 Expressive
stitch_url: https://stitch.withgoogle.com/
sans_flex_family: Google Sans Flex
colors:
  surface: "${d.surface}"
  surface-dim: "${d.surface_dim}"
  surface-bright: "${d.surface_bright}"
  surface-container-lowest: "${d.surface_container_lowest}"
  surface-container-low: "${d.surface_container_low}"
  surface-container: "${d.surface_container}"
  surface-container-high: "${d.surface_container_high}"
  surface-container-highest: "${d.surface_container_highest}"
  on-surface: "${d.on_surface}"
  on-surface-variant: "${d.on_surface_variant}"
  outline: "${d.outline}"
  outline-variant: "${d.outline_variant}"
  primary: "${d.primary}"
  on-primary: "${d.on_primary}"
  primary-container: "${d.primary_container}"
  on-primary-container: "${d.on_primary_container}"
  secondary: "${d.secondary}"
  on-secondary: "${d.on_secondary}"
  secondary-container: "${d.secondary_container}"
  on-secondary-container: "${d.on_secondary_container}"
  tertiary: "${d.tertiary}"
  on-tertiary: "${d.on_tertiary}"
  tertiary-container: "${d.tertiary_container}"
  on-tertiary-container: "${d.on_tertiary_container}"
  error: "${d.error}"
  on-error: "${d.on_error}"
  background: "${d.background}"
  on-background: "${d.on_background}"
  light-surface: "${l.surface}"
  light-on-surface: "${l.on_surface}"
  light-primary: "${l.primary}"
  light-on-primary: "${l.on_primary}"
typography:
  display-lg:
    fontFamily: "Google Sans Flex"
    fontSize: 57px
    fontWeight: "400"
    lineHeight: 64px
  headline-md:
    fontFamily: "Google Sans Flex"
    fontSize: 28px
    fontWeight: "500"
    lineHeight: 36px
  body-md:
    fontFamily: "Google Sans Flex"
    fontSize: 14px
    fontWeight: "400"
    lineHeight: 20px
  code-mono:
    fontFamily: "JetBrains Mono"
    fontSize: 13px
    fontWeight: "400"
    lineHeight: 20px
rounded:
  xs: 4px
  sm: 8px
  md: 12px
  lg: 16px
  lg-increased: 20px
  xl: 28px
  full: 9999px
spacing:
  gap: 8px
  panel: 16px
  canvas: 24px
  touch-min: 44px
---

# Google Design & Neural Expressive Specification (\`docs/aphrody/design/DESIGN.md\`)

Anchored to \`https://design.google/?home\` and \`https://design.google/library/code-is-a-design-material\`, uniting **GPUI**, **NSH**, **\`egui 0.37\` (\`egui_gpui\`)**, **Bevy Unreal Engine 5.8 (\`bevy_gpui\`)**, **FigCraft**, and **Craft Anything**.

## Motion Inventory

| Trigger / Interaction | Token / Physics Spring | Behavior & Reduced-Motion Rule |
| --- | --- | --- |
| Primary Pill Press | \`spatial_fast\` (\`k=1400, z=0.9\`) | Morphs corner radius \`9999px -> 12px\` & \`scale(0.98)\`; instant color change under reduced motion |
| Navigation Rail Collapse | \`spatial_default\` (\`k=700, z=0.9\`) | Animates width \`260px <-> 56px\` preserving icon anchor |
| Cascading Popover Open | \`spatial_fast\` + \`effects_default\` | Scales \`0.96 -> 1.0\` from trigger corner (\`20px\` radius); \`Escape\` restores trigger focus |
| Inspector / Sheet Slide | \`spatial_default\` (\`k=700, z=0.9\`) | Slides right inspector drawer without shifting primary scroll offset |
| Live Telemetry Pulse | \`effects_slow\` (\`k=800, z=1.0\`) | Critically damped status dot opacity pulse; static dot under reduced motion |

\`\`\`yaml
stitch:
  preserve_data_stitch_id: true
  studios:
    figcraft_google: "{comp.card.container_color_role}"
    craft_anything_google: "{comp.code_surface.container_color_role}"
    unreal_bevy_58: "{comp.shader_canvas.accent_color_role}"
\`\`\`
`;
}

function buildFigmaKitJson(): object {
  const tokens = buildCanonicalDesignTokens();
  const refPalette = Object.fromEntries(
    Object.entries(tokens.ref.palette).map(([k, hex]) => [
      k,
      { $type: "color", $value: hex, $extensions: { "com.figma.scopes": ["ALL_SCOPES"] } },
    ]),
  );
  const channelColors = {
    "channel-lts": { $type: "color", $value: "#BAE6FD" },
    "channel-stable": { $type: "color", $value: "#ECF4FF" },
    "channel-preview": { $type: "color", $value: "#E9D5FF" },
    "channel-canary": { $type: "color", $value: "#FBCFE8" },
    "channel-nightly": { $type: "color", $value: "#FDE08A" },
    "channel-dev": { $type: "color", $value: "#99F6E4" },
    "brand-bolt": { $type: "color", $value: "#FFB300" },
    "neural-indigo": { $type: "color", $value: "#111B3A" },
  };
  const colorVariables: Record<string, object> = {};
  for (const [role, darkHex] of Object.entries(tokens.sys.color.dark)) {
    const lightHex = (tokens.sys.color.light as unknown as Record<string, string>)[role] ?? darkHex;
    const neuralHex = (tokens.sys.color.neural_obsidian as unknown as Record<string, string> | undefined)?.[role] ?? darkHex;
    const oledHex = (tokens.sys.color.oled_pitch as unknown as Record<string, string> | undefined)?.[role] ?? darkHex;
    const cssName = role.replaceAll("_", "-");
    colorVariables[cssName] = {
      $type: "color",
      $value: neuralHex,
      $extensions: {
        "com.figma.modes": {
          "Neural Obsidian": neuralHex,
          "M3 Dark": darkHex,
          "M3 Light": lightHex,
          "OLED Pitch": oledHex,
        },
        "com.figma.scopes": ["ALL_SCOPES"],
        "com.figma.codeSyntax": {
          WEB: `var(--md-sys-color-${cssName})`,
          RUST_GPUI: `tokens.hsla(mode, "${role}")`,
          ANDROID: `MaterialTheme.colorScheme.${role}`,
          iOS: `AphrodyTheme.color("${role}")`,
        },
      },
    };
  }
  const responsiveVariables = {
    columns: {
      $type: "number",
      $value: 12,
      $extensions: { "com.figma.modes": { Mobile_390: 4, Tablet_834: 8, Desktop_1440: 12, Ultrawide_1920: 16 } },
    },
    "canvas-margin": {
      $type: "dimension",
      $value: "24px",
      $extensions: { "com.figma.modes": { Mobile_390: "16px", Tablet_834: "20px", Desktop_1440: "24px", Ultrawide_1920: "32px" } },
    },
    gutter: {
      $type: "dimension",
      $value: "16px",
      $extensions: { "com.figma.modes": { Mobile_390: "12px", Tablet_834: "16px", Desktop_1440: "16px", Ultrawide_1920: "24px" } },
    },
    "rail-width": {
      $type: "dimension",
      $value: "56px",
      $extensions: { "com.figma.modes": { Mobile_390: "0px", Tablet_834: "80px", Desktop_1440: "260px", Ultrawide_1920: "280px" } },
    },
    "touch-target-min": {
      $type: "dimension",
      $value: "44px",
      $extensions: { "com.figma.modes": { Mobile_390: "48px", Tablet_834: "44px", Desktop_1440: "44px", Ultrawide_1920: "44px" } },
    },
  };
  return {
    $schema: "https://tr.designtokens.org/format/",
    name: "Aphrody · Google Design Material 3 Expressive + Neural Figma Kit",
    version: "3.0.0",
    sourceUrl: "https://design.google/?home",
    collections: {
      "01_Ref/Palette": { ...refPalette, ...channelColors },
      "02_Sys/Color": colorVariables,
      "03_Sys/Typography": {
        ...Object.fromEntries(
          Object.entries(tokens.sys.typescale).map(([k, spec]) => [
            k,
            {
              $type: "typography",
              $value: {
                fontFamily: spec.font === "brand" ? "Google Sans Flex" : "Google Sans Text",
                fontSize: `${spec.size_sp}px`,
                lineHeight: `${spec.line_height_sp}px`,
                fontWeight: spec.weight,
              },
            },
          ]),
        ),
        axes: tokens.ref.variable_axes,
      },
      "04_Sys/Layout_Responsive": responsiveVariables,
      "05_Sys/Shape_Elevation": {
        shape: Object.fromEntries(
          Object.entries(tokens.sys.shape).map(([k, px]) => [k, { $type: "dimension", $value: `${px}px` }]),
        ),
        elevationLevels: ["surface_container_lowest", "surface_container_low", "surface_container", "surface_container_high", "surface_container_highest", "neural_aura_glow"],
      },
      "06_Sys/Motion_Springs": tokens.sys.motion.springs,
    },
    componentSets: [
      { id: "ExpressiveAppShell", variants: ["ExpandedRail", "CollapsedRail", "MobileSheet"], autoLayout: "HORIZONTAL", radiusToken: "large_px" },
      { id: "DesktopNavRail", variants: ["Collapsed56", "Expanded260", "Floating"], autoLayout: "VERTICAL", radiusToken: "large_px" },
      { id: "MobileBottomNavBar", variants: ["Neural", "Dark", "Light", "OLED"], autoLayout: "HORIZONTAL", radiusToken: "none_px" },
      { id: "SegmentedStudioBar", variants: ["2Up", "4Up", "Scrollable"], autoLayout: "HORIZONTAL", radiusToken: "full_px" },
      { id: "NeuralOmnibar", variants: ["Idle", "Focused", "AttachmentMenuOpen", "VoiceActive"], autoLayout: "HORIZONTAL", radiusToken: "full_px" },
      { id: "CascadingPopoverMenu", variants: ["RootMenu", "SubMenu", "WithBadges"], autoLayout: "VERTICAL", radiusToken: "large_increased_px" },
      { id: "GroundingCardStack", variants: ["Collapsed3", "ExpandedAll", "CompactMobile"], autoLayout: "VERTICAL", radiusToken: "extra_large_px" },
      { id: "ThinkingAuraBadge", variants: ["Thinking", "Grounded", "Complete"], autoLayout: "HORIZONTAL", radiusToken: "full_px" },
      { id: "ExpressivePillButton", variants: ["Primary", "Secondary", "Outline", "Ghost", "Destructive"], autoLayout: "HORIZONTAL", radiusToken: "full_px" },
      { id: "IdeEditorTabStrip", variants: ["Active", "Modified", "SplitPane"], autoLayout: "HORIZONTAL", radiusToken: "small_px" },
      { id: "DiffReviewHunkCard", variants: ["Added", "Modified", "Approved", "MobileSwipe"], autoLayout: "VERTICAL", radiusToken: "medium_px" },
      { id: "TerminalSheetDrawer", variants: ["Peek", "Half", "Expanded"], autoLayout: "VERTICAL", radiusToken: "extra_large_px" },
      { id: "CosmicTopPanel", variants: ["Compact", "Default", "FloatingPill"], autoLayout: "HORIZONTAL", radiusToken: "full_px" },
      { id: "CosmicDockBar", variants: ["BottomFloating", "LeftPinned", "AutoHide"], autoLayout: "HORIZONTAL", radiusToken: "extra_large_px" },
      { id: "FigCraftCanvasNode", variants: ["Frame", "Component", "TokenBound"], autoLayout: "VERTICAL", radiusToken: "medium_px" },
      { id: "CraftAnythingDisasmTable", variants: ["X64", "A64", "Wasm"], autoLayout: "VERTICAL", radiusToken: "medium_px" },
      { id: "UnrealBevyViewportHud", variants: ["LitPbrSolari", "NaniteMeshlets", "WorldNormals"], autoLayout: "HORIZONTAL", radiusToken: "large_px" },
      { id: "HeroPlaygroundBanner", variants: ["Desktop1440", "Mobile390", "WebGpuActive"], autoLayout: "VERTICAL", radiusToken: "extra_large_px" },
      { id: "ReleaseChannelMatrixCard", variants: ["LTS", "Stable", "Preview", "Canary", "Nightly", "Dev"], autoLayout: "VERTICAL", radiusToken: "large_px" },
      { id: "VpnShieldQuickTile", variants: ["Disconnected", "Connecting", "ProtectedWG", "MobileOLED"], autoLayout: "HORIZONTAL", radiusToken: "large_px" },
    ],
    material3Kit: MATERIAL3_FIGMA_KIT,
    referenceMirrors: REFERENCE_MIRRORS,
  };
}

/** Official Material 3 Design Kit (Community); indexed in assets/os/r2_assets_manifest.toml and served via R2. */
const MATERIAL3_FIGMA_KIT = {
  name: "Material 3 Design Kit (Community)",
  fileKey: "vEr6T5SQaJfMpLxjMUfD55",
  nodeId: "11:1833",
  url: "https://www.figma.com/design/vEr6T5SQaJfMpLxjMUfD55/Material-3-Design-Kit--Community-?node-id=11-1833",
  localArchive: "docs/aphrody/design/Material 3 Design Kit (Community).zip",
  r2Manifest: "assets/os/r2_assets_manifest.toml",
  license: "CC BY 4.0",
} as const;

/** design.google library and Material ecosystem references mirrored as native GPUI components. */
const REFERENCE_MIRRORS = [
  {
    id: "TransparentHudCard",
    sourceUrl: "https://design.google/library/transparent-screens",
    gpui: "aphrody_ui::google_design_mirror::TransparentHudCard",
    tokens: ["ref.palette.glimmer_*", "comp.glimmer_hud"],
    theme: "xr-glimmer",
  },
  {
    id: "GeminiSpectrumCard",
    sourceUrl: "https://design.google/library/gemini-ai-visual-design",
    gpui: "aphrody_ui::google_design_mirror::GeminiSpectrumCard",
    tokens: ["ref.palette.gemini_*", "comp.gemini_spectrum"],
    theme: "gemini-ai",
  },
  {
    id: "YoutubeGradientScrubber",
    sourceUrl: "https://design.google/library/youtube-new-red-color",
    gpui: "aphrody_ui::google_design_mirror::YoutubeGradientScrubber",
    tokens: ["ref.palette.youtube_*", "comp.youtube_scrubber"],
    theme: "youtube-red",
  },
  {
    id: "GlowbarShellCard",
    sourceUrl: "https://googlebook.google/",
    gpui: "aphrody_ui::google_design_mirror::GlowbarShellCard",
    tokens: ["ref.palette.googlebook_*", "comp.googlebook_glowbar"],
    theme: "google-expressive",
  },
  {
    id: "QuireGlassNote",
    sourceUrl: "https://github.com/Mhsbrian/OpenBook",
    gpui: "aphrody_ui::google_design_mirror::QuireGlassNote",
    tokens: ["ref.palette.quire_*", "sys.motion.springs.quire_*", "comp.quire_glass_note"],
    theme: "openbook-quire",
  },
  {
    id: "MaterialComponentMatrix",
    sourceUrl: "https://docs.flutter.dev/ui/widgets/material",
    gpui: "aphrody_ui::google_design_mirror::MaterialComponentMatrix",
    tokens: ["sys.color.*", "sys.shape.*"],
    theme: "m3",
  },
] as const;

function buildFigmaCodeConnectJson(): object {
  const kit = buildFigmaKitJson() as { componentSets: Array<{ id: string }> };
  const map: Record<string, { gpui: string; nsh: string; egui: string; web: string; mobile: string }> = {
    ExpressiveAppShell: { gpui: "aphrody_ui::cosmic_shell::render_nav_rail", nsh: "<AppShell />", egui: "egui::SidePanel", web: ".mq-shell-3col", mobile: "nsh-platform-ios::NavigationStack" },
    DesktopNavRail: { gpui: "aphrody_ui::cosmic_shell::NavRail", nsh: "<NavRail />", egui: "egui::SidePanel::left", web: ".mq-icon-rail", mobile: "Compact -> MobileBottomNavBar" },
    MobileBottomNavBar: { gpui: "aphrody_ui::google_design_components::MobileBottomNavBar", nsh: "<BottomTabs />", egui: "egui::TopBottomPanel::bottom", web: ".mq-bottom-nav", mobile: "examples/nsh/mobile-companion" },
    SegmentedStudioBar: { gpui: "aphrody_ui::google_design_components::SegmentedStudioBar", nsh: "<SegmentedBar />", egui: "egui::SelectableLabel", web: ".mq-chip-row", mobile: "<SegmentedBar density='compact' />" },
    NeuralOmnibar: { gpui: "aphrody_ui::google_design_components::ExpressiveOmnibar", nsh: "<NeuralOmnibar />", egui: "egui::TextEdit::singleline", web: ".mq-omnibar-pill", mobile: "examples/nsh/mobile-companion" },
    CascadingPopoverMenu: { gpui: "aphrody_ui::google_design_components::CascadingPopoverCard", nsh: "<CascadingPopover />", egui: "egui::menu", web: ".mq-popover-card", mobile: "<Sheet detent='medium' />" },
    GroundingCardStack: { gpui: "aphrody_ui::google_design_components::GroundingSourceCard", nsh: "<GroundingCard />", egui: "egui::Frame::group", web: ".mq-grounding-stack", mobile: "<ScrollView horizontal />" },
    ThinkingAuraBadge: { gpui: "aphrody_ui::google_design_components::ThinkingAuraBadge", nsh: "<ThinkingAuraBadge />", egui: "egui::Spinner", web: ".mq-thinking-aura", mobile: "<ThinkingAuraBadge compact />" },
    ExpressivePillButton: { gpui: "aphrody_ui::starter_pack::StarterButton", nsh: "<PillButton />", egui: "egui::Button", web: ".gd-pill-button", mobile: "min-height: 44px touch target" },
    IdeEditorTabStrip: { gpui: "workspace::Pane", nsh: "<TabStrip />", egui: "egui::TopBottomPanel::top", web: ".mq-tab-strip", mobile: "<Picker />" },
    DiffReviewHunkCard: { gpui: "editor::DiffView", nsh: "<DiffView />", egui: "egui_gpui::CodeSurface", web: ".mq-code-block", mobile: "examples/nsh/mobile-companion" },
    TerminalSheetDrawer: { gpui: "terminal_view::TerminalView", nsh: "<Sheet />", egui: "egui::Window", web: ".mq-popover-card", mobile: "<BottomSheet snapPoints={[0.35, 0.85]} />" },
    CosmicTopPanel: { gpui: "aphrody_desktop::panel", nsh: "<TopPanel />", egui: "egui::TopBottomPanel::top", web: ".mq-os-panel", mobile: "SafeArea Top Bar" },
    CosmicDockBar: { gpui: "aphrody_desktop::dock", nsh: "<DockBar />", egui: "egui::TopBottomPanel::bottom", web: ".mq-os-dock", mobile: "MobileBottomNavBar" },
    FigCraftCanvasNode: { gpui: "aphrody_craft::render_gpui_studio_panel", nsh: "<FigCraftGoogleScreen />", egui: "egui_gpui::EguiGpuiHost", web: "[data-stitch-id='stitch-figcraft-root']", mobile: "ViewerOnly" },
    CraftAnythingDisasmTable: { gpui: "aphrody_craft::CraftStudioSession", nsh: "<CraftDisasmGrid />", egui: "egui::Grid", web: ".mq-code-block", mobile: "SummaryKPIs" },
    UnrealBevyViewportHud: { gpui: "bevy_gpui::render_viewport_rgba8", nsh: "<ViewportCanvas />", egui: "bevy_gpui::render_bevy_egui_inspector", web: ".mq-shader-stage", mobile: "WebGPU Pocket Preview" },
    HeroPlaygroundBanner: { gpui: "aphrody_desktop::examples::web_os", nsh: "<HeroBanner />", egui: "egui::CentralPanel", web: ".mq-hero.mq-neural-aura", mobile: ".mq-hero compact" },
    ReleaseChannelMatrixCard: { gpui: "aphrody_ui::google_design_components::RoadmapHorizonCard", nsh: "<ChannelCard />", egui: "egui::Frame::group", web: ".mq-card", mobile: "Stacked Card" },
    VpnShieldQuickTile: { gpui: "aphrody_vpn::VpnStatusTile", nsh: "<VpnQuickTile />", egui: "egui::ToggleValue", web: ".mq-omnibar-pill", mobile: "examples/nsh/mobile-companion" },
  };
  return {
    version: "3.0.0",
    figmaFileHint: "Aphrody Labs Design System v3.0",
    material3Kit: { fileKey: MATERIAL3_FIGMA_KIT.fileKey, nodeId: MATERIAL3_FIGMA_KIT.nodeId, url: MATERIAL3_FIGMA_KIT.url },
    referenceMirrors: REFERENCE_MIRRORS.map((mirror) => ({ id: mirror.id, gpui: mirror.gpui, sourceUrl: mirror.sourceUrl })),
    sourceOfTruth: "https://design.google/",
    mappings: kit.componentSets.map((cs) => ({
      figmaComponentSet: cs.id,
      ...(map[cs.id] ?? {
        gpui: "aphrody_ui::starter_pack",
        nsh: "<View />",
        egui: "egui::Frame",
        web: ".mq-card",
        mobile: "examples/nsh/mobile-companion",
      }),
    })),
  };
}

function buildStitchKitJson(maquettes: Array<{
  id: string;
  file: string;
  path: string;
  stitchId: string;
  surface: string;
  deviceType: string;
  title: string;
  defaultTheme: string;
  bytes: number;
}>): object {
  return {
    name: "Aphrody · Google Stitch & Multi-Studio Design Kit",
    version: "3.0.0",
    sourceUrl: "https://stitch.withgoogle.com/",
    designMdRef: "../DESIGN.md",
    preserveDataStitchId: true,
    upstreamProjects: [
      {
        projectId: "7027971799774493367",
        title: "Aphrody merged app",
        designSystems: ["assets/1fce14953fc44e5ba3ce49e4903d8771", "assets/3e02f3e0e21a4659b041a81802caa30d"],
      },
      {
        projectId: "15555120843463058326",
        title: "Aphrody sites (aphrody.com, aphrody.app) - M3",
        designSystems: ["assets/17150568052080874355"],
      },
    ],
    screens: [
      {
        id: "stitch-screen-figcraft-google",
        stitchId: "stitch-figcraft-root",
        title: "Google FigCraft & Stitch Vector/Token Studio",
        deviceType: "DESKTOP",
        gpuiAction: "google_design_studio::OpenGoogleFigCraftStudio",
        dualParityTarget: "aphrody_craft::StudioPillar::FigCraftGoogle",
        slots: ["stitch-figcraft-root", "stitch-figcraft-omnibar", "stitch-figcraft-cta"],
      },
      {
        id: "stitch-screen-craft-anything-google",
        stitchId: "stitch-craft-anything-root",
        title: "Google Craft Anything Clean-Room Disassembly & AST Workbench",
        deviceType: "DESKTOP",
        gpuiAction: "google_design_studio::OpenGoogleCraftAnythingStudio",
        dualParityTarget: "aphrody_craft::StudioPillar::CraftAnythingGoogle",
        slots: ["stitch-ca-disasm-grid", "stitch-ca-oxc-ast", "stitch-ca-evidence-ledger"],
      },
      {
        id: "stitch-screen-unreal-bevy-58",
        stitchId: "stitch-unreal-bevy-root",
        title: "Unreal Engine 5.8 Bevy Viewport (Nanite Meshlets + Solari GI + MaterialX)",
        deviceType: "DESKTOP",
        gpuiAction: "google_design_studio::OpenUnrealBevyStudio",
        dualParityTarget: "aphrody_craft::StudioPillar::UnrealBevy58",
        slots: ["stitch-bevy-viewport", "stitch-bevy-outliner", "stitch-bevy-solari-controls"],
      },
      {
        id: "stitch-screen-agy-session-studio",
        stitchId: "stitch-agy-studio-root",
        title: "AGY & Gemini Live Auto-Reload Session Studio",
        deviceType: "DESKTOP",
        gpuiAction: "aphrody_nsh::OpenShowcaseSurface",
        dualParityTarget: "examples/nsh/agy-session-studio",
        slots: ["stitch-agy-rail", "stitch-agy-kpis", "stitch-agy-timeline", "stitch-agy-inspector"],
      },
    ],
    maquettes,
  };
}

const GUIDELINES_MDX: Array<{ id: string; title: string; sourceUrl: string; content: string }> = [
  {
    id: "01-code-is-a-design-material",
    title: "Code is a Design Material",
    sourceUrl: "https://design.google/library/code-is-a-design-material",
    content: `---
title: "Code is a Design Material"
source: "https://design.google/library/code-is-a-design-material"
---

# Code is a Design Material

In Aphrody, live Rust GPUI entities (\`crates/aphrody_ui\`, \`crates/aphrody_craft\`), NSH TSX signals (\`examples/nsh/agy-session-studio\`), migrated \`egui 0.37\` surfaces (\`crates/egui_gpui\`), and Naga v30 WGSL shaders (\`crates/bevy_gpui\`) operate directly on one shared token graph rather than static handoff mocks.

- **Single Token Source**: \`fw/crates/fw-m3/tokens/google_design_tokens.toml\` -> \`DESIGN.md\` -> \`docs/aphrody/design/stylesheet.css\` -> \`ShadcnGpuiTheme\` & \`egui::Visuals\`.
- **Dual-Engine Verification**: Every Craft Studio screen renders both in **GPUI Google Design** and **\`egui 0.37\`** side-by-side to validate layout ergonomics, AccessKit node parity, and sub-millisecond tessellation budgets.
`,
  },
  {
    id: "02-material-3-expressive-and-neural-surfaces",
    title: "Material 3 Expressive & Neural Surfaces",
    sourceUrl: "https://design.google/?home",
    content: `---
title: "Material 3 Expressive & Neural Surfaces"
source: "https://design.google/?home"
---

# Material 3 Expressive & Neural Surfaces

Derived from \`https://design.google/?home\` and the 5 Neural Expressive reference captures in \`docs/aphrody/design/*.png\`:

1. **Surface Hierarchy**: \`surface-container-lowest\` (\`#080D13\`) -> \`surface\` (\`#0E141B\`) -> \`surface-container-low\` (\`#161D25\`) -> \`surface-container-high\` (\`#252C36\`).
2. **Containment Geometry**: \`20px\` (\`large_increased_px\`) for cascading popovers, \`24px-28px\` (\`extra_large_px\`) for grounding card stacks, and \`9999px\` (\`full_px\`) morphing to \`12px\` (\`medium_px\`) on active press for primary pills.
3. **WCAG 2.2 AA Floor**: Every foreground/background role pair maintains \`>= 4.5:1\` contrast in both Light and Dark appearances.
`,
  },
  {
    id: "03-google-sans-flex-and-code-typography",
    title: "Google Sans Flex & Monospace Code Pairing",
    sourceUrl: "https://design.google/library/google-sans-flex-font",
    content: `---
title: "Google Sans Flex & Monospace Code Pairing"
source: "https://design.google/library/google-sans-flex-font"
---

# Google Sans Flex & Monospace Code Pairing

- **Display & UI Chrome**: \`Google Sans Flex\` tuned across 6 continuous variable axes (\`opsz\` 6..144, \`wght\` 100..1000, \`wdth\` 25..151, \`GRAD\` -50..200, \`slnt\` -10..0, \`ROND\` 0..100).
- **Code, Disassembly & Telemetry**: \`JetBrains Mono\` with \`font-variant-numeric: tabular-nums\` so live frame counters, X64/A64 disassembly columns, and timestamps never jitter horizontally.
`,
  },
  {
    id: "04-spatial-and-effects-spring-motion",
    title: "Spatial vs Effects Spring Motion Physics",
    sourceUrl: "https://m3.material.io/",
    content: `---
title: "Spatial vs Effects Spring Motion Physics"
source: "https://m3.material.io/"
---

# Spatial vs Effects Spring Motion Physics

Implemented analytically in \`aphrody_ui::google_design::SpringSpec::sample_position\` and exported to CSS \`linear(...)\` in \`docs/aphrody/design/stylesheet.css\`:

- **Spatial Springs (\`damping_ratio = 0.9\`)**: \`spatial_fast\` (\`k=1400\`), \`spatial_default\` (\`k=700\`), \`spatial_slow\` (\`k=300\`) for bounds, drawers, and corner morphing.
- **Effects Springs (\`damping_ratio = 1.0\`, critically damped)**: \`effects_fast\` (\`k=3800\`), \`effects_default\` (\`k=1600\`), \`effects_slow\` (\`k=800\`) for opacity and color washes with zero overshoot.
`,
  },
  {
    id: "05-modern-web-and-wasm-webgpu-baseline",
    title: "Modern Web Guidance & Wasm/WebGPU Baseline",
    sourceUrl: "https://github.com/GoogleChrome/modern-web-guidance-src",
    content: `---
title: "Modern Web Guidance & Wasm/WebGPU Baseline"
source: "https://github.com/GoogleChrome/modern-web-guidance-src"
---

# Modern Web Guidance & Wasm/WebGPU Baseline

Audited automatically by \`aphrody_design::modern_web_guidance\` and \`web_standards::audit_html_standards\`:

- Semantic HTML5 landmarks (\`<header>\`, \`<nav>\`, \`<main>\`, \`<aside>\`), responsive \`<meta name="viewport">\`, zero \`document.write()\`, and \`--md-sys-*\` custom properties.
- Portable single-file exports compress session payloads with \`CompressionStream("gzip")\` / \`DecompressionStream("gzip")\` for instant offline startup.
`,
  },
  {
    id: "06-gpui-egui-bevy-component-catalog",
    title: "Unified GPUI, NSH, egui 0.37 & Bevy UE5.8 Component Catalog",
    sourceUrl: "https://design.google/?home",
    content: `---
title: "Unified GPUI, NSH, egui 0.37 & Bevy UE5.8 Component Catalog"
source: "https://design.google/?home"
---

# Unified GPUI, NSH, \`egui 0.37\` & Bevy UE5.8 Component Catalog

| Component Contract | GPUI Symbol | \`egui 0.37\` / Bevy Symbol | NSH Element |
| --- | --- | --- | --- |
| \`expressive-app-shell\` | \`aphrody_ui::render_nav_rail\` + \`Workspace\` | \`egui::SidePanel\` + \`CentralPanel\` | \`<NavStack>\` + \`<View>\` |
| \`expressive-pill-button\` | \`aphrody_ui::starter_pack::StarterButton\` | \`egui::Button\` (M3 \`CornerRadius\`) | \`<Press>\` / \`<Button>\` |
| \`surface-kpi-card\` | \`aphrody_ui::starter_pack::StarterKpiCard\` | \`egui::Frame::group\` | \`<SurfaceCard>\` |
| \`figcraft-canvas\` | \`aphrody_craft::render_gpui_studio_panel\` | \`aphrody_craft::CraftStudioSession::run_egui_equivalent\` | \`<FigCraftGoogleScreen>\` |
| \`unreal-bevy-viewport\` | \`bevy_gpui::render_viewport_rgba8\` | \`bevy_gpui::render_bevy_egui_inspector\` | \`<Shader>\` / \`<Canvas>\` |
`,
  },
  {
    id: "07-mirrored-google-references",
    title: "Mirrored Google References & Material 3 Matrix",
    sourceUrl: "https://design.google/library/transparent-screens",
    content: `---
title: "Mirrored Google References & Material 3 Matrix"
source: "https://design.google/library/transparent-screens"
---

# Mirrored Google References & Material 3 Matrix

Each reference is a value-like GPUI element in \`aphrody_ui::google_design_mirror\`. Brand colors resolve through \`MirrorAccentPalette\` (\`ref.palette\`, \`comp.*\`), never through literals at call sites. Owners keep the state and receive requested changes through callbacks.

| Reference | GPUI element | Theme | Behavior contract |
| --- | --- | --- | --- |
| [Transparent Screens](https://design.google/library/transparent-screens) | \`TransparentHudCard\` | \`xr-glimmer\` | Read-only; status carries a text label, not only a dot |
| [Gemini visual design](https://design.google/library/gemini-ai-visual-design) | \`GeminiSpectrumCard\` | \`gemini-ai\` | Thinking state is written out |
| [YouTube new red](https://design.google/library/youtube-new-red-color) | \`YoutubeGradientScrubber\` | \`youtube-red\` | Slider role; arrows ±5 s, Home/End, ±10 s buttons; \`on_seek\` reports the target |
| [Googlebook](https://googlebook.google/) | \`GlowbarShellCard\` | \`google-expressive\` | Entries keyed by domain id; \`on_select\` reports the id |
| [OpenBook Quire](https://github.com/Mhsbrian/OpenBook) | \`QuireGlassNote\` | \`openbook-quire\` | Per-step Undo; consent card only for deletion (\`Delete "Old drafts"?\`, Keep it / Delete); Esc stops the agent when \`on_dismiss\` is set |
| [Angular](https://material.angular.dev/components/categories) · [Flutter](https://docs.flutter.dev/ui/widgets/material) · [Compose](https://developer.android.com/jetpack/androidx/releases/compose-material) · [Figma M3 kit](https://www.figma.com/design/vEr6T5SQaJfMpLxjMUfD55/Material-3-Design-Kit--Community-?node-id=11-1833) | \`MaterialComponentMatrix\` | \`m3\` | Read-only six-row matrix |

The Figma kit archive is versioned as \`docs/aphrody/design/Material 3 Design Kit (Community).zip\`; \`figma-kit.json\` exposes it under \`material3Kit\`.
`,
  },
];

const COMPONENT_CONTRACTS = [
  {
    id: "youtube-gradient-scrubber",
    category: "data_display",
    gpuiSymbol: "aphrody_ui::google_design_mirror::YoutubeGradientScrubber",
    eguiEquivalentSymbol: "egui::Slider",
    nshElement: "MediaScrubber",
    stitchSlotId: "stitch-media-scrubber",
    tokensConsumed: ["--md-ref-palette-youtube-red", "--md-ref-palette-youtube-magenta", "--md-sys-color-surface-container"],
    states: ["default", "focused", "disabled"],
  },
  {
    id: "quire-glass-note",
    category: "overlay",
    gpuiSymbol: "aphrody_ui::google_design_mirror::QuireGlassNote",
    eguiEquivalentSymbol: "egui::Window",
    nshElement: "QuireNote",
    stitchSlotId: "stitch-quire-note",
    tokensConsumed: ["--md-ref-palette-quire-folio-light", "--md-ref-palette-quire-folio-dark", "--md-sys-color-error"],
    states: ["default", "focused", "disabled"],
  },
  {
    id: "expressive-app-shell",
    category: "shell",
    gpuiSymbol: "aphrody_ui::cosmic_shell::render_nav_rail",
    eguiEquivalentSymbol: "egui::SidePanel",
    nshElement: "AppShell",
    stitchSlotId: "stitch-agy-rail",
    tokensConsumed: ["--md-sys-color-surface", "--md-sys-color-surface-container-low"],
    states: ["default", "selected", "focused"],
  },
  {
    id: "expressive-pill-button",
    category: "navigation",
    gpuiSymbol: "aphrody_ui::starter_pack::StarterButton",
    eguiEquivalentSymbol: "egui::Button",
    nshElement: "PillButton",
    stitchSlotId: "stitch-figcraft-cta",
    tokensConsumed: ["--md-sys-color-primary", "--md-sys-color-on-primary", "--md-sys-shape-corner-full"],
    states: ["default", "hover", "pressed", "focused", "disabled"],
  },
  {
    id: "neural-omnibar-composer",
    category: "composer",
    gpuiSymbol: "ui_input::InputField",
    eguiEquivalentSymbol: "egui::TextEdit::singleline",
    nshElement: "NeuralOmnibar",
    stitchSlotId: "stitch-figcraft-omnibar",
    tokensConsumed: ["--md-sys-color-surface-container", "--md-sys-shape-corner-full"],
    states: ["default", "hover", "focused"],
  },
  {
    id: "cascading-menu-popover",
    category: "overlay",
    gpuiSymbol: "ui::ContextMenu",
    eguiEquivalentSymbol: "egui::MenuBar",
    nshElement: "CascadingPopover",
    stitchSlotId: "stitch-menu-popover",
    tokensConsumed: ["--md-sys-color-surface-container-high", "--md-sys-shape-corner-lg-inc"],
    states: ["default", "hover", "selected", "focused"],
  },
  {
    id: "grounding-card-stack",
    category: "data_display",
    gpuiSymbol: "aphrody_ui::starter_pack::StarterCard",
    eguiEquivalentSymbol: "egui::Frame",
    nshElement: "GroundingCard",
    stitchSlotId: "stitch-grounding-stack",
    tokensConsumed: ["--md-sys-color-surface-container-low", "--md-sys-shape-corner-xl"],
    states: ["default", "hover", "selected"],
  },
  {
    id: "telemetry-kpi-strip",
    category: "data_display",
    gpuiSymbol: "aphrody_ui::starter_pack::StarterKpiCard",
    eguiEquivalentSymbol: "egui_gpui::EguiFrameStats",
    nshElement: "KpiTile",
    stitchSlotId: "stitch-agy-kpis",
    tokensConsumed: ["--md-sys-color-primary", "--md-ref-typeface-mono"],
    states: ["default"],
  },
  {
    id: "figcraft-auto-layout-frame",
    category: "data_display",
    gpuiSymbol: "aphrody_craft::FigCraftFrameNode",
    eguiEquivalentSymbol: "egui_gpui::EguiGpuiHost",
    nshElement: "FigCraftGoogleScreen",
    stitchSlotId: "stitch-figcraft-root",
    tokensConsumed: ["--md-sys-color-surface", "--md-sys-color-primary-container"],
    states: ["default", "hover", "selected"],
  },
  {
    id: "unreal-bevy-solari-viewport",
    category: "viewport_3d",
    gpuiSymbol: "bevy_gpui::render_viewport_rgba8",
    eguiEquivalentSymbol: "bevy_gpui::render_bevy_egui_inspector",
    nshElement: "ViewportCanvas",
    stitchSlotId: "stitch-bevy-viewport",
    tokensConsumed: ["--md-sys-color-surface-dim", "--md-sys-color-tertiary"],
    states: ["default", "focused", "selected"],
  },
] as const;

export function buildDesignDocsPackage(outDir = DESIGN_DOCS_DIR) {
  mkdirSync(join(outDir, "svg"), { recursive: true });
  mkdirSync(join(outDir, "kits"), { recursive: true });
  mkdirSync(join(outDir, "guidelines"), { recursive: true });

  // Clean legacy Hydrogen filenames if present while keeping reference PNGs untouched.
  for (const legacy of [
    join(outDir, "design.md"),
    join(outDir, "tokens.css"),
    join(outDir, "guidelines/code-is-a-design-material.mdx"),
  ]) {
    if (existsSync(legacy)) rmSync(legacy, { force: true });
  }

  // Normalize any existing design-google-*.svg and write curated SVGs.
  const svgEntries: Array<{
    id: string;
    title: string;
    path: string;
    viewBox: string;
    bytes: number;
    sha256: string;
    a11yRoleImg: boolean;
  }> = [];

  for (const spec of CURATED_SVGS) {
    const raw = `<svg xmlns="http://www.w3.org/2000/svg" role="img" viewBox="${spec.viewBox}"><title>${spec.title}</title>${spec.body}</svg>`;
    const { clean, viewBox } = normalizeSvgMarkup(raw, spec.title);
    const full = join(outDir, "svg", `${spec.id}.svg`);
    writeFileSync(full, clean, "utf8");
    const buf = Buffer.from(clean, "utf8");
    svgEntries.push({
      id: spec.id,
      title: spec.title,
      path: "./" + relative(outDir, full).replaceAll("\\", "/"),
      viewBox,
      bytes: buf.length,
      sha256: sha256Hex(buf),
      a11yRoleImg: true,
    });
  }

  for (const f of readdirSync(join(outDir, "svg")).sort()) {
    if (!f.startsWith("design-google-") || !f.endsWith(".svg")) continue;
    const full = join(outDir, "svg", f);
    const raw = readFileSync(full, "utf8");
    const id = f.replace(/\.svg$/, "");
    const { clean, viewBox } = normalizeSvgMarkup(raw, `Google Design Home Vector (${id})`);
    writeFileSync(full, clean, "utf8");
    const buf = Buffer.from(clean, "utf8");
    svgEntries.push({
      id,
      title: `Google Design Home Vector (${id})`,
      path: `./svg/${f}`,
      viewBox,
      bytes: buf.length,
      sha256: sha256Hex(buf),
      a11yRoleImg: true,
    });
  }

  // Reference PNG captures inventory
  const refSpecs: Array<{
    file: string;
    family: "gemini-neural-home" | "gemini-cascading-menu" | "google-ai-mode" | "chrome-omnibar" | "chrome-157-grounding";
    patterns: string[];
  }> = [
    { file: "gemini-home.png", family: "gemini-neural-home", patterns: ["260px rail", "indigo radial glow", "Account preferences 20px popover", "Carbon High selector"] },
    { file: "gemini-opions.png", family: "gemini-cascading-menu", patterns: ["Cascading + attachment menu", "Import code & Notebooks submenu"] },
    { file: "google-ai-mode.png", family: "google-ai-mode", patterns: ["48px collapsed rail", "Mode IA segmented bar", "4-color thinking aura"] },
    { file: "google-in-chrome.png", family: "chrome-omnibar", patterns: ["Pill attachment composer", "Thumbnail chips", "Primary circular submit"] },
    { file: "win11-chrome157.png", family: "chrome-157-grounding", patterns: ["24px source grounding card stack", "64x64 rounded media thumb", "Modern Web Guidance 1.0"] },
  ];

  const r2ByPath = new Map<string, { sha256: string; bytes: number; public_url: string; r2_key: string }>();
  const r2ManifestPath = join(repoRoot, "assets/os/r2_assets_manifest.toml");
  if (existsSync(r2ManifestPath)) {
    const parsedR2 = Bun.TOML.parse(readFileSync(r2ManifestPath, "utf8")) as {
      assets?: Array<{ logical_path: string; sha256: string; bytes: number; public_url: string; r2_key: string }>;
    };
    for (const item of parsedR2.assets ?? []) {
      r2ByPath.set(item.logical_path, item);
    }
  }

  const referenceCaptures = refSpecs
    .map((r) => {
      const full = join(outDir, r.file);
      const cached = join(repoRoot, "target/r2-cache/docs/aphrody/design", r.file);
      const r2Entry = r2ByPath.get(`docs/aphrody/design/${r.file}`) ?? r2ByPath.get(`docs/design/${r.file}`);
      if (existsSync(full)) {
        const bytes = readFileSync(full);
        return {
          id: r.file.replace(/\.png$/, ""),
          path: `./${r.file}`,
          r2Url: r2Entry?.public_url,
          bytes: statSync(full).size,
          sha256: sha256Hex(bytes),
          surfaceFamily: r.family,
          keyPatterns: r.patterns,
        };
      }
      if (existsSync(cached)) {
        const bytes = readFileSync(cached);
        return {
          id: r.file.replace(/\.png$/, ""),
          path: r2Entry?.public_url ?? `./${r.file}`,
          r2Url: r2Entry?.public_url,
          bytes: statSync(cached).size,
          sha256: sha256Hex(bytes),
          surfaceFamily: r.family,
          keyPatterns: r.patterns,
        };
      }
      if (r2Entry) {
        return {
          id: r.file.replace(/\.png$/, ""),
          path: r2Entry.public_url,
          r2Url: r2Entry.public_url,
          bytes: r2Entry.bytes,
          sha256: r2Entry.sha256,
          surfaceFamily: r.family,
          keyPatterns: r.patterns,
        };
      }
      return null;
    })
    .filter((entry): entry is NonNullable<typeof entry> => entry !== null);

  // Stylesheet, DESIGN.md, Schema, Kits, MDX Guidelines
  writeFileSync(join(outDir, "schema.ts"), buildSchemaTsSource(), "utf8");
  const css = buildStylesheetCss();
  writeFileSync(join(outDir, "stylesheet.css"), css, "utf8");
  const dmd = buildDesignMd();
  writeFileSync(join(outDir, "DESIGN.md"), dmd, "utf8");

  const figmaObj = buildFigmaKitJson() as { collections: object; componentSets: unknown[] };
  const figmaJson = JSON.stringify(figmaObj, null, 2) + "\n";
  writeFileSync(join(outDir, "kits/figma-design-kit.json"), figmaJson, "utf8");

  const codeConnectObj = buildFigmaCodeConnectJson() as { mappings: unknown[] };
  const codeConnectJson = JSON.stringify(codeConnectObj, null, 2) + "\n";
  writeFileSync(join(outDir, "kits/figma-code-connect.json"), codeConnectJson, "utf8");

  const { entries: maquetteEntries } = buildAllMaquettes(outDir, repoRoot);

  const stitchObj = buildStitchKitJson(maquetteEntries) as { screens: unknown[]; preserveDataStitchId: boolean };
  const stitchJson = JSON.stringify(stitchObj, null, 2) + "\n";
  writeFileSync(join(outDir, "kits/stitch-design-kit.json"), stitchJson, "utf8");

  const guidelineEntries = GUIDELINES_MDX.map((g) => {
    const rel = `./guidelines/${g.id}.mdx`;
    const full = join(outDir, "guidelines", `${g.id}.mdx`);
    writeFileSync(full, g.content, "utf8");
    const buf = Buffer.from(g.content, "utf8");
    return {
      id: g.id,
      title: g.title,
      path: rel,
      sourceUrl: g.sourceUrl,
      bytes: buf.length,
      sha256: sha256Hex(buf),
    };
  });

  const tokens = buildCanonicalDesignTokens();
  const darkContrast = Number(
    contrastRatioHex(tokens.sys.color.dark.on_primary, tokens.sys.color.dark.primary).toFixed(2),
  );
  const lightContrast = Number(
    contrastRatioHex(tokens.sys.color.light.on_primary, tokens.sys.color.light.primary).toFixed(2),
  );

  const manifest = {
    schemaVersion: 2 as const,
    name: "aphrody-google-neural-expressive-design-pack",
    sourceOfTruth: "https://design.google/" as const,
    homeUrl: "https://design.google/?home" as const,
    codeAsMaterialUrl: "https://design.google/library/code-is-a-design-material" as const,
    stitchUrl: "https://stitch.withgoogle.com/" as const,
    stylesheetPath: "./stylesheet.css",
    designMdPath: "./DESIGN.md",
    wcagContrastDarkPrimary: darkContrast,
    wcagContrastLightPrimary: lightContrast,
    referenceCaptures,
    svgs: svgEntries,
    guidelines: guidelineEntries,
    kits: {
      figma: {
        path: "./kits/figma-design-kit.json",
        sha256: sha256Hex(figmaJson),
        collectionCount: Object.keys(figmaObj.collections).length,
        componentCount: figmaObj.componentSets.length,
      },
      figmaCodeConnect: {
        path: "./kits/figma-code-connect.json",
        sha256: sha256Hex(codeConnectJson),
        mappingCount: codeConnectObj.mappings.length,
      },
      stitch: {
        path: "./kits/stitch-design-kit.json",
        sha256: sha256Hex(stitchJson),
        screenCount: stitchObj.screens.length,
        preserveDataStitchId: stitchObj.preserveDataStitchId,
      },
    },
    maquettes: maquetteEntries,
    components: COMPONENT_CONTRACTS,
  };

  writeFileSync(join(outDir, "manifest.json"), JSON.stringify(manifest, null, 2) + "\n", "utf8");
  return manifest;
}

if (import.meta.main) {
  const cmd = Bun.argv[2] ?? "build";
  if (cmd === "capture") {
    const captureRes = await captureMaquetteScreenshots(repoRoot);
    console.log(JSON.stringify(captureRes, null, 2));
    if (!captureRes.ok) process.exit(1);
    process.exit(0);
  }
  const manifest =
    cmd === "build"
      ? buildDesignDocsPackage()
      : JSON.parse(readFileSync(join(DESIGN_DOCS_DIR, "manifest.json"), "utf8"));
  const { validateDesignManifest } = await import(join(DESIGN_DOCS_DIR, "schema.ts"));
  const check = validateDesignManifest(manifest);
  console.log(
    JSON.stringify(
      {
        ok: check.ok,
        svgs: manifest.svgs.length,
        referenceCaptures: manifest.referenceCaptures.length,
        guidelines: manifest.guidelines.length,
        maquettes: manifest.maquettes?.length ?? MAQUETTE_DEFINITIONS.length,
        components: manifest.components.length,
        errors: check.errors,
      },
      null,
      2,
    ),
  );
  if (!check.ok) process.exit(1);
}
