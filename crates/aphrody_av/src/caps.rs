//! The capability matrix. Everything here is the single source the CLI (`-codecs`, `-formats`),
//! the MCP server and `docs/capabilities.md` are generated from; the tests in `tests/` prove each
//! claim with real data.

use crate::model::{Codec, Format, TrackKind};
use crate::mux::mux_supports;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CodecCaps {
    pub name: &'static str,
    pub long_name: &'static str,
    pub kind: TrackKind,
    /// `yes` (round-tripped by a test), `linked` (decoder wired, no fixture to verify it) or `no`.
    pub decode: &'static str,
    pub encode: bool,
    /// Containers the codec is written into and read back from, proven by `tests/matrix.rs`,
    /// `tests/audio_matrix.rs` and `tests/transcode.rs`.
    pub containers: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FormatCaps {
    pub name: &'static str,
    pub long_name: &'static str,
    pub extensions: &'static [&'static str],
    /// `tested` when a test writes a file in this format and reads it back, otherwise `unverified`.
    pub read: &'static str,
    pub write: bool,
    /// Packets can be copied (remux) rather than only decoded to samples.
    pub packet_copy: bool,
    pub note: &'static str,
}

pub fn can_decode(codec: Codec) -> bool {
    decode_status(codec) != "no"
}

pub fn decode_status(codec: Codec) -> &'static str {
    match codec {
        Codec::Mpeg4 => "no",
        Codec::Mp1 | Codec::Mp2 => "linked",
        _ => "yes",
    }
}

pub fn can_encode(codec: Codec) -> bool {
    !matches!(codec, Codec::Mp1 | Codec::Mp2)
}

/// Audio-only file formats written through `craft-audio`'s file encoder.
pub fn writes_audio_file(format: Format, codec: Codec) -> bool {
    matches!(
        (format, codec),
        (Format::Wav | Format::Aiff, Codec::Pcm)
            | (Format::Flac, Codec::Flac)
            | (Format::Mp3, Codec::Mp3)
            | (Format::Ac3, Codec::Ac3 | Codec::Eac3)
            | (Format::Ogg, Codec::Vorbis)
    )
}

pub fn muxes(format: Format, codec: Codec) -> bool {
    mux_supports(format, codec) || writes_audio_file(format, codec)
}

pub fn codec_caps() -> Vec<CodecCaps> {
    Codec::ALL
        .into_iter()
        .map(|codec| CodecCaps {
            name: codec.name(),
            long_name: codec.long_name(),
            kind: codec.kind(),
            decode: decode_status(codec),
            encode: can_encode(codec),
            containers: Format::ALL
                .into_iter()
                .filter(|f| can_encode(codec) && muxes(*f, codec))
                .map(Format::name)
                .collect(),
        })
        .collect()
}

pub fn format_caps() -> Vec<FormatCaps> {
    Format::ALL
        .into_iter()
        .map(|format| {
            let packet_copy = matches!(
                format,
                Format::Mp4
                    | Format::Mov
                    | Format::Matroska
                    | Format::WebM
                    | Format::Avi
                    | Format::MpegTs
            );
            let write = Codec::ALL.into_iter().any(|c| muxes(format, c));
            let tested = matches!(
                format,
                Format::Mp4
                    | Format::Mov
                    | Format::Matroska
                    | Format::WebM
                    | Format::Avi
                    | Format::MpegTs
                    | Format::Wav
                    | Format::Aiff
                    | Format::Flac
                    | Format::Mp3
                    | Format::Ogg
                    | Format::Ac3
                    | Format::Adts
            );
            let note = match format {
                Format::Mxf => "reader wired for picture essence; no fixture, unverified",
                Format::MpegPs => {
                    "reader shares the transport stream scanner; no writer or fixture, unverified"
                }
                Format::Caf => "audio decode through craft-audio; no writer or fixture, unverified",
                Format::Ogg => "write is Vorbis only, through the audio file encoder",
                Format::Adts => "read only (AAC encoded with ADTS headers in the test)",
                Format::Mp3 | Format::Ac3 | Format::Wav | Format::Aiff | Format::Flac => {
                    "audio file: decode and re-encode, no packet copy"
                }
                _ => "",
            };
            FormatCaps {
                name: format.name(),
                long_name: format.long_name(),
                extensions: format.extensions(),
                read: if tested { "tested" } else { "unverified" },
                write,
                packet_copy,
                note,
            }
        })
        .collect()
}

