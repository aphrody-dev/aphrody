//! Page renders: home, search, creator, following, library, short-form viewer and watch view,
//! plus the player surface, miniplayer and the root `Render` implementation.

use std::ops::Range;
use std::rc::Rc;

use gpui::{
    AnyElement, ClipboardItem, Context, FontWeight, InteractiveElement as _, MouseButton,
    ParentElement as _, Pixels, ScrollEvent, Styled as _, Window, black, div, linear_color_stop,
    linear_gradient, prelude::*, px, uniform_list, white,
};
use icons::IconName;

use crate::app::{CreatorSort, CreatorTab, HomeChip, Route, ShellLayout, VideoApp};
use crate::layout::{self, GRID_GAP, GRID_ROW_GAP, PAGE_PADDING, TOP_BAR_HEIGHT, thumbnail_height};
use crate::library::{SearchFilter, format_count, format_duration, relative_age, unix_now};
use crate::ui::{Activate, ChipSpec, glyph, h_flex, text, v_flex};

/// Body of a card below its thumbnail, including the 12 px gap.
const CARD_BODY_HEIGHT: f32 = 110.0;
const RESULTS_MAX_WIDTH: f32 = 1280.0;
const CREATOR_MAX_WIDTH: f32 = 1308.0;

fn clipboard_path(item_path: &std::path::Path) -> ClipboardItem {
    ClipboardItem::new_string(item_path.display().to_string())
}

impl gpui::Render for VideoApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.viewport = window.viewport_size();
        let shell = self.layout();
        let palette = self.palette.clone();
        let context = match self.route {
            Route::Watch(_) => "AphrodyVideo AphrodyWatch",
            Route::Shorts => "AphrodyVideo AphrodyShorts",
            _ => "AphrodyVideo AphrodyList",
        };
        let page = match self.route.clone() {
            Route::Home => self.render_home(&shell, cx),
            Route::Search(query) => self.render_search(&shell, &query, cx),
            Route::Creator(name) => self.render_creator(&shell, &name, cx),
            Route::Following => self.render_following(&shell, cx),
            Route::Library => self.render_library(&shell, cx),
            Route::Shorts => self.render_shorts(&shell, cx),
            Route::Watch(id) => self.render_watch(&shell, id, cx),
        };
        let show_mini = self.player.miniplayer && !matches!(self.route, Route::Watch(_));
        v_flex()
            .id("aphrody-video")
            .key_context(context)
            .track_focus(&self.focus_handle)
            .relative()
            .size_full()
            .bg(palette.base)
            .text_color(palette.text)
            .on_action(cx.listener(Self::on_toggle_play))
            .on_action(cx.listener(Self::on_toggle_mute))
            .on_action(cx.listener(Self::on_volume_up))
            .on_action(cx.listener(Self::on_volume_down))
            .on_action(cx.listener(Self::on_seek_back_5))
            .on_action(cx.listener(Self::on_seek_forward_5))
            .on_action(cx.listener(Self::on_seek_back_10))
            .on_action(cx.listener(Self::on_seek_forward_10))
            .on_action(cx.listener(Self::on_jump_to_start))
            .on_action(cx.listener(Self::on_jump_to_end))
            .on_action(cx.listener(|this, _: &crate::app::Jump0, _, cx| this.jump_to_tenth(0, cx)))
            .on_action(cx.listener(|this, _: &crate::app::Jump1, _, cx| this.jump_to_tenth(1, cx)))
            .on_action(cx.listener(|this, _: &crate::app::Jump2, _, cx| this.jump_to_tenth(2, cx)))
            .on_action(cx.listener(|this, _: &crate::app::Jump3, _, cx| this.jump_to_tenth(3, cx)))
            .on_action(cx.listener(|this, _: &crate::app::Jump4, _, cx| this.jump_to_tenth(4, cx)))
            .on_action(cx.listener(|this, _: &crate::app::Jump5, _, cx| this.jump_to_tenth(5, cx)))
            .on_action(cx.listener(|this, _: &crate::app::Jump6, _, cx| this.jump_to_tenth(6, cx)))
            .on_action(cx.listener(|this, _: &crate::app::Jump7, _, cx| this.jump_to_tenth(7, cx)))
            .on_action(cx.listener(|this, _: &crate::app::Jump8, _, cx| this.jump_to_tenth(8, cx)))
            .on_action(cx.listener(|this, _: &crate::app::Jump9, _, cx| this.jump_to_tenth(9, cx)))
            .on_action(cx.listener(Self::on_speed_up))
            .on_action(cx.listener(Self::on_speed_down))
            .on_action(cx.listener(Self::on_step_frame_back))
            .on_action(cx.listener(Self::on_step_frame_forward))
            .on_action(cx.listener(Self::on_toggle_theater))
            .on_action(cx.listener(Self::on_toggle_miniplayer))
            .on_action(cx.listener(Self::on_toggle_fullscreen))
            .on_action(cx.listener(Self::on_toggle_captions))
            .on_action(cx.listener(Self::on_toggle_autoplay))
            .on_action(cx.listener(Self::on_focus_search))
            .on_action(cx.listener(Self::on_close_layer))
            .on_action(cx.listener(Self::on_next_short))
            .on_action(cx.listener(Self::on_previous_short))
            .on_action(cx.listener(Self::on_toggle_menu))
            .on_action(cx.listener(Self::on_toggle_theme))
            .on_action(cx.listener(Self::on_go_home))
            .on_action(cx.listener(Self::on_go_back))
            .on_mouse_move(
                cx.listener(|this, event: &gpui::MouseMoveEvent, _window, cx| {
                    if this.player.scrubbing {
                        if let Some(fraction) = this.scrubber_fraction(event.position) {
                            this.scrub_to(fraction, false, cx);
                        }
                    }
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _event, _window, cx| this.end_scrub(cx)),
            )
            .when(
                !(self.player.fullscreen && matches!(self.route, Route::Watch(_))),
                |element| element.child(self.render_top_bar(&shell, window, cx)),
            )
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .items_start()
                    .w_full()
                    .child(self.render_nav(shell.nav, cx))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .overflow_hidden()
                            .child(page),
                    ),
            )
            .when(self.drawer_open, |element| {
                element.child(self.render_drawer(cx))
            })
            .when(show_mini, |element| {
                element.child(self.render_miniplayer(cx))
            })
    }
}

