//! With the `tui` feature the binary links OpenTUI's native library dynamically: its directory (`OPENTUI_LIB_DIR`, set
//! by `.cargo/config.toml`) becomes an rpath so `open` starts without LD_LIBRARY_PATH.

fn main() {
    println!("cargo:rerun-if-env-changed=OPENTUI_LIB_DIR");
    if std::env::var_os("CARGO_FEATURE_TUI").is_none() {
        return;
    }
    if let Some(dir) = std::env::var_os("OPENTUI_LIB_DIR") {
        let target = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
        if target == "linux" || target == "macos" {
            println!("cargo:rustc-link-arg-bins=-Wl,-rpath,{}", std::path::Path::new(&dir).display());
        }
    }
}
