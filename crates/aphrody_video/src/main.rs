use std::path::PathBuf;

fn main() {
    let root = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .or_else(aphrody_video::default_library_root);
    gpui_ui::application().run(move |cx| {
        gpui_ui::init(cx);
        aphrody_video::init(cx);
        if let Err(error) = aphrody_video::open_window(cx, root) {
            log::error!("failed to open {}: {error:#}", aphrody_video::APP_NAME);
            cx.quit();
            return;
        }
        cx.activate(true);
    });
}
