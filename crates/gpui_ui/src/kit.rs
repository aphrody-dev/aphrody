//! Unified GPUI Kit (`0.10.0`) & Material 3 Expressive integration for `gpui_ui`.
//!
//! Links `gpui_kit`, `gpui_base`, `gpui_component`, `gpui_kit_assets`, and `gpui_fps`
//! directly to Aphrody's single `crates/gpui` engine and bridges Aphrody's `theme::Theme`
//! with Google Design Material 3 Expressive (`gpui_component::material3`).

use std::sync::Arc;

use gpui::{App, WindowAppearance, WindowBackgroundAppearance};
use syntax_theme::SyntaxTheme;
use theme::{
    AccentColors, ActiveTheme as _, Appearance as GpuiAppearance, GlobalTheme, PlayerColors,
    StatusColors, SystemColors, Theme as GpuiTheme, ThemeColors, ThemeStyles,
};

pub use gpui_base;
pub use gpui_component;
pub use gpui_component::material3::{
    self, M3ActiveTheme, M3AppearanceMode, M3ColorRoles, M3ContrastReport, M3DensityTokens,
    M3DialogSheet, M3DialogSheetPlacement, M3ElevatedCard, M3ExpressiveSwitch, M3ExtendedFab,
    M3Fab, M3FabColor, M3FabSize, M3FilledButton, M3FilledCard, M3FilterChip,
    M3GoogleSansFlexAxes, M3NavDestination, M3NavigationBar, M3NavigationRail, M3OutlinedButton,
    M3OutlinedCard, M3SearchBar, M3SegmentItem, M3SegmentedButtonBar, M3ShapeTokens, M3SpringSpec,
    M3SpringToken, M3StateLayerTokens, M3TonalButton, M3TopAppBar, M3TopAppBarVariant,
    M3TypographyScale, M3WindowSizeClass, active_m3_theme, apply_m3_theme,
    compose15::{
        self, COMPOSE_BOM_VERSION, COMPOSE_M3_BLOG_URL, COMPOSE_M3_VERSION, COMPOSE_VERSION,
        FIGMA_M3_DESIGN_KIT_FILE_ID, FIGMA_M3_DESIGN_KIT_URL, M3_EXPRESSIVE_WGSL,
        M3_PARAMETRIC_BLOG_URL, M3BottomSheetAnchor, M3ButtonGroup, M3ButtonGroupItem,
        M3ButtonSize, M3Compose15Catalog, M3ComposeParityEntry, M3ExpressiveMenuItem,
        M3ExpressiveRuntimeConfig, M3ExpressiveTextField, M3ExpressiveThemeScope,
        M3FloatingToolbar, M3FloatingToolbarColor, M3LabelPosition, M3ListVerticalAlignment,
        M3LoadingIndicator, M3MaterialShape, M3MenuItemSelectionKind, M3MotionScheme,
        M3PopupAnchor, M3RangeSliderState, M3RippleThemeConfig, M3Scrim, M3ScrollField,
        M3SegmentPosition, M3SegmentedListItem, M3SliderState, M3SplitButton,
        M3StatefulRangeSlider, M3StatefulSlider, M3TextObfuscationMode, M3TimePickerDisplayMode,
        M3ToggleButton, M3ToggleButtonVariant, M3VibrantTimePicker,
        M3WavyCircularProgressIndicator, M3WavyLinearProgressIndicator,
        active_m3_expressive_config, apply_m3_expressive_theme,
        m3_typography_with_default_font_family,
    },
    m3_accessible_foreground, m3_contrast_ratio, m3_hsla_to_hex, m3_manifest_metadata,
    m3_meets_wcag_aa, m3_motion_tokens, m3_theme_config, m3_verify_mode_contrast,
    parametric::{
        self, M3ParamPreset, M3ParamTarget, M3ParamTetherRules, M3ParametricPrototyperView,
        M3ParametricState, M3ParametricVariables, M3SpecOverlayConfig, M3SweetSpotCell,
        M3SweetSpotGrid, M3SweetSpotVerdict,
    },
};
pub use gpui_fps;
pub use gpui_kit;
pub use gpui_kit_assets;

