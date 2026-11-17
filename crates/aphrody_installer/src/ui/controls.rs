//! Reusable styled controls for `aphrody_installer` built over `nsh_base` + semantic `theme()`.

use nsh_base::{button, progress, switch, theme};
use nsh_core::prelude::*;

pub fn section_card(title: impl Into<SharedString>, content: impl IntoElement) -> impl IntoElement {
    let title: SharedString = title.into();
    view()
        .p_3()
        .gap_2()
        .rounded(move || theme().radius)
        .bg(move || theme().card)
        .border_1()
        .border_color(move || theme().border)
        .child(
            text(title)
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || theme().card_foreground),
        )
        .child(content)
}

pub fn choice_chip<F, S>(
    label: impl Into<SharedString>,
    selected: S,
    on_press: F,
) -> impl IntoElement
where
    F: Fn() + 'static,
    S: Fn() -> bool + 'static,
{
    let label: SharedString = label.into();
    let selected = std::rc::Rc::new(selected);
    let sel_btn = std::rc::Rc::clone(&selected);
    let sel_border = std::rc::Rc::clone(&selected);
    let sel_bg = std::rc::Rc::clone(&selected);
    button()
        .selected(move || sel_btn())
        .on_press(on_press)
        .px_3()
        .py_2()
        .rounded(move || theme().radius)
        .border_1()
        .border_color(move || {
            if sel_border() {
                theme().primary
            } else {
                theme().border
            }
        })
        .bg(move || {
            if sel_bg() {
                theme().primary
            } else {
                theme().secondary
            }
        })
        .active(|s| s.opacity(0.75))
        .child(text(label).text_color(move || {
            if selected() {
                theme().primary_foreground
            } else {
                theme().secondary_foreground
            }
        }))
}

pub fn action_button<F>(
    label: impl Into<SharedString>,
    primary: bool,
    on_press: F,
) -> impl IntoElement
where
    F: Fn() + 'static,
{
    let label: SharedString = label.into();
    button()
        .on_press(on_press)
        .px_4()
        .py_2()
        .rounded(move || theme().radius)
        .border_1()
        .border_color(move || {
            if primary {
                theme().primary
            } else {
                theme().border
            }
        })
        .bg(move || {
            if primary {
                theme().primary
            } else {
                theme().secondary
            }
        })
        .active(|s| s.opacity(0.75))
        .child(text(label).text_color(move || {
            if primary {
                theme().primary_foreground
            } else {
                theme().secondary_foreground
            }
        }))
}

pub fn switch_row<C, G>(
    label: impl Into<SharedString>,
    checked: G,
    on_change: C,
) -> impl IntoElement
where
    G: Fn() -> bool + Copy + 'static,
    C: Fn(bool) + 'static,
{
    let label: SharedString = label.into();
    switch()
        .checked(checked)
        .on_change(move |&next: &bool| on_change(next))
        .flex_row()
        .items_center()
        .gap_3()
        .track_class("w-11 rounded-full bg-muted p-0.5 transition-colors checked:bg-primary")
        .thumb_class("size-5 rounded-full bg-background transition-transform checked:translate-x-5")
        .child(text(label).text_color(move || theme().foreground))
}

pub fn step_row(phase: impl Into<SharedString>, detail: impl Into<SharedString>) -> impl IntoElement {
    let phase: SharedString = phase.into();
    let detail: SharedString = detail.into();
    view()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_2()
        .p_2()
        .rounded(move || theme().radius)
        .bg(move || theme().muted)
        .child(
            text(phase)
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || theme().primary),
        )
        .child(
            text(detail)
                .flex_1()
                .text_color(move || theme().muted_foreground),
        )
}

pub fn status_banner(
    status: impl Fn() -> String + Copy + 'static,
    pct: impl Fn() -> f32 + Copy + 'static,
) -> impl IntoElement {
    view()
        .p_3()
        .gap_2()
        .rounded(move || theme().radius)
        .bg(move || theme().accent)
        .border_1()
        .border_color(move || theme().border)
        .child(text(move || status()).text_color(move || theme().accent_foreground))
        .child(
            progress()
                .value(pct)
                .track_class("h-2 w-full rounded-full bg-muted")
                .indicator_class("h-2 rounded-full bg-primary"),
        )
}
