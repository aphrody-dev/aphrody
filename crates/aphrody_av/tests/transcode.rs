mod common;
use aphrody_av::{
    AvError, Codec, Format, Scale, StreamChoice, TranscodeOptions, probe, remux, transcode,
};
use common::*;

const SIZE: (u32, u32) = (64, 48);

fn encode_to(codec: Codec) -> TranscodeOptions {
    TranscodeOptions {
        video: StreamChoice::Encode(codec),
        audio: StreamChoice::Disable,
        ..Default::default()
    }
}

/// H.264 MP4 -> another codec in another container -> decode: frame count and dimensions survive.
#[test]
fn h264_mp4_roundtrips_through_other_codecs_and_containers() {
    let source = av_file(Format::Mp4, Codec::H264, Codec::Aac, 10, SIZE);
    let original = decode_all(&source);
    assert_eq!(original.len(), 10);
    let targets = [
        (Format::Matroska, Codec::Vp9),
        (Format::WebM, Codec::Av1),
        (Format::WebM, Codec::Vp8),
        (Format::Mov, Codec::ProRes),
        (Format::Avi, Codec::Mjpeg),
        (Format::Avi, Codec::Mpeg4),
        (Format::Matroska, Codec::Ffv1),
        (Format::Mp4, Codec::Hevc),
        (Format::MpegTs, Codec::Mpeg2),
        (Format::Mp4, Codec::Dnxhd),
    ];
    for (format, codec) in targets {
        let label = format!("h264/mp4 -> {}/{}", codec.name(), format.name());
        let out = transcode(&source, format, &encode_to(codec))
            .unwrap_or_else(|e| panic!("{label}: {e}"));
        let info = probe(&out.bytes, None).unwrap_or_else(|e| panic!("{label}: probe {e}"));
        assert_eq!(info.format, format.name(), "{label}");
        let video = info
            .streams
            .iter()
            .find(|s| s.kind == "video")
            .unwrap_or_else(|| panic!("{label}: no video stream"));
        assert_eq!(
            (video.codec.as_str(), video.width, video.height),
            (codec.name(), Some(SIZE.0), Some(SIZE.1)),
            "{label}"
        );
        if codec == Codec::Mpeg4 {
            continue;
        }
        let decoded = decode_all(&out.bytes);
        assert_eq!(decoded.len(), original.len(), "{label}: frame count");
        for (a, b) in original.iter().zip(&decoded) {
            assert_eq!(
                (a.picture.width, a.picture.height),
                (b.picture.width, b.picture.height),
                "{label}: dimensions"
            );
        }
        if codec == Codec::Ffv1 {
            for (a, b) in original.iter().zip(&decoded) {
                assert_eq!(a.picture.y, b.picture.y, "{label}: lossless luma");
                assert_eq!(a.picture.u, b.picture.u, "{label}: lossless chroma");
            }
        } else if matches!(
            codec,
            Codec::Vp9
                | Codec::Av1
                | Codec::Hevc
                | Codec::Mjpeg
                | Codec::ProRes
                | Codec::Dnxhd
                | Codec::Vp8
        ) {
            let mean = original
                .iter()
                .zip(&decoded)
                .map(|(a, b)| psnr(&a.picture.y, &b.picture.y))
                .sum::<f64>()
                / original.len() as f64;
            assert!(mean > 20.0, "{label}: mean luma PSNR {mean:.1} dB");
        }
    }
}

#[test]
fn audio_and_video_transcode_together() {
    let source = av_file(Format::Mp4, Codec::H264, Codec::Aac, 10, SIZE);
    let options = TranscodeOptions {
        video: StreamChoice::Encode(Codec::Vp9),
        audio: StreamChoice::Encode(Codec::Opus),
        ..Default::default()
    };
    let out = transcode(&source, Format::WebM, &options).expect("transcode");
    let info = probe(&out.bytes, None).expect("probe");
    assert_eq!(info.format, "webm");
    let kinds: Vec<(&str, &str)> = info
        .streams
        .iter()
        .map(|s| (s.kind, s.codec.as_str()))
        .collect();
    assert_eq!(kinds, vec![("video", "vp9"), ("audio", "opus")]);
    assert_eq!(info.streams[1].sample_rate, Some(48_000));
    assert_eq!(decode_all(&out.bytes).len(), 10);
    let audio: Vec<_> = aphrody_av::decode_audio_blocks(&out.bytes, None)
        .expect("audio")
        .collect();
    let frames: usize = audio.iter().map(|b| b.channels[0].len()).sum();
    assert!(
        (frames as i64 - 16_000).abs() < 3_000,
        "opus decoded {frames} frames for 0.33 s"
    );
}