use gpui_component::{Theme as KitTheme, ThemeConfig as KitThemeConfig, ThemeMode as KitThemeMode};

/// Maps an Aphrody `theme::Appearance` to the baseline Material 3 Expressive mode.
#[must_use]
pub const fn kit_m3_appearance_from_gpui(appearance: GpuiAppearance) -> M3AppearanceMode {
    match appearance {
        GpuiAppearance::Light => M3AppearanceMode::Light,
        GpuiAppearance::Dark => M3AppearanceMode::Dark,
    }
}

/// Converts an Aphrody [`theme::Theme`] into a `gpui-component` [`ThemeConfig`] seeded by Material 3 Expressive tokens.
#[must_use]
pub fn kit_m3_theme_config_from_gpui(gpui_theme: &GpuiTheme) -> KitThemeConfig {
    let base_mode = kit_m3_appearance_from_gpui(gpui_theme.appearance);
    let mut config = m3_theme_config(base_mode);
    let active_m3 = M3ActiveTheme::for_mode(base_mode);
    let src_colors = &gpui_theme.styles.colors;
    let src_status = &gpui_theme.styles.status;

    config.name = gpui_theme.name.clone();
    config.mode = match gpui_theme.appearance {
        GpuiAppearance::Light => KitThemeMode::Light,
        GpuiAppearance::Dark => KitThemeMode::Dark,
    };

    let hex = m3_hsla_to_hex;
    let primary_fg = m3_accessible_foreground(
        src_colors.text_accent,
        active_m3.colors.on_primary,
        src_colors.text,
    );

    config.colors.background = Some(hex(src_colors.background));
    config.colors.foreground = Some(hex(src_colors.text));
    config.colors.border = Some(hex(src_colors.border));
    config.colors.input = Some(hex(src_colors.border_focused));
    config.colors.ring = Some(hex(src_colors.border_focused));
    config.colors.caret = Some(hex(src_colors.text_accent));

    config.colors.primary = Some(hex(src_colors.text_accent));
    config.colors.primary_foreground = Some(hex(primary_fg));
    config.colors.button_primary = Some(hex(src_colors.text_accent));
    config.colors.button_primary_foreground = Some(hex(primary_fg));

    config.colors.secondary = Some(hex(src_colors.element_background));
    config.colors.secondary_foreground = Some(hex(src_colors.text));
    config.colors.secondary_hover = Some(hex(src_colors.element_hover));
    config.colors.secondary_active = Some(hex(src_colors.element_active));

    config.colors.muted = Some(hex(src_colors.surface_background));
    config.colors.muted_foreground = Some(hex(src_colors.text_muted));
    config.colors.popover = Some(hex(src_colors.elevated_surface_background));
    config.colors.popover_foreground = Some(hex(src_colors.text));

    config.colors.list = Some(hex(src_colors.surface_background));
    config.colors.list_hover = Some(hex(src_colors.element_hover));
    config.colors.list_active = Some(hex(src_colors.element_selection_background));
    config.colors.list_active_border = Some(hex(src_colors.border_selected));

    config.colors.sidebar = Some(hex(src_colors.panel_background));
    config.colors.sidebar_foreground = Some(hex(src_colors.text));
    config.colors.sidebar_border = Some(hex(src_colors.border));

    config.colors.tab = Some(hex(src_colors.tab_inactive_background));
    config.colors.tab_bar = Some(hex(src_colors.tab_bar_background));
    config.colors.tab_active = Some(hex(src_colors.tab_active_background));
    config.colors.title_bar = Some(hex(src_colors.title_bar_background));
    config.colors.status_bar = Some(hex(src_colors.status_bar_background));
    config.colors.window_border = Some(hex(src_colors.border));

    config.colors.danger = Some(hex(src_status.error));
    config.colors.info = Some(hex(src_status.info));
    config.colors.warning = Some(hex(src_status.warning));
    config.colors.success = Some(hex(src_status.success));

    config
}

