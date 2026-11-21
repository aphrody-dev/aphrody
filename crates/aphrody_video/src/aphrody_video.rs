//! Aphrody Video: a GPUI desktop video app over a local media library. It has a global shell
//! (top bar, search, responsive navigation), a home grid with chip filters, a watch view with
//! theater mode and miniplayer, search results, creator pages and a short-form viewer. Data comes
//! only from a directory of video files indexed through `aphrody_media`; playback is the
//! pure-Rust MP4/H.264 path in [`decode`] (audio is not decoded).

use std::path::PathBuf;

use gpui::{
    App, AppContext as _, Bounds, Focusable as _, KeyBinding, WindowBounds, WindowHandle,
    WindowOptions, px, size,
};

pub mod app;
pub mod decode;
pub mod layout;
pub mod library;
mod pages;
pub mod player;
mod ui;
mod watch;

pub use app::{
    CloseLayer, CreatorSort, CreatorTab, FocusSearch, GoBack, GoHome, HomeChip, NextShort,
    PreviousShort, Route, ShellLayout, StepFrameBack, StepFrameForward, ToggleAutoplay,
    ToggleCaptions, ToggleFullscreen, ToggleMenu, ToggleMiniplayer, ToggleMute, TogglePlay,
    ToggleTheater, ToggleTheme, VideoApp, VolumeDown, VolumeUp,
};
pub use library::{Library, SearchFilter, VideoItem};
pub use player::PlayerModel;

pub const APP_NAME: &str = "Aphrody Video";
pub const LIBRARY_ENV: &str = "APHRODY_VIDEO_LIBRARY";

/// Library folder: the environment override, else the user's videos folder.
#[must_use]
pub fn default_library_root() -> Option<PathBuf> {
    std::env::var_os(LIBRARY_ENV)
        .map(PathBuf::from)
        .or_else(dirs::video_dir)
}

/// Registers the keyboard table of the app. Letter keys are inactive while the search field is
/// focused.
pub fn init(cx: &mut App) {
    use app::{
        Jump0, Jump1, Jump2, Jump3, Jump4, Jump5, Jump6, Jump7, Jump8, Jump9, JumpToEnd,
        JumpToStart, SeekBack5, SeekBack10, SeekForward5, SeekForward10, SpeedDown, SpeedUp,
    };
    const PLAYER: Option<&str> = Some("(AphrodyWatch || AphrodyShorts) && !AphrodySearch");
    const WATCH: Option<&str> = Some("AphrodyWatch && !AphrodySearch");
    const SHORTS: Option<&str> = Some("AphrodyShorts && !AphrodySearch");
    const ANY: Option<&str> = Some("AphrodyVideo && !AphrodySearch");
    cx.bind_keys([
        KeyBinding::new("k", TogglePlay, PLAYER),
        KeyBinding::new("space", TogglePlay, PLAYER),
        KeyBinding::new("m", ToggleMute, PLAYER),
        KeyBinding::new("up", VolumeUp, WATCH),
        KeyBinding::new("down", VolumeDown, WATCH),
        KeyBinding::new("up", PreviousShort, SHORTS),
        KeyBinding::new("down", NextShort, SHORTS),
        KeyBinding::new("left", SeekBack5, PLAYER),
        KeyBinding::new("right", SeekForward5, PLAYER),
        KeyBinding::new("j", SeekBack10, PLAYER),
        KeyBinding::new("l", SeekForward10, PLAYER),
        KeyBinding::new("home", JumpToStart, PLAYER),
        KeyBinding::new("end", JumpToEnd, PLAYER),
        KeyBinding::new("0", Jump0, WATCH),
        KeyBinding::new("1", Jump1, WATCH),
        KeyBinding::new("2", Jump2, WATCH),
        KeyBinding::new("3", Jump3, WATCH),
        KeyBinding::new("4", Jump4, WATCH),
        KeyBinding::new("5", Jump5, WATCH),
        KeyBinding::new("6", Jump6, WATCH),
        KeyBinding::new("7", Jump7, WATCH),
        KeyBinding::new("8", Jump8, WATCH),
        KeyBinding::new("9", Jump9, WATCH),
        KeyBinding::new("shift-,", SpeedDown, WATCH),
        KeyBinding::new("shift-.", SpeedUp, WATCH),
        KeyBinding::new(",", StepFrameBack, WATCH),
        KeyBinding::new(".", StepFrameForward, WATCH),
        KeyBinding::new("t", ToggleTheater, WATCH),
        KeyBinding::new("i", ToggleMiniplayer, ANY),
        KeyBinding::new("f", ToggleFullscreen, PLAYER),
        KeyBinding::new("c", ToggleCaptions, PLAYER),
        KeyBinding::new("/", FocusSearch, ANY),
        KeyBinding::new("escape", CloseLayer, Some("AphrodyVideo")),
        KeyBinding::new("alt-left", GoBack, ANY),
        KeyBinding::new("ctrl-alt-t", ToggleTheme, ANY),
    ]);
}

/// Opens the app window; `root` is scanned in the background.
pub fn open_window(cx: &mut App, root: Option<PathBuf>) -> anyhow::Result<WindowHandle<VideoApp>> {
    let bounds = Bounds::centered(None, size(px(1440.0), px(900.0)), cx);
    let handle = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(gpui::TitlebarOptions {
                title: Some(APP_NAME.into()),
                ..Default::default()
            }),
            ..Default::default()
        },
        |window, cx| cx.new(|cx| VideoApp::new(window, cx)),
    )?;
    handle.update(cx, |app, window, cx| {
        window.focus(&app.focus_handle(cx), cx);
        if let Some(root) = root {
            app.open_library(root, cx);
        }
    })?;
    Ok(handle)
}
