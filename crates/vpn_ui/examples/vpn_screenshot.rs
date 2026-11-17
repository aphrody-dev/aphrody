//! Renders the VPN view against the daemon that is running on this machine, headless, and saves
//! PNG screenshots.
//!
//! ```sh
//! cargo run -p vpn_ui --example vpn_screenshot -- <output-dir>
//! ```

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, bail};
use gpui::{AppContext as _, Bounds, HeadlessAppContext, WindowBounds, WindowOptions, px, size};
use vpn_ui::{DaemonBackend, VpnView};

const SETTLE_TIMEOUT: Duration = Duration::from_secs(30);

fn main() -> Result<()> {
    let output_dir = PathBuf::from(
        std::env::args()
            .nth(1)
            .context("usage: vpn_screenshot <output-dir>")?,
    );
    std::fs::create_dir_all(&output_dir)?;

    let text_system = Arc::new(gpui_wgpu::CosmicTextSystem::new(brand::FONT_SANS_ALIAS));
    let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(assets::Assets), || {
        gpui_platform::current_headless_renderer()
    });
    cx.update(|app| {
        gpui_tokio::init(app);
        gpui_ui::init(app);
    });

    let bounds = Bounds::new(gpui::point(px(0.0), px(0.0)), size(px(480.0), px(1180.0)));
    let handle = cx
        .update(|app| {
            app.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |window, app| {
                    app.new(|cx| VpnView::new(Arc::new(DaemonBackend::new()), window, cx))
                },
            )
        })
        .context("open the headless window")?;

    // The daemon answers on a tokio thread, so wall-clock time has to pass between pumps.
    let started = Instant::now();
    loop {
        cx.run_until_parked();
        let settled = cx.update(|app| {
            handle
                .read(app)
                .map(|view| view.snapshot().is_some() || view.daemon_error().is_some())
        })?;
        if settled {
            break;
        }
        if started.elapsed() > SETTLE_TIMEOUT {
            bail!("the daemon did not answer within {SETTLE_TIMEOUT:?}");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    std::thread::sleep(Duration::from_millis(500));
    cx.run_until_parked();

    let main_view = output_dir.join("vpn_main.png");
    cx.capture_screenshot(handle.into())?.save(&main_view)?;

    cx.update(|app| handle.update(app, |view, _window, cx| view.show_locations(true, cx)))?;
    cx.run_until_parked();
    let locations = output_dir.join("vpn_locations.png");
    cx.capture_screenshot(handle.into())?.save(&locations)?;

    println!("{}\n{}", main_view.display(), locations.display());
    Ok(())
}
