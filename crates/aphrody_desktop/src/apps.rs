//! Modular COSMIC Epoch & Pop!_OS application surfaces hosted inside `aphrody_desktop`.
//!
//! Each module maps 1:1 to a `pop-os/cosmic-epoch` desktop component without `mod.rs` boilerplate.

use aphrody_ui::OsTheme;
use gpui::{AnyElement, FontWeight, prelude::*};
use ui::{Label, LabelCommon as _, LabelSize, v_flex};

pub mod cosmic_settings;
pub mod edit;
pub mod installer;
pub mod store;
pub mod term;

pub use crate::studio_view::{StudioCallbacks, StudioState, render_studio_window};
pub use crate::windows::{
    DesktopPrefs, FileRow, FilesState, SettingsFlags, SettingsMessage, SettingsProgram,
    SettingsState, WindowKind, render_files_window, render_monitor_window,
};

#[must_use]
pub(crate) fn section_heading(theme: &OsTheme, title: impl Into<ui::SharedString>) -> AnyElement {
    let title = title.into();
    v_flex()
        .pb_1()
        .border_b_1()
        .border_color(theme.colors.outline_variant)
        .child(
            Label::new(title)
                .size(LabelSize::Small)
                .weight(FontWeight::SEMIBOLD)
                .color(ui::Color::Default),
        )
        .into_any_element()
}
