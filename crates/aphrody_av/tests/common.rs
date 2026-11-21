#![allow(dead_code)]
use aphrody_av::{Codec, Format, Yuv420};

/// A moving diagonal gradient with a bright block that travels one step per frame, so frames differ.
pub fn clip(width: u32, height: u32, frames: usize) -> Vec<Yuv420> {
    (0..frames)
        .map(|n| {
            let mut picture = Yuv420::flat(width, height, 16, 128, 128);
            for row in 0..height as usize {
                for column in 0..width as usize {
                    picture.y[row * width as usize + column] =
                        (40 + ((row + column + n * 3) % 160)) as u8;
                }
            }
            let block = (n * 4) % (width as usize - 8).max(1);
            for row in 4..12.min(height as usize) {
                for column in block..block + 8 {
                    picture.y[row * width as usize + column] = 235;
                }
            }
            let cw = (width as usize).div_ceil(2);
            for (i, value) in picture.u.iter_mut().enumerate() {
                *value = (100 + ((i % cw) * 2 + n) % 60) as u8;
            }
            for (i, value) in picture.v.iter_mut().enumerate() {
                *value = (110 + ((i / cw) * 2 + n) % 40) as u8;
            }
            picture.pts = n as i64;
            picture
        })
        .collect()
}

pub fn sine(rate: u32, channels: usize, seconds: f32) -> craft_audio::AudioBuffer {
    let frames = (rate as f32 * seconds) as usize;
    let planar = (0..channels)
        .map(|c| {
            (0..frames)
                .map(|i| {
                    0.4 * (2.0 * std::f32::consts::PI * (440.0 + 110.0 * c as f32) * i as f32
                        / rate as f32)
                        .sin()
                })
                .collect()
        })
        .collect();
    craft_audio::AudioBuffer {
        sample_rate: rate,
        channels: planar,
    }
}

pub fn fps_for(codec: Codec) -> (u32, u32) {
    match codec {
        Codec::Mpeg2 => (25, 1),
        _ => (30, 1),
    }
}

pub fn video_codecs() -> Vec<Codec> {
    Codec::ALL
        .into_iter()
        .filter(|c| c.kind() == aphrody_av::TrackKind::Video)
        .collect()
}

pub fn containers() -> Vec<Format> {
    vec![
        Format::Mp4,
        Format::Mov,
        Format::Matroska,
        Format::WebM,
        Format::Avi,
        Format::MpegTs,
    ]
}

pub fn psnr(a: &[u8], b: &[u8]) -> f64 {
    let mse = a
        .iter()
        .zip(b)
        .map(|(x, y)| (f64::from(*x) - f64::from(*y)).powi(2))
        .sum::<f64>()
        / a.len().max(1) as f64;
    if mse == 0.0 {
        99.0
    } else {
        10.0 * (255.0f64 * 255.0 / mse).log10()
    }
}

/// A file with one video and one audio stream, built from the crate's own encoders and writers.
pub fn av_file(
    format: Format,
    video: Codec,
    audio: Codec,
    frames: usize,
    size: (u32, u32),
) -> Vec<u8> {
    use aphrody_av::audio::encode_packets;
    use aphrody_av::mux::{OutTrack, mux, rebase};
    use aphrody_av::video::{Framing, VideoEncodeConfig, VideoEncoder};
    use aphrody_av::{Packet, TrackKind};
    let fps = fps_for(video);
    let framing = if matches!(format, Format::Avi | Format::MpegTs) {
        Framing::AnnexB
    } else {
        Framing::LengthPrefixed
    };
    let mut encoder = VideoEncoder::new(&VideoEncodeConfig {
        codec: video,
        width: size.0,
        height: size.1,
        fps,
        bitrate_kbps: None,
        framing,
    })
    .expect("video encoder");
    let mut coded = Vec::new();
    for picture in clip(size.0, size.1, frames) {
        coded.extend(encoder.encode(&picture).expect("encode"));
    }
    coded.extend(encoder.flush().expect("flush"));
    let seconds = frames as f32 * fps.1 as f32 / fps.0 as f32;
    let encoded_audio =
        encode_packets(&sine(48_000, 2, seconds), audio, None).expect("audio encoder");
    let tracks = vec![
        OutTrack {
            kind: TrackKind::Video,
            params: encoder.params().clone(),
            timescale: fps.0,
            frame_rate: Some(fps),
            language: None,
            delay: 0,
        },
        OutTrack {
            kind: TrackKind::Audio,
            params: encoded_audio.params.clone(),
            timescale: encoded_audio.timescale,
            frame_rate: None,
            language: None,
            delay: encoded_audio.delay,
        },
    ];
    let mut packets: Vec<Packet> = coded
        .into_iter()
        .map(|p| Packet {
            track: 0,
            data: p.data,
            pts: p.pts * i64::from(fps.1),
            dts: p.dts * i64::from(fps.1),
            duration: i64::from(fps.1),
            key: p.key,
        })
        .collect();
    packets.extend(encoded_audio.packets.iter().map(|p| Packet {
        track: 1,
        data: p.data.clone(),
        pts: p.pts,
        dts: p.pts,
        duration: p.duration,
        key: true,
    }));
    let scales = [fps.0, encoded_audio.timescale];
    packets.sort_by_key(|p| {
        (
            i128::from(p.dts) * 1_000_000_000 / i128::from(scales[p.track]),
            p.track,
        )
    });
    rebase(&tracks, &mut packets);
    mux(format, &tracks, &packets).expect("mux")
}

pub fn decode_all(file: &[u8]) -> Vec<aphrody_av::DecodedFrame> {
    aphrody_av::decode_video(file, None)
        .expect("open")
        .collect::<Result<Vec<_>, _>>()
        .expect("decode")
}

pub fn xorshift(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}
