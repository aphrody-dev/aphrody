//! Google Design Material 3 Expressive (`https://design.google/`) theme
//! factories, semantic M3 color role palettes across 4 modes (`Light`, `Dark`,
//! `NeuralObsidian`, `OledPitch`), physics spring presets (Spatial `0.9` damping
//! and Effects `1.0` damping), and shape/corner tokens for `nsh_base`.
//!
//! All values match `fw/crates/fw-m3/tokens/google_design_tokens.toml` and maintain
//! WCAG 2.2 AA contrast (`>= 4.5:1`) across all surface and action foregrounds.

use nsh_core::{
    LayoutTransition,
    color::{Hsla, rgb},
    element::{AdaptiveColor, FONT_MONO, FONT_SANS, Spring},
    geometry::{Points, px},
    platform::Appearance,
};

use crate::theme::{Theme, ThemeError};

/// Authoritative Google Design URL (`https://design.google/`).
pub const DESIGN_SOURCE_OF_TRUTH: &str = brand::design::SOURCE_OF_TRUTH;

/// Default Google Design Material 3 Expressive brand typeface.
pub const M3_FONT_BRAND: &str = brand::DESIGN_SANS_FLEX_FAMILY;

/// Default Google Design Material 3 Expressive plain body typeface.
pub const M3_FONT_PLAIN: &str = brand::DESIGN_SANS_FLEX_FAMILY;

/// Default Google Design Material 3 Expressive monospace typeface.
pub const M3_FONT_MONO: &str = "JetBrains Mono";

/// Minimum WCAG 2.2 AA normal-text contrast ratio.
pub const WCAG_AA_MIN_CONTRAST: f32 = brand::design::WCAG_AA_MIN_CONTRAST;

/// Four Material 3 Expressive surface modes synchronized with `fw/crates/fw-m3/tokens/google_design_tokens.toml`,
/// unified directly onto [`brand::design::ThemeModeSlug`].
pub use brand::design::ThemeModeSlug as M3Mode;

/// Maps a 4-mode [`M3Mode`] onto NSH's binary [`Appearance`].
#[must_use]
pub const fn m3_nsh_appearance(mode: M3Mode) -> Appearance {
    if mode.is_dark() {
        Appearance::Dark
    } else {
        Appearance::Light
    }
}

/// Google Design reference brand accents (`[ref.palette]` in `google_design_tokens.toml`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct M3BrandAccents {
    /// Gemini spectrum blue (`#4285F4`).
    pub gemini_blue: Hsla,
    /// Gemini spectrum violet (`#9B72CB`).
    pub gemini_violet: Hsla,
    /// Gemini spectrum rose (`#D96570`).
    pub gemini_rose: Hsla,
    /// Gemini spectrum peach (`#FFDCC1`).
    pub gemini_peach: Hsla,
    /// Glimmer HUD gold (`#FAE366`).
    pub glimmer_gold: Hsla,
    /// Glimmer HUD mint (`#C3ECD0`).
    pub glimmer_mint: Hsla,
    /// Glimmer HUD lime (`#BFF28D`).
    pub glimmer_lime: Hsla,
    /// Bolt amber accent (`#FFB300`).
    pub bolt_amber: Hsla,
    /// Deep neural indigo (`#111B3A`).
    pub neural_indigo: Hsla,
}

/// Canonical Google Design brand accents resolved from `brand::design`.
pub const M3_ACCENTS: M3BrandAccents = M3BrandAccents {
    gemini_blue: rgb(brand::design::reference_palette_u32_or("gemini_blue", 0x4285F4)).to_hsla(),
    gemini_violet: rgb(brand::design::reference_palette_u32_or("gemini_violet", 0x9B72CB))
        .to_hsla(),
    gemini_rose: rgb(brand::design::reference_palette_u32_or("gemini_rose", 0xD96570)).to_hsla(),
    gemini_peach: rgb(brand::design::reference_palette_u32_or("gemini_peach", 0xFFDCC1)).to_hsla(),
    glimmer_gold: rgb(brand::design::reference_palette_u32_or("glimmer_gold", 0xFAE366)).to_hsla(),
    glimmer_mint: rgb(brand::design::reference_palette_u32_or("glimmer_mint", 0xC3ECD0)).to_hsla(),
    glimmer_lime: rgb(brand::design::reference_palette_u32_or("glimmer_lime", 0xBFF28D)).to_hsla(),
    bolt_amber: rgb(brand::design::reference_palette_u32_or("bolt50", 0xFFB300)).to_hsla(),
    neural_indigo: rgb(brand::design::reference_palette_u32_or("neural_indigo", 0x111B3A))
        .to_hsla(),
};

/// Semantic Material 3 Expressive color role palette for a single mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct M3RolePalette {
    /// Mode this palette belongs to.
    pub mode: M3Mode,
    /// Root background color.
    pub background: Hsla,
    /// Foreground text/icons on [`Self::background`].
    pub on_background: Hsla,
    /// Default surface color.
    pub surface: Hsla,
    /// Dimmed backdrop surface color.
    pub surface_dim: Hsla,
    /// Bright elevated surface color.
    pub surface_bright: Hsla,
    /// Lowest-elevation container surface (recessed panes / code blocks).
    pub surface_container_lowest: Hsla,
    /// Low-elevation container surface (cards / sidebars).
    pub surface_container_low: Hsla,
    /// Default container surface (navigation rails / toolbars).
    pub surface_container: Hsla,
    /// High-elevation container surface (popovers / dialogs / search bars).
    pub surface_container_high: Hsla,
    /// Highest-elevation container surface (filled cards / switch tracks).
    pub surface_container_highest: Hsla,
    /// Primary ink on surfaces.
    pub on_surface: Hsla,
    /// Secondary ink on surfaces (captions, hints, inactive labels).
    pub on_surface_variant: Hsla,
    /// Strong divider / control outline color.
    pub outline: Hsla,
    /// Subtle hairlines and container borders.
    pub outline_variant: Hsla,
    /// Primary brand/action fill.
    pub primary: Hsla,
    /// Ink on [`Self::primary`].
    pub on_primary: Hsla,
    /// Primary tonal container (FABs, active badges).
    pub primary_container: Hsla,
    /// Ink on [`Self::primary_container`].
    pub on_primary_container: Hsla,
    /// Secondary accent color.
    pub secondary: Hsla,
    /// Ink on [`Self::secondary`].
    pub on_secondary: Hsla,
    /// Secondary tonal container (tonal buttons, selected chips, active rail indicator).
    pub secondary_container: Hsla,
    /// Ink on [`Self::secondary_container`].
    pub on_secondary_container: Hsla,
    /// Tertiary expressive accent.
    pub tertiary: Hsla,
    /// Ink on [`Self::tertiary`].
    pub on_tertiary: Hsla,
    /// Tertiary tonal container (aura badges, AI highlights).
    pub tertiary_container: Hsla,
    /// Ink on [`Self::tertiary_container`].
    pub on_tertiary_container: Hsla,
    /// Error / destructive role.
    pub error: Hsla,
    /// Ink on [`Self::error`].
    pub on_error: Hsla,
    /// Error tonal container.
    pub error_container: Hsla,
    /// Ink on [`Self::error_container`].
    pub on_error_container: Hsla,
}

