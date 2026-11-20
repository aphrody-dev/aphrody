---
name: Aphrody Google Neural Expressive Design System
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

# Google Design & Neural Expressive Specification (`docs/aphrody/design/DESIGN.md`)

Anchored to `https://design.google/?home` and `https://design.google/library/code-is-a-design-material`, uniting **GPUI**, **NSH**, **`egui 0.37` (`egui_gpui`)**, **Bevy Unreal Engine 5.8 (`bevy_gpui`)**, **FigCraft**, and **Craft Anything**.

## Motion Inventory

| Trigger / Interaction | Token / Physics Spring | Behavior & Reduced-Motion Rule |
| --- | --- | --- |
| Primary Pill Press | `spatial_fast` (`k=1400, z=0.9`) | Morphs corner radius `9999px -> 12px` & `scale(0.98)`; instant color change under reduced motion |
| Navigation Rail Collapse | `spatial_default` (`k=700, z=0.9`) | Animates width `260px <-> 56px` preserving icon anchor |
| Cascading Popover Open | `spatial_fast` + `effects_default` | Scales `0.96 -> 1.0` from trigger corner (`20px` radius); `Escape` restores trigger focus |
| Inspector / Sheet Slide | `spatial_default` (`k=700, z=0.9`) | Slides right inspector drawer without shifting primary scroll offset |
| Live Telemetry Pulse | `effects_slow` (`k=800, z=1.0`) | Critically damped status dot opacity pulse; static dot under reduced motion |

```yaml
stitch:
  preserve_data_stitch_id: true
  studios:
    figcraft_google: "{comp.card.container_color_role}"
    craft_anything_google: "{comp.code_surface.container_color_role}"
    unreal_bevy_58: "{comp.shader_canvas.accent_color_role}"
```
