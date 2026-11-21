//! Responsive layout rules of the video app as pure functions (no GPUI types), so every
//! breakpoint can be tested without a window. Numbers come from the behaviour spec
//! (`docs/specs/video-platform-behaviour.md`); where the spec is unverified the choice is
//! documented next to the constant.

pub const TOP_BAR_HEIGHT: f32 = 56.0;
pub const NAV_EXPANDED_WIDTH: f32 = 240.0;
pub const NAV_MINI_WIDTH: f32 = 72.0;
pub const PAGE_PADDING: f32 = 24.0;
pub const GRID_GAP: f32 = 16.0;
pub const GRID_ROW_GAP: f32 = 40.0;
pub const CHIP_BAR_HEIGHT: f32 = 56.0;
pub const SEARCH_MAX_WIDTH: f32 = 732.0;
pub const SEARCH_BOX_HEIGHT: f32 = 40.0;
pub const SPINNER_ROW_HEIGHT: f32 = 60.0;
pub const MINIPLAYER_WIDTH: f32 = 400.0;
pub const MINIPLAYER_VIDEO_HEIGHT: f32 = 225.0;
pub const MINIPLAYER_STRIP_HEIGHT: f32 = 76.0;
pub const MINIPLAYER_MARGIN: f32 = 16.0;
pub const CONTROL_BAR_HEIGHT: f32 = 59.0;
pub const SCRUBBER_HEIGHT: f32 = 6.0;
pub const CONTROLS_IDLE_HIDE_MS: u64 = 3_000;
pub const BATCH_SIZE: usize = 24;

/// Narrowest viewport (inclusive) at which no inline navigation exists, only the overlay drawer.
pub const NO_INLINE_NAV_MAX_WIDTH: f32 = 768.0;
/// The expanded rail is only allowed inline from this width. The spec forbids it below 1024
/// and shows mini at 1024; the exact crossover is unverified, so it is a design choice.
pub const EXPANDED_NAV_MIN_WIDTH: f32 = 1200.0;
/// Two-column watch layout starts here (spec: "roughly 1000-1100", unverified).
pub const WATCH_TWO_COLUMN_MIN_WIDTH: f32 = 1100.0;
/// Short-form viewer overlays its rail and metadata below this width.
pub const SHORTS_NARROW_MAX_WIDTH: f32 = 600.0;
pub const SEARCH_COLLAPSE_MAX_WIDTH: f32 = 500.0;

/// Page families that share navigation rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageKind {
    List,
    Watch,
    Shorts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavMode {
    Expanded,
    Mini,
    Hidden,
}

impl NavMode {
    #[must_use]
    pub const fn width(self) -> f32 {
        match self {
            Self::Expanded => NAV_EXPANDED_WIDTH,
            Self::Mini => NAV_MINI_WIDTH,
            Self::Hidden => 0.0,
        }
    }
}

/// What the menu button does at a given width and page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    /// Switch the inline rail between expanded and mini (persisted preference).
    ToggleRail,
    /// Open the expanded rail as an overlay drawer over a scrim.
    OpenDrawer,
}

#[must_use]
pub fn nav_mode(viewport_width: f32, page: PageKind, prefers_expanded: bool) -> NavMode {
    if page == PageKind::Watch || viewport_width <= NO_INLINE_NAV_MAX_WIDTH {
        NavMode::Hidden
    } else if viewport_width >= EXPANDED_NAV_MIN_WIDTH && prefers_expanded {
        NavMode::Expanded
    } else {
        NavMode::Mini
    }
}

#[must_use]
pub fn menu_action(viewport_width: f32, page: PageKind) -> MenuAction {
    if page == PageKind::Watch || viewport_width < EXPANDED_NAV_MIN_WIDTH {
        MenuAction::OpenDrawer
    } else {
        MenuAction::ToggleRail
    }
}

