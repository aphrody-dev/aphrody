use gpui::{
    App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, px,
};

use crate::starter_pack::ShadcnGpuiTheme;

#[derive(Debug, Clone)]
pub struct PopoverMenuItem {
    pub id: SharedString,
    pub title: SharedString,
    pub subtitle: SharedString,
    pub badge: Option<SharedString>,
    pub selected: bool,
}

impl PopoverMenuItem {
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        subtitle: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            subtitle: subtitle.into(),
            badge: None,
            selected: false,
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn badge(mut self, badge: impl Into<SharedString>) -> Self {
        self.badge = Some(badge.into());
        self
    }
}

#[derive(IntoElement)]
pub struct ExpressiveOmnibar {
    prompt_text: SharedString,
    mode_label: SharedString,
    shortcut_hint: SharedString,
    theme: ShadcnGpuiTheme,
}

impl ExpressiveOmnibar {
    pub fn new(prompt_text: impl Into<SharedString>, theme: ShadcnGpuiTheme) -> Self {
        Self {
            prompt_text: prompt_text.into(),
            mode_label: SharedString::from("Neural Expressive"),
            shortcut_hint: SharedString::from("Ctrl+K"),
            theme,
        }
    }

    pub fn mode_label(mut self, label: impl Into<SharedString>) -> Self {
        self.mode_label = label.into();
        self
    }

    pub fn shortcut_hint(mut self, hint: impl Into<SharedString>) -> Self {
        self.shortcut_hint = hint.into();
        self
    }
}

impl RenderOnce for ExpressiveOmnibar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .px_4()
            .py_2p5()
            .rounded(px(999.0))
            .bg(self.theme.popover)
            .border_1()
            .border_color(self.theme.primary)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .w(px(10.0))
                            .h(px(10.0))
                            .rounded(px(99.0))
                            .bg(self.theme.primary),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(self.theme.foreground)
                            .child(self.prompt_text),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .px_2p5()
                            .py_0p5()
                            .rounded(px(999.0))
                            .bg(self.theme.secondary)
                            .text_xs()
                            .text_color(self.theme.secondary_foreground)
                            .child(self.mode_label),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(self.theme.muted_foreground)
                            .child(self.shortcut_hint),
                    ),
            )
    }
}

#[derive(IntoElement)]
pub struct CascadingPopoverCard {
    heading: SharedString,
    items: Vec<PopoverMenuItem>,
    theme: ShadcnGpuiTheme,
}

impl CascadingPopoverCard {
    pub fn new(
        heading: impl Into<SharedString>,
        items: Vec<PopoverMenuItem>,
        theme: ShadcnGpuiTheme,
    ) -> Self {
        Self {
            heading: heading.into(),
            items,
            theme,
        }
    }
}

impl RenderOnce for CascadingPopoverCard {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut container = div()
            .flex()
            .flex_col()
            .gap_1p5()
            .p_3()
            .w(px(300.0))
            .rounded(px(self.theme.radius_xl_px))
            .bg(self.theme.popover)
            .border_1()
            .border_color(self.theme.border)
            .child(
                div()
                    .px_2()
                    .py_1()
                    .text_xs()
                    .text_color(self.theme.muted_foreground)
                    .child(self.heading),
            );

        for item in self.items {
            let row_bg = if item.selected {
                self.theme.secondary
            } else {
                self.theme.muted
            };
            let title_fg = if item.selected {
                self.theme.secondary_foreground
            } else {
                self.theme.foreground
            };
            let mut row = div()
                .flex()
                .items_center()
                .justify_between()
                .px_3()
                .py_2()
                .rounded(px(self.theme.radius_lg_px))
                .bg(row_bg)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .child(div().text_sm().text_color(title_fg).child(item.title))
                        .child(
                            div()
                                .text_xs()
                                .text_color(self.theme.muted_foreground)
                                .child(item.subtitle),
                        ),
                );
            if let Some(badge) = item.badge {
                row = row.child(
                    div()
                        .px_2()
                        .py_0p5()
                        .rounded(px(999.0))
                        .bg(self.theme.accent)
                        .text_xs()
                        .text_color(self.theme.accent_foreground)
                        .child(badge),
                );
            }
            container = container.child(row);
        }

        container
    }
}

#[derive(IntoElement)]
pub struct GroundingSourceCard {
    domain: SharedString,
    title: SharedString,
    snippet: SharedString,
    theme: ShadcnGpuiTheme,
}

