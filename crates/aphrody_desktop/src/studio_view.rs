use std::path::Path;
use std::sync::Arc;

use gpui::{
    AnyElement, App, Bounds, Hsla, PathBuilder, Pixels, Role, SharedString, Window, canvas, div,
    point, prelude::*, px, size,
};
use aphrody_craft::{CraftAppSpec, CraftCatalog, InspectionReport, inspect_bytes, inspect_path};
use aphrody_ui::{
    AdaptiveViewport, ImmediateMemory, ImmediateUi, NavItem, OsTheme, StatusTone,
    focusable_action_chip, render_nav_rail, slider_control, sparkline_chart, status_badge,
    toggle_control,
};
use ui::{
    Icon, IconName, IconSize, Label, LabelCommon as _, LabelSize, h_flex, rems_from_px, v_flex,
};

use crate::parse_icon_name;

const SAMPLE_VECTOR_SVG: &[u8] =
    br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 240 160">
  <rect x="8" y="8" width="224" height="144" rx="18" fill="#135B82"/>
  <circle cx="88" cy="80" r="42" fill="#D1D293"/>
  <path d="M132 42 L204 118 L132 118 Z" fill="#F1F5F9" opacity="0.9"/>
</svg>"##;

const SAMPLE_WASM_MODULE: &[u8] = b"\0asm\x01\x00\x00\x00\x01\x05\x01\x60\x01\x7f\x00\x03\x02\x01\x00\x07\x0a\x01\x06render\x00\x00\x0a\x04\x01\x02\x00\x0b";

/// State backing the unified 17-app ArtCraft + Craft Anything studio window.
#[derive(Debug, Clone)]
pub struct StudioState {
    pub active_app_id: String,
    pub active_tool_idx: usize,
    pub zoom_percent: f32,
    pub exposure_ev: f32,
    pub grid_size_px: f32,
    pub snap_to_grid: bool,
    pub high_contrast_guides: bool,
    pub active_report: Option<InspectionReport>,
    pub svg_raster_summary: Option<String>,
    pub memory: ImmediateMemory,
}

impl StudioState {
    pub fn new(catalog: &CraftCatalog, initial_app: Option<&str>) -> Self {
        let active_app_id = initial_app
            .and_then(|id| catalog.by_id(id))
            .map(|app| app.id.clone())
            .or_else(|| catalog.apps.first().map(|app| app.id.clone()))
            .unwrap_or_else(|| "photocraft".to_owned());

        let mut state = Self {
            active_app_id,
            active_tool_idx: 0,
            zoom_percent: 100.0,
            exposure_ev: 0.25,
            grid_size_px: 24.0,
            snap_to_grid: true,
            high_contrast_guides: true,
            active_report: None,
            svg_raster_summary: None,
            memory: ImmediateMemory::default(),
        };
        state.load_svg_sample(catalog);
        state
    }

    pub fn select_app(&mut self, app_id: &str, catalog: &CraftCatalog) {
        if let Some(app) = catalog.by_id(app_id) {
            self.active_app_id = app.id.clone();
            self.active_tool_idx = 0;
            if app.id == "craft-anything" {
                self.load_self_binary_sample(catalog);
            } else if app.id == "vectorcraft" || app.id == "figcraft" {
                self.load_svg_sample(catalog);
            }
        }
    }

    pub fn load_svg_sample(&mut self, catalog: &CraftCatalog) {
        let report = inspect_bytes(
            "aphrody_brand_specimen.svg",
            SAMPLE_VECTOR_SVG,
            Some(Path::new("aphrody_brand_specimen.svg")),
            Some(catalog),
        );
        self.svg_raster_summary = match aphrody_craft::render_svg_rgba(SAMPLE_VECTOR_SVG, 160) {
            Ok((w, h, rgba)) => {
                let non_zero = rgba.chunks_exact(4).filter(|px| px[3] > 0).count();
                Some(format!("resvg {w}x{h} RGBA8 ({non_zero} opaque px)"))
            }
            Err(err) => Some(format!("SVG raster note: {err}")),
        };
        self.active_report = Some(report);
    }