/// Converts a Material 3 Expressive `mode` into a synchronized Aphrody [`theme::Theme`].
#[must_use]
pub fn kit_theme_to_gpui_theme(
    mode: M3AppearanceMode,
    syntax: Option<Arc<SyntaxTheme>>,
) -> Arc<GpuiTheme> {
    let m3 = M3ActiveTheme::for_mode(mode);
    let roles = m3.colors;
    let state = m3.state;
    let is_dark = mode.is_dark();

    let mut colors = if is_dark {
        ThemeColors::dark()
    } else {
        ThemeColors::light()
    };

    colors.border = roles.outline;
    colors.border_variant = roles.outline_variant;
    colors.border_focused = roles.primary;
    colors.border_selected = roles.primary;
    colors.border_disabled = roles.on_surface.alpha(state.disabled_container_opacity);

    colors.background = roles.background;
    colors.surface_background = roles.surface_container_low;
    colors.elevated_surface_background = roles.surface_container_high;

    colors.element_background = roles.surface_container;
    colors.element_hover =
        roles.state_layer(roles.surface_container, roles.on_surface, state.hover_opacity);
    colors.element_active = roles.state_layer(
        roles.surface_container,
        roles.on_surface,
        state.pressed_opacity,
    );
    colors.element_selected = roles.secondary_container;
    colors.element_selection_background = roles.primary_container;
    colors.element_disabled = roles.on_surface.alpha(state.disabled_container_opacity);

    colors.ghost_element_hover = roles.on_surface.alpha(state.hover_opacity);
    colors.ghost_element_active = roles.on_surface.alpha(state.pressed_opacity);
    colors.ghost_element_selected = roles.secondary_container;

    colors.text = roles.on_surface;
    colors.text_muted = roles.on_surface_variant;
    colors.text_placeholder = roles.on_surface_variant;
    colors.text_disabled = roles.on_surface.alpha(state.disabled_content_opacity);
    colors.text_accent = roles.primary;

    colors.icon = roles.on_surface;
    colors.icon_muted = roles.on_surface_variant;
    colors.icon_disabled = roles.on_surface.alpha(state.disabled_content_opacity);
    colors.icon_accent = roles.primary;

    colors.status_bar_background = roles.surface_container;
    colors.title_bar_background = roles.surface_container_low;
    colors.title_bar_inactive_background = roles.surface_container_lowest;
    colors.toolbar_background = roles.surface;
    colors.tab_bar_background = roles.surface_container_low;
    colors.tab_inactive_background = roles.surface_container_low;
    colors.tab_active_background = roles.surface;

    colors.panel_background = roles.surface_container_low;
    colors.panel_focused_border = roles.primary;
    colors.pane_focused_border = roles.primary;
    colors.pane_group_border = roles.outline_variant;

    colors.scrollbar_thumb_background = roles.outline_variant;
    colors.scrollbar_thumb_hover_background = roles.outline;
    colors.scrollbar_thumb_active_background = roles.primary;

    colors.editor_foreground = roles.on_surface;
    colors.editor_background = roles.surface;
    colors.editor_gutter_background = roles.surface_container_lowest;
    colors.editor_active_line_background = roles.surface_container_low;
    colors.editor_line_number = roles.outline;
    colors.editor_active_line_number = roles.on_surface;

    colors.terminal_background = roles.surface;
    colors.terminal_foreground = roles.on_surface;
    colors.terminal_bright_foreground = roles.on_background;

    let mut status = if is_dark {
        StatusColors::dark()
    } else {
        StatusColors::light()
    };
    status.error = roles.error;
    status.error_background = roles.error_container;
    status.error_border = roles.error;
    status.info = roles.primary;
    status.info_background = roles.primary_container;
    status.info_border = roles.primary;
    status.warning = roles.tertiary;
    status.warning_background = roles.tertiary_container;
    status.warning_border = roles.tertiary;
    status.success = roles.secondary;
    status.success_background = roles.secondary_container;
    status.success_border = roles.secondary;

    let (accents, player) = if is_dark {
        (AccentColors::dark(), PlayerColors::dark())
    } else {
        (AccentColors::light(), PlayerColors::light())
    };

    Arc::new(GpuiTheme {
        id: mode.as_str().to_owned(),
        name: mode.display_name().into(),
        appearance: if is_dark {
            GpuiAppearance::Dark
        } else {
            GpuiAppearance::Light
        },
        styles: ThemeStyles {
            window_background_appearance: WindowBackgroundAppearance::Opaque,
            system: SystemColors::default(),
            accents,
            colors,
            status,
            player,
            syntax: syntax.unwrap_or_else(|| Arc::new(SyntaxTheme::default())),
        },
    })
}

