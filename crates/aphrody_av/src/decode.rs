use crate::audio;
use crate::demux;
use crate::error::{AvError, AvResult};
use crate::model::{Demuxed, TrackKind};
use crate::video::{VideoDecoder, Yuv420};
use std::collections::VecDeque;

/// One decoded picture with its presentation time.
#[derive(Debug, Clone)]
pub struct DecodedFrame {
    pub index: u64,
    pub pts_seconds: f64,
    pub picture: Yuv420,
}

/// Iterator over the pictures of the first video stream, in display order. Decoding is lazy: a
/// packet is decoded only when the previous pictures have been consumed. The first error ends the
/// iteration.
pub struct VideoFrames {
    demuxed: Demuxed,
    track: usize,
    timescale: f64,
    decoder: VideoDecoder,
    cursor: usize,
    packets: Vec<usize>,
    queue: VecDeque<Yuv420>,
    flushed: bool,
    failed: bool,
    index: u64,
}

impl VideoFrames {
    pub fn new(bytes: &[u8], hint: Option<&str>) -> AvResult<VideoFrames> {
        let (demuxed, _) = demux::demux(bytes, hint)?;
        let track = demuxed
            .first_track(TrackKind::Video)
            .ok_or_else(|| AvError::invalid("the input has no video stream"))?;
        let decoder = VideoDecoder::new(track)?;
        let (index, timescale) = (track.index, f64::from(track.timescale.max(1)));
        let packets = demuxed
            .packets
            .iter()
            .enumerate()
            .filter(|(_, p)| p.track == index)
            .map(|(i, _)| i)
            .collect();
        Ok(VideoFrames {
            demuxed,
            track: index,
            timescale,
            decoder,
            cursor: 0,
            packets,
            queue: VecDeque::new(),
            flushed: false,
            failed: false,
            index: 0,
        })
    }

    pub fn width_height(&self) -> Option<(u32, u32)> {
        let track = self.demuxed.tracks.get(self.track)?;
        Some((track.params.width?, track.params.height?))
    }
}

impl Iterator for VideoFrames {
    type Item = AvResult<DecodedFrame>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.failed {
            return None;
        }
        loop {
            if let Some(picture) = self.queue.pop_front() {
                let frame = DecodedFrame {
                    index: self.index,
                    pts_seconds: picture.pts as f64 / self.timescale,
                    picture,
                };
                self.index += 1;
                return Some(Ok(frame));
            }
            if self.flushed {
                return None;
            }
            let result = match self.packets.get(self.cursor) {
                Some(&at) => {
                    self.cursor += 1;
                    let packet = &self.demuxed.packets[at];
                    self.decoder.decode(&packet.data, packet.pts)
                }
                None => {
                    self.flushed = true;
                    self.decoder.flush()
                }
            };
            match result {
                Ok(frames) => self.queue.extend(frames),
                Err(error) => {
                    self.failed = true;
                    return Some(Err(error));
                }
            }
        }
    }
}

pub fn decode_video(bytes: &[u8], hint: Option<&str>) -> AvResult<VideoFrames> {
    VideoFrames::new(bytes, hint)
}

/// A run of decoded audio, planar `f32`.
#[derive(Debug, Clone)]
pub struct AudioBlock {
    pub sample_rate: u32,
    /// Index of the first sample frame of the block in the stream.
    pub start_frame: u64,
    pub channels: Vec<Vec<f32>>,
}

pub struct AudioBlocks {
    buffer: craft_audio::AudioBuffer,
    position: usize,
    block: usize,
}

impl AudioBlocks {
    pub fn new(bytes: &[u8], hint: Option<&str>, block_frames: usize) -> AvResult<AudioBlocks> {
        let (demuxed, _) = demux::demux(bytes, hint)?;
        let ordinal = demuxed
            .tracks
            .iter()
            .filter(|t| t.kind == TrackKind::Audio)
            .position(|_| true)
            .ok_or_else(|| AvError::invalid("the input has no audio stream"))?;
        let buffer = audio::decode_track(&demuxed, bytes, hint, ordinal)?;
        Ok(AudioBlocks {
            buffer,
            position: 0,
            block: block_frames.max(1),
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.buffer.sample_rate
    }
    pub fn total_frames(&self) -> usize {
        self.buffer.frames()
    }
}

impl Iterator for AudioBlocks {
    type Item = AudioBlock;
    fn next(&mut self) -> Option<AudioBlock> {
        let total = self.buffer.frames();
        if self.position >= total {
            return None;
        }
        let end = (self.position + self.block).min(total);
        let channels = self
            .buffer
            .channels
            .iter()
            .map(|c| c[self.position..end].to_vec())
            .collect();
        let block = AudioBlock {
            sample_rate: self.buffer.sample_rate,
            start_frame: self.position as u64,
            channels,
        };
        self.position = end;
        Some(block)
    }
}

pub fn decode_audio_blocks(bytes: &[u8], hint: Option<&str>) -> AvResult<AudioBlocks> {
    AudioBlocks::new(bytes, hint, 4096)
}
