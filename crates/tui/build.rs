//! OpenTUI's native library is linked dynamically (`default-features = false`), so an executable must find
//! `libopentui.so` at run time: its directory (`OPENTUI_LIB_DIR`, set by `.cargo/config.toml`) becomes an rpath.

fn main() {
    println!("cargo:rerun-if-env-changed=OPENTUI_LIB_DIR");
    if let Some(dir) = std::env::var_os("OPENTUI_LIB_DIR") {
        let target = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
        if target == "linux" || target == "macos" {
            println!("cargo:rustc-link-arg-bins=-Wl,-rpath,{}", std::path::Path::new(&dir).display());
        }
    }
}
