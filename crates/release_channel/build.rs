fn main() {
    println!("cargo::rustc-check-cfg=cfg(__do_not_set_zed_release_channel)");
    println!("cargo::rerun-if-env-changed=APHRODY_RELEASE_CHANNEL");
    println!("cargo::rerun-if-env-changed=ZED_RELEASE_CHANNEL");

    if std::env::var("APHRODY_RELEASE_CHANNEL").is_ok()
        || std::env::var("ZED_RELEASE_CHANNEL").is_ok()
    {
        println!("cargo::rustc-cfg=__do_not_set_zed_release_channel");
    }
}
