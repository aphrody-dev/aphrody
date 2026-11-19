//! `gpui-video [frames] [out.mp4]`: renders the demo scene at 1920x1080, 120 fps, verifies the
//! MP4 bit for bit and prints one JSON line with the timings.

use std::path::PathBuf;

use gpui::{Context, IntoElement, ParentElement as _, Render, Styled as _, Window, div, px, rgb};
use gpui_video_studio::{Scene, Settings, Studio, verify};

struct Demo {
    frame: u32,
    fps: u32,
    frames: u32,
}

impl Scene for Demo {
    fn seek(&mut self, frame: u32, fps: u32) {
        self.frame = frame;
        self.fps = fps;
    }
}

impl Render for Demo {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let t = self.frame as f32 / self.frames.max(1) as f32;
        let x = 160.0 + t * 1280.0;
        div()
            .size_full()
            .bg(rgb(0x0c0c10))
            .child(
                div()
                    .absolute()
                    .left(px(x))
                    .top(px(380.0))
                    .size(px(320.0))
                    .rounded(px(160.0))
                    .bg(rgb(0xe6406f)),
            )
            .child(
                div()
                    .absolute()
                    .left(px(96.0))
                    .top(px(96.0))
                    .text_size(px(64.0))
                    .text_color(rgb(0xf4f4f6))
                    .child(format!(
                        "frame {} / {} at {} fps",
                        self.frame + 1,
                        self.frames,
                        self.fps
                    )),
            )
    }
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let frames: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(720);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "gpui-video.mp4".into()));
    let settings = Settings {
        frames,
        ..Settings::default()
    };
    let mut studio = Studio::new()?;
    let report = studio.render(&settings, &out, move |_| Demo {
        frame: 0,
        fps: 120,
        frames,
    })?;
    let check = verify(&out, &report)?;
    println!(
        "{}",
        serde_json::json!({ "out": out, "report": report, "verify": check })
    );
    Ok(())
}
