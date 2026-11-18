//! Native Google Design Material 3 Expressive + Neural Expressive theme system and component kit.
//!
//! Derived directly from `assets/os/google_design_tokens.toml` (`https://design.google/`),
//! supporting all 4 appearance modes (`Light`, `Dark`, `NeuralObsidian`, `OledPitch`),
//! Google Sans Flex variable axes (`opsz`, `wght`, `wdth`, `GRAD`, `slnt`, `ROND`) +
//! `JetBrains Mono`, Spatial (`0.9` damping) / Effects (`1.0` damping) spring presets,
//! WCAG 2.2 AA (`>= 4.5:1`) contrast validation, and 16 native `RenderOnce` components.

use std::collections::BTreeSet;
use std::rc::Rc;
use std::time::Duration;

use anyhow::{Context as _, Result, ensure};
use gpui::{
    AnyElement, App, BoxShadow, ClickEvent, ElementId, Entity, FontWeight, Global, Hsla,
    InteractiveElement, IntoElement, KeyDownEvent, MouseButton, ParentElement, RenderOnce, Rgba,
    Role, SharedString, SpringConfig, StatefulInteractiveElement as _, StyleRefinement, Styled,
    Window, anchored, div, hsla, point, prelude::FluentBuilder as _, px, transparent_black,
};
use gpui_base::{
    Button as BaseButton, FocusTrapElement as _, LogicalStyledExt as _, Switch as BaseSwitch,
    SwitchThumb, SwitchTrack, TestSupportExt as _, Toggle as BaseToggle,
    ToggleGroup as BaseToggleGroup, actions::Cancel, spring,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::{
    Disableable, Icon, IconName, MotionTokens, Selectable, Sizable, Size, StyledExt as _, Theme,
    ThemeConfig, ThemeConfigColors, ThemeMode, box_shadow, h_flex,
    input::{Input, InputState},
    spinner::Spinner,
    styled::{FocusLine, focus_style},
    v_flex,
};

/// Authoritative URL for Google Design / Material 3 Expressive tokens.
pub const M3_DESIGN_SOURCE_OF_TRUTH: &str = brand::design::SOURCE_OF_TRUTH;

/// Minimum WCAG 2.2 AA contrast ratio for normal body and UI control text.
pub const M3_WCAG_AA_MIN_CONTRAST: f32 = brand::design::WCAG_AA_MIN_CONTRAST;

/// Minimum WCAG 2.2 AA contrast ratio for large display text (`>= 18pt` or `14pt` bold) and UI graphics.
pub const M3_WCAG_AA_LARGE_CONTRAST: f32 = brand::design::WCAG_AA_LARGE_CONTRAST;

/// Embedded 3-tier Material 3 Expressive + Google Sans Flex manifest (`assets/os/google_design_tokens.toml`).
pub use brand::design::BUNDLED_GOOGLE_DESIGN_TOKENS_TOML;

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;
type BoolChangeHandler = Rc<dyn Fn(&bool, &mut Window, &mut App) + 'static>;
type IdSelectHandler = Rc<dyn Fn(&SharedString, &mut Window, &mut App) + 'static>;
type ActionHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;
type QueryHandler = Rc<dyn Fn(&SharedString, &mut Window, &mut App) + 'static>;

/// Four authoritative Aphrody Material 3 Expressive + Neural Expressive appearance modes.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum M3AppearanceMode {
    Light,
    #[default]
    Dark,
    NeuralObsidian,
    OledPitch,
}

impl M3AppearanceMode {
    /// All four Material 3 Expressive surface modes in canonical order.
    pub const ALL: [Self; 4] = [
        Self::Light,
        Self::Dark,
        Self::NeuralObsidian,
        Self::OledPitch,
    ];

    /// TOML section key suffix (`light`, `dark`, `neural_obsidian`, `oled_pitch`).
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::NeuralObsidian => "neural_obsidian",
            Self::OledPitch => "oled_pitch",
        }
    }

    #[must_use]
    const fn to_brand_mode(self) -> brand::design::ThemeModeSlug {
        match self {
            Self::Light => brand::design::ThemeModeSlug::Light,
            Self::Dark => brand::design::ThemeModeSlug::Dark,
            Self::NeuralObsidian => brand::design::ThemeModeSlug::NeuralObsidian,
            Self::OledPitch => brand::design::ThemeModeSlug::OledPitch,
        }
    }

    /// Human-readable theme label used in [`ThemeConfig::name`].
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Light => "Google Design Light",
            Self::Dark => "Google Design Dark",
            Self::NeuralObsidian => "Google Neural Obsidian",
            Self::OledPitch => "Google OLED Pitch",
        }
    }

    /// Parses mode aliases (`"light"`, `"dark"`, `"neural_obsidian"`, `"oled_pitch"`, etc.).
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "light" | "material_light" | "material-light" => Some(Self::Light),
            "dark" | "material_dark" | "material-dark" => Some(Self::Dark),
            "neural_obsidian" | "neural-obsidian" | "neural" | "obsidian" => {
                Some(Self::NeuralObsidian)
            }
            "oled_pitch" | "oled-pitch" | "oled" | "pitch" => Some(Self::OledPitch),
            _ => None,
        }
    }

    /// Whether this mode uses dark polarity surfaces (`Dark`, `NeuralObsidian`, `OledPitch`).
    #[must_use]
    pub const fn is_dark(&self) -> bool {
        !matches!(self, Self::Light)
    }

    /// Maps to `gpui-component`'s binary [`ThemeMode`].
    #[must_use]
    pub const fn theme_mode(&self) -> ThemeMode {
        if self.is_dark() {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        }
    }
}

impl From<ThemeMode> for M3AppearanceMode {
    fn from(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Light => Self::Light,
            ThemeMode::Dark => Self::Dark,
        }
    }
}

impl From<M3AppearanceMode> for ThemeMode {
    fn from(mode: M3AppearanceMode) -> Self {
        mode.theme_mode()
    }
}

/// Variable axis bounds from `[ref.variable_axes.*]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct M3AxisBounds {
    pub tag: String,
    pub name: String,
    pub min: f32,
    pub default_value: f32,
    pub max: f32,
}

/// Google Sans Flex variable font axes (`opsz`, `wght`, `wdth`, `GRAD`, `slnt`, `ROND`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct M3GoogleSansFlexAxes {
    pub opsz: f32,
    pub wght: f32,
    pub wdth: f32,
    pub grad: f32,
    pub slnt: f32,
    pub rond: f32,
}

impl Default for M3GoogleSansFlexAxes {
    fn default() -> Self {
        Self {
            opsz: 14.0,
            wght: 400.0,
            wdth: 100.0,
            grad: 0.0,
            slnt: 0.0,
            rond: 0.0,
        }
    }
}

impl M3GoogleSansFlexAxes {
    /// Expressive display hero headline preset matching Google Neural Expressive canvas.
    #[must_use]
    pub const fn display_hero() -> Self {
        Self {
            opsz: 48.0,
            wght: 500.0,
            wdth: 100.0,
            grad: 0.0,
            slnt: 0.0,
            rond: 40.0,
        }
    }

    /// Section headline preset for cards, app bars, and dialog sheets.
    #[must_use]
    pub const fn section_headline() -> Self {
        Self {
            opsz: 28.0,
            wght: 500.0,
            wdth: 100.0,
            grad: 0.0,
            slnt: 0.0,
            rond: 25.0,
        }
    }

    /// Interactive control preset with expressive roundness for pill buttons and chips.
    #[must_use]
    pub const fn ui_control() -> Self {
        Self {
            opsz: 14.0,
            wght: 600.0,
            wdth: 100.0,
            grad: 0.0,
            slnt: 0.0,
            rond: 20.0,
        }
    }

    /// Compact mobile and high-density telemetry preset.
    #[must_use]
    pub const fn compact_mobile() -> Self {
        Self {
            opsz: 12.0,
            wght: 500.0,
            wdth: 95.0,
            grad: 10.0,
            slnt: 0.0,
            rond: 16.0,
        }
    }

    /// Tabular label preset for counters, badges, and status pills.
    #[must_use]
    pub const fn tabular_label() -> Self {
        Self {
            opsz: 11.0,
            wght: 600.0,
            wdth: 100.0,
            grad: 25.0,
            slnt: 0.0,
            rond: 0.0,
        }
    }

    /// Glanceable preset for additive transparent surfaces (`GRAD > 0` boosts stroke weight).
    #[must_use]
    pub const fn xr_glimmer_glanceable() -> Self {
        Self {
            opsz: 10.0,
            wght: 600.0,
            wdth: 104.0,
            grad: 20.0,
            slnt: 0.0,
            rond: 15.0,
        }
    }

    /// Long-form reading preset for body copy and documents.
    #[must_use]
    pub const fn quire_ink_body() -> Self {
        Self {
            opsz: 16.0,
            wght: 420.0,
            wdth: 100.0,
            grad: 0.0,
            slnt: 0.0,
            rond: 8.0,
        }
    }

    /// Clamps all 6 variable axes to the authoritative bounds in `assets/os/google_design_tokens.toml`.
    #[must_use]
    pub fn clamped(self) -> Self {
        Self {
            opsz: self.opsz.clamp(6.0, 144.0),
            wght: self.wght.clamp(100.0, 1000.0),
            wdth: self.wdth.clamp(25.0, 151.0),
            grad: self.grad.clamp(-50.0, 200.0),
            slnt: self.slnt.clamp(-10.0, 0.0),
            rond: self.rond.clamp(0.0, 100.0),
        }
    }

    /// Converts the variable `wght` axis to a GPUI [`FontWeight`].
    #[must_use]
    pub fn gpui_font_weight(self) -> FontWeight {
        FontWeight(self.wght.clamp(100.0, 1000.0))
    }

    /// Formats CSS `font-variation-settings` string across all 6 axes.
    #[must_use]
    pub fn css_font_variation_settings(&self) -> String {
        let clamped = self.clamped();
        format!(
            "\"opsz\" {:.1}, \"wght\" {:.0}, \"wdth\" {:.1}, \"GRAD\" {:.0}, \"slnt\" {:.1}, \"ROND\" {:.0}",
            clamped.opsz, clamped.wght, clamped.wdth, clamped.grad, clamped.slnt, clamped.rond,
        )
    }

    /// Validates that every axis lies within the specification bounds from `google_design_tokens.toml`.
    pub fn validate(&self) -> Result<()> {
        let checks = [
            ("opsz", self.opsz),
            ("wght", self.wght),
            ("wdth", self.wdth),
            ("GRAD", self.grad),
            ("slnt", self.slnt),
            ("ROND", self.rond),
        ];
        for (tag, value) in checks {
            let bounds = brand::design::axis_spec(tag)
                .with_context(|| format!("missing variable axis `{tag}` in token manifest"))?;
            ensure!(
                value.is_finite() && value >= bounds.min && value <= bounds.max,
                "Google Sans Flex axis `{tag}` value {value} outside [{}, {}]",
                bounds.min,
                bounds.max
            );
        }
        Ok(())
    }
}

/// Font stack roles from `[ref.typeface]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct M3TypefaceTokens {
    pub brand: SharedString,
    pub plain: SharedString,
    pub mono: SharedString,
    pub fallback: SharedString,
}

impl Default for M3TypefaceTokens {
    fn default() -> Self {
        Self {
            brand: brand::design::TYPEFACE_BRAND.into(),
            plain: brand::design::TYPEFACE_PLAIN.into(),
            mono: brand::design::TYPEFACE_MONO.into(),
            fallback: brand::design::TYPEFACE_FALLBACK.into(),
        }
    }
}

impl M3TypefaceTokens {
    /// Extracts the primary family name from a comma-separated CSS font stack.
    #[must_use]
    pub fn primary_family(stack: &str) -> SharedString {
        stack
            .split(',')
            .next()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or("Google Sans Flex")
            .to_owned()
            .into()
    }

    /// Primary UI display/sans family name (`Google Sans Flex`).
    #[must_use]
    pub fn brand_family(&self) -> SharedString {
        Self::primary_family(&self.brand)
    }

    /// Primary monospace family name (`JetBrains Mono`).
    #[must_use]
    pub fn mono_family(&self) -> SharedString {
        Self::primary_family(&self.mono)
    }
}

/// Material 3 Expressive typescale entry from `[sys.typescale.*]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct M3TypeScaleEntry {
    pub font_role: SharedString,
    pub size_sp: f32,
    pub line_height_sp: f32,
    pub weight: u16,
    pub tracking_sp: f32,
}

impl M3TypeScaleEntry {
    /// Returns the GPUI [`FontWeight`] corresponding to this typescale entry.
    #[must_use]
    pub fn font_weight(&self) -> FontWeight {
        FontWeight(f32::from(self.weight).clamp(100.0, 1000.0))
    }
}

/// Complete 15-step Material 3 Expressive typography scale + typeface stacks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct M3TypographyScale {
    pub typeface: M3TypefaceTokens,
    pub display_large: M3TypeScaleEntry,
    pub display_medium: M3TypeScaleEntry,
    pub display_small: M3TypeScaleEntry,
    pub headline_large: M3TypeScaleEntry,
    pub headline_medium: M3TypeScaleEntry,
    pub headline_small: M3TypeScaleEntry,
    pub title_large: M3TypeScaleEntry,
    pub title_medium: M3TypeScaleEntry,
    pub title_small: M3TypeScaleEntry,
    pub body_large: M3TypeScaleEntry,
    pub body_medium: M3TypeScaleEntry,
    pub body_small: M3TypeScaleEntry,
    pub label_large: M3TypeScaleEntry,
    pub label_medium: M3TypeScaleEntry,
    pub label_small: M3TypeScaleEntry,
}

impl Default for M3TypographyScale {
    fn default() -> Self {
        Self::bundled_or_default()
    }
}

impl M3TypographyScale {
    /// Resolves the typography scale from `assets/os/google_design_tokens.toml` via `brand::design`.
    pub fn try_bundled() -> Result<Self> {
        Ok(Self::bundled_or_default())
    }

    #[must_use]
    pub fn bundled_or_default() -> Self {
        let to_entry = |src: brand::design::TypeScaleEntry| M3TypeScaleEntry {
            font_role: src.font.into(),
            size_sp: src.size_sp,
            line_height_sp: src.line_height_sp,
            weight: src.weight.round().clamp(100.0, 1000.0) as u16,
            tracking_sp: src.tracking_sp,
        };
        let src = &brand::design::TYPESCALE;
        Self {
            typeface: M3TypefaceTokens::default(),
            display_large: to_entry(src.display_large),
            display_medium: to_entry(src.display_medium),
            display_small: to_entry(src.display_small),
            headline_large: to_entry(src.headline_large),
            headline_medium: to_entry(src.headline_medium),
            headline_small: to_entry(src.headline_small),
            title_large: to_entry(src.title_large),
            title_medium: to_entry(src.title_medium),
            title_small: to_entry(src.title_small),
            body_large: to_entry(src.body_large),
            body_medium: to_entry(src.body_medium),
            body_small: to_entry(src.body_small),
            label_large: to_entry(src.label_large),
            label_medium: to_entry(src.label_medium),
            label_small: to_entry(src.label_small),
        }
    }
}

/// Material 3 Expressive corner shape scale from `[sys.shape]`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct M3ShapeTokens {
    pub none_px: f32,
    pub extra_small_px: f32,
    pub small_px: f32,
    pub medium_px: f32,
    pub large_px: f32,
    pub large_increased_px: f32,
    pub extra_large_px: f32,
    pub full_px: f32,
}

impl Default for M3ShapeTokens {
    fn default() -> Self {
        Self::bundled_or_default()
    }
}

impl M3ShapeTokens {
    pub fn try_bundled() -> Result<Self> {
        Ok(Self::bundled_or_default())
    }

    #[must_use]
    pub const fn bundled_or_default() -> Self {
        Self {
            none_px: brand::design::SHAPES.none_px,
            extra_small_px: brand::design::SHAPES.extra_small_px,
            small_px: brand::design::SHAPES.small_px,
            medium_px: brand::design::SHAPES.medium_px,
            large_px: brand::design::SHAPES.large_px,
            large_increased_px: brand::design::SHAPES.large_increased_px,
            extra_large_px: brand::design::SHAPES.extra_large_px,
            full_px: brand::design::SHAPES.full_px,
        }
    }
}

/// Material 3 Expressive state-layer opacity tokens from `[sys.state]`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct M3StateLayerTokens {
    pub hover_opacity: f32,
    pub focus_opacity: f32,
    pub pressed_opacity: f32,
    pub dragged_opacity: f32,
    pub disabled_container_opacity: f32,
    pub disabled_content_opacity: f32,
    pub focus_ring_px: f32,
}

impl Default for M3StateLayerTokens {
    fn default() -> Self {
        Self::bundled_or_default()
    }
}

impl M3StateLayerTokens {
    pub fn try_bundled() -> Result<Self> {
        Ok(Self::bundled_or_default())
    }

    #[must_use]
    pub const fn bundled_or_default() -> Self {
        Self {
            hover_opacity: brand::design::STATES.hover_opacity,
            focus_opacity: brand::design::STATES.focus_opacity,
            pressed_opacity: brand::design::STATES.pressed_opacity,
            dragged_opacity: brand::design::STATES.dragged_opacity,
            disabled_container_opacity: brand::design::STATES.disabled_container_opacity,
            disabled_content_opacity: brand::design::STATES.disabled_content_opacity,
            focus_ring_px: brand::design::STATES.focus_ring_px,
        }
    }
}

/// Material 3 Expressive density and touch target constraints from `[sys.density]`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct M3DensityTokens {
    pub default: i32,
    pub compact_1: i32,
    pub compact_2: i32,
    pub compact_3: i32,
    pub touch_target_min_px: f32,
    pub control_height_min_px: f32,
}

impl Default for M3DensityTokens {
    fn default() -> Self {
        Self::bundled_or_default()
    }
}

impl M3DensityTokens {
    pub fn try_bundled() -> Result<Self> {
        Ok(Self::bundled_or_default())
    }

    #[must_use]
    pub const fn bundled_or_default() -> Self {
        Self {
            default: brand::design::DENSITY.default,
            compact_1: brand::design::DENSITY.compact_1,
            compact_2: brand::design::DENSITY.compact_2,
            compact_3: brand::design::DENSITY.compact_3,
            touch_target_min_px: brand::design::DENSITY.touch_target_min_px,
            control_height_min_px: brand::design::DENSITY.control_height_min_px,
        }
    }
}

