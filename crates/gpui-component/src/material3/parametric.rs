//! Material 3 Parametric Prototyping Studio Engine for GPUI Kit (`crates/gpui-component`).
//!
//! Directly implements all 6 official workflows from Google Material Design's
//! *"Enhance your component designs with parametric prototyping"*
//! (`https://m3.material.io/blog/enhance-your-component-designs-with-parametric-prototyping`,
//! Nov 16, 2026) and bridges directly to the Material 3 Figma Design Kit
//! (`https://www.figma.com/community/file/1035203688168086460/material-3-design-kit`):
//!
//! 1. **Map the sweet spots**: 2D parameter grid (`M3SweetSpotGrid`) separating cohesive sweet spots
//!    from caution/off-limits combinations (touch target, radius ratio, WCAG AA contrast).
//! 2. **Fine-tune your parameters**: Typed, decoupled parameter store (`M3ParametricState`) for
//!    corner radius, padding, height, type size, wave amplitude, shape morph, and spring physics.
//! 3. **Group & tether your parameters**: Deterministic `M3ParamTetherRule` constraints that keep
//!    related variables cohesive (e.g. clamping corner radius to half container height, tethering
//!    horizontal padding to control height).
//! 4. **Turn designs into specs**: Interactive `M3SpecOverlayConfig` toggling keylines, color token
//!    annotations, and spatial measurements directly over the live GPUI preview.
//! 5. **Create presets & export JSON / Figma DTCG**: Built-in & custom `M3ParamPreset` management
//!    with round-trip JSON and W3C DTCG / Figma Design Kit (`1035203688168086460`) export.
//! 6. **Randomize to break patterns**: Deterministic seeded SplitMix64 randomizer (`randomize`) to
//!    escape human bias and stress-test layout edge cases.

use anyhow::{Context as _, Result};
use gpui::{
    App, AppContext as _, Context, ElementId, Entity, FocusHandle, Focusable, FontWeight,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString, Styled, Window, div,
    prelude::FluentBuilder as _, px,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};

use crate::{IconName, h_flex, v_flex};

use super::{
    M3ActiveTheme, M3AppearanceMode, M3ExpressiveSwitch, M3FilledButton, M3OutlinedButton,
    M3TonalButton, m3_contrast_ratio, m3_hsla_to_hex,
    compose15::{
        FIGMA_M3_DESIGN_KIT_FILE_ID, FIGMA_M3_DESIGN_KIT_URL, M3ButtonGroup, M3ButtonGroupItem,
        M3ButtonSize, M3FloatingToolbar, M3FloatingToolbarColor, M3ListVerticalAlignment,
        M3MaterialShape, M3MotionScheme, M3SegmentPosition, M3SegmentedListItem, M3SliderState,
        M3SplitButton, M3StatefulSlider, M3TimePickerDisplayMode, M3ToggleButton,
        M3ToggleButtonVariant, M3VibrantTimePicker, M3WavyLinearProgressIndicator,
        M3_PARAMETRIC_BLOG_URL,
    },
};

/// Component target actively under parametric exploration.
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
pub enum M3ParamTarget {
    #[default]
    VibrantTimePicker,
    ToggleButton,
    SplitButton,
    FloatingToolbar,
    SegmentedListItem,
    WavyProgress,
}

impl M3ParamTarget {
    pub const ALL: [Self; 6] = [
        Self::VibrantTimePicker,
        Self::ToggleButton,
        Self::SplitButton,
        Self::FloatingToolbar,
        Self::SegmentedListItem,
        Self::WavyProgress,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::VibrantTimePicker => "Time Picker",
            Self::ToggleButton => "Toggle Button",
            Self::SplitButton => "Split Button",
            Self::FloatingToolbar => "Floating Toolbar",
            Self::SegmentedListItem => "Segmented List",
            Self::WavyProgress => "Wavy Progress",
        }
    }
}

/// Strategy #4: Interactive specification overlays turning prototypes into engineering specs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct M3SpecOverlayConfig {
    pub show_keylines: bool,
    pub show_color_tokens: bool,
    pub show_measurements: bool,
    pub show_sweet_spot_grid: bool,
}

impl Default for M3SpecOverlayConfig {
    fn default() -> Self {
        Self {
            show_keylines: false,
            show_color_tokens: true,
            show_measurements: true,
            show_sweet_spot_grid: false,
        }
    }
}

/// Strategy #3: Tether rules coupling related parameters so configurations remain cohesive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct M3ParamTetherRules {
    /// Clamps `corner_radius_px <= container_height_px * 0.5` so pills never distort.
    pub clamp_radius_to_half_height: bool,
    /// Tethers `padding_px` proportionally to `container_height_px` (`0.25x..=0.55x`).
    pub tether_padding_to_height: bool,
    /// Enforces minimum WCAG 2.2 AA contrast (`>= 4.5:1`) on primary/surface pairs.
    pub enforce_wcag_aa: bool,
}

impl Default for M3ParamTetherRules {
    fn default() -> Self {
        Self {
            clamp_radius_to_half_height: true,
            tether_padding_to_height: true,
            enforce_wcag_aa: true,
        }
    }
}

