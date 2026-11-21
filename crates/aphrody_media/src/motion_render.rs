//! Deterministic frame renderer for [`crate::motion`] timelines.
//!
//! Layers are rasterized with `tiny-skia` (shapes, gradients, glyph outlines from the bundled IBM
//! Plex Sans, bilinear image sampling) into premultiplied RGBA8 frames. Floating point work is
//! plain IEEE `f32` with no threading, so the same composition yields the same bytes on every run.
//! Clip transitions are delegated to a [`TransitionBlender`]: [`CpuBlender`] is the reference and
//! `aphrody_cli` provides a GPU blender that runs the `motion_transition` WGSL shader.
//!
//! With the `video-encode` feature, [`Mp4Encoder`] turns frames into an H.264 MP4 with the
//! pure-Rust `craft-video` encoder (no FFmpeg).

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context as _, Result, anyhow, bail, ensure};
pub use tiny_skia;
use tiny_skia::{
    ColorU8, FillRule, FilterQuality, GradientStop, LinearGradient, Paint, Path,
    PathBuilder, Pattern, Pixmap, Point, Rect, SpreadMode, Stroke as SkiaStroke, Transform,
};

use crate::motion::{
    Align, CaptionCue, CaptionWord, Color, Fill, Fit, Layer, Props, SceneItem, ShapeKind,
    ShapeLayer, Timeline, TransitionKind, TransitionState, Weight, active_caption,
};
use crate::subtitles::{SubtitleFormat, parse_subtitles};

const FONT_REGULAR: &[u8] = include_bytes!("../../../assets/fonts/ibm-plex-sans/IBMPlexSans-Regular.ttf");
const FONT_BOLD: &[u8] = include_bytes!("../../../assets/fonts/ibm-plex-sans/IBMPlexSans-SemiBold.ttf");

/// Where fonts, images, caption files and decoded video frames come from.
pub trait Assets {
    /// Raw bytes of `src` (an image, a font, an `.srt`/`.vtt` file).
    fn read(&mut self, src: &str) -> Result<Vec<u8>>;

    /// The decoded video frame of `src` at `seconds` (premultiplied RGBA8).
    fn video_frame(&mut self, src: &str, _seconds: f64) -> Result<Pixmap> {
        bail!("no video decoder is wired into this asset source; cannot read `{src}`")
    }
}

/// Asset source for compositions that only use shapes and text with the bundled font.
pub struct NoAssets;

impl Assets for NoAssets {
    fn read(&mut self, src: &str) -> Result<Vec<u8>> {
        bail!("asset `{src}` requested but no asset source is configured")
    }
}

/// Blends the picture below a clip (`before`) with the picture including it (`after`).
pub trait TransitionBlender {
    fn blend(&mut self, before: &Pixmap, after: &Pixmap, state: TransitionState)
    -> Result<Pixmap>;
}

/// Reference blender: the same math as `MOTION_TRANSITION_WGSL`, evaluated on the CPU.
pub struct CpuBlender;

impl TransitionBlender for CpuBlender {
    fn blend(
        &mut self,
        before: &Pixmap,
        after: &Pixmap,
        state: TransitionState,
    ) -> Result<Pixmap> {
        ensure!(
            before.width() == after.width() && before.height() == after.height(),
            "transition inputs differ in size"
        );
        let (width, height) = (before.width(), before.height());
        let mut output = Pixmap::new(width, height).ok_or_else(|| anyhow!("empty frame"))?;
        let progress = state.progress.clamp(0.0, 1.0);
        let feather = state.feather.max(0.0005);
        let source_a = before.data();
        let source_b = after.data();
        let destination = output.data_mut();

        let zoom_b = 1.0 + 0.25 * (1.0 - progress);
        let zoom_a = 1.0 + 0.25 * progress;
        for y in 0..height {
            for x in 0..width {
                let offset = ((y * width + x) * 4) as usize;
                let (a, b): ([f32; 4], [f32; 4]) = if state.kind == TransitionKind::ZoomBlur {
                    (
                        sample_zoomed(before, x, y, zoom_a),
                        sample_zoomed(after, x, y, zoom_b),
                    )
                } else {
                    (read_pixel(source_a, offset), read_pixel(source_b, offset))
                };
                let mixed = match state.kind {
                    TransitionKind::CrossFade | TransitionKind::ZoomBlur => mix4(a, b, progress),
                    TransitionKind::LinearWipe => {
                        let u = (x as f32 + 0.5) / width as f32;
                        let edge_center = progress * (1.0 + 2.0 * feather) - feather;
                        let edge = smoothstep(edge_center - feather, edge_center + feather, u);
                        mix4(b, a, edge)
                    }
                    TransitionKind::Dissolve => {
                        if dissolve_noise(x, y) < progress {
                            b
                        } else {
                            a
                        }
                    }
                };
                for channel in 0..4 {
                    destination[offset + channel] =
                        (mixed[channel] * 255.0 + 0.5).clamp(0.0, 255.0) as u8;
                }
            }
        }
        Ok(output)
    }
}