/// Width of the content area of a list page.
#[must_use]
pub fn content_width(viewport_width: f32, nav: NavMode) -> f32 {
    (viewport_width - nav.width() - 2.0 * PAGE_PADDING).max(0.0)
}

/// Grid columns: `floor((content + gap) / pitch)`, at least one. The spec gives a pitch of 436
/// (420 card + 16 gap) for windows of 1024 and wider; its table shows two columns at 768 where
/// that pitch would give one, so narrower windows use a 340 pitch (324 minimum card).
#[must_use]
pub fn grid_columns(viewport_width: f32, content_width: f32) -> usize {
    let pitch = if viewport_width >= 1024.0 {
        436.0
    } else {
        340.0
    };
    (((content_width + GRID_GAP) / pitch).floor() as usize).max(1)
}

#[must_use]
pub fn card_width(content_width: f32, columns: usize) -> f32 {
    let columns = columns.max(1) as f32;
    ((content_width - GRID_GAP * (columns - 1.0)) / columns).max(0.0)
}

/// Height of a 16:9 thumbnail for a card of this width.
#[must_use]
pub fn thumbnail_height(width: f32) -> f32 {
    width * 9.0 / 16.0
}

fn interpolate(points: &[(f32, f32)], x: f32) -> f32 {
    let Some(&(first_x, first_y)) = points.first() else {
        return 0.0;
    };
    if x <= first_x {
        return first_y;
    }
    for pair in points.windows(2) {
        let (x0, y0) = pair[0];
        let (x1, y1) = pair[1];
        if x <= x1 {
            return y0 + (y1 - y0) * (x - x0) / (x1 - x0);
        }
    }
    points.last().map_or(0.0, |&(_, y)| y)
}

/// Search result thumbnail width: 500 wide, 444 at 1024, 352 at 768, 176 at 400.
#[must_use]
pub fn search_thumbnail_width(viewport_width: f32) -> f32 {
    interpolate(
        &[
            (400.0, 176.0),
            (768.0, 352.0),
            (1024.0, 444.0),
            (1280.0, 500.0),
        ],
        viewport_width,
    )
}

/// Search field width: centred in the viewport with a fixed reserve for the start and end zones.
#[must_use]
pub fn search_box_width(viewport_width: f32) -> f32 {
    (viewport_width - 2.0 * 220.5).clamp(0.0, SEARCH_MAX_WIDTH)
}

#[must_use]
pub fn search_collapsed(viewport_width: f32) -> bool {
    viewport_width < SEARCH_COLLAPSE_MAX_WIDTH
}

/// Creator banner height: 207 at 1440, 138 at 1024, 103.5 at 768, 62 at 400.
#[must_use]
pub fn creator_banner_height(viewport_width: f32) -> f32 {
    interpolate(
        &[
            (400.0, 62.0),
            (768.0, 103.5),
            (1024.0, 138.0),
            (1440.0, 207.0),
        ],
        viewport_width,
    )
}

