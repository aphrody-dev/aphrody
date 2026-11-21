//! Video decode and encode adapters: one decoder front over `craft-video`'s track decoder and
//! Motion JPEG, one encoder enum over every `craft-video` encoder, and the pixel conversions
//! between them (8-bit 4:2:0 is the common form).

use crate::error::{AvError, AvResult};
use crate::model::{Codec, Track};
use craft_media::{CodecId, CodecParams};
use craft_video::frame::{ColorFamily, Plane, Samples, VideoFrame};

/// A decoded picture as 8-bit limited-range planar 4:2:0 (`ffmpeg -pix_fmt yuv420p`). Chroma
/// planes are `(width + 1) / 2` by `(height + 1) / 2`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Yuv420 {
    pub width: u32,
    pub height: u32,
    pub y: Vec<u8>,
    pub u: Vec<u8>,
    pub v: Vec<u8>,
    /// Presentation time in the units the decoder was given (track ticks).
    pub pts: i64,
}

impl Yuv420 {
    pub fn chroma_width(&self) -> usize {
        (self.width as usize).div_ceil(2)
    }
    pub fn chroma_height(&self) -> usize {
        (self.height as usize).div_ceil(2)
    }

    /// A flat frame of one luma and chroma value (test content, black frames).
    pub fn flat(width: u32, height: u32, y: u8, u: u8, v: u8) -> Yuv420 {
        let chroma = (width as usize).div_ceil(2) * (height as usize).div_ceil(2);
        Yuv420 {
            width,
            height,
            y: vec![y; width as usize * height as usize],
            u: vec![u; chroma],
            v: vec![v; chroma],
            pts: 0,
        }
    }

    pub fn validate(&self) -> AvResult<()> {
        let luma = self.width as usize * self.height as usize;
        let chroma = self.chroma_width() * self.chroma_height();
        if self.width == 0
            || self.height == 0
            || self.y.len() != luma
            || self.u.len() != chroma
            || self.v.len() != chroma
        {
            return Err(AvError::invalid(format!(
                "frame planes do not match {}x{} 4:2:0",
                self.width, self.height
            )));
        }
        Ok(())
    }

    /// Packed RGB24 (BT.709 limited range).
    pub fn to_rgb(&self) -> Vec<u8> {
        let (width, height) = (self.width as usize, self.height as usize);
        let chroma_width = self.chroma_width();
        let mut out = vec![0u8; width * height * 3];
        for row in 0..height {
            for column in 0..width {
                let luma = i32::from(self.y[row * width + column]) - 16;
                let blue_difference =
                    i32::from(self.u[(row / 2) * chroma_width + column / 2]) - 128;
                let red_difference = i32::from(self.v[(row / 2) * chroma_width + column / 2]) - 128;
                let red = (298 * luma + 459 * red_difference + 128) >> 8;
                let green = (298 * luma - 55 * blue_difference - 136 * red_difference + 128) >> 8;
                let blue = (298 * luma + 541 * blue_difference + 128) >> 8;
                let at = (row * width + column) * 3;
                out[at] = red.clamp(0, 255) as u8;
                out[at + 1] = green.clamp(0, 255) as u8;
                out[at + 2] = blue.clamp(0, 255) as u8;
            }
        }
        out
    }

