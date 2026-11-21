//! Pure-Rust video decode for the player: MP4/MOV demux (`craft-media` ISO BMFF) and H.264
//! decode (`craft-video`), converted to BGRA frames GPUI can upload. `aphrody_media::player` is a
//! state machine only and decodes nothing, so this module is the decoder behind it. Audio is not
//! decoded.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError, sync_channel};

use anyhow::{Context as _, Result, bail};
use craft_media::FileReader;
use craft_media::isobmff::{self, CodecConfig, TrackKind};
use craft_video::h264::decode::{Decoder, Picture};
use gpui::RenderImage;
use image::{Frame, ImageBuffer, Rgba};

/// Frames kept ahead of the playback clock; the decode thread blocks when the queue is full.
const STREAM_QUEUE_DEPTH: usize = 8;
const MAX_TOLERATED_DECODE_ERRORS: usize = 16;

#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub pts_ms: u64,
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}

impl VideoFrame {
    #[must_use]
    pub fn into_render_image(self) -> Option<Arc<RenderImage>> {
        let buffer =
            ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(self.width, self.height, self.bgra)?;
        Some(Arc::new(RenderImage::new(vec![Frame::new(buffer)])))
    }
}

struct Coefficients {
    y_scale: i32,
    y_offset: i32,
    red_v: i32,
    green_u: i32,
    green_v: i32,
    blue_u: i32,
}

impl Coefficients {
    fn for_picture(picture: &Picture) -> Self {
        let bt709 = match picture.color.matrix {
            1 => true,
            5 | 6 => false,
            _ => picture.height >= 720,
        };
        match (picture.color.full_range, bt709) {
            (false, false) => Self {
                y_scale: 298,
                y_offset: 16,
                red_v: 409,
                green_u: 100,
                green_v: 208,
                blue_u: 516,
            },
            (false, true) => Self {
                y_scale: 298,
                y_offset: 16,
                red_v: 459,
                green_u: 55,
                green_v: 136,
                blue_u: 541,
            },
            (true, false) => Self {
                y_scale: 256,
                y_offset: 0,
                red_v: 359,
                green_u: 88,
                green_v: 183,
                blue_u: 454,
            },
            (true, true) => Self {
                y_scale: 256,
                y_offset: 0,
                red_v: 403,
                green_u: 48,
                green_v: 120,
                blue_u: 475,
            },
        }
    }
}

/// Converts a decoded 4:2:0 picture to BGRA, scaled (nearest) down to `max_width` when wider.
fn picture_to_frame(picture: &Picture, pts_ms: u64, max_width: u32) -> VideoFrame {
    let (source_width, source_height) = (picture.width.max(1), picture.height.max(1));
    let out_width = source_width.min(max_width.max(2)) & !1;
    let out_width = out_width.max(2);
    let out_height =
        ((u64::from(source_height) * u64::from(out_width) / u64::from(source_width)) as u32 & !1)
            .max(2);
    let coefficients = Coefficients::for_picture(picture);
    let mut bgra = vec![255_u8; out_width as usize * out_height as usize * 4];
    for out_y in 0..out_height as usize {
        let source_y = (out_y as u64 * u64::from(source_height) / u64::from(out_height)) as usize;
        let luma_row = source_y * picture.y_stride;
        let chroma_row = (source_y / 2) * picture.uv_stride;
        for out_x in 0..out_width as usize {
            let source_x = (out_x as u64 * u64::from(source_width) / u64::from(out_width)) as usize;
            let luma = picture.y.get(luma_row + source_x).copied().unwrap_or(0);
            let chroma_index = chroma_row + source_x / 2;
            let blue_chroma = picture.u.get(chroma_index).copied().unwrap_or(128);
            let red_chroma = picture.v.get(chroma_index).copied().unwrap_or(128);
            let scaled = (i32::from(luma) - coefficients.y_offset) * coefficients.y_scale;
            let u = i32::from(blue_chroma) - 128;
            let v = i32::from(red_chroma) - 128;
            let red = (scaled + coefficients.red_v * v + 128) >> 8;
            let green = (scaled - coefficients.green_u * u - coefficients.green_v * v + 128) >> 8;
            let blue = (scaled + coefficients.blue_u * u + 128) >> 8;
            let offset = (out_y * out_width as usize + out_x) * 4;
            bgra[offset] = blue.clamp(0, 255) as u8;
            bgra[offset + 1] = green.clamp(0, 255) as u8;
            bgra[offset + 2] = red.clamp(0, 255) as u8;
        }
    }
    VideoFrame {
        pts_ms,
        width: out_width,
        height: out_height,
        bgra,
    }
}