/// Synchronizes `gpui-component`, `gpui-base`, and `M3ActiveTheme` from the active Aphrody [`GlobalTheme`].
pub fn sync_kit_with_gpui_theme(cx: &mut App) {
    if !cx.has_global::<KitTheme>() {
        gpui_component::init(cx);
    }

    if !cx.has_global::<GlobalTheme>() {
        let fallback_mode = match cx.window_appearance() {
            WindowAppearance::Light | WindowAppearance::VibrantLight => M3AppearanceMode::Light,
            WindowAppearance::Dark | WindowAppearance::VibrantDark => M3AppearanceMode::Dark,
        };
        apply_m3_theme(fallback_mode, cx);
        return;
    }

    let gpui_theme = cx.theme().clone();
    let base_mode = match gpui_theme.id.as_str() {
        "neural_obsidian" | "neural-obsidian" => M3AppearanceMode::NeuralObsidian,
        "oled_pitch" | "oled-pitch" => M3AppearanceMode::OledPitch,
        _ => kit_m3_appearance_from_gpui(gpui_theme.appearance),
    };
    let config = RcThemeConfig::new(kit_m3_theme_config_from_gpui(&gpui_theme));

    cx.set_global(M3ActiveTheme::for_mode(base_mode));
    KitTheme::update(cx, move |theme| {
        theme.apply_config(&config);
        theme.motion = m3_motion_tokens();
    });
}

type RcThemeConfig = std::rc::Rc<KitThemeConfig>;

