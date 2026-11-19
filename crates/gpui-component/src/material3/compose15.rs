//! End-to-end Compose Material 3 `1.5.0` Stable (`https://m3.material.io/blog/compose-material-3-is-stable`)
//! & Material 3 Design Kit (`https://www.figma.com/community/file/1035203688168086460/material-3-design-kit`)
//! architecture and component implementation for GPUI Kit (`crates/gpui-component`).
//!
//! Covers every core & Material 3 Expressive API graduated to stable in Compose Material 3 1.5.0
//! (published 2026-11-18 alongside Jetpack Compose 1.13 / BOM `2026.11.01`):
//! - Unified `M3ExpressiveThemeScope` (`MaterialExpressiveTheme` + `M3MotionScheme::Standard | Expressive`
//!   + `Typography(defaultFontFamily)` + `material3-ripple:1.5.0` Inset Focus Rings)
//! - All 35 normalized `M3MaterialShape` polygons (`MaterialShapes` + `MorphPolygonShape` -> `gpui::Path<Pixels>`)
//!   plus `M3LoadingIndicator` (`LoadingIndicator` / `ContainedLoadingIndicator`)
//! - Shape-morphing `M3ToggleButton` (`Filled`, `Elevated`, `FilledTonal`, `Outlined` + `M3ButtonSize`)
//! - `M3SplitButton`, `M3ButtonGroup` (adaptive width compression + overflow indicator), `M3FloatingToolbar`
//! - `M3WavyLinearProgressIndicator` & `M3WavyCircularProgressIndicator` (GPU `PathBuilder` wave geometry + finite-value safety)
//! - `M3SegmentedListItem` (`Single`/`First`/`Middle`/`Last` rounded container segmenting + vertical alignment)
//! - `M3StatefulSlider` & `M3StatefulRangeSlider` (`M3SliderState` / `M3RangeSliderState` with `start_value`/`end_value`,
//!   exact integer-tick snapping, and keyboard navigation)
//! - `M3ExpressiveTextField` (`FilledTonal`/`Outlined`, `Inside`/`Cutout` label, `System` default obfuscation)
//! - `M3ExpressiveMenuItem` (`Selectable` radio / `Checkable` checkbox + cascading `M3PopupAnchor`)
//! - `M3Scrim` & deterministic `M3BottomSheetAnchor` (`Hidden` / `PartiallyExpanded` / `Expanded`)
//! - `M3ScrollField` & `M3VibrantTimePicker` (Expressive Time Picker + ScrollField)
//! - Embedded pure WebGPU / Naga v30 WGSL shader (`assets/shaders/m3_expressive.wgsl`).

use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, ClickEvent, ElementId, Entity, FontWeight, Global, Hsla,
    InteractiveElement, IntoElement, KeyDownEvent, MouseButton, ParentElement, Path, PathBuilder,
    Pixels, RenderOnce, SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled,
    Window, canvas, div, hsla, point, prelude::FluentBuilder as _, px, transparent_black,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::{
    Icon, IconName, MotionTokens, StyledExt as _, Theme, h_flex, v_flex,
};

use super::{
    M3ActiveTheme, M3AppearanceMode, M3SpringSpec, M3TypographyScale,
    apply_m3_theme, m3_elevation_shadow,
};

/// Authoritative Compose Material 3 stable release version (`2026-11-18`).
pub const COMPOSE_M3_VERSION: &str = "1.5.0";

/// Paired Jetpack Compose stable release version.
pub const COMPOSE_VERSION: &str = "1.13.0";

/// Official November 2026 Jetpack Compose BOM coordinate version.
pub const COMPOSE_BOM_VERSION: &str = "2026.11.01";

/// Authoritative Material Design blog URL for Compose Material 3 1.5.0 Stable.
pub const COMPOSE_M3_BLOG_URL: &str = "https://m3.material.io/blog/compose-material-3-is-stable";

/// Authoritative Material Design blog URL for Parametric Prototyping.
pub const M3_PARAMETRIC_BLOG_URL: &str =
    "https://m3.material.io/blog/enhance-your-component-designs-with-parametric-prototyping";

/// Official Google Material 3 Figma Community Design Kit file identifier.
pub const FIGMA_M3_DESIGN_KIT_FILE_ID: &str = "1035203688168086460";

/// Official Google Material 3 Figma Community Design Kit URL.
pub const FIGMA_M3_DESIGN_KIT_URL: &str =
    "https://www.figma.com/community/file/1035203688168086460/material-3-design-kit";

/// Embedded pure WebGPU / Naga v30 WGSL shader implementing 35 MaterialShapes SDF morphing,
/// wavy linear/circular progress, `material3-ripple:1.5.0` inset focus rings, and progressive blur.
pub const M3_EXPRESSIVE_WGSL: &str = include_str!("../../../../assets/shaders/m3_expressive.wgsl");

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;
type BoolChangeHandler = Rc<dyn Fn(&bool, &mut Window, &mut App) + 'static>;
type FloatChangeHandler = Rc<dyn Fn(f32, &mut Window, &mut App) + 'static>;
type RangeChangeHandler = Rc<dyn Fn(f32, f32, &mut Window, &mut App) + 'static>;
type TimeChangeHandler = Rc<dyn Fn(u8, u8, bool, &mut Window, &mut App) + 'static>;
type IntChangeHandler = Rc<dyn Fn(i32, &mut Window, &mut App) + 'static>;

// ---------------------------------------------------------------------------
// 1. Compose Material 3 1.5.0 Stable Catalog & Parity Matrix
// ---------------------------------------------------------------------------

/// Machine-verifiable Compose Material 3 1.5.0 -> GPUI Kit parity entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct M3ComposeParityEntry {
    pub compose_api: &'static str,
    pub gpui_kit_api: &'static str,
    pub figma_kit_group: &'static str,
    pub stability: &'static str,
}

/// Authoritative Compose Material 3 1.5.0 Stable release snapshot mapped to GPUI Kit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct M3Compose15Catalog {
    pub material3_version: &'static str,
    pub compose_version: &'static str,
    pub compose_bom: &'static str,
    pub blog_url: &'static str,
    pub parametric_blog_url: &'static str,
    pub figma_kit_file_id: &'static str,
    pub figma_kit_url: &'static str,
    pub experimental_before: u32,
    pub experimental_after: u32,
    pub material2_deprecated: bool,
    pub material_shapes_count: usize,
    pub wgsl_bytes: usize,
    pub parity_entries: Vec<M3ComposeParityEntry>,
}

impl M3Compose15Catalog {
    /// Returns the complete Compose Material 3 1.5.0 -> GPUI Kit parity catalog.
    #[must_use]
    pub fn canonical() -> Self {
        Self {
            material3_version: COMPOSE_M3_VERSION,
            compose_version: COMPOSE_VERSION,
            compose_bom: COMPOSE_BOM_VERSION,
            blog_url: COMPOSE_M3_BLOG_URL,
            parametric_blog_url: M3_PARAMETRIC_BLOG_URL,
            figma_kit_file_id: FIGMA_M3_DESIGN_KIT_FILE_ID,
            figma_kit_url: FIGMA_M3_DESIGN_KIT_URL,
            experimental_before: 120,
            experimental_after: 58,
            material2_deprecated: true,
            material_shapes_count: M3MaterialShape::ALL_35.len(),
            wgsl_bytes: M3_EXPRESSIVE_WGSL.len(),
            parity_entries: vec![
                M3ComposeParityEntry {
                    compose_api: "MaterialExpressiveTheme / MaterialTheme.motionScheme",
                    gpui_kit_api: "M3ExpressiveThemeScope / M3MotionScheme",
                    figma_kit_group: "02_Sys/Color + Motion",
                    stability: "stable",
                },
                M3ComposeParityEntry {
                    compose_api: "material3-ripple:1.5.0 / LocalRippleThemeConfiguration",
                    gpui_kit_api: "M3RippleThemeConfig (Inset Focus Rings)",
                    figma_kit_group: "02_Sys/State",
                    stability: "stable",
                },
                M3ComposeParityEntry {
                    compose_api: "Typography(defaultFontFamily = ...)",
                    gpui_kit_api: "m3_typography_with_default_font_family",
                    figma_kit_group: "02_Sys/Typescale",
                    stability: "stable",
                },
                M3ComposeParityEntry {
                    compose_api: "ToggleButton / Elevated / FilledTonal / OutlinedToggleButton",
                    gpui_kit_api: "M3ToggleButton + M3ToggleButtonVariant + M3ButtonSize",
                    figma_kit_group: "Components/Buttons",
                    stability: "stable",
                },
                M3ComposeParityEntry {
                    compose_api: "SplitButton",
                    gpui_kit_api: "M3SplitButton",
                    figma_kit_group: "Components/Buttons",
                    stability: "stable",
                },
                M3ComposeParityEntry {
                    compose_api: "ButtonGroup + Modifier.animateWidth(compressionLimit)",
                    gpui_kit_api: "M3ButtonGroup",
                    figma_kit_group: "Components/Buttons",
                    stability: "stable",
                },
                M3ComposeParityEntry {
                    compose_api: "FloatingToolbar / HorizontalFloatingToolbar",
                    gpui_kit_api: "M3FloatingToolbar",
                    figma_kit_group: "Components/Toolbars",
                    stability: "stable",
                },
                M3ComposeParityEntry {
                    compose_api: "WavyLinearProgressIndicator / WavyCircularProgressIndicator",
                    gpui_kit_api: "M3WavyLinearProgressIndicator / M3WavyCircularProgressIndicator",
                    figma_kit_group: "Components/Progress",
                    stability: "stable",
                },
                M3ComposeParityEntry {
                    compose_api: "Segmented ListItem / InteractiveListItem",
                    gpui_kit_api: "M3SegmentedListItem",
                    figma_kit_group: "Components/Lists",
                    stability: "stable",
                },
                M3ComposeParityEntry {
                    compose_api: "Slider(SliderState) / RangeSlider(RangeSliderState)",
                    gpui_kit_api: "M3StatefulSlider / M3StatefulRangeSlider",
                    figma_kit_group: "Components/Sliders",
                    stability: "stable",
                },
                M3ComposeParityEntry {
                    compose_api: "TextField / OutlinedTextField / SecureTextField(System)",
                    gpui_kit_api: "M3ExpressiveTextField",
                    figma_kit_group: "Components/TextFields",
                    stability: "stable",
                },
                M3ComposeParityEntry {
                    compose_api: "SelectableDropdownMenuItem / CheckableDropdownMenuItem",
                    gpui_kit_api: "M3ExpressiveMenuItem + M3PopupAnchor",
                    figma_kit_group: "Components/Menus",
                    stability: "stable",
                },
                M3ComposeParityEntry {
                    compose_api: "Scrim + Deterministic BottomSheet anchors",
                    gpui_kit_api: "M3Scrim + M3BottomSheetAnchor",
                    figma_kit_group: "Components/Sheets",
                    stability: "stable",
                },
                M3ComposeParityEntry {
                    compose_api: "VibrantTimePickerDialog + ScrollField",
                    gpui_kit_api: "M3VibrantTimePicker + M3ScrollField",
                    figma_kit_group: "Components/Pickers",
                    stability: "stable",
                },
                M3ComposeParityEntry {
                    compose_api: "MaterialShapes (35) + LoadingIndicator / ContainedLoadingIndicator",
                    gpui_kit_api: "M3MaterialShape::ALL_35 + M3LoadingIndicator",
                    figma_kit_group: "02_Sys/Shape",
                    stability: "experimental_with_gpui_parity",
                },
            ],
        }
    }
}

// ---------------------------------------------------------------------------
// 2. Unified Expressive Theme, MotionScheme & material3-ripple Inset Focus Rings
// ---------------------------------------------------------------------------

/// Compose Material 3 1.5.0 `MotionScheme` (`Standard` vs `Expressive`).
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum M3MotionScheme {
    Standard,
    #[default]
    Expressive,
}

impl M3MotionScheme {
    /// Spatial fast spring (`damping_ratio = 0.6` expressive, `0.9` standard).
    #[must_use]
    pub const fn spatial_fast(self) -> M3SpringSpec {
        match self {
            Self::Standard => M3SpringSpec {
                stiffness: 1400.0,
                damping_ratio: 0.9,
            },
            Self::Expressive => M3SpringSpec {
                stiffness: 800.0,
                damping_ratio: 0.6,
            },
        }
    }

    /// Spatial default spring (`damping_ratio = 0.6` expressive, `0.9` standard).
    #[must_use]
    pub const fn spatial_default(self) -> M3SpringSpec {
        match self {
            Self::Standard => M3SpringSpec {
                stiffness: 700.0,
                damping_ratio: 0.9,
            },
            Self::Expressive => M3SpringSpec {
                stiffness: 380.0,
                damping_ratio: 0.6,
            },
        }
    }

    /// Spatial slow spring (`damping_ratio = 0.6` expressive, `0.9` standard).
    #[must_use]
    pub const fn spatial_slow(self) -> M3SpringSpec {
        match self {
            Self::Standard => M3SpringSpec {
                stiffness: 300.0,
                damping_ratio: 0.9,
            },
            Self::Expressive => M3SpringSpec {
                stiffness: 200.0,
                damping_ratio: 0.6,
            },
        }
    }

    /// Effects fast spring (critically damped `1.0`, zero overshoot).
    #[must_use]
    pub const fn effects_fast(self) -> M3SpringSpec {
        M3SpringSpec {
            stiffness: 3800.0,
            damping_ratio: 1.0,
        }
    }

    /// Effects default spring (critically damped `1.0`, zero overshoot).
    #[must_use]
    pub const fn effects_default(self) -> M3SpringSpec {
        M3SpringSpec {
            stiffness: 1600.0,
            damping_ratio: 1.0,
        }
    }

    /// Effects slow spring (critically damped `1.0`, zero overshoot).
    #[must_use]
    pub const fn effects_slow(self) -> M3SpringSpec {
        M3SpringSpec {
            stiffness: 800.0,
            damping_ratio: 1.0,
        }
    }

