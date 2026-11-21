use craft_media::{CodecId, CodecParams};
use serde::Serialize;

/// Container formats the crate names. Reading and writing support is per format in [`crate::caps`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Mp4,
    Mov,
    Matroska,
    WebM,
    Avi,
    MpegTs,
    MpegPs,
    Ogg,
    Mxf,
    Wav,
    Aiff,
    Flac,
    Mp3,
    Adts,
    Ac3,
    Caf,
}

impl Format {
    pub const ALL: [Format; 16] = [
        Format::Mp4,
        Format::Mov,
        Format::Matroska,
        Format::WebM,
        Format::Avi,
        Format::MpegTs,
        Format::MpegPs,
        Format::Ogg,
        Format::Mxf,
        Format::Wav,
        Format::Aiff,
        Format::Flac,
        Format::Mp3,
        Format::Adts,
        Format::Ac3,
        Format::Caf,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Format::Mp4 => "mp4",
            Format::Mov => "mov",
            Format::Matroska => "matroska",
            Format::WebM => "webm",
            Format::Avi => "avi",
            Format::MpegTs => "mpegts",
            Format::MpegPs => "mpegps",
            Format::Ogg => "ogg",
            Format::Mxf => "mxf",
            Format::Wav => "wav",
            Format::Aiff => "aiff",
            Format::Flac => "flac",
            Format::Mp3 => "mp3",
            Format::Adts => "adts",
            Format::Ac3 => "ac3",
            Format::Caf => "caf",
        }
    }

    pub fn long_name(self) -> &'static str {
        match self {
            Format::Mp4 => "MPEG-4 Part 14 (ISO base media)",
            Format::Mov => "QuickTime movie",
            Format::Matroska => "Matroska",
            Format::WebM => "WebM",
            Format::Avi => "Audio Video Interleave",
            Format::MpegTs => "MPEG transport stream",
            Format::MpegPs => "MPEG program stream",
            Format::Ogg => "Ogg",
            Format::Mxf => "Material eXchange Format",
            Format::Wav => "RIFF WAVE",
            Format::Aiff => "AIFF",
            Format::Flac => "FLAC native",
            Format::Mp3 => "MPEG audio stream",
            Format::Adts => "ADTS AAC stream",
            Format::Ac3 => "AC-3 / E-AC-3 stream",
            Format::Caf => "Core Audio Format",
        }
    }

    /// File extensions that select this format on output, first is canonical.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Format::Mp4 => &["mp4", "m4v", "m4a"],
            Format::Mov => &["mov"],
            Format::Matroska => &["mkv", "mka"],
            Format::WebM => &["webm"],
            Format::Avi => &["avi"],
            Format::MpegTs => &["ts", "m2ts", "mts"],
            Format::MpegPs => &["mpg", "mpeg", "vob"],
            Format::Ogg => &["ogg", "oga", "ogv", "opus"],
            Format::Mxf => &["mxf"],
            Format::Wav => &["wav"],
            Format::Aiff => &["aiff", "aif"],
            Format::Flac => &["flac"],
            Format::Mp3 => &["mp3"],
            Format::Adts => &["aac"],
            Format::Ac3 => &["ac3", "eac3"],
            Format::Caf => &["caf"],
        }
    }

    pub fn from_name(name: &str) -> Option<Format> {
        let lower = name.trim_start_matches('.').to_ascii_lowercase();
        Format::ALL
            .into_iter()
            .find(|f| f.name() == lower || f.extensions().contains(&lower.as_str()))
    }

    pub fn from_path(path: &str) -> Option<Format> {
        let (_, extension) = path.rsplit_once('.')?;
        Format::from_name(extension)
    }

    pub(crate) fn from_container(container: craft_media::Container) -> Format {
        use craft_media::Container as C;
        match container {
            C::Wav => Format::Wav,
            C::Aiff => Format::Aiff,
            C::Flac => Format::Flac,
            C::Mp4 => Format::Mp4,
            C::QuickTime => Format::Mov,
            C::Matroska => Format::Matroska,
            C::WebM => Format::WebM,
            C::Ogg => Format::Ogg,
            C::Caf => Format::Caf,
            C::Adts => Format::Adts,
            C::MpegAudio => Format::Mp3,
            C::Ac3 => Format::Ac3,
            C::Avi => Format::Avi,
            C::Mxf => Format::Mxf,
            C::MpegTs => Format::MpegTs,
            C::MpegPs => Format::MpegPs,
            _ => Format::Mp4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackKind {
    Video,
    Audio,
    Other,
}

/// Codec names as used on the command line (`-c:v h264`) and in the capability matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Codec {
    H264,
    Hevc,
    Av1,
    Vp8,
    Vp9,
    Ffv1,
    ProRes,
    Mjpeg,
    Mpeg2,
    Mpeg4,
    Dnxhd,
    Apv,
    Aac,
    Alac,
    Opus,
    Vorbis,
    Flac,
    Mp1,
    Mp2,
    Mp3,
    Ac3,
    Eac3,
    Pcm,
}

impl Codec {
    pub const ALL: [Codec; 23] = [
        Codec::H264,
        Codec::Hevc,
        Codec::Av1,
        Codec::Vp8,
        Codec::Vp9,
        Codec::Ffv1,
        Codec::ProRes,
        Codec::Mjpeg,
        Codec::Mpeg2,
        Codec::Mpeg4,
        Codec::Dnxhd,
        Codec::Apv,
        Codec::Aac,
        Codec::Alac,
        Codec::Opus,
        Codec::Vorbis,
        Codec::Flac,
        Codec::Mp1,
        Codec::Mp2,
        Codec::Mp3,
        Codec::Ac3,
        Codec::Eac3,
        Codec::Pcm,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Codec::H264 => "h264",
            Codec::Hevc => "hevc",
            Codec::Av1 => "av1",
            Codec::Vp8 => "vp8",
            Codec::Vp9 => "vp9",
            Codec::Ffv1 => "ffv1",
            Codec::ProRes => "prores",
            Codec::Mjpeg => "mjpeg",
            Codec::Mpeg2 => "mpeg2video",
            Codec::Mpeg4 => "mpeg4",
            Codec::Dnxhd => "dnxhd",
            Codec::Apv => "apv",
            Codec::Aac => "aac",
            Codec::Alac => "alac",
            Codec::Opus => "opus",
            Codec::Vorbis => "vorbis",
            Codec::Flac => "flac",
            Codec::Mp1 => "mp1",
            Codec::Mp2 => "mp2",
            Codec::Mp3 => "mp3",
            Codec::Ac3 => "ac3",
            Codec::Eac3 => "eac3",
            Codec::Pcm => "pcm_s16le",
        }
    }

    pub fn long_name(self) -> &'static str {
        match self {
            Codec::H264 => "H.264 / AVC",
            Codec::Hevc => "H.265 / HEVC",
            Codec::Av1 => "AV1",
            Codec::Vp8 => "VP8",
            Codec::Vp9 => "VP9",
            Codec::Ffv1 => "FFV1 lossless",
            Codec::ProRes => "Apple ProRes",
            Codec::Mjpeg => "Motion JPEG",
            Codec::Mpeg2 => "MPEG-2 video",
            Codec::Mpeg4 => "MPEG-4 Part 2",
            Codec::Dnxhd => "Avid DNxHD / DNxHR",
            Codec::Apv => "APV (RFC 9924)",
            Codec::Aac => "AAC-LC",
            Codec::Alac => "Apple Lossless",
            Codec::Opus => "Opus",
            Codec::Vorbis => "Vorbis",
            Codec::Flac => "FLAC",
            Codec::Mp1 => "MPEG audio layer I",
            Codec::Mp2 => "MPEG audio layer II",
            Codec::Mp3 => "MPEG audio layer III",
            Codec::Ac3 => "Dolby Digital AC-3",
            Codec::Eac3 => "Dolby Digital Plus E-AC-3",
            Codec::Pcm => "PCM signed 16-bit little-endian (decode: any PCM layout)",
        }
    }

    pub fn kind(self) -> TrackKind {
        if self.id().is_video() {
            TrackKind::Video
        } else {
            TrackKind::Audio
        }
    }

    pub fn id(self) -> CodecId {
        match self {
            Codec::H264 => CodecId::H264,
            Codec::Hevc => CodecId::Hevc,
            Codec::Av1 => CodecId::Av1,
            Codec::Vp8 => CodecId::Vp8,
            Codec::Vp9 => CodecId::Vp9,
            Codec::Ffv1 => CodecId::Ffv1,
            Codec::ProRes => CodecId::ProRes,
            Codec::Mjpeg => CodecId::Mjpeg,
            Codec::Mpeg2 => CodecId::Mpeg2Video,
            Codec::Mpeg4 => CodecId::Mpeg4Video,
            Codec::Dnxhd => CodecId::Dnxhd,
            Codec::Apv => CodecId::Apv,
            Codec::Aac => CodecId::Aac,
            Codec::Alac => CodecId::Alac,
            Codec::Opus => CodecId::Opus,
            Codec::Vorbis => CodecId::Vorbis,
            Codec::Flac => CodecId::Flac,
            Codec::Mp1 => CodecId::Mp1,
            Codec::Mp2 => CodecId::Mp2,
            Codec::Mp3 => CodecId::Mp3,
            Codec::Ac3 => CodecId::Ac3,
            Codec::Eac3 => CodecId::Eac3,
            Codec::Pcm => CodecId::Pcm,
        }
    }

    pub fn from_id(id: CodecId) -> Option<Codec> {
        let id = if id == CodecId::HeAac {
            CodecId::Aac
        } else {
            id
        };
        Codec::ALL.into_iter().find(|codec| codec.id() == id)
    }

    pub fn from_name(name: &str) -> Option<Codec> {
        let lower = name.to_ascii_lowercase();
        let canonical = match lower.as_str() {
            "avc" | "avc1" | "x264" | "libx264" | "h.264" => "h264",
            "h265" | "h.265" | "x265" | "libx265" => "hevc",
            "libaom-av1" | "libsvtav1" | "av01" => "av1",
            "libvpx" => "vp8",
            "libvpx-vp9" => "vp9",
            "mpeg2" | "mpeg1video" => "mpeg2video",
            "mpeg4part2" | "xvid" | "libxvid" => "mpeg4",
            "dnxhr" | "vc3" => "dnxhd",
            "libopus" => "opus",
            "libvorbis" => "vorbis",
            "libmp3lame" => "mp3",
            "pcm" | "pcm_s16le" | "pcm_s16" => "pcm_s16le",
            "ac-3" => "ac3",
            "e-ac-3" => "eac3",
            other => other,
        };
        Codec::ALL
            .into_iter()
            .find(|codec| codec.name() == canonical)
    }
}