    /// From packed RGB24 (BT.709 limited range), chroma averaged over 2x2 blocks.
    pub fn from_rgb(width: u32, height: u32, rgb: &[u8], pts: i64) -> AvResult<Yuv420> {
        let (w, h) = (width as usize, height as usize);
        if w == 0 || h == 0 || rgb.len() != w * h * 3 {
            return Err(AvError::invalid(format!(
                "{} bytes of RGB for a {width}x{height} picture",
                rgb.len()
            )));
        }
        let (cw, ch) = (w.div_ceil(2), h.div_ceil(2));
        let mut y = vec![0u8; w * h];
        let mut u = vec![0u8; cw * ch];
        let mut v = vec![0u8; cw * ch];
        for row in 0..h {
            for column in 0..w {
                let at = (row * w + column) * 3;
                let (r, g, b) = (
                    i32::from(rgb[at]),
                    i32::from(rgb[at + 1]),
                    i32::from(rgb[at + 2]),
                );
                y[row * w + column] =
                    (((47 * r + 157 * g + 16 * b + 128) >> 8) + 16).clamp(0, 255) as u8;
            }
        }
        for row in 0..ch {
            for column in 0..cw {
                let (mut r, mut g, mut b, mut n) = (0, 0, 0, 0);
                for dy in 0..2 {
                    for dx in 0..2 {
                        let (sy, sx) = ((row * 2 + dy).min(h - 1), (column * 2 + dx).min(w - 1));
                        let at = (sy * w + sx) * 3;
                        r += i32::from(rgb[at]);
                        g += i32::from(rgb[at + 1]);
                        b += i32::from(rgb[at + 2]);
                        n += 1;
                    }
                }
                let (r, g, b) = (r / n, g / n, b / n);
                u[row * cw + column] =
                    (((-26 * r - 87 * g + 112 * b + 128) >> 8) + 128).clamp(0, 255) as u8;
                v[row * cw + column] =
                    (((112 * r - 102 * g - 10 * b + 128) >> 8) + 128).clamp(0, 255) as u8;
            }
        }
        Ok(Yuv420 {
            width,
            height,
            y,
            u,
            v,
            pts,
        })
    }

    /// Crop to even dimensions (the 4:2:0 encoders need them).
    pub fn cropped_even(&self) -> Yuv420 {
        let (w, h) = (self.width & !1, self.height & !1);
        if (w, h) == (self.width, self.height) || w == 0 || h == 0 {
            return self.clone();
        }
        let crop = |plane: &[u8], stride: usize, nw: usize, nh: usize| -> Vec<u8> {
            (0..nh)
                .flat_map(|r| plane[r * stride..r * stride + nw].iter().copied())
                .collect()
        };
        let (cw, ch) = ((w / 2) as usize, (h / 2) as usize);
        Yuv420 {
            width: w,
            height: h,
            y: crop(&self.y, self.width as usize, w as usize, h as usize),
            u: crop(&self.u, self.chroma_width(), cw, ch),
            v: crop(&self.v, self.chroma_width(), cw, ch),
            pts: self.pts,
        }
    }

    /// Bilinear resize of every plane (`-vf scale=W:H`).
    pub fn scaled(&self, width: u32, height: u32) -> AvResult<Yuv420> {
        if width == 0 || height == 0 || width > 16384 || height > 16384 {
            return Err(AvError::invalid(format!(
                "scale target {width}x{height} is outside 1..=16384"
            )));
        }
        if (width, height) == (self.width, self.height) {
            return Ok(self.clone());
        }
        let (cw, ch) = ((width as usize).div_ceil(2), (height as usize).div_ceil(2));
        Ok(Yuv420 {
            width,
            height,
            y: resize_plane(
                &self.y,
                self.width as usize,
                self.height as usize,
                width as usize,
                height as usize,
            ),
            u: resize_plane(&self.u, self.chroma_width(), self.chroma_height(), cw, ch),
            v: resize_plane(&self.v, self.chroma_width(), self.chroma_height(), cw, ch),
            pts: self.pts,
        })
    }
}

fn resize_plane(source: &[u8], sw: usize, sh: usize, dw: usize, dh: usize) -> Vec<u8> {
    let mut out = vec![0u8; dw * dh];
    let x_ratio = sw as f32 / dw as f32;
    let y_ratio = sh as f32 / dh as f32;
    for row in 0..dh {
        let sy = ((row as f32 + 0.5) * y_ratio - 0.5).max(0.0);
        let y0 = (sy.floor() as usize).min(sh - 1);
        let y1 = (y0 + 1).min(sh - 1);
        let fy = sy - y0 as f32;
        for column in 0..dw {
            let sx = ((column as f32 + 0.5) * x_ratio - 0.5).max(0.0);
            let x0 = (sx.floor() as usize).min(sw - 1);
            let x1 = (x0 + 1).min(sw - 1);
            let fx = sx - x0 as f32;
            let top =
                f32::from(source[y0 * sw + x0]) * (1.0 - fx) + f32::from(source[y0 * sw + x1]) * fx;
            let bottom =
                f32::from(source[y1 * sw + x0]) * (1.0 - fx) + f32::from(source[y1 * sw + x1]) * fx;
            out[row * dw + column] = (top * (1.0 - fy) + bottom * fy + 0.5) as u8;
        }
    }
    out
}