    /// Converts this `MotionScheme` into `gpui-component` [`MotionTokens`].
    #[must_use]
    pub fn to_motion_tokens(self) -> MotionTokens {
        let mut tokens = MotionTokens::default();
        tokens.duration_fast = self.effects_fast().response_duration();
        tokens.duration_normal = self.effects_default().response_duration();
        tokens.duration_slow = self.spatial_slow().response_duration();
        tokens.spring_control = self.effects_default().gpui_spring();
        tokens.spring_move = self.spatial_default().gpui_spring().with_epsilon(0.1);
        tokens
    }
}

/// `androidx.compose.material3:material3-ripple:1.5.0` configuration with Inset Focus Rings.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct M3RippleThemeConfig {
    /// Renders visual inset focus rings instead of opacity-only focus overlays.
    pub inset_focus_ring_enabled: bool,
    /// Inset distance from outer container edge in logical px.
    pub focus_ring_inset_px: f32,
    /// Stroke thickness of the inset focus ring in logical px.
    pub focus_ring_thickness_px: f32,
    pub hover_alpha: f32,
    pub focus_alpha: f32,
    pub pressed_alpha: f32,
    pub dragged_alpha: f32,
}

impl Default for M3RippleThemeConfig {
    fn default() -> Self {
        Self {
            inset_focus_ring_enabled: true,
            focus_ring_inset_px: 2.0,
            focus_ring_thickness_px: 3.0,
            hover_alpha: 0.08,
            focus_alpha: 0.10,
            pressed_alpha: 0.12,
            dragged_alpha: 0.16,
        }
    }
}

/// Consolidated Material 3 1.5.0 runtime configuration stored as a single GPUI [`Global`],
/// mirroring Compose M3 1.5.0's consolidated `MaterialTheme.LocalMaterialTheme.current`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct M3ExpressiveRuntimeConfig {
    pub motion_scheme: M3MotionScheme,
    pub ripple: M3RippleThemeConfig,
    pub default_font_family: Option<SharedString>,
}

impl Global for M3ExpressiveRuntimeConfig {}

impl Default for M3ExpressiveRuntimeConfig {
    fn default() -> Self {
        Self {
            motion_scheme: M3MotionScheme::Expressive,
            ripple: M3RippleThemeConfig::default(),
            default_font_family: None,
        }
    }
}

/// Reads the active [`M3ExpressiveRuntimeConfig`] purely from `cx` (gpui-fast safe).
#[must_use]
pub fn active_m3_expressive_config(cx: &App) -> M3ExpressiveRuntimeConfig {
    cx.try_global::<M3ExpressiveRuntimeConfig>()
        .cloned()
        .unwrap_or_default()
}

/// Mirrors Compose M3 1.5.0 `Typography(defaultFontFamily = ...)` by propagating a single brand
/// font family across both `brand` and `plain` typeface roles in [`M3TypographyScale`].
#[must_use]
pub fn m3_typography_with_default_font_family(
    mut scale: M3TypographyScale,
    default_font_family: impl Into<SharedString>,
) -> M3TypographyScale {
    let family: SharedString = default_font_family.into();
    if !family.trim().is_empty() {
        scale.typeface.brand = family.clone();
        scale.typeface.plain = family;
    }
    scale
}

/// Equivalent of Compose M3 1.5.0 `MaterialExpressiveTheme(colorScheme, motionScheme, shapes, typography)`
/// with integrated `material3-ripple:1.5.0` inset focus ring settings.
#[derive(Debug, Clone, PartialEq)]
pub struct M3ExpressiveThemeScope {
    pub mode: M3AppearanceMode,
    pub motion_scheme: M3MotionScheme,
    pub ripple: M3RippleThemeConfig,
    pub default_font_family: Option<SharedString>,
}

impl Default for M3ExpressiveThemeScope {
    fn default() -> Self {
        Self::new(M3AppearanceMode::Dark)
    }
}

impl M3ExpressiveThemeScope {
    #[must_use]
    pub fn new(mode: M3AppearanceMode) -> Self {
        Self {
            mode,
            motion_scheme: M3MotionScheme::Expressive,
            ripple: M3RippleThemeConfig::default(),
            default_font_family: None,
        }
    }

    #[must_use]
    pub fn motion_scheme(mut self, motion_scheme: M3MotionScheme) -> Self {
        self.motion_scheme = motion_scheme;
        self
    }

    #[must_use]
    pub fn ripple(mut self, ripple: M3RippleThemeConfig) -> Self {
        self.ripple = ripple;
        self
    }

    #[must_use]
    pub fn default_font_family(mut self, family: impl Into<SharedString>) -> Self {
        self.default_font_family = Some(family.into());
        self
    }

    /// Atomically applies color scheme, motion scheme, typography brand font, and inset focus rings.
    pub fn apply(&self, cx: &mut App) {
        apply_m3_theme(self.mode, cx);

        if let Some(ref family) = self.default_font_family {
            let mut active = M3ActiveTheme::resolve(cx);
            active.typography =
                m3_typography_with_default_font_family(active.typography, family.to_string());
            cx.set_global(active);
        }

        let motion_tokens = self.motion_scheme.to_motion_tokens();
        let font_override = self.default_font_family.clone();
        Theme::update(cx, move |theme| {
            theme.motion = motion_tokens;
            if let Some(font) = font_override {
                theme.font_family = font;
            }
        });

        cx.set_global(M3ExpressiveRuntimeConfig {
            motion_scheme: self.motion_scheme,
            ripple: self.ripple,
            default_font_family: self.default_font_family.clone(),
        });
    }
}

/// Applies a [`M3ExpressiveThemeScope`] onto `cx`.
pub fn apply_m3_expressive_theme(scope: &M3ExpressiveThemeScope, cx: &mut App) {
    scope.apply(cx);
}

/// Helper rendering a `material3-ripple:1.5.0` visual inset focus ring overlay when `focused`.
fn render_inset_focus_ring(
    radius: Pixels,
    ring_color: Hsla,
    ripple: M3RippleThemeConfig,
) -> AnyElement {
    let inset_px = ripple.focus_ring_inset_px.max(1.0);
    let thickness_px = ripple.focus_ring_thickness_px.max(1.5);
    let inner_radius = px((f32::from(radius) - inset_px).max(2.0));
    div()
        .flex_none()
        .absolute()
        .inset(px(inset_px))
        .rounded(inner_radius)
        .border(px(thickness_px))
        .border_color(ring_color)
        .into_any_element()
}

// ---------------------------------------------------------------------------
// 3. All 35 MaterialShapes + MorphPolygonShape + M3LoadingIndicator
// ---------------------------------------------------------------------------

/// All 35 predefined Compose Material 3 `1.5.0` `MaterialShapes` polygons.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum M3MaterialShape {
    #[default]
    Circle,
    Square,
    Slanted,
    Arch,
    Fan,
    Arrow,
    SemiCircle,
    Oval,
    Pill,
    Triangle,
    Diamond,
    ClamShell,
    Pentagon,
    Gem,
    Sunny,
    VerySunny,
    Cookie4Sided,
    Cookie6Sided,
    Cookie7Sided,
    Cookie9Sided,
    Cookie12Sided,
    Ghostish,
    Clover4Leaf,
    Clover8Leaf,
    Burst,
    SoftBurst,
    Boom,
    SoftBoom,
    Flower,
    Puffy,
    PuffyDiamond,
    PixelCircle,
    PixelTriangle,
    Bun,
    Heart,
}

impl M3MaterialShape {
    /// Complete canonical 35-shape Material 3 1.5.0 catalog.
    pub const ALL_35: [Self; 35] = [
        Self::Circle,
        Self::Square,
        Self::Slanted,
        Self::Arch,
        Self::Fan,
        Self::Arrow,
        Self::SemiCircle,
        Self::Oval,
        Self::Pill,
        Self::Triangle,
        Self::Diamond,
        Self::ClamShell,
        Self::Pentagon,
        Self::Gem,
        Self::Sunny,
        Self::VerySunny,
        Self::Cookie4Sided,
        Self::Cookie6Sided,
        Self::Cookie7Sided,
        Self::Cookie9Sided,
        Self::Cookie12Sided,
        Self::Ghostish,
        Self::Clover4Leaf,
        Self::Clover8Leaf,
        Self::Burst,
        Self::SoftBurst,
        Self::Boom,
        Self::SoftBoom,
        Self::Flower,
        Self::Puffy,
        Self::PuffyDiamond,
        Self::PixelCircle,
        Self::PixelTriangle,
        Self::Bun,
        Self::Heart,
    ];

    /// Numeric shader ID matching `assets/shaders/m3_expressive.wgsl`.
    #[must_use]
    pub const fn shader_id(self) -> u32 {
        self as u32
    }

    /// Harmonic radial lobe count used for polygon generation and WGSL SDF morphing.
    #[must_use]
    pub const fn harmonic_lobes(self) -> f32 {
        match self {
            Self::Circle | Self::Oval | Self::Pill | Self::PixelCircle => 0.0,
            Self::Triangle | Self::PixelTriangle => 3.0,
            Self::Square
            | Self::Diamond
            | Self::Cookie4Sided
            | Self::Clover4Leaf
            | Self::PuffyDiamond => 4.0,
            Self::Pentagon => 5.0,
            Self::Cookie6Sided | Self::Gem => 6.0,
            Self::Cookie7Sided => 7.0,
            Self::Clover8Leaf | Self::Sunny | Self::Flower => 8.0,
            Self::Cookie9Sided | Self::SoftBurst => 9.0,
            Self::Cookie12Sided | Self::VerySunny | Self::Burst | Self::Boom | Self::SoftBoom => {
                12.0
            }
            Self::Heart => 2.0,
            _ => 4.0,
        }
    }

    /// Normalized radial modulation depth for polar contour evaluation.
    #[must_use]
    const fn modulation_depth(self) -> f32 {
        match self {
            Self::Circle | Self::Oval | Self::Pill | Self::PixelCircle => 0.0,
            Self::Burst | Self::Boom | Self::Clover4Leaf | Self::Clover8Leaf => 0.24,
            Self::SoftBurst | Self::SoftBoom | Self::Flower | Self::VerySunny => 0.18,
            _ => 0.13,
        }
    }

    /// Evaluates normalized polar radius `r(theta)` in `[0.4, 1.0]` for angle `theta`.
    #[must_use]
    pub fn sample_radius(self, theta: f32) -> f32 {
        if self == Self::Heart {
            let s = theta.sin();
            let c = theta.cos();
            let heart = 0.72 + 0.18 * s - 0.10 * (2.0 * theta).cos() + 0.06 * c.abs();
            return heart.clamp(0.42, 1.0);
        }
        let lobes = self.harmonic_lobes();
        let depth = self.modulation_depth();
        if lobes <= 0.5 || depth <= 0.0 {
            return 1.0;
        }
        (1.0 - depth + depth * (lobes * theta).cos()).clamp(0.4, 1.0)
    }

    /// Builds a GPUI [`Path<Pixels>`] morphing smoothly from `self` to `target` at `progress` (`0.0..=1.0`).
    #[must_use]
    pub fn build_morphed_path(
        self,
        bounds: Bounds<Pixels>,
        target: Self,
        progress: f32,
    ) -> Option<Path<Pixels>> {
        let width = f32::from(bounds.size.width);
        let height = f32::from(bounds.size.height);
        if width <= 1.0 || height <= 1.0 {
            return None;
        }
        let cx = f32::from(bounds.origin.x) + width * 0.5;
        let cy = f32::from(bounds.origin.y) + height * 0.5;
        let rx = width * 0.45;
        let ry = height * 0.45;
        let t = progress.clamp(0.0, 1.0);
        let steps = 48usize;

        let mut builder = PathBuilder::fill();
        for idx in 0..steps {
            let theta = (idx as f32 / steps as f32) * std::f32::consts::TAU;
            let r_a = self.sample_radius(theta);
            let r_b = target.sample_radius(theta);
            let r = r_a + (r_b - r_a) * t;
            let pt = point(px(cx + r * rx * theta.cos()), px(cy + r * ry * theta.sin()));
            if idx == 0 {
                builder.move_to(pt);
            } else {
                builder.line_to(pt);
            }
        }
        builder.close();
        builder.build().ok()
    }
}

/// Compose Material 3 `LoadingIndicator` / `ContainedLoadingIndicator` morphing across `MaterialShapes`.
#[derive(IntoElement)]
pub struct M3LoadingIndicator {
    id: ElementId,
    start_shape: M3MaterialShape,
    end_shape: M3MaterialShape,
    morph_progress: f32,
    size_px: f32,
    contained: bool,
    mode_override: Option<M3AppearanceMode>,
}

impl M3LoadingIndicator {
    #[must_use]
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            start_shape: M3MaterialShape::SoftBurst,
            end_shape: M3MaterialShape::Cookie9Sided,
            morph_progress: 0.5,
            size_px: 48.0,
            contained: false,
            mode_override: None,
        }
    }

    #[must_use]
    pub fn contained(mut self, contained: bool) -> Self {
        self.contained = contained;
        self
    }

    #[must_use]
    pub fn shapes(mut self, start: M3MaterialShape, end: M3MaterialShape) -> Self {
        self.start_shape = start;
        self.end_shape = end;
        self
    }

    #[must_use]
    pub fn progress(mut self, progress: f32) -> Self {
        self.morph_progress = if progress.is_finite() {
            progress.clamp(0.0, 1.0)
        } else {
            0.5
        };
        self
    }

    #[must_use]
    pub fn size_px(mut self, size_px: f32) -> Self {
        self.size_px = size_px.max(20.0);
        self
    }

    #[must_use]
    pub fn mode(mut self, mode: M3AppearanceMode) -> Self {
        self.mode_override = Some(mode);
        self
    }
}

impl RenderOnce for M3LoadingIndicator {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let m3 = self
            .mode_override
            .map(M3ActiveTheme::for_mode)
            .unwrap_or_else(|| M3ActiveTheme::resolve(cx));
        let start = self.start_shape;
        let end = self.end_shape;
        let progress = self.morph_progress;
        let fill = if self.contained {
            m3.colors.on_primary_container
        } else {
            m3.colors.primary
        };
        let container_bg = if self.contained {
            m3.colors.primary_container
        } else {
            transparent_black()
        };

