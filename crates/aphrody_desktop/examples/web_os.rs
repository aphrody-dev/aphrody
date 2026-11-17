use aphrody_desktop::DesktopOptions;

fn main() {
    #[cfg(target_family = "wasm")]
    aphrody_desktop::start_web_app(DesktopOptions::default());

    #[cfg(not(target_family = "wasm"))]
    if let Err(err) = aphrody_desktop::run_desktop_app(DesktopOptions::default()) {
        eprintln!("aphrody web_os launch error: {err:#}");
        std::process::exit(1);
    }
}
