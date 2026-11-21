//! `VideoApp`: routes, library state, playback orchestration and action handlers.

use std::cell::Cell;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use aphrody_ui::{AppearanceMode, OsTheme, OsThemeCatalog};
use gpui::{
    AppContext as _, Bounds, Context, FocusHandle, Focusable, Pixels, RenderImage, ScrollHandle,
    Size, Task, UniformListScrollHandle, Window, WindowAppearance, actions, px,
};

use crate::decode::{FrameStream, StreamEvent, VideoFrame, decode_poster, decode_preview};
use crate::layout::{self, BATCH_SIZE, MenuAction, NavMode, PageKind};
use crate::library::{Library, SearchFilter, SearchHit, unix_now};
use crate::player::{PlayerEvent, PlayerModel, SEEK_LONG_MS, SEEK_SHORT_MS, VOLUME_STEP};
use crate::ui::Palette;

actions!(
    aphrody_video,
    [
        TogglePlay,
        ToggleMute,
        VolumeUp,
        VolumeDown,
        SeekBack5,
        SeekForward5,
        SeekBack10,
        SeekForward10,
        JumpToStart,
        JumpToEnd,
        Jump0,
        Jump1,
        Jump2,
        Jump3,
        Jump4,
        Jump5,
        Jump6,
        Jump7,
        Jump8,
        Jump9,
        SpeedUp,
        SpeedDown,
        StepFrameBack,
        StepFrameForward,
        ToggleTheater,
        ToggleMiniplayer,
        ToggleFullscreen,
        ToggleCaptions,
        ToggleAutoplay,
        FocusSearch,
        CloseLayer,
        NextShort,
        PreviousShort,
        ToggleMenu,
        ToggleTheme,
        GoHome,
        GoBack,
    ]
);

pub const POSTER_WIDTH: u32 = 480;
pub const PREVIEW_WIDTH: u32 = 320;
pub const PLAYBACK_WIDTH: u32 = 1280;
pub const PREVIEW_FRAMES: usize = 40;
pub const HOVER_PREVIEW_DWELL: Duration = Duration::from_millis(500);
const TICK: Duration = Duration::from_millis(33);
const POSTER_LANES: usize = 3;
const MORE_BATCH_DELAY: Duration = Duration::from_millis(250);
const SCRUB_RESTART_DISTANCE_MS: u64 = 250;
pub const SHORTS_WHEEL_COOLDOWN: Duration = Duration::from_millis(350);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    Home,
    Shorts,
    Watch(usize),
    Search(String),
    Creator(String),
    Following,
    Library,
}