/// Per-pixel threshold in `[0, 1)` for the dissolve transition; the WGSL shader uses the same hash.
#[must_use]
pub fn dissolve_noise(x: u32, y: u32) -> f32 {
    let mut value = x.wrapping_mul(0x9E37_79B1) ^ y.wrapping_mul(0x85EB_CA77) ^ 0x27D4_EB2F;
    value ^= value >> 15;
    value = value.wrapping_mul(0x2C1B_3C6D);
    value ^= value >> 12;
    value = value.wrapping_mul(0x297A_2D39);
    value ^= value >> 15;
    (value >> 8) as f32 / 16_777_216.0
}

fn read_pixel(data: &[u8], offset: usize) -> [f32; 4] {
    [
        f32::from(data[offset]) / 255.0,
        f32::from(data[offset + 1]) / 255.0,
        f32::from(data[offset + 2]) / 255.0,
        f32::from(data[offset + 3]) / 255.0,
    ]
}

fn mix4(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Samples `pixmap` as if scaled by `zoom` around the frame center (bilinear, clamped edges).
fn sample_zoomed(pixmap: &Pixmap, x: u32, y: u32, zoom: f32) -> [f32; 4] {
    let (width, height) = (pixmap.width(), pixmap.height());
    let data = pixmap.data();
    if zoom == 1.0 {
        return read_pixel(data, ((y * width + x) * 4) as usize);
    }
    let center_x = width as f32 / 2.0;
    let center_y = height as f32 / 2.0;
    let source_x = center_x + ((x as f32 + 0.5) - center_x) / zoom - 0.5;
    let source_y = center_y + ((y as f32 + 0.5) - center_y) / zoom - 0.5;
    let x0 = source_x.floor();
    let y0 = source_y.floor();
    let (weight_x, weight_y) = (source_x - x0, source_y - y0);
    let clamp_index = |value: f32, limit: u32| (value.max(0.0) as u32).min(limit - 1);
    let (ix0, ix1) = (clamp_index(x0, width), clamp_index(x0 + 1.0, width));
    let (iy0, iy1) = (clamp_index(y0, height), clamp_index(y0 + 1.0, height));
    let at = |px: u32, py: u32| read_pixel(data, ((py * width + px) * 4) as usize);
    let top = mix4(at(ix0, iy0), at(ix1, iy0), weight_x);
    let bottom = mix4(at(ix0, iy1), at(ix1, iy1), weight_x);
    mix4(top, bottom, weight_y)
}

struct TextLayout {
    width: f32,
    height: f32,
    /// One path per color slot.
    paths: Vec<Option<Path>>,
}

struct GlyphSink<'a> {
    builder: &'a mut PathBuilder,
    scale: f32,
    origin_x: f32,
    baseline: f32,
}

impl ttf_parser::OutlineBuilder for GlyphSink<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        self.builder
            .move_to(self.origin_x + x * self.scale, self.baseline - y * self.scale);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.builder
            .line_to(self.origin_x + x * self.scale, self.baseline - y * self.scale);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.builder.quad_to(
            self.origin_x + x1 * self.scale,
            self.baseline - y1 * self.scale,
            self.origin_x + x * self.scale,
            self.baseline - y * self.scale,
        );
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.builder.cubic_to(
            self.origin_x + x1 * self.scale,
            self.baseline - y1 * self.scale,
            self.origin_x + x2 * self.scale,
            self.baseline - y2 * self.scale,
            self.origin_x + x * self.scale,
            self.baseline - y * self.scale,
        );
    }

    fn close(&mut self) {
        self.builder.close();
    }
}

struct TextStyle {
    size: f32,
    letter_spacing: f32,
    line_height: f32,
    align: Align,
    max_width: Option<f32>,
}