fn yes(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

/// `docs/capabilities.md`, generated.
pub fn markdown() -> String {
    let mut out = String::new();
    out.push_str("# aphrody-av capability matrix\n\n");
    out.push_str("Generated from `aphrody_av::caps` (`aphrody-av -codecs --markdown`). Do not edit by hand: `tests/capabilities.rs` fails when this file drifts, and every container cell below is written and read back by `tests/matrix.rs` (video), `tests/audio_matrix.rs` (audio) and `tests/transcode.rs` (audio files, remux).\n\n");
    out.push_str("## Codecs\n\n| codec | kind | decode | encode | containers (write and read back) |\n|---|---|---|---|---|\n");
    for caps in codec_caps() {
        let kind = match caps.kind {
            TrackKind::Video => "video",
            _ => "audio",
        };
        out.push_str(&format!(
            "| `{}` ({}) | {} | {} | {} | {} |\n",
            caps.name,
            caps.long_name,
            kind,
            caps.decode,
            yes(caps.encode),
            caps.containers.join(", ")
        ));
    }
    out.push_str("\n`decode`: `yes` = decoded in a round-trip test; `linked` = decoder wired but no fixture can be produced here (no encoder), so unverified.\n");
    out.push_str("\n## Containers\n\n| format | extensions | read | write | packet copy | note |\n|---|---|---|---|---|---|\n");
    for caps in format_caps() {
        out.push_str(&format!(
            "| `{}` ({}) | {} | {} | {} | {} | {} |\n",
            caps.name,
            caps.long_name,
            caps.extensions.join(", "),
            caps.read,
            yes(caps.write),
            yes(caps.packet_copy),
            caps.note
        ));
    }
    out.push_str("\n## Defaults\n\n| output | video | audio |\n|---|---|---|\n");
    for format in [
        Format::Mp4,
        Format::Mov,
        Format::Matroska,
        Format::WebM,
        Format::Avi,
        Format::MpegTs,
        Format::Wav,
        Format::Aiff,
        Format::Flac,
        Format::Mp3,
        Format::Ogg,
        Format::Ac3,
    ] {
        let video = crate::transcode::default_video_codec(format).map_or("-", |c| c.name());
        let audio = crate::transcode::default_audio_codec(format).map_or("-", |c| c.name());
        out.push_str(&format!(
            "| `{}` | {} | {} |\n",
            format.name(),
            video,
            audio
        ));
    }
    out.push_str("\n## Limits\n\n");
    out.push_str("- Pictures pass between decoder and encoder as 8-bit limited-range 4:2:0 (`-pix_fmt yuv420p`); 10-bit and 4:4:4 sources lose precision, and the source colour matrix is not carried over (BT.709 is signalled).\n");
    out.push_str("- Encoders emit I and P pictures only (no B-frame reordering): H.264 `bframes = 0`, MPEG-2 `b_pictures = 0`. VP8 and the intra codecs (MJPEG, ProRes, DNxHR, APV, FFV1) code every picture as a key frame.\n");
    out.push_str("- Audio encoders take 1 or 2 channels; more channels are refused unless `-ac 2` is given.\n");
    out.push_str("- Inputs are read fully into memory (the API takes bytes).\n");
    out.push_str("- `-ss`/`-t`/`-vf scale`/`-r`/`-b:v` need a re-encode; combined with copy they fail with a message.\n");
    out.push_str("- MPEG-4 Part 2 has an encoder but no decoder in `craft-video`: such inputs are probed and copied, not decoded.\n");
    out.push_str("- Opus, AV1 and MPEG-4 Part 2 are written into MPEG-TS by craft-media but its reader cannot read them back, so they are not claimed there.\n");
    out
}