fn sample_at(plane: &Plane, x: usize, y: usize) -> u32 {
    let stride = plane.stride();
    match plane.samples() {
        Samples::U8(data) => data.get(y * stride + x).copied().map_or(0, u32::from),
        Samples::U16(data) => data.get(y * stride + x).copied().map_or(0, u32::from),
    }
}

fn to_eight_bit(value: u32, depth: u8) -> u32 {
    if depth <= 8 {
        value << (8 - depth)
    } else {
        let shift = depth - 8;
        ((value + (1 << (shift - 1))) >> shift).min(255)
    }
}

/// Convert any decoded frame (YCbCr of any subsampling and depth, RGB, gray) to [`Yuv420`].
pub fn frame_to_yuv420(frame: &VideoFrame, pts: i64) -> AvResult<Yuv420> {
    let (width, height) = (frame.width() as usize, frame.height() as usize);
    let planes = frame.planes();
    match frame.family() {
        ColorFamily::Rgb => {
            let [red, green, blue] = planes else {
                return Err(AvError::malformed("RGB frame without three planes"));
            };
            let mut rgb = vec![0u8; width * height * 3];
            for row in 0..height {
                for column in 0..width {
                    let at = (row * width + column) * 3;
                    rgb[at] = to_eight_bit(sample_at(red, column, row), red.bit_depth()) as u8;
                    rgb[at + 1] =
                        to_eight_bit(sample_at(green, column, row), green.bit_depth()) as u8;
                    rgb[at + 2] =
                        to_eight_bit(sample_at(blue, column, row), blue.bit_depth()) as u8;
                }
            }
            Yuv420::from_rgb(frame.width(), frame.height(), &rgb, pts)
        }
        ColorFamily::Gray | ColorFamily::YCbCr => {
            let full_range = frame.color.full_range;
            let luma_plane = planes
                .first()
                .ok_or_else(|| AvError::malformed("frame without planes"))?;
            let luma_map = |value: u32| -> u8 {
                let eight = to_eight_bit(value, luma_plane.bit_depth());
                if full_range {
                    (16 + (eight * 219 + 127) / 255).min(255) as u8
                } else {
                    eight as u8
                }
            };
            let mut y = vec![0u8; width * height];
            for row in 0..height {
                for column in 0..width {
                    y[row * width + column] = luma_map(sample_at(luma_plane, column, row));
                }
            }
            let (cw, ch) = (width.div_ceil(2), height.div_ceil(2));
            let mut u = vec![128u8; cw * ch];
            let mut v = vec![128u8; cw * ch];
            if let (ColorFamily::YCbCr, Some(cb), Some(cr)) =
                (frame.family(), planes.get(1), planes.get(2))
            {
                let sub = frame.subsampling();
                for row in 0..ch {
                    for column in 0..cw {
                        let (mut sum_u, mut sum_v, mut n) = (0u32, 0u32, 0u32);
                        for dy in 0..2usize {
                            for dx in 0..2usize {
                                let lx = (column * 2 + dx).min(width - 1);
                                let ly = (row * 2 + dy).min(height - 1);
                                let (cx, cy) = (lx >> sub.x, ly >> sub.y);
                                sum_u += to_eight_bit(sample_at(cb, cx, cy), cb.bit_depth());
                                sum_v += to_eight_bit(sample_at(cr, cx, cy), cr.bit_depth());
                                n += 1;
                            }
                        }
                        let chroma_map = |value: u32| -> u8 {
                            if full_range {
                                (128 + ((value as i32 - 128) * 224 + 127) / 255).clamp(16, 240)
                                    as u8
                            } else {
                                value as u8
                            }
                        };
                        u[row * cw + column] = chroma_map((sum_u + n / 2) / n);
                        v[row * cw + column] = chroma_map((sum_v + n / 2) / n);
                    }
                }
            }
            Ok(Yuv420 {
                width: frame.width(),
                height: frame.height(),
                y,
                u,
                v,
                pts,
            })
        }
    }
}

/// Decodes the packets of one video track.
pub struct VideoDecoder {
    inner: DecoderInner,
}

enum DecoderInner {
    Track(Box<craft_video::TrackDecoder>),
    Mjpeg(craft_video::mjpeg::Decoder),
}