/// Decodes `path` from the sync sample at or before `start_ms`, handing each frame at or after
/// `start_ms` to `sink` in presentation order until the sink returns `false` or the stream ends.
pub fn decode_frames(
    path: &Path,
    start_ms: u64,
    max_width: u32,
    mut sink: impl FnMut(VideoFrame) -> bool,
) -> Result<()> {
    let file = isobmff::open(FileReader::open(path).context("open video")?).context("parse mp4")?;
    let source = FileReader::open(path).context("open video samples")?;
    let track_index = file
        .track_of_kind(TrackKind::Video)
        .context("no video track")?;
    let track = &file.tracks[track_index];
    let Some(CodecConfig::Avc(avc)) = track.codec() else {
        bail!("only H.264 video is decoded");
    };
    let mut decoder = Decoder::new();
    decoder
        .configure_avcc(&avc.to_bytes())
        .context("configure h264 decoder")?;
    let timescale = i64::from(track.timescale.max(1));
    let to_ms = |pts: i64| (pts.saturating_add(track.edit_offset).max(0) * 1000 / timescale) as u64;

    let start_pts = (start_ms as i64) * timescale / 1000 - track.edit_offset;
    let first = track
        .sample_at_pts(start_pts)
        .map_or(0, |index| track.sync_sample_before(index));

    let mut errors = 0;
    let mut keep_going = true;
    let mut emit = |pictures: Vec<Picture>, keep_going: &mut bool| {
        for picture in pictures {
            let pts_ms = to_ms(picture.pts);
            if pts_ms < start_ms {
                continue;
            }
            if !sink(picture_to_frame(&picture, pts_ms, max_width)) {
                *keep_going = false;
                return;
            }
        }
    };
    for index in first..track.samples.len() {
        let sample = track.samples[index];
        let data = file
            .read_sample(&source, track_index, index)
            .with_context(|| format!("read sample {index}"))?;
        match decoder.decode(&data, sample.pts) {
            Ok(pictures) => emit(pictures, &mut keep_going),
            Err(error) => {
                errors += 1;
                log::debug!("h264 decode error at sample {index}: {error}");
                if errors > MAX_TOLERATED_DECODE_ERRORS {
                    return Err(anyhow::Error::new(error).context("too many decode errors"));
                }
            }
        }
        if !keep_going {
            return Ok(());
        }
    }
    emit(decoder.flush(), &mut keep_going);
    Ok(())
}

/// A single frame near `at_ms`, used for library thumbnails.
pub fn decode_poster(path: &Path, at_ms: u64, max_width: u32) -> Result<VideoFrame> {
    let mut poster = None;
    decode_frames(path, at_ms, max_width, |frame| {
        poster = Some(frame);
        false
    })?;
    poster.context("no frame decoded")
}

/// A short run of frames for the muted hover preview.
pub fn decode_preview(
    path: &Path,
    start_ms: u64,
    max_frames: usize,
    max_width: u32,
) -> Result<Vec<VideoFrame>> {
    let mut frames = Vec::new();
    decode_frames(path, start_ms, max_width, |frame| {
        frames.push(frame);
        frames.len() < max_frames
    })?;
    Ok(frames)
}

