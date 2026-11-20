//! Pure-Rust media player state machine shared by GPUI desktop (`MediaPlayer`, `YouTube`,
//! `YouTubeMusic`), headless agent commands, and WebAssembly targets.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackKind {
    Video,
    Audio,
    Subtitle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubtitleTrackInfo {
    pub id: String,
    pub language: String,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlaybackSpeed(pub f32);

impl Default for PlaybackSpeed {
    fn default() -> Self {
        Self(1.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopRange {
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaPlayerState {
    pub title: String,
    pub source_uri: String,
    pub playing: bool,
    pub muted: bool,
    pub volume: f32,
    pub speed: PlaybackSpeed,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub ab_loop: Option<LoopRange>,
    pub active_subtitle_track: Option<String>,
    pub subtitle_tracks: Vec<SubtitleTrackInfo>,
    pub sponsorblock_enabled: bool,
    pub gapless_next_uri: Option<String>,
}

impl Default for MediaPlayerState {
    fn default() -> Self {
        Self {
            title: String::new(),
            source_uri: String::new(),
            playing: false,
            muted: false,
            volume: 0.85,
            speed: PlaybackSpeed::default(),
            position_ms: 0,
            duration_ms: 0,
            ab_loop: None,
            active_subtitle_track: None,
            subtitle_tracks: Vec::new(),
            sponsorblock_enabled: true,
            gapless_next_uri: None,
        }
    }
}

impl MediaPlayerState {
    pub fn seek_ms(&mut self, target_ms: u64) {
        self.position_ms = if self.duration_ms > 0 {
            target_ms.min(self.duration_ms)
        } else {
            target_ms
        };
    }

    pub fn step_ms(&mut self, delta_ms: u64) {
        if !self.playing {
            return;
        }
        let scaled = (delta_ms as f64 * f64::from(self.speed.0.clamp(0.25, 4.0))) as u64;
        let next = self.position_ms.saturating_add(scaled);
        if let Some(loop_range) = self.ab_loop
            && loop_range.end_ms > loop_range.start_ms
            && next >= loop_range.end_ms
        {
            self.position_ms = loop_range.start_ms;
            return;
        }
        if self.duration_ms > 0 && next >= self.duration_ms {
            self.position_ms = self.duration_ms;
            self.playing = false;
        } else {
            self.position_ms = next;
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_state_handles_speed_and_ab_loop() {
        let mut state = MediaPlayerState {
            title: "Test Stream".to_string(),
            duration_ms: 60_000,
            playing: true,
            speed: PlaybackSpeed(2.0),
            ab_loop: Some(LoopRange {
                start_ms: 5_000,
                end_ms: 10_000,
            }),
            ..Default::default()
        };
        state.seek_ms(8_000);
        state.step_ms(1_500);
        assert_eq!(state.position_ms, 5_000);
    }
}
