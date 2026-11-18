/**
 * Authoritative TypeScript Schema & Runtime Validator for `docs/design/manifest.json`.
 * Anchored to `https://design.google/?home`, Material 3 Expressive, Google Sans Flex,
 * W3C DTCG Figma Variables, Google Stitch (`data-stitch-id`), GPUI, and `egui 0.37` / Bevy UE5.8.
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