/// Responsive Material 3 window size classes from `[sys.window_size_classes]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum M3WindowSizeClass {
    Compact,
    Medium,
    Expanded,
    Large,
    ExtraLarge,
}

impl M3WindowSizeClass {
    /// Classifies viewport width in logical pixels against `[sys.window_size_classes]`.
    #[must_use]
    pub fn from_width_px(width_px: f32) -> Self {
        match brand::design::classify_window_width(width_px) {
            brand::design::WindowSizeTier::Compact => Self::Compact,
            brand::design::WindowSizeTier::Medium => Self::Medium,
            brand::design::WindowSizeTier::Expanded => Self::Expanded,
            brand::design::WindowSizeTier::Large => Self::Large,
            brand::design::WindowSizeTier::ExtraLarge => Self::ExtraLarge,
        }
    }
}

/// Analytical harmonic oscillator parameters for a spring token.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct M3SpringSpec {
    pub stiffness: f32,
    pub damping_ratio: f32,
}

impl M3SpringSpec {
    /// Evaluates normalized `0.0 -> 1.0` displacement at `t_secs` for a unit-mass spring.
    #[must_use]
    pub fn sample_position(&self, t_secs: f32) -> f32 {
        brand::design::sample_spring_position(self.stiffness, self.damping_ratio, t_secs)
    }

    /// Natural period $2\pi / \sqrt{k}$ used as the `response` argument of [`gpui_base::Spring`].
    #[must_use]
    pub fn response_duration(&self) -> Duration {
        let omega0 = self.stiffness.max(1.0).sqrt();
        Duration::from_secs_f32((std::f32::consts::TAU / omega0).max(0.001))
    }

    /// Converts into [`gpui_base::Spring`] while safely clamping invalid negative ratios.
    #[must_use]
    pub fn gpui_spring(&self) -> gpui_base::Spring {
        let spring = gpui_base::Spring::new(self.response_duration());
        spring
            .try_with_damping(self.damping_ratio.max(0.05))
            .unwrap_or(spring)
    }

    /// Converts into GPUI's low-level [`SpringConfig`] (`c = 2 * zeta * sqrt(k)` for unit mass).
    #[must_use]
    pub fn gpui_spring_config(&self) -> SpringConfig {
        let stiffness = self.stiffness.max(1.0);
        let damping = 2.0 * self.damping_ratio.max(0.0) * stiffness.sqrt();
        SpringConfig::new(stiffness, damping, 1.0)
    }
}

/// Material 3 Expressive motion spring tokens (`Spatial` at `0.9` damping, `Effects` at `1.0` damping,
/// plus Quire reference presets from `[sys.motion.springs.*]`).
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum M3SpringToken {
    SpatialFast,
    SpatialDefault,
    SpatialSlow,
    EffectsFast,
    EffectsDefault,
    EffectsSlow,
    QuireUnfold,
    QuireSettle,
    QuireExpand,
    QuireFold,
    QuireTick,
}

impl M3SpringToken {
    /// Core 6 Material 3 Expressive Spatial (`0.9` damping) and Effects (`1.0` damping) springs.
    pub const CORE_EXPRESSIVE: [Self; 6] = [
        Self::SpatialFast,
        Self::SpatialDefault,
        Self::SpatialSlow,
        Self::EffectsFast,
        Self::EffectsDefault,
        Self::EffectsSlow,
    ];

    /// All 11 spring presets defined in `assets/os/google_design_tokens.toml`.
    pub const ALL: [Self; 11] = [
        Self::SpatialFast,
        Self::SpatialDefault,
        Self::SpatialSlow,
        Self::EffectsFast,
        Self::EffectsDefault,
        Self::EffectsSlow,
        Self::QuireUnfold,
        Self::QuireSettle,
        Self::QuireExpand,
        Self::QuireFold,
        Self::QuireTick,
    ];

    /// TOML key name under `[sys.motion.springs.*]`.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::SpatialFast => "spatial_fast",
            Self::SpatialDefault => "spatial_default",
            Self::SpatialSlow => "spatial_slow",
            Self::EffectsFast => "effects_fast",
            Self::EffectsDefault => "effects_default",
            Self::EffectsSlow => "effects_slow",
            Self::QuireUnfold => "quire_unfold",
            Self::QuireSettle => "quire_settle",
            Self::QuireExpand => "quire_expand",
            Self::QuireFold => "quire_fold",
            Self::QuireTick => "quire_tick",
        }
    }

    /// Whether this token is a Spatial spring (position/size/morph motion with `0.9` damping).
    #[must_use]
    pub const fn is_spatial(&self) -> bool {
        matches!(
            self,
            Self::SpatialFast | Self::SpatialDefault | Self::SpatialSlow | Self::QuireSettle
        )
    }

    /// Whether this token is an Effects spring (opacity/color/elevation with `1.0` critical damping).
    #[must_use]
    pub const fn is_effects(&self) -> bool {
        matches!(
            self,
            Self::EffectsFast | Self::EffectsDefault | Self::EffectsSlow | Self::QuireFold
        )
    }

    /// Resolves spring stiffness & damping ratio from the bundled `google_design_tokens.toml` manifest.
    pub fn try_spec(&self) -> Result<M3SpringSpec> {
        let spec = brand::design::spring_spec(self.as_str())
            .with_context(|| format!("missing M3 spring `{}` in token manifest", self.as_str()))?;
        Ok(M3SpringSpec {
            stiffness: spec.stiffness,
            damping_ratio: spec.damping_ratio,
        })
    }

    /// Resolves spring stiffness & damping ratio from `brand::design`.
    #[must_use]
    pub const fn spec(&self) -> M3SpringSpec {
        match brand::design::spring_spec_const(self.as_str()) {
            Some(spec) => M3SpringSpec {
                stiffness: spec.stiffness,
                damping_ratio: spec.damping_ratio,
            },
            None => M3SpringSpec {
                stiffness: 700.0,
                damping_ratio: 0.9,
            },
        }
    }

    /// Evaluates normalized displacement `0.0 -> 1.0` at `t_secs`.
    #[must_use]
    pub fn sample_position(&self, t_secs: f32) -> f32 {
        self.spec().sample_position(t_secs)
    }

    /// Natural period duration for [`gpui_base::Spring`].
    #[must_use]
    pub fn response_duration(&self) -> Duration {
        self.spec().response_duration()
    }

    /// Builds a [`gpui_base::Spring`] tuned to this M3 spring token.
    #[must_use]
    pub fn gpui_spring(&self) -> gpui_base::Spring {
        self.spec().gpui_spring()
    }

    /// Builds a [`gpui::SpringConfig`] tuned to this M3 spring token.
    #[must_use]
    pub fn gpui_spring_config(&self) -> SpringConfig {
        self.spec().gpui_spring_config()
    }
}

/// Builds `gpui-component` [`MotionTokens`] anchored to M3 Spatial (`0.9`) & Effects (`1.0`) springs.
#[must_use]
pub fn m3_motion_tokens() -> MotionTokens {
    let mut tokens = MotionTokens::default();
    tokens.duration_fast = M3SpringToken::EffectsFast.response_duration();
    tokens.duration_normal = M3SpringToken::EffectsDefault.response_duration();
    tokens.duration_slow = M3SpringToken::SpatialSlow.response_duration();
    tokens.spring_control = M3SpringToken::EffectsDefault.gpui_spring();
    tokens.spring_move = M3SpringToken::SpatialDefault.gpui_spring().with_epsilon(0.1);
    tokens
}

/// Complete 30-role Material 3 semantic color palette for a single [`M3AppearanceMode`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct M3ColorRoles {
    pub background: Hsla,
    pub on_background: Hsla,
    pub surface: Hsla,
    pub surface_dim: Hsla,
    pub surface_bright: Hsla,
    pub surface_container_lowest: Hsla,
    pub surface_container_low: Hsla,
    pub surface_container: Hsla,
    pub surface_container_high: Hsla,
    pub surface_container_highest: Hsla,
    pub on_surface: Hsla,
    pub on_surface_variant: Hsla,
    pub outline: Hsla,
    pub outline_variant: Hsla,
    pub primary: Hsla,
    pub on_primary: Hsla,
    pub primary_container: Hsla,
    pub on_primary_container: Hsla,
    pub secondary: Hsla,
    pub on_secondary: Hsla,
    pub secondary_container: Hsla,
    pub on_secondary_container: Hsla,
    pub tertiary: Hsla,
    pub on_tertiary: Hsla,
    pub tertiary_container: Hsla,
    pub on_tertiary_container: Hsla,
    pub error: Hsla,
    pub on_error: Hsla,
    pub error_container: Hsla,
    pub on_error_container: Hsla,
}

impl Default for M3ColorRoles {
    fn default() -> Self {
        Self::for_mode(M3AppearanceMode::Dark)
    }
}

impl M3ColorRoles {
    /// Resolves all 30 color roles for `mode` directly from `brand::design`.
    pub fn try_for_mode(mode: M3AppearanceMode) -> Result<Self> {
        Ok(Self::for_mode(mode))
    }

    /// Resolves all 30 color roles for `mode` directly from compile-time `brand::design` palettes.
    #[must_use]
    pub fn for_mode(mode: M3AppearanceMode) -> Self {
        let p = brand::design::palette_rgb(mode.to_brand_mode());
        let color = |rgb_u32: u32| -> Hsla { gpui::rgb(rgb_u32).into() };
        Self {
            background: color(p.background),
            on_background: color(p.on_background),
            surface: color(p.surface),
            surface_dim: color(p.surface_dim),
            surface_bright: color(p.surface_bright),
            surface_container_lowest: color(p.surface_container_lowest),
            surface_container_low: color(p.surface_container_low),
            surface_container: color(p.surface_container),
            surface_container_high: color(p.surface_container_high),
            surface_container_highest: color(p.surface_container_highest),
            on_surface: color(p.on_surface),
            on_surface_variant: color(p.on_surface_variant),
            outline: color(p.outline),
            outline_variant: color(p.outline_variant),
            primary: color(p.primary),
            on_primary: color(p.on_primary),
            primary_container: color(p.primary_container),
            on_primary_container: color(p.on_primary_container),
            secondary: color(p.secondary),
            on_secondary: color(p.on_secondary),
            secondary_container: color(p.secondary_container),
            on_secondary_container: color(p.on_secondary_container),
            tertiary: color(p.tertiary),
            on_tertiary: color(p.on_tertiary),
            tertiary_container: color(p.tertiary_container),
            on_tertiary_container: color(p.on_tertiary_container),
            error: color(p.error),
            on_error: color(p.on_error),
            error_container: color(p.error_container),
            on_error_container: color(p.on_error_container),
        }
    }

    /// Composites an M3 state layer (`state_ink` at `opacity`) over `base`.
    #[must_use]
    pub fn state_layer(&self, base: Hsla, state_ink: Hsla, opacity: f32) -> Hsla {
        base.blend(state_ink.alpha(opacity.clamp(0.0, 1.0)))
    }

