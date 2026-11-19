//! Headless video studio for agents.
//!
//! A [`Scene`] is a GPUI view that knows how to draw frame `n`. [`Studio::render`] opens it in a
//! headless GPUI window backed by [`gpui_wgpu::WgpuHeadlessRenderer`], captures every frame as
//! RGBA, converts it to BT.709 4:2:0, encodes it with the pure-Rust H.264 encoder of
//! `craft-video` and muxes it into MP4 with `craft-media`. [`verify`] decodes the file again with
//! `craft-video`'s decoder and requires every picture to be bit-identical to the encoder's
//! reconstruction, so a run proves its own output without ffmpeg.

use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hasher};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, ensure};
use craft_media::isobmff::{
    AvcConfig, Brand, Mp4Writer, SampleEntry, TrackConfig, WriteSample, WriterOptions,
};
use craft_video::h264::{
    Encoder, EncoderConfig, Packet, PacketFormat, Preset, Profile, RateControl, YuvFrame,
};
use gpui::{
    AppContext as _, Bounds, Entity, HeadlessAppContext, Render, WindowBounds, WindowOptions, px,
    size,
};
use rayon::prelude::*;
use serde::Serialize;

/// A GPUI view that can be positioned at any frame of the timeline.
pub trait Scene: Render + 'static {
    /// Moves the view to frame `frame` of a timeline running at `fps`.
    fn seek(&mut self, frame: u32, fps: u32);
}

/// Output settings. Width and height must be even (4:2:0).
#[derive(Clone, Debug)]
pub struct Settings {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub frames: u32,
    /// Constant rate factor of the H.264 encoder (lower is better quality).
    pub crf: f32,
    pub preset: Preset,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            width: 1920,
            height: 1080,
            fps: 120,
            frames: 720,
            crf: 18.0,
            preset: Preset::Speed,
        }
    }
}

/// Timings and sizes of one render, in seconds and bytes.
#[derive(Clone, Debug, Serialize)]
pub struct Report {
    pub frames: u32,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub render_secs: f64,
    pub convert_secs: f64,
    pub encode_secs: f64,
    pub total_secs: f64,
    pub throughput_fps: f64,
    pub bytes: u64,
    /// Hash of each reconstructed picture by presentation index, for [`verify`].
    #[serde(skip)]
    pub recon: BTreeMap<i64, u64>,
}

fn hash_planes(planes: [(&[u8], usize, usize, usize); 3]) -> u64 {
    let mut h = DefaultHasher::new();
    for (data, width, rows, stride) in planes {
        for r in 0..rows {
            h.write(&data[r * stride..r * stride + width]);
        }
    }
    h.finish()
}

/// BT.709 limited-range RGBA to I420; rows are converted in parallel.
pub fn rgba_to_i420(
    rgba: &[u8],
    width: usize,
    height: usize,
    y: &mut [u8],
    u: &mut [u8],
    v: &mut [u8],
) {
    assert!(
        rgba.len() >= width * height * 4 && y.len() >= width * height,
        "plane sizes do not match {width}x{height}"
    );
    y.par_chunks_mut(width).enumerate().for_each(|(j, row)| {
        let src = &rgba[j * width * 4..(j + 1) * width * 4];
        for (i, out) in row.iter_mut().enumerate() {
            let (r, g, b) = (
                src[i * 4] as i32,
                src[i * 4 + 1] as i32,
                src[i * 4 + 2] as i32,
            );
            *out = ((47 * r + 157 * g + 16 * b + 128) / 256 + 16) as u8;
        }
    });
    let cw = width / 2;
    u.par_chunks_mut(cw)
        .zip(v.par_chunks_mut(cw))
        .enumerate()
        .for_each(|(j, (ur, vr))| {
            for i in 0..cw {
                // average of the 2x2 block
                let (mut r, mut g, mut b) = (0i32, 0i32, 0i32);
                for (dy, dx) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
                    let o = ((j * 2 + dy) * width + i * 2 + dx) * 4;
                    r += rgba[o] as i32;
                    g += rgba[o + 1] as i32;
                    b += rgba[o + 2] as i32;
                }
                let (r, g, b) = (r / 4, g / 4, b / 4);
                ur[i] = ((-26 * r - 87 * g + 112 * b + 128) / 256 + 128) as u8;
                vr[i] = ((112 * r - 102 * g - 10 * b + 128) / 256 + 128) as u8;
            }
        });
}

