export interface DesignTokenSet {
  color: Record<string, { light: string; dark: string; role: string }>;
  typography: {
    display: string;
    body: string;
    mono: string;
    scale: Record<string, { size: string; lineHeight: string; weight: number }>;
  };
  motion: {
    easing: Record<string, string>;
    duration: Record<string, string>;
  };
  radius: Record<string, string>;
}

export interface GpuiComponentSpec {
  id: string;
  crate: "os_ui" | "gpui_kit" | "nsh_base";
  symbol: string;
  tokensUsed: string[];
  states: ("default" | "hover" | "pressed" | "focused" | "disabled" | "selected")[];
}

export interface DesignKitSpec {
  name: string;
  platform: "figma" | "stitch";
  version: string;
  sourceUrl: string;
  tokensRef: string;
  components: string[];
}

export interface DesignManifest {
  $schema: string;
  name: string;
  version: string;
  source: string;
  tokensStylesheet: string;
  designDoc: string;
  svgs: { id: string; path: string; optimized: boolean }[];
  guidelines: { id: string; path: string; format: "mdx" }[];
  kits: { figma: string; stitch: string };
  gpuiComponents: GpuiComponentSpec[];
}