    /// Canonical foreground/background pairs validated against WCAG 2.2 AA (`>= 4.5:1`).
    #[must_use]
    pub fn contrast_pairs(&self) -> [(&'static str, Hsla, Hsla); 14] {
        [
            ("on_background/background", self.on_background, self.background),
            ("on_surface/surface", self.on_surface, self.surface),
            (
                "on_surface/surface_container_low",
                self.on_surface,
                self.surface_container_low,
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
            (
                "on_surface_variant/surface_container_highest",
                self.on_surface_variant,
                self.surface_container_highest,
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
            ("on_error/error", self.on_error, self.error),
        ]
    }
}

/// Active Material 3 Expressive runtime theme stored as a GPUI [`Global`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct M3ActiveTheme {
    pub mode: M3AppearanceMode,
    pub colors: M3ColorRoles,
    pub shape: M3ShapeTokens,
    pub state: M3StateLayerTokens,
    pub density: M3DensityTokens,
    pub typography: M3TypographyScale,
    pub sans_flex_axes: M3GoogleSansFlexAxes,
}

impl Global for M3ActiveTheme {}

impl Default for M3ActiveTheme {
    fn default() -> Self {
        Self::for_mode(M3AppearanceMode::Dark)
    }
}

impl M3ActiveTheme {
    /// Constructs a full [`M3ActiveTheme`] for `mode` from `assets/os/google_design_tokens.toml`.
    #[must_use]
    pub fn for_mode(mode: M3AppearanceMode) -> Self {
        Self {
            mode,
            colors: M3ColorRoles::for_mode(mode),
            shape: M3ShapeTokens::bundled_or_default(),
            state: M3StateLayerTokens::bundled_or_default(),
            density: M3DensityTokens::bundled_or_default(),
            typography: M3TypographyScale::bundled_or_default(),
            sans_flex_axes: M3GoogleSansFlexAxes::ui_control(),
        }
    }

    /// Reads the active M3 theme purely from `cx` (compatible with `gpui-fast` retention).
    ///
    /// If the host switched [`Theme::mode`] via `Theme::change(...)` without calling
    /// [`apply_m3_theme`], this reconciles light/dark polarity without mutating globals during render.
    #[must_use]
    pub fn resolve(cx: &App) -> Self {
        let theme_is_dark = cx.try_global::<Theme>().map(|theme| theme.is_dark());
        if let Some(active) = cx.try_global::<Self>() {
            if let Some(is_dark) = theme_is_dark
                && is_dark != active.mode.is_dark()
            {
                return Self::for_mode(if is_dark {
                    M3AppearanceMode::Dark
                } else {
                    M3AppearanceMode::Light
                });
            }
            return active.clone();
        }
        Self::for_mode(if theme_is_dark.unwrap_or(false) {
            M3AppearanceMode::Dark
        } else {
            M3AppearanceMode::Light
        })
    }
}

/// Initializes Material 3 global theme state without overriding [`Theme`]'s default palette.
pub fn init(cx: &mut App) {
    if cx.has_global::<M3ActiveTheme>() {
        return;
    }
    let mode = cx
        .try_global::<Theme>()
        .map(|theme| M3AppearanceMode::from(theme.mode))
        .unwrap_or(M3AppearanceMode::Light);
    cx.set_global(M3ActiveTheme::for_mode(mode));
}

/// Reads the currently active [`M3ActiveTheme`] snapshot from `cx`.
#[must_use]
pub fn active_m3_theme(cx: &App) -> M3ActiveTheme {
    M3ActiveTheme::resolve(cx)
}

/// Computes W3C WCAG 2.2 relative luminance ($0.0..=1.0$) for an [`Hsla`] color.
#[must_use]
pub fn m3_relative_luminance(color: Hsla) -> f32 {
    let rgb: Rgba = color.to_rgb();
    brand::design::relative_luminance_srgb(rgb.r, rgb.g, rgb.b)
}

/// Computes WCAG 2.2 contrast ratio `(L1 + 0.05) / (L2 + 0.05)` in range `1.0..=21.0`.
#[must_use]
pub fn m3_contrast_ratio(foreground: Hsla, background: Hsla) -> f32 {
    let fg_rgb: Rgba = foreground.to_rgb();
    let bg_rgb: Rgba = background.to_rgb();
    brand::design::contrast_ratio_rgba(
        [fg_rgb.r, fg_rgb.g, fg_rgb.b, foreground.a],
        [bg_rgb.r, bg_rgb.g, bg_rgb.b],
    )
}

/// Returns `true` when `(foreground, background)` meets WCAG 2.2 AA (`>= 4.5:1`).
#[must_use]
pub fn m3_meets_wcag_aa(foreground: Hsla, background: Hsla) -> bool {
    m3_contrast_ratio(foreground, background) >= M3_WCAG_AA_MIN_CONTRAST
}

/// Verifies that `(foreground, background)` meets WCAG 2.2 AA (`>= 4.5:1`), returning the ratio.
pub fn m3_verify_contrast(label: &str, foreground: Hsla, background: Hsla) -> Result<f32> {
    let ratio = m3_contrast_ratio(foreground, background);
    ensure!(
        ratio >= M3_WCAG_AA_MIN_CONTRAST,
        "M3 contrast check `{label}` failed WCAG 2.2 AA: {ratio:.2}:1 < {M3_WCAG_AA_MIN_CONTRAST:.1}:1"
    );
    Ok(ratio)
}

/// Report for a single semantic foreground/background WCAG contrast check.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct M3ContrastReport {
    pub pair: &'static str,
    pub ratio: f32,
    pub passes_aa: bool,
    pub passes_aaa: bool,
}

/// Verifies all 14 canonical foreground/background role pairs in `mode` meet WCAG 2.2 AA (`>= 4.5:1`).
pub fn m3_verify_mode_contrast(mode: M3AppearanceMode) -> Result<Vec<M3ContrastReport>> {
    let roles = M3ColorRoles::try_for_mode(mode)?;
    let mut reports = Vec::with_capacity(14);
    for (pair, fg, bg) in roles.contrast_pairs() {
        let ratio = m3_contrast_ratio(fg, bg);
        ensure!(
            ratio >= M3_WCAG_AA_MIN_CONTRAST,
            "Mode {:?} pair `{pair}` contrast {ratio:.2}:1 < {:.1}:1",
            mode,
            M3_WCAG_AA_MIN_CONTRAST
        );
        reports.push(M3ContrastReport {
            pair,
            ratio,
            passes_aa: ratio >= M3_WCAG_AA_MIN_CONTRAST,
            passes_aaa: ratio >= 7.0,
        });
    }
    Ok(reports)
}

/// Chooses whichever of `preferred_dark` or `preferred_light` achieves higher contrast on `background`.
#[must_use]
pub fn m3_accessible_foreground(
    background: Hsla,
    preferred_dark: Hsla,
    preferred_light: Hsla,
) -> Hsla {
    if m3_contrast_ratio(preferred_dark, background) >= m3_contrast_ratio(preferred_light, background)
    {
        preferred_dark
    } else {
        preferred_light
    }
}

/// Formats an [`Hsla`] color as `#RRGGBB` or `#RRGGBBAA` without truncation drift.
#[must_use]
pub fn m3_hsla_to_hex(color: Hsla) -> SharedString {
    let rgb = color.to_rgb();
    let r = (rgb.r * 255.0).round().clamp(0.0, 255.0) as u8;
    let g = (rgb.g * 255.0).round().clamp(0.0, 255.0) as u8;
    let b = (rgb.b * 255.0).round().clamp(0.0, 255.0) as u8;
    let a = (color.a * 255.0).round().clamp(0.0, 255.0) as u8;
    if a < 255 {
        format!("#{r:02X}{g:02X}{b:02X}{a:02X}").into()
    } else {
        format!("#{r:02X}{g:02X}{b:02X}").into()
    }
}

/// Returns `(source_of_truth, system, seed_hex)` resolved from `brand::design`.
pub fn m3_manifest_metadata() -> Result<(&'static str, &'static str, &'static str)> {
    Ok((
        brand::design::SOURCE_OF_TRUTH,
        brand::design::SYSTEM_NAME,
        brand::design::SEED_HEX,
    ))
}

/// Builds a `gpui-component` [`ThemeConfig`] derived from `assets/os/google_design_tokens.toml` for `mode`.
pub fn try_m3_theme_config(mode: M3AppearanceMode) -> Result<ThemeConfig> {
    let roles = M3ColorRoles::for_mode(mode);
    let shape = M3ShapeTokens::bundled_or_default();
    let state = M3StateLayerTokens::bundled_or_default();
    let typography = M3TypographyScale::bundled_or_default();

    let hex = m3_hsla_to_hex;
    let hover_bg = |base: Hsla, ink: Hsla| hex(roles.state_layer(base, ink, state.hover_opacity));
    let active_bg = |base: Hsla, ink: Hsla| hex(roles.state_layer(base, ink, state.pressed_opacity));

    let mut colors = ThemeConfigColors::default();
    colors.background = Some(hex(roles.background));
    colors.foreground = Some(hex(roles.on_background));
    colors.border = Some(hex(roles.outline_variant));
    colors.input = Some(hex(roles.outline));
    colors.ring = Some(hex(roles.primary));
    colors.caret = Some(hex(roles.primary));

    colors.primary = Some(hex(roles.primary));
    colors.primary_foreground = Some(hex(roles.on_primary));
    colors.primary_hover = Some(hover_bg(roles.primary, roles.on_primary));
    colors.primary_active = Some(active_bg(roles.primary, roles.on_primary));
    colors.button_primary = Some(hex(roles.primary));
    colors.button_primary_foreground = Some(hex(roles.on_primary));
    colors.button_primary_hover = colors.primary_hover.clone();
    colors.button_primary_active = colors.primary_active.clone();

    colors.secondary = Some(hex(roles.secondary_container));
    colors.secondary_foreground = Some(hex(roles.on_secondary_container));
    colors.secondary_hover = Some(hover_bg(
        roles.secondary_container,
        roles.on_secondary_container,
    ));
    colors.secondary_active = Some(active_bg(
        roles.secondary_container,
        roles.on_secondary_container,
    ));
    colors.button_secondary = colors.secondary.clone();
    colors.button_secondary_foreground = colors.secondary_foreground.clone();
    colors.button_secondary_hover = colors.secondary_hover.clone();
    colors.button_secondary_active = colors.secondary_active.clone();

    colors.button = Some(hex(roles.surface_container_low));
    colors.button_foreground = Some(hex(roles.primary));
    colors.button_hover = Some(hover_bg(roles.surface_container_low, roles.primary));
    colors.button_active = Some(active_bg(roles.surface_container_low, roles.primary));

    colors.accent = Some(hex(roles.tertiary_container));
    colors.accent_foreground = Some(hex(roles.on_tertiary_container));
    colors.muted = Some(hex(roles.surface_container));
    colors.muted_foreground = Some(hex(roles.on_surface_variant));
    colors.popover = Some(hex(roles.surface_container_high));
    colors.popover_foreground = Some(hex(roles.on_surface));
    colors.group_box = Some(hex(roles.surface_container_low));
    colors.group_box_foreground = Some(hex(roles.on_surface));
    colors.accordion = Some(hex(roles.surface_container_low));

    colors.danger = Some(hex(roles.error));
    colors.danger_foreground = Some(hex(roles.on_error));
    colors.danger_hover = Some(hover_bg(roles.error, roles.on_error));
    colors.danger_active = Some(active_bg(roles.error, roles.on_error));
    colors.button_danger = Some(hex(roles.error_container));
    colors.button_danger_foreground = Some(hex(roles.on_error_container));
    colors.button_danger_hover = Some(hover_bg(roles.error_container, roles.on_error_container));
    colors.button_danger_active = Some(active_bg(roles.error_container, roles.on_error_container));

    colors.info = Some(hex(roles.primary_container));
    colors.info_foreground = Some(hex(roles.on_primary_container));
    colors.warning = Some(hex(roles.tertiary_container));
    colors.warning_foreground = Some(hex(roles.on_tertiary_container));
    colors.success = Some(hex(roles.secondary_container));
    colors.success_foreground = Some(hex(roles.on_secondary_container));

    colors.list = Some(hex(roles.surface));
    colors.list_hover = Some(hex(roles.surface_container_low));
    colors.list_active = Some(hex(roles.primary_container));
    colors.list_active_border = Some(hex(roles.primary));
    colors.list_even = Some(hex(roles.surface_container_lowest));
    colors.list_head = Some(hex(roles.surface_container));
    colors.list_highlight_foreground = Some(hex(roles.primary));

    colors.sidebar = Some(hex(roles.surface_container_low));
    colors.sidebar_foreground = Some(hex(roles.on_surface));
    colors.sidebar_border = Some(hex(roles.outline_variant));
    colors.sidebar_primary = Some(hex(roles.primary));
    colors.sidebar_primary_foreground = Some(hex(roles.on_primary));
    colors.sidebar_accent = Some(hex(roles.secondary_container));
    colors.sidebar_accent_foreground = Some(hex(roles.on_secondary_container));

    colors.tab = Some("#00000000".into());
    colors.tab_bar = Some(hex(roles.surface_container_low));
    colors.tab_bar_segmented = Some(hex(roles.surface_container));
    colors.tab_active = Some(hex(roles.secondary_container));
    colors.tab_active_foreground = Some(hex(roles.on_secondary_container));
    colors.tab_foreground = Some(hex(roles.on_surface_variant));

    colors.table = Some(hex(roles.surface));
    colors.table_even = Some(hex(roles.surface_container_lowest));
    colors.table_head = Some(hex(roles.surface_container));
    colors.table_head_foreground = Some(hex(roles.on_surface_variant));
    colors.table_foot = Some(hex(roles.surface_container));
    colors.table_foot_foreground = Some(hex(roles.on_surface_variant));
    colors.table_hover = Some(hex(roles.surface_container_low));
    colors.table_active = Some(hex(roles.primary_container));
    colors.table_active_border = Some(hex(roles.primary));
    colors.table_row_border = Some(hex(roles.outline_variant));

    colors.title_bar = Some(hex(roles.surface_container_low));
    colors.title_bar_border = Some(hex(roles.outline_variant));
    colors.status_bar = Some(hex(roles.surface_container));
    colors.status_bar_border = Some(hex(roles.outline_variant));

    colors.switch = Some(hex(roles.surface_container_highest));
    colors.switch_thumb = Some(hex(roles.outline));
    colors.slider_bar = Some(hex(roles.primary));
    colors.slider_thumb = Some(hex(roles.on_primary));
    colors.progress_bar = Some(hex(roles.primary));
    colors.link = Some(hex(roles.primary));
    colors.link_hover = Some(hex(roles.tertiary));
    colors.link_active = Some(hex(roles.primary));
    colors.selection = Some(hex(roles.primary_container));
    colors.skeleton = Some(hex(roles.surface_container_high));
    colors.scrollbar = Some("#00000000".into());
    colors.scrollbar_thumb = Some(hex(roles.outline_variant));
    colors.scrollbar_thumb_hover = Some(hex(roles.outline));
    colors.overlay = Some(if mode.is_dark() {
        "#00000066".into()
    } else {
        "#0E141B52".into()
    });
    colors.window_border = Some(hex(roles.outline_variant));
    colors.drag_border = Some(hex(roles.primary));
    colors.drop_target = Some(hex(roles.primary_container.alpha(0.35)));
    colors.description_list_label = Some(hex(roles.surface_container_low));
    colors.description_list_label_foreground = Some(hex(roles.on_surface_variant));

    colors.chart_1 = Some(hex(roles.primary));
    colors.chart_2 = Some(hex(roles.secondary));
    colors.chart_3 = Some(hex(roles.tertiary));
    colors.chart_4 = brand::design::reference_palette_hex("gemini_blue")
        .map(Into::into)
        .or_else(|| Some(hex(roles.primary)));
    colors.chart_5 = brand::design::reference_palette_hex("gemini_violet")
        .map(Into::into)
        .or_else(|| Some(hex(roles.tertiary)));
    colors.chart_grid = Some(hex(roles.outline_variant));

    let mut config = ThemeConfig::default();
    config.is_default = matches!(mode, M3AppearanceMode::Light | M3AppearanceMode::Dark);
    config.name = mode.display_name().into();
    config.mode = mode.theme_mode();
    config.font_size = Some(typography.body_medium.size_sp);
    config.font_family = Some(typography.typeface.brand_family());
    config.mono_font_family = Some(typography.typeface.mono_family());
    config.mono_font_size = Some(13.0);
    config.radius = Some(shape.medium_px.round() as usize);
    config.radius_lg = Some(shape.large_px.round() as usize);
    config.window_radius = Some(shape.medium_px.round() as usize);
    config.shadow = Some(true);
    config.colors = colors;
    Ok(config)
}

/// Builds a `gpui-component` [`ThemeConfig`] for `mode`.
#[must_use]
pub fn m3_theme_config(mode: M3AppearanceMode) -> ThemeConfig {
    try_m3_theme_config(mode).unwrap_or_else(|_| {
        let mut fallback = ThemeConfig::default();
        fallback.name = mode.display_name().into();
        fallback.mode = mode.theme_mode();
        fallback.font_family = Some("Google Sans Flex".into());
        fallback.mono_font_family = Some("JetBrains Mono".into());
        fallback.font_size = Some(14.0);
        fallback.mono_font_size = Some(13.0);
        fallback.radius = Some(12);
        fallback.radius_lg = Some(16);
        fallback.window_radius = Some(12);
        fallback
    })
}

fn resolve_available_family(
    preferred: &str,
    alternates: &[&str],
    installed: &[String],
) -> SharedString {
    if installed.is_empty() {
        return preferred.to_owned().into();
    }
    for candidate in std::iter::once(&preferred).chain(alternates.iter()) {
        if let Some(found) = installed
            .iter()
            .find(|name| name.eq_ignore_ascii_case(candidate))
        {
            return found.clone().into();
        }
    }
    ".SystemUIFont".into()
}

/// Applies the Material 3 Expressive theme for `mode` onto [`M3ActiveTheme`], [`Theme`], and `gpui_base::Theme`.
pub fn apply_m3_theme(mode: M3AppearanceMode, cx: &mut App) {
    if !cx.has_global::<Theme>() {
        crate::theme::init(cx);
    }

    let active = M3ActiveTheme::for_mode(mode);
    let mut config = m3_theme_config(mode);
    let installed = cx.text_system().all_font_names();
    if !installed.is_empty() {
        let brand = active.typography.typeface.brand_family();
        let mono = active.typography.typeface.mono_family();
        config.font_family = Some(resolve_available_family(
            brand.as_ref(),
            &[
                "Google Sans",
                "Google Sans Code",
                "Inter",
                "IBM Plex Sans",
                "Noto Sans",
            ],
            &installed,
        ));
        config.mono_font_family = Some(resolve_available_family(
            mono.as_ref(),
            &[
                "Roboto Mono",
                "Cascadia Mono",
                "DejaVu Sans Mono",
                "Menlo",
                "Consolas",
            ],
            &installed,
        ));
    }

    cx.set_global(active);
    let config_rc = Rc::new(config);
    Theme::update(cx, move |theme| {
        theme.apply_config(&config_rc);
        theme.motion = m3_motion_tokens();
    });
}

// ---------------------------------------------------------------------------
// Shared internal helpers for the 16 native M3 Expressive components
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum M3ButtonKind {
    Filled,
    Tonal,
    Outlined,
    Fab(M3FabSize, M3FabColor, bool),
    ExtendedFab(M3FabColor, bool),
}

/// Size scale for Material 3 Expressive Floating Action Buttons (`Small` 40px, `Standard` 56px, `Large` 96px).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum M3FabSize {
    Small,
    #[default]
    Standard,
    Large,
}

/// Semantic color container for [`M3Fab`] and [`M3ExtendedFab`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum M3FabColor {
    #[default]
    Primary,
    Secondary,
    Tertiary,
    Surface,
}

fn m3_elevation_shadow(colors: &M3ColorRoles, level: u8) -> Vec<BoxShadow> {
    let ink = if m3_relative_luminance(colors.surface) < 0.2 {
        hsla(0.0, 0.0, 0.0, 0.42)
    } else {
        hsla(0.0, 0.0, 0.0, 0.16)
    };
    match level {
        0 => Vec::new(),
        1 => vec![box_shadow(px(0.), px(1.), px(3.), px(0.), ink)],
        2 => vec![
            box_shadow(px(0.), px(2.), px(6.), px(0.), ink),
            box_shadow(px(0.), px(1.), px(2.), px(0.), ink.opacity(0.65)),
        ],
        _ => vec![
            box_shadow(px(0.), px(6.), px(14.), px(-2.), ink),
            box_shadow(px(0.), px(2.), px(5.), px(0.), ink.opacity(0.65)),
        ],
    }
}

fn apply_m3_focus_ring<T: Styled + ParentElement>(
    element: T,
    focus_color: Hsla,
    ring_width_px: f32,
    window: &Window,
    cx: &App,
) -> T {
    if cx.has_global::<Theme>() {
        focus_style(element, FocusLine::Edge, window, cx)
    } else {
        element.child(
            div()
                .flex_none()
                .absolute()
                .inset(px(-ring_width_px))
                .rounded(px(9999.))
                .border(px(ring_width_px))
                .border_color(focus_color),
        )
    }
}

struct M3ButtonSurfaceSpec {
    id: ElementId,
    kind: M3ButtonKind,
    label: Option<SharedString>,
    leading_icon: Option<Icon>,
    trailing_icon: Option<Icon>,
    accessibility_label: Option<SharedString>,
    size: Size,
    compact: bool,
    disabled: bool,
    loading: bool,
    style: StyleRefinement,
    on_click: Option<ClickHandler>,
    mode_override: Option<M3AppearanceMode>,
}

fn render_m3_button_surface(
    spec: M3ButtonSurfaceSpec,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let m3 = spec
        .mode_override
        .map(M3ActiveTheme::for_mode)
        .unwrap_or_else(|| M3ActiveTheme::resolve(cx));
    let colors = m3.colors;
    let shape = m3.shape;
    let state = m3.state;

    let (base_bg, fg, border_color, resting_radius, pressed_radius, elevation_level) =
        match spec.kind {
            M3ButtonKind::Filled => (
                colors.primary,
                colors.on_primary,
                None,
                px(shape.full_px),
                px(shape.medium_px),
                0u8,
            ),
            M3ButtonKind::Tonal => (
                colors.secondary_container,
                colors.on_secondary_container,
                None,
                px(shape.full_px),
                px(shape.medium_px),
                0u8,
            ),
            M3ButtonKind::Outlined => (
                transparent_black(),
                colors.primary,
                Some(colors.outline),
                px(shape.full_px),
                px(shape.medium_px),
                0u8,
            ),
            M3ButtonKind::Fab(fab_size, fab_color, lowered) => {
                let (bg, fg) = match fab_color {
                    M3FabColor::Primary => (colors.primary_container, colors.on_primary_container),
                    M3FabColor::Secondary => {
                        (colors.secondary_container, colors.on_secondary_container)
                    }
                    M3FabColor::Tertiary => {
                        (colors.tertiary_container, colors.on_tertiary_container)
                    }
                    M3FabColor::Surface => (colors.surface_container_high, colors.primary),
                };
                let radius = match fab_size {
                    M3FabSize::Small => px(shape.medium_px),
                    M3FabSize::Standard => px(shape.large_px),
                    M3FabSize::Large => px(shape.extra_large_px),
                };
                (
                    bg,
                    fg,
                    None,
                    radius,
                    px(shape.small_px),
                    if lowered { 1 } else { 3 },
                )
            }
            M3ButtonKind::ExtendedFab(fab_color, lowered) => {
                let (bg, fg) = match fab_color {
                    M3FabColor::Primary => (colors.primary_container, colors.on_primary_container),
                    M3FabColor::Secondary => {
                        (colors.secondary_container, colors.on_secondary_container)
                    }
                    M3FabColor::Tertiary => {
                        (colors.tertiary_container, colors.on_tertiary_container)
                    }
                    M3FabColor::Surface => (colors.surface_container_high, colors.primary),
                };
                (
                    bg,
                    fg,
                    None,
                    px(shape.large_px),
                    px(shape.medium_px),
                    if lowered { 1 } else { 3 },
                )
            }
        };

    let hover_bg = if base_bg.a == 0.0 {
        fg.alpha(state.hover_opacity)
    } else {
        colors.state_layer(base_bg, fg, state.hover_opacity)
    };
    let active_bg = if base_bg.a == 0.0 {
        fg.alpha(state.pressed_opacity)
    } else {
        colors.state_layer(base_bg, fg, state.pressed_opacity)
    };
    let disabled_bg = if matches!(spec.kind, M3ButtonKind::Outlined) {
        transparent_black()
    } else {
        colors
            .on_surface
            .alpha(state.disabled_container_opacity)
    };
    let disabled_fg = colors.on_surface.alpha(state.disabled_content_opacity);

    let interactive = !(spec.disabled || spec.loading);
    let focus_handle = window
        .use_keyed_state(spec.id.clone(), cx, |_, cx| cx.focus_handle())
        .read(cx)
        .clone();
    let is_focused = focus_handle.is_focused(window);

    let a11y_label = spec
        .accessibility_label
        .clone()
        .or_else(|| spec.label.clone());
    let icon_size = match spec.kind {
        M3ButtonKind::Fab(M3FabSize::Large, _, _) => Size::Size(px(32.)),
        M3ButtonKind::Fab(_, _, _) | M3ButtonKind::ExtendedFab(_, _) => Size::Size(px(22.)),
        _ => match spec.size {
            Size::XSmall => Size::XSmall,
            Size::Small => Size::Small,
            Size::Large => Size::Medium,
            Size::Medium | Size::Size(_) => Size::Small,
        },
    };
    let label_weight = m3.typography.label_large.font_weight();

    let leading = if spec.loading {
        Some(
            Spinner::new()
                .with_size(icon_size)
                .color(fg)
                .into_any_element(),
        )
    } else {
        spec.leading_icon
            .map(|icon| icon.with_size(icon_size).text_color(fg).into_any_element())
    };
    let trailing = spec
        .trailing_icon
        .map(|icon| icon.with_size(icon_size).text_color(fg).into_any_element());

    let debug_id = spec.id.clone();
    let mut button = BaseButton::new(spec.id)
        .disabled(spec.disabled)
        .track_focus(&focus_handle)
        .when_some(a11y_label, |this, label| this.accessibility_label(label))
        .when_some(
            interactive.then_some(spec.on_click).flatten(),
            |this, on_click| {
                this.on_click(move |event, window, cx| on_click(event, window, cx))
            },
        )
        .debug_selector(move || debug_id.to_string())
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .rounded(resting_radius)
        .font_weight(label_weight)
        .text_color(if spec.disabled { disabled_fg } else { fg })
        .bg(if spec.disabled { disabled_bg } else { base_bg })
        .when(!spec.disabled && elevation_level > 0, |this| {
            this.shadow(m3_elevation_shadow(&colors, elevation_level))
        })
        .when_some(border_color, |this, border| {
            this.border_1().border_color(if spec.disabled {
                colors.on_surface.alpha(state.disabled_container_opacity)
            } else {
                border
            })
        })
        .map(|this| match spec.kind {
            M3ButtonKind::Fab(M3FabSize::Small, _, _) => this.size(px(40.)),
            M3ButtonKind::Fab(M3FabSize::Standard, _, _) => this.size(px(56.)),
            M3ButtonKind::Fab(M3FabSize::Large, _, _) => this.size(px(96.)),
            M3ButtonKind::ExtendedFab(_, _) => this.h(px(56.)).px(px(20.)),
            M3ButtonKind::Filled | M3ButtonKind::Tonal | M3ButtonKind::Outlined => {
                match spec.size {
                    Size::XSmall => this.h(px(32.)).px(px(if spec.compact { 10. } else { 14. })),
                    Size::Small => this.h(px(36.)).px(px(if spec.compact { 12. } else { 16. })),
                    Size::Large => this.h(px(48.)).px(px(if spec.compact { 18. } else { 24. })),
                    Size::Size(height) => this.h(height).px(px(20.)),
                    Size::Medium => this.h(px(40.)).px(px(if spec.compact { 16. } else { 24. })),
                }
            }
        })
        .when(interactive, |this| {
            this.cursor_pointer()
                .hover(move |style| style.bg(hover_bg))
                .active(move |style| style.bg(active_bg).rounded(pressed_radius))
        })
        .when(spec.disabled, |this| this.cursor_not_allowed())
        .when(spec.loading && !spec.disabled, |this| this.opacity(0.85))
        .when_some(leading, |this, icon| this.child(icon))
        .when_some(spec.label, |this, label| {
            this.child(
                div()
                    .text_size(px(m3.typography.label_large.size_sp))
                    .whitespace_nowrap()
                    .child(label),
            )
        })
        .when_some(trailing, |this, icon| this.child(icon))
        .refine_style(&spec.style);

    if is_focused && !spec.disabled {
        button = apply_m3_focus_ring(button, colors.primary, state.focus_ring_px, window, cx);
    }

    button.into_any_element()
}