#[test]
fn remux_chain_keeps_the_coded_pictures_bit_exact() {
    let source = av_file(Format::Mp4, Codec::H264, Codec::Aac, 10, SIZE);
    let reference = decode_all(&source);
    let mut current = source;
    for format in [
        Format::Matroska,
        Format::Avi,
        Format::MpegTs,
        Format::Mov,
        Format::Mp4,
    ] {
        let out = remux(&current, None, format)
            .unwrap_or_else(|e| panic!("remux to {}: {e}", format.name()));
        let decoded = decode_all(&out.bytes);
        assert_eq!(decoded.len(), reference.len(), "{}", format.name());
        for (a, b) in reference.iter().zip(&decoded) {
            assert_eq!(
                a.picture.y,
                b.picture.y,
                "luma changed in {}",
                format.name()
            );
            assert_eq!(
                a.picture.u,
                b.picture.u,
                "chroma changed in {}",
                format.name()
            );
        }
        let info = probe(&out.bytes, None).expect("probe");
        assert!(
            info.streams
                .iter()
                .any(|s| s.kind == "audio" && s.codec == "aac"),
            "audio stream lost in {}",
            format.name()
        );
        current = out.bytes;
    }
}

#[test]
fn scale_trim_and_frame_rate_filters() {
    let source = av_file(Format::Mp4, Codec::H264, Codec::Aac, 30, (64, 48));
    let scaled = transcode(
        &source,
        Format::Matroska,
        &TranscodeOptions {
            video: StreamChoice::Encode(Codec::H264),
            audio: StreamChoice::Disable,
            scale: Some(Scale {
                width: 32,
                height: -1,
            }),
            ..Default::default()
        },
    )
    .expect("scale");
    let frames = decode_all(&scaled.bytes);
    assert_eq!(frames.len(), 30);
    assert_eq!(
        (frames[0].picture.width, frames[0].picture.height),
        (32, 24)
    );

    let trimmed = transcode(
        &source,
        Format::Mp4,
        &TranscodeOptions {
            video: StreamChoice::Encode(Codec::H264),
            audio: StreamChoice::Encode(Codec::Aac),
            start_seconds: Some(0.2),
            duration_seconds: Some(0.3),
            ..Default::default()
        },
    )
    .expect("trim");
    let frames = decode_all(&trimmed.bytes);
    assert_eq!(frames.len(), 9, "0.3 s at 30 fps");
    let info = probe(&trimmed.bytes, None).expect("probe");
    let audio = info
        .streams
        .iter()
        .find(|s| s.kind == "audio")
        .expect("audio");
    assert!(
        (audio.duration_seconds.unwrap_or(0.0) - 0.3).abs() < 0.12,
        "audio {:?}",
        audio.duration_seconds
    );

    let resampled = transcode(
        &source,
        Format::Mp4,
        &TranscodeOptions {
            video: StreamChoice::Encode(Codec::H264),
            audio: StreamChoice::Disable,
            fps: Some((15, 1)),
            ..Default::default()
        },
    )
    .expect("fps");
    let info = probe(&resampled.bytes, None).expect("probe");
    assert_eq!(info.streams[0].frames, Some(15));
    assert_eq!(info.streams[0].frame_rate_ratio.as_deref(), Some("15/1"));
}

#[test]
fn audio_only_files_roundtrip() {
    let source = av_file(Format::Mp4, Codec::H264, Codec::Aac, 15, SIZE);
    for (format, codec, lossless) in [
        (Format::Wav, Codec::Pcm, true),
        (Format::Aiff, Codec::Pcm, true),
        (Format::Flac, Codec::Flac, true),
        (Format::Mp3, Codec::Mp3, false),
        (Format::Ogg, Codec::Vorbis, false),
        (Format::Ac3, Codec::Ac3, false),
        (Format::Ac3, Codec::Eac3, false),
    ] {
        let label = format!("aac/mp4 -> {}", format.name());
        let out = transcode(
            &source,
            format,
            &TranscodeOptions {
                audio: StreamChoice::Encode(codec),
                video: StreamChoice::Disable,
                ..Default::default()
            },
        )
        .unwrap_or_else(|e| panic!("{label}: {e}"));
        let info = probe(&out.bytes, None).unwrap_or_else(|e| panic!("{label}: probe {e}"));
        assert_eq!(info.format, format.name(), "{label}");
        assert_eq!(info.streams.len(), 1, "{label}");
        assert_eq!(info.streams[0].kind, "audio", "{label}");
        assert_eq!(info.streams[0].channels, Some(2), "{label}");
        let blocks: usize = aphrody_av::decode_audio_blocks(&out.bytes, None)
            .unwrap_or_else(|e| panic!("{label}: {e}"))
            .map(|b| b.channels[0].len())
            .sum();
        let seconds = blocks as f64 / f64::from(info.streams[0].sample_rate.unwrap_or(48_000));
        let tolerance = if lossless { 0.1 } else { 0.15 };
        assert!(
            (seconds - 0.5).abs() < tolerance,
            "{label}: {seconds:.3} s decoded"
        );
    }
}

