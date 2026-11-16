use std::collections::BTreeSet;

use gpui::{
    AnyElement, App, ElementId, PathBuilder, SharedString, Window, canvas, div, point, prelude::*,
    px,
};
use ui::{Icon, IconName, IconSize, Label, LabelCommon as _, LabelSize, h_flex, v_flex};

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
            div()
                .pb_1()
                .border_b_1()
                .border_color(self.theme.colors.outline_variant)
                .child(Label::new(title.into()).size(LabelSize::Default))
                .into_any_element(),
        );
    }

    pub fn label(&mut self, text: impl Into<SharedString>) {
        self.children.push(
            Label::new(text.into())
                .size(LabelSize::Small)
                .into_any_element(),
        );
    }

    pub fn separator(&mut self) {
        self.children.push(
            div()
                .w_full()
                .h(px(1.0))
                .my_1()
                .bg(self.theme.colors.outline_variant)
                .into_any_element(),
        );
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
            .p_2p5()
            .gap_2()
            .rounded(px(8.0))
            .bg(self.theme.colors.surface_container_low)
            .border_1()
            .border_color(self.theme.colors.outline_variant)
            .child(Label::new(title.into()).size(LabelSize::Small))
            .children(sub.children)
            .into_any_element();
        self.children.push(card);
    }

    pub fn kv_row(&mut self, key: impl Into<SharedString>, value: impl Into<SharedString>) {
        self.children.push(
            h_flex()
                .w_full()
                .min_w(px(0.0))
                .justify_between()
                .items_start()
                .gap_2()
                .child(
                    div().flex_shrink_0().child(
                        Label::new(key.into())
                            .size(LabelSize::XSmall)
                            .color(ui::Color::Muted),
                    ),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex()
                        .justify_end()
                        .child(Label::new(value.into()).size(LabelSize::XSmall)),
                )
                .into_any_element(),
        );
    }

    pub fn color_swatch(
        &mut self,
        label: impl Into<SharedString>,
        hex_or_color: gpui::Hsla,
        text: impl Into<SharedString>,
    ) {
        self.children.push(
            h_flex()
                .w_full()
                .justify_between()
                .items_center()
                .gap_2()
                .child(
                    Label::new(label.into())
                        .size(LabelSize::XSmall)
                        .color(ui::Color::Muted),
                )
                .child(
                    h_flex()
                        .gap_1p5()
                        .items_center()
                        .child(
                            div()
                                .w(px(14.0))
                                .h(px(14.0))
                                .rounded(px(4.0))
                                .border_1()
                                .border_color(self.theme.colors.outline)
                                .bg(hex_or_color),
                        )
                        .child(Label::new(text.into()).size(LabelSize::XSmall)),
                )
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
            .gap_2()
            .children(self.children)
            .into_any_element()
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
    let (bg, fg, border) = match tone {
        StatusTone::Neutral => (
            theme.colors.surface_container_high,
            theme.colors.on_surface_variant,
            theme.colors.outline_variant,
        ),
        StatusTone::Accent => (
            theme.colors.primary_container,
            theme.colors.on_primary_container,
            theme.colors.primary,
        ),
        StatusTone::Success => (
            theme.colors.secondary_container,
            theme.colors.on_secondary_container,
            theme.colors.secondary,
        ),
        StatusTone::Warning => (
            theme.colors.primary_container,
            theme.colors.on_primary_container,
            theme.colors.outline,
        ),
        StatusTone::Error => (
            theme.colors.error_container,
            theme.colors.on_error_container,
            theme.colors.error,
        ),
    };

    h_flex()
        .px_2()
        .py_0p5()
        .gap_1()
        .items_center()
        .rounded(px(6.0))
        .bg(bg)
        .border_1()
        .border_color(border)
        .text_color(fg)
        .child(Icon::new(icon).size(IconSize::XSmall))
        .child(Label::new(text.into()).size(LabelSize::XSmall))
        .into_any_element()
}

/// GPU-accelerated data sparkline rendered on `gpui::canvas` with explicit min/max legend and text values.
#[must_use]
pub fn sparkline_chart(
    title: impl Into<SharedString>,
    unit: &str,
    samples: &[f32],
    theme: &OsTheme,
) -> AnyElement {
    let title = title.into();
    let points: Vec<f32> = samples.to_vec();
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

    let line_color = theme.colors.primary;
    let grid_color = theme.colors.outline_variant;

    v_flex()
        .w_full()
        .p_2()
        .gap_1p5()
        .rounded(px(8.0))
        .bg(theme.colors.surface_container_low)
        .border_1()
        .border_color(theme.colors.outline_variant)
        .child(
            h_flex()
                .w_full()
                .justify_between()
                .child(Label::new(title).size(LabelSize::XSmall))
                .child(Label::new(format!("cur {latest:.1}{unit}")).size(LabelSize::XSmall)),
        )
        .child(div().w_full().h(px(56.0)).child(canvas(
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
                .justify_between()
                .child(
                    Label::new(format!("min {min_v:.1}{unit}"))
                        .size(LabelSize::XSmall)
                        .color(ui::Color::Muted),
                )
                .child(
                    Label::new(format!("max {max_v:.1}{unit}"))
                        .size(LabelSize::XSmall)
                        .color(ui::Color::Muted),
                ),
        )
        .into_any_element()
}