impl VideoDecoder {
    pub fn new(track: &Track) -> AvResult<VideoDecoder> {
        let params = &track.params;
        let inner = match params.codec {
            CodecId::Mjpeg => DecoderInner::Mjpeg(craft_video::mjpeg::Decoder::new()),
            CodecId::H264
            | CodecId::Hevc
            | CodecId::Vp9
            | CodecId::Av1
            | CodecId::Mpeg2Video
            | CodecId::ProRes
            | CodecId::Dnxhd
            | CodecId::Apv
            | CodecId::Ffv1
            | CodecId::Vp8 => DecoderInner::Track(Box::new(
                craft_video::TrackDecoder::new(params).map_err(video_error)?,
            )),
            other => {
                return Err(AvError::unsupported(format!(
                    "no decoder for video codec {} ({})",
                    other.name(),
                    codec_support_hint(other)
                )));
            }
        };
        Ok(VideoDecoder { inner })
    }

    pub fn decode(&mut self, packet: &[u8], pts: i64) -> AvResult<Vec<Yuv420>> {
        match &mut self.inner {
            DecoderInner::Track(decoder) => {
                let frames = decoder.decode(packet, pts).map_err(video_error)?;
                frames
                    .iter()
                    .map(|frame| frame_to_yuv420(frame, frame.pts.unwrap_or(pts)))
                    .collect()
            }
            DecoderInner::Mjpeg(decoder) => {
                let frame = decoder
                    .decode(packet)
                    .map_err(|e| AvError::malformed(format!("mjpeg: {e}")))?;
                Ok(vec![mjpeg_to_yuv(&frame, pts)?])
            }
        }
    }

    pub fn flush(&mut self) -> AvResult<Vec<Yuv420>> {
        match &mut self.inner {
            DecoderInner::Track(decoder) => {
                let frames = decoder.flush().map_err(video_error)?;
                frames
                    .iter()
                    .map(|frame| frame_to_yuv420(frame, frame.pts.unwrap_or(0)))
                    .collect()
            }
            DecoderInner::Mjpeg(_) => Ok(Vec::new()),
        }
    }
}

fn mjpeg_to_yuv(frame: &craft_video::mjpeg::Frame, pts: i64) -> AvResult<Yuv420> {
    let craft_video::mjpeg::Samples::U8(pixels) = &frame.samples else {
        return Err(AvError::unsupported("12-bit Motion JPEG pictures"));
    };
    let (w, h) = (frame.width as usize, frame.height as usize);
    match frame.channels {
        3 => Yuv420::from_rgb(frame.width, frame.height, pixels, pts),
        1 => {
            let mut out = Yuv420::flat(frame.width, frame.height, 16, 128, 128);
            for (target, &gray) in out.y.iter_mut().zip(pixels.iter()).take(w * h) {
                *target = (16 + (u32::from(gray) * 219 + 127) / 255) as u8;
            }
            out.pts = pts;
            Ok(out)
        }
        n => Err(AvError::unsupported(format!(
            "Motion JPEG pictures with {n} components"
        ))),
    }
}

fn video_error(error: craft_video::VideoError) -> AvError {
    match error {
        craft_video::VideoError::Unsupported(message) => AvError::Unsupported(message),
        other => AvError::Malformed(other.to_string()),
    }
}

fn codec_support_hint(id: CodecId) -> &'static str {
    match id {
        CodecId::Theora => "Theora has no decoder in craft-video",
        CodecId::VmdVideo => "Sierra VMD video is not wired",
        CodecId::Unknown => "the container names a codec this build does not know",
        _ => "not a video codec",
    }
}

/// NAL framing the muxer needs for H.264 and HEVC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Framing {
    /// Length-prefixed NAL units with the parameter sets in the track configuration (MP4, Matroska).
    LengthPrefixed,
    /// Start-code NAL units with in-band parameter sets (AVI, MPEG-TS).
    AnnexB,
}

#[derive(Debug, Clone)]
pub struct VideoEncodeConfig {
    pub codec: Codec,
    pub width: u32,
    pub height: u32,
    pub fps: (u32, u32),
    pub bitrate_kbps: Option<u32>,
    pub framing: Framing,
}