macro_rules! impl_m3_button_builders {
    ($name:ident) => {
        impl $name {
            /// Sets the leading icon.
            #[must_use]
            pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
                self.leading_icon = Some(icon.into());
                self
            }

            /// Sets the trailing icon.
            #[must_use]
            pub fn trailing_icon(mut self, icon: impl Into<Icon>) -> Self {
                self.trailing_icon = Some(icon.into());
                self
            }

            /// Updates the button text label.
            #[must_use]
            pub fn label(mut self, label: impl Into<SharedString>) -> Self {
                self.label = label.into();
                self
            }

            /// Uses compact horizontal padding.
            #[must_use]
            pub fn compact(mut self) -> Self {
                self.compact = true;
                self
            }

            /// Shows an indeterminate spinner and blocks repeat activation.
            #[must_use]
            pub fn loading(mut self, loading: bool) -> Self {
                self.loading = loading;
                self
            }

            /// Sets the screen-reader accessible label.
            #[must_use]
            pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
                self.accessibility_label = Some(label.into());
                self
            }

            /// Overrides the M3 appearance mode for this control subtree.
            #[must_use]
            pub fn theme_override(mut self, mode: M3AppearanceMode) -> Self {
                self.mode_override = Some(mode);
                self
            }

            /// Registers the click/Enter/Space activation handler.
            #[must_use]
            pub fn on_click(
                mut self,
                handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
            ) -> Self {
                self.on_click = Some(Rc::new(handler));
                self
            }
        }

        impl Styled for $name {
            fn style(&mut self) -> &mut StyleRefinement {
                &mut self.style
            }
        }

        impl Disableable for $name {
            fn disabled(mut self, disabled: bool) -> Self {
                self.disabled = disabled;
                self
            }
        }

        impl Sizable for $name {
            fn with_size(mut self, size: impl Into<Size>) -> Self {
                self.size = size.into();
                self
            }
        }
    };
}

// ---------------------------------------------------------------------------
// 1. M3FilledButton
// ---------------------------------------------------------------------------

/// Material 3 Expressive Filled Button (`primary` fill, `on_primary` label, pill -> 12px press morph).
#[derive(IntoElement)]
pub struct M3FilledButton {
    id: ElementId,
    label: SharedString,
    leading_icon: Option<Icon>,
    trailing_icon: Option<Icon>,
    accessibility_label: Option<SharedString>,
    size: Size,
    compact: bool,
    disabled: bool,
    loading: bool,
    style: StyleRefinement,
    on_click: Option<ClickHandler>,
    mode_override: Option<M3AppearanceMode>,
}

impl M3FilledButton {
    /// Creates a filled M3 button with stable domain `id` and `label`.
    #[must_use]
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            leading_icon: None,
            trailing_icon: None,
            accessibility_label: None,
            size: Size::Medium,
            compact: false,
            disabled: false,
            loading: false,
            style: StyleRefinement::default(),
            on_click: None,
            mode_override: None,
        }
    }
}

impl_m3_button_builders!(M3FilledButton);

impl RenderOnce for M3FilledButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        render_m3_button_surface(
            M3ButtonSurfaceSpec {
                id: self.id,
                kind: M3ButtonKind::Filled,
                label: Some(self.label),
                leading_icon: self.leading_icon,
                trailing_icon: self.trailing_icon,
                accessibility_label: self.accessibility_label,
                size: self.size,
                compact: self.compact,
                disabled: self.disabled,
                loading: self.loading,
                style: self.style,
                on_click: self.on_click,
                mode_override: self.mode_override,
            },
            window,
            cx,
        )
    }
}

// ---------------------------------------------------------------------------
// 2. M3TonalButton
// ---------------------------------------------------------------------------

/// Material 3 Expressive Filled Tonal Button (`secondary_container` fill, `on_secondary_container` ink).
#[derive(IntoElement)]
pub struct M3TonalButton {
    id: ElementId,
    label: SharedString,
    leading_icon: Option<Icon>,
    trailing_icon: Option<Icon>,
    accessibility_label: Option<SharedString>,
    size: Size,
    compact: bool,
    disabled: bool,
    loading: bool,
    style: StyleRefinement,
    on_click: Option<ClickHandler>,
    mode_override: Option<M3AppearanceMode>,
}

impl M3TonalButton {
    /// Creates a tonal M3 button with stable domain `id` and `label`.
    #[must_use]
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            leading_icon: None,
            trailing_icon: None,
            accessibility_label: None,
            size: Size::Medium,
            compact: false,
            disabled: false,
            loading: false,
            style: StyleRefinement::default(),
            on_click: None,
            mode_override: None,
        }
    }
}

impl_m3_button_builders!(M3TonalButton);

impl RenderOnce for M3TonalButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        render_m3_button_surface(
            M3ButtonSurfaceSpec {
                id: self.id,
                kind: M3ButtonKind::Tonal,
                label: Some(self.label),
                leading_icon: self.leading_icon,
                trailing_icon: self.trailing_icon,
                accessibility_label: self.accessibility_label,
                size: self.size,
                compact: self.compact,
                disabled: self.disabled,
                loading: self.loading,
                style: self.style,
                on_click: self.on_click,
                mode_override: self.mode_override,
            },
            window,
            cx,
        )
    }
}

// ---------------------------------------------------------------------------
// 3. M3OutlinedButton
// ---------------------------------------------------------------------------

/// Material 3 Expressive Outlined Button (`outline` 1px border, `primary` foreground).
#[derive(IntoElement)]
pub struct M3OutlinedButton {
    id: ElementId,
    label: SharedString,
    leading_icon: Option<Icon>,
    trailing_icon: Option<Icon>,
    accessibility_label: Option<SharedString>,
    size: Size,
    compact: bool,
    disabled: bool,
    loading: bool,
    style: StyleRefinement,
    on_click: Option<ClickHandler>,
    mode_override: Option<M3AppearanceMode>,
}

impl M3OutlinedButton {
    /// Creates an outlined M3 button with stable domain `id` and `label`.
    #[must_use]
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            leading_icon: None,
            trailing_icon: None,
            accessibility_label: None,
            size: Size::Medium,
            compact: false,
            disabled: false,
            loading: false,
            style: StyleRefinement::default(),
            on_click: None,
            mode_override: None,
        }
    }
}

impl_m3_button_builders!(M3OutlinedButton);

impl RenderOnce for M3OutlinedButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        render_m3_button_surface(
            M3ButtonSurfaceSpec {
                id: self.id,
                kind: M3ButtonKind::Outlined,
                label: Some(self.label),
                leading_icon: self.leading_icon,
                trailing_icon: self.trailing_icon,
                accessibility_label: self.accessibility_label,
                size: self.size,
                compact: self.compact,
                disabled: self.disabled,
                loading: self.loading,
                style: self.style,
                on_click: self.on_click,
                mode_override: self.mode_override,
            },
            window,
            cx,
        )
    }
}

// ---------------------------------------------------------------------------
// 4. M3Fab
// ---------------------------------------------------------------------------

/// Material 3 Expressive Floating Action Button (`Small` 40px, `Standard` 56px, `Large` 96px).
#[derive(IntoElement)]
pub struct M3Fab {
    id: ElementId,
    icon: Icon,
    fab_size: M3FabSize,
    color: M3FabColor,
    lowered: bool,
    disabled: bool,
    accessibility_label: Option<SharedString>,
    style: StyleRefinement,
    on_click: Option<ClickHandler>,
    mode_override: Option<M3AppearanceMode>,
}

impl M3Fab {
    /// Creates an icon Floating Action Button.
    #[must_use]
    pub fn new(id: impl Into<ElementId>, icon: impl Into<Icon>) -> Self {
        Self {
            id: id.into(),
            icon: icon.into(),
            fab_size: M3FabSize::Standard,
            color: M3FabColor::Primary,
            lowered: false,
            disabled: false,
            accessibility_label: None,
            style: StyleRefinement::default(),
            on_click: None,
            mode_override: None,
        }
    }

    /// Selects `Small` (40px), `Standard` (56px), or `Large` (96px).
    #[must_use]
    pub fn fab_size(mut self, fab_size: M3FabSize) -> Self {
        self.fab_size = fab_size;
        self
    }

    /// Selects container color mapping (`Primary`, `Secondary`, `Tertiary`, `Surface`).
    #[must_use]
    pub fn color(mut self, color: M3FabColor) -> Self {
        self.color = color;
        self
    }

    /// Uses lowered resting elevation when `true`.
    #[must_use]
    pub fn lowered(mut self, lowered: bool) -> Self {
        self.lowered = lowered;
        self
    }

    /// Sets the accessible label announced by assistive technology.
    #[must_use]
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    /// Overrides the M3 appearance mode for this control subtree.
    #[must_use]
    pub fn theme_override(mut self, mode: M3AppearanceMode) -> Self {
        self.mode_override = Some(mode);
        self
    }

    /// Registers the activation callback.
    #[must_use]
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl Styled for M3Fab {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Disableable for M3Fab {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for M3Fab {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        render_m3_button_surface(
            M3ButtonSurfaceSpec {
                id: self.id,
                kind: M3ButtonKind::Fab(self.fab_size, self.color, self.lowered),
                label: None,
                leading_icon: Some(self.icon),
                trailing_icon: None,
                accessibility_label: self.accessibility_label,
                size: Size::Medium,
                compact: false,
                disabled: self.disabled,
                loading: false,
                style: self.style,
                on_click: self.on_click,
                mode_override: self.mode_override,
            },
            window,
            cx,
        )
    }
}

// ---------------------------------------------------------------------------
// 5. M3ExtendedFab
// ---------------------------------------------------------------------------

/// Material 3 Expressive Extended Floating Action Button (icon + label pill at 56px height).
#[derive(IntoElement)]
pub struct M3ExtendedFab {
    id: ElementId,
    label: SharedString,
    icon: Option<Icon>,
    color: M3FabColor,
    lowered: bool,
    disabled: bool,
    accessibility_label: Option<SharedString>,
    style: StyleRefinement,
    on_click: Option<ClickHandler>,
    mode_override: Option<M3AppearanceMode>,
}

impl M3ExtendedFab {
    /// Creates an extended FAB with stable domain `id` and `label`.
    #[must_use]
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            color: M3FabColor::Primary,
            lowered: false,
            disabled: false,
            accessibility_label: None,
            style: StyleRefinement::default(),
            on_click: None,
            mode_override: None,
        }
    }

    /// Sets the leading icon.
    #[must_use]
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Selects container color mapping (`Primary`, `Secondary`, `Tertiary`, `Surface`).
    #[must_use]
    pub fn color(mut self, color: M3FabColor) -> Self {
        self.color = color;
        self
    }

    /// Uses lowered resting elevation when `true`.
    #[must_use]
    pub fn lowered(mut self, lowered: bool) -> Self {
        self.lowered = lowered;
        self
    }

    /// Sets the accessible label announced by assistive technology.
    #[must_use]
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    /// Overrides the M3 appearance mode for this control subtree.
    #[must_use]
    pub fn theme_override(mut self, mode: M3AppearanceMode) -> Self {
        self.mode_override = Some(mode);
        self
    }

    /// Registers the activation callback.
    #[must_use]
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl Styled for M3ExtendedFab {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Disableable for M3ExtendedFab {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for M3ExtendedFab {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        render_m3_button_surface(
            M3ButtonSurfaceSpec {
                id: self.id,
                kind: M3ButtonKind::ExtendedFab(self.color, self.lowered),
                label: Some(self.label),
                leading_icon: self.icon,
                trailing_icon: None,
                accessibility_label: self.accessibility_label,
                size: Size::Large,
                compact: false,
                disabled: self.disabled,
                loading: false,
                style: self.style,
                on_click: self.on_click,
                mode_override: self.mode_override,
            },
            window,
            cx,
        )
    }
}

// ---------------------------------------------------------------------------
// 6. M3SegmentedButtonBar
// ---------------------------------------------------------------------------

/// One option inside an [`M3SegmentedButtonBar`].
#[derive(Clone)]
pub struct M3SegmentItem {
    pub id: SharedString,
    pub label: SharedString,
    pub icon: Option<Icon>,
    pub disabled: bool,
}

impl M3SegmentItem {
    /// Creates a segment identified by stable domain `id`.
    #[must_use]
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            disabled: false,
        }
    }

    /// Sets an optional icon shown when the segment is unselected (or alongside the label).
    #[must_use]
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Marks this individual segment disabled.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Material 3 Expressive Segmented Button Bar supporting single or multi selection.
#[derive(IntoElement)]
pub struct M3SegmentedButtonBar {
    id: ElementId,
    segments: Vec<M3SegmentItem>,
    selected_ids: BTreeSet<SharedString>,
    multi_select: bool,
    disabled: bool,
    style: StyleRefinement,
    on_select: Option<IdSelectHandler>,
    mode_override: Option<M3AppearanceMode>,
}

impl M3SegmentedButtonBar {
    /// Creates an empty segmented button bar.
    #[must_use]
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            segments: Vec::new(),
            selected_ids: BTreeSet::new(),
            multi_select: false,
            disabled: false,
            style: StyleRefinement::default(),
            on_select: None,
            mode_override: None,
        }
    }

    /// Appends a single [`M3SegmentItem`].
    #[must_use]
    pub fn segment(mut self, segment: M3SegmentItem) -> Self {
        self.segments.push(segment);
        self
    }

    /// Appends multiple [`M3SegmentItem`] options.
    #[must_use]
    pub fn segments(mut self, segments: impl IntoIterator<Item = M3SegmentItem>) -> Self {
        self.segments.extend(segments);
        self
    }

    /// Selects a single segment ID (replacing previous selection).
    #[must_use]
    pub fn selected_id(mut self, id: impl Into<SharedString>) -> Self {
        self.selected_ids.clear();
        self.selected_ids.insert(id.into());
        self
    }

    /// Selects multiple segment IDs.
    #[must_use]
    pub fn selected_ids(mut self, ids: impl IntoIterator<Item = SharedString>) -> Self {
        self.selected_ids = ids.into_iter().collect();
        self
    }

    /// Enables multi-selection mode.
    #[must_use]
    pub fn multi_select(mut self, multi_select: bool) -> Self {
        self.multi_select = multi_select;
        self
    }

    /// Overrides the M3 appearance mode for this control subtree.
    #[must_use]
    pub fn theme_override(mut self, mode: M3AppearanceMode) -> Self {
        self.mode_override = Some(mode);
        self
    }

    /// Registers a callback fired with the domain ID of the toggled/selected segment.
    #[must_use]
    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl Styled for M3SegmentedButtonBar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Disableable for M3SegmentedButtonBar {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for M3SegmentedButtonBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let m3 = self
            .mode_override
            .map(M3ActiveTheme::for_mode)
            .unwrap_or_else(|| M3ActiveTheme::resolve(cx));
        let colors = m3.colors;
        let state = m3.state;
        let shape = m3.shape;
        let count = self.segments.len();

        let debug_id = self.id.clone();
        let mut group = BaseToggleGroup::new(self.id.clone())
            .debug_selector(move || debug_id.to_string())
            .flex()
            .flex_row_dir(cx)
            .items_center()
            .overflow_hidden()
            .h(px(40.))
            .rounded(px(shape.full_px))
            .border_1()
            .border_color(colors.outline)
            .bg(colors.surface);

        for (index, segment) in self.segments.into_iter().enumerate() {
            let selected = self.selected_ids.contains(&segment.id);
            let item_disabled = self.disabled || segment.disabled;
            let domain_id = segment.id.clone();
            let on_select = self.on_select.clone();

            let bg = if item_disabled {
                if selected {
                    colors.on_surface.alpha(state.disabled_container_opacity)
                } else {
                    transparent_black()
                }
            } else if selected {
                colors.secondary_container
            } else {
                transparent_black()
            };
            let fg = if item_disabled {
                colors.on_surface.alpha(state.disabled_content_opacity)
            } else if selected {
                colors.on_secondary_container
            } else {
                colors.on_surface
            };
            let hover_bg = colors.state_layer(bg, fg, state.hover_opacity);
            let active_bg = colors.state_layer(bg, fg, state.pressed_opacity);

            let leading_icon = if selected {
                Some(Icon::new(IconName::Check).small().text_color(fg))
            } else {
                segment.icon.map(|icon| icon.small().text_color(fg))
            };

            let item_id: ElementId = (self.id.clone(), segment.id).into();
            let item_debug_id = item_id.clone();
            let item = BaseToggle::new(item_id)
                .debug_selector(move || item_debug_id.to_string())
                .pressed(selected)
                .disabled(item_disabled)
                .accessibility_label(segment.label.clone())
                .when_some(
                    (!item_disabled).then_some(on_select).flatten(),
                    |this, on_select| {
                        this.on_change(move |_, _, window, cx| on_select(&domain_id, window, cx))
                    },
                )
                .flex()
                .flex_1()
                .h_full()
                .min_w(px(48.))
                .px_3()
                .gap_1p5()
                .items_center()
                .justify_center()
                .bg(bg)
                .text_color(fg)
                .font_weight(m3.typography.label_large.font_weight())
                .text_size(px(m3.typography.label_large.size_sp))
                .when(index + 1 < count, |this| {
                    this.border_r_1().border_color(colors.outline)
                })
                .when(!item_disabled, |this| {
                    this.cursor_pointer()
                        .hover(move |style| style.bg(hover_bg))
                        .active(move |style| style.bg(active_bg))
                })
                .when(item_disabled, |this| this.cursor_not_allowed())
                .when_some(leading_icon, |this, icon| this.child(icon))
                .child(div().whitespace_nowrap().child(segment.label));

            group = group.child(item);
        }

        group.refine_style(&self.style)
    }
}