    pub fn load_wasm_sample(&mut self, catalog: &CraftCatalog) {
        let report = inspect_bytes(
            "shader_kernel.wasm",
            SAMPLE_WASM_MODULE,
            Some(Path::new("shader_kernel.wasm")),
            Some(catalog),
        );
        self.svg_raster_summary = None;
        self.active_report = Some(report);
    }

    pub fn load_self_binary_sample(&mut self, catalog: &CraftCatalog) {
        if let Ok(exe_path) = std::env::current_exe()
            && let Ok(report) = inspect_path(&exe_path, Some(catalog))
        {
            self.svg_raster_summary = None;
            self.active_report = Some(report);
            return;
        }
        self.load_wasm_sample(catalog);
    }

    pub fn inspect_local_file(&mut self, path: &Path, catalog: &CraftCatalog) {
        match inspect_path(path, Some(catalog)) {
            Ok(report) => {
                if let Some(app_id) = report.recommended_app.as_deref()
                    && let Some(app) = catalog.by_id(app_id)
                {
                    self.active_app_id = app.id.clone();
                    self.active_tool_idx = 0;
                }
                self.svg_raster_summary = None;
                self.active_report = Some(report);
            }
            Err(err) => {
                log::warn!("failed to inspect {}: {err:#}", path.display());
            }
        }
    }
}

/// Callback table decoupling `studio_view` from parent entity storage.
pub struct StudioCallbacks {
    pub on_select_app: Arc<dyn Fn(String, &mut Window, &mut App) + Send + Sync>,
    pub on_select_tool: Arc<dyn Fn(usize, &mut Window, &mut App) + Send + Sync>,
    pub on_zoom: Arc<dyn Fn(f32, &mut Window, &mut App) + Send + Sync>,
    pub on_exposure: Arc<dyn Fn(f32, &mut Window, &mut App) + Send + Sync>,
    pub on_grid_size: Arc<dyn Fn(f32, &mut Window, &mut App) + Send + Sync>,
    pub on_toggle_snap: Arc<dyn Fn(bool, &mut Window, &mut App) + Send + Sync>,
    pub on_toggle_guides: Arc<dyn Fn(bool, &mut Window, &mut App) + Send + Sync>,
    pub on_sample_svg: Arc<dyn Fn(&mut Window, &mut App) + Send + Sync>,
    pub on_sample_wasm: Arc<dyn Fn(&mut Window, &mut App) + Send + Sync>,
    pub on_sample_self_bin: Arc<dyn Fn(&mut Window, &mut App) + Send + Sync>,
    pub on_toggle_section: Arc<dyn Fn(String, &mut Window, &mut App) + Send + Sync>,
}

/// Renders the complete ArtCraft Studio window: catalog `NavRail`, parametric GPU canvas, and `ImmediateUi` inspector.
pub fn render_studio_window(
    state: &StudioState,
    catalog: &CraftCatalog,
    frame_samples: &[f32],
    window_width: f32,
    theme: &OsTheme,
    cb: StudioCallbacks,
) -> AnyElement {
    let active_app: Option<&CraftAppSpec> = catalog
        .by_id(&state.active_app_id)
        .or_else(|| catalog.apps.first());

    let nav_items: Vec<NavItem> = catalog
        .apps
        .iter()
        .map(|app| NavItem {
            id: SharedString::from(app.id.clone()),
            label: SharedString::from(app.name.clone()),
            icon: parse_icon_name(&app.icon),
            // A count of supported extensions read as an unread-notification badge; formats are
            // listed under the canvas instead.
            badge: None,
        })
        .collect();

    let vp = AdaptiveViewport::from_size(window_width, 900.0);
    let compact_rail = vp.compact_nav_rail;
    let stacked_inspector = vp.stacked_inspector;
    let inspector_width = vp.studio_inspector_width;
    let on_select_app = Arc::clone(&cb.on_select_app);
    let rail = render_nav_rail(
        "studio-catalog-rail",
        &nav_items,
        active_app.map_or("", |a| a.id.as_str()),
        compact_rail,
        theme,
        move |app_id, window, cx| on_select_app(app_id.to_string(), window, cx),
    );

    let center_pane = render_center_workspace(state, active_app, compact_rail, theme, &cb);
    let inspector_pane = render_inspector_pane(
        state,
        active_app,
        frame_samples,
        inspector_width,
        stacked_inspector,
        theme,
        &cb,
    );

    h_flex()
        .w_full()
        .flex_1()
        .min_h(px(0.0))
        .overflow_hidden()
        .child(rail)
        .child(
            div()
                .flex_1()
                .h_full()
                .min_w(px(0.0))
                .flex()
                .when(stacked_inspector, |el| el.flex_col())
                .when(!stacked_inspector, |el| el.flex_row())
                .overflow_hidden()
                .child(center_pane)
                .child(inspector_pane),
        )
        .into_any_element()
}

