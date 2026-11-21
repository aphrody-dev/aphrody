//! wasm-bindgen surface (wasm32-unknown-unknown): probe, decode and convert over in-memory bytes.

use crate::convert::convert;
use crate::decode::{VideoFrames, decode_audio_blocks};
use crate::service::ConvertSpec;
use wasm_bindgen::prelude::*;

fn js_error(error: impl std::fmt::Display) -> JsError {
    JsError::new(&error.to_string())
}

/// Version of the crate.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// Container, streams, codecs and duration as a JSON string.
#[wasm_bindgen]
pub fn probe(bytes: &[u8], hint: Option<String>) -> Result<String, JsError> {
    let info = crate::probe::probe(bytes, hint.as_deref()).map_err(js_error)?;
    serde_json::to_string(&info).map_err(js_error)
}

/// The capability matrix as a JSON string.
#[wasm_bindgen]
pub fn capabilities() -> String {
    serde_json::json!({ "codecs": crate::caps::codec_caps(), "formats": crate::caps::format_caps() }).to_string()
}

/// One decoded picture.
#[wasm_bindgen]
pub struct Frame {
    width: u32,
    height: u32,
    pts_seconds: f64,
    rgba: Vec<u8>,
}

#[wasm_bindgen]
impl Frame {
    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 {
        self.width
    }
    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 {
        self.height
    }
    #[wasm_bindgen(getter)]
    pub fn pts_seconds(&self) -> f64 {
        self.pts_seconds
    }
    /// Straight RGBA, `width * height * 4` bytes.
    #[wasm_bindgen(getter)]
    pub fn rgba(&self) -> Vec<u8> {
        self.rgba.clone()
    }
}

/// Decodes the first video stream picture by picture.
#[wasm_bindgen]
pub struct FrameDecoder {
    frames: VideoFrames,
}

#[wasm_bindgen]
impl FrameDecoder {
    #[wasm_bindgen(constructor)]
    pub fn new(bytes: &[u8], hint: Option<String>) -> Result<FrameDecoder, JsError> {
        Ok(FrameDecoder {
            frames: VideoFrames::new(bytes, hint.as_deref()).map_err(js_error)?,
        })
    }

    /// The next picture, or `undefined` at the end of the stream.
    pub fn next_frame(&mut self) -> Result<Option<Frame>, JsError> {
        match self.frames.next() {
            None => Ok(None),
            Some(Err(error)) => Err(js_error(error)),
            Some(Ok(frame)) => {
                let rgb = frame.picture.to_rgb();
                let rgba = rgb
                    .chunks_exact(3)
                    .flat_map(|p| [p[0], p[1], p[2], 255])
                    .collect();
                Ok(Some(Frame {
                    width: frame.picture.width,
                    height: frame.picture.height,
                    pts_seconds: frame.pts_seconds,
                    rgba,
                }))
            }
        }
    }
}

/// Decoded audio of the first audio stream: interleaved `f32` samples.
#[wasm_bindgen]
pub struct AudioClip {
    sample_rate: u32,
    channels: u32,
    samples: Vec<f32>,
}

#[wasm_bindgen]
impl AudioClip {
    #[wasm_bindgen(getter)]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    #[wasm_bindgen(getter)]
    pub fn channels(&self) -> u32 {
        self.channels
    }
    #[wasm_bindgen(getter)]
    pub fn samples(&self) -> Vec<f32> {
        self.samples.clone()
    }
}

#[wasm_bindgen]
pub fn decode_audio(bytes: &[u8], hint: Option<String>) -> Result<AudioClip, JsError> {
    let blocks = decode_audio_blocks(bytes, hint.as_deref()).map_err(js_error)?;
    let sample_rate = blocks.sample_rate();
    let mut samples = Vec::new();
    let mut channels = 0u32;
    for block in blocks {
        channels = block.channels.len() as u32;
        for frame in 0..block.channels.first().map_or(0, Vec::len) {
            for channel in &block.channels {
                samples.push(channel[frame]);
            }
        }
    }
    Ok(AudioClip {
        sample_rate,
        channels,
        samples,
    })
}

/// Documents produced by `convert`.
#[wasm_bindgen]
pub struct Converted {
    frames: u32,
    decoded: u32,
    paths: Vec<String>,
    files: Vec<Vec<u8>>,
}

#[wasm_bindgen]
impl Converted {
    #[wasm_bindgen(getter)]
    pub fn frames(&self) -> u32 {
        self.frames
    }
    #[wasm_bindgen(getter)]
    pub fn decoded(&self) -> u32 {
        self.decoded
    }
    #[wasm_bindgen(getter)]
    pub fn file_count(&self) -> u32 {
        self.files.len() as u32
    }
    pub fn file_path(&self, index: u32) -> Option<String> {
        self.paths.get(index as usize).cloned()
    }
    pub fn file_bytes(&self, index: u32) -> Option<Vec<u8>> {
        self.files.get(index as usize).cloned()
    }
}

/// Vectorise a video; `options_json` is a `ConvertSpec` object (`{"formats":["svg","md"],"fps":8,"width":160}`).
#[wasm_bindgen]
pub fn convert_video(
    bytes: &[u8],
    hint: Option<String>,
    options_json: Option<String>,
) -> Result<Converted, JsError> {
    let spec: ConvertSpec = match options_json.as_deref() {
        None | Some("") => ConvertSpec::default(),
        Some(json) => serde_json::from_str(json).map_err(js_error)?,
    };
    let options = spec
        .resolve(hint.as_deref().unwrap_or("video"))
        .map_err(js_error)?;
    let converted = convert(bytes, hint.as_deref(), &options).map_err(js_error)?;
    let mut result = Converted {
        frames: converted.frames as u32,
        decoded: converted.decoded as u32,
        paths: Vec::new(),
        files: Vec::new(),
    };
    for file in converted.files {
        result.paths.push(file.path);
        result.files.push(file.bytes);
    }
    Ok(result)
}
