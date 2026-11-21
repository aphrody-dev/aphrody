mod common;
use aphrody_av::{
    Codec, Format, StreamChoice, TranscodeOptions, decode_audio_blocks, decode_video, probe,
    transcode,
};
use common::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// Run every entry point on `bytes`. Errors are fine; a panic is the failure.
fn exercise(bytes: &[u8]) {
    let _ = probe(bytes, None);
    if let Ok(frames) = decode_video(bytes, None) {
        for frame in frames.take(20) {
            if frame.is_err() {
                break;
            }
        }
    }
    if let Ok(blocks) = decode_audio_blocks(bytes, None) {
        let _ = blocks.take(50).count();
    }
    let _ = transcode(
        bytes,
        Format::Matroska,
        &TranscodeOptions {
            video: StreamChoice::Copy,
            audio: StreamChoice::Copy,
            ..Default::default()
        },
    );
    let _ = transcode(
        bytes,
        Format::Mp4,
        &TranscodeOptions {
            video: StreamChoice::Encode(Codec::H264),
            audio: StreamChoice::Disable,
            ..Default::default()
        },
    );
}

fn corpus() -> Vec<(String, Vec<u8>)> {
    let size = (32, 32);
    let mut files = vec![
        (
            "mp4/h264+aac".to_string(),
            av_file(Format::Mp4, Codec::H264, Codec::Aac, 4, size),
        ),
        (
            "mov/prores+pcm".to_string(),
            av_file(Format::Mov, Codec::ProRes, Codec::Pcm, 3, size),
        ),
        (
            "mkv/hevc+flac".to_string(),
            av_file(Format::Matroska, Codec::Hevc, Codec::Flac, 3, size),
        ),
        (
            "webm/vp9+opus".to_string(),
            av_file(Format::WebM, Codec::Vp9, Codec::Opus, 3, size),
        ),
        (
            "webm/av1+vorbis".to_string(),
            av_file(Format::WebM, Codec::Av1, Codec::Vorbis, 3, size),
        ),
        (
            "avi/mjpeg+pcm".to_string(),
            av_file(Format::Avi, Codec::Mjpeg, Codec::Pcm, 3, size),
        ),
        (
            "avi/mpeg4+ac3".to_string(),
            av_file(Format::Avi, Codec::Mpeg4, Codec::Ac3, 3, size),
        ),
        (
            "ts/h264+aac".to_string(),
            av_file(Format::MpegTs, Codec::H264, Codec::Aac, 4, size),
        ),
        (
            "ts/mpeg2+mp3".to_string(),
            av_file(Format::MpegTs, Codec::Mpeg2, Codec::Mp3, 3, size),
        ),
        (
            "mkv/ffv1+alac".to_string(),
            av_file(Format::Matroska, Codec::Ffv1, Codec::Alac, 3, size),
        ),
        (
            "mp4/dnxhd".to_string(),
            av_file(Format::Mp4, Codec::Dnxhd, Codec::Aac, 2, (64, 48)),
        ),
        (
            "mp4/apv".to_string(),
            av_file(Format::Mp4, Codec::Apv, Codec::Aac, 2, (64, 48)),
        ),
    ];
    let source = av_file(Format::Mp4, Codec::H264, Codec::Aac, 6, size);
    for (format, codec) in [
        (Format::Wav, Codec::Pcm),
        (Format::Flac, Codec::Flac),
        (Format::Mp3, Codec::Mp3),
        (Format::Ogg, Codec::Vorbis),
        (Format::Ac3, Codec::Ac3),
        (Format::Aiff, Codec::Pcm),
    ] {
        let out = transcode(
            &source,
            format,
            &TranscodeOptions {
                video: StreamChoice::Disable,
                audio: StreamChoice::Encode(codec),
                ..Default::default()
            },
        )
        .expect("audio file");
        files.push((format!("{}/{}", format.name(), codec.name()), out.bytes));
    }
    files
}

#[test]
fn truncated_flipped_and_garbage_inputs_error_without_panicking() {
    std::panic::set_hook(Box::new(|_| {}));
    let mut panics = Vec::new();
    let mut runs = 0usize;
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    for (name, bytes) in corpus() {
        let mut inputs: Vec<(String, Vec<u8>)> = Vec::new();
        for cut in [0usize, 1, 3, 4, 7, 8, 11, 12, 16, 31, 64] {
            inputs.push((
                format!("prefix {cut}"),
                bytes[..cut.min(bytes.len())].to_vec(),
            ));
        }
        for tenth in 1..10 {
            inputs.push((
                format!("truncated at {}0%", tenth),
                bytes[..bytes.len() * tenth / 10].to_vec(),
            ));
        }
        inputs.push(("one byte short".into(), bytes[..bytes.len() - 1].to_vec()));
        for n in 0..60 {
            let mut copy = bytes.clone();
            let flips = 1 + (xorshift(&mut state) % 4) as usize;
            for _ in 0..flips {
                let at = (xorshift(&mut state) as usize) % copy.len();
                copy[at] ^= 1 << (xorshift(&mut state) % 8);
            }
            inputs.push((format!("bitflip {n}"), copy));
        }
        for n in 0..10 {
            let mut copy = bytes.clone();
            let at = (xorshift(&mut state) as usize) % copy.len();
            let end = (at + 16 + (xorshift(&mut state) as usize) % 200).min(copy.len());
            for byte in &mut copy[at..end] {
                *byte = xorshift(&mut state) as u8;
            }
            inputs.push((format!("garbage span {n}"), copy));
        }
        for n in 0..6 {
            let mut copy: Vec<u8> = (0..bytes.len().min(4096))
                .map(|_| xorshift(&mut state) as u8)
                .collect();
            let keep = 16.min(copy.len());
            copy[..keep].copy_from_slice(&bytes[..keep]);
            inputs.push((format!("magic + noise {n}"), copy));
        }
        for (label, input) in inputs {
            runs += 1;
            if catch_unwind(AssertUnwindSafe(|| exercise(&input))).is_err() {
                panics.push(format!("{name}: {label} ({} bytes)", input.len()));
            }
        }
    }
    let _ = std::panic::take_hook();
    println!("{runs} hostile inputs");
    assert!(
        panics.is_empty(),
        "{} panics:\n{}",
        panics.len(),
        panics.join("\n")
    );
}

#[test]
fn pure_noise_is_rejected_not_decoded() {
    let mut state = 12345u64;
    for length in [1usize, 2, 7, 188, 4096, 100_000] {
        let noise: Vec<u8> = (0..length).map(|_| xorshift(&mut state) as u8).collect();
        assert!(
            probe(&noise, None).is_err(),
            "noise of {length} bytes probed as a container"
        );
    }
}