/// One coded picture; times are in frames (multiply by the frame duration for ticks).
#[derive(Debug, Clone)]
pub struct EncodedPicture {
    pub data: Vec<u8>,
    pub pts: i64,
    pub dts: i64,
    pub key: bool,
}

pub struct VideoEncoder {
    inner: EncoderInner,
    params: CodecParams,
    width: u32,
    height: u32,
    frames: i64,
    annexb: bool,
}

enum EncoderInner {
    H264(Box<craft_video::h264::Encoder>),
    Hevc(Box<craft_video::hevc::Encoder>),
    Vp9(Box<craft_video::vp9::Vp9Encoder>),
    Av1(Box<craft_video::av1::Encoder>),
    Vp8,
    Mpeg2(Box<craft_video::mpeg2::Encoder>),
    Mpeg4(Box<craft_video::mpeg4::Encoder>),
    Ffv1(Box<craft_video::ffv1::Encoder>),
    Mjpeg(craft_video::mjpeg::Encoder),
    ProRes(Box<craft_video::prores::Encoder>),
    Dnx(Box<craft_video::dnx::Encoder>),
    Apv(Box<craft_video::apv::Encoder>),
}

fn encode_error(codec: &str, error: impl std::fmt::Display) -> AvError {
    AvError::Unsupported(format!("{codec} encoder refused the stream: {error}"))
}

fn widen(plane: &[u8]) -> Vec<u16> {
    plane.iter().map(|&v| u16::from(v)).collect()
}

