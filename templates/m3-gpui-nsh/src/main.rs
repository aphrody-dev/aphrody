//! Starter template unifying Compose Material 3 1.5.0 Stable,
//! Material 3 Parametric Prototyping, Figma Design Kit (1035203688168086460),
//! GPUI Kit, WGPU/WGSL shaders, and NSH TSX.

use gpui_ui::kit::{
    M3AppearanceMode, M3ExpressiveThemeScope, M3MotionScheme, M3ParametricPrototyperView,
    apply_m3_expressive_theme,
};
use gpui_ui::prelude::*;

fn main() {
    gpui_ui::application().run(|cx: &mut App| {
        gpui_ui::init(cx);
        let scope = M3ExpressiveThemeScope::new(M3AppearanceMode::NeuralObsidian)
            .motion_scheme(M3MotionScheme::Expressive)
            .default_font_family("Google Sans Flex");
        apply_m3_expressive_theme(&scope, cx);

        if let Err(err) = cx.open_window(WindowOptions::default(), |_, cx| {
            cx.new(M3ParametricPrototyperView::new)
        }) {
            eprintln!("failed to open Material 3 parametric studio window: {err:#}");
        }
    });
}
