# gpui_video_studio

Headless video studio for agents. A GPUI view implements `Scene::seek(frame, fps)`. `Studio::render` draws it in
a headless GPUI window backed by `gpui_wgpu::WgpuHeadlessRenderer` and reads back each frame as RGBA. Each frame is
converted to BT.709 4:2:0, encoded with the pure-Rust H.264 encoder of `craft-video` (FilmCraft's craft-libs), and
muxed to MP4 with `craft-media`. `verify` demuxes the file, decodes it with `craft-video`'s decoder and requires every
picture to be bit-identical to the encoder's reconstruction. No ffmpeg, no Node, no C codec.

```sh
cargo run --release -p gpui_video_studio --bin gpui-video -- 720 out.mp4
```

The command prints one JSON line: per-stage seconds, overall fps, bytes, and the verification counts.

## Measured

Windows 11, 24 threads, release, `Preset::Speed`, CRF 18, 1920x1080, 120 fps, 720 frames (6 s):

| Stage | Seconds |
| --- | --- |
| GPUI render + GPU readback | 2.54 |
| RGBA to I420 | 0.35 |
| H.264 encode + MP4 mux | 3.15 |
| Total | 6.18 (116 fps) |
| Decode verification | 1.23, 720/720 frames identical |

With a tiny-skia CPU scene instead of GPUI, the same encoder and muxer took 3.15 s (229 fps) without recon capture.

On Windows `gpui_platform::current_headless_renderer()` returns `None`, so the studio builds the wgpu headless
renderer itself (feature `test-support` of `gpui_wgpu`). The headless test window starts at scale factor 2; the
studio sets it to 1 so one point is one pixel.
