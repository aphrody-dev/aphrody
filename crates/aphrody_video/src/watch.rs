//! Watch view, player surface with controls, related list and miniplayer.

use std::rc::Rc;

use gpui::{
    AnyElement, ClickEvent, ClipboardItem, Context, FontWeight, InteractiveElement as _,
    MouseButton, MouseMoveEvent, ParentElement as _, Styled as _, black, canvas, div, img,
    linear_color_stop, linear_gradient, prelude::*, px, transparent_black, white,
};
use icons::IconName;

use crate::app::{Route, SettingsPanel, ShellLayout, VideoApp};
use crate::layout::{
    self, CONTROL_BAR_HEIGHT, MINIPLAYER_MARGIN, MINIPLAYER_STRIP_HEIGHT, MINIPLAYER_VIDEO_HEIGHT,
    MINIPLAYER_WIDTH, PAGE_PADDING, thumbnail_height,
};
use crate::library::{VideoItem, format_count, format_duration, relative_age, unix_now};
use crate::player::{CenterIcon, SPEEDS, active_cue};
use crate::ui::{Activate, ChipSpec, glyph, h_flex, text, v_flex};

const RELATED_LIMIT: usize = 30;

impl VideoApp {
    pub(crate) fn render_watch(
        &mut self,
        shell: &ShellLayout,
        id: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(item) = self.library.get(id).cloned() else {
            return self.unavailable_state(cx);
        };
        if !self.library_loading && !item.path.is_file() {
            return self.unavailable_state(cx);
        }
        let watch = layout::watch_layout(
            shell.viewport_width,
            shell.viewport_height,
            item.aspect(),
            self.player.theater,
        );
        let player = if self.skeleton {
            self.skeleton_player(watch.player_width, watch.player_height)
        } else {
            self.player_surface(
                id,
                watch.player_width,
                watch.player_height,
                if watch.theater { 0.0 } else { 12.0 },
                false,
                cx,
            )
        };
        let info_width = if watch.two_column {
            shell.viewport_width - 32.0 - watch.secondary_width
        } else {
            shell.viewport_width - 2.0 * PAGE_PADDING
        };
        let info = self.watch_info(&item, info_width, cx);
        let related = self.related_column(id, &item, watch.secondary_width, cx);
        let content = if watch.two_column && watch.theater {
            v_flex().w_full().child(player).child(
                h_flex()
                    .items_start()
                    .gap(px(16.0))
                    .px(px(16.0))
                    .pt(px(12.0))
                    .child(info)
                    .child(related),
            )
        } else if watch.two_column {
            h_flex()
                .w_full()
                .items_start()
                .gap(px(16.0))
                .px(px(16.0))
                .pt(px(12.0))
                .child(
                    v_flex()
                        .w(px(info_width))
                        .flex_none()
                        .gap(px(12.0))
                        .child(player)
                        .child(info),
                )
                .child(related)
        } else {
            v_flex()
                .w_full()
                .px(px(PAGE_PADDING))
                .pt(px(12.0))
                .gap(px(12.0))
                .child(player)
                .child(info)
                .child(related)
        };
        div()
            .id("watch-page")
            .size_full()
            .overflow_y_scroll()
            .child(content.pb(px(32.0)))
            .into_any_element()
    }

    fn pill_button(
        &self,
        id: &'static str,
        icon: IconName,
        label: Option<String>,
        active: bool,
        cx: &mut Context<Self>,
        on_activate: Activate,
    ) -> AnyElement {
        let palette = self.palette.clone();
        let element = h_flex()
            .id(id)
            .h(px(36.0))
            .px(px(14.0))
            .gap(px(6.0))
            .rounded_full()
            .bg(if active {
                palette.inverse_fill
            } else {
                palette.hover
            })
            .child(glyph(
                icon,
                18.0,
                if active {
                    palette.inverse_text
                } else {
                    palette.text
                },
            ))
            .when_some(label, |element, label| {
                element.child(text(
                    label,
                    14.0,
                    20.0,
                    FontWeight::MEDIUM,
                    if active {
                        palette.inverse_text
                    } else {
                        palette.text
                    },
                ))
            });
        self.activatable(element, !self.drawer_open, cx, on_activate)
            .into_any_element()
    }