const fn palette_from_brand(
    mode: M3Mode,
    src: &brand::design::ColorPaletteRgb,
) -> M3RolePalette {
    M3RolePalette {
        mode,
        background: rgb(src.background).to_hsla(),
        on_background: rgb(src.on_background).to_hsla(),
        surface: rgb(src.surface).to_hsla(),
        surface_dim: rgb(src.surface_dim).to_hsla(),
        surface_bright: rgb(src.surface_bright).to_hsla(),
        surface_container_lowest: rgb(src.surface_container_lowest).to_hsla(),
        surface_container_low: rgb(src.surface_container_low).to_hsla(),
        surface_container: rgb(src.surface_container).to_hsla(),
        surface_container_high: rgb(src.surface_container_high).to_hsla(),
        surface_container_highest: rgb(src.surface_container_highest).to_hsla(),
        on_surface: rgb(src.on_surface).to_hsla(),
        on_surface_variant: rgb(src.on_surface_variant).to_hsla(),
        outline: rgb(src.outline).to_hsla(),
        outline_variant: rgb(src.outline_variant).to_hsla(),
        primary: rgb(src.primary).to_hsla(),
        on_primary: rgb(src.on_primary).to_hsla(),
        primary_container: rgb(src.primary_container).to_hsla(),
        on_primary_container: rgb(src.on_primary_container).to_hsla(),
        secondary: rgb(src.secondary).to_hsla(),
        on_secondary: rgb(src.on_secondary).to_hsla(),
        secondary_container: rgb(src.secondary_container).to_hsla(),
        on_secondary_container: rgb(src.on_secondary_container).to_hsla(),
        tertiary: rgb(src.tertiary).to_hsla(),
        on_tertiary: rgb(src.on_tertiary).to_hsla(),
        tertiary_container: rgb(src.tertiary_container).to_hsla(),
        on_tertiary_container: rgb(src.on_tertiary_container).to_hsla(),
        error: rgb(src.error).to_hsla(),
        on_error: rgb(src.on_error).to_hsla(),
        error_container: rgb(src.error_container).to_hsla(),
        on_error_container: rgb(src.on_error_container).to_hsla(),
    }
}

const LIGHT_PALETTE: M3RolePalette =
    palette_from_brand(M3Mode::Light, &brand::design::PALETTE_LIGHT_RGB);
const DARK_PALETTE: M3RolePalette =
    palette_from_brand(M3Mode::Dark, &brand::design::PALETTE_DARK_RGB);
const NEURAL_OBSIDIAN_PALETTE: M3RolePalette = palette_from_brand(
    M3Mode::NeuralObsidian,
    &brand::design::PALETTE_NEURAL_OBSIDIAN_RGB,
);
const OLED_PITCH_PALETTE: M3RolePalette =
    palette_from_brand(M3Mode::OledPitch, &brand::design::PALETTE_OLED_PITCH_RGB);

impl M3RolePalette {
    /// Returns the authoritative M3 Expressive role palette for `mode`.
    pub const fn for_mode(mode: M3Mode) -> Self {
        match mode {
            M3Mode::Light => LIGHT_PALETTE,
            M3Mode::Dark => DARK_PALETTE,
            M3Mode::NeuralObsidian => NEURAL_OBSIDIAN_PALETTE,
            M3Mode::OledPitch => OLED_PITCH_PALETTE,
        }
    }