/// Lays out `words` (each with a color slot) on lines and returns glyph outlines per color slot.
/// Characters the font lacks use its `.notdef` glyph; kerning, ligatures, bidi and complex-script
/// shaping are not applied.
fn layout_text(
    font: &[u8],
    words: &[(String, usize)],
    slots: usize,
    style: &TextStyle,
) -> Result<TextLayout> {
    let face = ttf_parser::Face::parse(font, 0).context("unreadable font")?;
    let units = f32::from(face.units_per_em()).max(1.0);
    let scale = style.size / units;
    let ascent = f32::from(face.ascender()) * scale;
    let descent = -f32::from(face.descender()) * scale;
    let line_advance = style.size * style.line_height;

    let advance_of = |character: char| -> f32 {
        let glyph = face.glyph_index(character).unwrap_or(ttf_parser::GlyphId(0));
        f32::from(face.glyph_hor_advance(glyph).unwrap_or(0)) * scale + style.letter_spacing
    };
    let word_width = |word: &str| word.chars().map(advance_of).sum::<f32>();
    let space = advance_of(' ');

    let mut lines: Vec<Vec<usize>> = vec![Vec::new()];
    let mut line_width = 0.0_f32;
    for (index, (word, _)) in words.iter().enumerate() {
        if word == "\n" {
            lines.push(Vec::new());
            line_width = 0.0;
            continue;
        }
        let width = word_width(word);
        let current = lines.last_mut().ok_or_else(|| anyhow!("no line"))?;
        let needed = if current.is_empty() { width } else { line_width + space + width };
        if let Some(limit) = style.max_width
            && !current.is_empty()
            && needed > limit
        {
            lines.push(vec![index]);
            line_width = width;
            continue;
        }
        current.push(index);
        line_width = needed;
    }

    let widths: Vec<f32> = lines
        .iter()
        .map(|line| {
            line.iter()
                .enumerate()
                .map(|(position, &index)| {
                    word_width(&words[index].0) + if position > 0 { space } else { 0.0 }
                })
                .sum()
        })
        .collect();
    let content_width = widths.iter().copied().fold(0.0_f32, f32::max);
    let box_width = style.max_width.unwrap_or(content_width);

    let mut builders: Vec<PathBuilder> = (0..slots).map(|_| PathBuilder::new()).collect();
    for (line_index, line) in lines.iter().enumerate() {
        let baseline =
            line_index as f32 * line_advance + (line_advance - (ascent + descent)) / 2.0 + ascent;
        let mut pen = match style.align {
            Align::Left => 0.0,
            Align::Center => (box_width - widths[line_index]) / 2.0,
            Align::Right => box_width - widths[line_index],
        };
        for (position, &index) in line.iter().enumerate() {
            if position > 0 {
                pen += space;
            }
            let (word, slot) = &words[index];
            let builder = builders
                .get_mut(*slot)
                .ok_or_else(|| anyhow!("color slot {slot} out of range"))?;
            for character in word.chars() {
                let glyph = face.glyph_index(character).unwrap_or(ttf_parser::GlyphId(0));
                let mut sink = GlyphSink {
                    builder,
                    scale,
                    origin_x: pen,
                    baseline,
                };
                face.outline_glyph(glyph, &mut sink);
                pen += advance_of(character);
            }
        }
    }
    Ok(TextLayout {
        width: box_width,
        height: lines.len() as f32 * line_advance,
        paths: builders.into_iter().map(PathBuilder::finish).collect(),
    })
}

fn paint_color(color: Color, opacity: f32) -> tiny_skia::Color {
    let [red, green, blue, alpha] = color.0;
    tiny_skia::Color::from_rgba(
        f32::from(red) / 255.0,
        f32::from(green) / 255.0,
        f32::from(blue) / 255.0,
        (f32::from(alpha) / 255.0 * opacity).clamp(0.0, 1.0),
    )
    .unwrap_or(tiny_skia::Color::TRANSPARENT)
}

fn solid_paint(color: Color, opacity: f32) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color(paint_color(color, opacity));
    paint.anti_alias = true;
    paint
}

fn item_transform(anchor: [f32; 2], props: &Props, box_width: f32, box_height: f32) -> Transform {
    Transform::from_translate(props.x, props.y)
        .pre_concat(Transform::from_rotate(props.rotation))
        .pre_scale(props.scale_x, props.scale_y)
        .pre_translate(-anchor[0] * box_width, -anchor[1] * box_height)
}

fn rounded_rectangle(width: f32, height: f32, radius: f32) -> Option<Path> {
    let radius = radius.clamp(0.0, width.min(height) / 2.0);
    if radius <= 0.0 {
        return Some(PathBuilder::from_rect(Rect::from_xywh(0.0, 0.0, width, height)?));
    }
    let handle = radius * 0.552_284_75;
    let mut builder = PathBuilder::new();
    builder.move_to(radius, 0.0);
    builder.line_to(width - radius, 0.0);
    builder.cubic_to(width - radius + handle, 0.0, width, radius - handle, width, radius);
    builder.line_to(width, height - radius);
    builder.cubic_to(
        width,
        height - radius + handle,
        width - radius + handle,
        height,
        width - radius,
        height,
    );
    builder.line_to(radius, height);
    builder.cubic_to(radius - handle, height, 0.0, height - radius + handle, 0.0, height - radius);
    builder.line_to(0.0, radius);
    builder.cubic_to(0.0, radius - handle, radius - handle, 0.0, radius, 0.0);
    builder.close();
    builder.finish()
}

fn shape_path(shape: &ShapeLayer, width: f32, height: f32) -> Option<Path> {
    match shape.shape {
        ShapeKind::Rect => rounded_rectangle(width, height, shape.radius),
        ShapeKind::Ellipse => PathBuilder::from_oval(Rect::from_xywh(0.0, 0.0, width, height)?),
    }
}

