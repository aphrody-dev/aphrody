use std::marker::PhantomData;
use std::time::Duration;

use gpui::{
    AnyElement, App, ClickEvent, Context, ElementId, Render, Role, SharedString, Subscription,
    Task, Window, div, prelude::*, px,
};
use ui::{Icon, IconName, IconSize, Label, LabelCommon as _, LabelSize, h_flex, v_flex};

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
#[allow(clippy::too_many_arguments)]
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
    let label = label.into();
    let aria_label = aria_label.into();
    let bg = if selected {
        theme.colors.primary_container
    } else {
        theme.colors.surface_container_high
    };
    let fg = if selected {
        theme.colors.on_primary_container
    } else {
        theme.colors.on_surface
    };
    let border = if selected {
        theme.colors.primary
    } else {
        theme.colors.outline_variant
    };
    let hover_bg = if selected {
        theme.colors.primary_container
    } else {
        theme.colors.surface_container_highest
    };
    let focus_ring = theme.colors.primary;

    h_flex()
        .id(id)
        .min_h(px(theme.layout.min_control_height))
        .px_2p5()
        .gap_2()
        .items_center()
        .rounded(px(8.0))
        .bg(bg)
        .border_1()
        .border_color(border)
        .text_color(fg)
        .cursor_pointer()
        .focusable()
        .tab_index(tab_index)
        .tab_stop(true)
        .role(Role::Button)
        .aria_label(aria_label)
        .hover(move |style| style.bg(hover_bg))
        .focus_visible(move |style| style.border_2().border_color(focus_ring))
        .on_click(on_click)
        .child(Icon::new(icon).size(IconSize::Small))
        .when(!label.is_empty(), |el| {
            el.child(Label::new(label).size(LabelSize::Small))
        })
        .when_some(badge, |el, badge_text| {
            el.child(
                div()
                    .px_1p5()
                    .py_0p5()
                    .rounded(px(6.0))
                    .bg(theme.colors.surface_container_lowest)
                    .text_color(theme.colors.on_surface_variant)
                    .child(Label::new(badge_text).size(LabelSize::XSmall)),
            )
        })
        .into_any_element()
}

/// Accessible slider bar with explicit keyboard/button steppers and visible value text (`iced`/`libcosmic` pattern).
#[allow(clippy::too_many_arguments)]
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
    let id_root = SharedString::from(id.to_owned());
    let dec_id = SharedString::from(format!("{id}-dec"));
    let inc_id = SharedString::from(format!("{id}-inc"));
    let label = label.into();
    let span = (max - min).max(0.0001);
    let ratio = ((value - min) / span).clamp(0.0, 1.0);
    let step = (span * 0.05).max(0.01);

    let on_dec = on_change.clone();
    let on_inc = on_change;

    v_flex()
        .id(id_root)
        .w_full()
        .gap_1()
        .child(
            h_flex()
                .w_full()
                .justify_between()
                .child(Label::new(label.clone()).size(LabelSize::Small))
                .child(
                    Label::new(format!("{value:.2}{unit}"))
                        .size(LabelSize::XSmall)
                        .color(ui::Color::Muted),
                ),
        )
        .child(
            h_flex()
                .w_full()
                .gap_2()
                .items_center()
                .child(
                    div()
                        .id(dec_id)
                        .w(px(28.0))
                        .h(px(28.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(6.0))
                        .bg(theme.colors.surface_container_high)
                        .border_1()
                        .border_color(theme.colors.outline_variant)
                        .cursor_pointer()
                        .focusable()
                        .tab_stop(true)
                        .role(Role::Button)
                        .aria_label(SharedString::from(format!("Decrease {label}")))
                        .focus_visible({
                            let ring = theme.colors.primary;
                            move |s| s.border_2().border_color(ring)
                        })
                        .on_click(move |_, window, cx| {
                            on_dec((value - step).clamp(min, max), window, cx);
                        })
                        .child(Icon::new(IconName::Dash).size(IconSize::XSmall)),
                )
                .child(
                    div()
                        .flex_1()
                        .h(px(8.0))
                        .rounded(px(4.0))
                        .bg(theme.colors.surface_container_highest)
                        .border_1()
                        .border_color(theme.colors.outline_variant)
                        .child(
                            div()
                                .h_full()
                                .w(gpui::relative(ratio))
                                .rounded(px(4.0))
                                .bg(theme.colors.primary),
                        ),
                )
                .child(
                    div()
                        .id(inc_id)
                        .w(px(28.0))
                        .h(px(28.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(6.0))
                        .bg(theme.colors.surface_container_high)
                        .border_1()
                        .border_color(theme.colors.outline_variant)
                        .cursor_pointer()
                        .focusable()
                        .tab_stop(true)
                        .role(Role::Button)
                        .aria_label(SharedString::from(format!("Increase {label}")))
                        .focus_visible({
                            let ring = theme.colors.primary;
                            move |s| s.border_2().border_color(ring)
                        })
                        .on_click(move |_, window, cx| {
                            on_inc((value + step).clamp(min, max), window, cx);
                        })
                        .child(Icon::new(IconName::Plus).size(IconSize::XSmall)),
                ),
        )
        .into_any_element()
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
    let label = label.into();
    focusable_action_chip(
        id,
        if checked {
            IconName::Check
        } else {
            IconName::Dash
        },
        label.clone(),
        Some(SharedString::from(if checked { "ON" } else { "OFF" })),
        SharedString::from(format!(
            "{label}, {}",
            if checked { "enabled" } else { "disabled" }
        )),
        checked,
        0,
        theme,
        move |_, window, cx| on_toggle(!checked, window, cx),
    )
}

/// Accessible labeled progress meter.
#[must_use]
pub fn progress_meter(
    label: impl Into<SharedString>,
    fraction: f32,
    theme: &OsTheme,
) -> AnyElement {
    let clamped = fraction.clamp(0.0, 1.0);
    let pct = (clamped * 100.0).round() as u32;
    v_flex()
        .w_full()
        .gap_1()
        .child(
            h_flex()
                .w_full()
                .justify_between()
                .child(Label::new(label.into()).size(LabelSize::XSmall))
                .child(Label::new(format!("{pct}%")).size(LabelSize::XSmall)),
        )
        .child(
            div()
                .w_full()
                .h(px(6.0))
                .rounded(px(3.0))
                .bg(theme.colors.surface_container_highest)
                .child(
                    div()
                        .h_full()
                        .w(gpui::relative(clamped))
                        .rounded(px(3.0))
                        .bg(theme.colors.primary),
                ),
        )
        .into_any_element()
}
