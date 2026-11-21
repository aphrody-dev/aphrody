//! Declarative, frame-exact programmatic video: composition format, timeline resolution,
//! keyframe animation (easing curves and springs) and per-frame scene evaluation.
//!
//! A [`Composition`] (JSON or TOML) lists tracks of clips. [`Timeline::new`] resolves every time
//! expression to integer frames, so frame `n` always shows exactly the same scene. Rendering the
//! scene to pixels lives in `motion_render`.
//!
//! Time model: all absolute times are seconds (`1.5`), explicit frames (`"45f"`), or references to
//! another clip's end (`"intro"`, `"intro + 0.5"`, `"intro - 6f"`). A clip is active on the frame
//! range `[round(start * fps), round(end * fps))`, so clips that touch tile with no gap and no
//! overlap. Keyframe times are clip-local (seconds, frames, or a percentage of the clip).
//!
//! Paint order: later tracks are in front of earlier tracks, then a higher `z`, then later
//! declaration order.

use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;

use anyhow::{Context as _, Result, anyhow, bail, ensure};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionKind {
    CrossFade,
    LinearWipe,
    Dissolve,
    /// Zoom-through blend: the incoming clip scales from 125% to 100% while the cut fades.
    ZoomBlur,
}

/// Straight (non-premultiplied) RGBA8 color written as `#rgb`, `#rrggbb`, `#rrggbbaa`,
/// `white`, `black` or `transparent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color(pub [u8; 4]);

impl Color {
    pub const WHITE: Self = Self([255, 255, 255, 255]);
    pub const BLACK: Self = Self([0, 0, 0, 255]);
}

impl FromStr for Color {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> Result<Self> {
        match raw.trim() {
            "white" => return Ok(Self::WHITE),
            "black" => return Ok(Self::BLACK),
            "transparent" => return Ok(Self([0, 0, 0, 0])),
            _ => {}
        }
        let hex = raw
            .trim()
            .strip_prefix('#')
            .ok_or_else(|| anyhow!("color `{raw}` must be #rgb, #rrggbb or #rrggbbaa"))?;
        let digit = |index: usize| -> Result<u8> {
            let character = hex
                .as_bytes()
                .get(index)
                .copied()
                .ok_or_else(|| anyhow!("color `{raw}` is too short"))?;
            let value = (character as char)
                .to_digit(16)
                .ok_or_else(|| anyhow!("color `{raw}` has a non-hex digit"))?;
            Ok(value as u8)
        };
        match hex.len() {
            3 => Ok(Self([
                digit(0)? * 17,
                digit(1)? * 17,
                digit(2)? * 17,
                255,
            ])),
            6 | 8 => {
                let byte = |index: usize| -> Result<u8> { Ok(digit(index)? * 16 + digit(index + 1)?) };
                let alpha = if hex.len() == 8 { byte(6)? } else { 255 };
                Ok(Self([byte(0)?, byte(2)?, byte(4)?, alpha]))
            }
            _ => bail!("color `{raw}` must be #rgb, #rrggbb or #rrggbbaa"),
        }
    }
}

impl Serialize for Color {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let [red, green, blue, alpha] = self.0;
        serializer.serialize_str(&format!("#{red:02x}{green:02x}{blue:02x}{alpha:02x}"))
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        raw.parse().map_err(serde::de::Error::custom)
    }
}

/// A position or size: pixels (`120`, `"120px"`) or a percentage of the frame axis (`"50%"`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Length {
    Pixels(f32),
    Text(String),
}

impl Length {
    fn resolve(&self, axis_pixels: f32) -> Result<f32> {
        match self {
            Self::Pixels(value) => Ok(*value),
            Self::Text(text) => {
                let trimmed = text.trim();
                if let Some(percent) = trimmed.strip_suffix('%') {
                    let value: f32 = percent
                        .trim()
                        .parse()
                        .with_context(|| format!("invalid percentage `{text}`"))?;
                    Ok(axis_pixels * value / 100.0)
                } else {
                    trimmed
                        .strip_suffix("px")
                        .unwrap_or(trimmed)
                        .trim()
                        .parse()
                        .with_context(|| format!("invalid length `{text}`"))
                }
            }
        }
    }
}

/// A time value: seconds, or a string expression (see the module docs).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Time {
    Seconds(f64),
    Expression(String),
}

/// Frames per second: a whole number or a `"num/den"` string such as `"30000/1001"`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Fps {
    Whole(u32),
    Rational(String),
}