        let debug_id = self.id.clone();
        div()
            .id(self.id)
            .debug_selector(move || debug_id.to_string())
            .w(px(self.size_px))
            .h(px(self.size_px))
            .rounded_full()
            .bg(container_bg)
            .flex()
            .items_center()
            .justify_center()
            .p(px(if self.contained { 6.0 } else { 0.0 }))
            .child(
                canvas(
                    move |bounds, _, _| start.build_morphed_path(bounds, end, progress),
                    move |_bounds, path_opt, window, _| {
                        if let Some(path) = path_opt {
                            window.paint_path(path, fill);
                        }
                    },
                )
                .size_full(),
            )
    }
}

// ---------------------------------------------------------------------------
// 4. Expressive Buttons: M3ToggleButton, M3SplitButton, M3ButtonGroup
// ---------------------------------------------------------------------------

/// Compose Material 3 1.5.0 `ButtonSize` scale (`XSmall`..`XLarge`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum M3ButtonSize {
    XSmall,
    Small,
    #[default]
    Medium,
    Large,
    XLarge,
}

impl M3ButtonSize {
    #[must_use]
    pub const fn height_px(self) -> f32 {
        match self {
            Self::XSmall => 32.0,
            Self::Small => 36.0,
            Self::Medium => 40.0,
            Self::Large => 56.0,
            Self::XLarge => 72.0,
        }
    }

    #[must_use]
    pub const fn padding_x_px(self) -> f32 {
        match self {
            Self::XSmall => 12.0,
            Self::Small => 16.0,
            Self::Medium => 20.0,
            Self::Large => 28.0,
            Self::XLarge => 36.0,
        }
    }

    /// Expressive shape-morph corner radius when checked/toggled (squircle instead of pill).
    #[must_use]
    pub const fn checked_radius_px(self) -> f32 {
        match self {
            Self::XSmall | Self::Small => 8.0,
            Self::Medium => 12.0,
            Self::Large => 16.0,
            Self::XLarge => 20.0,
        }
    }
}

/// Compose Material 3 1.5.0 `ToggleButton` variants.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum M3ToggleButtonVariant {
    #[default]
    Filled,
    Elevated,
    FilledTonal,
    Outlined,
}

/// Compose Material 3 1.5.0 shape-morphing `ToggleButton` (`Filled`, `Elevated`, `FilledTonal`, `Outlined`).
#[derive(IntoElement)]
pub struct M3ToggleButton {
    id: ElementId,
    label: SharedString,
    variant: M3ToggleButtonVariant,
    button_size: M3ButtonSize,
    checked: bool,
    focused: bool,
    disabled: bool,
    icon: Option<Icon>,
    on_checked_change: Option<BoolChangeHandler>,
    mode_override: Option<M3AppearanceMode>,
    style: StyleRefinement,
}

impl M3ToggleButton {
    #[must_use]
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            variant: M3ToggleButtonVariant::Filled,
            button_size: M3ButtonSize::Medium,
            checked: false,
            focused: false,
            disabled: false,
            icon: None,
            on_checked_change: None,
            mode_override: None,
            style: StyleRefinement::default(),
        }
    }

    #[must_use]
    pub fn variant(mut self, variant: M3ToggleButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    #[must_use]
    pub fn button_size(mut self, size: M3ButtonSize) -> Self {
        self.button_size = size;
        self
    }

    #[must_use]
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    #[must_use]
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    #[must_use]
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    #[must_use]
    pub fn on_checked_change(
        mut self,
        handler: impl Fn(&bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_checked_change = Some(Rc::new(handler));
        self
    }

    #[must_use]
    pub fn mode(mut self, mode: M3AppearanceMode) -> Self {
        self.mode_override = Some(mode);
        self
    }

    /// Computes resting corner radius: pill (`full_px`) when unchecked, expressive squircle when checked.
    #[must_use]
    pub fn resolved_corner_radius_px(&self, full_px: f32) -> f32 {
        if self.checked {
            self.button_size.checked_radius_px()
        } else {
            full_px
        }
    }
}

impl Styled for M3ToggleButton {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for M3ToggleButton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let m3 = self
            .mode_override
            .map(M3ActiveTheme::for_mode)
            .unwrap_or_else(|| M3ActiveTheme::resolve(cx));
        let runtime = active_m3_expressive_config(cx);
        let colors = m3.colors;
        let radius = px(self.resolved_corner_radius_px(m3.shape.full_px));
        let height = px(self.button_size.height_px());
        let pad_x = px(self.button_size.padding_x_px());

        let (bg, fg, border_color, elevation) = match (self.variant, self.checked) {
            (M3ToggleButtonVariant::Filled, true) => (colors.primary, colors.on_primary, None, 0u8),
            (M3ToggleButtonVariant::Filled, false) => (
                colors.surface_container,
                colors.on_surface_variant,
                None,
                0u8,
            ),
            (M3ToggleButtonVariant::Elevated, true) => {
                (colors.primary, colors.on_primary, None, 2u8)
            }
            (M3ToggleButtonVariant::Elevated, false) => (
                colors.surface_container_low,
                colors.primary,
                None,
                1u8,
            ),
            (M3ToggleButtonVariant::FilledTonal, true) => (
                colors.secondary_container,
                colors.on_secondary_container,
                None,
                0u8,
            ),
            (M3ToggleButtonVariant::FilledTonal, false) => (
                colors.surface_container_highest,
                colors.on_surface_variant,
                None,
                0u8,
            ),
            (M3ToggleButtonVariant::Outlined, true) => (
                colors.on_surface,
                colors.surface,
                Some(colors.on_surface),
                0u8,
            ),
            (M3ToggleButtonVariant::Outlined, false) => (
                transparent_black(),
                colors.on_surface_variant,
                Some(colors.outline),
                0u8,
            ),
        };

        let hover_bg = colors.state_layer(bg, fg, runtime.ripple.hover_alpha);
        let pressed_bg = colors.state_layer(bg, fg, runtime.ripple.pressed_alpha);
        let checked = self.checked;
        let disabled = self.disabled;
        let on_change = self.on_checked_change;
        let pressed_radius = px(m3.shape.small_px);

        let debug_id = self.id.clone();
        let mut root = h_flex()
            .id(self.id)
            .debug_selector(move || debug_id.to_string())
            .relative()
            .h(height)
            .px(pad_x)
            .gap(px(8.0))
            .items_center()
            .justify_center()
            .rounded(radius)
            .bg(if disabled {
                colors.on_surface.alpha(m3.state.disabled_container_opacity)
            } else {
                bg
            })
            .text_color(if disabled {
                colors.on_surface.alpha(m3.state.disabled_content_opacity)
            } else {
                fg
            })
            .text_sm()
            .font_weight(FontWeight::MEDIUM)
            .when(elevation > 0 && !disabled, |this| {
                this.shadow(m3_elevation_shadow(&colors, elevation))
            })
            .when_some(border_color, |this, border| {
                this.border_1().border_color(border)
            })
            .when(!disabled, |this| {
                this.cursor_pointer()
                    .hover(move |s| s.bg(hover_bg))
                    .active(move |s| s.bg(pressed_bg).rounded(pressed_radius))
                    .when_some(on_change, |el, handler| {
                        el.on_click(move |_, window, cx| {
                            let next = !checked;
                            handler(&next, window, cx);
                        })
                    })
            });

        if let Some(icon) = self.icon {
            root = root.child(icon.text_color(fg));
        } else if checked {
            root = root.child(Icon::new(IconName::Check).text_color(fg));
        }

        root = root.child(self.label);

        if self.focused && runtime.ripple.inset_focus_ring_enabled {
            root = root.child(render_inset_focus_ring(
                radius,
                colors.primary,
                runtime.ripple,
            ));
        }

        root.refine_style(&self.style)
    }
}

/// Compose Material 3 1.5.0 `SplitButton` combining a primary action button and trailing menu trigger.
#[derive(IntoElement)]
pub struct M3SplitButton {
    id: ElementId,
    label: SharedString,
    button_size: M3ButtonSize,
    expanded: bool,
    disabled: bool,
    icon: Option<Icon>,
    on_primary_click: Option<ClickHandler>,
    on_trailing_click: Option<ClickHandler>,
    mode_override: Option<M3AppearanceMode>,
}

impl M3SplitButton {
    #[must_use]
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            button_size: M3ButtonSize::Medium,
            expanded: false,
            disabled: false,
            icon: None,
            on_primary_click: None,
            on_trailing_click: None,
            mode_override: None,
        }
    }

    #[must_use]
    pub fn button_size(mut self, size: M3ButtonSize) -> Self {
        self.button_size = size;
        self
    }

    #[must_use]
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    #[must_use]
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    #[must_use]
    pub fn on_primary_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_primary_click = Some(Rc::new(handler));
        self
    }

    #[must_use]
    pub fn on_trailing_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_trailing_click = Some(Rc::new(handler));
        self
    }

    #[must_use]
    pub fn mode(mut self, mode: M3AppearanceMode) -> Self {
        self.mode_override = Some(mode);
        self
    }
}

impl RenderOnce for M3SplitButton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let m3 = self
            .mode_override
            .map(M3ActiveTheme::for_mode)
            .unwrap_or_else(|| M3ActiveTheme::resolve(cx));
        let colors = m3.colors;
        let height = px(self.button_size.height_px());
        let outer_radius = px(m3.shape.full_px);
        let inner_radius = px(m3.shape.extra_small_px);
        let trailing_radius = if self.expanded {
            outer_radius
        } else {
            inner_radius
        };
        let bg = colors.primary;
        let fg = colors.on_primary;
        let leading_id = ElementId::Name(format!("{}-leading", self.id).into());
        let trailing_id = ElementId::Name(format!("{}-trailing", self.id).into());
        let hover_bg = colors.state_layer(bg, fg, m3.state.hover_opacity);
        let chevron = if self.expanded {
            IconName::ChevronUp
        } else {
            IconName::ChevronDown
        };

        let leading_debug = leading_id.clone();
        let trailing_debug = trailing_id.clone();
        let root_debug = self.id.clone();

        h_flex()
            .id(self.id)
            .debug_selector(move || root_debug.to_string())
            .gap(px(2.0))
            .items_center()
            .child(
                h_flex()
                    .id(leading_id)
                    .debug_selector(move || leading_debug.to_string())
                    .h(height)
                    .px(px(16.0))
                    .gap(px(8.0))
                    .items_center()
                    .rounded_l(outer_radius)
                    .rounded_r(inner_radius)
                    .bg(bg)
                    .text_color(fg)
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .when(!self.disabled, |this| {
                        this.cursor_pointer()
                            .hover(move |s| s.bg(hover_bg))
                            .when_some(self.on_primary_click, |el, handler| {
                                el.on_click(move |event, window, cx| handler(event, window, cx))
                            })
                    })
                    .when_some(self.icon, |this, icon| this.child(icon.text_color(fg)))
                    .child(self.label),
            )
            .child(
                h_flex()
                    .id(trailing_id)
                    .debug_selector(move || trailing_debug.to_string())
                    .h(height)
                    .w(px(self.button_size.height_px()))
                    .items_center()
                    .justify_center()
                    .rounded_l(trailing_radius)
                    .rounded_r(outer_radius)
                    .bg(bg)
                    .text_color(fg)
                    .when(!self.disabled, |this| {
                        this.cursor_pointer()
                            .hover(move |s| s.bg(hover_bg))
                            .when_some(self.on_trailing_click, |el, handler| {
                                el.on_click(move |event, window, cx| handler(event, window, cx))
                            })
                    })
                    .child(Icon::new(chevron).text_color(fg)),
            )
    }
}

/// Item descriptor for [`M3ButtonGroup`].
pub struct M3ButtonGroupItem {
    pub id: SharedString,
    pub label: SharedString,
    pub checked: bool,
    pub on_click: Option<ClickHandler>,
}

impl M3ButtonGroupItem {
    #[must_use]
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            checked: false,
            on_click: None,
        }
    }

    #[must_use]
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    #[must_use]
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

/// Compose Material 3 1.5.0 `ButtonGroup` with adaptive width squishing (`compression_limit`)
/// and automatic overflow indicator when items exceed `max_visible`.
#[derive(IntoElement)]
pub struct M3ButtonGroup {
    id: ElementId,
    items: SmallVec<[M3ButtonGroupItem; 8]>,
    connected: bool,
    compression_limit: f32,
    max_visible: usize,
    on_overflow_click: Option<ClickHandler>,
    mode_override: Option<M3AppearanceMode>,
}

impl M3ButtonGroup {
    #[must_use]
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            items: SmallVec::new(),
            connected: true,
            compression_limit: 0.15,
            max_visible: 5,
            on_overflow_click: None,
            mode_override: None,
        }
    }

    #[must_use]
    pub fn item(mut self, item: M3ButtonGroupItem) -> Self {
        self.items.push(item);
        self
    }

    #[must_use]
    pub fn connected(mut self, connected: bool) -> Self {
        self.connected = connected;
        self
    }

    /// Mirrors `Modifier.animateWidth(compressionLimit = ...)` (`0.0..=0.5`).
    #[must_use]
    pub fn compression_limit(mut self, limit: f32) -> Self {
        self.compression_limit = limit.clamp(0.0, 0.5);
        self
    }

    #[must_use]
    pub fn max_visible(mut self, max_visible: usize) -> Self {
        self.max_visible = max_visible.max(1);
        self
    }

    #[must_use]
    pub fn on_overflow_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_overflow_click = Some(Rc::new(handler));
        self
    }

    #[must_use]
    pub fn mode(mut self, mode: M3AppearanceMode) -> Self {
        self.mode_override = Some(mode);
        self
    }

    /// Computes the adaptive width multiplier for item `index` when an adjacent item is checked.
    #[must_use]
    pub fn item_width_scale(&self, index: usize) -> f32 {
        let Some(item) = self.items.get(index) else {
            return 1.0;
        };
        if item.checked {
            1.0 + self.compression_limit
        } else if self.items.iter().any(|candidate| candidate.checked) {
            (1.0 - self.compression_limit * 0.5).max(0.5)
        } else {
            1.0
        }
    }
}