fn fill_paint(fill: &Fill, width: f32, height: f32, opacity: f32) -> Paint<'static> {
    match fill {
        Fill::Solid(color) => solid_paint(*color, opacity),
        Fill::Gradient { from, to, angle } => {
            let radians = angle.to_radians();
            let (dx, dy) = (radians.cos(), radians.sin());
            let half = (dx.abs() * width + dy.abs() * height) / 2.0;
            let (cx, cy) = (width / 2.0, height / 2.0);
            let shader = LinearGradient::new(
                Point::from_xy(cx - dx * half, cy - dy * half),
                Point::from_xy(cx + dx * half, cy + dy * half),
                vec![
                    GradientStop::new(0.0, paint_color(*from, opacity)),
                    GradientStop::new(1.0, paint_color(*to, opacity)),
                ],
                SpreadMode::Pad,
                Transform::identity(),
            );
            match shader {
                Some(shader) => {
                    let mut paint = Paint::default();
                    paint.shader = shader;
                    paint.anti_alias = true;
                    paint
                }
                None => solid_paint(*from, opacity),
            }
        }
    }
}

/// Decodes PNG, JPEG, GIF, BMP, WebP-less formats supported by `image` into a premultiplied pixmap.
pub fn decode_image(bytes: &[u8]) -> Result<Pixmap> {
    let rgba = image::load_from_memory(bytes)
        .context("unsupported or corrupt image")?
        .to_rgba8();
    let (width, height) = rgba.dimensions();
    let mut pixmap = Pixmap::new(width, height).ok_or_else(|| anyhow!("image has no pixels"))?;
    for (target, source) in pixmap.pixels_mut().iter_mut().zip(rgba.pixels()) {
        let [red, green, blue, alpha] = source.0;
        *target = ColorU8::from_rgba(red, green, blue, alpha).premultiply();
    }
    Ok(pixmap)
}

/// Straight-alpha RGBA8 bytes of a frame, for PNG output.
#[must_use]
pub fn demultiplied_rgba(pixmap: &Pixmap) -> Vec<u8> {
    let mut output = Vec::with_capacity(pixmap.data().len());
    for pixel in pixmap.pixels() {
        let color = pixel.demultiply();
        output.extend_from_slice(&[color.red(), color.green(), color.blue(), color.alpha()]);
    }
    output
}

/// Planar BT.709 limited-range 4:2:0 picture.
pub struct Yuv420 {
    pub width: usize,
    pub height: usize,
    pub y: Vec<u8>,
    pub u: Vec<u8>,
    pub v: Vec<u8>,
}

/// Converts a frame (composited over black where translucent) to BT.709 limited-range 4:2:0 with
/// integer arithmetic. Width and height must be even.
pub fn rgba_to_yuv420(pixmap: &Pixmap) -> Result<Yuv420> {
    let (width, height) = (pixmap.width() as usize, pixmap.height() as usize);
    ensure!(
        width.is_multiple_of(2) && height.is_multiple_of(2),
        "4:2:0 needs even dimensions, got {width}x{height}"
    );
    let data = pixmap.data();
    let mut luma = vec![0u8; width * height];
    let mut blue_difference = vec![0u8; width * height / 4];
    let mut red_difference = vec![0u8; width * height / 4];
    for row in 0..height {
        for column in 0..width {
            let offset = (row * width + column) * 4;
            let (r, g, b) = (
                i32::from(data[offset]),
                i32::from(data[offset + 1]),
                i32::from(data[offset + 2]),
            );
            luma[row * width + column] = (((47 * r + 157 * g + 16 * b + 128) >> 8) + 16) as u8;
        }
    }
    for row in 0..height / 2 {
        for column in 0..width / 2 {
            let (mut r, mut g, mut b) = (0i32, 0i32, 0i32);
            for (dy, dx) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
                let offset = ((row * 2 + dy) * width + column * 2 + dx) * 4;
                r += i32::from(data[offset]);
                g += i32::from(data[offset + 1]);
                b += i32::from(data[offset + 2]);
            }
            let (r, g, b) = ((r + 2) >> 2, (g + 2) >> 2, (b + 2) >> 2);
            let index = row * (width / 2) + column;
            blue_difference[index] = ((((-26 * r - 86 * g + 112 * b + 128) >> 8) + 128).clamp(0, 255)) as u8;
            red_difference[index] = ((((112 * r - 102 * g - 10 * b + 128) >> 8) + 128).clamp(0, 255)) as u8;
        }
    }
    Ok(Yuv420 {
        width,
        height,
        y: luma,
        u: blue_difference,
        v: red_difference,
    })
}

struct RenderState<A: Assets> {
    assets: A,
    blender: Box<dyn TransitionBlender>,
    images: HashMap<String, Pixmap>,
    fonts: HashMap<String, Arc<[u8]>>,
    captions: HashMap<usize, Vec<CaptionCue>>,
}

impl<A: Assets> RenderState<A> {
    fn font(&mut self, path: Option<&str>, weight: Weight) -> Result<Arc<[u8]>> {
        let key = match (path, weight) {
            (Some(path), _) => path.to_string(),
            (None, Weight::Regular) => "builtin:regular".to_string(),
            (None, Weight::Bold) => "builtin:bold".to_string(),
        };
        if let Some(font) = self.fonts.get(&key) {
            return Ok(Arc::clone(font));
        }
        let bytes: Arc<[u8]> = match path {
            Some(path) => self.assets.read(path)?.into(),
            None if weight == Weight::Bold => FONT_BOLD.into(),
            None => FONT_REGULAR.into(),
        };
        self.fonts.insert(key, Arc::clone(&bytes));
        Ok(bytes)
    }