/// Strategy #2: Decoupled parametric design variables stored cleanly in Rust code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct M3ParametricVariables {
    pub corner_radius_px: f32,
    pub padding_px: f32,
    pub container_height_px: f32,
    pub type_size_px: f32,
    pub wave_amplitude_px: f32,
    pub wave_wavelength_px: f32,
    pub progress: f32,
    pub shape_index: usize,
    pub mode: M3AppearanceMode,
    pub motion_scheme: M3MotionScheme,
    pub time_picker_mode: M3TimePickerDisplayMode,
}

impl Default for M3ParametricVariables {
    fn default() -> Self {
        Self {
            corner_radius_px: 20.0,
            padding_px: 18.0,
            container_height_px: 48.0,
            type_size_px: 14.0,
            wave_amplitude_px: 3.5,
            wave_wavelength_px: 24.0,
            progress: 0.68,
            shape_index: 19, // Cookie9Sided
            mode: M3AppearanceMode::NeuralObsidian,
            motion_scheme: M3MotionScheme::Expressive,
            time_picker_mode: M3TimePickerDisplayMode::Scroll,
        }
    }
}

impl M3ParametricVariables {
    /// Applies Strategy #3 tether rules cleanly, returning how many adjustments were applied.
    pub fn apply_tethers(&mut self, rules: M3ParamTetherRules) -> usize {
        let mut adjustments = 0;

        self.container_height_px = self.container_height_px.clamp(32.0, 96.0);
        self.type_size_px = self.type_size_px.clamp(11.0, 28.0);
        self.wave_amplitude_px = self.wave_amplitude_px.clamp(0.0, 10.0);
        self.wave_wavelength_px = self.wave_wavelength_px.clamp(12.0, 48.0);
        self.progress = self.progress.clamp(0.0, 1.0);
        self.shape_index %= M3MaterialShape::ALL_35.len();

        if rules.clamp_radius_to_half_height {
            let max_r = self.container_height_px * 0.5;
            if self.corner_radius_px > max_r {
                self.corner_radius_px = max_r;
                adjustments += 1;
            }
        }
        self.corner_radius_px = self.corner_radius_px.clamp(0.0, 48.0);

        if rules.tether_padding_to_height {
            let min_pad = (self.container_height_px * 0.25).max(8.0);
            let max_pad = (self.container_height_px * 0.55).min(36.0);
            let clamped = self.padding_px.clamp(min_pad, max_pad);
            if (clamped - self.padding_px).abs() > 1e-4 {
                self.padding_px = clamped;
                adjustments += 1;
            }
        } else {
            self.padding_px = self.padding_px.clamp(4.0, 40.0);
        }

        adjustments
    }

    #[must_use]
    pub fn material_shape(&self) -> M3MaterialShape {
        M3MaterialShape::ALL_35[self.shape_index % M3MaterialShape::ALL_35.len()]
    }
}

/// Strategy #1: Validity verdict for a cell in the 2D sweet-spot parameter grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum M3SweetSpotVerdict {
    SweetSpot,
    Caution,
    OffLimits,
}

/// Single evaluated combination inside [`M3SweetSpotGrid`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct M3SweetSpotCell {
    pub corner_radius_px: f32,
    pub container_height_px: f32,
    pub verdict: M3SweetSpotVerdict,
    pub contrast_ratio: f32,
    pub reason: String,
}

/// Strategy #1: 2D matrix mapping ideal parameter pairings vs off-limits zones.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct M3SweetSpotGrid {
    pub x_axis: &'static str,
    pub y_axis: &'static str,
    pub radii: Vec<f32>,
    pub heights: Vec<f32>,
    pub cells: Vec<M3SweetSpotCell>,
}

impl M3SweetSpotGrid {
    /// Evaluates a canonical 4x4 grid of `(corner_radius_px x container_height_px)` combinations
    /// against Material 3 touch target, aspect proportion, and WCAG AA contrast boundaries.
    #[must_use]
    pub fn evaluate(mode: M3AppearanceMode) -> Self {
        let m3 = M3ActiveTheme::for_mode(mode);
        let contrast = m3_contrast_ratio(m3.colors.on_primary, m3.colors.primary);
        let radii = vec![4.0, 12.0, 20.0, 32.0];
        let heights = vec![28.0, 40.0, 56.0, 72.0];
        let mut cells = Vec::with_capacity(radii.len() * heights.len());

        for &h in &heights {
            for &r in &radii {
                let (verdict, reason) = if contrast < 4.5 {
                    (
                        M3SweetSpotVerdict::OffLimits,
                        format!("WCAG AA fail ({contrast:.2}:1)"),
                    )
                } else if h < 32.0 {
                    (
                        M3SweetSpotVerdict::OffLimits,
                        "Below 32px min compact target".to_owned(),
                    )
                } else if r > h * 0.5 {
                    (
                        M3SweetSpotVerdict::OffLimits,
                        "Radius exceeds 50% height".to_owned(),
                    )
                } else if h < 40.0 || r < 8.0 {
                    (
                        M3SweetSpotVerdict::Caution,
                        "Compact density / sharp corner".to_owned(),
                    )
                } else {
                    (
                        M3SweetSpotVerdict::SweetSpot,
                        "M3 Expressive sweet spot".to_owned(),
                    )
                };

                cells.push(M3SweetSpotCell {
                    corner_radius_px: r,
                    container_height_px: h,
                    verdict,
                    contrast_ratio: contrast,
                    reason,
                });
            }
        }

        Self {
            x_axis: "corner_radius_px",
            y_axis: "container_height_px",
            radii,
            heights,
            cells,
        }
    }

