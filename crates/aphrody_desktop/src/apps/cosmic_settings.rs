//! Multi-page `cosmic-settings` Control Center (`NavRail` sidebar + live `CosmicSettingsDaemon`).

use aphrody_session::{CosmicSettingsDaemon, PowerProfile, TilingMode};
use aphrody_ui::{
    GlassMaterialPreset, KeyValueRow, M3_DYNAMIC_SEEDS, M3DynamicVariant, NavItem, OsTheme,
    OsThemeCatalog, StatusTone, render_nav_rail, status_badge,
};
use gpui::{AnyElement, App, SharedString, div, prelude::*, px};
use ui::{IconName, Label, LabelCommon as _, LabelSize, h_flex, v_flex};

use crate::apps::section_heading;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsPage {
    #[default]
    Appearance,
    DesktopTiling,
    Displays,
    PowerDaemon,
    Language,
}

impl SettingsPage {
    pub const ALL: [Self; 5] = [
        Self::Appearance,
        Self::DesktopTiling,
        Self::Displays,
        Self::PowerDaemon,
        Self::Language,
    ];

    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Appearance => "appearance",
            Self::DesktopTiling => "desktop",
            Self::Displays => "displays",
            Self::PowerDaemon => "power",
            Self::Language => "language",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Appearance => "Appearance",
            Self::DesktopTiling => "Desktop & Tiling",
            Self::Displays => "Displays (RandR)",
            Self::PowerDaemon => "Power & Daemon",
            Self::Language => "Language (i18n)",
        }
    }

    #[must_use]
    pub const fn icon(self) -> IconName {
        match self {
            Self::Appearance => IconName::Image,
            Self::DesktopTiling => IconName::Settings,
            Self::Displays => IconName::Server,
            Self::PowerDaemon => IconName::Terminal,
            Self::Language => IconName::Code,
        }
    }

    #[must_use]
    pub fn from_id(id: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|p| p.id() == id)
            .unwrap_or_default()
    }
}

pub struct CosmicSettingsCallbacks {
    pub on_select_page: Box<dyn Fn(SettingsPage, &mut App) + 'static>,
    pub on_select_theme: Box<dyn Fn(&str, &mut App) + 'static>,
    pub on_select_m3_seed: Box<dyn Fn([u8; 3], &mut App) + 'static>,
    pub on_select_m3_variant: Box<dyn Fn(M3DynamicVariant, &mut App) + 'static>,
    pub on_select_glass_preset: Box<dyn Fn(GlassMaterialPreset, &mut App) + 'static>,
    pub on_toggle_mode: Box<dyn Fn(&mut App) + 'static>,
    pub on_set_layout: Box<dyn Fn(TilingMode, &mut App) + 'static>,
    pub on_toggle_active_hint: Box<dyn Fn(&mut App) + 'static>,
    pub on_set_power_profile: Box<dyn Fn(PowerProfile, &mut App) + 'static>,
    pub on_adjust_brightness: Box<dyn Fn(i8, &mut App) + 'static>,
    pub on_open_de_switcher: Box<dyn Fn(&mut App) + 'static>,
}

