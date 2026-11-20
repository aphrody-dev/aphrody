use std::marker::PhantomData;
use std::rc::Rc;
use std::time::Duration;

use gpui::{ClickEvent, Render, Role, Subscription, Task, Toggled};
use ui::{ButtonSize, ButtonStyle, IconButton, ProgressBar, Tooltip, prelude::*};

use crate::cosmic_theme::OsTheme;

/// Asynchronous command returned by [`ReactiveProgram::update`], modeled after `iced::Task`.
pub enum TaskCommand<M: 'static> {
    None,
    Message(M),
    Perform(Task<Option<M>>),
    Batch(Vec<TaskCommand<M>>),
}

impl<M: 'static> TaskCommand<M> {
    #[must_use]
    pub fn none() -> Self {
        Self::None
    }

    #[must_use]
    pub fn done(message: M) -> Self {
        Self::Message(message)
    }

    #[must_use]
    pub fn batch(commands: impl IntoIterator<Item = TaskCommand<M>>) -> Self {
        Self::Batch(commands.into_iter().collect())
    }
}

/// Declarative runtime subscription (`iced::Subscription` equivalent) driven per active state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionSpec<M> {
    None,
    Interval {
        id: &'static str,
        every: Duration,
        tick: M,
    },
}

/// Elm-architecture (`init` / `update` / `subscription` / `view`) hosted cleanly on a GPUI `Entity`.
pub trait ReactiveProgram: Sized + 'static {
    type State: 'static;
    type Message: Clone + Send + 'static;
    type Flags: 'static;

    fn init(flags: Self::Flags) -> (Self::State, TaskCommand<Self::Message>);
    fn update(state: &mut Self::State, message: Self::Message) -> TaskCommand<Self::Message>;

    fn subscription(_state: &Self::State) -> SubscriptionSpec<Self::Message> {
        SubscriptionSpec::None
    }

    fn view(
        state: &Self::State,
        window: &mut Window,
        cx: &mut Context<ReactiveHost<Self>>,
    ) -> AnyElement;
}

/// GPUI entity adapter running any [`ReactiveProgram`].
pub struct ReactiveHost<P: ReactiveProgram> {
    pub state: P::State,
    active_interval_id: Option<&'static str>,
    interval_task: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
    _marker: PhantomData<P>,
}

impl<P: ReactiveProgram> ReactiveHost<P> {
    pub fn new(flags: P::Flags, cx: &mut Context<Self>) -> Self {
        let (state, cmd) = P::init(flags);
        let mut host = Self {
            state,
            active_interval_id: None,
            interval_task: None,
            _subscriptions: Vec::new(),
            _marker: PhantomData,
        };
        host.run_command(cmd, cx);
        host.reconcile_subscription(cx);
        host
    }

    pub fn dispatch(&mut self, message: P::Message, cx: &mut Context<Self>) {
        let cmd = P::update(&mut self.state, message);
        self.run_command(cmd, cx);
        self.reconcile_subscription(cx);
        cx.notify();
    }

    fn run_command(&mut self, command: TaskCommand<P::Message>, cx: &mut Context<Self>) {
        match command {
            TaskCommand::None => {}
            TaskCommand::Message(msg) => {
                self.dispatch(msg, cx);
            }
            TaskCommand::Perform(task) => {
                cx.spawn(async move |this, cx| {
                    if let Some(msg) = task.await
                        && let Err(err) = this.update(cx, |host, cx| host.dispatch(msg, cx))
                    {
                        log::debug!("reactive task target dropped before update: {err:#}");
                    }
                })
                .detach();
            }
            TaskCommand::Batch(commands) => {
                for cmd in commands {
                    self.run_command(cmd, cx);
                }
            }
        }
    }

    fn reconcile_subscription(&mut self, cx: &mut Context<Self>) {
        match P::subscription(&self.state) {
            SubscriptionSpec::None => {
                self.active_interval_id = None;
                self.interval_task = None;
            }
            SubscriptionSpec::Interval { id, every, tick } => {
                if self.active_interval_id == Some(id) {
                    return;
                }
                self.active_interval_id = Some(id);
                let task = cx.spawn(async move |this, cx| {
                    loop {
                        cx.background_executor().timer(every).await;
                        let alive = this.update(cx, |host, cx| host.dispatch(tick.clone(), cx));
                        if alive.is_err() {
                            break;
                        }
                    }
                });
                self.interval_task = Some(task);
            }
        }
    }
}