impl RenderOnce for M3ButtonGroup {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let m3 = self
            .mode_override
            .map(M3ActiveTheme::for_mode)
            .unwrap_or_else(|| M3ActiveTheme::resolve(cx));
        let colors = m3.colors;
        let gap = if self.connected { px(2.0) } else { px(8.0) };
        let total = self.items.len();
        let visible_count = total.min(self.max_visible);
        let overflow_count = total.saturating_sub(visible_count);
        let group_prefix = self.id.to_string();
        let scales: Vec<f32> = (0..visible_count)
            .map(|idx| self.item_width_scale(idx))
            .collect();

        let mut row = h_flex().id(self.id).gap(gap).items_center();

        for (idx, item) in self.items.into_iter().take(visible_count).enumerate() {
            let is_first = idx == 0;
            let is_last = idx + 1 == visible_count && overflow_count == 0;
            let base_pad = 16.0 * scales.get(idx).copied().unwrap_or(1.0);
            let outer = px(m3.shape.full_px);
            let inner = px(m3.shape.small_px);
            let bg = if item.checked {
                colors.secondary_container
            } else {
                colors.surface_container_high
            };
            let fg = if item.checked {
                colors.on_secondary_container
            } else {
                colors.on_surface
            };
            let item_id = ElementId::Name(format!("{group_prefix}-{}", item.id).into());
            let item_debug = item_id.clone();
            let hover_bg = colors.state_layer(bg, fg, m3.state.hover_opacity);

            let mut el = h_flex()
                .id(item_id)
                .debug_selector(move || item_debug.to_string())
                .h(px(40.0))
                .px(px(base_pad))
                .items_center()
                .justify_center()
                .cursor_pointer()
                .bg(bg)
                .hover(move |s| s.bg(hover_bg))
                .text_color(fg)
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .when_some(item.on_click, |button, handler| {
                    button.on_click(move |event, window, cx| handler(event, window, cx))
                })
                .child(item.label);

            if !self.connected || item.checked {
                el = el.rounded(if item.checked { px(12.0) } else { outer });
            } else {
                el = el
                    .rounded_l(if is_first { outer } else { inner })
                    .rounded_r(if is_last { outer } else { inner });
            }
            row = row.child(el);
        }

        if overflow_count > 0 {
            let overflow_id = ElementId::Name(format!("{group_prefix}-overflow").into());
            let overflow_debug = overflow_id.clone();
            row = row.child(
                h_flex()
                    .id(overflow_id)
                    .debug_selector(move || overflow_debug.to_string())
                    .h(px(40.0))
                    .px(px(12.0))
                    .gap(px(4.0))
                    .items_center()
                    .cursor_pointer()
                    .rounded_full()
                    .bg(colors.surface_container_highest)
                    .text_color(colors.on_surface_variant)
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .when_some(self.on_overflow_click, |button, handler| {
                        button.on_click(move |event, window, cx| handler(event, window, cx))
                    })
                    .child(Icon::new(IconName::Ellipsis))
                    .child(format!("+{overflow_count}")),
            );
        }

        row
    }
}

// ---------------------------------------------------------------------------
// 5. FloatingToolbar (Horizontal / Vertical, Standard / Vibrant, Overflow + FAB)
// ---------------------------------------------------------------------------

/// Color scheme variant for [`M3FloatingToolbar`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum M3FloatingToolbarColor {
    #[default]
    Standard,
    Vibrant,
}

/// Compose Material 3 1.5.0 `FloatingToolbar` / `HorizontalFloatingToolbar`.
#[derive(IntoElement)]
pub struct M3FloatingToolbar {
    id: ElementId,
    vertical: bool,
    color: M3FloatingToolbarColor,
    actions: SmallVec<[AnyElement; 8]>,
    fab: Option<AnyElement>,
    overflow_count: usize,
    on_overflow_click: Option<ClickHandler>,
    mode_override: Option<M3AppearanceMode>,
}

impl M3FloatingToolbar {
    #[must_use]
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            vertical: false,
            color: M3FloatingToolbarColor::Standard,
            actions: SmallVec::new(),
            fab: None,
            overflow_count: 0,
            on_overflow_click: None,
            mode_override: None,
        }
    }

    #[must_use]
    pub fn vertical(mut self, vertical: bool) -> Self {
        self.vertical = vertical;
        self
    }

    #[must_use]
    pub fn color(mut self, color: M3FloatingToolbarColor) -> Self {
        self.color = color;
        self
    }

    #[must_use]
    pub fn action(mut self, element: impl IntoElement) -> Self {
        self.actions.push(element.into_any_element());
        self
    }

    #[must_use]
    pub fn fab(mut self, element: impl IntoElement) -> Self {
        self.fab = Some(element.into_any_element());
        self
    }

    #[must_use]
    pub fn overflow_count(mut self, count: usize) -> Self {
        self.overflow_count = count;
        self
    }

    #[must_use]
    pub fn on_overflow_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_overflow_click = Some(Rc::new(handler));
        self
    }

    #[must_use]
    pub fn mode(mut self, mode: M3AppearanceMode) -> Self {
        self.mode_override = Some(mode);
        self
    }
}

impl RenderOnce for M3FloatingToolbar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let m3 = self
            .mode_override
            .map(M3ActiveTheme::for_mode)
            .unwrap_or_else(|| M3ActiveTheme::resolve(cx));
        let colors = m3.colors;
        let (bg, fg) = match self.color {
            M3FloatingToolbarColor::Standard => (colors.surface_container, colors.on_surface),
            M3FloatingToolbarColor::Vibrant => {
                (colors.primary_container, colors.on_primary_container)
            }
        };
        let overflow_id = ElementId::Name(format!("{}-overflow", self.id).into());

        let mut bar = if self.vertical {
            v_flex().py(px(8.0)).px(px(6.0))
        } else {
            h_flex().px(px(10.0)).py(px(6.0))
        };

        bar = bar
            .gap(px(6.0))
            .items_center()
            .rounded_full()
            .bg(bg)
            .text_color(fg)
            .border_1()
            .border_color(colors.outline_variant)
            .shadow(m3_elevation_shadow(&colors, 2))
            .children(self.actions);

        if self.overflow_count > 0 {
            bar = bar.child(
                h_flex()
                    .id(overflow_id)
                    .h(px(32.0))
                    .px(px(10.0))
                    .gap(px(4.0))
                    .items_center()
                    .cursor_pointer()
                    .rounded_full()
                    .bg(colors.surface_container_highest)
                    .text_color(colors.on_surface_variant)
                    .text_xs()
                    .when_some(self.on_overflow_click, |button, handler| {
                        button.on_click(move |event, window, cx| handler(event, window, cx))
                    })
                    .child(Icon::new(IconName::Ellipsis))
                    .child(format!("+{}", self.overflow_count)),
            );
        }

        let root_debug = self.id.clone();
        let wrapper = if self.vertical {
            v_flex()
                .id(self.id)
                .debug_selector(move || root_debug.to_string())
                .gap(px(8.0))
                .items_center()
        } else {
            h_flex()
                .id(self.id)
                .debug_selector(move || root_debug.to_string())
                .gap(px(8.0))
                .items_center()
        };

        wrapper.child(bar).when_some(self.fab, |this, fab| this.child(fab))
    }
}

// ---------------------------------------------------------------------------
// 6. WavyLinearProgressIndicator & WavyCircularProgressIndicator
// ---------------------------------------------------------------------------

/// Compose Material 3 1.5.0 `WavyLinearProgressIndicator` (determinate & NaN-safe indeterminate).
#[derive(IntoElement)]
pub struct M3WavyLinearProgressIndicator {
    id: ElementId,
    /// `None` or non-finite (`NaN`/`inf`) renders an indeterminate wave per Compose M3 1.5/1.6.
    progress: Option<f32>,
    phase_rad: f32,
    wavelength_px: f32,
    amplitude_px: f32,
    stroke_px: f32,
    width_px: f32,
    mode_override: Option<M3AppearanceMode>,
}

impl M3WavyLinearProgressIndicator {
    #[must_use]
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            progress: Some(0.65),
            phase_rad: 0.0,
            wavelength_px: 24.0,
            amplitude_px: 3.5,
            stroke_px: 3.0,
            width_px: 240.0,
            mode_override: None,
        }
    }

    #[must_use]
    pub fn progress(mut self, progress: Option<f32>) -> Self {
        self.progress = progress.filter(|v| v.is_finite()).map(|v| v.clamp(0.0, 1.0));
        self
    }

    #[must_use]
    pub fn phase_rad(mut self, phase_rad: f32) -> Self {
        self.phase_rad = phase_rad;
        self
    }

    #[must_use]
    pub fn wavelength_px(mut self, wavelength_px: f32) -> Self {
        self.wavelength_px = wavelength_px.max(8.0);
        self
    }

    #[must_use]
    pub fn amplitude_px(mut self, amplitude_px: f32) -> Self {
        self.amplitude_px = amplitude_px.clamp(0.0, 12.0);
        self
    }

    #[must_use]
    pub fn width_px(mut self, width_px: f32) -> Self {
        self.width_px = width_px.max(48.0);
        self
    }

    #[must_use]
    pub fn mode(mut self, mode: M3AppearanceMode) -> Self {
        self.mode_override = Some(mode);
        self
    }

    /// Returns `true` when operating in indeterminate mode (`None` or originally non-finite).
    #[must_use]
    pub const fn is_indeterminate(&self) -> bool {
        self.progress.is_none()
    }
}

impl RenderOnce for M3WavyLinearProgressIndicator {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let m3 = self
            .mode_override
            .map(M3ActiveTheme::for_mode)
            .unwrap_or_else(|| M3ActiveTheme::resolve(cx));
        let active_color = m3.colors.primary;
        let track_color = m3.colors.secondary_container;
        let progress = self.progress;
        let phase = self.phase_rad;
        let wavelength = self.wavelength_px;
        let amp = self.amplitude_px;
        let stroke = px(self.stroke_px);
        let height_px = (amp * 2.0 + self.stroke_px * 2.0 + 4.0).max(16.0);

        let lin_debug = self.id.clone();
        div()
            .id(self.id)
            .debug_selector(move || lin_debug.to_string())
            .w(px(self.width_px))
            .h(px(height_px))
            .child(
                canvas(
                    move |bounds, _, _| {
                        let w = f32::from(bounds.size.width).max(1.0);
                        let mid_y =
                            f32::from(bounds.origin.y) + f32::from(bounds.size.height) * 0.5;
                        let start_x = f32::from(bounds.origin.x);
                        let (active_start, active_end) = match progress {
                            Some(p) => (0.0, w * p),
                            None => {
                                let offset = ((phase.sin() * 0.5) + 0.5) * (w * 0.55);
                                (offset, (offset + w * 0.45).min(w))
                            }
                        };

                        let mut track_builder = PathBuilder::stroke(stroke);
                        track_builder.move_to(point(px(start_x), px(mid_y)));
                        track_builder.line_to(point(px(start_x + w), px(mid_y)));

                        let mut wave_builder = PathBuilder::stroke(stroke);
                        let steps = ((active_end - active_start) / 2.5).ceil().max(4.0) as usize;
                        for idx in 0..=steps {
                            let local_x = active_start
                                + (active_end - active_start) * (idx as f32 / steps as f32);
                            let y = mid_y
                                + ((local_x / wavelength) * std::f32::consts::TAU + phase).sin()
                                    * amp;
                            let pt = point(px(start_x + local_x), px(y));
                            if idx == 0 {
                                wave_builder.move_to(pt);
                            } else {
                                wave_builder.line_to(pt);
                            }
                        }

                        (track_builder.build().ok(), wave_builder.build().ok())
                    },
                    move |_bounds, (track_opt, wave_opt), window, _| {
                        if let Some(track) = track_opt {
                            window.paint_path(track, track_color);
                        }
                        if let Some(wave) = wave_opt {
                            window.paint_path(wave, active_color);
                        }
                    },
                )
                .size_full(),
            )
    }
}

/// Compose Material 3 1.5.0 `WavyCircularProgressIndicator`.
#[derive(IntoElement)]
pub struct M3WavyCircularProgressIndicator {
    id: ElementId,
    progress: Option<f32>,
    phase_rad: f32,
    size_px: f32,
    stroke_px: f32,
    mode_override: Option<M3AppearanceMode>,
}

impl M3WavyCircularProgressIndicator {
    #[must_use]
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            progress: Some(0.72),
            phase_rad: 0.0,
            size_px: 48.0,
            stroke_px: 3.2,
            mode_override: None,
        }
    }

    #[must_use]
    pub fn progress(mut self, progress: Option<f32>) -> Self {
        self.progress = progress.filter(|v| v.is_finite()).map(|v| v.clamp(0.0, 1.0));
        self
    }

    #[must_use]
    pub fn phase_rad(mut self, phase_rad: f32) -> Self {
        self.phase_rad = phase_rad;
        self
    }

    #[must_use]
    pub fn size_px(mut self, size_px: f32) -> Self {
        self.size_px = size_px.max(24.0);
        self
    }

    #[must_use]
    pub fn mode(mut self, mode: M3AppearanceMode) -> Self {
        self.mode_override = Some(mode);
        self
    }
}

impl RenderOnce for M3WavyCircularProgressIndicator {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let m3 = self
            .mode_override
            .map(M3ActiveTheme::for_mode)
            .unwrap_or_else(|| M3ActiveTheme::resolve(cx));
        let active_color = m3.colors.primary;
        let track_color = m3.colors.secondary_container;
        let progress = self.progress;
        let phase = self.phase_rad;
        let stroke = px(self.stroke_px);

