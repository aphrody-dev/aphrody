//! Role palette, drawing primitives and the shared surfaces: top bar, navigation, chips, cards,
//! skeletons and empty/error states.

use std::rc::Rc;

use aphrody_ui::OsTheme;
use gpui::{
    AnyElement, Context, Div, FontWeight, Hsla, KeyDownEvent, MouseButton, SharedString, Stateful,
    Svg, Window, black, div, img, point, prelude::*, px, svg, white,
};
use icons::IconName;

use crate::app::{HomeChip, Route, ShellLayout, VideoApp};
use crate::layout::{
    self, CHIP_BAR_HEIGHT, NAV_EXPANDED_WIDTH, NAV_MINI_WIDTH, NavMode, PAGE_PADDING,
    SEARCH_BOX_HEIGHT, TOP_BAR_HEIGHT, thumbnail_height,
};
use crate::library::{format_duration, relative_age, unix_now};

/// Colour roles resolved from the active Aphrody theme (Material 3 tokens from `aphrody_ui`).
#[derive(Clone)]
pub struct Palette {
    pub base: Hsla,
    pub surface: Hsla,
    pub surface_high: Hsla,
    pub text: Hsla,
    pub text_muted: Hsla,
    pub accent: Hsla,
    pub link: Hsla,
    pub outline: Hsla,
    pub hover: Hsla,
    pub inverse_fill: Hsla,
    pub inverse_text: Hsla,
    pub scrim: Hsla,
    pub avatars: [(Hsla, Hsla); 3],
}

impl Palette {
    #[must_use]
    pub fn from_theme(theme: &OsTheme) -> Self {
        let colors = &theme.colors;
        Self {
            base: colors.background,
            surface: colors.surface_container,
            surface_high: colors.surface_container_high,
            text: colors.on_surface,
            text_muted: colors.on_surface_variant,
            accent: colors.error,
            link: colors.primary,
            outline: colors.outline_variant,
            hover: colors.on_surface.opacity(0.10),
            inverse_fill: colors.on_surface,
            inverse_text: colors.surface,
            scrim: black().opacity(0.6),
            avatars: [
                (colors.primary_container, colors.on_primary_container),
                (colors.secondary_container, colors.on_secondary_container),
                (colors.tertiary, colors.on_tertiary),
            ],
        }
    }
}

pub type Activate = Rc<dyn Fn(&mut VideoApp, &mut Window, &mut Context<VideoApp>)>;

#[must_use]
pub fn h_flex() -> Div {
    div().flex().flex_row().items_center()
}

#[must_use]
pub fn v_flex() -> Div {
    div().flex().flex_col()
}

#[must_use]
pub fn glyph(name: IconName, size: f32, color: Hsla) -> Svg {
    svg()
        .path(SharedString::from(name.path().to_string()))
        .flex_none()
        .w(px(size))
        .h(px(size))
        .text_color(color)
}

#[must_use]
pub fn text(
    content: impl Into<SharedString>,
    size: f32,
    line: f32,
    weight: FontWeight,
    color: Hsla,
) -> Div {
    div()
        .text_size(px(size))
        .line_height(px(line))
        .font_weight(weight)
        .text_color(color)
        .child(content.into())
}

pub struct ChipSpec {
    pub label: SharedString,
    pub selected: bool,
    pub on_select: Activate,
}

