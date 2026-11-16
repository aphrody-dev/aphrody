use std::path::PathBuf;
use std::sync::Arc;

use gpui::{AppContext as _, Bounds, HeadlessAppContext, WindowBounds, WindowOptions, px, size};
use os_desktop::windows::WindowKind;
use os_desktop::{DesktopOptions, OsDesktopShell, bind_desktop_keys};
use os_session::TilingMode;
use os_ui::AppearanceMode;

fn screenshot_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("APHRODY_SHOT_DIR") {
        return PathBuf::from(custom);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/os_screenshots")
}

fn assert_non_blank_png(img: &image::RgbaImage, path: &std::path::Path) {
    assert!(
        img.width() >= 1440 && img.height() >= 900,
        "expected at least 1440x900 pixels for {} (got {}x{})",
        path.display(),
        img.width(),
        img.height()
    );

    let mut distinct_buckets = std::collections::BTreeSet::new();
    for pixel in img.pixels().step_by(37) {
        let bucket = (pixel[0] / 16, pixel[1] / 16, pixel[2] / 16);
        distinct_buckets.insert(bucket);
    }
    assert!(
        distinct_buckets.len() >= 4,
        "rendered image {} looks nearly blank ({} buckets)",
        path.display(),
        distinct_buckets.len()
    );
}

#[test]
fn desktop_headless_renders_and_interacts() {
    let text_system = Arc::new(gpui_wgpu::CosmicTextSystem::new(brand::FONT_SANS_ALIAS));
    let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(assets::Assets), || {
        gpui_platform::current_headless_renderer()
    });

    cx.update(|app| {
        gpui_ui::init(app);
        bind_desktop_keys(app);
    });

    let bounds = Bounds::new(gpui::point(px(0.0), px(0.0)), size(px(1440.0), px(900.0)));
    let handle = cx
        .update(|app| {
            app.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |window, app| {
                    app.new(|cx| {
                        OsDesktopShell::new(
                            DesktopOptions {
                                initial_theme: Some("aphrody".to_owned()),
                                initial_mode: Some(AppearanceMode::Dark),
                                initial_app: Some("photocraft".to_owned()),
                                ..Default::default()
                            },
                            window,
                            cx,
                        )
                    })
                },
            )
        })
        .expect("headless desktop window must open");

    cx.run_until_parked();

    let out_dir = screenshot_dir();
    let can_capture_gpu = gpui_platform::current_headless_renderer()
        .ok()
        .flatten()
        .is_some();

    if can_capture_gpu {
        std::fs::create_dir_all(&out_dir).expect("create screenshot output dir");

        let shot_dark = cx
            .capture_screenshot(handle.into())
            .expect("capture Aphrody Dark studio screenshot");
        let p_dark = out_dir.join("aphrody_os_studio_dark.png");
        shot_dark.save(&p_dark).expect("save dark studio PNG");
        assert_non_blank_png(&shot_dark, &p_dark);
    }

    // Mutate state through real desktop commands: tile multiple windows, switch light theme, toggle drawer.
    cx.update(|app| {
        handle
            .update(app, |shell, _window, cx| {
                shell.select_craft_app("vectorcraft", cx);
                shell.open_or_focus_window(WindowKind::Settings, "COSMIC Settings", cx);
                shell.open_or_focus_window(WindowKind::Monitor, "System Monitor", cx);
                shell.set_layout(TilingMode::CosmicGrid, cx);
                shell.toggle_mode(cx);
            })
            .expect("update desktop shell for light tiled view");
    });
    cx.run_until_parked();

    if can_capture_gpu {
        let shot_tiled_light = cx
            .capture_screenshot(handle.into())
            .expect("capture Light tiled multi-window screenshot");
        let p_light = out_dir.join("aphrody_os_tiled_light.png");
        shot_tiled_light
            .save(&p_light)
            .expect("save light tiled PNG");
        assert_non_blank_png(&shot_tiled_light, &p_light);
    }

    // Switch to COSMIC dark theme + open Craft Anything + Launcher modal + ContextDrawer.
    cx.update(|app| {
        handle
            .update(app, |shell, _window, cx| {
                shell.toggle_mode(cx);
                shell.select_theme("cosmic", cx);
                shell.select_craft_app("craft-anything", cx);
                shell.toggle_drawer(cx);
                shell.toggle_launcher(cx);
            })
            .expect("update desktop shell for cosmic launcher + drawer view");
    });
    cx.run_until_parked();

    if can_capture_gpu {
        let shot_cosmic = cx
            .capture_screenshot(handle.into())
            .expect("capture COSMIC dark launcher + inspector drawer screenshot");
        let p_cosmic = out_dir.join("aphrody_os_cosmic_launcher.png");
        shot_cosmic
            .save(&p_cosmic)
            .expect("save cosmic launcher PNG");
        assert_non_blank_png(&shot_cosmic, &p_cosmic);
    }

    #[cfg(target_os = "linux")]
    assert!(
        can_capture_gpu,
        "Linux test environment on fr1 should expose WgpuHeadlessRenderer for real PNG visual proof"
    );
}