// ---------------------------------------------------------------------------
// 7. M3FilterChip
// ---------------------------------------------------------------------------

/// Material 3 Expressive Filter Chip (`secondary_container` selected fill + checkmark + optional dismiss).
#[derive(IntoElement)]
pub struct M3FilterChip {
    id: ElementId,
    label: SharedString,
    selected: bool,
    disabled: bool,
    elevated: bool,
    show_checkmark: bool,
    icon: Option<Icon>,
    accessibility_label: Option<SharedString>,
    style: StyleRefinement,
    on_change: Option<BoolChangeHandler>,
    on_remove: Option<ActionHandler>,
    mode_override: Option<M3AppearanceMode>,
}

impl M3FilterChip {
    /// Creates an M3 Filter Chip with stable domain `id` and `label`.
    #[must_use]
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            selected: false,
            disabled: false,
            elevated: false,
            show_checkmark: true,
            icon: None,
            accessibility_label: None,
            style: StyleRefinement::default(),
            on_change: None,
            on_remove: None,
            mode_override: None,
        }
    }

    /// Sets elevated surface treatment instead of outlined surface.
    #[must_use]
    pub fn elevated(mut self, elevated: bool) -> Self {
        self.elevated = elevated;
        self
    }

    /// Controls whether the leading checkmark replaces the icon when selected.
    #[must_use]
    pub fn show_checkmark(mut self, show: bool) -> Self {
        self.show_checkmark = show;
        self
    }

    /// Sets an optional leading icon.
    #[must_use]
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Sets the accessible label announced by assistive technology.
    #[must_use]
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    /// Overrides the M3 appearance mode for this control subtree.
    #[must_use]
    pub fn theme_override(mut self, mode: M3AppearanceMode) -> Self {
        self.mode_override = Some(mode);
        self
    }

    /// Registers the selection toggle handler receiving the requested next boolean value.
    #[must_use]
    pub fn on_change(
        mut self,
        handler: impl Fn(&bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    /// Registers an optional trailing close/dismiss icon button handler.
    #[must_use]
    pub fn on_remove(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_remove = Some(Rc::new(handler));
        self
    }
}

impl Styled for M3FilterChip {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Selectable for M3FilterChip {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl Disableable for M3FilterChip {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for M3FilterChip {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let m3 = self
            .mode_override
            .map(M3ActiveTheme::for_mode)
            .unwrap_or_else(|| M3ActiveTheme::resolve(cx));
        let colors = m3.colors;
        let shape = m3.shape;
        let state = m3.state;

        let bg = if self.disabled {
            if self.selected || self.elevated {
                colors.on_surface.alpha(state.disabled_container_opacity)
            } else {
                transparent_black()
            }
        } else if self.selected {
            colors.secondary_container
        } else if self.elevated {
            colors.surface_container_low
        } else {
            transparent_black()
        };

        let fg = if self.disabled {
            colors.on_surface.alpha(state.disabled_content_opacity)
        } else if self.selected {
            colors.on_secondary_container
        } else {
            colors.on_surface_variant
        };
        let hover_bg = colors.state_layer(bg, fg, state.hover_opacity);
        let active_bg = colors.state_layer(bg, fg, state.pressed_opacity);
        let a11y_label = self
            .accessibility_label
            .unwrap_or_else(|| self.label.clone());

        let leading_icon = if self.selected && self.show_checkmark {
            Some(Icon::new(IconName::Check).small().text_color(fg))
        } else {
            self.icon.map(|icon| icon.small().text_color(fg))
        };

        let chip_id = self.id.clone();
        let debug_id = chip_id.clone();
        let on_change = self.on_change;
        let disabled = self.disabled;

        BaseToggle::new(self.id)
            .debug_selector(move || debug_id.to_string())
            .pressed(self.selected)
            .disabled(disabled)
            .accessibility_label(a11y_label)
            .when_some(
                (!disabled).then_some(on_change).flatten(),
                |this, on_change| {
                    this.on_change(move |next, _, window, cx| on_change(&next, window, cx))
                },
            )
            .flex()
            .items_center()
            .gap_1p5()
            .h(px(32.))
            .min_w(px(44.))
            .px(px(12.))
            .rounded(px(if self.selected {
                shape.medium_px
            } else {
                shape.small_px
            }))
            .bg(bg)
            .text_color(fg)
            .font_weight(m3.typography.label_large.font_weight())
            .text_size(px(m3.typography.label_large.size_sp))
            .when(!self.selected && !self.elevated, |this| {
                this.border_1().border_color(if disabled {
                    colors.on_surface.alpha(state.disabled_container_opacity)
                } else {
                    colors.outline_variant
                })
            })
            .when(self.elevated && !disabled, |this| {
                this.shadow(m3_elevation_shadow(&colors, 1))
            })
            .when(!disabled, |this| {
                this.cursor_pointer()
                    .hover(move |style| style.bg(hover_bg))
                    .active(move |style| style.bg(active_bg))
            })
            .when(disabled, |this| this.cursor_not_allowed())
            .when_some(leading_icon, |this, icon| this.child(icon))
            .child(div().whitespace_nowrap().child(self.label))
            .when_some(
                (!disabled).then_some(self.on_remove).flatten(),
                |this, on_remove| {
                    this.child(
                        BaseButton::new((chip_id, "remove"))
                            .accessibility_label("Remove filter")
                            .size(px(20.))
                            .rounded(px(shape.full_px))
                            .flex()
                            .items_center()
                            .justify_center()
                            .on_click(move |_, window, cx| {
                                cx.stop_propagation();
                                on_remove(window, cx);
                            })
                            .child(Icon::new(IconName::Close).xsmall().text_color(fg)),
                    )
                },
            )
            .refine_style(&self.style)
    }
}

// ---------------------------------------------------------------------------
// 8, 9, 10. M3ElevatedCard, M3FilledCard, M3OutlinedCard
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum M3CardKind {
    Elevated,
    Filled,
    Outlined,
}

struct M3CardSpec {
    id: Option<ElementId>,
    kind: M3CardKind,
    title: Option<SharedString>,
    subtitle: Option<SharedString>,
    header_action: Option<AnyElement>,
    footer: Option<AnyElement>,
    children: SmallVec<[AnyElement; 2]>,
    disabled: bool,
    style: StyleRefinement,
    on_click: Option<ClickHandler>,
    mode_override: Option<M3AppearanceMode>,
}

fn render_m3_card(spec: M3CardSpec, cx: &mut App) -> AnyElement {
    let m3 = spec
        .mode_override
        .map(M3ActiveTheme::for_mode)
        .unwrap_or_else(|| M3ActiveTheme::resolve(cx));
    let colors = m3.colors;
    let shape = m3.shape;
    let state = m3.state;

    let (bg, border, shadow_level) = match spec.kind {
        M3CardKind::Elevated => (colors.surface_container_low, None, 1u8),
        M3CardKind::Filled => (colors.surface_container_highest, None, 0u8),
        M3CardKind::Outlined => (colors.surface, Some(colors.outline_variant), 0u8),
    };
    let fg = colors.on_surface;
    let subtitle_fg = colors.on_surface_variant;
    let hover_bg = colors.state_layer(bg, fg, state.hover_opacity);
    let active_bg = colors.state_layer(bg, fg, state.pressed_opacity);
    let has_header =
        spec.title.is_some() || spec.subtitle.is_some() || spec.header_action.is_some();

    let inner = v_flex()
        .gap_3()
        .p(px(16.))
        .rounded(px(shape.large_px))
        .bg(if spec.disabled {
            colors.on_surface.alpha(state.disabled_container_opacity)
        } else {
            bg
        })
        .text_color(if spec.disabled {
            colors.on_surface.alpha(state.disabled_content_opacity)
        } else {
            fg
        })
        .when_some(border, |this, border| {
            this.border_1().border_color(border)
        })
        .when(shadow_level > 0 && !spec.disabled, |this| {
            this.shadow(m3_elevation_shadow(&colors, shadow_level))
        })
        .when(has_header, |this| {
            this.child(
                h_flex()
                    .justify_between()
                    .items_start()
                    .gap_2()
                    .child(
                        v_flex()
                            .gap_0p5()
                            .min_w_0()
                            .when_some(spec.title, |col, title| {
                                col.child(
                                    div()
                                        .text_size(px(m3.typography.title_medium.size_sp))
                                        .font_weight(m3.typography.title_medium.font_weight())
                                        .child(title),
                                )
                            })
                            .when_some(spec.subtitle, |col, subtitle| {
                                col.child(
                                    div()
                                        .text_size(px(m3.typography.body_medium.size_sp))
                                        .text_color(subtitle_fg)
                                        .child(subtitle),
                                )
                            }),
                    )
                    .when_some(spec.header_action, |row, action| row.child(action)),
            )
        })
        .children(spec.children)
        .when_some(spec.footer, |this, footer| {
            this.child(
                div()
                    .pt_2()
                    .border_t_1()
                    .border_color(colors.outline_variant)
                    .child(footer),
            )
        });

    if let Some(on_click) = (!spec.disabled).then_some(spec.on_click).flatten() {
        let card_id = spec.id.unwrap_or_else(|| match spec.kind {
            M3CardKind::Elevated => "m3-elevated-card".into(),
            M3CardKind::Filled => "m3-filled-card".into(),
            M3CardKind::Outlined => "m3-outlined-card".into(),
        });
        let debug_id = card_id.clone();
        inner
            .id(card_id)
            .debug_selector(move || debug_id.to_string())
            .test_support()
            .cursor_pointer()
            .hover(move |style| style.bg(hover_bg))
            .active(move |style| style.bg(active_bg))
            .on_click(move |event, window, cx| on_click(event, window, cx))
            .refine_style(&spec.style)
            .into_any_element()
    } else if let Some(id) = spec.id {
        let debug_id = id.clone();
        inner
            .refine_style(&spec.style)
            .id(id)
            .debug_selector(move || debug_id.to_string())
            .test_support()
            .into_any_element()
    } else {
        inner.refine_style(&spec.style).into_any_element()
    }
}

macro_rules! define_m3_card {
    ($name:ident, $kind:expr, $doc:literal) => {
        #[doc = $doc]
        #[derive(IntoElement)]
        pub struct $name {
            id: Option<ElementId>,
            title: Option<SharedString>,
            subtitle: Option<SharedString>,
            header_action: Option<AnyElement>,
            footer: Option<AnyElement>,
            children: SmallVec<[AnyElement; 2]>,
            disabled: bool,
            style: StyleRefinement,
            on_click: Option<ClickHandler>,
            mode_override: Option<M3AppearanceMode>,
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl $name {
            /// Creates a new card container.
            #[must_use]
            pub fn new() -> Self {
                Self {
                    id: None,
                    title: None,
                    subtitle: None,
                    header_action: None,
                    footer: None,
                    children: SmallVec::new(),
                    disabled: false,
                    style: StyleRefinement::default(),
                    on_click: None,
                    mode_override: None,
                }
            }

            /// Assigns an explicit domain [`ElementId`].
            #[must_use]
            pub fn id(mut self, id: impl Into<ElementId>) -> Self {
                self.id = Some(id.into());
                self
            }

            /// Sets the card header title (`title_medium`).
            #[must_use]
            pub fn title(mut self, title: impl Into<SharedString>) -> Self {
                self.title = Some(title.into());
                self
            }

            /// Sets the card header subtitle (`body_medium` in `on_surface_variant`).
            #[must_use]
            pub fn subtitle(mut self, subtitle: impl Into<SharedString>) -> Self {
                self.subtitle = Some(subtitle.into());
                self
            }

            /// Places an optional trailing element in the card header.
            #[must_use]
            pub fn header_action(mut self, action: impl IntoElement) -> Self {
                self.header_action = Some(action.into_any_element());
                self
            }

            /// Sets an optional footer separated by an `outline_variant` hairline.
            #[must_use]
            pub fn footer(mut self, footer: impl IntoElement) -> Self {
                self.footer = Some(footer.into_any_element());
                self
            }

            /// Overrides the M3 appearance mode for this card subtree.
            #[must_use]
            pub fn theme_override(mut self, mode: M3AppearanceMode) -> Self {
                self.mode_override = Some(mode);
                self
            }

            /// Makes the card interactive and registers a click callback.
            #[must_use]
            pub fn on_click(
                mut self,
                handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
            ) -> Self {
                self.on_click = Some(Rc::new(handler));
                self
            }
        }

        impl ParentElement for $name {
            fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
                self.children.extend(elements);
            }
        }

        impl Styled for $name {
            fn style(&mut self) -> &mut StyleRefinement {
                &mut self.style
            }
        }

        impl Disableable for $name {
            fn disabled(mut self, disabled: bool) -> Self {
                self.disabled = disabled;
                self
            }
        }

        impl RenderOnce for $name {
            fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
                render_m3_card(
                    M3CardSpec {
                        id: self.id,
                        kind: $kind,
                        title: self.title,
                        subtitle: self.subtitle,
                        header_action: self.header_action,
                        footer: self.footer,
                        children: self.children,
                        disabled: self.disabled,
                        style: self.style,
                        on_click: self.on_click,
                        mode_override: self.mode_override,
                    },
                    cx,
                )
            }
        }
    };
}

define_m3_card!(
    M3ElevatedCard,
    M3CardKind::Elevated,
    "Material 3 Expressive Elevated Card (`surface_container_low` + soft elevation shadow)."
);
define_m3_card!(
    M3FilledCard,
    M3CardKind::Filled,
    "Material 3 Expressive Filled Card (`surface_container_highest` surface)."
);
define_m3_card!(
    M3OutlinedCard,
    M3CardKind::Outlined,
    "Material 3 Expressive Outlined Card (`surface` + `outline_variant` 1px border)."
);

// ---------------------------------------------------------------------------
// 11. M3TopAppBar
// ---------------------------------------------------------------------------

/// Layout variant for [`M3TopAppBar`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum M3TopAppBarVariant {
    #[default]
    Small,
    CenterAligned,
    Medium,
    Large,
}

/// Material 3 Expressive Top App Bar (`Small`, `CenterAligned`, `Medium`, `Large`).
#[derive(IntoElement)]
pub struct M3TopAppBar {
    id: ElementId,
    title: SharedString,
    subtitle: Option<SharedString>,
    variant: M3TopAppBarVariant,
    scrolled: bool,
    navigation_icon: Option<AnyElement>,
    actions: SmallVec<[AnyElement; 3]>,
    style: StyleRefinement,
    mode_override: Option<M3AppearanceMode>,
}

impl M3TopAppBar {
    /// Creates a Top App Bar with stable domain `id` and `title`.
    #[must_use]
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            subtitle: None,
            variant: M3TopAppBarVariant::Small,
            scrolled: false,
            navigation_icon: None,
            actions: SmallVec::new(),
            style: StyleRefinement::default(),
            mode_override: None,
        }
    }

    /// Sets an optional secondary subtitle line.
    #[must_use]
    pub fn subtitle(mut self, subtitle: impl Into<SharedString>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Selects the M3 Top App Bar height/alignment variant.
    #[must_use]
    pub fn variant(mut self, variant: M3TopAppBarVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Transitions the container fill from `surface` to `surface_container` when scrolled.
    #[must_use]
    pub fn scrolled(mut self, scrolled: bool) -> Self {
        self.scrolled = scrolled;
        self
    }

    /// Sets the leading navigation icon/button.
    #[must_use]
    pub fn navigation_icon(mut self, icon: impl IntoElement) -> Self {
        self.navigation_icon = Some(icon.into_any_element());
        self
    }

    /// Appends a trailing action control.
    #[must_use]
    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.actions.push(action.into_any_element());
        self
    }

    /// Appends multiple trailing action controls.
    #[must_use]
    pub fn actions(mut self, actions: impl IntoIterator<Item = AnyElement>) -> Self {
        self.actions.extend(actions);
        self
    }

    /// Overrides the M3 appearance mode for this app bar.
    #[must_use]
    pub fn theme_override(mut self, mode: M3AppearanceMode) -> Self {
        self.mode_override = Some(mode);
        self
    }
}