#[test]
fn unsupported_requests_fail_with_a_precise_message() {
    let source = av_file(Format::Mp4, Codec::H264, Codec::Aac, 6, SIZE);
    let check = |result: Result<aphrody_av::Transcoded, AvError>, needle: &str| match result {
        Err(AvError::Unsupported(message)) => assert!(
            message.contains(needle),
            "message {message:?} lacks {needle:?}"
        ),
        other => panic!(
            "expected Unsupported containing {needle:?}, got {:?}",
            other.map(|t| t.report)
        ),
    };
    check(
        transcode(&source, Format::WebM, &encode_to(Codec::H264)),
        "webm cannot hold h264",
    );
    check(
        transcode(&source, Format::Mp4, &encode_to(Codec::Mpeg4)),
        "mp4 cannot hold mpeg4",
    );
    check(
        transcode(&source, Format::Mxf, &encode_to(Codec::H264)),
        "mxf",
    );
    check(
        transcode(
            &source,
            Format::Wav,
            &TranscodeOptions {
                video: StreamChoice::Encode(Codec::H264),
                ..Default::default()
            },
        ),
        "audio-only",
    );
    check(
        transcode(
            &source,
            Format::Mp4,
            &TranscodeOptions {
                video: StreamChoice::Copy,
                audio: StreamChoice::Copy,
                start_seconds: Some(1.0),
                ..Default::default()
            },
        ),
        "need a re-encode",
    );
    assert!(
        matches!(transcode(&source, Format::Mp4, &TranscodeOptions { video: StreamChoice::Encode(Codec::Aac), ..Default::default() }), Err(AvError::InvalidArgument(ref m)) if m.contains("not a video codec"))
    );
    let mpeg4 = av_file(Format::Avi, Codec::Mpeg4, Codec::Pcm, 6, SIZE);
    check(
        transcode(&mpeg4, Format::Mp4, &encode_to(Codec::H264)),
        "no decoder for video codec",
    );
    check(
        transcode(
            &source,
            Format::Mp4,
            &TranscodeOptions {
                audio: StreamChoice::Encode(Codec::Mp2),
                ..Default::default()
            },
        ),
        "mp4 cannot hold mp2",
    );
    assert!(matches!(
        transcode(
            &source,
            Format::Mp4,
            &TranscodeOptions {
                start_seconds: Some(-1.0),
                ..Default::default()
            }
        ),
        Err(AvError::InvalidArgument(_))
    ));
    assert!(matches!(
        transcode(&[], Format::Mp4, &Default::default()),
        Err(AvError::Malformed(_))
    ));
}

#[test]
fn stream_selection_errors() {
    let video_only = transcode(
        &av_file(Format::Mp4, Codec::H264, Codec::Aac, 6, SIZE),
        Format::Mp4,
        &TranscodeOptions {
            audio: StreamChoice::Disable,
            ..Default::default()
        },
    )
    .expect("video only");
    let err = transcode(
        &video_only.bytes,
        Format::Mp4,
        &TranscodeOptions {
            audio: StreamChoice::Encode(Codec::Aac),
            ..Default::default()
        },
    );
    assert!(
        matches!(err, Err(AvError::InvalidArgument(ref m)) if m.contains("no audio stream")),
        "{err:?}"
    );
}

#[test]
fn raw_adts_aac_is_probed_and_decoded() {
    let input = sine(44_100, 2, 0.5);
    let planes: Vec<&[f32]> = input.channels.iter().map(Vec::as_slice).collect();
    let mut encoder = craft_audio::aac::Encoder::new(
        craft_audio::aac::EncoderConfig::cbr(44_100, 2, 128_000).with_adts(true),
    )
    .expect("encoder");
    let mut stream: Vec<u8> = encoder.encode(&planes).concat();
    stream.extend(encoder.flush().concat());
    let info = probe(&stream, None).expect("probe adts");
    assert_eq!(info.format, "adts");
    assert_eq!(info.streams[0].codec, "aac");
    assert_eq!(info.streams[0].sample_rate, Some(44_100));
    let frames: usize = aphrody_av::decode_audio_blocks(&stream, None)
        .expect("decode adts")
        .map(|b| b.channels[0].len())
        .sum();
    assert!((frames as i64 - 22_050).abs() < 3_000, "{frames}");
}

#[test]
fn webm_remuxes_to_matroska_and_back() {
    let source = av_file(Format::WebM, Codec::Vp9, Codec::Opus, 8, SIZE);
    let reference = decode_all(&source);
    let mkv = remux(&source, None, Format::Matroska).expect("to mkv");
    let back = remux(&mkv.bytes, None, Format::WebM).expect("to webm");
    let decoded = decode_all(&back.bytes);
    assert_eq!(decoded.len(), reference.len());
    for (a, b) in reference.iter().zip(&decoded) {
        assert_eq!(a.picture.y, b.picture.y);
    }
    let info = probe(&back.bytes, None).expect("probe");
    assert_eq!(info.format, "webm");
    assert!(info.streams.iter().any(|s| s.codec == "opus"));
}