impl<P: ReactiveProgram> Render for ReactiveHost<P> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        P::view(&self.state, window, cx)
    }
}

/// Accessible focus-visible action pill meeting WCAG contrast, 2px focus ring, and 36-44px target ergonomics.
#[derive(IntoElement)]
pub struct FocusableActionChip {
    id: ElementId,
    icon: IconName,
    label: SharedString,
    badge: Option<SharedString>,
    aria_label: SharedString,
    selected: bool,
    toggled: Option<bool>,
    tab_index: isize,
    theme: OsTheme,
    on_click: Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>,
}

impl FocusableActionChip {
    pub fn new(
        id: impl Into<ElementId>,
        icon: IconName,
        label: impl Into<SharedString>,
        aria_label: impl Into<SharedString>,
        theme: &OsTheme,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            icon,
            label: label.into(),
            badge: None,
            aria_label: aria_label.into(),
            selected: false,
            toggled: None,
            tab_index: 0,
            theme: theme.clone(),
            on_click: Box::new(on_click),
        }
    }

    pub fn badge(mut self, badge: impl Into<Option<SharedString>>) -> Self {
        self.badge = badge.into();
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn toggled(mut self, toggled: impl Into<Option<bool>>) -> Self {
        self.toggled = toggled.into();
        self
    }

    pub fn tab_index(mut self, tab_index: isize) -> Self {
        self.tab_index = tab_index;
        self
    }
}

impl RenderOnce for FocusableActionChip {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let bg = if self.selected {
            self.theme.colors.primary_container
        } else {
            self.theme.colors.surface_container_high
        };
        let fg = if self.selected {
            self.theme.colors.on_primary_container
        } else {
            self.theme.colors.on_surface
        };
        let border = if self.selected {
            self.theme.colors.primary.opacity(0.65)
        } else {
            self.theme.colors.outline_variant.opacity(0.32)
        };
        let hover_bg = if self.selected {
            self.theme.colors.primary_container
        } else {
            self.theme.colors.surface_container_highest
        };
        let focus_ring = self.theme.colors.primary;
        let aria_label = self.aria_label;

        h_flex()
            .id(self.id)
            .min_w_0()
            .min_h(rems_from_px(self.theme.layout.min_control_height))
            .px_3()
            .gap_2()
            .items_center()
            .rounded_full()
            .bg(bg)
            .border_1()
            .border_color(border)
            .text_color(fg)
            .cursor_pointer()
            .focusable()
            .tab_index(self.tab_index)
            .tab_stop(true)
            .role(Role::Button)
            .aria_label(aria_label.clone())
            .when_some(self.toggled, |el, toggled| {
                el.aria_toggled(if toggled {
                    Toggled::True
                } else {
                    Toggled::False
                })
            })
            .tooltip(Tooltip::text(aria_label))
            .hover(move |style| style.bg(hover_bg))
            .focus_visible(move |style| style.border_2().border_color(focus_ring))
            .on_click(self.on_click)
            .child(
                Icon::new(self.icon)
                    .size(IconSize::Small)
                    .color(Color::Custom(fg)),
            )
            .when(!self.label.is_empty(), |el| {
                el.child(
                    div().min_w_0().overflow_hidden().child(
                        Label::new(self.label)
                            .size(LabelSize::Small)
                            .color(Color::Custom(fg))
                            .truncate(),
                    ),
                )
            })
            .when_some(self.badge, |el, badge_text| {
                el.child(
                    div()
                        .flex_shrink_0()
                        .px_2()
                        .py_0p5()
                        .rounded_full()
                        .bg(self.theme.colors.surface_container_lowest.opacity(0.85))
                        .child(
                            Label::new(badge_text)
                                .size(LabelSize::XSmall)
                                .color(Color::Custom(self.theme.colors.on_surface_variant))
                                .single_line(),
                        ),
                )
            })
    }
}

