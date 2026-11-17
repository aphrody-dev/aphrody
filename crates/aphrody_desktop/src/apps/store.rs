//! `cosmic-store` powered live by `aphrody_store::AwesomeGpuiCatalog`.

use aphrody_store::{AwesomeGpuiCatalog, StoreCategory};
use aphrody_ui::{KeyValueRow, OsTheme, StatusTone, status_badge};
use gpui::{AnyElement, App, div, prelude::*, px};
use ui::{IconName, Label, LabelCommon as _, LabelSize, h_flex, v_flex};

use crate::apps::section_heading;

pub fn render_cosmic_store_window(
    theme: &OsTheme,
    catalog: &AwesomeGpuiCatalog,
    active_category: Option<StoreCategory>,
    on_select_category: Box<dyn Fn(Option<StoreCategory>, &mut App) + 'static>,
) -> AnyElement {
    let cb = std::rc::Rc::new(on_select_category);
    let mut filter_bar = h_flex().gap_1p5().flex_wrap();
    let cb_all = std::rc::Rc::clone(&cb);
    filter_bar = filter_bar.child(
        div()
            .id("cosmic-store-cat-all")
            .cursor_pointer()
            .px_2()
            .py_1()
            .rounded(px(6.0))
            .bg(if active_category.is_none() {
                theme.colors.primary_container
            } else {
                theme.colors.surface_container_high
            })
            .on_click(move |_ev, _w, cx| cb_all(None, cx))
            .child(Label::new("All").size(LabelSize::XSmall)),
    );
    for (idx, cat) in StoreCategory::ALL.into_iter().enumerate() {
        let on_cat = std::rc::Rc::clone(&cb);
        let selected = active_category == Some(cat);
        filter_bar = filter_bar.child(
            div()
                .id(("cosmic-store-cat", idx))
                .cursor_pointer()
                .px_2()
                .py_1()
                .rounded(px(6.0))
                .bg(if selected {
                    theme.colors.primary_container
                } else {
                    theme.colors.surface_container_high
                })
                .on_click(move |_ev, _w, cx| on_cat(Some(cat), cx))
                .child(Label::new(cat.label()).size(LabelSize::XSmall)),
        );
    }

    let items = catalog.search("", active_category, None, None);
    let mut list = v_flex().gap_1p5();
    for entry in items.iter().take(8) {
        list = list.child(
            v_flex()
                .p_2()
                .rounded(px(8.0))
                .bg(theme.colors.surface_container_high)
                .border_1()
                .border_color(theme.colors.outline_variant)
                .child(
                    h_flex()
                        .justify_between()
                        .child(Label::new(entry.name.clone()).size(LabelSize::Small))
                        .child(status_badge(
                            IconName::Download,
                            entry.category.label(),
                            StatusTone::Neutral,
                            theme,
                        )),
                )
                .child(
                    Label::new(entry.tagline.clone())
                        .size(LabelSize::XSmall)
                        .color(ui::Color::Muted),
                ),
        );
    }

    v_flex()
        .size_full()
        .gap_2()
        .child(section_heading(
            theme,
            "COSMIC Store · Awesome GPUI + 22 Targets",
        ))
        .child(KeyValueRow::new(
            "Catalog Entries",
            catalog.entries.len().to_string(),
        ))
        .child(filter_bar)
        .child(list)
        .into_any_element()
}