        let debug_id = self.id.clone();
        div()
            .id(self.id)
            .debug_selector(move || debug_id.to_string())
            .w(px(self.size_px))
            .h(px(self.size_px))
            .child(
                canvas(
                    move |bounds, _, _| {
                        let w = f32::from(bounds.size.width);
                        let h = f32::from(bounds.size.height);
                        let cx = f32::from(bounds.origin.x) + w * 0.5;
                        let cy = f32::from(bounds.origin.y) + h * 0.5;
                        let radius = (w.min(h) * 0.5 - 5.0).max(6.0);

                        let mut track = PathBuilder::stroke(stroke);
                        for idx in 0..=48 {
                            let theta = (idx as f32 / 48.0) * std::f32::consts::TAU;
                            let pt =
                                point(px(cx + radius * theta.cos()), px(cy + radius * theta.sin()));
                            if idx == 0 {
                                track.move_to(pt);
                            } else {
                                track.line_to(pt);
                            }
                        }

                        let (arc_start, arc_span) = match progress {
                            Some(p) => (-std::f32::consts::FRAC_PI_2, p * std::f32::consts::TAU),
                            None => (phase, std::f32::consts::TAU * 0.62),
                        };
                        let mut wave = PathBuilder::stroke(stroke);
                        let steps = 48usize;
                        for idx in 0..=steps {
                            let theta = arc_start + arc_span * (idx as f32 / steps as f32);
                            let r = radius + (theta * 8.0 + phase).sin() * 2.0;
                            let pt = point(px(cx + r * theta.cos()), px(cy + r * theta.sin()));
                            if idx == 0 {
                                wave.move_to(pt);
                            } else {
                                wave.line_to(pt);
                            }
                        }
                        (track.build().ok(), wave.build().ok())
                    },
                    move |_bounds, (track_opt, wave_opt), window, _| {
                        if let Some(track) = track_opt {
                            window.paint_path(track, track_color);
                        }
                        if let Some(wave) = wave_opt {
                            window.paint_path(wave, active_color);
                        }
                    },
                )
                .size_full(),
            )
    }
}

// ---------------------------------------------------------------------------
// 7. Segmented ListItem (Single / First / Middle / Last + vertical alignment)
// ---------------------------------------------------------------------------

/// Container segment position for [`M3SegmentedListItem`] (16dp outer / 4dp inner corners).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum M3SegmentPosition {
    #[default]
    Single,
    First,
    Middle,
    Last,
}

/// Content vertical alignment for [`M3SegmentedListItem`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum M3ListVerticalAlignment {
    Top,
    #[default]
    Center,
    Bottom,
}

/// Compose Material 3 1.5.0 segmented `ListItem` / `InteractiveListItem`.
#[derive(IntoElement)]
pub struct M3SegmentedListItem {
    id: ElementId,
    headline: SharedString,
    supporting_text: Option<SharedString>,
    overline: Option<SharedString>,
    position: M3SegmentPosition,
    vertical_alignment: M3ListVerticalAlignment,
    selected: bool,
    leading: Option<AnyElement>,
    trailing: Option<AnyElement>,
    on_click: Option<ClickHandler>,
    mode_override: Option<M3AppearanceMode>,
}

impl M3SegmentedListItem {
    #[must_use]
    pub fn new(id: impl Into<ElementId>, headline: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            headline: headline.into(),
            supporting_text: None,
            overline: None,
            position: M3SegmentPosition::Single,
            vertical_alignment: M3ListVerticalAlignment::Center,
            selected: false,
            leading: None,
            trailing: None,
            on_click: None,
            mode_override: None,
        }
    }

    #[must_use]
    pub fn supporting_text(mut self, text: impl Into<SharedString>) -> Self {
        self.supporting_text = Some(text.into());
        self
    }

    #[must_use]
    pub fn overline(mut self, text: impl Into<SharedString>) -> Self {
        self.overline = Some(text.into());
        self
    }

    #[must_use]
    pub fn position(mut self, position: M3SegmentPosition) -> Self {
        self.position = position;
        self
    }

    #[must_use]
    pub fn vertical_alignment(mut self, alignment: M3ListVerticalAlignment) -> Self {
        self.vertical_alignment = alignment;
        self
    }

    #[must_use]
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    #[must_use]
    pub fn leading(mut self, el: impl IntoElement) -> Self {
        self.leading = Some(el.into_any_element());
        self
    }

    #[must_use]
    pub fn trailing(mut self, el: impl IntoElement) -> Self {
        self.trailing = Some(el.into_any_element());
        self
    }

    #[must_use]
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    #[must_use]
    pub fn mode(mut self, mode: M3AppearanceMode) -> Self {
        self.mode_override = Some(mode);
        self
    }
}

impl RenderOnce for M3SegmentedListItem {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let m3 = self
            .mode_override
            .map(M3ActiveTheme::for_mode)
            .unwrap_or_else(|| M3ActiveTheme::resolve(cx));
        let colors = m3.colors;
        let outer = px(m3.shape.large_px);
        let inner = px(m3.shape.extra_small_px);
        let (top_r, bottom_r) = match self.position {
            M3SegmentPosition::Single => (outer, outer),
            M3SegmentPosition::First => (outer, inner),
            M3SegmentPosition::Middle => (inner, inner),
            M3SegmentPosition::Last => (inner, outer),
        };
        let bg = if self.selected {
            colors.secondary_container
        } else {
            colors.surface_container_low
        };
        let fg = if self.selected {
            colors.on_secondary_container
        } else {
            colors.on_surface
        };

        let seg_debug = self.id.clone();
        let mut row = h_flex()
            .id(self.id)
            .debug_selector(move || seg_debug.to_string())
            .w_full()
            .min_h(px(56.0))
            .px(px(16.0))
            .py(px(10.0))
            .gap(px(12.0))
            .rounded_t(top_r)
            .rounded_b(bottom_r)
            .bg(bg)
            .text_color(fg);

        row = match self.vertical_alignment {
            M3ListVerticalAlignment::Top => row.items_start(),
            M3ListVerticalAlignment::Center => row.items_center(),
            M3ListVerticalAlignment::Bottom => row.items_end(),
        };

        if let Some(handler) = self.on_click {
            row = row
                .cursor_pointer()
                .hover(move |s| s.bg(colors.state_layer(bg, fg, m3.state.hover_opacity)))
                .on_click(move |event, window, cx| handler(event, window, cx));
        }

        row.when_some(self.leading, |this, el| this.child(el))
            .child(
                v_flex()
                    .flex_1()
                    .gap(px(2.0))
                    .when_some(self.overline, |this, overline| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(colors.on_surface_variant)
                                .child(overline),
                        )
                    })
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .child(self.headline),
                    )
                    .when_some(self.supporting_text, |this, supporting| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(colors.on_surface_variant)
                                .child(supporting),
                        )
                    }),
            )
            .when_some(self.trailing, |this, el| this.child(el))
    }
}

// ---------------------------------------------------------------------------
// 8. Stateful Slider & RangeSlider (Compose M3 1.5.0 SliderState / RangeSliderState)
// ---------------------------------------------------------------------------

/// Stateful single-value slider model matching Compose Material 3 `1.5.0` `SliderState`
/// with exact integer-tick discrete snapping (eliminating floating-point drift).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct M3SliderState {
    pub value: f32,
    pub min: f32,
    pub max: f32,
    /// Number of interior discrete steps (`0` = continuous).
    pub steps: u32,
}

impl M3SliderState {
    #[must_use]
    pub fn new(value: f32, min: f32, max: f32, steps: u32) -> Self {
        let lo = min.min(max);
        let hi = max.max(min + f32::EPSILON);
        let mut state = Self {
            value,
            min: lo,
            max: hi,
            steps,
        };
        state.set_value(value);
        state
    }

    /// Snaps `raw` cleanly to exact integer ticks when `steps > 0` (Compose M3 1.5.0 fix).
    #[must_use]
    pub fn snap(&self, raw: f32) -> f32 {
        let clamped = if raw.is_finite() {
            raw.clamp(self.min, self.max)
        } else {
            self.min
        };
        if self.steps == 0 {
            return clamped;
        }
        let segments = self.steps + 1;
        let normalized = (clamped - self.min) / (self.max - self.min);
        let tick = (normalized * segments as f32).round() as u32;
        let tick_clamped = tick.min(segments);
        self.min + (self.max - self.min) * (tick_clamped as f32 / segments as f32)
    }

    pub fn set_value(&mut self, next: f32) -> f32 {
        self.value = self.snap(next);
        self.value
    }

    #[must_use]
    pub fn fraction(&self) -> f32 {
        ((self.value - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
    }

    /// Steps value by `delta_ticks` (`+1` / `-1` for keyboard ArrowRight / ArrowLeft).
    pub fn step_by(&mut self, delta_ticks: i32) -> f32 {
        let segments = if self.steps > 0 {
            (self.steps + 1) as f32
        } else {
            20.0
        };
        let step_size = (self.max - self.min) / segments;
        self.set_value(self.value + step_size * delta_ticks as f32)
    }
}

/// Stateful range slider model matching Compose Material 3 `1.5.0` `RangeSliderState`
/// (`start_value` / `end_value` replacing legacy `activeRangeStart` / `activeRangeEnd`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct M3RangeSliderState {
    pub start_value: f32,
    pub end_value: f32,
    pub min: f32,
    pub max: f32,
    pub steps: u32,
}

impl M3RangeSliderState {
    #[must_use]
    pub fn new(start_value: f32, end_value: f32, min: f32, max: f32, steps: u32) -> Self {
        let lo = min.min(max);
        let hi = max.max(min + f32::EPSILON);
        let helper = M3SliderState::new(start_value, lo, hi, steps);
        let a = helper.snap(start_value);
        let b = helper.snap(end_value);
        Self {
            start_value: a.min(b),
            end_value: a.max(b),
            min: lo,
            max: hi,
            steps,
        }
    }

    pub fn set_range(&mut self, start: f32, end: f32) -> (f32, f32) {
        let helper = M3SliderState::new(start, self.min, self.max, self.steps);
        let a = helper.snap(start);
        let b = helper.snap(end);
        self.start_value = a.min(b);
        self.end_value = a.max(b);
        (self.start_value, self.end_value)
    }

    /// Updates the nearest thumb when tapping the track, even when thumbs overlap (1.5.0 fix).
    pub fn tap_fraction(&mut self, fraction: f32) -> (f32, f32) {
        let target = self.min + (self.max - self.min) * fraction.clamp(0.0, 1.0);
        let dist_start = (target - self.start_value).abs();
        let dist_end = (target - self.end_value).abs();
        if (self.start_value - self.end_value).abs() < f32::EPSILON {
            if target >= self.end_value {
                self.set_range(self.start_value, target)
            } else {
                self.set_range(target, self.end_value)
            }
        } else if dist_start <= dist_end {
            self.set_range(target, self.end_value)
        } else {
            self.set_range(self.start_value, target)
        }
    }
}

/// Compose Material 3 1.5.0 stateful `Slider(state = SliderState, onValueChange = ...)`.
#[derive(IntoElement)]
pub struct M3StatefulSlider {
    id: ElementId,
    label: Option<SharedString>,
    state: Entity<M3SliderState>,
    on_value_change: Option<FloatChangeHandler>,
    on_value_change_finished: Option<FloatChangeHandler>,
    focused: bool,
    mode_override: Option<M3AppearanceMode>,
}

impl M3StatefulSlider {
    #[must_use]
    pub fn new(id: impl Into<ElementId>, state: Entity<M3SliderState>) -> Self {
        Self {
            id: id.into(),
            label: None,
            state,
            on_value_change: None,
            on_value_change_finished: None,
            focused: false,
            mode_override: None,
        }
    }

    #[must_use]
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    #[must_use]
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    #[must_use]
    pub fn on_value_change(
        mut self,
        handler: impl Fn(f32, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_value_change = Some(Rc::new(handler));
        self
    }

    #[must_use]
    pub fn on_value_change_finished(
        mut self,
        handler: impl Fn(f32, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_value_change_finished = Some(Rc::new(handler));
        self
    }

    #[must_use]
    pub fn mode(mut self, mode: M3AppearanceMode) -> Self {
        self.mode_override = Some(mode);
        self
    }
}

impl RenderOnce for M3StatefulSlider {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let m3 = self
            .mode_override
            .map(M3ActiveTheme::for_mode)
            .unwrap_or_else(|| M3ActiveTheme::resolve(cx));
        let runtime = active_m3_expressive_config(cx);
        let colors = m3.colors;
        let snapshot = self.state.read(cx).clone();
        let fraction = snapshot.fraction();
        let prefix = self.id.to_string();
        let dec_id = ElementId::Name(format!("{prefix}-dec").into());
        let inc_id = ElementId::Name(format!("{prefix}-inc").into());
        let state_dec = self.state.clone();
        let state_inc = self.state.clone();
        let state_key = self.state.clone();
        let on_change_dec = self.on_value_change.clone();
        let on_change_inc = self.on_value_change.clone();
        let on_change_key = self.on_value_change.clone();
        let on_finish_dec = self.on_value_change_finished.clone();
        let on_finish_inc = self.on_value_change_finished.clone();

        let slider_debug = self.id.clone();
        let dec_debug = dec_id.clone();
        let inc_debug = inc_id.clone();
        v_flex()
            .id(self.id)
            .debug_selector(move || slider_debug.to_string())
            .w_full()
            .gap(px(6.0))
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                let delta = match event.keystroke.key.as_str() {
                    "left" | "down" => Some(-1),
                    "right" | "up" => Some(1),
                    _ => None,
                };
                if let Some(d) = delta {
                    let next = state_key.update(cx, |s, cx| {
                        let v = s.step_by(d);
                        cx.notify();
                        v
                    });
                    if let Some(ref handler) = on_change_key {
                        handler(next, window, cx);
                    }
                }
            })
            .when_some(self.label, |this, label| {
                this.child(
                    h_flex()
                        .justify_between()
                        .text_xs()
                        .text_color(colors.on_surface_variant)
                        .child(label)
                        .child(format!("{:.2}", snapshot.value)),
                )
            })
            .child(
                h_flex()
                    .relative()
                    .w_full()
                    .h(px(32.0))
                    .gap(px(8.0))
                    .items_center()
                    .child(
                        h_flex()
                            .id(dec_id)
                            .debug_selector(move || dec_debug.to_string())
                            .w(px(28.0))
                            .h(px(28.0))
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .rounded_full()
                            .bg(colors.surface_container_highest)
                            .text_color(colors.on_surface)
                            .on_click(move |_, window, cx| {
                                let next = state_dec.update(cx, |s, cx| {
                                    let v = s.step_by(-1);
                                    cx.notify();
                                    v
                                });
                                if let Some(ref handler) = on_change_dec {
                                    handler(next, window, cx);
                                }
                                if let Some(ref handler) = on_finish_dec {
                                    handler(next, window, cx);
                                }
                            })
                            .child(Icon::new(IconName::Minus)),
                    )
                    .child(
                        div()
                            .relative()
                            .flex_1()
                            .h(px(12.0))
                            .rounded_full()
                            .bg(colors.secondary_container)
                            .overflow_hidden()
                            .child(
                                div()
                                    .h_full()
                                    .w(gpui::relative(fraction))
                                    .rounded_full()
                                    .bg(colors.primary),
                            ),
                    )
                    .child(
                        h_flex()
                            .id(inc_id)
                            .debug_selector(move || inc_debug.to_string())
                            .w(px(28.0))
                            .h(px(28.0))
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .rounded_full()
                            .bg(colors.surface_container_highest)
                            .text_color(colors.on_surface)
                            .on_click(move |_, window, cx| {
                                let next = state_inc.update(cx, |s, cx| {
                                    let v = s.step_by(1);
                                    cx.notify();
                                    v
                                });
                                if let Some(ref handler) = on_change_inc {
                                    handler(next, window, cx);
                                }
                                if let Some(ref handler) = on_finish_inc {
                                    handler(next, window, cx);
                                }
                            })
                            .child(Icon::new(IconName::Plus)),
                    )
                    .when(
                        self.focused && runtime.ripple.inset_focus_ring_enabled,
                        |this| {
                            this.child(render_inset_focus_ring(
                                px(m3.shape.full_px),
                                colors.primary,
                                runtime.ripple,
                            ))
                        },
                    ),
            )
    }
}