    fn image(&mut self, src: &str) -> Result<&Pixmap> {
        if !self.images.contains_key(src) {
            let bytes = self.assets.read(src)?;
            let pixmap = decode_image(&bytes).with_context(|| format!("decoding `{src}`"))?;
            self.images.insert(src.to_string(), pixmap);
        }
        self.images
            .get(src)
            .ok_or_else(|| anyhow!("image cache miss for `{src}`"))
    }

    fn caption_cues(&mut self, clip: usize, timeline: &Timeline) -> Result<&Vec<CaptionCue>> {
        if !self.captions.contains_key(&clip) {
            let Layer::Captions(layer) = &timeline.clip(clip).layer else {
                bail!("clip {clip} is not a captions layer");
            };
            let mut cues = layer.cues.clone();
            if let Some(src) = &layer.src {
                let text = String::from_utf8(self.assets.read(src)?)
                    .with_context(|| format!("caption file `{src}` is not UTF-8"))?;
                let format = if src.to_ascii_lowercase().ends_with(".vtt") {
                    SubtitleFormat::WebVtt
                } else {
                    SubtitleFormat::Srt
                };
                for cue in parse_subtitles(format, &text)
                    .with_context(|| format!("parsing `{src}`"))?
                {
                    cues.push(CaptionCue {
                        start: cue.start_ms as f64 / 1000.0,
                        end: cue.end_ms as f64 / 1000.0,
                        text: cue.text,
                        words: Vec::new(),
                    });
                }
            }
            cues.sort_by(|a, b| a.start.total_cmp(&b.start));
            self.captions.insert(clip, cues);
        }
        self.captions
            .get(&clip)
            .ok_or_else(|| anyhow!("caption cache miss"))
    }
}

pub struct Renderer<A: Assets> {
    timeline: Timeline,
    state: RenderState<A>,
}

impl<A: Assets> Renderer<A> {
    #[must_use]
    pub fn new(timeline: Timeline, assets: A) -> Self {
        Self {
            timeline,
            state: RenderState {
                assets,
                blender: Box::new(CpuBlender),
                images: HashMap::new(),
                fonts: HashMap::new(),
                captions: HashMap::new(),
            },
        }
    }

    /// Replaces the transition blender (for example with the GPU one).
    #[must_use]
    pub fn with_blender(mut self, blender: Box<dyn TransitionBlender>) -> Self {
        self.state.blender = blender;
        self
    }

    #[must_use]
    pub fn timeline(&self) -> &Timeline {
        &self.timeline
    }

    /// Renders frame `frame` (premultiplied RGBA8).
    pub fn render_frame(&mut self, frame: u64) -> Result<Pixmap> {
        ensure!(
            frame < self.timeline.duration_frames(),
            "frame {frame} is outside the {}-frame timeline",
            self.timeline.duration_frames()
        );
        let scene = self.timeline.scene(frame)?;
        let composition = self.timeline.composition();
        let mut canvas = Pixmap::new(composition.width, composition.height)
            .ok_or_else(|| anyhow!("invalid frame size"))?;
        let background = composition.background.unwrap_or(Color::BLACK);
        canvas.fill(paint_color(background, 1.0));

        let Self { timeline, state } = self;
        for item in &scene.items {
            match item.transition {
                Some(transition) => {
                    let mut after = canvas.clone();
                    draw_item(state, timeline, &mut after, item)?;
                    canvas = state.blender.blend(&canvas, &after, transition)?;
                }
                None => draw_item(state, timeline, &mut canvas, item)?,
            }
        }
        Ok(canvas)
    }
}

