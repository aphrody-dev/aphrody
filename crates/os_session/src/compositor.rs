use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use taffy::TaffyTree;
use taffy::prelude::{Display, FlexDirection, LengthPercentageAuto, Style, auto, length};

pub type WorkspaceId = u32;
pub type ToplevelId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    #[must_use]
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    #[must_use]
    pub fn inset(&self, margin: f32) -> Self {
        Self {
            x: self.x + margin,
            y: self.y + margin,
            w: (self.w - margin * 2.0).max(0.0),
            h: (self.h - margin * 2.0).max(0.0),
        }
    }
}

/// COSMIC + Hyprland + Niri unified workspace arrangement strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TilingMode {
    #[default]
    Dwindle,
    CosmicGrid,
    NiriScroll,
    Floating,
}

impl TilingMode {
    pub const ALL: [Self; 4] = [
        Self::Dwindle,
        Self::CosmicGrid,
        Self::NiriScroll,
        Self::Floating,
    ];

    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Dwindle => "dwindle",
            Self::CosmicGrid => "cosmic_grid",
            Self::NiriScroll => "niri_scroll",
            Self::Floating => "floating",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Dwindle => "Dwindle",
            Self::CosmicGrid => "COSMIC Grid",
            Self::NiriScroll => "Strip Scroll",
            Self::Floating => "Floating",
        }
    }

    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|mode| mode.slug().eq_ignore_ascii_case(raw.trim()))
    }

    #[must_use]
    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|&m| m == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerKind {
    Background,
    Bottom,
    Top,
    Overlay,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct LayerAnchor: u32 {
        const TOP = 1;
        const BOTTOM = 2;
        const LEFT = 4;
        const RIGHT = 8;
    }
}

impl Serialize for LayerAnchor {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(self.bits())
    }
}

impl<'de> Deserialize<'de> for LayerAnchor {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bits = u32::deserialize(deserializer)?;
        Ok(Self::from_bits_truncate(bits))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerSurface {
    pub id: u32,
    pub name: String,
    pub kind: LayerKind,
    pub anchor: LayerAnchor,
    pub exclusive_zone: f32,
}

/// Manages Wayland-compatible `zwlr_layer_shell_v1` exclusive zones using `taffy::TaffyTree`.
#[derive(Debug, Default, Clone)]
pub struct LayerShellManager {
    pub surfaces: BTreeMap<u32, LayerSurface>,
}

impl LayerShellManager {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_surface(&mut self, surface: LayerSurface) {
        self.surfaces.insert(surface.id, surface);
    }

    pub fn remove_surface(&mut self, id: u32) {
        self.surfaces.remove(&id);
    }

    /// Computes usable content bounds via a Taffy flex tree reserving top/bottom/left/right panels.
    #[must_use]
    pub fn usable_area(&self, screen: Rect) -> Rect {
        let mut top = 0.0_f32;
        let mut bottom = 0.0_f32;
        let mut left = 0.0_f32;
        let mut right = 0.0_f32;

        for surface in self.surfaces.values() {
            if surface.exclusive_zone <= 0.0 || matches!(surface.kind, LayerKind::Overlay) {
                continue;
            }
            if surface.anchor.contains(LayerAnchor::TOP)
                && !surface.anchor.contains(LayerAnchor::BOTTOM)
            {
                top += surface.exclusive_zone;
            } else if surface.anchor.contains(LayerAnchor::BOTTOM)
                && !surface.anchor.contains(LayerAnchor::TOP)
            {
                bottom += surface.exclusive_zone;
            } else if surface.anchor.contains(LayerAnchor::LEFT)
                && !surface.anchor.contains(LayerAnchor::RIGHT)
            {
                left += surface.exclusive_zone;
            } else if surface.anchor.contains(LayerAnchor::RIGHT)
                && !surface.anchor.contains(LayerAnchor::LEFT)
            {
                right += surface.exclusive_zone;
            }
        }

        match Self::try_taffy_content_rect(screen, top, bottom, left, right) {
            Ok(rect) => rect,
            Err(err) => {
                log::warn!("taffy layer-shell layout fallback: {err}");
                Rect::new(
                    screen.x + left,
                    screen.y + top,
                    (screen.w - left - right).max(120.0),
                    (screen.h - top - bottom).max(120.0),
                )
            }
        }
    }

