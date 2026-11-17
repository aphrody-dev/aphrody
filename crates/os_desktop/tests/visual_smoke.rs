use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    AppContext as _, Bounds, HeadlessAppContext, Pixels, Render, WindowBounds, WindowOptions, div,
    prelude::*, px, relative, size,
};
use os_desktop::windows::WindowKind;
use os_desktop::{DesktopOptions, OsDesktopShell, bind_desktop_keys};
use os_session::TilingMode;
use os_ui::{AppearanceMode, ImmediateUi, KeyValueRow, OsThemeCatalog};
use ui::{Color, Label, LabelCommon as _, LabelSize, h_flex, v_flex};

fn screenshot_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("APHRODY_SHOT_DIR") {
        return PathBuf::from(custom);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/os_screenshots")
}

fn assert_non_blank_png(img: &image::RgbaImage, min_w: u32, min_h: u32, path: &std::path::Path) {
    assert!(
        img.width() >= min_w && img.height() >= min_h,
        "expected at least {min_w}x{min_h} pixels for {} (got {}x{})",
        path.display(),
        img.width(),
        img.height()
    );

    let mut distinct_buckets = std::collections::BTreeSet::new();
    for pixel in img.pixels().step_by(37) {
        let bucket = (pixel[0] / 16, pixel[1] / 16, pixel[2] / 16);
        distinct_buckets.insert(bucket);
    }
    assert!(
        distinct_buckets.len() >= 4,
        "rendered image {} looks nearly blank ({} buckets)",
        path.display(),
        distinct_buckets.len()
    );
}