/// Accessible focus-visible action pill meeting WCAG contrast, 2px focus ring, and 36-44px target ergonomics.
pub fn focusable_action_chip<F>(
    id: impl Into<ElementId>,
    icon: IconName,
    label: impl Into<SharedString>,
    badge: Option<SharedString>,
    aria_label: impl Into<SharedString>,
    selected: bool,
    tab_index: isize,
    theme: &OsTheme,
    on_click: F,
) -> AnyElement
where
    F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
{
    FocusableActionChip::new(id, icon, label, aria_label, theme, on_click)
        .badge(badge)
        .selected(selected)
        .tab_index(tab_index)
        .into_any_element()
}

/// Accessible slider bar with explicit keyboard/button steppers and visible value text (`iced`/`libcosmic` pattern).
#[derive(IntoElement)]
pub struct SliderControl {
    id: SharedString,
    label: SharedString,
    value: f32,
    min: f32,
    max: f32,
    unit: SharedString,
    theme: OsTheme,
    on_change: Rc<dyn Fn(f32, &mut Window, &mut App) + 'static>,
}

impl SliderControl {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        value: f32,
        min: f32,
        max: f32,
        unit: impl Into<SharedString>,
        theme: &OsTheme,
        on_change: impl Fn(f32, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value,
            min,
            max,
            unit: unit.into(),
            theme: theme.clone(),
            on_change: Rc::new(on_change),
        }
    }
}

impl RenderOnce for SliderControl {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let dec_id = SharedString::from(format!("{}-dec", self.id));
        let inc_id = SharedString::from(format!("{}-inc", self.id));
        let span = (self.max - self.min).max(0.0001);
        let ratio = ((self.value - self.min) / span).clamp(0.0, 1.0);
        let step = (span * 0.05).max(0.01);

        let on_dec = Rc::clone(&self.on_change);
        let on_inc = self.on_change;
        let value = self.value;
        let min = self.min;
        let max = self.max;

        v_flex()
            .id(self.id)
            .w_full()
            .min_w_0()
            .gap_1()
            .child(
                h_flex()
                    .w_full()
                    .min_w_0()
                    .justify_between()
                    .gap_2()
                    .child(
                        div().flex_1().min_w_0().overflow_hidden().child(
                            Label::new(self.label.clone())
                                .size(LabelSize::Small)
                                .truncate(),
                        ),
                    )
                    .child(
                        div().flex_shrink_0().child(
                            Label::new(format!("{value:.2}{}", self.unit))
                                .size(LabelSize::XSmall)
                                .color(Color::Muted)
                                .single_line(),
                        ),
                    ),
            )
            .child(
                h_flex()
                    .w_full()
                    .min_w_0()
                    .gap_2()
                    .items_center()
                    .child(
                        IconButton::new(dec_id, IconName::Dash)
                            .style(ButtonStyle::Outlined)
                            .size(ButtonSize::Medium)
                            .width(rems_from_px(28.0_f32))
                            .icon_size(IconSize::XSmall)
                            .icon_color(Color::Custom(self.theme.colors.on_surface))
                            .tab_index(0_isize)
                            .aria_label(SharedString::from(format!("Decrease {}", self.label)))
                            .tooltip(Tooltip::text(format!("Decrease {}", self.label)))
                            .on_click(move |_, window, cx| {
                                on_dec((value - step).clamp(min, max), window, cx);
                            }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .h(rems_from_px(8.0_f32))
                            .rounded(rems_from_px(4.0_f32))
                            .overflow_hidden()
                            .bg(self.theme.colors.surface_container_highest)
                            .border_1()
                            .border_color(self.theme.colors.outline_variant)
                            .child(
                                div()
                                    .h_full()
                                    .w(relative(ratio))
                                    .rounded(rems_from_px(4.0_f32))
                                    .bg(self.theme.colors.primary),
                            ),
                    )
                    .child(
                        IconButton::new(inc_id, IconName::Plus)
                            .style(ButtonStyle::Outlined)
                            .size(ButtonSize::Medium)
                            .width(rems_from_px(28.0_f32))
                            .icon_size(IconSize::XSmall)
                            .icon_color(Color::Custom(self.theme.colors.on_surface))
                            .tab_index(0_isize)
                            .aria_label(SharedString::from(format!("Increase {}", self.label)))
                            .tooltip(Tooltip::text(format!("Increase {}", self.label)))
                            .on_click(move |_, window, cx| {
                                on_inc((value + step).clamp(min, max), window, cx);
                            }),
                    ),
            )
    }
}

