---
name: video-studio
description: "Headless video studio for agents, pure Rust end to end: GPUI views rendered offscreen with wgpu, H.264 encoded and MP4 muxed with FilmCraft's craft-video/craft-media, verified bit for bit by decoding again; 1920x1080 at 120 fps. Also the asset dataset (optimized, content-addressed, manifest, tar.zst on R2). Use when an agent must produce, benchmark or validate a video, or build a video asset dataset, without ffmpeg or Node."
---
# Video studio (GPUI + FilmCraft, no ffmpeg, no Node)

## Pipeline

1. Scene: a GPUI view implementing `gpui_video_studio::Scene` (`seek(frame, fps)` then `Render`).
2. Render: `HeadlessAppContext::with_platform` with `gpui_wgpu::WgpuHeadlessRenderer` (feature `test-support`).
   On Windows `gpui_platform::current_headless_renderer()` returns `None`, so pass the wgpu renderer directly.
   Each frame: `view.update(.. seek; cx.notify())`, `run_until_parked()`, `capture_screenshot(window)` gives RGBA.
3. Convert: BT.709 limited range RGBA to I420 (2x2 chroma average), rows in parallel (rayon).
4. Encode: `craft_video::h264::Encoder` (features `h264,threads`), `Profile::High`, `Preset::Speed`,
   `RateControl::Crf(18..20)`, `bframes = 0`, `keyint = fps`, `PacketFormat::LengthPrefixed`, `set_recon_capture(true)`.
5. Mux: `craft_media::isobmff::Mp4Writer` (feature `isobmff`), track timescale = fps, sample duration 1, `avcC` from
   `AvcConfig::parse(&enc.avcc())`. Use `finish()`; `finish_faststart()` needs a `Read + Seek` writer.
6. Verify: `craft_media::isobmff::open`, `Decoder::from_avcc`, decode every sample; each picture's hash must equal the
   encoder reconstruction of the same pts. `mismatches = 0` and `decoded = frames` is the pixel-perfect proof.

Code: `aphrody-labs/aphrody` crate `crates/gpui_video_studio` (bin `gpui-video [frames] [out.mp4]`, one JSON line).
Codecs live in `vendor/craft-libs/crates/{video,media}` (storytold/craft-libs fork, Apache-2.0 / MIT).

## Measured (2026-11-19, Windows 11, 24 threads, release)

- tiny-skia CPU scene + craft H.264 + MP4: 720 frames 1920x1080@120 in 3.15 s (229 fps); with recon capture and
  decode verification 4.24 s + 1.22 s, 720/720 frames bit-identical.
- GPUI wgpu headless (`gpui-video 720`): 720 frames 1920x1080@120 in 6.18 s (116 fps): render + readback 2.54 s,
  I420 0.35 s, encode + mux 3.15 s; decode verification 1.23 s, 720/720 frames bit-identical. The encoder is the
  bottleneck; frame 361 decoded to PNG shows the expected text and circle position.
- The headless test window starts at scale factor 2 (3840x2160 captures): call
  `simulate_window_scale_factor_change(window, 1.0)` before rendering.

## Dataset

`dataset <repo> <out>` (Rust: image, ruzstd, tar, sha2): tracked assets -> PNG to lossless WebP when smaller, other
formats copied, deduplicated by sha256 into `objects/<2>/<sha>.<ext>`, `manifest.json` (paths, mime, sizes, dims,
`license_ref`, `review`), `SHA256SUMS`, one `*.tar.zst`. Fonts count only a licence file in their own directory;
assets with no licence file are `review: true` and go to the partner review, never decided by agents.
Upload to a private R2 bucket (check `wrangler r2 bucket dev-url get <b>` says disabled) under `<name>/v<N>/`.

## Rules

- Third-party game assets (Level-5 / Inazuma Eleven) never enter a dataset or a video without the owner's written OK.
- Nothing is sent or published from a render until the owner validates it.
- Quote only numbers from a run in the same session.