#[test]
fn desktop_headless_renders_and_interacts() {
    let text_system = Arc::new(gpui_wgpu::CosmicTextSystem::new(brand::FONT_SANS_ALIAS));
    let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(assets::Assets), || {
        gpui_platform::current_headless_renderer()
    });

    cx.update(|app| {
        gpui_ui::init(app);
        bind_desktop_keys(app);
    });

    let bounds = Bounds::new(gpui::point(px(0.0), px(0.0)), size(px(1440.0), px(900.0)));
    let handle = cx
        .update(|app| {
            app.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |window, app| {
                    app.new(|cx| {
                        OsDesktopShell::new(
                            DesktopOptions {
                                initial_theme: Some("aphrody".to_owned()),
                                initial_mode: Some(AppearanceMode::Dark),
                                initial_app: Some("photocraft".to_owned()),
                                ..Default::default()
                            },
                            window,
                            cx,
                        )
                    })
                },
            )
        })
        .expect("headless desktop window must open");

    cx.run_until_parked();

    let out_dir = screenshot_dir();
    let can_capture_gpu = gpui_platform::current_headless_renderer()
        .ok()
        .flatten()
        .is_some();

    if can_capture_gpu {
        std::fs::create_dir_all(&out_dir).expect("create screenshot output dir");

        let shot_dark = cx
            .capture_screenshot(handle.into())
            .expect("capture Aphrody Dark studio screenshot");
        let p_dark = out_dir.join("aphrody_os_studio_dark.png");
        shot_dark.save(&p_dark).expect("save dark studio PNG");
        assert_non_blank_png(&shot_dark, 1440, 900, &p_dark);
    }

    // Mutate state through real desktop commands: tile multiple windows, switch light theme, toggle drawer.
    cx.update(|app| {
        handle
            .update(app, |shell, _window, cx| {
                shell.select_craft_app("vectorcraft", cx);
                shell.open_or_focus_window(WindowKind::Settings, "COSMIC Settings", cx);
                shell.open_or_focus_window(WindowKind::Monitor, "System Monitor", cx);
                shell.set_layout(TilingMode::CosmicGrid, cx);
                shell.toggle_mode(cx);
            })
            .expect("update desktop shell for light tiled view");
    });
    cx.run_until_parked();

    if can_capture_gpu {
        let shot_tiled_light = cx
            .capture_screenshot(handle.into())
            .expect("capture Light tiled multi-window screenshot");
        let p_light = out_dir.join("aphrody_os_tiled_light.png");
        shot_tiled_light
            .save(&p_light)
            .expect("save light tiled PNG");
        assert_non_blank_png(&shot_tiled_light, 1440, 900, &p_light);
    }

    // Switch to COSMIC dark theme + open Craft Anything + Launcher modal + ContextDrawer.
    cx.update(|app| {
        handle
            .update(app, |shell, _window, cx| {
                shell.toggle_mode(cx);
                shell.select_theme("cosmic", cx);
                shell.select_craft_app("craft-anything", cx);
                shell.toggle_drawer(cx);
                shell.toggle_launcher(cx);
            })
            .expect("update desktop shell for cosmic launcher + drawer view");
    });
    cx.run_until_parked();

    if can_capture_gpu {
        let shot_cosmic = cx
            .capture_screenshot(handle.into())
            .expect("capture COSMIC dark launcher + inspector drawer screenshot");
        let p_cosmic = out_dir.join("aphrody_os_cosmic_launcher.png");
        shot_cosmic
            .save(&p_cosmic)
            .expect("save cosmic launcher PNG");
        assert_non_blank_png(&shot_cosmic, 1440, 900, &p_cosmic);
    }

    // Verify adaptive layout at compact 800x600 viewport (handheld / small screen).
    let compact_bounds = Bounds::new(gpui::point(px(0.0), px(0.0)), size(px(800.0), px(600.0)));
    let compact_handle = cx
        .update(|app| {
            app.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(compact_bounds)),
                    ..Default::default()
                },
                |window, app| {
                    app.new(|cx| {
                        OsDesktopShell::new(
                            DesktopOptions {
                                initial_theme: Some("aphrody".to_owned()),
                                initial_mode: Some(AppearanceMode::Dark),
                                initial_app: Some("soundcraft".to_owned()),
                                width: 800.0,
                                height: 600.0,
                                ..Default::default()
                            },
                            window,
                            cx,
                        )
                    })
                },
            )
        })
        .expect("compact 800x600 window must open");
    cx.run_until_parked();

    if can_capture_gpu {
        let shot_compact = cx
            .capture_screenshot(compact_handle.into())
            .expect("capture compact 800x600 adaptive screenshot");
        let p_compact = out_dir.join("aphrody_os_compact_800x600.png");
        shot_compact
            .save(&p_compact)
            .expect("save compact 800x600 PNG");
        assert_non_blank_png(&shot_compact, 800, 600, &p_compact);
    }

    #[cfg(target_os = "linux")]
    assert!(
        can_capture_gpu,
        "Linux test environment on fr1 should expose WgpuHeadlessRenderer for real PNG visual proof"
    );
}

#[derive(Default, Clone)]
struct MeasuredRowBounds {
    row: Option<Bounds<Pixels>>,
    key_box: Option<Bounds<Pixels>>,
    val_box: Option<Bounds<Pixels>>,
    key_label: Option<Bounds<Pixels>>,
    val_label: Option<Bounds<Pixels>>,
}

struct InspectorBoundsHarness {
    width: f32,
    theme: os_ui::OsTheme,
    pairs: Vec<(&'static str, &'static str)>,
    measured: Vec<Rc<RefCell<MeasuredRowBounds>>>,
    card_children: Rc<RefCell<Vec<Bounds<Pixels>>>>,
}

impl Render for InspectorBoundsHarness {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let mut imm = ImmediateUi::new(&self.theme);
        imm.heading("PhotoCraft");
        imm.section("Inspection Report", |sec| {
            for &(k, v) in &self.pairs {
                sec.kv_row(k, v);
            }
            sec.separator();
            sec.color_swatch(
                "Primary Seed",
                self.theme.colors.primary,
                "Interactive Token",
            );
        });

