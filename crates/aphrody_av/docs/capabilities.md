# aphrody-av capability matrix

Generated from `aphrody_av::caps` (`aphrody-av -codecs --markdown`). Do not edit by hand: `tests/capabilities.rs` fails when this file drifts, and every container cell below is written and read back by `tests/matrix.rs` (video), `tests/audio_matrix.rs` (audio) and `tests/transcode.rs` (audio files, remux).

## Codecs

| codec | kind | decode | encode | containers (write and read back) |
|---|---|---|---|---|
| `h264` (H.264 / AVC) | video | yes | yes | mp4, mov, matroska, avi, mpegts |
| `hevc` (H.265 / HEVC) | video | yes | yes | mp4, mov, matroska, avi, mpegts |
| `av1` (AV1) | video | yes | yes | mp4, mov, matroska, webm, avi |
| `vp8` (VP8) | video | yes | yes | matroska, webm, avi |
| `vp9` (VP9) | video | yes | yes | mp4, mov, matroska, webm, avi |
| `ffv1` (FFV1 lossless) | video | yes | yes | matroska, avi |
| `prores` (Apple ProRes) | video | yes | yes | mp4, mov, matroska |
| `mjpeg` (Motion JPEG) | video | yes | yes | mp4, mov, matroska, avi |
| `mpeg2video` (MPEG-2 video) | video | yes | yes | matroska, avi, mpegts |
| `mpeg4` (MPEG-4 Part 2) | video | no | yes | matroska, avi |
| `dnxhd` (Avid DNxHD / DNxHR) | video | yes | yes | mp4, mov |
| `apv` (APV (RFC 9924)) | video | yes | yes | mp4, mov |
| `aac` (AAC-LC) | audio | yes | yes | mp4, mov, matroska, avi, mpegts |
| `alac` (Apple Lossless) | audio | yes | yes | mp4, mov, matroska |
| `opus` (Opus) | audio | yes | yes | mp4, mov, matroska, webm |
| `vorbis` (Vorbis) | audio | yes | yes | matroska, webm, ogg |
| `flac` (FLAC) | audio | yes | yes | mp4, mov, matroska, flac |
| `mp1` (MPEG audio layer I) | audio | linked | no |  |
| `mp2` (MPEG audio layer II) | audio | linked | no |  |
| `mp3` (MPEG audio layer III) | audio | yes | yes | matroska, mpegts, mp3 |
| `ac3` (Dolby Digital AC-3) | audio | yes | yes | mp4, mov, matroska, avi, mpegts, ac3 |
| `eac3` (Dolby Digital Plus E-AC-3) | audio | yes | yes | mp4, mov, matroska, mpegts, ac3 |
| `pcm_s16le` (PCM signed 16-bit little-endian (decode: any PCM layout)) | audio | yes | yes | mp4, mov, matroska, avi, wav, aiff |

`decode`: `yes` = decoded in a round-trip test; `linked` = decoder wired but no fixture can be produced here (no encoder), so unverified.

## Containers

| format | extensions | read | write | packet copy | note |
|---|---|---|---|---|---|
| `mp4` (MPEG-4 Part 14 (ISO base media)) | mp4, m4v, m4a | tested | yes | yes |  |
| `mov` (QuickTime movie) | mov | tested | yes | yes |  |
| `matroska` (Matroska) | mkv, mka | tested | yes | yes |  |
| `webm` (WebM) | webm | tested | yes | yes |  |
| `avi` (Audio Video Interleave) | avi | tested | yes | yes |  |
| `mpegts` (MPEG transport stream) | ts, m2ts, mts | tested | yes | yes |  |
| `mpegps` (MPEG program stream) | mpg, mpeg, vob | unverified | no | no | reader shares the transport stream scanner; no writer or fixture, unverified |
| `ogg` (Ogg) | ogg, oga, ogv, opus | tested | yes | no | write is Vorbis only, through the audio file encoder |
| `mxf` (Material eXchange Format) | mxf | unverified | no | no | reader wired for picture essence; no fixture, unverified |
| `wav` (RIFF WAVE) | wav | tested | yes | no | audio file: decode and re-encode, no packet copy |
| `aiff` (AIFF) | aiff, aif | tested | yes | no | audio file: decode and re-encode, no packet copy |
| `flac` (FLAC native) | flac | tested | yes | no | audio file: decode and re-encode, no packet copy |
| `mp3` (MPEG audio stream) | mp3 | tested | yes | no | audio file: decode and re-encode, no packet copy |
| `adts` (ADTS AAC stream) | aac | tested | no | no | read only (AAC encoded with ADTS headers in the test) |
| `ac3` (AC-3 / E-AC-3 stream) | ac3, eac3 | tested | yes | no | audio file: decode and re-encode, no packet copy |
| `caf` (Core Audio Format) | caf | unverified | no | no | audio decode through craft-audio; no writer or fixture, unverified |

## Defaults

| output | video | audio |
|---|---|---|
| `mp4` | h264 | aac |
| `mov` | h264 | aac |
| `matroska` | h264 | aac |
| `webm` | vp9 | opus |
| `avi` | mpeg4 | pcm_s16le |
| `mpegts` | h264 | aac |
| `wav` | - | pcm_s16le |
| `aiff` | - | pcm_s16le |
| `flac` | - | flac |
| `mp3` | - | mp3 |
| `ogg` | - | vorbis |
| `ac3` | - | ac3 |

## Limits

- Pictures pass between decoder and encoder as 8-bit limited-range 4:2:0 (`-pix_fmt yuv420p`); 10-bit and 4:4:4 sources lose precision, and the source colour matrix is not carried over (BT.709 is signalled).
- Encoders emit I and P pictures only (no B-frame reordering): H.264 `bframes = 0`, MPEG-2 `b_pictures = 0`. VP8 and the intra codecs (MJPEG, ProRes, DNxHR, APV, FFV1) code every picture as a key frame.
- Audio encoders take 1 or 2 channels; more channels are refused unless `-ac 2` is given.
- Inputs are read fully into memory (the API takes bytes).
- `-ss`/`-t`/`-vf scale`/`-r`/`-b:v` need a re-encode; combined with copy they fail with a message.
- MPEG-4 Part 2 has an encoder but no decoder in `craft-video`: such inputs are probed and copied, not decoded.
- Opus, AV1 and MPEG-4 Part 2 are written into MPEG-TS by craft-media but its reader cannot read them back, so they are not claimed there.