impl VideoApp {
    pub(crate) fn scrubber_fraction(&self, position: gpui::Point<Pixels>) -> Option<f32> {
        let bounds = self.scrubber_bounds.get();
        let width = f32::from(bounds.size.width);
        if width <= 0.0 {
            return None;
        }
        Some(((f32::from(position.x) - f32::from(bounds.origin.x)) / width).clamp(0.0, 1.0))
    }

    // ---- home ----------------------------------------------------------------------------------

    fn home_chips(&self) -> Vec<ChipSpec> {
        let mut chips = vec![
            (HomeChip::All, "All".to_owned()),
            (HomeChip::Recent, "Recently added".to_owned()),
            (HomeChip::Unwatched, "Unwatched".to_owned()),
        ];
        for creator in self.library.creators() {
            if creator.video_count > 0 {
                chips.push((HomeChip::Creator(creator.name.clone()), creator.name));
            }
        }
        chips
            .into_iter()
            .map(|(chip, label)| ChipSpec {
                label: label.into(),
                selected: self.home_chip == chip,
                on_select: Rc::new(move |this, _window, cx| this.select_chip(chip.clone(), cx)),
            })
            .collect()
    }

    fn library_empty_state(&self, cx: &mut Context<Self>) -> AnyElement {
        if let Some(error) = &self.library_error {
            return self.empty_state("The library could not be read", error, None, cx);
        }
        self.empty_state(
            "No videos in this library yet",
            "Put MP4 or MOV files in the library folder. Folders become creators and are indexed automatically.",
            Some((
                "Scan again",
                Rc::new(|this, _window, cx| {
                    let root = this.library.root().to_owned();
                    if !root.as_os_str().is_empty() {
                        this.open_library(root, cx);
                    }
                }),
            )),
            cx,
        )
    }

    fn skeleton_grid(&self, shell: &ShellLayout, with_avatar: bool) -> AnyElement {
        let rows = ((shell.viewport_height - TOP_BAR_HEIGHT)
            / (thumbnail_height(shell.card_width) + CARD_BODY_HEIGHT))
            .ceil()
            .max(1.0) as usize;
        let mut column = v_flex()
            .px(px(PAGE_PADDING))
            .pt(px(8.0))
            .gap(px(GRID_ROW_GAP))
            .overflow_hidden();
        for _ in 0..rows {
            let mut row = h_flex().items_start().gap(px(GRID_GAP));
            for _ in 0..shell.columns {
                row = row.child(self.skeleton_card(shell.card_width, with_avatar));
            }
            column = column.child(row);
        }
        column.into_any_element()
    }

    pub(crate) fn render_home(
        &mut self,
        shell: &ShellLayout,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let chip_bar = self.render_chip_bar(self.home_chips(), cx);
        let total = self.home_ids.len();
        let body = if self.library_loading || self.skeleton {
            self.skeleton_grid(shell, true)
        } else if total == 0 {
            if self.library.is_empty() {
                self.library_empty_state(cx)
            } else {
                self.empty_state(
                    "Nothing matches this filter",
                    "Pick another chip to see more of your library.",
                    None,
                    cx,
                )
            }
        } else {
            let columns = shell.columns;
            let card_width = shell.card_width;
            let visible = self.visible_count(total);
            let rows = visible.div_ceil(columns);
            let row_height = thumbnail_height(card_width) + CARD_BODY_HEIGHT + GRID_ROW_GAP;
            let list = uniform_list(
                "home-grid",
                rows,
                cx.processor(move |this, range: Range<usize>, _window, cx| {
                    if range.end + 1 >= rows && this.has_more(this.home_ids.len()) {
                        this.load_more(this.home_ids.len(), cx);
                    }
                    range
                        .map(|row| {
                            let mut element = h_flex()
                                .items_start()
                                .h(px(row_height))
                                .px(px(PAGE_PADDING))
                                .gap(px(GRID_GAP));
                            for column in 0..columns {
                                let index = row * columns + column;
                                if index >= visible {
                                    break;
                                }
                                let id = this.home_ids[index];
                                element = element.child(this.video_card(id, card_width, true, cx));
                            }
                            element
                        })
                        .collect()
                }),
            )
            .track_scroll(&self.grid_scroll)
            .size_full();
            v_flex()
                .size_full()
                .child(div().flex_1().min_h_0().pt(px(8.0)).child(list))
                .when(self.has_more(total) || self.loading_more, |element| {
                    element.child(self.spinner_row())
                })
                .into_any_element()
        };
        v_flex()
            .size_full()
            .child(chip_bar)
            .child(div().flex_1().min_h_0().child(body))
            .into_any_element()
    }

