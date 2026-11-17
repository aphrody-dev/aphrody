//! App-owned styled `ui` layer for `aphrody_installer` built cleanly above `nsh_base`
//! using semantic `theme()` tokens (zero raw color hex literals).

pub mod controls;

pub use controls::{
    action_button, choice_chip, section_card, status_banner, step_row, switch_row,
};