/// Headless GPUI context with a wgpu renderer; one per process is enough.
pub struct Studio {
    cx: HeadlessAppContext,
}

impl Studio {
    pub fn new() -> Result<Self> {
        let text_system = Arc::new(gpui_wgpu::CosmicTextSystem::new("sans-serif"));
        let cx = HeadlessAppContext::with_platform(text_system, Arc::new(()), || {
            gpui_wgpu::WgpuHeadlessRenderer::new()
                .map(|r| Some(Box::new(r) as Box<dyn gpui::PlatformHeadlessRenderer>))
        });
        Ok(Studio { cx })
    }

    /// Renders `frames` frames of the scene built by `build` into an MP4 at `out`.
    pub fn render<S: Scene>(
        &mut self,
        settings: &Settings,
        out: &Path,
        build: impl FnOnce(&mut gpui::Context<S>) -> S + 'static,
    ) -> Result<Report> {
        let Settings {
            width,
            height,
            fps,
            frames,
            crf,
            preset,
        } = settings.clone();
        ensure!(
            width % 2 == 0 && height % 2 == 0,
            "width and height must be even"
        );
        let bounds = Bounds::new(
            gpui::point(px(0.0), px(0.0)),
            size(px(width as f32), px(height as f32)),
        );
        let handle = self.cx.update(|app| {
            app.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_, app| app.new(build),
            )
        })?;
        // the test window defaults to a 2x display; the video is drawn at one pixel per point
        self.cx
            .simulate_window_scale_factor_change(handle.into(), 1.0);
        self.cx.run_until_parked();
        let view: Entity<S> = self.cx.update(|app| handle.entity(app))?;

        let mut cfg = EncoderConfig::new(width, height, fps, 1);
        cfg.profile = Profile::High;
        cfg.preset = preset;
        cfg.rate = RateControl::Crf(crf);
        cfg.bframes = 0;
        cfg.keyint = fps;
        cfg.format = PacketFormat::LengthPrefixed;
        let mut enc = Encoder::new(cfg)?;
        enc.set_recon_capture(true);
        let avcc = AvcConfig::parse(&enc.avcc())?;
        let file = std::io::BufWriter::new(
            std::fs::File::create(out).with_context(|| format!("create {}", out.display()))?,
        );
        let mut mp4 = Mp4Writer::new(file, WriterOptions::new(Brand::Mp4))?;
        let track = mp4.add_track(TrackConfig::new(
            SampleEntry::avc(avcc, width as u16, height as u16),
            fps,
        ))?;

        let (w, h) = (width as usize, height as usize);
        let (mut y, mut u, mut v) = (vec![0u8; w * h], vec![0u8; w * h / 4], vec![0u8; w * h / 4]);
        let (mut t_render, mut t_convert, mut t_encode) =
            (Duration::ZERO, Duration::ZERO, Duration::ZERO);
        let mut bytes = 0u64;
        let mut recon = BTreeMap::new();
        let mut sink =
            |enc: &mut Encoder, mp4: &mut Mp4Writer<_>, packets: Vec<Packet>| -> Result<()> {
                for p in packets {
                    bytes += p.data.len() as u64;
                    mp4.write_sample(
                        track,
                        WriteSample {
                            data: &p.data,
                            duration: 1,
                            composition_offset: (p.pts - p.dts) as i32,
                            is_sync: p.keyframe,
                        },
                    )?;
                }
                for r in enc.take_recon() {
                    recon.insert(
                        r.pts,
                        hash_planes([
                            (&r.y, w, h, w),
                            (&r.u, w / 2, h / 2, w / 2),
                            (&r.v, w / 2, h / 2, w / 2),
                        ]),
                    );
                }
                Ok(())
            };