    // ---- plain grids (creator, following, library) ----------------------------------------------

    fn grid_of(
        &self,
        ids: &[usize],
        shell: &ShellLayout,
        content_width: f32,
        with_avatar: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let columns = shell
            .columns
            .min(layout::grid_columns(shell.viewport_width, content_width))
            .max(1);
        let width = layout::card_width(content_width, columns);
        let mut grid = h_flex()
            .flex_wrap()
            .items_start()
            .gap_x(px(GRID_GAP))
            .gap_y(px(GRID_ROW_GAP))
            .w_full();
        for id in ids {
            grid = grid.child(self.video_card(*id, width, with_avatar, cx));
        }
        grid.into_any_element()
    }

    fn list_page(
        &mut self,
        shell: &ShellLayout,
        ids: Vec<usize>,
        heading: &str,
        empty_headline: &str,
        empty_body: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = self.palette.clone();
        if self.skeleton || self.library_loading {
            return self.skeleton_grid(shell, true);
        }
        if ids.is_empty() {
            return self.empty_state(empty_headline, empty_body, None, cx);
        }
        let grid = self.grid_of(&ids, shell, shell.content_width, true, cx);
        div()
            .id("list-page")
            .size_full()
            .overflow_y_scroll()
            .px(px(PAGE_PADDING))
            .py(px(16.0))
            .child(
                text(
                    heading.to_owned(),
                    24.0,
                    32.0,
                    FontWeight::BOLD,
                    palette.text,
                )
                .mb(px(16.0)),
            )
            .child(grid)
            .into_any_element()
    }

    pub(crate) fn render_following(
        &mut self,
        shell: &ShellLayout,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut ids: Vec<usize> = self
            .followed
            .iter()
            .flat_map(|creator| self.library.videos_by(creator))
            .collect();
        ids.sort_by_key(|id| {
            std::cmp::Reverse(self.library.get(*id).map_or(0, |item| item.modified_secs))
        });
        self.list_page(
            shell,
            ids,
            "Following",
            "You follow nobody yet",
            "Open a creator page and press Follow to collect their newest videos here.",
            cx,
        )
    }

    pub(crate) fn render_library(
        &mut self,
        shell: &ShellLayout,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ids = self.history.clone();
        self.list_page(
            shell,
            ids,
            "History",
            "Nothing watched yet",
            "Videos you open appear here with where you stopped.",
            cx,
        )
    }

    // ---- search ----------------------------------------------------------------------------------

