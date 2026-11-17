//! COSMIC Epoch bottom dock (`cosmic-dock`) renderer with adaptive compact layout & active indicators.

use gpui::{AnyElement, Context, Role, SharedString, div, prelude::*, px};
use aphrody_ui::{AdaptiveViewport, OsTheme};
use ui::{Icon, IconName, IconSize, Tooltip, h_flex};

use crate::shell::OsDesktopShell;
use crate::windows::WindowKind;

pub struct DockTileSpec {
    pub id: &'static str,
    pub icon: IconName,
    pub label: SharedString,
    pub kind: Option<WindowKind>,
    pub craft_id: Option<&'static str>,
}

/// Renders the interactive `cosmic-dock` launcher strip at the bottom of the shell.
pub fn render_cosmic_dock(
    shell: &OsDesktopShell,
    theme: &OsTheme,
    dock_h: f32,
    vw: f32,
    cx: &mut Context<OsDesktopShell>,
) -> AnyElement {
    let compact_dock = AdaptiveViewport::from_size(vw, dock_h).compact_dock;
    let items: Vec<DockTileSpec> = if compact_dock {
        vec![
            DockTileSpec {
                id: "dock-launcher",
                icon: IconName::Command,
                label: SharedString::from("Launcher"),
                kind: None,
                craft_id: None,
            },
            DockTileSpec {
                id: "dock-studio",
                icon: IconName::Image,
                label: SharedString::from("ArtCraft Studio"),
                kind: Some(WindowKind::Studio),
                craft_id: None,
            },
            DockTileSpec {
                id: "dock-craft-anything",
                icon: IconName::Binary,
                label: SharedString::from("Craft Anything"),
                kind: None,
                craft_id: Some("craft-anything"),
            },
            DockTileSpec {
                id: "dock-files",
                icon: IconName::FolderOpen,
                label: SharedString::from("Files"),
                kind: Some(WindowKind::Files),
                craft_id: None,
            },
            DockTileSpec {
                id: "dock-settings",
                icon: IconName::Settings,
                label: SharedString::from("Settings"),
                kind: Some(WindowKind::Settings),
                craft_id: None,
            },
            DockTileSpec {
                id: "dock-monitor",
                icon: IconName::Server,
                label: SharedString::from("Monitor"),
                kind: Some(WindowKind::Monitor),
                craft_id: None,
            },
        ]
    } else {
        vec![
            DockTileSpec {
                id: "dock-launcher",
                icon: IconName::Command,
                label: SharedString::from("Launcher"),
                kind: None,
                craft_id: None,
            },
            DockTileSpec {
                id: "dock-studio",
                icon: IconName::Image,
                label: SharedString::from("ArtCraft Studio"),
                kind: Some(WindowKind::Studio),
                craft_id: None,
            },
            DockTileSpec {
                id: "dock-photocraft",
                icon: IconName::Sparkle,
                label: SharedString::from("PhotoCraft"),
                kind: None,
                craft_id: Some("photocraft"),
            },
            DockTileSpec {
                id: "dock-vectorcraft",
                icon: IconName::Pencil,
                label: SharedString::from("VectorCraft"),
                kind: None,
                craft_id: Some("vectorcraft"),
            },
            DockTileSpec {
                id: "dock-craft-anything",
                icon: IconName::Binary,
                label: SharedString::from("Craft Anything"),
                kind: None,
                craft_id: Some("craft-anything"),
            },
            DockTileSpec {
                id: "dock-files",
                icon: IconName::FolderOpen,
                label: SharedString::from("Files"),
                kind: Some(WindowKind::Files),
                craft_id: None,
            },
            DockTileSpec {
                id: "dock-settings",
                icon: IconName::Settings,
                label: SharedString::from("Settings"),
                kind: Some(WindowKind::Settings),
                craft_id: None,
            },
            DockTileSpec {
                id: "dock-monitor",
                icon: IconName::Server,
                label: SharedString::from("Monitor"),
                kind: Some(WindowKind::Monitor),
                craft_id: None,
            },
            DockTileSpec {
                id: "dock-term",
                icon: IconName::Terminal,
                label: SharedString::from("Terminal"),
                kind: Some(WindowKind::Term),
                craft_id: None,
            },
            DockTileSpec {
                id: "dock-edit",
                icon: IconName::FileCode,
                label: SharedString::from("Editor"),
                kind: Some(WindowKind::Edit),
                craft_id: None,
            },
            DockTileSpec {
                id: "dock-store",
                icon: IconName::Download,
                label: SharedString::from("Store"),
                kind: Some(WindowKind::Store),
                craft_id: None,
            },
            DockTileSpec {
                id: "dock-installer",
                icon: IconName::Code,
                label: SharedString::from("Installer / DE"),
                kind: Some(WindowKind::Installer),
                craft_id: None,
            },
        ]
    };

    let tile_size = if compact_dock {
        38.0
    } else {
        theme.layout.min_touch_target
    };

    let active_kind = shell
        .compositor
        .focused_toplevel
        .and_then(|id| shell.window_kinds.get(&id).copied());

    let mut dock_row = h_flex()
        .px_3()
        .py_1p5()
        .gap_2()
        .items_center()
        .rounded(px(16.0))
        .bg(theme.colors.surface_container_high)
        .border_1()
        .border_color(theme.colors.outline_variant);

    for (idx, item) in items.into_iter().enumerate() {
        let is_active = match (item.kind, item.craft_id) {
            (Some(kind), _) => active_kind == Some(kind),
            (None, Some(cid)) => {
                shell.studio.active_app_id == cid && active_kind == Some(WindowKind::Studio)
            }
            (None, None) => shell.launcher_open,
        };
        let entity = cx.entity();
        let ring = theme.colors.primary;
        let bg = if is_active {
            theme.colors.primary_container
        } else {
            theme.colors.surface_container
        };
        let fg = if is_active {
            theme.colors.on_primary_container
        } else {
            theme.colors.on_surface
        };
        let border = if is_active {
            theme.colors.primary
        } else {
            theme.colors.outline_variant
        };
        let kind_opt = item.kind;
        let craft_id = item.craft_id;
        let label = item.label.clone();

        dock_row = dock_row.child(
            div()
                .id(item.id)
                .min_w(px(tile_size))
                .min_h(px(tile_size))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_0p5()
                .rounded(px(12.0))
                .bg(bg)
                .text_color(fg)
                .border_1()
                .border_color(border)
                .cursor_pointer()
                .focusable()
                .tab_index((700 + idx) as isize)
                .tab_stop(true)
                .role(Role::Button)
                .aria_label(label.clone())
                .tooltip(Tooltip::text(label.clone()))
                .focus_visible(move |s| s.border_2().border_color(ring))
                .on_click(move |_, _w, cx| {
                    entity.update(cx, |s, cx| {
                        if let Some(kind) = kind_opt {
                            s.open_or_focus_window(kind, label.as_ref(), cx);
                        } else if let Some(cid) = craft_id {
                            s.select_craft_app(cid, cx);
                        } else {
                            s.toggle_launcher(cx);
                        }
                    });
                })
                .child(Icon::new(item.icon).size(IconSize::Medium))
                .when(is_active, |el| {
                    el.child(
                        div()
                            .w(px(10.0))
                            .h(px(2.5))
                            .rounded(px(2.0))
                            .bg(theme.colors.primary),
                    )
                }),
        );
    }

    h_flex()
        .id("os-bottom-dock")
        .role(Role::Toolbar)
        .aria_label("Desktop Application Dock")
        .w_full()
        .h(px(dock_h))
        .justify_center()
        .items_center()
        .bg(theme.colors.background)
        .child(dock_row)
        .into_any_element()
}