fn render_center_workspace(
    state: &StudioState,
    active_app: Option<&CraftAppSpec>,
    compact: bool,
    theme: &OsTheme,
    cb: &StudioCallbacks,
) -> AnyElement {
    let app_name = active_app.map_or("Studio", |a| a.name.as_str());
    let app_tagline = active_app.map_or("Unified creative & inspection workbench", |a| {
        a.tagline.as_str()
    });
    let app_category = active_app.map_or("image", |a| a.category.as_str());
    let app_icon = active_app.map_or(IconName::Blocks, |a| parse_icon_name(&a.icon));

    let on_svg = Arc::clone(&cb.on_sample_svg);
    let on_wasm = Arc::clone(&cb.on_sample_wasm);
    let on_bin = Arc::clone(&cb.on_sample_self_bin);

    let mut tools_bar = h_flex()
        .w_full()
        .px_1p5()
        .py_1()
        .gap_1p5()
        .flex_wrap()
        .items_center()
        .rounded_full()
        .bg(theme.colors.surface_container_low)
        .border_1()
        .border_color(theme.colors.outline_variant.opacity(0.35));
    if let Some(app) = active_app {
        for (idx, tool_name) in app.tools.iter().enumerate() {
            let selected = idx == state.active_tool_idx;
            let on_tool = Arc::clone(&cb.on_select_tool);
            tools_bar = tools_bar.child(focusable_action_chip(
                gpui::ElementId::Name(format!("studio-tool-{idx}").into()),
                IconName::ToolPencil,
                SharedString::from(tool_name.replace('_', " ")),
                None,
                SharedString::from(format!("Select {} tool {tool_name}", app.name)),
                selected,
                (100 + idx) as isize,
                theme,
                move |_, window, cx| on_tool(idx, window, cx),
            ));
        }
    }

    let canvas_widget = render_parametric_canvas(state, app_category, theme);

    let ext_line = active_app
        .map(|app| {
            app.extensions
                .iter()
                .map(|ext| format!(".{ext}"))
                .collect::<Vec<_>>()
                .join("  ")
        })
        .unwrap_or_default();

    v_flex()
        .flex_1()
        .h_full()
        .min_w(px(0.0))
        .p_3()
        .gap_2p5()
        .bg(theme.colors.surface)
        .child(
            h_flex()
                .w_full()
                .flex_wrap()
                .justify_between()
                .items_center()
                .gap_x_3()
                .gap_y_2()
                .child(
                    h_flex()
                        .min_w(px(300.0))
                        .flex_1()
                        .gap_2p5()
                        .items_center()
                        .child(
                            div()
                                .w(px(36.0))
                                .h(px(36.0))
                                .flex_shrink_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(12.0))
                                .bg(theme.colors.primary_container)
                                .text_color(theme.colors.on_primary_container)
                                .child(
                                    Icon::new(app_icon)
                                        .size(IconSize::Medium)
                                        .color(ui::Color::Custom(theme.colors.on_primary_container)),
                                ),
                        )
                        .child(
                            v_flex()
                                .min_w(px(0.0))
                                .flex_1()
                                .overflow_hidden()
                                .gap_0p5()
                                .child(
                                    Label::new(app_name.to_owned())
                                        .size(LabelSize::Large)
                                        .truncate(),
                                )
                                .when(!compact, |el| {
                                    el.child(
                                        Label::new(app_tagline.to_owned())
                                            .size(LabelSize::XSmall)
                                            .color(ui::Color::Muted)
                                            .truncate(),
                                    )
                                }),
                        ),
                )
                .child(
                    h_flex()
                        .flex_shrink_0()
                        .gap_1p5()
                        .flex_wrap()
                        .items_center()
                        .child(focusable_action_chip(
                            "sample-svg",
                            IconName::Image,
                            SharedString::from("SVG Sample"),
                            None,
                            SharedString::from("Inspect and rasterize built-in SVG asset"),
                            false,
                            110,
                            theme,
                            move |_, window, cx| on_svg(window, cx),
                        ))
                        .child(focusable_action_chip(
                            "sample-wasm",
                            IconName::Code,
                            SharedString::from("Wasm"),
                            None,
                            SharedString::from("Inspect built-in WebAssembly module"),
                            false,
                            111,
                            theme,
                            move |_, window, cx| on_wasm(window, cx),
                        ))
                        .child(focusable_action_chip(
                            "sample-binary",
                            IconName::Binary,
                            SharedString::from("Self Binary"),
                            None,
                            SharedString::from("Inspect running aphrody-os executable"),
                            false,
                            112,
                            theme,
                            move |_, window, cx| on_bin(window, cx),
                        )),
                ),
        )
        .child(tools_bar)
        .child(
            div()
                .id("studio-viewport")
                .role(Role::Region)
                .aria_label(SharedString::from(format!("{app_name} viewport")))
                .flex_1()
                .w_full()
                .min_h(px(180.0))
                .rounded(px(16.0))
                .overflow_hidden()
                .bg(theme.colors.surface_container_lowest)
                .border_1()
                .border_color(theme.colors.outline_variant.opacity(0.45))
                .child(canvas_widget),
        )
        .child(
            h_flex()
                .w_full()
                .min_w(px(0.0))
                .justify_between()
                .items_center()
                .gap_2()
                .child(
                    div().flex_1().min_w(px(0.0)).overflow_hidden().child(
                        Label::new(format!("Formats: {ext_line}"))
                            .size(LabelSize::XSmall)
                            .color(ui::Color::Muted)
                            .truncate_middle(),
                    ),
                )
                .child(status_badge(
                    IconName::Check,
                    format!("{:.0}% · {:+.2} EV", state.zoom_percent, state.exposure_ev),
                    StatusTone::Accent,
                    theme,
                )),
        )
        .into_any_element()
}

