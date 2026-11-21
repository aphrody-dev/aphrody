use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

use aphrody_video::layout::{self, NavMode};
use aphrody_video::library::{Library, SearchFilter};
use aphrody_video::{Route, VideoApp};
use gpui::{Entity, TestAppContext, VisualContext as _, VisualTestContext, px, size};

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/library")
}

fn id_of(library: &Library, file_stem: &str) -> usize {
    library
        .items()
        .iter()
        .find(|item| item.path.file_stem().and_then(|stem| stem.to_str()) == Some(file_stem))
        .unwrap_or_else(|| panic!("fixture {file_stem} missing"))
        .id
}

fn open_app(
    cx: &mut TestAppContext,
    library: Library,
) -> (Entity<VideoApp>, &mut VisualTestContext) {
    cx.update(aphrody_video::init);
    let (app, window) = cx.add_window_view(|window, cx| VideoApp::new(window, cx));
    app.update(window, |app, cx| {
        app.set_skeleton_duration(Duration::ZERO);
        app.set_library(library, cx);
    });
    window.simulate_resize(size(px(1440.0), px(900.0)));
    window.run_until_parked();
    window.focus(&app);
    (app, window)
}

fn fixture_app(cx: &mut TestAppContext) -> (Entity<VideoApp>, &mut VisualTestContext) {
    open_app(cx, Library::scan(&fixture_root()).expect("scan fixtures"))
}

fn layout_at(
    app: &Entity<VideoApp>,
    window: &mut VisualTestContext,
    width: f32,
) -> layout::NavMode {
    window.simulate_resize(size(px(width), px(900.0)));
    app.update(window, |_, cx| cx.notify());
    window.run_until_parked();
    app.read_with(window, |app, _| app.layout().nav)
}

#[test]
fn library_indexes_fixtures_through_aphrody_media() {
    let library = Library::scan(&fixture_root()).expect("scan");
    assert_eq!(library.items().len(), 12);
    let creators: Vec<String> = library
        .creators()
        .into_iter()
        .map(|creator| creator.name)
        .collect();
    assert_eq!(
        creators,
        [
            "Field Notes",
            "Harbour Films",
            "Local files",
            "Night Studio"
        ]
    );
    assert_eq!(library.shorts().len(), 2);
    let archive = &library.items()[id_of(&library, "archive_reel")];
    assert!(!archive.playable, "mkv is listed but not decoded");
    let walk = &library.items()[id_of(&library, "coastline_walk")];
    assert!(walk.playable);
    assert_eq!((walk.width, walk.height), (320, 180));
    assert_eq!(walk.title, "Coastline walk at low tide");
    assert!(
        (2_900..=3_100).contains(&walk.duration_ms),
        "duration {}",
        walk.duration_ms
    );
    assert_eq!(walk.subtitles.len(), 2);
    assert!(walk.description.starts_with("A slow walk"));
}

#[test]
fn search_filters_the_local_library() {
    let library = Library::scan(&fixture_root()).expect("scan");
    let watched = HashSet::new();
    let ids = |query: &str, filter| -> Vec<String> {
        library
            .search(query, filter, &watched, u64::MAX / 2)
            .into_iter()
            .map(|hit| library.items()[hit.id].title.clone())
            .collect()
    };
    // Title, description and subtitle all mention "tide" in the first, only the title in the second.
    assert_eq!(
        ids("tide", SearchFilter::All),
        ["Coastline walk at low tide", "Reading the tide tables"]
    );
    let creator = ids("night studio", SearchFilter::All);
    assert_eq!(creator.len(), 3, "creator folder name matches: {creator:?}");
    let subtitle_hits = library.search("gulls", SearchFilter::All, &watched, 0);
    assert_eq!(subtitle_hits.len(), 1);
    let snippet = subtitle_hits[0]
        .snippet
        .as_ref()
        .expect("timestamped subtitle snippet");
    assert_eq!(snippet.start_ms, 1_500);
    assert!(ids("", SearchFilter::Shorts).len() == 2 && ids("", SearchFilter::Videos).len() == 10);
    assert!(ids("zzzz nothing", SearchFilter::All).is_empty());
    assert_eq!(
        library.suggest("harbur films").as_deref(),
        Some("harbour films")
    );
    assert_eq!(library.suggest("harbour"), None);
    let mut watched = HashSet::new();
    watched.insert(id_of(&library, "tide_tables"));
    let unwatched = library.search("tide", SearchFilter::Unwatched, &watched, 0);
    assert!(
        unwatched
            .iter()
            .all(|hit| library.items()[hit.id].title != "Reading the tide tables")
    );
}