/// Compose Material 3 1.5.0 stateful `RangeSlider(state = RangeSliderState, onValueChange = ...)`.
#[derive(IntoElement)]
pub struct M3StatefulRangeSlider {
    id: ElementId,
    label: Option<SharedString>,
    state: Entity<M3RangeSliderState>,
    on_value_change: Option<RangeChangeHandler>,
    mode_override: Option<M3AppearanceMode>,
}

impl M3StatefulRangeSlider {
    #[must_use]
    pub fn new(id: impl Into<ElementId>, state: Entity<M3RangeSliderState>) -> Self {
        Self {
            id: id.into(),
            label: None,
            state,
            on_value_change: None,
            mode_override: None,
        }
    }

    #[must_use]
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    #[must_use]
    pub fn on_value_change(
        mut self,
        handler: impl Fn(f32, f32, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_value_change = Some(Rc::new(handler));
        self
    }

    #[must_use]
    pub fn mode(mut self, mode: M3AppearanceMode) -> Self {
        self.mode_override = Some(mode);
        self
    }
}

impl RenderOnce for M3StatefulRangeSlider {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let m3 = self
            .mode_override
            .map(M3ActiveTheme::for_mode)
            .unwrap_or_else(|| M3ActiveTheme::resolve(cx));
        let colors = m3.colors;
        let snapshot = self.state.read(cx).clone();
        let span = (snapshot.max - snapshot.min).max(f32::EPSILON);
        let start_frac = ((snapshot.start_value - snapshot.min) / span).clamp(0.0, 1.0);
        let end_frac = ((snapshot.end_value - snapshot.min) / span).clamp(start_frac, 1.0);
        let widen_id = ElementId::Name(format!("{}-widen", self.id).into());
        let state_widen = self.state.clone();
        let on_change = self.on_value_change;

        let range_debug = self.id.clone();
        v_flex()
            .id(self.id)
            .debug_selector(move || range_debug.to_string())
            .w_full()
            .gap(px(6.0))
            .when_some(self.label, |this, label| {
                this.child(
                    h_flex()
                        .justify_between()
                        .text_xs()
                        .text_color(colors.on_surface_variant)
                        .child(label)
                        .child(format!(
                            "{:.1}..{:.1}",
                            snapshot.start_value, snapshot.end_value
                        )),
                )
            })
            .child(
                h_flex()
                    .w_full()
                    .h(px(28.0))
                    .gap(px(8.0))
                    .items_center()
                    .child(
                        div()
                            .relative()
                            .flex_1()
                            .h(px(12.0))
                            .rounded_full()
                            .bg(colors.secondary_container)
                            .overflow_hidden()
                            .child(
                                div()
                                    .absolute()
                                    .top_0()
                                    .bottom_0()
                                    .left(gpui::relative(start_frac))
                                    .w(gpui::relative((end_frac - start_frac).max(0.04)))
                                    .rounded_full()
                                    .bg(colors.primary),
                            ),
                    )
                    .child(
                        h_flex()
                            .id(widen_id)
                            .h(px(26.0))
                            .px(px(10.0))
                            .items_center()
                            .cursor_pointer()
                            .rounded_full()
                            .bg(colors.surface_container_highest)
                            .text_color(colors.on_surface)
                            .text_xs()
                            .on_click(move |_, window, cx| {
                                let (a, b) = state_widen.update(cx, |s, cx| {
                                    let next = s.tap_fraction(0.85);
                                    cx.notify();
                                    next
                                });
                                if let Some(ref handler) = on_change {
                                    handler(a, b, window, cx);
                                }
                            })
                            .child("Range"),
                    ),
            )
    }
}

// ---------------------------------------------------------------------------
// 9. Expressive TextField, Expressive MenuItem, Scrim & BottomSheetAnchor
// ---------------------------------------------------------------------------

/// Label placement for [`M3ExpressiveTextField`] (`TextFieldLabelPosition.Inside` / `Cutout`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum M3LabelPosition {
    #[default]
    Inside,
    Cutout,
}

/// Secure obfuscation policy mirroring Compose M3 `1.5.0` (`System` default).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum M3TextObfuscationMode {
    #[default]
    System,
    RevealLastTyped,
    Visible,
}

/// Compose Material 3 1.5.0 expressive `TextField` / `OutlinedTextField` / `SecureTextField`.
#[derive(IntoElement)]
pub struct M3ExpressiveTextField {
    id: ElementId,
    label: SharedString,
    value: SharedString,
    supporting_text: Option<SharedString>,
    outlined: bool,
    rounded: bool,
    label_position: M3LabelPosition,
    secure: bool,
    obfuscation_mode: M3TextObfuscationMode,
    error: bool,
    mode_override: Option<M3AppearanceMode>,
}

impl M3ExpressiveTextField {
    #[must_use]
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value: SharedString::default(),
            supporting_text: None,
            outlined: false,
            rounded: true,
            label_position: M3LabelPosition::Inside,
            secure: false,
            obfuscation_mode: M3TextObfuscationMode::System,
            error: false,
            mode_override: None,
        }
    }

    #[must_use]
    pub fn value(mut self, value: impl Into<SharedString>) -> Self {
        self.value = value.into();
        self
    }

    #[must_use]
    pub fn supporting_text(mut self, text: impl Into<SharedString>) -> Self {
        self.supporting_text = Some(text.into());
        self
    }

    #[must_use]
    pub fn outlined(mut self, outlined: bool) -> Self {
        self.outlined = outlined;
        self
    }

    #[must_use]
    pub fn rounded(mut self, rounded: bool) -> Self {
        self.rounded = rounded;
        self
    }

    #[must_use]
    pub fn label_position(mut self, pos: M3LabelPosition) -> Self {
        self.label_position = pos;
        self
    }

    #[must_use]
    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    #[must_use]
    pub fn obfuscation_mode(mut self, mode: M3TextObfuscationMode) -> Self {
        self.obfuscation_mode = mode;
        self
    }

    #[must_use]
    pub fn error(mut self, error: bool) -> Self {
        self.error = error;
        self
    }

    #[must_use]
    pub fn mode(mut self, mode: M3AppearanceMode) -> Self {
        self.mode_override = Some(mode);
        self
    }

    /// Formats displayed value according to `secure` and `obfuscation_mode` (1.5.0 `System` default).
    #[must_use]
    pub fn display_value(&self) -> SharedString {
        if !self.secure || self.obfuscation_mode == M3TextObfuscationMode::Visible {
            return self.value.clone();
        }
        let chars: Vec<char> = self.value.chars().collect();
        if chars.is_empty() {
            return SharedString::default();
        }
        match self.obfuscation_mode {
            M3TextObfuscationMode::System => "•".repeat(chars.len()).into(),
            M3TextObfuscationMode::RevealLastTyped => {
                let mut out = "•".repeat(chars.len().saturating_sub(1));
                if let Some(&last) = chars.last() {
                    out.push(last);
                }
                out.into()
            }
            M3TextObfuscationMode::Visible => self.value.clone(),
        }
    }
}

impl RenderOnce for M3ExpressiveTextField {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let m3 = self
            .mode_override
            .map(M3ActiveTheme::for_mode)
            .unwrap_or_else(|| M3ActiveTheme::resolve(cx));
        let colors = m3.colors;
        let display = self.display_value();
        let radius = if self.rounded {
            px(m3.shape.large_px)
        } else {
            px(m3.shape.extra_small_px)
        };
        let border = if self.error {
            colors.error
        } else if self.outlined {
            colors.outline
        } else {
            colors.outline_variant
        };
        let bg = if self.outlined {
            transparent_black()
        } else {
            colors.surface_container_highest
        };
        let label_color = if self.error {
            colors.error
        } else {
            colors.on_surface_variant
        };

        let field_debug = self.id.clone();
        v_flex()
            .id(self.id)
            .debug_selector(move || field_debug.to_string())
            .w_full()
            .gap(px(4.0))
            .when(self.label_position == M3LabelPosition::Cutout, |this| {
                this.child(
                    div()
                        .px(px(8.0))
                        .text_xs()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(label_color)
                        .child(self.label.clone()),
                )
            })
            .child(
                v_flex()
                    .w_full()
                    .px(px(14.0))
                    .py(px(10.0))
                    .gap(px(2.0))
                    .rounded(radius)
                    .bg(bg)
                    .border_1()
                    .border_color(border)
                    .when(self.label_position == M3LabelPosition::Inside, |this| {
                        this.child(div().text_xs().text_color(label_color).child(self.label))
                    })
                    .child(
                        div()
                            .text_sm()
                            .text_color(colors.on_surface)
                            .child(if display.is_empty() {
                                SharedString::from(" ")
                            } else {
                                display
                            }),
                    ),
            )
            .when_some(self.supporting_text, |this, supporting| {
                this.child(
                    div()
                        .px(px(12.0))
                        .text_xs()
                        .text_color(label_color)
                        .child(supporting),
                )
            })
    }
}

/// Selection semantics for [`M3ExpressiveMenuItem`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum M3MenuItemSelectionKind {
    #[default]
    Selectable,
    Checkable,
}

/// Anchor-relative submenu placement (`DropdownMenuPopupPositionProvider`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum M3PopupAnchor {
    #[default]
    BelowStart,
    BelowEnd,
    RightCascading,
    LeftCascading,
}

/// Compose Material 3 1.5.0 `SelectableDropdownMenuItem` & `CheckableDropdownMenuItem`.
#[derive(IntoElement)]
pub struct M3ExpressiveMenuItem {
    id: ElementId,
    label: SharedString,
    supporting_text: Option<SharedString>,
    selection_kind: M3MenuItemSelectionKind,
    selected: bool,
    vibrant: bool,
    has_submenu: bool,
    popup_anchor: M3PopupAnchor,
    on_click: Option<ClickHandler>,
    mode_override: Option<M3AppearanceMode>,
}

impl M3ExpressiveMenuItem {
    #[must_use]
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            supporting_text: None,
            selection_kind: M3MenuItemSelectionKind::Selectable,
            selected: false,
            vibrant: false,
            has_submenu: false,
            popup_anchor: M3PopupAnchor::BelowStart,
            on_click: None,
            mode_override: None,
        }
    }

    #[must_use]
    pub fn selection_kind(mut self, kind: M3MenuItemSelectionKind) -> Self {
        self.selection_kind = kind;
        self
    }

    #[must_use]
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    #[must_use]
    pub fn supporting_text(mut self, text: impl Into<SharedString>) -> Self {
        self.supporting_text = Some(text.into());
        self
    }

    #[must_use]
    pub fn vibrant(mut self, vibrant: bool) -> Self {
        self.vibrant = vibrant;
        self
    }

    #[must_use]
    pub fn has_submenu(mut self, has_submenu: bool, anchor: M3PopupAnchor) -> Self {
        self.has_submenu = has_submenu;
        self.popup_anchor = anchor;
        self
    }

    #[must_use]
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    #[must_use]
    pub fn mode(mut self, mode: M3AppearanceMode) -> Self {
        self.mode_override = Some(mode);
        self
    }
}

impl RenderOnce for M3ExpressiveMenuItem {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let m3 = self
            .mode_override
            .map(M3ActiveTheme::for_mode)
            .unwrap_or_else(|| M3ActiveTheme::resolve(cx));
        let colors = m3.colors;
        let (bg, fg) = if self.selected {
            if self.vibrant {
                (colors.tertiary_container, colors.on_tertiary_container)
            } else {
                (colors.secondary_container, colors.on_secondary_container)
            }
        } else {
            (colors.surface_container, colors.on_surface)
        };