    /// Returns all `(role_token_name, color)` pairs with `"md-"` prefix removed
    /// (e.g. `("background", ...)` through `("on-error-container", ...)`).
    pub fn role_entries(self) -> [(&'static str, Hsla); 30] {
        [
            ("background", self.background),
            ("on-background", self.on_background),
            ("surface", self.surface),
            ("surface-dim", self.surface_dim),
            ("surface-bright", self.surface_bright),
            ("surface-container-lowest", self.surface_container_lowest),
            ("surface-container-low", self.surface_container_low),
            ("surface-container", self.surface_container),
            ("surface-container-high", self.surface_container_high),
            ("surface-container-highest", self.surface_container_highest),
            ("on-surface", self.on_surface),
            ("on-surface-variant", self.on_surface_variant),
            ("outline", self.outline),
            ("outline-variant", self.outline_variant),
            ("primary", self.primary),
            ("on-primary", self.on_primary),
            ("primary-container", self.primary_container),
            ("on-primary-container", self.on_primary_container),
            ("secondary", self.secondary),
            ("on-secondary", self.on_secondary),
            ("secondary-container", self.secondary_container),
            ("on-secondary-container", self.on_secondary_container),
            ("tertiary", self.tertiary),
            ("on-tertiary", self.on_tertiary),
            ("tertiary-container", self.tertiary_container),
            ("on-tertiary-container", self.on_tertiary_container),
            ("error", self.error),
            ("on-error", self.on_error),
            ("error-container", self.error_container),
            ("on-error-container", self.on_error_container),
        ]
    }

    /// Verifies that every semantic foreground/background pair meets WCAG 2.2 AA (`>= 4.5:1`).
    pub fn verify_wcag_aa(self) -> Result<(), ThemeError> {
        let pairs = [
            ("on_background/background", self.on_background, self.background),
            ("on_surface/surface", self.on_surface, self.surface),
            (
                "on_surface/surface_container_lowest",
                self.on_surface,
                self.surface_container_lowest,
            ),
            (
                "on_surface/surface_container_low",
                self.on_surface,
                self.surface_container_low,
            ),
            (
                "on_surface/surface_container",
                self.on_surface,
                self.surface_container,
            ),
            (
                "on_surface/surface_container_high",
                self.on_surface,
                self.surface_container_high,
            ),
            (
                "on_surface/surface_container_highest",
                self.on_surface,
                self.surface_container_highest,
            ),
            (
                "on_surface_variant/surface",
                self.on_surface_variant,
                self.surface,
            ),
            (
                "on_surface_variant/surface_container",
                self.on_surface_variant,
                self.surface_container,
            ),
            ("on_primary/primary", self.on_primary, self.primary),
            (
                "on_primary_container/primary_container",
                self.on_primary_container,
                self.primary_container,
            ),
            ("on_secondary/secondary", self.on_secondary, self.secondary),
            (
                "on_secondary_container/secondary_container",
                self.on_secondary_container,
                self.secondary_container,
            ),
            ("on_tertiary/tertiary", self.on_tertiary, self.tertiary),
            (
                "on_tertiary_container/tertiary_container",
                self.on_tertiary_container,
                self.tertiary_container,
            ),
            ("on_error/error", self.on_error, self.error),
            (
                "on_error_container/error_container",
                self.on_error_container,
                self.error_container,
            ),
        ];
        for (name, fg, bg) in pairs {
            let ratio = wcag_contrast_ratio(fg, bg);
            if ratio < WCAG_AA_MIN_CONTRAST {
                return Err(ThemeError(format!(
                    "{} `{name}` WCAG contrast {ratio:.2}:1 < {WCAG_AA_MIN_CONTRAST:.1}:1",
                    self.mode.slug()
                )));
            }
        }
        Ok(())
    }
}

/// Computes the WCAG 2.x relative contrast ratio (`1.0..=21.0`) between `foreground` and `background`.
pub fn wcag_contrast_ratio(foreground: Hsla, background: Hsla) -> f32 {
    let fg = foreground.to_rgb();
    let bg = background.to_rgb();
    brand::design::contrast_ratio_rgba([fg.r, fg.g, fg.b, foreground.a], [bg.r, bg.g, bg.b])
}

/// Material 3 Expressive corner radius scale (`[sys.shape]` in `google_design_tokens.toml`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct M3ShapeTokens {
    /// Sharp corner (`0px`).
    pub none_px: f32,
    /// Extra-small corner (`4px`).
    pub extra_small_px: f32,
    /// Small corner (`8px`).
    pub small_px: f32,
    /// Medium corner (`12px` — pressed button morph, code surfaces).
    pub medium_px: f32,
    /// Large corner (`16px` — cards, FABs, shader canvases).
    pub large_px: f32,
    /// Large-increased corner (`20px` — toolbars, HUDs).
    pub large_increased_px: f32,
    /// Extra-large corner (`28px` — dialogs, glass notes).
    pub extra_large_px: f32,
    /// Full pill/circle corner (`9999px` — buttons, chips, switches, search bars).
    pub full_px: f32,
}

/// Authoritative M3 Expressive shape/corner tokens resolved from `brand::design`.
pub const M3_SHAPES: M3ShapeTokens = M3ShapeTokens {
    none_px: brand::design::SHAPES.none_px,
    extra_small_px: brand::design::SHAPES.extra_small_px,
    small_px: brand::design::SHAPES.small_px,
    medium_px: brand::design::SHAPES.medium_px,
    large_px: brand::design::SHAPES.large_px,
    large_increased_px: brand::design::SHAPES.large_increased_px,
    extra_large_px: brand::design::SHAPES.extra_large_px,
    full_px: brand::design::SHAPES.full_px,
};

impl M3ShapeTokens {
    /// `0pt` corner radius.
    pub const fn none(self) -> Points {
        px(self.none_px)
    }

    /// `4pt` corner radius.
    pub const fn extra_small(self) -> Points {
        px(self.extra_small_px)
    }

    /// `8pt` corner radius.
    pub const fn small(self) -> Points {
        px(self.small_px)
    }

    /// `12pt` corner radius.
    pub const fn medium(self) -> Points {
        px(self.medium_px)
    }

    /// `16pt` corner radius.
    pub const fn large(self) -> Points {
        px(self.large_px)
    }

    /// `20pt` corner radius.
    pub const fn large_increased(self) -> Points {
        px(self.large_increased_px)
    }

    /// `28pt` corner radius.
    pub const fn extra_large(self) -> Points {
        px(self.extra_large_px)
    }

    /// `9999pt` full pill/circle radius.
    pub const fn full(self) -> Points {
        px(self.full_px)
    }
}

/// Material 3 Expressive interaction state opacities (`[sys.state]` in `google_design_tokens.toml`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct M3StateLayerTokens {
    /// Hover tint opacity (`0.08`).
    pub hover_opacity: f32,
    /// Focus tint opacity (`0.10`).
    pub focus_opacity: f32,
    /// Pressed tint opacity (`0.10`).
    pub pressed_opacity: f32,
    /// Dragged tint opacity (`0.16`).
    pub dragged_opacity: f32,
    /// Disabled container opacity (`0.12`).
    pub disabled_container_opacity: f32,
    /// Disabled content opacity (`0.38`).
    pub disabled_content_opacity: f32,
    /// Keyboard focus ring width (`2.0pt`).
    pub focus_ring_px: f32,
}