impl VideoEncoder {
    pub fn new(config: &VideoEncodeConfig) -> AvResult<VideoEncoder> {
        let (width, height) = (config.width, config.height);
        let (fps_num, fps_den) = config.fps;
        if width == 0 || height == 0 || fps_num == 0 || fps_den == 0 {
            return Err(AvError::invalid(
                "video size and frame rate must be non-zero",
            ));
        }
        let name = config.codec.name();
        let needs_even = !matches!(
            config.codec,
            Codec::Mjpeg
                | Codec::Vp8
                | Codec::Vp9
                | Codec::Ffv1
                | Codec::Dnxhd
                | Codec::Apv
                | Codec::ProRes
        );
        if needs_even && (width % 2 != 0 || height % 2 != 0) {
            return Err(AvError::invalid(format!(
                "{name} needs even picture dimensions, got {width}x{height}"
            )));
        }
        let annexb = config.framing == Framing::AnnexB;
        let (inner, mut params) = match config.codec {
            Codec::H264 => {
                let mut cfg =
                    craft_video::h264::EncoderConfig::new(width, height, fps_num, fps_den);
                cfg.bframes = 0;
                cfg.format = if annexb {
                    craft_video::h264::PacketFormat::AnnexB
                } else {
                    craft_video::h264::PacketFormat::LengthPrefixed
                };
                if let Some(kbps) = config.bitrate_kbps {
                    cfg.rate = craft_video::h264::RateControl::Cbr { kbps };
                }
                let encoder =
                    craft_video::h264::Encoder::new(cfg).map_err(|e| encode_error(name, e))?;
                let params = encoder.codec_params();
                (EncoderInner::H264(Box::new(encoder)), params)
            }
            Codec::Hevc => {
                let mut cfg =
                    craft_video::hevc::EncoderConfig::new(width, height, fps_num, fps_den);
                if let Some(kbps) = config.bitrate_kbps {
                    cfg.rate = craft_video::hevc::RateControl::Bitrate { kbps };
                }
                let encoder =
                    craft_video::hevc::Encoder::new(cfg).map_err(|e| encode_error(name, e))?;
                let params = encoder.codec_params();
                (EncoderInner::Hevc(Box::new(encoder)), params)
            }
            Codec::Vp9 => {
                let cfg = craft_video::vp9::EncoderConfig {
                    width,
                    height,
                    frame_rate: f64::from(fps_num) / f64::from(fps_den),
                    target_kbps: config.bitrate_kbps,
                    ..Default::default()
                };
                let encoder = craft_video::vp9::Vp9Encoder::new(cfg);
                let params = encoder.codec_params();
                (EncoderInner::Vp9(Box::new(encoder)), params)
            }
            Codec::Av1 => {
                let mut cfg = craft_video::av1::EncoderConfig::new(width, height, fps_num, fps_den);
                if let Some(kbps) = config.bitrate_kbps {
                    cfg.rate = craft_video::av1::RateControl::Bitrate { kbps };
                }
                let encoder =
                    craft_video::av1::Encoder::new(cfg).map_err(|e| encode_error(name, e))?;
                let params = encoder.codec_params();
                (EncoderInner::Av1(Box::new(encoder)), params)
            }
            Codec::Vp8 => (
                EncoderInner::Vp8,
                CodecParams::new(CodecId::Vp8).with_size(width, height),
            ),
            Codec::Mpeg2 => {
                let mut cfg =
                    craft_video::mpeg2::EncoderConfig::new(width, height, fps_num, fps_den);
                cfg.b_pictures = 0;
                if let Some(kbps) = config.bitrate_kbps {
                    cfg.rate = craft_video::mpeg2::RateControl::Abr { kbps };
                }
                let encoder =
                    craft_video::mpeg2::Encoder::new(cfg).map_err(|e| encode_error(name, e))?;
                (
                    EncoderInner::Mpeg2(Box::new(encoder)),
                    CodecParams::new(CodecId::Mpeg2Video).with_size(width, height),
                )
            }
            Codec::Mpeg4 => {
                let mut cfg =
                    craft_video::mpeg4::EncoderConfig::new(width, height, fps_num, fps_den);
                cfg.headers_in_band = annexb;
                if let Some(kbps) = config.bitrate_kbps {
                    cfg.rate = craft_video::mpeg4::RateControl::Bitrate { kbps };
                }
                let encoder =
                    craft_video::mpeg4::Encoder::new(cfg).map_err(|e| encode_error(name, e))?;
                let params = encoder.codec_params();
                (EncoderInner::Mpeg4(Box::new(encoder)), params)
            }
            Codec::Ffv1 => {
                let template = ffv1_frame(&Yuv420::flat(width, height, 16, 128, 128));
                let encoder = craft_video::ffv1::Encoder::new(
                    &template,
                    craft_video::ffv1::EncoderConfig::default(),
                )
                .map_err(|e| encode_error(name, e))?;
                let params = CodecParams::new(CodecId::Ffv1)
                    .with_size(width, height)
                    .with_extradata(encoder.config_record());
                (EncoderInner::Ffv1(Box::new(encoder)), params)
            }
            Codec::Mjpeg => {
                let encoder =
                    craft_video::mjpeg::Encoder::new(craft_video::mjpeg::EncoderConfig::default())
                        .map_err(|e| encode_error(name, e))?;
                (
                    EncoderInner::Mjpeg(encoder),
                    CodecParams::new(CodecId::Mjpeg).with_size(width, height),
                )
            }
            Codec::ProRes => {
                let encoder = craft_video::prores::Encoder::new(
                    craft_video::prores::Profile::Standard,
                    width,
                    height,
                );
                let params = encoder.codec_params();
                (EncoderInner::ProRes(Box::new(encoder)), params)
            }
            Codec::Dnxhd => {
                let encoder =
                    craft_video::dnx::Encoder::new(craft_video::dnx::Profile::Sq, width, height)
                        .map_err(|e| encode_error(name, e))?;
                let params = encoder.codec_params();
                (EncoderInner::Dnx(Box::new(encoder)), params)
            }
            Codec::Apv => {
                let encoder = craft_video::apv::Encoder::new(
                    craft_video::apv::Profile::P422_10,
                    width,
                    height,
                )
                .map_err(|e| encode_error(name, e))?;
                let params = CodecParams::new(CodecId::Apv)
                    .with_size(width, height)
                    .with_extradata(encoder.decoder_config_record());
                (EncoderInner::Apv(Box::new(encoder)), params)
            }
            other => {
                return Err(AvError::unsupported(format!(
                    "{} is not a video codec",
                    other.name()
                )));
            }
        };
        if annexb && matches!(config.codec, Codec::H264 | Codec::Hevc) {
            params.extradata.clear();
        }
        Ok(VideoEncoder {
            inner,
            params,
            width,
            height,
            frames: 0,
            annexb,
        })
    }

    /// Track description (codec, tag, size, extradata) for the muxer.
    pub fn params(&self) -> &CodecParams {
        &self.params
    }

