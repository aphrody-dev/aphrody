//! Composable Native Shell (`nsh`) Material 3 Expressive UI Kit anchored to
//! `nsh_base::material3` (`assets/os/google_design_tokens.toml` / `https://design.google/`).
//!
//! Provides:
//! - 12 composable NSH M3 Expressive UI builders (`m3_surface`, `m3_filled_button`,
//!   `m3_tonal_button`, `m3_fab`, `m3_chip`, `m3_card`, `m3_navigation_rail`,
//!   `m3_top_app_bar`, `m3_search_omnibar`, `m3_segmented_tabs`, `m3_switch`,
//!   `m3_aura_badge`) powered by `nsh_base` headless primitives and `nsh_core`
//!   physics springs (`Spatial` 0.9 damping, `Effects` 1.0 damping).
//! - [`export_m3_nsh_tsx_kit`] emitting idiomatic NSH TSX components alongside
//!   4-mode token stylesheet definitions (`Light`, `Dark`, `NeuralObsidian`, `OledPitch`).

use std::fmt;
use std::rc::Rc;

use gpui_ui::nsh::nsh_base::{
    ButtonElement, SwitchElement, TabElement, TabsElement, button, input,
    material3::{
        M3_FONT_PLAIN, M3_SHAPES, M3_STATE_LAYERS, M3Mode, M3RolePalette, M3SpringPreset,
        m3_css_custom_properties, m3_theme_css_vars_json, resolve_m3_role,
    },
    switch, tab, tabs, theme,
};
use gpui_ui::nsh::nsh_core::prelude::*;

/// Surface elevation levels mapped onto Material 3 tonal container roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum M3SurfaceLevel {
    /// Root canvas surface (`md-surface` -> `theme().background`).
    #[default]
    Base,
    /// Dimmed backdrop surface (`md-surface-dim` -> `theme().background`).
    Dim,
    /// Bright elevated surface (`md-surface-bright` -> `theme().popover`).
    Bright,
    /// Lowest-elevation recessed surface (`md-surface-container-lowest` -> `theme().background`).
    ContainerLowest,
    /// Low-elevation container surface (`md-surface-container-low` -> `theme().card`).
    ContainerLow,
    /// Default container surface (`md-surface-container` -> `theme().muted`).
    Container,
    /// High-elevation container surface (`md-surface-container-high` -> `theme().popover`).
    ContainerHigh,
    /// Highest-elevation container surface (`md-surface-container-highest` -> `theme().muted`).
    ContainerHighest,
}

/// Visual container styles for [`m3_card`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum M3CardVariant {
    /// Filled tonal card (`surface_container_highest`).
    #[default]
    Filled,
    /// Elevated surface card with subtle shadow (`surface_container_low`).
    Elevated,
    /// Hairline outlined card (`surface` + `outline_variant` border).
    Outlined,
}

/// Destination item rendered inside [`m3_navigation_rail`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct M3NavItem {
    /// Stable identifier passed to `on_select`.
    pub id: SharedString,
    /// Visible destination label and accessibility name.
    pub label: SharedString,
    /// Compact glyph / monogram displayed inside the active pill indicator.
    pub icon: SharedString,
    /// Optional status counter or tag shown beside the icon.
    pub badge: Option<SharedString>,
}

impl M3NavItem {
    /// Creates a navigation rail destination.
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        icon: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: icon.into(),
            badge: None,
        }
    }

    /// Attaches a compact badge label to this destination.
    pub fn badge(mut self, badge: impl Into<SharedString>) -> Self {
        self.badge = Some(badge.into());
        self
    }
}

/// Segment entry rendered inside [`m3_segmented_tabs`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct M3SegmentItem {
    /// Stable identifier passed to `on_select`.
    pub id: SharedString,
    /// Visible segment label.
    pub label: SharedString,
    /// Whether this segment cannot be selected.
    pub disabled: bool,
}

impl M3SegmentItem {
    /// Creates an enabled segmented tab item.
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            disabled: false,
        }
    }

    /// Marks this segment as disabled or enabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

fn surface_color(
    current: &gpui_ui::nsh::nsh_base::Theme,
    elevation: M3SurfaceLevel,
) -> AdaptiveColor {
    match elevation {
        M3SurfaceLevel::Base => resolve_m3_role(current, "md-surface", |t| t.background),
        M3SurfaceLevel::Dim => resolve_m3_role(current, "md-surface-dim", |t| t.background),
        M3SurfaceLevel::Bright => resolve_m3_role(current, "md-surface-bright", |t| t.popover),
        M3SurfaceLevel::ContainerLowest => {
            resolve_m3_role(current, "md-surface-container-lowest", |t| t.background)
        }
        M3SurfaceLevel::ContainerLow => {
            resolve_m3_role(current, "md-surface-container-low", |t| t.card)
        }
        M3SurfaceLevel::Container => {
            resolve_m3_role(current, "md-surface-container", |t| t.muted)
        }
        M3SurfaceLevel::ContainerHigh => {
            resolve_m3_role(current, "md-surface-container-high", |t| t.popover)
        }
        M3SurfaceLevel::ContainerHighest => {
            resolve_m3_role(current, "md-surface-container-highest", |t| t.muted)
        }
    }
}

fn m3_role(
    md_token: &'static str,
    fallback: fn(&gpui_ui::nsh::nsh_base::Theme) -> AdaptiveColor,
) -> AdaptiveColor {
    let current = theme();
    resolve_m3_role(&current, md_token, fallback)
}

/// 1. Composable Material 3 Expressive tonal surface container (`View`).
pub fn m3_surface(elevation: M3SurfaceLevel) -> View {
    view()
        .bg(move || surface_color(&theme(), elevation))
        .text_color(move || m3_role("md-on-surface", |t| t.foreground))
        .font_family(move || {
            theme()
                .font_sans
                .clone()
                .unwrap_or_else(|| SharedString::from(M3_FONT_PLAIN))
        })
        .transition_colors()
        .ease_spring(M3SpringPreset::EffectsDefault.to_nsh_spring())
}

/// 2. High-emphasis Material 3 Expressive Filled Button (`ButtonElement`) with pill-to-medium
/// pressed shape morph and Spatial Fast spring physics.
pub fn m3_filled_button(label: impl IntoReactive<SharedString>) -> ButtonElement {
    button()
        .flex_row()
        .items_center()
        .justify_center()
        .gap(px(8.0))
        .min_h(px(40.0))
        .px(px(24.0))
        .py(px(10.0))
        .rounded(px(M3_SHAPES.full_px))
        .cursor_pointer()
        .bg(move || m3_role("md-primary", |t| t.primary))
        .transition_all()
        .ease_spring(M3SpringPreset::SpatialFast.to_nsh_spring())
        .hover(|s| s.opacity(1.0 - M3_STATE_LAYERS.hover_opacity))
        .active(|s| {
            s.rounded(px(M3_SHAPES.medium_px))
                .scale(0.98)
                .opacity(1.0 - M3_STATE_LAYERS.pressed_opacity)
        })
        .focus_visible(|s| {
            s.outline(px(M3_STATE_LAYERS.focus_ring_px))
                .outline_color(move || m3_role("md-primary", |t| t.ring))
        })
        .disabled_style(|s| s.opacity(M3_STATE_LAYERS.disabled_content_opacity))
        .child(
            text(label)
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || m3_role("md-on-primary", |t| t.primary_foreground)),
        )
}