    fn watch_info(&self, item: &VideoItem, width: f32, cx: &mut Context<Self>) -> AnyElement {
        let palette = self.palette.clone();
        let id = item.id;
        let creator = item.creator.clone();
        let following = self.followed.contains(&creator);
        let liked = self.likes.contains(&id);
        let disliked = self.dislikes.contains(&id);
        let path = item.path.clone();

        let actions = h_flex()
            .gap(px(8.0))
            .child(self.pill_button(
                "watch-like",
                IconName::ThumbsUp,
                Some(if liked { "1".into() } else { "Like".into() }),
                liked,
                cx,
                Rc::new(move |this, _w, cx| {
                    this.dislikes.remove(&id);
                    if !this.likes.remove(&id) {
                        this.likes.insert(id);
                    }
                    cx.notify();
                }),
            ))
            .child(self.pill_button(
                "watch-dislike",
                IconName::ThumbsDown,
                None,
                disliked,
                cx,
                Rc::new(move |this, _w, cx| {
                    this.likes.remove(&id);
                    if !this.dislikes.remove(&id) {
                        this.dislikes.insert(id);
                    }
                    cx.notify();
                }),
            ))
            .child(self.pill_button(
                "watch-share",
                IconName::Share,
                Some("Copy path".into()),
                false,
                cx,
                Rc::new(move |_this, _w, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(path.display().to_string()));
                }),
            ));

        let follow_target = creator.clone();
        let follow = self.pill_button(
            "watch-follow",
            IconName::UserCheck,
            Some(if following {
                "Following".into()
            } else {
                "Follow".into()
            }),
            following,
            cx,
            Rc::new(move |this, _w, cx| this.toggle_follow(&follow_target, cx)),
        );
        let owner_target = creator.clone();
        let owner_count = self
            .library
            .creator(&creator)
            .map_or(0, |creator| creator.video_count);
        let owner = h_flex()
            .gap(px(12.0))
            .child(
                self.activatable(
                    h_flex()
                        .id("watch-owner")
                        .gap(px(12.0))
                        .child(self.avatar(&creator, 40.0))
                        .child(
                            v_flex()
                                .child(text(
                                    creator.clone(),
                                    16.0,
                                    22.0,
                                    FontWeight::MEDIUM,
                                    palette.text,
                                ))
                                .child(text(
                                    format_count(owner_count as u64, "video"),
                                    12.0,
                                    18.0,
                                    FontWeight::NORMAL,
                                    palette.text_muted,
                                )),
                        ),
                    !self.drawer_open,
                    cx,
                    Rc::new(move |this, _w, cx| {
                        this.navigate(Route::Creator(owner_target.clone()), cx)
                    }),
                ),
            )
            .child(follow);

        let facts = format!(
            "{} - {}x{} - {:.1} MB - {}",
            item.path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(""),
            item.width,
            item.height,
            item.size_bytes as f64 / 1_048_576.0,
            if item.playable {
                "H.264"
            } else {
                "not decoded by the built-in player"
            },
        );
        let body = if item.description.is_empty() {
            facts
        } else {
            format!("{}\n{facts}", item.description)
        };
        let expanded = self.description_expanded;
        let description = self.activatable(
            v_flex()
                .id("description")
                .w_full()
                .gap(px(4.0))
                .p(px(12.0))
                .rounded(px(12.0))
                .bg(palette.surface)
                .child(text(
                    format!(
                        "{} - {}",
                        format_count(self.plays.get(&id).copied().unwrap_or(0), "play"),
                        relative_age(item.modified_secs, unix_now())
                    ),
                    14.0,
                    20.0,
                    FontWeight::MEDIUM,
                    palette.text,
                ))
                .child(
                    div()
                        .when(!expanded, |element| element.line_clamp(3))
                        .child(text(body, 14.0, 20.0, FontWeight::NORMAL, palette.text)),
                )
                .child(text(
                    if expanded { "Show less" } else { "...more" },
                    14.0,
                    20.0,
                    FontWeight::MEDIUM,
                    palette.text,
                )),
            !self.drawer_open,
            cx,
            Rc::new(|this, _w, cx| {
                this.description_expanded = !this.description_expanded;
                cx.notify();
            }),
        );

        let mut column = v_flex()
            .w(px(width))
            .flex_none()
            .gap(px(12.0))
            .child(div().line_clamp(2).child(text(
                item.title.clone(),
                20.0,
                28.0,
                FontWeight::BOLD,
                palette.text,
            )))
            .child(
                h_flex()
                    .justify_between()
                    .flex_wrap()
                    .gap(px(12.0))
                    .child(owner)
                    .child(actions),
            )
            .child(description);
        if !item.subtitles.is_empty() {
            column = column.child(self.transcript(item, cx));
        }
        column.into_any_element()
    }

    /// Subtitle cues as a clickable transcript that seeks the player.
    fn transcript(&self, item: &VideoItem, cx: &mut Context<Self>) -> AnyElement {
        let palette = self.palette.clone();
        let mut list = v_flex().gap(px(2.0)).child(text(
            "Transcript",
            16.0,
            22.0,
            FontWeight::BOLD,
            palette.text,
        ));
        for (index, cue) in item.subtitles.iter().take(60).enumerate() {
            let start = cue.start_ms;
            let row = h_flex()
                .id(("cue", index))
                .gap(px(12.0))
                .py(px(4.0))
                .px(px(8.0))
                .rounded(px(8.0))
                .hover({
                    let hover = palette.hover;
                    move |style| style.bg(hover)
                })
                .child(text(
                    format_duration(start),
                    12.0,
                    18.0,
                    FontWeight::MEDIUM,
                    palette.link,
                ))
                .child(div().flex_1().child(text(
                    cue.text.replace('\n', " "),
                    14.0,
                    20.0,
                    FontWeight::NORMAL,
                    palette.text,
                )));
            list = list.child(self.activatable(
                row,
                !self.drawer_open,
                cx,
                Rc::new(move |this, _w, cx| this.seek_to(start, cx)),
            ));
        }
        list.into_any_element()
    }

    fn related_column(
        &self,
        id: usize,
        item: &VideoItem,
        width: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = self.palette.clone();
        let creator = item.creator.clone();
        let chips = vec![
            ChipSpec {
                label: "All".into(),
                selected: !self.related_same_creator,
                on_select: Rc::new(|this, _w, cx| {
                    this.related_same_creator = false;
                    cx.notify();
                }),
            },
            ChipSpec {
                label: format!("From {creator}").into(),
                selected: self.related_same_creator,
                on_select: Rc::new(|this, _w, cx| {
                    this.related_same_creator = true;
                    cx.notify();
                }),
            },
        ];
        let mut ids = self.library.related(id, RELATED_LIMIT);
        if self.related_same_creator {
            ids.retain(|other| {
                self.library
                    .get(*other)
                    .is_some_and(|candidate| candidate.creator == creator)
            });
        }
        let thumb_width = (width * 0.6).min(242.0);
        let mut chip_row = h_flex().gap(px(8.0));
        for (index, chip) in chips.into_iter().enumerate() {
            let element = h_flex()
                .id(("related-chip", index))
                .h(px(32.0))
                .px(px(12.0))
                .rounded(px(8.0))
                .bg(if chip.selected {
                    palette.inverse_fill
                } else {
                    palette.hover
                })
                .child(text(
                    chip.label,
                    14.0,
                    20.0,
                    FontWeight::MEDIUM,
                    if chip.selected {
                        palette.inverse_text
                    } else {
                        palette.text
                    },
                ));
            chip_row =
                chip_row.child(self.activatable(element, !self.drawer_open, cx, chip.on_select));
        }
        let mut list = v_flex()
            .w(px(width))
            .flex_none()
            .gap(px(8.0))
            .child(chip_row.pb(px(4.0)));
        if ids.is_empty() {
            list = list.child(text(
                "Nothing else to watch yet.",
                14.0,
                20.0,
                FontWeight::NORMAL,
                palette.text_muted,
            ));
        }
        for other in ids {
            let Some(candidate) = self.library.get(other) else {
                continue;
            };
            let open: Activate = Rc::new(move |this, _w, cx| this.open_watch(other, cx));
            list = list.child(
                h_flex()
                    .items_start()
                    .gap(px(8.0))
                    .child(self.activatable(
                        self.thumbnail(other, thumb_width, thumbnail_height(thumb_width), 8.0, cx),
                        !self.drawer_open,
                        cx,
                        open.clone(),
                    ))
                    .child(
                        v_flex().flex_1().min_w_0().child(
                            self.activatable(
                                v_flex()
                                    .id(("related-title", other))
                                    .w_full()
                                    .min_w_0()
                                    .child(div().line_clamp(2).child(text(
                                        candidate.title.clone(),
                                        14.0,
                                        20.0,
                                        FontWeight::MEDIUM,
                                        palette.text,
                                    )))
                                    .child(text(
                                        candidate.creator.clone(),
                                        12.0,
                                        18.0,
                                        FontWeight::NORMAL,
                                        palette.text_muted,
                                    ))
                                    .child(text(
                                        self.meta_line(other),
                                        12.0,
                                        18.0,
                                        FontWeight::NORMAL,
                                        palette.text_muted,
                                    )),
                                !self.drawer_open,
                                cx,
                                open,
                            ),
                        ),
                    ),
            );
        }
        list.into_any_element()
    }

    // ---- player surface ----------------------------------------------------------------------------

    fn center_glyph(icon: CenterIcon) -> IconName {
        match icon {
            CenterIcon::Play => IconName::PlayFilled,
            CenterIcon::Pause => IconName::DebugPause,
            CenterIcon::VolumeUp | CenterIcon::VolumeDown | CenterIcon::Unmuted => {
                IconName::AudioOn
            }
            CenterIcon::Muted => IconName::AudioOff,
            CenterIcon::SeekForward => IconName::ForwardArrow,
            CenterIcon::SeekBack => IconName::ArrowLeft,
        }
    }

    fn control_button(
        &self,
        id: &'static str,
        icon: IconName,
        size: f32,
        cx: &mut Context<Self>,
        on_activate: Activate,
    ) -> AnyElement {
        let element = h_flex()
            .id(id)
            .w(px(size))
            .h(px(40.0))
            .flex_none()
            .justify_center()
            .rounded_full()
            .hover(|style| style.bg(white().opacity(0.15)))
            .child(glyph(icon, 22.0, white()));
        self.activatable(element, !self.drawer_open, cx, on_activate)
            .into_any_element()
    }

    fn control_label_button(
        &self,
        id: &'static str,
        label: &'static str,
        active: bool,
        cx: &mut Context<Self>,
        on_activate: Activate,
    ) -> AnyElement {
        let element = h_flex()
            .id(id)
            .w(px(48.0))
            .h(px(40.0))
            .flex_none()
            .justify_center()
            .child(
                div()
                    .px(px(4.0))
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(white().opacity(if active { 1.0 } else { 0.5 }))
                    .bg(if active {
                        white().opacity(0.25)
                    } else {
                        transparent_black()
                    })
                    .child(text(label, 12.0, 16.0, FontWeight::BOLD, white())),
            );
        self.activatable(element, !self.drawer_open, cx, on_activate)
            .into_any_element()
    }

    fn scrubber(&self, id: usize, cx: &mut Context<Self>) -> AnyElement {
        let palette = self.palette.clone();
        let bounds_cell = self.scrubber_bounds.clone();
        let hovered = self.scrub_hover.is_some() || self.player.scrubbing;
        let track_height = if hovered { 6.0 } else { 4.0 };
        let progress = self.player.progress();
        let duration = self.player.state.duration_ms.max(1) as f32;
        let loaded = self.loaded_fraction(duration).max(progress);
        let width = f32::from(self.scrubber_bounds.get().size.width);
        let tooltip = self.scrub_hover.map(|fraction| {
            let time = format_duration((fraction * duration) as u64);
            let left = (fraction * width - 80.0).clamp(0.0, (width - 160.0).max(0.0));
            let mut card = v_flex()
                .absolute()
                .left(px(left))
                .bottom(px(18.0))
                .items_center()
                .gap(px(4.0));
            if let Some(poster) = self.posters.get(&id) {
                card = card.child(
                    img(poster.clone())
                        .w(px(160.0))
                        .h(px(90.0))
                        .rounded(px(8.0))
                        .object_fit(gpui::ObjectFit::Cover),
                );
            }
            card.child(
                div()
                    .px(px(6.0))
                    .rounded(px(4.0))
                    .bg(black().opacity(0.7))
                    .child(text(time, 12.0, 18.0, FontWeight::MEDIUM, white())),
            )
        });
        div()
            .id("scrubber")
            .relative()
            .w_full()
            .h(px(12.0))
            .flex_none()
            .cursor_pointer()
            .child(
                canvas(
                    move |bounds, _window, _cx| bounds_cell.set(bounds),
                    |_, (), _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &gpui::MouseDownEvent, _w, cx| {
                    if let Some(fraction) = this.scrubber_fraction(event.position) {
                        this.begin_scrub(fraction, cx);
                    }
                    cx.stop_propagation();
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _w, cx| {
                this.scrub_hover = this.scrubber_fraction(event.position);
                cx.notify();
            }))
            .on_hover(cx.listener(|this, hovered: &bool, _w, cx| {
                if !*hovered {
                    this.scrub_hover = None;
                    cx.notify();
                }
            }))
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px((12.0 - track_height) / 2.0))
                    .h(px(track_height))
                    .rounded_full()
                    .bg(white().opacity(0.25))
                    .child(
                        div()
                            .absolute()
                            .left_0()
                            .top_0()
                            .h_full()
                            .w(gpui::relative(loaded))
                            .rounded_full()
                            .bg(white().opacity(0.4)),
                    )
                    .child(
                        div()
                            .absolute()
                            .left_0()
                            .top_0()
                            .h_full()
                            .w(gpui::relative(progress))
                            .rounded_full()
                            .bg(palette.accent),
                    )
                    .when(hovered, |element| {
                        element.child(
                            div()
                                .absolute()
                                .left(gpui::relative(progress))
                                .top(px(-4.0))
                                .ml(px(-7.0))
                                .w(px(14.0))
                                .h(px(14.0))
                                .rounded_full()
                                .bg(palette.accent),
                        )
                    }),
            )
            .when_some(tooltip, ParentElement::child)
            .into_any_element()
    }

    fn loaded_fraction(&self, duration: f32) -> f32 {
        self.frame_lead_ms()
            .map_or(0.0, |lead| (lead as f32 / duration).clamp(0.0, 1.0))
    }

    fn settings_panel_view(&self, panel: SettingsPanel, cx: &mut Context<Self>) -> AnyElement {
        let row = |this: &Self,
                   id: &'static str,
                   label: String,
                   value: String,
                   cx: &mut Context<Self>,
                   on_activate: Activate|
         -> AnyElement {
            let element = h_flex()
                .id(id)
                .w_full()
                .h(px(40.0))
                .px(px(12.0))
                .justify_between()
                .hover(|style| style.bg(white().opacity(0.12)))
                .child(text(label, 14.0, 20.0, FontWeight::NORMAL, white()))
                .child(text(
                    value,
                    14.0,
                    20.0,
                    FontWeight::NORMAL,
                    white().opacity(0.8),
                ));
            this.activatable(element, !this.drawer_open, cx, on_activate)
                .into_any_element()
        };
        let mut column = v_flex()
            .id("settings-panel")
            .absolute()
            .right(px(12.0))
            .bottom(px(CONTROL_BAR_HEIGHT + 12.0))
            .w(px(250.0))
            .py(px(8.0))
            .rounded(px(12.0))
            .bg(black().opacity(0.85))
            .occlude();
        match panel {
            SettingsPanel::Root => {
                let item = self.watching.and_then(|id| self.library.get(id));
                column = column
                    .child(row(
                        self,
                        "settings-speed",
                        "Playback speed".to_owned(),
                        format!("{}x", self.player.state.speed.0),
                        cx,
                        Rc::new(|this, _w, cx| {
                            this.settings_panel = Some(SettingsPanel::Speed);
                            cx.notify();
                        }),
                    ))
                    .child(row(
                        self,
                        "settings-captions",
                        "Captions".to_owned(),
                        if self.player.captions {
                            "On".to_owned()
                        } else {
                            "Off".to_owned()
                        },
                        cx,
                        Rc::new(|this, _w, cx| {
                            this.player.toggle_captions();
                            cx.notify();
                        }),
                    ))
                    .child(row(
                        self,
                        "settings-autoplay",
                        "Autoplay next".to_owned(),
                        if self.player.autoplay_next {
                            "On".to_owned()
                        } else {
                            "Off".to_owned()
                        },
                        cx,
                        Rc::new(|this, _w, cx| {
                            this.player.toggle_autoplay();
                            cx.notify();
                        }),
                    ))
                    .child(row(
                        self,
                        "settings-quality",
                        "Quality".to_owned(),
                        item.map_or_else(String::new, |item| format!("{}p original", item.height)),
                        cx,
                        Rc::new(|_this, _w, _cx| {}),
                    ));
            }
            SettingsPanel::Speed => {
                column = column.child(row(
                    self,
                    "settings-back",
                    "< Playback speed".to_owned(),
                    String::new(),
                    cx,
                    Rc::new(|this, _w, cx| {
                        this.settings_panel = Some(SettingsPanel::Root);
                        cx.notify();
                    }),
                ));
                for (index, speed) in SPEEDS.into_iter().enumerate() {
                    let current = (self.player.state.speed.0 - speed).abs() < 0.001;
                    column = column.child(row(
                        self,
                        [
                            "speed-0", "speed-1", "speed-2", "speed-3", "speed-4", "speed-5",
                            "speed-6", "speed-7",
                        ][index],
                        format!("{speed}x"),
                        if current {
                            "selected".to_owned()
                        } else {
                            String::new()
                        },
                        cx,
                        Rc::new(move |this, _w, cx| {
                            this.player.state.speed = aphrody_media::PlaybackSpeed(speed);
                            this.settings_panel = None;
                            cx.notify();
                        }),
                    ));
                }
            }
        }
        column.into_any_element()
    }

    fn volume_slider(&self, cx: &mut Context<Self>) -> AnyElement {
        let level = self.player.effective_volume();
        let mut row = h_flex()
            .id("volume-slider")
            .h(px(40.0))
            .gap(px(1.0))
            .w(px(52.0))
            .flex_none();
        for step in 0..10_u32 {
            let filled = (step as f32 + 1.0) / 10.0 <= level + 0.001;
            let segment = div()
                .id(("volume-step", step as usize))
                .h(px(16.0))
                .w(px(4.2))
                .rounded(px(1.0))
                .bg(white().opacity(if filled { 0.95 } else { 0.3 }))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _e: &ClickEvent, _w, cx| {
                    this.player.set_volume((step as f32 + 1.0) / 10.0);
                    cx.notify();
                }));
            row = row.child(segment);
        }
        row.into_any_element()
    }

    fn controls_bar(&self, id: usize, cx: &mut Context<Self>) -> AnyElement {
        let item_height = self.library.get(id).map_or(0, |item| item.height);
        let _ = item_height;
        let player = &self.player;
        let time = format!(
            "{} / {}",
            format_duration(player.state.position_ms),
            format_duration(player.state.duration_ms)
        );
        let playing = player.state.playing;
        let fullscreen = player.fullscreen;
        let theater_outline = div()
            .w(px(22.0))
            .h(px(if player.theater { 10.0 } else { 14.0 }))
            .rounded(px(2.0))
            .border_2()
            .border_color(white());
        let mini_outline = div()
            .relative()
            .w(px(22.0))
            .h(px(14.0))
            .rounded(px(2.0))
            .border_2()
            .border_color(white())
            .child(
                div()
                    .absolute()
                    .right(px(1.0))
                    .bottom(px(1.0))
                    .w(px(8.0))
                    .h(px(5.0))
                    .bg(white()),
            );
        let drawn_button = |this: &Self,
                            button_id: &'static str,
                            drawing: gpui::Div,
                            cx: &mut Context<Self>,
                            on_activate: Activate|
         -> AnyElement {
            let element = h_flex()
                .id(button_id)
                .w(px(48.0))
                .h(px(40.0))
                .flex_none()
                .justify_center()
                .rounded_full()
                .hover(|style| style.bg(white().opacity(0.15)))
                .child(drawing);
            this.activatable(element, !this.drawer_open, cx, on_activate)
                .into_any_element()
        };
        let autoplay_switch = {
            let on = player.autoplay_next;
            let element = h_flex()
                .id("autoplay-switch")
                .w(px(48.0))
                .h(px(40.0))
                .flex_none()
                .justify_center()
                .child(
                    h_flex()
                        .w(px(34.0))
                        .h(px(16.0))
                        .px(px(2.0))
                        .rounded_full()
                        .bg(white().opacity(if on { 0.9 } else { 0.35 }))
                        .when(on, |element| element.justify_end())
                        .child(div().w(px(12.0)).h(px(12.0)).rounded_full().bg(black())),
                );
            self.activatable(
                element,
                !self.drawer_open,
                cx,
                Rc::new(|this, _w, cx| {
                    this.player.toggle_autoplay();
                    cx.notify();
                }),
            )
            .into_any_element()
        };

        let left = h_flex()
            .gap(px(2.0))
            .child(self.control_button(
                "control-play",
                if playing {
                    IconName::DebugPause
                } else {
                    IconName::PlayFilled
                },
                40.0,
                cx,
                Rc::new(|this, _w, cx| this.toggle_play(cx)),
            ))
            .child(self.control_button(
                "control-mute",
                if player.effective_volume() <= 0.0 {
                    IconName::AudioOff
                } else {
                    IconName::AudioOn
                },
                40.0,
                cx,
                Rc::new(|this, _w, cx| {
                    this.player.toggle_mute();
                    cx.notify();
                }),
            ))
            .child(self.volume_slider(cx))
            .child(
                div()
                    .px(px(8.0))
                    .child(text(time, 13.0, 18.0, FontWeight::MEDIUM, white())),
            );

        let right = h_flex()
            .gap(px(0.0))
            .child(autoplay_switch)
            .child(self.control_label_button(
                "control-captions",
                "CC",
                player.captions,
                cx,
                Rc::new(|this, _w, cx| {
                    this.player.toggle_captions();
                    cx.notify();
                }),
            ))
            .child(self.control_button(
                "control-settings",
                IconName::Settings,
                48.0,
                cx,
                Rc::new(|this, _w, cx| {
                    this.settings_panel = if this.settings_panel.is_some() {
                        None
                    } else {
                        Some(SettingsPanel::Root)
                    };
                    cx.notify();
                }),
            ))
            .child(drawn_button(
                self,
                "control-theater",
                theater_outline,
                cx,
                Rc::new(|this, _w, cx| {
                    this.player.toggle_theater();
                    cx.notify();
                }),
            ))
            .child(drawn_button(
                self,
                "control-mini",
                mini_outline,
                cx,
                Rc::new(|this, _w, cx| {
                    if let Some(id) = this.watching {
                        this.player.miniplayer = true;
                        this.navigate_keep_player(id, cx);
                    }
                }),
            ))
            .child(self.control_button(
                "control-fullscreen",
                if fullscreen {
                    IconName::Minimize
                } else {
                    IconName::Maximize
                },
                48.0,
                cx,
                Rc::new(|this, window, cx| {
                    this.player.toggle_fullscreen();
                    window.toggle_fullscreen();
                    cx.notify();
                }),
            ));

        v_flex()
            .id("controls")
            .absolute()
            .left(px(12.0))
            .right(px(12.0))
            .bottom_0()
            .h(px(CONTROL_BAR_HEIGHT))
            .on_mouse_down(MouseButton::Left, |_e, _w, cx| cx.stop_propagation())
            .child(self.scrubber(id, cx))
            .child(
                h_flex()
                    .h(px(44.0))
                    .justify_between()
                    .child(left)
                    .child(right),
            )
            .into_any_element()
    }

    /// The player rectangle: frame, overlays and (in the full variant) the control bar. The
    /// compact variant is the short-form player: top buttons and a thin progress bar.
    pub(crate) fn player_surface(
        &self,
        id: usize,
        width: f32,
        height: f32,
        radius: f32,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = self.palette.clone();
        let active = self.watching == Some(id);
        let image = if active { self.frame.clone() } else { None }
            .or_else(|| self.posters.get(&id).cloned());
        let show_controls = active && self.player.controls_visible && self.playback_error.is_none();
        let cues = self
            .library
            .get(id)
            .map(|item| item.subtitles.clone())
            .unwrap_or_default();
        let caption = if self.player.captions && active {
            active_cue(&cues, self.player.state.position_ms).map(|cue| cue.text.replace('\n', " "))
        } else {
            None
        };
        let mut surface = div()
            .id(("player", id))
            .relative()
            .flex_none()
            .w(px(width))
            .h(px(height))
            .rounded(px(radius))
            .overflow_hidden()
            .bg(black())
            .on_mouse_move(cx.listener(|this, _event: &MouseMoveEvent, _w, cx| {
                this.player.pointer_activity();
                cx.notify();
            }))
            .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                if this.settings_panel.is_some() {
                    this.settings_panel = None;
                } else if event.click_count() >= 2 {
                    this.player.toggle_fullscreen();
                    window.toggle_fullscreen();
                } else {
                    this.toggle_play(cx);
                }
                cx.notify();
            }));
        surface = match image {
            Some(image) => surface.child(
                img(image)
                    .w(px(width))
                    .h(px(height))
                    .object_fit(gpui::ObjectFit::Contain),
            ),
            None => surface.child(h_flex().size_full().justify_center().child(glyph(
                IconName::PlayFilled,
                40.0,
                white().opacity(0.3),
            ))),
        };
        if let Some(error) = self.playback_error.as_ref().filter(|_| active) {
            surface = surface.child(
                v_flex()
                    .absolute()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .gap(px(8.0))
                    .px(px(24.0))
                    .bg(black().opacity(0.7))
                    .child(glyph(IconName::Warning, 32.0, white()))
                    .child(text(error.clone(), 14.0, 20.0, FontWeight::MEDIUM, white())),
            );
        }
        if active && self.player.buffering && self.playback_error.is_none() {
            surface = surface.child(
                h_flex()
                    .absolute()
                    .size_full()
                    .justify_center()
                    .child(glyph(IconName::LoadCircle, 64.0, white().opacity(0.9))),
            );
        }
        if let Some((icon, _)) = self.player.center_icon.filter(|_| active) {
            surface = surface.child(
                h_flex().absolute().size_full().justify_center().child(
                    h_flex()
                        .w(px(64.0))
                        .h(px(64.0))
                        .justify_center()
                        .rounded_full()
                        .bg(black().opacity(0.6))
                        .child(glyph(Self::center_glyph(icon), 32.0, white())),
                ),
            );
        }
        if let Some(caption) = caption {
            surface = surface.child(
                h_flex()
                    .absolute()
                    .left_0()
                    .right_0()
                    .bottom(px(if show_controls && !compact {
                        CONTROL_BAR_HEIGHT + 16.0
                    } else {
                        24.0
                    }))
                    .justify_center()
                    .child(
                        div()
                            .max_w(px(width * 0.8))
                            .px(px(8.0))
                            .py(px(2.0))
                            .rounded(px(4.0))
                            .bg(black().opacity(0.75))
                            .child(text(
                                caption,
                                if compact { 14.0 } else { 18.0 },
                                if compact { 20.0 } else { 26.0 },
                                FontWeight::MEDIUM,
                                white(),
                            )),
                    ),
            );
        }
        if let Some(remaining) = self.player.autoplay_countdown_ms.filter(|_| active) {
            let next_title = self
                .library
                .next_after(id)
                .and_then(|next| self.library.get(next))
                .map(|item| item.title.clone())
                .unwrap_or_default();
            let cancel = h_flex()
                .id("autoplay-cancel")
                .h(px(36.0))
                .px(px(20.0))
                .rounded_full()
                .border_1()
                .border_color(white().opacity(0.7))
                .child(text("Cancel", 14.0, 20.0, FontWeight::MEDIUM, white()));
            surface = surface.child(
                v_flex()
                    .absolute()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .gap(px(12.0))
                    .bg(black().opacity(0.6))
                    .child(text(
                        format!("Up next in {}", remaining.div_ceil(1000)),
                        20.0,
                        28.0,
                        FontWeight::BOLD,
                        white(),
                    ))
                    .child(text(
                        next_title,
                        14.0,
                        20.0,
                        FontWeight::NORMAL,
                        white().opacity(0.85),
                    ))
                    .child(self.activatable(
                        cancel,
                        true,
                        cx,
                        Rc::new(|this, _w, cx| {
                            this.player.cancel_autoplay();
                            cx.notify();
                        }),
                    )),
            );
        }
        if show_controls && !compact {
            surface = surface
                .child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .h(px(110.0))
                        .bg(linear_gradient(
                            180.0,
                            linear_color_stop(transparent_black(), 0.0),
                            linear_color_stop(black().opacity(0.7), 1.0),
                        )),
                )
                .child(self.controls_bar(id, cx));
            if let Some(panel) = self.settings_panel {
                surface = surface.child(self.settings_panel_view(panel, cx));
            }
        }
        if compact && active {
            let playing = self.player.state.playing;
            let progress = self.player.progress();
            if show_controls {
                surface = surface.child(
                    h_flex()
                        .absolute()
                        .left(px(16.0))
                        .top(px(16.0))
                        .gap(px(8.0))
                        .child(self.control_button(
                            "short-play",
                            if playing {
                                IconName::DebugPause
                            } else {
                                IconName::PlayFilled
                            },
                            48.0,
                            cx,
                            Rc::new(|this, _w, cx| this.toggle_play(cx)),
                        ))
                        .child(self.control_button(
                            "short-mute",
                            if self.player.effective_volume() <= 0.0 {
                                IconName::AudioOff
                            } else {
                                IconName::AudioOn
                            },
                            48.0,
                            cx,
                            Rc::new(|this, _w, cx| {
                                this.player.toggle_mute();
                                cx.notify();
                            }),
                        )),
                );
            }
            surface = surface.child(
                div()
                    .absolute()
                    .left_0()
                    .bottom_0()
                    .w_full()
                    .h(px(4.0))
                    .bg(white().opacity(0.25))
                    .child(
                        div()
                            .h_full()
                            .w(gpui::relative(progress))
                            .bg(palette.accent),
                    ),
            );
        }
        surface.into_any_element()
    }

    // ---- miniplayer ----------------------------------------------------------------------------------

    pub(crate) fn render_miniplayer(&self, cx: &mut Context<Self>) -> AnyElement {
        let palette = self.palette.clone();
        let Some(item) = self.watching.and_then(|id| self.library.get(id)) else {
            return div().into_any_element();
        };
        let playing = self.player.state.playing;
        let video = match self.frame.clone() {
            Some(frame) => img(frame)
                .w(px(MINIPLAYER_WIDTH))
                .h(px(MINIPLAYER_VIDEO_HEIGHT))
                .object_fit(gpui::ObjectFit::Contain)
                .into_any_element(),
            None => h_flex()
                .size_full()
                .justify_center()
                .child(glyph(IconName::PlayFilled, 32.0, white().opacity(0.3)))
                .into_any_element(),
        };
        let button = |this: &Self,
                      id: &'static str,
                      icon: IconName,
                      cx: &mut Context<Self>,
                      on_activate: Activate|
         -> AnyElement {
            let element = h_flex()
                .id(id)
                .w(px(40.0))
                .h(px(40.0))
                .flex_none()
                .justify_center()
                .rounded_full()
                .hover({
                    let hover = this.palette.hover;
                    move |style| style.bg(hover)
                })
                .child(glyph(icon, 20.0, this.palette.text));
            this.activatable(element, true, cx, on_activate)
                .into_any_element()
        };
        v_flex()
            .id("miniplayer")
            .absolute()
            .right(px(MINIPLAYER_MARGIN))
            .bottom(px(MINIPLAYER_MARGIN))
            .w(px(MINIPLAYER_WIDTH))
            .h(px(MINIPLAYER_VIDEO_HEIGHT + MINIPLAYER_STRIP_HEIGHT))
            .rounded(px(12.0))
            .overflow_hidden()
            .bg(palette.surface)
            .occlude()
            .child(
                div()
                    .w_full()
                    .h(px(MINIPLAYER_VIDEO_HEIGHT))
                    .bg(black())
                    .child(video),
            )
            .child(
                h_flex()
                    .h(px(MINIPLAYER_STRIP_HEIGHT))
                    .px(px(12.0))
                    .gap(px(4.0))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .child(div().truncate().child(text(
                                item.title.clone(),
                                14.0,
                                20.0,
                                FontWeight::MEDIUM,
                                palette.text,
                            )))
                            .child(div().truncate().child(text(
                                item.creator.clone(),
                                12.0,
                                18.0,
                                FontWeight::NORMAL,
                                palette.text_muted,
                            ))),
                    )
                    .child(button(
                        self,
                        "mini-play",
                        if playing {
                            IconName::DebugPause
                        } else {
                            IconName::PlayFilled
                        },
                        cx,
                        Rc::new(|this, _w, cx| this.toggle_play(cx)),
                    ))
                    .child(button(
                        self,
                        "mini-expand",
                        IconName::Maximize,
                        cx,
                        Rc::new(|this, _w, cx| this.expand_miniplayer(cx)),
                    ))
                    .child(button(
                        self,
                        "mini-close",
                        IconName::Close,
                        cx,
                        Rc::new(|this, _w, cx| this.close_miniplayer(cx)),
                    )),
            )
            .into_any_element()
    }
}