        let inner_w = (self.width - 40.0).max(120.0);
        let mut probe_rows = v_flex().w(px(inner_w)).min_w_0().gap_1();
        for (idx, &(k, v)) in self.pairs.iter().enumerate() {
            let m = Rc::clone(&self.measured[idx]);
            let m_row = Rc::clone(&m);
            let m_key = Rc::clone(&m);
            let m_val = Rc::clone(&m);
            probe_rows = probe_rows
                .child(
                    h_flex()
                        .w_full()
                        .min_w_0()
                        .justify_between()
                        .items_center()
                        .gap_2()
                        .on_children_prepainted(move |bounds, _, _| {
                            let mut slot = m_row.borrow_mut();
                            if bounds.len() >= 2 {
                                slot.key_box = Some(bounds[0]);
                                slot.val_box = Some(bounds[1]);
                                slot.row = Some(Bounds::from_corners(
                                    bounds[0].origin,
                                    gpui::point(
                                        bounds[1].right(),
                                        bounds[0].bottom().max(bounds[1].bottom()),
                                    ),
                                ));
                            }
                        })
                        .child(
                            div()
                                .min_w_0()
                                .flex_shrink_0()
                                .max_w(relative(0.5))
                                .overflow_hidden()
                                .on_children_prepainted(move |bounds, _, _| {
                                    m_key.borrow_mut().key_label = bounds.first().copied();
                                })
                                .child(
                                    Label::new(k)
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
                                .on_children_prepainted(move |bounds, _, _| {
                                    m_val.borrow_mut().val_label = bounds.first().copied();
                                })
                                .child(Label::new(v).size(LabelSize::XSmall).truncate_middle()),
                        ),
                )
                .child(KeyValueRow::new(k, v));
        }

        let card_capture = Rc::clone(&self.card_children);
        v_flex()
            .w(px(self.width))
            .p_2p5()
            .gap_2()
            .bg(self.theme.colors.surface_container)
            .on_children_prepainted(move |bounds, _, _| {
                *card_capture.borrow_mut() = bounds;
            })
            .child(imm.finish())
            .child(probe_rows)
    }
}

#[test]
fn inspector_kv_rows_never_overlap_at_narrow_and_studio_widths() {
    let text_system = Arc::new(gpui_wgpu::CosmicTextSystem::new(brand::FONT_SANS_ALIAS));
    let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(assets::Assets), || {
        gpui_platform::current_headless_renderer()
    });

    let theme = OsThemeCatalog::bundled()
        .expect("bundled themes load")
        .resolve("aphrody", Some(AppearanceMode::Dark))
        .expect("aphrody dark theme resolves");

    cx.update(|app| {
        gpui_ui::init(app);
        theme.apply_to_gpui(app);
    });

    let pairs = vec![
        ("Raster", "resvg 160x107 RGBA8 (14418 opaque px)"),
        ("Inspector", "canvas · layers · histogram · color_profile"),
        ("Upstream", "../../aphrody-labs/vectorcraft.git"),
        (
            "Layers (12)",
            "gpui · nsh · gpui-kit · craft · cosmic · iced · egui · wgpu · taffy · blitz · winit · tauri-bun",
        ),
    ];

    for inspector_w in [228.0_f32, 292.0_f32, 310.0_f32] {
        let measured: Vec<_> = (0..pairs.len())
            .map(|_| Rc::new(RefCell::new(MeasuredRowBounds::default())))
            .collect();
        let card_children = Rc::new(RefCell::new(Vec::new()));

        let bounds = Bounds::new(gpui::point(px(0.0), px(0.0)), size(px(800.0), px(600.0)));
        let m_clone = measured.clone();
        let c_clone = Rc::clone(&card_children);
        let p_clone = pairs.clone();
        let t_clone = theme.clone();

        cx.update(|app| {
            app.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_window, app| {
                    app.new(|_| InspectorBoundsHarness {
                        width: inspector_w,
                        theme: t_clone,
                        pairs: p_clone,
                        measured: m_clone,
                        card_children: c_clone,
                    })
                },
            )
        })
        .expect("bounds harness window opens");

        cx.run_until_parked();

        let children = card_children.borrow();
        assert_eq!(children.len(), 2, "expected ImmediateUi tree + probe rows");
        assert!(
            children[0].size.width <= px(inspector_w - 20.0 + 0.5),
            "ImmediateUi overflowed inspector width {inspector_w}: {:?}",
            children[0]
        );

        for (idx, m) in measured.iter().enumerate() {
            let b = m.borrow();
            let row = b.row.expect("row bounds recorded");
            let key_box = b.key_box.expect("key_box bounds recorded");
            let val_box = b.val_box.expect("val_box bounds recorded");
            let key_label = b.key_label.expect("key_label bounds recorded");
            let val_label = b.val_label.expect("val_label bounds recorded");

            assert!(
                key_label.right() + px(4.0) <= val_label.left(),
                "width={inspector_w} pair {:?}: key_label ({:?}) overlapped or touched val_label ({:?})",
                pairs[idx],
                key_label,
                val_label
            );
            assert!(
                key_box.right() + px(4.0) <= val_box.left(),
                "width={inspector_w} pair {:?}: key_box ({:?}) overlapped val_box ({:?})",
                pairs[idx],
                key_box,
                val_box
            );
            assert!(
                val_label.left() >= val_box.left() - px(0.5),
                "width={inspector_w} pair {:?}: val_label ({:?}) bled left past val_box ({:?})",
                pairs[idx],
                val_label,
                val_box
            );
            assert!(
                val_label.right() <= row.right() + px(0.5),
                "width={inspector_w} pair {:?}: val_label ({:?}) overflowed right past row ({:?})",
                pairs[idx],
                val_label,
                row
            );
            assert!(
                row.size.height <= px(28.0),
                "width={inspector_w} pair {:?}: row wrapped unexpectedly tall ({:?})",
                pairs[idx],
                row.size.height
            );
        }
    }
}