/// Authoritative M3 Expressive interaction state layer tokens resolved from `brand::design`.
pub const M3_STATE_LAYERS: M3StateLayerTokens = M3StateLayerTokens {
    hover_opacity: brand::design::STATES.hover_opacity,
    focus_opacity: brand::design::STATES.focus_opacity,
    pressed_opacity: brand::design::STATES.pressed_opacity,
    dragged_opacity: brand::design::STATES.dragged_opacity,
    disabled_container_opacity: brand::design::STATES.disabled_container_opacity,
    disabled_content_opacity: brand::design::STATES.disabled_content_opacity,
    focus_ring_px: brand::design::STATES.focus_ring_px,
};

/// Material 3 Expressive physics spring presets (`[sys.motion.springs]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum M3SpringPreset {
    /// Spatial fast (`stiffness: 1400`, `damping_ratio: 0.90`).
    SpatialFast,
    /// Spatial default (`stiffness: 700`, `damping_ratio: 0.90`).
    SpatialDefault,
    /// Spatial slow (`stiffness: 300`, `damping_ratio: 0.90`).
    SpatialSlow,
    /// Effects fast (`stiffness: 3800`, `damping_ratio: 1.00`).
    EffectsFast,
    /// Effects default (`stiffness: 1600`, `damping_ratio: 1.00`).
    EffectsDefault,
    /// Effects slow (`stiffness: 800`, `damping_ratio: 1.00`).
    EffectsSlow,
}

/// Parameters of a Material 3 Expressive physics spring.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct M3SpringSpec {
    /// Canonical token key in `google_design_tokens.toml`.
    pub name: &'static str,
    /// Spring stiffness `k` (unit mass `m = 1`).
    pub stiffness: f32,
    /// Damping ratio `zeta` (`0.9` spatial, `1.0` critically damped effects).
    pub damping_ratio: f32,
    /// True for spatial springs (`position/scale/shape`), false for critically damped effects springs (`color/opacity`).
    pub is_spatial: bool,
}

/// Full catalog of M3 Expressive Spatial & Effects spring tokens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct M3SpringTokens {
    /// Spatial fast (`1400`, `0.90`).
    pub spatial_fast: M3SpringSpec,
    /// Spatial default (`700`, `0.90`).
    pub spatial_default: M3SpringSpec,
    /// Spatial slow (`300`, `0.90`).
    pub spatial_slow: M3SpringSpec,
    /// Effects fast (`3800`, `1.00`).
    pub effects_fast: M3SpringSpec,
    /// Effects default (`1600`, `1.00`).
    pub effects_default: M3SpringSpec,
    /// Effects slow (`800`, `1.00`).
    pub effects_slow: M3SpringSpec,
}

const fn spring_from_brand(
    key: &'static str,
    fallback_stiffness: f32,
    fallback_damping: f32,
    is_spatial: bool,
) -> M3SpringSpec {
    match brand::design::spring_spec_const(key) {
        Some(spec) => M3SpringSpec {
            name: spec.key,
            stiffness: spec.stiffness,
            damping_ratio: spec.damping_ratio,
            is_spatial,
        },
        None => M3SpringSpec {
            name: key,
            stiffness: fallback_stiffness,
            damping_ratio: fallback_damping,
            is_spatial,
        },
    }
}

/// Authoritative M3 Expressive Spatial & Effects spring tokens resolved from `brand::design`.
pub const M3_SPRINGS: M3SpringTokens = M3SpringTokens {
    spatial_fast: spring_from_brand("spatial_fast", 1400.0, 0.90, true),
    spatial_default: spring_from_brand("spatial_default", 700.0, 0.90, true),
    spatial_slow: spring_from_brand("spatial_slow", 300.0, 0.90, true),
    effects_fast: spring_from_brand("effects_fast", 3800.0, 1.00, false),
    effects_default: spring_from_brand("effects_default", 1600.0, 1.00, false),
    effects_slow: spring_from_brand("effects_slow", 800.0, 1.00, false),
};

impl M3SpringSpec {
    /// Converts this spring specification into an NSH [`Spring`] (`stiffness`, computed `damping`, `mass = 1`).
    pub fn to_nsh_spring(self) -> Spring {
        Spring::from_stiffness(self.stiffness).damping_ratio(self.damping_ratio)
    }

    /// Converts this spring specification into an NSH [`LayoutTransition`].
    pub fn to_layout_transition(self) -> LayoutTransition {
        LayoutTransition::spring(self.to_nsh_spring())
    }

    /// Computed settling time in milliseconds from rest (`1/1000` threshold).
    pub fn settle_ms(self) -> f32 {
        self.to_nsh_spring().settle_ms()
    }
}

impl M3SpringPreset {
    /// All 6 M3 Expressive spring presets.
    pub const ALL: [Self; 6] = [
        Self::SpatialFast,
        Self::SpatialDefault,
        Self::SpatialSlow,
        Self::EffectsFast,
        Self::EffectsDefault,
        Self::EffectsSlow,
    ];

    /// Returns the numeric [`M3SpringSpec`] for this preset.
    pub const fn spec(self) -> M3SpringSpec {
        match self {
            Self::SpatialFast => M3_SPRINGS.spatial_fast,
            Self::SpatialDefault => M3_SPRINGS.spatial_default,
            Self::SpatialSlow => M3_SPRINGS.spatial_slow,
            Self::EffectsFast => M3_SPRINGS.effects_fast,
            Self::EffectsDefault => M3_SPRINGS.effects_default,
            Self::EffectsSlow => M3_SPRINGS.effects_slow,
        }
    }

    /// Converts this preset into an NSH [`Spring`].
    pub fn to_nsh_spring(self) -> Spring {
        self.spec().to_nsh_spring()
    }

    /// Converts this preset into an NSH [`LayoutTransition`].
    pub fn to_layout_transition(self) -> LayoutTransition {
        self.spec().to_layout_transition()
    }

    /// Computed settling time in milliseconds.
    pub fn settle_ms(self) -> f32 {
        self.spec().settle_ms()
    }
}

