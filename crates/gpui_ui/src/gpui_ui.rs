//! # `gpui_ui` — GPUI UI Engine & Design System
//!
//! Reusable Rust UI library combining:
//! - **`gpui` & `gpui_platform`**: GPU-accelerated hybrid immediate/retained UI framework
//!   supporting desktop (Windows DirectX, macOS Metal, Linux Vulkan/Blade/Wayland/X11),
//!   Web (`wasm32`), iOS (`gpui_ios`), Wasm/host embedding (`Application::run_embedded`),
//!   and headless testing.
//! - **`ui` & `ui_input`**: Production-tested UI primitives, layout helpers, buttons, popovers,
//!   context menus, modals, trees, tables, lists, scrollbars, tooltips, and input fields.
//! - **`theme` & `icons` & `assets`**: Full theming engine (light/dark/system, syntax tokens),
//!   vector icon set, and embedded default fonts/themes.

pub use anyhow;
pub use assets::{self, Assets};
pub use collections;
pub use component;
pub use file_icons;
pub use gpui::{self, App, Application, ApplicationHandle, Context, Entity, WeakEntity, Window};
pub use gpui_macros;
pub use gpui_platform;
pub use gpui_shared_string;
pub use gpui_util;
pub use icons::{self, IconName};
pub use menu;
pub use refineable;
pub use scheduler;
pub use sum_tree;
pub use syntax_theme;
pub use theme::{self, ActiveTheme, GlobalTheme, SystemAppearance, Theme, ThemeRegistry};
pub use ui::{self, prelude as ui_prelude};
pub use ui_input::{self, InputField};
pub use ui_macros;

#[cfg(feature = "nsh")]
pub mod nsh;

/// Convenient prelude for building GPUI views and components.
pub mod prelude {
    pub use gpui::{
        Action, App, AppContext as _, Application, ApplicationHandle, Context, Element, Entity,
        EventEmitter, FocusHandle, Focusable, Global, InteractiveElement, IntoElement, KeyBinding,
        ParentElement, Pixels, Render, RenderOnce, SharedString, StatefulInteractiveElement,
        Styled, Subscription, Task, WeakEntity, Window, WindowBounds, WindowOptions, actions, div,
        point, px, rems, size,
    };
    pub use icons::IconName;
    pub use theme::ActiveTheme;
    pub use ui::prelude::*;
    pub use ui_input::InputField;
}

/// Create a cross-platform [`Application`] preloaded with the default [`Assets`]
/// (fonts, icons, and themes).
pub fn application() -> Application {
    #[cfg(target_family = "wasm")]
    gpui_platform::web_init();
    let platform = gpui_platform::current_platform(false);
    Application::with_platform(platform).with_assets(Assets)
}

/// Create a headless [`Application`] suitable for CLI/background tools or tests.
pub fn headless_application() -> Application {
    #[cfg(target_family = "wasm")]
    gpui_platform::web_init();
    let platform = gpui_platform::current_platform(true);
    Application::with_platform(platform).with_assets(Assets)
}

/// Default standalone [`theme::ThemeSettingsProvider`] used when running `gpui_ui`
/// without the full `settings` / `theme_settings` workspace layer.
pub struct StandaloneThemeSettings {
    ui_font: gpui::Font,
    buffer_font: gpui::Font,
    ui_font_size: gpui::Pixels,
    buffer_font_size: gpui::Pixels,
    ui_density: theme::UiDensity,
}

impl Default for StandaloneThemeSettings {
    fn default() -> Self {
        let ui_font_name =
            brand::env_var("UI_FONT").unwrap_or_else(|_| brand::FONT_SANS_ALIAS.to_owned());
        let buffer_font_name =
            brand::env_var("MONO_FONT").unwrap_or_else(|_| brand::FONT_MONO_ALIAS.to_owned());
        Self {
            ui_font: gpui::font(ui_font_name),
            buffer_font: gpui::font(buffer_font_name),
            ui_font_size: gpui::px(14.0),
            buffer_font_size: gpui::px(13.0),
            ui_density: theme::UiDensity::Default,
        }
    }
}

impl theme::ThemeSettingsProvider for StandaloneThemeSettings {
    fn ui_font<'a>(&'a self, _cx: &'a App) -> &'a gpui::Font {
        &self.ui_font
    }

    fn buffer_font<'a>(&'a self, _cx: &'a App) -> &'a gpui::Font {
        &self.buffer_font
    }

    fn ui_font_size(&self, _cx: &App) -> gpui::Pixels {
        self.ui_font_size
    }

    fn buffer_font_size(&self, _cx: &App) -> gpui::Pixels {
        self.buffer_font_size
    }

    fn ui_density(&self, _cx: &App) -> theme::UiDensity {
        self.ui_density
    }
}

/// Initialize the core UI system (embedded fonts, standalone font/density settings,
/// and base themes) inside an [`App`].
pub fn init(cx: &mut App) {
    if let Err(err) = load_embedded_fonts(cx) {
        log::warn!("gpui_ui::init failed to load embedded fonts: {err:#}");
    }
    theme::init(theme::LoadThemes::JustBase, cx);
    theme::set_theme_settings_provider(Box::new(StandaloneThemeSettings::default()), cx);
}

fn load_embedded_fonts(cx: &App) -> anyhow::Result<()> {
    use gpui::AssetSource as _;

    let font_paths = Assets.list("fonts")?;
    let mut embedded_fonts = Vec::new();
    for font_path in font_paths {
        if font_path.ends_with(".ttf")
            && let Some(font_bytes) = Assets.load(&font_path)?
        {
            embedded_fonts.push(font_bytes);
        }
    }
    cx.text_system().add_fonts(embedded_fonts)
}

#[cfg(test)]
mod tests {
    use super::prelude::*;
    use gpui::TestAppContext;

    struct CounterView {
        count: usize,
    }

    impl Render for CounterView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            h_flex()
                .gap_2()
                .child(Label::new(format!("Count: {}", self.count)))
        }
    }

    #[gpui::test]
    fn test_gpui_ui_view_renders(cx: &mut TestAppContext) {
        cx.update(|cx| {
            super::init(cx);
        });
        let (view, cx) = cx.add_window_view(|_window, _cx| CounterView { count: 42 });
        view.read_with(cx, |view, _cx| {
            assert_eq!(view.count, 42);
        });
    }
}