/// 3. Medium-emphasis Material 3 Expressive Filled Tonal Button (`ButtonElement`) using
/// `secondary_container` / `on_secondary_container` tokens.
pub fn m3_tonal_button(label: impl IntoReactive<SharedString>) -> ButtonElement {
    button()
        .flex_row()
        .items_center()
        .justify_center()
        .gap(px(8.0))
        .min_h(px(40.0))
        .px(px(20.0))
        .py(px(10.0))
        .rounded(px(M3_SHAPES.full_px))
        .cursor_pointer()
        .bg(move || m3_role("md-secondary-container", |t| t.secondary))
        .transition_all()
        .ease_spring(M3SpringPreset::SpatialFast.to_nsh_spring())
        .hover(|s| s.opacity(1.0 - M3_STATE_LAYERS.hover_opacity))
        .active(|s| {
            s.rounded(px(M3_SHAPES.medium_px))
                .scale(0.98)
                .opacity(1.0 - M3_STATE_LAYERS.pressed_opacity)
        })
        .focus_visible(|s| {
            s.outline(px(M3_STATE_LAYERS.focus_ring_px))
                .outline_color(move || m3_role("md-primary", |t| t.ring))
        })
        .disabled_style(|s| s.opacity(M3_STATE_LAYERS.disabled_content_opacity))
        .child(
            text(label)
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || m3_role("md-on-secondary-container", |t| t.secondary_foreground)),
        )
}

/// 4. Material 3 Expressive Floating Action Button (`ButtonElement`): compact `56x56pt` squircle
/// when `label` is `None`, or Extended FAB when `label` is `Some(...)`.
pub fn m3_fab(
    icon: impl IntoReactive<SharedString>,
    label: Option<SharedString>,
) -> ButtonElement {
    let is_extended = label.is_some();
    let base = button()
        .flex_row()
        .items_center()
        .justify_center()
        .min_w(px(56.0))
        .min_h(px(56.0))
        .when(!is_extended, |b| b.w(px(56.0)).h(px(56.0)))
        .when(is_extended, |b| b.px(px(20.0)).gap(px(12.0)))
        .rounded(px(M3_SHAPES.large_px))
        .cursor_pointer()
        .shadow_md()
        .bg(move || m3_role("md-primary-container", |t| t.primary))
        .transition_all()
        .ease_spring(M3SpringPreset::SpatialFast.to_nsh_spring())
        .hover(|s| s.shadow_lg().scale(1.02))
        .active(|s| s.rounded(px(M3_SHAPES.medium_px)).scale(0.96))
        .focus_visible(|s| {
            s.outline(px(M3_STATE_LAYERS.focus_ring_px))
                .outline_color(move || m3_role("md-primary", |t| t.ring))
        })
        .disabled_style(|s| s.opacity(M3_STATE_LAYERS.disabled_content_opacity))
        .child(
            text(icon)
                .text_lg()
                .font_weight(FontWeight::BOLD)
                .text_color(move || m3_role("md-on-primary-container", |t| t.primary_foreground)),
        );

    match label {
        Some(text_label) => base.accessibility_label(text_label.clone()).child(
            text(text_label)
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || m3_role("md-on-primary-container", |t| t.primary_foreground)),
        ),
        None => base,
    }
}

/// 5. Material 3 Expressive Filter/Action Chip (`ButtonElement`) with reactive selection state.
pub fn m3_chip(
    label: impl IntoReactive<SharedString>,
    selected: impl IntoReactive<bool>,
) -> ButtonElement {
    let selected = selected.into_reactive();
    let sel_attr = selected.clone();
    let sel_bg = selected.clone();
    let sel_border = selected.clone();
    let sel_fg = selected;

    button()
        .selected(sel_attr)
        .flex_row()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .min_h(px(36.0))
        .px(px(14.0))
        .py(px(6.0))
        .rounded(px(M3_SHAPES.small_px))
        .border_1()
        .cursor_pointer()
        .bg(move || {
            if sel_bg.get() {
                m3_role("md-secondary-container", |t| t.secondary)
            } else {
                m3_role("md-surface-container-low", |t| t.card)
            }
        })
        .border_color(move || {
            if sel_border.get() {
                m3_role("md-secondary-container", |t| t.secondary)
            } else {
                m3_role("md-outline-variant", |t| t.border)
            }
        })
        .transition_all()
        .ease_spring(M3SpringPreset::SpatialFast.to_nsh_spring())
        .hover(|s| s.opacity(1.0 - M3_STATE_LAYERS.hover_opacity))
        .active(|s| s.scale(0.97))
        .focus_visible(|s| {
            s.outline(px(M3_STATE_LAYERS.focus_ring_px))
                .outline_color(move || m3_role("md-primary", |t| t.ring))
        })
        .disabled_style(|s| s.opacity(M3_STATE_LAYERS.disabled_content_opacity))
        .child(
            text(label)
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || {
                    if sel_fg.get() {
                        m3_role("md-on-secondary-container", |t| t.secondary_foreground)
                    } else {
                        m3_role("md-on-surface-variant", |t| t.muted_foreground)
                    }
                }),
        )
}

/// 6. Material 3 Expressive Card container (`View`) supporting Filled, Elevated, and Outlined styles.
pub fn m3_card(variant: M3CardVariant) -> View {
    let base = match variant {
        M3CardVariant::Filled => m3_surface(M3SurfaceLevel::ContainerHighest),
        M3CardVariant::Elevated => m3_surface(M3SurfaceLevel::ContainerLow)
            .shadow_sm()
            .elevation(px(2.0)),
        M3CardVariant::Outlined => m3_surface(M3SurfaceLevel::Base)
            .border_1()
            .border_color(move || m3_role("md-outline-variant", |t| t.border)),
    };
    base.flex_col()
        .p(px(16.0))
        .gap(px(8.0))
        .rounded(px(M3_SHAPES.large_px))
}