impl Route {
    #[must_use]
    pub fn page_kind(&self) -> PageKind {
        match self {
            Self::Watch(_) => PageKind::Watch,
            Self::Shorts => PageKind::Shorts,
            _ => PageKind::List,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HomeChip {
    All,
    Recent,
    Unwatched,
    Creator(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsPanel {
    Root,
    Speed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatorTab {
    Home,
    Videos,
    Shorts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatorSort {
    Latest,
    Popular,
    Oldest,
}

pub(crate) struct HoverPreview {
    pub id: usize,
    pub since: std::time::Instant,
    pub requested: bool,
}

/// Geometry of the current frame, derived from the viewport and navigation state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellLayout {
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub page: PageKind,
    pub nav: NavMode,
    pub content_width: f32,
    pub columns: usize,
    pub card_width: f32,
}

pub struct VideoApp {
    pub(crate) focus_handle: FocusHandle,
    pub(crate) search_focus: FocusHandle,
    pub(crate) library: Arc<Library>,
    pub(crate) library_loading: bool,
    pub(crate) library_error: Option<String>,
    pub(crate) route: Route,
    pub(crate) back_stack: Vec<Route>,
    pub(crate) prefers_expanded: bool,
    pub(crate) drawer_open: bool,
    pub(crate) search_text: String,
    pub(crate) search_expanded: bool,
    pub(crate) search_filter: SearchFilter,
    pub(crate) search_results: Vec<SearchHit>,
    pub(crate) home_chip: HomeChip,
    pub(crate) chip_scroll: ScrollHandle,
    pub(crate) creator_tab: CreatorTab,
    pub(crate) creator_sort: CreatorSort,
    pub(crate) followed: BTreeSet<String>,
    pub(crate) history: Vec<usize>,
    pub(crate) positions: HashMap<usize, u64>,
    pub(crate) plays: HashMap<usize, u64>,
    pub(crate) watched: HashSet<usize>,
    pub(crate) home_ids: Vec<usize>,
    pub(crate) batches: usize,
    pub(crate) loading_more: bool,
    pub(crate) skeleton: bool,
    pub(crate) skeleton_duration: Duration,
    pub(crate) grid_scroll: UniformListScrollHandle,
    pub(crate) posters: HashMap<usize, Arc<RenderImage>>,
    pub(crate) previews: HashMap<usize, Vec<(u64, Arc<RenderImage>)>>,
    pub(crate) hover: Option<HoverPreview>,
    pub(crate) watching: Option<usize>,
    pub(crate) player: PlayerModel,
    pub(crate) frame: Option<Arc<RenderImage>>,
    pub(crate) playback_error: Option<String>,
    stream: Option<FrameStream>,
    pending_frame: Option<VideoFrame>,
    stream_ended: bool,
    awaiting_frame: bool,
    last_tick: Option<std::time::Instant>,
    tick_task: Option<Task<()>>,
    last_scrub_restart_ms: u64,
    pub(crate) shorts_cursor: usize,
    shorts_wheel_ready_at: Option<std::time::Instant>,
    pub(crate) scrubber_bounds: Rc<Cell<Bounds<Pixels>>>,
    pub(crate) scrub_hover: Option<f32>,
    pub(crate) description_expanded: bool,
    pub(crate) related_same_creator: bool,
    pub(crate) likes: HashSet<usize>,
    pub(crate) dislikes: HashSet<usize>,
    pub(crate) settings_panel: Option<SettingsPanel>,
    pub(crate) page_scroll: ScrollHandle,
    pub(crate) mode: AppearanceMode,
    follow_system_appearance: bool,
    catalog: OsThemeCatalog,
    pub(crate) theme: OsTheme,
    pub(crate) palette: Palette,
    pub(crate) reduced_motion: bool,
    pub(crate) viewport: Size<Pixels>,
    _appearance_subscription: gpui::Subscription,
}

impl Focusable for VideoApp {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

fn mode_from_appearance(appearance: WindowAppearance) -> AppearanceMode {
    match appearance {
        WindowAppearance::Dark | WindowAppearance::VibrantDark => AppearanceMode::Dark,
        WindowAppearance::Light | WindowAppearance::VibrantLight => AppearanceMode::Light,
    }
}

impl VideoApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mode = mode_from_appearance(window.appearance());
        let catalog = OsThemeCatalog::bundled().unwrap_or_default();
        let theme = catalog
            .resolve("aphrody", Some(mode))
            .or_else(|| {
                catalog
                    .ids()
                    .first()
                    .and_then(|id| catalog.resolve(id, Some(mode)))
            })
            .unwrap_or_else(|| OsTheme {
                id: "fallback".to_owned(),
                name: "Fallback".to_owned(),
                mode,
                colors: aphrody_ui::OsColorTokens::default(),
                layout: aphrody_ui::OsLayoutTokens::default(),
            });
        let palette = Palette::from_theme(&theme);
        let focus_handle = cx.focus_handle();
        let subscription = cx.observe_window_appearance(window, |this, window, cx| {
            if this.follow_system_appearance {
                this.set_mode(mode_from_appearance(window.appearance()), cx);
            }
        });
        Self {
            focus_handle,
            search_focus: cx.focus_handle(),
            library: Arc::new(Library::empty()),
            library_loading: true,
            library_error: None,
            route: Route::Home,
            back_stack: Vec::new(),
            prefers_expanded: true,
            drawer_open: false,
            search_text: String::new(),
            search_expanded: false,
            search_filter: SearchFilter::All,
            search_results: Vec::new(),
            home_chip: HomeChip::All,
            chip_scroll: ScrollHandle::new(),
            creator_tab: CreatorTab::Videos,
            creator_sort: CreatorSort::Latest,
            followed: BTreeSet::new(),
            history: Vec::new(),
            positions: HashMap::new(),
            plays: HashMap::new(),
            watched: HashSet::new(),
            home_ids: Vec::new(),
            batches: 1,
            loading_more: false,
            skeleton: true,
            skeleton_duration: Duration::from_millis(180),
            grid_scroll: UniformListScrollHandle::new(),
            posters: HashMap::new(),
            previews: HashMap::new(),
            hover: None,
            watching: None,
            player: PlayerModel::default(),
            frame: None,
            playback_error: None,
            stream: None,
            pending_frame: None,
            stream_ended: false,
            awaiting_frame: false,
            last_tick: None,
            tick_task: None,
            last_scrub_restart_ms: 0,
            shorts_cursor: 0,
            shorts_wheel_ready_at: None,
            scrubber_bounds: Rc::new(Cell::new(Bounds::default())),
            scrub_hover: None,
            description_expanded: false,
            related_same_creator: false,
            likes: HashSet::new(),
            dislikes: HashSet::new(),
            settings_panel: None,
            page_scroll: ScrollHandle::new(),
            mode,
            follow_system_appearance: true,
            catalog,
            theme,
            palette,
            reduced_motion: false,
            viewport: window.viewport_size(),
            _appearance_subscription: subscription,
        }
    }

    // ---- library -------------------------------------------------------------------------

    /// Scans `root` on a background thread and shows skeletons until it is indexed.
    pub fn open_library(&mut self, root: PathBuf, cx: &mut Context<Self>) {
        self.library_loading = true;
        self.library_error = None;
        let scan = cx.background_spawn(async move { Library::scan(&root) });
        let task = cx.spawn(async move |this, cx| {
            let result = scan.await;
            this.update(cx, |this, cx| match result {
                Ok(library) => this.set_library(library, cx),
                Err(error) => {
                    this.library_loading = false;
                    this.library_error = Some(format!("{error:#}"));
                    cx.notify();
                }
            })
            .ok();
        });
        task.detach();
        cx.notify();
    }

    pub fn set_library(&mut self, library: Library, cx: &mut Context<Self>) {
        self.library = Arc::new(library);
        self.library_loading = false;
        self.library_error = None;
        self.posters.clear();
        self.previews.clear();
        self.refresh_lists();
        self.spawn_posters(cx);
        self.begin_skeleton(cx);
        cx.notify();
    }

    fn spawn_posters(&mut self, cx: &mut Context<Self>) {
        let library = self.library.clone();
        for lane in 0..POSTER_LANES {
            let library = library.clone();
            let task = cx.spawn(async move |this, cx| {
                let ids: Vec<usize> = library
                    .items()
                    .iter()
                    .filter(|item| item.playable)
                    .map(|item| item.id)
                    .skip(lane)
                    .step_by(POSTER_LANES)
                    .collect();
                for id in ids {
                    let Some(item) = library.get(id) else {
                        continue;
                    };
                    let path = item.path.clone();
                    let at_ms = item.duration_ms / 10;
                    let decoded = cx
                        .background_spawn(async move { decode_poster(&path, at_ms, POSTER_WIDTH) })
                        .await;
                    match decoded {
                        Ok(frame) => {
                            if let Some(image) = frame.into_render_image() {
                                this.update(cx, |this, cx| {
                                    this.posters.insert(id, image);
                                    cx.notify();
                                })
                                .ok();
                            }
                        }
                        Err(error) => log::debug!("poster {id}: {error:#}"),
                    }
                }
            });
            task.detach();
        }
    }

    pub(crate) fn refresh_lists(&mut self) {
        let library = self.library.clone();
        let now = unix_now();
        self.home_ids = match &self.home_chip {
            HomeChip::All => library.feed(),
            HomeChip::Recent => library
                .search("", SearchFilter::Recent, &self.watched, now)
                .into_iter()
                .map(|hit| hit.id)
                .filter(|id| library.get(*id).is_some_and(|item| !item.is_short()))
                .collect(),
            HomeChip::Unwatched => library
                .feed()
                .into_iter()
                .filter(|id| !self.watched.contains(id))
                .collect(),
            HomeChip::Creator(name) => library.videos_by(name),
        };
        self.batches = 1;
        self.loading_more = false;
        self.refresh_search();
        let shorts = library.shorts().len();
        if self.shorts_cursor >= shorts {
            self.shorts_cursor = 0;
        }
    }

    pub(crate) fn refresh_search(&mut self) {
        if let Route::Search(query) = &self.route {
            self.search_results =
                self.library
                    .search(query, self.search_filter, &self.watched, unix_now());
        }
    }

    // ---- skeleton and infinite scroll ----------------------------------------------------

    pub(crate) fn begin_skeleton(&mut self, cx: &mut Context<Self>) {
        self.skeleton = true;
        let duration = self.skeleton_duration;
        let task = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(duration).await;
            this.update(cx, |this, cx| {
                this.skeleton = false;
                cx.notify();
            })
            .ok();
        });
        task.detach();
    }

    #[must_use]
    pub fn visible_count(&self, total: usize) -> usize {
        (self.batches * BATCH_SIZE).min(total)
    }

    #[must_use]
    pub fn has_more(&self, total: usize) -> bool {
        self.visible_count(total) < total
    }

    #[must_use]
    pub fn home_ids(&self) -> &[usize] {
        &self.home_ids
    }

    #[must_use]
    pub fn search_hits(&self) -> &[SearchHit] {
        &self.search_results
    }

    /// Requests the next batch; the spinner row shows for a short delay first.
    pub fn load_more(&mut self, total: usize, cx: &mut Context<Self>) {
        if self.loading_more || !self.has_more(total) {
            return;
        }
        self.loading_more = true;
        let task = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(MORE_BATCH_DELAY).await;
            this.update(cx, |this, cx| {
                this.batches += 1;
                this.loading_more = false;
                cx.notify();
            })
            .ok();
        });
        task.detach();
        cx.notify();
    }

    pub fn select_chip(&mut self, chip: HomeChip, cx: &mut Context<Self>) {
        if self.home_chip == chip {
            return;
        }
        self.home_chip = chip;
        self.refresh_lists();
        self.grid_scroll
            .scroll_to_item(0, gpui::ScrollStrategy::Top);
        self.begin_skeleton(cx);
        cx.notify();
    }

    // ---- navigation ----------------------------------------------------------------------

    #[must_use]
    pub fn route(&self) -> &Route {
        &self.route
    }

    /// Address of the current page; the short-form viewer keeps the item id in it.
    #[must_use]
    pub fn location(&self) -> String {
        match &self.route {
            Route::Home => "/".to_owned(),
            Route::Watch(id) => format!("/watch/{id}"),
            Route::Search(query) => format!("/results?q={query}"),
            Route::Creator(name) => format!("/creator/{name}"),
            Route::Following => "/following".to_owned(),
            Route::Library => "/library".to_owned(),
            Route::Shorts => match self.library.shorts().get(self.shorts_cursor) {
                Some(id) => format!("/shorts/{id}"),
                None => "/shorts".to_owned(),
            },
        }
    }

    pub fn navigate(&mut self, route: Route, cx: &mut Context<Self>) {
        if self.route == route {
            self.drawer_open = false;
            cx.notify();
            return;
        }
        let previous = std::mem::replace(&mut self.route, route.clone());
        self.back_stack.push(previous.clone());
        self.after_route_change(&previous, &route, cx);
    }

    pub fn go_back(&mut self, cx: &mut Context<Self>) {
        let Some(target) = self.back_stack.pop() else {
            return;
        };
        let previous = std::mem::replace(&mut self.route, target.clone());
        self.after_route_change(&previous, &target, cx);
    }

    fn after_route_change(&mut self, previous: &Route, route: &Route, cx: &mut Context<Self>) {
        self.drawer_open = false;
        self.search_expanded = false;
        self.batches = 1;
        self.loading_more = false;
        self.settings_panel = None;
        self.page_scroll.set_offset(gpui::point(px(0.0), px(0.0)));
        self.scrub_hover = None;
        self.hover = None;
        if let Route::Watch(previous_id) = previous
            && *route != Route::Watch(*previous_id)
        {
            if self.player.state.playing && self.watching == Some(*previous_id) {
                self.player.miniplayer = true;
            } else {
                self.unload();
            }
        }
        if matches!(previous, Route::Shorts) {
            self.unload();
        }
        match route {
            Route::Watch(id) => self.enter_watch(*id, cx),
            Route::Shorts => self.enter_short(cx),
            Route::Search(query) => {
                self.search_text.clone_from(query);
                self.refresh_search();
            }
            Route::Following | Route::Library | Route::Home | Route::Creator(_) => {}
        }
        if !matches!(route, Route::Watch(_)) {
            self.description_expanded = false;
        }
        self.begin_skeleton(cx);
        cx.notify();
    }

    pub fn open_watch(&mut self, id: usize, cx: &mut Context<Self>) {
        self.navigate(Route::Watch(id), cx);
    }

    pub fn submit_search(&mut self, cx: &mut Context<Self>) {
        let query = self.search_text.trim().to_owned();
        self.search_expanded = false;
        self.navigate(Route::Search(query), cx);
    }

    pub fn set_search_filter(&mut self, filter: SearchFilter, cx: &mut Context<Self>) {
        self.search_filter = filter;
        self.batches = 1;
        self.refresh_search();
        self.begin_skeleton(cx);
        cx.notify();
    }

    pub fn toggle_follow(&mut self, creator: &str, cx: &mut Context<Self>) {
        if !self.followed.remove(creator) {
            self.followed.insert(creator.to_owned());
        }
        cx.notify();
    }

    // ---- shell state -----------------------------------------------------------------------

    #[must_use]
    pub fn layout(&self) -> ShellLayout {
        self.layout_for(self.viewport)
    }

    #[must_use]
    pub fn layout_for(&self, viewport: Size<Pixels>) -> ShellLayout {
        let width = f32::from(viewport.width);
        let height = f32::from(viewport.height);
        let page = self.route.page_kind();
        let nav = layout::nav_mode(width, page, self.prefers_expanded);
        let content = layout::content_width(width, nav);
        let columns = layout::grid_columns(width, content);
        ShellLayout {
            viewport_width: width,
            viewport_height: height,
            page,
            nav,
            content_width: content,
            columns,
            card_width: layout::card_width(content, columns),
        }
    }

    pub fn toggle_menu(&mut self, cx: &mut Context<Self>) {
        let width = f32::from(self.viewport.width);
        match layout::menu_action(width, self.route.page_kind()) {
            MenuAction::ToggleRail => self.prefers_expanded = !self.prefers_expanded,
            MenuAction::OpenDrawer => self.drawer_open = !self.drawer_open,
        }
        cx.notify();
    }

    pub fn set_mode(&mut self, mode: AppearanceMode, cx: &mut Context<Self>) {
        self.mode = mode;
        if let Some(theme) = self.catalog.resolve(&self.theme.id, Some(mode)) {
            self.palette = Palette::from_theme(&theme);
            self.theme = theme;
        }
        cx.notify();
    }

    pub fn toggle_theme(&mut self, cx: &mut Context<Self>) {
        self.follow_system_appearance = false;
        let next = match self.mode {
            AppearanceMode::Dark => AppearanceMode::Light,
            AppearanceMode::Light => AppearanceMode::Dark,
        };
        self.set_mode(next, cx);
    }

    #[must_use]
    pub fn mode(&self) -> AppearanceMode {
        self.mode
    }

    #[must_use]
    pub fn drawer_open(&self) -> bool {
        self.drawer_open
    }

    #[must_use]
    pub fn poster_count(&self) -> usize {
        self.posters.len()
    }

    pub fn set_theater(&mut self, theater: bool) {
        self.player.theater = theater;
    }

    #[must_use]
    pub fn has_frame(&self) -> bool {
        self.frame.is_some()
    }

    #[must_use]
    pub fn playback_error(&self) -> Option<&str> {
        self.playback_error.as_deref()
    }

    #[must_use]
    pub fn showing_skeleton(&self) -> bool {
        self.skeleton
    }

    #[must_use]
    pub fn base_color(&self) -> gpui::Hsla {
        self.palette.base
    }

    #[must_use]
    pub fn player(&self) -> &PlayerModel {
        &self.player
    }

    #[must_use]
    pub fn watching(&self) -> Option<usize> {
        self.watching
    }

    #[must_use]
    pub fn library(&self) -> &Library {
        &self.library
    }

    pub fn set_reduced_motion(&mut self, reduced: bool) {
        self.reduced_motion = reduced;
    }

    pub fn set_skeleton_duration(&mut self, duration: Duration) {
        self.skeleton_duration = duration;
    }

    // ---- playback ----------------------------------------------------------------------------

    fn enter_watch(&mut self, id: usize, cx: &mut Context<Self>) {
        if self.watching == Some(id) && self.player.miniplayer {
            self.player.miniplayer = false;
            self.player.pointer_activity();
            return;
        }
        self.start_item(id, cx);
    }

    fn enter_short(&mut self, cx: &mut Context<Self>) {
        let shorts = self.library.shorts();
        if let Some(id) = shorts.get(self.shorts_cursor).copied() {
            self.start_item(id, cx);
        }
    }

    pub(crate) fn start_item(&mut self, id: usize, cx: &mut Context<Self>) {
        self.unload();
        let Some(item) = self.library.get(id).cloned() else {
            return;
        };
        self.player.load(
            &item.title,
            &item.path.display().to_string(),
            item.duration_ms,
        );
        self.watching = Some(id);
        *self.plays.entry(id).or_insert(0) += 1;
        self.history.retain(|existing| *existing != id);
        self.history.insert(0, id);
        if !item.path.is_file() {
            self.playback_error = Some("The file is no longer on disk.".to_owned());
            return;
        }
        if !item.playable {
            self.playback_error = Some(
                "This file is not MP4/MOV with H.264, which the built-in decoder plays.".to_owned(),
            );
            return;
        }
        let start = self
            .positions
            .get(&id)
            .copied()
            .filter(|position| *position + 2_000 < item.duration_ms);
        self.player.state.playing = true;
        self.player.buffering = true;
        if let Some(start) = start {
            self.player.state.position_ms = start;
        }
        self.restart_stream(self.player.state.position_ms);
        self.ensure_ticker(cx);
    }

    fn unload(&mut self) {
        if let Some(id) = self.watching {
            let position = self.player.state.position_ms;
            if position > 0 {
                self.positions.insert(id, position);
            }
            if self.player.ended
                || (self.player.state.duration_ms > 0
                    && position + 1_000 >= self.player.state.duration_ms)
            {
                self.watched.insert(id);
                self.positions.remove(&id);
            }
        }
        self.stream = None;
        self.pending_frame = None;
        self.frame = None;
        self.watching = None;
        self.playback_error = None;
        self.awaiting_frame = false;
        self.stream_ended = false;
        self.player.state.playing = false;
        self.player.miniplayer = false;
        self.player.buffering = false;
    }

    fn restart_stream(&mut self, start_ms: u64) {
        self.pending_frame = None;
        self.stream_ended = false;
        self.awaiting_frame = true;
        self.last_scrub_restart_ms = start_ms;
        let Some(item) = self.watching.and_then(|id| self.library.get(id)) else {
            return;
        };
        self.stream = Some(FrameStream::spawn(
            item.path.clone(),
            start_ms,
            PLAYBACK_WIDTH,
        ));
    }

    pub fn close_miniplayer(&mut self, cx: &mut Context<Self>) {
        self.unload();
        cx.notify();
    }

    pub fn expand_miniplayer(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.watching {
            self.player.miniplayer = true;
            self.navigate(Route::Watch(id), cx);
        }
    }

    /// Seeks and restarts decoding at the sync frame before `target_ms`.
    pub fn seek_to(&mut self, target_ms: u64, cx: &mut Context<Self>) {
        self.player.seek_to_ms(target_ms);
        let position = self.player.state.position_ms;
        if self.watching.is_some() && self.playback_error.is_none() {
            self.restart_stream(position);
            self.ensure_ticker(cx);
        }
        cx.notify();
    }

    fn seek_relative(&mut self, delta_ms: i64, cx: &mut Context<Self>) {
        let before = self.player.state.position_ms;
        self.player.seek_by_ms(delta_ms);
        let after = self.player.state.position_ms;
        if before != after && self.watching.is_some() && self.playback_error.is_none() {
            self.restart_stream(after);
            self.ensure_ticker(cx);
        }
        cx.notify();
    }

    pub(crate) fn begin_scrub(&mut self, fraction: f32, cx: &mut Context<Self>) {
        self.player.scrubbing = true;
        self.scrub_to(fraction, true, cx);
    }

    pub(crate) fn scrub_to(&mut self, fraction: f32, force: bool, cx: &mut Context<Self>) {
        let target = (fraction.clamp(0.0, 1.0) * self.player.state.duration_ms as f32) as u64;
        self.player.seek_to_ms(target);
        if self.watching.is_some()
            && self.playback_error.is_none()
            && (force || target.abs_diff(self.last_scrub_restart_ms) > SCRUB_RESTART_DISTANCE_MS)
        {
            self.restart_stream(target);
            self.ensure_ticker(cx);
        }
        cx.notify();
    }

    pub(crate) fn end_scrub(&mut self, cx: &mut Context<Self>) {
        if !self.player.scrubbing {
            return;
        }
        self.player.scrubbing = false;
        let target = self.player.state.position_ms;
        self.seek_to(target, cx);
    }

    pub(crate) fn ensure_ticker(&mut self, cx: &mut Context<Self>) {
        if self.tick_task.is_some() {
            return;
        }
        self.last_tick = Some(cx.background_executor().now());
        self.tick_task = Some(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(TICK).await;
                let keep = this.update(cx, |this, cx| this.tick(cx));
                match keep {
                    Ok(true) => {}
                    Ok(false) => {
                        this.update(cx, |this, _| this.tick_task = None).ok();
                        break;
                    }
                    Err(_) => break,
                }
            }
        }));
    }

    fn is_animating(&self) -> bool {
        self.player.state.playing
            || self.player.center_icon.is_some()
            || self.player.autoplay_countdown_ms.is_some()
            || self.awaiting_frame
            || self.hover.is_some()
    }

    /// One playback step. Returns whether the ticker should keep running.
    pub fn tick(&mut self, cx: &mut Context<Self>) -> bool {
        let now = cx.background_executor().now();
        let elapsed = self
            .last_tick
            .map_or(TICK, |last| now.saturating_duration_since(last));
        self.last_tick = Some(now);
        let mut changed = self.pump_hover_preview(now, cx);

        changed |= self.pull_frames();
        let stalled = self.player.state.playing
            && !self.stream_ended
            && self.pending_frame.is_none()
            && self.stream.is_some();
        if self.player.buffering != stalled {
            self.player.buffering = stalled;
            changed = true;
        }
        let elapsed_ms = if stalled || self.awaiting_frame {
            0
        } else {
            elapsed.as_millis() as u64
        };
        let was_ended = self.player.ended;
        let event = self.player.advance(elapsed_ms);
        changed |= self.player.state.playing || self.player.center_icon.is_some();
        if !was_ended && self.player.ended {
            changed = true;
            if self.route == Route::Shorts {
                self.player.state.playing = true;
                self.player.ended = false;
                self.player.autoplay_countdown_ms = None;
                self.seek_to(0, cx);
            }
        }
        if event == Some(PlayerEvent::AutoplayNext)
            && let Some(current) = self.watching
            && let Some(next) = self.library.next_after(current)
        {
            self.navigate_to_next(next, cx);
            changed = true;
        }
        if changed {
            cx.notify();
        }
        self.is_animating()
    }

    fn navigate_to_next(&mut self, next: usize, cx: &mut Context<Self>) {
        if matches!(self.route, Route::Watch(_)) {
            self.route = Route::Watch(next);
            self.start_item(next, cx);
        } else {
            self.start_item(next, cx);
        }
    }

    /// Takes decoded frames up to the playback clock; returns whether the shown frame changed.
    fn pull_frames(&mut self) -> bool {
        let position = self.player.state.position_ms;
        let tolerance = self.player.frame_ms.max(1);
        let mut chosen: Option<VideoFrame> = None;
        let mut failure: Option<String> = None;
        loop {
            let candidate = match self.pending_frame.take() {
                Some(frame) => Some(frame),
                None => match self.stream.as_mut().and_then(FrameStream::try_next) {
                    Some(StreamEvent::Frame(frame)) => Some(frame),
                    Some(StreamEvent::Ended) => {
                        self.stream_ended = true;
                        None
                    }
                    Some(StreamEvent::Failed(message)) => {
                        self.stream_ended = true;
                        failure = Some(message);
                        None
                    }
                    None => None,
                },
            };
            let Some(frame) = candidate else {
                break;
            };
            if self.awaiting_frame || frame.pts_ms <= position + tolerance {
                self.awaiting_frame = false;
                chosen = Some(frame);
            } else {
                self.pending_frame = Some(frame);
                break;
            }
        }
        if let Some(message) = failure {
            self.playback_error = Some(message);
            self.player.state.playing = false;
            return true;
        }
        if self.stream_ended {
            self.awaiting_frame = false;
        }
        let Some(frame) = chosen else {
            return false;
        };
        self.frame = frame.into_render_image();
        true
    }

    // ---- hover preview -----------------------------------------------------------------------

    pub(crate) fn set_card_hover(&mut self, id: usize, hovered: bool, cx: &mut Context<Self>) {
        if hovered {
            if self.reduced_motion {
                return;
            }
            self.hover = Some(HoverPreview {
                id,
                since: cx.background_executor().now(),
                requested: false,
            });
            self.ensure_ticker(cx);
        } else if self.hover.as_ref().is_some_and(|hover| hover.id == id) {
            self.hover = None;
        }
        cx.notify();
    }

    fn pump_hover_preview(&mut self, now: std::time::Instant, cx: &mut Context<Self>) -> bool {
        let Some(hover) = &mut self.hover else {
            return false;
        };
        let dwelled = now.saturating_duration_since(hover.since) >= HOVER_PREVIEW_DWELL;
        let id = hover.id;
        if dwelled && !hover.requested && !self.previews.contains_key(&id) {
            hover.requested = true;
            if let Some(item) = self.library.get(id).filter(|item| item.playable) {
                let path = item.path.clone();
                let start = item.duration_ms / 10;
                let task = cx.spawn(async move |this, cx| {
                    let decoded = cx
                        .background_spawn(async move {
                            decode_preview(&path, start, PREVIEW_FRAMES, PREVIEW_WIDTH)
                        })
                        .await;
                    if let Ok(frames) = decoded {
                        let images: Vec<(u64, Arc<RenderImage>)> = frames
                            .into_iter()
                            .filter_map(|frame| {
                                let pts = frame.pts_ms;
                                frame.into_render_image().map(|image| (pts, image))
                            })
                            .collect();
                        this.update(cx, |this, cx| {
                            this.previews.insert(id, images);
                            cx.notify();
                        })
                        .ok();
                    }
                });
                task.detach();
            }
        }
        dwelled
    }

    /// The inline-preview image for a card, if its dwell time has passed and frames are decoded.
    pub(crate) fn preview_frame(
        &self,
        id: usize,
        now: std::time::Instant,
    ) -> Option<(Arc<RenderImage>, f32)> {
        let hover = self.hover.as_ref().filter(|hover| hover.id == id)?;
        let dwell = now.saturating_duration_since(hover.since);
        if dwell < HOVER_PREVIEW_DWELL {
            return None;
        }
        let frames = self.previews.get(&id)?;
        if frames.is_empty() {
            return None;
        }
        let tick = (dwell - HOVER_PREVIEW_DWELL).as_millis() as usize / 100;
        let index = tick % frames.len();
        Some((
            frames[index].1.clone(),
            (index + 1) as f32 / frames.len() as f32,
        ))
    }

    // ---- short-form viewer -------------------------------------------------------------------

    pub fn step_short(&mut self, direction: isize, cx: &mut Context<Self>) {
        let shorts = self.library.shorts();
        let target = self.shorts_cursor as isize + direction;
        if target < 0 || target as usize >= shorts.len() {
            return;
        }
        self.shorts_cursor = target as usize;
        if let Some(id) = shorts.get(self.shorts_cursor).copied() {
            self.start_item(id, cx);
        }
        cx.notify();
    }

    pub(crate) fn wheel_short(&mut self, direction: isize, cx: &mut Context<Self>) {
        let now = cx.background_executor().now();
        if self.shorts_wheel_ready_at.is_some_and(|ready| now < ready) {
            return;
        }
        self.shorts_wheel_ready_at = Some(now + SHORTS_WHEEL_COOLDOWN);
        self.step_short(direction, cx);
    }

    // ---- action handlers ---------------------------------------------------------------------

    fn in_player_route(&self) -> bool {
        matches!(self.route, Route::Watch(_) | Route::Shorts)
    }

    pub(crate) fn on_toggle_play(
        &mut self,
        _: &TogglePlay,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_play(cx);
    }

    pub fn toggle_play(&mut self, cx: &mut Context<Self>) {
        if self.watching.is_none() || self.playback_error.is_some() {
            return;
        }
        let was_ended = self.player.ended;
        self.player.toggle_play();
        if was_ended {
            self.restart_stream(0);
        }
        self.ensure_ticker(cx);
        cx.notify();
    }

    pub(crate) fn on_toggle_mute(
        &mut self,
        _: &ToggleMute,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.in_player_route() || self.player.miniplayer {
            self.player.toggle_mute();
            self.ensure_ticker(cx);
            cx.notify();
        }
    }

    pub(crate) fn on_volume_up(&mut self, _: &VolumeUp, _: &mut Window, cx: &mut Context<Self>) {
        self.player.adjust_volume(VOLUME_STEP);
        self.ensure_ticker(cx);
        cx.notify();
    }

    pub(crate) fn on_volume_down(
        &mut self,
        _: &VolumeDown,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.player.adjust_volume(-VOLUME_STEP);
        self.ensure_ticker(cx);
        cx.notify();
    }

    pub(crate) fn on_seek_back_5(&mut self, _: &SeekBack5, _: &mut Window, cx: &mut Context<Self>) {
        self.seek_relative(-SEEK_SHORT_MS, cx);
    }

    pub(crate) fn on_seek_forward_5(
        &mut self,
        _: &SeekForward5,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.seek_relative(SEEK_SHORT_MS, cx);
    }

    pub(crate) fn on_seek_back_10(
        &mut self,
        _: &SeekBack10,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.seek_relative(-SEEK_LONG_MS, cx);
    }

    pub(crate) fn on_seek_forward_10(
        &mut self,
        _: &SeekForward10,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.seek_relative(SEEK_LONG_MS, cx);
    }

    pub(crate) fn on_jump_to_start(
        &mut self,
        _: &JumpToStart,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.seek_to(0, cx);
    }

    pub(crate) fn on_jump_to_end(&mut self, _: &JumpToEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.player.jump_to_end();
        let end = self.player.state.position_ms;
        if self.watching.is_some() && self.playback_error.is_none() {
            self.restart_stream(end.saturating_sub(self.player.frame_ms * 2));
            self.ensure_ticker(cx);
        }
        cx.notify();
    }

    pub fn jump_to_tenth(&mut self, tenth: u64, cx: &mut Context<Self>) {
        let target = self.player.state.duration_ms * tenth.min(9) / 10;
        self.seek_to(target, cx);
    }

    pub(crate) fn on_speed_up(&mut self, _: &SpeedUp, _: &mut Window, cx: &mut Context<Self>) {
        self.player.step_speed(1);
        cx.notify();
    }

    pub(crate) fn on_speed_down(&mut self, _: &SpeedDown, _: &mut Window, cx: &mut Context<Self>) {
        self.player.step_speed(-1);
        cx.notify();
    }

    pub(crate) fn on_step_frame_back(
        &mut self,
        _: &StepFrameBack,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.step_frame(-1, cx);
    }

    pub(crate) fn on_step_frame_forward(
        &mut self,
        _: &StepFrameForward,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.step_frame(1, cx);
    }

    fn step_frame(&mut self, direction: i32, cx: &mut Context<Self>) {
        if self.player.state.playing {
            return;
        }
        self.player.step_frame(direction);
        let position = self.player.state.position_ms;
        if self.watching.is_some() && self.playback_error.is_none() {
            self.restart_stream(position);
            self.ensure_ticker(cx);
        }
        cx.notify();
    }

    pub(crate) fn on_toggle_theater(
        &mut self,
        _: &ToggleTheater,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if matches!(self.route, Route::Watch(_)) {
            self.player.toggle_theater();
            cx.notify();
        }
    }

    /// `i`: the miniplayer opens and the page returns to home while the video keeps playing.
    pub(crate) fn on_toggle_miniplayer(
        &mut self,
        _: &ToggleMiniplayer,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.player.miniplayer {
            self.expand_miniplayer(cx);
        } else if matches!(self.route, Route::Watch(_)) && self.watching.is_some() {
            self.player.miniplayer = true;
            if let Some(id) = self.watching {
                self.navigate_keep_player(id, cx);
            }
        }
    }

    /// Leaves the watch page for home while the video keeps playing in the miniplayer.
    pub(crate) fn navigate_keep_player(&mut self, _id: usize, cx: &mut Context<Self>) {
        self.player.miniplayer = true;
        self.back_stack.clear();
        self.route = Route::Home;
        self.drawer_open = false;
        self.settings_panel = None;
        self.batches = 1;
        self.begin_skeleton(cx);
        self.ensure_ticker(cx);
        cx.notify();
    }

    pub(crate) fn frame_lead_ms(&self) -> Option<u64> {
        self.pending_frame.as_ref().map(|frame| frame.pts_ms)
    }

    pub(crate) fn on_toggle_fullscreen(
        &mut self,
        _: &ToggleFullscreen,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.in_player_route() {
            self.player.toggle_fullscreen();
            window.toggle_fullscreen();
            cx.notify();
        }
    }

    pub(crate) fn on_toggle_captions(
        &mut self,
        _: &ToggleCaptions,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.in_player_route() {
            self.player.toggle_captions();
            cx.notify();
        }
    }

    pub(crate) fn on_toggle_autoplay(
        &mut self,
        _: &ToggleAutoplay,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.player.toggle_autoplay();
        cx.notify();
    }

    pub(crate) fn on_focus_search(
        &mut self,
        _: &FocusSearch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.search_expanded = true;
        window.focus(&self.search_focus, cx);
        cx.notify();
    }

    /// Escape closes the top-most layer: search focus, drawer, fullscreen, miniplayer.
    pub(crate) fn on_close_layer(
        &mut self,
        _: &CloseLayer,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.search_focus.is_focused(window) {
            window.focus(&self.focus_handle, cx);
            self.search_expanded = false;
        } else if self.drawer_open {
            self.drawer_open = false;
        } else if self.player.fullscreen {
            self.player.toggle_fullscreen();
            window.toggle_fullscreen();
        } else if self.player.autoplay_countdown_ms.is_some() {
            self.player.cancel_autoplay();
        }
        cx.notify();
    }

    pub(crate) fn on_next_short(&mut self, _: &NextShort, _: &mut Window, cx: &mut Context<Self>) {
        self.step_short(1, cx);
    }

    pub(crate) fn on_previous_short(
        &mut self,
        _: &PreviousShort,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.step_short(-1, cx);
    }

    pub(crate) fn on_toggle_menu(
        &mut self,
        _: &ToggleMenu,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_menu(cx);
    }

    pub(crate) fn on_toggle_theme(
        &mut self,
        _: &ToggleTheme,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_theme(cx);
    }

    pub(crate) fn on_go_home(&mut self, _: &GoHome, _: &mut Window, cx: &mut Context<Self>) {
        self.navigate(Route::Home, cx);
    }

    pub(crate) fn on_go_back(&mut self, _: &GoBack, _: &mut Window, cx: &mut Context<Self>) {
        self.go_back(cx);
    }
}