/// Creator avatar diameter: 160 from 1024, 120 from 768, 72 below.
#[must_use]
pub fn creator_avatar_size(viewport_width: f32) -> f32 {
    if viewport_width >= 1024.0 {
        160.0
    } else if viewport_width >= 768.0 {
        120.0
    } else {
        72.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WatchLayout {
    pub two_column: bool,
    pub theater: bool,
    pub player_width: f32,
    pub player_height: f32,
    pub secondary_width: f32,
    /// In theater mode the related list moves under the player instead of beside it.
    pub secondary_below_player: bool,
}

/// Watch view geometry. `page_width` is the viewport width (the navigation is always an overlay
/// here), `aspect` is video width over height.
#[must_use]
pub fn watch_layout(
    page_width: f32,
    viewport_height: f32,
    aspect: f32,
    theater: bool,
) -> WatchLayout {
    let aspect = if aspect.is_finite() && aspect > 0.0 {
        aspect
    } else {
        16.0 / 9.0
    };
    let two_column = page_width >= WATCH_TWO_COLUMN_MIN_WIDTH;
    let secondary_width = if two_column {
        (page_width * 0.2834).clamp(300.0, 426.0)
    } else {
        page_width - 2.0 * PAGE_PADDING
    };
    let primary_width = if two_column {
        page_width - 32.0 - secondary_width
    } else {
        page_width - 2.0 * PAGE_PADDING
    };
    let theater_height_cap = (viewport_height - 169.0).max(0.0);
    let (player_width, player_height) = if theater {
        let width = page_width;
        (width, (width / aspect).min(theater_height_cap))
    } else {
        (primary_width, (primary_width / aspect).max(480.0))
    };
    WatchLayout {
        two_column,
        theater,
        player_width,
        player_height,
        secondary_width,
        secondary_below_player: theater && two_column,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShortsLayout {
    pub narrow: bool,
    pub container_width: f32,
    pub container_height: f32,
    pub player_width: f32,
    pub player_height: f32,
    /// Left edge of the player inside the container.
    pub player_x: f32,
    pub rail_overlay: bool,
    pub nav_buttons: bool,
}

#[must_use]
pub fn shorts_layout(viewport_width: f32, viewport_height: f32, nav: NavMode) -> ShortsLayout {
    let narrow = viewport_width < SHORTS_NARROW_MAX_WIDTH;
    if narrow {
        let width = (viewport_width - 52.0).max(0.0);
        let container_height = (viewport_height - TOP_BAR_HEIGHT - 40.0).max(0.0);
        let player_height = (width * 16.0 / 9.0).min(container_height);
        ShortsLayout {
            narrow,
            container_width: width,
            container_height,
            player_width: player_height * 9.0 / 16.0,
            player_height,
            player_x: 0.0,
            rail_overlay: true,
            nav_buttons: false,
        }
    } else {
        let container_width = (viewport_width - nav.width()).max(0.0);
        let container_height = (viewport_height - TOP_BAR_HEIGHT - 8.0 - 32.0).max(0.0);
        let player_width = (container_height * 9.0 / 16.0).min(container_width);
        ShortsLayout {
            narrow,
            container_width,
            container_height,
            player_width,
            player_height: container_height,
            player_x: (container_width - player_width) / 2.0,
            rail_overlay: false,
            nav_buttons: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn columns_at(viewport: f32, nav: NavMode) -> usize {
        grid_columns(viewport, content_width(viewport, nav))
    }

    #[test]
    fn grid_columns_follow_the_responsive_table() {
        assert_eq!(columns_at(1920.0, NavMode::Expanded), 3);
        assert_eq!(columns_at(1920.0, NavMode::Mini), 4);
        assert_eq!(columns_at(1440.0, NavMode::Mini), 3);
        assert_eq!(columns_at(1440.0, NavMode::Expanded), 2);
        assert_eq!(columns_at(1024.0, NavMode::Mini), 2);
        assert_eq!(columns_at(768.0, NavMode::Hidden), 2);
        assert_eq!(columns_at(400.0, NavMode::Hidden), 1);
    }

    #[test]
    fn card_fills_the_grid_exactly() {
        let content = content_width(1440.0, NavMode::Mini);
        let columns = grid_columns(1440.0, content);
        let width = card_width(content, columns);
        assert!(
            (width * columns as f32 + GRID_GAP * (columns as f32 - 1.0) - content).abs() < 0.01
        );
        assert!((width - 429.3).abs() < 1.0);
        assert!((thumbnail_height(420.0) - 236.25).abs() < 0.01);
    }

    #[test]
    fn nav_collapses_at_breakpoints() {
        assert_eq!(nav_mode(1440.0, PageKind::List, true), NavMode::Expanded);
        assert_eq!(nav_mode(1440.0, PageKind::List, false), NavMode::Mini);
        assert_eq!(nav_mode(1024.0, PageKind::List, true), NavMode::Mini);
        assert_eq!(nav_mode(769.0, PageKind::Shorts, true), NavMode::Mini);
        assert_eq!(nav_mode(768.0, PageKind::List, true), NavMode::Hidden);
        assert_eq!(nav_mode(400.0, PageKind::List, false), NavMode::Hidden);
        assert_eq!(nav_mode(1920.0, PageKind::Watch, true), NavMode::Hidden);
    }

    #[test]
    fn menu_button_toggles_rail_or_opens_drawer() {
        assert_eq!(menu_action(1440.0, PageKind::List), MenuAction::ToggleRail);
        assert_eq!(menu_action(1024.0, PageKind::List), MenuAction::OpenDrawer);
        assert_eq!(menu_action(1440.0, PageKind::Watch), MenuAction::OpenDrawer);
        assert_eq!(
            content_width(1440.0, NavMode::Expanded),
            1440.0 - 240.0 - 48.0
        );
    }

    #[test]
    fn search_geometry_matches_samples() {
        assert_eq!(search_thumbnail_width(1440.0), 500.0);
        assert_eq!(search_thumbnail_width(1024.0), 444.0);
        assert_eq!(search_thumbnail_width(768.0), 352.0);
        assert_eq!(search_thumbnail_width(400.0), 176.0);
        assert_eq!(search_box_width(1440.0), SEARCH_MAX_WIDTH);
        assert_eq!(search_box_width(1024.0), 583.0);
        assert!(search_collapsed(400.0) && !search_collapsed(768.0));
    }

    #[test]
    fn creator_geometry_matches_samples() {
        assert_eq!(creator_banner_height(1440.0), 207.0);
        assert_eq!(creator_banner_height(1024.0), 138.0);
        assert_eq!(creator_banner_height(768.0), 103.5);
        assert_eq!(creator_banner_height(400.0), 62.0);
        assert_eq!(creator_avatar_size(1440.0), 160.0);
        assert_eq!(creator_avatar_size(768.0), 120.0);
        assert_eq!(creator_avatar_size(400.0), 72.0);
    }

    #[test]
    fn watch_layout_matches_the_1440_measurement() {
        let default_mode = watch_layout(1424.8, 900.0, 16.0 / 9.0, false);
        assert!(default_mode.two_column);
        assert!((default_mode.player_width - 989.0).abs() < 1.0);
        assert!((default_mode.player_height - 556.3).abs() < 1.0);
        assert!((default_mode.secondary_width - 403.8).abs() < 1.0);
        assert!(!default_mode.secondary_below_player);

        let theater = watch_layout(1424.8, 900.0, 16.0 / 9.0, true);
        assert!((theater.player_width - 1424.8).abs() < 0.01);
        assert!((theater.player_height - 731.0).abs() < 0.01);
        assert!(theater.secondary_below_player);

        let tall = watch_layout(1424.8, 900.0, 4.0 / 3.0, false);
        assert!((tall.player_height - 741.8).abs() < 1.5);

        let narrow = watch_layout(1024.0, 768.0, 16.0 / 9.0, false);
        assert!(!narrow.two_column);
        assert_eq!(narrow.player_width, 976.0);
    }

    #[test]
    fn shorts_layout_matches_the_1440_and_400_samples() {
        let wide = shorts_layout(1440.0, 900.0, NavMode::Mini);
        assert_eq!(wide.container_height, 804.0);
        assert!((wide.player_width - 452.25).abs() < 0.01);
        assert!((wide.player_x - (wide.container_width - wide.player_width) / 2.0).abs() < 0.01);
        assert!(wide.nav_buttons && !wide.rail_overlay);

        let narrow = shorts_layout(400.0, 800.0, NavMode::Hidden);
        assert!(narrow.narrow && narrow.rail_overlay && !narrow.nav_buttons);
        assert_eq!(narrow.container_width, 348.0);
        assert!((narrow.player_height - 618.7).abs() < 0.1);
    }
}