/// 7. Material 3 Expressive collapsed Navigation Rail (`80pt` wide) with full-pill
/// active indicator (`secondary_container`) and `>= 44pt` touch targets.
pub fn m3_navigation_rail(
    items: Vec<M3NavItem>,
    selected_id: impl IntoReactive<SharedString>,
    on_select: impl Fn(SharedString) + Clone + 'static,
) -> View {
    let selected_id = selected_id.into_reactive();
    let mut rail = m3_surface(M3SurfaceLevel::Container)
        .w(px(80.0))
        .flex_col()
        .items_center()
        .py(px(12.0))
        .px(px(8.0))
        .gap(px(12.0))
        .border_r_1()
        .border_color(move || m3_role("md-outline-variant", |t| t.border))
        .role(AccessibilityRole::Tablist)
        .label("Navigation rail");

    for item in items {
        let id_sel = item.id.clone();
        let id_bg = item.id.clone();
        let id_icon_fg = item.id.clone();
        let id_label_fg = item.id.clone();
        let id_press = item.id.clone();

        let sel_btn = selected_id.clone();
        let sel_bg = selected_id.clone();
        let sel_icon_fg = selected_id.clone();
        let sel_label_fg = selected_id.clone();
        let on_select = on_select.clone();

        let mut pill = view()
            .w(px(56.0))
            .h(px(32.0))
            .flex_row()
            .items_center()
            .justify_center()
            .gap(px(4.0))
            .rounded(px(M3_SHAPES.full_px))
            .bg(move || {
                if sel_bg.get() == id_bg {
                    m3_role("md-secondary-container", |t| t.secondary)
                } else {
                    AdaptiveColor::TRANSPARENT
                }
            })
            .transition_all()
            .ease_spring(M3SpringPreset::SpatialFast.to_nsh_spring())
            .child(
                text(item.icon)
                    .text_sm()
                    .font_weight(FontWeight::BOLD)
                    .text_color(move || {
                        if sel_icon_fg.get() == id_icon_fg {
                            m3_role("md-on-secondary-container", |t| t.secondary_foreground)
                        } else {
                            m3_role("md-on-surface-variant", |t| t.muted_foreground)
                        }
                    }),
            );

        if let Some(badge) = item.badge {
            pill = pill.child(
                view()
                    .px(px(5.0))
                    .py(px(1.0))
                    .rounded(px(M3_SHAPES.full_px))
                    .bg(move || m3_role("md-error", |t| t.destructive))
                    .child(
                        text(badge)
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .text_color(move || m3_role("md-on-error", |t| t.background)),
                    ),
            );
        }

        let nav_button = button()
            .role(AccessibilityRole::Tab)
            .accessibility_label(item.label.clone())
            .selected(move || sel_btn.get() == id_sel)
            .on_press(move || on_select(id_press.clone()))
            .min_w(px(64.0))
            .min_h(px(52.0))
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(4.0))
            .rounded(px(M3_SHAPES.large_px))
            .cursor_pointer()
            .active(|s| s.scale(0.96))
            .focus_visible(|s| {
                s.outline(px(M3_STATE_LAYERS.focus_ring_px))
                    .outline_color(move || m3_role("md-primary", |t| t.ring))
            })
            .child(pill)
            .child(
                text(item.label)
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(move || {
                        if sel_label_fg.get() == id_label_fg {
                            m3_role("md-on-surface", |t| t.foreground)
                        } else {
                            m3_role("md-on-surface-variant", |t| t.muted_foreground)
                        }
                    }),
            );

        rail = rail.child(nav_button);
    }

    rail
}

/// Builder for [`m3_top_app_bar`], implementing [`IntoElement`].
pub struct M3TopAppBar {
    title: Reactive<SharedString>,
    subtitle: Option<Reactive<SharedString>>,
    leading: Option<AnyElement>,
    trailing: Vec<AnyElement>,
    elevated: bool,
}

impl M3TopAppBar {
    /// Sets an optional secondary caption below the title.
    pub fn subtitle(mut self, subtitle: impl IntoReactive<SharedString>) -> Self {
        self.subtitle = Some(subtitle.into_reactive());
        self
    }

    /// Sets a leading navigation/menu element on the left of the bar.
    pub fn leading(mut self, element: impl IntoElement) -> Self {
        self.leading = Some(element.into_element());
        self
    }

    /// Appends a trailing action element on the right of the bar.
    pub fn trailing(mut self, element: impl IntoElement) -> Self {
        self.trailing.push(element.into_element());
        self
    }

    /// Uses elevated `surface_container` background when `true`, or flat `surface` when `false`.
    pub fn elevated(mut self, elevated: bool) -> Self {
        self.elevated = elevated;
        self
    }
}

impl IntoElement for M3TopAppBar {
    fn into_element(self) -> AnyElement {
        let level = if self.elevated {
            M3SurfaceLevel::Container
        } else {
            M3SurfaceLevel::Base
        };

        let title_col = view()
            .flex_1()
            .flex_col()
            .gap(px(2.0))
            .child(
                text(self.title)
                    .role(AccessibilityRole::Header)
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(move || m3_role("md-on-surface", |t| t.foreground)),
            )
            .when_some(self.subtitle, |col, subtitle| {
                col.child(
                    text(subtitle)
                        .text_xs()
                        .text_color(move || m3_role("md-on-surface-variant", |t| t.muted_foreground)),
                )
            });

        m3_surface(level)
            .w_full()
            .min_h(px(64.0))
            .flex_row()
            .items_center()
            .justify_between()
            .px(px(16.0))
            .py(px(10.0))
            .gap(px(12.0))
            .border_b_1()
            .border_color(move || m3_role("md-outline-variant", |t| t.border))
            .child(
                view()
                    .flex_1()
                    .flex_row()
                    .items_center()
                    .gap(px(12.0))
                    .when_some(self.leading, |row, leading| row.child(leading))
                    .child(title_col),
            )
            .when(!self.trailing.is_empty(), |bar| {
                bar.child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .children(self.trailing),
                )
            })
            .into_element()
    }
}

/// 8. Composable Material 3 Expressive Top App Bar builder (`M3TopAppBar`).
pub fn m3_top_app_bar(title: impl IntoReactive<SharedString>) -> M3TopAppBar {
    M3TopAppBar {
        title: title.into_reactive(),
        subtitle: None,
        leading: None,
        trailing: Vec::new(),
        elevated: false,
    }
}

/// Builder for [`m3_search_omnibar`], implementing [`IntoElement`].
pub struct M3SearchOmnibar {
    placeholder: Reactive<String>,
    query: Reactive<String>,
    on_change: Rc<dyn Fn(&str)>,
    on_submit: Option<Rc<dyn Fn(&str)>>,
    mode_badge: Reactive<SharedString>,
    shortcut_hint: Reactive<SharedString>,
}

impl M3SearchOmnibar {
    /// Sets the trailing mode pill label (default `"M3 Expressive"`).
    pub fn mode_badge(mut self, label: impl IntoReactive<SharedString>) -> Self {
        self.mode_badge = label.into_reactive();
        self
    }

