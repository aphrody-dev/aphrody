//! Design Gallery: native GPUI mirrors of the design.google references and the Material 3
//! component matrix, rendered with the active desktop theme.

use aphrody_ui::{
    GoogleDesignTokens, MirrorAccentPalette, OsTheme, ReferenceGallery, ReferenceGalleryChange,
    ReferenceGalleryState, ShadcnGpuiTheme, ShapeScale,
};
use gpui::{AnyElement, App, prelude::*};
use ui::{Label, LabelCommon as _, LabelSize, v_flex};

use crate::apps::section_heading;

pub struct DesignGalleryState {
    gallery: ReferenceGalleryState,
    tokens: Result<(MirrorAccentPalette, ShapeScale), String>,
}

impl DesignGalleryState {
    #[must_use]
    pub fn bundled() -> Self {
        let tokens = GoogleDesignTokens::bundled().and_then(|tokens| {
            Ok((MirrorAccentPalette::from_tokens(&tokens)?, tokens.sys.shape))
        });
        Self {
            gallery: ReferenceGalleryState::default(),
            tokens: tokens.map_err(|err| {
                log::warn!("design gallery tokens unavailable: {err:#}");
                format!("{err:#}")
            }),
        }
    }

    pub fn apply(&mut self, change: &ReferenceGalleryChange) -> bool {
        self.gallery.apply(change)
    }
}

pub fn render_design_gallery_window(
    theme: &OsTheme,
    state: &DesignGalleryState,
    on_change: Box<dyn Fn(&ReferenceGalleryChange, &mut App) + 'static>,
) -> AnyElement {
    let content = match &state.tokens {
        Ok((accents, shape)) => {
            let starter_theme = ShadcnGpuiTheme::from_os_theme(theme, shape);
            ReferenceGallery::new("desktop-reference-gallery", &state.gallery, &starter_theme, accents)
                .on_change(move |change, _window, cx| on_change(change, cx))
                .into_any_element()
        }
        Err(message) => Label::new(format!("Design tokens could not be loaded: {message}"))
            .size(LabelSize::Small)
            .into_any_element(),
    };

    v_flex()
        .id("design-gallery-scroll")
        .size_full()
        .p_3()
        .gap_3()
        .overflow_y_scroll()
        .bg(theme.colors.surface)
        .child(section_heading(theme, format!("Google references · {}", theme.name)))
        .child(content)
        .into_any_element()
}