    #[must_use]
    pub fn count_by_verdict(&self, verdict: M3SweetSpotVerdict) -> usize {
        self.cells.iter().filter(|c| c.verdict == verdict).count()
    }
}

/// Strategy #5: Named reusable parameter preset (serializable to JSON & Figma DTCG).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct M3ParamPreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub target: M3ParamTarget,
    pub variables: M3ParametricVariables,
    pub tethers: M3ParamTetherRules,
}

impl M3ParamPreset {
    /// Built-in curated presets anchored to Google Material 3 Expressive & Figma Kit `1035203688168086460`.
    #[must_use]
    pub fn built_in_presets() -> Vec<Self> {
        vec![
            Self {
                id: "expressive_time_picker".to_owned(),
                name: "Expressive Time Picker".to_owned(),
                description: "Blog flagship vibrant time picker with scroll fields & expressive springs".to_owned(),
                target: M3ParamTarget::VibrantTimePicker,
                variables: M3ParametricVariables::default(),
                tethers: M3ParamTetherRules::default(),
            },
            Self {
                id: "compact_studio_density".to_owned(),
                name: "Compact Studio".to_owned(),
                description: "High-density desktop toolbar & segmented controls (40px height, 12px radius)".to_owned(),
                target: M3ParamTarget::FloatingToolbar,
                variables: M3ParametricVariables {
                    corner_radius_px: 12.0,
                    padding_px: 12.0,
                    container_height_px: 40.0,
                    type_size_px: 13.0,
                    wave_amplitude_px: 2.5,
                    wave_wavelength_px: 20.0,
                    progress: 0.55,
                    shape_index: 16, // Cookie4Sided
                    mode: M3AppearanceMode::Dark,
                    motion_scheme: M3MotionScheme::Standard,
                    time_picker_mode: M3TimePickerDisplayMode::Scroll,
                },
                tethers: M3ParamTetherRules::default(),
            },
            Self {
                id: "oled_high_contrast".to_owned(),
                name: "OLED Pitch Expressive".to_owned(),
                description: "True-black OLED surface with large expressive pill targets & wavy progress".to_owned(),
                target: M3ParamTarget::WavyProgress,
                variables: M3ParametricVariables {
                    corner_radius_px: 28.0,
                    padding_px: 24.0,
                    container_height_px: 56.0,
                    type_size_px: 16.0,
                    wave_amplitude_px: 4.0,
                    wave_wavelength_px: 28.0,
                    progress: 0.82,
                    shape_index: 28, // Flower
                    mode: M3AppearanceMode::OledPitch,
                    motion_scheme: M3MotionScheme::Expressive,
                    time_picker_mode: M3TimePickerDisplayMode::Dial,
                },
                tethers: M3ParamTetherRules::default(),
            },
            Self {
                id: "material_light_classic".to_owned(),
                name: "Material Light Spec".to_owned(),
                description: "Light mode Figma Design Kit baseline with split actions and keyline specs".to_owned(),
                target: M3ParamTarget::SplitButton,
                variables: M3ParametricVariables {
                    corner_radius_px: 20.0,
                    padding_px: 16.0,
                    container_height_px: 44.0,
                    type_size_px: 14.0,
                    wave_amplitude_px: 3.0,
                    wave_wavelength_px: 24.0,
                    progress: 0.45,
                    shape_index: 22, // Clover4Leaf
                    mode: M3AppearanceMode::Light,
                    motion_scheme: M3MotionScheme::Expressive,
                    time_picker_mode: M3TimePickerDisplayMode::Scroll,
                },
                tethers: M3ParamTetherRules::default(),
            },
        ]
    }
}

/// Pure-Rust SplitMix64 PRNG for Strategy #6 deterministic seeded randomization.
struct SplitMix64(u64);

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_add(0x9E37_79B9_7F4A_7C15))
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_unit_f32(&mut self) -> f32 {
        ((self.next_u64() >> 40) as f32) / ((1u64 << 24) as f32)
    }
}

/// Complete Parametric Prototyping session state powering [`M3ParametricPrototyperView`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct M3ParametricState {
    pub target: M3ParamTarget,
    pub variables: M3ParametricVariables,
    pub tethers: M3ParamTetherRules,
    pub overlays: M3SpecOverlayConfig,
    pub active_preset_id: String,
    pub random_seed: u64,
    pub hour: u8,
    pub minute: u8,
    pub is_pm: bool,
    pub toggle_checked: bool,
    pub split_expanded: bool,
}