pub enum StreamEvent {
    Frame(VideoFrame),
    Ended,
    Failed(String),
}

/// Frames of one playback run, decoded on a dedicated thread. Dropping the stream stops it.
pub struct FrameStream {
    receiver: Receiver<StreamEvent>,
    cancel: Arc<AtomicBool>,
    finished: bool,
}

impl FrameStream {
    #[must_use]
    pub fn spawn(path: PathBuf, start_ms: u64, max_width: u32) -> Self {
        let (sender, receiver) = sync_channel(STREAM_QUEUE_DEPTH);
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let spawned = std::thread::Builder::new()
            .name("aphrody-video-decode".to_owned())
            .spawn({
                let sender = sender.clone();
                move || {
                    let result = decode_frames(&path, start_ms, max_width, |frame| {
                        !worker_cancel.load(Ordering::Relaxed)
                            && sender.send(StreamEvent::Frame(frame)).is_ok()
                    });
                    let last = match result {
                        Ok(()) => StreamEvent::Ended,
                        Err(error) => StreamEvent::Failed(format!("{error:#}")),
                    };
                    sender.send(last).ok();
                }
            });
        if let Err(error) = spawned {
            sender
                .send(StreamEvent::Failed(format!("decode thread: {error}")))
                .ok();
        }
        Self {
            receiver,
            cancel,
            finished: false,
        }
    }

    /// Next ready event without blocking; `None` when nothing is ready yet.
    pub fn try_next(&mut self) -> Option<StreamEvent> {
        if self.finished {
            return None;
        }
        match self.receiver.try_recv() {
            Ok(event) => {
                if !matches!(event, StreamEvent::Frame(_)) {
                    self.finished = true;
                }
                Some(event)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.finished = true;
                Some(StreamEvent::Ended)
            }
        }
    }
}

impl Drop for FrameStream {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/library")
            .join(name)
    }

    #[test]
    fn decodes_all_frames_in_presentation_order() {
        let mut pts = Vec::new();
        decode_frames(
            &fixture("Harbour Films/coastline_walk.mp4"),
            0,
            1280,
            |frame| {
                assert_eq!((frame.width, frame.height), (320, 180));
                pts.push(frame.pts_ms);
                true
            },
        )
        .expect("decode");
        assert!((44..=46).contains(&pts.len()), "frames: {}", pts.len());
        assert!(
            pts.windows(2).all(|pair| pair[0] < pair[1]),
            "B-frames are reordered: {pts:?}"
        );
        assert_eq!(pts[0], 0);
    }

    #[test]
    fn poster_has_real_picture_content_and_downscales() {
        let frame =
            decode_poster(&fixture("Harbour Films/tide_tables.mp4"), 300, 160).expect("poster");
        assert_eq!((frame.width, frame.height), (160, 90));
        let distinct: std::collections::BTreeSet<[u8; 3]> = frame
            .bgra
            .chunks_exact(4)
            .map(|pixel| [pixel[0] / 32, pixel[1] / 32, pixel[2] / 32])
            .collect();
        assert!(distinct.len() >= 4, "colour bars decode to several colours");
        assert!(frame.into_render_image().is_some());
    }

    #[test]
    fn seeking_starts_at_or_after_the_target() {
        let mut first = None;
        decode_frames(
            &fixture("Harbour Films/coastline_walk.mp4"),
            1_500,
            1280,
            |frame| {
                first = Some(frame.pts_ms);
                false
            },
        )
        .expect("decode");
        let first = first.expect("frame");
        assert!((1_500..1_600).contains(&first), "first frame at {first}");
    }

    #[test]
    fn non_h264_containers_are_rejected_with_an_error() {
        assert!(decode_poster(&fixture("Harbour Films/archive_reel.mkv"), 0, 160).is_err());
        let preview = decode_preview(&fixture("test_pattern.mp4"), 0, 5, 160).expect("preview");
        assert_eq!(preview.len(), 5);
    }
}
