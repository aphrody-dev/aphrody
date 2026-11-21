mod common;
use aphrody_av::audio::{decode_track, encode_packets};
use aphrody_av::caps::can_encode;
use aphrody_av::demux::demux;
use aphrody_av::mux::{OutTrack, mux, mux_supports};
use aphrody_av::{Codec, Packet, TrackKind, probe};
use common::*;

fn rms(channel: &[f32]) -> f64 {
    (channel.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>() / channel.len().max(1) as f64)
        .sqrt()
}

/// Every (audio codec, container) pair the table claims: encode, mux, probe, decode back with the
/// length and level preserved (exactly for the lossless codecs).
#[test]
fn audio_codec_by_container_matrix() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for codec in Codec::ALL
        .into_iter()
        .filter(|c| c.kind() == TrackKind::Audio && can_encode(*c))
    {
        for format in containers() {
            if !mux_supports(format, codec) {
                continue;
            }
            checked += 1;
            let label = format!("{} in {}", codec.name(), format.name());
            let input = sine(48_000, 2, 1.0);
            let encoded = match encode_packets(&input, codec, None) {
                Ok(encoded) => encoded,
                Err(error) => {
                    failures.push(format!("{label}: encode failed: {error}"));
                    continue;
                }
            };
            let track = OutTrack {
                kind: TrackKind::Audio,
                params: encoded.params.clone(),
                timescale: encoded.timescale,
                frame_rate: None,
                language: None,
                delay: encoded.delay,
            };
            let packets: Vec<Packet> = encoded
                .packets
                .iter()
                .map(|p| Packet {
                    track: 0,
                    data: p.data.clone(),
                    pts: p.pts,
                    dts: p.pts,
                    duration: p.duration,
                    key: true,
                })
                .collect();
            let file = match mux(format, &[track], &packets) {
                Ok(file) => file,
                Err(error) => {
                    failures.push(format!("{label}: mux failed: {error}"));
                    continue;
                }
            };
            match probe(&file, None) {
                Ok(info) => {
                    let audio = info.streams.iter().find(|s| s.kind == "audio");
                    match audio {
                        Some(a)
                            if a.codec == codec.name()
                                || (codec == Codec::Pcm && a.codec == "pcm_s16le") =>
                        {
                            if a.channels != Some(2)
                                || a.sample_rate != Some(encoded.params.sample_rate.unwrap_or(0))
                            {
                                failures.push(format!(
                                    "{label}: probe says {:?} ch {:?} Hz",
                                    a.channels, a.sample_rate
                                ));
                            }
                        }
                        other => failures.push(format!("{label}: probe stream {other:?}")),
                    }
                }
                Err(error) => {
                    failures.push(format!("{label}: probe failed: {error}"));
                    continue;
                }
            }
            match demux(&file, None).and_then(|(demuxed, _)| decode_track(&demuxed, &file, None, 0))
            {
                Ok(decoded) => {
                    let expected = (decoded.sample_rate as f64 * 1.0) as i64;
                    let got = decoded.frames() as i64;
                    let lossless = matches!(codec, Codec::Flac | Codec::Alac | Codec::Pcm);
                    let tolerance = if lossless { 0 } else { 3000 };
                    if (got - expected).abs() > tolerance {
                        failures.push(format!(
                            "{label}: decoded {got} frames, expected {expected} +-{tolerance}"
                        ));
                    }
                    let (a, b) = (rms(&input.channels[0]), rms(&decoded.channels[0]));
                    if b < a * 0.5 || b > a * 1.5 {
                        failures.push(format!("{label}: level {b:.3} vs input {a:.3}"));
                    }
                }
                Err(error) => failures.push(format!("{label}: decode failed: {error}")),
            }
        }
    }
    println!("checked {checked} audio cells");
    assert!(
        failures.is_empty(),
        "{} of {checked} cells failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