        let indicator: AnyElement = match self.selection_kind {
            M3MenuItemSelectionKind::Checkable => div()
                .w(px(18.0))
                .h(px(18.0))
                .rounded(px(4.0))
                .border_1()
                .border_color(if self.selected { colors.primary } else { colors.outline })
                .bg(if self.selected {
                    colors.primary
                } else {
                    transparent_black()
                })
                .flex()
                .items_center()
                .justify_center()
                .when(self.selected, |this| {
                    this.child(Icon::new(IconName::Check).text_color(colors.on_primary))
                })
                .into_any_element(),
            M3MenuItemSelectionKind::Selectable => div()
                .w(px(18.0))
                .h(px(18.0))
                .rounded_full()
                .border_2()
                .border_color(if self.selected { colors.primary } else { colors.outline })
                .flex()
                .items_center()
                .justify_center()
                .when(self.selected, |this| {
                    this.child(div().w(px(8.0)).h(px(8.0)).rounded_full().bg(colors.primary))
                })
                .into_any_element(),
        };

        let menu_debug = self.id.clone();
        let hover_bg = colors.state_layer(bg, fg, m3.state.hover_opacity);
        h_flex()
            .id(self.id)
            .debug_selector(move || menu_debug.to_string())
            .w_full()
            .min_h(px(44.0))
            .px(px(12.0))
            .py(px(8.0))
            .gap(px(10.0))
            .items_center()
            .cursor_pointer()
            .rounded(px(m3.shape.medium_px))
            .bg(bg)
            .hover(move |s| s.bg(hover_bg))
            .text_color(fg)
            .when_some(self.on_click, |button, handler| {
                button.on_click(move |event, window, cx| handler(event, window, cx))
            })
            .child(indicator)
            .child(
                v_flex()
                    .flex_1()
                    .gap(px(2.0))
                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(self.label))
                    .when_some(self.supporting_text, |this, supporting| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(colors.on_surface_variant)
                                .child(supporting),
                        )
                    }),
            )
            .when(self.has_submenu, |this| {
                this.child(Icon::new(IconName::ChevronRight).text_color(fg))
            })
    }
}

/// Deterministic bottom-sheet anchor state matching Compose Material 3 `1.5.0`
/// (explicit anchors; never heuristically drops `PartiallyExpanded`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum M3BottomSheetAnchor {
    Hidden,
    #[default]
    PartiallyExpanded,
    Expanded,
}

impl M3BottomSheetAnchor {
    #[must_use]
    pub const fn height_fraction(self) -> f32 {
        match self {
            Self::Hidden => 0.0,
            Self::PartiallyExpanded => 0.5,
            Self::Expanded => 1.0,
        }
    }
}

/// Compose Material 3 1.5.0 standalone decoupled `Scrim` overlay component.
#[derive(IntoElement)]
pub struct M3Scrim {
    id: ElementId,
    visible: bool,
    opacity: f32,
    on_dismiss: Option<ClickHandler>,
}

impl M3Scrim {
    #[must_use]
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            visible: true,
            opacity: 0.32,
            on_dismiss: None,
        }
    }

    #[must_use]
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    #[must_use]
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 0.9);
        self
    }

    #[must_use]
    pub fn on_dismiss(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_dismiss = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for M3Scrim {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        if !self.visible {
            return div().into_any_element();
        }
        let mut layer = div()
            .id(self.id)
            .absolute()
            .inset_0()
            .bg(hsla(0.0, 0.0, 0.0, self.opacity));
        if let Some(handler) = self.on_dismiss {
            layer = layer.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                handler(&ClickEvent::default(), window, cx);
            });
        }
        layer.into_any_element()
    }
}

// ---------------------------------------------------------------------------
// 10. Expressive Time Picker & ScrollField (Compose M3 1.5.0 + Parametric Prototyping Flagship)
// ---------------------------------------------------------------------------

/// Compose Material 3 1.5.0 `ScrollField` wheel/stepper numeric selector.
#[derive(IntoElement)]
pub struct M3ScrollField {
    id: ElementId,
    label: SharedString,
    value: i32,
    min: i32,
    max: i32,
    pad_two_digits: bool,
    corner_radius_px: Option<f32>,
    on_change: Option<IntChangeHandler>,
    mode_override: Option<M3AppearanceMode>,
}

impl M3ScrollField {
    #[must_use]
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>, value: i32) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value,
            min: 0,
            max: 59,
            pad_two_digits: true,
            corner_radius_px: None,
            on_change: None,
            mode_override: None,
        }
    }

    #[must_use]
    pub fn range(mut self, min: i32, max: i32) -> Self {
        self.min = min.min(max);
        self.max = max.max(min);
        self.value = self.value.clamp(self.min, self.max);
        self
    }

    #[must_use]
    pub fn corner_radius_px(mut self, radius: f32) -> Self {
        self.corner_radius_px = Some(radius.max(0.0));
        self
    }

    #[must_use]
    pub fn on_change(mut self, handler: impl Fn(i32, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    #[must_use]
    pub fn mode(mut self, mode: M3AppearanceMode) -> Self {
        self.mode_override = Some(mode);
        self
    }

    fn wrap_step(current: i32, delta: i32, min: i32, max: i32) -> i32 {
        let span = (max - min + 1).max(1);
        min + (current - min + delta).rem_euclid(span)
    }
}

impl RenderOnce for M3ScrollField {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let m3 = self
            .mode_override
            .map(M3ActiveTheme::for_mode)
            .unwrap_or_else(|| M3ActiveTheme::resolve(cx));
        let colors = m3.colors;
        let radius = px(self.corner_radius_px.unwrap_or(m3.shape.large_px));
        let prefix = self.id.to_string();
        let up_id = ElementId::Name(format!("{prefix}-up").into());
        let down_id = ElementId::Name(format!("{prefix}-down").into());
        let next_up = Self::wrap_step(self.value, 1, self.min, self.max);
        let next_down = Self::wrap_step(self.value, -1, self.min, self.max);
        let on_up = self.on_change.clone();
        let on_down = self.on_change;
        let formatted = if self.pad_two_digits {
            format!("{:02}", self.value)
        } else {
            format!("{}", self.value)
        };

        let up_debug = up_id.clone();
        let down_debug = down_id.clone();
        v_flex()
            .id(self.id)
            .w(px(76.0))
            .p(px(6.0))
            .gap(px(4.0))
            .items_center()
            .rounded(radius)
            .bg(colors.surface_container_highest)
            .border_1()
            .border_color(colors.outline_variant)
            .child(
                h_flex()
                    .id(up_id)
                    .debug_selector(move || up_debug.to_string())
                    .w(px(28.0))
                    .h(px(22.0))
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .rounded_full()
                    .text_color(colors.on_surface_variant)
                    .when_some(on_up, |btn, handler| {
                        btn.on_click(move |_, window, cx| handler(next_up, window, cx))
                    })
                    .child(Icon::new(IconName::ChevronUp)),
            )
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.on_surface)
                    .child(formatted),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(colors.on_surface_variant)
                    .child(self.label),
            )
            .child(
                h_flex()
                    .id(down_id)
                    .debug_selector(move || down_debug.to_string())
                    .w(px(28.0))
                    .h(px(22.0))
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .rounded_full()
                    .text_color(colors.on_surface_variant)
                    .when_some(on_down, |btn, handler| {
                        btn.on_click(move |_, window, cx| handler(next_down, window, cx))
                    })
                    .child(Icon::new(IconName::ChevronDown)),
            )
    }
}

/// Display mode for [`M3VibrantTimePicker`] (`Dial` vs `Scroll` input).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum M3TimePickerDisplayMode {
    #[default]
    Scroll,
    Dial,
}

/// Compose Material 3 1.5.0 `VibrantTimePickerDialog` flagship expressive component.
#[derive(IntoElement)]
pub struct M3VibrantTimePicker {
    id: ElementId,
    title: SharedString,
    hour: u8,
    minute: u8,
    is_pm: bool,
    display_mode: M3TimePickerDisplayMode,
    corner_radius_px: Option<f32>,
    padding_px: f32,
    on_change: Option<TimeChangeHandler>,
    mode_override: Option<M3AppearanceMode>,
}

impl M3VibrantTimePicker {
    #[must_use]
    pub fn new(id: impl Into<ElementId>, hour: u8, minute: u8, is_pm: bool) -> Self {
        Self {
            id: id.into(),
            title: SharedString::from("Select time"),
            hour: hour.clamp(1, 12),
            minute: minute.min(59),
            is_pm,
            display_mode: M3TimePickerDisplayMode::Scroll,
            corner_radius_px: None,
            padding_px: 20.0,
            on_change: None,
            mode_override: None,
        }
    }

    #[must_use]
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    #[must_use]
    pub fn display_mode(mut self, mode: M3TimePickerDisplayMode) -> Self {
        self.display_mode = mode;
        self
    }

    #[must_use]
    pub fn corner_radius_px(mut self, radius: f32) -> Self {
        self.corner_radius_px = Some(radius.max(0.0));
        self
    }

    #[must_use]
    pub fn padding_px(mut self, padding: f32) -> Self {
        self.padding_px = padding.clamp(8.0, 40.0);
        self
    }

    #[must_use]
    pub fn on_change(
        mut self,
        handler: impl Fn(u8, u8, bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    #[must_use]
    pub fn mode(mut self, mode: M3AppearanceMode) -> Self {
        self.mode_override = Some(mode);
        self
    }
}

impl RenderOnce for M3VibrantTimePicker {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let m3 = self
            .mode_override
            .map(M3ActiveTheme::for_mode)
            .unwrap_or_else(|| M3ActiveTheme::resolve(cx));
        let colors = m3.colors;
        let card_radius = px(self.corner_radius_px.unwrap_or(m3.shape.extra_large_px));
        let prefix = self.id.to_string();
        let hour = self.hour;
        let minute = self.minute;
        let is_pm = self.is_pm;
        let on_hour = self.on_change.clone();
        let on_min = self.on_change.clone();
        let on_am = self.on_change.clone();
        let on_pm = self.on_change;

        v_flex()
            .id(self.id)
            .p(px(self.padding_px))
            .gap(px(14.0))
            .rounded(card_radius)
            .bg(colors.surface_container_high)
            .border_1()
            .border_color(colors.outline_variant)
            .shadow(m3_elevation_shadow(&colors, 2))
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.on_surface_variant)
                            .child(self.title),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(colors.primary)
                            .child(match self.display_mode {
                                M3TimePickerDisplayMode::Scroll => "Scroll",
                                M3TimePickerDisplayMode::Dial => "Dial",
                            }),
                    ),
            )
            .child(
                h_flex()
                    .gap(px(10.0))
                    .items_center()
                    .child(
                        M3ScrollField::new(
                            ElementId::Name(format!("{prefix}-hour").into()),
                            "Hour",
                            i32::from(hour),
                        )
                        .range(1, 12)
                        .corner_radius_px(
                            self.corner_radius_px
                                .map(|r| (r * 0.6).max(6.0))
                                .unwrap_or(m3.shape.large_px),
                        )
                        .when_some(on_hour, |field, handler| {
                            field.on_change(move |next_h, window, cx| {
                                handler(next_h.clamp(1, 12) as u8, minute, is_pm, window, cx);
                            })
                        }),
                    )
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(FontWeight::BOLD)
                            .text_color(colors.on_surface)
                            .child(":"),
                    )
                    .child(
                        M3ScrollField::new(
                            ElementId::Name(format!("{prefix}-minute").into()),
                            "Min",
                            i32::from(minute),
                        )
                        .range(0, 59)
                        .corner_radius_px(
                            self.corner_radius_px
                                .map(|r| (r * 0.6).max(6.0))
                                .unwrap_or(m3.shape.large_px),
                        )
                        .when_some(on_min, |field, handler| {
                            field.on_change(move |next_m, window, cx| {
                                handler(hour, next_m.clamp(0, 59) as u8, is_pm, window, cx);
                            })
                        }),
                    )
                    .child(
                        v_flex()
                            .gap(px(4.0))
                            .child({
                                let am_id = ElementId::Name(format!("{prefix}-am").into());
                                let am_debug = am_id.clone();
                                h_flex()
                                    .id(am_id)
                                    .debug_selector(move || am_debug.to_string())
                                    .w(px(52.0))
                                    .h(px(36.0))
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .rounded_t(px(m3.shape.medium_px))
                                    .rounded_b(px(m3.shape.extra_small_px))
                                    .bg(if !is_pm {
                                        colors.tertiary_container
                                    } else {
                                        colors.surface_container_low
                                    })
                                    .text_color(if !is_pm {
                                        colors.on_tertiary_container
                                    } else {
                                        colors.on_surface_variant
                                    })
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .when_some(on_am, |btn, handler| {
                                        btn.on_click(move |_, window, cx| {
                                            handler(hour, minute, false, window, cx);
                                        })
                                    })
                                    .child("AM")
                            })
                            .child({
                                let pm_id = ElementId::Name(format!("{prefix}-pm").into());
                                let pm_debug = pm_id.clone();
                                h_flex()
                                    .id(pm_id)
                                    .debug_selector(move || pm_debug.to_string())
                                    .w(px(52.0))
                                    .h(px(36.0))
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .rounded_t(px(m3.shape.extra_small_px))
                                    .rounded_b(px(m3.shape.medium_px))
                                    .bg(if is_pm {
                                        colors.tertiary_container
                                    } else {
                                        colors.surface_container_low
                                    })
                                    .text_color(if is_pm {
                                        colors.on_tertiary_container
                                    } else {
                                        colors.on_surface_variant
                                    })
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .when_some(on_pm, |btn, handler| {
                                        btn.on_click(move |_, window, cx| {
                                            handler(hour, minute, true, window, cx);
                                        })
                                    })
                                    .child("PM")
                            }),
                    ),
            )
    }
}