#[gpui::test]
fn grid_columns_follow_the_width_table(cx: &mut TestAppContext) {
    let (app, window) = fixture_app(cx);
    let expected = [
        (1920.0, 3),
        (1440.0, 2),
        (1024.0, 2),
        (768.0, 2),
        (400.0, 1),
    ];
    for (width, columns) in expected {
        layout_at(&app, window, width);
        let shell = app.read_with(window, |app, _| app.layout());
        assert_eq!(shell.columns, columns, "expanded preference at {width}");
        assert_eq!(shell.viewport_width, width, "render recorded the viewport");
    }
    // Collapsing the rail at 1440 gives the third column of the spec table.
    layout_at(&app, window, 1440.0);
    app.update(window, |app, cx| app.toggle_menu(cx));
    window.run_until_parked();
    let shell = app.read_with(window, |app, _| app.layout());
    assert_eq!((shell.nav, shell.columns), (NavMode::Mini, 3));
    layout_at(&app, window, 1920.0);
    assert_eq!(app.read_with(window, |app, _| app.layout().columns), 4);
}

#[gpui::test]
fn navigation_collapses_at_breakpoints_and_drawer_closes(cx: &mut TestAppContext) {
    let (app, window) = fixture_app(cx);
    assert_eq!(layout_at(&app, window, 1440.0), NavMode::Expanded);
    assert_eq!(layout_at(&app, window, 1024.0), NavMode::Mini);
    assert_eq!(layout_at(&app, window, 768.0), NavMode::Hidden);

    // Below the inline-expanded width the menu button opens the overlay drawer.
    layout_at(&app, window, 1024.0);
    app.update(window, |app, cx| app.toggle_menu(cx));
    window.run_until_parked();
    assert!(app.read_with(window, |app, _| app.drawer_open()));
    window.simulate_keystrokes("escape");
    assert!(!app.read_with(window, |app, _| app.drawer_open()));

    // The watch view never shows an inline rail, even on a wide window.
    layout_at(&app, window, 1440.0);
    let id = app.read_with(window, |app, _| app.library().items()[0].id);
    app.update(window, |app, cx| app.open_watch(id, cx));
    window.run_until_parked();
    assert_eq!(
        app.read_with(window, |app, _| app.layout().nav),
        NavMode::Hidden
    );
    app.update(window, |app, cx| app.toggle_menu(cx));
    assert!(app.read_with(window, |app, _| app.drawer_open()));
    // Selecting an entry closes it.
    app.update(window, |app, cx| app.navigate(Route::Library, cx));
    assert!(!app.read_with(window, |app, _| app.drawer_open()));
}

#[gpui::test]
fn keyboard_table_drives_the_player(cx: &mut TestAppContext) {
    let (app, window) = fixture_app(cx);
    let id = app.read_with(window, |app, _| id_of(app.library(), "coastline_walk"));
    app.update(window, |app, cx| app.open_watch(id, cx));
    window.run_until_parked();
    let player =
        |window: &mut VisualTestContext| app.read_with(window, |app, _| app.player().clone());
    assert!(player(window).state.playing, "opening a video starts it");

    window.simulate_keystrokes("k");
    assert!(!player(window).state.playing);
    window.simulate_keystrokes("space");
    assert!(player(window).state.playing);
    window.simulate_keystrokes("k");

    window.simulate_keystrokes("m");
    assert!(player(window).state.muted);
    window.simulate_keystrokes("m");
    assert!(!player(window).state.muted);

    window.simulate_keystrokes("down");
    assert!((player(window).state.volume - 0.80).abs() < 1e-4);
    window.simulate_keystrokes("up up");
    assert!((player(window).state.volume - 0.90).abs() < 1e-4);

    window.simulate_keystrokes("t");
    assert!(player(window).theater);
    window.simulate_keystrokes("t");
    assert!(!player(window).theater);

    window.simulate_keystrokes("right");
    assert_eq!(
        player(window).state.position_ms,
        5_000.min(player(window).state.duration_ms)
    );
    window.simulate_keystrokes("5");
    let duration = player(window).state.duration_ms;
    assert_eq!(player(window).state.position_ms, duration / 2);
    window.simulate_keystrokes("home");
    assert_eq!(player(window).state.position_ms, 0);
    window.simulate_keystrokes("shift-.");
    assert_eq!(player(window).state.speed.0, 1.25);
    window.simulate_keystrokes("c");
    assert!(player(window).captions);

    window.simulate_keystrokes("i");
    assert!(player(window).miniplayer);
    assert_eq!(
        app.read_with(window, |app, _| app.route().clone()),
        Route::Home
    );
}