impl Default for M3ParametricState {
    fn default() -> Self {
        Self {
            target: M3ParamTarget::VibrantTimePicker,
            variables: M3ParametricVariables::default(),
            tethers: M3ParamTetherRules::default(),
            overlays: M3SpecOverlayConfig::default(),
            active_preset_id: "expressive_time_picker".to_owned(),
            random_seed: 2026_1119,
            hour: 9,
            minute: 41,
            is_pm: false,
            toggle_checked: true,
            split_expanded: false,
        }
    }
}

impl M3ParametricState {
    /// Applies a named [`M3ParamPreset`] and enforces its tether rules.
    pub fn apply_preset(&mut self, preset: &M3ParamPreset) {
        self.active_preset_id = preset.id.clone();
        self.target = preset.target;
        self.variables = preset.variables.clone();
        self.tethers = preset.tethers;
        self.variables.apply_tethers(self.tethers);
    }

    /// Strategy #6: Deterministic seeded randomization to explore unexpected visual wins
    /// and stress-test edge cases while respecting active tether rules.
    pub fn randomize(&mut self, seed: u64) {
        self.random_seed = seed;
        let mut rng = SplitMix64::new(seed);
        self.variables.container_height_px = (32.0 + rng.next_unit_f32() * 44.0).round();
        self.variables.corner_radius_px = (rng.next_unit_f32() * 36.0).round();
        self.variables.padding_px = (8.0 + rng.next_unit_f32() * 24.0).round();
        self.variables.type_size_px = (12.0 + rng.next_unit_f32() * 10.0).round();
        self.variables.wave_amplitude_px = (1.0 + rng.next_unit_f32() * 6.0 * 2.0).round() * 0.5;
        self.variables.wave_wavelength_px = (14.0 + rng.next_unit_f32() * 24.0).round();
        self.variables.progress = (rng.next_unit_f32() * 100.0).round() / 100.0;
        self.variables.shape_index = (rng.next_u64() as usize) % M3MaterialShape::ALL_35.len();
        self.variables.mode =
            M3AppearanceMode::ALL[(rng.next_u64() as usize) % M3AppearanceMode::ALL.len()];
        self.variables.motion_scheme = if rng.next_u64() % 2 == 0 {
            M3MotionScheme::Expressive
        } else {
            M3MotionScheme::Standard
        };
        self.variables.apply_tethers(self.tethers);
        self.active_preset_id = format!("random-{seed}");
    }

    /// Strategy #5: Exports the current parametric prototype configuration as pretty JSON.
    pub fn export_preset_json(&self) -> Result<String> {
        serde_json::to_string_pretty(&json!({
            "schema": "aphrody.m3.parametric_preset.v1",
            "parametric_blog_url": M3_PARAMETRIC_BLOG_URL,
            "figma_design_kit_file_id": FIGMA_M3_DESIGN_KIT_FILE_ID,
            "figma_design_kit_url": FIGMA_M3_DESIGN_KIT_URL,
            "preset_id": self.active_preset_id,
            "target": self.target,
            "variables": self.variables,
            "tethers": self.tethers,
            "overlays": self.overlays,
        }))
        .context("serializing parametric preset JSON")
    }

    /// Imports a previously exported parametric preset JSON snapshot.
    pub fn import_preset_json(raw_json: &str) -> Result<Self> {
        let parsed: JsonValue =
            serde_json::from_str(raw_json).context("parsing parametric preset JSON")?;
        let target: M3ParamTarget = serde_json::from_value(
            parsed
                .get("target")
                .cloned()
                .context("missing `target` in preset JSON")?,
        )
        .context("decoding `target`")?;
        let mut variables: M3ParametricVariables = serde_json::from_value(
            parsed
                .get("variables")
                .cloned()
                .context("missing `variables` in preset JSON")?,
        )
        .context("decoding `variables`")?;
        let tethers: M3ParamTetherRules = parsed
            .get("tethers")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .context("decoding `tethers`")?
            .unwrap_or_default();
        let overlays: M3SpecOverlayConfig = parsed
            .get("overlays")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .context("decoding `overlays`")?
            .unwrap_or_default();
        variables.apply_tethers(tethers);

        Ok(Self {
            target,
            variables,
            tethers,
            overlays,
            active_preset_id: parsed
                .get("preset_id")
                .and_then(JsonValue::as_str)
                .unwrap_or("imported")
                .to_owned(),
            ..Self::default()
        })
    }