pub fn render_cosmic_settings_window(
    page: SettingsPage,
    theme: &OsTheme,
    catalog: &OsThemeCatalog,
    active_theme_id: &str,
    active_seed_rgb: Option<[u8; 3]>,
    active_m3_variant: M3DynamicVariant,
    active_glass_preset: GlassMaterialPreset,
    active_layout: TilingMode,
    active_hint_enabled: bool,
    active_hint_px: f32,
    daemon: &CosmicSettingsDaemon,
    cb: CosmicSettingsCallbacks,
) -> AnyElement {
    let nav_items: Vec<NavItem> = SettingsPage::ALL
        .into_iter()
        .map(|p| NavItem {
            id: SharedString::from(p.id()),
            label: SharedString::from(p.label()),
            icon: p.icon(),
            badge: None,
        })
        .collect();

    let on_select_page = std::rc::Rc::new(cb.on_select_page);
    let rail = render_nav_rail(
        "cosmic-settings-nav",
        &nav_items,
        page.id(),
        false,
        theme,
        move |selected_id, _w, cx| {
            on_select_page(SettingsPage::from_id(selected_id.as_ref()), cx);
        },
    );

    let on_theme = std::rc::Rc::new(cb.on_select_theme);
    let on_seed = std::rc::Rc::new(cb.on_select_m3_seed);
    let on_variant = std::rc::Rc::new(cb.on_select_m3_variant);
    let on_glass = std::rc::Rc::new(cb.on_select_glass_preset);
    let on_toggle_mode = std::rc::Rc::new(cb.on_toggle_mode);
    let on_layout = std::rc::Rc::new(cb.on_set_layout);
    let on_hint = std::rc::Rc::new(cb.on_toggle_active_hint);
    let on_power = std::rc::Rc::new(cb.on_set_power_profile);
    let on_brightness = std::rc::Rc::new(cb.on_adjust_brightness);
    let on_switcher = std::rc::Rc::new(cb.on_open_de_switcher);

    let detail = match page {
        SettingsPage::Appearance => {
            let mut swatches = h_flex().gap_2().flex_wrap();
            for (idx, (id_key, entry)) in catalog.entries.iter().enumerate() {
                let id_owned = id_key.clone();
                let selected = active_seed_rgb.is_none() && id_owned == active_theme_id;
                let cb_t = std::rc::Rc::clone(&on_theme);
                swatches = swatches.child(
                    div()
                        .id(("cosmic-set-theme", idx))
                        .cursor_pointer()
                        .px_2p5()
                        .py_1p5()
                        .rounded(px(theme.layout.window_radius))
                        .border_1()
                        .border_color(if selected {
                            theme.colors.primary
                        } else {
                            theme.colors.outline_variant
                        })
                        .bg(if selected {
                            theme.colors.primary_container
                        } else {
                            theme.colors.surface_container_high
                        })
                        .on_click(move |_ev, _w, cx| cb_t(&id_owned, cx))
                        .child(Label::new(entry.name.clone()).size(LabelSize::XSmall)),
                );
            }

            let mut seed_row = h_flex().gap_2().flex_wrap();
            for (idx, seed) in M3_DYNAMIC_SEEDS.iter().enumerate() {
                let selected = active_seed_rgb == Some(seed.rgb);
                let cb_s = std::rc::Rc::clone(&on_seed);
                let rgb = seed.rgb;
                let swatch_color: gpui::Hsla = gpui::Rgba {
                    r: f32::from(rgb[0]) / 255.0,
                    g: f32::from(rgb[1]) / 255.0,
                    b: f32::from(rgb[2]) / 255.0,
                    a: 1.0,
                }
                .into();
                seed_row = seed_row.child(
                    div()
                        .id(("m3-seed-swatch", idx))
                        .cursor_pointer()
                        .px_2p5()
                        .py_1()
                        .rounded_full()
                        .border_1()
                        .border_color(if selected {
                            theme.colors.primary
                        } else {
                            theme.colors.outline_variant
                        })
                        .bg(if selected {
                            theme.colors.primary_container
                        } else {
                            theme.colors.surface_container_high
                        })
                        .on_click(move |_ev, _w, cx| cb_s(rgb, cx))
                        .child(
                            h_flex()
                                .gap_1p5()
                                .items_center()
                                .child(div().size(px(10.0)).rounded_full().bg(swatch_color))
                                .child(Label::new(seed.name).size(LabelSize::XSmall)),
                        ),
                );
            }

            let mut variant_row = h_flex().gap_1p5().flex_wrap();
            for (idx, variant) in M3DynamicVariant::ALL.into_iter().enumerate() {
                let selected = variant == active_m3_variant;
                let cb_v = std::rc::Rc::clone(&on_variant);
                variant_row = variant_row.child(
                    div()
                        .id(("m3-variant-pill", idx))
                        .cursor_pointer()
                        .px_2()
                        .py_1()
                        .rounded_full()
                        .border_1()
                        .border_color(if selected {
                            theme.colors.primary
                        } else {
                            theme.colors.outline_variant.opacity(0.6)
                        })
                        .bg(if selected {
                            theme.colors.secondary_container
                        } else {
                            theme.colors.surface_container
                        })
                        .on_click(move |_ev, _w, cx| cb_v(variant, cx))
                        .child(Label::new(variant.label()).size(LabelSize::XSmall)),
                );
            }

            let glass_plan = active_glass_preset.resolve_gpu_plan(2.0, 16.0);
            let mut glass_row = h_flex().gap_2().flex_wrap();
            for (idx, preset) in GlassMaterialPreset::ALL.into_iter().enumerate() {
                let selected = preset == active_glass_preset;
                let cb_g = std::rc::Rc::clone(&on_glass);
                glass_row = glass_row.child(
                    div()
                        .id(("glass-preset-pill", idx))
                        .cursor_pointer()
                        .px_2p5()
                        .py_1()
                        .rounded_full()
                        .border_1()
                        .border_color(if selected {
                            theme.colors.primary
                        } else {
                            theme.colors.outline_variant
                        })
                        .bg(if selected {
                            theme.colors.primary_container
                        } else {
                            theme.colors.surface_container_high
                        })
                        .on_click(move |_ev, _w, cx| cb_g(preset, cx))
                        .child(Label::new(preset.label()).size(LabelSize::XSmall)),
                );
            }

            v_flex()
                .gap_3()
                .child(section_heading(
                    theme,
                    "COSMIC Palette & Material 3 Expressive",
                ))
                .child(swatches)
                .child(section_heading(
                    theme,
                    "Dynamic Material 3 HCT Seed (material-colors 0.5 Spec2025)",
                ))
                .child(seed_row)
                .child(variant_row)
                .child(section_heading(
                    theme,
                    "Frosted & Liquid Glass (backdrop-blur-core + window-vibrancy)",
                ))
                .child(glass_row)
                .child(
                    h_flex()
                        .gap_2()
                        .flex_wrap()
                        .child(
                            div()
                                .id("cosmic-settings-toggle-mode")
                                .cursor_pointer()
                                .px_3()
                                .py_1p5()
                                .rounded(px(8.0))
                                .bg(theme.colors.secondary_container)
                                .on_click(move |_ev, _w, cx| on_toggle_mode(cx))
                                .child(
                                    Label::new(format!("Appearance: {:?}", theme.mode))
                                        .size(LabelSize::XSmall),
                                ),
                        )
                        .child(status_badge(
                            IconName::Sparkle,
                            format!(
                                "{} · {:.0}px · {} Kawase · Fresnel {:.0}%",
                                active_glass_preset.label(),
                                glass_plan.physical_radius_px,
                                glass_plan.kawase_levels,
                                glass_plan.specular_intensity * 100.0
                            ),
                            StatusTone::Accent,
                            theme,
                        )),
                )
                .child(KeyValueRow::new(
                    "Active Token Source",
                    "material-colors 0.5 Spec2025 + assets/os/google_design_tokens.toml",
                ))
        }
        SettingsPage::DesktopTiling => {
            let mut modes_row = h_flex().gap_2().flex_wrap();
            for mode in TilingMode::ALL {
                let cb_l = std::rc::Rc::clone(&on_layout);
                let is_curr = mode == active_layout;
                modes_row = modes_row.child(
                    div()
                        .id(("cosmic-set-layout", mode as usize))
                        .cursor_pointer()
                        .px_2p5()
                        .py_1()
                        .rounded(px(8.0))
                        .border_1()
                        .border_color(if is_curr {
                            theme.colors.primary
                        } else {
                            theme.colors.outline_variant
                        })
                        .bg(if is_curr {
                            theme.colors.primary_container
                        } else {
                            theme.colors.surface_container_high
                        })
                        .on_click(move |_ev, _w, cx| cb_l(mode, cx))
                        .child(Label::new(mode.label()).size(LabelSize::XSmall)),
                );
            }
            v_flex()
                .gap_3()
                .child(section_heading(
                    theme,
                    "Pop Shell Auto-Tiling & Active Hint",
                ))
                .child(modes_row)
                .child(
                    div()
                        .id("cosmic-set-active-hint")
                        .cursor_pointer()
                        .px_3()
                        .py_1p5()
                        .rounded(px(8.0))
                        .bg(if active_hint_enabled {
                            theme.colors.primary_container
                        } else {
                            theme.colors.surface_container_high
                        })
                        .on_click(move |_ev, _w, cx| on_hint(cx))
                        .child(
                            Label::new(format!(
                                "Active Hint Border: {} ({:.0}px)",
                                if active_hint_enabled { "Enabled" } else { "Off" },
                                active_hint_px
                            ))
                            .size(LabelSize::XSmall),
                        ),
                )
                .child(
                    div()
                        .id("cosmic-set-open-de-switcher")
                        .cursor_pointer()
                        .px_3()
                        .py_1p5()
                        .rounded(px(8.0))
                        .bg(theme.colors.secondary_container)
                        .on_click(move |_ev, _w, cx| on_switcher(cx))
                        .child(
                            Label::new("Open Rust DE Switcher (GPUI / Iced COSMIC / egui / KDE)")
                                .size(LabelSize::XSmall),
                        ),
                )
        }
        SettingsPage::Displays => {
            let mut list =
                v_flex()
                    .gap_2()
                    .child(section_heading(theme, "cosmic-randr Outputs"));
            for disp in &daemon.outputs {
                list = list.child(
                    v_flex()
                        .p_2()
                        .rounded(px(8.0))
                        .bg(theme.colors.surface_container_high)
                        .child(
                            h_flex()
                                .justify_between()
                                .child(
                                    Label::new(format!("{} ({})", disp.connector, disp.make_model))
                                        .size(LabelSize::Small),
                                )
                                .child(status_badge(
                                    IconName::Server,
                                    if disp.vrr_enabled { "VRR On" } else { "Fixed Hz" },
                                    StatusTone::Success,
                                    theme,
                                )),
                        )
                        .child(KeyValueRow::new(
                            "Mode",
                            format!("{}x{} @ {}Hz", disp.width, disp.height, disp.refresh_hz),
                        ))
                        .child(KeyValueRow::new(
                            "Scale / Night Light",
                            format!("{:.2}x · {}K", disp.scale, daemon.night_light_kelvin),
                        )),
                );
            }
            list
        }
        SettingsPage::PowerDaemon => {
            let mut pow_row = h_flex().gap_2().flex_wrap();
            for prof in PowerProfile::ALL {
                let cb_p = std::rc::Rc::clone(&on_power);
                let active = prof == daemon.power_profile;
                pow_row = pow_row.child(
                    div()
                        .id(("cosmic-pow-prof", prof as usize))
                        .cursor_pointer()
                        .px_2p5()
                        .py_1()
                        .rounded(px(8.0))
                        .border_1()
                        .border_color(if active {
                            theme.colors.primary
                        } else {
                            theme.colors.outline_variant
                        })
                        .bg(if active {
                            theme.colors.primary_container
                        } else {
                            theme.colors.surface_container_high
                        })
                        .on_click(move |_ev, _w, cx| cb_p(prof, cx))
                        .child(Label::new(prof.label()).size(LabelSize::XSmall)),
                );
            }
            let b_down = std::rc::Rc::clone(&on_brightness);
            let b_up = std::rc::Rc::clone(&on_brightness);
            v_flex()
                .gap_3()
                .child(section_heading(
                    theme,
                    "cosmic-settings-daemon Power & Brightness",
                ))
                .child(pow_row)
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .id("cosmic-bright-down")
                                .cursor_pointer()
                                .px_2()
                                .py_1()
                                .rounded(px(6.0))
                                .bg(theme.colors.surface_container_high)
                                .on_click(move |_ev, _w, cx| b_down(-10, cx))
                                .child(Label::new("-10%").size(LabelSize::XSmall)),
                        )
                        .child(
                            Label::new(format!(
                                "Backlight {}% · Volume {}%",
                                daemon.brightness_percent, daemon.volume_percent
                            ))
                            .size(LabelSize::XSmall),
                        )
                        .child(
                            div()
                                .id("cosmic-bright-up")
                                .cursor_pointer()
                                .px_2()
                                .py_1()
                                .rounded(px(6.0))
                                .bg(theme.colors.surface_container_high)
                                .on_click(move |_ev, _w, cx| b_up(10, cx))
                                .child(Label::new("+10%").size(LabelSize::XSmall)),
                        ),
                )
        }
        SettingsPage::Language => v_flex()
            .gap_2()
            .child(section_heading(theme, "W3C i18n & Bidirectional Locale"))
            .child(status_badge(
                IconName::Code,
                "BCP-47 + UBA BiDi Active",
                StatusTone::Success,
                theme,
            ))
            .child(KeyValueRow::new(
                "Engine",
                "aphrody_session::i18n + aphrody_ui::i18n",
            )),
    };

    h_flex()
        .size_full()
        .gap_3()
        .child(rail)
        .child(div().flex_1().min_w_0().overflow_hidden().child(detail))
        .into_any_element()
}