fn render_parametric_canvas(state: &StudioState, category: &str, theme: &OsTheme) -> AnyElement {
    let snap = state.snap_to_grid;
    let guides = state.high_contrast_guides;
    let grid_step = state.grid_size_px.max(12.0);
    let zoom = (state.zoom_percent / 100.0).clamp(0.4, 2.4);
    let exposure = state.exposure_ev;
    let category = category.to_owned();

    let primary = theme.colors.primary;
    let secondary = theme.colors.secondary;
    let tertiary = theme.colors.tertiary;
    let grid_color = if guides {
        theme.colors.outline
    } else {
        theme.colors.outline_variant
    };
    let card_bg = theme.colors.surface_container_low;
    let on_bg = theme.colors.on_surface;

    canvas(
        |_bounds, _window, _cx| (),
        move |bounds: Bounds<Pixels>, (), window, _cx| {
            if snap {
                let mut x = bounds.origin.x + px(grid_step);
                while x < bounds.origin.x + bounds.size.width {
                    let mut vertical = PathBuilder::stroke(px(0.5));
                    vertical.move_to(point(x, bounds.origin.y));
                    vertical.line_to(point(x, bounds.origin.y + bounds.size.height));
                    if let Ok(path) = vertical.build() {
                        window.paint_path(
                            path,
                            Hsla {
                                a: 0.14,
                                ..grid_color
                            },
                        );
                    }
                    x += px(grid_step);
                }
                let mut y = bounds.origin.y + px(grid_step);
                while y < bounds.origin.y + bounds.size.height {
                    let mut horiz = PathBuilder::stroke(px(0.5));
                    horiz.move_to(point(bounds.origin.x, y));
                    horiz.line_to(point(bounds.origin.x + bounds.size.width, y));
                    if let Ok(path) = horiz.build() {
                        window.paint_path(
                            path,
                            Hsla {
                                a: 0.14,
                                ..grid_color
                            },
                        );
                    }
                    y += px(grid_step);
                }
            }

            let center = point(
                bounds.origin.x + bounds.size.width * 0.5,
                bounds.origin.y + bounds.size.height * 0.5,
            );
            let art_w = px((320.0 * zoom).min(f32::from(bounds.size.width) * 0.84));
            let art_h = px((200.0 * zoom).min(f32::from(bounds.size.height) * 0.80));
            let art_bounds = Bounds::new(
                point(center.x - art_w * 0.5, center.y - art_h * 0.5),
                size(art_w, art_h),
            );
            let shadow_bounds = Bounds::new(
                point(art_bounds.origin.x + px(4.0), art_bounds.origin.y + px(8.0)),
                art_bounds.size,
            );
            window.paint_quad(
                gpui::fill(shadow_bounds, gpui::black().opacity(0.32)).corner_radii(px(16.0)),
            );
            window.paint_quad(
                gpui::fill(art_bounds, card_bg)
                    .corner_radii(px(14.0))
                    .border_widths(px(1.0))
                    .border_color(Hsla {
                        a: 0.35,
                        ..grid_color
                    }),
            );

            let lum_shift = (exposure * 0.08).clamp(-0.25, 0.25);
            let tone_primary = Hsla {
                l: (primary.l + lum_shift).clamp(0.08, 0.92),
                ..primary
            };

            match category.as_str() {
                "vector" | "layout" => {
                    let mut bezier = PathBuilder::stroke(px(2.5));
                    bezier.move_to(point(
                        art_bounds.origin.x + art_w * 0.14,
                        art_bounds.origin.y + art_h * 0.72,
                    ));
                    bezier.curve_to(
                        point(
                            art_bounds.origin.x + art_w * 0.86,
                            art_bounds.origin.y + art_h * 0.72,
                        ),
                        point(
                            art_bounds.origin.x + art_w * 0.35,
                            art_bounds.origin.y + art_h * 0.15,
                        ),
                    );
                    if let Ok(path) = bezier.build() {
                        window.paint_path(path, tone_primary);
                    }
                    let node_box = Bounds::new(
                        point(
                            art_bounds.origin.x + art_w * 0.60,
                            art_bounds.origin.y + art_h * 0.24,
                        ),
                        size(px(46.0 * zoom), px(46.0 * zoom)),
                    );
                    window.paint_quad(gpui::fill(
                        node_box,
                        Hsla {
                            a: 0.85,
                            ..secondary
                        },
                    ));
                }
                "audio" => {
                    let bars = 18_usize;
                    let bar_step = art_w * 0.8 / bars as f32;
                    for idx in 0..bars {
                        let phase = ((idx as f32 * 0.7 + exposure * 2.0).sin().abs() * 0.65 + 0.18)
                            .clamp(0.15, 0.88);
                        let h = art_h * phase * 0.7;
                        let r = Bounds::new(
                            point(
                                art_bounds.origin.x + art_w * 0.1 + bar_step * idx as f32,
                                center.y - h * 0.5,
                            ),
                            size((bar_step * 0.65).max(px(3.0)), h),
                        );
                        let color = if idx % 3 == 0 {
                            secondary
                        } else {
                            tone_primary
                        };
                        window.paint_quad(gpui::fill(r, color));
                    }
                }
                "engineering" => {
                    for row in 0..5_usize {
                        let width_frac = 0.25 + ((row * 37) % 60) as f32 / 100.0;
                        let band = Bounds::new(
                            point(
                                art_bounds.origin.x + art_w * 0.08,
                                art_bounds.origin.y + art_h * (0.14 + row as f32 * 0.16),
                            ),
                            size(art_w * width_frac, px(14.0)),
                        );
                        let color = match row % 3 {
                            0 => tone_primary,
                            1 => secondary,
                            _ => tertiary,
                        };
                        window.paint_quad(gpui::fill(band, color));
                    }
                }
                _ => {
                    let layer_a = Bounds::new(
                        point(
                            art_bounds.origin.x + art_w * 0.08,
                            art_bounds.origin.y + art_h * 0.14,
                        ),
                        size(art_w * 0.48, art_h * 0.56),
                    );
                    let layer_b = Bounds::new(
                        point(
                            art_bounds.origin.x + art_w * 0.36,
                            art_bounds.origin.y + art_h * 0.26,
                        ),
                        size(art_w * 0.44, art_h * 0.50),
                    );
                    let layer_c = Bounds::new(
                        point(
                            art_bounds.origin.x + art_w * 0.64,
                            art_bounds.origin.y + art_h * 0.12,
                        ),
                        size(art_w * 0.26, art_h * 0.34),
                    );
                    window.paint_quad(
                        gpui::fill(layer_a, tone_primary).corner_radii(px(14.0)),
                    );
                    window.paint_quad(
                        gpui::fill(
                            layer_b,
                            Hsla {
                                a: 0.84,
                                ..secondary
                            },
                        )
                        .corner_radii(px(18.0)),
                    );
                    window.paint_quad(
                        gpui::fill(
                            layer_c,
                            Hsla {
                                a: 0.78,
                                ..tertiary
                            },
                        )
                        .corner_radii(px(28.0)),
                    );
                    // Expressive tone-curve spline overlay across the artboard
                    let mut spline = PathBuilder::stroke(px(2.0));
                    spline.move_to(point(
                        art_bounds.origin.x + art_w * 0.08,
                        art_bounds.origin.y + art_h * 0.82,
                    ));
                    spline.curve_to(
                        point(
                            art_bounds.origin.x + art_w * 0.92,
                            art_bounds.origin.y + art_h * 0.20,
                        ),
                        point(
                            art_bounds.origin.x + art_w * 0.42,
                            art_bounds.origin.y + art_h * 0.18,
                        ),
                    );
                    if let Ok(path) = spline.build() {
                        window.paint_path(path, Hsla { a: 0.72, ..on_bg });
                    }
                    // Bottom chromatic exposure swatch ribbon
                    for (idx, swatch) in [tone_primary, secondary, tertiary].into_iter().enumerate() {
                        let sw = Bounds::new(
                            point(
                                art_bounds.origin.x + art_w * (0.08 + idx as f32 * 0.10),
                                art_bounds.origin.y + art_h * 0.86,
                            ),
                            size(art_w * 0.08, px(6.0)),
                        );
                        window.paint_quad(gpui::fill(sw, swatch).corner_radii(px(3.0)));
                    }
                }
            }

            if guides {
                let mut frame = PathBuilder::stroke(px(1.5));
                frame.move_to(art_bounds.origin);
                frame.line_to(point(
                    art_bounds.origin.x + art_bounds.size.width,
                    art_bounds.origin.y,
                ));
                frame.line_to(point(
                    art_bounds.origin.x + art_bounds.size.width,
                    art_bounds.origin.y + art_bounds.size.height,
                ));
                frame.line_to(point(
                    art_bounds.origin.x,
                    art_bounds.origin.y + art_bounds.size.height,
                ));
                frame.close();
                if let Ok(path) = frame.build() {
                    window.paint_path(path, Hsla { a: 0.55, ..on_bg });
                }
            }
        },
    )
    .size_full()
    .into_any_element()
}

