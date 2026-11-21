//! Renders the app offscreen through the headless GPUI renderer and saves PNGs. Skipped (with a
//! message) when no GPU or software Vulkan renderer is available.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use aphrody_ui::AppearanceMode;
use aphrody_video::library::Library;
use aphrody_video::{Route, VideoApp};
use gpui::{
    AppContext as _, Bounds, Entity, HeadlessAppContext, WindowBounds, WindowHandle, WindowOptions,
    point, px, size,
};

fn shot_dir() -> PathBuf {
    std::env::var_os("APHRODY_SHOT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/video_screenshots")
        })
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/library")
}

fn assert_picture(image: &image::RgbaImage, width: u32, height: u32, name: &str) {
    assert!(
        image.width() >= width && image.height() >= height,
        "{name}: {}x{}",
        image.width(),
        image.height()
    );
    let buckets: std::collections::BTreeSet<_> = image
        .pixels()
        .step_by(37)
        .map(|pixel| (pixel[0] / 16, pixel[1] / 16, pixel[2] / 16))
        .collect();
    assert!(
        buckets.len() >= 6,
        "{name} looks blank ({} colour buckets)",
        buckets.len()
    );
}

#[test]
fn renders_every_surface_offscreen() {
    let text_system = Arc::new(gpui_wgpu::CosmicTextSystem::new(brand::FONT_SANS_ALIAS));
    let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(assets::Assets), || {
        gpui_platform::current_headless_renderer()
    });
    if gpui_platform::current_headless_renderer()
        .ok()
        .flatten()
        .is_none()
    {
        eprintln!("no headless renderer on this host: screenshots skipped");
        return;
    }
    cx.update(|app| {
        gpui_ui::init(app);
        aphrody_video::init(app);
    });
    let directory = shot_dir();
    std::fs::create_dir_all(&directory).expect("create screenshot directory");
    let library = Library::scan(&fixture_root()).expect("scan");

    let scenes: Vec<(
        &str,
        f32,
        f32,
        bool,
        fn(&mut VideoApp, &mut gpui::Context<VideoApp>),
        bool,
    )> = vec![
        ("home_dark_1440", 1440.0, 900.0, false, |_, _| {}, false),
        ("home_light_1440", 1440.0, 900.0, true, |_, _| {}, false),
        (
            "home_mini_nav_1440",
            1440.0,
            900.0,
            false,
            |app, cx| app.toggle_menu(cx),
            false,
        ),
        ("home_narrow_400", 400.0, 800.0, false, |_, _| {}, false),
        (
            "watch_dark_1440",
            1440.0,
            900.0,
            false,
            |app, cx| {
                let id = app
                    .library()
                    .items()
                    .iter()
                    .find(|item| item.title.starts_with("Coastline"))
                    .map_or(0, |item| item.id);
                app.open_watch(id, cx);
            },
            true,
        ),
        (
            "watch_theater_light_1440",
            1440.0,
            900.0,
            true,
            |app, cx| {
                let id = app
                    .library()
                    .items()
                    .iter()
                    .find(|item| item.title.starts_with("Neon"))
                    .map_or(0, |item| item.id);
                app.open_watch(id, cx);
                app.set_theater(true);
            },
            true,
        ),
        (
            "search_dark_1440",
            1440.0,
            900.0,
            false,
            |app, cx| app.navigate(Route::Search("tide".into()), cx),
            false,
        ),
        (
            "creator_dark_1440",
            1440.0,
            900.0,
            false,
            |app, cx| app.navigate(Route::Creator("Harbour Films".into()), cx),
            false,
        ),
        (
            "shorts_dark_1440",
            1440.0,
            900.0,
            false,
            |app, cx| app.navigate(Route::Shorts, cx),
            true,
        ),
    ];

    for (name, width, height, light, setup, wants_frame) in scenes {
        let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(width), px(height)));
        let library = library.clone();
        let handle: WindowHandle<VideoApp> = cx
            .update(|app| {
                app.open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        ..Default::default()
                    },
                    |window, app| {
                        let entity: Entity<VideoApp> = app.new(|cx| VideoApp::new(window, cx));
                        entity.update(app, |video, cx| {
                            video.set_skeleton_duration(Duration::ZERO);
                            video.set_library(library, cx);
                        });
                        entity
                    },
                )
            })
            .expect("open window");
        cx.run_until_parked();
        cx.update(|app| {
            handle
                .update(app, |video, _window, cx| {
                    video.set_mode(
                        if light {
                            AppearanceMode::Light
                        } else {
                            AppearanceMode::Dark
                        },
                        cx,
                    );
                    setup(video, cx);
                })
                .expect("setup scene");
        });
        for _ in 0..600 {
            cx.advance_clock(Duration::from_millis(33));
            cx.run_until_parked();
            let ready = cx.update(|app| {
                handle
                    .read_with(app, |video, _| {
                        video.poster_count() >= 9 && (!wants_frame || video.has_frame())
                    })
                    .unwrap_or(false)
            });
            if ready {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        // Let playback advance a little so the control bar and clock show real values.
        for _ in 0..20 {
            cx.advance_clock(Duration::from_millis(33));
            cx.run_until_parked();
        }
        let image = cx.capture_screenshot(handle.into()).expect("capture");
        let path = directory.join(format!("{name}.png"));
        image.save(&path).expect("save png");
        assert_picture(&image, width as u32, height as u32, name);
        eprintln!("saved {}", path.display());
    }
}