impl GroundingSourceCard {
    pub fn new(
        domain: impl Into<SharedString>,
        title: impl Into<SharedString>,
        snippet: impl Into<SharedString>,
        theme: ShadcnGpuiTheme,
    ) -> Self {
        Self {
            domain: domain.into(),
            title: title.into(),
            snippet: snippet.into(),
            theme,
        }
    }
}

impl RenderOnce for GroundingSourceCard {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_1p5()
            .p_3()
            .rounded(px(self.theme.radius_lg_px))
            .bg(self.theme.card)
            .border_1()
            .border_color(self.theme.border)
            .child(
                div()
                    .px_2()
                    .py_0p5()
                    .rounded(px(999.0))
                    .bg(self.theme.muted)
                    .text_xs()
                    .text_color(self.theme.primary)
                    .child(self.domain),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(self.theme.card_foreground)
                    .child(self.title),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(self.theme.muted_foreground)
                    .child(self.snippet),
            )
    }
}

#[derive(IntoElement)]
pub struct SegmentedStudioBar {
    segments: Vec<(SharedString, bool)>,
    theme: ShadcnGpuiTheme,
}

impl SegmentedStudioBar {
    pub fn new(segments: Vec<(SharedString, bool)>, theme: ShadcnGpuiTheme) -> Self {
        Self { segments, theme }
    }
}

impl RenderOnce for SegmentedStudioBar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut bar = div()
            .flex()
            .items_center()
            .gap_1()
            .p_1()
            .rounded(px(999.0))
            .bg(self.theme.card)
            .border_1()
            .border_color(self.theme.border);

        for (label, active) in self.segments {
            let bg = if active {
                self.theme.primary
            } else {
                self.theme.muted
            };
            let fg = if active {
                self.theme.primary_foreground
            } else {
                self.theme.muted_foreground
            };
            bar = bar.child(
                div()
                    .px_3()
                    .py_1()
                    .rounded(px(999.0))
                    .bg(bg)
                    .text_xs()
                    .text_color(fg)
                    .child(label),
            );
        }

        bar
    }
}

#[derive(IntoElement)]
pub struct ThinkingAuraBadge {
    label: SharedString,
    detail: SharedString,
    theme: ShadcnGpuiTheme,
}

impl ThinkingAuraBadge {
    pub fn new(
        label: impl Into<SharedString>,
        detail: impl Into<SharedString>,
        theme: ShadcnGpuiTheme,
    ) -> Self {
        Self {
            label: label.into(),
            detail: detail.into(),
            theme,
        }
    }
}

impl RenderOnce for ThinkingAuraBadge {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_1()
            .rounded(px(999.0))
            .bg(self.theme.accent)
            .border_1()
            .border_color(self.theme.ring)
            .child(
                div()
                    .w(px(8.0))
                    .h(px(8.0))
                    .rounded(px(99.0))
                    .bg(self.theme.primary),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(self.theme.accent_foreground)
                    .child(self.label),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(self.theme.muted_foreground)
                    .child(self.detail),
            )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MobileNavTab {
    pub id: SharedString,
    pub label: SharedString,
    pub badge: Option<SharedString>,
    pub selected: bool,
}

impl MobileNavTab {
    /// Creates a new mobile navigation destination adhering to `>= 44x44pt` hit targets.
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            badge: None,
            selected: false,
        }
    }

    #[must_use]
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    #[must_use]
    pub fn badge(mut self, badge: impl Into<SharedString>) -> Self {
        self.badge = Some(badge.into());
        self
    }
}

/// Material 3 Expressive bottom navigation bar for `Compact` (`< 600px`) iOS and Android shells.
#[derive(IntoElement)]
pub struct MobileBottomNavBar {
    tabs: Vec<MobileNavTab>,
    theme: ShadcnGpuiTheme,
}

impl MobileBottomNavBar {
    pub fn new(tabs: Vec<MobileNavTab>, theme: ShadcnGpuiTheme) -> Self {
        Self { tabs, theme }
    }
}