fn render_inspector_pane(
    state: &StudioState,
    active_app: Option<&CraftAppSpec>,
    frame_samples: &[f32],
    inspector_width: f32,
    stacked: bool,
    theme: &OsTheme,
    cb: &StudioCallbacks,
) -> AnyElement {
    let mut ui = ImmediateUi::new(theme);
    ui.heading(active_app.map_or("Studio Inspector", |a| a.name.as_str()));

    let on_zoom = Arc::clone(&cb.on_zoom);
    let on_exp = Arc::clone(&cb.on_exposure);
    let on_grid = Arc::clone(&cb.on_grid_size);
    let on_snap = Arc::clone(&cb.on_toggle_snap);
    let on_guides = Arc::clone(&cb.on_toggle_guides);

    ui.section("Viewport Parameters", |sec| {
        sec.push_element(slider_control(
            "studio-zoom",
            "Zoom",
            state.zoom_percent,
            40.0,
            240.0,
            "%",
            sec.theme,
            move |v, window, cx| on_zoom(v, window, cx),
        ));
        sec.push_element(slider_control(
            "studio-exposure",
            "Exposure",
            state.exposure_ev,
            -2.0,
            2.0,
            " EV",
            sec.theme,
            move |v, window, cx| on_exp(v, window, cx),
        ));
        sec.push_element(slider_control(
            "studio-grid",
            "Grid Step",
            state.grid_size_px,
            12.0,
            64.0,
            "px",
            sec.theme,
            move |v, window, cx| on_grid(v, window, cx),
        ));
        sec.push_element(toggle_control(
            "studio-snap",
            "Snap Grid",
            state.snap_to_grid,
            sec.theme,
            move |next, window, cx| on_snap(next, window, cx),
        ));
        sec.push_element(toggle_control(
            "studio-guides",
            "Vector Guides",
            state.high_contrast_guides,
            sec.theme,
            move |next, window, cx| on_guides(next, window, cx),
        ));
    });

    if let Some(report) = &state.active_report {
        let report_open = state.memory.is_open("inspection");
        let on_sec = Arc::clone(&cb.on_toggle_section);
        ui.button(
            "toggle-sec-inspection",
            if report_open {
                IconName::ChevronDown
            } else {
                IconName::ChevronRight
            },
            "Craft-Libs Inspection",
            report_open,
            move |_, window, cx| on_sec("inspection".to_owned(), window, cx),
        );

        if report_open {
            ui.section("Inspection Report", |sec| {
                sec.kv_row("File", report.source_name.clone());
                sec.kv_row(
                    "Format",
                    format!("{} ({})", report.format_name, report.family),
                );
                sec.kv_row("MIME", report.mime_type.clone());
                sec.kv_row("Bytes", format!("{} B", report.byte_len));
                sec.kv_row("Entropy", format!("{:.3} bits/B", report.shannon_entropy));
                if let Some((w, h)) = report.dimensions {
                    sec.kv_row("Dimensions", format!("{w} x {h} px"));
                }
                if let Some(summary) = &state.svg_raster_summary {
                    sec.kv_row("Raster", summary.clone());
                }
                if let Some(rec) = &report.recommended_app {
                    sec.kv_row("Routed App", rec.clone());
                }
                if !report.layers.is_empty() {
                    sec.kv_row("Layers", report.layers.len().to_string());
                }
                if let Some(bin) = &report.binary {
                    sec.kv_row("Container", format!("{} / {}", bin.container, bin.arch));
                    sec.kv_row("Sections", bin.sections.len().to_string());
                    sec.kv_row("Symbols", bin.symbol_count.to_string());
                    if !bin.wasm_exports.is_empty() {
                        sec.kv_row("Wasm Exports", bin.wasm_exports.join(", "));
                    }
                }
            });
        }
    }

    if let Some(app) = active_app {
        ui.section("Active Groups", |sec| {
            sec.kv_row("Upstream", app.upstream.clone());
            sec.kv_row("Inspector", app.inspector_groups.join(" · "));
        });
    }

    ui.push_element(sparkline_chart("Render Frame", "ms", frame_samples, theme));

    v_flex()
        .id("studio-inspector-pane")
        .role(Role::Region)
        .aria_label("Studio Inspector")
        .when(!stacked, |el| {
            el.flex_shrink_0()
                .w(rems_from_px(inspector_width))
                .h_full()
                .border_l_1()
        })
        .when(stacked, |el| {
            el.w_full()
                .min_w_0()
                .h(rems_from_px(210.0_f32))
                .border_t_1()
        })
        .p_2p5()
        .pb_6()
        .overflow_y_scroll()
        .bg(theme.colors.surface_container)
        .border_color(theme.colors.outline_variant)
        .child(ui.finish())
        .into_any_element()
}
