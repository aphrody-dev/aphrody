---
name: Aphrody Google Design System
source_of_truth: https://design.google/
system: Material 3 Expressive
stitch_url: https://stitch.withgoogle.com/
sans_flex_family: Google Sans Flex
colors:
  surface: "#0E141B"
  surface-dim: "#0B1016"
  surface-bright: "#343B44"
  surface-container-lowest: "#080D13"
  surface-container-low: "#161D25"
  surface-container: "#1B222B"
  surface-container-high: "#252C36"
  surface-container-highest: "#303843"
  on-surface: "#E0E6F0"
  on-surface-variant: "#C0CAD7"
  outline: "#8A94A2"
  outline-variant: "#404956"
  primary: "#8FD0FF"
  on-primary: "#003352"
  primary-container: "#004B75"
  on-primary-container: "#CEE5FF"
  secondary: "#B4C9E5"
  on-secondary: "#1D3248"
  secondary-container: "#34485F"
  on-secondary-container: "#D2E4FF"
  tertiary: "#C6C2FF"
  on-tertiary: "#2C295F"
  tertiary-container: "#434077"
  on-tertiary-container: "#E2DFFF"
  error: "#FFB4AB"
  on-error: "#690005"
  error-container: "#93000A"
  on-error-container: "#FFDAD6"
  background: "#0E141B"
  on-background: "#E0E6F0"
  light-surface: "#F7F9FF"
  light-on-surface: "#161C24"
  light-primary: "#006399"
  light-on-primary: "#FFFFFF"
typography:
  display-lg:
    fontFamily: "Google Sans Flex"
    fontSize: 57px
    fontWeight: "400"
    lineHeight: 64px
    letterSpacing: -0.004em
  headline-lg:
    fontFamily: "Google Sans Flex"
    fontSize: 32px
    fontWeight: "500"
    lineHeight: 40px
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

# Aphrody Visual Identity (`DESIGN.md`)

> Authoritative design system specification anchored to [`https://design.google/`](https://design.google/) (**Material 3 Expressive**), **Google Sans Flex**, [`google-labs-code/design.md`](https://github.com/google-labs-code/design.md), [`GoogleChrome/modern-web-guidance-src`](https://github.com/GoogleChrome/modern-web-guidance-src), and [**Google Stitch**](https://stitch.withgoogle.com/).

## Design Thesis: Code Is a Design Material

Inspired by [*Code is a Design Material*](https://design.google/library/code-is-a-design-material), Aphrody treats live Rust GPUI views, NSH TSX surfaces, and WGSL WebGPU shaders as malleable design materials unified by semantic tokens rather than hand-coded hex literals.

## Google Sans Flex Variable Axes & Code Pairings

Derived from [*Google Sans: Evolving Google's Typeface*](https://design.google/library/google-sans-flex-font):
- **UI Display & Controls**: `Google Sans Flex` with continuous axes `opsz` (6..144), `wght` (100..1000), `wdth` (25..151), `GRAD` (-50..200), `slnt` (-10..0), and `ROND` (0..100).
- **Code & Terminal Buffers**: `JetBrains Mono` / `Roboto Mono` / `.AphrodyMono` for unambiguous code legibility where proportional sans stops.

```yaml
stitch:
  preserve_data_stitch_id: true
  default_viewport_px: 1440
  modern_web_baseline: true
  components:
    primary_button: "{comp.button.container_shape}"
    surface_card: "{comp.card.container_color_role}"
    shader_canvas: "{comp.shader_canvas.accent_color_role}"
```