    fn try_taffy_content_rect(
        screen: Rect,
        top: f32,
        bottom: f32,
        left: f32,
        right: f32,
    ) -> Result<Rect, taffy::TaffyError> {
        let mut tree: TaffyTree<()> = TaffyTree::new();
        let top_node = tree.new_leaf(Style {
            size: taffy::Size {
                width: auto(),
                height: length(top),
            },
            flex_shrink: 0.0,
            ..Default::default()
        })?;
        let bottom_node = tree.new_leaf(Style {
            size: taffy::Size {
                width: auto(),
                height: length(bottom),
            },
            flex_shrink: 0.0,
            ..Default::default()
        })?;
        let left_node = tree.new_leaf(Style {
            size: taffy::Size {
                width: length(left),
                height: auto(),
            },
            flex_shrink: 0.0,
            ..Default::default()
        })?;
        let right_node = tree.new_leaf(Style {
            size: taffy::Size {
                width: length(right),
                height: auto(),
            },
            flex_shrink: 0.0,
            ..Default::default()
        })?;
        let content_node = tree.new_leaf(Style {
            flex_grow: 1.0,
            size: taffy::Size {
                width: auto(),
                height: auto(),
            },
            ..Default::default()
        })?;

        let middle = tree.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                flex_grow: 1.0,
                ..Default::default()
            },
            &[left_node, content_node, right_node],
        )?;
        let root = tree.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: taffy::Size {
                    width: length(screen.w.max(0.0)),
                    height: length(screen.h.max(0.0)),
                },
                margin: taffy::Rect {
                    left: LengthPercentageAuto::length(0.0),
                    right: LengthPercentageAuto::length(0.0),
                    top: LengthPercentageAuto::length(0.0),
                    bottom: LengthPercentageAuto::length(0.0),
                },
                ..Default::default()
            },
            &[top_node, middle, bottom_node],
        )?;
        tree.compute_layout(
            root,
            taffy::Size {
                width: taffy::AvailableSpace::Definite(screen.w.max(0.0)),
                height: taffy::AvailableSpace::Definite(screen.h.max(0.0)),
            },
        )?;
        let mid_layout = tree.layout(middle)?;
        let content_layout = tree.layout(content_node)?;

        Ok(Rect::new(
            screen.x + mid_layout.location.x + content_layout.location.x,
            screen.y + mid_layout.location.y + content_layout.location.y,
            content_layout.size.width.max(120.0),
            content_layout.size.height.max(120.0),
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToplevelState {
    pub id: ToplevelId,
    pub title: String,
    pub app_id: String,
    pub workspace_id: WorkspaceId,
    pub floating: bool,
    pub maximized: bool,
    pub minimized: bool,
    pub float_rect: Rect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceState {
    pub id: WorkspaceId,
    pub name: String,
    pub mode: TilingMode,
    pub scroll_offset_x: f32,
    pub window_ids: Vec<ToplevelId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacedWindow {
    pub id: ToplevelId,
    pub title: String,
    pub app_id: String,
    pub rect: Rect,
    pub focused: bool,
    pub maximized: bool,
    pub floating: bool,
}

/// Computes tiled or floating window rectangles for a workspace inside `usable`.
#[must_use]
pub fn layout_windows(
    usable: Rect,
    mode: TilingMode,
    gap: f32,
    windows: &[&ToplevelState],
) -> Vec<Rect> {
    let count = windows.len();
    if count == 0 {
        return Vec::new();
    }

    match mode {
        TilingMode::Floating => windows.iter().map(|w| w.float_rect).collect(),
        TilingMode::Dwindle => {
            let mut rest = usable.inset(gap * 0.5);
            let mut out = Vec::with_capacity(count);
            for i in 0..count {
                if i + 1 == count {
                    out.push(rest.inset(gap * 0.5));
                    break;
                }
                if rest.w >= rest.h {
                    let first_w = rest.w * 0.5;
                    out.push(Rect::new(rest.x, rest.y, first_w, rest.h).inset(gap * 0.5));
                    rest = Rect::new(rest.x + first_w, rest.y, rest.w - first_w, rest.h);
                } else {
                    let first_h = rest.h * 0.5;
                    out.push(Rect::new(rest.x, rest.y, rest.w, first_h).inset(gap * 0.5));
                    rest = Rect::new(rest.x, rest.y + first_h, rest.w, rest.h - first_h);
                }
            }
            out
        }
        TilingMode::CosmicGrid => {
            let cols = (count as f32).sqrt().ceil() as usize;
            let rows = count.div_ceil(cols);
            let area = usable.inset(gap * 0.5);
            let cell_h = area.h / rows as f32;
            let mut out = Vec::with_capacity(count);
            for i in 0..count {
                let row = i / cols;
                let col = i % cols;
                let items_in_row = if row + 1 == rows {
                    count - row * cols
                } else {
                    cols
                };
                let cell_w = area.w / items_in_row as f32;
                out.push(
                    Rect::new(
                        area.x + col as f32 * cell_w,
                        area.y + row as f32 * cell_h,
                        cell_w,
                        cell_h,
                    )
                    .inset(gap * 0.5),
                );
            }
            out
        }
        TilingMode::NiriScroll => {
            let area = usable.inset(gap * 0.5);
            let col_w = (area.w * 0.62).max(280.0);
            (0..count)
                .map(|i| {
                    Rect::new(area.x + i as f32 * col_w, area.y, col_w, area.h).inset(gap * 0.5)
                })
                .collect()
        }
    }
}

/// Unified desktop workspace + toplevel window + LayerShell compositor model.
#[derive(Debug, Clone)]
pub struct CompositorState {
    pub screen: Rect,
    pub gap: f32,
    pub layers: LayerShellManager,
    pub workspaces: BTreeMap<WorkspaceId, WorkspaceState>,
    pub active_workspace: WorkspaceId,
    pub toplevels: BTreeMap<ToplevelId, ToplevelState>,
    pub focused_toplevel: Option<ToplevelId>,
    next_id: ToplevelId,
}

impl CompositorState {
    #[must_use]
    pub fn new(screen: Rect, workspace_names: &[&str], default_mode: TilingMode) -> Self {
        let mut workspaces = BTreeMap::new();
        for (idx, name) in workspace_names.iter().enumerate() {
            let id = (idx + 1) as WorkspaceId;
            workspaces.insert(
                id,
                WorkspaceState {
                    id,
                    name: (*name).to_owned(),
                    mode: default_mode,
                    scroll_offset_x: 0.0,
                    window_ids: Vec::new(),
                },
            );
        }
        if workspaces.is_empty() {
            workspaces.insert(
                1,
                WorkspaceState {
                    id: 1,
                    name: "Studio".to_owned(),
                    mode: default_mode,
                    scroll_offset_x: 0.0,
                    window_ids: Vec::new(),
                },
            );
        }

        Self {
            screen,
            gap: 8.0,
            layers: LayerShellManager::new(),
            workspaces,
            active_workspace: 1,
            toplevels: BTreeMap::new(),
            focused_toplevel: None,
            next_id: 1,
        }
    }

    pub fn open_window(
        &mut self,
        title: impl Into<String>,
        app_id: impl Into<String>,
    ) -> ToplevelId {
        let id = self.next_id;
        self.next_id += 1;
        let offset = ((id - 1) % 6) as f32 * 28.0;
        let usable = self.layers.usable_area(self.screen);
        let float_rect = Rect::new(
            usable.x + 32.0 + offset,
            usable.y + 32.0 + offset,
            (usable.w * 0.56).max(360.0),
            (usable.h * 0.64).max(260.0),
        );

        self.toplevels.insert(
            id,
            ToplevelState {
                id,
                title: title.into(),
                app_id: app_id.into(),
                workspace_id: self.active_workspace,
                floating: false,
                maximized: false,
                minimized: false,
                float_rect,
            },
        );
        if let Some(ws) = self.workspaces.get_mut(&self.active_workspace) {
            ws.window_ids.push(id);
        }
        self.focused_toplevel = Some(id);
        id
    }

    pub fn switch_workspace(&mut self, workspace_id: WorkspaceId) -> bool {
        if !self.workspaces.contains_key(&workspace_id) {
            return false;
        }
        self.active_workspace = workspace_id;
        let last_win = self
            .workspaces
            .get(&workspace_id)
            .and_then(|ws| ws.window_ids.last().copied());
        self.focused_toplevel = last_win;
        true
    }

    pub fn set_active_mode(&mut self, mode: TilingMode) {
        if let Some(ws) = self.workspaces.get_mut(&self.active_workspace) {
            ws.mode = mode;
        }
    }

    pub fn cycle_active_mode(&mut self) -> TilingMode {
        if let Some(ws) = self.workspaces.get_mut(&self.active_workspace) {
            ws.mode = ws.mode.next();
            ws.mode
        } else {
            TilingMode::Dwindle
        }
    }

    pub fn focus_window(&mut self, id: ToplevelId) -> bool {
        if let Some(win) = self.toplevels.get(&id) {
            self.active_workspace = win.workspace_id;
            self.focused_toplevel = Some(id);
            true
        } else {
            false
        }
    }

    pub fn close_window(&mut self, id: ToplevelId) -> Option<ToplevelState> {
        let removed = self.toplevels.remove(&id)?;
        if let Some(ws) = self.workspaces.get_mut(&removed.workspace_id) {
            ws.window_ids.retain(|&wid| wid != id);
        }
        if self.focused_toplevel == Some(id) {
            self.focused_toplevel = self
                .workspaces
                .get(&self.active_workspace)
                .and_then(|ws| ws.window_ids.last().copied());
        }
        Some(removed)
    }

    pub fn toggle_maximize(&mut self, id: ToplevelId) -> bool {
        if let Some(win) = self.toplevels.get_mut(&id) {
            win.maximized = !win.maximized;
            true
        } else {
            false
        }
    }

    #[must_use]
    pub fn arrange_active_workspace(&self) -> Vec<PlacedWindow> {
        let usable = self.layers.usable_area(self.screen);
        let Some(ws) = self.workspaces.get(&self.active_workspace) else {
            return Vec::new();
        };
        let visible: Vec<&ToplevelState> = ws
            .window_ids
            .iter()
            .filter_map(|id| self.toplevels.get(id))
            .filter(|w| !w.minimized)
            .collect();

        if let Some(max_win) = visible.iter().find(|w| w.maximized) {
            return vec![PlacedWindow {
                id: max_win.id,
                title: max_win.title.clone(),
                app_id: max_win.app_id.clone(),
                rect: usable.inset(self.gap * 0.5),
                focused: self.focused_toplevel == Some(max_win.id),
                maximized: true,
                floating: max_win.floating,
            }];
        }

        let tiled: Vec<&ToplevelState> = visible.iter().copied().filter(|w| !w.floating).collect();
        let tiled_rects = layout_windows(usable, ws.mode, self.gap, &tiled);
        let mut tiled_map = BTreeMap::new();
        for (win, rect) in tiled.iter().zip(tiled_rects) {
            tiled_map.insert(win.id, rect);
        }

        visible
            .into_iter()
            .map(|win| PlacedWindow {
                id: win.id,
                title: win.title.clone(),
                app_id: win.app_id.clone(),
                rect: if win.floating {
                    win.float_rect
                } else {
                    tiled_map.get(&win.id).copied().unwrap_or(win.float_rect)
                },
                focused: self.focused_toplevel == Some(win.id),
                maximized: win.maximized,
                floating: win.floating,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taffy_layer_shell_reserves_panel_and_dock_zones() {
        let mut comp = CompositorState::new(
            Rect::new(0.0, 0.0, 1440.0, 900.0),
            &["Studio", "Code"],
            TilingMode::Dwindle,
        );
        comp.layers.set_surface(LayerSurface {
            id: 1,
            name: "top-panel".to_owned(),
            kind: LayerKind::Top,
            anchor: LayerAnchor::TOP | LayerAnchor::LEFT | LayerAnchor::RIGHT,
            exclusive_zone: 40.0,
        });
        comp.layers.set_surface(LayerSurface {
            id: 2,
            name: "bottom-dock".to_owned(),
            kind: LayerKind::Top,
            anchor: LayerAnchor::BOTTOM,
            exclusive_zone: 64.0,
        });

        let usable = comp.layers.usable_area(comp.screen);
        assert_eq!(usable.y, 40.0);
        assert_eq!(usable.h, 796.0);
        assert_eq!(usable.w, 1440.0);

        let a = comp.open_window("ArtCraft Studio", "aphrody.studio");
        let b = comp.open_window("Craft Inspector", "aphrody.inspector");
        assert_ne!(a, b);
        let placed = comp.arrange_active_workspace();
        assert_eq!(placed.len(), 2);
        assert!(placed[0].rect.w > 600.0 && placed[1].rect.w > 600.0);
    }
}