#[gpui::test]
fn letter_keys_type_into_the_focused_search_field(cx: &mut TestAppContext) {
    let (app, window) = fixture_app(cx);
    window.simulate_keystrokes("/");
    window.simulate_keystrokes("t i d e");
    window.simulate_keystrokes("enter");
    assert_eq!(
        app.read_with(window, |app, _| app.route().clone()),
        Route::Search("tide".to_owned())
    );
    let titles: Vec<String> = app.read_with(window, |app, _| {
        app.search_hits()
            .iter()
            .map(|hit| app.library().items()[hit.id].title.clone())
            .collect()
    });
    assert_eq!(
        titles,
        ["Coastline walk at low tide", "Reading the tide tables"]
    );
    // The `t` typed into the field did not toggle theater, `k` did not pause anything.
    assert!(!app.read_with(window, |app, _| app.player().theater));
    window.simulate_keystrokes("escape");
}

#[gpui::test]
fn miniplayer_persists_across_pages_and_expands_or_closes(cx: &mut TestAppContext) {
    let (app, window) = fixture_app(cx);
    let id = app.read_with(window, |app, _| id_of(app.library(), "neon_city"));
    app.update(window, |app, cx| app.open_watch(id, cx));
    window.run_until_parked();
    window.simulate_keystrokes("i");
    app.update(window, |app, cx| {
        app.navigate(Route::Search("neon".into()), cx)
    });
    app.update(window, |app, cx| app.navigate(Route::Library, cx));
    window.run_until_parked();
    assert!(app.read_with(window, |app, _| app.player().miniplayer
        && app.watching() == Some(id)));
    app.update(window, |app, cx| app.expand_miniplayer(cx));
    window.run_until_parked();
    assert_eq!(
        app.read_with(window, |app, _| app.route().clone()),
        Route::Watch(id)
    );
    assert!(!app.read_with(window, |app, _| app.player().miniplayer));
    window.simulate_keystrokes("i");
    app.update(window, |app, cx| app.close_miniplayer(cx));
    assert_eq!(app.read_with(window, |app, _| app.watching()), None);
}

#[gpui::test]
fn short_form_viewer_moves_one_item_and_updates_the_location(cx: &mut TestAppContext) {
    let (app, window) = fixture_app(cx);
    app.update(window, |app, cx| app.navigate(Route::Shorts, cx));
    window.run_until_parked();
    let first = app.read_with(window, |app, _| app.location());
    window.simulate_keystrokes("down");
    let second = app.read_with(window, |app, _| app.location());
    assert_ne!(first, second);
    window.simulate_keystrokes("down");
    assert_eq!(
        app.read_with(window, |app, _| app.location()),
        second,
        "stops at the last item"
    );
    window.simulate_keystrokes("up");
    assert_eq!(app.read_with(window, |app, _| app.location()), first);
}

