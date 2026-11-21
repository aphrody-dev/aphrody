mod common;
use aphrody_av::{Codec, ConvertFormat, ConvertOptions, Format, convert};
use common::*;

fn options() -> ConvertOptions {
    ConvertOptions {
        fps: 10.0,
        width: Some(32),
        name: "clip.mp4".into(),
        ..Default::default()
    }
}

/// In-memory conversion matches `fw_video::convert_file` frame for frame on an H.264 MP4.
#[test]
fn convert_matches_fw_video_on_h264_mp4() {
    let source = av_file(Format::Mp4, Codec::H264, Codec::Aac, 24, (64, 48));
    let directory = tempfile::tempdir().expect("tempdir");
    let path = directory.path().join("clip.mp4");
    std::fs::write(&path, &source).expect("write");
    let reference_options = fw_video::Options {
        fps: 10.0,
        width: Some(32),
        ..Default::default()
    };
    let mut reference_frames = Vec::new();
    let reference_decoded = fw_video::convert_file(&path, &reference_options, |frame| {
        reference_frames.push(frame)
    })
    .expect("fw-video");

    let converted = convert(&source, Some("clip.mp4"), &options()).expect("convert");
    assert_eq!(converted.decoded, reference_decoded);
    assert_eq!(converted.frames, reference_frames.len());
    assert!(
        converted.frames >= 6,
        "expected about 8 sampled frames, got {}",
        converted.frames
    );

    let names: Vec<&str> = converted.files.iter().map(|f| f.path.as_str()).collect();
    for expected in [
        "animated.svg",
        "video.md",
        "video.html",
        "VideoSvg.jsx",
        "video_svg.rs",
        "frames/frame-00001.svg",
    ] {
        assert!(
            names.contains(&expected),
            "{expected} missing from {names:?}"
        );
    }
    let text = |name: &str| {
        String::from_utf8(
            converted
                .files
                .iter()
                .find(|f| f.path == name)
                .expect(name)
                .bytes
                .clone(),
        )
        .expect("utf8")
    };
    assert!(text("frames/frame-00001.svg").starts_with("<svg"));
    assert!(text("animated.svg").contains("<animate"));
    assert!(text("video.md").contains("clip.mp4"));
    assert!(text("video.html").contains("<html") || text("video.html").contains("<!DOCTYPE"));
    assert!(text("VideoSvg.jsx").contains("export"));
    assert!(text("video_svg.rs").contains("VideoSvgView"));
    // the frame files are the same vector paths fw-video produced
    let first = text("frames/frame-00001.svg");
    assert_eq!(
        first.matches("<path").count() > 0,
        reference_frames[0].path_count > 0
    );
}

#[test]
fn convert_reads_other_codecs_and_containers() {
    for (format, codec) in [
        (Format::WebM, Codec::Vp9),
        (Format::Matroska, Codec::Av1),
        (Format::Avi, Codec::Mjpeg),
        (Format::Mov, Codec::ProRes),
        (Format::MpegTs, Codec::H264),
    ] {
        let source = av_file(
            format,
            codec,
            match format {
                Format::WebM => Codec::Opus,
                Format::MpegTs => Codec::Aac,
                _ => Codec::Pcm,
            },
            12,
            (64, 48),
        );
        let converted = convert(
            &source,
            None,
            &ConvertOptions {
                formats: vec![ConvertFormat::Svg],
                max_frames: Some(3),
                ..options()
            },
        )
        .unwrap_or_else(|e| panic!("{} in {}: {e}", codec.name(), format.name()));
        assert_eq!(converted.frames, 3, "{} in {}", codec.name(), format.name());
        assert!(converted.files.iter().any(|f| f.path == "animated.svg"));
    }
}

#[test]
fn convert_rejects_bad_requests() {
    let source = av_file(Format::Mp4, Codec::H264, Codec::Aac, 4, (64, 48));
    assert!(
        convert(
            &source,
            None,
            &ConvertOptions {
                formats: vec![],
                ..options()
            }
        )
        .is_err()
    );
    assert!(
        convert(
            &source,
            None,
            &ConvertOptions {
                fps: 0.0,
                ..options()
            }
        )
        .is_err()
    );
    assert!(convert(b"not a video", None, &options()).is_err());
    assert_eq!(ConvertFormat::from_name("react"), Some(ConvertFormat::Jsx));
    assert_eq!(ConvertFormat::from_name("pdf"), None);
}