    /// Sets the keyboard shortcut hint label (default `"Ctrl+K"`).
    pub fn shortcut_hint(mut self, hint: impl IntoReactive<SharedString>) -> Self {
        self.shortcut_hint = hint.into_reactive();
        self
    }

    /// Attaches an Enter/Return submission callback receiving the input string.
    pub fn on_submit(mut self, handler: impl Fn(&str) + 'static) -> Self {
        self.on_submit = Some(Rc::new(handler));
        self
    }
}

impl IntoElement for M3SearchOmnibar {
    fn into_element(self) -> AnyElement {
        let on_change = self.on_change;
        let on_submit = self.on_submit;

        let mut field = input()
            .flex_1()
            .h(px(36.0))
            .value(self.query)
            .placeholder(self.placeholder)
            .placeholder_color(|| color(m3_role("md-on-surface-variant", |t| t.muted_foreground)))
            .caret_color(|| color(m3_role("md-primary", |t| t.primary)))
            .selection_color(|| color(theme().selection()))
            .text_sm()
            .text_color(move || m3_role("md-on-surface", |t| t.foreground))
            .on_change(move |value: &str| on_change(value));

        if let Some(submit_handler) = on_submit {
            field = field.on_submit(move |value: &str| submit_handler(value));
        }

        m3_surface(M3SurfaceLevel::ContainerHigh)
            .w_full()
            .min_h(px(52.0))
            .flex_row()
            .items_center()
            .justify_between()
            .gap(px(10.0))
            .px(px(16.0))
            .py(px(8.0))
            .rounded(px(M3_SHAPES.full_px))
            .border_1()
            .border_color(move || m3_role("md-outline-variant", |t| t.border))
            .focus_within(|s| s.border_color(move || m3_role("md-primary", |t| t.ring)))
            .child(
                view()
                    .w(px(10.0))
                    .h(px(10.0))
                    .rounded(px(M3_SHAPES.full_px))
                    .bg(move || m3_role("md-primary", |t| t.primary)),
            )
            .child(field)
            .child(
                view()
                    .flex_row()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        view()
                            .px(px(10.0))
                            .py(px(4.0))
                            .rounded(px(M3_SHAPES.full_px))
                            .bg(move || m3_role("md-secondary-container", |t| t.secondary))
                            .child(
                                text(self.mode_badge)
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(move || {
                                        m3_role("md-on-secondary-container", |t| {
                                            t.secondary_foreground
                                        })
                                    }),
                            ),
                    )
                    .child(
                        text(self.shortcut_hint)
                            .text_xs()
                            .text_color(move || {
                                m3_role("md-on-surface-variant", |t| t.muted_foreground)
                            }),
                    ),
            )
            .into_element()
    }
}

/// 9. Material 3 Expressive Search Omnibar (`M3SearchOmnibar`) backed by a native `nsh_base::input()`.
pub fn m3_search_omnibar(
    placeholder: impl IntoReactive<String>,
    query: impl IntoReactive<String>,
    on_change: impl Fn(&str) + 'static,
) -> M3SearchOmnibar {
    M3SearchOmnibar {
        placeholder: placeholder.into_reactive(),
        query: query.into_reactive(),
        on_change: Rc::new(on_change),
        on_submit: None,
        mode_badge: SharedString::from("M3 Expressive").into_reactive(),
        shortcut_hint: SharedString::from("Ctrl+K").into_reactive(),
    }
}

/// 10. Material 3 Expressive Segmented Tabs (`TabsElement`) with pill sliding indicator
/// (`nsh_base::tabs` + `nsh_base::tab`) and arrow-key roving focus.
pub fn m3_segmented_tabs(
    items: Vec<M3SegmentItem>,
    selected_id: impl IntoReactive<SharedString>,
    on_select: impl Fn(SharedString) + Clone + 'static,
) -> TabsElement {
    let selected_id = selected_id.into_reactive();
    let mut bar = tabs()
        .flex_row()
        .items_center()
        .p(px(4.0))
        .gap(px(4.0))
        .rounded(px(M3_SHAPES.full_px))
        .bg(move || m3_role("md-surface-container-low", |t| t.card))
        .border_1()
        .border_color(move || m3_role("md-outline-variant", |t| t.border))
        .indicator_style(|s| {
            s.top(px(4.0))
                .bottom(px(4.0))
                .rounded(px(M3_SHAPES.full_px))
                .bg(move || m3_role("md-secondary-container", |t| t.secondary))
        });

    for item in items {
        let id_sel = item.id.clone();
        let id_fg = item.id.clone();
        let id_press = item.id.clone();
        let sel_attr = selected_id.clone();
        let sel_fg = selected_id.clone();
        let on_select = on_select.clone();

        let segment: TabElement = tab()
            .selected(move || sel_attr.get() == id_sel)
            .disabled(item.disabled)
            .on_press(move || on_select(id_press.clone()))
            .min_h(px(36.0))
            .px(px(16.0))
            .py(px(6.0))
            .flex_row()
            .items_center()
            .justify_center()
            .rounded(px(M3_SHAPES.full_px))
            .cursor_pointer()
            .focus_visible(|s| {
                s.outline(px(M3_STATE_LAYERS.focus_ring_px))
                    .outline_color(move || m3_role("md-primary", |t| t.ring))
            })
            .disabled_style(|s| s.opacity(M3_STATE_LAYERS.disabled_content_opacity))
            .child(
                text(item.label)
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(move || {
                        if sel_fg.get() == id_fg {
                            m3_role("md-on-secondary-container", |t| t.secondary_foreground)
                        } else {
                            m3_role("md-on-surface-variant", |t| t.muted_foreground)
                        }
                    }),
            );
        bar = bar.child(segment);
    }

    bar
}

/// 11. Material 3 Expressive Switch (`SwitchElement`) pre-styled with a `52x32pt` pill track,
/// expanding thumb, Spatial Fast motion, and `>= 44pt` touch hit-target.
pub fn m3_switch() -> SwitchElement {
    switch()
        .flex_row()
        .items_center()
        .gap(px(12.0))
        .min_h(px(44.0))
        .hit_slop(EdgeInsets::all(px(6.0)))
        .cursor_pointer()
        .track_style(|s| {
            s.w(px(52.0))
                .h(px(32.0))
                .p(px(2.0))
                .flex_row()
                .items_center()
                .rounded(px(M3_SHAPES.full_px))
                .border_2()
                .border_color(move || m3_role("md-outline", |t| t.border))
                .bg(move || m3_role("md-surface-container-highest", |t| t.muted))
                .checked(|c| {
                    c.border_color(move || m3_role("md-primary", |t| t.primary))
                        .bg(move || m3_role("md-primary", |t| t.primary))
                })
                .transition_colors()
                .ease_spring(M3SpringPreset::EffectsFast.to_nsh_spring())
        })
        .thumb_style(|s| {
            s.w(px(16.0))
                .h(px(16.0))
                .translate_x(px(4.0))
                .rounded(px(M3_SHAPES.full_px))
                .bg(move || m3_role("md-outline", |t| t.muted_foreground))
                .checked(|c| {
                    c.w(px(24.0))
                        .h(px(24.0))
                        .translate_x(px(20.0))
                        .bg(move || m3_role("md-on-primary", |t| t.primary_foreground))
                })
                .group_active_named("nsh-switch", |a| a.scale_x(1.15))
                .transition_all()
                .ease_spring(M3SpringPreset::SpatialFast.to_nsh_spring())
        })
        .focus_visible(|s| {
            s.outline(px(M3_STATE_LAYERS.focus_ring_px))
                .outline_color(move || m3_role("md-primary", |t| t.ring))
        })
        .disabled_style(|s| s.opacity(M3_STATE_LAYERS.disabled_content_opacity))
}

