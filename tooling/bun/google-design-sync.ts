#!/usr/bin/env bun
/**
 * Google Design (https://design.google/) Material 3 Expressive, Google Sans Flex,
 * DESIGN.md, Modern Web Guidance, and Google Stitch Sync & Audit CLI.
 *
 * Usage:
 *   bun tooling/bun/google-design-sync.ts sync [--offline] [--out assets/os/google_design_tokens.toml]
 *   bun tooling/bun/google-design-sync.ts audit
 */
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

export const repoRoot = resolve(import.meta.dir, "../..");
export const DESIGN_SOURCE_URL = "https://design.google/";
export const STITCH_SOURCE_URL = "https://stitch.withgoogle.com/";
export const DEFAULT_TOKENS_PATH = join(repoRoot, "assets/os/google_design_tokens.toml");
export const DEFAULT_SITEMAP_SNAPSHOT_PATH = join(
  repoRoot,
  "assets/os/design_google_sitemap_snapshot.toml",
);
export const DEFAULT_DESIGN_MD_PATH = join(repoRoot, "DESIGN.md");
export const DEFAULT_THEMES_PATH = join(repoRoot, "assets/os/themes.json");

export interface RgbColor {
  r: number;
  g: number;
  b: number;
}

export function parseHexRgb(hex: string): RgbColor | null {
  const clean = hex.trim().replace(/^#/, "");
  if (clean.length !== 6 || !/^[0-9a-fA-F]{6}$/.test(clean)) return null;
  return {
    r: Number.parseInt(clean.slice(0, 2), 16),
    g: Number.parseInt(clean.slice(2, 4), 16),
    b: Number.parseInt(clean.slice(4, 6), 16),
  };
}

export function relativeLuminance(c: RgbColor): number {
  const linearize = (channel: number) => {
    const s = channel / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * linearize(c.r) + 0.7152 * linearize(c.g) + 0.0722 * linearize(c.b);
}

export function contrastRatioHex(fgHex: string, bgHex: string): number {
  const fg = parseHexRgb(fgHex);
  const bg = parseHexRgb(bgHex);
  if (!fg || !bg) return 0;
  const l1 = relativeLuminance(fg);
  const l2 = relativeLuminance(bg);
  const hi = Math.max(l1, l2);
  const lo = Math.min(l1, l2);
  return (hi + 0.05) / (lo + 0.05);
}

export interface SpringToken {
  stiffness: number;
  damping_ratio: number;
}

export interface VariableAxisSpec {
  tag: string;
  name: string;
  min: number;
  default_value: number;
  max: number;
}

export interface ColorPaletteRoleSet {
  background: string;
  on_background: string;
  surface: string;
  surface_dim: string;
  surface_bright: string;
  surface_container_lowest: string;
  surface_container_low: string;
  surface_container: string;
  surface_container_high: string;
  surface_container_highest: string;
  on_surface: string;
  on_surface_variant: string;
  outline: string;
  outline_variant: string;
  primary: string;
  on_primary: string;
  primary_container: string;
  on_primary_container: string;
  secondary: string;
  on_secondary: string;
  secondary_container: string;
  on_secondary_container: string;
  tertiary: string;
  on_tertiary: string;
  tertiary_container: string;
  on_tertiary_container: string;
  error: string;
  on_error: string;
  error_container: string;
  on_error_container: string;
}

export interface GoogleDesignTokenDocument {
  schema_version: number;
  source_of_truth: string;
  system: string;
  spec_url: string;
  fonts_url: string;
  stitch_url: string;
  sans_flex_family: string;
  synced_at: string;
  seed_hex: string;
  variant: string;
  principles: string[];
  ref: {
    palette: Record<string, string>;
    typeface: Record<string, string>;
    variable_axes: Record<string, VariableAxisSpec>;
  };
  sys: {
    color: {
      light: ColorPaletteRoleSet;
      dark: ColorPaletteRoleSet;
    };
    motion: {
      springs: Record<string, SpringToken>;
    };
    shape: Record<string, number>;
    state: Record<string, number>;
    density: Record<string, number>;
    window_size_classes: Record<string, number>;
    typescale: Record<
      string,
      { font: string; size_sp: number; line_height_sp: number; weight: number; tracking_sp: number }
    >;
  };
  comp: Record<string, Record<string, string | number>>;
}

export function buildCanonicalDesignTokens(
  syncedAt = new Date().toISOString(),
): GoogleDesignTokenDocument {
  return {
    schema_version: 1,
    source_of_truth: DESIGN_SOURCE_URL,
    system: "Material 3 Expressive",
    spec_url: "https://m3.material.io/",
    fonts_url: "https://fonts.google.com/",
    stitch_url: STITCH_SOURCE_URL,
    sans_flex_family: "Google Sans Flex",
    synced_at: syncedAt,
    seed_hex: "#135B82",
    variant: "expressive",
    principles: [
      "Code is a design material: interactive software bridges intent and visual craft",
      "Google Sans Flex optical size, weight, width, grade, slant, and roundness tune hierarchy",
      "Strict 3-tier design tokens (reference -> system -> component) exported via DESIGN.md",
      "Physics-based spring motion separated cleanly into spatial vs effects springs",
      "WCAG 2.2 AA contrast >= 4.5:1 across all surface and action foregrounds",
    ],
    ref: {
      palette: {
        primary0: "#000000",
        primary10: "#001D32",
        primary20: "#003352",
        primary30: "#004B75",
        primary40: "#006399",
        primary80: "#8FD0FF",
        primary90: "#CEE5FF",
        primary100: "#FFFFFF",
        secondary30: "#34485F",
        secondary40: "#4B6078",
        secondary80: "#B4C9E5",
        secondary90: "#D2E4FF",
        tertiary30: "#434077",
        tertiary40: "#5B5891",
        tertiary80: "#C6C2FF",
        tertiary90: "#E2DFFF",
        neutral6: "#0E141B",
        neutral10: "#161C24",
        neutral90: "#D9E1EC",
        neutral98: "#F7F9FF",
        neutral100: "#FFFFFF",
        error30: "#93000A",
        error40: "#BA1A1A",
        error80: "#FFB4AB",
        error90: "#FFDAD6",
      },
      typeface: {
        brand: "Google Sans Flex, Google Sans, Inter, system-ui, sans-serif",
        plain: "Google Sans Flex, Inter, Google Sans Text, system-ui, sans-serif",
        mono: "JetBrains Mono, Roboto Mono, monospace",
        fallback: "Noto Sans, sans-serif",
      },
      variable_axes: {
        opsz: { tag: "opsz", name: "Optical Size", min: 6, default_value: 14, max: 144 },
        wght: { tag: "wght", name: "Weight", min: 100, default_value: 400, max: 1000 },
        wdth: { tag: "wdth", name: "Width", min: 25, default_value: 100, max: 151 },
        GRAD: { tag: "GRAD", name: "Grade", min: -50, default_value: 0, max: 200 },
        slnt: { tag: "slnt", name: "Slant", min: -10, default_value: 0, max: 0 },
        ROND: { tag: "ROND", name: "Roundness", min: 0, default_value: 0, max: 100 },
      },
    },
    sys: {
      color: {
        light: {
          background: "#F7F9FF",
          on_background: "#161C24",
          surface: "#F7F9FF",
          surface_dim: "#D5DBE4",
          surface_bright: "#FFFFFF",
          surface_container_lowest: "#FFFFFF",
          surface_container_low: "#EEF3FA",
          surface_container: "#E7EDF6",
          surface_container_high: "#E0E7F1",
          surface_container_highest: "#D9E1EC",
          on_surface: "#161C24",
          on_surface_variant: "#3D4754",
          outline: "#6E7886",
          outline_variant: "#BEC7D4",
          primary: "#006399",
          on_primary: "#FFFFFF",
          primary_container: "#CEE5FF",
          on_primary_container: "#004972",
          secondary: "#4B6078",
          on_secondary: "#FFFFFF",
          secondary_container: "#D2E4FF",
          on_secondary_container: "#34485F",
          tertiary: "#5B5891",
          on_tertiary: "#FFFFFF",
          tertiary_container: "#E2DFFF",
          on_tertiary_container: "#434077",
          error: "#BA1A1A",
          on_error: "#FFFFFF",
          error_container: "#FFDAD6",
          on_error_container: "#93000A",
        },
        dark: {
          background: "#0E141B",
          on_background: "#E0E6F0",
          surface: "#0E141B",
          surface_dim: "#0B1016",
          surface_bright: "#343B44",
          surface_container_lowest: "#080D13",
          surface_container_low: "#161D25",
          surface_container: "#1B222B",
          surface_container_high: "#252C36",
          surface_container_highest: "#303843",
          on_surface: "#E0E6F0",
          on_surface_variant: "#C0CAD7",
          outline: "#8A94A2",
          outline_variant: "#404956",
          primary: "#8FD0FF",
          on_primary: "#003352",
          primary_container: "#004B75",
          on_primary_container: "#CEE5FF",
          secondary: "#B4C9E5",
          on_secondary: "#1D3248",
          secondary_container: "#34485F",
          on_secondary_container: "#D2E4FF",
          tertiary: "#C6C2FF",
          on_tertiary: "#2C295F",
          tertiary_container: "#434077",
          on_tertiary_container: "#E2DFFF",
          error: "#FFB4AB",
          on_error: "#690005",
          error_container: "#93000A",
          on_error_container: "#FFDAD6",
        },
      },
      motion: {
        springs: {
          spatial_fast: { stiffness: 1400.0, damping_ratio: 0.9 },
          spatial_default: { stiffness: 700.0, damping_ratio: 0.9 },
          spatial_slow: { stiffness: 300.0, damping_ratio: 0.9 },
          effects_fast: { stiffness: 3800.0, damping_ratio: 1.0 },
          effects_default: { stiffness: 1600.0, damping_ratio: 1.0 },
          effects_slow: { stiffness: 800.0, damping_ratio: 1.0 },
        },
      },
      shape: {
        none_px: 0,
        extra_small_px: 4,
        small_px: 8,
        medium_px: 12,
        large_px: 16,
        large_increased_px: 20,
        extra_large_px: 28,
        full_px: 9999,
      },
      state: {
        hover_opacity: 0.08,
        focus_opacity: 0.1,
        pressed_opacity: 0.1,
        dragged_opacity: 0.16,
        disabled_container_opacity: 0.12,
        disabled_content_opacity: 0.38,
        focus_ring_px: 2,
      },
      density: {
        default: 0,
        compact_1: -1,
        compact_2: -2,
        compact_3: -3,
        touch_target_min_px: 44,
        control_height_min_px: 36,
      },
      window_size_classes: {
        compact_max_px: 599,
        medium_min_px: 600,
        expanded_min_px: 840,
        large_min_px: 1200,
        extra_large_min_px: 1600,
      },
      typescale: {
        display_large: {
          font: "brand",
          size_sp: 57,
          line_height_sp: 64,
          weight: 400,
          tracking_sp: -0.25,
        },
        display_medium: {
          font: "brand",
          size_sp: 45,
          line_height_sp: 52,
          weight: 400,
          tracking_sp: 0,
        },
        display_small: {
          font: "brand",
          size_sp: 36,
          line_height_sp: 44,
          weight: 400,
          tracking_sp: 0,
        },
        headline_large: {
          font: "brand",
          size_sp: 32,
          line_height_sp: 40,
          weight: 500,
          tracking_sp: 0,
        },
        headline_medium: {
          font: "brand",
          size_sp: 28,
          line_height_sp: 36,
          weight: 500,
          tracking_sp: 0,
        },
        headline_small: {
          font: "brand",
          size_sp: 24,
          line_height_sp: 32,
          weight: 500,
          tracking_sp: 0,
        },
        title_large: {
          font: "brand",
          size_sp: 22,
          line_height_sp: 28,
          weight: 500,
          tracking_sp: 0,
        },
        title_medium: {
          font: "plain",
          size_sp: 16,
          line_height_sp: 24,
          weight: 600,
          tracking_sp: 0.15,
        },
        title_small: {
          font: "plain",
          size_sp: 14,
          line_height_sp: 20,
          weight: 600,
          tracking_sp: 0.1,
        },
        body_large: {
          font: "plain",
          size_sp: 16,
          line_height_sp: 24,
          weight: 400,
          tracking_sp: 0.5,
        },
        body_medium: {
          font: "plain",
          size_sp: 14,
          line_height_sp: 20,
          weight: 400,
          tracking_sp: 0.25,
        },
        body_small: {
          font: "plain",
          size_sp: 12,
          line_height_sp: 16,
          weight: 400,
          tracking_sp: 0.4,
        },
        label_large: {
          font: "plain",
          size_sp: 14,
          line_height_sp: 20,
          weight: 600,
          tracking_sp: 0.1,
        },
        label_medium: {
          font: "plain",
          size_sp: 12,
          line_height_sp: 16,
          weight: 600,
          tracking_sp: 0.5,
        },
        label_small: {
          font: "plain",
          size_sp: 11,
          line_height_sp: 16,
          weight: 600,
          tracking_sp: 0.5,
        },
      },
    },
    comp: {
      button: {
        container_shape: "full_px",
        pressed_shape: "medium_px",
        container_height_px: 40,
        focus_indicator_thickness_px: 2,
      },
      card: {
        container_color_role: "surface_container_low",
        elevated_color_role: "surface_container_high",
        shape: "large_px",
        padding_px: 16,
      },
      navigation_rail: {
        width_collapsed_px: 80,
        active_indicator_color_role: "secondary_container",
        active_indicator_shape: "full_px",
      },
      dialog: {
        container_color_role: "surface_container_high",
        shape: "extra_large_px",
        padding_px: 24,
      },
      toolbar: {
        height_px: 48,
        container_color_role: "surface_container",
        shape: "large_increased_px",
      },
      code_surface: {
        container_color_role: "surface_container_lowest",
        font_family_role: "mono",
        shape: "medium_px",
      },
      shader_canvas: {
        backdrop_color_role: "surface_dim",
        accent_color_role: "primary",
        secondary_accent_color_role: "tertiary",
        shape: "large_px",
      },
    },
  };
}

function serializeKvTable(header: string, record: Record<string, string | number>): string[] {
  const lines = [header];
  for (const [k, v] of Object.entries(record)) {
    lines.push(`${k} = ${typeof v === "string" ? JSON.stringify(v) : v}`);
  }
  lines.push("");
  return lines;
}

export function serializeDesignTokensToml(doc: GoogleDesignTokenDocument): string {
  const lines: string[] = [
    "# Authoritative 3-tier Design Tokens anchored to https://design.google/ (Material 3 Expressive + Google Sans Flex).",
    "# Generated by tooling/bun/google-design-sync.ts sync — re-run `bun run design:sync` after updates.",
    `schema_version = ${doc.schema_version}`,
    `source_of_truth = ${JSON.stringify(doc.source_of_truth)}`,
    `system = ${JSON.stringify(doc.system)}`,
    `spec_url = ${JSON.stringify(doc.spec_url)}`,
    `fonts_url = ${JSON.stringify(doc.fonts_url)}`,
    `stitch_url = ${JSON.stringify(doc.stitch_url)}`,
    `sans_flex_family = ${JSON.stringify(doc.sans_flex_family)}`,
    `synced_at = ${JSON.stringify(doc.synced_at)}`,
    `seed_hex = ${JSON.stringify(doc.seed_hex)}`,
    `variant = ${JSON.stringify(doc.variant)}`,
    `principles = [${doc.principles.map((p) => JSON.stringify(p)).join(", ")}]`,
    "",
    ...serializeKvTable("[ref.palette]", doc.ref.palette),
    ...serializeKvTable("[ref.typeface]", doc.ref.typeface),
    ...serializeKvTable("[sys.color.light]", doc.sys.color.light as unknown as Record<string, string>),
    ...serializeKvTable("[sys.color.dark]", doc.sys.color.dark as unknown as Record<string, string>),
    ...serializeKvTable("[sys.shape]", doc.sys.shape),
    ...serializeKvTable("[sys.state]", doc.sys.state),
    ...serializeKvTable("[sys.density]", doc.sys.density),
    ...serializeKvTable("[sys.window_size_classes]", doc.sys.window_size_classes),
  ];

  for (const [name, axis] of Object.entries(doc.ref.variable_axes)) {
    lines.push(`[ref.variable_axes.${name}]`);
    lines.push(`tag = ${JSON.stringify(axis.tag)}`);
    lines.push(`name = ${JSON.stringify(axis.name)}`);
    lines.push(`min = ${axis.min}`);
    lines.push(`default_value = ${axis.default_value}`);
    lines.push(`max = ${axis.max}`);
    lines.push("");
  }

  for (const [name, spring] of Object.entries(doc.sys.motion.springs)) {
    lines.push(`[sys.motion.springs.${name}]`);
    lines.push(`stiffness = ${spring.stiffness.toFixed(1)}`);
    lines.push(`damping_ratio = ${spring.damping_ratio.toFixed(2)}`);
    lines.push("");
  }

  for (const [name, scale] of Object.entries(doc.sys.typescale)) {
    lines.push(`[sys.typescale.${name}]`);
    lines.push(`font = ${JSON.stringify(scale.font)}`);
    lines.push(`size_sp = ${scale.size_sp}`);
    lines.push(`line_height_sp = ${scale.line_height_sp}`);
    lines.push(`weight = ${scale.weight}`);
    lines.push(`tracking_sp = ${scale.tracking_sp}`);
    lines.push("");
  }

  for (const [compName, props] of Object.entries(doc.comp)) {
    lines.push(...serializeKvTable(`[comp.${compName}]`, props));
  }

  return lines.join("\n");
}

export interface DesignArticleSnapshot {
  url: string;
  title: string;
  description: string;
  headings: string[];
  summary: string;
}

export interface SitemapSnapshotDocument {
  schema_version: number;
  synced_at: string;
  design_google_sitemap_url: string;
  stitch_url: string;
  total_urls: number;
  urls: Array<{ loc: string; lastmod: string; priority: string }>;
  featured_articles: DesignArticleSnapshot[];
  upstream_ecosystem: Array<{ repo: string; url: string; license: string; role: string }>;
  stitch_routes: Array<{ path: string; title: string; capability: string }>;
}

export function parseSitemapXmlUrls(
  xml: string,
): Array<{ loc: string; lastmod: string; priority: string }> {
  const rows: Array<{ loc: string; lastmod: string; priority: string }> = [];
  for (const m of xml.matchAll(/<url>([\s\S]*?)<\/url>/gi)) {
    const block = m[1];
    const loc = /<loc>([^<]+)<\/loc>/i.exec(block)?.[1]?.trim() ?? "";
    const lastmod = /<lastmod>([^<]+)<\/lastmod>/i.exec(block)?.[1]?.trim() ?? "";
    const priority = /<priority>([^<]+)<\/priority>/i.exec(block)?.[1]?.trim() ?? "0.5";
    if (loc) rows.push({ loc, lastmod, priority });
  }
  return rows;
}

export function extractHtmlSummary(url: string, html: string, fallback: DesignArticleSnapshot): DesignArticleSnapshot {
  const title = /<title[^>]*>([^<]+)<\/title>/i.exec(html)?.[1]?.trim() || fallback.title;
  const desc =
    /<meta[^>]+name=["']description["'][^>]+content=["']([^"']+)["']/i.exec(html)?.[1]?.trim() ||
    /<meta[^>]+content=["']([^"']+)["'][^>]+name=["']description["']/i.exec(html)?.[1]?.trim() ||
    fallback.description;
  const headings = [...html.matchAll(/<h[1-3][^>]*>([\s\S]*?)<\/h[1-3]>/gi)]
    .map((m) => m[1].replace(/<[^>]+>/g, " ").replace(/\s+/g, " ").trim())
    .filter(Boolean)
    .slice(0, 14);
  return {
    url,
    title,
    description: desc,
    headings: headings.length > 0 ? headings : fallback.headings,
    summary: fallback.summary,
  };
}

const DEFAULT_ARTICLES: DesignArticleSnapshot[] = [
  {
    url: "https://design.google/library/code-is-a-design-material",
    title: "Code is a Design Material",
    description:
      "A designer's reflections from Config, and why UX fundamentals matter more than ever",
    headings: [
      "Code is a Design Material",
      "The danger of perfecting too early",
      "Why thoughtwork still matters",
      "Designers' roles in the future of AI",
    ],
    summary:
      "Treats interactive Rust/GPUI/NSH code directly as a fast prototyping and expressive design medium while anchoring layout, accessibility, and hierarchy in durable UX thoughtwork.",
  },
  {
    url: "https://design.google/library/google-sans-flex-font",
    title: "Google Sans: Evolving Google's Typeface",
    description:
      "Discover how Google Sans Flex evolved with variable optical size, weight, width, grade, slant, and roundness axes across products and code.",
    headings: [
      "Making Google Sans Flex",
      "Product lockups didn't match the new Google logo",
      "Product Sans didn't work in marketing or user interfaces",
      "Google Sans wasn't legible enough at smaller sizes",
      "Google Sans was just for European languages",
      "Google Sans Mono wasn't great for coding",
      "Google Sans wasn't flexible enough for expression",
      "Google Sans Flex couldn't be used for everything",
    ],
    summary:
      "Guides Aphrody's dual typography strategy: Google Sans Flex (6 variable axes opsz/wght/wdth/GRAD/slnt/ROND) for brand display and UI chrome paired with dedicated monospace fonts (JetBrains Mono / Zed Mono) for code buffers.",
  },
];

export function renderDesignMdFromTokens(doc: GoogleDesignTokenDocument): string {
  const d = doc.sys.color.dark;
  const l = doc.sys.color.light;
  return `---
name: Aphrody Google Design System
source_of_truth: ${doc.source_of_truth}
system: ${doc.system}
stitch_url: ${doc.stitch_url}
sans_flex_family: ${doc.sans_flex_family}
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
  error-container: "${d.error_container}"
  on-error-container: "${d.on_error_container}"
  background: "${d.background}"
  on-background: "${d.on_background}"
  light-surface: "${l.surface}"
  light-on-surface: "${l.on_surface}"
  light-primary: "${l.primary}"
  light-on-primary: "${l.on_primary}"
typography:
  display-lg:
    fontFamily: "${doc.sans_flex_family}"
    fontSize: 57px
    fontWeight: "400"
    lineHeight: 64px
    letterSpacing: -0.004em
  headline-lg:
    fontFamily: "${doc.sans_flex_family}"
    fontSize: 32px
    fontWeight: "500"
    lineHeight: 40px
  body-md:
    fontFamily: "${doc.sans_flex_family}"
    fontSize: 14px
    fontWeight: "400"
    lineHeight: 20px
  code-mono:
    fontFamily: "JetBrains Mono"
    fontSize: 13px
    fontWeight: "400"
    lineHeight: 20px
rounded:
  none: 0px
  xs: 4px
  sm: 8px
  md: 12px
  lg: 16px
  xl: 28px
  full: 9999px
spacing:
  gap: 8px
  section: 16px
  touch-min: 44px
motion:
  spatial-fast: { stiffness: 1400, dampingRatio: 0.9 }
  spatial-default: { stiffness: 700, dampingRatio: 0.9 }
  effects-default: { stiffness: 1600, dampingRatio: 1.0 }
---

# Aphrody Visual Identity (\`DESIGN.md\`)

> Authoritative design system specification anchored to [\`https://design.google/\`](https://design.google/) (**Material 3 Expressive**), **Google Sans Flex**, [\`google-labs-code/design.md\`](https://github.com/google-labs-code/design.md), [\`GoogleChrome/modern-web-guidance-src\`](https://github.com/GoogleChrome/modern-web-guidance-src), and [**Google Stitch**](https://stitch.withgoogle.com/).

## Design Thesis: Code Is a Design Material

Inspired by [*Code is a Design Material*](https://design.google/library/code-is-a-design-material), Aphrody treats live Rust GPUI views, NSH TSX surfaces, and WGSL WebGPU shaders as malleable design materials unified by semantic tokens rather than hand-coded hex literals.

## Google Sans Flex Variable Axes & Code Pairings

Derived from [*Google Sans: Evolving Google's Typeface*](https://design.google/library/google-sans-flex-font):
- **UI Display & Controls**: \`Google Sans Flex\` with continuous axes \`opsz\` (6..144), \`wght\` (100..1000), \`wdth\` (25..151), \`GRAD\` (-50..200), \`slnt\` (-10..0), and \`ROND\` (0..100).
- **Code & Terminal Buffers**: \`JetBrains Mono\` / \`Roboto Mono\` / \`.AphrodyMono\` for unambiguous code legibility where proportional sans stops.

\`\`\`yaml
stitch:
  preserve_data_stitch_id: true
  default_viewport_px: 1440
  modern_web_baseline: true
  components:
    primary_button: "{comp.button.container_shape}"
    surface_card: "{comp.card.container_color_role}"
    shader_canvas: "{comp.shader_canvas.accent_color_role}"
\`\`\`
`;
}

export function serializeSitemapSnapshotToml(doc: SitemapSnapshotDocument): string {
  const lines: string[] = [
    "# Auto-generated by tooling/bun/google-design-sync.ts sync — mirrors https://design.google/ and Stitch ecosystem.",
    `schema_version = ${doc.schema_version}`,
    `synced_at = ${JSON.stringify(doc.synced_at)}`,
    `design_google_sitemap_url = ${JSON.stringify(doc.design_google_sitemap_url)}`,
    `stitch_url = ${JSON.stringify(doc.stitch_url)}`,
    `total_urls = ${doc.total_urls}`,
    "",
  ];
  for (const r of doc.upstream_ecosystem) {
    lines.push("[[upstream_ecosystem]]");
    lines.push(`repo = ${JSON.stringify(r.repo)}`);
    lines.push(`url = ${JSON.stringify(r.url)}`);
    lines.push(`license = ${JSON.stringify(r.license)}`);
    lines.push(`role = ${JSON.stringify(r.role)}`);
    lines.push("");
  }
  for (const sr of doc.stitch_routes) {
    lines.push("[[stitch_routes]]");
    lines.push(`path = ${JSON.stringify(sr.path)}`);
    lines.push(`title = ${JSON.stringify(sr.title)}`);
    lines.push(`capability = ${JSON.stringify(sr.capability)}`);
    lines.push("");
  }
  for (const a of doc.featured_articles) {
    lines.push("[[featured_articles]]");
    lines.push(`url = ${JSON.stringify(a.url)}`);
    lines.push(`title = ${JSON.stringify(a.title)}`);
    lines.push(`description = ${JSON.stringify(a.description)}`);
    lines.push(`headings = [${a.headings.map((h) => JSON.stringify(h)).join(", ")}]`);
    lines.push(`summary = ${JSON.stringify(a.summary)}`);
    lines.push("");
  }
  for (const u of doc.urls) {
    lines.push("[[urls]]");
    lines.push(`loc = ${JSON.stringify(u.loc)}`);
    lines.push(`lastmod = ${JSON.stringify(u.lastmod)}`);
    lines.push(`priority = ${JSON.stringify(u.priority)}`);
    lines.push("");
  }
  return lines.join("\n");
}

export async function syncGoogleDesignTokens(
  outPath = DEFAULT_TOKENS_PATH,
  offline = false,
  snapshotPath = DEFAULT_SITEMAP_SNAPSHOT_PATH,
  designMdPath = DEFAULT_DESIGN_MD_PATH,
): Promise<{ path: string; reachable: boolean; source: string; sitemapCount: number }> {
  let reachable = false;
  let urls: Array<{ loc: string; lastmod: string; priority: string }> = [
    {
      loc: "https://design.google/library/code-is-a-design-material",
      lastmod: "2026-01-01",
      priority: "0.9",
    },
    {
      loc: "https://design.google/library/google-sans-flex-font",
      lastmod: "2026-01-01",
      priority: "0.9",
    },
  ];
  const articles: DesignArticleSnapshot[] = [...DEFAULT_ARTICLES];

  if (!offline) {
    try {
      const [rootRes, sitemapRes, codeRes, fontRes] = await Promise.all([
        fetch(DESIGN_SOURCE_URL, { headers: { "User-Agent": "Aphrody-Design-Sync/1.0" } }),
        fetch("https://design.google/sitemap.xml", {
          headers: { "User-Agent": "Aphrody-Design-Sync/1.0" },
        }),
        fetch(DEFAULT_ARTICLES[0].url, { headers: { "User-Agent": "Aphrody-Design-Sync/1.0" } }),
        fetch(DEFAULT_ARTICLES[1].url, { headers: { "User-Agent": "Aphrody-Design-Sync/1.0" } }),
      ]);
      reachable = rootRes.ok;
      if (sitemapRes.ok) {
        const parsed = parseSitemapXmlUrls(await sitemapRes.text());
        if (parsed.length > 0) urls = parsed;
      }
      if (codeRes.ok) {
        articles[0] = extractHtmlSummary(
          DEFAULT_ARTICLES[0].url,
          await codeRes.text(),
          DEFAULT_ARTICLES[0],
        );
      }
      if (fontRes.ok) {
        articles[1] = extractHtmlSummary(
          DEFAULT_ARTICLES[1].url,
          await fontRes.text(),
          DEFAULT_ARTICLES[1],
        );
      }
    } catch {
      reachable = false;
    }
  }

  const doc = buildCanonicalDesignTokens();
  mkdirSync(resolve(outPath, ".."), { recursive: true });
  writeFileSync(outPath, serializeDesignTokensToml(doc), "utf8");

  const snapshot: SitemapSnapshotDocument = {
    schema_version: 1,
    synced_at: doc.synced_at,
    design_google_sitemap_url: "https://design.google/sitemap.xml",
    stitch_url: STITCH_SOURCE_URL,
    total_urls: urls.length,
    urls,
    featured_articles: articles,
    upstream_ecosystem: [
      {
        repo: "GoogleChrome/modern-web-guidance-src",
        url: "https://github.com/GoogleChrome/modern-web-guidance-src",
        license: "Apache-2.0",
        role: "Baseline web features, Coherence Auditor, and modern CSS/WebGPU/Wasm guidance",
      },
      {
        repo: "google-labs-code/design.md",
        url: "https://github.com/google-labs-code/design.md",
        license: "Apache-2.0",
        role: "DESIGN.md YAML-frontmatter visual identity specification and token exporter",
      },
      {
        repo: "google-labs-code/stitch-sdk",
        url: "https://github.com/google-labs-code/stitch-sdk",
        license: "Apache-2.0",
        role: "Google Stitch AI-native UI generation SDK, screen HTML & PNG artifact download",
      },
      {
        repo: "google-labs-code/stitch-skills",
        url: "https://github.com/google-labs-code/stitch-skills",
        license: "Apache-2.0",
        role: "Stitch MCP workflows (data-stitch-id tracking, DESIGN.md synthesis, TSX extraction)",
      },
    ],
    stitch_routes: [
      { path: "/", title: "Stitch AI Infinite Canvas", capability: "vibe-design-and-prompt" },
      { path: "/projects", title: "Stitch Projects", capability: "project-list-and-create" },
      {
        path: "/projects/:id/screens",
        title: "Stitch Screens & Variants",
        capability: "screen-generate-edit-variants",
      },
      {
        path: "/projects/:id/design-md",
        title: "Stitch DESIGN.md Context",
        capability: "design-md-import-export",
      },
      {
        path: "/projects/:id/capture",
        title: "Stitch Screen Capture & HTML",
        capability: "signed-html-and-png-capture",
      },
    ],
  };
  mkdirSync(resolve(snapshotPath, ".."), { recursive: true });
  writeFileSync(snapshotPath, serializeSitemapSnapshotToml(snapshot), "utf8");

  mkdirSync(resolve(designMdPath, ".."), { recursive: true });
  writeFileSync(designMdPath, renderDesignMdFromTokens(doc), "utf8");

  return { path: outPath, reachable, source: DESIGN_SOURCE_URL, sitemapCount: urls.length };
}

export interface DesignAuditReport {
  ok: boolean;
  sourceOfTruth: string;
  wcagFailures: string[];
  templateFailures: string[];
  errors: string[];
}

export function checkPaletteWcagAa(label: string, palette: ColorPaletteRoleSet): string[] {
  const failures: string[] = [];
  const pairs: Array<[string, string, string]> = [
    ["on_surface/surface", palette.on_surface, palette.surface],
    ["on_surface_variant/surface_container", palette.on_surface_variant, palette.surface_container],
    ["on_primary/primary", palette.on_primary, palette.primary],
    [
      "on_primary_container/primary_container",
      palette.on_primary_container,
      palette.primary_container,
    ],
    ["on_secondary/secondary", palette.on_secondary, palette.secondary],
    ["on_tertiary/tertiary", palette.on_tertiary, palette.tertiary],
    ["on_error/error", palette.on_error, palette.error],
  ];
  for (const [pairName, fg, bg] of pairs) {
    const ratio = contrastRatioHex(fg, bg);
    if (ratio < 4.5) {
      failures.push(`${label} ${pairName} contrast ${ratio.toFixed(2)} < 4.5 (${fg} on ${bg})`);
    }
  }
  return failures;
}

export function auditDesignCompliance(root = repoRoot): DesignAuditReport {
  const errors: string[] = [];
  const wcagFailures: string[] = [];
  const templateFailures: string[] = [];

  const productPath = join(root, "product.toml");
  const product = Bun.TOML.parse(readFileSync(productPath, "utf8")) as {
    design?: {
      source_of_truth?: string;
      tokens_manifest?: string;
      themes_manifest?: string;
      design_md?: string;
    };
  };
  const sourceOfTruth = product.design?.source_of_truth ?? "";
  if (sourceOfTruth !== DESIGN_SOURCE_URL) {
    errors.push(`product.toml [design].source_of_truth must be ${DESIGN_SOURCE_URL}`);
  }

  const designMdPath = join(root, product.design?.design_md ?? "DESIGN.md");
  if (!existsSync(designMdPath)) {
    errors.push(`Missing root DESIGN.md manifest: ${designMdPath}`);
  } else {
    const mdRaw = readFileSync(designMdPath, "utf8");
    if (!mdRaw.startsWith("---") || !mdRaw.includes("https://design.google/")) {
      errors.push("Root DESIGN.md must contain YAML frontmatter anchored to https://design.google/");
    }
  }

  const tokensPath = join(
    root,
    product.design?.tokens_manifest ?? "assets/os/google_design_tokens.toml",
  );
  if (!existsSync(tokensPath)) {
    errors.push(`Missing design tokens manifest: ${tokensPath}`);
  } else {
    const parsed = Bun.TOML.parse(readFileSync(tokensPath, "utf8")) as unknown as GoogleDesignTokenDocument;
    if (parsed.source_of_truth !== DESIGN_SOURCE_URL) {
      errors.push(`Tokens manifest source_of_truth mismatch: ${parsed.source_of_truth}`);
    }
    if (!parsed.ref?.variable_axes?.wght || !parsed.ref?.variable_axes?.opsz) {
      errors.push("Tokens manifest missing Google Sans Flex variable_axes (opsz/wght)");
    }
    wcagFailures.push(...checkPaletteWcagAa("light", parsed.sys.color.light));
    wcagFailures.push(...checkPaletteWcagAa("dark", parsed.sys.color.dark));
  }

  const snapshotPath = join(root, "assets/os/design_google_sitemap_snapshot.toml");
  if (!existsSync(snapshotPath)) {
    errors.push(`Missing sitemap snapshot: ${snapshotPath}`);
  }

  const themesPath = join(root, product.design?.themes_manifest ?? "assets/os/themes.json");
  if (!existsSync(themesPath)) {
    errors.push(`Missing themes manifest: ${themesPath}`);
  } else {
    const themes = JSON.parse(readFileSync(themesPath, "utf8")) as {
      themes?: Record<string, { light?: Record<string, string>; dark?: Record<string, string> }>;
    };
    if (!themes.themes?.["google-expressive"]) {
      errors.push(`themes.json missing "google-expressive" entry`);
    }
  }

  const templatesDir = join(root, "templates/wasm-wgpu");
  if (existsSync(templatesDir)) {
    for (const entry of readdirSync(templatesDir, { withFileTypes: true })) {
      if (!entry.isDirectory()) continue;
      const appDir = join(templatesDir, entry.name);
      const designToml = join(appDir, "design.toml");
      const tplDesignMd = join(appDir, "DESIGN.md");
      if (!existsSync(designToml)) {
        templateFailures.push(`templates/wasm-wgpu/${entry.name} missing design.toml`);
        continue;
      }
      if (!existsSync(tplDesignMd)) {
        templateFailures.push(`templates/wasm-wgpu/${entry.name} missing DESIGN.md`);
      }
      const meta = Bun.TOML.parse(readFileSync(designToml, "utf8")) as {
        source_of_truth?: string;
      };
      if (meta.source_of_truth !== DESIGN_SOURCE_URL) {
        templateFailures.push(
          `templates/wasm-wgpu/${entry.name}/design.toml must set source_of_truth = "${DESIGN_SOURCE_URL}"`,
        );
      }
    }
  }

  return {
    ok: errors.length === 0 && wcagFailures.length === 0 && templateFailures.length === 0,
    sourceOfTruth,
    wcagFailures,
    templateFailures,
    errors,
  };
}

if (import.meta.main) {
  const args = Bun.argv.slice(2);
  const cmd = args[0] ?? "audit";
  const offline = args.includes("--offline");
  if (cmd === "sync") {
    const outIdx = args.indexOf("--out");
    const outPath =
      outIdx >= 0 && args[outIdx + 1] ? resolve(repoRoot, args[outIdx + 1]) : DEFAULT_TOKENS_PATH;
    const res = await syncGoogleDesignTokens(outPath, offline);
    console.log(JSON.stringify({ synced: true, ...res }, null, 2));
  } else if (cmd === "audit") {
    const report = auditDesignCompliance(repoRoot);
    console.log(JSON.stringify(report, null, 2));
    if (!report.ok) process.exit(1);
  } else {
    console.error("Usage: bun tooling/bun/google-design-sync.ts <sync|audit> [--offline]");
    process.exit(1);
  }
}
