#[cfg(not(target_os = "android"))]
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(ayvpn_daemon::cli::main(&args));
}

/// Mullvad's management interface has no Android client; the Android app embeds the daemon.
#[cfg(target_os = "android")]
fn main() {
    eprintln!("ayvpn: the command line is not available on Android");
    std::process::exit(1);
}