/// Builds a Material 3 Expressive [`Theme`] locked to `mode` (`Light`, `Dark`,
/// `NeuralObsidian`, or `OledPitch`), populating all shadcn compatibility tokens,
/// all `md-*` semantic role tokens, M3 shape tokens, spring tokens, and
/// `Google Sans Flex` / `JetBrains Mono` typography.
pub fn m3_theme(mode: M3Mode) -> Theme {
    let palette = M3RolePalette::for_mode(mode);
    build_m3_theme(palette, palette)
}

/// Builds a Material 3 Expressive [`Theme`] for [`M3Mode::Light`].
pub fn m3_light_theme() -> Theme {
    m3_theme(M3Mode::Light)
}

/// Builds a Material 3 Expressive [`Theme`] for [`M3Mode::Dark`].
pub fn m3_dark_theme() -> Theme {
    m3_theme(M3Mode::Dark)
}

/// Builds a Material 3 Expressive [`Theme`] for [`M3Mode::NeuralObsidian`].
pub fn m3_neural_obsidian_theme() -> Theme {
    m3_theme(M3Mode::NeuralObsidian)
}

/// Builds a Material 3 Expressive [`Theme`] for [`M3Mode::OledPitch`].
pub fn m3_oled_pitch_theme() -> Theme {
    m3_theme(M3Mode::OledPitch)
}

/// Builds a dual-appearance adaptive [`Theme`] whose light palette is [`M3Mode::Light`]
/// and whose dark palette is `dark_mode` (falling back to [`M3Mode::Dark`] if
/// [`M3Mode::Light`] is passed).
pub fn m3_adaptive_theme(dark_mode: M3Mode) -> Theme {
    let resolved_dark = if dark_mode.is_dark() {
        dark_mode
    } else {
        M3Mode::Dark
    };
    build_m3_theme(
        M3RolePalette::for_mode(M3Mode::Light),
        M3RolePalette::for_mode(resolved_dark),
    )
}

fn adaptive(light: Hsla, dark: Hsla) -> AdaptiveColor {
    AdaptiveColor::new(light, dark)
}

fn build_m3_theme(light: M3RolePalette, dark: M3RolePalette) -> Theme {
    let mut theme = Theme::default();

    // 1. Standard shadcn/ui compatibility tokens
    theme.set_color("background", adaptive(light.surface, dark.surface));
    theme.set_color("foreground", adaptive(light.on_surface, dark.on_surface));
    theme.set_color(
        "card",
        adaptive(light.surface_container_low, dark.surface_container_low),
    );
    theme.set_color(
        "card-foreground",
        adaptive(light.on_surface, dark.on_surface),
    );
    theme.set_color(
        "popover",
        adaptive(light.surface_container_high, dark.surface_container_high),
    );
    theme.set_color(
        "popover-foreground",
        adaptive(light.on_surface, dark.on_surface),
    );
    theme.set_color("primary", adaptive(light.primary, dark.primary));
    theme.set_color(
        "primary-foreground",
        adaptive(light.on_primary, dark.on_primary),
    );
    theme.set_color(
        "secondary",
        adaptive(light.secondary_container, dark.secondary_container),
    );
    theme.set_color(
        "secondary-foreground",
        adaptive(light.on_secondary_container, dark.on_secondary_container),
    );
    theme.set_color(
        "muted",
        adaptive(light.surface_container, dark.surface_container),
    );
    theme.set_color(
        "muted-foreground",
        adaptive(light.on_surface_variant, dark.on_surface_variant),
    );
    theme.set_color(
        "accent",
        adaptive(light.tertiary_container, dark.tertiary_container),
    );
    theme.set_color(
        "accent-foreground",
        adaptive(light.on_tertiary_container, dark.on_tertiary_container),
    );
    theme.set_color("destructive", adaptive(light.error, dark.error));
    theme.set_color(
        "destructive-foreground",
        adaptive(light.on_error, dark.on_error),
    );
    theme.set_color("border", adaptive(light.outline_variant, dark.outline_variant));
    theme.set_color("input", adaptive(light.outline_variant, dark.outline_variant));
    theme.set_color("ring", adaptive(light.primary, dark.primary));
    theme.set_color(
        "selection",
        adaptive(light.primary.opacity(0.22), dark.primary.opacity(0.22)),
    );

    // Chart palette anchored to M3 primary/secondary/tertiary & Gemini spectrum
    theme.set_color("chart-1", adaptive(light.primary, dark.primary));
    theme.set_color("chart-2", adaptive(light.secondary, dark.secondary));
    theme.set_color("chart-3", adaptive(light.tertiary, dark.tertiary));
    theme.set_color("chart-4", M3_ACCENTS.gemini_blue.into());
    theme.set_color("chart-5", M3_ACCENTS.gemini_violet.into());

    // Sidebar tokens
    theme.set_color(
        "sidebar",
        adaptive(light.surface_container_low, dark.surface_container_low),
    );
    theme.set_color(
        "sidebar-foreground",
        adaptive(light.on_surface, dark.on_surface),
    );
    theme.set_color("sidebar-primary", adaptive(light.primary, dark.primary));
    theme.set_color(
        "sidebar-primary-foreground",
        adaptive(light.on_primary, dark.on_primary),
    );
    theme.set_color(
        "sidebar-accent",
        adaptive(light.secondary_container, dark.secondary_container),
    );
    theme.set_color(
        "sidebar-accent-foreground",
        adaptive(light.on_secondary_container, dark.on_secondary_container),
    );
    theme.set_color(
        "sidebar-border",
        adaptive(light.outline_variant, dark.outline_variant),
    );
    theme.set_color("sidebar-ring", adaptive(light.primary, dark.primary));

    // 2. Semantic M3 Role palette (`md-*`) for NSH Tailwind classes & token() lookups
    for ((role, l_color), (_, d_color)) in light
        .role_entries()
        .into_iter()
        .zip(dark.role_entries())
    {
        theme.set_color(&format!("md-{role}"), adaptive(l_color, d_color));
    }

    theme.set_color("md-gemini-blue", M3_ACCENTS.gemini_blue.into());
    theme.set_color("md-gemini-violet", M3_ACCENTS.gemini_violet.into());
    theme.set_color("md-gemini-rose", M3_ACCENTS.gemini_rose.into());
    theme.set_color("md-bolt-amber", M3_ACCENTS.bolt_amber.into());

    // 3. M3 shape / corner & state-layer tokens
    theme.set_number("radius", M3_SHAPES.medium_px);
    theme.set_number("radius-none", M3_SHAPES.none_px);
    theme.set_number("radius-xs", M3_SHAPES.extra_small_px);
    theme.set_number("radius-sm", M3_SHAPES.small_px);
    theme.set_number("radius-md", M3_SHAPES.medium_px);
    theme.set_number("radius-lg", M3_SHAPES.large_px);
    theme.set_number("radius-lg-increased", M3_SHAPES.large_increased_px);
    theme.set_number("radius-xl", M3_SHAPES.extra_large_px);
    theme.set_number("radius-full", M3_SHAPES.full_px);
    theme.set_number("focus-ring", M3_STATE_LAYERS.focus_ring_px);

    // 4. Spatial (0.9 damping) and Effects (1.0 damping) spring tokens
    for preset in M3SpringPreset::ALL {
        let spec = preset.spec();
        let slug = spec.name.replace('_', "-");
        theme.set_number(&format!("spring-{slug}-stiffness"), spec.stiffness);
        theme.set_number(&format!("spring-{slug}-damping"), spec.damping_ratio);
    }

    // 5. Google Sans Flex & JetBrains Mono typography
    theme.set_font(FONT_SANS, M3_FONT_PLAIN);
    theme.set_font(FONT_MONO, M3_FONT_MONO);

    theme
}