fn draw_item<A: Assets>(
    state: &mut RenderState<A>,
    timeline: &Timeline,
    canvas: &mut Pixmap,
    item: &SceneItem,
) -> Result<()> {
    let clip = timeline.clip(item.clip);
    let props = &item.props;
    let (frame_width, frame_height) = (canvas.width() as f32, canvas.height() as f32);
    let box_width = props.width.unwrap_or(frame_width);
    let box_height = props.height.unwrap_or(frame_height);
    match &clip.layer {
        Layer::Audio(_) => {}
        Layer::Shape(shape) => {
            let transform = item_transform(clip.anchor, props, box_width, box_height);
            if let Some(path) = shape_path(shape, box_width, box_height) {
                let paint = fill_paint(&shape.fill, box_width, box_height, props.opacity);
                canvas.fill_path(&path, &paint, FillRule::Winding, transform, None);
                if let Some(stroke) = &shape.stroke {
                    let paint = solid_paint(stroke.color, props.opacity);
                    let stroke = SkiaStroke {
                        width: stroke.width,
                        ..SkiaStroke::default()
                    };
                    canvas.stroke_path(&path, &paint, &stroke, transform, None);
                }
            }
        }
        Layer::Text(text) => {
            let font = state.font(text.font.as_deref(), text.weight)?;
            let words = split_words(&text.text, 0);
            let style = TextStyle {
                size: text.size,
                letter_spacing: text.letter_spacing,
                line_height: text.line_height,
                align: text.align,
                max_width: props.width,
            };
            let layout = layout_text(&font, &words, 1, &style)?;
            let transform = item_transform(clip.anchor, props, layout.width, layout.height);
            draw_text_paths(canvas, &layout, &[text.color], props.opacity, transform);
        }
        Layer::Captions(captions) => {
            let local_seconds = item.local_seconds;
            let cues = state.caption_cues(item.clip, timeline)?;
            let Some((cue, active_word)) = active_caption(cues, local_seconds) else {
                return Ok(());
            };
            let words: Vec<(String, usize)> = if cue.words.is_empty() {
                split_words(&cue.text, 0)
            } else {
                cue.words
                    .iter()
                    .enumerate()
                    .map(|(index, CaptionWord { text, .. })| {
                        (text.clone(), usize::from(Some(index) == active_word))
                    })
                    .collect()
            };
            let font = state.font(None, captions.weight)?;
            let style = TextStyle {
                size: captions.size,
                letter_spacing: 0.0,
                line_height: 1.25,
                align: Align::Center,
                max_width: Some(props.width.unwrap_or(frame_width * 0.8)),
            };
            let layout = layout_text(&font, &words, 2, &style)?;
            let transform = item_transform(clip.anchor, props, layout.width, layout.height);
            let colors = [captions.color, captions.highlight.unwrap_or(captions.color)];
            draw_text_paths(canvas, &layout, &colors, props.opacity, transform);
        }
        Layer::Image(media) => {
            let fit = media.fit;
            let pixmap = state.image(&media.src)?;
            draw_fitted(canvas, pixmap, fit, props, clip.anchor, box_width, box_height);
        }
        Layer::Video(media) => {
            let pixmap = state.assets.video_frame(&media.src, item.source_seconds)?;
            draw_fitted(canvas, &pixmap, media.fit, props, clip.anchor, box_width, box_height);
        }
    }
    Ok(())
}

fn split_words(text: &str, slot: usize) -> Vec<(String, usize)> {
    let mut words = Vec::new();
    for (index, line) in text.split('\n').enumerate() {
        if index > 0 {
            words.push(("\n".to_string(), slot));
        }
        words.extend(
            line.split(' ')
                .filter(|word| !word.is_empty())
                .map(|word| (word.to_string(), slot)),
        );
    }
    words
}

fn draw_text_paths(
    canvas: &mut Pixmap,
    layout: &TextLayout,
    colors: &[Color],
    opacity: f32,
    transform: Transform,
) {
    for (path, color) in layout.paths.iter().zip(colors) {
        if let Some(path) = path {
            canvas.fill_path(
                path,
                &solid_paint(*color, opacity),
                FillRule::Winding,
                transform,
                None,
            );
        }
    }
}

fn draw_fitted(
    canvas: &mut Pixmap,
    source: &Pixmap,
    fit: Fit,
    props: &Props,
    anchor: [f32; 2],
    box_width: f32,
    box_height: f32,
) {
    let (source_width, source_height) = (source.width() as f32, source.height() as f32);
    let (scale_x, scale_y) = match fit {
        Fit::Fill => (box_width / source_width, box_height / source_height),
        Fit::Cover => {
            let scale = (box_width / source_width).max(box_height / source_height);
            (scale, scale)
        }
        Fit::Contain => {
            let scale = (box_width / source_width).min(box_height / source_height);
            (scale, scale)
        }
    };
    let (target_width, target_height) = (source_width * scale_x, source_height * scale_y);
    let (offset_x, offset_y) = (
        (box_width - target_width) / 2.0,
        (box_height - target_height) / 2.0,
    );
    let left = offset_x.max(0.0);
    let top = offset_y.max(0.0);
    let right = (offset_x + target_width).min(box_width);
    let bottom = (offset_y + target_height).min(box_height);
    let Some(rect) = Rect::from_ltrb(left, top, right, bottom) else {
        return;
    };
    let mut paint = Paint::default();
    paint.anti_alias = true;
    paint.shader = Pattern::new(
        source.as_ref(),
        SpreadMode::Pad,
        FilterQuality::Bilinear,
        props.opacity,
        Transform::from_translate(offset_x, offset_y).pre_scale(scale_x, scale_y),
    );
    canvas.fill_rect(rect, &paint, item_transform(anchor, props, box_width, box_height), None);
}

#[cfg(feature = "video-encode")]
pub use mp4::Mp4Encoder;

#[cfg(feature = "video-encode")]
mod mp4 {
    use std::io::Cursor;

    use anyhow::{Context as _, Result, anyhow, ensure};
    use craft_media::isobmff::{
        AvcConfig, Brand, CodecConfig, FourCc, Mp4Writer, SampleEntry, TrackConfig, WriteSample,
        WriterOptions,
    };
    use craft_video::h264::{
        Encoder, EncoderConfig, Packet, PacketFormat, Preset, RateControl, YuvFrame,
    };
    use tiny_skia::Pixmap;

