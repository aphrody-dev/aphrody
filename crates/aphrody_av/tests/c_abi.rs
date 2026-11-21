#![allow(
    clippy::disallowed_methods,
    reason = "the test drives a separate process and no GPUI executor is involved"
)]
//! Compiles `smoke/c_smoke.c` with `cc` against the cdylib and runs it.
mod common;
use aphrody_av::{Codec, Format};
use common::*;
use std::path::PathBuf;
use std::process::Command;

fn library_directory() -> PathBuf {
    // the CLI binary and the cdylib are both final artifacts of target/<profile>
    PathBuf::from(env!("CARGO_BIN_EXE_aphrody-av"))
        .parent()
        .expect("profile dir")
        .to_path_buf()
}

#[test]
fn c_program_drives_the_abi() {
    let library = library_directory();
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let shared = ["libaphrody_av.so", "libaphrody_av.dylib"]
        .iter()
        .map(|n| library.join(n))
        .find(|p| p.exists());
    let Some(_shared) = shared else {
        panic!(
            "cdylib not found in {}: build the crate library first (cargo build -p aphrody_av --lib)",
            library.display()
        );
    };
    let work = tempfile::tempdir().expect("tempdir");
    let binary = work.path().join("c_smoke");
    let compile = Command::new("cc")
        .arg(manifest.join("smoke/c_smoke.c"))
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg("-I")
        .arg(manifest.join("include"))
        .arg("-L")
        .arg(&library)
        .arg("-laphrody_av")
        .arg("-o")
        .arg(&binary)
        .output()
        .expect("run cc");
    assert!(
        compile.status.success(),
        "cc failed:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let input = work.path().join("in.mp4");
    std::fs::write(
        &input,
        av_file(Format::Mp4, Codec::H264, Codec::Aac, 12, (64, 48)),
    )
    .expect("write");
    let run = Command::new(&binary)
        .arg(&input)
        .arg(work.path())
        .env("LD_LIBRARY_PATH", &library)
        .env("DYLD_LIBRARY_PATH", &library)
        .output()
        .expect("run c smoke");
    let stdout = String::from_utf8_lossy(&run.stdout);
    let stderr = String::from_utf8_lossy(&run.stderr);
    println!("{stdout}");
    assert!(
        run.status.success(),
        "c smoke failed:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(stdout.contains("c smoke: ok"), "{stdout}");
}