        let all = Instant::now();
        for i in 0..frames {
            let t = Instant::now();
            self.cx.update(|app| {
                view.update(app, |scene, cx| {
                    scene.seek(i, fps);
                    cx.notify();
                })
            });
            self.cx.run_until_parked();
            let image = self.cx.capture_screenshot(handle.into())?;
            ensure!(
                image.width() == width && image.height() == height,
                "captured {}x{}",
                image.width(),
                image.height()
            );
            t_render += t.elapsed();
            let t = Instant::now();
            rgba_to_i420(image.as_raw(), w, h, &mut y, &mut u, &mut v);
            t_convert += t.elapsed();
            let t = Instant::now();
            let packets = enc.encode(
                &YuvFrame {
                    y: &y,
                    u: &u,
                    v: &v,
                    y_stride: w,
                    uv_stride: w / 2,
                },
                i as i64,
            )?;
            sink(&mut enc, &mut mp4, packets)?;
            t_encode += t.elapsed();
        }
        let packets = enc.flush();
        sink(&mut enc, &mut mp4, packets)?;
        mp4.finish()?;
        let total = all.elapsed().as_secs_f64();
        Ok(Report {
            frames,
            width,
            height,
            fps,
            render_secs: t_render.as_secs_f64(),
            convert_secs: t_convert.as_secs_f64(),
            encode_secs: t_encode.as_secs_f64(),
            total_secs: total,
            throughput_fps: frames as f64 / total,
            bytes,
            recon,
        })
    }
}

/// Result of [`verify`].
#[derive(Clone, Debug, Serialize)]
pub struct Verification {
    pub samples: usize,
    pub decoded: usize,
    pub mismatches: usize,
    pub secs: f64,
}

/// Demuxes `path`, decodes every sample with craft-video's H.264 decoder and compares each picture
/// with the encoder's reconstruction recorded in `report`. Passes only when all frames match bit for bit.
pub fn verify(path: &Path, report: &Report) -> Result<Verification> {
    let t = Instant::now();
    let mp4 = craft_media::isobmff::open(std::fs::File::open(path)?)?;
    let src = std::fs::File::open(path)?;
    let track = mp4.tracks.first().context("no track")?;
    let avcc = match track.codec() {
        Some(craft_media::isobmff::CodecConfig::Avc(c)) => c.to_bytes(),
        _ => anyhow::bail!("first track is not H.264"),
    };
    let mut dec = craft_video::h264::decode::Decoder::from_avcc(&avcc)?;
    let mut pictures = Vec::new();
    for (i, s) in track.samples.iter().enumerate() {
        pictures.extend(dec.decode(&mp4.read_sample(&src, 0, i)?, s.pts)?);
    }
    pictures.extend(dec.flush());
    let mismatches = pictures
        .iter()
        .filter(|p| {
            let (cw, ch) = (p.chroma_width as usize, p.chroma_height as usize);
            let got = hash_planes([
                (&p.y, p.width as usize, p.height as usize, p.y_stride),
                (&p.u, cw, ch, p.uv_stride),
                (&p.v, cw, ch, p.uv_stride),
            ]);
            report.recon.get(&p.pts) != Some(&got)
        })
        .count();
    let v = Verification {
        samples: track.samples.len(),
        decoded: pictures.len(),
        mismatches,
        secs: t.elapsed().as_secs_f64(),
    };
    ensure!(
        v.mismatches == 0 && v.decoded == report.frames as usize,
        "decoded {} of {} frames, {} differ from the encoder reconstruction",
        v.decoded,
        report.frames,
        v.mismatches
    );
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i420_uses_bt709_limited_range() {
        let (w, h) = (4usize, 2usize);
        let mut rgba = vec![255u8; w * h * 4];
        rgba[..2 * 4].fill(0); // first two pixels of row 0 black
        for px in rgba.chunks_mut(4) {
            px[3] = 255;
        }
        let (mut y, mut u, mut v) = (vec![0u8; w * h], vec![0u8; w * h / 4], vec![0u8; w * h / 4]);
        rgba_to_i420(&rgba, w, h, &mut y, &mut u, &mut v);
        assert_eq!(&y[..4], &[16, 16, 235, 235]);
        assert_eq!(u[1], 128);
        assert_eq!(v[1], 128);
    }
}