/// Applies a Material 3 Expressive `mode` to `gpui-component`, `gpui-base`, `M3ActiveTheme`, and `GlobalTheme`.
pub fn apply_kit_m3_theme(mode: M3AppearanceMode, cx: &mut App) {
    apply_m3_theme(mode, cx);

    if cx.has_global::<GlobalTheme>() {
        let existing_syntax = Some(cx.theme().styles.syntax.clone());
        let gpui_theme = kit_theme_to_gpui_theme(mode, existing_syntax);
        GlobalTheme::update_theme(cx, gpui_theme);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use gpui::{Context, IntoElement, ParentElement as _, Render, SharedString, Styled as _, TestAppContext, Window, px};
    use gpui_component::{IconName, Selectable as _, h_flex, v_flex};
    use theme::LoadThemes;

    #[test]
    fn kit_bridges_all_four_m3_modes_to_gpui_theme() {
        for mode in M3AppearanceMode::ALL {
            let active = M3ActiveTheme::for_mode(mode);
            let gpui_theme = kit_theme_to_gpui_theme(mode, None);

            assert_eq!(gpui_theme.id, mode.as_str());
            assert_eq!(gpui_theme.name, mode.display_name());
            assert_eq!(gpui_theme.styles.colors.background, active.colors.background);
            assert_eq!(gpui_theme.styles.colors.text, active.colors.on_surface);
            assert_eq!(gpui_theme.styles.colors.text_accent, active.colors.primary);
            assert_eq!(gpui_theme.styles.status.error, active.colors.error);
            assert_eq!(
                gpui_theme.appearance == GpuiAppearance::Dark,
                mode.is_dark()
            );

            let bridged = kit_m3_theme_config_from_gpui(&gpui_theme);
            assert_eq!(bridged.name, gpui_theme.name);
            assert_eq!(
                bridged.colors.background.as_deref(),
                Some(m3_hsla_to_hex(active.colors.background).as_ref())
            );
            assert_eq!(
                bridged.colors.foreground.as_deref(),
                Some(m3_hsla_to_hex(active.colors.on_surface).as_ref())
            );

            let text = gpui_theme.styles.colors.text;
            let bg = gpui_theme.styles.colors.background;
            let contrast = m3_contrast_ratio(text, bg);
            assert!(
                m3_meets_wcag_aa(text, bg),
                "{mode:?} bridged text/background contrast {contrast:.2}:1 < 4.5:1"
            );
        }
    }

    #[test]
    fn kit_verifies_m3_manifest_wcag_axes_and_springs() -> anyhow::Result<()> {
        let (source, system, seed) = m3_manifest_metadata()?;
        assert_eq!(source, material3::M3_DESIGN_SOURCE_OF_TRUTH);
        assert_eq!(system, "Material 3 Expressive");
        assert_eq!(seed, "#135B82");

        for mode in M3AppearanceMode::ALL {
            let reports = m3_verify_mode_contrast(mode)?;
            assert_eq!(reports.len(), 14);
            for report in reports {
                assert!(
                    report.passes_aa && report.ratio >= 4.5,
                    "{mode:?} pair {} contrast {:.2} < 4.5",
                    report.pair,
                    report.ratio
                );
            }
            let theme = M3ActiveTheme::for_mode(mode);
            assert_eq!(theme.typography.typeface.brand_family().as_ref(), "Google Sans Flex");
            assert_eq!(theme.typography.typeface.mono_family().as_ref(), "JetBrains Mono");
        }

        for preset in [
            M3GoogleSansFlexAxes::default(),
            M3GoogleSansFlexAxes::display_hero(),
            M3GoogleSansFlexAxes::section_headline(),
            M3GoogleSansFlexAxes::ui_control(),
            M3GoogleSansFlexAxes::compact_mobile(),
            M3GoogleSansFlexAxes::tabular_label(),
            M3GoogleSansFlexAxes::xr_glimmer_glanceable(),
            M3GoogleSansFlexAxes::quire_ink_body(),
        ] {
            preset.validate()?;
        }

        for spring in M3SpringToken::ALL {
            let spec = spring.try_spec()?;
            assert_eq!(spring.sample_position(0.0), 0.0);
            assert!((spring.sample_position(3.0) - 1.0).abs() < 1e-2);
            if spring.is_spatial() {
                assert!((spec.damping_ratio - 0.9).abs() < 1e-4);
            }
            if spring.is_effects() {
                assert!((spec.damping_ratio - 1.0).abs() < 1e-4);
            }
            let spring_cfg = spring.gpui_spring_config();
            assert!((spring_cfg.stiffness - spec.stiffness).abs() < 1e-4);
            assert!(
                (spring_cfg.damping - 2.0 * spec.damping_ratio * spec.stiffness.sqrt()).abs()
                    < 1e-3
            );
            assert!(format!("{:?}", spring.gpui_spring()).contains("Spring"));
        }

        Ok(())
    }

    #[gpui::test]
    fn kit_applies_and_syncs_themes_across_gpui_and_kit(cx: &mut TestAppContext) {
        cx.update(|cx| {
            theme::init(LoadThemes::JustBase, cx);
            gpui_component::init(cx);
        });

        for mode in M3AppearanceMode::ALL {
            cx.update(|cx| {
                apply_kit_m3_theme(mode, cx);

                let m3 = active_m3_theme(cx);
                let kit_theme = KitTheme::global(cx);
                let base_theme = gpui_base::Theme::global(cx);
                let gpui_theme = cx.theme().clone();

                assert_eq!(m3.mode, mode);
                assert_eq!(kit_theme.primary, m3.colors.primary);
                assert_eq!(base_theme.tokens.colors.background, m3.colors.background);
                assert_eq!(gpui_theme.styles.colors.background, m3.colors.background);
                assert_eq!(gpui_theme.styles.colors.text_accent, m3.colors.primary);

                sync_kit_with_gpui_theme(cx);
                assert_eq!(active_m3_theme(cx).mode, mode);
                assert_eq!(KitTheme::global(cx).theme_name(), &gpui_theme.name);
                assert_eq!(KitTheme::global(cx).background, gpui_theme.styles.colors.background);
            });
        }
    }

    #[gpui::test]
    fn kit_renders_and_interacts_with_all_sixteen_m3_components(cx: &mut TestAppContext) {
        struct GalleryHarness {
            clicks: Rc<Cell<usize>>,
            chip_selected: Rc<Cell<bool>>,
            switch_checked: Rc<Cell<bool>>,
            selected_segment: Rc<RefCell<SharedString>>,
            selected_nav: Rc<RefCell<SharedString>>,
            dialog_open: Rc<Cell<bool>>,
            dismissed: Rc<Cell<usize>>,
        }

        impl Render for GalleryHarness {
            fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
                let clicks = self.clicks.clone();
                let chip_selected = self.chip_selected.clone();
                let switch_checked = self.switch_checked.clone();
                let selected_segment = self.selected_segment.clone();
                let selected_nav = self.selected_nav.clone();
                let current_segment = selected_segment.borrow().clone();
                let current_nav = selected_nav.borrow().clone();
                let dialog_open = self.dialog_open.clone();
                let dismissed = self.dismissed.clone();
                let view = cx.entity().downgrade();

                let notify = move |cx: &mut App| {
                    assert!(
                        view.update(cx, |_, cx| cx.notify()).is_ok(),
                        "GalleryHarness view should stay alive during test"
                    );
                };

                v_flex()
                    .w(px(720.))
                    .gap_2()
                    .p_3()
                    .child(
                        M3TopAppBar::new("top-bar", "Material 3 Expressive")
                            .subtitle("Neural Expressive Kit")
                            .variant(M3TopAppBarVariant::Small)
                            .scrolled(true)
                            .action(M3FilledButton::new("top-save", "Save")),
                    )
                    .child(
                        M3SearchBar::new("search")
                            .query("design tokens")
                            .shortcut_hint("Ctrl+K")
                            .on_click({
                                let clicks = clicks.clone();
                                move |_, _, _| clicks.set(clicks.get() + 1)
                            }),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                M3FilledButton::new("filled-btn", "Primary")
                                    .icon(IconName::Plus)
                                    .on_click({
                                        let clicks = clicks.clone();
                                        move |_, _, _| clicks.set(clicks.get() + 1)
                                    }),
                            )
                            .child(M3TonalButton::new("tonal-btn", "Tonal"))
                            .child(M3OutlinedButton::new("outlined-btn", "Outlined"))
                            .child(
                                M3Fab::new("fab", IconName::Plus)
                                    .fab_size(M3FabSize::Standard)
                                    .accessibility_label("Create"),
                            )
                            .child(
                                M3ExtendedFab::new("xfab", "Compose")
                                    .icon(IconName::Plus)
                                    .color(M3FabColor::Tertiary),
                            ),
                    )
                    .child(
                        M3SegmentedButtonBar::new("segments")
                            .segment(M3SegmentItem::new("day", "Day"))
                            .segment(M3SegmentItem::new("week", "Week"))
                            .segment(M3SegmentItem::new("month", "Month"))
                            .selected_id(current_segment)
                            .on_select({
                                let notify = notify.clone();
                                move |id, _, cx| {
                                    *selected_segment.borrow_mut() = id.clone();
                                    notify(cx);
                                }
                            }),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                M3FilterChip::new("filter-chip", "Accessible")
                                    .selected(chip_selected.get())
                                    .on_change({
                                        let notify = notify.clone();
                                        move |next, _, cx| {
                                            chip_selected.set(*next);
                                            notify(cx);
                                        }
                                    }),
                            )
                            .child(
                                M3ExpressiveSwitch::new("switch")
                                    .label("OLED Pitch")
                                    .checked(switch_checked.get())
                                    .on_change({
                                        let notify = notify.clone();
                                        move |next, _, cx| {
                                            switch_checked.set(*next);
                                            notify(cx);
                                        }
                                    }),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                M3ElevatedCard::new()
                                    .id("elevated-card")
                                    .title("Elevated")
                                    .subtitle("Surface container low")
                                    .child("Body"),
                            )
                            .child(
                                M3FilledCard::new()
                                    .id("filled-card")
                                    .title("Filled")
                                    .child("Body"),
                            )
                            .child(
                                M3OutlinedCard::new()
                                    .id("outlined-card")
                                    .title("Outlined")
                                    .footer("Footer"),
                            ),
                    )
                    .child(
                        h_flex()
                            .h(px(110.))
                            .gap_2()
                            .child(
                                M3NavigationRail::new("rail")
                                    .destination(M3NavDestination::new(
                                        "home",
                                        "Home",
                                        IconName::Inbox,
                                    ))
                                    .destination(
                                        M3NavDestination::new("tokens", "Tokens", IconName::Palette)
                                            .badge("4"),
                                    )
                                    .selected_id(current_nav.clone())
                                    .on_select({
                                        let selected_nav = selected_nav.clone();
                                        let notify = notify.clone();
                                        move |id, _, cx| {
                                            *selected_nav.borrow_mut() = id.clone();
                                            notify(cx);
                                        }
                                    }),
                            )
                            .child(
                                M3DialogSheet::new("sheet")
                                    .open(dialog_open.get())
                                    .title("Sync Tokens")
                                    .supporting_text("Apply Neural Obsidian across surfaces?")
                                    .dismiss_label("Cancel")
                                    .confirm_label("Apply")
                                    .on_dismiss({
                                        let notify = notify.clone();
                                        move |_, _, cx| {
                                            dismissed.set(dismissed.get() + 1);
                                            dialog_open.set(false);
                                            notify(cx);
                                        }
                                    }),
                            ),
                    )
                    .child(
                        M3NavigationBar::new("nav-bar")
                            .destination(M3NavDestination::new("home", "Home", IconName::Inbox))
                            .destination(M3NavDestination::new(
                                "settings",
                                "Settings",
                                IconName::Settings,
                            ))
                            .selected_id(current_nav)
                            .on_select({
                                let selected_nav = selected_nav.clone();
                                move |id, _, cx| {
                                    *selected_nav.borrow_mut() = id.clone();
                                    notify(cx);
                                }
                            }),
                    )
            }
        }

        cx.update(|cx| {
            gpui_component::init(cx);
            apply_m3_theme(M3AppearanceMode::OledPitch, cx);
        });

        let clicks = Rc::new(Cell::new(0));
        let chip_selected = Rc::new(Cell::new(false));
        let switch_checked = Rc::new(Cell::new(false));
        let selected_segment = Rc::new(RefCell::new(SharedString::from("day")));
        let selected_nav = Rc::new(RefCell::new(SharedString::from("home")));
        let dialog_open = Rc::new(Cell::new(true));
        let dismissed = Rc::new(Cell::new(0));

        let (_, cx) = cx.add_window_view({
            let clicks = clicks.clone();
            let chip_selected = chip_selected.clone();
            let switch_checked = switch_checked.clone();
            let selected_segment = selected_segment.clone();
            let selected_nav = selected_nav.clone();
            let dialog_open = dialog_open.clone();
            let dismissed = dismissed.clone();
            move |_, _| GalleryHarness {
                clicks,
                chip_selected,
                switch_checked,
                selected_segment,
                selected_nav,
                dialog_open,
                dismissed,
            }
        });

        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });

        assert!(cx.debug_bounds("sheet").is_some());
        let dismiss_bounds = cx
            .debug_bounds("sheet-dismiss")
            .expect("dialog dismiss button must lay out");
        cx.simulate_click(dismiss_bounds.center(), gpui::Modifiers::default());
        cx.run_until_parked();
        assert_eq!(dismissed.get(), 1);
        assert!(!dialog_open.get());
        assert!(cx.debug_bounds("sheet").is_none());

        let filled_bounds = cx
            .debug_bounds("filled-btn")
            .expect("filled button must lay out");
        cx.simulate_click(filled_bounds.center(), gpui::Modifiers::default());
        assert_eq!(clicks.get(), 1);

        let search_bounds = cx
            .debug_bounds("search")
            .expect("search bar must lay out");
        cx.simulate_click(search_bounds.center(), gpui::Modifiers::default());
        assert_eq!(clicks.get(), 2);

        let segment_week_bounds = cx
            .debug_bounds("segments-week")
            .expect("segmented item must lay out");
        cx.simulate_click(segment_week_bounds.center(), gpui::Modifiers::default());
        cx.run_until_parked();
        assert_eq!(*selected_segment.borrow(), SharedString::from("week"));

        let chip_bounds = cx
            .debug_bounds("filter-chip")
            .expect("filter chip must lay out");
        cx.simulate_click(chip_bounds.center(), gpui::Modifiers::default());
        cx.run_until_parked();
        assert!(chip_selected.get());

        let switch_bounds = cx
            .debug_bounds("switch")
            .expect("expressive switch must lay out");
        cx.simulate_click(switch_bounds.center(), gpui::Modifiers::default());
        cx.run_until_parked();
        assert!(switch_checked.get());

        let rail_tokens_bounds = cx
            .debug_bounds("rail-tokens")
            .expect("navigation rail destination must lay out");
        cx.simulate_click(rail_tokens_bounds.center(), gpui::Modifiers::default());
        cx.run_until_parked();
        assert_eq!(*selected_nav.borrow(), SharedString::from("tokens"));

        let nav_settings_bounds = cx
            .debug_bounds("nav-bar-settings")
            .expect("navigation bar destination must lay out");
        cx.simulate_click(nav_settings_bounds.center(), gpui::Modifiers::default());
        cx.run_until_parked();
        assert_eq!(*selected_nav.borrow(), SharedString::from("settings"));

        assert!(cx.debug_bounds("top-bar").is_some());
        assert!(cx.debug_bounds("tonal-btn").is_some());
        assert!(cx.debug_bounds("outlined-btn").is_some());
        assert!(cx.debug_bounds("fab").is_some());
        assert!(cx.debug_bounds("xfab").is_some());
        assert!(cx.debug_bounds("segments").is_some());
        assert!(cx.debug_bounds("elevated-card").is_some());
        assert!(cx.debug_bounds("filled-card").is_some());
        assert!(cx.debug_bounds("outlined-card").is_some());
        assert!(cx.debug_bounds("rail").is_some());
        assert!(cx.debug_bounds("nav-bar").is_some());
    }

    #[gpui::test]
    fn kit_proves_compose15_and_parametric_prototyper_end_to_end(cx: &mut TestAppContext) {
        let scope = M3ExpressiveThemeScope::new(M3AppearanceMode::NeuralObsidian)
            .motion_scheme(M3MotionScheme::Expressive)
            .default_font_family("Google Sans Flex");

        cx.update(|cx| {
            theme::init(LoadThemes::JustBase, cx);
            gpui_component::init(cx);
            apply_m3_expressive_theme(&scope, cx);
            apply_kit_m3_theme(M3AppearanceMode::NeuralObsidian, cx);

            let catalog = M3Compose15Catalog::canonical();
            assert_eq!(catalog.material3_version, "1.5.0");
            assert_eq!(catalog.compose_bom, "2026.11.01");
            assert_eq!(catalog.figma_kit_file_id, "1035203688168086460");
            assert_eq!(catalog.material_shapes_count, 35);
            assert!(M3_EXPRESSIVE_WGSL.contains("fn sdf_material_polygon"));
            assert_eq!(
                active_m3_expressive_config(cx).motion_scheme,
                M3MotionScheme::Expressive
            );
            assert!(active_m3_expressive_config(cx).ripple.inset_focus_ring_enabled);
        });

        let (view, cx) = cx.add_window_view(|_, cx| M3ParametricPrototyperView::new(cx));
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        assert!(cx.debug_bounds("m3-parametric-prototyper").is_some());
        let dtcg = view
            .read_with(cx, |v, _| v.state().export_figma_dtcg_json())
            .expect("Figma DTCG export must succeed");
        assert!(dtcg.contains("1035203688168086460"));
    }
}
