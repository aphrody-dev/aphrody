mod common;
use aphrody_av::{Codec, Format, probe};
use common::*;

const FIXTURE: &str = "smoke/fixtures/clip.mp4";

/// The tiny clip the Python, Bun and C smoke tests read. `UPDATE_FIXTURES=1` rewrites it.
#[test]
fn smoke_fixture_is_a_valid_h264_aac_clip() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    if std::env::var_os("UPDATE_FIXTURES").is_some() {
        std::fs::write(
            &path,
            av_file(Format::Mp4, Codec::H264, Codec::Aac, 12, (64, 48)),
        )
        .expect("write fixture");
    }
    let bytes = std::fs::read(&path).expect("fixture present");
    let info = probe(&bytes, Some("clip.mp4")).expect("probe fixture");
    assert_eq!(info.format, "mp4");
    assert_eq!(
        (
            info.streams[0].codec.as_str(),
            info.streams[0].frames,
            info.streams[1].codec.as_str()
        ),
        ("h264", Some(12), "aac")
    );
    assert!(bytes.len() < 20_000, "fixture is {} bytes", bytes.len());
}
