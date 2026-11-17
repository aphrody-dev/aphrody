use std::collections::BTreeSet;

use gpui::{FontWeight, Hsla, PathBuilder, canvas, point};
use ui::{Divider, prelude::*};

use crate::cosmic_theme::OsTheme;
use crate::reactive::focusable_action_chip;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusTone {
    Neutral,
    Accent,
    Success,
    Warning,
    Error,
}

/// Ephemeral UI state persisted across frames (`egui::Memory` analog) such as collapsed sections.
#[derive(Debug, Default, Clone)]
pub struct ImmediateMemory {
    pub collapsed_sections: BTreeSet<String>,
}

impl ImmediateMemory {
    pub fn is_open(&self, id: &str) -> bool {
        !self.collapsed_sections.contains(id)
    }

    pub fn toggle_section(&mut self, id: &str) {
        if !self.collapsed_sections.insert(id.to_owned()) {
            self.collapsed_sections.remove(id);
        }
    }
}

/// Implemented by any inspector target (ArtCraft layer, WGPU pass, session unit, or RE finding).
pub trait PropertyInspect {
    fn inspect_ui(&mut self, ui: &mut ImmediateUi<'_>);
}

/// Non-overlapping inspector key-value row with constrained truncation for narrow panes.
#[derive(IntoElement)]
pub struct KeyValueRow {
    key: SharedString,
    value: SharedString,
}

impl KeyValueRow {
    #[must_use]
    pub fn new(key: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

impl RenderOnce for KeyValueRow {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        h_flex()
            .w_full()
            .min_w_0()
            .justify_between()
            .items_center()
            .gap_2()
            .child(
                div()
                    .min_w_0()
                    .flex_shrink_0()
                    .max_w(relative(0.5))
                    .overflow_hidden()
                    .child(
                        Label::new(self.key)
                            .size(LabelSize::XSmall)
                            .color(Color::Muted)
                            .truncate(),
                    ),
            )
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .justify_end()
                    .overflow_hidden()
                    .child(
                        Label::new(self.value)
                            .size(LabelSize::XSmall)
                            .truncate_middle(),
                    ),
            )
    }
}

/// Inspector row displaying a labeled color swatch token without narrow-pane overflow.
#[derive(IntoElement)]
pub struct ColorSwatchRow {
    label: SharedString,
    swatch_color: Hsla,
    text: SharedString,
    border_color: Hsla,
}

impl ColorSwatchRow {
    #[must_use]
    pub fn new(
        label: impl Into<SharedString>,
        swatch_color: Hsla,
        text: impl Into<SharedString>,
        border_color: Hsla,
    ) -> Self {
        Self {
            label: label.into(),
            swatch_color,
            text: text.into(),
            border_color,
        }
    }
}

impl RenderOnce for ColorSwatchRow {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        h_flex()
            .w_full()
            .min_w_0()
            .justify_between()
            .items_center()
            .gap_2()
            .child(
                div()
                    .min_w_0()
                    .flex_shrink_0()
                    .max_w(relative(0.5))
                    .overflow_hidden()
                    .child(
                        Label::new(self.label)
                            .size(LabelSize::XSmall)
                            .color(Color::Muted)
                            .truncate(),
                    ),
            )
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .justify_end()
                    .overflow_hidden()
                    .gap_1p5()
                    .child(
                        div()
                            .flex_shrink_0()
                            .w(rems_from_px(14.0_f32))
                            .h(rems_from_px(14.0_f32))
                            .rounded(rems_from_px(4.0_f32))
                            .border_1()
                            .border_color(self.border_color)
                            .bg(self.swatch_color),
                    )
                    .child(Label::new(self.text).size(LabelSize::XSmall).truncate()),
            )
    }
}

/// Single-pass `egui`-ergonomic immediate builder outputting composable native GPUI elements.
pub struct ImmediateUi<'a> {
    pub theme: &'a OsTheme,
    children: Vec<AnyElement>,
}

impl<'a> ImmediateUi<'a> {
    #[must_use]
    pub fn new(theme: &'a OsTheme) -> Self {
        Self {
            theme,
            children: Vec::new(),
        }
    }

    pub fn heading(&mut self, title: impl Into<SharedString>) {
        self.children.push(
            v_flex()
                .w_full()
                .min_w_0()
                .gap_1()
                .child(
                    Label::new(title.into())
                        .size(LabelSize::Default)
                        .weight(FontWeight::SEMIBOLD)
                        .truncate(),
                )
                .child(Divider::horizontal())
                .into_any_element(),
        );
    }

    pub fn label(&mut self, text: impl Into<SharedString>) {
        self.children.push(
            div()
                .w_full()
                .min_w_0()
                .child(Label::new(text.into()).size(LabelSize::Small))
                .into_any_element(),
        );
    }

