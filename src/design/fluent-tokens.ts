/**
 * Microsoft Fluent UI 2 Design System Tokens & Components Parity
 * Reference: https://github.com/microsoft/fluentui (packages/tokens)
 * Zero external npm dependencies - 100% pure Web Standards & CSS Variables
 */

export interface FluentThemeTokens {
  colorNeutralBackground1: string;
  colorNeutralBackground2: string;
  colorNeutralBackground3: string;
  colorNeutralBackground4: string;
  colorNeutralBackground5: string;
  colorNeutralBackground6: string;
  colorNeutralForeground1: string;
  colorNeutralForeground2: string;
  colorNeutralForeground3: string;
  colorNeutralForegroundDisabled: string;
  colorBrandBackground: string;
  colorBrandBackgroundHover: string;
  colorBrandBackgroundPressed: string;
  colorBrandBackgroundSelected: string;
  colorBrandForeground1: string;
  colorBrandForegroundLink: string;
  colorNeutralStroke1: string;
  colorNeutralStroke2: string;
  colorBrandStroke1: string;
  colorStrokeFocus1: string;
  colorStrokeFocus2: string;
  colorBackgroundMica: string;
  colorBackgroundAcrylic: string;
  colorNeutralCardBackground: string;
  colorNeutralCardBackgroundHover: string;
  shadow4: string;
  shadow8: string;
  shadow16: string;
  shadow28: string;
}

export const FLUENT_LIGHT_TOKENS: FluentThemeTokens = {
  colorNeutralBackground1: "#ffffff",
  colorNeutralBackground2: "#fafafa",
  colorNeutralBackground3: "#f5f5f5",
  colorNeutralBackground4: "#f0f0f0",
  colorNeutralBackground5: "#ebebeb",
  colorNeutralBackground6: "#e6e6e6",
  colorNeutralForeground1: "#242424",
  colorNeutralForeground2: "#424242",
  colorNeutralForeground3: "#616161",
  colorNeutralForegroundDisabled: "#bdbdbd",
  colorBrandBackground: "#0f6cbd",
  colorBrandBackgroundHover: "#115ea3",
  colorBrandBackgroundPressed: "#0c3b5e",
  colorBrandBackgroundSelected: "#0f548c",
  colorBrandForeground1: "#0f6cbd",
  colorBrandForegroundLink: "#0f6cbd",
  colorNeutralStroke1: "#d1d1d1",
  colorNeutralStroke2: "#e0e0e0",
  colorBrandStroke1: "#0f6cbd",
  colorStrokeFocus1: "#ffffff",
  colorStrokeFocus2: "#000000",
  colorBackgroundMica: "rgba(255, 255, 255, 0.78)",
  colorBackgroundAcrylic: "rgba(243, 243, 243, 0.65)",
  colorNeutralCardBackground: "#ffffff",
  colorNeutralCardBackgroundHover: "#f5f5f5",
  shadow4: "0 0 2px rgba(0,0,0,0.12), 0 2px 4px rgba(0,0,0,0.14)",
  shadow8: "0 0 2px rgba(0,0,0,0.12), 0 4px 8px rgba(0,0,0,0.14)",
  shadow16: "0 0 2px rgba(0,0,0,0.12), 0 8px 16px rgba(0,0,0,0.14)",
  shadow28: "0 0 8px rgba(0,0,0,0.12), 0 14px 28px rgba(0,0,0,0.20)",
};

export const FLUENT_DARK_TOKENS: FluentThemeTokens = {
  colorNeutralBackground1: "#292929",
  colorNeutralBackground2: "#1f1f1f",
  colorNeutralBackground3: "#141414",
  colorNeutralBackground4: "#0a0a0a",
  colorNeutralBackground5: "#000000",
  colorNeutralBackground6: "#050505",
  colorNeutralForeground1: "#ffffff",
  colorNeutralForeground2: "#d6d6d6",
  colorNeutralForeground3: "#adadad",
  colorNeutralForegroundDisabled: "#5c5c5c",
  colorBrandBackground: "#115ea3",
  colorBrandBackgroundHover: "#0f6cbd",
  colorBrandBackgroundPressed: "#0c3b5e",
  colorBrandBackgroundSelected: "#0f548c",
  colorBrandForeground1: "#479ef5",
  colorBrandForegroundLink: "#479ef5",
  colorNeutralStroke1: "#424242",
  colorNeutralStroke2: "#333333",
  colorBrandStroke1: "#115ea3",
  colorStrokeFocus1: "#000000",
  colorStrokeFocus2: "#ffffff",
  colorBackgroundMica: "rgba(32, 32, 32, 0.78)",
  colorBackgroundAcrylic: "rgba(20, 20, 20, 0.65)",
  colorNeutralCardBackground: "#242424",
  colorNeutralCardBackgroundHover: "#2d2d2d",
  shadow4: "0 0 2px rgba(0,0,0,0.24), 0 2px 4px rgba(0,0,0,0.28)",
  shadow8: "0 0 2px rgba(0,0,0,0.24), 0 4px 8px rgba(0,0,0,0.28)",
  shadow16: "0 0 2px rgba(0,0,0,0.24), 0 8px 16px rgba(0,0,0,0.28)",
  shadow28: "0 0 8px rgba(0,0,0,0.24), 0 14px 28px rgba(0,0,0,0.40)",
};

export const FLUENT_GLOBAL_TOKENS = {
  strokeWidthThin: "1px",
  strokeWidthThick: "2px",
  strokeWidthThicker: "3px",
  strokeWidthThickest: "4px",
  borderRadiusNone: "0px",
  borderRadiusSmall: "2px",
  borderRadiusMedium: "4px",
  borderRadiusLarge: "8px",
  borderRadiusXLarge: "12px",
  borderRadiusCircular: "9999px",
  fontFamilyBase:
    "'Segoe UI Variable', 'Segoe UI', -apple-system, BlinkMacSystemFont, Roboto, sans-serif",
  fontFamilyMonospace: "Consolas, 'Cascadia Code', 'Courier New', monospace",
  fontSizeCaption1: "10px",
  fontSizeCaption2: "12px",
  fontSizeBody1: "14px",
  fontSizeBody2: "16px",
  fontSizeSubtitle2: "16px",
  fontSizeSubtitle1: "20px",
  fontSizeTitle3: "24px",
  fontSizeTitle2: "28px",
  fontSizeTitle1: "32px",
  fontSizeDisplay: "68px",
  fontWeightRegular: "400",
  fontWeightMedium: "500",
  fontWeightSemibold: "600",
  fontWeightBold: "700",
};

export function getFluentCssVariables(
  theme: "light" | "dark" = "light",
): string {
  const t = theme === "dark" ? FLUENT_DARK_TOKENS : FLUENT_LIGHT_TOKENS;
  const entries: string[] = [];

  for (const [key, val] of Object.entries(t)) {
    entries.push(`--fluent-${key}: ${val};`);
  }
  for (const [key, val] of Object.entries(FLUENT_GLOBAL_TOKENS)) {
    entries.push(`--fluent-${key}: ${val};`);
  }

  return entries.join("\n  ");
}