    pub(crate) fn render_search(
        &mut self,
        shell: &ShellLayout,
        query: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = self.palette.clone();
        let chips: Vec<ChipSpec> = SearchFilter::ALL
            .into_iter()
            .map(|filter| ChipSpec {
                label: filter.label().into(),
                selected: self.search_filter == filter,
                on_select: Rc::new(move |this, _window, cx| this.set_search_filter(filter, cx)),
            })
            .collect();
        let chip_bar = self.render_chip_bar(chips, cx);
        let hits = self.search_results.clone();
        let content_width = shell.content_width.min(RESULTS_MAX_WIDTH);
        let thumb_width = layout::search_thumbnail_width(shell.viewport_width);

        let mut column = v_flex().w_full().max_w(px(RESULTS_MAX_WIDTH)).gap(px(16.0));
        if self.skeleton || self.library_loading {
            for _ in 0..5 {
                column = column.child(
                    h_flex()
                        .items_start()
                        .gap(px(16.0))
                        .child(
                            div()
                                .w(px(thumb_width))
                                .h(px(thumbnail_height(thumb_width)))
                                .flex_none()
                                .rounded(px(12.0))
                                .bg(palette.surface_high),
                        )
                        .child(
                            v_flex()
                                .flex_1()
                                .gap(px(10.0))
                                .child(
                                    div()
                                        .w_full()
                                        .h(px(18.0))
                                        .rounded(px(4.0))
                                        .bg(palette.surface_high),
                                )
                                .child(
                                    div()
                                        .w(px(160.0))
                                        .h(px(12.0))
                                        .rounded(px(4.0))
                                        .bg(palette.surface_high),
                                ),
                        ),
                );
            }
        } else if hits.is_empty() {
            let suggestion = self.library.suggest(query);
            let mut block = v_flex().gap(px(12.0)).pt(px(32.0));
            block = block.child(text(
                format!("No results for \"{query}\""),
                20.0,
                28.0,
                FontWeight::BOLD,
                palette.text,
            ));
            if let Some(suggestion) = suggestion {
                let target = suggestion.clone();
                let link = div().id("did-you-mean").child(text(
                    format!("Did you mean {suggestion}?"),
                    14.0,
                    20.0,
                    FontWeight::MEDIUM,
                    palette.link,
                ));
                block = block.child(self.activatable(
                    link,
                    true,
                    cx,
                    Rc::new(move |this, _window, cx| {
                        this.search_text.clone_from(&target);
                        this.navigate(Route::Search(target.clone()), cx);
                    }),
                ));
            }
            block = block.child(text(
                "Try a title, a creator folder, or words from a subtitle file.",
                14.0,
                20.0,
                FontWeight::NORMAL,
                palette.text_muted,
            ));
            column = column.child(block);
        } else {
            column = column.child(text(
                format!(
                    "{} for \"{query}\"",
                    format_count(hits.len() as u64, "result")
                ),
                12.0,
                18.0,
                FontWeight::NORMAL,
                palette.text_muted,
            ));
            let lowered = query.to_lowercase();
            if let Some(creator) = self.library.creators().into_iter().find(|creator| {
                !lowered.is_empty() && creator.name.to_lowercase().contains(&lowered)
            }) {
                column = column.child(self.channel_row(
                    &creator.name,
                    creator.video_count,
                    creator.short_count,
                    cx,
                ));
                column = column.child(self.mix_row(&creator.name, thumb_width, cx));
            }
            let total = hits.len();
            let visible = self.visible_count(total);
            let mut shelf_done = false;
            for (position, hit) in hits.iter().take(visible).enumerate() {
                if self.library.get(hit.id).is_some_and(|item| item.is_short()) {
                    continue;
                }
                column = column.child(self.result_row(hit, thumb_width, cx));
                if position == 2 && !shelf_done {
                    shelf_done = true;
                    let shorts: Vec<usize> = hits
                        .iter()
                        .filter(|hit| self.library.get(hit.id).is_some_and(|item| item.is_short()))
                        .map(|hit| hit.id)
                        .collect();
                    if !shorts.is_empty() {
                        column = column.child(self.shorts_shelf(&shorts, cx));
                    }
                }
            }
            if !shelf_done {
                let shorts: Vec<usize> = hits
                    .iter()
                    .filter(|hit| self.library.get(hit.id).is_some_and(|item| item.is_short()))
                    .map(|hit| hit.id)
                    .collect();
                if !shorts.is_empty() {
                    column = column.child(self.shorts_shelf(&shorts, cx));
                }
            }
            if self.has_more(total) {
                column = column.child(self.spinner_row());
                let remaining = f32::from(self.page_scroll.max_offset().y)
                    + f32::from(self.page_scroll.offset().y);
                if remaining < shell.viewport_height {
                    self.load_more(total, cx);
                }
            }
        }
        let _ = content_width;
        v_flex()
            .size_full()
            .child(chip_bar)
            .child(
                div()
                    .id("results")
                    .track_scroll(&self.page_scroll)
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(px(PAGE_PADDING))
                    .pb(px(24.0))
                    .child(column),
            )
            .into_any_element()
    }

