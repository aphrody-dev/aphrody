# Aphrody Icons

## Guidelines

Icons are a major part of Aphrody's UI, conveying actions cleanly without cluttering toolbars with labeled buttons.
When introducing a new icon, keep it consistent with the existing icon set:

1. The SVG viewBox should be `16x16`.
2. For outlined icons, use a `1.2px` stroke width.
3. Keep the icon optically balanced inside an internal `12x12` bounding box for clear legibility.
4. Use `filled` and `outlined` naming when introducing paired variants.
5. Prefix deeply contextual icons with their domain (for example, `ToolWeb`, `ReplPlay`, `DebugStepInto`).
6. Avoid complex layer structures, clipping masks, or unnecessary transforms; optimize SVGs with [SVGOMG](https://jakearchibald.github.io/svgomg/) before committing.

## Sourcing

Most UI icons are adapted from [Lucide](https://lucide.dev/) or [Phosphor](https://phosphoricons.com/), or crafted directly to match the GPUI design system.

App and release-channel icons (`stable`, `preview`, `nightly`, `dev`) are generated deterministically from [`product.toml`](../../product.toml) and [`assets/images/aphrody_logo.svg`](../../assets/images/aphrody_logo.svg) via `cargo xtask brand apply`.

## Contributing

To introduce a new UI icon, add the `.svg` file under `assets/icons/` (snake_case filename) and register its variant in `crates/icons/src/icons.rs` (PascalCase enum variant).

Request a visual review on your PR with light and dark theme screenshots per [`docs/src/development/ui-checklist.md`](../../docs/src/development/ui-checklist.md).