    pub fn encode(&mut self, picture: &Yuv420) -> AvResult<Vec<EncodedPicture>> {
        picture.validate()?;
        if (picture.width, picture.height) != (self.width, self.height) {
            return Err(AvError::invalid(format!(
                "picture is {}x{}, encoder is {}x{}",
                picture.width, picture.height, self.width, self.height
            )));
        }
        let index = self.frames;
        self.frames += 1;
        let w = picture.width as usize;
        let cw = picture.chroma_width();
        let single = |data: Vec<u8>, key: bool| {
            vec![EncodedPicture {
                data,
                pts: index,
                dts: index,
                key,
            }]
        };
        match &mut self.inner {
            EncoderInner::H264(encoder) => {
                let frame = craft_video::h264::YuvFrame {
                    y: &picture.y,
                    u: &picture.u,
                    v: &picture.v,
                    y_stride: w,
                    uv_stride: cw,
                };
                let packets = encoder
                    .encode(&frame, index)
                    .map_err(|e| encode_error("h264", e))?;
                Ok(packets
                    .into_iter()
                    .map(|p| EncodedPicture {
                        data: p.data,
                        pts: p.pts,
                        dts: p.dts,
                        key: p.keyframe,
                    })
                    .collect())
            }
            EncoderInner::Hevc(encoder) => {
                let (y, u, v) = (widen(&picture.y), widen(&picture.u), widen(&picture.v));
                let frame = craft_video::hevc::Frame {
                    y: &y,
                    u: &u,
                    v: &v,
                    y_stride: w,
                    uv_stride: cw,
                };
                let packet = encoder
                    .encode(&frame)
                    .map_err(|e| encode_error("hevc", e))?;
                if self.annexb {
                    let mut data = Vec::new();
                    if packet.keyframe {
                        for set in encoder.parameter_sets() {
                            data.extend_from_slice(&[0, 0, 0, 1]);
                            data.extend_from_slice(&set);
                        }
                    }
                    data.extend(crate::mux::length_prefixed_to_annexb(&packet.data, 4)?);
                    return Ok(single(data, packet.keyframe));
                }
                Ok(single(packet.data, packet.keyframe))
            }
            EncoderInner::Vp9(encoder) => {
                let coded = encoder.encode_frame(&picture.y, &picture.u, &picture.v, false);
                if coded.is_empty() {
                    return Err(encode_error("vp9", "no frame was coded"));
                }
                Ok(single(coded.data, coded.key))
            }
            EncoderInner::Av1(encoder) => {
                let (y, u, v) = (widen(&picture.y), widen(&picture.u), widen(&picture.v));
                let frame = craft_video::av1::Frame {
                    y: &y,
                    u: &u,
                    v: &v,
                    y_stride: w,
                    uv_stride: cw,
                };
                let packet = encoder.encode(&frame).map_err(|e| encode_error("av1", e))?;
                Ok(single(packet.data, packet.keyframe))
            }
            EncoderInner::Vp8 => {
                let rgba: Vec<u8> = picture
                    .to_rgb()
                    .chunks_exact(3)
                    .flat_map(|p| [p[0], p[1], p[2], 255])
                    .collect();
                let coded = craft_video::vp8::encode_rgba(
                    &rgba,
                    picture.width,
                    picture.height,
                    &craft_video::vp8::Vp8Params::default(),
                )
                .map_err(|e| encode_error("vp8", e))?;
                Ok(single(coded.data, true))
            }
            EncoderInner::Mpeg2(encoder) => {
                let frame = craft_video::mpeg2::YuvFrame {
                    y: &picture.y,
                    cb: &picture.u,
                    cr: &picture.v,
                    y_stride: w,
                    c_stride: cw,
                };
                let packets = encoder
                    .encode(&frame, index)
                    .map_err(|e| encode_error("mpeg2video", e))?;
                Ok(packets
                    .into_iter()
                    .map(|p| EncodedPicture {
                        data: p.data,
                        pts: p.pts,
                        dts: p.dts,
                        key: p.keyframe,
                    })
                    .collect())
            }
            EncoderInner::Mpeg4(encoder) => {
                let frame = craft_video::mpeg4::YuvFrame {
                    y: &picture.y,
                    u: &picture.u,
                    v: &picture.v,
                    y_stride: w,
                    uv_stride: cw,
                };
                let packet = encoder
                    .encode(&frame)
                    .map_err(|e| encode_error("mpeg4", e))?;
                Ok(single(packet.data, packet.keyframe))
            }
            EncoderInner::Ffv1(encoder) => {
                let data = encoder
                    .encode(&ffv1_frame(picture))
                    .map_err(|e| encode_error("ffv1", e))?;
                Ok(single(data, true))
            }
            EncoderInner::Mjpeg(encoder) => {
                let data = encoder
                    .encode(picture.width, picture.height, 3, &picture.to_rgb())
                    .map_err(|e| encode_error("mjpeg", e))?;
                Ok(single(data, true))
            }
            EncoderInner::ProRes(encoder) => {
                let mut frame = craft_video::prores::Frame::new(
                    picture.width,
                    picture.height,
                    craft_video::prores::ChromaFormat::Yuv422,
                    10,
                    false,
                )
                .map_err(|e| encode_error("prores", e))?;
                fill_422(&mut frame.y, &mut frame.cb, &mut frame.cr, picture, 2);
                let data = encoder
                    .encode(&frame)
                    .map_err(|e| encode_error("prores", e))?;
                Ok(single(data, true))
            }
            EncoderInner::Dnx(encoder) => {
                let mut frame = craft_video::dnx::Frame::new(
                    picture.width,
                    picture.height,
                    craft_video::dnx::ChromaFormat::Yuv422,
                    8,
                    false,
                );
                fill_422(&mut frame.y, &mut frame.cb, &mut frame.cr, picture, 0);
                let data = encoder
                    .encode(&frame)
                    .map_err(|e| encode_error("dnxhd", e))?;
                Ok(single(data, true))
            }
            EncoderInner::Apv(encoder) => {
                let mut frame = craft_video::apv::Frame::new(
                    picture.width,
                    picture.height,
                    craft_video::apv::ChromaFormat::Yuv422,
                    10,
                    false,
                );
                fill_422(&mut frame.y, &mut frame.cb, &mut frame.cr, picture, 2);
                let data = encoder.encode(&frame).map_err(|e| encode_error("apv", e))?;
                Ok(single(data, true))
            }
        }
    }

