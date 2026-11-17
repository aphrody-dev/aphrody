use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use gpui::{FontWeight, Role};
use serde::{Deserialize, Serialize};
use ui::{ButtonSize, ButtonStyle, IconButton, Tooltip, prelude::*};

use crate::cosmic_theme::OsTheme;
use crate::reactive::FocusableActionChip;

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

/// Accessible `libcosmic`-style navigation rail supporting compact and expanded widths.
#[derive(IntoElement)]
pub struct NavRail {
    id: ElementId,
    items: Vec<NavItem>,
    active_id: SharedString,
    compact: bool,
    theme: OsTheme,
    on_select: Rc<dyn Fn(SharedString, &mut Window, &mut App) + 'static>,
}

impl NavRail {
    pub fn new(
        id: impl Into<ElementId>,
        items: Vec<NavItem>,
        active_id: impl Into<SharedString>,
        compact: bool,
        theme: &OsTheme,
        on_select: impl Fn(SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            items,
            active_id: active_id.into(),
            compact,
            theme: theme.clone(),
            on_select: Rc::new(on_select),
        }
    }
}

impl RenderOnce for NavRail {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut rail = v_flex()
            .id(self.id)
            .role(Role::Navigation)
            .aria_label("Navigation Rail")
            .flex_shrink_0()
            .h_full()
            .when(self.compact, |el| el.w(rems_from_px(64.0_f32)))
            .when(!self.compact, |el| el.w(rems_from_px(212.0_f32)))
            .p_2()
            .pb_5()
            .gap_2()
            .overflow_y_scroll()
            .bg(self.theme.colors.surface_container_low)
            .border_r_1()
            .border_color(self.theme.colors.outline_variant);

        for (idx, item) in self.items.into_iter().enumerate() {
            let selected = item.id == self.active_id;
            let item_id = item.id.clone();
            let handler = Rc::clone(&self.on_select);
            let (label, badge) = if self.compact {
                (SharedString::default(), None)
            } else {
                (item.label.clone(), item.badge)
            };

            let chip = FocusableActionChip::new(
                ElementId::Name(format!("nav-{}", item.id).into()),
                item.icon,
                label,
                item.label,
                &self.theme,
                move |_, window, cx| handler(item_id.clone(), window, cx),
            )
            .badge(badge)
            .selected(selected)
            .tab_index(idx as isize + 1);

            rail = rail.child(chip);
        }

        rail
    }
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
    NavRail::new(
        id,
        items.to_vec(),
        SharedString::from(active_id.to_owned()),
        compact,
        theme,
        on_select,
    )
    .into_any_element()
}

/// Right-hand contextual drawer (`libcosmic::widget::context_drawer`) with explicit close escape control.
#[derive(IntoElement)]
pub struct ContextDrawer {
    title: SharedString,
    body: AnyElement,
    theme: OsTheme,
    on_close: Box<dyn Fn(&mut Window, &mut App) + 'static>,
}

impl ContextDrawer {
    pub fn new(
        title: impl Into<SharedString>,
        body: AnyElement,
        theme: &OsTheme,
        on_close: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            title: title.into(),
            body,
            theme: theme.clone(),
            on_close: Box::new(on_close),
        }
    }
}

impl RenderOnce for ContextDrawer {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let close_label = SharedString::from(format!("Close {} drawer", self.title));
        let on_close = self.on_close;

        v_flex()
            .id("os-context-drawer")
            .role(Role::Region)
            .aria_label(self.title.clone())
            .flex_shrink_0()
            .w(rems_from_px(320.0_f32))
            .max_w(relative(0.45))
            .h_full()
            .bg(self.theme.colors.surface_container)
            .border_l_1()
            .border_color(self.theme.colors.outline_variant)
            .child(
                h_flex()
                    .w_full()
                    .min_w_0()
                    .min_h(rems_from_px(44.0_f32))
                    .px_3()
                    .gap_2()
                    .justify_between()
                    .items_center()
                    .border_b_1()
                    .border_color(self.theme.colors.outline_variant)
                    .child(
                        div().flex_1().min_w_0().overflow_hidden().child(
                            Label::new(self.title)
                                .size(LabelSize::Default)
                                .weight(FontWeight::MEDIUM)
                                .truncate(),
                        ),
                    )
                    .child(
                        IconButton::new("close-drawer", IconName::Close)
                            .style(ButtonStyle::Outlined)
                            .size(ButtonSize::Large)
                            .width(rems_from_px(32.0_f32))
                            .icon_size(IconSize::Small)
                            .icon_color(Color::Custom(self.theme.colors.on_surface))
                            .tab_index(1_isize)
                            .aria_label(close_label.clone())
                            .tooltip(Tooltip::text(close_label))
                            .on_click(move |_, window, cx| on_close(window, cx)),
                    ),
            )
            .child(
                div()
                    .id("os-context-drawer-body")
                    .flex_1()
                    .min_w_0()
                    .size_full()
                    .p_3()
                    .pb_6()
                    .overflow_y_scroll()
                    .child(self.body),
            )
    }
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
    ContextDrawer::new(title, body, theme, on_close).into_any_element()
}
