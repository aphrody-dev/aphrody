mod common;
use aphrody_av::caps::{can_decode, can_encode};
use aphrody_av::mux::mux_supports;
use aphrody_av::{TrackKind, decode_video, encode_video, probe};
use common::*;

const FRAMES: usize = 8;
const SIZE: (u32, u32) = (64, 48);

/// Every (video codec, container) pair the table claims: encode, mux, probe, demux, and decode back
/// with frame count and dimensions preserved.
#[test]
fn video_codec_by_container_matrix() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for codec in video_codecs().into_iter().filter(|c| can_encode(*c)) {
        for format in containers() {
            if !mux_supports(format, codec) {
                continue;
            }
            checked += 1;
            let label = format!("{} in {}", codec.name(), format.name());
            let pictures = clip(SIZE.0, SIZE.1, FRAMES);
            let file = match encode_video(&pictures, fps_for(codec), format, codec, None) {
                Ok(file) => file,
                Err(error) => {
                    failures.push(format!("{label}: encode/mux failed: {error}"));
                    continue;
                }
            };
            let info = match probe(&file, None) {
                Ok(info) => info,
                Err(error) => {
                    failures.push(format!("{label}: probe failed: {error}"));
                    continue;
                }
            };
            let video = info.streams.iter().find(|s| s.kind == "video");
            let Some(video) = video else {
                failures.push(format!(
                    "{label}: probe found no video stream in {:?}",
                    info.streams
                ));
                continue;
            };
            if info.format != format.name()
                && !(format == aphrody_av::Format::Mov && info.format == "mov")
            {
                failures.push(format!("{label}: probed as {}", info.format));
            }
            if video.codec != codec.name()
                || video.width != Some(SIZE.0)
                || video.height != Some(SIZE.1)
            {
                failures.push(format!(
                    "{label}: probe says {} {:?}x{:?}",
                    video.codec, video.width, video.height
                ));
            }
            if video.frames != Some(FRAMES as u64) {
                failures.push(format!("{label}: probe counts {:?} frames", video.frames));
            }
            if can_decode(codec) {
                match decode_video(&file, None)
                    .and_then(|frames| frames.collect::<Result<Vec<_>, _>>())
                {
                    Ok(decoded) => {
                        if decoded.len() != FRAMES {
                            failures.push(format!(
                                "{label}: decoded {} frames, expected {FRAMES}",
                                decoded.len()
                            ));
                        } else if decoded
                            .iter()
                            .any(|f| (f.picture.width, f.picture.height) != SIZE)
                        {
                            failures.push(format!("{label}: decoded dimensions differ"));
                        }
                    }
                    Err(error) => failures.push(format!("{label}: decode failed: {error}")),
                }
            }
        }
    }
    println!("checked {checked} video cells");
    assert!(
        failures.is_empty(),
        "{} of {checked} cells failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
    let _ = TrackKind::Video;
}