impl ParentElement for M3TopAppBar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.actions.extend(elements);
    }
}

impl Styled for M3TopAppBar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for M3TopAppBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let m3 = self
            .mode_override
            .map(M3ActiveTheme::for_mode)
            .unwrap_or_else(|| M3ActiveTheme::resolve(cx));
        let colors = m3.colors;
        let bg = if self.scrolled {
            colors.surface_container
        } else {
            colors.surface
        };

        let title_spec = match self.variant {
            M3TopAppBarVariant::Large => &m3.typography.headline_medium,
            M3TopAppBarVariant::Medium => &m3.typography.headline_small,
            M3TopAppBarVariant::Small | M3TopAppBarVariant::CenterAligned => {
                &m3.typography.title_large
            }
        };

        let variant = self.variant;
        let title = self.title;
        let subtitle = self.subtitle;
        let title_size = title_spec.size_sp;
        let title_weight = title_spec.font_weight();
        let subtitle_size = m3.typography.body_small.size_sp;
        let on_surface = colors.on_surface;
        let on_surface_variant = colors.on_surface_variant;
        let build_title_block = move || {
            v_flex()
                .min_w_0()
                .when(variant == M3TopAppBarVariant::CenterAligned, |col| {
                    col.items_center()
                })
                .child(
                    div()
                        .text_size(px(title_size))
                        .font_weight(title_weight)
                        .text_color(on_surface)
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .child(title.clone()),
                )
                .when_some(subtitle.clone(), |col, subtitle| {
                    col.child(
                        div()
                            .text_size(px(subtitle_size))
                            .text_color(on_surface_variant)
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(subtitle),
                    )
                })
        };

        let top_row = h_flex()
            .flex_row_dir(cx)
            .w_full()
            .h(px(64.))
            .px_4()
            .gap_3()
            .justify_between()
            .items_center()
            .child(
                h_flex()
                    .flex_row_dir(cx)
                    .items_center()
                    .gap_2()
                    .min_w_0()
                    .when(variant != M3TopAppBarVariant::CenterAligned, |row| {
                        row.flex_1()
                    })
                    .when_some(self.navigation_icon, |row, nav| row.child(nav))
                    .when(matches!(variant, M3TopAppBarVariant::Small), |row| {
                        row.child(build_title_block())
                    }),
            )
            .when(variant == M3TopAppBarVariant::CenterAligned, |bar| {
                bar.child(div().flex_1().min_w_0().child(build_title_block()))
            })
            .child(
                h_flex()
                    .flex_row_dir(cx)
                    .items_center()
                    .gap_1p5()
                    .children(self.actions),
            );

        let debug_id = self.id.clone();
        v_flex()
            .id(self.id)
            .debug_selector(move || debug_id.to_string())
            .test_support()
            .w_full()
            .bg(bg)
            .text_color(colors.on_surface)
            .when(self.scrolled, |bar| {
                bar.border_b_1().border_color(colors.outline_variant)
            })
            .child(top_row)
            .when(
                matches!(
                    variant,
                    M3TopAppBarVariant::Medium | M3TopAppBarVariant::Large
                ),
                |bar| {
                    let bottom_h = if variant == M3TopAppBarVariant::Large {
                        px(88.)
                    } else {
                        px(48.)
                    };
                    bar.child(
                        div()
                            .w_full()
                            .h(bottom_h)
                            .px_4()
                            .pb_4()
                            .flex()
                            .items_end()
                            .child(build_title_block()),
                    )
                },
            )
            .refine_style(&self.style)
    }
}

// ---------------------------------------------------------------------------
// 12 & 13. M3NavigationRail & M3NavigationBar
// ---------------------------------------------------------------------------

/// Destination item shared by [`M3NavigationRail`] and [`M3NavigationBar`].
#[derive(Clone)]
pub struct M3NavDestination {
    pub id: SharedString,
    pub label: SharedString,
    pub icon: Icon,
    pub badge: Option<SharedString>,
    pub disabled: bool,
}

impl M3NavDestination {
    /// Creates a navigation destination with stable domain `id`, visible `label`, and `icon`.
    #[must_use]
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        icon: impl Into<Icon>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: icon.into(),
            badge: None,
            disabled: false,
        }
    }

    /// Attaches an M3 notification badge (`error` container + `on_error`).
    #[must_use]
    pub fn badge(mut self, badge: impl Into<SharedString>) -> Self {
        self.badge = Some(badge.into());
        self
    }

    /// Disables this navigation destination.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Material 3 Expressive vertical Navigation Rail (`80px` collapsed, `220px` expanded, `secondary_container` pill).
#[derive(IntoElement)]
pub struct M3NavigationRail {
    id: ElementId,
    destinations: Vec<M3NavDestination>,
    selected_id: Option<SharedString>,
    expanded: bool,
    header: Option<AnyElement>,
    footer: Option<AnyElement>,
    style: StyleRefinement,
    on_select: Option<IdSelectHandler>,
    mode_override: Option<M3AppearanceMode>,
}

impl M3NavigationRail {
    /// Creates an M3 Navigation Rail.
    #[must_use]
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            destinations: Vec::new(),
            selected_id: None,
            expanded: false,
            header: None,
            footer: None,
            style: StyleRefinement::default(),
            on_select: None,
            mode_override: None,
        }
    }

    /// Appends a single [`M3NavDestination`].
    #[must_use]
    pub fn destination(mut self, destination: M3NavDestination) -> Self {
        self.destinations.push(destination);
        self
    }

    /// Appends multiple [`M3NavDestination`] entries.
    #[must_use]
    pub fn destinations(
        mut self,
        destinations: impl IntoIterator<Item = M3NavDestination>,
    ) -> Self {
        self.destinations.extend(destinations);
        self
    }

    /// Sets the active destination domain ID.
    #[must_use]
    pub fn selected_id(mut self, id: impl Into<SharedString>) -> Self {
        self.selected_id = Some(id.into());
        self
    }

    /// Toggles between compact `80px` rail and `220px` expanded drawer rail.
    #[must_use]
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    /// Sets an optional top header (such as a FAB or brand mark).
    #[must_use]
    pub fn header(mut self, header: impl IntoElement) -> Self {
        self.header = Some(header.into_any_element());
        self
    }

    /// Sets an optional bottom footer.
    #[must_use]
    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    /// Overrides the M3 appearance mode for this rail.
    #[must_use]
    pub fn theme_override(mut self, mode: M3AppearanceMode) -> Self {
        self.mode_override = Some(mode);
        self
    }

    /// Registers a selection callback receiving the selected destination domain ID.
    #[must_use]
    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl Styled for M3NavigationRail {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for M3NavigationRail {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let m3 = self
            .mode_override
            .map(M3ActiveTheme::for_mode)
            .unwrap_or_else(|| M3ActiveTheme::resolve(cx));
        let colors = m3.colors;
        let shape = m3.shape;
        let state = m3.state;

        let mut list = v_flex().flex_1().gap_2().w_full();
        for dest in self.destinations {
            let selected = self.selected_id.as_ref() == Some(&dest.id);
            let domain_id = dest.id.clone();
            let on_select = self.on_select.clone();
            let disabled = dest.disabled;

            let pill_bg = if selected {
                colors.secondary_container
            } else {
                transparent_black()
            };
            let icon_fg = if disabled {
                colors.on_surface.alpha(state.disabled_content_opacity)
            } else if selected {
                colors.on_secondary_container
            } else {
                colors.on_surface_variant
            };
            let label_fg = if disabled {
                colors.on_surface.alpha(state.disabled_content_opacity)
            } else if selected {
                colors.on_surface
            } else {
                colors.on_surface_variant
            };
            let hover_bg = colors.state_layer(pill_bg, icon_fg, state.hover_opacity);

            let badge = dest.badge.map(|text| {
                div()
                    .px_1p5()
                    .py_0p5()
                    .rounded(px(shape.full_px))
                    .bg(colors.error)
                    .text_size(px(m3.typography.label_small.size_sp))
                    .font_weight(m3.typography.label_small.font_weight())
                    .text_color(colors.on_error)
                    .child(text)
            });

            let item_id: ElementId = (self.id.clone(), dest.id).into();
            let item_debug_id = item_id.clone();
            let item = BaseButton::new(item_id)
                .debug_selector(move || item_debug_id.to_string())
                .disabled(disabled)
                .selected(selected)
                .accessibility_label(dest.label.clone())
                .when_some((!disabled).then_some(on_select).flatten(), |this, handler| {
                    this.on_click(move |_, window, cx| handler(&domain_id, window, cx))
                })
                .min_h(px(m3.density.touch_target_min_px))
                .w_full()
                .cursor(if disabled {
                    gpui::CursorStyle::OperationNotAllowed
                } else {
                    gpui::CursorStyle::PointingHand
                })
                .map(|this| {
                    if self.expanded {
                        this.h_flex()
                            .justify_between()
                            .h(px(48.))
                            .px_4()
                            .rounded(px(shape.full_px))
                            .bg(pill_bg)
                            .when(!disabled, |el| el.hover(move |s| s.bg(hover_bg)))
                            .child(
                                h_flex()
                                    .gap_3()
                                    .min_w_0()
                                    .child(dest.icon.with_size(Size::Medium).text_color(icon_fg))
                                    .child(
                                        div()
                                            .text_size(px(m3.typography.label_large.size_sp))
                                            .font_weight(m3.typography.label_large.font_weight())
                                            .text_color(icon_fg)
                                            .whitespace_nowrap()
                                            .child(dest.label),
                                    ),
                            )
                            .when_some(badge, |row, b| row.child(b))
                    } else {
                        this.v_flex()
                            .items_center()
                            .justify_center()
                            .gap_1()
                            .py_1()
                            .child(
                                h_flex()
                                    .w(px(56.))
                                    .h(px(32.))
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(shape.full_px))
                                    .bg(pill_bg)
                                    .when(!disabled, |el| el.hover(move |s| s.bg(hover_bg)))
                                    .child(dest.icon.with_size(Size::Medium).text_color(icon_fg)),
                            )
                            .child(
                                div()
                                    .text_size(px(m3.typography.label_medium.size_sp))
                                    .font_weight(if selected {
                                        FontWeight::SEMIBOLD
                                    } else {
                                        FontWeight::MEDIUM
                                    })
                                    .text_color(label_fg)
                                    .whitespace_nowrap()
                                    .child(dest.label),
                            )
                            .when_some(badge, |col, b| col.child(b))
                    }
                });

            list = list.child(item);
        }

        let debug_id = self.id.clone();
        v_flex()
            .id(self.id)
            .debug_selector(move || debug_id.to_string())
            .role(Role::Navigation)
            .test_support()
            .h_full()
            .w(px(if self.expanded { 220. } else { 80. }))
            .px_2()
            .py_3()
            .gap_3()
            .bg(colors.surface_container_low)
            .border_r_1()
            .border_color(colors.outline_variant)
            .when_some(self.header, |this, header| this.child(header))
            .child(list)
            .when_some(self.footer, |this, footer| this.child(footer))
            .refine_style(&self.style)
    }
}

/// Material 3 Expressive bottom Navigation Bar (`>= 44x44pt` hit targets, `secondary_container` pill).
#[derive(IntoElement)]
pub struct M3NavigationBar {
    id: ElementId,
    destinations: Vec<M3NavDestination>,
    selected_id: Option<SharedString>,
    show_labels: bool,
    style: StyleRefinement,
    on_select: Option<IdSelectHandler>,
    mode_override: Option<M3AppearanceMode>,
}

impl M3NavigationBar {
    /// Creates a horizontal M3 Navigation Bar.
    #[must_use]
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            destinations: Vec::new(),
            selected_id: None,
            show_labels: true,
            style: StyleRefinement::default(),
            on_select: None,
            mode_override: None,
        }
    }

    /// Appends a single [`M3NavDestination`].
    #[must_use]
    pub fn destination(mut self, destination: M3NavDestination) -> Self {
        self.destinations.push(destination);
        self
    }

    /// Appends multiple [`M3NavDestination`] entries.
    #[must_use]
    pub fn destinations(
        mut self,
        destinations: impl IntoIterator<Item = M3NavDestination>,
    ) -> Self {
        self.destinations.extend(destinations);
        self
    }

    /// Sets the active destination domain ID.
    #[must_use]
    pub fn selected_id(mut self, id: impl Into<SharedString>) -> Self {
        self.selected_id = Some(id.into());
        self
    }

    /// Shows or hides text labels beneath the active pill.
    #[must_use]
    pub fn show_labels(mut self, show: bool) -> Self {
        self.show_labels = show;
        self
    }

    /// Overrides the M3 appearance mode for this navigation bar.
    #[must_use]
    pub fn theme_override(mut self, mode: M3AppearanceMode) -> Self {
        self.mode_override = Some(mode);
        self
    }

    /// Registers a selection callback receiving the selected destination domain ID.
    #[must_use]
    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl Styled for M3NavigationBar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for M3NavigationBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let m3 = self
            .mode_override
            .map(M3ActiveTheme::for_mode)
            .unwrap_or_else(|| M3ActiveTheme::resolve(cx));
        let colors = m3.colors;
        let shape = m3.shape;
        let state = m3.state;

        let debug_id = self.id.clone();
        let mut bar = h_flex()
            .id(self.id.clone())
            .debug_selector(move || debug_id.to_string())
            .role(Role::Navigation)
            .test_support()
            .w_full()
            .min_h(px(68.))
            .px_3()
            .py_2()
            .gap_2()
            .justify_between()
            .bg(colors.surface_container)
            .border_t_1()
            .border_color(colors.outline_variant);

        for dest in self.destinations {
            let selected = self.selected_id.as_ref() == Some(&dest.id);
            let domain_id = dest.id.clone();
            let on_select = self.on_select.clone();
            let disabled = dest.disabled;

            let pill_bg = if selected {
                colors.secondary_container
            } else {
                transparent_black()
            };
            let icon_fg = if disabled {
                colors.on_surface.alpha(state.disabled_content_opacity)
            } else if selected {
                colors.on_secondary_container
            } else {
                colors.on_surface_variant
            };
            let label_fg = if disabled {
                colors.on_surface.alpha(state.disabled_content_opacity)
            } else if selected {
                colors.on_surface
            } else {
                colors.on_surface_variant
            };
            let hover_bg = colors.state_layer(pill_bg, icon_fg, state.hover_opacity);

            let badge = dest.badge.map(|text| {
                div()
                    .px_1p5()
                    .rounded(px(shape.full_px))
                    .bg(colors.error)
                    .text_size(px(m3.typography.label_small.size_sp))
                    .font_weight(m3.typography.label_small.font_weight())
                    .text_color(colors.on_error)
                    .child(text)
            });

            let item_id: ElementId = (self.id.clone(), dest.id).into();
            let item_debug_id = item_id.clone();
            let item = BaseButton::new(item_id)
                .debug_selector(move || item_debug_id.to_string())
                .disabled(disabled)
                .selected(selected)
                .accessibility_label(dest.label.clone())
                .when_some((!disabled).then_some(on_select).flatten(), |this, handler| {
                    this.on_click(move |_, window, cx| handler(&domain_id, window, cx))
                })
                .flex()
                .flex_col()
                .flex_1()
                .min_w(px(m3.density.touch_target_min_px))
                .min_h(px(m3.density.touch_target_min_px))
                .items_center()
                .justify_center()
                .gap_1()
                .cursor(if disabled {
                    gpui::CursorStyle::OperationNotAllowed
                } else {
                    gpui::CursorStyle::PointingHand
                })
                .child(
                    h_flex()
                        .w(px(64.))
                        .h(px(32.))
                        .items_center()
                        .justify_center()
                        .gap_1()
                        .rounded(px(shape.full_px))
                        .bg(pill_bg)
                        .when(!disabled, |el| el.hover(move |s| s.bg(hover_bg)))
                        .child(dest.icon.with_size(Size::Medium).text_color(icon_fg))
                        .when_some(badge, |row, b| row.child(b)),
                )
                .when(self.show_labels, |col| {
                    col.child(
                        div()
                            .text_size(px(m3.typography.label_medium.size_sp))
                            .font_weight(if selected {
                                FontWeight::SEMIBOLD
                            } else {
                                FontWeight::MEDIUM
                            })
                            .text_color(label_fg)
                            .whitespace_nowrap()
                            .child(dest.label),
                    )
                });

            bar = bar.child(item);
        }

        bar.refine_style(&self.style)
    }
}

// ---------------------------------------------------------------------------
// 14. M3SearchBar
// ---------------------------------------------------------------------------

/// Material 3 Expressive Search Bar / Neural Omnibar (`surface_container_high`, full pill shape).
#[derive(IntoElement)]
pub struct M3SearchBar {
    id: ElementId,
    query: SharedString,
    placeholder: SharedString,
    leading_icon: Option<Icon>,
    trailing_actions: SmallVec<[AnyElement; 2]>,
    shortcut_hint: Option<SharedString>,
    clearable: bool,
    disabled: bool,
    size: Size,
    input_state: Option<Entity<InputState>>,
    accessibility_label: Option<SharedString>,
    style: StyleRefinement,
    on_click: Option<ClickHandler>,
    on_clear: Option<ClickHandler>,
    on_submit: Option<QueryHandler>,
    mode_override: Option<M3AppearanceMode>,
}

