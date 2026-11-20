//! Design Gallery: native GPUI mirrors of the design.google references and the Material 3
//! component matrix, rendered with the active desktop theme.

use aphrody_ui::{
    GlassMaterialPreset, GoogleDesignTokens, LUCIDE_SHOWCASE_ICONS, M3_DYNAMIC_SEEDS,
    MATERIAL_SYMBOL_NAMES, MaterialSymbolStyle, MirrorAccentPalette, OsTheme, ReferenceGallery,
    ReferenceGalleryChange, ReferenceGalleryState, ShadcnGpuiTheme, ShapeScale, StatusTone,
    lucide_icon_svg, material_symbol_svg, status_badge,
};
use gpui::{AnyElement, App, FontWeight, SharedString, div, prelude::*, px};
use ui::{IconName, Label, LabelCommon as _, LabelSize, h_flex, v_flex};

use crate::apps::section_heading;
use crate::windows::WindowKind;

pub struct DesignGalleryState {
    gallery: ReferenceGalleryState,
    tokens: Result<(MirrorAccentPalette, ShapeScale), String>,
}

impl DesignGalleryState {
    #[must_use]
    pub fn bundled() -> Self {
        let tokens = GoogleDesignTokens::bundled().and_then(|tokens| {
            Ok((MirrorAccentPalette::from_tokens(&tokens)?, tokens.sys.shape))
        });
        Self {
            gallery: ReferenceGalleryState::default(),
            tokens: tokens.map_err(|err| {
                log::warn!("design gallery tokens unavailable: {err:#}");
                format!("{err:#}")
            }),
        }
    }

    pub fn apply(&mut self, change: &ReferenceGalleryChange) -> bool {
        self.gallery.apply(change)
    }
}

/// Maps each canonical Aphrody fork pillar ID to its interactive OS window inside `aphrody_desktop`.
#[must_use]
pub fn pillar_target_window(pillar_id: &str) -> WindowKind {
    match pillar_id {
        "craft-suite" => WindowKind::Studio,
        "browser" => WindowKind::Browser,
        "vpn" => WindowKind::Vpn,
        "store" => WindowKind::Store,
        "installer" => WindowKind::Installer,
        "cosmic-os" => WindowKind::Settings,
        "pilot-mcp" => WindowKind::Monitor,
        "nsh-studio" => WindowKind::Edit,
        "auto" => WindowKind::Term,
        _ => WindowKind::Design,
    }
}

pub struct DesignGalleryCallbacks {
    pub on_change: Box<dyn Fn(&ReferenceGalleryChange, &mut App) + 'static>,
    pub on_open_window: Box<dyn Fn(WindowKind, &mut App) + 'static>,
    pub on_select_seed: Box<dyn Fn([u8; 3], &mut App) + 'static>,
    pub on_select_glass: Box<dyn Fn(GlassMaterialPreset, &mut App) + 'static>,
}