impl Fps {
    fn parts(&self) -> Result<(u32, u32)> {
        let (numerator, denominator) = match self {
            Self::Whole(value) => (*value, 1),
            Self::Rational(text) => {
                let (numerator, denominator) = text
                    .split_once('/')
                    .ok_or_else(|| anyhow!("fps `{text}` must be a number or `num/den`"))?;
                (
                    numerator.trim().parse().context("fps numerator")?,
                    denominator.trim().parse().context("fps denominator")?,
                )
            }
        };
        ensure!(numerator > 0 && denominator > 0, "fps must be positive");
        Ok((numerator, denominator))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Composition {
    pub id: String,
    pub width: u32,
    pub height: u32,
    #[serde(default = "default_fps")]
    pub fps: Fps,
    /// Total length; defaults to the latest explicit clip end.
    #[serde(default)]
    pub duration: Option<Time>,
    #[serde(default)]
    pub background: Option<Color>,
    /// Declared inputs, referenced as `{{name}}` in text, captions and asset paths.
    #[serde(default)]
    pub variables: BTreeMap<String, String>,
    #[serde(default)]
    pub tracks: Vec<Track>,
}

fn default_fps() -> Fps {
    Fps::Whole(30)
}

impl Composition {
    pub fn from_json(input: &str) -> Result<Self> {
        serde_json::from_str(input).context("invalid JSON composition")
    }

    pub fn from_toml(input: &str) -> Result<Self> {
        toml::from_str(input).context("invalid TOML composition")
    }

    /// Parses by file extension (`json` or `toml`).
    pub fn from_text(extension: &str, input: &str) -> Result<Self> {
        match extension.to_ascii_lowercase().as_str() {
            "json" => Self::from_json(input),
            "toml" => Self::from_toml(input),
            other => bail!("unsupported composition format `.{other}` (use .json or .toml)"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Track {
    pub id: String,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub clips: Vec<Clip>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clip {
    pub id: String,
    #[serde(default)]
    pub start: Option<Time>,
    #[serde(default)]
    pub duration: Option<Time>,
    #[serde(default)]
    pub end: Option<Time>,
    #[serde(default)]
    pub z: i32,
    pub layer: Layer,
    /// Anchor position; defaults to the frame center.
    #[serde(default)]
    pub x: Option<Length>,
    #[serde(default)]
    pub y: Option<Length>,
    /// Box size; defaults to the full frame (shapes, images, video) or the text extent.
    #[serde(default)]
    pub width: Option<Length>,
    #[serde(default)]
    pub height: Option<Length>,
    /// Anchor inside the box (0..1); `[0.5, 0.5]` is the center.
    #[serde(default = "default_anchor")]
    pub anchor: [f32; 2],
    #[serde(default)]
    pub rotation: f32,
    #[serde(default = "one")]
    pub scale: f32,
    #[serde(default = "one")]
    pub opacity: f32,
    #[serde(default = "one")]
    pub volume: f32,
    /// Offset into the source media (video, audio) at the clip's first frame.
    #[serde(default)]
    pub media_start: Option<Time>,
    #[serde(default = "one")]
    pub playback_rate: f32,
    #[serde(default)]
    pub animate: Vec<Animation>,
    #[serde(default)]
    pub transition_in: Option<Transition>,
    #[serde(default)]
    pub transition_out: Option<Transition>,
}

fn default_anchor() -> [f32; 2] {
    [0.5, 0.5]
}

fn one() -> f32 {
    1.0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Layer {
    Shape(ShapeLayer),
    Text(TextLayer),
    Image(MediaLayer),
    Video(MediaLayer),
    Audio(AudioLayer),
    /// Timed captions; unless positioned they sit at 85% of the frame height, wrapped at 80% of its width.
    Captions(CaptionsLayer),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShapeKind {
    #[default]
    Rect,
    Ellipse,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Fill {
    Solid(Color),
    Gradient {
        from: Color,
        to: Color,
        /// Degrees; 0 runs left to right, 90 top to bottom.
        #[serde(default)]
        angle: f32,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stroke {
    pub color: Color,
    pub width: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShapeLayer {
    #[serde(default)]
    pub shape: ShapeKind,
    pub fill: Fill,
    #[serde(default)]
    pub stroke: Option<Stroke>,
    /// Corner radius in pixels for `rect`.
    #[serde(default)]
    pub radius: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Weight {
    #[default]
    Regular,
    Bold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Align {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextLayer {
    pub text: String,
    /// TTF/OTF path resolved by the asset source; defaults to the bundled IBM Plex Sans.
    #[serde(default)]
    pub font: Option<String>,
    #[serde(default = "default_text_size")]
    pub size: f32,
    #[serde(default)]
    pub weight: Weight,
    #[serde(default = "default_white")]
    pub color: Color,
    #[serde(default)]
    pub align: Align,
    #[serde(default = "default_line_height")]
    pub line_height: f32,
    #[serde(default)]
    pub letter_spacing: f32,
}

fn default_text_size() -> f32 {
    64.0
}

fn default_white() -> Color {
    Color::WHITE
}

fn default_line_height() -> f32 {
    1.25
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fit {
    #[default]
    Cover,
    Contain,
    Fill,
}

/// Image or video layer: `src` is resolved by the asset source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaLayer {
    pub src: String,
    #[serde(default)]
    pub fit: Fit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioLayer {
    pub src: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptionWord {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

/// A caption cue with clip-local times in seconds; `words` enables the active-word highlight.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptionCue {
    pub start: f64,
    pub end: f64,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub words: Vec<CaptionWord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptionsLayer {
    /// `.srt` or `.vtt` file, merged with the inline `cues`.
    #[serde(default)]
    pub src: Option<String>,
    #[serde(default)]
    pub cues: Vec<CaptionCue>,
    #[serde(default = "default_caption_size")]
    pub size: f32,
    #[serde(default)]
    pub weight: Weight,
    #[serde(default = "default_white")]
    pub color: Color,
    /// Color of the word being spoken (needs `words`).
    #[serde(default)]
    pub highlight: Option<Color>,
}

fn default_caption_size() -> f32 {
    48.0
}

/// Keyframe-animatable clip property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Property {
    X,
    Y,
    Width,
    Height,
    Scale,
    ScaleX,
    ScaleY,
    Rotation,
    Opacity,
    Volume,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Animation {
    pub property: Property,
    pub keyframes: Vec<Keyframe>,
}

/// A value at a clip-local time. `ease` shapes the segment from this keyframe to the next.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Keyframe {
    pub at: Time,
    pub value: f32,
    #[serde(default)]
    pub ease: Ease,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpringParams {
    #[serde(default = "default_stiffness")]
    pub stiffness: f32,
    #[serde(default = "default_damping_ratio")]
    pub damping_ratio: f32,
}

fn default_stiffness() -> f32 {
    400.0
}

fn default_damping_ratio() -> f32 {
    0.7
}

impl Default for SpringParams {
    fn default() -> Self {
        Self {
            stiffness: default_stiffness(),
            damping_ratio: default_damping_ratio(),
        }
    }
}

/// Segment easing. `spring` is stretched so it settles exactly at the segment end.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ease {
    #[default]
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    EaseOutExpo,
    EaseOutBack,
    EaseOutBounce,
    /// Keeps the starting value until the next keyframe.
    Hold,
    Steps(u32),
    CubicBezier([f32; 4]),
    Spring(SpringParams),
}

impl Ease {
    /// Maps linear segment progress in `[0, 1]` to eased progress (springs and back may overshoot).
    #[must_use]
    pub fn apply(self, progress: f32) -> f32 {
        let u = progress.clamp(0.0, 1.0);
        match self {
            Self::Linear => u,
            Self::EaseIn => cubic_bezier(0.42, 0.0, 1.0, 1.0, u),
            Self::EaseOut => cubic_bezier(0.0, 0.0, 0.58, 1.0, u),
            Self::EaseInOut => cubic_bezier(0.42, 0.0, 0.58, 1.0, u),
            Self::EaseInCubic => u * u * u,
            Self::EaseOutCubic => 1.0 - (1.0 - u).powi(3),
            Self::EaseInOutCubic => {
                if u < 0.5 {
                    4.0 * u * u * u
                } else {
                    1.0 - (-2.0 * u + 2.0).powi(3) / 2.0
                }
            }
            Self::EaseOutExpo => {
                if u >= 1.0 {
                    1.0
                } else {
                    1.0 - 2f32.powf(-10.0 * u)
                }
            }
            Self::EaseOutBack => {
                let overshoot = 1.701_58_f32;
                let shifted = u - 1.0;
                1.0 + (overshoot + 1.0) * shifted.powi(3) + overshoot * shifted * shifted
            }
            Self::EaseOutBounce => bounce_out(u),
            Self::Hold => {
                if u >= 1.0 {
                    1.0
                } else {
                    0.0
                }
            }
            Self::Steps(count) => {
                let steps = count.max(1) as f32;
                if u >= 1.0 {
                    1.0
                } else {
                    (u * steps).floor() / steps
                }
            }
            Self::CubicBezier([x1, y1, x2, y2]) => cubic_bezier(x1, y1, x2, y2, u),
            Self::Spring(params) => settled_spring(params, u),
        }
    }
}

fn bounce_out(u: f32) -> f32 {
    let (n, d) = (7.5625_f32, 2.75_f32);
    if u < 1.0 / d {
        n * u * u
    } else if u < 2.0 / d {
        let t = u - 1.5 / d;
        n * t * t + 0.75
    } else if u < 2.5 / d {
        let t = u - 2.25 / d;
        n * t * t + 0.9375
    } else {
        let t = u - 2.625 / d;
        n * t * t + 0.984_375
    }
}

fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, u: f32) -> f32 {
    if u <= 0.0 {
        return 0.0;
    }
    if u >= 1.0 {
        return 1.0;
    }
    let (x1, y1, x2, y2, target) = (
        f64::from(x1),
        f64::from(y1),
        f64::from(x2),
        f64::from(y2),
        f64::from(u),
    );
    let curve = |a: f64, b: f64, t: f64| {
        let inverse = 1.0 - t;
        3.0 * inverse * inverse * t * a + 3.0 * inverse * t * t * b + t * t * t
    };
    let slope = |a: f64, b: f64, t: f64| {
        let inverse = 1.0 - t;
        3.0 * inverse * inverse * a + 6.0 * inverse * t * (b - a) + 3.0 * t * t * (1.0 - b)
    };
    let mut t = target;
    for _ in 0..8 {
        let error = curve(x1, x2, t) - target;
        if error.abs() < 1e-9 {
            return curve(y1, y2, t) as f32;
        }
        let derivative = slope(x1, x2, t);
        if derivative.abs() < 1e-7 {
            break;
        }
        t -= error / derivative;
    }
    let (mut low, mut high) = (0.0_f64, 1.0_f64);
    t = target;
    for _ in 0..64 {
        let error = curve(x1, x2, t) - target;
        if error.abs() < 1e-9 {
            break;
        }
        if error > 0.0 {
            high = t;
        } else {
            low = t;
        }
        t = (low + high) / 2.0;
    }
    curve(y1, y2, t) as f32
}

/// Material 3 Expressive spring (`brand::design::sample_spring_position`) with its time axis
/// stretched so the envelope is below 0.1% at progress 1, which then snaps to exactly 1.
fn settled_spring(params: SpringParams, u: f32) -> f32 {
    if u <= 0.0 {
        return 0.0;
    }
    if u >= 1.0 {
        return 1.0;
    }
    let omega = params.stiffness.max(1.0).sqrt();
    let damping = params.damping_ratio.max(0.05);
    let decay_rate = if damping < 1.0 {
        damping * omega
    } else {
        omega * (damping - (damping * damping - 1.0).sqrt())
    };
    let settle_seconds = (1000f32.ln() + 1.0) / decay_rate.max(1e-3);
    brand::design::sample_spring_position(params.stiffness.max(1.0), damping, u * settle_seconds)
}

/// Remotion-style clamped linear interpolation between input and output ranges.
#[must_use]
pub fn interpolate_clamped(
    value: f32,
    input_start: f32,
    input_end: f32,
    output_start: f32,
    output_end: f32,
) -> f32 {
    if (input_end - input_start).abs() < f32::EPSILON {
        return output_start;
    }
    let normalized = ((value - input_start) / (input_end - input_start)).clamp(0.0, 1.0);
    output_start + normalized * (output_end - output_start)
}

/// Evaluates the raw (unstretched) Material 3 Expressive spring at a frame index.
#[must_use]
pub fn spring_progress(frame: u64, fps: f32, stiffness: f32, damping_ratio: f32) -> f32 {
    let t_secs = frame as f32 / fps.max(1.0);
    brand::design::sample_spring_position(stiffness, damping_ratio, t_secs)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transition {
    pub kind: TransitionKind,
    pub duration: Time,
    /// Edge softness for `linear_wipe`, as a fraction of the frame width.
    #[serde(default = "default_feather")]
    pub feather: f32,
    #[serde(default)]
    pub ease: Ease,
}

fn default_feather() -> f32 {
    0.08
}

/// How far a clip is blended into (or out of) the picture below it on one frame.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TransitionState {
    pub kind: TransitionKind,
    /// 0 shows only what is below the clip, 1 shows the clip fully.
    pub progress: f32,
    pub feather: f32,
}

// ---------------------------------------------------------------------------------------------
// Time expressions
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
enum Amount {
    Seconds(f64),
    Frames(i64),
}

#[derive(Debug, Clone, PartialEq)]
enum TimeExpression {
    Amount(Amount),
    Percent(f64),
    Reference { id: String, offset: Amount },
}

fn parse_amount(raw: &str) -> Result<Amount> {
    let text = raw.trim();
    if let Some(frames) = text.strip_suffix('f') {
        return Ok(Amount::Frames(
            frames
                .trim()
                .parse()
                .with_context(|| format!("invalid frame count `{raw}`"))?,
        ));
    }
    let seconds = text.strip_suffix('s').unwrap_or(text).trim();
    let value: f64 = seconds
        .parse()
        .with_context(|| format!("invalid time `{raw}`"))?;
    ensure!(value.is_finite(), "time `{raw}` is not finite");
    Ok(Amount::Seconds(value))
}

fn parse_time(time: &Time) -> Result<TimeExpression> {
    let text = match time {
        Time::Seconds(value) => {
            ensure!(value.is_finite(), "time {value} is not finite");
            return Ok(TimeExpression::Amount(Amount::Seconds(*value)));
        }
        Time::Expression(text) => text.trim(),
    };
    if let Some(percent) = text.strip_suffix('%') {
        return Ok(TimeExpression::Percent(
            percent
                .trim()
                .parse()
                .with_context(|| format!("invalid percentage `{text}`"))?,
        ));
    }
    if let Ok(amount) = parse_amount(text) {
        return Ok(TimeExpression::Amount(amount));
    }
    let (id, offset) = if let Some((id, offset)) = text.split_once(" + ") {
        (id, parse_amount(offset)?)
    } else if let Some((id, offset)) = text.split_once(" - ") {
        let negated = match parse_amount(offset)? {
            Amount::Seconds(value) => Amount::Seconds(-value),
            Amount::Frames(value) => Amount::Frames(-value),
        };
        (id, negated)
    } else {
        (text, Amount::Seconds(0.0))
    };
    ensure!(
        !id.is_empty() && !id.contains(char::is_whitespace),
        "invalid time expression `{text}`"
    );
    Ok(TimeExpression::Reference {
        id: id.to_string(),
        offset,
    })
}

// ---------------------------------------------------------------------------------------------
// Timeline
// ---------------------------------------------------------------------------------------------

/// One clip with its start and end resolved to integer frames.
#[derive(Debug, Clone)]
pub struct ResolvedClip {
    pub track: usize,
    pub index_in_track: usize,
    pub start_frame: u64,
    /// Exclusive.
    pub end_frame: u64,
    animations: Vec<(Property, Vec<ResolvedKeyframe>)>,
    transition_in: Option<ResolvedTransition>,
    transition_out: Option<ResolvedTransition>,
    media_start_seconds: f64,
}

#[derive(Debug, Clone, Copy)]
struct ResolvedKeyframe {
    seconds: f64,
    value: f32,
    ease: Ease,
}

#[derive(Debug, Clone, Copy)]
struct ResolvedTransition {
    kind: TransitionKind,
    frames: u64,
    feather: f32,
    ease: Ease,
}

/// Resolved per-frame values of one clip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Props {
    pub x: f32,
    pub y: f32,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub scale_x: f32,
    pub scale_y: f32,
    pub rotation: f32,
    pub opacity: f32,
    pub volume: f32,
}

/// A clip that is visible (or audible) on one frame.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneItem {
    /// Index into [`Timeline::clip`] and [`Timeline::resolved`].
    pub clip: usize,
    pub props: Props,
    pub local_frame: u64,
    pub local_seconds: f64,
    /// `media_start + local_seconds * playback_rate`, the position inside video or audio sources.
    pub source_seconds: f64,
    pub transition: Option<TransitionState>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FrameScene {
    pub frame: u64,
    pub seconds: f64,
    /// Back to front.
    pub items: Vec<SceneItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AudioCue {
    pub clip_id: String,
    pub src: String,
    pub start_frame: u64,
    pub end_frame: u64,
    pub start_seconds: f64,
    pub source_offset_seconds: f64,
    pub playback_rate: f32,
    pub volume: f32,
}

#[derive(Debug, Clone)]
pub struct Timeline {
    composition: Composition,
    fps_num: u32,
    fps_den: u32,
    duration_frames: u64,
    resolved: Vec<ResolvedClip>,
    paint_order: Vec<usize>,
}

impl Timeline {
    /// Resolves variables, time expressions, animations and paint order.
    pub fn new(composition: &Composition, overrides: &BTreeMap<String, String>) -> Result<Self> {
        let mut composition = composition.clone();
        ensure!(
            composition.width > 0 && composition.height > 0,
            "composition `{}` needs a non-zero width and height",
            composition.id
        );
        let (fps_num, fps_den) = composition.fps.parts()?;

        let mut variables = composition.variables.clone();
        for (name, value) in overrides {
            ensure!(
                variables.contains_key(name),
                "variable `{name}` is not declared in composition `{}`",
                composition.id
            );
            variables.insert(name.clone(), value.clone());
        }
        substitute_variables(&mut composition, &variables)?;

        let mut ids: HashMap<String, usize> = HashMap::new();
        let mut flat: Vec<(usize, usize)> = Vec::new();
        for (track_index, track) in composition.tracks.iter().enumerate() {
            for (clip_index, clip) in track.clips.iter().enumerate() {
                ensure!(
                    ids.insert(clip.id.clone(), flat.len()).is_none(),
                    "duplicate clip id `{}`",
                    clip.id
                );
                flat.push((track_index, clip_index));
            }
        }

        let frames_per_second = f64::from(fps_num) / f64::from(fps_den);
        let seconds_of = |amount: Amount| match amount {
            Amount::Seconds(value) => value,
            Amount::Frames(frames) => frames as f64 / frames_per_second,
        };

        let mut starts: Vec<Option<f64>> = vec![None; flat.len()];
        let mut ends: Vec<Option<f64>> = vec![None; flat.len()];
        let mut open_ended = vec![false; flat.len()];
        let mut pending: Vec<usize> = (0..flat.len()).collect();
        while !pending.is_empty() {
            let before = pending.len();
            let mut still_pending = Vec::new();
            for &index in &pending {
                let (track_index, clip_index) = flat[index];
                let clip = &composition.tracks[track_index].clips[clip_index];
                let lookup = |id: &str| -> Result<Option<f64>> {
                    let position = ids.get(id).ok_or_else(|| {
                        anyhow!("clip `{}` references unknown clip `{id}`", clip.id)
                    })?;
                    ensure!(
                        !open_ended[*position],
                        "clip `{}` references `{id}`, which has no end (give it a duration or end)",
                        clip.id
                    );
                    Ok(ends[*position])
                };
                let start = match clip.start.as_ref().map(parse_time).transpose()? {
                    None => Some(0.0),
                    Some(TimeExpression::Amount(amount)) => Some(seconds_of(amount)),
                    Some(TimeExpression::Reference { id, offset }) => {
                        lookup(&id)?.map(|end| end + seconds_of(offset))
                    }
                    Some(TimeExpression::Percent(_)) => {
                        bail!("clip `{}`: start cannot be a percentage", clip.id)
                    }
                };
                let Some(start) = start else {
                    still_pending.push(index);
                    continue;
                };
                let end = match (&clip.duration, &clip.end) {
                    (Some(_), Some(_)) => {
                        bail!("clip `{}` sets both duration and end", clip.id)
                    }
                    (Some(duration), None) => match parse_time(duration)? {
                        TimeExpression::Amount(amount) => Some(Some(start + seconds_of(amount))),
                        _ => bail!("clip `{}`: duration must be seconds or frames", clip.id),
                    },
                    (None, Some(end)) => match parse_time(end)? {
                        TimeExpression::Amount(amount) => Some(Some(seconds_of(amount))),
                        TimeExpression::Reference { id, offset } => {
                            lookup(&id)?.map(|end| Some(end + seconds_of(offset)))
                        }
                        TimeExpression::Percent(_) => {
                            bail!("clip `{}`: end cannot be a percentage", clip.id)
                        }
                    },
                    (None, None) => Some(None),
                };
                let Some(end) = end else {
                    still_pending.push(index);
                    continue;
                };
                starts[index] = Some(start);
                match end {
                    Some(end) => ends[index] = Some(end),
                    None => open_ended[index] = true,
                }
            }
            ensure!(
                still_pending.len() < before,
                "clips {:?} have a circular start or end reference",
                still_pending
                    .iter()
                    .map(|&index| {
                        let (track_index, clip_index) = flat[index];
                        composition.tracks[track_index].clips[clip_index].id.clone()
                    })
                    .collect::<Vec<_>>()
            );
            pending = still_pending;
        }

        let to_frame = |seconds: f64| -> Result<u64> {
            let frame = (seconds * frames_per_second + 1e-6).round();
            ensure!(
                frame >= 0.0 && frame < 1e12,
                "time {seconds}s is outside the timeline"
            );
            Ok(frame as u64)
        };

        let duration_seconds = match composition.duration.as_ref().map(parse_time).transpose()? {
            Some(TimeExpression::Amount(amount)) => seconds_of(amount),
            Some(_) => bail!("composition duration must be seconds or frames"),
            None => ends.iter().flatten().copied().fold(f64::NAN, f64::max),
        };
        ensure!(
            duration_seconds.is_finite() && duration_seconds > 0.0,
            "composition `{}` has no duration: set `duration` or give a clip an end",
            composition.id
        );
        let duration_frames = to_frame(duration_seconds)?;
        ensure!(duration_frames > 0, "composition duration is under one frame");

        let mut resolved = Vec::with_capacity(flat.len());
        for (index, &(track_index, clip_index)) in flat.iter().enumerate() {
            let clip = &composition.tracks[track_index].clips[clip_index];
            let start_seconds = starts[index].unwrap_or(0.0);
            let end_seconds = ends[index].unwrap_or(duration_seconds);
            let start_frame = to_frame(start_seconds)?;
            let end_frame = to_frame(end_seconds)?;
            ensure!(
                end_frame > start_frame,
                "clip `{}` lasts less than one frame ({start_frame}..{end_frame})",
                clip.id
            );
            let clip_seconds = (end_frame - start_frame) as f64 / frames_per_second;
            resolved.push(ResolvedClip {
                track: track_index,
                index_in_track: clip_index,
                start_frame,
                end_frame,
                animations: resolve_animations(clip, clip_seconds, frames_per_second)?,
                transition_in: resolve_transition(
                    clip,
                    clip.transition_in.as_ref(),
                    frames_per_second,
                    end_frame - start_frame,
                )?,
                transition_out: resolve_transition(
                    clip,
                    clip.transition_out.as_ref(),
                    frames_per_second,
                    end_frame - start_frame,
                )?,
                media_start_seconds: match clip.media_start.as_ref().map(parse_time).transpose()? {
                    None => 0.0,
                    Some(TimeExpression::Amount(amount)) => seconds_of(amount),
                    Some(_) => bail!("clip `{}`: media_start must be seconds or frames", clip.id),
                },
            });
        }

        let mut paint_order: Vec<usize> = (0..resolved.len())
            .filter(|&index| !composition.tracks[resolved[index].track].hidden)
            .collect();
        paint_order.sort_by_key(|&index| {
            let entry = &resolved[index];
            let clip = &composition.tracks[entry.track].clips[entry.index_in_track];
            (entry.track, clip.z, index)
        });

        Ok(Self {
            composition,
            fps_num,
            fps_den,
            duration_frames,
            resolved,
            paint_order,
        })
    }

    #[must_use]
    pub fn composition(&self) -> &Composition {
        &self.composition
    }

    #[must_use]
    pub fn fps(&self) -> (u32, u32) {
        (self.fps_num, self.fps_den)
    }

    #[must_use]
    pub fn duration_frames(&self) -> u64 {
        self.duration_frames
    }

    #[must_use]
    pub fn duration_seconds(&self) -> f64 {
        self.frame_seconds(self.duration_frames)
    }

    #[must_use]
    pub fn frame_seconds(&self, frame: u64) -> f64 {
        frame as f64 * f64::from(self.fps_den) / f64::from(self.fps_num)
    }

    #[must_use]
    pub fn resolved(&self) -> &[ResolvedClip] {
        &self.resolved
    }

    #[must_use]
    pub fn clip(&self, index: usize) -> &Clip {
        let entry = &self.resolved[index];
        &self.composition.tracks[entry.track].clips[entry.index_in_track]
    }

    /// Clip indices from back to front.
    #[must_use]
    pub fn paint_order(&self) -> &[usize] {
        &self.paint_order
    }

    /// Everything visible on `frame`, back to front. Audio clips are included; use
    /// [`Timeline::audio_cues`] for the mix schedule.
    pub fn scene(&self, frame: u64) -> Result<FrameScene> {
        let mut items = Vec::new();
        for &index in &self.paint_order {
            let entry = &self.resolved[index];
            if frame < entry.start_frame || frame >= entry.end_frame {
                continue;
            }
            items.push(self.item(index, frame)?);
        }
        Ok(FrameScene {
            frame,
            seconds: self.frame_seconds(frame),
            items,
        })
    }

    fn item(&self, index: usize, frame: u64) -> Result<SceneItem> {
        let entry = &self.resolved[index];
        let clip = self.clip(index);
        let local_frame = frame - entry.start_frame;
        let local_seconds = self.frame_seconds(local_frame);
        let width = self.composition.width as f32;
        let height = self.composition.height as f32;

        let mut props = Props {
            x: match &clip.x {
                Some(length) => length.resolve(width)?,
                None => width / 2.0,
            },
            y: match &clip.y {
                Some(length) => length.resolve(height)?,
                None if matches!(clip.layer, Layer::Captions(_)) => height * 0.85,
                None => height / 2.0,
            },
            width: clip.width.as_ref().map(|l| l.resolve(width)).transpose()?,
            height: clip.height.as_ref().map(|l| l.resolve(height)).transpose()?,
            scale_x: clip.scale,
            scale_y: clip.scale,
            rotation: clip.rotation,
            opacity: clip.opacity,
            volume: clip.volume,
        };
        for (property, keyframes) in &entry.animations {
            let value = sample_keyframes(keyframes, local_seconds);
            match property {
                Property::X => props.x = value,
                Property::Y => props.y = value,
                Property::Width => props.width = Some(value),
                Property::Height => props.height = Some(value),
                Property::Scale => {
                    props.scale_x = value;
                    props.scale_y = value;
                }
                Property::ScaleX => props.scale_x = value,
                Property::ScaleY => props.scale_y = value,
                Property::Rotation => props.rotation = value,
                Property::Opacity => props.opacity = value,
                Property::Volume => props.volume = value,
            }
        }
        props.opacity = props.opacity.clamp(0.0, 1.0);

        let transition = if let Some(spec) = entry
            .transition_in
            .filter(|spec| local_frame < spec.frames)
        {
            Some(TransitionState {
                kind: spec.kind,
                progress: spec.ease.apply(local_frame as f32 / spec.frames as f32),
                feather: spec.feather,
            })
        } else if let Some(spec) = entry
            .transition_out
            .filter(|spec| entry.end_frame - frame <= spec.frames)
        {
            Some(TransitionState {
                kind: spec.kind,
                progress: spec
                    .ease
                    .apply((entry.end_frame - frame) as f32 / spec.frames as f32),
                feather: spec.feather,
            })
        } else {
            None
        };

        Ok(SceneItem {
            clip: index,
            props,
            local_frame,
            local_seconds,
            source_seconds: entry.media_start_seconds
                + local_seconds * f64::from(clip.playback_rate),
            transition,
        })
    }

    /// Audio mix schedule: every audio clip with its frame range and source offset.
    #[must_use]
    pub fn audio_cues(&self) -> Vec<AudioCue> {
        let mut cues = Vec::new();
        for (index, entry) in self.resolved.iter().enumerate() {
            let clip = self.clip(index);
            if let Layer::Audio(audio) = &clip.layer {
                cues.push(AudioCue {
                    clip_id: clip.id.clone(),
                    src: audio.src.clone(),
                    start_frame: entry.start_frame,
                    end_frame: entry.end_frame,
                    start_seconds: self.frame_seconds(entry.start_frame),
                    source_offset_seconds: entry.media_start_seconds,
                    playback_rate: clip.playback_rate,
                    volume: clip.volume,
                });
            }
        }
        cues
    }
}

fn resolve_animations(
    clip: &Clip,
    clip_seconds: f64,
    frames_per_second: f64,
) -> Result<Vec<(Property, Vec<ResolvedKeyframe>)>> {
    let mut resolved = Vec::new();
    for animation in &clip.animate {
        ensure!(
            !animation.keyframes.is_empty(),
            "clip `{}`: animation of {:?} has no keyframes",
            clip.id,
            animation.property
        );
        let mut keyframes = Vec::with_capacity(animation.keyframes.len());
        for keyframe in &animation.keyframes {
            let seconds = match parse_time(&keyframe.at)? {
                TimeExpression::Amount(Amount::Seconds(value)) => value,
                TimeExpression::Amount(Amount::Frames(frames)) => frames as f64 / frames_per_second,
                TimeExpression::Percent(percent) => clip_seconds * percent / 100.0,
                TimeExpression::Reference { .. } => {
                    bail!("clip `{}`: keyframe times cannot reference clips", clip.id)
                }
            };
            keyframes.push(ResolvedKeyframe {
                seconds,
                value: keyframe.value,
                ease: keyframe.ease,
            });
        }
        keyframes.sort_by(|a, b| a.seconds.total_cmp(&b.seconds));
        resolved.push((animation.property, keyframes));
    }
    Ok(resolved)
}

fn resolve_transition(
    clip: &Clip,
    transition: Option<&Transition>,
    frames_per_second: f64,
    clip_frames: u64,
) -> Result<Option<ResolvedTransition>> {
    let Some(transition) = transition else {
        return Ok(None);
    };
    let seconds = match parse_time(&transition.duration)? {
        TimeExpression::Amount(Amount::Seconds(value)) => value,
        TimeExpression::Amount(Amount::Frames(frames)) => frames as f64 / frames_per_second,
        _ => bail!(
            "clip `{}`: transition duration must be seconds or frames",
            clip.id
        ),
    };
    ensure!(seconds > 0.0, "clip `{}`: transition duration must be positive", clip.id);
    let frames = ((seconds * frames_per_second + 1e-6).round() as u64).clamp(1, clip_frames);
    Ok(Some(ResolvedTransition {
        kind: transition.kind,
        frames,
        feather: transition.feather.clamp(0.0, 0.5),
        ease: transition.ease,
    }))
}

fn sample_keyframes(keyframes: &[ResolvedKeyframe], seconds: f64) -> f32 {
    let (Some(first), Some(last)) = (keyframes.first(), keyframes.last()) else {
        return 0.0;
    };
    if seconds <= first.seconds {
        return first.value;
    }
    if seconds >= last.seconds {
        return last.value;
    }
    let next = keyframes.partition_point(|keyframe| keyframe.seconds <= seconds);
    let (from, to) = (keyframes[next - 1], keyframes[next]);
    let span = to.seconds - from.seconds;
    if span <= 0.0 {
        return to.value;
    }
    let progress = from.ease.apply(((seconds - from.seconds) / span) as f32);
    from.value + (to.value - from.value) * progress
}

fn substitute(text: &mut String, variables: &BTreeMap<String, String>) -> Result<()> {
    if !text.contains("{{") {
        return Ok(());
    }
    let mut output = String::with_capacity(text.len());
    let mut rest = text.as_str();
    while let Some(open) = rest.find("{{") {
        output.push_str(&rest[..open]);
        let after = &rest[open + 2..];
        let close = after
            .find("}}")
            .ok_or_else(|| anyhow!("unterminated `{{{{` in `{text}`"))?;
        let name = after[..close].trim();
        let value = variables
            .get(name)
            .ok_or_else(|| anyhow!("undeclared variable `{name}` used in `{text}`"))?;
        output.push_str(value);
        rest = &after[close + 2..];
    }
    output.push_str(rest);
    *text = output;
    Ok(())
}

fn substitute_variables(
    composition: &mut Composition,
    variables: &BTreeMap<String, String>,
) -> Result<()> {
    for track in &mut composition.tracks {
        for clip in &mut track.clips {
            match &mut clip.layer {
                Layer::Text(text) => {
                    substitute(&mut text.text, variables)?;
                    if let Some(font) = &mut text.font {
                        substitute(font, variables)?;
                    }
                }
                Layer::Image(media) | Layer::Video(media) => substitute(&mut media.src, variables)?,
                Layer::Audio(audio) => substitute(&mut audio.src, variables)?,
                Layer::Captions(captions) => {
                    if let Some(src) = &mut captions.src {
                        substitute(src, variables)?;
                    }
                    for cue in &mut captions.cues {
                        substitute(&mut cue.text, variables)?;
                        for word in &mut cue.words {
                            substitute(&mut word.text, variables)?;
                        }
                    }
                }
                Layer::Shape(_) => {}
            }
        }
    }
    Ok(())
}

/// The cue visible at `seconds` (clip-local) and, when the cue has word timings, the index of the
/// word being spoken.
#[must_use]
pub fn active_caption(cues: &[CaptionCue], seconds: f64) -> Option<(&CaptionCue, Option<usize>)> {
    let cue = cues
        .iter()
        .find(|cue| seconds >= cue.start && seconds < cue.end)?;
    let word = cue
        .words
        .iter()
        .position(|word| seconds >= word.start && seconds < word.end);
    Some((cue, word))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(id: &str, extra: &str) -> String {
        format!(
            r##"{{ "id": "{id}", {extra} "layer": {{ "type": "shape", "fill": "#ffffff" }} }}"##
        )
    }

    fn composition(fps: &str, tracks: &str) -> Composition {
        Composition::from_json(&format!(
            r#"{{ "id": "t", "width": 64, "height": 36, "fps": {fps}, {tracks} }}"#
        ))
        .expect("composition parses")
    }

    fn timeline(fps: &str, tracks: &str) -> Timeline {
        Timeline::new(&composition(fps, tracks), &BTreeMap::new()).expect("timeline resolves")
    }

    #[test]
    fn clips_are_active_on_exact_frame_ranges_and_tile_without_gaps() {
        let a = rect("a", r#""start": 0, "duration": 1.0,"#);
        let b = rect("b", r#""start": "a", "duration": 1.5,"#);
        let timeline = timeline(
            "30",
            &format!(r#""tracks": [{{ "id": "main", "clips": [{a}, {b}] }}]"#),
        );
        let [first, second] = timeline.resolved() else {
            panic!("two clips");
        };
        assert_eq!((first.start_frame, first.end_frame), (0, 30));
        assert_eq!((second.start_frame, second.end_frame), (30, 75));
        assert_eq!(timeline.duration_frames(), 75);

        let ids_on = |frame: u64| -> Vec<String> {
            timeline
                .scene(frame)
                .expect("scene")
                .items
                .iter()
                .map(|item| timeline.clip(item.clip).id.clone())
                .collect()
        };
        assert_eq!(ids_on(0), ["a"]);
        assert_eq!(ids_on(29), ["a"]);
        assert_eq!(ids_on(30), ["b"]);
        assert_eq!(ids_on(74), ["b"]);
        assert!(ids_on(75).is_empty());
    }

    #[test]
    fn rational_fps_and_frame_literals_round_to_whole_frames() {
        let a = rect("a", r#""start": "12f", "duration": "24f","#);
        let b = rect("b", r#""start": "a + 6f", "end": "a + 1","#);
        let timeline = timeline(
            r#""30000/1001""#,
            &format!(r#""tracks": [{{ "id": "main", "clips": [{a}, {b}] }}]"#),
        );
        assert_eq!(timeline.fps(), (30000, 1001));
        let [first, second] = timeline.resolved() else {
            panic!("two clips");
        };
        assert_eq!((first.start_frame, first.end_frame), (12, 36));
        assert_eq!(second.start_frame, 42);
        // a ends at 36f (1.2012s); "a + 1" is that plus one second = 66.03 frames.
        assert_eq!(second.end_frame, 66);
        assert!((timeline.frame_seconds(30000) - 1001.0).abs() < 1e-9);
    }

    #[test]
    fn invalid_timelines_are_rejected_with_the_clip_named() {
        let cycle = composition(
            "30",
            &format!(
                r#""tracks": [{{ "id": "m", "clips": [{}, {}] }}]"#,
                rect("a", r#""start": "b", "duration": 1,"#),
                rect("b", r#""start": "a", "duration": 1,"#),
            ),
        );
        let error = Timeline::new(&cycle, &BTreeMap::new()).expect_err("cycle");
        assert!(error.to_string().contains("circular"), "{error}");

        let open = composition(
            "30",
            &format!(
                r#""duration": 3, "tracks": [{{ "id": "m", "clips": [{}, {}] }}]"#,
                rect("a", ""),
                rect("b", r#""start": "a","#),
            ),
        );
        let error = Timeline::new(&open, &BTreeMap::new()).expect_err("open reference");
        assert!(error.to_string().contains("no end"), "{error}");

        let duplicate = composition(
            "30",
            &format!(
                r#""duration": 1, "tracks": [{{ "id": "m", "clips": [{}, {}] }}]"#,
                rect("a", ""),
                rect("a", ""),
            ),
        );
        assert!(Timeline::new(&duplicate, &BTreeMap::new()).is_err());

        let none = composition("30", r#""tracks": []"#);
        assert!(Timeline::new(&none, &BTreeMap::new()).is_err());
    }

    #[test]
    fn layers_paint_by_track_then_z_then_declaration_order() {
        let low = rect("low", r#""duration": 1, "z": 5,"#);
        let high = rect("high", r#""duration": 1, "z": 9,"#);
        let late = rect("late", r#""duration": 1, "z": 5,"#);
        let top_track = rect("top_track", r#""duration": 1, "z": -100,"#);
        let hidden = rect("hidden", r#""duration": 1,"#);
        let timeline = timeline(
            "10",
            &format!(
                r#""tracks": [
                    {{ "id": "back", "clips": [{high}, {low}] }},
                    {{ "id": "front", "clips": [{late}, {top_track}] }},
                    {{ "id": "off", "hidden": true, "clips": [{hidden}] }}
                ]"#
            ),
        );
        let order: Vec<String> = timeline
            .scene(0)
            .expect("scene")
            .items
            .iter()
            .map(|item| timeline.clip(item.clip).id.clone())
            .collect();
        assert_eq!(order, ["low", "high", "top_track", "late"]);
    }

    #[test]
    fn keyframes_interpolate_per_frame_with_holds_and_percentages() {
        let clip = r##"{ "id": "a", "duration": 2,
            "layer": { "type": "shape", "fill": "#fff" },
            "animate": [
              { "property": "opacity", "keyframes": [
                  { "at": 0, "value": 0 },
                  { "at": "50%", "value": 1, "ease": "hold" },
                  { "at": "100%", "value": 0.25 } ] },
              { "property": "x", "keyframes": [ { "at": "0f", "value": 10 }, { "at": "20f", "value": 30 } ] } ] }"##;
        let timeline = timeline("20", &format!(r#""tracks": [{{ "id": "m", "clips": [{clip}] }}]"#));
        let at = |frame: u64| timeline.scene(frame).expect("scene").items[0].props;
        assert!((at(0).opacity - 0.0).abs() < 1e-6);
        assert!((at(5).opacity - 0.25).abs() < 1e-6);
        assert!((at(10).opacity - 0.5).abs() < 1e-6);
        assert!((at(20).opacity - 1.0).abs() < 1e-6);
        assert!((at(39).opacity - 1.0).abs() < 1e-6, "hold keeps the value until the next keyframe");
        assert!((at(10).x - 20.0).abs() < 1e-5);
        assert!((at(25).x - 30.0).abs() < 1e-6);
        assert_eq!(at(0).y, 18.0);
    }

    #[test]
    fn overdamped_spring_is_monotonic_and_settles_exactly_on_the_last_frame() {
        let ease = Ease::Spring(SpringParams {
            stiffness: 300.0,
            damping_ratio: 1.4,
        });
        let samples: Vec<f32> = (0..=240).map(|step| ease.apply(step as f32 / 240.0)).collect();
        assert_eq!(samples[0], 0.0);
        assert_eq!(samples[240], 1.0);
        for pair in samples.windows(2) {
            assert!(pair[1] >= pair[0] - 1e-6, "not monotonic: {pair:?}");
        }
        assert!(samples.iter().all(|value| *value <= 1.0 + 1e-6));
        assert!((samples[239] - 1.0).abs() < 2e-3, "{}", samples[239]);
    }

    #[test]
    fn underdamped_spring_overshoots_with_decaying_peaks_then_settles() {
        let ease = Ease::Spring(SpringParams {
            stiffness: 400.0,
            damping_ratio: 0.3,
        });
        let samples: Vec<f32> = (0..=2000).map(|step| ease.apply(step as f32 / 2000.0)).collect();
        let mut peaks = Vec::new();
        for window in samples.windows(3) {
            if window[1] > window[0] && window[1] >= window[2] && window[1] > 1.0 {
                peaks.push(window[1] - 1.0);
            }
        }
        assert!(peaks.len() >= 2, "expected oscillation, got {peaks:?}");
        for pair in peaks.windows(2) {
            assert!(pair[1] < pair[0], "peaks must shrink: {peaks:?}");
        }
        assert_eq!(samples[2000], 1.0);
        assert!((samples[1990] - 1.0).abs() < 2e-3);
    }

    #[test]
    fn named_easings_hit_their_endpoints_and_bezier_matches_linear() {
        let eases = [
            Ease::Linear,
            Ease::EaseIn,
            Ease::EaseOut,
            Ease::EaseInOut,
            Ease::EaseInCubic,
            Ease::EaseOutCubic,
            Ease::EaseInOutCubic,
            Ease::EaseOutExpo,
            Ease::EaseOutBack,
            Ease::EaseOutBounce,
            Ease::Steps(4),
            Ease::CubicBezier([0.25, 0.1, 0.25, 1.0]),
            Ease::Spring(SpringParams::default()),
        ];
        for ease in eases {
            assert!(ease.apply(0.0).abs() < 1e-6, "{ease:?} start");
            assert!((ease.apply(1.0) - 1.0).abs() < 1e-6, "{ease:?} end");
        }
        let linear = Ease::CubicBezier([1.0 / 3.0, 1.0 / 3.0, 2.0 / 3.0, 2.0 / 3.0]);
        for step in 0..=10 {
            let u = step as f32 / 10.0;
            assert!((linear.apply(u) - u).abs() < 1e-4);
        }
        assert!(Ease::EaseOutBack.apply(0.8) > 1.0);
    }

    #[test]
    fn transitions_report_progress_in_and_out_of_a_clip() {
        let clip = r##"{ "id": "a", "start": 1, "duration": 3,
            "layer": { "type": "shape", "fill": "#fff" },
            "transition_in": { "kind": "cross_fade", "duration": 0.5 },
            "transition_out": { "kind": "linear_wipe", "duration": "5f" } }"##;
        let timeline = timeline("10", &format!(r#""tracks": [{{ "id": "m", "clips": [{clip}] }}]"#));
        let state = |frame: u64| timeline.scene(frame).expect("scene").items[0].transition;
        assert_eq!(state(10).map(|s| s.progress), Some(0.0));
        assert!((state(12).expect("in").progress - 0.4).abs() < 1e-6);
        assert_eq!(state(15), None);
        assert_eq!(state(34), None);
        assert_eq!(state(35).map(|s| s.progress), Some(1.0));
        assert_eq!(state(35).map(|s| s.kind), Some(TransitionKind::LinearWipe));
        assert!((state(39).expect("out").progress - 0.2).abs() < 1e-6);
    }

    #[test]
    fn variables_are_substituted_and_overrides_must_be_declared() {
        let text = r##"{ "id": "t", "duration": 1,
            "layer": { "type": "text", "text": "Hello {{ name }}!" } }"##;
        let composition = Composition::from_json(&format!(
            r#"{{ "id": "v", "width": 64, "height": 36, "variables": {{ "name": "world" }},
                "tracks": [{{ "id": "m", "clips": [{text}] }}] }}"#
        ))
        .expect("parse");
        let rendered = Timeline::new(&composition, &BTreeMap::new()).expect("defaults");
        let Layer::Text(layer) = &rendered.clip(0).layer else {
            panic!("text layer");
        };
        assert_eq!(layer.text, "Hello world!");

        let overrides = BTreeMap::from([("name".to_string(), "Aphrody".to_string())]);
        let rendered = Timeline::new(&composition, &overrides).expect("override");
        let Layer::Text(layer) = &rendered.clip(0).layer else {
            panic!("text layer");
        };
        assert_eq!(layer.text, "Hello Aphrody!");

        let unknown = BTreeMap::from([("nope".to_string(), "x".to_string())]);
        assert!(Timeline::new(&composition, &unknown).is_err());
    }

    #[test]
    fn toml_compositions_parse_with_nested_tracks_and_tables() {
        let source = r##"
            id = "toml"
            width = 128
            height = 72
            fps = 24
            background = "#101820"

            [[tracks]]
            id = "main"

            [[tracks.clips]]
            id = "card"
            start = 0.5
            duration = 1.0
            x = "25%"
            layer = { type = "shape", shape = "ellipse", fill = { from = "#ff0066", to = "#3300ff", angle = 90 } }
            animate = [
              { property = "scale", keyframes = [ { at = 0, value = 0.5, ease = { spring = { stiffness = 300.0, damping_ratio = 0.8 } } }, { at = "100%", value = 1 } ] },
            ]
            transition_in = { kind = "dissolve", duration = 0.25 }

            [[tracks.clips]]
            id = "voice"
            duration = 2
            media_start = 1.5
            layer = { type = "audio", src = "voice.wav" }
        "##;
        let composition = Composition::from_toml(source).expect("toml parses");
        let timeline = Timeline::new(&composition, &BTreeMap::new()).expect("resolves");
        assert_eq!(timeline.duration_frames(), 48);
        let scene = timeline.scene(12).expect("scene");
        assert_eq!(scene.items.len(), 2);
        assert!((scene.items[0].props.x - 32.0).abs() < 1e-4);
        let cues = timeline.audio_cues();
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].source_offset_seconds, 1.5);
        assert_eq!((cues[0].start_frame, cues[0].end_frame), (0, 48));
        assert!((scene.items[1].source_seconds - 2.0).abs() < 1e-9);
    }

    #[test]
    fn unknown_fields_are_rejected_so_typos_fail_loudly() {
        let error = Composition::from_json(
            r#"{ "id": "t", "width": 4, "height": 4, "tracks": [], "durration": 1 }"#,
        )
        .expect_err("typo");
        assert!(format!("{error:#}").contains("durration"));
    }

    #[test]
    fn captions_expose_the_active_cue_and_word() {
        let cues = vec![CaptionCue {
            start: 1.0,
            end: 3.0,
            text: String::new(),
            words: vec![
                CaptionWord { start: 1.0, end: 1.5, text: "Hello".into() },
                CaptionWord { start: 1.5, end: 3.0, text: "world".into() },
            ],
        }];
        assert!(active_caption(&cues, 0.99).is_none());
        assert_eq!(active_caption(&cues, 1.2).map(|(_, word)| word), Some(Some(0)));
        assert_eq!(active_caption(&cues, 1.5).map(|(_, word)| word), Some(Some(1)));
        assert!(active_caption(&cues, 3.0).is_none());
    }

    #[test]
    fn colors_parse_hex_and_names() {
        assert_eq!("#fff".parse::<Color>().expect("short"), Color::WHITE);
        assert_eq!("#10203040".parse::<Color>().expect("alpha").0, [0x10, 0x20, 0x30, 0x40]);
        assert_eq!("transparent".parse::<Color>().expect("name").0[3], 0);
        assert!("#12".parse::<Color>().is_err());
        assert!("red".parse::<Color>().is_err());
        assert!((interpolate_clamped(45.0, 30.0, 60.0, 0.0, 1.0) - 0.5).abs() < 1e-5);
        assert!(spring_progress(30, 60.0, 1400.0, 0.9) > spring_progress(0, 60.0, 1400.0, 0.9));
    }
}