    use super::rgba_to_yuv420;

    /// Streams frames into a faststart H.264 (High profile, BT.709) MP4 in memory.
    pub struct Mp4Encoder {
        encoder: Encoder,
        writer: Mp4Writer<Cursor<Vec<u8>>>,
        track: usize,
        width: usize,
        height: usize,
        frame_duration: u32,
        next_pts: i64,
        frames: u64,
    }

    impl Mp4Encoder {
        /// `crf` is the constant-quality level (lower is better; 18 is visually lossless).
        pub fn new(width: u32, height: u32, fps_num: u32, fps_den: u32, crf: f32) -> Result<Self> {
            let mut config = EncoderConfig::new(width, height, fps_num, fps_den);
            config.preset = Preset::Balanced;
            config.rate = RateControl::Crf(crf);
            config.bframes = 0;
            config.keyint = (fps_num / fps_den.max(1)).max(1) * 2;
            config.scenecut = false;
            config.slices = (height as usize).div_ceil(16).div_ceil(4).max(1);
            config.format = PacketFormat::LengthPrefixed;
            let encoder = Encoder::new(config).map_err(|error| anyhow!("h264 encoder: {error}"))?;
            let avc = AvcConfig::parse(&encoder.avcc()).map_err(|error| anyhow!("avcC: {error}"))?;
            let entry = SampleEntry::video(
                FourCc::new(b"avc1"),
                CodecConfig::Avc(avc),
                u16::try_from(width).context("width exceeds MP4 limits")?,
                u16::try_from(height).context("height exceeds MP4 limits")?,
            );
            let mut writer = Mp4Writer::new(Cursor::new(Vec::new()), WriterOptions::new(Brand::Mp4))
                .map_err(|error| anyhow!("mp4 writer: {error}"))?;
            let track = writer
                .add_track(TrackConfig::new(entry, fps_num))
                .map_err(|error| anyhow!("mp4 track: {error}"))?;
            Ok(Self {
                encoder,
                writer,
                track,
                width: width as usize,
                height: height as usize,
                frame_duration: fps_den,
                next_pts: 0,
                frames: 0,
            })
        }

        pub fn push_frame(&mut self, frame: &Pixmap) -> Result<()> {
            ensure!(
                frame.width() as usize == self.width && frame.height() as usize == self.height,
                "frame size changed mid-stream"
            );
            let yuv = rgba_to_yuv420(frame)?;
            let picture = YuvFrame {
                y: &yuv.y,
                u: &yuv.u,
                v: &yuv.v,
                y_stride: yuv.width,
                uv_stride: yuv.width / 2,
            };
            let packets = self
                .encoder
                .encode(&picture, self.next_pts)
                .map_err(|error| anyhow!("h264 encode: {error}"))?;
            self.next_pts += 1;
            self.frames += 1;
            self.write_packets(&packets)
        }

        fn write_packets(&mut self, packets: &[Packet]) -> Result<()> {
            for packet in packets {
                let offset = (packet.pts - packet.dts) as i32 * self.frame_duration as i32;
                self.writer
                    .write_sample(
                        self.track,
                        WriteSample {
                            data: &packet.data,
                            duration: self.frame_duration,
                            composition_offset: offset,
                            is_sync: packet.keyframe,
                        },
                    )
                    .map_err(|error| anyhow!("mp4 sample: {error}"))?;
            }
            Ok(())
        }

        /// Flushes the encoder and returns the finished MP4 file bytes.
        pub fn finish(mut self) -> Result<Vec<u8>> {
            let tail = self.encoder.flush();
            self.write_packets(&tail)?;
            let cursor = self
                .writer
                .finish_faststart()
                .map_err(|error| anyhow!("mp4 finish: {error}"))?;
            Ok(cursor.into_inner())
        }

