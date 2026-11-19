//! Native GPUI Kit showcase proving end-to-end Compose Material 3 1.5.0 Stable
//! (`https://m3.material.io/blog/compose-material-3-is-stable`), Parametric Prototyping
//! (`https://m3.material.io/blog/enhance-your-component-designs-with-parametric-prototyping`),
//! and Material 3 Figma Design Kit (`1035203688168086460`) inside Aphrody.

use gpui_ui::kit::{
    M3AppearanceMode, M3Compose15Catalog, M3ExpressiveThemeScope, M3FloatingToolbar,
    M3FloatingToolbarColor, M3LoadingIndicator, M3MaterialShape, M3MotionScheme,
    M3ParametricPrototyperView, M3SplitButton, M3ToggleButton, M3ToggleButtonVariant,
    M3WavyCircularProgressIndicator, M3WavyLinearProgressIndicator, apply_m3_expressive_theme,
};
use gpui_ui::prelude::*;

struct M3Compose15ShowcaseView {
    prototyper: Entity<M3ParametricPrototyperView>,
}

impl M3Compose15ShowcaseView {
    fn new(cx: &mut Context<Self>) -> Self {
        let prototyper = cx.new(M3ParametricPrototyperView::new);
        Self { prototyper }
    }
}

impl Render for M3Compose15ShowcaseView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let catalog = M3Compose15Catalog::canonical();
        div()
            .flex()
            .flex_col()
            .size_full()
            .gap_3()
            .p_4()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .child(format!(
                        "Compose Material 3 {} Stable (BOM {}) -> GPUI Kit + Parametric Studio (Figma {})",
                        catalog.material3_version, catalog.compose_bom, catalog.figma_kit_file_id
                    ))
                    .child(
                        M3FloatingToolbar::new("showcase-toolbar")
                            .color(M3FloatingToolbarColor::Vibrant)
                            .action(
                                M3ToggleButton::new("showcase-toggle", "Expressive")
                                    .variant(M3ToggleButtonVariant::FilledTonal)
                                    .checked(true),
                            )
                            .action(M3SplitButton::new("showcase-split", "Export Kit"))
                            .action(
                                M3LoadingIndicator::new("showcase-shape")
                                    .contained(true)
                                    .shapes(M3MaterialShape::Flower, M3MaterialShape::Cookie9Sided)
                                    .size_px(36.0),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_4()
                    .child(
                        M3WavyLinearProgressIndicator::new("showcase-wavy-linear")
                            .progress(Some(0.72))
                            .width_px(320.0),
                    )
                    .child(
                        M3WavyCircularProgressIndicator::new("showcase-wavy-circular")
                            .progress(Some(0.78))
                            .size_px(44.0),
                    ),
            )
            .child(self.prototyper.clone())
    }
}

fn main() {
    gpui_ui::application().run(|cx: &mut App| {
        gpui_ui::init(cx);
        let scope = M3ExpressiveThemeScope::new(M3AppearanceMode::NeuralObsidian)
            .motion_scheme(M3MotionScheme::Expressive)
            .default_font_family("Google Sans Flex");
        apply_m3_expressive_theme(&scope, cx);

        if let Err(err) = cx.open_window(WindowOptions::default(), |_, cx| {
            cx.new(M3Compose15ShowcaseView::new)
        }) {
            eprintln!("failed to launch Material 3 1.5.0 showcase: {err:#}");
        }
    });
}
