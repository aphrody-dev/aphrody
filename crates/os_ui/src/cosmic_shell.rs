use std::fs;
use std::path::PathBuf;

use gpui::{AnyElement, App, ElementId, SharedString, Window, div, prelude::*, px};
use serde::{Deserialize, Serialize};
use ui::{IconName, Label, LabelCommon as _, LabelSize, h_flex, v_flex};

use crate::cosmic_theme::OsTheme;
use crate::reactive::focusable_action_chip;

/// Horizontal slot inside a COSMIC-style panel/topbar where applets mount dynamically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppletSlot {
    Left,
    Center,
    Right,
}

/// Pluggable COSMIC/GPUI panel applet contract.
///
/// New desktop indicators, workspace pills, media controls, or telemetry badges register
/// uniformly through this trait instead of hardcoding bar children.
pub trait PanelApplet: 'static {
    fn id(&self) -> &'static str;
    fn title(&self) -> SharedString;
    fn slot(&self) -> AppletSlot;
    fn order(&self) -> i32 {
        0
    }
    fn render_pill(&self, theme: &OsTheme, window: &mut Window, cx: &mut App) -> AnyElement;
    fn render_popover(
        &self,
        _theme: &OsTheme,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Option<AnyElement> {
        None
    }
}

/// Versioned atomic configuration store modeled after Pop!_OS `cosmic-config`.
#[derive(Debug, Clone)]
pub struct OsConfigStore<T> {
    pub app_id: String,
    pub key: String,
    pub path: PathBuf,
    pub value: T,
}

impl<T> OsConfigStore<T>
where
    T: Clone + Serialize + for<'de> Deserialize<'de> + Default,
{
    #[must_use]
    pub fn load_or_default(app_id: &str, key: &str) -> Self {
        let base = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(brand::APP_SLUG)
            .join(app_id);
        let path = base.join(format!("{key}.json"));
        let value = fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str::<T>(&raw).ok())
            .unwrap_or_default();
        Self {
            app_id: app_id.to_owned(),
            key: key.to_owned(),
            path,
            value,
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp_path = self.path.with_extension("json.tmp");
        let serialized = serde_json::to_string_pretty(&self.value)?;
        fs::write(&tmp_path, serialized)?;
        fs::rename(&tmp_path, &self.path)?;
        Ok(())
    }
}

/// Item descriptor for `libcosmic`-style navigation rails.
#[derive(Debug, Clone)]
pub struct NavItem {
    pub id: SharedString,
    pub label: SharedString,
    pub icon: IconName,
    pub badge: Option<SharedString>,
}

/// Renders an accessible `libcosmic`-style navigation rail supporting compact (icon-only <= 900px) and expanded modes.
pub fn render_nav_rail<F>(
    id: impl Into<ElementId>,
    items: &[NavItem],
    active_id: &str,
    compact: bool,
    theme: &OsTheme,
    on_select: F,
) -> AnyElement
where
    F: Fn(SharedString, &mut Window, &mut App) + Clone + 'static,
{
    let mut rail = v_flex()
        .id(id)
        .h_full()
        .when(compact, |el| el.w(px(64.0)))
        .when(!compact, |el| el.w(px(212.0)))
        .p_2()
        .gap_2()
        .overflow_y_scroll()
        .bg(theme.colors.surface_container_low)
        .border_r_1()
        .border_color(theme.colors.outline_variant);

    for (idx, item) in items.iter().enumerate() {
        let selected = item.id.as_ref() == active_id;
        let item_id = item.id.clone();
        let handler = on_select.clone();
        let (label, badge) = if compact {
            (SharedString::default(), None)
        } else {
            (item.label.clone(), item.badge.clone())
        };

        let chip = focusable_action_chip(
            ElementId::Name(format!("nav-{}", item.id).into()),
            item.icon,
            label,
            badge,
            item.label.clone(),
            selected,
            idx as isize + 1,
            theme,
            move |_, window, cx| handler(item_id.clone(), window, cx),
        );
        rail = rail.child(chip);
    }

    rail.into_any_element()
}

/// Renders a right-hand contextual drawer (`libcosmic::widget::context_drawer`) with explicit close escape control.
pub fn render_context_drawer<F>(
    title: impl Into<SharedString>,
    body: AnyElement,
    theme: &OsTheme,
    on_close: F,
) -> AnyElement
where
    F: Fn(&mut Window, &mut App) + 'static,
{
    let title = title.into();
    v_flex()
        .id("os-context-drawer")
        .w(px(320.0))
        .h_full()
        .bg(theme.colors.surface_container)
        .border_l_1()
        .border_color(theme.colors.outline_variant)
        .child(
            h_flex()
                .w_full()
                .min_h(px(44.0))
                .px_3()
                .justify_between()
                .items_center()
                .border_b_1()
                .border_color(theme.colors.outline_variant)
                .child(Label::new(title.clone()).size(LabelSize::Default))
                .child(focusable_action_chip(
                    "close-drawer",
                    IconName::Close,
                    "",
                    None,
                    SharedString::from(format!("Close {title} drawer")),
                    false,
                    1,
                    theme,
                    move |_, window, cx| on_close(window, cx),
                )),
        )
        .child(
            div()
                .id("os-context-drawer-body")
                .flex_1()
                .size_full()
                .p_3()
                .overflow_y_scroll()
                .child(body),
        )
        .into_any_element()
}