impl RenderOnce for MobileBottomNavBar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut bar = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_1p5()
            .min_h(px(64.0))
            .px_3()
            .py_2()
            .bg(self.theme.card)
            .border_t_1()
            .border_color(self.theme.border);

        for tab in self.tabs {
            let bg = if tab.selected {
                self.theme.primary
            } else {
                self.theme.muted
            };
            let fg = if tab.selected {
                self.theme.primary_foreground
            } else {
                self.theme.muted_foreground
            };
            let mut item = div()
                .flex()
                .flex_1()
                .items_center()
                .justify_center()
                .gap_1()
                .min_w(px(44.0))
                .min_h(px(44.0))
                .px_2()
                .rounded(px(999.0))
                .bg(bg)
                .text_xs()
                .text_color(fg)
                .child(tab.label);
            if let Some(badge) = tab.badge {
                item = item.child(
                    div()
                        .px_1p5()
                        .py_0p5()
                        .rounded(px(999.0))
                        .bg(self.theme.accent)
                        .text_xs()
                        .text_color(self.theme.accent_foreground)
                        .child(badge),
                );
            }
            bar = bar.child(item);
        }

        bar
    }
}

/// Roadmap horizon card representing one phase (`H0`..`H3`) across Website, Desktop & Mobile.
#[derive(IntoElement)]
pub struct RoadmapHorizonCard {
    horizon_id: SharedString,
    window_label: SharedString,
    status_label: SharedString,
    title: SharedString,
    summary: SharedString,
    highlighted: bool,
    theme: ShadcnGpuiTheme,
}

impl RoadmapHorizonCard {
    pub fn new(
        horizon_id: impl Into<SharedString>,
        window_label: impl Into<SharedString>,
        status_label: impl Into<SharedString>,
        title: impl Into<SharedString>,
        summary: impl Into<SharedString>,
        theme: ShadcnGpuiTheme,
    ) -> Self {
        Self {
            horizon_id: horizon_id.into(),
            window_label: window_label.into(),
            status_label: status_label.into(),
            title: title.into(),
            summary: summary.into(),
            highlighted: false,
            theme,
        }
    }

    #[must_use]
    pub fn highlighted(mut self, highlighted: bool) -> Self {
        self.highlighted = highlighted;
        self
    }
}

impl RenderOnce for RoadmapHorizonCard {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let border = if self.highlighted {
            self.theme.primary
        } else {
            self.theme.border
        };
        let pill_bg = if self.highlighted {
            self.theme.primary
        } else {
            self.theme.secondary
        };
        let pill_fg = if self.highlighted {
            self.theme.primary_foreground
        } else {
            self.theme.secondary_foreground
        };
        div()
            .flex()
            .flex_col()
            .gap_1p5()
            .p_3()
            .rounded(px(self.theme.radius_lg_px))
            .bg(self.theme.card)
            .border_1()
            .border_color(border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .px_2()
                            .py_0p5()
                            .rounded(px(999.0))
                            .bg(pill_bg)
                            .text_xs()
                            .text_color(pill_fg)
                            .child(format!("{} · {}", self.horizon_id, self.window_label)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(self.theme.muted_foreground)
                            .child(self.status_label),
                    ),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(self.theme.card_foreground)
                    .child(self.title),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(self.theme.muted_foreground)
                    .child(self.summary),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cosmic_theme::{AppearanceMode, contrast_ratio};

    #[test]
    fn google_design_components_preserve_wcag_aa_contrast_in_light_and_dark() -> anyhow::Result<()> {
        for mode in [AppearanceMode::Dark, AppearanceMode::Light] {
            let theme = ShadcnGpuiTheme::from_bundled(mode)?;
            assert!(contrast_ratio(theme.foreground, theme.popover) >= 4.5);
            assert!(contrast_ratio(theme.primary_foreground, theme.primary) >= 4.5);
            let items = vec![
                PopoverMenuItem::new("fast", "Fast", "Answers quickly").selected(true),
                PopoverMenuItem::new("pro", "Pro", "Thinks deeper for complex math & code")
                    .badge("3.1"),
            ];
            assert_eq!(items.len(), 2);
            assert!(items[0].selected);

            let tabs = vec![
                MobileNavTab::new("agent", "Agent").selected(true).badge("2"),
                MobileNavTab::new("diffs", "Diffs"),
                MobileNavTab::new("fleet", "Fleet"),
                MobileNavTab::new("craft", "Craft"),
            ];
            let _nav = MobileBottomNavBar::new(tabs.clone(), theme);
            let _roadmap = RoadmapHorizonCard::new(
                "H1",
                "2027-Q1",
                "ACTIVE",
                "Figma DTCG + Stitch Studio GA",
                "4-mode tokens and 12 cross-platform maquettes",
                theme,
            )
            .highlighted(true);
            assert_eq!(tabs.len(), 4);
        }
        Ok(())
    }
}