/// Resolves an M3 role token from `theme` by `"md-<role>"`, falling back via `fallback`
/// when `theme` is a non-M3 host theme (such as Zed's One Dark via `nsh_theme_from_gpui`).
pub fn resolve_m3_role(
    theme: &Theme,
    md_token: &str,
    fallback: impl FnOnce(&Theme) -> AdaptiveColor,
) -> AdaptiveColor {
    theme.color(md_token).unwrap_or_else(|| fallback(theme))
}

/// Serializes `mode` as a shadcn/NSH-compatible `{ "cssVars": { ... } }` JSON string
/// ready for `AppView::set_theme_json` or NSH TSX `setTheme(...)`.
pub fn m3_theme_css_vars_json(mode: M3Mode) -> String {
    let palette = M3RolePalette::for_mode(mode);
    let mut vars = serde_json::Map::new();

    vars.insert(
        "background".to_owned(),
        serde_json::Value::String(palette.surface.to_hex()),
    );
    vars.insert(
        "foreground".to_owned(),
        serde_json::Value::String(palette.on_surface.to_hex()),
    );
    vars.insert(
        "card".to_owned(),
        serde_json::Value::String(palette.surface_container_low.to_hex()),
    );
    vars.insert(
        "card-foreground".to_owned(),
        serde_json::Value::String(palette.on_surface.to_hex()),
    );
    vars.insert(
        "popover".to_owned(),
        serde_json::Value::String(palette.surface_container_high.to_hex()),
    );
    vars.insert(
        "popover-foreground".to_owned(),
        serde_json::Value::String(palette.on_surface.to_hex()),
    );
    vars.insert(
        "primary".to_owned(),
        serde_json::Value::String(palette.primary.to_hex()),
    );
    vars.insert(
        "primary-foreground".to_owned(),
        serde_json::Value::String(palette.on_primary.to_hex()),
    );
    vars.insert(
        "secondary".to_owned(),
        serde_json::Value::String(palette.secondary_container.to_hex()),
    );
    vars.insert(
        "secondary-foreground".to_owned(),
        serde_json::Value::String(palette.on_secondary_container.to_hex()),
    );
    vars.insert(
        "muted".to_owned(),
        serde_json::Value::String(palette.surface_container.to_hex()),
    );
    vars.insert(
        "muted-foreground".to_owned(),
        serde_json::Value::String(palette.on_surface_variant.to_hex()),
    );
    vars.insert(
        "accent".to_owned(),
        serde_json::Value::String(palette.tertiary_container.to_hex()),
    );
    vars.insert(
        "accent-foreground".to_owned(),
        serde_json::Value::String(palette.on_tertiary_container.to_hex()),
    );
    vars.insert(
        "destructive".to_owned(),
        serde_json::Value::String(palette.error.to_hex()),
    );
    vars.insert(
        "destructive-foreground".to_owned(),
        serde_json::Value::String(palette.on_error.to_hex()),
    );
    vars.insert(
        "border".to_owned(),
        serde_json::Value::String(palette.outline_variant.to_hex()),
    );
    vars.insert(
        "input".to_owned(),
        serde_json::Value::String(palette.outline_variant.to_hex()),
    );
    vars.insert(
        "ring".to_owned(),
        serde_json::Value::String(palette.primary.to_hex()),
    );
    vars.insert(
        "sidebar".to_owned(),
        serde_json::Value::String(palette.surface_container_low.to_hex()),
    );
    vars.insert(
        "sidebar-foreground".to_owned(),
        serde_json::Value::String(palette.on_surface.to_hex()),
    );
    vars.insert(
        "sidebar-primary".to_owned(),
        serde_json::Value::String(palette.primary.to_hex()),
    );
    vars.insert(
        "sidebar-primary-foreground".to_owned(),
        serde_json::Value::String(palette.on_primary.to_hex()),
    );
    vars.insert(
        "sidebar-accent".to_owned(),
        serde_json::Value::String(palette.secondary_container.to_hex()),
    );
    vars.insert(
        "sidebar-accent-foreground".to_owned(),
        serde_json::Value::String(palette.on_secondary_container.to_hex()),
    );
    vars.insert(
        "sidebar-border".to_owned(),
        serde_json::Value::String(palette.outline_variant.to_hex()),
    );
    vars.insert(
        "sidebar-ring".to_owned(),
        serde_json::Value::String(palette.primary.to_hex()),
    );

    for (role, color) in palette.role_entries() {
        vars.insert(
            format!("md-{role}"),
            serde_json::Value::String(color.to_hex()),
        );
    }

    let mut shared = serde_json::Map::new();
    shared.insert(
        "font-sans".to_owned(),
        serde_json::Value::String(format!("{M3_FONT_PLAIN}, Inter, system-ui, sans-serif")),
    );
    shared.insert(
        "font-mono".to_owned(),
        serde_json::Value::String(format!("{M3_FONT_MONO}, Roboto Mono, monospace")),
    );
    shared.insert(
        "radius".to_owned(),
        serde_json::Value::String(format!("{}px", M3_SHAPES.medium_px)),
    );
    shared.insert(
        "radius-xs".to_owned(),
        serde_json::Value::String(format!("{}px", M3_SHAPES.extra_small_px)),
    );
    shared.insert(
        "radius-sm".to_owned(),
        serde_json::Value::String(format!("{}px", M3_SHAPES.small_px)),
    );
    shared.insert(
        "radius-lg".to_owned(),
        serde_json::Value::String(format!("{}px", M3_SHAPES.large_px)),
    );
    shared.insert(
        "radius-xl".to_owned(),
        serde_json::Value::String(format!("{}px", M3_SHAPES.extra_large_px)),
    );
    shared.insert(
        "radius-full".to_owned(),
        serde_json::Value::String(format!("{}px", M3_SHAPES.full_px)),
    );

    let vars_obj = serde_json::Value::Object(vars);
    let root = serde_json::json!({
        "name": format!("Material 3 Expressive ({})", mode.label()),
        "cssVars": {
            "theme": serde_json::Value::Object(shared),
            "light": &vars_obj,
            "dark": vars_obj,
        }
    });
    root.to_string()
}