// ---------------------------------------------------------------------------
// End-to-End Automated Proofs (`cargo test -p gpui-component --lib material3::compose15`)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    use gpui::{AppContext as _, Context, Render, TestAppContext, size};

    #[test]
    fn compose15_catalog_shapes_motion_and_slider_precision() {
        let catalog = M3Compose15Catalog::canonical();
        assert_eq!(catalog.material3_version, "1.5.0");
        assert_eq!(catalog.compose_version, "1.13.0");
        assert_eq!(catalog.compose_bom, "2026.11.01");
        assert_eq!(catalog.figma_kit_file_id, "1035203688168086460");
        assert_eq!(catalog.material_shapes_count, 35);
        assert!(catalog.wgsl_bytes > 2000);
        assert!(catalog.parity_entries.len() >= 15);

        // Verify all 35 MaterialShapes produce valid GPUI paths across morph transitions
        let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(64.0), px(64.0)));
        for (idx, shape) in M3MaterialShape::ALL_35.iter().enumerate() {
            assert_eq!(shape.shader_id(), idx as u32);
            let next = M3MaterialShape::ALL_35[(idx + 1) % 35];
            let path = shape.build_morphed_path(bounds, next, 0.5);
            assert!(path.is_some(), "shape {shape:?} -> {next:?} must build GPUI path");
        }

        // Verify Standard vs Expressive MotionScheme spring differences
        assert!(
            M3MotionScheme::Expressive.spatial_default().damping_ratio
                < M3MotionScheme::Standard.spatial_default().damping_ratio
        );
        assert!(
            (M3MotionScheme::Expressive.effects_default().damping_ratio - 1.0).abs() < f32::EPSILON
        );

        // Verify discrete SliderState exact integer tick snapping (no floating-point drift)
        let mut slider = M3SliderState::new(0.0, 0.0, 10.0, 9);
        assert!((slider.set_value(3.01) - 3.0).abs() < 1e-5);
        assert!((slider.step_by(1) - 4.0).abs() < 1e-5);
        assert!((slider.set_value(f32::NAN) - 0.0).abs() < 1e-5);

        // Verify RangeSliderState overlapping thumb tap fix (1.5.0)
        let mut range = M3RangeSliderState::new(5.0, 5.0, 0.0, 10.0, 9);
        let (a, b) = range.tap_fraction(0.8);
        assert!((a - 5.0).abs() < 1e-5);
        assert!((b - 8.0).abs() < 1e-5);

        // Verify SecureTextField defaults to System obfuscation
        let secure = M3ExpressiveTextField::new("pw", "Password")
            .value("secret")
            .secure(true);
        assert_eq!(secure.display_value(), SharedString::from("••••••"));

        // Verify Wavy progress NaN safety -> indeterminate mode
        let wavy = M3WavyLinearProgressIndicator::new("wavy").progress(Some(f32::NAN));
        assert!(wavy.is_indeterminate());

        // Verify ToggleButton expressive shape morph radius (pill -> squircle)
        let unchecked = M3ToggleButton::new("tb", "Bold").checked(false);
        let checked = M3ToggleButton::new("tb", "Bold").checked(true);
        assert!(unchecked.resolved_corner_radius_px(9999.0) > 100.0);
        assert!(checked.resolved_corner_radius_px(9999.0) <= 20.0);
    }

    #[gpui::test]
    fn compose15_full_component_gallery_renders_and_interacts(cx: &mut TestAppContext) {
        struct Compose15Harness {
            toggle_checked: Rc<Cell<bool>>,
            split_primary_clicks: Rc<Cell<usize>>,
            split_trailing_expanded: Rc<Cell<bool>>,
            group_selected: Rc<Cell<usize>>,
            overflow_clicks: Rc<Cell<usize>>,
            list_selected: Rc<Cell<bool>>,
            menu_checked: Rc<Cell<bool>>,
            time_hour: Rc<Cell<u8>>,
            time_is_pm: Rc<Cell<bool>>,
            slider_state: Entity<M3SliderState>,
            range_state: Entity<M3RangeSliderState>,
        }

        impl Render for Compose15Harness {
            fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
                let view = cx.entity().downgrade();
                let notify = move |cx: &mut App| {
                    assert!(
                        view.update(cx, |_, cx| cx.notify()).is_ok(),
                        "Compose15Harness view must stay alive during test"
                    );
                };
                let toggle_checked = self.toggle_checked.clone();
                let split_primary = self.split_primary_clicks.clone();
                let split_expanded = self.split_trailing_expanded.clone();
                let group_selected = self.group_selected.clone();
                let overflow_clicks = self.overflow_clicks.clone();
                let list_selected = self.list_selected.clone();
                let menu_checked = self.menu_checked.clone();
                let time_hour = self.time_hour.clone();
                let time_is_pm = self.time_is_pm.clone();

                v_flex()
                    .w(px(760.0))
                    .gap(px(10.0))
                    .p(px(12.0))
                    .child(
                        h_flex()
                            .gap(px(8.0))
                            .child(
                                M3ToggleButton::new("c15-toggle", "Expressive")
                                    .variant(M3ToggleButtonVariant::FilledTonal)
                                    .checked(toggle_checked.get())
                                    .focused(true)
                                    .on_checked_change({
                                        let notify = notify.clone();
                                        move |next, _, cx| {
                                            toggle_checked.set(*next);
                                            notify(cx);
                                        }
                                    }),
                            )
                            .child(
                                M3SplitButton::new("c15-split", "Deploy")
                                    .expanded(split_expanded.get())
                                    .on_primary_click({
                                        let notify = notify.clone();
                                        move |_, _, cx| {
                                            split_primary.set(split_primary.get() + 1);
                                            notify(cx);
                                        }
                                    })
                                    .on_trailing_click({
                                        let notify = notify.clone();
                                        move |_, _, cx| {
                                            split_expanded.set(!split_expanded.get());
                                            notify(cx);
                                        }
                                    }),
                            )
                            .child(
                                M3LoadingIndicator::new("c15-loader")
                                    .contained(true)
                                    .shapes(M3MaterialShape::Flower, M3MaterialShape::Cookie9Sided)
                                    .progress(0.4),
                            ),
                    )
                    .child(
                        M3ButtonGroup::new("c15-group")
                            .max_visible(2)
                            .item(
                                M3ButtonGroupItem::new("a", "Alpha")
                                    .checked(group_selected.get() == 0)
                                    .on_click({
                                        let group_selected = group_selected.clone();
                                        let notify = notify.clone();
                                        move |_, _, cx| {
                                            group_selected.set(0);
                                            notify(cx);
                                        }
                                    }),
                            )
                            .item(
                                M3ButtonGroupItem::new("b", "Beta")
                                    .checked(group_selected.get() == 1)
                                    .on_click({
                                        let notify = notify.clone();
                                        move |_, _, cx| {
                                            group_selected.set(1);
                                            notify(cx);
                                        }
                                    }),
                            )
                            .item(M3ButtonGroupItem::new("c", "Gamma"))
                            .on_overflow_click({
                                let notify = notify.clone();
                                move |_, _, cx| {
                                    overflow_clicks.set(overflow_clicks.get() + 1);
                                    notify(cx);
                                }
                            }),
                    )
                    .child(
                        M3FloatingToolbar::new("c15-toolbar")
                            .color(M3FloatingToolbarColor::Vibrant)
                            .action(M3ToggleButton::new("tb-act", "Bold"))
                            .overflow_count(2),
                    )
                    .child(
                        h_flex()
                            .gap(px(12.0))
                            .items_center()
                            .child(
                                M3WavyLinearProgressIndicator::new("c15-wavy-lin")
                                    .progress(Some(0.6)),
                            )
                            .child(
                                M3WavyCircularProgressIndicator::new("c15-wavy-circ")
                                    .progress(Some(0.75)),
                            ),
                    )
                    .child(
                        M3SegmentedListItem::new("c15-list-first", "Compose M3 1.5.0")
                            .supporting_text("Segmented rounded container")
                            .position(M3SegmentPosition::First)
                            .selected(list_selected.get())
                            .on_click({
                                let notify = notify.clone();
                                move |_, _, cx| {
                                    list_selected.set(!list_selected.get());
                                    notify(cx);
                                }
                            }),
                    )
                    .child(
                        M3StatefulSlider::new("c15-slider", self.slider_state.clone())
                            .label("Corner Radius")
                            .focused(true),
                    )
                    .child(
                        M3StatefulRangeSlider::new("c15-range", self.range_state.clone())
                            .label("Sweet Spot"),
                    )
                    .child(
                        M3ExpressiveTextField::new("c15-field", "Token Seed")
                            .value("#135B82")
                            .outlined(true)
                            .label_position(M3LabelPosition::Cutout),
                    )
                    .child(
                        M3ExpressiveMenuItem::new("c15-menu", "Inset Focus Rings")
                            .selection_kind(M3MenuItemSelectionKind::Checkable)
                            .selected(menu_checked.get())
                            .vibrant(true)
                            .has_submenu(true, M3PopupAnchor::RightCascading)
                            .on_click({
                                let notify = notify.clone();
                                move |_, _, cx| {
                                    menu_checked.set(!menu_checked.get());
                                    notify(cx);
                                }
                            }),
                    )
                    .child(
                        M3VibrantTimePicker::new(
                            "c15-time",
                            time_hour.get(),
                            30,
                            time_is_pm.get(),
                        )
                        .on_change(move |h, _m, pm, _, cx| {
                            time_hour.set(h);
                            time_is_pm.set(pm);
                            notify(cx);
                        }),
                    )
            }
        }

        let scope = M3ExpressiveThemeScope::new(M3AppearanceMode::NeuralObsidian)
            .motion_scheme(M3MotionScheme::Expressive)
            .default_font_family("Google Sans Flex");

        let (slider_state, range_state) = cx.update(|cx| {
            crate::init(cx);
            apply_m3_expressive_theme(&scope, cx);
            assert_eq!(
                active_m3_expressive_config(cx).motion_scheme,
                M3MotionScheme::Expressive
            );
            assert!(active_m3_expressive_config(cx).ripple.inset_focus_ring_enabled);
            let s = cx.new(|_| M3SliderState::new(4.0, 0.0, 10.0, 9));
            let r = cx.new(|_| M3RangeSliderState::new(2.0, 6.0, 0.0, 10.0, 9));
            (s, r)
        });

        let toggle_checked = Rc::new(Cell::new(false));
        let split_primary = Rc::new(Cell::new(0));
        let split_expanded = Rc::new(Cell::new(false));
        let group_selected = Rc::new(Cell::new(0));
        let overflow_clicks = Rc::new(Cell::new(0));
        let list_selected = Rc::new(Cell::new(false));
        let menu_checked = Rc::new(Cell::new(false));
        let time_hour = Rc::new(Cell::new(9));
        let time_is_pm = Rc::new(Cell::new(false));

        let (_, cx) = cx.add_window_view({
            let toggle_checked = toggle_checked.clone();
            let split_primary = split_primary.clone();
            let split_expanded = split_expanded.clone();
            let group_selected = group_selected.clone();
            let overflow_clicks = overflow_clicks.clone();
            let list_selected = list_selected.clone();
            let menu_checked = menu_checked.clone();
            let time_hour = time_hour.clone();
            let time_is_pm = time_is_pm.clone();
            let slider_state = slider_state.clone();
            let range_state = range_state.clone();
            move |_, _| Compose15Harness {
                toggle_checked,
                split_primary_clicks: split_primary,
                split_trailing_expanded: split_expanded,
                group_selected,
                overflow_clicks,
                list_selected,
                menu_checked,
                time_hour,
                time_is_pm,
                slider_state,
                range_state,
            }
        });

        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });

        let toggle_bounds = cx.debug_bounds("c15-toggle");
        assert!(toggle_bounds.is_some());
        if let Some(bounds) = toggle_bounds {
            cx.simulate_click(bounds.center(), gpui::Modifiers::default());
            cx.run_until_parked();
        }
        assert!(toggle_checked.get());

        if let Some(bounds) = cx.debug_bounds("c15-split-leading") {
            cx.simulate_click(bounds.center(), gpui::Modifiers::default());
            cx.run_until_parked();
        }
        assert_eq!(split_primary.get(), 1);

        if let Some(bounds) = cx.debug_bounds("c15-split-trailing") {
            cx.simulate_click(bounds.center(), gpui::Modifiers::default());
            cx.run_until_parked();
        }
        assert!(split_expanded.get());

        if let Some(bounds) = cx.debug_bounds("c15-group-b") {
            cx.simulate_click(bounds.center(), gpui::Modifiers::default());
            cx.run_until_parked();
        }
        assert_eq!(group_selected.get(), 1);

        if let Some(bounds) = cx.debug_bounds("c15-group-overflow") {
            cx.simulate_click(bounds.center(), gpui::Modifiers::default());
            cx.run_until_parked();
        }
        assert_eq!(overflow_clicks.get(), 1);

        if let Some(bounds) = cx.debug_bounds("c15-list-first") {
            cx.simulate_click(bounds.center(), gpui::Modifiers::default());
            cx.run_until_parked();
        }
        assert!(list_selected.get());

        if let Some(bounds) = cx.debug_bounds("c15-slider-inc") {
            cx.simulate_click(bounds.center(), gpui::Modifiers::default());
            cx.run_until_parked();
        }
        let next_slider = cx.read_entity(&slider_state, |s, _| s.value);
        assert!((next_slider - 5.0).abs() < 1e-4);

        if let Some(bounds) = cx.debug_bounds("c15-menu") {
            cx.simulate_click(bounds.center(), gpui::Modifiers::default());
            cx.run_until_parked();
        }
        assert!(menu_checked.get());

        if let Some(bounds) = cx.debug_bounds("c15-time-hour-up") {
            cx.simulate_click(bounds.center(), gpui::Modifiers::default());
            cx.run_until_parked();
        }
        assert_eq!(time_hour.get(), 10);

        if let Some(bounds) = cx.debug_bounds("c15-time-pm") {
            cx.simulate_click(bounds.center(), gpui::Modifiers::default());
            cx.run_until_parked();
        }
        assert!(time_is_pm.get());

        assert!(cx.debug_bounds("c15-toolbar").is_some());
        assert!(cx.debug_bounds("c15-wavy-lin").is_some());
        assert!(cx.debug_bounds("c15-wavy-circ").is_some());
        assert!(cx.debug_bounds("c15-loader").is_some());
        assert!(cx.debug_bounds("c15-field").is_some());
        assert!(cx.debug_bounds("c15-range").is_some());
    }
}
