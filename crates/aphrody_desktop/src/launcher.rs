//! Pop!_OS `pop-launcher` (`=calc` / `:cmd` / `sh `) & `cosmic-app-library` Folder Grid.

use aphrody_craft::CraftCatalog;
use aphrody_session::{AppEntry, LauncherActionKind, LauncherResultItem, query_pop_launcher};
use aphrody_ui::{OsTheme, StatusTone, status_badge};
use gpui::{AnyElement, App, FontWeight, div, prelude::*, px};
use ui::{Icon, IconName, IconSize, Label, LabelCommon as _, LabelSize, h_flex, v_flex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LauncherFolder {
    #[default]
    All,
    AphrodyUserland,
    StorytoldCraft,
    CosmicEpoch,
    Developer,
}

impl LauncherFolder {
    pub const ALL: [Self; 5] = [
        Self::All,
        Self::AphrodyUserland,
        Self::StorytoldCraft,
        Self::CosmicEpoch,
        Self::Developer,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "Library Home",
            Self::AphrodyUserland => "Aphrody OS",
            Self::StorytoldCraft => "ArtCraft Suite",
            Self::CosmicEpoch => "COSMIC Epoch",
            Self::Developer => "Rust & WGPU",
        }
    }
}

pub fn render_pop_launcher_and_app_library(
    theme: &OsTheme,
    query: &str,
    folder: LauncherFolder,
    catalog: &CraftCatalog,
    discovered: &[AppEntry],
    on_select_folder: Box<dyn Fn(LauncherFolder, &mut App) + 'static>,
    on_launch_result: Box<dyn Fn(LauncherResultItem, &mut App) + 'static>,
    on_close: Box<dyn Fn(&mut App) + 'static>,
) -> AnyElement {
    let craft_pairs: Vec<(String, String, String)> = catalog
        .apps
        .iter()
        .map(|a| (a.id.clone(), a.name.clone(), a.tagline.clone()))
        .collect();
    let results = query_pop_launcher(query, &craft_pairs, discovered);

    let on_folder = std::rc::Rc::new(on_select_folder);
    let on_launch = std::rc::Rc::new(on_launch_result);
    let on_dismiss = std::rc::Rc::new(on_close);

    let mut folder_bar = h_flex().gap_1p5().flex_wrap();
    for (idx, f) in LauncherFolder::ALL.into_iter().enumerate() {
        let cb_f = std::rc::Rc::clone(&on_folder);
        let active = f == folder;
        folder_bar = folder_bar.child(
            div()
                .id(("cosmic-applib-folder", idx))
                .cursor_pointer()
                .px_2p5()
                .py_1()
                .rounded(px(8.0))
                .bg(if active {
                    theme.colors.primary_container
                } else {
                    theme.colors.surface_container_high
                })
                .on_click(move |_ev, _w, cx| cb_f(f, cx))
                .child(Label::new(f.label()).size(LabelSize::XSmall)),
        );
    }

    let filtered: Vec<LauncherResultItem> = results
        .into_iter()
        .filter(|item| match folder {
            LauncherFolder::All => true,
            LauncherFolder::AphrodyUserland => match &item.action {
                LauncherActionKind::DesktopApp(t) | LauncherActionKind::CompositorCommand(t) => {
                    t.contains("installer") || t.contains("aphrody")
                }
                _ => false,
            },
            LauncherFolder::StorytoldCraft => matches!(item.action, LauncherActionKind::CraftApp(_)),
            LauncherFolder::CosmicEpoch => match &item.action {
                LauncherActionKind::DesktopApp(t) | LauncherActionKind::CompositorCommand(t) => {
                    t.contains("cosmic") || t.starts_with(':')
                }
                _ => false,
            },
            LauncherFolder::Developer => matches!(
                item.action,
                LauncherActionKind::CompositorCommand(_)
                    | LauncherActionKind::ShellCommand(_)
                    | LauncherActionKind::DesktopApp(_)
            ),
        })
        .take(12)
        .collect();

    let mut grid = h_flex().gap_2().flex_wrap();
    for (idx, item) in filtered.into_iter().enumerate() {
        let item_clone = item.clone();
        let cb_l = std::rc::Rc::clone(&on_launch);
        let cb_c = std::rc::Rc::clone(&on_dismiss);
        grid = grid.child(
            div()
                .id(("pop-launcher-item", idx))
                .cursor_pointer()
                .w(px(236.0))
                .p_2p5()
                .rounded(px(theme.layout.window_radius))
                .border_1()
                .border_color(theme.colors.outline_variant)
                .bg(theme.colors.surface_container_high)
                .on_click(move |_ev, _w, cx| {
                    cb_l(item_clone.clone(), cx);
                    cb_c(cx);
                })
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            h_flex()
                                .justify_between()
                                .child(
                                    h_flex()
                                        .gap_1()
                                        .items_center()
                                        .child(Icon::new(IconName::Code).size(IconSize::XSmall))
                                        .child(
                                            Label::new(item.title)
                                                .size(LabelSize::Small)
                                                .weight(FontWeight::SEMIBOLD),
                                        ),
                                )
                                .child(status_badge(
                                    IconName::Settings,
                                    item.badge,
                                    StatusTone::Accent,
                                    theme,
                                )),
                        )
                        .child(
                            Label::new(item.subtitle)
                                .size(LabelSize::XSmall)
                                .color(ui::Color::Muted),
                        ),
                ),
        );
    }

    let close_btn = std::rc::Rc::clone(&on_dismiss);
    v_flex()
        .id("pop-launcher-overlay")
        .p_4()
        .gap_3()
        .w(px(780.0))
        .rounded(px(theme.layout.window_radius))
        .border_1()
        .border_color(theme.colors.outline)
        .bg(theme.colors.surface_container)
        .child(
            h_flex()
                .justify_between()
                .items_center()
                .child(
                    Label::new("Pop! Launcher & COSMIC App Library (`=calc`, `:cmd`, `sh`)")
                        .size(LabelSize::Small)
                        .weight(FontWeight::BOLD),
                )
                .child(
                    div()
                        .id("pop-launcher-close")
                        .cursor_pointer()
                        .px_2()
                        .py_1()
                        .rounded(px(6.0))
                        .bg(theme.colors.surface_container_high)
                        .on_click(move |_ev, _w, cx| close_btn(cx))
                        .child(Label::new("Esc").size(LabelSize::XSmall)),
                ),
        )
        .child(folder_bar)
        .child(grid)
        .into_any_element()
}