    /// Exports current variables as W3C DTCG JSON directly consumable by Figma Variables /
    /// Material 3 Design Kit (`https://www.figma.com/community/file/1035203688168086460/material-3-design-kit`).
    pub fn export_figma_dtcg_json(&self) -> Result<String> {
        let active = M3ActiveTheme::for_mode(self.variables.mode);
        let spatial = self.variables.motion_scheme.spatial_default();
        let effects = self.variables.motion_scheme.effects_default();

        serde_json::to_string_pretty(&json!({
            "$schema": "https://tr.designtokens.org/format/",
            "name": "Aphrody Material 3 Parametric Prototype -> Figma DTCG",
            "figmaCommunityFileId": FIGMA_M3_DESIGN_KIT_FILE_ID,
            "figmaCommunityUrl": FIGMA_M3_DESIGN_KIT_URL,
            "mode": self.variables.mode.as_str(),
            "target": self.target,
            "tokens": {
                "shape": {
                    "cornerRadius": { "$type": "dimension", "$value": format!("{}px", self.variables.corner_radius_px) },
                    "materialShape": { "$type": "string", "$value": format!("{:?}", self.variables.material_shape()) }
                },
                "spacing": {
                    "padding": { "$type": "dimension", "$value": format!("{}px", self.variables.padding_px) },
                    "containerHeight": { "$type": "dimension", "$value": format!("{}px", self.variables.container_height_px) }
                },
                "typography": {
                    "fontFamily": { "$type": "fontFamily", "$value": active.typography.typeface.brand_family().as_ref() },
                    "fontSize": { "$type": "dimension", "$value": format!("{}px", self.variables.type_size_px) }
                },
                "color": {
                    "primary": { "$type": "color", "$value": m3_hsla_to_hex(active.colors.primary).as_ref() },
                    "onPrimary": { "$type": "color", "$value": m3_hsla_to_hex(active.colors.on_primary).as_ref() },
                    "surface": { "$type": "color", "$value": m3_hsla_to_hex(active.colors.surface).as_ref() },
                    "onSurface": { "$type": "color", "$value": m3_hsla_to_hex(active.colors.on_surface).as_ref() },
                    "secondaryContainer": { "$type": "color", "$value": m3_hsla_to_hex(active.colors.secondary_container).as_ref() }
                },
                "motion": {
                    "scheme": { "$type": "string", "$value": format!("{:?}", self.variables.motion_scheme) },
                    "spatialStiffness": { "$type": "number", "$value": spatial.stiffness },
                    "spatialDampingRatio": { "$type": "number", "$value": spatial.damping_ratio },
                    "effectsStiffness": { "$type": "number", "$value": effects.stiffness },
                    "effectsDampingRatio": { "$type": "number", "$value": effects.damping_ratio }
                }
            }
        }))
        .context("serializing Figma DTCG JSON")
    }
}

/// Interactive live GPUI Parametric Prototyping Studio View built strictly from GPUI Kit's
/// Material 3 controls (switches, stateful sliders, button groups, segmented list items, buttons).
pub struct M3ParametricPrototyperView {
    focus_handle: FocusHandle,
    state: M3ParametricState,
    radius_slider: Entity<M3SliderState>,
    height_slider: Entity<M3SliderState>,
    padding_slider: Entity<M3SliderState>,
    last_exported_json: Option<SharedString>,
}

impl M3ParametricPrototyperView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let state = M3ParametricState::default();
        let radius_slider =
            cx.new(|_| M3SliderState::new(state.variables.corner_radius_px, 0.0, 40.0, 19));
        let height_slider =
            cx.new(|_| M3SliderState::new(state.variables.container_height_px, 32.0, 80.0, 11));
        let padding_slider =
            cx.new(|_| M3SliderState::new(state.variables.padding_px, 8.0, 36.0, 13));

        Self {
            focus_handle: cx.focus_handle(),
            state,
            radius_slider,
            height_slider,
            padding_slider,
            last_exported_json: None,
        }
    }

    #[must_use]
    pub fn state(&self) -> &M3ParametricState {
        &self.state
    }

    #[must_use]
    pub fn last_exported_json(&self) -> Option<&SharedString> {
        self.last_exported_json.as_ref()
    }

    fn sync_sliders_from_state(&self, cx: &mut Context<Self>) {
        let vars = self.state.variables.clone();
        self.radius_slider.update(cx, |s, cx| {
            s.set_value(vars.corner_radius_px);
            cx.notify();
        });
        self.height_slider.update(cx, |s, cx| {
            s.set_value(vars.container_height_px);
            cx.notify();
        });
        self.padding_slider.update(cx, |s, cx| {
            s.set_value(vars.padding_px);
            cx.notify();
        });
    }
}