impl M3SearchBar {
    /// Creates an M3 Search Bar with stable domain `id`.
    #[must_use]
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            query: SharedString::default(),
            placeholder: "Search…".into(),
            leading_icon: Some(Icon::new(IconName::Search)),
            trailing_actions: SmallVec::new(),
            shortcut_hint: None,
            clearable: true,
            disabled: false,
            size: Size::Medium,
            input_state: None,
            accessibility_label: None,
            style: StyleRefinement::default(),
            on_click: None,
            on_clear: None,
            on_submit: None,
            mode_override: None,
        }
    }

    /// Sets the displayed query string (used when no retained [`InputState`] is attached).
    #[must_use]
    pub fn query(mut self, query: impl Into<SharedString>) -> Self {
        self.query = query.into();
        self
    }

    /// Sets the placeholder prompt text.
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Replaces the leading search icon.
    #[must_use]
    pub fn leading_icon(mut self, icon: impl Into<Icon>) -> Self {
        self.leading_icon = Some(icon.into());
        self
    }

    /// Appends a trailing action slot (for example voice mic or filter trigger).
    #[must_use]
    pub fn trailing_action(mut self, action: impl IntoElement) -> Self {
        self.trailing_actions.push(action.into_any_element());
        self
    }

    /// Displays a monospace keyboard shortcut hint chip (using `JetBrains Mono`).
    #[must_use]
    pub fn shortcut_hint(mut self, hint: impl Into<SharedString>) -> Self {
        self.shortcut_hint = Some(hint.into());
        self
    }

    /// Shows or hides the clear button when `query` is non-empty.
    #[must_use]
    pub fn clearable(mut self, clearable: bool) -> Self {
        self.clearable = clearable;
        self
    }

    /// Attaches a retained [`InputState`] for inline text editing inside the M3 search pill.
    #[must_use]
    pub fn input_state(mut self, state: Entity<InputState>) -> Self {
        self.input_state = Some(state);
        self
    }

    /// Sets the accessible label exposed to assistive technology.
    #[must_use]
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    /// Overrides the M3 appearance mode for this search bar.
    #[must_use]
    pub fn theme_override(mut self, mode: M3AppearanceMode) -> Self {
        self.mode_override = Some(mode);
        self
    }

    /// Registers a click handler for trigger-style search bars.
    #[must_use]
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Registers a handler called when the clear button is activated.
    #[must_use]
    pub fn on_clear(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_clear = Some(Rc::new(handler));
        self
    }

    /// Registers a submit handler invoked on Enter when focused without an attached [`InputState`].
    #[must_use]
    pub fn on_submit(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_submit = Some(Rc::new(handler));
        self
    }
}

impl Styled for M3SearchBar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Disableable for M3SearchBar {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Sizable for M3SearchBar {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for M3SearchBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let m3 = self
            .mode_override
            .map(M3ActiveTheme::for_mode)
            .unwrap_or_else(|| M3ActiveTheme::resolve(cx));
        let colors = m3.colors;
        let shape = m3.shape;
        let state = m3.state;

        let height = match self.size {
            Size::XSmall | Size::Small => px(44.),
            Size::Large => px(64.),
            Size::Size(px_val) => px_val,
            Size::Medium => px(56.),
        };

        let bg = if self.disabled {
            colors.on_surface.alpha(state.disabled_container_opacity)
        } else {
            colors.surface_container_high
        };
        let fg = if self.disabled {
            colors.on_surface.alpha(state.disabled_content_opacity)
        } else {
            colors.on_surface
        };
        let muted_fg = if self.disabled {
            colors.on_surface.alpha(state.disabled_content_opacity)
        } else {
            colors.on_surface_variant
        };
        let hover_bg = colors.state_layer(bg, fg, state.hover_opacity);

        let focus_handle = window
            .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let is_focused = focus_handle.is_focused(window);
        let a11y_label = self
            .accessibility_label
            .unwrap_or_else(|| self.placeholder.clone());
        let has_query = !self.query.is_empty();
        let bar_id = self.id.clone();

        let center_content: AnyElement = if let Some(input_state) = self.input_state {
            div()
                .flex_1()
                .min_w_0()
                .child(
                    Input::new(&input_state)
                        .id((bar_id.clone(), "input"))
                        .appearance(false)
                        .bordered(false)
                        .focus_bordered(false)
                        .disabled(self.disabled),
                )
                .into_any_element()
        } else {
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(m3.typography.body_large.size_sp))
                .text_color(if has_query { fg } else { muted_fg })
                .whitespace_nowrap()
                .text_ellipsis()
                .child(if has_query {
                    self.query.clone()
                } else {
                    self.placeholder.clone()
                })
                .into_any_element()
        };

        let query_for_submit = self.query.clone();
        let debug_id = bar_id.clone();

        h_flex()
            .id(self.id)
            .debug_selector(move || debug_id.to_string())
            .role(Role::SearchInput)
            .aria_label(a11y_label)
            .test_support()
            .track_focus(&focus_handle.tab_stop(!self.disabled))
            .w_full()
            .h(height)
            .px_4()
            .gap_3()
            .items_center()
            .rounded(px(shape.full_px))
            .bg(bg)
            .border_1()
            .border_color(if is_focused && !self.disabled {
                colors.primary
            } else {
                colors.outline_variant
            })
            .when(!self.disabled, |this| {
                this.hover(move |s| s.bg(hover_bg))
            })
            .when_some((!self.disabled).then_some(self.on_click).flatten(), |this, on_click| {
                this.cursor_pointer()
                    .on_click(move |event, window, cx| on_click(event, window, cx))
            })
            .when_some((!self.disabled).then_some(self.on_submit).flatten(), |this, on_submit| {
                this.on_key_down(move |event: &KeyDownEvent, window, cx| {
                    if event.keystroke.key == "enter" {
                        cx.stop_propagation();
                        on_submit(&query_for_submit, window, cx);
                    }
                })
            })
            .when_some(self.leading_icon, |this, icon| {
                this.child(icon.with_size(Size::Medium).text_color(muted_fg))
            })
            .child(center_content)
            .when(has_query && self.clearable && !self.disabled, |this| {
                let on_clear = self.on_clear.clone();
                this.child(
                    BaseButton::new((bar_id.clone(), "clear"))
                        .accessibility_label("Clear search")
                        .size(px(32.))
                        .rounded(px(shape.full_px))
                        .flex()
                        .items_center()
                        .justify_center()
                        .on_click(move |event, window, cx| {
                            cx.stop_propagation();
                            if let Some(ref on_clear) = on_clear {
                                on_clear(event, window, cx);
                            }
                        })
                        .child(Icon::new(IconName::Close).small().text_color(muted_fg)),
                )
            })
            .when_some(self.shortcut_hint, |this, hint| {
                this.child(
                    div()
                        .px_2()
                        .py_0p5()
                        .rounded(px(shape.small_px))
                        .bg(colors.surface_container_highest)
                        .font_family(m3.typography.typeface.mono_family())
                        .text_size(px(m3.typography.label_small.size_sp))
                        .text_color(muted_fg)
                        .child(hint),
                )
            })
            .children(self.trailing_actions)
            .refine_style(&self.style)
    }
}

// ---------------------------------------------------------------------------
// 15. M3ExpressiveSwitch
// ---------------------------------------------------------------------------

/// Material 3 Expressive Switch (52x32 track, dynamic thumb scaling 16px -> 24px, Spatial spring).
#[derive(IntoElement)]
pub struct M3ExpressiveSwitch {
    id: ElementId,
    checked: bool,
    disabled: bool,
    show_icons: bool,
    label: Option<SharedString>,
    accessibility_label: Option<SharedString>,
    style: StyleRefinement,
    on_change: Option<BoolChangeHandler>,
    mode_override: Option<M3AppearanceMode>,
}

