---
name: Aphrody Mobile Companion
source_of_truth: https://design.google/
system: Material 3 Expressive + Neural Obsidian / OLED Pitch
typography:
  display: Google Sans Flex
  mono: Google Sans Code
touch_target_min_pt: 44
breakpoints:
  compact_max_px: 599
  medium_min_px: 600
---

# Aphrody Mobile Companion (`iOS` & `Android`)

- Targets `gpui_ios` + `NativeShell.xcframework` on iOS and `nsh-platform-android` (`.aar`) on Android.
- Uses `Compact` bottom navigation (`64px` height, `>= 44pt` touch targets) and cycles `neural` -> `dark` -> `light` -> `oled` themes.
- All colors are supplied dynamically via `theme.json`; UI classes use semantic tokens only.