/// One elementary stream of a demuxed input.
#[derive(Debug, Clone)]
pub struct Track {
    pub index: usize,
    pub kind: TrackKind,
    pub params: CodecParams,
    /// Ticks per second of every timestamp of this track.
    pub timescale: u32,
    /// Container-reported duration in ticks, when known.
    pub duration: Option<u64>,
    pub language: Option<String>,
    /// Frames per second (video), when the container states it or the packets imply it.
    pub frame_rate: Option<(u32, u32)>,
    /// A codec this build has no [`Codec`] for (named in the probe).
    pub codec_name: String,
}

impl Track {
    pub fn codec(&self) -> Option<Codec> {
        Codec::from_id(self.params.codec)
    }
}

/// One compressed access unit or audio packet.
#[derive(Debug, Clone)]
pub struct Packet {
    pub track: usize,
    pub data: Vec<u8>,
    /// Presentation and decoding time in track ticks.
    pub pts: i64,
    pub dts: i64,
    /// Duration in track ticks (0 when unknown).
    pub duration: i64,
    pub key: bool,
}

/// A fully read input: its tracks and every packet in decode order per track, interleaved by time.
#[derive(Debug, Clone)]
pub struct Demuxed {
    pub format: Format,
    pub tracks: Vec<Track>,
    pub packets: Vec<Packet>,
}