    pub fn flush(&mut self) -> AvResult<Vec<EncodedPicture>> {
        match &mut self.inner {
            EncoderInner::H264(encoder) => Ok(encoder
                .flush()
                .into_iter()
                .map(|p| EncodedPicture {
                    data: p.data,
                    pts: p.pts,
                    dts: p.dts,
                    key: p.keyframe,
                })
                .collect()),
            EncoderInner::Mpeg2(encoder) => Ok(encoder
                .flush()
                .into_iter()
                .map(|p| EncodedPicture {
                    data: p.data,
                    pts: p.pts,
                    dts: p.dts,
                    key: p.keyframe,
                })
                .collect()),
            _ => Ok(Vec::new()),
        }
    }
}

fn ffv1_frame(picture: &Yuv420) -> craft_video::ffv1::Frame {
    craft_video::ffv1::Frame {
        width: picture.width,
        height: picture.height,
        colorspace: craft_video::ffv1::Colorspace::YCbCr,
        bit_depth: 8,
        chroma_shift: (1, 1),
        planes: vec![widen(&picture.y), widen(&picture.u), widen(&picture.v)],
    }
}

/// 4:2:0 to 4:2:2 (each chroma row repeated), samples shifted up by `shift` bits.
fn fill_422(y: &mut [u16], cb: &mut [u16], cr: &mut [u16], picture: &Yuv420, shift: u32) {
    let h = picture.height as usize;
    let cw = picture.chroma_width();
    for (target, &source) in y.iter_mut().zip(&picture.y) {
        *target = u16::from(source) << shift;
    }
    let ch = picture.chroma_height();
    for row in 0..h {
        let source_row = (row / 2).min(ch - 1);
        for column in 0..cw {
            let at = row * cw + column;
            if let (Some(t_cb), Some(t_cr)) = (cb.get_mut(at), cr.get_mut(at)) {
                *t_cb = u16::from(picture.u[source_row * cw + column]) << shift;
                *t_cr = u16::from(picture.v[source_row * cw + column]) << shift;
            }
        }
    }
}