    pub fn separator(&mut self) {
        self.children
            .push(Divider::horizontal().my_1().into_any_element());
    }

    pub fn push_element(&mut self, element: AnyElement) {
        self.children.push(element);
    }

    pub fn section(
        &mut self,
        title: impl Into<SharedString>,
        build: impl FnOnce(&mut ImmediateUi<'a>),
    ) {
        let mut sub = ImmediateUi::new(self.theme);
        build(&mut sub);
        let card = v_flex()
            .w_full()
            .min_w_0()
            .overflow_hidden()
            .p_2p5()
            .gap_2()
            .rounded(rems_from_px(8.0_f32))
            .bg(self.theme.colors.surface_container_low)
            .border_1()
            .border_color(self.theme.colors.outline_variant)
            .child(
                Label::new(title.into())
                    .size(LabelSize::Small)
                    .weight(FontWeight::MEDIUM)
                    .truncate(),
            )
            .children(sub.children)
            .into_any_element();
        self.children.push(card);
    }

    pub fn kv_row(&mut self, key: impl Into<SharedString>, value: impl Into<SharedString>) {
        self.children
            .push(KeyValueRow::new(key, value).into_any_element());
    }

    pub fn kv_stack(&mut self, key: impl Into<SharedString>, value: impl Into<SharedString>) {
        self.children.push(
            v_flex()
                .w_full()
                .min_w_0()
                .gap_0p5()
                .child(
                    Label::new(key.into())
                        .size(LabelSize::XSmall)
                        .color(Color::Muted)
                        .truncate(),
                )
                .child(
                    div()
                        .w_full()
                        .min_w_0()
                        .overflow_hidden()
                        .child(Label::new(value.into()).size(LabelSize::XSmall)),
                )
                .into_any_element(),
        );
    }

    pub fn color_swatch(
        &mut self,
        label: impl Into<SharedString>,
        hex_or_color: Hsla,
        text: impl Into<SharedString>,
    ) {
        self.children.push(
            ColorSwatchRow::new(label, hex_or_color, text, self.theme.colors.outline)
                .into_any_element(),
        );
    }

    pub fn button<F>(
        &mut self,
        id: impl Into<ElementId>,
        icon: IconName,
        label: impl Into<SharedString>,
        selected: bool,
        on_click: F,
    ) where
        F: Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    {
        let label = label.into();
        let chip = focusable_action_chip(
            id,
            icon,
            label.clone(),
            None,
            label,
            selected,
            0,
            self.theme,
            on_click,
        );
        self.children.push(chip);
    }

    #[must_use]
    pub fn finish(self) -> AnyElement {
        v_flex()
            .w_full()
            .min_w_0()
            .gap_2()
            .children(self.children)
            .into_any_element()
    }
}

/// Semantic badge pairing an SVG vector icon and explicit text label (never color-only).
#[derive(IntoElement)]
pub struct StatusBadge {
    icon: IconName,
    text: SharedString,
    tone: StatusTone,
    theme: OsTheme,
}

impl StatusBadge {
    #[must_use]
    pub fn new(
        icon: IconName,
        text: impl Into<SharedString>,
        tone: StatusTone,
        theme: &OsTheme,
    ) -> Self {
        Self {
            icon,
            text: text.into(),
            tone,
            theme: theme.clone(),
        }
    }
}

impl RenderOnce for StatusBadge {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let (bg, fg, border) = match self.tone {
            StatusTone::Neutral => (
                self.theme.colors.surface_container_high,
                self.theme.colors.on_surface_variant,
                self.theme.colors.outline_variant,
            ),
            StatusTone::Accent => (
                self.theme.colors.primary_container,
                self.theme.colors.on_primary_container,
                self.theme.colors.primary,
            ),
            StatusTone::Success => (
                self.theme.colors.secondary_container,
                self.theme.colors.on_secondary_container,
                self.theme.colors.secondary,
            ),
            StatusTone::Warning => (
                self.theme.colors.primary_container,
                self.theme.colors.on_primary_container,
                self.theme.colors.outline,
            ),
            StatusTone::Error => (
                self.theme.colors.error_container,
                self.theme.colors.on_error_container,
                self.theme.colors.error,
            ),
        };

        h_flex()
            .flex_none()
            .min_w_0()
            .px_2()
            .py_0p5()
            .gap_1()
            .items_center()
            .rounded(rems_from_px(6.0_f32))
            .bg(bg)
            .border_1()
            .border_color(border)
            .text_color(fg)
            .child(
                Icon::new(self.icon)
                    .size(IconSize::XSmall)
                    .color(Color::Custom(fg)),
            )
            .child(
                Label::new(self.text)
                    .size(LabelSize::XSmall)
                    .color(Color::Custom(fg))
                    .single_line(),
            )
    }
}

