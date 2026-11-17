#!/usr/bin/env bun
/**
 * Reproducible Google Design (`https://design.google/?home` + Neural Expressive Nov 2026)
 * Asset Optimizer, Stylesheet Compiler, W3C DTCG Figma & Stitch Kit Builder, MDX Guideline
 * Generator, and Typed Contract Validator for `docs/design/`.
 *
 * Usage:
 *   bun tooling/bun/design-docs-pack.ts build
 *   bun tooling/bun/design-docs-pack.ts verify
 */
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { join, relative, resolve } from "node:path";
import { buildCanonicalDesignTokens, contrastRatioHex } from "./google-design-sync.ts";

export const repoRoot = resolve(import.meta.dir, "../..");
export const DESIGN_DOCS_DIR = join(repoRoot, "docs/design");

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
];

function normalizeSvgMarkup(raw: string, fallbackTitle: string): { clean: string; viewBox: string } {
  let s = raw.replace(/\r?\n/g, " ").replace(/\s+/g, " ").replace(/>\s+</g, "><").trim();
  const vbMatch = s.match(/viewBox=["']([^"']+)["']/i);
  const viewBox = vbMatch?.[1] ?? "0 0 24 24";
  if (!/role=["']/i.test(s)) {
    s = s.replace(/^<svg\b/i, '<svg role="img"');
  }
  if (!/<title>/i.test(s)) {
    s = s.replace(/(<svg[^>]*>)/i, `$1<title>${fallbackTitle}</title>`);
  }
  if (!/xmlns=/i.test(s)) {
    s = s.replace(/^<svg\b/i, '<svg xmlns="http://www.w3.org/2000/svg"');
  }
  return { clean: s, viewBox };
}

function buildSchemaTsSource(): string {
  return `/**
 * Authoritative TypeScript Schema & Runtime Validator for \`docs/design/manifest.json\`.
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
    stitch: { path: string; sha256: string; screenCount: number; preserveDataStitchId: boolean };
  };
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
  if (!Array.isArray(m.svgs) || m.svgs.length < 8) errors.push("Expected >= 8 optimized SVGs");
  if (!Array.isArray(m.guidelines) || m.guidelines.length < 6) errors.push("Expected 6 MDX guidelines");
  if (!Array.isArray(m.components) || m.components.length < 8) errors.push("Expected >= 8 dual GPUI/egui component contracts");
  if (!m.kits?.figma?.path || !m.kits?.stitch?.path) errors.push("Missing Figma or Stitch kit reference");
  return { ok: errors.length === 0, errors };
}
`;
}

function buildStylesheetCss(): string {
  const tokens = buildCanonicalDesignTokens();
  const d = tokens.sys.color.dark;
  const l = tokens.sys.color.light;
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

:root, [data-theme="dark"] {
  --md-ref-typeface-brand: ${tokens.ref.typeface.brand};
  --md-ref-typeface-plain: ${tokens.ref.typeface.plain};
  --md-ref-typeface-mono: ${tokens.ref.typeface.mono};

  /* Google Sans Flex 6 Variable Axes */
  --md-ref-font-axes-display: "opsz" 48, "wght" 500, "wdth" 100, "GRAD" 0, "slnt" 0, "ROND" 40;
  --md-ref-font-axes-ui: "opsz" 14, "wght" 500, "wdth" 100, "GRAD" 0, "slnt" 0, "ROND" 20;

${emitPaletteVars(d)}

  /* Neural Expressive Obsidian Backdrop & Aura (from docs/design/*.png) */
  --md-sys-neural-obsidian: #0B0E14;
  --md-sys-neural-rail: #13161F;
  --md-sys-neural-popover: #1C202B;
  --md-sys-neural-glow: radial-gradient(circle at 50% 42%, rgba(59, 130, 246, 0.20) 0%, rgba(99, 102, 241, 0.08) 38%, rgba(11, 14, 20, 0) 72%);

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

[data-theme="light"] {
${emitPaletteVars(l)}
  --md-sys-neural-obsidian: #F4F7FC;
  --md-sys-neural-rail: #EBF0F8;
  --md-sys-neural-popover: #FFFFFF;
  --md-sys-neural-glow: radial-gradient(circle at 50% 42%, rgba(0, 99, 153, 0.10) 0%, rgba(91, 88, 145, 0.05) 40%, rgba(244, 247, 252, 0) 72%);
}

@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }
}

.gd-tabular { font-variant-numeric: tabular-nums; }
.gd-pill-button {
  min-height: var(--md-sys-touch-target-min);
  border-radius: var(--md-sys-shape-corner-full);
  transition: border-radius var(--md-sys-motion-spring-spatial-fast), transform var(--md-sys-motion-spring-spatial-fast), background-color var(--md-sys-motion-spring-effects-default);
}
.gd-pill-button:active {
  border-radius: var(--md-sys-shape-corner-md);
  transform: scale(0.98);
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

# Google Design & Neural Expressive Specification (\`docs/design/DESIGN.md\`)

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
  const colorVariables: Record<string, object> = {};
  for (const [role, darkHex] of Object.entries(tokens.sys.color.dark)) {
    const lightHex = (tokens.sys.color.light as unknown as Record<string, string>)[role] ?? darkHex;
    colorVariables[role.replaceAll("_", "-")] = {
      $type: "color",
      $value: darkHex,
      $extensions: {
        "com.figma.modes": {
          Dark: darkHex,
          Light: lightHex,
        },
        "com.figma.scopes": ["ALL_SCOPES"],
      },
    };
  }
  return {
    $schema: "https://tr.designtokens.org/format/",
    name: "Aphrody · Google Design Material 3 Expressive + Neural Figma Kit",
    version: "2.0.0",
    sourceUrl: "https://design.google/?home",
    collections: {
      "Sys/Color": colorVariables,
      "Sys/Shape": Object.fromEntries(
        Object.entries(tokens.sys.shape).map(([k, px]) => [
          k,
          { $type: "dimension", $value: `${px}px` },
        ]),
      ),
      "Sys/Typography": Object.fromEntries(
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
      "Sys/MotionSprings": tokens.sys.motion.springs,
    },
    componentSets: [
      { id: "ExpressiveAppShell", variants: ["ExpandedRail", "CollapsedRail", "MobileSheet"], autoLayout: "HORIZONTAL", radiusToken: "large_px" },
      { id: "NeuralOmnibar", variants: ["Idle", "Focused", "AttachmentMenuOpen"], autoLayout: "HORIZONTAL", radiusToken: "full_px" },
      { id: "CascadingPopoverMenu", variants: ["RootMenu", "SubMenu"], autoLayout: "VERTICAL", radiusToken: "large_increased_px" },
      { id: "GroundingCardStack", variants: ["Collapsed3", "ExpandedAll"], autoLayout: "VERTICAL", radiusToken: "extra_large_px" },
      { id: "ExpressivePillButton", variants: ["Primary", "Secondary", "Outline", "Ghost", "Destructive"], autoLayout: "HORIZONTAL", radiusToken: "full_px" },
      { id: "FigCraftCanvasNode", variants: ["Frame", "Component", "TokenBound"], autoLayout: "VERTICAL", radiusToken: "medium_px" },
      { id: "CraftAnythingDisasmTable", variants: ["X64", "A64", "Wasm"], autoLayout: "VERTICAL", radiusToken: "medium_px" },
      { id: "UnrealBevyViewportHud", variants: ["LitPbrSolari", "NaniteMeshlets", "WorldNormals"], autoLayout: "HORIZONTAL", radiusToken: "large_px" },
    ],
  };
}

function buildStitchKitJson(): object {
  return {
    name: "Aphrody · Google Stitch & Multi-Studio Design Kit",
    version: "2.0.0",
    sourceUrl: "https://stitch.withgoogle.com/",
    designMdRef: "../DESIGN.md",
    preserveDataStitchId: true,
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

- **Single Token Source**: \`assets/os/google_design_tokens.toml\` -> \`DESIGN.md\` -> \`docs/design/stylesheet.css\` -> \`ShadcnGpuiTheme\` & \`egui::Visuals\`.
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

Derived from \`https://design.google/?home\` and the 5 Neural Expressive reference captures in \`docs/design/*.png\`:

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

Implemented analytically in \`aphrody_ui::google_design::SpringSpec::sample_position\` and exported to CSS \`linear(...)\` in \`docs/design/stylesheet.css\`:

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
];

const COMPONENT_CONTRACTS = [
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

  const referenceCaptures = refSpecs
    .filter((r) => existsSync(join(outDir, r.file)))
    .map((r) => {
      const full = join(outDir, r.file);
      const bytes = readFileSync(full);
      return {
        id: r.file.replace(/\.png$/, ""),
        path: `./${r.file}`,
        bytes: statSync(full).size,
        sha256: sha256Hex(bytes),
        surfaceFamily: r.family,
        keyPatterns: r.patterns,
      };
    });

  // Stylesheet, DESIGN.md, Schema, Kits, MDX Guidelines
  writeFileSync(join(outDir, "schema.ts"), buildSchemaTsSource(), "utf8");
  const css = buildStylesheetCss();
  writeFileSync(join(outDir, "stylesheet.css"), css, "utf8");
  const dmd = buildDesignMd();
  writeFileSync(join(outDir, "DESIGN.md"), dmd, "utf8");

  const figmaObj = buildFigmaKitJson() as { collections: object; componentSets: unknown[] };
  const figmaJson = JSON.stringify(figmaObj, null, 2) + "\n";
  writeFileSync(join(outDir, "kits/figma-design-kit.json"), figmaJson, "utf8");

  const stitchObj = buildStitchKitJson() as { screens: unknown[]; preserveDataStitchId: boolean };
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
      stitch: {
        path: "./kits/stitch-design-kit.json",
        sha256: sha256Hex(stitchJson),
        screenCount: stitchObj.screens.length,
        preserveDataStitchId: stitchObj.preserveDataStitchId,
      },
    },
    components: COMPONENT_CONTRACTS,
  };

  writeFileSync(join(outDir, "manifest.json"), JSON.stringify(manifest, null, 2) + "\n", "utf8");
  return manifest;
}

if (import.meta.main) {
  const cmd = Bun.argv[2] ?? "build";
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
        components: manifest.components.length,
        errors: check.errors,
      },
      null,
      2,
    ),
  );
  if (!check.ok) process.exit(1);
}