impl Demuxed {
    pub fn first_track(&self, kind: TrackKind) -> Option<&Track> {
        self.tracks.iter().find(|track| track.kind == kind)
    }

    pub fn track_packets(&self, track: usize) -> impl Iterator<Item = &Packet> {
        self.packets
            .iter()
            .filter(move |packet| packet.track == track)
    }

    /// Seconds from the first to the last timestamp plus last duration, over all tracks.
    pub fn duration_seconds(&self) -> Option<f64> {
        let mut longest: Option<f64> = None;
        for track in &self.tracks {
            let scale = f64::from(track.timescale.max(1));
            let mut end = 0i64;
            let mut start = i64::MAX;
            let mut any = false;
            for packet in self.track_packets(track.index) {
                any = true;
                start = start.min(packet.pts.min(packet.dts));
                end = end.max(packet.pts + packet.duration.max(0));
            }
            let from_packets = any.then(|| (end - start.min(end)) as f64 / scale);
            let from_container = track.duration.map(|ticks| ticks as f64 / scale);
            let value = from_container.or(from_packets);
            if let Some(value) = value {
                longest = Some(longest.map_or(value, |current| current.max(value)));
            }
        }
        longest
    }
}

/// Rescale `value` from `from` ticks per second to `to` ticks per second, rounding to nearest.
pub fn rescale(value: i64, from: u32, to: u32) -> i64 {
    if from == to || from == 0 {
        return value;
    }
    let numerator = i128::from(value) * i128::from(to);
    let denominator = i128::from(from);
    let half = denominator / 2;
    let rounded = if numerator >= 0 {
        (numerator + half) / denominator
    } else {
        (numerator - half) / denominator
    };
    rounded.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
}