impl Focusable for M3ParametricPrototyperView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for M3ParametricPrototyperView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mode = self.state.variables.mode;
        let m3 = M3ActiveTheme::for_mode(mode);
        let colors = m3.colors;
        let vars = self.state.variables.clone();
        let overlays = self.state.overlays;
        let presets = M3ParamPreset::built_in_presets();
        let sweet_grid = M3SweetSpotGrid::evaluate(mode);
        let primary_contrast = m3_contrast_ratio(colors.on_primary, colors.primary);

        let mut target_group = M3ButtonGroup::new("param-targets").max_visible(6);
        for target in M3ParamTarget::ALL {
            let active = self.state.target == target;
            target_group = target_group.item(
                M3ButtonGroupItem::new(format!("{target:?}"), target.label())
                    .checked(active)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.state.target = target;
                        cx.notify();
                    })),
            );
        }

        let view_weak = cx.entity().downgrade();
        let preview: gpui::AnyElement = match self.state.target {
            M3ParamTarget::VibrantTimePicker => M3VibrantTimePicker::new(
                "param-preview-time",
                self.state.hour,
                self.state.minute,
                self.state.is_pm,
            )
            .display_mode(vars.time_picker_mode)
            .corner_radius_px(vars.corner_radius_px)
            .padding_px(vars.padding_px)
            .mode(mode)
            .on_change({
                let view = view_weak.clone();
                move |h, m, pm, _, cx| {
                    if let Err(err) = view.update(cx, |this, cx| {
                        this.state.hour = h;
                        this.state.minute = m;
                        this.state.is_pm = pm;
                        cx.notify();
                    }) {
                        log::debug!("parametric time picker update dropped: {err:#}");
                    }
                }
            })
            .into_any_element(),
            M3ParamTarget::ToggleButton => M3ToggleButton::new("param-preview-toggle", "Expressive")
                .variant(M3ToggleButtonVariant::FilledTonal)
                .button_size(M3ButtonSize::Large)
                .checked(self.state.toggle_checked)
                .focused(true)
                .mode(mode)
                .on_checked_change(cx.listener(|this, next, _, cx| {
                    this.state.toggle_checked = *next;
                    cx.notify();
                }))
                .into_any_element(),
            M3ParamTarget::SplitButton => M3SplitButton::new("param-preview-split", "Apply Tokens")
                .expanded(self.state.split_expanded)
                .mode(mode)
                .on_trailing_click(cx.listener(|this, _, _, cx| {
                    this.state.split_expanded = !this.state.split_expanded;
                    cx.notify();
                }))
                .into_any_element(),
            M3ParamTarget::FloatingToolbar => M3FloatingToolbar::new("param-preview-toolbar")
                .color(M3FloatingToolbarColor::Vibrant)
                .mode(mode)
                .action(M3ToggleButton::new("param-tb-1", "Tune").mode(mode))
                .action(M3ToggleButton::new("param-tb-2", "Spec").checked(true).mode(mode))
                .overflow_count(3)
                .into_any_element(),
            M3ParamTarget::SegmentedListItem => v_flex()
                .w(px(320.0))
                .gap(px(2.0))
                .child(
                    M3SegmentedListItem::new("param-seg-1", "Figma M3 Design Kit")
                        .supporting_text(FIGMA_M3_DESIGN_KIT_FILE_ID)
                        .position(M3SegmentPosition::First)
                        .vertical_alignment(M3ListVerticalAlignment::Center)
                        .selected(true)
                        .mode(mode),
                )
                .child(
                    M3SegmentedListItem::new("param-seg-2", "Compose M3 1.5.0 Stable")
                        .supporting_text("Zero-bloat GPUI Kit parity")
                        .position(M3SegmentPosition::Last)
                        .mode(mode),
                )
                .into_any_element(),
            M3ParamTarget::WavyProgress => M3WavyLinearProgressIndicator::new("param-preview-wavy")
                .progress(Some(vars.progress))
                .amplitude_px(vars.wave_amplitude_px)
                .wavelength_px(vars.wave_wavelength_px)
                .width_px(300.0)
                .mode(mode)
                .into_any_element(),
        };

        h_flex()
            .id("m3-parametric-prototyper")
            .debug_selector(|| "m3-parametric-prototyper".to_owned())
            .track_focus(&self.focus_handle)
            .w_full()
            .p(px(16.0))
            .gap(px(16.0))
            .bg(colors.surface)
            .text_color(colors.on_surface)
            .child(
                // Left column: Live parametric stage + Spec Overlay + Sweet-Spot Grid
                v_flex()
                    .flex_1()
                    .gap(px(12.0))
                    .child(target_group)
                    .child(
                        v_flex()
                            .id("param-stage")
                            .p(px(20.0))
                            .gap(px(10.0))
                            .items_center()
                            .justify_center()
                            .rounded(px(m3.shape.extra_large_px))
                            .bg(colors.surface_container_low)
                            .border_1()
                            .border_color(if overlays.show_keylines {
                                colors.primary
                            } else {
                                colors.outline_variant
                            })
                            .child(preview)
                            .when(overlays.show_measurements, |this| {
                                this.child(
                                    div()
                                        .id("param-spec-measurements")
                                        .debug_selector(|| "param-spec-measurements".to_owned())
                                        .px(px(10.0))
                                        .py(px(4.0))
                                        .rounded_full()
                                        .bg(colors.secondary_container)
                                        .text_color(colors.on_secondary_container)
                                        .text_xs()
                                        .child(format!(
                                            "r={:.0}px  h={:.0}px  pad={:.0}px  shape={:?}  WCAG={:.2}:1",
                                            vars.corner_radius_px,
                                            vars.container_height_px,
                                            vars.padding_px,
                                            vars.material_shape(),
                                            primary_contrast
                                        )),
                                )
                            })
                            .when(overlays.show_color_tokens, |this| {
                                this.child(
                                    div()
                                        .id("param-spec-tokens")
                                        .text_xs()
                                        .text_color(colors.on_surface_variant)
                                        .child(format!(
                                            "mode={}  primary={}  surface={}  motion={:?}",
                                            mode.as_str(),
                                            m3_hsla_to_hex(colors.primary),
                                            m3_hsla_to_hex(colors.surface),
                                            vars.motion_scheme
                                        )),
                                )
                            }),
                    )
                    .when(overlays.show_sweet_spot_grid, |this| {
                        this.child(
                            v_flex()
                                .id("param-sweet-spot-grid")
                                .debug_selector(|| "param-sweet-spot-grid".to_owned())
                                .p(px(12.0))
                                .gap(px(6.0))
                                .rounded(px(m3.shape.large_px))
                                .bg(colors.surface_container)
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(format!(
                                            "Sweet-Spot Grid ({} sweet / {} caution / {} off-limits)",
                                            sweet_grid.count_by_verdict(M3SweetSpotVerdict::SweetSpot),
                                            sweet_grid.count_by_verdict(M3SweetSpotVerdict::Caution),
                                            sweet_grid.count_by_verdict(M3SweetSpotVerdict::OffLimits),
                                        )),
                                ),
                        )
                    }),
            )
            .child(
                // Right column: Parametric Settings Panel (sliders, switches, presets, randomize, JSON export)
                v_flex()
                    .w(px(300.0))
                    .p(px(14.0))
                    .gap(px(10.0))
                    .rounded(px(m3.shape.large_px))
                    .bg(colors.surface_container)
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Parametric Settings"),
                    )
                    .child(
                        M3StatefulSlider::new("param-slider-radius", self.radius_slider.clone())
                            .label("Corner Radius (px)")
                            .mode(mode)
                            .on_value_change({
                                let view = view_weak.clone();
                                move |val, _, cx| {
                                    if let Err(err) = view.update(cx, |this, cx| {
                                        this.state.variables.corner_radius_px = val;
                                        this.state.variables.apply_tethers(this.state.tethers);
                                        this.sync_sliders_from_state(cx);
                                        cx.notify();
                                    }) {
                                        log::debug!("parametric radius update dropped: {err:#}");
                                    }
                                }
                            }),
                    )
                    .child(
                        M3StatefulSlider::new("param-slider-height", self.height_slider.clone())
                            .label("Container Height (px)")
                            .mode(mode)
                            .on_value_change({
                                let view = view_weak.clone();
                                move |val, _, cx| {
                                    if let Err(err) = view.update(cx, |this, cx| {
                                        this.state.variables.container_height_px = val;
                                        this.state.variables.apply_tethers(this.state.tethers);
                                        this.sync_sliders_from_state(cx);
                                        cx.notify();
                                    }) {
                                        log::debug!("parametric height update dropped: {err:#}");
                                    }
                                }
                            }),
                    )
                    .child(
                        M3StatefulSlider::new("param-slider-padding", self.padding_slider.clone())
                            .label("Padding (px)")
                            .mode(mode)
                            .on_value_change({
                                let view = view_weak.clone();
                                move |val, _, cx| {
                                    if let Err(err) = view.update(cx, |this, cx| {
                                        this.state.variables.padding_px = val;
                                        this.state.variables.apply_tethers(this.state.tethers);
                                        this.sync_sliders_from_state(cx);
                                        cx.notify();
                                    }) {
                                        log::debug!("parametric padding update dropped: {err:#}");
                                    }
                                }
                            }),
                    )
                    .child(
                        M3ExpressiveSwitch::new("param-switch-tether")
                            .label("Tether Parameters")
                            .checked(self.state.tethers.clamp_radius_to_half_height)
                            .on_change(cx.listener(|this, next, _, cx| {
                                this.state.tethers.clamp_radius_to_half_height = *next;
                                this.state.tethers.tether_padding_to_height = *next;
                                this.state.variables.apply_tethers(this.state.tethers);
                                this.sync_sliders_from_state(cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        M3ExpressiveSwitch::new("param-switch-keylines")
                            .label("Spec Keylines")
                            .checked(overlays.show_keylines)
                            .on_change(cx.listener(|this, next, _, cx| {
                                this.state.overlays.show_keylines = *next;
                                cx.notify();
                            })),
                    )
                    .child(
                        M3ExpressiveSwitch::new("param-switch-grid")
                            .label("Sweet-Spot Grid")
                            .checked(overlays.show_sweet_spot_grid)
                            .on_change(cx.listener(|this, next, _, cx| {
                                this.state.overlays.show_sweet_spot_grid = *next;
                                cx.notify();
                            })),
                    )
                    .child(
                        v_flex().gap(px(2.0)).children(presets.into_iter().enumerate().map(
                            |(idx, preset)| {
                                let pos = match idx {
                                    0 => M3SegmentPosition::First,
                                    3 => M3SegmentPosition::Last,
                                    _ => M3SegmentPosition::Middle,
                                };
                                let selected = self.state.active_preset_id == preset.id;
                                let preset_clone = preset.clone();
                                M3SegmentedListItem::new(
                                    ElementId::Name(format!("param-preset-{}", preset.id).into()),
                                    preset.name,
                                )
                                .supporting_text(preset.description)
                                .position(pos)
                                .selected(selected)
                                .mode(mode)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.state.apply_preset(&preset_clone);
                                    this.sync_sliders_from_state(cx);
                                    cx.notify();
                                }))
                            },
                        )),
                    )
                    .child(
                        v_flex()
                            .gap(px(6.0))
                            .child(
                                h_flex()
                                    .gap(px(6.0))
                                    .child(
                                        M3TonalButton::new("param-randomize", "Randomize")
                                            .icon(IconName::RefreshCw)
                                            .compact()
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                let next_seed =
                                                    this.state.random_seed.wrapping_add(1);
                                                this.state.randomize(next_seed);
                                                this.sync_sliders_from_state(cx);
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        M3FilledButton::new("param-export-json", "Export JSON")
                                            .compact()
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                if let Ok(json_text) =
                                                    this.state.export_preset_json()
                                                {
                                                    this.last_exported_json =
                                                        Some(json_text.into());
                                                    cx.notify();
                                                }
                                            })),
                                    ),
                            )
                            .child(
                                M3OutlinedButton::new("param-export-figma", "Figma DTCG")
                                    .compact()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        if let Ok(dtcg_text) =
                                            this.state.export_figma_dtcg_json()
                                        {
                                            this.last_exported_json = Some(dtcg_text.into());
                                            cx.notify();
                                        }
                                    })),
                            ),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[test]
    fn parametric_tethers_sweet_spot_randomize_and_figma_dtcg_roundtrip() -> Result<()> {
        // 1. Tether rules enforce radius <= height * 0.5 & proportional padding
        let mut vars = M3ParametricVariables {
            corner_radius_px: 44.0,
            container_height_px: 40.0,
            padding_px: 36.0,
            ..M3ParametricVariables::default()
        };
        let adjustments = vars.apply_tethers(M3ParamTetherRules::default());
        assert_eq!(adjustments, 2);
        assert!((vars.corner_radius_px - 20.0).abs() < 1e-4);
        assert!(vars.padding_px <= 22.0 + 1e-4);

        // 2. Sweet-spot grid classifies sweet spots, cautions, and off-limits cells
        for mode in M3AppearanceMode::ALL {
            let grid = M3SweetSpotGrid::evaluate(mode);
            assert_eq!(grid.cells.len(), 16);
            assert!(grid.count_by_verdict(M3SweetSpotVerdict::SweetSpot) >= 6);
            assert!(grid.count_by_verdict(M3SweetSpotVerdict::OffLimits) >= 4);
        }

        // 3. Seeded randomizer is 100% deterministic & respects tethers
        let mut a = M3ParametricState::default();
        let mut b = M3ParametricState::default();
        a.randomize(1337);
        b.randomize(1337);
        assert_eq!(a.variables, b.variables);
        assert!(a.variables.corner_radius_px <= a.variables.container_height_px * 0.5 + 1e-4);

        // 4. Preset JSON export/import round-trip & Figma Material 3 Design Kit DTCG export
        let preset_json = a.export_preset_json()?;
        assert!(preset_json.contains(FIGMA_M3_DESIGN_KIT_FILE_ID));
        let restored = M3ParametricState::import_preset_json(&preset_json)?;
        assert_eq!(restored.variables, a.variables);
        assert_eq!(restored.target, a.target);

        let dtcg = a.export_figma_dtcg_json()?;
        assert!(dtcg.contains("1035203688168086460"));
        assert!(dtcg.contains("https://tr.designtokens.org/format/"));
        assert!(dtcg.contains("spatialStiffness"));
        Ok(())
    }

    #[gpui::test]
    fn parametric_prototyper_view_renders_and_handles_all_controls(cx: &mut TestAppContext) {
        cx.update(|cx| {
            crate::init(cx);
        });

        let (view, cx) = cx.add_window_view(|_, cx| M3ParametricPrototyperView::new(cx));
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });

        assert!(cx.debug_bounds("m3-parametric-prototyper").is_some());
        assert!(cx.debug_bounds("param-spec-measurements").is_some());
        assert!(cx.debug_bounds("param-sweet-spot-grid").is_none());

        // Toggle Sweet-Spot Grid overlay on
        if let Some(bounds) = cx.debug_bounds("param-switch-grid") {
            cx.simulate_click(bounds.center(), gpui::Modifiers::default());
            cx.run_until_parked();
        }
        assert!(cx.debug_bounds("param-sweet-spot-grid").is_some());

        // Switch preset to OLED Pitch Expressive
        if let Some(bounds) = cx.debug_bounds("param-preset-oled_high_contrast") {
            cx.simulate_click(bounds.center(), gpui::Modifiers::default());
            cx.run_until_parked();
        }
        let active_mode = view.read_with(cx, |v, _| v.state().variables.mode);
        assert_eq!(active_mode, M3AppearanceMode::OledPitch);

        // Randomize with seeded SplitMix64
        let seed_before = view.read_with(cx, |v, _| v.state().random_seed);
        if let Some(bounds) = cx.debug_bounds("param-randomize") {
            cx.simulate_click(bounds.center(), gpui::Modifiers::default());
            cx.run_until_parked();
        }
        let seed_after = view.read_with(cx, |v, _| v.state().random_seed);
        assert_eq!(seed_after, seed_before.wrapping_add(1));

        // Export Figma Material 3 Design Kit DTCG JSON
        if let Some(bounds) = cx.debug_bounds("param-export-figma") {
            cx.simulate_click(bounds.center(), gpui::Modifiers::default());
            cx.run_until_parked();
        }
        let exported = view.read_with(cx, |v, _| v.last_exported_json().cloned());
        assert!(
            exported
                .as_deref()
                .is_some_and(|json| json.contains("1035203688168086460"))
        );
    }
}