impl VideoApp {
    /// Makes `element` clickable and, when `tab_stop`, reachable by Tab with a visible focus
    /// ring that Enter activates.
    pub(crate) fn activatable(
        &self,
        element: Stateful<Div>,
        tab_stop: bool,
        cx: &mut Context<Self>,
        on_activate: Activate,
    ) -> Stateful<Div> {
        let ring = self.palette.text;
        let click = on_activate.clone();
        element
            .cursor_pointer()
            .on_click(cx.listener(move |this, _event, window, cx| click(this, window, cx)))
            .when(tab_stop, |element| {
                element
                    .tab_index(0)
                    .focus_visible(move |style| style.border_2().border_color(ring))
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                        if event.keystroke.key == "enter" {
                            on_activate(this, window, cx);
                            cx.stop_propagation();
                        }
                    }))
            })
    }

    pub(crate) fn icon_button(
        &self,
        id: impl Into<gpui::ElementId>,
        icon: IconName,
        size: f32,
        cx: &mut Context<Self>,
        on_activate: Activate,
    ) -> Stateful<Div> {
        let palette = self.palette.clone();
        let element = h_flex()
            .id(id)
            .w(px(size))
            .h(px(size))
            .flex_none()
            .justify_center()
            .rounded_full()
            .hover(move |style| style.bg(palette.hover))
            .child(glyph(icon, 20.0, self.palette.text));
        self.activatable(element, true, cx, on_activate)
    }

    // ---- top bar -------------------------------------------------------------------------------

    pub(crate) fn render_top_bar(
        &self,
        shell: &ShellLayout,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = self.palette.clone();
        let collapsed = layout::search_collapsed(shell.viewport_width);
        let search_width = layout::search_box_width(shell.viewport_width);
        let wordmark = h_flex()
            .id("wordmark")
            .gap(px(8.0))
            .h(px(40.0))
            .px(px(4.0))
            .child(glyph(IconName::AphrodyCharacterFill, 26.0, palette.link))
            .when(shell.viewport_width >= 420.0, |element| {
                element.child(text(
                    "Aphrody Video",
                    16.0,
                    22.0,
                    FontWeight::BOLD,
                    palette.text,
                ))
            });
        let wordmark = self.activatable(
            wordmark,
            true,
            cx,
            Rc::new(|this, _window, cx| {
                this.navigate(Route::Home, cx);
                this.select_chip(HomeChip::All, cx);
                this.grid_scroll
                    .scroll_to_item(0, gpui::ScrollStrategy::Top);
            }),
        );

        let mut bar = h_flex()
            .id("top-bar")
            .relative()
            .flex_none()
            .w_full()
            .h(px(TOP_BAR_HEIGHT))
            .px(px(16.0))
            .gap(px(8.0))
            .bg(palette.base)
            .child(self.icon_button(
                "menu-toggle",
                IconName::Menu,
                40.0,
                cx,
                Rc::new(|this, _window, cx| this.toggle_menu(cx)),
            ))
            .child(wordmark);

        let search_active = !collapsed || self.search_expanded;
        if search_active {
            let left = if collapsed {
                56.0
            } else {
                (shell.viewport_width - search_width) / 2.0
            };
            let width = if collapsed {
                shell.viewport_width - 56.0 - 16.0
            } else {
                search_width
            };
            bar = bar.child(
                div()
                    .absolute()
                    .top(px((TOP_BAR_HEIGHT - SEARCH_BOX_HEIGHT) / 2.0))
                    .left(px(left))
                    .child(self.render_search_field(width, window, cx)),
            );
        }

        let mut end = h_flex().ml_auto().gap(px(4.0)).flex_none();
        if collapsed && !self.search_expanded {
            end = end.child(self.icon_button(
                "search-open",
                IconName::MagnifyingGlass,
                40.0,
                cx,
                Rc::new(|this, window, cx| {
                    this.search_expanded = true;
                    window.focus(&this.search_focus.clone(), cx);
                    cx.notify();
                }),
            ));
        }
        end = end
            .child(self.icon_button(
                "theme-toggle",
                IconName::Sparkle,
                40.0,
                cx,
                Rc::new(|this, _window, cx| this.toggle_theme(cx)),
            ))
            .child(self.icon_button(
                "rescan",
                IconName::RefreshTitle,
                40.0,
                cx,
                Rc::new(|this, _window, cx| {
                    let root = this.library.root().to_owned();
                    if root.as_os_str().is_empty() {
                        return;
                    }
                    this.open_library(root, cx);
                }),
            ));
        bar.child(end).into_any_element()
    }

    fn render_search_field(
        &self,
        width: f32,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = self.palette.clone();
        let focused = self.search_focus.is_focused(window);
        let empty = self.search_text.is_empty();
        let field = h_flex()
            .id("search-field")
            .key_context("AphrodySearch")
            .track_focus(&self.search_focus)
            .w(px(width))
            .h(px(SEARCH_BOX_HEIGHT))
            .rounded_full()
            .border_1()
            .border_color(if focused {
                palette.link
            } else {
                palette.outline
            })
            .overflow_hidden()
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                let keystroke = &event.keystroke;
                match keystroke.key.as_str() {
                    "enter" => this.submit_search(cx),
                    "backspace" => {
                        this.search_text.pop();
                        cx.notify();
                    }
                    _ => {
                        let typed = !keystroke.modifiers.control
                            && !keystroke.modifiers.platform
                            && !keystroke.modifiers.alt;
                        if let Some(character) = keystroke.key_char.as_ref().filter(|_| typed) {
                            this.search_text.push_str(character);
                            cx.notify();
                        }
                    }
                }
            }))
            .on_click(cx.listener(|this, _event, window, cx| {
                window.focus(&this.search_focus.clone(), cx);
                cx.notify();
            }))
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .px(px(16.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_size(px(16.0))
                    .child(if empty {
                        text(
                            "Search your library",
                            16.0,
                            22.0,
                            FontWeight::NORMAL,
                            palette.text_muted,
                        )
                    } else {
                        text(
                            self.search_text.clone(),
                            16.0,
                            22.0,
                            FontWeight::NORMAL,
                            palette.text,
                        )
                    })
                    .when(focused, |element| {
                        element.child(div().w(px(1.5)).h(px(20.0)).ml(px(1.0)).bg(palette.text))
                    }),
            )
            .when(!empty, |element| {
                let clear = h_flex()
                    .id("search-clear")
                    .w(px(32.0))
                    .h(px(32.0))
                    .mr(px(4.0))
                    .flex_none()
                    .justify_center()
                    .rounded_full()
                    .child(glyph(IconName::Close, 16.0, palette.text));
                element.child(self.activatable(
                    clear,
                    false,
                    cx,
                    Rc::new(|this, window, cx| {
                        this.search_text.clear();
                        window.focus(&this.search_focus.clone(), cx);
                        cx.notify();
                    }),
                ))
            })
            .child({
                let submit = h_flex()
                    .id("search-submit")
                    .w(px(56.0))
                    .h_full()
                    .flex_none()
                    .justify_center()
                    .border_l_1()
                    .border_color(palette.outline)
                    .bg(palette.surface)
                    .child(glyph(IconName::MagnifyingGlass, 20.0, palette.text));
                self.activatable(
                    submit,
                    true,
                    cx,
                    Rc::new(|this, _window, cx| this.submit_search(cx)),
                )
            });
        field.into_any_element()
    }

    // ---- navigation ----------------------------------------------------------------------------

    fn nav_items(&self) -> Vec<(&'static str, IconName, &'static str, Route)> {
        vec![
            ("nav-home", IconName::Screen, "Home", Route::Home),
            (
                "nav-shorts",
                IconName::MobilePhone,
                "Short-form",
                Route::Shorts,
            ),
            (
                "nav-following",
                IconName::UserGroup,
                "Following",
                Route::Following,
            ),
            (
                "nav-library",
                IconName::HistoryRerun,
                "Library",
                Route::Library,
            ),
        ]
    }

    pub(crate) fn render_nav(&self, mode: NavMode, cx: &mut Context<Self>) -> AnyElement {
        let palette = self.palette.clone();
        match mode {
            NavMode::Hidden => div().into_any_element(),
            NavMode::Mini => {
                let mut rail = v_flex()
                    .flex_none()
                    .w(px(NAV_MINI_WIDTH))
                    .h_full()
                    .px(px(4.0))
                    .bg(palette.base);
                for (id, icon, label, route) in self.nav_items() {
                    let selected = self.route == route;
                    let entry = v_flex()
                        .id(id)
                        .w(px(64.0))
                        .h(px(76.0))
                        .items_center()
                        .justify_center()
                        .gap(px(6.0))
                        .rounded(px(10.0))
                        .hover({
                            let hover = palette.hover;
                            move |style| style.bg(hover)
                        })
                        .when(selected, |element| element.bg(palette.hover))
                        .child(glyph(icon, 24.0, palette.text))
                        .child(text(label, 10.0, 14.0, FontWeight::NORMAL, palette.text));
                    let target = route;
                    rail = rail.child(self.activatable(
                        entry,
                        !self.drawer_open,
                        cx,
                        Rc::new(move |this, _window, cx| this.navigate(target.clone(), cx)),
                    ));
                }
                rail.into_any_element()
            }
            NavMode::Expanded => self.render_expanded_nav(cx),
        }
    }

    fn nav_entry(
        &self,
        id: impl Into<gpui::ElementId>,
        icon: Option<IconName>,
        avatar: Option<&str>,
        label: SharedString,
        selected: bool,
        tab_stop: bool,
        cx: &mut Context<Self>,
        on_activate: Activate,
    ) -> AnyElement {
        let palette = self.palette.clone();
        let entry = h_flex()
            .id(id)
            .w_full()
            .h(px(40.0))
            .px(px(12.0))
            .gap(px(24.0))
            .rounded(px(10.0))
            .hover({
                let hover = palette.hover;
                move |style| style.bg(hover)
            })
            .when(selected, |element| element.bg(palette.hover))
            .when_some(icon, |element, icon| {
                element.child(glyph(icon, 24.0, palette.text))
            })
            .when_some(avatar, |element, name| {
                element.child(self.avatar(name, 24.0))
            })
            .child(div().flex_1().min_w_0().truncate().child(text(
                label,
                14.0,
                20.0,
                if selected {
                    FontWeight::MEDIUM
                } else {
                    FontWeight::NORMAL
                },
                palette.text,
            )));
        self.activatable(entry, tab_stop, cx, on_activate)
            .into_any_element()
    }

    pub(crate) fn render_expanded_nav(&self, cx: &mut Context<Self>) -> AnyElement {
        let palette = self.palette.clone();
        let tab_stop = true;
        let mut column = v_flex()
            .id("nav-expanded")
            .flex_none()
            .w(px(NAV_EXPANDED_WIDTH))
            .h_full()
            .px(px(12.0))
            .gap(px(2.0))
            .overflow_y_scroll()
            .bg(palette.base);
        for (id, icon, label, route) in self.nav_items() {
            let selected = self.route == route;
            column = column.child(self.nav_entry(
                id,
                Some(icon),
                None,
                label.into(),
                selected,
                tab_stop,
                cx,
                Rc::new(move |this, _window, cx| this.navigate(route.clone(), cx)),
            ));
        }
        column = column.child(div().my(px(12.0)).h(px(1.0)).w_full().bg(palette.outline));
        column = column.child(div().px(px(12.0)).pb(px(4.0)).child(text(
            "Creators",
            16.0,
            24.0,
            FontWeight::MEDIUM,
            palette.text,
        )));
        for (index, creator) in self.library.creators().into_iter().take(12).enumerate() {
            let name = creator.name;
            let target = name.clone();
            let selected = self.route == Route::Creator(name.clone());
            column = column.child(self.nav_entry(
                ("nav-creator", index),
                None,
                Some(&name),
                name.clone().into(),
                selected,
                tab_stop,
                cx,
                Rc::new(move |this, _window, cx| this.navigate(Route::Creator(target.clone()), cx)),
            ));
        }
        column = column.child(div().my(px(12.0)).h(px(1.0)).w_full().bg(palette.outline));
        column = column
            .child(self.nav_entry(
                "nav-theme",
                Some(IconName::Sparkle),
                None,
                "Switch light or dark".into(),
                false,
                tab_stop,
                cx,
                Rc::new(|this, _window, cx| this.toggle_theme(cx)),
            ))
            .child(div().px(px(12.0)).py(px(12.0)).child(text(
                format!(
                    "{} local videos. Nothing leaves this computer.",
                    self.library.items().len()
                ),
                12.0,
                18.0,
                FontWeight::NORMAL,
                palette.text_muted,
            )));
        column.into_any_element()
    }

    pub(crate) fn render_drawer(&self, cx: &mut Context<Self>) -> AnyElement {
        let palette = self.palette.clone();
        let panel = v_flex()
            .id("drawer")
            .absolute()
            .top_0()
            .left_0()
            .bottom_0()
            .w(px(NAV_EXPANDED_WIDTH))
            .bg(palette.base)
            .occlude()
            .child(
                h_flex()
                    .h(px(TOP_BAR_HEIGHT))
                    .px(px(16.0))
                    .child(self.icon_button(
                        "drawer-close",
                        IconName::Menu,
                        40.0,
                        cx,
                        Rc::new(|this, _window, cx| {
                            this.drawer_open = false;
                            cx.notify();
                        }),
                    )),
            )
            .child(div().flex_1().min_h_0().child(self.render_expanded_nav(cx)));
        div()
            .id("drawer-scrim")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .bg(palette.scrim)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _event, _window, cx| {
                    this.drawer_open = false;
                    cx.notify();
                }),
            )
            .child(panel)
            .into_any_element()
    }

    // ---- avatars, chips ----------------------------------------------------------------------

    pub(crate) fn avatar(&self, name: &str, size: f32) -> Div {
        let hash = name.bytes().fold(0_usize, |acc, byte| {
            acc.wrapping_mul(31).wrapping_add(usize::from(byte))
        });
        let (fill, ink) = self.palette.avatars[hash % self.palette.avatars.len()];
        let initial: String = name
            .chars()
            .next()
            .map(|c| c.to_uppercase().collect())
            .unwrap_or_default();
        h_flex()
            .flex_none()
            .w(px(size))
            .h(px(size))
            .justify_center()
            .rounded_full()
            .bg(fill)
            .child(text(
                initial,
                size * 0.45,
                size * 0.6,
                FontWeight::MEDIUM,
                ink,
            ))
    }

    pub(crate) fn render_chip_bar(
        &self,
        chips: Vec<ChipSpec>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = self.palette.clone();
        let scroll = self.chip_scroll.clone();
        let max = scroll.max_offset();
        let offset = scroll.offset();
        let overflowing = f32::from(max.x) > 1.0;
        let at_end = f32::from(offset.x) <= -f32::from(max.x) + 1.0;
        let mut row = h_flex()
            .id("chip-row")
            .track_scroll(&scroll)
            .overflow_x_scroll()
            .gap(px(8.0))
            .flex_1()
            .min_w_0();
        for (index, chip) in chips.into_iter().enumerate() {
            let element = h_flex()
                .id(("chip", index))
                .h(px(32.0))
                .px(px(12.0))
                .flex_none()
                .rounded(px(8.0))
                .when(chip.selected, |element| element.bg(palette.inverse_fill))
                .when(!chip.selected, |element| {
                    let hover = palette.hover;
                    element
                        .bg(palette.hover)
                        .hover(move |style| style.bg(hover.opacity(0.18)))
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
            row = row.child(self.activatable(element, !self.drawer_open, cx, chip.on_select));
        }
        let viewport_width = f32::from(self.viewport.width);
        h_flex()
            .relative()
            .flex_none()
            .w_full()
            .h(px(CHIP_BAR_HEIGHT))
            .px(px(PAGE_PADDING))
            .bg(palette.base)
            .child(row)
            .when(overflowing && !at_end, |element| {
                let scroll = scroll.clone();
                element.child(
                    h_flex()
                        .absolute()
                        .right(px(PAGE_PADDING))
                        .top_0()
                        .h_full()
                        .w(px(48.0))
                        .justify_end()
                        .bg(palette.base)
                        .child(self.icon_button(
                            "chip-next",
                            IconName::ChevronRight,
                            32.0,
                            cx,
                            Rc::new(move |_this, _window, cx| {
                                let current = scroll.offset();
                                let limit = -f32::from(scroll.max_offset().x);
                                let target =
                                    (f32::from(current.x) - viewport_width * 0.75).max(limit);
                                scroll.set_offset(point(px(target), current.y));
                                cx.notify();
                            }),
                        )),
                )
            })
            .into_any_element()
    }

    // ---- cards ---------------------------------------------------------------------------------

    pub(crate) fn thumbnail(
        &self,
        id: usize,
        width: f32,
        height: f32,
        radius: f32,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let palette = self.palette.clone();
        let item = self.library.get(id);
        let hovered_preview = self.preview_frame(id, cx.background_executor().now());
        let image = hovered_preview
            .as_ref()
            .map(|(image, _)| image.clone())
            .or_else(|| self.posters.get(&id).cloned());
        let progress = item.map_or(0.0, |item| {
            if self.watched.contains(&id) {
                1.0
            } else {
                self.positions.get(&id).map_or(0.0, |position| {
                    *position as f32 / item.duration_ms.max(1) as f32
                })
            }
        });
        div()
            .id(("thumb", id))
            .relative()
            .flex_none()
            .w(px(width))
            .h(px(height))
            .rounded(px(radius))
            .overflow_hidden()
            .bg(palette.surface_high)
            .on_hover(cx.listener(move |this, hovered: &bool, _window, cx| {
                this.set_card_hover(id, *hovered, cx);
            }))
            .map(|element| match image {
                Some(image) => element.child(
                    img(image)
                        .w(px(width))
                        .h(px(height))
                        .object_fit(gpui::ObjectFit::Cover),
                ),
                None => element.child(h_flex().size_full().justify_center().child(glyph(
                    IconName::PlayOutlined,
                    32.0,
                    palette.text_muted.opacity(0.5),
                ))),
            })
            .when_some(item, |element, item| {
                element.child(
                    div()
                        .absolute()
                        .right(px(8.0))
                        .bottom(px(if progress > 0.0 { 12.0 } else { 8.0 }))
                        .px(px(4.0))
                        .py(px(1.0))
                        .rounded(px(4.0))
                        .bg(black().opacity(0.6))
                        .child(text(
                            format_duration(item.duration_ms),
                            12.0,
                            18.0,
                            FontWeight::MEDIUM,
                            white(),
                        )),
                )
            })
            .when(progress > 0.0, |element| {
                element.child(
                    div()
                        .absolute()
                        .left_0()
                        .bottom_0()
                        .w_full()
                        .h(px(4.0))
                        .bg(white().opacity(0.3))
                        .child(
                            div()
                                .h_full()
                                .w(px(width * progress.clamp(0.0, 1.0)))
                                .bg(palette.accent),
                        ),
                )
            })
            .when_some(hovered_preview, |element, (_, fraction)| {
                element.child(
                    div()
                        .absolute()
                        .left_0()
                        .bottom_0()
                        .w_full()
                        .h(px(3.0))
                        .child(div().h_full().w(px(width * fraction)).bg(palette.accent)),
                )
            })
    }

    pub(crate) fn meta_line(&self, id: usize) -> String {
        let now = unix_now();
        let Some(item) = self.library.get(id) else {
            return String::new();
        };
        let mut meta = if item.height > 0 {
            format!(
                "{}p · {}",
                item.height,
                relative_age(item.modified_secs, now)
            )
        } else {
            relative_age(item.modified_secs, now)
        };
        if let Some(plays) = self.plays.get(&id).filter(|plays| **plays > 0) {
            meta.push_str(&format!(
                " · {} play{}",
                plays,
                if *plays == 1 { "" } else { "s" }
            ));
        }
        meta
    }

    /// Vertical card of the feed and the creator grid.
    pub(crate) fn video_card(
        &self,
        id: usize,
        width: f32,
        with_avatar: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = self.palette.clone();
        let Some(item) = self.library.get(id) else {
            return div().into_any_element();
        };
        let height = thumbnail_height(width);
        let title = item.title.clone();
        let creator = item.creator.clone();
        let meta = self.meta_line(id);
        let open: Activate = Rc::new(move |this, _window, cx| this.open_watch(id, cx));
        let creator_target = creator.clone();
        let more = h_flex()
            .id(("card-more", id))
            .w(px(40.0))
            .h(px(40.0))
            .flex_none()
            .justify_center()
            .rounded_full()
            .opacity(0.0)
            .group_hover(SharedString::from(format!("card-{id}")), |style| {
                style.opacity(1.0)
            })
            .child(glyph(IconName::Ellipsis, 20.0, palette.text));
        let more = self.activatable(
            more,
            !self.drawer_open,
            cx,
            Rc::new(move |_this, _window, _cx| {}),
        );
        let card = v_flex()
            .id(("card", id))
            .group(SharedString::from(format!("card-{id}")))
            .w(px(width))
            .flex_none()
            .child(self.activatable(
                self.thumbnail(id, width, height, 12.0, cx),
                !self.drawer_open,
                cx,
                open.clone(),
            ))
            .child(
                h_flex()
                    .items_start()
                    .gap(px(12.0))
                    .mt(px(12.0))
                    .when(with_avatar, |element| {
                        element.child(self.avatar(&creator, 36.0))
                    })
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .child(self.activatable(
                                div().id(("card-title", id)).line_clamp(2).child(text(
                                    title,
                                    16.0,
                                    22.0,
                                    FontWeight::BOLD,
                                    palette.text,
                                )),
                                !self.drawer_open,
                                cx,
                                open,
                            ))
                            .child(self.activatable(
                                div().id(("card-creator", id)).truncate().child(text(
                                    creator,
                                    12.0,
                                    18.0,
                                    FontWeight::NORMAL,
                                    palette.text_muted,
                                )),
                                !self.drawer_open,
                                cx,
                                Rc::new(move |this, _window, cx| {
                                    this.navigate(Route::Creator(creator_target.clone()), cx);
                                }),
                            ))
                            .child(text(
                                meta,
                                12.0,
                                18.0,
                                FontWeight::NORMAL,
                                palette.text_muted,
                            )),
                    )
                    .child(more),
            );
        card.into_any_element()
    }

    pub(crate) fn skeleton_card(&self, width: f32, with_avatar: bool) -> AnyElement {
        let palette = self.palette.clone();
        v_flex()
            .w(px(width))
            .flex_none()
            .child(
                div()
                    .w(px(width))
                    .h(px(thumbnail_height(width)))
                    .rounded(px(12.0))
                    .bg(palette.surface_high),
            )
            .child(
                h_flex()
                    .items_start()
                    .gap(px(12.0))
                    .mt(px(12.0))
                    .when(with_avatar, |element| {
                        element.child(
                            div()
                                .w(px(36.0))
                                .h(px(36.0))
                                .flex_none()
                                .rounded_full()
                                .bg(palette.surface_high),
                        )
                    })
                    .child(
                        v_flex()
                            .flex_1()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .w_full()
                                    .h(px(14.0))
                                    .rounded(px(4.0))
                                    .bg(palette.surface_high),
                            )
                            .child(
                                div()
                                    .w(px(width * 0.6))
                                    .h(px(12.0))
                                    .rounded(px(4.0))
                                    .bg(palette.surface_high),
                            ),
                    ),
            )
            .into_any_element()
    }

    /// Centered card with a headline, two lines of text and an action.
    pub(crate) fn empty_state(
        &self,
        headline: &str,
        body: &str,
        action: Option<(&'static str, Activate)>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = self.palette.clone();
        let mut card = v_flex()
            .w(px(400.0))
            .p(px(24.0))
            .gap(px(12.0))
            .rounded(px(12.0))
            .bg(palette.surface)
            .child(text(
                headline.to_owned(),
                20.0,
                28.0,
                FontWeight::BOLD,
                palette.text,
            ))
            .child(text(
                body.to_owned(),
                14.0,
                20.0,
                FontWeight::NORMAL,
                palette.text_muted,
            ));
        if let Some((label, on_activate)) = action {
            let button = h_flex()
                .id("empty-action")
                .w_full()
                .h(px(36.0))
                .justify_center()
                .rounded_full()
                .bg(palette.hover)
                .child(text(label, 14.0, 20.0, FontWeight::MEDIUM, palette.text));
            card = card.child(self.activatable(button, true, cx, on_activate));
        }
        h_flex()
            .size_full()
            .justify_center()
            .pt(px(64.0))
            .items_start()
            .child(card)
            .into_any_element()
    }

    /// Unavailable video: illustration, one-line headline, outlined pill back to home.
    pub(crate) fn unavailable_state(&self, cx: &mut Context<Self>) -> AnyElement {
        let palette = self.palette.clone();
        let illustration = h_flex()
            .w(px(160.0))
            .h(px(160.0))
            .justify_center()
            .rounded_full()
            .bg(palette.surface)
            .child(glyph(IconName::Warning, 72.0, palette.text_muted));
        let button = h_flex()
            .id("unavailable-home")
            .h(px(36.0))
            .px(px(20.0))
            .justify_center()
            .rounded_full()
            .border_1()
            .border_color(palette.outline)
            .child(text(
                "Go to home",
                14.0,
                20.0,
                FontWeight::MEDIUM,
                palette.link,
            ));
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap(px(24.0))
            .child(illustration)
            .child(text(
                "This video is no longer available",
                20.0,
                28.0,
                FontWeight::BOLD,
                palette.text,
            ))
            .child(self.activatable(
                button,
                true,
                cx,
                Rc::new(|this, _window, cx| this.navigate(Route::Home, cx)),
            ))
            .into_any_element()
    }

    pub(crate) fn spinner_row(&self) -> AnyElement {
        h_flex()
            .w_full()
            .h(px(layout::SPINNER_ROW_HEIGHT))
            .justify_center()
            .child(glyph(IconName::LoadCircle, 28.0, self.palette.text_muted))
            .into_any_element()
    }
}
