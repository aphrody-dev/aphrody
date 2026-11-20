//! Deterministic frame-accurate programmatic video engine (HyperFrames & Remotion parity)
//! sharing Material 3 Expressive spring physics (`brand::design::sample_spring_position`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionKind {
    CrossFade,
    LinearWipe,
    Dissolve,
    ZoomBlur,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SequenceTrack {
    pub id: String,
    pub from_frame: u64,
    pub duration_frames: u64,
    pub transition_in: Option<TransitionKind>,
    pub layer_index: u32,
}

impl SequenceTrack {
    #[must_use]
    pub fn is_active(&self, frame: u64) -> bool {
        frame >= self.from_frame && frame < self.from_frame.saturating_add(self.duration_frames)
    }

    #[must_use]
    pub fn local_frame(&self, global_frame: u64) -> Option<u64> {
        if self.is_active(global_frame) {
            Some(global_frame - self.from_frame)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositionSpec {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub fps_num: u32,
    pub fps_den: u32,
    pub duration_frames: u64,
    pub sequences: Vec<SequenceTrack>,
}

impl CompositionSpec {
    #[must_use]
    pub fn fps(&self) -> f64 {
        f64::from(self.fps_num.max(1)) / f64::from(self.fps_den.max(1))
    }

    #[must_use]
    pub fn duration_seconds(&self) -> f64 {
        self.duration_frames as f64 / self.fps()
    }

    #[must_use]
    pub fn active_sequences(&self, frame: u64) -> Vec<&SequenceTrack> {
        let mut active: Vec<&SequenceTrack> = self
            .sequences
            .iter()
            .filter(|seq| seq.is_active(frame))
            .collect();
        active.sort_by_key(|seq| seq.layer_index);
        active
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameClock {
    pub current_frame: u64,
    pub fps_num: u32,
    pub fps_den: u32,
}

impl FrameClock {
    #[must_use]
    pub const fn new(fps_num: u32, fps_den: u32) -> Self {
        Self {
            current_frame: 0,
            fps_num,
            fps_den,
        }
    }

    #[must_use]
    pub fn time_seconds(&self) -> f64 {
        let fps = f64::from(self.fps_num.max(1)) / f64::from(self.fps_den.max(1));
        self.current_frame as f64 / fps
    }

    pub fn advance(&mut self) {
        self.current_frame = self.current_frame.saturating_add(1);
    }
}

/// Remotion-equivalent clamped linear interpolation between input and output ranges.
#[must_use]
pub fn interpolate_clamped(
    value: f32,
    input_start: f32,
    input_end: f32,
    output_start: f32,
    output_end: f32,
) -> f32 {
    if (input_end - input_start).abs() < f32::EPSILON {
        return output_start;
    }
    let normalized = ((value - input_start) / (input_end - input_start)).clamp(0.0, 1.0);
    output_start + normalized * (output_end - output_start)
}

/// Evaluates an analytical Material 3 Expressive spring at a specific frame index.
#[must_use]
pub fn spring_progress(frame: u64, fps: f32, stiffness: f32, damping_ratio: f32) -> f32 {
    let t_secs = frame as f32 / fps.max(1.0);
    brand::design::sample_spring_position(stiffness, damping_ratio, t_secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_frame_clock_and_spring_evaluation() {
        let comp = CompositionSpec {
            id: "intro".to_string(),
            width: 1920,
            height: 1080,
            fps_num: 60,
            fps_den: 1,
            duration_frames: 180,
            sequences: vec![
                SequenceTrack {
                    id: "bg".to_string(),
                    from_frame: 0,
                    duration_frames: 180,
                    transition_in: None,
                    layer_index: 0,
                },
                SequenceTrack {
                    id: "title".to_string(),
                    from_frame: 30,
                    duration_frames: 90,
                    transition_in: Some(TransitionKind::CrossFade),
                    layer_index: 1,
                },
            ],
        };
        assert_eq!(comp.active_sequences(15).len(), 1);
        assert_eq!(comp.active_sequences(45).len(), 2);
        assert!((interpolate_clamped(45.0, 30.0, 60.0, 0.0, 1.0) - 0.5).abs() < 1e-5);
        let p0 = spring_progress(0, 60.0, 1400.0, 0.9);
        let p30 = spring_progress(30, 60.0, 1400.0, 0.9);
        assert!(p30 > p0);
        assert!(p30 <= 1.1);
    }
}