        #[must_use]
        pub fn frames(&self) -> u64 {
            self.frames
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::motion::Composition;
    use std::collections::BTreeMap;

    fn state(kind: TransitionKind, progress: f32) -> TransitionState {
        TransitionState {
            kind,
            progress,
            feather: 0.1,
        }
    }

    fn solid(width: u32, height: u32, color: [u8; 4]) -> Pixmap {
        let mut pixmap = Pixmap::new(width, height).expect("pixmap");
        pixmap.fill(paint_color(Color(color), 1.0));
        pixmap
    }

    #[test]
    fn every_transition_is_clean_at_both_ends() {
        let below = solid(16, 8, [255, 0, 0, 255]);
        let above = solid(16, 8, [0, 0, 255, 255]);
        let mut blender = CpuBlender;
        for kind in [
            TransitionKind::CrossFade,
            TransitionKind::LinearWipe,
            TransitionKind::Dissolve,
            TransitionKind::ZoomBlur,
        ] {
            let start = blender.blend(&below, &above, state(kind, 0.0)).expect("start");
            let end = blender.blend(&below, &above, state(kind, 1.0)).expect("end");
            assert_eq!(start.data(), below.data(), "{kind:?} at 0");
            assert_eq!(end.data(), above.data(), "{kind:?} at 1");
        }
    }

    #[test]
    fn wipe_reveals_left_to_right_and_dissolve_is_partial_in_between() {
        let below = solid(32, 4, [255, 0, 0, 255]);
        let above = solid(32, 4, [0, 0, 255, 255]);
        let mut blender = CpuBlender;
        let wipe = blender
            .blend(&below, &above, state(TransitionKind::LinearWipe, 0.5))
            .expect("wipe");
        let data = wipe.data();
        assert_eq!(&data[0..4], &[0, 0, 255, 255], "left is revealed first");
        assert_eq!(&data[(31 * 4)..(31 * 4 + 4)], &[255, 0, 0, 255], "right still below");

        let dissolve = blender
            .blend(&below, &above, state(TransitionKind::Dissolve, 0.5))
            .expect("dissolve");
        let revealed = dissolve
            .data()
            .chunks_exact(4)
            .filter(|pixel| pixel[2] == 255)
            .count();
        assert!((40..=88).contains(&revealed), "about half of 128 pixels, got {revealed}");
    }

    #[test]
    fn crossfade_midpoint_is_the_average() {
        let below = solid(4, 4, [200, 0, 0, 255]);
        let above = solid(4, 4, [0, 100, 0, 255]);
        let out = CpuBlender
            .blend(&below, &above, state(TransitionKind::CrossFade, 0.5))
            .expect("blend");
        assert_eq!(&out.data()[0..4], &[100, 50, 0, 255]);
    }

    #[test]
    fn text_layout_measures_advances_and_wraps() {
        let style = |max_width| TextStyle {
            size: 40.0,
            letter_spacing: 0.0,
            line_height: 1.2,
            align: Align::Left,
            max_width,
        };
        let words = split_words("alpha beta gamma", 0);
        let one_line = layout_text(FONT_REGULAR, &words, 1, &style(None)).expect("layout");
        assert!(one_line.width > 200.0 && one_line.paths[0].is_some());
        assert!((one_line.height - 48.0).abs() < 1e-3);
        let wrapped = layout_text(FONT_REGULAR, &words, 1, &style(Some(150.0))).expect("wrapped");
        assert!(wrapped.height > one_line.height * 1.9, "{}", wrapped.height);
        assert_eq!(wrapped.width, 150.0);
    }

    #[test]
    fn yuv_conversion_matches_bt709_limited_range_anchors() {
        let white = rgba_to_yuv420(&solid(4, 4, [255, 255, 255, 255])).expect("white");
        assert_eq!((white.y[0], white.u[0], white.v[0]), (235, 128, 128));
        let black = rgba_to_yuv420(&solid(4, 4, [0, 0, 0, 255])).expect("black");
        assert_eq!((black.y[0], black.u[0], black.v[0]), (16, 128, 128));
        let red = rgba_to_yuv420(&solid(4, 4, [255, 0, 0, 255])).expect("red");
        assert!(red.v[0] > 235 && red.u[0] < 110);
        assert!(rgba_to_yuv420(&solid(3, 4, [0, 0, 0, 255])).is_err());
    }

    #[test]
    fn items_render_in_paint_order_and_transforms_move_pixels() {
        let composition = Composition::from_json(
            r##"{ "id": "order", "width": 64, "height": 64, "fps": 10, "background": "#000",
                "tracks": [
                  { "id": "back", "clips": [
                    { "id": "red", "duration": 1, "width": 40, "height": 40,
                      "layer": { "type": "shape", "fill": "#ff0000" } } ] },
                  { "id": "front", "clips": [
                    { "id": "blue", "duration": 1, "width": 20, "height": 20, "x": 32, "y": 32,
                      "layer": { "type": "shape", "fill": "#0000ff" },
                      "animate": [ { "property": "x", "keyframes": [
                        { "at": 0, "value": 32 }, { "at": 0.9, "value": 48 } ] } ] } ] }
                ] }"##,
        )
        .expect("parse");
        let timeline = Timeline::new(&composition, &BTreeMap::new()).expect("timeline");
        let mut renderer = Renderer::new(timeline, NoAssets);
        let first = renderer.render_frame(0).expect("frame 0");
        let pixel = |frame: &Pixmap, x: u32, y: u32| {
            let offset = ((y * frame.width() + x) * 4) as usize;
            [frame.data()[offset], frame.data()[offset + 1], frame.data()[offset + 2]]
        };
        assert_eq!(pixel(&first, 32, 32), [0, 0, 255], "blue is in front of red");
        assert_eq!(pixel(&first, 16, 32), [255, 0, 0], "red visible beside blue");
        assert_eq!(pixel(&first, 2, 2), [0, 0, 0], "background");
        let last = renderer.render_frame(9).expect("frame 9");
        assert_eq!(pixel(&last, 32, 32), [255, 0, 0], "blue moved right, red shows");
        assert_eq!(pixel(&last, 48, 32), [0, 0, 255]);
        assert!(renderer.render_frame(10).is_err());
    }
}