/// Semantic badge pairing an SVG vector icon and explicit text label (never color-only).
#[must_use]
pub fn status_badge(
    icon: IconName,
    text: impl Into<SharedString>,
    tone: StatusTone,
    theme: &OsTheme,
) -> AnyElement {
    StatusBadge::new(icon, text, tone, theme).into_any_element()
}

/// GPU-accelerated data sparkline rendered on `gpui::canvas` with explicit min/max legend and text values.
#[derive(IntoElement)]
pub struct SparklineChart {
    title: SharedString,
    unit: SharedString,
    samples: Vec<f32>,
    theme: OsTheme,
}

impl SparklineChart {
    #[must_use]
    pub fn new(
        title: impl Into<SharedString>,
        unit: impl Into<SharedString>,
        samples: &[f32],
        theme: &OsTheme,
    ) -> Self {
        Self {
            title: title.into(),
            unit: unit.into(),
            samples: samples.to_vec(),
            theme: theme.clone(),
        }
    }
}

impl RenderOnce for SparklineChart {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let points = self.samples;
        let (min_v, max_v, latest) = if points.is_empty() {
            (0.0, 0.0, 0.0)
        } else {
            let mut lo = f32::INFINITY;
            let mut hi = f32::NEG_INFINITY;
            for &v in &points {
                lo = lo.min(v);
                hi = hi.max(v);
            }
            (lo, hi, *points.last().unwrap_or(&0.0))
        };

        let line_color = self.theme.colors.primary;
        let grid_color = self.theme.colors.outline_variant;
        let unit = self.unit;

        v_flex()
            .w_full()
            .min_w_0()
            .overflow_hidden()
            .p_2()
            .gap_1p5()
            .rounded(rems_from_px(8.0_f32))
            .bg(self.theme.colors.surface_container_low)
            .border_1()
            .border_color(self.theme.colors.outline_variant)
            .child(
                h_flex()
                    .w_full()
                    .min_w_0()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .child(Label::new(self.title).size(LabelSize::XSmall).truncate()),
                    )
                    .child(
                        div().flex_shrink_0().child(
                            Label::new(format!("cur {latest:.1}{unit}"))
                                .size(LabelSize::XSmall)
                                .single_line(),
                        ),
                    ),
            )
            .child(div().w_full().h(rems_from_px(56.0_f32)).child(canvas(
                move |_bounds, _window, _cx| points,
                move |bounds, data, window, _cx| {
                    let mid_y = bounds.origin.y + bounds.size.height * 0.5;
                    let mut grid = PathBuilder::stroke(px(1.0));
                    grid.move_to(point(bounds.origin.x, mid_y));
                    grid.line_to(point(bounds.origin.x + bounds.size.width, mid_y));
                    if let Ok(path) = grid.build() {
                        window.paint_path(path, grid_color);
                    }

                    if data.len() < 2 {
                        return;
                    }
                    let span = (max_v - min_v).max(1.0);
                    let mut builder = PathBuilder::stroke(px(2.0));
                    for (idx, val) in data.iter().enumerate() {
                        let tx = idx as f32 / (data.len() - 1) as f32;
                        let ty = 1.0 - ((*val - min_v) / span).clamp(0.0, 1.0);
                        let pt = point(
                            bounds.origin.x + bounds.size.width * tx,
                            bounds.origin.y + px(4.0) + (bounds.size.height - px(8.0)) * ty,
                        );
                        if idx == 0 {
                            builder.move_to(pt);
                        } else {
                            builder.line_to(pt);
                        }
                    }
                    if let Ok(path) = builder.build() {
                        window.paint_path(path, line_color);
                    }
                },
            )))
            .child(
                h_flex()
                    .w_full()
                    .min_w_0()
                    .justify_between()
                    .gap_2()
                    .child(
                        div().min_w_0().overflow_hidden().child(
                            Label::new(format!("min {min_v:.1}{unit}"))
                                .size(LabelSize::XSmall)
                                .color(Color::Muted)
                                .truncate(),
                        ),
                    )
                    .child(
                        div().min_w_0().overflow_hidden().child(
                            Label::new(format!("max {max_v:.1}{unit}"))
                                .size(LabelSize::XSmall)
                                .color(Color::Muted)
                                .truncate(),
                        ),
                    ),
            )
    }
}

/// GPU-accelerated data sparkline rendered on `gpui::canvas` with explicit min/max legend and text values.
#[must_use]
pub fn sparkline_chart(
    title: impl Into<SharedString>,
    unit: &str,
    samples: &[f32],
    theme: &OsTheme,
) -> AnyElement {
    SparklineChart::new(title, unit.to_owned(), samples, theme).into_any_element()
}