/// Formats CSS custom properties (`--md-sys-color-*`, `--md-sys-shape-corner-*`,
/// `--md-sys-motion-spring-*`, `--md-ref-typeface-*`) for `mode`.
pub fn m3_css_custom_properties(mode: M3Mode) -> String {
    let palette = M3RolePalette::for_mode(mode);
    let mut out = String::new();
    let selector = match mode {
        M3Mode::Light => ":root, [data-m3-mode=\"light\"]",
        M3Mode::Dark => "[data-m3-mode=\"dark\"]",
        M3Mode::NeuralObsidian => "[data-m3-mode=\"neural_obsidian\"]",
        M3Mode::OledPitch => "[data-m3-mode=\"oled_pitch\"]",
    };
    out.push_str(&format!("{selector} {{\n"));
    out.push_str(&format!(
        "  --md-ref-typeface-brand: \"{M3_FONT_BRAND}\", Inter, system-ui, sans-serif;\n"
    ));
    out.push_str(&format!(
        "  --md-ref-typeface-mono: \"{M3_FONT_MONO}\", Roboto Mono, monospace;\n"
    ));
    for (role, color) in palette.role_entries() {
        out.push_str(&format!("  --md-sys-color-{role}: {};\n", color.to_hex()));
    }
    out.push_str(&format!(
        "  --md-sys-shape-corner-none: {}px;\n",
        M3_SHAPES.none_px
    ));
    out.push_str(&format!(
        "  --md-sys-shape-corner-extra-small: {}px;\n",
        M3_SHAPES.extra_small_px
    ));
    out.push_str(&format!(
        "  --md-sys-shape-corner-small: {}px;\n",
        M3_SHAPES.small_px
    ));
    out.push_str(&format!(
        "  --md-sys-shape-corner-medium: {}px;\n",
        M3_SHAPES.medium_px
    ));
    out.push_str(&format!(
        "  --md-sys-shape-corner-large: {}px;\n",
        M3_SHAPES.large_px
    ));
    out.push_str(&format!(
        "  --md-sys-shape-corner-large-increased: {}px;\n",
        M3_SHAPES.large_increased_px
    ));
    out.push_str(&format!(
        "  --md-sys-shape-corner-extra-large: {}px;\n",
        M3_SHAPES.extra_large_px
    ));
    out.push_str(&format!(
        "  --md-sys-shape-corner-full: {}px;\n",
        M3_SHAPES.full_px
    ));
    for preset in M3SpringPreset::ALL {
        let spec = preset.spec();
        let slug = spec.name.replace('_', "-");
        out.push_str(&format!(
            "  --md-sys-motion-spring-{slug}-stiffness: {:.1};\n",
            spec.stiffness
        ));
        out.push_str(&format!(
            "  --md-sys-motion-spring-{slug}-damping: {:.2};\n",
            spec.damping_ratio
        ));
    }
    out.push_str("}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use nsh_core::Color;

    fn hex24(c: Hsla) -> u32 {
        Color::from(c).0 >> 8
    }

    #[test]
    fn all_four_modes_produce_valid_themes_and_pass_wcag_aa() {
        let expected_backgrounds = [
            (M3Mode::Light, 0xFAF9F9_u32, false),
            (M3Mode::Dark, 0x131314_u32, true),
            (M3Mode::NeuralObsidian, 0x0E0E0E_u32, true),
            (M3Mode::OledPitch, 0x000000_u32, true),
        ];

        for (mode, expected_bg, is_dark) in expected_backgrounds {
            let palette = M3RolePalette::for_mode(mode);
            assert_eq!(palette.mode, mode);
            assert_eq!(mode.is_dark(), is_dark);
            palette
                .verify_wcag_aa()
                .unwrap_or_else(|e| panic!("WCAG failure for {mode:?}: {e}"));

            let theme = m3_theme(mode);
            assert_eq!(hex24(theme.background.light), expected_bg);
            assert_eq!(hex24(theme.background.dark), expected_bg);
            assert_eq!(theme.font_sans.as_deref(), Some(M3_FONT_PLAIN));
            assert_eq!(theme.font_mono.as_deref(), Some(M3_FONT_MONO));
            assert_eq!(theme.radius, px(M3_SHAPES.medium_px));
            assert!(theme.color("md-surface-container").is_some());
            assert!(theme.color("md-primary-container").is_some());
            assert!(theme.color("md-secondary-container").is_some());
            assert!(theme.color("md-tertiary-container").is_some());
            assert!(theme.color("destructive-foreground").is_some());

            // Check shadcn theme contrast as well
            let fg_bg = wcag_contrast_ratio(theme.foreground.dark, theme.background.dark);
            let pri_fg = wcag_contrast_ratio(theme.primary_foreground.dark, theme.primary.dark);
            let sec_fg =
                wcag_contrast_ratio(theme.secondary_foreground.dark, theme.secondary.dark);
            let acc_fg = wcag_contrast_ratio(theme.accent_foreground.dark, theme.accent.dark);
            assert!(fg_bg >= WCAG_AA_MIN_CONTRAST, "{mode:?} fg/bg {fg_bg:.2}");
            assert!(
                pri_fg >= WCAG_AA_MIN_CONTRAST,
                "{mode:?} pri_fg/pri {pri_fg:.2}"
            );
            assert!(
                sec_fg >= WCAG_AA_MIN_CONTRAST,
                "{mode:?} sec_fg/sec {sec_fg:.2}"
            );
            assert!(
                acc_fg >= WCAG_AA_MIN_CONTRAST,
                "{mode:?} acc_fg/acc {acc_fg:.2}"
            );
        }

        // Individual convenience factories
        assert_eq!(hex24(m3_light_theme().background.light), 0xFAF9F9);
        assert_eq!(hex24(m3_dark_theme().background.dark), 0x131314);
        assert_eq!(hex24(m3_neural_obsidian_theme().background.dark), 0x0E0E0E);
        assert_eq!(hex24(m3_oled_pitch_theme().background.dark), 0x000000);

        // Adaptive factory preserves Light in .light and chosen dark variant in .dark
        let adaptive_oled = m3_adaptive_theme(M3Mode::OledPitch);
        assert_eq!(hex24(adaptive_oled.background.light), 0xFAF9F9);
        assert_eq!(hex24(adaptive_oled.background.dark), 0x000000);
    }

    #[test]
    fn spring_presets_match_m3_spatial_and_effects_physics() {
        let spatial_cases = [
            (M3SpringPreset::SpatialFast, 1400.0_f32, 0.90_f32),
            (M3SpringPreset::SpatialDefault, 700.0_f32, 0.90_f32),
            (M3SpringPreset::SpatialSlow, 300.0_f32, 0.90_f32),
        ];
        for (preset, stiffness, zeta) in spatial_cases {
            let spec = preset.spec();
            assert!(spec.is_spatial);
            assert_eq!(spec.stiffness, stiffness);
            assert_eq!(spec.damping_ratio, zeta);
            let spring = preset.to_nsh_spring();
            let expected_c = 2.0 * zeta * stiffness.sqrt();
            assert!((spring.damping - expected_c).abs() < 1e-3);
            assert!(preset.settle_ms() > 0.0);
            let transition = preset.to_layout_transition();
            assert!(transition.run_ms() > 0.0);
        }

        let effects_cases = [
            (M3SpringPreset::EffectsFast, 3800.0_f32, 1.00_f32),
            (M3SpringPreset::EffectsDefault, 1600.0_f32, 1.00_f32),
            (M3SpringPreset::EffectsSlow, 800.0_f32, 1.00_f32),
        ];
        for (preset, stiffness, zeta) in effects_cases {
            let spec = preset.spec();
            assert!(!spec.is_spatial);
            assert_eq!(spec.stiffness, stiffness);
            assert_eq!(spec.damping_ratio, zeta);
            let spring = preset.to_nsh_spring();
            // Critically damped effects springs must not overshoot 1.0
            for step in 0..=40 {
                let p = spring.progress(step as f32 * 5.0);
                assert!(p <= 1.001, "{preset:?} overshot at step {step}: {p}");
            }
        }
    }

    #[test]
    fn shape_tokens_and_json_round_trip_with_theme_parser() {
        assert_eq!(M3_SHAPES.none(), px(0.0));
        assert_eq!(M3_SHAPES.extra_small(), px(4.0));
        assert_eq!(M3_SHAPES.small(), px(8.0));
        assert_eq!(M3_SHAPES.medium(), px(12.0));
        assert_eq!(M3_SHAPES.large(), px(16.0));
        assert_eq!(M3_SHAPES.large_increased(), px(20.0));
        assert_eq!(M3_SHAPES.extra_large(), px(28.0));
        assert_eq!(M3_SHAPES.full(), px(9999.0));

        for mode in M3Mode::ALL {
            let json = m3_theme_css_vars_json(mode);
            let (parsed, ignored) = Theme::from_json_lenient(&json).unwrap();
            assert!(ignored.is_empty(), "{mode:?} ignored tokens: {ignored:?}");
            assert_eq!(
                hex24(parsed.background.dark),
                hex24(m3_theme(mode).background.dark)
            );
            let css = m3_css_custom_properties(mode);
            assert!(css.contains("--md-sys-color-primary:"));
            assert!(css.contains("--md-sys-motion-spring-spatial-fast-stiffness: 1400.0;"));
        }
    }

    #[test]
    fn bundled_google_design_toml_matches_m3_palettes_and_springs() {
        let toml_text = brand::design::BUNDLED_GOOGLE_DESIGN_TOKENS_TOML;
        for mode in M3Mode::ALL {
            let palette = M3RolePalette::for_mode(mode);
            let section_header = format!("[sys.color.{}]", mode.slug());
            assert!(toml_text.contains(&section_header), "missing {section_header}");
            let bg_hex = palette.background.to_hex();
            assert!(
                toml_text.contains(&format!("background = \"{bg_hex}\"")),
                "{mode:?} background {bg_hex} mismatch against google_design_tokens.toml"
            );
        }
        assert_eq!(M3Mode::from_name("neural-obsidian"), Some(M3Mode::NeuralObsidian));
        assert_eq!(M3Mode::from_name("oled"), Some(M3Mode::OledPitch));
        assert_eq!(M3Mode::Light.next().next().next().next(), M3Mode::Light);
    }
}
