//! Player behaviour of the watch view as a pure model on top of `aphrody_media::MediaPlayerState`:
//! the keyboard table, volume and seek rules, control auto-hide, transient centre icons,
//! autoplay-next countdown and theater/mini/fullscreen flags.

use aphrody_media::{MediaPlayerState, PlaybackSpeed, SubtitleCue};

use crate::layout::CONTROLS_IDLE_HIDE_MS;

pub const VOLUME_STEP: f32 = 0.05;
pub const SEEK_SHORT_MS: i64 = 5_000;
pub const SEEK_LONG_MS: i64 = 10_000;
pub const AUTOPLAY_COUNTDOWN_MS: u64 = 5_000;
pub const CENTER_ICON_MS: u64 = 500;
pub const SPEEDS: [f32; 8] = [0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CenterIcon {
    Play,
    Pause,
    VolumeUp,
    VolumeDown,
    Muted,
    Unmuted,
    SeekForward,
    SeekBack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerEvent {
    /// The autoplay countdown ran out: start the next recommended item.
    AutoplayNext,
}

#[derive(Debug, Clone)]
pub struct PlayerModel {
    pub state: MediaPlayerState,
    pub theater: bool,
    pub fullscreen: bool,
    pub miniplayer: bool,
    pub captions: bool,
    pub autoplay_next: bool,
    pub controls_visible: bool,
    pub buffering: bool,
    pub ended: bool,
    pub scrubbing: bool,
    pub center_icon: Option<(CenterIcon, u64)>,
    pub autoplay_countdown_ms: Option<u64>,
    /// Frame duration used by the `,` and `.` keys; refined from the stream when known.
    pub frame_ms: u64,
    idle_ms: u64,
}

impl Default for PlayerModel {
    fn default() -> Self {
        Self {
            state: MediaPlayerState::default(),
            theater: false,
            fullscreen: false,
            miniplayer: false,
            captions: false,
            autoplay_next: true,
            controls_visible: true,
            buffering: false,
            ended: false,
            scrubbing: false,
            center_icon: None,
            autoplay_countdown_ms: None,
            frame_ms: 33,
            idle_ms: 0,
        }
    }
}

impl PlayerModel {
    /// Resets playback for a new item; layout preferences (theater, captions, autoplay,
    /// volume, mute, speed) persist across items.
    pub fn load(&mut self, title: &str, source_uri: &str, duration_ms: u64) {
        let volume = self.state.volume;
        let muted = self.state.muted;
        let speed = self.state.speed;
        self.state = MediaPlayerState {
            title: title.to_owned(),
            source_uri: source_uri.to_owned(),
            duration_ms,
            volume,
            muted,
            speed,
            ..MediaPlayerState::default()
        };
        self.ended = false;
        self.buffering = false;
        self.scrubbing = false;
        self.center_icon = None;
        self.autoplay_countdown_ms = None;
        self.miniplayer = false;
        self.pointer_activity();
    }

    pub fn pointer_activity(&mut self) {
        self.idle_ms = 0;
        self.controls_visible = true;
    }

    fn flash(&mut self, icon: CenterIcon) {
        self.center_icon = Some((icon, CENTER_ICON_MS));
        self.pointer_activity();
    }

    pub fn toggle_play(&mut self) {
        self.autoplay_countdown_ms = None;
        if self.ended
            || (self.state.duration_ms > 0 && self.state.position_ms >= self.state.duration_ms)
        {
            self.state.position_ms = 0;
            self.ended = false;
            self.state.playing = true;
            self.flash(CenterIcon::Play);
            return;
        }
        self.state.playing = !self.state.playing;
        self.flash(if self.state.playing {
            CenterIcon::Play
        } else {
            CenterIcon::Pause
        });
    }

    pub fn toggle_mute(&mut self) {
        self.state.muted = !self.state.muted;
        self.flash(if self.state.muted {
            CenterIcon::Muted
        } else {
            CenterIcon::Unmuted
        });
    }

    pub fn adjust_volume(&mut self, delta: f32) {
        let volume = ((self.state.volume + delta) * 100.0).round() / 100.0;
        self.state.set_volume(volume);
        if delta > 0.0 {
            self.state.muted = false;
        }
        self.flash(if delta >= 0.0 {
            CenterIcon::VolumeUp
        } else {
            CenterIcon::VolumeDown
        });
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.state.set_volume(volume);
        self.state.muted = self.state.volume <= 0.0;
        self.pointer_activity();
    }

    pub fn seek_by_ms(&mut self, delta_ms: i64) {
        let target = if delta_ms < 0 {
            self.state
                .position_ms
                .saturating_sub(delta_ms.unsigned_abs())
        } else {
            self.state
                .position_ms
                .saturating_add(delta_ms.unsigned_abs())
        };
        self.seek_to_ms(target);
        self.flash(if delta_ms < 0 {
            CenterIcon::SeekBack
        } else {
            CenterIcon::SeekForward
        });
    }

    pub fn seek_to_ms(&mut self, target_ms: u64) {
        self.state.seek_ms(target_ms);
        self.ended = false;
        self.autoplay_countdown_ms = None;
        self.pointer_activity();
    }

    /// Digit keys: `0`-`9` jump to 0-90 percent of the duration.
    pub fn jump_to_tenth(&mut self, tenth: u64) {
        self.seek_to_ms(self.state.duration_ms * tenth.min(9) / 10);
    }

    pub fn jump_to_end(&mut self) {
        self.seek_to_ms(self.state.duration_ms);
        self.state.playing = false;
        self.ended = self.state.duration_ms > 0;
    }

    pub fn step_speed(&mut self, direction: i32) {
        let current = SPEEDS
            .iter()
            .position(|speed| (*speed - self.state.speed.0).abs() < 0.001)
            .unwrap_or(3);
        let next = (current as i32 + direction).clamp(0, SPEEDS.len() as i32 - 1) as usize;
        self.state.speed = PlaybackSpeed(SPEEDS[next]);
        self.pointer_activity();
    }

    /// `,` and `.` step one frame while paused.
    pub fn step_frame(&mut self, direction: i32) {
        if self.state.playing {
            return;
        }
        let frame = self.frame_ms.max(1);
        let target = if direction < 0 {
            self.state.position_ms.saturating_sub(frame)
        } else {
            self.state.position_ms.saturating_add(frame)
        };
        self.seek_to_ms(target);
    }

    pub fn toggle_theater(&mut self) {
        self.theater = !self.theater;
        self.pointer_activity();
    }

    pub fn toggle_fullscreen(&mut self) {
        self.fullscreen = !self.fullscreen;
        self.pointer_activity();
    }

    pub fn toggle_captions(&mut self) {
        self.captions = !self.captions;
        self.pointer_activity();
    }

    pub fn toggle_autoplay(&mut self) {
        self.autoplay_next = !self.autoplay_next;
        if !self.autoplay_next {
            self.autoplay_countdown_ms = None;
        }
        self.pointer_activity();
    }

    pub fn cancel_autoplay(&mut self) {
        self.autoplay_countdown_ms = None;
    }

    /// Advances the playback clock, fades overlays and runs the autoplay countdown.
    pub fn advance(&mut self, elapsed_ms: u64) -> Option<PlayerEvent> {
        let mut event = None;
        if let Some(remaining) = self.autoplay_countdown_ms {
            match remaining.checked_sub(elapsed_ms).filter(|left| *left > 0) {
                Some(left) => self.autoplay_countdown_ms = Some(left),
                None => {
                    self.autoplay_countdown_ms = None;
                    event = Some(PlayerEvent::AutoplayNext);
                }
            }
        }
        let was_playing = self.state.playing;
        self.state.step_ms(elapsed_ms);
        if was_playing && !self.state.playing && self.state.duration_ms > 0 {
            self.ended = true;
            if self.autoplay_next {
                self.autoplay_countdown_ms = Some(AUTOPLAY_COUNTDOWN_MS);
            }
        }
        if let Some((icon, remaining)) = self.center_icon {
            self.center_icon = remaining
                .checked_sub(elapsed_ms)
                .filter(|left| *left > 0)
                .map(|left| (icon, left));
        }
        if self.state.playing && !self.scrubbing {
            self.idle_ms += elapsed_ms;
            if self.idle_ms >= CONTROLS_IDLE_HIDE_MS {
                self.controls_visible = false;
            }
        } else {
            self.controls_visible = true;
        }
        event
    }

    #[must_use]
    pub fn progress(&self) -> f32 {
        if self.state.duration_ms == 0 {
            0.0
        } else {
            (self.state.position_ms as f32 / self.state.duration_ms as f32).clamp(0.0, 1.0)
        }
    }

    #[must_use]
    pub fn effective_volume(&self) -> f32 {
        if self.state.muted {
            0.0
        } else {
            self.state.volume
        }
    }
}

/// The caption cue shown at `position_ms`, if any.
#[must_use]
pub fn active_cue(cues: &[SubtitleCue], position_ms: u64) -> Option<&SubtitleCue> {
    cues.iter()
        .find(|cue| cue.start_ms <= position_ms && position_ms < cue.end_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> PlayerModel {
        let mut model = PlayerModel::default();
        model.load("clip", "file:clip.mp4", 60_000);
        model
    }

    #[test]
    fn volume_steps_clamp_and_unmute() {
        let mut player = model();
        player.adjust_volume(-VOLUME_STEP);
        assert!((player.state.volume - 0.80).abs() < 1e-4);
        player.toggle_mute();
        assert!(player.state.muted);
        player.adjust_volume(VOLUME_STEP);
        assert!(!player.state.muted);
        for _ in 0..40 {
            player.adjust_volume(VOLUME_STEP);
        }
        assert_eq!(player.state.volume, 1.0);
        for _ in 0..40 {
            player.adjust_volume(-VOLUME_STEP);
        }
        assert_eq!(player.state.volume, 0.0);
    }

    #[test]
    fn seek_keys_and_digits_clamp() {
        let mut player = model();
        player.seek_by_ms(SEEK_LONG_MS);
        assert_eq!(player.state.position_ms, 10_000);
        player.seek_by_ms(-SEEK_SHORT_MS);
        assert_eq!(player.state.position_ms, 5_000);
        player.seek_by_ms(-SEEK_LONG_MS);
        assert_eq!(player.state.position_ms, 0);
        player.jump_to_tenth(5);
        assert_eq!(player.state.position_ms, 30_000);
        player.jump_to_end();
        assert_eq!(player.state.position_ms, 60_000);
        assert!(player.ended);
    }

    #[test]
    fn speed_and_frame_step_follow_the_table() {
        let mut player = model();
        player.step_speed(1);
        assert_eq!(player.state.speed.0, 1.25);
        for _ in 0..20 {
            player.step_speed(-1);
        }
        assert_eq!(player.state.speed.0, 0.25);
        player.state.playing = true;
        player.step_frame(1);
        assert_eq!(player.state.position_ms, 0);
        player.state.playing = false;
        player.step_frame(1);
        assert_eq!(player.state.position_ms, 33);
    }

    #[test]
    fn controls_hide_after_idle_and_return_on_activity() {
        let mut player = model();
        player.toggle_play();
        assert!(player.state.playing);
        player.advance(2_900);
        assert!(player.controls_visible);
        player.advance(200);
        assert!(!player.controls_visible);
        player.pointer_activity();
        assert!(player.controls_visible);
        player.toggle_play();
        player.advance(10_000);
        assert!(player.controls_visible, "paused keeps controls");
    }

    #[test]
    fn autoplay_counts_down_after_the_end_and_can_be_disabled() {
        let mut player = model();
        player.toggle_play();
        player.seek_to_ms(59_000);
        assert_eq!(player.advance(2_000), None);
        assert!(player.ended);
        assert_eq!(player.autoplay_countdown_ms, Some(AUTOPLAY_COUNTDOWN_MS));
        assert_eq!(player.advance(4_000), None);
        assert_eq!(player.advance(1_500), Some(PlayerEvent::AutoplayNext));

        let mut player = model();
        player.toggle_autoplay();
        player.toggle_play();
        player.seek_to_ms(59_900);
        player.advance(500);
        assert!(player.ended);
        assert_eq!(player.autoplay_countdown_ms, None);
    }

    #[test]
    fn centre_icon_fades_and_replay_restarts_from_the_end() {
        let mut player = model();
        player.toggle_play();
        assert_eq!(
            player.center_icon.map(|(icon, _)| icon),
            Some(CenterIcon::Play)
        );
        player.advance(600);
        assert_eq!(player.center_icon, None);
        player.seek_to_ms(59_900);
        player.advance(500);
        player.cancel_autoplay();
        player.toggle_play();
        assert_eq!(player.state.position_ms, 0);
        assert!(player.state.playing && !player.ended);
    }

    #[test]
    fn load_keeps_user_preferences() {
        let mut player = model();
        player.toggle_theater();
        player.toggle_mute();
        player.step_speed(2);
        player.load("next", "file:next.mp4", 1_000);
        assert!(player.theater && player.state.muted);
        assert_eq!(player.state.speed.0, 1.5);
        assert_eq!(player.state.position_ms, 0);
    }

    #[test]
    fn active_cue_matches_the_position() {
        let cues = vec![SubtitleCue {
            start_ms: 1_000,
            end_ms: 2_000,
            text: "hello".into(),
            is_rtl: false,
        }];
        assert!(active_cue(&cues, 999).is_none());
        assert_eq!(
            active_cue(&cues, 1_500).map(|cue| cue.text.as_str()),
            Some("hello")
        );
        assert!(active_cue(&cues, 2_000).is_none());
    }
}