pub fn render_design_gallery_window(
    theme: &OsTheme,
    state: &DesignGalleryState,
    active_seed_rgb: Option<[u8; 3]>,
    active_glass: GlassMaterialPreset,
    cb: DesignGalleryCallbacks,
) -> AnyElement {
    let on_change = cb.on_change;
    let on_open_window = std::rc::Rc::new(cb.on_open_window);
    let on_select_seed = std::rc::Rc::new(cb.on_select_seed);
    let on_select_glass = std::rc::Rc::new(cb.on_select_glass);

    let content = match &state.tokens {
        Ok((accents, shape)) => {
            let starter_theme = ShadcnGpuiTheme::from_os_theme(theme, shape);
            ReferenceGallery::new("desktop-reference-gallery", &state.gallery, &starter_theme, accents)
                .on_change(move |change, _window, cx| on_change(change, cx))
                .into_any_element()
        }
        Err(message) => Label::new(format!("Design tokens could not be loaded: {message}"))
            .size(LabelSize::Small)
            .into_any_element(),
    };

    let glass_plan = active_glass.resolve_gpu_plan(2.0, 20.0);
    let md_svg_bytes: usize = MATERIAL_SYMBOL_NAMES
        .iter()
        .filter_map(|name| material_symbol_svg(name, MaterialSymbolStyle::Rounded))
        .map(str::len)
        .sum();
    let lucide_svg_bytes: usize = LUCIDE_SHOWCASE_ICONS
        .iter()
        .map(|(_, icon)| lucide_icon_svg(*icon).len())
        .sum();

    let material_glass_lab = v_flex()
        .w_full()
        .p_3()
        .gap_2()
        .rounded(px(16.0))
        .border_1()
        .border_color(theme.colors.primary.opacity(active_glass.specular_intensity()))
        .bg(theme.colors.surface_container_high.opacity(active_glass.tint_alpha().max(0.75)))
        .child(
            h_flex()
                .justify_between()
                .flex_wrap()
                .gap_2()
                .child(
                    Label::new("Material 3 HCT · Real Vector Icons · Dual-Kawase Liquid Glass")
                        .size(LabelSize::Small)
                        .weight(FontWeight::SEMIBOLD),
                )
                .child(
                    h_flex()
                        .gap_1p5()
                        .flex_wrap()
                        .child(status_badge(
                            IconName::Sparkle,
                            format!(
                                "md-icons {} SVGs ({:.1} KB)",
                                MATERIAL_SYMBOL_NAMES.len(),
                                md_svg_bytes as f32 / 1024.0
                            ),
                            StatusTone::Success,
                            theme,
                        ))
                        .child(status_badge(
                            IconName::Sparkle,
                            format!(
                                "lucide {} SVGs ({:.1} KB)",
                                LUCIDE_SHOWCASE_ICONS.len(),
                                lucide_svg_bytes as f32 / 1024.0
                            ),
                            StatusTone::Success,
                            theme,
                        ))
                        .child(status_badge(
                            IconName::Image,
                            format!(
                                "{} · {:.0}px · {} Kawase",
                                active_glass.label(),
                                glass_plan.physical_radius_px,
                                glass_plan.kawase_levels
                            ),
                            StatusTone::Accent,
                            theme,
                        )),
                ),
        )
        .child(
            h_flex()
                .gap_1p5()
                .flex_wrap()
                .children(M3_DYNAMIC_SEEDS.iter().enumerate().map(|(idx, seed)| {
                    let selected = active_seed_rgb == Some(seed.rgb);
                    let rgb = seed.rgb;
                    let cb_seed = std::rc::Rc::clone(&on_select_seed);
                    let dot_color: gpui::Hsla = gpui::Rgba {
                        r: f32::from(rgb[0]) / 255.0,
                        g: f32::from(rgb[1]) / 255.0,
                        b: f32::from(rgb[2]) / 255.0,
                        a: 1.0,
                    }
                    .into();
                    div()
                        .id(("gallery-m3-seed", idx))
                        .cursor_pointer()
                        .px_2()
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
                            theme.colors.surface_container
                        })
                        .on_click(move |_ev, _w, cx| cb_seed(rgb, cx))
                        .child(
                            h_flex()
                                .gap_1()
                                .items_center()
                                .child(div().size(px(10.0)).rounded_full().bg(dot_color))
                                .child(Label::new(seed.name).size(LabelSize::XSmall)),
                        )
                }))
                .children(GlassMaterialPreset::ALL.into_iter().enumerate().map(|(idx, preset)| {
                    let selected = preset == active_glass;
                    let cb_glass = std::rc::Rc::clone(&on_select_glass);
                    div()
                        .id(("gallery-glass-preset", idx))
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
                            theme.colors.surface_container_low
                        })
                        .on_click(move |_ev, _w, cx| cb_glass(preset, cx))
                        .child(Label::new(preset.label()).size(LabelSize::XSmall))
                })),
        );

    let suite_grid = h_flex()
        .w_full()
        .flex_wrap()
        .gap_2()
        .children(brand::suite::APHRODY_PILLARS.iter().enumerate().map(|(idx, pillar)| {
            let target = pillar_target_window(pillar.id);
            let cb_open = std::rc::Rc::clone(&on_open_window);
            v_flex()
                .id(("gallery-pillar-card", idx))
                .cursor_pointer()
                .w(px(236.0))
                .p_2p5()
                .gap_1()
                .rounded(px(12.0))
                .border_1()
                .border_color(theme.colors.outline_variant.opacity(0.45))
                .bg(theme.colors.surface_container)
                .hover(|s| s.border_color(theme.colors.primary))
                .on_click(move |_ev, _w, cx| cb_open(target, cx))
                .child(
                    h_flex()
                        .justify_between()
                        .gap_1()
                        .child(Label::new(pillar.name).size(LabelSize::Small).truncate())
                        .child(
                            div()
                                .px_1p5()
                                .py_0p5()
                                .rounded_full()
                                .bg(theme.colors.primary_container)
                                .child(
                                    Label::new(pillar.primary_crate)
                                        .size(LabelSize::XSmall)
                                        .color(ui::Color::Custom(
                                            theme.colors.on_primary_container,
                                        )),
                                ),
                        ),
                )
                .child(
                    Label::new(pillar.summary)
                        .size(LabelSize::XSmall)
                        .color(ui::Color::Muted),
                )
                .child(
                    Label::new(SharedString::from(format!("Launch {}", target.default_title())))
                        .size(LabelSize::XSmall)
                        .color(ui::Color::Accent),
                )
        }));

    v_flex()
        .id("design-gallery-scroll")
        .size_full()
        .p_3()
        .gap_3()
        .overflow_y_scroll()
        .bg(theme.colors.surface)
        .child(section_heading(
            theme,
            format!(
                "{} Hub ({} interactive pillars · Compose M3 1.5.0 · {})",
                brand::APP_NAME,
                brand::suite::APHRODY_PILLARS.len(),
                theme.name
            ),
        ))
        .child(material_glass_lab)
        .child(suite_grid)
        .child(section_heading(theme, format!("Google references · {}", theme.name)))
        .child(content)
        .into_any_element()
}