/// Accessible slider bar with explicit keyboard/button steppers and visible value text (`iced`/`libcosmic` pattern).
pub fn slider_control<F>(
    id: &str,
    label: impl Into<SharedString>,
    value: f32,
    min: f32,
    max: f32,
    unit: &str,
    theme: &OsTheme,
    on_change: F,
) -> AnyElement
where
    F: Fn(f32, &mut Window, &mut App) + Clone + 'static,
{
    SliderControl::new(
        id.to_owned(),
        label,
        value,
        min,
        max,
        unit.to_owned(),
        theme,
        on_change,
    )
    .into_any_element()
}

/// Accessible on/off toggle control with explicit state text (`On`/`Off` — never color-only).
#[derive(IntoElement)]
pub struct ToggleControl {
    id: ElementId,
    label: SharedString,
    checked: bool,
    theme: OsTheme,
    on_toggle: Box<dyn Fn(bool, &mut Window, &mut App) + 'static>,
}

impl ToggleControl {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        checked: bool,
        theme: &OsTheme,
        on_toggle: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            checked,
            theme: theme.clone(),
            on_toggle: Box::new(on_toggle),
        }
    }
}

impl RenderOnce for ToggleControl {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let checked = self.checked;
        let on_toggle = self.on_toggle;
        let aria_label = SharedString::from(format!(
            "{}, {}",
            self.label,
            if checked { "enabled" } else { "disabled" }
        ));
        FocusableActionChip::new(
            self.id,
            if checked {
                IconName::Check
            } else {
                IconName::Dash
            },
            self.label,
            aria_label,
            &self.theme,
            move |_, window, cx| on_toggle(!checked, window, cx),
        )
        .badge(Some(SharedString::from(if checked { "ON" } else { "OFF" })))
        .selected(checked)
        .toggled(Some(checked))
        .tab_index(0)
    }
}

/// Accessible on/off toggle control with explicit state text (`On`/`Off` — never color-only).
pub fn toggle_control<F>(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    checked: bool,
    theme: &OsTheme,
    on_toggle: F,
) -> AnyElement
where
    F: Fn(bool, &mut Window, &mut App) + 'static,
{
    ToggleControl::new(id, label, checked, theme, on_toggle).into_any_element()
}

/// Accessible labeled progress meter built on [`ui::ProgressBar`].
#[derive(IntoElement)]
pub struct ProgressMeter {
    id: ElementId,
    label: SharedString,
    fraction: f32,
    theme: OsTheme,
}

impl ProgressMeter {
    #[must_use]
    pub fn new(label: impl Into<SharedString>, fraction: f32, theme: &OsTheme) -> Self {
        let label = label.into();
        let id = ElementId::Name(SharedString::from(format!("progress-{label}")));
        Self {
            id,
            label,
            fraction,
            theme: theme.clone(),
        }
    }
}

impl RenderOnce for ProgressMeter {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let clamped = self.fraction.clamp(0.0, 1.0);
        let pct = (clamped * 100.0).round() as u32;
        v_flex()
            .w_full()
            .min_w_0()
            .gap_1()
            .child(
                h_flex()
                    .w_full()
                    .min_w_0()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .child(Label::new(self.label).size(LabelSize::XSmall).truncate()),
                    )
                    .child(
                        div().flex_shrink_0().child(
                            Label::new(format!("{pct}%"))
                                .size(LabelSize::XSmall)
                                .single_line(),
                        ),
                    ),
            )
            .child(
                ProgressBar::new(self.id, clamped, 1.0, cx)
                    .bg_color(self.theme.colors.surface_container_highest)
                    .fg_color(self.theme.colors.primary),
            )
    }
}

/// Accessible labeled progress meter.
#[must_use]
pub fn progress_meter(
    label: impl Into<SharedString>,
    fraction: f32,
    theme: &OsTheme,
) -> AnyElement {
    ProgressMeter::new(label, fraction, theme).into_any_element()
}