#[gpui::test]
fn playback_decodes_h264_frames_and_advances_the_clock(cx: &mut TestAppContext) {
    let (app, window) = fixture_app(cx);
    let id = app.read_with(window, |app, _| id_of(app.library(), "coastline_walk"));
    app.update(window, |app, cx| app.open_watch(id, cx));
    window.executor().allow_parking();
    let mut shown = false;
    for _ in 0..400 {
        window.executor().advance_clock(Duration::from_millis(33));
        window.run_until_parked();
        if app.read_with(window, |app, _| {
            app.has_frame() && app.player().state.position_ms > 500
        }) {
            shown = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(shown, "no decoded frame reached the player");
    window.simulate_keystrokes("k");
    let paused_at = app.read_with(window, |app, _| app.player().state.position_ms);
    window.executor().advance_clock(Duration::from_millis(500));
    window.run_until_parked();
    assert_eq!(
        app.read_with(window, |app, _| app.player().state.position_ms),
        paused_at
    );
}

#[gpui::test]
fn unavailable_and_unsupported_videos_show_states_instead_of_panicking(cx: &mut TestAppContext) {
    let (app, window) = fixture_app(cx);
    app.update(window, |app, cx| app.navigate(Route::Watch(9_999), cx));
    window.run_until_parked();
    let id = app.read_with(window, |app, _| id_of(app.library(), "archive_reel"));
    app.update(window, |app, cx| app.open_watch(id, cx));
    window.run_until_parked();
    assert!(app.read_with(window, |app, _| app.playback_error().is_some()));
    app.update(window, |app, cx| {
        app.navigate(Route::Creator("Nobody".into()), cx)
    });
    app.update(window, |app, cx| app.navigate(Route::Following, cx));
    window.run_until_parked();
}

#[gpui::test]
fn infinite_scroll_appends_batches_with_a_spinner_delay(cx: &mut TestAppContext) {
    let directory = tempfile::tempdir().expect("tempdir");
    let source = fixture_root().join("Night Studio/loop_station.mp4");
    for index in 0..60 {
        std::fs::copy(
            &source,
            directory.path().join(format!("clip_{index:02}.mp4")),
        )
        .expect("copy");
    }
    let library = Library::scan(directory.path()).expect("scan");
    let (app, window) = open_app(cx, library);
    let state = |window: &mut VisualTestContext| {
        app.read_with(window, |app, _| {
            (
                app.visible_count(app.home_ids().len()),
                app.has_more(app.home_ids().len()),
            )
        })
    };
    assert_eq!(state(window), (layout::BATCH_SIZE, true));
    app.update(window, |app, cx| {
        let total = app.home_ids().len();
        app.load_more(total, cx);
    });
    assert_eq!(
        state(window).0,
        layout::BATCH_SIZE,
        "spinner shows before the batch lands"
    );
    window.executor().advance_clock(Duration::from_millis(300));
    window.run_until_parked();
    assert_eq!(state(window).0, layout::BATCH_SIZE * 2);
    for _ in 0..2 {
        app.update(window, |app, cx| {
            let total = app.home_ids().len();
            app.load_more(total, cx);
        });
        window.executor().advance_clock(Duration::from_millis(300));
        window.run_until_parked();
    }
    assert_eq!(state(window), (60, false));
}

#[gpui::test]
fn chips_filter_the_grid_and_reset_to_skeletons(cx: &mut TestAppContext) {
    let (app, window) = fixture_app(cx);
    let all = app.read_with(window, |app, _| app.home_ids().len());
    assert_eq!(all, 10);
    app.update(window, |app, cx| {
        app.set_skeleton_duration(Duration::from_millis(100));
        app.select_chip(aphrody_video::HomeChip::Creator("Night Studio".into()), cx);
    });
    assert!(app.read_with(window, |app, _| app.showing_skeleton()));
    window.executor().advance_clock(Duration::from_millis(150));
    window.run_until_parked();
    assert!(!app.read_with(window, |app, _| app.showing_skeleton()));
    assert_eq!(app.read_with(window, |app, _| app.home_ids().len()), 3);
}

#[gpui::test]
fn theme_follows_the_toggle_with_role_tokens(cx: &mut TestAppContext) {
    let (app, window) = fixture_app(cx);
    let (mode, base) = app.read_with(window, |app, _| (app.mode(), app.base_color()));
    app.update(window, |app, cx| app.toggle_theme(cx));
    let (next_mode, next_base) = app.read_with(window, |app, _| (app.mode(), app.base_color()));
    assert_ne!(mode, next_mode);
    assert_ne!(base, next_base);
}