/// Builder for [`m3_aura_badge`], implementing [`IntoElement`].
pub struct M3AuraBadge {
    label: Reactive<SharedString>,
    detail: Reactive<SharedString>,
    pulsing: bool,
}

impl M3AuraBadge {
    /// Enables or disables the gentle pulse animation on the indicator dot (always
    /// suppressed automatically when `reduce_motion()` is active).
    pub fn pulsing(mut self, pulsing: bool) -> Self {
        self.pulsing = pulsing;
        self
    }
}

impl IntoElement for M3AuraBadge {
    fn into_element(self) -> AnyElement {
        let pulsing = self.pulsing;
        view()
            .flex_row()
            .items_center()
            .gap(px(8.0))
            .px(px(12.0))
            .py(px(6.0))
            .rounded(px(M3_SHAPES.full_px))
            .bg(move || m3_role("md-tertiary-container", |t| t.accent))
            .border_1()
            .border_color(move || m3_role("md-outline-variant", |t| t.border))
            .transition_colors()
            .ease_spring(M3SpringPreset::EffectsDefault.to_nsh_spring())
            .child(
                view()
                    .w(px(8.0))
                    .h(px(8.0))
                    .rounded(px(M3_SHAPES.full_px))
                    .bg(move || m3_role("md-primary", |t| t.primary))
                    .style_when(move || pulsing && !reduce_motion(), |s| s.animate_pulse()),
            )
            .child(
                text(self.label)
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(move || m3_role("md-on-tertiary-container", |t| t.accent_foreground)),
            )
            .child(
                text(self.detail)
                    .text_xs()
                    .text_color(move || m3_role("md-on-surface-variant", |t| t.muted_foreground)),
            )
            .into_element()
    }
}

/// 12. Material 3 Expressive Neural Aura Badge (`M3AuraBadge`) indicating live mode/state.
pub fn m3_aura_badge(
    label: impl IntoReactive<SharedString>,
    detail: impl IntoReactive<SharedString>,
) -> M3AuraBadge {
    M3AuraBadge {
        label: label.into_reactive(),
        detail: detail.into_reactive(),
        pulsing: true,
    }
}

/// Exported bundle containing idiomatic NSH TSX component sources plus 4-mode M3 token stylesheets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct M3NshTsxKit {
    /// Idiomatic NSH TSX module exporting all 12 M3 Expressive components, spring presets, and theme switcher.
    pub tsx: String,
    /// Multi-mode `nsh.json` / `setTheme` JSON stylesheet defining `light`, `dark`, `neural_obsidian`, `oled_pitch`, shapes, and springs.
    pub token_stylesheet_json: String,
    /// CSS custom-property token stylesheet across all 4 modes (`--md-sys-color-*`, `--md-sys-shape-corner-*`, `--md-sys-motion-spring-*`).
    pub token_css: String,
}

impl fmt::Display for M3NshTsxKit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "// === M3_EXPRESSIVE_NSH_TSX ===\n{}\n// === M3_TOKEN_CSS ===\n{}\n// === M3_TOKEN_STYLESHEET_JSON ===\n{}\n",
            self.tsx, self.token_css, self.token_stylesheet_json
        )
    }
}

fn build_token_stylesheet_json() -> String {
    let mut modes = serde_json::Map::new();
    for mode in M3Mode::ALL {
        let palette = M3RolePalette::for_mode(mode);
        let theme_json = m3_theme_css_vars_json(mode);
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&theme_json) {
            modes.insert(mode.slug().to_owned(), parsed);
        } else {
            modes.insert(
                mode.slug().to_owned(),
                serde_json::json!({ "name": mode.label(), "surface": palette.surface.to_hex() }),
            );
        }
    }

    let mut springs = serde_json::Map::new();
    for preset in M3SpringPreset::ALL {
        let spec = preset.spec();
        springs.insert(
            spec.name.to_owned(),
            serde_json::json!({
                "stiffness": spec.stiffness,
                "dampingRatio": spec.damping_ratio,
                "kind": if spec.is_spatial { "spatial" } else { "effects" },
            }),
        );
    }

    serde_json::json!({
        "system": "Material 3 Expressive",
        "sourceOfTruth": "https://design.google/",
        "fontBrand": M3_FONT_PLAIN,
        "modes": modes,
        "shapesPx": {
            "none": M3_SHAPES.none_px,
            "extraSmall": M3_SHAPES.extra_small_px,
            "small": M3_SHAPES.small_px,
            "medium": M3_SHAPES.medium_px,
            "large": M3_SHAPES.large_px,
            "largeIncreased": M3_SHAPES.large_increased_px,
            "extraLarge": M3_SHAPES.extra_large_px,
            "full": M3_SHAPES.full_px,
        },
        "springs": springs,
    })
    .to_string()
}

fn build_nsh_tsx_themes_literal() -> String {
    let mut out = String::from("export const M3_TOKEN_THEMES: Record<M3Mode, Record<string, unknown>> = {\n");
    for mode in M3Mode::ALL {
        let json = m3_theme_css_vars_json(mode);
        out.push_str(&format!("  {}: {},\n", mode.slug(), json));
    }
    out.push_str("};\n");
    out
}

