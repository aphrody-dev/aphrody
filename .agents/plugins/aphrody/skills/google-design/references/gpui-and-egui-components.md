# Google Design GPUI, `egui_gpui` & `aphrody_craft` Component Mapping

Canonical mapping between [`aphrody/docs/design/manifest.json`](../../../../../../aphrody/docs/design/manifest.json), [`crates/aphrody_ui`](../../../../../../crates/aphrody_ui/src/aphrody_ui.rs), [`crates/egui_gpui`](../../../../../../crates/egui_gpui/src/egui_gpui.rs), [`crates/bevy_gpui`](../../../../../../crates/bevy_gpui/src/bevy_gpui.rs), and [`crates/aphrody_craft`](../../../../../../crates/aphrody_craft/src/aphrody_craft.rs).

| Surface / Pattern | Reference Capture (`aphrody/docs/design/assets/reference-captures/`) | GPUI Component (`aphrody_ui`) | `egui 0.37` Equivalent (`egui_gpui` / `aphrody_craft`) |
| --- | --- | --- | --- |
| Floating AI Omnibar & Mode Pill | `gemini-home.png`, `google-in-chrome.png` | `ExpressiveOmnibar` | `CraftToolbarChip` + `egui::TextEdit::singleline` |
| Cascading Model / Tool Picker | `gemini-opions.png` | `CascadingPopoverCard` (`PopoverMenuItem`) | `CraftComboMenu` |
| AI Mode Grounding Citation Card | `google-ai-mode.png` | `GroundingSourceCard` | `egui::Frame` + `StarterCard` parity |
| Chrome 157 / Studio Mode Switcher | `win11-chrome157.png` | `SegmentedStudioBar` | `CraftDockTabs` |
| Live Agent / Shader Aura Pill | `gemini-home.png`, `google-ai-mode.png` | `ThinkingAuraBadge` | `CraftImeBanner` / `CraftToolbarChip` |
| FigCraft Vector & Variable Canvas | `aphrody/docs/design/kits/figma-design-kit.tokens.json` | `aphrody_craft::render_gpui_studio_card` (`FigCraftGoogle`) | `aphrody_craft::render_egui_studio_frame` |
| Craft Anything Binary/Oxc JS Audit | `aphrody/docs/design/kits/stitch-design-kit.json` | `aphrody_craft::CraftAnythingGoogleReport` | `aphrody_craft::render_egui_studio_frame` |
| Unreal Bevy 5.8 3D Viewport | `aphrody/docs/design/assets/svg/bevy-unreal-pipeline.svg` | `bevy_gpui::ViewportFrame` + GPUI telemetry | `bevy_gpui::render_bevy_egui_inspector` |

## Contract Rules

1. Never hardcode hex values in Rust views or TSX pages — read `GoogleDesignTokens` in Rust (`aphrody_ui::GoogleDesignTokens::dark()` / `light()`) and `--md-sys-color-*` CSS variables from `aphrody/docs/design/stylesheet.css` in web/NSH apps.
2. Maintain WCAG 2.2 AA contrast (`>= 4.5:1` body text, `>= 3.0:1` interactive outlines) across both Light and Dark palettes.
3. Validate schema changes with `bun test tooling/bun/design-docs-pack.test.ts`.