impl M3ExpressiveSwitch {
    /// Creates an M3 Expressive Switch with stable domain `id`.
    #[must_use]
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            checked: false,
            disabled: false,
            show_icons: true,
            label: None,
            accessibility_label: None,
            style: StyleRefinement::default(),
            on_change: None,
            mode_override: None,
        }
    }

    /// Sets the controlled checked boolean state.
    #[must_use]
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Enables or disables the inner check/close icons inside the thumb.
    #[must_use]
    pub fn show_icons(mut self, show: bool) -> Self {
        self.show_icons = show;
        self
    }

    /// Sets an optional visible text label beside the switch track.
    #[must_use]
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the screen-reader accessible label.
    #[must_use]
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    /// Overrides the M3 appearance mode for this switch.
    #[must_use]
    pub fn theme_override(mut self, mode: M3AppearanceMode) -> Self {
        self.mode_override = Some(mode);
        self
    }

    /// Registers a change handler receiving the requested next checked state.
    #[must_use]
    pub fn on_change(
        mut self,
        handler: impl Fn(&bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl Styled for M3ExpressiveSwitch {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Disableable for M3ExpressiveSwitch {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for M3ExpressiveSwitch {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let m3 = self
            .mode_override
            .map(M3ActiveTheme::for_mode)
            .unwrap_or_else(|| M3ActiveTheme::resolve(cx));
        let colors = m3.colors;
        let shape = m3.shape;
        let state = m3.state;

        let checked = self.checked;
        let disabled = self.disabled;
        let focus_handle = window
            .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let is_focused = focus_handle.is_focused(window);
        let a11y_label = self.accessibility_label.or_else(|| self.label.clone());

        let track_w = px(52.);
        let track_h = px(32.);
        let target_thumb_size = if checked {
            px(24.)
        } else if self.show_icons {
            px(24.)
        } else {
            px(16.)
        };
        let target_thumb_x = if checked {
            px(22.)
        } else if self.show_icons {
            px(2.)
        } else {
            px(6.)
        };

        let spatial_spring = M3SpringToken::SpatialFast.gpui_spring().with_epsilon(0.1);
        let thumb_x = spring(
            (self.id.clone(), "thumb-x"),
            target_thumb_x,
            spatial_spring,
            window,
            cx,
        );
        let thumb_size = spring(
            (self.id.clone(), "thumb-size"),
            target_thumb_size,
            spatial_spring,
            window,
            cx,
        );

        let track_bg = if disabled {
            colors.on_surface.alpha(state.disabled_container_opacity)
        } else if checked {
            colors.primary
        } else {
            colors.surface_container_highest
        };
        let track_border = if disabled {
            colors.on_surface.alpha(state.disabled_container_opacity)
        } else if checked {
            colors.primary
        } else {
            colors.outline
        };
        let thumb_bg = if disabled {
            if checked {
                colors.surface
            } else {
                colors.on_surface.alpha(state.disabled_content_opacity)
            }
        } else if checked {
            colors.on_primary
        } else {
            colors.outline
        };
        let icon_color = if checked {
            colors.on_primary_container
        } else {
            colors.surface_container_highest
        };

        let mut track = SwitchTrack::new((self.id.clone(), "track"))
            .checked(checked)
            .disabled(disabled)
            .w(track_w)
            .h(track_h)
            .flex_shrink_0()
            .rounded(px(shape.full_px))
            .flex_row_dir(cx)
            .items_center()
            .border_2()
            .border_color(track_border)
            .bg(track_bg)
            .child(
                SwitchThumb::new(checked)
                    .disabled(disabled)
                    .size(thumb_size)
                    .inset_s(thumb_x, cx)
                    .rounded(px(shape.full_px))
                    .bg(thumb_bg)
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(self.show_icons && !disabled, |thumb| {
                        thumb.child(
                            Icon::new(if checked {
                                IconName::Check
                            } else {
                                IconName::Close
                            })
                            .xsmall()
                            .text_color(icon_color),
                        )
                    }),
            );

        if is_focused && !disabled {
            track = apply_m3_focus_ring(track, colors.primary, state.focus_ring_px, window, cx);
        }

        let switch_id = self.id.clone();
        div().refine_style(&self.style).child(
            BaseSwitch::new(self.id)
                .debug_selector(move || switch_id.to_string())
                .checked(checked)
                .disabled(disabled)
                .track_focus(&focus_handle)
                .when_some(a11y_label, |this, label| this.accessibility_label(label))
                .when_some((!disabled).then_some(self.on_change).flatten(), |this, handler| {
                    this.on_change(move |next, _, window, cx| handler(&next, window, cx))
                })
                .h_flex()
                .flex_row_dir(cx)
                .min_h(px(m3.density.touch_target_min_px))
                .gap_3()
                .cursor(if disabled {
                    gpui::CursorStyle::OperationNotAllowed
                } else {
                    gpui::CursorStyle::PointingHand
                })
                .child(track)
                .when_some(self.label, |this, label| {
                    this.child(
                        div()
                            .text_size(px(m3.typography.body_medium.size_sp))
                            .text_color(if disabled {
                                colors.on_surface.alpha(state.disabled_content_opacity)
                            } else {
                                colors.on_surface
                            })
                            .child(label),
                    )
                }),
        )
    }
}

// ---------------------------------------------------------------------------
// 16. M3DialogSheet
// ---------------------------------------------------------------------------

/// Placement mode for [`M3DialogSheet`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum M3DialogSheetPlacement {
    #[default]
    CenterDialog,
    BottomSheet,
    SideSheet,
}

/// Material 3 Expressive Dialog & Sheet (`surface_container_high`, `28px` extra-large shape,
/// `24px` padding, optional modal scrim + Escape dismissal).
#[derive(IntoElement)]
pub struct M3DialogSheet {
    id: ElementId,
    open: bool,
    modal: bool,
    placement: M3DialogSheetPlacement,
    icon: Option<Icon>,
    title: Option<SharedString>,
    supporting_text: Option<SharedString>,
    confirm_label: Option<SharedString>,
    dismiss_label: Option<SharedString>,
    footer: Option<AnyElement>,
    children: SmallVec<[AnyElement; 2]>,
    style: StyleRefinement,
    on_confirm: Option<ClickHandler>,
    on_dismiss: Option<ClickHandler>,
    mode_override: Option<M3AppearanceMode>,
}

impl M3DialogSheet {
    /// Creates an M3 Dialog/Sheet with stable domain `id` (visible by default unless `.open(false)`).
    #[must_use]
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            open: true,
            modal: false,
            placement: M3DialogSheetPlacement::CenterDialog,
            icon: None,
            title: None,
            supporting_text: None,
            confirm_label: None,
            dismiss_label: None,
            footer: None,
            children: SmallVec::new(),
            style: StyleRefinement::default(),
            on_confirm: None,
            on_dismiss: None,
            mode_override: None,
        }
    }

    /// Controls visibility; when `false`, renders an empty placeholder.
    #[must_use]
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Wraps the sheet in a full-viewport modal scrim with Escape and backdrop dismissal.
    #[must_use]
    pub fn modal(mut self, modal: bool) -> Self {
        self.modal = modal;
        self
    }

    /// Selects `CenterDialog`, `BottomSheet`, or `SideSheet` geometry.
    #[must_use]
    pub fn placement(mut self, placement: M3DialogSheetPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Sets an optional top hero icon (`secondary` tint).
    #[must_use]
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Sets the dialog/sheet headline (`headline_small`).
    #[must_use]
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets supporting body text (`body_medium` in `on_surface_variant`).
    #[must_use]
    pub fn supporting_text(mut self, text: impl Into<SharedString>) -> Self {
        self.supporting_text = Some(text.into());
        self
    }

    /// Sets the primary confirm action label (renders [`M3FilledButton`]).
    #[must_use]
    pub fn confirm_label(mut self, label: impl Into<SharedString>) -> Self {
        self.confirm_label = Some(label.into());
        self
    }

    /// Sets the secondary dismiss action label (renders [`M3TonalButton`]).
    #[must_use]
    pub fn dismiss_label(mut self, label: impl Into<SharedString>) -> Self {
        self.dismiss_label = Some(label.into());
        self
    }

    /// Replaces the default action row with a custom footer element.
    #[must_use]
    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    /// Overrides the M3 appearance mode for this dialog/sheet.
    #[must_use]
    pub fn theme_override(mut self, mode: M3AppearanceMode) -> Self {
        self.mode_override = Some(mode);
        self
    }

    /// Registers the confirm button handler.
    #[must_use]
    pub fn on_confirm(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_confirm = Some(Rc::new(handler));
        self
    }

    /// Registers the dismiss/close handler (called on dismiss button, backdrop click, or Escape).
    #[must_use]
    pub fn on_dismiss(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_dismiss = Some(Rc::new(handler));
        self
    }
}

impl ParentElement for M3DialogSheet {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for M3DialogSheet {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for M3DialogSheet {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        if !self.open {
            return gpui::Empty.into_any_element();
        }

        let m3 = self
            .mode_override
            .map(M3ActiveTheme::for_mode)
            .unwrap_or_else(|| M3ActiveTheme::resolve(cx));
        let colors = m3.colors;
        let shape = m3.shape;
        let sheet_id = self.id.clone();

        let actions_row = if let Some(footer) = self.footer {
            Some(footer)
        } else if self.dismiss_label.is_some() || self.confirm_label.is_some() {
            let mut row = h_flex().justify_end().gap_2().pt_2();
            if let Some(dismiss_label) = self.dismiss_label {
                let mut button = M3TonalButton::new((sheet_id.clone(), "dismiss"), dismiss_label);
                if let Some(mode) = self.mode_override {
                    button = button.theme_override(mode);
                }
                if let Some(ref on_dismiss) = self.on_dismiss {
                    let handler = Rc::clone(on_dismiss);
                    button = button.on_click(move |ev, window, cx| handler(ev, window, cx));
                }
                row = row.child(button);
            }
            if let Some(confirm_label) = self.confirm_label {
                let mut button = M3FilledButton::new((sheet_id.clone(), "confirm"), confirm_label);
                if let Some(mode) = self.mode_override {
                    button = button.theme_override(mode);
                }
                if let Some(on_confirm) = self.on_confirm {
                    button = button.on_click(move |ev, window, cx| on_confirm(ev, window, cx));
                }
                row = row.child(button);
            }
            Some(row.into_any_element())
        } else {
            None
        };

        let mut surface = v_flex()
            .id(self.id.clone())
            .debug_selector(move || sheet_id.to_string())
            .role(Role::Dialog)
            .test_support()
            .occlude()
            .bg(colors.surface_container_high)
            .text_color(colors.on_surface)
            .p(px(24.))
            .gap_4()
            .shadow(m3_elevation_shadow(&colors, 3))
            .map(|this| match self.placement {
                M3DialogSheetPlacement::CenterDialog => this
                    .min_w(px(280.))
                    .max_w(px(560.))
                    .rounded(px(shape.extra_large_px)),
                M3DialogSheetPlacement::BottomSheet => this
                    .w_full()
                    .max_w(px(640.))
                    .rounded_t(px(shape.extra_large_px)),
                M3DialogSheetPlacement::SideSheet => this
                    .w(px(360.))
                    .h_full()
                    .rounded_l(px(shape.extra_large_px))
                    .border_l_1()
                    .border_color(colors.outline_variant),
            })
            .when_some(self.icon, |this, icon| {
                this.child(
                    div()
                        .flex()
                        .justify_center()
                        .child(icon.large().text_color(colors.secondary)),
                )
            })
            .when_some(self.title.clone(), |this, title| {
                this.child(
                    div()
                        .text_size(px(m3.typography.headline_small.size_sp))
                        .font_weight(m3.typography.headline_small.font_weight())
                        .text_color(colors.on_surface)
                        .child(title),
                )
            })
            .when_some(self.supporting_text, |this, text| {
                this.child(
                    div()
                        .text_size(px(m3.typography.body_medium.size_sp))
                        .text_color(colors.on_surface_variant)
                        .child(text),
                )
            })
            .children(self.children)
            .when_some(actions_row, |this, row| this.child(row))
            .refine_style(&self.style);

        if let Some(title) = self.title {
            surface = surface.aria_label(title);
        }

        if !self.modal {
            return surface.into_any_element();
        }

        let viewport = window.viewport_size();
        let focus_handle = window
            .use_keyed_state((self.id.clone(), "dialog-focus"), cx, |_, cx| {
                cx.focus_handle()
            })
            .read(cx)
            .clone();
        let on_dismiss = self.on_dismiss;
        let escape_dismiss = on_dismiss.clone();
        let scrim_dismiss = on_dismiss;

        anchored()
            .position(point(px(0.), px(0.)))
            .child(
                div()
                    .id((self.id, "modal-backdrop"))
                    .absolute()
                    .top_0()
                    .left_0()
                    .w(viewport.width)
                    .h(viewport.height)
                    .bg(colors.surface_dim.alpha(0.55))
                    .track_focus(&focus_handle)
                    .focus_trap("m3-dialog-sheet", &focus_handle)
                    .on_action(move |_: &Cancel, window, cx| {
                        cx.stop_propagation();
                        if let Some(ref handler) = escape_dismiss {
                            handler(&ClickEvent::default(), window, cx);
                        }
                    })
                    .on_any_mouse_down(move |event, window, cx| {
                        cx.stop_propagation();
                        if event.button == MouseButton::Left
                            && let Some(ref handler) = scrim_dismiss
                        {
                            handler(&ClickEvent::default(), window, cx);
                        }
                    })
                    .flex()
                    .map(|container| match self.placement {
                        M3DialogSheetPlacement::CenterDialog => {
                            container.items_center().justify_center().p_4()
                        }
                        M3DialogSheetPlacement::BottomSheet => {
                            container.items_end().justify_center()
                        }
                        M3DialogSheetPlacement::SideSheet => {
                            container.items_stretch().justify_end()
                        }
                    })
                    .child(
                        div()
                            .on_any_mouse_down(|_, _, cx| cx.stop_propagation())
                            .child(surface),
                    ),
            )
            .into_any_element()
    }
}

// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// Unit & GPUI Integration Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use gpui::{Context, Render, TestAppContext};

    use super::*;

    #[test]
    fn bundled_manifest_parses_all_four_modes_typefaces_axes_and_springs() -> Result<()> {
        let (source_of_truth, system, seed_hex) = m3_manifest_metadata()?;
        assert_eq!(source_of_truth, M3_DESIGN_SOURCE_OF_TRUTH);
        assert_eq!(system, "Material 3 Expressive");
        assert_eq!(seed_hex, "#135B82");
        let typography = M3TypographyScale::try_bundled()?;
        let shape = M3ShapeTokens::try_bundled()?;
        let density = M3DensityTokens::try_bundled()?;
        assert_eq!(typography.typeface.brand_family().as_ref(), "Google Sans Flex");
        assert_eq!(typography.typeface.mono_family().as_ref(), "JetBrains Mono");
        assert_eq!(shape.medium_px, 12.0);
        assert_eq!(shape.large_px, 16.0);
        assert_eq!(shape.extra_large_px, 28.0);
        assert_eq!(density.touch_target_min_px, 44.0);

        for axis in ["opsz", "wght", "wdth", "GRAD", "slnt", "ROND"] {
            assert!(
                brand::design::axis_spec(axis).is_some(),
                "missing Google Sans Flex axis {axis}"
            );
        }

        for mode in M3AppearanceMode::ALL {
            let parsed = M3ColorRoles::try_for_mode(mode)?;
            let direct = M3ColorRoles::for_mode(mode);
            assert_eq!(parsed, direct, "palette mismatch for {mode:?}");
        }

        Ok(())
    }

    #[test]
    fn all_four_modes_pass_wcag_aa_contrast_across_semantic_pairs() -> Result<()> {
        for mode in M3AppearanceMode::ALL {
            let reports = m3_verify_mode_contrast(mode)?;
            assert_eq!(reports.len(), 14);
            for report in reports {
                assert!(
                    report.passes_aa && report.ratio >= 4.5,
                    "{mode:?} {} contrast {:.2} < 4.5",
                    report.pair,
                    report.ratio
                );
            }

            let config = try_m3_theme_config(mode)?;
            assert_eq!(config.name.as_ref(), mode.display_name());
            assert_eq!(config.mode, mode.theme_mode());
            assert_eq!(config.font_family.as_deref(), Some("Google Sans Flex"));
            assert_eq!(config.mono_font_family.as_deref(), Some("JetBrains Mono"));
        }

        assert_eq!(
            M3AppearanceMode::from_name("neural-obsidian"),
            Some(M3AppearanceMode::NeuralObsidian)
        );
        assert_eq!(
            M3AppearanceMode::from_name("oled_pitch"),
            Some(M3AppearanceMode::OledPitch)
        );
        assert_eq!(M3AppearanceMode::OledPitch.as_str(), "oled_pitch");
        assert_eq!(M3ColorRoles::for_mode(M3AppearanceMode::OledPitch).surface.l, 0.0);
        assert!(m3_verify_contrast("pure-bw", gpui::white(), gpui::black())? >= 20.9);
        assert!(m3_verify_contrast("low-contrast", gpui::white(), gpui::white()).is_err());

        Ok(())
    }

    #[test]
    fn google_sans_flex_axes_and_spring_presets_follow_spec() -> Result<()> {
        for preset in [
            M3GoogleSansFlexAxes::default(),
            M3GoogleSansFlexAxes::display_hero(),
            M3GoogleSansFlexAxes::section_headline(),
            M3GoogleSansFlexAxes::ui_control(),
            M3GoogleSansFlexAxes::compact_mobile(),
            M3GoogleSansFlexAxes::tabular_label(),
            M3GoogleSansFlexAxes::xr_glimmer_glanceable(),
            M3GoogleSansFlexAxes::quire_ink_body(),
        ] {
            preset.validate()?;
        }

        let variation = M3GoogleSansFlexAxes::display_hero().css_font_variation_settings();
        assert!(variation.contains("\"opsz\" 48.0"));
        assert!(variation.contains("\"ROND\" 40"));

        for spring in M3SpringToken::ALL {
            let spec = spring.try_spec()?;
            assert_eq!(spring.sample_position(0.0), 0.0);
            assert!(
                (spring.sample_position(3.0) - 1.0).abs() < 1e-2,
                "{spring:?} did not settle"
            );
            if spring.is_spatial() {
                assert!((spec.damping_ratio - 0.9).abs() < 1e-4);
            }
            if spring.is_effects() {
                assert!((spec.damping_ratio - 1.0).abs() < 1e-4);
                for step in 1..=25 {
                    let pos = spring.sample_position(step as f32 * 0.02);
                    assert!(pos <= 1.001, "{spring:?} overshot: {pos}");
                }
            }
            let gpui_spring = spring.gpui_spring();
            let spring_cfg = spring.gpui_spring_config();
            let expected_damping = 2.0 * spec.damping_ratio * spec.stiffness.sqrt();
            assert!(format!("{gpui_spring:?}").contains("Spring"));
            assert!((spring_cfg.stiffness - spec.stiffness).abs() < 1e-4);
            assert!((spring_cfg.damping - expected_damping).abs() < 1e-3);
            assert!((spring_cfg.mass - 1.0).abs() < 1e-4);
        }

        let motion = m3_motion_tokens();
        assert!(motion.duration_fast < motion.duration_normal);
        assert!(motion.duration_normal < motion.duration_slow);
        Ok(())
    }

    #[gpui::test]
    fn apply_m3_theme_cycles_all_four_modes_and_syncs_base(cx: &mut TestAppContext) {
        cx.update(crate::init);

        for mode in M3AppearanceMode::ALL {
            cx.update(|cx| {
                apply_m3_theme(mode, cx);
                let active = active_m3_theme(cx);
                assert_eq!(active.mode, mode);
                assert_eq!(Theme::global(cx).mode, mode.theme_mode());
                assert_eq!(Theme::global(cx).primary, active.colors.primary);
                assert_eq!(
                    gpui_base::Theme::global(cx).tokens.colors.background,
                    active.colors.background
                );
            });
        }
    }

    #[gpui::test]
    fn all_sixteen_m3_components_build_render_and_dispatch_callbacks(cx: &mut TestAppContext) {
        struct GalleryHarness {
            clicks: Rc<Cell<usize>>,
            chip_selected: Rc<Cell<bool>>,
            switch_checked: Rc<Cell<bool>>,
            selected_segment: Rc<std::cell::RefCell<SharedString>>,
            selected_nav: Rc<std::cell::RefCell<SharedString>>,
            dialog_open: Rc<Cell<bool>>,
            dismissed: Rc<Cell<usize>>,
        }

        impl Render for GalleryHarness {
            fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
                let clicks = self.clicks.clone();
                let chip_selected = self.chip_selected.clone();
                let switch_checked = self.switch_checked.clone();
                let selected_segment = self.selected_segment.clone();
                let selected_nav = self.selected_nav.clone();
                let current_segment = selected_segment.borrow().clone();
                let current_nav = selected_nav.borrow().clone();
                let dialog_open = self.dialog_open.clone();
                let dismissed = self.dismissed.clone();
                let view = cx.entity().downgrade();

                let notify = move |cx: &mut App| {
                    assert!(
                        view.update(cx, |_, cx| cx.notify()).is_ok(),
                        "GalleryHarness view should stay alive during test"
                    );
                };

                v_flex()
                    .w(px(720.))
                    .gap_2()
                    .p_3()
                    .child(
                        M3TopAppBar::new("top-bar", "Material 3 Expressive")
                            .subtitle("Neural Expressive Kit")
                            .variant(M3TopAppBarVariant::Small)
                            .scrolled(true)
                            .action(M3FilledButton::new("top-save", "Save")),
                    )
                    .child(
                        M3SearchBar::new("search")
                            .query("design tokens")
                            .shortcut_hint("Ctrl+K")
                            .on_click({
                                let clicks = clicks.clone();
                                move |_, _, _| clicks.set(clicks.get() + 1)
                            }),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                M3FilledButton::new("filled-btn", "Primary")
                                    .icon(IconName::Plus)
                                    .on_click({
                                        let clicks = clicks.clone();
                                        move |_, _, _| clicks.set(clicks.get() + 1)
                                    }),
                            )
                            .child(M3TonalButton::new("tonal-btn", "Tonal"))
                            .child(M3OutlinedButton::new("outlined-btn", "Outlined"))
                            .child(
                                M3Fab::new("fab", IconName::Plus)
                                    .fab_size(M3FabSize::Standard)
                                    .accessibility_label("Create"),
                            )
                            .child(
                                M3ExtendedFab::new("xfab", "Compose")
                                    .icon(IconName::Plus)
                                    .color(M3FabColor::Tertiary),
                            ),
                    )
                    .child(
                        M3SegmentedButtonBar::new("segments")
                            .segment(M3SegmentItem::new("day", "Day"))
                            .segment(M3SegmentItem::new("week", "Week"))
                            .segment(M3SegmentItem::new("month", "Month"))
                            .selected_id(current_segment)
                            .on_select({
                                let notify = notify.clone();
                                move |id, _, cx| {
                                    *selected_segment.borrow_mut() = id.clone();
                                    notify(cx);
                                }
                            }),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                M3FilterChip::new("filter-chip", "Accessible")
                                    .selected(chip_selected.get())
                                    .on_change({
                                        let notify = notify.clone();
                                        move |next, _, cx| {
                                            chip_selected.set(*next);
                                            notify(cx);
                                        }
                                    }),
                            )
                            .child(
                                M3ExpressiveSwitch::new("switch")
                                    .label("OLED Pitch")
                                    .checked(switch_checked.get())
                                    .on_change({
                                        let notify = notify.clone();
                                        move |next, _, cx| {
                                            switch_checked.set(*next);
                                            notify(cx);
                                        }
                                    }),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                M3ElevatedCard::new()
                                    .id("elevated-card")
                                    .title("Elevated")
                                    .subtitle("Surface container low")
                                    .child("Body"),
                            )
                            .child(
                                M3FilledCard::new()
                                    .id("filled-card")
                                    .title("Filled")
                                    .child("Body"),
                            )
                            .child(
                                M3OutlinedCard::new()
                                    .id("outlined-card")
                                    .title("Outlined")
                                    .footer("Footer"),
                            ),
                    )
                    .child(
                        h_flex()
                            .h(px(110.))
                            .gap_2()
                            .child(
                                M3NavigationRail::new("rail")
                                    .destination(M3NavDestination::new(
                                        "home",
                                        "Home",
                                        IconName::Inbox,
                                    ))
                                    .destination(
                                        M3NavDestination::new("tokens", "Tokens", IconName::Palette)
                                            .badge("4"),
                                    )
                                    .selected_id(current_nav.clone())
                                    .on_select({
                                        let selected_nav = selected_nav.clone();
                                        let notify = notify.clone();
                                        move |id, _, cx| {
                                            *selected_nav.borrow_mut() = id.clone();
                                            notify(cx);
                                        }
                                    }),
                            )
                            .child(
                                M3DialogSheet::new("sheet")
                                    .open(dialog_open.get())
                                    .title("Sync Tokens")
                                    .supporting_text("Apply Neural Obsidian across surfaces?")
                                    .dismiss_label("Cancel")
                                    .confirm_label("Apply")
                                    .on_dismiss({
                                        let notify = notify.clone();
                                        move |_, _, cx| {
                                            dismissed.set(dismissed.get() + 1);
                                            dialog_open.set(false);
                                            notify(cx);
                                        }
                                    }),
                            ),
                    )
                    .child(
                        M3NavigationBar::new("nav-bar")
                            .destination(M3NavDestination::new("home", "Home", IconName::Inbox))
                            .destination(M3NavDestination::new(
                                "settings",
                                "Settings",
                                IconName::Settings,
                            ))
                            .selected_id(current_nav)
                            .on_select({
                                let selected_nav = selected_nav.clone();
                                move |id, _, cx| {
                                    *selected_nav.borrow_mut() = id.clone();
                                    notify(cx);
                                }
                            }),
                    )
            }
        }

        cx.update(|cx| {
            crate::init(cx);
            apply_m3_theme(M3AppearanceMode::NeuralObsidian, cx);
        });

        let clicks = Rc::new(Cell::new(0));
        let chip_selected = Rc::new(Cell::new(false));
        let switch_checked = Rc::new(Cell::new(false));
        let selected_segment = Rc::new(std::cell::RefCell::new(SharedString::from("day")));
        let selected_nav = Rc::new(std::cell::RefCell::new(SharedString::from("home")));
        let dialog_open = Rc::new(Cell::new(true));
        let dismissed = Rc::new(Cell::new(0));

        let (_, cx) = cx.add_window_view({
            let clicks = clicks.clone();
            let chip_selected = chip_selected.clone();
            let switch_checked = switch_checked.clone();
            let selected_segment = selected_segment.clone();
            let selected_nav = selected_nav.clone();
            let dialog_open = dialog_open.clone();
            let dismissed = dismissed.clone();
            move |_, _| GalleryHarness {
                clicks,
                chip_selected,
                switch_checked,
                selected_segment,
                selected_nav,
                dialog_open,
                dismissed,
            }
        });

        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });

        // Verify modal dialog sheet mounts with action buttons and dismisses cleanly before hitting underlying controls.
        assert!(cx.debug_bounds("sheet").is_some());
        let dismiss_bounds = cx
            .debug_bounds("sheet-dismiss")
            .expect("dialog dismiss button must lay out");
        cx.simulate_click(dismiss_bounds.center(), gpui::Modifiers::default());
        cx.run_until_parked();
        assert_eq!(dismissed.get(), 1);
        assert!(!dialog_open.get());
        assert!(cx.debug_bounds("sheet").is_none());

        let filled_bounds = cx
            .debug_bounds("filled-btn")
            .expect("filled button must lay out");
        cx.simulate_click(filled_bounds.center(), gpui::Modifiers::default());
        assert_eq!(clicks.get(), 1);

        let search_bounds = cx
            .debug_bounds("search")
            .expect("search bar must lay out");
        cx.simulate_click(search_bounds.center(), gpui::Modifiers::default());
        assert_eq!(clicks.get(), 2);

        let segment_week_bounds = cx
            .debug_bounds("segments-week")
            .expect("segmented item must lay out");
        cx.simulate_click(segment_week_bounds.center(), gpui::Modifiers::default());
        cx.run_until_parked();
        assert_eq!(*selected_segment.borrow(), SharedString::from("week"));

        let chip_bounds = cx
            .debug_bounds("filter-chip")
            .expect("filter chip must lay out");
        cx.simulate_click(chip_bounds.center(), gpui::Modifiers::default());
        cx.run_until_parked();
        assert!(chip_selected.get());

        let switch_bounds = cx
            .debug_bounds("switch")
            .expect("expressive switch must lay out");
        cx.simulate_click(switch_bounds.center(), gpui::Modifiers::default());
        cx.run_until_parked();
        assert!(switch_checked.get());

        let rail_tokens_bounds = cx
            .debug_bounds("rail-tokens")
            .expect("navigation rail destination must lay out");
        cx.simulate_click(rail_tokens_bounds.center(), gpui::Modifiers::default());
        cx.run_until_parked();
        assert_eq!(*selected_nav.borrow(), SharedString::from("tokens"));

        let nav_settings_bounds = cx
            .debug_bounds("nav-bar-settings")
            .expect("navigation bar destination must lay out");
        cx.simulate_click(nav_settings_bounds.center(), gpui::Modifiers::default());
        cx.run_until_parked();
        assert_eq!(*selected_nav.borrow(), SharedString::from("settings"));

        assert!(cx.debug_bounds("top-bar").is_some());
        assert!(cx.debug_bounds("tonal-btn").is_some());
        assert!(cx.debug_bounds("outlined-btn").is_some());
        assert!(cx.debug_bounds("fab").is_some());
        assert!(cx.debug_bounds("xfab").is_some());
        assert!(cx.debug_bounds("segments").is_some());
        assert!(cx.debug_bounds("elevated-card").is_some());
        assert!(cx.debug_bounds("filled-card").is_some());
        assert!(cx.debug_bounds("outlined-card").is_some());
        assert!(cx.debug_bounds("rail").is_some());
        assert!(cx.debug_bounds("nav-bar").is_some());
    }
}