/// 13. Emits an idiomatic NSH TSX M3 Expressive UI kit (`M3Surface`, `M3FilledButton`,
/// `M3TonalButton`, `M3Fab`, `M3Chip`, `M3Card`, `M3NavigationRail`, `M3TopAppBar`,
/// `M3SearchOmnibar`, `M3SegmentedTabs`, `M3Switch`, `M3AuraBadge`) together with
/// 4-mode `cssVars` JSON + CSS custom-property token stylesheets.
pub fn export_m3_nsh_tsx_kit() -> M3NshTsxKit {
    let mut token_css = String::new();
    for mode in M3Mode::ALL {
        token_css.push_str(&m3_css_custom_properties(mode));
    }

    let token_stylesheet_json = build_token_stylesheet_json();
    let themes_literal = build_nsh_tsx_themes_literal();

    let tsx = format!(
        r#"import {{ type JSX, setTheme, signal, Text, token, View }} from "nsh";
import {{ Button, Input, Switch, Tab, Tabs }} from "nsh/base";

export type M3Mode = "light" | "dark" | "neural_obsidian" | "oled_pitch";
export type M3SurfaceLevel =
  | "base"
  | "dim"
  | "bright"
  | "container-lowest"
  | "container-low"
  | "container"
  | "container-high"
  | "container-highest";
export type M3CardVariant = "filled" | "elevated" | "outlined";

export const M3_SPRINGS = {{
  spatialFast: {{ stiffness: 1400, dampingRatio: 0.90 }},
  spatialDefault: {{ stiffness: 700, dampingRatio: 0.90 }},
  spatialSlow: {{ stiffness: 300, dampingRatio: 0.90 }},
  effectsFast: {{ stiffness: 3800, dampingRatio: 1.00 }},
  effectsDefault: {{ stiffness: 1600, dampingRatio: 1.00 }},
  effectsSlow: {{ stiffness: 800, dampingRatio: 1.00 }},
}} as const;

export const M3_SHAPES_PX = {{
  none: 0,
  extraSmall: 4,
  small: 8,
  medium: 12,
  large: 16,
  largeIncreased: 20,
  extraLarge: 28,
  full: 9999,
}} as const;

{themes_literal}
export const activeM3Mode = signal<M3Mode>("dark");

export function applyM3Mode(mode: M3Mode) {{
  activeM3Mode.set(mode);
  setTheme(M3_TOKEN_THEMES[mode]);
}}

function surfaceBg(elevation: M3SurfaceLevel | undefined): string | undefined {{
  const map: Record<M3SurfaceLevel, string> = {{
    base: "md-surface",
    dim: "md-surface-dim",
    bright: "md-surface-bright",
    "container-lowest": "md-surface-container-lowest",
    "container-low": "md-surface-container-low",
    container: "md-surface-container",
    "container-high": "md-surface-container-high",
    "container-highest": "md-surface-container-highest",
  }};
  const key = map[elevation ?? "base"];
  return (token(key) as string | undefined) ?? (token("background") as string | undefined);
}}

export function M3Surface(props: {{
  elevation?: M3SurfaceLevel;
  class?: string;
  children?: JSX.Element;
}}) {{
  return (
    <View
      class={{`bg-background text-foreground transition-colors ${{props.class ?? ""}}`}}
      style={{{{ backgroundColor: surfaceBg(props.elevation) }}}}
    >
      {{props.children}}
    </View>
  );
}}

export function M3FilledButton(props: {{
  label: string;
  disabled?: boolean;
  onPress?: () => void;
}}) {{
  return (
    <Button
      disabled={{props.disabled}}
      onPress={{props.onPress}}
      class="flex-row items-center justify-center rounded-full bg-primary px-6 py-2.5 min-h-[40px] active:rounded-xl active:opacity-90 disabled:opacity-38 transition-all"
    >
      <Text class="text-sm font-semibold text-primary-foreground">{{props.label}}</Text>
    </Button>
  );
}}

export function M3TonalButton(props: {{
  label: string;
  disabled?: boolean;
  onPress?: () => void;
}}) {{
  return (
    <Button
      disabled={{props.disabled}}
      onPress={{props.onPress}}
      class="flex-row items-center justify-center rounded-full bg-secondary px-5 py-2.5 min-h-[40px] active:rounded-xl active:opacity-90 disabled:opacity-38 transition-all"
    >
      <Text class="text-sm font-semibold text-secondary-foreground">{{props.label}}</Text>
    </Button>
  );
}}

export function M3Fab(props: {{
  icon: string;
  label?: string;
  onPress?: () => void;
}}) {{
  return (
    <Button
      accessibilityLabel={{props.label ?? props.icon}}
      onPress={{props.onPress}}
      class="flex-row items-center justify-center gap-3 rounded-2xl bg-primary px-5 min-w-[56px] min-h-[56px] shadow-md active:rounded-xl transition-all"
      style={{{{ backgroundColor: token("md-primary-container") as string | undefined }}}}
    >
      <Text class="text-lg font-bold text-primary-foreground">{{props.icon}}</Text>
      <If when={{Boolean(props.label)}}>
        <Text class="text-sm font-semibold text-primary-foreground">{{props.label}}</Text>
      </If>
    </Button>
  );
}}

export function M3Chip(props: {{
  label: string;
  selected?: boolean;
  onPress?: () => void;
}}) {{
  return (
    <Button
      selected={{props.selected}}
      onPress={{props.onPress}}
      class="flex-row items-center justify-center rounded-lg border border-border px-3.5 py-1.5 min-h-[36px] transition-all"
      classList={{{{
        "bg-secondary border-secondary": Boolean(props.selected),
        "bg-card": !props.selected,
      }}}}
    >
      <Text class={{props.selected ? "text-xs font-semibold text-secondary-foreground" : "text-xs font-semibold text-muted-foreground"}}>
        {{props.label}}
      </Text>
    </Button>
  );
}}

export function M3Card(props: {{
  variant?: M3CardVariant;
  class?: string;
  children?: JSX.Element;
}}) {{
  const variant = () => props.variant ?? "filled";
  return (
    <View
      class={{`rounded-2xl p-4 gap-2 transition-colors ${{props.class ?? ""}}`}}
      classList={{{{
        "bg-muted": variant() === "filled",
        "bg-card shadow-sm": variant() === "elevated",
        "bg-background border border-border": variant() === "outlined",
      }}}}
    >
      {{props.children}}
    </View>
  );
}}

export function M3NavigationRail(props: {{
  items: Array<{{ id: string; label: string; icon: string; badge?: string }}>;
  selectedId: string;
  onSelect: (id: string) => void;
}}) {{
  return (
    <View
      role="tablist"
      accessibilityLabel="Navigation rail"
      class="w-[80px] items-center py-3 px-2 gap-3 bg-muted border-r border-border"
    >
      <Each of={{props.items}} key={{(item) => item.id}}>
        {{(item) => (
          <Button
            role="tab"
            selected={{props.selectedId === item().id}}
            onPress={{() => props.onSelect(item().id)}}
            class="items-center justify-center gap-1 min-w-[64px] min-h-[52px]"
          >
            <View
              class="w-[56px] h-[32px] flex-row items-center justify-center gap-1 rounded-full transition-all"
              classList={{{{ "bg-secondary": props.selectedId === item().id }}}}
            >
              <Text class={{props.selectedId === item().id ? "text-sm font-bold text-secondary-foreground" : "text-sm font-bold text-muted-foreground"}}>
                {{item().icon}}
              </Text>
            </View>
            <Text class={{props.selectedId === item().id ? "text-xs font-medium text-foreground" : "text-xs font-medium text-muted-foreground"}}>
              {{item().label}}
            </Text>
          </Button>
        )}}
      </Each>
    </View>
  );
}}

export function M3TopAppBar(props: {{
  title: string;
  subtitle?: string;
  trailing?: JSX.Element;
}}) {{
  return (
    <View class="w-full min-h-[64px] flex-row items-center justify-between px-4 py-2.5 gap-3 bg-background border-b border-border">
      <View class="flex-1 gap-0.5">
        <Text role="header" class="text-lg font-semibold text-foreground">{{props.title}}</Text>
        <If when={{Boolean(props.subtitle)}}>
          <Text class="text-xs text-muted-foreground">{{props.subtitle}}</Text>
        </If>
      </View>
      {{props.trailing}}
    </View>
  );
}}

export function M3SearchOmnibar(props: {{
  value: string;
  placeholder?: string;
  modeLabel?: string;
  shortcutHint?: string;
  onChange: (value: string) => void;
}}) {{
  return (
    <View class="w-full min-h-[52px] flex-row items-center gap-2.5 px-4 py-2 rounded-full bg-popover border border-border">
      <View class="w-2.5 h-2.5 rounded-full bg-primary" />
      <Input
        value={{props.value}}
        placeholder={{props.placeholder ?? "Search M3 Expressive tokens..."}}
        onChange={{props.onChange}}
        class="flex-1 h-9 text-sm text-foreground"
      />
      <View class="px-2.5 py-1 rounded-full bg-secondary">
        <Text class="text-xs font-semibold text-secondary-foreground">
          {{props.modeLabel ?? "M3 Expressive"}}
        </Text>
      </View>
      <Text class="text-xs text-muted-foreground">{{props.shortcutHint ?? "Ctrl+K"}}</Text>
    </View>
  );
}}

export function M3SegmentedTabs(props: {{
  items: Array<{{ id: string; label: string; disabled?: boolean }}>;
  selectedId: string;
  onSelect: (id: string) => void;
}}) {{
  return (
    <Tabs
      class="flex-row items-center p-1 gap-1 rounded-full bg-card border border-border"
      indicatorClass="inset-y-1 rounded-full bg-secondary"
    >
      <Each of={{props.items}} key={{(item) => item.id}}>
        {{(item) => (
          <Tab
            selected={{props.selectedId === item().id}}
            disabled={{Boolean(item().disabled)}}
            onPress={{() => props.onSelect(item().id)}}
            class="min-h-[36px] px-4 py-1.5 rounded-full items-center justify-center"
          >
            <Text class={{props.selectedId === item().id ? "text-sm font-semibold text-secondary-foreground" : "text-sm font-semibold text-muted-foreground"}}>
              {{item().label}}
            </Text>
          </Tab>
        )}}
      </Each>
    </Tabs>
  );
}}

export function M3Switch(props: {{
  checked: boolean;
  label?: string;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
}}) {{
  return (
    <Switch
      checked={{props.checked}}
      disabled={{props.disabled}}
      onChange={{props.onChange}}
      class="flex-row items-center gap-3 min-h-[44px]"
      trackClass="w-[52px] h-[32px] p-0.5 flex-row items-center rounded-full border-2 border-border bg-muted checked:border-primary checked:bg-primary transition-colors"
      thumbClass="w-4 h-4 translate-x-1 rounded-full bg-muted-foreground checked:w-6 checked:h-6 checked:translate-x-5 checked:bg-primary-foreground transition-all"
    >
      <If when={{Boolean(props.label)}}>
        <Text class="text-sm text-foreground">{{props.label}}</Text>
      </If>
    </Switch>
  );
}}

export function M3AuraBadge(props: {{
  label: string;
  detail: string;
}}) {{
  return (
    <View class="flex-row items-center gap-2 px-3 py-1.5 rounded-full bg-accent border border-border">
      <View class="w-2 h-2 rounded-full bg-primary animate-pulse" />
      <Text class="text-xs font-semibold text-accent-foreground">{{props.label}}</Text>
      <Text class="text-xs text-muted-foreground">{{props.detail}}</Text>
    </View>
  );
}}
"#
    );

    M3NshTsxKit {
        tsx,
        token_stylesheet_json,
        token_css,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_ui::nsh::nsh_base::material3::{WCAG_AA_MIN_CONTRAST, m3_theme, wcag_contrast_ratio};
    use gpui_ui::nsh::nsh_base::{self, Theme};
    use gpui_ui::nsh::nsh_bundler;
    use gpui_ui::nsh::nsh_core::{ApproxTextEngine, MountBatch, element::Root};
    use gpui_ui::nsh::nsh_engine::{App, EngineConfig, LocalEngine, MountSink};
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    #[test]
    fn all_four_m3_modes_and_springs_meet_wcag_and_physics_contracts() {
        for mode in M3Mode::ALL {
            let palette = M3RolePalette::for_mode(mode);
            assert!(palette.verify_wcag_aa().is_ok());

            let theme = m3_theme(mode);
            let fg_bg = wcag_contrast_ratio(theme.foreground.dark, theme.background.dark);
            let pri = wcag_contrast_ratio(theme.primary_foreground.dark, theme.primary.dark);
            assert!(fg_bg >= WCAG_AA_MIN_CONTRAST);
            assert!(pri >= WCAG_AA_MIN_CONTRAST);
        }

        assert_eq!(M3SpringPreset::SpatialFast.spec().stiffness, 1400.0);
        assert_eq!(M3SpringPreset::SpatialFast.spec().damping_ratio, 0.90);
        assert_eq!(M3SpringPreset::SpatialDefault.spec().stiffness, 700.0);
        assert_eq!(M3SpringPreset::SpatialSlow.spec().stiffness, 300.0);
        assert_eq!(M3SpringPreset::EffectsFast.spec().stiffness, 3800.0);
        assert_eq!(M3SpringPreset::EffectsFast.spec().damping_ratio, 1.00);
        assert_eq!(M3SpringPreset::EffectsDefault.spec().stiffness, 1600.0);
        assert_eq!(M3SpringPreset::EffectsSlow.spec().stiffness, 800.0);
    }

    struct TestMountSink {
        batches: Arc<AtomicUsize>,
        items: Arc<AtomicUsize>,
        errors: Arc<Mutex<Vec<String>>>,
    }

    impl MountSink for TestMountSink {
        fn submit(&self, batch: MountBatch) {
            self.batches.fetch_add(1, Ordering::Relaxed);
            self.items.fetch_add(batch.items.len(), Ordering::Relaxed);
        }

        fn report_error(&self, message: &str) {
            if let Ok(mut guard) = self.errors.lock() {
                guard.push(message.to_owned());
            }
        }
    }

    #[test]
    fn all_twelve_nsh_builders_construct_cleanly() {
        for mode in M3Mode::ALL {
            let batches = Arc::new(AtomicUsize::new(0));
            let items = Arc::new(AtomicUsize::new(0));
            let errors = Arc::new(Mutex::new(Vec::new()));

            let sink = TestMountSink {
                batches: Arc::clone(&batches),
                items: Arc::clone(&items),
                errors: Arc::clone(&errors),
            };

            let root = Root::of::<KitTestRoot>().tokens(m3_theme(mode).into_tokens());
            let mut config = EngineConfig::new(root);
            config.ui_sets = vec![nsh_base::registry()];

            let engine = LocalEngine::start(config, ApproxTextEngine, sink);
            assert!(engine.is_some(), "LocalEngine failed to mount KitTestRoot for {mode:?}");
            assert!(
                batches.load(Ordering::Relaxed) >= 1,
                "expected at least 1 mount batch for {mode:?}"
            );
            assert!(
                items.load(Ordering::Relaxed) > 0,
                "expected non-empty mount items for {mode:?}"
            );
            if let Ok(err_list) = errors.lock() {
                assert!(
                    err_list.is_empty(),
                    "unexpected NSH engine errors for {mode:?}: {err_list:?}"
                );
            }
            drop(engine);
        }
    }

    #[test]
    fn export_m3_nsh_tsx_kit_emits_valid_tsx_and_four_mode_stylesheets() {
        let kit = export_m3_nsh_tsx_kit();

        // All 12 exported TSX components + spring tokens + 4 modes present
        for symbol in [
            "M3Surface",
            "M3FilledButton",
            "M3TonalButton",
            "M3Fab",
            "M3Chip",
            "M3Card",
            "M3NavigationRail",
            "M3TopAppBar",
            "M3SearchOmnibar",
            "M3SegmentedTabs",
            "M3Switch",
            "M3AuraBadge",
            "M3_SPRINGS",
            "M3_TOKEN_THEMES",
            "applyM3Mode",
            "spatialFast",
            "effectsFast",
        ] {
            assert!(
                kit.tsx.contains(symbol),
                "exported TSX missing symbol `{symbol}`"
            );
        }

        // Token stylesheet JSON parses cleanly and carries all 4 modes + springs + shapes
        let json_val: serde_json::Value =
            serde_json::from_str(&kit.token_stylesheet_json).unwrap();
        assert_eq!(json_val["system"], "Material 3 Expressive");
        for mode in M3Mode::ALL {
            let mode_obj = &json_val["modes"][mode.slug()];
            assert!(mode_obj.is_object(), "missing mode {}", mode.slug());
            let parsed_theme = Theme::from_json(&mode_obj.to_string()).unwrap();
            assert_eq!(
                parsed_theme.background.dark,
                M3RolePalette::for_mode(mode).surface
            );
        }

        // CSS custom properties include all 4 modes and spring tokens
        for selector in [
            "[data-m3-mode=\"light\"]",
            "[data-m3-mode=\"dark\"]",
            "[data-m3-mode=\"neural_obsidian\"]",
            "[data-m3-mode=\"oled_pitch\"]",
            "--md-sys-color-surface-container:",
            "--md-sys-shape-corner-extra-large:",
            "--md-sys-motion-spring-spatial-fast-stiffness: 1400.0;",
            "--md-sys-motion-spring-effects-fast-stiffness: 3800.0;",
        ] {
            assert!(
                kit.token_css.contains(selector),
                "token_css missing `{selector}`"
            );
        }

        // Compile the emitted TSX module with the real NSH bundler!
        let ui_sets = [nsh_base::registry().into()];
        let compiled = nsh_bundler::compile_module("M3ExpressiveKit.tsx", &kit.tsx, &ui_sets);
        assert!(
            compiled.is_ok(),
            "NSH bundler failed to compile exported M3 TSX kit: {compiled:?}"
        );
    }

    struct KitTestRoot;

    impl Component for KitTestRoot {
        type Props = ();
        type Event = NoEvent;

        fn view(_props: Self::Props, _cx: Scope<Self::Event>) -> impl IntoElement {
            let nav_items = vec![
                M3NavItem::new("home", "Home", "H").badge("2"),
                M3NavItem::new("tokens", "Tokens", "T"),
            ];
            let tab_items = vec![
                M3SegmentItem::new("a", "Alpha"),
                M3SegmentItem::new("b", "Beta"),
                M3SegmentItem::new("c", "Locked").disabled(true),
            ];
            let nav_sel = signal(SharedString::from("home"));
            let tab_sel = signal(SharedString::from("a"));
            let selected_chip = signal(true);
            let query = signal(String::new());
            let checked = signal(false);

            m3_surface(M3SurfaceLevel::Base)
                .flex_1()
                .flex_col()
                .child(
                    m3_top_app_bar("M3 Test")
                        .subtitle("Google Design M3 Expressive")
                        .elevated(true)
                        .trailing(m3_aura_badge("M3", "4 modes")),
                )
                .child(
                    m3_surface(M3SurfaceLevel::Container)
                        .flex_row()
                        .flex_1()
                        .child(m3_navigation_rail(nav_items, nav_sel, move |s| {
                            nav_sel.set(s)
                        }))
                        .child(
                            m3_card(M3CardVariant::Elevated)
                                .flex_1()
                                .child(
                                    m3_search_omnibar("Search", query, move |s| {
                                        query.set(s.to_owned())
                                    })
                                    .mode_badge("Neural Obsidian")
                                    .shortcut_hint("Ctrl+K"),
                                )
                                .child(m3_filled_button("Filled"))
                                .child(m3_tonal_button("Tonal"))
                                .child(m3_fab("+", None))
                                .child(m3_fab("+", Some(SharedString::from("Create"))))
                                .child(m3_chip("Chip", selected_chip))
                                .child(m3_segmented_tabs(tab_items, tab_sel, move |s| {
                                    tab_sel.set(s)
                                }))
                                .child(
                                    m3_switch()
                                        .checked(checked)
                                        .on_change(move |v: &bool| checked.set(*v)),
                                )
                                .child(m3_aura_badge("Aura", "Ready").pulsing(true))
                                .child(m3_card(M3CardVariant::Filled))
                                .child(
                                    m3_surface(M3SurfaceLevel::ContainerHigh)
                                        .child(m3_card(M3CardVariant::Outlined)),
                                ),
                        ),
                )
        }
    }

    #[test]
    fn m3_theme_tokens_work_with_nsh_engine_config() {
        let root = Root::of::<KitTestRoot>().tokens(m3_theme(M3Mode::OledPitch).into_tokens());
        assert!(root.tokens_ref().is_some());
        let config = EngineConfig::new(root);
        assert!(matches!(
            config.app,
            App::Rust(ref app_root) if app_root.tokens_ref().is_some()
        ));
    }
}
