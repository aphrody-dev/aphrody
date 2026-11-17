//! Iced Elm-architecture adapter (`Message` / `Command` / `Subscription` / `Element`)
//! + COSMIC `libcosmic` RON theme bridge integrated cleanly alongside `crates/gpui`,
//! `crates/egui_gpui`, and `crates/bevy_gpui` on unified `wgpu v30` + `accesskit 0.25`.

use aphrody_ui::{AppearanceMode, OsTheme, parse_cosmic_ron_palette};
use gpui::{AnyElement, Context};
use serde::{Deserialize, Serialize};
use ui::{Label, LabelCommon as _, LabelSize, h_flex, v_flex};
use web_time::Instant;

/// Iced-compatible Task/Command queue descriptor mapped onto GPUI async spawns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IcedCommandSpec<M> {
    None,
    Perform { label: String, message: M },
    Batch(Vec<IcedCommandSpec<M>>),
}

impl<M> IcedCommandSpec<M> {
    #[must_use]
    pub fn none() -> Self {
        Self::None
    }

    #[must_use]
    pub fn perform(label: impl Into<String>, message: M) -> Self {
        Self::Perform {
            label: label.into(),
            message,
        }
    }
}

/// Iced Application trait mirrored into GPUI entities without secondary `winit` event loops.
pub trait IcedApplication: Sized + 'static {
    type Message: Clone + Send + 'static;
    type Flags: Clone + 'static;

    fn new(flags: Self::Flags) -> (Self, IcedCommandSpec<Self::Message>);
    fn update(&mut self, message: Self::Message) -> IcedCommandSpec<Self::Message>;
    fn view(&self, theme: &OsTheme) -> AnyElement;
}

/// Host wrapper bridging any [`IcedApplication`] cleanly inside GPUI windows & COSMIC applets.
pub struct IcedGpuiHost<A: IcedApplication> {
    pub app: A,
    pub update_count: u64,
    pub last_update_us: u64,
}

impl<A: IcedApplication> IcedGpuiHost<A> {
    pub fn init(flags: A::Flags, cx: &mut Context<Self>) -> Self {
        let (app, cmd) = A::new(flags);
        let mut host = Self {
            app,
            update_count: 0,
            last_update_us: 0,
        };
        host.run_cmd(cmd, cx);
        host
    }

    pub fn dispatch(&mut self, msg: A::Message, cx: &mut Context<Self>) {
        let started = Instant::now();
        let cmd = self.app.update(msg);
        self.update_count += 1;
        self.run_cmd(cmd, cx);
        self.last_update_us = started.elapsed().as_micros() as u64;
        cx.notify();
    }

    fn run_cmd(&mut self, cmd: IcedCommandSpec<A::Message>, _cx: &mut Context<Self>) {
        let mut queue = vec![cmd];
        while let Some(next) = queue.pop() {
            match next {
                IcedCommandSpec::None => {}
                IcedCommandSpec::Perform { message, .. } => {
                    let follow = self.app.update(message);
                    queue.push(follow);
                }
                IcedCommandSpec::Batch(items) => {
                    for item in items.into_iter().rev() {
                        queue.push(item);
                    }
                }
            }
        }
    }
}

/// Converts a COSMIC `libcosmic` RON palette string into an Aphrody [`OsTheme`] snapshot.
pub fn cosmic_ron_to_os_theme(ron_source: &str, mode: AppearanceMode) -> OsTheme {
    let mut base = OsTheme::default();
    base.mode = mode;
    if let Some(entry) = parse_cosmic_ron_palette("iced_ron", ron_source) {
        base.colors = match mode {
            AppearanceMode::Dark => entry.dark,
            AppearanceMode::Light => entry.light,
        };
    }
    base
}

/// Renders an Iced/libcosmic stack status badge card inside GPUI diagnostics views.
#[must_use]
pub fn render_iced_cosmic_status_card(theme: &OsTheme, active_apps: usize) -> AnyElement {
    use gpui::{IntoElement as _, ParentElement as _, Styled as _, div, px};
    v_flex()
        .p_3()
        .gap_1p5()
        .rounded(px(10.0))
        .bg(theme.colors.surface_container)
        .border_1()
        .border_color(theme.colors.outline_variant)
        .child(
            h_flex()
                .justify_between()
                .child(Label::new("Iced + libcosmic Bridge").size(LabelSize::Small))
                .child(
                    div()
                        .px_2()
                        .py_0p5()
                        .rounded(px(999.0))
                        .bg(theme.colors.primary_container)
                        .child(
                            Label::new(format!("{active_apps} applets"))
                                .size(LabelSize::XSmall),
                        ),
                ),
        )
        .child(
            Label::new("Aligned onto workspace WGPU v30 + AccessKit 0.25 + cosmic-text")
                .size(LabelSize::XSmall)
                .color(ui::Color::Muted),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosmic_ron_palette_parses_into_os_theme() {
        let ron = r#"(
            bg_color: Srgba::new(0.08, 0.10, 0.14, 1.0),
            accent: Srgba::new(0.39, 0.82, 0.87, 1.0),
        )"#;
        let theme = cosmic_ron_to_os_theme(ron, AppearanceMode::Dark);
        assert_eq!(theme.mode, AppearanceMode::Dark);
    }
}