    fn result_row(
        &self,
        hit: &crate::library::SearchHit,
        thumb_width: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = self.palette.clone();
        let id = hit.id;
        let Some(item) = self.library.get(id) else {
            return div().into_any_element();
        };
        let open: Activate = Rc::new(move |this, _window, cx| this.open_watch(id, cx));
        let creator = item.creator.clone();
        let mut badges = h_flex().gap(px(6.0));
        if !item.subtitles.is_empty() {
            badges = badges.child(self.badge("Subtitles"));
        }
        if !item.playable {
            badges = badges.child(self.badge("Format not decoded"));
        }
        if item.height >= 1080 {
            badges = badges.child(self.badge("HD"));
        }
        let snippet = hit.snippet.as_ref().map(|snippet| {
            format!(
                "{} - {}",
                format_duration(snippet.start_ms),
                snippet.text.replace('\n', " ")
            )
        });
        h_flex()
            .id(("result", id))
            .items_start()
            .gap(px(16.0))
            .w_full()
            .child(self.activatable(
                self.thumbnail(id, thumb_width, thumbnail_height(thumb_width), 12.0, cx),
                !self.drawer_open,
                cx,
                open.clone(),
            ))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(4.0))
                    .child(self.activatable(
                        div().id(("result-title", id)).line_clamp(2).child(text(
                            item.title.clone(),
                            18.0,
                            26.0,
                            FontWeight::NORMAL,
                            palette.text,
                        )),
                        !self.drawer_open,
                        cx,
                        open,
                    ))
                    .child(text(
                        self.meta_line(id),
                        12.0,
                        18.0,
                        FontWeight::NORMAL,
                        palette.text_muted,
                    ))
                    .child(
                        h_flex()
                            .gap(px(8.0))
                            .py(px(4.0))
                            .child(self.avatar(&creator, 24.0))
                            .child(text(
                                creator,
                                12.0,
                                18.0,
                                FontWeight::NORMAL,
                                palette.text_muted,
                            )),
                    )
                    .when_some(snippet, |element, snippet| {
                        element.child(div().line_clamp(2).child(text(
                            snippet,
                            12.0,
                            18.0,
                            FontWeight::NORMAL,
                            palette.text_muted,
                        )))
                    })
                    .child(badges),
            )
            .into_any_element()
    }

    fn badge(&self, label: &str) -> gpui::Div {
        div()
            .px(px(6.0))
            .py(px(1.0))
            .rounded(px(4.0))
            .bg(self.palette.hover)
            .child(text(
                label.to_owned(),
                12.0,
                18.0,
                FontWeight::MEDIUM,
                self.palette.text_muted,
            ))
    }

    fn channel_row(
        &self,
        name: &str,
        videos: usize,
        shorts: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = self.palette.clone();
        let open_name = name.to_owned();
        let follow_name = name.to_owned();
        let following = self.followed.contains(name);
        let pill = h_flex()
            .id("channel-follow")
            .h(px(36.0))
            .px(px(16.0))
            .rounded_full()
            .bg(if following {
                palette.hover
            } else {
                palette.inverse_fill
            })
            .child(text(
                if following { "Following" } else { "Follow" },
                14.0,
                20.0,
                FontWeight::MEDIUM,
                if following {
                    palette.text
                } else {
                    palette.inverse_text
                },
            ));
        h_flex()
            .id("channel-row")
            .gap(px(24.0))
            .py(px(8.0))
            .child(self.activatable(
                div().id("channel-avatar").child(self.avatar(name, 136.0)),
                !self.drawer_open,
                cx,
                Rc::new(move |this, _window, cx| {
                    this.navigate(Route::Creator(open_name.clone()), cx);
                }),
            ))
            .child(
                v_flex()
                    .gap(px(4.0))
                    .child(text(
                        name.to_owned(),
                        18.0,
                        26.0,
                        FontWeight::MEDIUM,
                        palette.text,
                    ))
                    .child(text(
                        format!(
                            "{} - {}",
                            format_count(videos as u64, "video"),
                            format_count(shorts as u64, "short-form clip")
                        ),
                        12.0,
                        18.0,
                        FontWeight::NORMAL,
                        palette.text_muted,
                    ))
                    .child(div().mt(px(8.0)).child(self.activatable(
                        pill,
                        !self.drawer_open,
                        cx,
                        Rc::new(move |this, _window, cx| this.toggle_follow(&follow_name, cx)),
                    ))),
            )
            .into_any_element()
    }

    /// Folder "mix": the newest entries of a creator folder as one stacked row.
    fn mix_row(&self, name: &str, thumb_width: f32, cx: &mut Context<Self>) -> AnyElement {
        let palette = self.palette.clone();
        let ids = self.library.videos_by(name);
        let Some(first) = ids.first().copied() else {
            return div().into_any_element();
        };
        let target = name.to_owned();
        let mut lines = v_flex().flex_1().min_w_0().gap(px(2.0)).child(text(
            format!("Mix - {name}"),
            18.0,
            26.0,
            FontWeight::NORMAL,
            palette.text,
        ));
        for id in ids.iter().take(3) {
            if let Some(item) = self.library.get(*id) {
                lines = lines.child(div().truncate().child(text(
                    item.title.clone(),
                    12.0,
                    18.0,
                    FontWeight::NORMAL,
                    palette.text_muted,
                )));
            }
        }
        let stack = div()
            .relative()
            .flex_none()
            .w(px(thumb_width))
            .h(px(thumbnail_height(thumb_width) + 12.0))
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left(px(12.0))
                    .w(px(thumb_width - 24.0))
                    .h(px(8.0))
                    .rounded_t(px(6.0))
                    .bg(palette.surface_high),
            )
            .child(
                div()
                    .absolute()
                    .top(px(8.0))
                    .left(px(6.0))
                    .w(px(thumb_width - 12.0))
                    .h(px(8.0))
                    .rounded_t(px(8.0))
                    .bg(palette.hover),
            )
            .child(
                div()
                    .absolute()
                    .top(px(12.0))
                    .left_0()
                    .child(self.thumbnail(
                        first,
                        thumb_width,
                        thumbnail_height(thumb_width),
                        12.0,
                        cx,
                    )),
            )
            .child(
                div()
                    .absolute()
                    .right(px(8.0))
                    .bottom(px(8.0))
                    .px(px(4.0))
                    .rounded(px(4.0))
                    .bg(black().opacity(0.6))
                    .child(text(
                        format_count(ids.len() as u64, "video"),
                        12.0,
                        18.0,
                        FontWeight::MEDIUM,
                        white(),
                    )),
            );
        self.activatable(
            h_flex()
                .id("mix-row")
                .items_start()
                .gap(px(16.0))
                .w_full()
                .child(stack)
                .child(lines),
            !self.drawer_open,
            cx,
            Rc::new(move |this, _window, cx| this.navigate(Route::Creator(target.clone()), cx)),
        )
        .into_any_element()
    }

    fn shorts_shelf(&self, ids: &[usize], cx: &mut Context<Self>) -> AnyElement {
        let palette = self.palette.clone();
        let mut row = h_flex().gap(px(12.0)).items_start();
        for id in ids.iter().take(6) {
            let id = *id;
            let Some(item) = self.library.get(id) else {
                continue;
            };
            let position = self
                .library
                .shorts()
                .iter()
                .position(|short| *short == id)
                .unwrap_or(0);
            let card = v_flex()
                .id(("shelf", id))
                .w(px(180.0))
                .gap(px(6.0))
                .child(self.thumbnail(id, 180.0, 320.0, 12.0, cx))
                .child(div().line_clamp(2).child(text(
                    item.title.clone(),
                    14.0,
                    20.0,
                    FontWeight::MEDIUM,
                    palette.text,
                )));
            row = row.child(self.activatable(
                card,
                !self.drawer_open,
                cx,
                Rc::new(move |this, _window, cx| {
                    this.shorts_cursor = position;
                    this.navigate(Route::Shorts, cx);
                }),
            ));
        }
        v_flex()
            .gap(px(8.0))
            .py(px(8.0))
            .child(text(
                "Short-form",
                20.0,
                28.0,
                FontWeight::BOLD,
                palette.text,
            ))
            .child(row)
            .into_any_element()
    }

    // ---- creator -----------------------------------------------------------------------------------

    pub(crate) fn render_creator(
        &mut self,
        shell: &ShellLayout,
        name: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = self.palette.clone();
        let Some(creator) = self.library.creator(name) else {
            return self.unavailable_state(cx);
        };
        let narrow = shell.viewport_width < 500.0;
        let content_width = shell.content_width.min(CREATOR_MAX_WIDTH);
        let banner_height = layout::creator_banner_height(shell.viewport_width);
        let avatar_size = layout::creator_avatar_size(shell.viewport_width);
        let following = self.followed.contains(name);
        let follow_name = name.to_owned();
        let latest = relative_age(creator.newest_secs, unix_now());
        let follow = h_flex()
            .id("creator-follow")
            .h(px(36.0))
            .px(px(16.0))
            .justify_center()
            .rounded_full()
            .when(narrow, |element| element.w_full())
            .bg(if following {
                palette.hover
            } else {
                palette.inverse_fill
            })
            .child(text(
                if following { "Following" } else { "Follow" },
                14.0,
                20.0,
                FontWeight::MEDIUM,
                if following {
                    palette.text
                } else {
                    palette.inverse_text
                },
            ));
        let follow = self.activatable(
            follow,
            !self.drawer_open,
            cx,
            Rc::new(move |this, _window, cx| this.toggle_follow(&follow_name, cx)),
        );

        let (inline_follow, wrapped_follow) = if narrow {
            (None, Some(follow))
        } else {
            (Some(follow), None)
        };
        let banner = div()
            .w_full()
            .h(px(banner_height))
            .rounded(px(16.0))
            .bg(linear_gradient(
                90.0,
                linear_color_stop(self.theme.colors.primary_container, 0.0),
                linear_color_stop(self.theme.colors.tertiary.opacity(0.7), 1.0),
            ));
        let header = h_flex()
            .items_start()
            .gap(px(if narrow { 12.0 } else { 24.0 }))
            .py(px(16.0))
            .flex_wrap()
            .child(self.avatar(name, avatar_size))
            .child(
                v_flex()
                    .gap(px(6.0))
                    .flex_1()
                    .min_w_0()
                    .items_start()
                    .child(text(
                        name.to_owned(),
                        if narrow { 24.0 } else { 36.0 },
                        if narrow { 32.0 } else { 44.0 },
                        FontWeight::BOLD,
                        palette.text,
                    ))
                    .child(text(
                        format!(
                            "{} - {} - latest {latest}",
                            format_count(creator.video_count as u64, "video"),
                            format_count(creator.short_count as u64, "short-form clip"),
                        ),
                        14.0,
                        20.0,
                        FontWeight::NORMAL,
                        palette.text_muted,
                    ))
                    .child(div().truncate().child(text(
                        format!("Folder {name}"),
                        14.0,
                        20.0,
                        FontWeight::NORMAL,
                        palette.text_muted,
                    )))
                    .when_some(inline_follow, |element, follow| {
                        element.child(div().mt(px(4.0)).child(follow))
                    }),
            )
            .when_some(wrapped_follow, |element, follow| {
                element.child(div().w_full().child(follow))
            });

        let mut tabs = h_flex()
            .h(px(48.0))
            .gap(px(24.0))
            .border_b_1()
            .border_color(palette.outline);
        for (index, (tab, label)) in [
            (CreatorTab::Home, "Home"),
            (CreatorTab::Videos, "Videos"),
            (CreatorTab::Shorts, "Short-form"),
        ]
        .into_iter()
        .enumerate()
        {
            let selected = self.creator_tab == tab;
            let tab_element = v_flex()
                .id(("creator-tab", index))
                .h_full()
                .justify_center()
                .border_b_2()
                .border_color(if selected {
                    palette.text
                } else {
                    gpui::transparent_black()
                })
                .child(text(
                    label,
                    14.0,
                    20.0,
                    FontWeight::MEDIUM,
                    if selected {
                        palette.text
                    } else {
                        palette.text_muted
                    },
                ));
            tabs = tabs.child(self.activatable(
                tab_element,
                !self.drawer_open,
                cx,
                Rc::new(move |this, _window, cx| {
                    this.creator_tab = tab;
                    this.batches = 1;
                    cx.notify();
                }),
            ));
        }

        let body = if self.skeleton || self.library_loading {
            self.skeleton_grid(shell, false)
        } else {
            let mut ids = match self.creator_tab {
                CreatorTab::Shorts => self.library.shorts_by(name),
                _ => self.library.videos_by(name),
            };
            match self.creator_sort {
                CreatorSort::Latest => {}
                CreatorSort::Oldest => ids.reverse(),
                CreatorSort::Popular => ids
                    .sort_by_key(|id| std::cmp::Reverse(self.plays.get(id).copied().unwrap_or(0))),
            }
            if self.creator_tab == CreatorTab::Home {
                ids.truncate(3);
            }
            let sort_chips: Vec<ChipSpec> = [
                (CreatorSort::Latest, "Latest"),
                (CreatorSort::Popular, "Popular"),
                (CreatorSort::Oldest, "Oldest"),
            ]
            .into_iter()
            .map(|(sort, label)| ChipSpec {
                label: label.into(),
                selected: self.creator_sort == sort,
                on_select: Rc::new(move |this, _window, cx| {
                    this.creator_sort = sort;
                    cx.notify();
                }),
            })
            .collect();
            let grid = if self.creator_tab == CreatorTab::Shorts {
                let mut row = h_flex().flex_wrap().gap(px(GRID_GAP)).items_start();
                for id in &ids {
                    let id = *id;
                    let position = self
                        .library
                        .shorts()
                        .iter()
                        .position(|short| *short == id)
                        .unwrap_or(0);
                    row = row.child(
                        self.activatable(
                            div()
                                .id(("creator-short", id))
                                .child(self.thumbnail(id, 180.0, 320.0, 12.0, cx)),
                            !self.drawer_open,
                            cx,
                            Rc::new(move |this, _window, cx| {
                                this.shorts_cursor = position;
                                this.navigate(Route::Shorts, cx);
                            }),
                        ),
                    );
                }
                row.into_any_element()
            } else if ids.is_empty() {
                text(
                    "No videos here yet.",
                    14.0,
                    20.0,
                    FontWeight::NORMAL,
                    palette.text_muted,
                )
                .into_any_element()
            } else {
                self.grid_of(&ids, shell, content_width, false, cx)
            };
            v_flex()
                .gap(px(8.0))
                .child(if self.creator_tab == CreatorTab::Videos {
                    div()
                        .ml(px(-PAGE_PADDING))
                        .child(self.render_chip_bar(sort_chips, cx))
                        .into_any_element()
                } else {
                    div().into_any_element()
                })
                .child(grid)
                .into_any_element()
        };

        div()
            .id("creator-page")
            .size_full()
            .overflow_y_scroll()
            .px(px(if narrow { 8.0 } else { PAGE_PADDING }))
            .child(
                v_flex()
                    .w_full()
                    .max_w(px(CREATOR_MAX_WIDTH))
                    .pt(px(8.0))
                    .pb(px(32.0))
                    .child(banner)
                    .child(header)
                    .child(tabs)
                    .child(div().pt(px(16.0)).child(body)),
            )
            .into_any_element()
    }

    // ---- short-form viewer -----------------------------------------------------------------------

    pub(crate) fn render_shorts(
        &mut self,
        shell: &ShellLayout,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = self.palette.clone();
        let shorts = self.library.shorts();
        let geometry =
            layout::shorts_layout(shell.viewport_width, shell.viewport_height, shell.nav);
        if !self.library_loading && shorts.is_empty() {
            return self.empty_state(
                "No short-form clips",
                "Portrait videos (taller than wide) in the library show up here.",
                None,
                cx,
            );
        }
        let Some(id) = shorts.get(self.shorts_cursor).copied() else {
            return self.skeleton_player(geometry.player_width, geometry.player_height);
        };
        let Some(item) = self.library.get(id).cloned() else {
            return div().into_any_element();
        };
        let player_height = if self.library_loading || self.skeleton {
            geometry.player_height
        } else {
            geometry.player_height
        };
        let surface = if self.skeleton || self.library_loading {
            self.skeleton_player(geometry.player_width, player_height)
        } else {
            self.player_surface(id, geometry.player_width, player_height, 12.0, true, cx)
        };
        let has_previous = self.shorts_cursor > 0;
        let has_next = self.shorts_cursor + 1 < shorts.len();
        let liked = self.likes.contains(&id);
        let following = self.followed.contains(&item.creator);
        let creator = item.creator.clone();
        let path = item.path.clone();

        let rail_button = |this: &Self,
                           button_id: &'static str,
                           icon: IconName,
                           label: String,
                           active: bool,
                           cx: &mut Context<Self>,
                           on_activate: Activate|
         -> AnyElement {
            let circle = h_flex()
                .id(button_id)
                .w(px(48.0))
                .h(px(48.0))
                .justify_center()
                .rounded_full()
                .bg(if active {
                    this.palette.inverse_fill
                } else {
                    this.palette.hover
                })
                .child(glyph(
                    icon,
                    22.0,
                    if active {
                        this.palette.inverse_text
                    } else {
                        this.palette.text
                    },
                ));
            v_flex()
                .items_center()
                .h(px(78.0))
                .gap(px(2.0))
                .child(this.activatable(circle, !this.drawer_open, cx, on_activate))
                .child(text(
                    label,
                    12.0,
                    18.0,
                    FontWeight::NORMAL,
                    this.palette.text,
                ))
                .into_any_element()
        };
        let like_id = id;
        let rail = v_flex()
            .child(rail_button(
                self,
                "short-like",
                IconName::ThumbsUp,
                if liked {
                    "1".to_owned()
                } else {
                    "0".to_owned()
                },
                liked,
                cx,
                Rc::new(move |this, _window, cx| {
                    if !this.likes.remove(&like_id) {
                        this.likes.insert(like_id);
                    }
                    cx.notify();
                }),
            ))
            .child(rail_button(
                self,
                "short-follow",
                IconName::UserCheck,
                if following {
                    "Following".to_owned()
                } else {
                    "Follow".to_owned()
                },
                following,
                cx,
                Rc::new({
                    let creator = creator.clone();
                    move |this, _window, cx| this.toggle_follow(&creator, cx)
                }),
            ))
            .child(rail_button(
                self,
                "short-share",
                IconName::Share,
                "Copy path".to_owned(),
                false,
                cx,
                Rc::new(move |_this, _window, cx| cx.write_to_clipboard(clipboard_path(&path))),
            ));

        let metadata = h_flex()
            .gap(px(10.0))
            .items_center()
            .child(self.avatar(&creator, 32.0))
            .child(
                v_flex()
                    .min_w_0()
                    .child(text(
                        creator.clone(),
                        14.0,
                        20.0,
                        FontWeight::MEDIUM,
                        palette.text,
                    ))
                    .child(div().truncate().child(text(
                        item.title,
                        16.0,
                        22.0,
                        FontWeight::NORMAL,
                        palette.text,
                    ))),
            );

        let nav_button = |this: &Self,
                          button_id: &'static str,
                          icon: IconName,
                          direction: isize,
                          cx: &mut Context<Self>|
         -> AnyElement {
            let element = h_flex()
                .id(button_id)
                .w(px(56.0))
                .h(px(56.0))
                .justify_center()
                .rounded_full()
                .bg(this.palette.hover)
                .child(glyph(icon, 24.0, this.palette.text));
            this.activatable(
                element,
                !this.drawer_open,
                cx,
                Rc::new(move |this, _window, cx| this.step_short(direction, cx)),
            )
            .into_any_element()
        };

        let mut stage = div()
            .id("shorts-stage")
            .relative()
            .w(px(geometry.container_width))
            .h(px(geometry.container_height))
            .mt(px(if geometry.narrow { 0.0 } else { 8.0 }))
            .on_scroll(cx.listener(|this, event: &ScrollEvent, _window, cx| {
                let delta = f32::from(event.delta.pixel_delta(px(20.0)).y);
                if delta.abs() > 1.0 {
                    this.wheel_short(if delta < 0.0 { 1 } else { -1 }, cx);
                }
            }))
            .child(
                div()
                    .absolute()
                    .left(px(geometry.player_x))
                    .top_0()
                    .child(surface),
            );
        if geometry.rail_overlay {
            stage = stage
                .child(div().absolute().right(px(8.0)).bottom(px(96.0)).child(rail))
                .child(
                    div()
                        .absolute()
                        .left(px(12.0))
                        .bottom(px(16.0))
                        .w(px(geometry.player_width * 0.7))
                        .child(metadata),
                );
        } else {
            stage = stage
                .child(
                    div()
                        .absolute()
                        .left(px(geometry.player_x + geometry.player_width + 12.0))
                        .bottom_0()
                        .child(rail),
                )
                .child(
                    div()
                        .absolute()
                        .left(px((geometry.player_x - 394.0).max(8.0)))
                        .bottom(px(54.0))
                        .w(px(378.0))
                        .child(metadata),
                );
        }
        if geometry.nav_buttons {
            let mut buttons = v_flex()
                .absolute()
                .right(px(16.0))
                .top(px(geometry.container_height / 2.0 - 60.0))
                .gap(px(8.0));
            if has_previous {
                buttons = buttons.child(nav_button(
                    self,
                    "short-previous",
                    IconName::ArrowUp,
                    -1,
                    cx,
                ));
            }
            if has_next {
                buttons = buttons.child(nav_button(self, "short-next", IconName::ArrowDown, 1, cx));
            }
            stage = stage.child(buttons);
        }
        h_flex()
            .size_full()
            .justify_center()
            .items_start()
            .child(stage)
            .into_any_element()
    }

    pub(crate) fn skeleton_player(&self, width: f32, height: f32) -> AnyElement {
        div()
            .w(px(width))
            .h(px(height))
            .flex_none()
            .rounded(px(12.0))
            .bg(black())
            .child(h_flex().size_full().justify_center().child(glyph(
                IconName::PlayFilled,
                36.0,
                white().opacity(0.25),
            )))
            .into_any_element()
    }
}