#[test]
fn ai_pilot_drives_gpui_desktop_shell_and_updates_hud() {
    let text_system = Arc::new(gpui_wgpu::CosmicTextSystem::new(brand::FONT_SANS_ALIAS));
    let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(assets::Assets), || {
        gpui_platform::current_headless_renderer()
    });

    cx.update(|app| {
        gpui_ui::init(app);
        bind_desktop_keys(app);
    });

    let bounds = Bounds::new(gpui::point(px(0.0), px(0.0)), size(px(1440.0), px(900.0)));
    let handle = cx
        .update(|app| {
            app.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |window, app| {
                    app.new(|cx| {
                        OsDesktopShell::new(
                            DesktopOptions {
                                initial_theme: Some("aphrody".to_owned()),
                                initial_mode: Some(AppearanceMode::Dark),
                                ..Default::default()
                            },
                            window,
                            cx,
                        )
                    })
                },
            )
        })
        .expect("desktop shell window opens");

    cx.run_until_parked();

    handle
        .update(&mut cx, |shell, _win, cx| {
            let snap = shell
                .pilot_execute_step(
                    "desktop_snapshot",
                    serde_json::json!({ "interactive": true }),
                    os_pilot::TargetBackend::Gpui,
                    cx,
                )
                .expect("AI snapshot inside live shell");
            assert!(snap["ref_count"].as_u64().unwrap_or(0) >= 6);

            let switched = shell
                .pilot_execute_step(
                    "desktop_ai_act_verify",
                    serde_json::json!({
                        "locator": { "role": "button", "name_contains": "Workspace 2" },
                        "action": { "kind": "click" },
                        "include_post_snapshot": true
                    }),
                    os_pilot::TargetBackend::Gpui,
                    cx,
                )
                .expect("AI clicks Workspace 2");
            assert_eq!(switched["action_result"]["verified"], true);

            let dark_toggled = shell
                .pilot_execute_step(
                    "desktop_ai_act_verify",
                    serde_json::json!({
                        "locator": { "role": "switch", "identifier": "shell.dark_mode" },
                        "action": { "kind": "toggle" },
                        "include_post_snapshot": true
                    }),
                    os_pilot::TargetBackend::Gpui,
                    cx,
                )
                .expect("AI toggles Dark Mode");
            assert_eq!(dark_toggled["action_result"]["verified"], true);
            assert!(
                shell.pilot.recent_hud_activity.len() >= 2,
                "HUD activity trail must record AI actions"
            );
        })
        .expect("handle update succeeds");

    cx.run_until_parked();
}
